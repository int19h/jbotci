"""Exercise damaged records, directory membership, and declaration checks."""
from __future__ import annotations

from contextlib import redirect_stderr
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from tools import cargo_inputs as c
from tools.cargo_inputs_trace import Parser, Process, process_snapshot, string_at


class CargoUnitTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.workspace = self.root / "source"
        self.workspace.mkdir()
        self.layout = c.Layout(self.workspace, self.root / "target")

    def test_workspace_paths_are_relative(self) -> None:
        path = self.workspace / "src/lib.rs"
        self.assertEqual(self.layout.path_key(path), "workspace:src/lib.rs")
        self.assertEqual(self.layout.path("workspace:src/lib.rs"), path)

    def test_relative_path_cannot_escape_its_root(self) -> None:
        for key in ("workspace:../secret", "target:/outside", "unknown:foo"):
            with self.assertRaises(c.InputError):
                self.layout.path(key)

    def test_target_cannot_contain_the_checkout(self) -> None:
        for target in (self.workspace, self.root):
            with self.assertRaisesRegex(c.InputError, "must not contain"):
                c.Layout(self.workspace, target)

    def test_identical_dependency_fingerprints_bind_all_profiles(self) -> None:
        first = c.Unit("debug/child", self.layout.target / "debug/.fingerprint/child-a", "lib-child", c.Kind.COMPILE, {"deps": []}, 42)
        second = c.Unit("release/child", self.layout.target / "release/.fingerprint/child-b", "lib-child", c.Kind.COMPILE, {"deps": []}, 42)
        parent = c.Unit("release/parent", self.layout.target / "release/.fingerprint/parent-c", "lib-parent", c.Kind.COMPILE,
                        {"deps": [[0, "child", False, 42]]}, 43)
        units = {unit.key: unit for unit in (first, second, parent)}
        self.assertEqual(c.dependency_keys(parent, units), (first.key, second.key))

    def test_declared_target_directory_includes_its_files(self) -> None:
        directory = self.layout.target / "generated"
        directory.mkdir(parents=True)
        path = directory / "input"
        path.write_text("generated input")
        self.assertEqual(c.expand_directory(directory, self.layout.target), [path])
        self.assertEqual(c.directory_snapshot(self.layout, directory)["files"], ["target:generated/input"])

    def test_explicit_directory_keeps_target_and_git_children(self) -> None:
        layout = c.Layout(self.workspace, self.workspace / "target")
        layout.target.mkdir()
        (layout.target / "input").write_text("generated input")
        (self.workspace / ".git").write_text("repository pointer")
        self.assertEqual(c.directory_snapshot(layout, self.workspace)["files"],
                         ["target:input", "workspace:.git"])
        self.assertEqual(c.directory_snapshot(layout, self.workspace, package_default=True)["files"], [])

    def test_directory_membership_catches_added_and_deleted_files(self) -> None:
        directory = self.workspace / "data"
        directory.mkdir()
        first = c.directory_snapshot(self.layout, directory)
        path = directory / "input"
        path.write_text("one")
        second = c.directory_snapshot(self.layout, directory)
        self.assertNotEqual(first, second)
        path.unlink()
        self.assertEqual(first, c.directory_snapshot(self.layout, directory))

    def test_directory_membership_catches_empty_directories(self) -> None:
        directory = self.workspace / "data"
        directory.mkdir()
        before = c.directory_snapshot(self.layout, directory)
        (directory / "empty").mkdir()
        self.assertNotEqual(before, c.directory_snapshot(self.layout, directory))

    def test_dep_info_preserves_spaces_and_optional_environment_values(self) -> None:
        path = self.root / "source.d"
        path.write_text('out: src/with\\ space.rs src/lib.rs\n# env-dep:OPTIONAL\n# env-dep:URL=a\\nb\n')
        files, environment = c.rustc_dep_info(path, self.workspace)
        self.assertEqual(files, [self.workspace / "src/with space.rs", self.workspace / "src/lib.rs"])
        self.assertIsNone(environment["OPTIONAL"])
        self.assertEqual(environment["URL"], c.sha(b"a\nb"))

    def test_unknown_encoded_dep_info_fails_closed(self) -> None:
        path = self.root / "dep-info"
        path.write_bytes(b"unknown")
        with self.assertRaises(c.InputError):
            c.encoded_environment(path)

    def test_trace_decodes_filesystem_bytes(self) -> None:
        value, _ = string_at(r'"caf\303\251.txt"')
        self.assertEqual(value, "café.txt")

    def test_trace_rejects_abbreviated_arguments(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        parser.feed([b'1 execve("/bin/true", ["tr"...], ["OUT_DIR=/safe"]) = 0\n'])
        self.assertEqual(json.loads(output.getvalue().splitlines()[-1])["errors"], 1)

    def test_trace_does_not_store_credentials(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        parser.feed([b'1 execve("/bin/true", ["true", "secret-argument"], ["GH_TOKEN=secret-token"]) = 0\n'])
        self.assertNotIn("secret", output.getvalue())

    def test_trace_errors_report_only_sanitized_events(self) -> None:
        output = io.StringIO()
        diagnostic = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        with redirect_stderr(diagnostic):
            parser.feed([
                b'1 execve("/bin/true", ["secret-argument"...], ["GH_TOKEN=secret-token"]) = 0\n',
                b'1 openat(AT_FDCWD, "secret-path", O_RDONLY <unfinished ...>\n',
            ])
        self.assertEqual(parser.errors, 2)
        events = [json.loads(line.removeprefix("Input trace error: "))
                  for line in diagnostic.getvalue().splitlines()]
        self.assertEqual([event["event"] for event in events],
                         ["trace-error", "incomplete-processes"])
        self.assertEqual(events[0]["operation"], "execve")
        self.assertEqual(events[1]["pending"]["1"]["operation"], "openat")
        self.assertNotIn("secret", diagnostic.getvalue())
        self.assertNotIn("secret", output.getvalue())

    def test_trace_reads_flag_names_in_filenames_as_filenames(self) -> None:
        for name in ("O_DIRECTORY", "O_WRONLY", "O_RDWR"):
            output = io.StringIO()
            parser = Parser(self.workspace, self.layout.target, output)
            parser.processes[1] = Process(str(self.workspace), "unit")
            path = self.workspace / name
            parser.feed([f'1 openat(AT_FDCWD, "{path}", O_RDONLY|O_CLOEXEC) = 3<{path}>\n'.encode()])
            events = [json.loads(line) for line in output.getvalue().splitlines()]
            self.assertEqual(events[0]["event"], "open")
            self.assertTrue(events[0]["read"])
            self.assertFalse(events[0]["write"])
            self.assertEqual(events[-1]["errors"], 0)

    def test_trace_handles_child_exec_before_parent_clone_returns(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        parser.feed([
            b'1 clone(child_stack=NULL, flags=SIGCHLD <unfinished ...>\n',
            b'2 execve("/bin/true", ["true"], []) = 0\n',
            b'1 <... clone resumed>) = 2\n',
            b'2 openat(AT_FDCWD, "/source/data", O_RDONLY) = 3</source/data>\n',
        ])
        self.assertEqual(json.loads(output.getvalue().splitlines()[-1])["errors"], 0)

    def test_explicit_thread_exit_closes_strace_unknown_placeholder(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        parser.feed([
            b'1 clone3({flags=CLONE_THREAD}, 88) = 2\n',
            b'2 ???( <unfinished ...>\n',
            b'2 +++ exited with 0 +++\n',
            b'1 +++ exited with 0 +++\n',
        ])
        self.assertEqual(json.loads(output.getvalue().splitlines()[-1])["errors"], 0)

    def test_unknown_placeholder_without_exit_still_fails_closed(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        parser.feed([b'1 ???( <unfinished ...>\n'])
        self.assertEqual(json.loads(output.getvalue().splitlines()[-1])["errors"], 1)

    def test_killed_process_cannot_finish_a_pending_file_read(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        parser.feed([
            b'1 openat(AT_FDCWD, "pipe", O_RDONLY <unfinished ...>\n',
            b'1 +++ killed by SIGKILL +++\n',
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events, [{"event": "complete", "errors": 0}])

    def test_group_exit_before_clone_result_closes_only_teardown_events(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        # This ordering comes from the native thread-exit reproduction. The
        # process exits during clone3, so strace never reports the new thread ID.
        parser.feed([
            b'1 execve("/fixture/exit-thread", ["exit-thread"], []) = 0\n',
            b'1 clone3({flags=CLONE_VM|CLONE_FS|CLONE_FILES|CLONE_SIGHAND|CLONE_THREAD|CLONE_SYSVSEM|CLONE_SETTLS|CLONE_PARENT_SETTID|CLONE_CHILD_CLEARTID}, 88 <unfinished ...>\n',
            b'2 ???( <unfinished ...>\n',
            b'1 <... clone3 resumed> <unfinished ...>) = ?\n',
            b'2 <... ??? resumed>) = ?\n',
            b'1 +++ exited with 0 +++\n',
            b'2 +++ exited with 0 +++\n',
        ])
        self.assertEqual(parser.errors, 0)
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        proof = next(event for event in events if event["event"] == "unattributed-exit")
        self.assertEqual(proof["pid"], 2)
        self.assertEqual(proof["teardown_events"], 2)
        self.assertEqual(events[-1], {"event": "complete", "errors": 0})
        self.assertEqual(parser.waiting, {})
        session = self.root / "session"
        session.mkdir()
        (session / "trace-race.jsonl").write_text(output.getvalue())
        self.assertEqual(c.journals(session), ({}, {}, set()))

    def test_unknown_exited_process_with_a_file_open_still_fails_closed(self) -> None:
        output = io.StringIO()
        diagnostic = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output,
                        inspect_process=lambda pid: {"parent": 7, "exe": "/fixture/helper",
                                                     "cmdline": {"arguments": "<redacted>", "argument_count": 1}})
        with redirect_stderr(diagnostic):
            parser.feed([
                b'1 execve("/bin/true", ["true"], []) = 0\n',
                b'2<helper> execve("/fixture/helper", ["helper", "secret-argument"], ["TOKEN=secret-token"]) = 0\n',
                b'2<helper> openat(AT_FDCWD, "secret-file", O_RDONLY) = 3</source/secret-file>\n',
                b'2<helper> +++ exited with 0 +++\n',
                b'1 +++ exited with 0 +++\n',
            ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        failure = next(event for event in events if event["event"] == "incomplete-processes")
        self.assertEqual(events[-1]["errors"], 1)
        self.assertEqual(failure["waiting"], [2])
        identity = failure["waiting_processes"]["2"]
        self.assertEqual(identity["exe"], "/fixture/helper")
        self.assertEqual(identity["parent"], 7)
        self.assertEqual(identity["cmdline"], {"arguments": "<redacted>", "argument_count": 1})
        self.assertNotIn("secret", diagnostic.getvalue())
        self.assertNotIn("secret", output.getvalue())
        session = self.root / "session"
        session.mkdir()
        (session / "trace-incomplete.jsonl").write_text(output.getvalue())
        with self.assertRaisesRegex(c.InputError, "did not finish without errors"):
            c.journals(session)

    def test_child_open_and_exit_before_clone_return_replay_in_parent_context(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output,
                        inspect_process=lambda pid: {"parent": 99, "exe": "/unrelated"})
        parser.processes[1] = Process(str(self.workspace), "producing-unit")
        parser.feed([
            b'1 clone(child_stack=NULL, flags=SIGCHLD <unfinished ...>\n',
            b'2 openat(AT_FDCWD, "/source/input", O_RDONLY) = 3</source/input>\n',
            b'2 +++ exited with 0 +++\n',
            b'1 <... clone resumed>) = 2\n',
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        opened = next(event for event in events if event["event"] == "open")
        self.assertEqual(opened["unit"], "producing-unit")
        self.assertEqual(opened["path"], "/source/input")
        self.assertEqual(parser.processes[2].parent, 1)
        self.assertEqual(events[-1]["errors"], 0)

    def test_unknown_teardown_without_an_exit_remains_incomplete(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        with redirect_stderr(io.StringIO()):
            parser.feed([b'1 chdir("/source") = 0\n', b'2 ???( <unfinished ...>\n'])
        self.assertEqual(json.loads(output.getvalue().splitlines()[-1])["errors"], 1)

    def test_unsplit_teardown_and_exit_only_have_no_input_operations(self) -> None:
        for teardown in ([], [b'2 ???() = ?\n']):
            with self.subTest(teardown=teardown):
                output = io.StringIO()
                parser = Parser(self.workspace, self.layout.target, output)
                parser.feed([b'1 chdir("/source") = 0\n', *teardown,
                             b'2 +++ exited with 0 +++\n'])
                events = [json.loads(line) for line in output.getvalue().splitlines()]
                proof = next(event for event in events if event["event"] == "unattributed-exit")
                self.assertEqual(proof["teardown_events"], len(teardown))
                self.assertEqual(events[-1]["errors"], 0)

    def test_other_unknown_operations_cannot_use_the_teardown_exception(self) -> None:
        for operation in (b'2 ???(42) = ?\n', b'2 chdir("/source") = 0\n',
                          b'2 openat(AT_FDCWD, "input", O_RDONLY) = -1 ENOENT\n'):
            with self.subTest(operation=operation):
                output = io.StringIO()
                parser = Parser(self.workspace, self.layout.target, output)
                with redirect_stderr(io.StringIO()):
                    parser.feed([b'1 chdir("/source") = 0\n', operation,
                                 b'2 +++ exited with 0 +++\n'])
                self.assertEqual(json.loads(output.getvalue().splitlines()[-1])["errors"], 1)

    def test_process_identity_inherits_parent_without_retaining_arguments(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        parser.feed([
            b'1<parent> execve("/bin/parent", ["parent", "secret-argument"], []) = 0\n',
            b'1<parent> clone(child_stack=NULL, flags=SIGCHLD) = 2<parent>\n',
            b'2<parent> ???( <unfinished ...>\n',
            b'2<parent> +++ exited with 0 +++\n',
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        child = next(event for event in events if event["event"] == "process" and event["pid"] == 2)
        self.assertEqual(child["parent"], 1)
        self.assertEqual(child["exe"], "/bin/parent")
        self.assertEqual(child["cmdline"]["argument_count"], 1)
        self.assertNotIn("secret", output.getvalue())

    def test_live_process_snapshot_discards_argument_values(self) -> None:
        with patch("tools.cargo_inputs_trace.Path.read_text", return_value="PPid:\t7\nTgid:\t2\n"), \
             patch("tools.cargo_inputs_trace.os.readlink", return_value="/bin/helper"), \
             patch("tools.cargo_inputs_trace.Path.read_bytes", return_value=b"helper\0secret-token\0"):
            identity = process_snapshot(2)
        self.assertEqual(identity["parent"], 7)
        self.assertEqual(identity["tgid"], 2)
        self.assertEqual(identity["exe"], "/bin/helper")
        self.assertEqual(identity["cmdline"], {"arguments": "<redacted>", "argument_count": 1})
        self.assertNotIn("secret", json.dumps(identity))

    def test_pending_operation_diagnostics_do_not_contain_resumed_arguments(self) -> None:
        output = io.StringIO()
        diagnostic = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        with redirect_stderr(diagnostic):
            parser.feed([b'1 <... execve resumed> ["secret-argument"] <unfinished ...>\n'])
        self.assertNotIn("secret", diagnostic.getvalue())
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[0]["pending"]["1"]["operation"], "execve")

    def test_inventory_reads_both_fingerprint_shapes(self) -> None:
        classic = self.layout.target / "debug/.fingerprint/demo-0123456789abcdef"
        build_unit = self.layout.target / "opt/build/demo/fedcba9876543210/fingerprint"
        self.write_fingerprint(classic, "bin-demo", 42)
        self.write_fingerprint(build_unit, "run-build-script-build-script-build", 42)
        units = c.inventory(self.layout)
        classic_key = classic.relative_to(self.layout.target).as_posix()
        build_key = build_unit.relative_to(self.layout.target).as_posix()
        self.assertEqual(set(units), {classic_key, build_key})
        self.assertEqual(units[classic_key].shape, c.FingerprintShape.CLASSIC)
        self.assertEqual(units[classic_key].kind, c.Kind.COMPILE)
        self.assertEqual(units[classic_key].cargo_hash, 42)
        self.assertEqual(units[classic_key].suffix, "0123456789abcdef")
        self.assertEqual(units[build_key].shape, c.FingerprintShape.BUILD_UNIT)
        self.assertEqual(units[build_key].kind, c.Kind.RUN)
        self.assertEqual(units[build_key].cargo_hash, 42)
        self.assertEqual(units[build_key].suffix, "fedcba9876543210")
        self.assertEqual(units[build_key].profile, self.layout.target / "opt")

    def test_unknown_fingerprint_entry_fails_closed(self) -> None:
        fingerprints = self.layout.target / "debug/.fingerprint"
        fingerprints.mkdir(parents=True)
        (fingerprints / "notes").write_text("not a unit")
        with self.assertRaisesRegex(c.InputError, "Unknown fingerprint entry"):
            c.inventory(self.layout)

    def test_two_metadata_files_fail_closed(self) -> None:
        directory = self.layout.target / "debug/build/demo/0123456789abcdef/fingerprint"
        self.write_fingerprint(directory, "bin-demo", 7)
        (directory / "other.json").write_text('{"deps":[]}')
        with self.assertRaisesRegex(c.InputError, "one metadata file"):
            c.inventory(self.layout)

    def test_a_short_fingerprint_hash_fails_closed(self) -> None:
        directory = self.layout.target / "debug/.fingerprint/demo-0123456789abcdef"
        directory.mkdir(parents=True)
        (directory / "bin-demo").write_text("2a")
        (directory / "bin-demo.json").write_text('{"deps":[]}')
        with self.assertRaisesRegex(c.InputError, "16 hex characters"):
            c.inventory(self.layout)

    def test_a_fingerprint_file_in_place_of_a_directory_fails_closed(self) -> None:
        unit = self.layout.target / "debug/build/demo/0123456789abcdef"
        unit.mkdir(parents=True)
        (unit / "fingerprint").write_text("not a directory")
        with self.assertRaisesRegex(c.InputError, "not a directory"):
            c.inventory(self.layout)

    def test_build_script_output_follows_the_fingerprint_shape(self) -> None:
        classic_dir = self.layout.target / "debug/.fingerprint/demo-0123456789abcdef"
        classic_dir.mkdir(parents=True)
        classic_output = self.layout.target / "debug/build/demo-0123456789abcdef/output"
        classic_output.parent.mkdir(parents=True)
        classic_output.write_text("cargo:rerun-if-changed=build.rs\n")
        classic = c.Unit(
            "classic", classic_dir, "run-build-script-build-script-build",
            c.Kind.RUN, {"deps": []}, 1, c.FingerprintShape.CLASSIC,
        )
        self.assertEqual(c.build_script_output(classic), classic_output)
        build_dir = self.layout.target / "opt/build/demo/fedcba9876543210/fingerprint"
        build_dir.mkdir(parents=True)
        build_output = build_dir.parent / "run/stdout"
        build_output.parent.mkdir(parents=True)
        build_output.write_text("cargo:rerun-if-changed=version.txt\n")
        build_unit = c.Unit(
            "build-unit", build_dir, "run-build-script-build-script-build",
            c.Kind.RUN, {"deps": []}, 1, c.FingerprintShape.BUILD_UNIT,
        )
        self.assertEqual(c.build_script_output(build_unit), build_output)
        self.assertEqual(build_unit.profile, self.layout.target / "opt")

    def test_encoded_dep_info_bytes_match_both_observed_shapes(self) -> None:
        # Cargo 1.99.1 and Cargo 1.100.0 write these same bytes for src/main.rs.
        path = self.root / "dep-bin"
        path.write_bytes(bytes.fromhex(
            "01000000ff0101000000000b0000007372632f6d61696e2e72730000000000"))
        self.assertEqual(c.encoded_environment(path), {})

    def test_trace_binds_both_compiler_layouts(self) -> None:
        target = self.layout.target
        classic = target / "debug/.fingerprint/demo-0123456789abcdef"
        classic.mkdir(parents=True)
        plain = target / "debug/build/demo/fedcba9876543210/fingerprint"
        plain.mkdir(parents=True)
        hashed = target / "debug/build/demo/0123456789abcdef/fingerprint"
        hashed.mkdir(parents=True)
        # The compile unit that produced the build script, not the run unit.
        (hashed / "build-script-build-script-build").write_text("0123456789abcdef")
        output = io.StringIO()
        parser = Parser(self.workspace, target, output)
        environment = '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]'
        classic_out = target / "debug/deps"
        plain_out = target / "debug/build/demo/fedcba9876543210/out"
        hashed_out = target / "debug/build/demo/0123456789abcdef/out"
        classic_script = target / "debug/build/demo-aaaaaaaaaaaaaaaa/build-script-build"
        classic_script_out = target / "debug/build/demo-0123456789abcdef/out"
        parser.feed([
            (
                f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{classic_out}", '
                f'"-C", "extra-filename=-0123456789abcdef", "-C", "metadata=0123456789abcdef"], {environment}) = 0\n'
            ).encode(),
            (
                f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{plain_out}", '
                f'"-C", "metadata=aaaaaaaaaaaaaaaa"], {environment}) = 0\n'
            ).encode(),
            (
                f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{hashed_out}", '
                f'"-C", "extra-filename=-0123456789abcdef", "-C", "metadata=0123456789abcdef"], {environment}) = 0\n'
            ).encode(),
            (
                f'1 execve("{hashed_out}/build_script_build", ["{hashed_out}/build_script_build"], '
                f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={plain_out}"]) = 0\n'
            ).encode(),
            (
                f'1 execve("{classic_script}", ["{classic_script}"], '
                f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={classic_script_out}"]) = 0\n'
            ).encode(),
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        units = [event for event in events if event["event"] == "unit"]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual(
            [(event["kind"], event["unit"], event["dep_info"]) for event in units],
            [
                ("compile", "debug/.fingerprint/demo-0123456789abcdef",
                 str(classic_out / "demo-0123456789abcdef.d")),
                ("compile", "debug/build/demo/fedcba9876543210/fingerprint",
                 str(plain_out / "demo.d")),
                ("compile", "debug/build/demo/0123456789abcdef/fingerprint",
                 str(hashed_out / "demo-0123456789abcdef.d")),
                ("run", "debug/build/demo/fedcba9876543210/fingerprint", None),
                ("run", "debug/.fingerprint/demo-0123456789abcdef", None),
            ],
        )

    def test_trace_matches_custom_build_scripts_by_place(self) -> None:
        target = self.layout.target
        environment = '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]'
        classic_configure = target / "debug/.fingerprint/demo-1111111111111111"
        classic_main = target / "debug/.fingerprint/demo-2222222222222222"
        classic_configure.mkdir(parents=True)
        classic_main.mkdir(parents=True)
        configure_compile = target / "debug/build/pkg/0123456789abcdef/fingerprint"
        main_compile = target / "debug/build/pkg/3333333333333333/fingerprint"
        configure_run = target / "debug/build/pkg/fedcba9876543210/fingerprint"
        main_run = target / "debug/build/pkg/abcdefabcdefabcd/fingerprint"
        for directory, marker in (
            (configure_compile, "build-script-build-script-configure"),
            (main_compile, "build-script-build-script-main"),
        ):
            directory.mkdir(parents=True)
            (directory / marker).write_text("0123456789abcdef")
        configure_run.mkdir(parents=True)
        main_run.mkdir(parents=True)
        classic_configure_exe = target / "debug/build/demo-aaaaaaaaaaaaaaaa/build-script-configure"
        classic_main_exe = target / "debug/build/demo-bbbbbbbbbbbbbbbb/build-script-main"
        configure_out = target / "debug/build/demo-1111111111111111/out"
        main_out = target / "debug/build/demo-2222222222222222/out"
        unit_configure_exe = configure_compile.parent / "out/build_script_configure"
        unit_main_exe = main_compile.parent / "out/build_script_main"
        unit_configure_out = configure_run.parent / "out"
        unit_main_out = main_run.parent / "out"
        output = io.StringIO()
        parser = Parser(self.workspace, target, output)
        parser.feed([
            f'1 chdir("{classic_configure_exe.parent}") = 0\n'.encode(),
            (
                '1 execve("build-script-configure", ["build-script-configure"], '
                f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={configure_out}"]) = 0\n'
            ).encode(),
            (
                f'1 execve("{classic_main_exe}", ["{classic_main_exe}"], '
                f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={main_out}"]) = 0\n'
            ).encode(),
            (
                f'1 execve("{unit_configure_exe}", ["{unit_configure_exe}"], '
                f'["CARGO_PKG_NAME=pkg", "CARGO_MANIFEST_DIR=/src/pkg", "OUT_DIR={unit_configure_out}"]) = 0\n'
            ).encode(),
            (
                f'1 execve("{unit_main_exe}", ["{unit_main_exe}"], '
                f'["CARGO_PKG_NAME=pkg", "CARGO_MANIFEST_DIR=/src/pkg", "OUT_DIR={unit_main_out}"]) = 0\n'
            ).encode(),
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        units = [event for event in events if event["event"] == "unit"]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual(
            [(event["kind"], event["unit"]) for event in units],
            [
                ("run", "debug/.fingerprint/demo-1111111111111111"),
                ("run", "debug/.fingerprint/demo-2222222222222222"),
                ("run", "debug/build/pkg/fedcba9876543210/fingerprint"),
                ("run", "debug/build/pkg/abcdefabcdefabcd/fingerprint"),
            ],
        )

    def test_trace_does_not_treat_a_bin_named_build_script_as_a_run_unit(self) -> None:
        target = self.layout.target
        run = target / "debug/build/demo/fedcba9876543210/fingerprint"
        run.mkdir(parents=True)
        (run / "run-build-script-build-script-build").write_text("fedcba9876543210")
        out = run.parent / "out"
        decoy = target / "debug/build/demo/4444444444444444/fingerprint"
        decoy.mkdir(parents=True)
        (decoy / "bin-build-script-build").write_text("4444444444444444")
        environment = f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={out}"]'
        cases = [
            "/tmp/build-script-build",
            str(target / "debug/build-script-build"),
            str(target / "debug/deps/build-script-build-0123456789abcdef"),
            str(out / "sub/build-script-build"),
            str(decoy.parent / "out/build_script_build"),
        ]
        for executable in cases:
            with self.subTest(executable=executable):
                output = io.StringIO()
                parser = Parser(self.workspace, target, output)
                parser.feed([(
                    f'1 execve("{executable}", ["{executable}"], {environment}) = 0\n'
                ).encode()])
                events = [json.loads(line) for line in output.getvalue().splitlines()]
                self.assertEqual(events[-1]["errors"], 0)
                self.assertFalse(any(event["event"] == "unit" for event in events))
        output = io.StringIO()
        parser = Parser(self.workspace, target, output)
        parser.feed([
            f'1 chdir("{target}") = 0\n'.encode(),
            (
                f'1 execve("../build-script-build", ["../build-script-build"], {environment}) = 0\n'
            ).encode(),
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertFalse(any(event["event"] == "unit" for event in events))

    def test_trace_rejects_a_build_script_whose_out_dir_is_outside_its_unit(self) -> None:
        target = self.layout.target
        classic_exe = target / "debug/build/demo-aaaaaaaaaaaaaaaa/build-script-configure"
        compile_dir = target / "debug/build/demo/0123456789abcdef/fingerprint"
        compile_dir.mkdir(parents=True)
        (compile_dir / "build-script-build-script-build").write_text("0123456789abcdef")
        unit_exe = compile_dir.parent / "out/build_script_build"
        for executable in (classic_exe, unit_exe):
            with self.subTest(executable=executable.name):
                output = io.StringIO()
                parser = Parser(self.workspace, target, output)
                parser.feed([(
                    f'1 execve("{executable}", ["{executable}"], '
                    '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR=/tmp/out"]) = 0\n'
                ).encode()])
                events = [json.loads(line) for line in output.getvalue().splitlines()]
                self.assertEqual(events[-1]["errors"], 1)
                self.assertEqual(events[1]["reason"], "Unknown build script unit layout")

    def test_trace_ignores_an_autocfg_probe_in_a_build_unit_out_dir(self) -> None:
        target = self.layout.target
        run = target / "debug/build/demo/fedcba9876543210/fingerprint"
        run.mkdir(parents=True)
        (run / "run-build-script-build-script-build").write_text("fedcba9876543210")
        out = run.parent / "out"
        environment = '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]'
        compile_dir = target / "debug/build/demo/0123456789abcdef/fingerprint"
        compile_dir.mkdir(parents=True)
        (compile_dir / "build-script-build-script-build").write_text("0123456789abcdef")
        script = compile_dir.parent / "out/build_script_build"
        output = io.StringIO()
        parser = Parser(self.workspace, target, output)
        parser.feed([
            (
                f'1 execve("{script}", ["{script}"], '
                f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={out}"]) = 0\n'
            ).encode(),
            b'1 clone(child_stack=NULL, flags=SIGCHLD) = 2\n',
            (
                f'2 execve("/usr/bin/rustc", ["rustc", "--crate-name", "autocfg_probe", "--crate-type=lib", '
                f'"--out-dir", "{out}", "--emit=llvm-ir", "-"], {environment}) = 0\n'
            ).encode(),
            (
                f'2 execve("/usr/bin/rustc", ["rustc", "--crate-name", "probe", "--crate-type=lib", '
                f'"-C", "extra-filename=-probe", "--out-dir", "{out}", "--emit=llvm-ir", "-"], {environment}) = 0\n'
            ).encode(),
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        units = [event for event in events if event["event"] == "unit"]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual([(event["kind"], event["unit"]) for event in units], [
            ("run", "debug/build/demo/fedcba9876543210/fingerprint"),
        ])
        output = io.StringIO()
        parser = Parser(self.workspace, target, output)
        parser.feed([
            (
                f'1 execve("{script}", ["{script}"], '
                f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={out}"]) = 0\n'
            ).encode(),
            b'1 clone(child_stack=NULL, flags=SIGCHLD) = 2\n',
            (
                f'2 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{out}", '
                f'"-C", "metadata=0123456789abcdef"], {environment}) = 0\n'
            ).encode(),
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 1)
        self.assertEqual(
            next(event["reason"] for event in events if event["event"] == "trace-error"),
            "Unknown rustc unit layout",
        )

    def test_unknown_build_entry_fails_closed(self) -> None:
        unknown = self.layout.target / "debug/build/demo/0123456789abcdef0123456789abcdef"
        unknown.mkdir(parents=True)
        with self.assertRaisesRegex(c.InputError, "Unknown build entry"):
            c.inventory(self.layout)

    def test_a_file_under_a_build_package_fails_closed(self) -> None:
        directory = self.layout.target / "debug/build/demo/0123456789abcdef/fingerprint"
        self.write_fingerprint(directory, "bin-demo", 1)
        (directory.parent.parent / "notes.txt").write_text("x")
        with self.assertRaisesRegex(c.InputError, "Unknown build entry"):
            c.inventory(self.layout)

    def test_a_build_directory_inside_out_dir_records_a_real_unit(self) -> None:
        nested = self.layout.target / "debug/build/demo/0123456789abcdef/out/build/pkg/aaaaaaaaaaaaaaaa/fingerprint"
        self.write_fingerprint(nested, "bin-demo", 1)
        weird = self.layout.target / "debug/build/demo/0123456789abcdef/out/build/not-a-unit"
        weird.mkdir(parents=True)
        (weird / "notes").write_text("x")
        key = nested.relative_to(self.layout.target).as_posix()
        self.assertEqual(set(c.inventory(self.layout)), {key})

    def test_a_fingerprint_file_inside_out_dir_fails_closed(self) -> None:
        fingerprint = self.layout.target / "debug/build/demo/0123456789abcdef/out/build/pkg/aaaaaaaaaaaaaaaa/fingerprint"
        fingerprint.parent.mkdir(parents=True)
        fingerprint.write_text("not a directory")
        with self.assertRaisesRegex(c.InputError, "not a directory"):
            c.inventory(self.layout)

    def test_an_unexecuted_build_unit_is_not_an_error(self) -> None:
        (self.layout.target / "debug/build/demo/0123456789abcdef/out").mkdir(parents=True)
        self.assertEqual(c.inventory(self.layout), {})

    def test_a_classic_build_directory_is_not_a_build_unit_package(self) -> None:
        classic = self.layout.target / "debug/build/demo-0123456789abcdef/out"
        classic.mkdir(parents=True)
        (classic / "generated.rs").write_text("fn generated() {}\n")
        self.assertEqual(c.inventory(self.layout), {})

    def test_restore_and_save_remove_unrecorded_artifacts_for_both_layouts(self) -> None:
        self.ensure_git()
        for shape in ("classic", "build-unit"):
            for mode in ("restore", "save"):
                with self.subTest(shape=shape, mode=mode):
                    self.assert_recorded_unit_stays_and_stale_artifacts_go(shape, mode)
        with self.subTest(shape="explicit-profile"):
            self.assert_explicit_profile_without_fingerprints_is_scrubbed()

    def assert_recorded_unit_stays_and_stale_artifacts_go(self, shape: str, mode: str) -> None:
        good = "aaaaaaaaaaaaaaaa"
        bad = "1111111111111111"
        stale = "2222222222222222"
        target = self.root / f"{mode}-{shape}"
        profile = target / "release"
        layout = c.Layout(self.workspace, target)
        if shape == "classic":
            good_dir = profile / f".fingerprint/app-{good}"
            bad_dir = profile / f".fingerprint/app-{bad}"
            good_artifact = profile / f"build/app-{good}/output"
            bad_artifact = profile / f"build/app-{bad}/out/stale"
            stale_build = profile / f"build/other-{stale}/out/lib.rlib"
            fingerprint_shape = c.FingerprintShape.CLASSIC
        else:
            good_dir = profile / f"build/app/{good}/fingerprint"
            bad_dir = profile / f"build/app/{bad}/fingerprint"
            good_artifact = profile / f"build/app/{good}/out/libapp.rlib"
            bad_artifact = profile / f"build/app/{bad}/out/stale"
            stale_build = profile / f"build/other/{stale}/out/libother.rlib"
            fingerprint_shape = c.FingerprintShape.BUILD_UNIT
        self.write_record(layout, good_dir, "bin-app", fingerprint_shape)
        self.write_fingerprint(bad_dir, "bin-bad", 9)
        stale_deps = profile / f"deps/libother-{stale}.rlib"
        good_deps = profile / f"deps/libapp-{good}.rlib"
        unhashed_deps = profile / "deps/unhashed.rlib"
        stale_example = profile / f"examples/demo-{stale}"
        good_example = profile / f"examples/demo-{good}"
        unhashed_example = profile / "examples/alias"
        uplifted = profile / "stale-binary"
        stray = profile / "build/stray.txt"
        unexecuted = profile / f"build/app/{'c' * 16}/out/probe"
        paths = [
            good_artifact, bad_artifact, stale_build, stale_deps, good_deps, unhashed_deps,
            stale_example, good_example, unhashed_example, uplifted,
        ]
        if shape == "build-unit":
            paths.extend((stray, unexecuted))
        for path in paths:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("data")
        if mode == "restore":
            stats = c.restore(layout, normalize=False)
            self.assertEqual(stats, {"retained": 1, "invalidated": 1})
        else:
            session = self.root / f"session-{shape}"
            c.begin(layout, session)
            marker_ns = json.loads((session / "session.json").read_text())["marker_ns"]
            os.utime(good_dir / "bin-app", ns=(marker_ns - 1_000_000, marker_ns - 1_000_000))
            stats = c.save(layout, session)
            self.assertEqual(stats["retained"], 1)
            self.assertEqual(stats["discarded"], 1)
            self.assertEqual(stats["recorded"], 0)
        self.assertTrue(good_dir.is_dir())
        self.assertTrue(good_artifact.is_file())
        self.assertTrue(good_deps.is_file())
        self.assertTrue(good_example.is_file())
        self.assertFalse(bad_dir.exists())
        self.assertFalse(bad_artifact.exists())
        self.assertFalse(stale_build.exists())
        self.assertFalse(stale_deps.exists())
        self.assertFalse(unhashed_deps.exists())
        self.assertFalse(stale_example.exists())
        self.assertFalse(unhashed_example.exists())
        self.assertFalse(uplifted.exists())
        if shape == "build-unit":
            self.assertFalse(stray.exists())
            self.assertFalse((profile / "build/app" / bad).exists())
            self.assertFalse(unexecuted.exists())

    def assert_explicit_profile_without_fingerprints_is_scrubbed(self) -> None:
        target = self.root / "explicit-target"
        profile = target / "custom"
        uplifted = profile / "stale-binary"
        stale_build = profile / "build/other/2222222222222222/out/file"
        for path in (uplifted, stale_build):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("stale")
        layout = c.Layout(self.workspace, target, (profile,))
        stats = c.restore(layout, normalize=False)
        self.assertEqual(stats, {"retained": 0, "invalidated": 0})
        self.assertFalse(uplifted.exists())
        self.assertFalse(stale_build.exists())

    def ensure_git(self) -> None:
        if (self.workspace / ".git").exists():
            return
        subprocess.run(["git", "init", "-q", "--initial-branch=main"], cwd=self.workspace, check=True)
        subprocess.run(["git", "config", "user.email", "cache@example.invalid"], cwd=self.workspace, check=True)
        subprocess.run(["git", "config", "user.name", "Cache Test"], cwd=self.workspace, check=True)

    def write_record(self, layout: c.Layout, directory: Path, stem: str, shape: c.FingerprintShape) -> None:
        self.write_fingerprint(directory, stem, 7)
        unit = c.read_unit(layout, directory, shape)
        data = {
            "version": c.VERSION,
            "unit": unit.key,
            "kind": unit.kind.value,
            "cargo": unit.fingerprint_digest(),
            "package": "workspace:.",
            "files": {},
            "directories": {},
            "env": {},
            "cargo_env": {},
            "dependencies": {},
        }
        c.atomic_json(directory / c.RECORD, {"data": data, "sha256": c.record_signature(data)})

    def test_trace_rejects_an_unknown_cargo_rustc_layout(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        parser.feed([(
            '1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "/tmp/other", '
            '"-C", "metadata=0123456789abcdef"], ["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]) = 0\n'
        ).encode()])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 1)
        self.assertEqual(events[1]["reason"], "Unknown rustc unit layout")

    def test_trace_ignores_rustc_without_cargo_metadata(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        parser.feed([(
            '1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "/tmp/other"], '
            '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]) = 0\n'
        ).encode()])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertFalse(any(event["event"] == "unit" for event in events))

    def write_fingerprint(self, directory: Path, stem: str, cargo_hash: int) -> None:
        directory.mkdir(parents=True, exist_ok=True)
        (directory / stem).write_text(cargo_hash.to_bytes(8, "little").hex())
        (directory / f"{stem}.json").write_text('{"deps":[]}')

    def test_a_package_name_that_ends_in_16_hex_characters_stays(self) -> None:
        target = self.layout.target
        fingerprint = target / "debug/build/demo-aaaaaaaaaaaaaaaa/bbbbbbbbbbbbbbbb/fingerprint"
        self.write_fingerprint(fingerprint, "bin-demo", 1)
        classic = target / "debug/build/demo-0123456789abcdef/out/generated.rs"
        classic.parent.mkdir(parents=True)
        classic.write_text("fn generated() {}\n")
        key = fingerprint.relative_to(target).as_posix()
        units = c.inventory(self.layout)
        self.assertEqual(set(units), {key})
        dependency = target / "debug/deps/libdemo-bbbbbbbbbbbbbbbb.rlib"
        dependency.parent.mkdir(parents=True)
        dependency.write_text("data")
        c.discard_unrecorded(self.layout, {key}, c.profile_directories(self.layout, units))
        self.assertTrue(fingerprint.is_dir())
        self.assertTrue(dependency.is_file())
        self.assertFalse(classic.exists())

    def test_a_profile_or_package_named_out_keeps_its_build_unit(self) -> None:
        target = self.layout.target
        named_profile = target / "out/build/demo/0123456789abcdef/fingerprint"
        named_package = target / "debug/build/out/fedcba9876543210/fingerprint"
        self.write_fingerprint(named_profile, "bin-demo", 1)
        self.write_fingerprint(named_package, "bin-demo", 2)
        keys = {path.relative_to(target).as_posix() for path in (named_profile, named_package)}
        self.assertEqual(set(c.inventory(self.layout)), keys)

    def test_a_classic_fingerprint_inside_out_dir_is_a_unit(self) -> None:
        fingerprint = self.layout.target / (
            "debug/build/demo/0123456789abcdef/out/nested-target/debug/.fingerprint/child-aaaaaaaaaaaaaaaa"
        )
        self.write_fingerprint(fingerprint, "bin-child", 1)
        key = fingerprint.relative_to(self.layout.target).as_posix()
        self.assertIn(key, c.inventory(self.layout))

    def test_cleanup_keeps_hashes_inside_one_profile(self) -> None:
        target = self.layout.target
        suffix = "a" * 16
        debug_fingerprint = target / f"debug/.fingerprint/app-{suffix}"
        self.write_fingerprint(debug_fingerprint, "bin-app", 1)
        (target / "release/.fingerprint").mkdir(parents=True)
        debug_dependency = target / f"debug/deps/libapp-{suffix}.rlib"
        debug_example = target / f"debug/examples/demo-{suffix}"
        release_dependency = target / f"release/deps/libapp-{suffix}.rlib"
        release_example = target / f"release/examples/demo-{suffix}"
        for path in (debug_dependency, debug_example, release_dependency, release_example):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("data")
        units = c.inventory(self.layout)
        key = debug_fingerprint.relative_to(target).as_posix()
        c.discard_unrecorded(self.layout, {key}, c.profile_directories(self.layout, units))
        self.assertTrue(debug_dependency.is_file())
        self.assertTrue(debug_example.is_file())
        self.assertFalse(release_dependency.exists())
        self.assertFalse(release_example.exists())

    def test_restore_skips_a_nested_profile_removed_with_its_outer_unit(self) -> None:
        target = self.layout.target
        outer = target / "release/build/demo/0123456789abcdef/fingerprint"
        nested = target / (
            "release/build/demo/0123456789abcdef/out/nested-target/debug/.fingerprint/child-aaaaaaaaaaaaaaaa"
        )
        self.write_fingerprint(outer, "run-build-script-build-script-build", 1)
        (outer / "run-build-script-build-script-build.json").write_text('{"deps":[],"local":[]}')
        self.write_fingerprint(nested, "bin-child", 2)
        stats = c.restore(self.layout, normalize=False)
        self.assertEqual(stats, {"retained": 0, "invalidated": 2})
        self.assertFalse(outer.exists())
        self.assertFalse(nested.exists())

    def test_trace_rejects_a_missing_or_wrong_kind_fingerprint(self) -> None:
        target = self.layout.target
        compile_dir = target / "debug/build/demo/0123456789abcdef/fingerprint"
        compile_dir.mkdir(parents=True)
        (compile_dir / "build-script-build-script-build").write_text("0123456789abcdef")
        run = target / "debug/build/demo/fedcba9876543210/fingerprint"
        run.mkdir(parents=True)
        (run / "bin-demo").write_text("fedcba9876543210")
        executable = compile_dir.parent / "out/build_script_build"
        out = run.parent / "out"
        script = (
            f'1 execve("{executable}", ["{executable}"], '
            f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={out}"]) = 0\n'
        )
        self.assert_trace_layout_error(script, "Unknown build script unit layout")
        (run / "bin-demo").unlink()
        run.rmdir()
        self.assert_trace_layout_error(script, "Unknown build script unit layout")
        classic_executable = target / "debug/build/demo-aaaaaaaaaaaaaaaa/build-script-build"
        classic_out = target / "debug/build/demo-0123456789abcdef/out"
        classic = target / "debug/.fingerprint/demo-0123456789abcdef"
        classic.mkdir(parents=True)
        (classic / "bin-demo").write_text("0123456789abcdef")
        classic_script = (
            f'1 execve("{classic_executable}", ["{classic_executable}"], '
            f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={classic_out}"]) = 0\n'
        )
        self.assert_trace_layout_error(classic_script, "Unknown build script unit layout")
        (classic / "bin-demo").unlink()
        classic.rmdir()
        self.assert_trace_layout_error(classic_script, "Unknown build script unit layout")
        missing_out = target / "debug/build/demo/abcdefabcdefabcd/out"
        self.assert_trace_layout_error(
            (
                f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{missing_out}", '
                '"-C", "metadata=abcdefabcdefabcd"], ["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]) = 0\n'
            ),
            "Unknown rustc unit layout",
        )
        deps = target / "debug/deps"
        self.assert_trace_layout_error(
            (
                f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{deps}", '
                '"-C", "extra-filename=-cccccccccccccccc", "-C", "metadata=cccccccccccccccc"], '
                '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]) = 0\n'
            ),
            "Unknown rustc unit layout",
        )
        wrong_compile = target / "debug/.fingerprint/demo-dddddddddddddddd"
        wrong_compile.mkdir(parents=True)
        (wrong_compile / "run-build-script-build").write_text("dddddddddddddddd")
        self.assert_trace_layout_error(
            (
                f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{deps}", '
                '"-C", "extra-filename=-dddddddddddddddd", "-C", "metadata=dddddddddddddddd"], '
                '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]) = 0\n'
            ),
            "Unknown rustc unit layout",
        )

    def assert_trace_layout_error(self, line: str, reason: str) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output)
        parser.feed([line.encode()])
        events = [json.loads(item) for item in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 1)
        self.assertEqual(events[1]["reason"], reason)
        self.assertFalse(any(event["event"] == "unit" for event in events))

    def test_an_undeclared_read_through_a_nested_unit_fails(self) -> None:
        self.ensure_git()
        (self.workspace / "Cargo.toml").write_text('[package]\nname = "child"\nversion = "0.1.0"\nedition = "2021"\n')
        (self.workspace / "src").mkdir()
        (self.workspace / "src/lib.rs").write_text("pub fn demo() {}\n")
        secret = self.workspace / "secret.txt"
        secret.write_text("secret\n")
        subprocess.run(["git", "add", "."], cwd=self.workspace, check=True)
        subprocess.run(["git", "commit", "-qm", "Add the nested unit fixture"], cwd=self.workspace, check=True)
        target = self.layout.target
        fingerprint = target / (
            "release/build/demo/0123456789abcdef/out/nested-target/debug/build/child/fedcba9876543210/fingerprint"
        )
        session = self.root / "session-nested"
        c.begin(self.layout, session)
        self.write_fingerprint(fingerprint, "lib-child", 4)
        marker_ns = json.loads((session / "session.json").read_text())["marker_ns"]
        os.utime(fingerprint / "lib-child", ns=(marker_ns + 1_000_000_000, marker_ns + 1_000_000_000))
        out = fingerprint.parent / "out"
        out.mkdir(parents=True)
        (out / "child-fedcba9876543210.d").write_text("out: src/lib.rs\n")
        (fingerprint / "dep-lib-child").write_bytes(bytes.fromhex("01000000ff010000000000000000"))
        key = fingerprint.relative_to(target).as_posix()
        self.assertIn(key, c.inventory(self.layout))
        with (session / "trace-nested.jsonl").open("w") as journal:
            parser = Parser(self.workspace, target, journal)
            parser.feed([
                (
                    f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "child", "--out-dir", "{out}", '
                    f'"-C", "extra-filename=-fedcba9876543210", "-C", "metadata=fedcba9876543210"], '
                    f'["CARGO_PKG_NAME=child", "CARGO_MANIFEST_DIR={self.workspace}"]) = 0\n'
                ).encode(),
                f'1 openat(AT_FDCWD, "{secret}", O_RDONLY) = 3<{secret}>\n'.encode(),
            ])
        with (session / "trace-nested.jsonl").open() as journal:
            events = [json.loads(line) for line in journal]
        units = [event for event in events if event["event"] == "unit"]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual([(event["kind"], event["unit"]) for event in units], [("compile", key)])
        with self.assertRaisesRegex(c.InputError, "Undeclared input.*secret.txt"):
            c.save(self.layout, session, scrub=False)

    def test_save_rejects_a_trace_unit_missing_from_inventory(self) -> None:
        self.ensure_git()
        (self.workspace / "Cargo.toml").write_text('[package]\nname = "demo"\nversion = "0.1.0"\nedition = "2021"\n')
        subprocess.run(["git", "add", "."], cwd=self.workspace, check=True)
        subprocess.run(["git", "commit", "-qm", "Add an empty package"], cwd=self.workspace, check=True)
        session = self.root / "session-missing"
        c.begin(self.layout, session)
        event = {"event": "unit", "unit": "debug/build/missing/aaaaaaaaaaaaaaaa/fingerprint", "kind": "compile",
                 "package": str(self.workspace), "cwd": str(self.workspace), "dep_info": None, "via_dx": False}
        (session / "trace-missing.jsonl").write_text(
            json.dumps(event) + "\n" + json.dumps({"event": "complete", "errors": 0}) + "\n")
        with self.assertRaisesRegex(c.InputError, "inventory did not"):
            c.save(self.layout, session, scrub=False)

    def test_save_rejects_a_run_descriptor_whose_fingerprint_is_compile(self) -> None:
        self.commit_demo_package()
        target = self.layout.target
        fingerprint = target / "debug/.fingerprint/demo-0123456789abcdef"
        fingerprint.mkdir(parents=True)
        executable = target / "debug/build/demo-aaaaaaaaaaaaaaaa/build-script-build"
        out = target / "debug/build/demo-0123456789abcdef/out"
        session = self.root / "session-run-to-compile"
        c.begin(self.layout, session)
        line = (
            f'1 execve("{executable}", ["{executable}"], '
            f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR={self.workspace}", "OUT_DIR={out}"]) = 0\n'
        )
        self.write_trace(session, "run-to-compile", line)
        marker = fingerprint / "bin-demo"
        marker.write_text("0123456789abcdef")
        (fingerprint / "bin-demo.json").write_text('{"deps":[]}')
        self.touch_after_marker(session, marker)
        key = fingerprint.relative_to(target).as_posix()
        self.assertEqual(c.inventory(self.layout)[key].kind, c.Kind.COMPILE)
        with self.assertRaisesRegex(
                c.InputError, r"^The trace kind run does not match the fingerprint kind compile:"):
            c.save(self.layout, session, scrub=False)
        self.assertFalse((fingerprint / c.RECORD).is_file())

    def test_save_rejects_a_compile_descriptor_whose_fingerprint_is_run(self) -> None:
        self.commit_demo_package()
        target = self.layout.target
        unit_id = "0123456789abcdef"
        fingerprint = target / f"debug/build/demo/{unit_id}/fingerprint"
        fingerprint.mkdir(parents=True)
        out = fingerprint.parent / "out"
        session = self.root / "session-compile-to-run"
        c.begin(self.layout, session)
        line = (
            f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{out}", '
            f'"-C", "metadata={unit_id}"], '
            f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR={self.workspace}"]) = 0\n'
        )
        self.write_trace(session, "compile-to-run", line)
        marker = fingerprint / "run-build-script-build-script-build"
        marker.write_text(unit_id)
        (fingerprint / f"{marker.name}.json").write_text('{"deps":[],"local":[]}')
        stdout = fingerprint.parent / "run/stdout"
        stdout.parent.mkdir(parents=True)
        stdout.write_text("cargo:rerun-if-changed=build.rs\n")
        self.touch_after_marker(session, marker)
        key = fingerprint.relative_to(target).as_posix()
        self.assertEqual(c.inventory(self.layout)[key].kind, c.Kind.RUN)
        with self.assertRaisesRegex(
                c.InputError, r"^The trace kind compile does not match the fingerprint kind run:"):
            c.save(self.layout, session, scrub=False)
        self.assertFalse((fingerprint / c.RECORD).is_file())

    def commit_demo_package(self) -> None:
        self.ensure_git()
        (self.workspace / "Cargo.toml").write_text('[package]\nname = "demo"\nversion = "0.1.0"\nedition = "2021"\n')
        (self.workspace / "src").mkdir()
        (self.workspace / "src/lib.rs").write_text("pub fn demo() {}\n")
        (self.workspace / "build.rs").write_text("fn main() {}\n")
        subprocess.run(["git", "add", "."], cwd=self.workspace, check=True)
        subprocess.run(["git", "commit", "-qm", "Add the demo package"], cwd=self.workspace, check=True)

    def write_trace(self, session: Path, name: str, line: str) -> None:
        with (session / f"trace-{name}.jsonl").open("w") as journal:
            parser = Parser(self.workspace, self.layout.target, journal)
            parser.feed([line.encode()])
        with (session / f"trace-{name}.jsonl").open() as journal:
            events = [json.loads(item) for item in journal]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual(sum(event["event"] == "unit" for event in events), 1)

    def touch_after_marker(self, session: Path, path: Path) -> None:
        marker_ns = json.loads((session / "session.json").read_text())["marker_ns"]
        os.utime(path, ns=(marker_ns + 2_000_000_000, marker_ns + 2_000_000_000))

    def test_save_rejects_a_classic_run_whose_path_package_differs(self) -> None:
        self.commit_demo_package()
        target = self.layout.target
        fingerprint = target / "debug/.fingerprint/foo-0123456789abcdef"
        fingerprint.mkdir(parents=True)
        output = target / "debug/build/foo-0123456789abcdef/output"
        output.parent.mkdir(parents=True)
        output.write_text("cargo:rerun-if-changed=build.rs\n")
        executable = target / "debug/build/foo-aaaaaaaaaaaaaaaa/build-script-build"
        out = target / "debug/build/foo-0123456789abcdef/out"
        session = self.root / "session-classic-package"
        c.begin(self.layout, session)
        line = (
            f'1 execve("{executable}", ["{executable}"], '
            f'["CARGO_PKG_NAME=other", "CARGO_MANIFEST_DIR={self.workspace}", "OUT_DIR={out}"]) = 0\n'
        )
        self.write_rejected_trace(session, "classic-package", line, "target package foo")
        self.finish_run_fingerprint(session, fingerprint, "0123456789abcdef")
        with self.assertRaisesRegex(c.InputError, "did not finish without errors"):
            c.save(self.layout, session, scrub=False)
        self.assertFalse((fingerprint / c.RECORD).is_file())

    def test_save_rejects_a_build_unit_run_whose_path_package_differs(self) -> None:
        self.commit_demo_package()
        target = self.layout.target
        compile_dir = target / "debug/build/foo/aaaaaaaaaaaaaaaa/fingerprint"
        compile_dir.mkdir(parents=True)
        (compile_dir / "build-script-build-script-build").write_text("aaaaaaaaaaaaaaaa")
        fingerprint = target / "debug/build/foo/0123456789abcdef/fingerprint"
        fingerprint.mkdir(parents=True)
        stdout = fingerprint.parent / "run/stdout"
        stdout.parent.mkdir(parents=True)
        stdout.write_text("cargo:rerun-if-changed=build.rs\n")
        executable = compile_dir.parent / "out/build_script_build"
        out = fingerprint.parent / "out"
        session = self.root / "session-build-unit-package"
        c.begin(self.layout, session)
        line = (
            f'1 execve("{executable}", ["{executable}"], '
            f'["CARGO_PKG_NAME=other", "CARGO_MANIFEST_DIR={self.workspace}", "OUT_DIR={out}"]) = 0\n'
        )
        self.write_rejected_trace(session, "build-unit-package", line, "target package foo")
        self.finish_run_fingerprint(session, fingerprint, "0123456789abcdef")
        with self.assertRaisesRegex(c.InputError, "did not finish without errors"):
            c.save(self.layout, session, scrub=False)
        self.assertFalse((fingerprint / c.RECORD).is_file())

    def test_save_rejects_a_compile_unit_whose_path_package_differs(self) -> None:
        self.commit_demo_package()
        target = self.layout.target
        cases = {
            "build-unit": (
                target / "debug/build/foo/0123456789abcdef/fingerprint",
                target / "debug/build/foo/0123456789abcdef/out",
                '"-C", "metadata=aaaaaaaaaaaaaaaa"',
            ),
            "classic": (
                target / "debug/.fingerprint/other-0123456789abcdef",
                target / "debug/build/foo-aaaaaaaaaaaaaaaa",
                '"-C", "extra-filename=-0123456789abcdef", "-C", "metadata=0123456789abcdef"',
            ),
        }
        for name, (fingerprint, out, flags) in cases.items():
            with self.subTest(name=name):
                fingerprint.mkdir(parents=True, exist_ok=True)
                session = self.root / f"session-compile-package-{name}"
                c.begin(self.layout, session)
                line = (
                    f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{out}", '
                    f'{flags}], ["CARGO_PKG_NAME=other", "CARGO_MANIFEST_DIR={self.workspace}"]) = 0\n'
                )
                self.write_rejected_trace(session, name, line, "target package foo")
                marker = fingerprint / "bin-demo"
                marker.write_text("0123456789abcdef")
                (fingerprint / "bin-demo.json").write_text('{"deps":[]}')
                (fingerprint / "dep-bin-demo").write_bytes(bytes.fromhex("01000000ff010000000000000000"))
                (out / "demo-0123456789abcdef.d").parent.mkdir(parents=True, exist_ok=True)
                (out / "demo.d").write_text("out: src/lib.rs\n")
                (out / "demo-0123456789abcdef.d").write_text("out: src/lib.rs\n")
                self.touch_after_marker(session, marker)
                with self.assertRaisesRegex(c.InputError, "did not finish without errors"):
                    c.save(self.layout, session, scrub=False)
                self.assertFalse((fingerprint / c.RECORD).is_file())

    def test_save_rejects_a_manifest_name_that_differs_from_the_unit_path(self) -> None:
        self.commit_demo_package()
        target = self.layout.target
        classic = target / "debug/.fingerprint/foo-0123456789abcdef"
        build_unit = target / "debug/build/foo/0123456789abcdef/fingerprint"
        for name, fingerprint in (("classic", classic), ("build-unit", build_unit)):
            with self.subTest(name=name):
                fingerprint.mkdir(parents=True)
                if name == "classic":
                    output = target / "debug/build/foo-0123456789abcdef/output"
                    executable = target / "debug/build/foo-aaaaaaaaaaaaaaaa/build-script-build"
                    out = target / "debug/build/foo-0123456789abcdef/out"
                else:
                    compile_dir = target / "debug/build/foo/aaaaaaaaaaaaaaaa/fingerprint"
                    self.write_fingerprint(compile_dir, "build-script-build-script-build", 1)
                    output = fingerprint.parent / "run/stdout"
                    executable = compile_dir.parent / "out/build_script_build"
                    out = fingerprint.parent / "out"
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_text("cargo:rerun-if-changed=build.rs\n")
                session = self.root / f"session-manifest-{name}"
                c.begin(self.layout, session)
                line = (
                    f'1 execve("{executable}", ["{executable}"], '
                    f'["CARGO_PKG_NAME=foo", "CARGO_MANIFEST_DIR={self.workspace}", "OUT_DIR={out}"]) = 0\n'
                )
                self.write_trace(session, name, line)
                self.finish_run_fingerprint(session, fingerprint, "0123456789abcdef")
                with self.assertRaisesRegex(
                        c.InputError, r"^The manifest package demo does not match the target package foo:"):
                    c.save(self.layout, session, scrub=False)
                self.assertFalse((fingerprint / c.RECORD).is_file())

    def finish_run_fingerprint(self, session: Path, fingerprint: Path, unit_hash: str) -> None:
        marker = fingerprint / "run-build-script-build-script-build"
        marker.write_text(unit_hash)
        (fingerprint / f"{marker.name}.json").write_text('{"deps":[],"local":[]}')
        self.touch_after_marker(session, marker)

    def write_rejected_trace(self, session: Path, name: str, line: str, reason: str) -> None:
        with (session / f"trace-{name}.jsonl").open("w") as journal:
            parser = Parser(self.workspace, self.layout.target, journal)
            parser.feed([line.encode()])
        with (session / f"trace-{name}.jsonl").open() as journal:
            events = [json.loads(item) for item in journal]
        self.assertGreater(events[-1]["errors"], 0)
        self.assertFalse(any(event["event"] == "unit" for event in events))
        self.assertIn(reason, next(event["reason"] for event in events if event["event"] == "trace-error"))


if __name__ == "__main__":
    unittest.main()
