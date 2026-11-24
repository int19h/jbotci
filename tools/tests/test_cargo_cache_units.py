"""Exercise damaged records, directory membership, and declaration checks."""
from __future__ import annotations

from contextlib import redirect_stderr
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import sys
import unittest
from unittest.mock import patch

from tools import cargo_inputs as c
from tools.cargo_inputs_trace import (
    UNSUPPORTED_COMPILER, Parser, Process, TraceError,
    accept_pinned_compiler, dioxus_cli_version, dx_version_line_matches, pinned_dx,
    process_snapshot, query_rustup_rustc, rustup_binary, string_at, supported_compiler, trace,
    shared_rustup_home, version_line_matches,
)
from tools.release_compiler import toolchain_channel


FIXTURE_COMPILER = Path("/usr/bin/rustc")


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
        first = c.Unit("debug/child", self.layout.target / "debug/build/child/aaaaaaaaaaaaaaaa/fingerprint", "lib-child", c.Kind.COMPILE, (), {}, 42)
        second = c.Unit("release/child", self.layout.target / "release/build/child/bbbbbbbbbbbbbbbb/fingerprint", "lib-child", c.Kind.COMPILE, (), {}, 42)
        parent = c.Unit("release/parent", self.layout.target / "release/build/parent/cccccccccccccccc/fingerprint", "lib-parent", c.Kind.COMPILE,
                        (42,), {}, 43)
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
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
        parser.feed([b'1 execve("/bin/true", ["tr"...], ["OUT_DIR=/safe"]) = 0\n'])
        self.assertEqual(json.loads(output.getvalue().splitlines()[-1])["errors"], 1)

    def test_trace_does_not_store_credentials(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
        parser.feed([b'1 execve("/bin/true", ["true", "secret-argument"], ["GH_TOKEN=secret-token"]) = 0\n'])
        self.assertNotIn("secret", output.getvalue())
        escaped = self.layout.target / ".." / "secret-outside" / "tool"
        self.assertIsNone(parser.executable_in_target(
            f'1 execve("{escaped}", ["tool"], ["GH_TOKEN=secret-token"]) = 0\n'.encode()))
        folded = self.layout.target / "debug" / ".." / "debug" / "build_script_build"
        self.assertEqual(parser.executable_in_target(
            f'1 execve("{folded}", ["build_script_build"], []) = 0\n'.encode()),
            "debug/build_script_build")

    def test_trace_errors_report_only_sanitized_events(self) -> None:
        output = io.StringIO()
        diagnostic = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
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
            parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
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
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
        parser.feed([
            b'1 clone(child_stack=NULL, flags=SIGCHLD <unfinished ...>\n',
            b'2 execve("/bin/true", ["true"], []) = 0\n',
            b'1 <... clone resumed>) = 2\n',
            b'2 openat(AT_FDCWD, "/source/data", O_RDONLY) = 3</source/data>\n',
        ])
        self.assertEqual(json.loads(output.getvalue().splitlines()[-1])["errors"], 0)

    def test_explicit_thread_exit_closes_strace_unknown_placeholder(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
        parser.feed([
            b'1 clone3({flags=CLONE_THREAD}, 88) = 2\n',
            b'2 ???( <unfinished ...>\n',
            b'2 +++ exited with 0 +++\n',
            b'1 +++ exited with 0 +++\n',
        ])
        self.assertEqual(json.loads(output.getvalue().splitlines()[-1])["errors"], 0)

    def test_unknown_placeholder_without_exit_still_fails_closed(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
        parser.feed([b'1 ???( <unfinished ...>\n'])
        self.assertEqual(json.loads(output.getvalue().splitlines()[-1])["errors"], 1)

    def test_killed_process_cannot_finish_a_pending_file_read(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
        parser.feed([
            b'1 openat(AT_FDCWD, "pipe", O_RDONLY <unfinished ...>\n',
            b'1 +++ killed by SIGKILL +++\n',
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events, [{"event": "complete", "errors": 0}])

    def test_group_exit_before_clone_result_closes_only_teardown_events(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
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
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER,
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
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER,
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
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
        with redirect_stderr(io.StringIO()):
            parser.feed([b'1 chdir("/source") = 0\n', b'2 ???( <unfinished ...>\n'])
        self.assertEqual(json.loads(output.getvalue().splitlines()[-1])["errors"], 1)

    def test_unsplit_teardown_and_exit_only_have_no_input_operations(self) -> None:
        for teardown in ([], [b'2 ???() = ?\n']):
            with self.subTest(teardown=teardown):
                output = io.StringIO()
                parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
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
                parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
                with redirect_stderr(io.StringIO()):
                    parser.feed([b'1 chdir("/source") = 0\n', operation,
                                 b'2 +++ exited with 0 +++\n'])
                self.assertEqual(json.loads(output.getvalue().splitlines()[-1])["errors"], 1)

    def test_process_identity_inherits_parent_without_retaining_arguments(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
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
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
        with redirect_stderr(diagnostic):
            parser.feed([b'1 <... execve resumed> ["secret-argument"] <unfinished ...>\n'])
        self.assertNotIn("secret", diagnostic.getvalue())
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[0]["pending"]["1"]["operation"], "execve")

    def test_inventory_reads_a_build_unit_fingerprint(self) -> None:
        build_unit = self.layout.target / "opt/build/demo/fedcba9876543210/fingerprint"
        self.write_fingerprint(build_unit, "run-build-script-build-script-build", 42)
        units = c.inventory(self.layout)
        build_key = build_unit.relative_to(self.layout.target).as_posix()
        self.assertEqual(set(units), {build_key})
        self.assertEqual(units[build_key].kind, c.Kind.RUN)
        self.assertEqual(units[build_key].cargo_hash, 42)
        self.assertEqual(units[build_key].suffix, "fedcba9876543210")
        self.assertEqual(units[build_key].profile, self.layout.target / "opt")

    def test_two_metadata_files_fail_closed(self) -> None:
        directory = self.layout.target / "debug/build/demo/0123456789abcdef/fingerprint"
        self.write_fingerprint(directory, "bin-demo", 7)
        (directory / "bin-demo").write_bytes(b"")
        (directory / "other.json").write_text('{"deps":[]}')
        with self.assertRaisesRegex(c.InputError, "one metadata file"):
            c.inventory(self.layout)

    def test_a_malformed_non_empty_fingerprint_hash_fails_closed(self) -> None:
        directory = self.layout.target / "debug/build/demo/0123456789abcdef/fingerprint"
        directory.mkdir(parents=True)
        (directory / "bin-demo.json").write_text('{"deps":[]}')
        for value in ("2a", "\n", "x" * 16):
            with self.subTest(value=value):
                (directory / "bin-demo").write_text(value)
                with self.assertRaisesRegex(c.InputError, "16 hex characters"):
                    c.inventory(self.layout)

    def test_restore_removes_an_empty_cargo_hash_without_metadata(self) -> None:
        directory = self.layout.target / "debug/build/demo/0123456789abcdef/fingerprint"
        directory.mkdir(parents=True)
        marker = directory / "bin-demo"
        marker.write_bytes(b"")
        self.assertEqual(c.inventory(self.layout), {})
        self.assertEqual(c.restore(self.layout, normalize=False), {"retained": 0, "invalidated": 0})
        self.assertFalse(directory.parent.exists())

    def test_metadata_validation_is_the_same_for_both_commitment_states(self) -> None:
        directory = self.layout.target / "debug/build/demo/0123456789abcdef/fingerprint"
        directory.mkdir(parents=True)
        cases = [
            ("bin-demo", "{}", "no dependency list"),
            ("bin-demo", "not JSON", "unreadable"),
            ("bin-demo", '{"deps":[null]}', "invalid dependency entry"),
            ("bin-demo", '{"deps":[[0,"child",false]]}', "invalid dependency entry"),
            ("bin-demo", '{"deps":[[false,"child",false,5]]}', "invalid dependency entry"),
            ("bin-demo", '{"deps":[[0,2,false,5]]}', "invalid dependency entry"),
            ("bin-demo", '{"deps":[[0,"child",0,5]]}', "invalid dependency entry"),
            ("bin-demo", '{"deps":[[0,"child",false,true]]}', "invalid dependency entry"),
            ("bin-demo", '{"deps":[[0,"child",false,-1]]}', "invalid dependency entry"),
            ("bin-demo", '{"deps":[[0,"child",false,18446744073709551616]]}', "invalid dependency entry"),
            ("format-k2", '{"deps":[]}', "Unknown fingerprint entry"),
            ("run-build-script-demo", '{"deps":[]}', "invalid local inputs"),
            ("run-build-script-demo", '{"deps":[],"local":[null]}', "invalid local inputs"),
            ("run-build-script-demo", '{"deps":[],"local":[{"RerunIfEnvChanged":{"var":"ENV"}}]}', "invalid local inputs"),
            ("run-build-script-demo", '{"deps":[],"local":[{"RerunIfEnvChanged":{"var":8,"val":null}}]}', "invalid local inputs"),
            ("run-build-script-demo", '{"deps":[],"local":[{"RerunIfEnvChanged":{"var":"ENV","val":8}}]}', "invalid local inputs"),
        ]
        for stem, value, error in cases:
            marker = directory / stem
            metadata = directory / (stem + ".json")
            metadata.write_text(value)
            for hash_text in ("", "1111111111111111"):
                with self.subTest(stem=stem, value=value, committed=bool(hash_text)):
                    marker.write_text(hash_text)
                    with self.assertRaisesRegex(c.InputError, error):
                        c.inventory(self.layout)
            marker.unlink()
            metadata.unlink()

    def test_a_second_marker_is_an_error_even_with_valid_metadata(self) -> None:
        directory = self.layout.target / "debug/build/demo/0123456789abcdef/fingerprint"
        self.write_fingerprint(directory, "bin-demo", 1)
        (directory / "bin-stray").write_bytes(b"")
        with self.assertRaisesRegex(c.InputError, "Unknown fingerprint entry"):
            c.inventory(self.layout)

    def test_a_fingerprint_file_in_place_of_a_directory_fails_closed(self) -> None:
        unit = self.layout.target / "debug/build/demo/0123456789abcdef"
        unit.mkdir(parents=True)
        (unit / "fingerprint").write_text("not a directory")
        with self.assertRaisesRegex(c.InputError, "not a directory"):
            c.inventory(self.layout)

    def test_build_script_output_uses_the_run_stdout(self) -> None:
        build_dir = self.layout.target / "opt/build/demo/fedcba9876543210/fingerprint"
        build_dir.mkdir(parents=True)
        build_output = build_dir.parent / "run/stdout"
        build_output.parent.mkdir(parents=True)
        build_output.write_text("cargo:rerun-if-changed=version.txt\n")
        build_unit = c.Unit(
            "build-unit", build_dir, "run-build-script-build-script-build",
            c.Kind.RUN, (), {}, 1,
        )
        self.assertEqual(c.build_script_output(build_unit), build_output)
        self.assertEqual(build_unit.profile, self.layout.target / "opt")

    def test_encoded_dep_info_matches_cargo_bytes(self) -> None:
        # Cargo 1.100.0 writes these same bytes for src/main.rs.
        path = self.root / "dep-bin"
        path.write_bytes(bytes.fromhex(
            "01000000ff0101000000000b0000007372632f6d61696e2e72730000000000"))
        self.assertEqual(c.encoded_environment(path), {})

    def test_trace_binds_compile_and_run_units(self) -> None:
        target = self.layout.target
        plain = target / "debug/build/demo/fedcba9876543210/fingerprint"
        plain.mkdir(parents=True)
        hashed = target / "debug/build/demo/0123456789abcdef/fingerprint"
        hashed.mkdir(parents=True)
        # The compile unit that produced the build script, not the run unit.
        (hashed / "build-script-build-script-build").write_text("0123456789abcdef")
        output = io.StringIO()
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
        environment = '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]'
        plain_out = target / "debug/build/demo/fedcba9876543210/out"
        hashed_out = target / "debug/build/demo/0123456789abcdef/out"
        parser.feed([
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
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        units = [event for event in events if event["event"] == "unit"]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual(
            [(event["kind"], event["unit"], event["dep_info"]) for event in units],
            [
                ("compile", "debug/build/demo/fedcba9876543210/fingerprint",
                 str(plain_out / "demo.d")),
                ("compile", "debug/build/demo/0123456789abcdef/fingerprint",
                 str(hashed_out / "demo-0123456789abcdef.d")),
                ("run", "debug/build/demo/fedcba9876543210/fingerprint", None),
            ],
        )

    def test_trace_matches_custom_build_scripts_by_place(self) -> None:
        target = self.layout.target
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
        unit_configure_exe = configure_compile.parent / "out/build_script_configure"
        unit_main_exe = main_compile.parent / "out/build_script_main"
        unit_configure_out = configure_run.parent / "out"
        unit_main_out = main_run.parent / "out"
        output = io.StringIO()
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
        parser.feed([
            f'1 chdir("{unit_configure_exe.parent}") = 0\n'.encode(),
            (
                '1 execve("build_script_configure", ["build_script_configure"], '
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
        environment = f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={out}"]'
        cases = [
            "/tmp/build-script-build",
            str(target / "debug/build-script-build"),
            str(target / "debug/deps/build-script-build-0123456789abcdef"),
        ]
        for executable in cases:
            with self.subTest(executable=executable):
                output = io.StringIO()
                parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
                parser.feed([(
                    f'1 execve("{executable}", ["{executable}"], {environment}) = 0\n'
                ).encode()])
                events = [json.loads(line) for line in output.getvalue().splitlines()]
                self.assertEqual(events[-1]["errors"], 0)
                self.assertFalse(any(event["event"] == "unit" for event in events))
        output = io.StringIO()
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
        parser.feed([
            f'1 chdir("{target}") = 0\n'.encode(),
            (
                f'1 execve("../build-script-build", ["../build-script-build"], {environment}) = 0\n'
            ).encode(),
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertFalse(any(event["event"] == "unit" for event in events))
        test_binary = target / "release/build/bityzba/fc33c4b6d4589c22/out/bityzba-fc33c4b6d4589c22"
        output = io.StringIO()
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
        parser.feed([(
            f'1 execve("{test_binary}", ["{test_binary}"], {environment}) = 0\n'
        ).encode()])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertFalse(any(event["event"] == "unit" for event in events))
        inside = out / "sub/build-script-build"
        self.assert_trace_layout_error(
            f'1 execve("{inside}", ["{inside}"], {environment}) = 0\n',
            "Unknown build script unit layout",
        )

    def test_trace_rejects_a_build_script_candidate_without_its_environment(self) -> None:
        target = self.layout.target
        compile_dir = target / "debug/build/demo/0123456789abcdef/fingerprint"
        compile_dir.mkdir(parents=True)
        (compile_dir / "build-script-build-script-build").write_text("0123456789abcdef")
        unit_executable = compile_dir.parent / "out/build_script_build"
        unit_out = target / "debug/build/demo/fedcba9876543210/out"
        (unit_out.parent / "fingerprint").mkdir(parents=True)
        for name in ("CARGO_PKG_NAME", "CARGO_MANIFEST_DIR", "OUT_DIR"):
            with self.subTest(omitted=name):
                fields = {
                    "CARGO_PKG_NAME": "demo",
                    "CARGO_MANIFEST_DIR": "/src/demo",
                    "OUT_DIR": str(unit_out),
                }
                del fields[name]
                environment = "[" + ", ".join(f'"{key}={value}"' for key, value in fields.items()) + "]"
                self.assert_trace_layout_error(
                    f'1 execve("{unit_executable}", ["{unit_executable}"], {environment}) = 0\n',
                    "Unknown build script unit layout",
                )

    def test_trace_rejects_a_malformed_id_under_a_profile_build_directory(self) -> None:
        target = self.layout.target
        out = target / "debug/build/demo/fedcba9876543210/out"
        (out.parent / "fingerprint").mkdir(parents=True)
        executable = target / "debug/build/demo/nothex/out/build_script_build"
        self.assert_trace_layout_error(
            f'1 execve("{executable}", ["{executable}"], '
            f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={out}"]) = 0\n',
            "Unknown build script unit layout",
        )

    def test_trace_rejects_a_rustc_candidate_without_package_or_manifest(self) -> None:
        target = self.layout.target
        fingerprint = target / "debug/build/demo/0123456789abcdef/fingerprint"
        fingerprint.mkdir(parents=True)
        out = fingerprint.parent / "out"
        arguments = (
            f'"--out-dir", "{out}", "-C", "extra-filename=-0123456789abcdef", '
            '"-C", "metadata=0123456789abcdef"'
        )
        environments = (
            '["CARGO_MANIFEST_DIR=/src/demo"]',
            '["CARGO_PKG_NAME=demo"]',
        )
        for environment in environments:
            with self.subTest(environment=environment):
                self.assert_trace_layout_error(
                    (
                        '1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", '
                        f'{arguments}], {environment}) = 0\n'
                    ),
                    "Unknown rustc unit layout",
                )

    def test_trace_rejects_a_build_script_whose_out_dir_is_outside_its_unit(self) -> None:
        target = self.layout.target
        compile_dir = target / "debug/build/demo/0123456789abcdef/fingerprint"
        compile_dir.mkdir(parents=True)
        (compile_dir / "build-script-build-script-build").write_text("0123456789abcdef")
        executable = compile_dir.parent / "out/build_script_build"
        output = io.StringIO()
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
        parser.feed([(
            f'1 execve("{executable}", ["{executable}"], '
            '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR=/tmp/out"]) = 0\n'
        ).encode()])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 1)
        self.assertEqual(events[1]["reason"], "Unknown build script unit layout")

    def test_trace_matches_a_build_script_for_an_explicit_target(self) -> None:
        target = self.layout.target
        triple = "aarch64-unknown-linux-gnu"
        compile_dir = target / "debug/build/demo/0123456789abcdef/fingerprint"
        compile_dir.mkdir(parents=True)
        (compile_dir / "build-script-build-script-build").write_text("0123456789abcdef")
        run = target / f"{triple}/debug/build/demo/fedcba9876543210/fingerprint"
        run.mkdir(parents=True)
        nested_compile = target / "tests/trybuild/debug/build/demo/1111111111111111/fingerprint"
        nested_compile.mkdir(parents=True)
        (nested_compile / "build-script-build-script-build").write_text("1111111111111111")
        nested_run = target / f"tests/trybuild/{triple}/debug/build/demo/2222222222222222/fingerprint"
        nested_run.mkdir(parents=True)
        cases = (
            (compile_dir.parent / "out/build_script_build", run.parent / "out",
             f"{triple}/debug/build/demo/fedcba9876543210/fingerprint"),
            (nested_compile.parent / "out/build_script_build", nested_run.parent / "out",
             f"tests/trybuild/{triple}/debug/build/demo/2222222222222222/fingerprint"),
        )
        for executable, out, expected in cases:
            with self.subTest(expected=expected):
                output = io.StringIO()
                parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
                parser.feed([(
                    f'1 execve("{executable}", ["{executable}"], '
                    f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", '
                    f'"TARGET={triple}", "OUT_DIR={out}"]) = 0\n'
                ).encode()])
                events = [json.loads(line) for line in output.getvalue().splitlines()]
                units = [event for event in events if event["event"] == "unit"]
                self.assertEqual(events[-1]["errors"], 0)
                self.assertEqual([(event["kind"], event["unit"]) for event in units], [
                    ("run", expected),
                ])

    def test_trace_rejects_a_build_script_for_a_different_target(self) -> None:
        target = self.layout.target
        requested = "x86_64-unknown-linux-gnu"
        placed = "aarch64-unknown-linux-gnu"
        compile_dir = target / "debug/build/demo/0123456789abcdef/fingerprint"
        compile_dir.mkdir(parents=True)
        (compile_dir / "build-script-build-script-build").write_text("0123456789abcdef")
        unit_out = target / f"{placed}/debug/build/demo/fedcba9876543210/out"
        (unit_out.parent / "fingerprint").mkdir(parents=True)
        executable = compile_dir.parent / "out/build_script_build"
        for target_value in (requested, None):
            with self.subTest(target_value=target_value):
                fields = ["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", f"OUT_DIR={unit_out}"]
                if target_value is not None:
                    fields.append(f"TARGET={target_value}")
                environment = "[" + ", ".join(f'"{field}"' for field in fields) + "]"
                output = io.StringIO()
                parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
                parser.feed([(
                    f'1 execve("{executable}", ["{executable}"], {environment}) = 0\n'
                ).encode()])
                events = [json.loads(line) for line in output.getvalue().splitlines()]
                self.assertEqual(events[-1]["errors"], 1)
                self.assertEqual(events[1]["reason"], "Unknown build script unit layout")
                self.assertFalse(any(event["event"] == "unit" for event in events))

    def test_trace_matches_a_short_target_name(self) -> None:
        target = self.layout.target
        triple = "wasm32-wasi"
        compile_dir = target / "debug/build/demo/0123456789abcdef/fingerprint"
        compile_dir.mkdir(parents=True)
        (compile_dir / "build-script-build-script-build").write_text("0123456789abcdef")
        unit_run = target / f"{triple}/debug/build/demo/fedcba9876543210/fingerprint"
        unit_run.mkdir(parents=True)
        executable = compile_dir.parent / "out/build_script_build"
        out = unit_run.parent / "out"
        expected = unit_run.relative_to(target).as_posix()
        output = io.StringIO()
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
        parser.feed([(
            f'1 execve("{executable}", ["{executable}"], '
            f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", '
            f'"TARGET={triple}", "OUT_DIR={out}"]) = 0\n'
        ).encode()])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        units = [event for event in events if event["event"] == "unit"]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual([(event["kind"], event["unit"]) for event in units], [
            ("run", expected),
        ])

    def test_trace_rejects_a_build_unit_script_without_its_compile_marker(self) -> None:
        target = self.layout.target
        executable = target / "debug/build/demo/0123456789abcdef/out/build_script_build"
        out = target / "debug/build/demo/fedcba9876543210/out"
        (out.parent / "fingerprint").mkdir(parents=True)
        script = (
            f'1 execve("{executable}", ["{executable}"], '
            f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={out}"]) = 0\n'
        )
        self.assert_trace_layout_error(script, "Unknown build script unit layout")
        compile_dir = executable.parent.parent / "fingerprint"
        compile_dir.mkdir(parents=True)
        (compile_dir / "bin-demo").write_text("0123456789abcdef")
        self.assert_trace_layout_error(script, "Unknown build script unit layout")

    def test_trace_rejects_a_build_script_whose_profiles_do_not_match(self) -> None:
        target = self.layout.target
        compile_dir = target / "debug/build/demo/0123456789abcdef/fingerprint"
        compile_dir.mkdir(parents=True)
        (compile_dir / "build-script-build-script-build").write_text("0123456789abcdef")
        executable = compile_dir.parent / "out/build_script_build"
        nested_out = target / "tests/trybuild/aarch64-unknown-linux-gnu/debug/build/demo/fedcba9876543210/out"
        other_out = target / "aarch64-unknown-linux-gnu/debug/build/other/fedcba9876543210/out"
        release_out = target / "release/build/demo/fedcba9876543210/out"
        short_out = target / "desktop-dev/debug/build/demo/fedcba9876543210/out"
        for out in (nested_out, other_out, release_out, short_out):
            with self.subTest(out=out.relative_to(target).as_posix()):
                output = io.StringIO()
                parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
                parser.feed([(
                    f'1 execve("{executable}", ["{executable}"], '
                    f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={out}"]) = 0\n'
                ).encode()])
                events = [json.loads(line) for line in output.getvalue().splitlines()]
                self.assertEqual(events[-1]["errors"], 1)
                self.assertEqual(events[1]["reason"], "Unknown build script unit layout")
                self.assertEqual(
                    events[1]["path"],
                    "debug/build/demo/0123456789abcdef/out/build_script_build",
                )
                self.assertFalse(any(event["event"] == "unit" for event in events))

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
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
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
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
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

    def test_a_build_directory_inside_out_dir_is_not_a_unit(self) -> None:
        nested = self.layout.target / "debug/build/demo/0123456789abcdef/out/build/pkg/aaaaaaaaaaaaaaaa/fingerprint"
        self.write_fingerprint(nested, "bin-demo", 1)
        weird = self.layout.target / "debug/build/demo/0123456789abcdef/out/build/not-a-unit"
        weird.mkdir(parents=True)
        (weird / "notes").write_text("x")
        self.assertEqual(c.inventory(self.layout), {})
        self.assertTrue((weird / "notes").is_file())

    def test_a_fingerprint_file_inside_out_dir_is_output(self) -> None:
        fingerprint = self.layout.target / "debug/build/demo/0123456789abcdef/out/build/pkg/aaaaaaaaaaaaaaaa/fingerprint"
        fingerprint.parent.mkdir(parents=True)
        fingerprint.write_text("not a directory")
        self.assertEqual(c.inventory(self.layout), {})
        self.assertTrue(fingerprint.is_file())

    def test_an_unexecuted_build_unit_is_not_an_error(self) -> None:
        (self.layout.target / "debug/build/demo/0123456789abcdef/out").mkdir(parents=True)
        self.assertEqual(c.inventory(self.layout), {})

    def test_restore_and_save_remove_unrecorded_artifacts(self) -> None:
        self.ensure_git()
        for mode in ("restore", "save"):
            with self.subTest(mode=mode):
                self.assert_recorded_unit_stays_and_stale_artifacts_go(mode)
        self.assert_explicit_profile_without_fingerprints_is_scrubbed()

    def assert_recorded_unit_stays_and_stale_artifacts_go(self, mode: str) -> None:
        good = "aaaaaaaaaaaaaaaa"
        bad = "1111111111111111"
        stale = "2222222222222222"
        target = self.root / mode
        profile = target / "release"
        layout = c.Layout(self.workspace, target)
        good_dir = profile / f"build/app/{good}/fingerprint"
        bad_dir = profile / f"build/app/{bad}/fingerprint"
        good_artifact = profile / f"build/app/{good}/out/libapp.rlib"
        bad_artifact = profile / f"build/app/{bad}/out/stale"
        stale_build = profile / f"build/other/{stale}/out/libother.rlib"
        self.write_record(layout, good_dir, "bin-app")
        self.write_fingerprint(bad_dir, f"example-demo-{good}", 9)
        stale_deps = profile / f"deps/libother-{stale}.rlib"
        good_deps = profile / f"deps/libapp-{good}.rlib"
        unhashed_deps = profile / "deps/unhashed.rlib"
        stale_example = profile / f"examples/demo-{stale}"
        # The failed example name ends in the retained sibling's unit ID.
        colliding_example = profile / f"examples/demo-{good}"
        unhashed_example = profile / "examples/alias"
        uplifted = profile / "stale-binary"
        stray = profile / "build/stray.txt"
        unexecuted = profile / f"build/app/{'c' * 16}/out/probe"
        paths = [
            good_artifact, bad_artifact, stale_build, stale_deps, good_deps, unhashed_deps,
            stale_example, colliding_example, unhashed_example, uplifted, stray, unexecuted,
        ]
        for path in paths:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("data")
        if mode == "restore":
            stats = c.restore(layout, normalize=False)
            self.assertEqual(stats, {"retained": 1, "invalidated": 1})
        else:
            session = self.root / "session-save"
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
        self.assertFalse(colliding_example.exists())
        self.assertFalse(bad_dir.exists())
        self.assertFalse(bad_artifact.exists())
        self.assertFalse(stale_build.exists())
        self.assertFalse(stale_deps.exists())
        self.assertFalse(unhashed_deps.exists())
        self.assertFalse(stale_example.exists())
        self.assertFalse(unhashed_example.exists())
        self.assertFalse(uplifted.exists())
        self.assertFalse(stray.exists())
        self.assertFalse((profile / "build/app" / bad).exists())
        self.assertFalse(unexecuted.exists())

    def test_an_explicit_profile_inside_out_dir_is_not_a_unit(self) -> None:
        target = self.layout.target
        outer = target / "debug/build/demo/0123456789abcdef"
        nested_profile = outer / "out/guest-target/debug"
        nested = nested_profile / "build/pkg/aaaaaaaaaaaaaaaa/fingerprint"
        self.write_fingerprint(nested, "bin-pkg", 1)
        note = outer / "out/bindings.rs"
        note.parent.mkdir(parents=True, exist_ok=True)
        note.write_text("keep")
        layout = c.Layout(self.workspace, target, (nested_profile,))
        self.assertEqual(c.inventory(layout), {})
        stats = c.restore(layout, normalize=False)
        self.assertEqual(stats, {"retained": 0, "invalidated": 0})
        self.assertTrue(nested.is_dir())
        self.assertTrue(note.is_file())

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

    def write_record(self, layout: c.Layout, directory: Path, stem: str) -> None:
        self.write_fingerprint(directory, stem, 7)
        unit = c.read_unit(layout, directory)
        assert unit is not None
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
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
        parser.feed([(
            '1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "/tmp/other", '
            '"-C", "metadata=0123456789abcdef"], ["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]) = 0\n'
        ).encode()])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 1)
        self.assertEqual(events[1]["reason"], "Unknown rustc unit layout")

    def test_trace_ignores_rustc_without_cargo_metadata(self) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
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
        (directory / f"{stem}.json").write_text('{"deps":[],"local":[]}')

    def test_a_package_name_that_ends_in_16_hex_characters_stays(self) -> None:
        target = self.layout.target
        fingerprint = target / "debug/build/demo-aaaaaaaaaaaaaaaa/bbbbbbbbbbbbbbbb/fingerprint"
        self.write_fingerprint(fingerprint, "bin-demo", 1)
        key = fingerprint.relative_to(target).as_posix()
        scanned = c.scan_units(self.layout)
        self.assertEqual(set(scanned.committed), {key})
        dependency = target / "debug/deps/libdemo-bbbbbbbbbbbbbbbb.rlib"
        dependency.parent.mkdir(parents=True)
        dependency.write_text("data")
        c.discard_unrecorded(self.layout, scanned, {key}, c.profile_directories(self.layout, scanned))
        self.assertTrue(fingerprint.is_dir())
        self.assertTrue(dependency.is_file())

    def test_a_profile_or_package_named_out_keeps_its_build_unit(self) -> None:
        target = self.layout.target
        paths = (
            target / "out/build/demo/0123456789abcdef/fingerprint",
            target / "debug/build/out/fedcba9876543210/fingerprint",
            target / "build/build/demo/0123456789abcdef/fingerprint",
            target / "debug/build/build/fedcba9876543210/fingerprint",
        )
        for index, path in enumerate(paths, start=1):
            self.write_fingerprint(path, "bin-demo", index)
        keys = {path.relative_to(target).as_posix() for path in paths}
        self.assertEqual(set(c.inventory(self.layout)), keys)

    def test_a_nested_target_inside_out_dir_is_not_a_unit(self) -> None:
        fingerprint = self.layout.target / (
            "debug/build/demo/0123456789abcdef/out/nested-target/debug/build/child/aaaaaaaaaaaaaaaa/fingerprint"
        )
        self.write_fingerprint(fingerprint, "bin-child", 1)
        self.assertEqual(c.inventory(self.layout), {})
        self.assertTrue(fingerprint.is_dir())

    def test_cleanup_keeps_hashes_inside_one_profile(self) -> None:
        target = self.layout.target
        suffix = "a" * 16
        debug_fingerprint = target / f"debug/build/app/{suffix}/fingerprint"
        self.write_fingerprint(debug_fingerprint, "bin-app", 1)
        (target / f"release/build/app/{suffix}/fingerprint").mkdir(parents=True)
        debug_dependency = target / f"debug/deps/libapp-{suffix}.rlib"
        debug_example = target / f"debug/examples/demo-{suffix}"
        release_dependency = target / f"release/deps/libapp-{suffix}.rlib"
        release_example = target / f"release/examples/demo-{suffix}"
        for path in (debug_dependency, debug_example, release_dependency, release_example):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("data")
        scanned = c.scan_units(self.layout)
        key = debug_fingerprint.relative_to(target).as_posix()
        c.discard_unrecorded(self.layout, scanned, {key}, c.profile_directories(self.layout, scanned))
        self.assertTrue(debug_dependency.is_file())
        self.assertFalse(debug_example.exists())
        self.assertFalse(release_dependency.exists())
        self.assertFalse(release_example.exists())

    def test_restore_skips_a_nested_profile_removed_with_its_outer_unit(self) -> None:
        target = self.layout.target
        outer = target / "release/build/demo/0123456789abcdef/fingerprint"
        nested = target / (
            "release/build/demo/0123456789abcdef/out/nested-target/debug/build/child/aaaaaaaaaaaaaaaa/fingerprint"
        )
        self.write_fingerprint(outer, "run-build-script-build-script-build", 1)
        (outer / "run-build-script-build-script-build.json").write_text('{"deps":[],"local":[]}')
        self.write_fingerprint(nested, "bin-child", 2)
        stats = c.restore(self.layout, normalize=False)
        self.assertEqual(stats, {"retained": 0, "invalidated": 1})
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
        wrong_compile = target / "debug/build/demo/dddddddddddddddd/fingerprint"
        wrong_out = wrong_compile.parent / "out"
        wrong_compile.mkdir(parents=True)
        (wrong_compile / "run-build-script-build").write_text("dddddddddddddddd")
        self.assert_trace_layout_error(
            (
                f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{wrong_out}", '
                '"-C", "extra-filename=-dddddddddddddddd", "-C", "metadata=dddddddddddddddd"], '
                '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]) = 0\n'
            ),
            "Unknown rustc unit layout",
        )

    def test_trace_rejects_a_compiler_override(self) -> None:
        target = self.layout.target
        fingerprint = target / "debug/build/demo/0123456789abcdef/fingerprint"
        fingerprint.mkdir(parents=True)
        out = fingerprint.parent / "out"
        arguments = (
            f'"--crate-name", "demo", "--out-dir", "{out}", '
            '"-C", "extra-filename=-0123456789abcdef", "-C", "metadata=0123456789abcdef"'
        )
        rustc = (
            '1 execve("/usr/bin/rustc", ["rustc", ' + arguments + '], '
            '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "{name}={value}"]) = 0\n'
        )
        for name in ("RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"):
            with self.subTest(name=name):
                self.assert_trace_layout_error(
                    rustc.format(name=name, value="/tmp/rustc-renamed"),
                    UNSUPPORTED_COMPILER,
                )
        renamed = (
            f'1 execve("/tmp/rustc-renamed", ["rustc-renamed", {arguments}], '
            '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]) = 0\n'
        )
        self.assert_trace_layout_error(renamed, UNSUPPORTED_COMPILER)
        empty = (
            f'1 execve("/usr/bin/rustc", ["rustc", {arguments}], '
            '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", '
            '"RUSTC=", "RUSTC_WRAPPER=", "RUSTC_WORKSPACE_WRAPPER="]) = 0\n'
        )
        output = io.StringIO()
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
        parser.feed([empty.encode()])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual(
            [event["unit"] for event in events if event["event"] == "unit"],
            ["debug/build/demo/0123456789abcdef/fingerprint"],
        )
        rustdoc = (
            f'1 execve("/usr/bin/rustdoc", ["rustdoc", "--crate-name", "demo", "--out-dir", "{out}", '
            '"-C", "metadata=0123456789abcdef"], ["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]) = 0\n'
        )
        self.assert_trace_layout_error(rustdoc, UNSUPPORTED_COMPILER)
        doctest = (
            f'1 execve("/usr/bin/rustdoc", ["rustdoc", "--crate-name", "demo", "--test", "src/lib.rs"], '
            '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]) = 0\n'
        )
        output = io.StringIO()
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
        parser.feed([doctest.encode()])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertFalse(any(event["event"] == "unit" for event in events))

    def test_trace_rejects_a_compiler_key_in_a_cargo_config(self) -> None:
        project = self.workspace / ".cargo/config.toml"
        project.parent.mkdir()
        home = self.root / "cargo-home"
        home.mkdir()
        home_config = home / "config.toml"
        cases = (
            (project, '[build]\nrustc = "/tmp/rustc-renamed"\n', None),
            (project, 'build.rustc-wrapper = "/usr/bin/sccache"\n', None),
            (project, '[build]\nrustc-wrapper = ""\n', None),
            (project, '[build]\nrustc-workspace-wrapper = "/opt/alternate/rustc"\n', None),
            (home_config, '[build]\nrustc = "/tmp/rustc-renamed"\n', home),
        )
        for path, text, cargo_home in cases:
            with self.subTest(path=path.name, text=text):
                path.write_text(text)
                environment = '["PATH=/usr/bin"]'
                if cargo_home is not None:
                    environment = f'["PATH=/usr/bin", "CARGO_HOME={cargo_home}"]'
                output = io.StringIO()
                parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
                parser.feed([
                    f'1 execve("/usr/bin/cargo", ["cargo", "test"], {environment}) = 0\n'.encode(),
                    f'1 openat(AT_FDCWD, "{path}", O_RDONLY) = 3<{path}>\n'.encode(),
                ])
                events = [json.loads(line) for line in output.getvalue().splitlines()]
                self.assertEqual(events[-1]["errors"], 1)
                self.assertEqual(events[1]["reason"], UNSUPPORTED_COMPILER)
                self.assertFalse(any(event["event"] == "unit" for event in events))
        project.write_text("[build]\njobs = 16\n")
        broken = self.workspace / ".cargo/config"
        broken.write_text("not = [")
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
        parser.feed([
            b'1 execve("/usr/bin/cargo", ["cargo", "test"], ["PATH=/usr/bin"]) = 0\n',
            f'1 openat(AT_FDCWD, "{broken}", O_RDONLY) = 3<{broken}>\n'.encode(),
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 1)
        self.assertEqual(events[1]["reason"], "A Cargo config is unreadable.")
        fingerprint = self.layout.target / "debug/build/demo/0123456789abcdef/fingerprint"
        fingerprint.mkdir(parents=True)
        out = fingerprint.parent / "out"
        repository_config = Path(__file__).resolve().parents[2] / ".cargo/config.toml"
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
        parser.feed([
            b'1 execve("/usr/bin/cargo", ["cargo", "test"], ["PATH=/usr/bin"]) = 0\n',
            f'1 openat(AT_FDCWD, "{project}", O_RDONLY) = 3<{project}>\n'.encode(),
            f'1 openat(AT_FDCWD, "{repository_config}", O_RDONLY) = 4<{repository_config}>\n'.encode(),
            (
                f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{out}", '
                '"-C", "extra-filename=-0123456789abcdef", "-C", "metadata=0123456789abcdef"], '
                '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]) = 0\n'
            ).encode(),
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual(
            [event["unit"] for event in events if event["event"] == "unit"],
            ["debug/build/demo/0123456789abcdef/fingerprint"],
        )

    def test_trace_keeps_cargo_rustc_on_a_build_script(self) -> None:
        target = self.layout.target
        compile_dir = target / "debug/build/demo/0123456789abcdef/fingerprint"
        compile_dir.mkdir(parents=True)
        (compile_dir / "build-script-build-script-build").write_text("0123456789abcdef")
        run = target / "debug/build/demo/fedcba9876543210/fingerprint"
        run.mkdir(parents=True)
        executable = compile_dir.parent / "out/build_script_build"
        out = run.parent / "out"
        output = io.StringIO()
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
        parser.feed([
            b'1 execve("/usr/bin/cargo", ["cargo", "test"], ["PATH=/usr/bin"]) = 0\n',
            b"1 clone(child_stack=NULL, flags=SIGCHLD) = 2\n",
            (
                f'2 execve("{executable}", ["{executable}"], '
                f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={out}", '
                '"RUSTC=/usr/bin/rustc"]) = 0\n'
            ).encode(),
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual(
            [(event["kind"], event["unit"]) for event in events if event["event"] == "unit"],
            [("run", "debug/build/demo/fedcba9876543210/fingerprint")],
        )

    def test_supported_compiler_is_the_workspace_toolchain_rustc(self) -> None:
        workspace = Path(__file__).resolve().parents[2]
        env = os.environ.copy()
        env.pop("RUSTUP_TOOLCHAIN", None)
        reported = subprocess.check_output(
            ["rustup", "which", "rustc"], cwd=workspace, env=env, text=True,
        ).strip()
        expected = Path(os.path.realpath(reported))
        with patch.dict(os.environ, {"RUSTUP_TOOLCHAIN": "not-a-toolchain"}):
            resolved = supported_compiler(workspace)
        self.assertEqual(resolved, expected)
        channel = toolchain_channel((workspace / "rust-toolchain.toml").read_text(encoding="utf-8"))
        version = subprocess.check_output([str(resolved), "--version"], text=True).splitlines()[0]
        self.assertTrue(version_line_matches(version, channel))
        rustup = rustup_binary()
        self.assertIsNotNone(rustup)
        assert rustup is not None
        self.assertFalse(os.path.samefile(resolved, rustup))

    def test_version_line_matches_only_the_channel_token(self) -> None:
        self.assertTrue(version_line_matches("rustc 9.9.9 (hash date)", "9.9.9"))
        self.assertTrue(version_line_matches("rustc 9.9.9", "9.9.9"))
        self.assertFalse(version_line_matches("rustc 9.9.9-dev", "9.9.9"))
        self.assertFalse(version_line_matches("rustc 9.9.90 (hash)", "9.9.9"))
        self.assertFalse(version_line_matches("rustc 9.9.8 (hash)", "9.9.9"))

    def test_pinned_compiler_rejects_a_proxy_and_a_different_version(self) -> None:
        workspace = Path(__file__).resolve().parents[2]
        compiler = supported_compiler(workspace)
        rustup = rustup_binary()
        self.assertIsNotNone(rustup)
        assert rustup is not None
        with self.assertRaises(TraceError) as proxy:
            accept_pinned_compiler(rustup, workspace)
        self.assertEqual(str(proxy.exception), "The pinned compiler is the rustup proxy.")
        other = self.root / "other-pin"
        other.mkdir()
        (other / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "9.9.9"\n', encoding="utf-8")
        with self.assertRaises(TraceError) as version:
            accept_pinned_compiler(compiler, other)
        self.assertEqual(str(version.exception), "The compiler version is not the pinned toolchain.")

    def test_trace_resolves_a_bare_rustc_name_through_path(self) -> None:
        workspace = Path(__file__).resolve().parents[2]
        compiler = supported_compiler(workspace)
        rustup = rustup_binary()
        self.assertIsNotNone(rustup)
        assert rustup is not None
        target = self.layout.target

        def feed(path: Path, extra: str = "") -> list[dict]:
            output = io.StringIO()
            parser = Parser(
                workspace, target, output, compiler=compiler, rustup=rustup,
            )
            # A real exec inherits the caller rustup variables. The proxy reads them.
            inherited = [
                f'"{name}={os.environ[name]}"'
                for name in ("RUSTUP_HOME", "RUSTUP_TOOLCHAIN")
                if os.environ.get(name)
            ]
            environment = ", ".join([f'"PATH={path}"', *inherited])
            if extra:
                environment = f"{environment}, {extra}"
            parser.feed([
                (
                    '1 execve("rustc", ["rustc", "--edition=2024", "main.rs", "-o", "lowered"], '
                    f'[{environment}]) = 0\n'
                ).encode(),
            ])
            return [json.loads(item) for item in output.getvalue().splitlines()]

        link_dir = self.root / "link-bin"
        link_dir.mkdir()
        (link_dir / "rustc").symlink_to(compiler)
        linked = feed(link_dir)
        self.assertEqual(linked[-1]["errors"], 0)
        self.assertFalse(any(event["event"] == "unit" for event in linked))

        proxy_dir = self.root / "proxy-bin"
        proxy_dir.mkdir()
        (proxy_dir / "rustc").symlink_to(rustup)
        proxied = feed(proxy_dir)
        self.assertEqual(proxied[-1]["errors"], 0)
        self.assertFalse(any(event["event"] == "unit" for event in proxied))
        proxied_rustc = feed(proxy_dir, '"RUSTC=rustc"')
        self.assertEqual(proxied_rustc[-1]["errors"], 0)

        fake_dir = self.root / "fake-bin"
        fake_dir.mkdir()
        fake = fake_dir / "rustc"
        fake.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        fake.chmod(0o755)
        rejected = feed(fake_dir)
        self.assertEqual(rejected[-1]["errors"], 1)
        self.assertEqual(
            [event["reason"] for event in rejected if event["event"] == "trace-error"],
            [UNSUPPORTED_COMPILER],
        )

        overridden = feed(proxy_dir, '"RUSTUP_TOOLCHAIN=not-a-toolchain"')
        self.assertEqual(overridden[-1]["errors"], 1)
        self.assertEqual(
            [event["reason"] for event in overridden if event["event"] == "trace-error"],
            [UNSUPPORTED_COMPILER],
        )

    def test_trace_records_a_proxy_compile_unit(self) -> None:
        workspace = Path(__file__).resolve().parents[2]
        compiler = supported_compiler(workspace)
        rustup = rustup_binary()
        self.assertIsNotNone(rustup)
        assert rustup is not None
        target = self.layout.target
        fingerprint = target / "debug/build/demo/0123456789abcdef/fingerprint"
        fingerprint.mkdir(parents=True)
        out = fingerprint.parent / "out"
        proxy_dir = self.root / "proxy-compile"
        proxy_dir.mkdir()
        (proxy_dir / "rustc").symlink_to(rustup)
        inherited = [
            f'"{name}={os.environ[name]}"'
            for name in ("RUSTUP_HOME", "RUSTUP_TOOLCHAIN")
            if os.environ.get(name)
        ]
        environment = ", ".join([
            f'"PATH={proxy_dir}"',
            '"CARGO_PKG_NAME=demo"',
            '"CARGO_MANIFEST_DIR=/src/demo"',
            *inherited,
        ])
        output = io.StringIO()
        parser = Parser(workspace, target, output, compiler=compiler, rustup=rustup)
        parser.feed([(
            '1 execve("rustc", ["rustc", "--crate-name", "demo", "--out-dir", '
            f'"{out}", "-C", "extra-filename=-0123456789abcdef", "-C", "metadata=0123456789abcdef"], '
            f'[{environment}]) = 0\n'
        ).encode()])
        events = [json.loads(item) for item in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual(
            [(event["kind"], event["unit"]) for event in events if event["event"] == "unit"],
            [("compile", "debug/build/demo/0123456789abcdef/fingerprint")],
        )

    def test_trace_rejects_a_compiler_exec_at_another_path(self) -> None:
        target = self.layout.target
        fingerprint = target / "debug/build/demo/0123456789abcdef/fingerprint"
        fingerprint.mkdir(parents=True)
        out = fingerprint.parent / "out"
        alternate = "/opt/alternate/rustc"

        def compiler_line(path: str) -> str:
            return (
                f'1 execve("{path}", ["{path}", "--crate-name", "demo", "--out-dir", "{out}", '
                '"-C", "extra-filename=-0123456789abcdef", "-C", "metadata=0123456789abcdef"], '
                '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]) = 0\n'
            )

        def assert_exec_rejected(lines: list[str]) -> None:
            output = io.StringIO()
            parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
            parser.feed([line.encode() for line in lines])
            events = [json.loads(item) for item in output.getvalue().splitlines()]
            self.assertEqual(events[-1]["errors"], 1)
            errors = [event for event in events if event["event"] == "trace-error"]
            self.assertEqual([event["reason"] for event in errors], [UNSUPPORTED_COMPILER])
            self.assertEqual(errors[0]["operation"], "execve")
            self.assertFalse(any(event["event"] == "unit" for event in events))

        for name in (
            "CARGO_BUILD_RUSTC",
            "CARGO_BUILD_RUSTC_WRAPPER",
            "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
        ):
            with self.subTest(name=name):
                assert_exec_rejected([
                    f'1 execve("/usr/bin/cargo", ["cargo", "test"], ["{name}={alternate}"]) = 0\n',
                    compiler_line(alternate),
                ])
        assert_exec_rejected([
            '1 execve("/usr/bin/cargo", ["cargo", "test", "--config", '
            f'"build.rustc={alternate}"], ["PATH=/usr/bin"]) = 0\n',
            compiler_line(alternate),
        ])
        config = self.root / "compiler-settings.toml"
        config.write_text(f'[build]\nrustc = "{alternate}"\n', encoding="utf-8")
        assert_exec_rejected([
            f'1 execve("/usr/bin/cargo", ["cargo", "test", "--config", "{config}"], ["PATH=/usr/bin"]) = 0\n',
            f'1 openat(AT_FDCWD, "{config}", O_RDONLY) = 3<{config}>\n',
            compiler_line(alternate),
        ])
        assert_exec_rejected([
            '1 execve("/usr/bin/cargo", ["cargo", "test"], ["PATH=/usr/bin"]) = 0\n',
            compiler_line("/opt/alternate/sccache"),
        ])
        output = io.StringIO()
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
        parser.feed([
            f'1 execve("/usr/bin/cargo", ["cargo", "test"], ["CARGO_BUILD_RUSTC={alternate}"]) = 0\n'.encode(),
            compiler_line("/usr/bin/rustc").encode(),
        ])
        events = [json.loads(item) for item in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual(
            [event["unit"] for event in events if event["event"] == "unit"],
            ["debug/build/demo/0123456789abcdef/fingerprint"],
        )

    def test_trace_rejects_a_build_script_rustc_at_another_path(self) -> None:
        target = self.layout.target
        compile_dir = target / "debug/build/demo/0123456789abcdef/fingerprint"
        compile_dir.mkdir(parents=True)
        (compile_dir / "build-script-build-script-build").write_text("0123456789abcdef")
        run = target / "debug/build/demo/fedcba9876543210/fingerprint"
        run.mkdir(parents=True)
        executable = compile_dir.parent / "out/build_script_build"
        out = run.parent / "out"
        rejected = (
            f'1 execve("{executable}", ["{executable}"], '
            f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={out}", '
            '"RUSTC=/opt/alternate/rustc"]) = 0\n'
        )
        self.assert_trace_layout_error(rejected, UNSUPPORTED_COMPILER)
        fingerprint = target / "debug/build/demo/aaaaaaaaaaaaaaaa/fingerprint"
        fingerprint.mkdir(parents=True)
        compile_out = fingerprint.parent / "out"
        output = io.StringIO()
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
        parser.feed([
            (
                f'1 execve("{executable}", ["{executable}"], '
                f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "OUT_DIR={out}", '
                '"RUSTC=/usr/bin/rustc"]) = 0\n'
            ).encode(),
            (
                f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{compile_out}", '
                '"-C", "extra-filename=-aaaaaaaaaaaaaaaa", "-C", "metadata=aaaaaaaaaaaaaaaa"], '
                '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "RUSTC=/usr/bin/rustc"]) = 0\n'
            ).encode(),
        ])
        events = [json.loads(item) for item in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual(
            [(event["kind"], event["unit"]) for event in events if event["event"] == "unit"],
            [
                ("run", "debug/build/demo/fedcba9876543210/fingerprint"),
                ("compile", "debug/build/demo/aaaaaaaaaaaaaaaa/fingerprint"),
            ],
        )

    def test_trace_keeps_a_prefix_named_test_execution_outside_the_unit_contract(self) -> None:
        target = self.layout.target
        unit = "3e3ed1173044f8b5"
        spellings = (
            ("build-script-demo", "test-lib-build_script_demo"),
            ("build_script_demo", "test-integration-test-build_script_demo"),
        )
        for package, marker in spellings:
            with self.subTest(package=package, marker=marker):
                fingerprint = target / f"debug/build/{package}/{unit}/fingerprint"
                self.write_fingerprint(fingerprint, marker, 1)
                executable = fingerprint.parent / f"out/build_script_demo-{unit}"
                output = io.StringIO()
                parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
                parser.feed([(
                    f'1 execve("{executable}", ["{executable}"], ["CARGO_PKG_NAME={package}"]) = 0\n'
                ).encode()])
                events = [json.loads(line) for line in output.getvalue().splitlines()]
                self.assertEqual(events[-1]["errors"], 0)
                self.assertFalse(any(event["event"] == "unit" for event in events))
        for marker in ("test-bin-build_script_demo", "test-example-build_script_demo", "test-bench-build_script_demo"):
            with self.subTest(marker=marker):
                fingerprint = target / f"debug/build/build-script-demo/{unit}/fingerprint"
                for child in fingerprint.iterdir():
                    child.unlink()
                self.write_fingerprint(fingerprint, marker, 1)
                executable = fingerprint.parent / f"out/build_script_demo-{unit}"
                output = io.StringIO()
                parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
                parser.feed([(
                    f'1 execve("{executable}", ["{executable}"], []) = 0\n'
                ).encode()])
                events = [json.loads(line) for line in output.getvalue().splitlines()]
                self.assertEqual(events[-1]["errors"], 0)
                self.assertFalse(any(event["event"] == "unit" for event in events))
        fingerprint = target / f"debug/build/build-script-demo/{unit}/fingerprint"
        for child in fingerprint.iterdir():
            child.unlink()
        self.write_fingerprint(fingerprint, "test-lib-build_script_demo", 1)
        (fingerprint / "build-script-build-script-build").write_text(unit)
        executable = fingerprint.parent / f"out/build_script_demo-{unit}"
        out = fingerprint.parent / "out"
        output = io.StringIO()
        parser = Parser(self.workspace, target, output, compiler=FIXTURE_COMPILER)
        parser.feed([
            b'1 execve("/usr/bin/cargo", ["cargo", "test"], ["PATH=/usr/bin"]) = 0\n',
            b"1 clone(child_stack=NULL, flags=SIGCHLD) = 2\n",
            (
                f'2 execve("/usr/bin/rustc", ["rustc", "--crate-name", "build_script_demo", "--out-dir", "{out}", '
                f'"-C", "metadata={unit}"], ["CARGO_PKG_NAME=build-script-demo", '
                f'"CARGO_MANIFEST_DIR=/src/build-script-demo"]) = 0\n'
            ).encode(),
            b"1 clone(child_stack=NULL, flags=SIGCHLD) = 3\n",
            f'3 execve("{executable}", ["{executable}"], ["CARGO_PKG_NAME=build-script-demo"]) = 0\n'.encode(),
        ])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual(
            [(event["kind"], event["unit"]) for event in events if event["event"] == "unit"],
            [("compile", f"debug/build/build-script-demo/{unit}/fingerprint")],
        )

    def test_trace_rejects_a_prefix_named_executable_without_a_test_marker(self) -> None:
        target = self.layout.target
        unit = "3e3ed1173044f8b5"
        fingerprint = target / f"debug/build/build-script-demo/{unit}/fingerprint"
        executable = fingerprint.parent / f"out/build_script_demo-{unit}"
        out = target / f"debug/build/build-script-demo/fedcba9876543210/out"
        environment = (
            f'["CARGO_PKG_NAME=build-script-demo", "CARGO_MANIFEST_DIR=/src/build-script-demo", "OUT_DIR={out}"]'
        )
        line = f'1 execve("{executable}", ["{executable}"], {environment}) = 0\n'
        self.assert_trace_layout_error(line, "Unknown build script unit layout")
        fingerprint.mkdir(parents=True)
        (fingerprint / "test-lib-build_script_demo.json").write_text("{}")
        self.assert_trace_layout_error(line, "Unknown build script unit layout")
        (fingerprint / "test-lib-build_script_demo.json").unlink()
        (fingerprint / "lib-build_script_demo").write_text(unit)
        self.assert_trace_layout_error(line, "Unknown build script unit layout")
        (fingerprint / "lib-build_script_demo").unlink()
        (fingerprint / "dep-test-lib-build_script_demo").write_text(unit)
        self.assert_trace_layout_error(line, "Unknown build script unit layout")
        (fingerprint / "dep-test-lib-build_script_demo").unlink()
        (fingerprint / "test-garbage").write_text(unit)
        self.assert_trace_layout_error(line, "Unknown build script unit layout")
        (out.parent / "fingerprint").mkdir(parents=True)
        (fingerprint / "build-script-build-script-build").write_text(unit)
        self.assert_trace_layout_error(line, "Unknown build script unit layout")
        (fingerprint / "test-garbage").unlink()
        (fingerprint / "build-script-build-script-build").unlink()
        (fingerprint / "bench-bin-build_script_demo").write_text(unit)
        self.assert_trace_layout_error(line, "Unknown build script unit layout")
        (fingerprint / "bench-bin-build_script_demo").unlink()
        (fingerprint / "test-lib-fake").mkdir()
        self.assert_trace_layout_error(line, "Unknown build script unit layout")
        (fingerprint / "test-lib-fake").rmdir()
        marker_target = fingerprint / "marker-target"
        marker_target.write_text(unit)
        (fingerprint / "test-bin-link").symlink_to(marker_target)
        self.assert_trace_layout_error(line, "Unknown build script unit layout")

    def test_trace_records_one_unit_for_the_pinned_dx_wrapper(self) -> None:
        target = self.layout.target
        fingerprint = target / "debug/build/demo/0123456789abcdef/fingerprint"
        fingerprint.mkdir(parents=True)
        out = fingerprint.parent / "out"
        compiler = self.root / "toolchain" / "rustc"
        compiler.parent.mkdir()
        compiler.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        compiler.chmod(0o755)
        dx_version = dioxus_cli_version()
        dx = self.root / "dx-home" / "dx"
        dx.parent.mkdir()
        dx.write_text(f"#!/bin/sh\necho 'dioxus {dx_version} (bfcc111)'\n", encoding="utf-8")
        dx.chmod(0o755)
        nightly = self.root / "nightly" / "rustc"
        nightly.parent.mkdir()
        nightly.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        nightly.chmod(0o755)
        other = self.root / "other-dx"
        other.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        other.chmod(0o755)
        link = self.root / "dx-link"
        link.symlink_to(dx)
        compile_args = (
            f'"--crate-name", "demo", "--out-dir", "{out}", '
            '"-C", "extra-filename=-0123456789abcdef", "-C", "metadata=0123456789abcdef"'
        )
        cargo = (
            f'"CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", '
            f'"RUSTC_WORKSPACE_WRAPPER={dx}"'
        )

        def events_of(lines: list[str]) -> list[dict]:
            output = io.StringIO()
            parser = Parser(self.workspace, target, output, compiler=compiler, dx=dx)
            parser.feed([line.encode() for line in lines])
            return [json.loads(item) for item in output.getvalue().splitlines()]

        opened = events_of([
            f'1 execve("{dx}", ["{dx}", "bundle"], ["PATH={dx.parent}"]) = 0\n',
            "1 clone(child_stack=NULL, flags=SIGCHLD) = 2\n",
            f'2 execve("{dx}", ["{dx}", "{compiler}", {compile_args}], [{cargo}]) = 0\n',
            "2 clone(child_stack=NULL, flags=SIGCHLD) = 3\n",
            f'3 execve("{compiler}", ["rustc", {compile_args}], [{cargo}]) = 0\n',
        ])
        self.assertEqual(opened[-1]["errors"], 0)
        self.assertEqual(
            [(event["kind"], event["unit"], event["via_dx"]) for event in opened if event["event"] == "unit"],
            [("compile", "debug/build/demo/0123456789abcdef/fingerprint", True)],
        )
        wrapper_only = events_of([
            f'1 execve("{dx}", ["{dx}", "{compiler}", {compile_args}], [{cargo}]) = 0\n',
        ])
        self.assertEqual(wrapper_only[-1]["errors"], 0)
        self.assertFalse(any(event["event"] == "unit" for event in wrapper_only))
        rejected_wrapper = events_of([
            f'1 execve("{dx}", ["{dx}", "{compiler}", {compile_args}], '
            f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", "RUSTC_WRAPPER={dx}"]) = 0\n',
        ])
        self.assertEqual(rejected_wrapper[-1]["errors"], 1)
        self.assertEqual(rejected_wrapper[1]["reason"], UNSUPPORTED_COMPILER)
        rejected_other = events_of([
            f'1 execve("{other}", ["{other}", "{compiler}", {compile_args}], '
            f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", '
            f'"RUSTC_WORKSPACE_WRAPPER={other}"]) = 0\n',
        ])
        self.assertEqual(rejected_other[-1]["errors"], 1)
        self.assertFalse(any(event["event"] == "unit" for event in rejected_other))
        linked = events_of([
            f'1 execve("{dx}", ["{dx}", "{compiler}", {compile_args}], '
            f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", '
            f'"RUSTC_WORKSPACE_WRAPPER={link}"]) = 0\n',
            f'1 execve("{compiler}", ["rustc", {compile_args}], '
            f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo", '
            f'"RUSTC_WORKSPACE_WRAPPER={link}"]) = 0\n',
        ])
        self.assertEqual(linked[-1]["errors"], 0)
        self.assertEqual(
            [event["unit"] for event in linked if event["event"] == "unit"],
            ["debug/build/demo/0123456789abcdef/fingerprint"],
        )
        # One process runs the wrapper, then the compiler, then the compiler again.
        # The second compiler exec is the real rustc after the rustup proxy.
        build_out = target / "debug/build/demo/fedcba9876543210/out"
        build_fingerprint = build_out.parent / "fingerprint"
        build_fingerprint.mkdir(parents=True)
        build_args = (
            f'"--crate-name", "demo", "--out-dir", "{build_out}", '
            '"-C", "extra-filename=-fedcba9876543210", "-C", "metadata=fedcba9876543210"'
        )
        handed = events_of([
            f'4 execve("{dx}", ["{dx}", "{compiler}", {build_args}], [{cargo}]) = 0\n',
            f'4 execve("{compiler}", ["rustc", {build_args}], [{cargo}]) = 0\n',
            f'4 execve("{compiler}", ["rustc", {build_args}], [{cargo}]) = 0\n',
        ])
        self.assertEqual(handed[-1]["errors"], 0)
        self.assertEqual(
            [(event["kind"], event["unit"], event["via_dx"]) for event in handed if event["event"] == "unit"],
            [("compile", "debug/build/demo/fedcba9876543210/fingerprint", True)],
        )
        wrong_compiler = events_of([
            f'1 execve("{dx}", ["{dx}", "{nightly}", {compile_args}], [{cargo}]) = 0\n',
        ])
        self.assertEqual(wrong_compiler[-1]["errors"], 1)
        self.assertFalse(any(event["event"] == "unit" for event in wrong_compiler))
        probe = events_of([
            f'1 execve("{dx}", ["{dx}", "bundle"], ["PATH={dx.parent}"]) = 0\n',
            "1 clone(child_stack=NULL, flags=SIGCHLD) = 2\n",
            f'2 execve("{nightly}", ["{nightly}", "-vV"], ["RUSTC_WORKSPACE_WRAPPER={dx}"]) = 0\n',
            "2 clone(child_stack=NULL, flags=SIGCHLD) = 3\n",
            (
                f'3 execve("{nightly}", ["{nightly}", "-", "--crate-name", "___", '
                f'"--print=file-names"], ["RUSTC_WORKSPACE_WRAPPER={dx}"]) = 0\n'
            ),
        ])
        self.assertEqual(probe[-1]["errors"], 0)
        self.assertFalse(any(event["event"] == "unit" for event in probe))
        nightly_compile = events_of([
            f'1 execve("{dx}", ["{dx}", "bundle"], ["PATH={dx.parent}"]) = 0\n',
            "1 clone(child_stack=NULL, flags=SIGCHLD) = 2\n",
            f'2 execve("{nightly}", ["rustc", {compile_args}], [{cargo}]) = 0\n',
        ])
        self.assertEqual(nightly_compile[-1]["errors"], 1)
        self.assertFalse(any(event["event"] == "unit" for event in nightly_compile))
        bare_nightly = events_of([
            f'1 execve("{nightly}", ["{nightly}", "-vV"], ["PATH=/usr/bin"]) = 0\n',
        ])
        self.assertEqual(bare_nightly[-1]["errors"], 1)

    def test_pinned_dx_version_line_and_path(self) -> None:
        dx_version = dioxus_cli_version()
        self.assertTrue(dx_version_line_matches(f"dioxus {dx_version} (bfcc111)", dx_version))
        self.assertTrue(dx_version_line_matches(f"dioxus {dx_version} (BFCC111)", dx_version))
        self.assertTrue(dx_version_line_matches(f"dioxus {dx_version}", dx_version))
        self.assertFalse(dx_version_line_matches(f"dioxus {dx_version} garbage", dx_version))
        self.assertFalse(dx_version_line_matches(f"dioxus {dx_version} (not a hash)", dx_version))
        self.assertFalse(dx_version_line_matches(f"dioxus {dx_version} (hash)", dx_version))
        self.assertFalse(dx_version_line_matches(f"dioxus {dx_version} ()", dx_version))
        self.assertFalse(dx_version_line_matches(f"dioxus {dx_version} (bfcc111) extra", dx_version))
        self.assertFalse(dx_version_line_matches(f"dioxus {dx_version}(bfcc111)", dx_version))
        self.assertFalse(dx_version_line_matches(f"dioxus {dx_version} ", dx_version))
        self.assertFalse(dx_version_line_matches(f"dioxus {dx_version}.1", dx_version))
        self.assertFalse(dx_version_line_matches("dioxus 0.7.8", dx_version))
        bin_dir = self.root / "bin"
        bin_dir.mkdir()
        path_dx = bin_dir / "dx"
        path_dx.write_text(f"#!/bin/sh\necho 'dioxus {dx_version} (bfcc111)'\n", encoding="utf-8")
        path_dx.chmod(0o755)
        self.assertEqual(pinned_dx({"PATH": str(bin_dir)}, str(self.root), dx_version), path_dx.resolve())
        path_dx.write_text("#!/bin/sh\necho 'dioxus 0.7.8'\n", encoding="utf-8")
        self.assertIsNone(pinned_dx({"PATH": str(bin_dir)}, str(self.root), dx_version))
        self.assertIsNone(pinned_dx({"PATH": str(self.root / "missing")}, str(self.root), dx_version))

    def test_dioxus_cli_version_requires_one_workflow_pin(self) -> None:
        workflow = self.root / "render-image.yml"
        workflow.write_text(
            "env:\n  DIOXUS_CLI_VERSION: 0.7.9 # pin\n"
            "  # DIOXUS_CLI_VERSION: 0.8.0\n"
            "  run: echo ${{ env.DIOXUS_CLI_VERSION }}\n",
            encoding="utf-8",
        )
        self.assertEqual(dioxus_cli_version(workflow), "0.7.9")
        workflow.write_text("env: {DIOXUS_CLI_VERSION: 0.7.9}\n", encoding="utf-8")
        self.assertEqual(dioxus_cli_version(workflow), "0.7.9")
        workflow.write_text('env: {"DIOXUS_CLI_VERSION": "0.7.9"}\n', encoding="utf-8")
        self.assertEqual(dioxus_cli_version(workflow), "0.7.9")
        workflow.write_text(
            "env: {DIOXUS_CLI_VERSION: 0.7.9, DIOXUS_CLI_VERSION: 0.8.0}\n",
            encoding="utf-8",
        )
        with self.assertRaises(TraceError):
            dioxus_cli_version(workflow)
        workflow.write_text(
            "env:\n  DIOXUS_CLI_VERSION: 0.7.9\n  DIOXUS_CLI_VERSION: 0.8.0\n",
            encoding="utf-8",
        )
        with self.assertRaises(TraceError):
            dioxus_cli_version(workflow)
        workflow.write_text(
            "env:\n  DIOXUS_CLI_VERSION: 0.7.9\n"
            "jobs:\n  build:\n    env: {\"DIOXUS_CLI_VERSION\": \"0.8.0\"}\n",
            encoding="utf-8",
        )
        with self.assertRaises(TraceError):
            dioxus_cli_version(workflow)
        workflow.write_text(
            "env:\n  DIOXUS_CLI_VERSION: 0.7.9\n"
            "jobs:\n  build:\n    env:\n      DIOXUS_CLI_VERSION: garbage\n",
            encoding="utf-8",
        )
        with self.assertRaises(TraceError):
            dioxus_cli_version(workflow)
        workflow.write_text("env:\n  DIOXUS_CLI_VERSION: garbage\n", encoding="utf-8")
        with self.assertRaises(TraceError):
            dioxus_cli_version(workflow)
        workflow.write_text("env:\n  OTHER: 1\n", encoding="utf-8")
        with self.assertRaises(TraceError):
            dioxus_cli_version(workflow)

    def test_supported_compiler_disables_rustup_auto_install(self) -> None:
        if rustup_binary() is None:
            self.skipTest("rustup is absent")
        workspace = Path(__file__).resolve().parents[2]
        captured: dict[str, str] = {}
        real_run = subprocess.run

        def wrapped(args, **kwargs):
            command = list(args)
            if command[-2:] == ["which", "rustc"]:
                env = kwargs.get("env") or {}
                captured.update(env)
                class Result:
                    returncode = 1
                    stdout = ""
                    stderr = ""
                return Result()
            return real_run(args, **kwargs)

        with patch.dict(os.environ, {
            "RUSTUP_AUTO_INSTALL": "1",
            "RUSTUP_HOME": "/srv/rustup-pin",
            "RUSTUP_TOOLCHAIN": "nightly",
        }):
            with patch("tools.cargo_inputs_trace.subprocess.run", wrapped):
                with self.assertRaises(TraceError):
                    supported_compiler(workspace)
        self.assertEqual(captured.get("RUSTUP_AUTO_INSTALL"), "0")
        self.assertEqual(captured.get("RUSTUP_HOME"), "/srv/rustup-pin")
        self.assertNotIn("RUSTUP_TOOLCHAIN", captured)

    def test_query_rustup_rustc_disables_auto_install(self) -> None:
        record = self.root / "auto.txt"
        rustup = self.root / "rustup"
        rustup.write_text(
            "#!/bin/sh\n"
            f"printf '%s\\n' \"$RUSTUP_AUTO_INSTALL\" > '{record}'\n"
            "printf '%s\\n' /bin/true\n",
            encoding="utf-8",
        )
        rustup.chmod(0o755)
        with patch.dict(os.environ, {"RUSTUP_AUTO_INSTALL": "1"}):
            found = query_rustup_rustc(rustup, str(self.root), None, None)
        self.assertEqual(found, Path("/bin/true").resolve())
        self.assertEqual(record.read_text(encoding="utf-8").strip(), "0")

    def test_trace_disables_rustup_auto_install(self) -> None:
        workspace = Path(__file__).resolve().parents[2]
        journal = self.root / "journal.jsonl"
        marker = self.root / "auto.txt"
        target = self.layout.target.resolve()
        code = (
            "import os\n"
            "from pathlib import Path\n"
            f"Path({str(marker)!r}).write_text(os.environ.get('RUSTUP_AUTO_INSTALL', ''))\n"
        )
        with patch.dict(os.environ, {
            "GITHUB_ACTIONS": "true",
            "RUSTUP_AUTO_INSTALL": "1",
            "RUSTUP_HOME": shared_rustup_home(),
            "CARGO_TARGET_DIR": str(target),
        }):
            os.environ.pop("RUSTUP_TOOLCHAIN", None)
            with patch(
                "tools.cargo_inputs_trace.supported_compiler",
                return_value=Path("/usr/bin/rustc"),
            ) as compiler:
                status = trace(workspace, target, journal, ["python3", "-c", code])
        self.assertEqual(status, 0, journal.read_text(encoding="utf-8") if journal.exists() else "")
        self.assertEqual(marker.read_text(encoding="utf-8"), "0")
        compiler.assert_called_once_with(workspace)

    def test_trace_rejects_each_unpinned_environment(self) -> None:
        workspace = Path(__file__).resolve().parents[2]
        channel = toolchain_channel((workspace / "rust-toolchain.toml").read_text(encoding="utf-8"))
        wrong = "9.9.9" if channel != "9.9.9" else "8.8.8"
        target = self.layout.target.resolve()
        rustup_home = shared_rustup_home()
        pinned = {
            "RUSTUP_HOME": rustup_home,
            "CARGO_TARGET_DIR": str(target),
        }
        other = str((self.root / "other-target").resolve())
        cases = {
            "home": ({**pinned, "RUSTUP_HOME": str(self.root / "not-the-pin")}, ("RUSTUP_TOOLCHAIN",)),
            "target": ({**pinned, "CARGO_TARGET_DIR": other}, ("RUSTUP_TOOLCHAIN",)),
            "toolchain": ({**pinned, "RUSTUP_TOOLCHAIN": wrong}, ()),
            "empty-toolchain": ({**pinned, "RUSTUP_TOOLCHAIN": ""}, ()),
            "empty-home": ({**pinned, "RUSTUP_HOME": ""}, ("RUSTUP_TOOLCHAIN",)),
            "missing-home": ({"CARGO_TARGET_DIR": str(target)}, ("RUSTUP_HOME", "RUSTUP_TOOLCHAIN")),
            "missing-target": ({"RUSTUP_HOME": rustup_home}, ("CARGO_TARGET_DIR", "RUSTUP_TOOLCHAIN")),
        }
        for name, (updates, remove) in cases.items():
            with self.subTest(name=name):
                journal = self.root / f"reject-{name}.jsonl"
                with patch.dict(os.environ, {**updates, "GITHUB_ACTIONS": "true"}):
                    for key in remove:
                        os.environ.pop(key, None)
                    with patch(
                        "tools.cargo_inputs_trace.supported_compiler",
                        return_value=Path("/usr/bin/rustc"),
                    ) as compiler:
                        with self.assertRaises(TraceError) as caught:
                            trace(workspace, target, journal, ["/bin/true"])
                        compiler.assert_not_called()
                self.assertEqual(str(caught.exception), UNSUPPORTED_COMPILER)
                self.assertFalse(journal.exists())

    def test_trace_queries_the_compiler_when_the_toolchain_matches(self) -> None:
        workspace = Path(__file__).resolve().parents[2]
        channel = toolchain_channel((workspace / "rust-toolchain.toml").read_text(encoding="utf-8"))
        target = self.layout.target.resolve()
        journal = self.root / "match.jsonl"
        with patch.dict(os.environ, {
            "GITHUB_ACTIONS": "true",
            "RUSTUP_HOME": shared_rustup_home(),
            "CARGO_TARGET_DIR": str(target),
            "RUSTUP_TOOLCHAIN": channel,
        }):
            with patch(
                "tools.cargo_inputs_trace.supported_compiler",
                return_value=Path("/usr/bin/rustc"),
            ) as compiler:
                status = trace(workspace, target, journal, ["/bin/true"])
        detail = journal.read_text(encoding="utf-8") if journal.exists() else ""
        self.assertEqual(status, 0, detail)
        compiler.assert_called_once_with(workspace)

    def test_trace_accepts_a_resolved_session_target(self) -> None:
        workspace = Path(__file__).resolve().parents[2]
        target = self.layout.target.resolve()
        previous = Path.cwd()
        os.chdir(self.root)
        self.addCleanup(os.chdir, previous)
        journal = self.root / "relative.jsonl"
        with patch.dict(os.environ, {
            "GITHUB_ACTIONS": "true",
            "RUSTUP_HOME": shared_rustup_home(),
            "CARGO_TARGET_DIR": target.name,
        }):
            os.environ.pop("RUSTUP_TOOLCHAIN", None)
            with patch(
                "tools.cargo_inputs_trace.supported_compiler",
                return_value=Path("/usr/bin/rustc"),
            ) as compiler:
                status = trace(workspace, target, journal, ["/bin/true"])
        detail = journal.read_text(encoding="utf-8") if journal.exists() else ""
        self.assertEqual(status, 0, detail)
        compiler.assert_called_once_with(workspace)

    def test_trace_accepts_an_existing_local_rustup_home(self) -> None:
        workspace = Path(__file__).resolve().parents[2]
        channel = toolchain_channel((workspace / "rust-toolchain.toml").read_text(encoding="utf-8"))
        home = self.root / "local-rustup"
        home.mkdir()
        link = self.root / "link-rustup"
        link.symlink_to(home, target_is_directory=True)
        target = self.layout.target.resolve()
        homes = {"directory": home, "symlink": link}
        spellings = {"unset": None, "true-word": "True", "one": "1"}
        for home_name, home_path in homes.items():
            for spelling, actions in spellings.items():
                with self.subTest(home=home_name, actions=spelling):
                    journal = self.root / f"local-{home_name}-{spelling}.jsonl"
                    updates = {
                        "RUSTUP_HOME": str(home_path),
                        "CARGO_TARGET_DIR": str(target),
                        "RUSTUP_TOOLCHAIN": channel,
                    }
                    if actions is not None:
                        updates["GITHUB_ACTIONS"] = actions
                    with patch.dict(os.environ, updates):
                        if actions is None:
                            os.environ.pop("GITHUB_ACTIONS", None)
                        with patch(
                            "tools.cargo_inputs_trace.supported_compiler",
                            return_value=Path("/usr/bin/rustc"),
                        ) as compiler:
                            status = trace(workspace, target, journal, ["/bin/true"])
                    detail = journal.read_text(encoding="utf-8") if journal.exists() else ""
                    self.assertEqual(status, 0, detail)
                    compiler.assert_called_once_with(workspace)

    def test_trace_rejects_a_foreign_home_on_github_actions(self) -> None:
        workspace = Path(__file__).resolve().parents[2]
        home = self.root / "local-rustup"
        home.mkdir()
        target = self.layout.target.resolve()
        journal = self.root / "foreign-home.jsonl"
        with patch.dict(os.environ, {
            "GITHUB_ACTIONS": "true",
            "RUSTUP_HOME": str(home),
            "CARGO_TARGET_DIR": str(target),
        }):
            os.environ.pop("RUSTUP_TOOLCHAIN", None)
            with patch(
                "tools.cargo_inputs_trace.supported_compiler",
                return_value=Path("/usr/bin/rustc"),
            ) as compiler:
                with self.assertRaises(TraceError) as caught:
                    trace(workspace, target, journal, ["/bin/true"])
                compiler.assert_not_called()
        self.assertEqual(str(caught.exception), UNSUPPORTED_COMPILER)
        self.assertFalse(journal.exists())

    def test_trace_rejects_a_missing_local_rustup_home(self) -> None:
        workspace = Path(__file__).resolve().parents[2]
        target = self.layout.target.resolve()
        (self.root / "not-dir").write_text("x", encoding="utf-8")
        cases = {
            "empty": {"RUSTUP_HOME": "", "CARGO_TARGET_DIR": str(target)},
            "missing": {"CARGO_TARGET_DIR": str(target)},
            "absent": {"RUSTUP_HOME": str(self.root / "missing"), "CARGO_TARGET_DIR": str(target)},
            "file": {"RUSTUP_HOME": str(self.root / "not-dir"), "CARGO_TARGET_DIR": str(target)},
            "empty-toolchain": {
                "RUSTUP_HOME": str(self.root / "local-rustup"),
                "CARGO_TARGET_DIR": str(target),
                "RUSTUP_TOOLCHAIN": "",
            },
        }
        (self.root / "local-rustup").mkdir()
        for name, updates in cases.items():
            with self.subTest(name=name):
                journal = self.root / f"local-reject-{name}.jsonl"
                with patch.dict(os.environ, updates):
                    os.environ.pop("GITHUB_ACTIONS", None)
                    if name != "empty-toolchain":
                        os.environ.pop("RUSTUP_TOOLCHAIN", None)
                    if name == "missing":
                        os.environ.pop("RUSTUP_HOME", None)
                    with patch(
                        "tools.cargo_inputs_trace.supported_compiler",
                        return_value=Path("/usr/bin/rustc"),
                    ) as compiler:
                        with self.assertRaises(TraceError) as caught:
                            trace(workspace, target, journal, ["/bin/true"])
                        compiler.assert_not_called()
                self.assertEqual(str(caught.exception), UNSUPPORTED_COMPILER)
                self.assertFalse(journal.exists())

    def assert_trace_layout_error(self, line: str, reason: str) -> None:
        output = io.StringIO()
        parser = Parser(self.workspace, self.layout.target, output, compiler=FIXTURE_COMPILER)
        parser.feed([line.encode()])
        events = [json.loads(item) for item in output.getvalue().splitlines()]
        self.assertEqual(events[-1]["errors"], 1)
        self.assertEqual(events[1]["reason"], reason)
        self.assertFalse(any(event["event"] == "unit" for event in events))

    def test_an_undeclared_read_inside_out_dir_stays_on_the_outer_run_unit(self) -> None:
        self.commit_demo_package()
        guest = self.workspace / "guest/src/main.rs"
        guest.parent.mkdir(parents=True)
        guest.write_text('fn main() { print!("guest-v1"); }\n')
        subprocess.run(["git", "add", "guest"], cwd=self.workspace, check=True)
        subprocess.run(["git", "commit", "-qm", "Add the guest source"], cwd=self.workspace, check=True)
        target = self.layout.target
        compile_dir = target / "debug/build/demo/aaaaaaaaaaaaaaaa/fingerprint"
        self.write_fingerprint(compile_dir, "build-script-build-script-build", 1)
        fingerprint = target / "debug/build/demo/0123456789abcdef/fingerprint"
        fingerprint.mkdir(parents=True)
        out = fingerprint.parent / "out"
        nested_out = out / "guest-target/debug/build/guest/aaaaaaaaaaaaaaaa/out"
        nested_out.mkdir(parents=True)
        stdout = fingerprint.parent / "run/stdout"
        stdout.parent.mkdir(parents=True)
        stdout.write_text("cargo:rerun-if-changed=build.rs\n")
        executable = compile_dir.parent / "out/build_script_build"
        session = self.root / "session-nested-read"
        c.begin(self.layout, session)
        with (session / "trace-nested-read.jsonl").open("w") as journal:
            parser = Parser(self.workspace, target, journal, compiler=FIXTURE_COMPILER)
            parser.feed([
                (
                    f'1 execve("{executable}", ["{executable}"], '
                    f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR={self.workspace}", "OUT_DIR={out}"]) = 0\n'
                ).encode(),
                b"1 clone(child_stack=NULL, flags=SIGCHLD) = 2\n",
                (
                    f'2 execve("/usr/bin/rustc", ["rustc", "--crate-name", "guest", "--out-dir", "{nested_out}", '
                    f'"-C", "extra-filename=-fedcba9876543210", "-C", "metadata=fedcba9876543210"], '
                    f'["CARGO_PKG_NAME=guest", "CARGO_MANIFEST_DIR={self.workspace / "guest"}"]) = 0\n'
                ).encode(),
                f'2 openat(AT_FDCWD, "{guest}", O_RDONLY) = 3<{guest}>\n'.encode(),
            ])
        with (session / "trace-nested-read.jsonl").open() as journal:
            events = [json.loads(line) for line in journal]
        units = [event for event in events if event["event"] == "unit"]
        key = fingerprint.relative_to(target).as_posix()
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual([(event["kind"], event["unit"]) for event in units], [("run", key)])
        self.assertFalse(any("guest-target" in key for key in c.inventory(self.layout)))
        self.finish_run_fingerprint(session, fingerprint, "0123456789abcdef")
        with self.assertRaisesRegex(c.InputError, "Undeclared input.*guest/src/main.rs"):
            c.save(self.layout, session, scrub=False)
        self.assertFalse((fingerprint / c.RECORD).is_file())

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
        fingerprint = target / "debug/build/demo/0123456789abcdef/fingerprint"
        fingerprint.mkdir(parents=True)
        compile_dir = target / "debug/build/demo/aaaaaaaaaaaaaaaa/fingerprint"
        self.write_fingerprint(compile_dir, "build-script-build-script-build", 1)
        executable = compile_dir.parent / "out/build_script_build"
        out = fingerprint.parent / "out"
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
            parser = Parser(self.workspace, self.layout.target, journal, compiler=FIXTURE_COMPILER)
            parser.feed([line.encode()])
        with (session / f"trace-{name}.jsonl").open() as journal:
            events = [json.loads(item) for item in journal]
        self.assertEqual(events[-1]["errors"], 0)
        self.assertEqual(sum(event["event"] == "unit" for event in events), 1)

    def touch_after_marker(self, session: Path, path: Path) -> None:
        marker_ns = json.loads((session / "session.json").read_text())["marker_ns"]
        os.utime(path, ns=(marker_ns + 2_000_000_000, marker_ns + 2_000_000_000))

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
        fingerprint = target / "debug/build/foo/0123456789abcdef/fingerprint"
        out = fingerprint.parent / "out"
        fingerprint.mkdir(parents=True, exist_ok=True)
        session = self.root / "session-compile-package"
        c.begin(self.layout, session)
        line = (
            f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{out}", '
            f'"-C", "metadata=aaaaaaaaaaaaaaaa"], ["CARGO_PKG_NAME=other", "CARGO_MANIFEST_DIR={self.workspace}"]) = 0\n'
        )
        self.write_rejected_trace(session, "compile-package", line, "target package foo")
        marker = fingerprint / "bin-demo"
        marker.write_text("0123456789abcdef")
        (fingerprint / "bin-demo.json").write_text('{"deps":[]}')
        (fingerprint / "dep-bin-demo").write_bytes(bytes.fromhex("01000000ff010000000000000000"))
        out.mkdir(parents=True, exist_ok=True)
        (out / "demo.d").write_text("out: src/lib.rs\n")
        self.touch_after_marker(session, marker)
        with self.assertRaisesRegex(c.InputError, "did not finish without errors"):
            c.save(self.layout, session, scrub=False)
        self.assertFalse((fingerprint / c.RECORD).is_file())

    def test_save_rejects_a_manifest_name_that_differs_from_the_unit_path(self) -> None:
        self.commit_demo_package()
        target = self.layout.target
        fingerprint = target / "debug/build/foo/0123456789abcdef/fingerprint"
        fingerprint.mkdir(parents=True)
        compile_dir = target / "debug/build/foo/aaaaaaaaaaaaaaaa/fingerprint"
        self.write_fingerprint(compile_dir, "build-script-build-script-build", 1)
        output = fingerprint.parent / "run/stdout"
        executable = compile_dir.parent / "out/build_script_build"
        out = fingerprint.parent / "out"
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text("cargo:rerun-if-changed=build.rs\n")
        session = self.root / "session-manifest"
        c.begin(self.layout, session)
        line = (
            f'1 execve("{executable}", ["{executable}"], '
            f'["CARGO_PKG_NAME=foo", "CARGO_MANIFEST_DIR={self.workspace}", "OUT_DIR={out}"]) = 0\n'
        )
        self.write_trace(session, "manifest", line)
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

    def test_an_edit_to_a_nested_source_invalidates_the_outer_run_unit(self) -> None:
        self.commit_demo_package()
        guest = self.workspace / "guest/src/main.rs"
        guest.parent.mkdir(parents=True)
        guest.write_text('fn main() { print!("guest-v1"); }\n')
        subprocess.run(["git", "add", "guest"], cwd=self.workspace, check=True)
        subprocess.run(["git", "commit", "-qm", "Add the guest source"], cwd=self.workspace, check=True)
        target = self.layout.target
        compile_dir = target / "debug/build/demo/aaaaaaaaaaaaaaaa/fingerprint"
        self.write_fingerprint(compile_dir, "build-script-build-script-build", 1)
        fingerprint = target / "debug/build/demo/0123456789abcdef/fingerprint"
        fingerprint.mkdir(parents=True)
        out = fingerprint.parent / "out"
        kept = out / "embedded.txt"
        kept.parent.mkdir(parents=True)
        kept.write_text("guest-v1\n")
        nested = out / "guest-target/debug/guest"
        nested.parent.mkdir(parents=True)
        nested.write_text("guest-binary")
        stdout = fingerprint.parent / "run/stdout"
        stdout.parent.mkdir(parents=True)
        stdout.write_text("cargo:rerun-if-changed=build.rs\ncargo:rerun-if-changed=guest/src/main.rs\n")
        executable = compile_dir.parent / "out/build_script_build"
        session = self.root / "session-nested-edit"
        c.begin(self.layout, session)
        with (session / "trace-nested-edit.jsonl").open("w") as journal:
            parser = Parser(self.workspace, target, journal, compiler=FIXTURE_COMPILER)
            parser.feed([
                (
                    f'1 execve("{executable}", ["{executable}"], '
                    f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR={self.workspace}", "OUT_DIR={out}"]) = 0\n'
                ).encode(),
                f'1 openat(AT_FDCWD, "{guest}", O_RDONLY) = 3<{guest}>\n'.encode(),
            ])
        self.finish_run_fingerprint(session, fingerprint, "0123456789abcdef")
        saved = c.save(self.layout, session, scrub=False)
        self.assertEqual(saved["recorded"], 1)
        self.assertTrue(kept.is_file())
        self.assertTrue(nested.is_file())
        guest.write_text('fn main() { print!("guest-v2"); }\n')
        stats = c.restore(self.layout, normalize=False)
        self.assertEqual(stats["invalidated"], 1)
        self.assertFalse(fingerprint.exists())
        self.assertFalse(kept.exists())

    def test_save_deletes_an_uncommitted_first_compile(self) -> None:
        self.commit_demo_package()
        target = self.layout.target
        fingerprint = target / "debug/build/demo/0123456789abcdef/fingerprint"
        out = fingerprint.parent / "out"
        artifact = out / "libdemo.rlib"
        fingerprint.mkdir(parents=True)
        (fingerprint / "invoked.timestamp").write_text("1")
        (fingerprint / "output-bin-demo").write_text("failed")
        artifact.parent.mkdir(parents=True, exist_ok=True)
        artifact.write_text("stale")
        session = self.root / "session-uncommitted"
        c.begin(self.layout, session)
        line = (
            f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", "{out}", '
            f'"-C", "metadata=aaaaaaaaaaaaaaaa"], ["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR={self.workspace}"]) = 0\n'
        )
        self.write_trace(session, "uncommitted", line)
        stats = c.save(self.layout, session, scrub=False)
        self.assertEqual(stats["recorded"], 0)
        self.assertFalse(fingerprint.exists())
        self.assertFalse(artifact.exists())

    def test_cache_commands_do_not_import_pyyaml(self) -> None:
        self.commit_demo_package()
        repository = Path(__file__).resolve().parents[2]
        session = self.root / "session-no-yaml"
        command = [
            sys.executable, "-S", "-m", "tools.cargo_inputs", "--workspace", str(self.workspace),
            "--target", str(self.layout.target), "--session", str(session),
        ]
        for mode in ("begin", "restore", "save"):
            with self.subTest(mode=mode):
                result = subprocess.run(command + [mode], cwd=repository, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_an_unknown_file_in_an_uncommitted_fingerprint_fails(self) -> None:
        directory = self.layout.target / "debug/build/demo/0123456789abcdef/fingerprint"
        directory.mkdir(parents=True)
        (directory / "invoked.timestamp").write_text("1")
        (directory / "notes").write_bytes(b"")
        with self.assertRaisesRegex(c.InputError, "Unknown fingerprint entry"):
            c.inventory(self.layout)

    def test_save_rejects_a_missing_manifest_outside_the_workspace(self) -> None:
        self.commit_demo_package()
        secret = self.workspace / "secret.txt"
        secret.write_text("secret\n")
        target = self.layout.target
        compile_dir = target / "debug/build/demo/aaaaaaaaaaaaaaaa/fingerprint"
        self.write_fingerprint(compile_dir, "build-script-build-script-build", 1)
        fingerprint = target / "debug/build/demo/0123456789abcdef/fingerprint"
        fingerprint.mkdir(parents=True)
        stdout = fingerprint.parent / "run/stdout"
        stdout.parent.mkdir(parents=True)
        stdout.write_text("cargo:rerun-if-changed=build.rs\n")
        executable = compile_dir.parent / "out/build_script_build"
        out = fingerprint.parent / "out"
        missing = self.root / "outside-missing-dir"
        session = self.root / "session-missing-manifest"
        c.begin(self.layout, session)
        with (session / "trace-missing-manifest.jsonl").open("w") as journal:
            parser = Parser(self.workspace, target, journal, compiler=FIXTURE_COMPILER)
            parser.feed([
                (
                    f'1 execve("{executable}", ["{executable}"], '
                    f'["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR={missing}", "OUT_DIR={out}"]) = 0\n'
                ).encode(),
                f'1 openat(AT_FDCWD, "{secret}", O_RDONLY) = 3<{secret}>\n'.encode(),
            ])
        self.finish_run_fingerprint(session, fingerprint, "0123456789abcdef")
        with self.assertRaisesRegex(c.InputError, r"^A unit manifest is unreadable:"):
            c.save(self.layout, session, scrub=False)
        self.assertFalse((fingerprint / c.RECORD).is_file())

    def test_manifest_package_name_reads_toml_forms(self) -> None:
        cases = {
            "spaced_header": '[ package ]\nname = "demo"\nversion = "0.1.0"\n',
            "dotted_key": 'package.name = "demo"\npackage.version = "0.1.0"\n',
            "multiline_brackets": '[package]\ndescription = """\n[notes]\n"""\nname = "demo"\nversion = "0.1.0"\n',
        }
        for name, text in cases.items():
            with self.subTest(name=name):
                path = self.root / f"{name}.toml"
                path.write_text(text)
                self.assertEqual(c.manifest_package_name(path), "demo")

    def test_trace_rejects_a_profile_that_is_the_target_root(self) -> None:
        target = self.layout.target
        (target / "build/demo/fedcba9876543210/fingerprint").mkdir(parents=True)
        self.assert_trace_layout_error(
            (
                f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "demo", "--out-dir", '
                f'"{target / "build/demo/fedcba9876543210/out"}", "-C", "metadata=fedcba9876543210"], '
                '["CARGO_PKG_NAME=demo", "CARGO_MANIFEST_DIR=/src/demo"]) = 0\n'
            ),
            "Unknown rustc unit layout",
        )

    def test_a_nested_rustc_without_an_outer_unit_fails_closed(self) -> None:
        target = self.layout.target
        out = target / "debug/build/demo/0123456789abcdef/out"
        nested_out = out / "guest-target/debug/build/guest/aaaaaaaaaaaaaaaa/out"
        nested_out.mkdir(parents=True)
        self.assert_trace_layout_error(
            (
                f'1 execve("/usr/bin/rustc", ["rustc", "--crate-name", "guest", "--out-dir", "{nested_out}", '
                '"-C", "extra-filename=-aaaaaaaaaaaaaaaa", "-C", "metadata=aaaaaaaaaaaaaaaa"], '
                '["CARGO_PKG_NAME=guest", "CARGO_MANIFEST_DIR=/src/guest"]) = 0\n'
            ),
            "A nested build has no outer unit",
        )

    def write_rejected_trace(self, session: Path, name: str, line: str, reason: str) -> None:
        with (session / f"trace-{name}.jsonl").open("w") as journal:
            parser = Parser(self.workspace, self.layout.target, journal, compiler=FIXTURE_COMPILER)
            parser.feed([line.encode()])
        with (session / f"trace-{name}.jsonl").open() as journal:
            events = [json.loads(item) for item in journal]
        self.assertGreater(events[-1]["errors"], 0)
        self.assertFalse(any(event["event"] == "unit" for event in events))
        self.assertIn(reason, next(event["reason"] for event in events if event["event"] == "trace-error"))


if __name__ == "__main__":
    unittest.main()
