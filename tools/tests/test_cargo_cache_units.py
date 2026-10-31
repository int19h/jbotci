"""Reject incomplete Cargo reports and remove unselected unit variants."""
from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from tools.cargo_cache import CacheError, prune, write_json


class CargoUnitTests(unittest.TestCase):
    def setUp(self) -> None:
        # These fixtures contain only small JSON and empty artifact files.
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "target"
        self.state = Path(self.temporary.name) / "state"
        self.state.mkdir()
        self.profile = self.root / "release"
        self.selected = self.profile / ".fingerprint/current-1111111111111111"
        self.selected.mkdir(parents=True)
        self.other = self.profile / ".fingerprint/other-2222222222222222"
        self.other.mkdir()
        for name in ("deps/libcurrent-1111111111111111.rlib", "deps/libother-2222222222222222.rlib",
                     "build/current-1111111111111111/output", "build/other-2222222222222222/output",
                     "incremental/current-session/part.o"):
            path = self.profile / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.touch()
        self.record = {"complete": True, "success": True, "target": str(self.root),
                       "units": [str(self.selected)], "aliases": []}

    def save_record(self) -> None:
        write_json(self.state / "invocation-test.json", self.record)

    def test_pruning_removes_all_unselected_unit_storage(self) -> None:
        self.save_record()
        result = prune(self.root, self.state)
        self.assertEqual(result["removed_units"], 1)
        self.assertTrue(self.selected.exists())
        self.assertFalse(self.other.exists())
        self.assertTrue((self.profile / "deps/libcurrent-1111111111111111.rlib").exists())
        self.assertFalse((self.profile / "deps/libother-2222222222222222.rlib").exists())
        self.assertFalse((self.profile / "build/other-2222222222222222").exists())
        self.assertFalse(list((self.profile / "incremental").iterdir()))

    def test_missing_invocation_reports_reject_the_save(self) -> None:
        with self.assertRaises(CacheError):
            prune(self.root, self.state)

    def test_incomplete_invocation_rejects_the_save(self) -> None:
        self.record["complete"] = False
        self.save_record()
        with self.assertRaises(CacheError):
            prune(self.root, self.state)

    def test_failed_root_build_rejects_the_save(self) -> None:
        self.record["success"] = False
        self.save_record()
        with self.assertRaises(CacheError):
            prune(self.root, self.state)

    def test_compiler_without_a_report_rejects_the_save(self) -> None:
        self.save_record()
        write_json(self.state / "rejected-bypass.json", {"reason": "unreported compiler"})
        with self.assertRaisesRegex(CacheError, "unreported compiler"):
            prune(self.root, self.state)

    def test_every_successful_invocation_contributes_units(self) -> None:
        self.save_record()
        write_json(self.state / "invocation-second.json", dict(self.record, units=[str(self.other)]))
        result = prune(self.root, self.state)
        self.assertEqual(result["selected_units"], 2)
        self.assertTrue(self.other.exists())

    def test_isolated_compile_fail_targets_are_discarded(self) -> None:
        self.save_record()
        isolated = self.root / "tests/trybuild"
        isolated.mkdir(parents=True)
        (isolated / "old-artifact").touch()
        write_json(self.state / "invocation-failed.json", dict(self.record, success=False, target=str(isolated)))
        prune(self.root, self.state)
        self.assertFalse(isolated.exists())

    def test_declared_uncached_install_target_is_reported_but_not_saved(self) -> None:
        self.save_record()
        outside = self.state / "installation-target"
        outside.mkdir()
        write_json(self.state / "environment.json", {"JBOTCI_CARGO_UNCACHED_TARGETS": json.dumps([str(outside)])})
        write_json(self.state / "invocation-install.json", dict(self.record, target=str(outside), units=[]))
        self.assertEqual(prune(self.root, self.state)["invocations"], 2)

    def test_undeclared_target_rejects_the_save(self) -> None:
        self.record["target"] = str(self.state)
        self.save_record()
        with self.assertRaisesRegex(CacheError, "undeclared"):
            prune(self.root, self.state)



if __name__ == "__main__":
    unittest.main()
