"""Exercise damaged records, directory membership, and declaration checks."""
from __future__ import annotations

import io
import json
import os
from pathlib import Path
import tempfile
import unittest

from tools import cargo_inputs as c
from tools.cargo_inputs_trace import Parser, Process, string_at


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


if __name__ == "__main__":
    unittest.main()
