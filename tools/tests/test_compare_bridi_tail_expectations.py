"""The comparer's transcribed bridi-tail shapes, checked against the grammar itself.

`compare-bridi-tail-expectations.py` rewrites the BASELINE tree, so every one of its five
classes is keyed on a baseline node's name and its exact baseline field tuple, and produces the
node HEAD spells.  Both sides are hand-transcriptions of the `rule … -> struct` bodies in
`crates/jbotci-syntax/src/grammar/generated.rs` — at `ARCHIVE_COMMIT` for the old side and at
HEAD for the new one.  A transcription that drifts from either grammar fails open in a way the
report cannot show: a class keyed on a field tuple the baseline never had simply never fires,
and the fixtures it should have classified land in manual residue looking like ordinary
population instead of like a broken comparer.  Epoch 6b lost 639 fixtures to exactly that, so
the transcriptions are re-derived here rather than asserted.

The second class of test covers the two checks no mechanical class inspects: the retired-shape
invariant on the regenerated tree, and the completeness check on epoch-new witnesses.
"""

from __future__ import annotations

import importlib.util
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).parents[2]
GRAMMAR_PATH = Path("crates/jbotci-syntax/src/grammar/generated.rs")

SCRIPT_PATH = Path(__file__).parents[1] / "compare-bridi-tail-expectations.py"
SPEC = importlib.util.spec_from_file_location("bridi_tail_comparator", SCRIPT_PATH)
assert SPEC is not None
assert SPEC.loader is not None
COMPARATOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = COMPARATOR
SPEC.loader.exec_module(COMPARATOR)


RULE = re.compile(r'^    rule "[^"]*" (\w+)\([^)]*\) -> struct \{\n(.*?)\n    \}', re.S | re.M)
# A gated field is a field exactly as an ungated one is: the Debug rendering carries it either
# way, so the field tuple the comparer matches has to see it.
FIELD = re.compile(r"^        (?:when feature\(\w+\) )?field (\w+) <-", re.M)


def _rule_name(struct: str) -> str:
    """The DSL's snake-case rule name for a Debug struct name."""
    return re.sub(r"(?<!^)(?=[A-Z])", "_", struct[: -len("Syntax")]).lower()


def _struct_fields(source: str) -> dict[str, tuple[str, ...]]:
    return {name: tuple(FIELD.findall(body)) for name, body in RULE.findall(source)}


def _grammar_at_head() -> str:
    return (REPO_ROOT / GRAMMAR_PATH).read_text(encoding="utf-8")


def _grammar_at_archive_commit() -> str:
    return subprocess.run(
        ["git", "show", f"{COMPARATOR.ARCHIVE_COMMIT}:{GRAMMAR_PATH.as_posix()}"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout


class TranscribedShapeTest(unittest.TestCase):
    def setUp(self) -> None:
        self.head = _struct_fields(_grammar_at_head())
        self.archive = _struct_fields(_grammar_at_archive_commit())

    def test_extraction_finds_the_productions_every_class_names(self) -> None:
        # A regex that stopped matching would make every assertion below vacuous.
        named = {COMPARATOR.COLLAPSE_STRUCT, COMPARATOR.KE_JOIN_POSITION[0]}
        named |= set(COMPARATOR.LEADING_CU_STRUCTS) | set(COMPARATOR.JOINT_CU_DROP_STRUCTS)
        named |= {parent for parent, _ in COMPARATOR.BO_JOINT_POSITIONS}
        for struct in sorted(named):
            with self.subTest(struct=struct):
                self.assertIn(_rule_name(struct), self.archive)
                self.assertIn(_rule_name(struct), self.head)

    def test_the_collapse_matches_both_grammars(self) -> None:
        # The class is licensed by the macro rendering a single-child product as a tuple: the
        # baseline has the two fields it names, and HEAD has exactly the one it keeps.
        self.assertEqual(
            self.archive[_rule_name(COMPARATOR.COLLAPSE_STRUCT)], COMPARATOR.COLLAPSE_OLD_FIELDS
        )
        self.assertEqual(
            self.head[_rule_name(COMPARATOR.COLLAPSE_STRUCT)],
            (COMPARATOR.COLLAPSE_PAYLOAD_FIELD,),
        )

    def test_the_leading_cu_field_is_inserted_at_the_head_of_the_tuple(self) -> None:
        for struct, old_fields in COMPARATOR.LEADING_CU_STRUCTS.items():
            with self.subTest(struct=struct):
                self.assertEqual(self.archive[_rule_name(struct)], old_fields)
                self.assertEqual(self.head[_rule_name(struct)], ("cu", *old_fields))

    def test_the_joints_lose_exactly_their_own_cu(self) -> None:
        for struct, old_fields in COMPARATOR.JOINT_CU_DROP_STRUCTS.items():
            with self.subTest(struct=struct):
                self.assertEqual(self.archive[_rule_name(struct)], old_fields)
                self.assertIn("cu", old_fields)
                self.assertEqual(
                    self.head[_rule_name(struct)], tuple(f for f in old_fields if f != "cu")
                )

    def test_the_bo_joint_wraps_a_product_the_baseline_grammar_spells(self) -> None:
        # Each wrap names the sourced product the BASELINE could put at that position and the
        # parent whose field holds it; a typo in either would stop classifying silently.
        for (parent, field), (arm, product) in COMPARATOR.BO_JOINT_POSITIONS.items():
            with self.subTest(position=(parent, field)):
                self.assertIn(field, self.archive[_rule_name(parent)])
                self.assertIn(_rule_name(product), self.archive)
                self.assertEqual(product[: -len("Syntax")], arm)

    def test_the_ke_join_narrows_to_a_production_that_predates_the_epoch(self) -> None:
        parent, field = COMPARATOR.KE_JOIN_POSITION
        self.assertIn(field, self.archive[_rule_name(parent)])
        self.assertEqual(
            self.archive[_rule_name(COMPARATOR.KE_JOIN_OLD_STRUCT)], COMPARATOR.KE_JOIN_FIELDS
        )
        # The GIhA-only join is not new: the with-tail-terms family already named it, which is
        # why the narrowing carries every field across and only drops the connective's wrapper.
        self.assertEqual(
            self.archive[_rule_name(COMPARATOR.KE_JOIN_NEW_STRUCT)], COMPARATOR.KE_JOIN_FIELDS
        )
        self.assertEqual(
            self.head[_rule_name(COMPARATOR.KE_JOIN_NEW_STRUCT)], COMPARATOR.KE_JOIN_FIELDS
        )

    def test_every_retired_shape_is_gone_from_the_head_grammar(self) -> None:
        # The invariant the comparer enforces on regenerated trees is only as good as its list:
        # a name still spelled at HEAD would make the check assert something already true.
        head = _grammar_at_head()
        for name in COMPARATOR.RETIRED_SHAPES:
            with self.subTest(name=name):
                self.assertIsNone(
                    re.search(rf'^    rule "[^"]*" {_rule_name(name + "Syntax")}[( ]', head, re.M)
                )


class RetiredShapeInvariantTest(unittest.TestCase):
    """The retired-shape check reads the regenerated TEXT, so it has to read identifiers."""

    def test_a_retired_node_is_reported_in_both_debug_renderings(self) -> None:
        for tree in (
            "Foo { bar: BridiWithPostCuTerms(BridiWithPostCuTermsSyntax { cu: None }) }",
            "Foo { bar: CuTermsBridiTailSyntax { cu: None } }",
        ):
            with self.subTest(tree=tree):
                with self.assertRaises(COMPARATOR.Divergence):
                    COMPARATOR.assert_no_retired_shapes(tree, "expectations.syntax.raw")

    def test_the_narrowings_own_output_is_not_the_shape_it_retires(self) -> None:
        # `GihekBridiTailKeContinuationSyntax` ENDS with the retired `BridiTailKeContinuationSyntax`.
        # A substring test reports the KE narrowing's own product as the node the epoch deletes,
        # which both hides the class and invents residue; the check matches whole identifiers.
        COMPARATOR.assert_no_retired_shapes(
            "Some(GihekBridiTailKeContinuationSyntax { connective: GihekConnectiveSyntax { } })",
            "expectations.syntax.raw",
        )


class EpochNewDiagnosticsPinTest(unittest.TestCase):
    """The completeness check on epoch-new witnesses, exercised on a synthetic tree.

    Nothing classifies an epoch-new witness, so the pin it carries is the whole audit.  This is
    the epoch that adds a warning to a pre-existing node, so a witness that omits the key would
    leave the construct free to stop warning with no expectation moving.
    """

    def _tree(self, fixtures: dict[str, str]) -> Path:
        root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, root)
        for name, contents in fixtures.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(contents, encoding="utf-8")
        return root

    def test_a_pinned_witness_is_accepted_whether_or_not_the_list_is_empty(self) -> None:
        root = self._tree(
            {
                "adhoc/warned.toml": 'id = "warned"\nlojban = "cu broda"\n\n'
                "[expectations.syntax]\n"
                'status = "success"\n'
                'raw = "tree"\n'
                'diagnostics = [{ severity = "warning", code = "x", byte-span = [0, 2],'
                ' source-text = "cu", message = "m" }]\n',
                "adhoc/silent.toml": 'id = "silent"\nlojban = "mi cu broda"\n\n'
                "[expectations.syntax]\n"
                'status = "success"\n'
                'raw = "tree"\n'
                "diagnostics = []\n",
            }
        )
        self.assertEqual(
            COMPARATOR.epoch_new_missing_diagnostics(
                root, ["tests/fixtures/adhoc/warned.toml", "tests/fixtures/adhoc/silent.toml"]
            ),
            [],
        )

    def test_an_unpinned_witness_is_reported_by_name(self) -> None:
        root = self._tree(
            {
                "adhoc/unpinned.toml": 'id = "unpinned"\nlojban = "mi broda gi\'e brode"\n\n'
                "[expectations.syntax]\n"
                'status = "success"\n'
                'raw = "tree"\n',
                "adhoc/facetless.toml": 'id = "facetless"\nlojban = "mi broda"\n\n'
                "[expectations.morphology]\n"
                'status = "success"\n',
            }
        )
        self.assertEqual(
            COMPARATOR.epoch_new_missing_diagnostics(
                root,
                ["tests/fixtures/adhoc/unpinned.toml", "tests/fixtures/adhoc/facetless.toml"],
            ),
            [
                "tests/fixtures/adhoc/facetless.toml: no expectations.syntax to pin diagnostics on",
                "tests/fixtures/adhoc/unpinned.toml: expectations.syntax pins no diagnostics list",
            ],
        )


if __name__ == "__main__":
    unittest.main()
