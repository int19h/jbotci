"""Exercise damaged records, directory membership, and declaration checks."""
from __future__ import annotations

from contextlib import redirect_stderr
import io
import json
import os
from pathlib import Path
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


if __name__ == "__main__":
    unittest.main()
