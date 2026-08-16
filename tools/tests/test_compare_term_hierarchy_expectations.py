"""The comparer's transcribed term-level inventories, checked against the grammar itself.

`compare-term-hierarchy-expectations.py` decides membership from `NEW_LEVEL_INVENTORY` and
`OLD_LEVEL_INVENTORY`, which are hand-transcriptions of the `rule "term" … -> enum` arm lists
in `crates/jbotci-syntax/src/grammar/generated.rs` — at HEAD for the new side and at
`ARCHIVE_COMMIT` (the epoch-6a merge, which is the baseline archive's commit) for the old.  A
transcription that drifts from the grammar silently weakens every mechanical class: a leaf the
comparer does not know about makes its fixture "not a member of the level" and drops it into
manual residue, and a leaf the comparer knows about but the grammar no longer spells would let
a stale shape pass as mechanical.

Epoch 6b's first comparer run failed in exactly the first way and put 639 of the 646 re-typed
fixtures into residue, so the transcription is re-derived here rather than asserted.  The
extraction is deliberately a regex over the DSL text and not an import of the grammar: the
point is to read the same source a human transcribes from.
"""

from __future__ import annotations

import importlib.util
import re
import subprocess
import sys
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
ENUM_ARM = re.compile(r"^        (\w+),\s*$", re.M)


def _product_name(arm: str) -> str:
    """The Debug struct name the DSL's snake-case arm produces."""
    return "".join(part.capitalize() for part in arm.split("_"))


def _term_level_inventories(source: str) -> dict[str, frozenset[str]]:
    return {
        name: frozenset(_product_name(arm) for arm in ENUM_ARM.findall(body))
        for name, body in TERM_LEVEL_RULE.findall(source)
    }


def _named_enum_arms(source: str, rule_name: str) -> frozenset[str]:
    match = re.search(
        r'^    rule "[^"]*" ' + re.escape(rule_name) + r"\([^)]*\) -> enum \{\n(.*?)\n    \}",
        source,
        re.S | re.M,
    )
    assert match is not None, f"no enum rule named {rule_name!r}"
    return frozenset(_product_name(arm) for arm in ENUM_ARM.findall(match.group(1)))


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
        self.assertEqual(
            set(self.archive),
            {"term", "cehe_term", "loose_term", "nonabs_term", "bound_term", "simple_term"},
        )

    def test_new_inventory_matches_the_grammar_arm_for_arm(self) -> None:
        for level, inventory in COMPARATOR.NEW_LEVEL_INVENTORY.items():
            if level == "<deleted>":
                continue
            with self.subTest(level=level):
                self.assertEqual(inventory, self.head[level])

    def test_old_inventory_matches_the_archive_commit_arm_for_arm(self) -> None:
        for level, inventory in COMPARATOR.OLD_LEVEL_INVENTORY.items():
            with self.subTest(level=level):
                if level == "relative_sumti":
                    self.assertEqual(
                        inventory, _named_enum_arms(_grammar_at_archive_commit(), level)
                    )
                    continue
                self.assertEqual(inventory, self.archive[level])

    def test_epoch_leaf_delta_is_exactly_the_two_termset_leaves(self) -> None:
        # What licenses writing the old inventory as the new one minus a pair rather than
        # transcribing it twice: across every level both grammars share, that pair is the
        # whole difference, in both directions.
        for level in sorted(self.archive):
            with self.subTest(level=level):
                self.assertEqual(
                    self.head[level] - self.archive[level], COMPARATOR.EPOCH_TERMSET_LEAVES
                )
                self.assertEqual(self.archive[level] - self.head[level], frozenset())

    def test_retired_payload_level_is_gone_from_the_new_grammar(self) -> None:
        self.assertIsNone(
            re.search(r'^    rule "[^"]*" relative_sumti\(', _grammar_at_head(), re.M)
        )
        self.assertEqual(
            COMPARATOR.RELATIVE_SUMTI_ARMS, set(COMPARATOR.OLD_LEVEL_INVENTORY["relative_sumti"])
        )

    def test_every_position_names_a_known_level_on_both_sides(self) -> None:
        for position, (old_level, new_level) in COMPARATOR.POSITIONS.items():
            with self.subTest(position=position):
                self.assertIn(old_level, COMPARATOR.OLD_LEVEL_INVENTORY)
                self.assertIn(new_level, COMPARATOR.NEW_LEVEL_INVENTORY)


if __name__ == "__main__":
    unittest.main()
