"""The comparer's transcribed term-level inventories, checked against the grammar itself.

`compare-term-hierarchy-expectations.py` decides membership from `NEW_LEVEL_INVENTORY` and
`OLD_LEVEL_INVENTORY`, which are hand-transcriptions of the `rule "term" … -> enum` arm lists
in `crates/jbotci-syntax/src/grammar/generated.rs` — at HEAD for the new side and at
`ARCHIVE_COMMIT` (the epoch-6b merge, which is the baseline archive's commit) for the old.  A
transcription that drifts from the grammar silently weakens every mechanical class: a leaf the
comparer does not know about makes its fixture "not a member of the level" and drops it into
manual residue, and a leaf the comparer knows about but the grammar no longer spells would let
a stale shape pass as mechanical.

Epoch 6b's first comparer run failed in exactly the first way and put 639 of the 646 re-typed
fixtures into residue, so the transcription is re-derived here rather than asserted.  The
extraction is deliberately a regex over the DSL text and not an import of the grammar: the
point is to read the same source a human transcribes from.

The second class of test here covers the comparer's completeness check on epoch-new witnesses,
which are the one population no mechanical class inspects.
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

SCRIPT_PATH = Path(__file__).parents[1] / "compare-term-hierarchy-expectations.py"
SPEC = importlib.util.spec_from_file_location("term_hierarchy_comparator", SCRIPT_PATH)
assert SPEC is not None
assert SPEC.loader is not None
COMPARATOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = COMPARATOR
SPEC.loader.exec_module(COMPARATOR)


TERM_LEVEL_RULE = re.compile(
    r'^    rule "term" (\w+)\([^)]*\) -> enum \{\n(.*?)\n    \}', re.S | re.M
)
# Gated arms carry a `when feature(X)` prefix; they are members of the level exactly as the
# ungated ones are, and the comparer's membership check has to see them.
ENUM_ARM = re.compile(r"^        (?:when feature\(\w+\) )?(\w+),\s*$", re.M)


def _rule_name(struct: str) -> str:
    """The DSL's snake-case rule name for a Debug struct name."""
    return re.sub(r"(?<!^)(?=[A-Z])", "_", struct[: -len("Syntax")]).lower()


def _product_name(arm: str) -> str:
    """The Debug struct name the DSL's snake-case arm produces."""
    return "".join(part.capitalize() for part in arm.split("_"))


def _term_level_inventories(source: str) -> dict[str, frozenset[str]]:
    return {
        name: frozenset(_product_name(arm) for arm in ENUM_ARM.findall(body))
        for name, body in TERM_LEVEL_RULE.findall(source)
    }


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


class TermLevelInventoryTest(unittest.TestCase):
    def setUp(self) -> None:
        self.head = _term_level_inventories(_grammar_at_head())
        self.archive = _term_level_inventories(_grammar_at_archive_commit())

    def test_extraction_finds_the_whole_ladder(self) -> None:
        # A regex that stopped matching would make every assertion below vacuous.
        self.assertEqual(
            set(self.head),
            {
                "term",
                "cehe_term",
                "loose_term",
                "nonabs_term",
                "bound_term",
                "simple_term",
                "normal_term",
                "bound_normal_term",
                "normal_term_atom",
            },
        )
        self.assertEqual(set(self.archive), set(self.head))

    def test_new_inventory_matches_the_grammar_arm_for_arm(self) -> None:
        for level, inventory in COMPARATOR.NEW_LEVEL_INVENTORY.items():
            if level == "<deleted>":
                continue
            with self.subTest(level=level):
                self.assertEqual(inventory, self.head[level])

    def test_old_inventory_matches_the_archive_commit_arm_for_arm(self) -> None:
        for level, inventory in COMPARATOR.OLD_LEVEL_INVENTORY.items():
            if level == "<deleted>":
                continue
            with self.subTest(level=level):
                self.assertEqual(inventory, self.archive[level])

    def test_epoch_leaf_delta_is_exactly_the_one_added_leaf(self) -> None:
        # What licenses writing the old inventory as the new one minus a single name rather
        # than transcribing it twice: across every level, that name is the whole difference,
        # in both directions.
        for level in sorted(self.archive):
            with self.subTest(level=level):
                self.assertEqual(
                    self.head[level] - self.archive[level], COMPARATOR.EPOCH_LEAF_DELTA
                )
                self.assertEqual(self.archive[level] - self.head[level], frozenset())

    def test_retired_payload_level_is_gone_from_both_grammars(self) -> None:
        # Epoch 6b retired the narrow GOI payload node, so it is absent from the baseline
        # archive too and there is nothing left for its class to classify.
        for source in (_grammar_at_head(), _grammar_at_archive_commit()):
            self.assertIsNone(re.search(r'^    rule "[^"]*" relative_sumti\(', source, re.M))
        self.assertNotIn("relative_sumti", COMPARATOR.OLD_LEVEL_INVENTORY)

    def test_sum_wrapper_positions_name_rules_the_archive_grammar_has(self) -> None:
        # Each wrap names the sourced product the BASELINE grammar could put at that position,
        # and the parent whose field holds it; a typo in either would silently stop classifying
        # and inflate manual residue instead of failing.
        archive = _grammar_at_archive_commit()
        for (parent, field), (arm, product, _class) in COMPARATOR.SUM_WRAPPER_POSITIONS.items():
            with self.subTest(position=(parent, field)):
                for struct in (parent, product):
                    self.assertIsNotNone(
                        re.search(
                            r'^    rule "[^"]*" ' + _rule_name(struct) + r"[( ]", archive, re.M
                        ),
                        f"{struct} has no rule in the archive grammar",
                    )
                self.assertEqual(_product_name(_rule_name(product)) + "Syntax", product)
                self.assertEqual(_product_name(_rule_name(product)), arm)

    def test_every_position_names_a_known_level_on_both_sides(self) -> None:
        for position, (old_level, new_level) in COMPARATOR.POSITIONS.items():
            with self.subTest(position=position):
                self.assertIn(old_level, COMPARATOR.OLD_LEVEL_INVENTORY)
                self.assertIn(new_level, COMPARATOR.NEW_LEVEL_INVENTORY)


class EpochNewDiagnosticsPinTest(unittest.TestCase):
    """The completeness check on epoch-new witnesses, exercised on a synthetic tree.

    Nothing classifies an epoch-new witness, so the pin it carries is the whole audit; the
    check is run against fixtures written here rather than against the repository's, so both
    the accept and the reject side are exercised whatever the tree happens to hold.
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
                "adhoc/warned.toml": 'id = "warned"\nlojban = "ko\'a bo ko\'e broda"\n\n'
                "[expectations.syntax]\n"
                'status = "success"\n'
                'raw = "tree"\n'
                'diagnostics = [{ severity = "warning", code = "x", byte-span = [0, 2],'
                ' source-text = "ko\'a", message = "m" }]\n',
                "adhoc/silent.toml": 'id = "silent"\nlojban = "ko\'a broda"\n\n'
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

    def test_an_unpinned_witness_is_reported(self) -> None:
        root = self._tree(
            {
                "adhoc/unpinned.toml": 'id = "unpinned"\nlojban = "ko\'a bo ko\'e broda"\n\n'
                "[expectations.syntax]\n"
                'status = "success"\n'
                'raw = "tree"\n',
            }
        )
        self.assertEqual(
            COMPARATOR.epoch_new_missing_diagnostics(root, ["tests/fixtures/adhoc/unpinned.toml"]),
            ["tests/fixtures/adhoc/unpinned.toml: expectations.syntax pins no diagnostics list"],
        )

    def test_a_witness_without_a_syntax_expectation_is_reported(self) -> None:
        # Reported rather than skipped: a fixture that pins no syntax at all cannot carry the
        # key, and that exception has to be argued in this function rather than fall out of a
        # lookup that quietly finds nothing.
        root = self._tree(
            {
                "adhoc/morphology-only.toml": 'id = "morphology-only"\nlojban = "ko\'a"\n\n'
                "[expectations.morphology]\n"
                'status = "success"\n',
            }
        )
        self.assertEqual(
            COMPARATOR.epoch_new_missing_diagnostics(
                root, ["tests/fixtures/adhoc/morphology-only.toml"]
            ),
            [
                "tests/fixtures/adhoc/morphology-only.toml: "
                "no expectations.syntax to pin diagnostics on"
            ],
        )

    def test_every_epoch_new_witness_in_the_repository_is_pinned(self) -> None:
        # The check above is only worth having if it is also true of the tree it guards.
        witnesses = subprocess.run(
            [
                "git",
                "diff",
                "--diff-filter=A",
                "--name-only",
                f"{COMPARATOR.EPOCH_BASE}..HEAD",
                "--",
                "tests/fixtures",
            ],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            check=True,
        ).stdout.split()
        self.assertEqual(len(witnesses), COMPARATOR.EXPECTED_NEW_WITNESSES)
        self.assertEqual(
            COMPARATOR.epoch_new_missing_diagnostics(REPO_ROOT / "tests/fixtures", witnesses), []
        )


if __name__ == "__main__":
    unittest.main()
