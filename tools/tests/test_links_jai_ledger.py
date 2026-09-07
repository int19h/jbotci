"""Synthetic transition data; these tests make no corpus/parser-winner claims."""

import copy
import csv
from dataclasses import replace
import io
from pathlib import Path
import tempfile
import unittest

from tools import links_jai_ledger as ledger


class LedgerTests(unittest.TestCase):
    def setUp(self):
        self.identity = ledger.Identity(
            id="synthetic.empty", path="synthetic.toml", source="mi broda be be'o",
            dialect=None, target={"marker": "be", "byte_start": 9, "byte_end": 11},
            refs=False, gentufa_tree=True, gentufa_json=True,
        )

    def observation(self, stage="base", *, full=False, failure=False):
        identity = self.identity
        observation = {
            "stage": stage, "id": identity.id, "path": identity.path,
            "source": identity.source, "dialect": identity.dialect,
            "target": dict(identity.target),
            "syntax": {
                "status": "success", "raw": "synthetic Full tree" if full else "synthetic Empty tree",
                "target": {"status": "found", "link": {
                    "anchor": dict(identity.target),
                    "owner": "full-linked-term" if full else "empty-linked-sumti",
                }}, "diagnostics": [],
            },
            "refs": {"status": "inapplicable"},
            "gentufa_tree": {"status": "success", "value": "synthetic rendered tree"},
            "gentufa_json": {"status": "success", "value": "synthetic rendered JSON"},
        }
        if failure:
            observation["syntax"] = {
                "status": "failure", "error": "synthetic strict error",
                "diagnostics": [{"severity": "error", "code": "syntax.unexpected-cmavo",
                                 "byte-span": [9, 11], "source-text": "be"}],
            }
            observation["gentufa_tree"] = {"status": "blocked-by-syntax-failure"}
            observation["gentufa_json"] = {"status": "blocked-by-syntax-failure"}
        return observation

    def snap(self, observation):
        return ledger.snapshot(self.identity, observation, observation["stage"])

    def disposition(self, before, after, kind):
        return {"kind": kind, "reviewed_surfaces": sorted(ledger.changed_surfaces(before, after)),
                "reason": "synthetic transition-predicate test, not a reviewed corpus result"}

    def test_empty_preservation_then_actual_failure(self):
        base = self.snap(self.observation())
        ca = self.snap(self.observation("c-a"))
        cb = self.snap(self.observation("c-b", failure=True))
        self.assertFalse(ledger.transition(base, ca, "c-a", self.disposition(base, ca, "unchanged")))
        self.assertTrue(ledger.transition(ca, cb, "c-b", self.disposition(ca, cb, "empty-route-removed")))
        self.assertEqual(cb.error_count, 1)
        self.assertIsNone(cb.owner)
        self.assertEqual(cb.values["gentufa_tree_sha256"], ledger.INAPPLICABLE)

    def test_full_reownership_then_exact_preservation(self):
        base = self.snap(self.observation())
        ca = self.snap(self.observation("c-a", full=True))
        cb = self.snap(self.observation("c-b", full=True))
        self.assertEqual(ledger.transition(base, ca, "c-a", self.disposition(base, ca, "d1-reowner-preserved")),
                         {"owner", "syntax_raw_full_tree_sha256"})
        self.assertFalse(ledger.transition(ca, cb, "c-b", self.disposition(ca, cb, "reowner-preserved")))

    def test_identity_changes_fail(self):
        for field, value in [("id", "different"), ("path", "elsewhere.toml"),
                             ("source", "mi brode be be'o"), ("dialect", "(zantufa)"),
                             ("target", {"marker": "bei", "byte_start": 9, "byte_end": 11})]:
            with self.subTest(field=field):
                observation = self.observation()
                observation[field] = value
                with self.assertRaises(ledger.ValidationError):
                    self.snap(observation)

    def test_failure_needs_real_error_and_no_strict_tree(self):
        for change in ["no-diagnostics", "warning-only", "fake-tree", "fake-target", "missing-error"]:
            with self.subTest(change=change):
                observation = self.observation("c-b", failure=True)
                syntax = observation["syntax"]
                if change == "no-diagnostics":
                    syntax["diagnostics"] = []
                elif change == "warning-only":
                    syntax["diagnostics"][0]["severity"] = "warning"
                elif change == "fake-tree":
                    syntax["raw"] = "manufactured strict tree"
                elif change == "fake-target":
                    syntax["target"] = {"status": "missing"}
                else:
                    syntax["error"] = ""
                with self.assertRaises(ledger.ValidationError):
                    self.snap(observation)

    def test_failure_blocks_every_applicable_output(self):
        for field in ["gentufa_tree", "gentufa_json"]:
            for status in ["inapplicable", "success"]:
                with self.subTest(field=field, status=status):
                    observation = self.observation("c-b", failure=True)
                    observation[field] = {"status": status}
                    with self.assertRaises(ledger.ValidationError):
                        self.snap(observation)

    def test_missing_and_ambiguous_targets_fail(self):
        for target in [{"status": "missing"}, {"status": "ambiguous", "matches": []}]:
            observation = self.observation("c-a")
            observation["syntax"]["target"] = target
            with self.assertRaises(ledger.ValidationError):
                self.snap(observation)

    def test_owner_change_without_tree_change_fails(self):
        base = self.snap(self.observation())
        observation = self.observation("c-a", full=True)
        observation["syntax"]["raw"] = self.observation()["syntax"]["raw"]
        ca = self.snap(observation)
        with self.assertRaisesRegex(ledger.ValidationError, "without a changed strict tree"):
            ledger.transition(base, ca, "c-a", self.disposition(base, ca, "d1-reowner-preserved"))

    def test_c_a_rejection_is_not_authorized(self):
        base = self.snap(self.observation())
        ca = self.snap(self.observation("c-a", failure=True))
        with self.assertRaisesRegex(ledger.ValidationError, "C-a rejection"):
            ledger.transition(base, ca, "c-a", self.disposition(base, ca, "empty-route-removed"))

    def test_manual_labels_cannot_hide_surfaces_or_owners(self):
        base = self.snap(self.observation())
        ca = self.snap(self.observation("c-a", full=True))
        disposition = self.disposition(base, ca, "d1-reowner-preserved")
        disposition["reviewed_surfaces"] = ["owner"]
        with self.assertRaises(ledger.ValidationError):
            ledger.transition(base, ca, "c-a", disposition)
        observation = self.observation("c-a")
        observation["syntax"]["target"]["link"]["owner"] = "plain-linked-sumti"
        other = self.snap(observation)
        with self.assertRaises(ledger.ValidationError):
            ledger.transition(base, other, "c-a", self.disposition(base, other, "manual"))

    def test_full_c_b_changes_fail(self):
        ca = self.snap(self.observation("c-a", full=True))
        cb = self.snap(self.observation("c-b", failure=True))
        with self.assertRaises(ledger.ValidationError):
            ledger.transition(ca, cb, "c-b", self.disposition(ca, cb, "empty-route-removed"))

    def test_population_is_bidirectional_and_stage_exact(self):
        identities = {self.identity.id: self.identity}
        observation = self.observation("c-a")
        self.assertEqual(set(ledger.population(identities, [observation], "c-a")), set(identities))
        for rows in [[], [observation, copy.deepcopy(observation)], [self.observation("base")]]:
            with self.assertRaises(ledger.ValidationError):
                ledger.population(identities, rows, "c-a")

    def test_diagnostic_counts_are_derived_from_arrays(self):
        observation = self.observation("c-b", failure=True)
        observation["syntax"]["error_count"] = 99
        with self.assertRaises(ledger.ValidationError):
            self.snap(observation)

    def test_c_b_cannot_skip_a_measured_c_a_predecessor(self):
        base = self.snap(self.observation())
        cb = self.snap(self.observation("c-b", failure=True))
        with self.assertRaisesRegex(ledger.ValidationError, "measured c-a predecessor"):
            ledger.transition(base, cb, "c-b", self.disposition(base, cb, "empty-route-removed"))

    def base_data(self):
        return {"semantic_base": "a" * 40, "rows": [{
            "id": self.identity.id, "path": self.identity.path, "dialect": None,
            "target": dict(self.identity.target),
            "coverage": {"refs": False, "gentufa_tree": True, "gentufa_json": True},
            "base": dict(self.snap(self.observation()).values),
        }]}

    def fixture_root(self):
        temporary = tempfile.TemporaryDirectory(prefix="links-jai-unit-")
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        (root / self.identity.path).write_text(
            'id = "synthetic.empty"\nlojban = "mi broda be be\'o"\n'
            '[expectations.syntax]\nstatus = "success"\n', encoding="utf-8")
        return root

    def test_base_loading_preserves_missing_arrays(self):
        identities, snapshots = ledger.load_base(self.base_data(), self.fixture_root())
        self.assertEqual(identities, {self.identity.id: self.identity})
        base = snapshots[self.identity.id]
        self.assertIsNone(base.diagnostics)
        with self.assertRaisesRegex(ledger.ValidationError, "actual array"):
            _ = base.error_count

    def test_expectation_edits_do_not_change_fixture_identity(self):
        root = self.fixture_root()
        identities, _ = ledger.load_base(self.base_data(), root)
        path = root / self.identity.path
        path.write_text(path.read_text().replace('status = "success"', 'status = "xfail"'), encoding="utf-8")
        identities[self.identity.id].check_fixture(root)

    def test_file_backed_source_identity_uses_source_bytes(self):
        root = self.fixture_root()
        path = root / self.identity.path
        path.write_text('id = "synthetic.empty"\nlojban-filename = "source.lojban"\n', encoding="utf-8")
        source = root / "source.lojban"
        source.write_text(self.identity.source, encoding="utf-8")
        identities, _ = ledger.load_base(self.base_data(), root)
        identities[self.identity.id].check_fixture(root)
        source.write_text(self.identity.source + "\n", encoding="utf-8")
        with self.assertRaisesRegex(ledger.ValidationError, "changed fixture source"):
            identities[self.identity.id].check_fixture(root)

    def test_source_id_and_dialect_edits_fail_fixture_identity(self):
        root = self.fixture_root()
        path = root / self.identity.path
        original = path.read_text()
        for changed in [original.replace("synthetic.empty", "synthetic.other"),
                        original.replace("broda", "brode"),
                        'dialect = "(zantufa)"\n' + original]:
            with self.subTest(changed=changed):
                path.write_text(changed, encoding="utf-8")
                with self.assertRaises(ledger.ValidationError):
                    self.identity.check_fixture(root)

    def test_base_data_rejects_lost_surfaces_and_duplicate_rows(self):
        root = self.fixture_root()
        for mutation in ["duplicate", "lost-surface", "changed-coverage", "unprobed", "row-width", "empty"]:
            with self.subTest(mutation=mutation):
                data = self.base_data()
                row = data["rows"][0]
                if mutation == "duplicate":
                    data["rows"].append(copy.deepcopy(row))
                elif mutation == "lost-surface":
                    del row["base"]["syntax_diagnostics_sha256"]
                elif mutation == "changed-coverage":
                    row["coverage"]["gentufa_tree"] = False
                elif mutation == "unprobed":
                    row["base"]["syntax_raw_full_tree_sha256"] = ledger.UNPROBED
                elif mutation == "row-width":
                    row["source_sha256"] = "not an identity field"
                else:
                    data["rows"] = []
                with self.assertRaises(ledger.ValidationError):
                    ledger.load_base(data, root)

    def test_malformed_or_missing_observation_objects_fail(self):
        for field in ["syntax", "refs", "gentufa_tree", "gentufa_json"]:
            for value in [None, [], "missing"]:
                with self.subTest(field=field, value=value):
                    observation = self.observation()
                    observation[field] = value
                    with self.assertRaises(ledger.ValidationError):
                        self.snap(observation)
        with self.assertRaises(ledger.ValidationError):
            ledger.population({self.identity.id: self.identity}, [None], "c-a")

    def test_report_has_32_columns_and_only_future_stage_unprobed(self):
        identities = {self.identity.id: self.identity}
        base = {self.identity.id: self.snap(self.observation())}
        ca = self.snap(self.observation("c-a"))
        dispositions = {"c-a": {self.identity.id: self.disposition(base[self.identity.id], ca, "unchanged")}}
        observations = {"c-a": [self.observation("c-a")]}
        rows = ledger.stage_rows(identities, base, observations, dispositions, "c-a")
        parsed = list(csv.reader(io.StringIO(ledger.render_tsv(rows)), delimiter="\t"))
        self.assertEqual([len(row) for row in parsed], [32, 32])
        self.assertNotIn(ledger.UNPROBED, parsed[1][:21])
        self.assertEqual(parsed[1][21:30], [ledger.UNPROBED] * 9)
        self.assertEqual(parsed[1][31], ledger.UNPROBED)
        with self.assertRaises(ledger.ValidationError):
            ledger.render_tsv([rows[0][:-1]])

    def test_stage_report_needs_each_predecessor_and_every_disposition(self):
        identities = {self.identity.id: self.identity}
        base = {self.identity.id: self.snap(self.observation())}
        ca = self.snap(self.observation("c-a"))
        good_dispositions = {"c-a": {self.identity.id: self.disposition(base[self.identity.id], ca, "unchanged")}}
        for observations, dispositions, stage in [
            ({"c-a": [self.observation("c-a")]}, good_dispositions, "c-b"),
            ({"c-a": []}, good_dispositions, "c-a"),
            ({"c-a": [self.observation("c-a")]}, {"c-a": {}}, "c-a"),
            ({"c-a": [self.observation("c-a")], "c-b": []}, good_dispositions, "c-a"),
        ]:
            with self.subTest(stage=stage, observations=observations, dispositions=dispositions):
                with self.assertRaises(ledger.ValidationError):
                    ledger.stage_rows(identities, base, observations, dispositions, stage)

    def test_alice_cannot_use_the_generic_full_reowner_disposition(self):
        self.identity = replace(self.identity, id=ledger.ALICE_ID)
        base, ca = self.snap(self.observation()), self.snap(self.observation("c-a", full=True))
        dispositions = {"c-a": {ledger.ALICE_ID: self.disposition(base, ca, "d1-reowner-preserved")}}
        with self.assertRaisesRegex(ledger.ValidationError, "Alice C-a reownership"):
            ledger.stage_rows({ledger.ALICE_ID: self.identity}, {ledger.ALICE_ID: base},
                              {"c-a": [self.observation("c-a", full=True)]}, dispositions, "c-a")

    def test_alice_removal_requires_the_independent_full_recovery_capture(self):
        self.identity = replace(self.identity, id=ledger.ALICE_ID)
        base, ca = self.snap(self.observation()), self.snap(self.observation("c-a"))
        cb_observation = self.observation("c-b", failure=True)
        cb = self.snap(cb_observation)
        dispositions = {
            "c-a": {ledger.ALICE_ID: self.disposition(base, ca, "unchanged")},
            "c-b": {ledger.ALICE_ID: self.disposition(ca, cb, "empty-route-removed")},
        }
        observations = {"c-a": [self.observation("c-a")], "c-b": [cb_observation]}
        identities, baseline = {ledger.ALICE_ID: self.identity}, {ledger.ALICE_ID: base}
        with self.assertRaisesRegex(ledger.ValidationError, "Alice recovery capture"):
            ledger.stage_rows(identities, baseline, observations, dispositions, "c-b")
        cb_observation["recovered"] = {"status": "success", "value": {
            "parser_status": "failure", "raw": "synthetic recovered full tree",
            "diagnostics": copy.deepcopy(cb_observation["syntax"]["diagnostics"]),
            "errors": ["synthetic error"], "tree": {"valid-tokens": [], "recovery-items": []},
            "target": {"status": "missing"},
        }}
        self.assertEqual(len(ledger.stage_rows(identities, baseline, observations, dispositions, "c-b")), 1)
        for missing in ["raw", "diagnostics", "tree", "target", "errors", "parser_status"]:
            with self.subTest(missing=missing):
                changed = copy.deepcopy(observations)
                del changed["c-b"][0]["recovered"]["value"][missing]
                with self.assertRaises(ledger.ValidationError):
                    ledger.stage_rows(identities, baseline, changed, dispositions, "c-b")

    def test_malformed_diagnostics_and_unhashable_ids_or_owners_fail_cleanly(self):
        for changes in [{"severity": []}, {"severity": "unknown"}, {"code": ""},
                        {"byte-span": [False, 2]}, {"source-text": "wrong"}]:
            with self.subTest(changes=changes):
                observation = self.observation("c-b", failure=True)
                observation["syntax"]["diagnostics"][0].update(changes)
                with self.assertRaises(ledger.ValidationError):
                    self.snap(observation)
        observation = self.observation()
        observation["id"] = []
        with self.assertRaises(ledger.ValidationError):
            ledger.population({self.identity.id: self.identity}, [observation], "base")
        observation = self.observation()
        observation["syntax"]["target"]["link"]["owner"] = {}
        with self.assertRaises(ledger.ValidationError):
            self.snap(observation)


if __name__ == "__main__":
    unittest.main()
