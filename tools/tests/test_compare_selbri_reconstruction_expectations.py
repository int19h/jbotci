from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


SCRIPT_PATH = (
    Path(__file__).parents[1] / "compare-selbri-reconstruction-expectations.py"
)
SPEC = importlib.util.spec_from_file_location("selbri_reconstruction_comparator", SCRIPT_PATH)
assert SPEC is not None
assert SPEC.loader is not None
COMPARATOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = COMPARATOR
SPEC.loader.exec_module(COMPARATOR)


def form(text: str):  # type: ignore[no-untyped-def]
    return COMPARATOR.DebugParser(text).parse()


def linked(word: str, start: int) -> str:
    end = start + len(word)
    return f"""LinkedTanruUnitSyntax {{
        base: TanruUnitAtomSyntax {{ conversions: [], base: WordTanruUnit(
            WordTanruUnitSyntax(WithFreeModifiers {{ value: Plain(PlainWord(Gismu {{
                phonemes: Phonemes {{ text: \"{word}\" }},
                span: SourceSpan {{ source_id: None, byte_start: {start}, byte_end: {end},
                    char_start: {start}, char_end: {end}, start: None, end: None }}
            }})), free_modifiers: [] }})
        ) }}, linkargs: None
    }}"""


def cmavo(word: str, start: int) -> str:
    end = start + len(word)
    return f"""WithFreeModifiers {{ value: Plain(PlainWord(Cmavo {{
        phonemes: Phonemes {{ text: \"{word}\" }},
        span: SourceSpan {{ source_id: None, byte_start: {start}, byte_end: {end},
            char_start: {start}, char_end: {end}, start: None, end: None }}
    }})), free_modifiers: [] }}"""


def old_simple_unit(payload: str) -> str:
    return f"TanruUnitSyntax(Chain {{ first: LinkedTanruUnit({payload}), links: [] }})"


def new_simple_bound(payload: str) -> str:
    return f"""BoundSelbriSyntax {{
        leading_selbri: PlainBoTanruUnit(PlainBoTanruUnitSyntax {{
            leading_unit: TanruUnitSyntax {{ base: {payload}, assignments: [] }},
            bo_tail: None
        }}), bo_tail: None
    }}"""


def old_root(first: str, additional: str = "[]") -> str:
    return f"""CoSelbriSyntax {{ leading_selbri: ConnectedSelbriSyntax {{
        leading_selbri: TanruSelbriSyntax {{ first_unit: {first},
            additional_units: {additional} }}, continuations: []
    }}, co_tail: None }}"""


def new_root(first: str, additional: str = "[]") -> str:
    return f"""CoSelbriSyntax {{ leading_selbri: TanruSelbriSyntax {{
        first_selbri: ConnectedSelbriSyntax {{ leading_selbri: {first}, continuations: [] }},
        additional_selbri: {additional}
    }}, co_tail: None }}"""


class SelbriReconstructionComparatorTests(unittest.TestCase):
    def assert_class(self, old: str, new: str, expected: str) -> None:
        classes: set[str] = set()
        self.assertTrue(COMPARATOR.equivalent_forms(form(old), form(new), classes))
        self.assertEqual(classes, {expected})

    def test_single_unit_wrapper(self) -> None:
        broda = linked("broda", 0)
        self.assert_class(
            old_root(old_simple_unit(broda)),
            new_root(new_simple_bound(broda)),
            "single-unit-wrapper",
        )

    def test_pure_adjacency(self) -> None:
        broda = linked("broda", 0)
        brode = linked("brode", 6)
        self.assert_class(
            old_root(old_simple_unit(broda), f"[{old_simple_unit(brode)}]"),
            new_root(
                new_simple_bound(broda),
                f"[ConnectedSelbriSyntax {{ leading_selbri: {new_simple_bound(brode)}, continuations: [] }}]",
            ),
            "pure-adjacency",
        )

    def test_actual_old_legacy_connective_level(self) -> None:
        broda = linked("broda", 0)
        brode = linked("brode", 9)
        je = cmavo("je", 6)
        connector = f"JekConnective(JekConnectiveSyntax {{ na: None, se: None, ja: {je}, nai: None }})"
        old_unit = f"""TanruUnitSyntax(Chain {{ first: LinkedTanruUnit({broda}), links: [
            TanruUnitContinuationSyntax {{ connective: {connector},
                trailing_unit: LinkedTanruUnit({brode}) }}
        ] }})"""
        new = f"""CoSelbriSyntax {{ leading_selbri: TanruSelbriSyntax {{
            first_selbri: ConnectedSelbriSyntax {{
                leading_selbri: {new_simple_bound(broda)},
                continuations: [SimpleConnectedSelbriContinuation(
                    SimpleConnectedSelbriContinuationSyntax {{ connective: {connector},
                        trailing_selbri: {new_simple_bound(brode)} }}
                )]
            }}, additional_selbri: []
        }}, co_tail: None }}"""
        self.assert_class(old_root(old_unit), new, "pure-joik-jek")

    def test_plain_bo_must_be_connectiveless_and_stagless(self) -> None:
        broda = linked("broda", 0)
        brode = linked("brode", 9)
        bo = cmavo("bo", 6)
        old_unit = f"""TanruUnitSyntax(Chain {{ first: BoundTanruUnit(
            BoundTanruUnitSyntax {{ leading_unit: {broda}, bo_connective: None,
                bo_tense_modal: None, bo: {bo}, trailing_unit: LinkedTanruUnit({brode}) }}
        ), links: [] }})"""
        new_plain = f"""PlainBoTanruUnit(PlainBoTanruUnitSyntax {{
            leading_unit: TanruUnitSyntax {{ base: {broda}, assignments: [] }},
            bo_tail: Some(PlainBoSelbriTailSyntax {{ bo: {bo},
                trailing_selbri: PlainBoTanruUnit(PlainBoTanruUnitSyntax {{
                    leading_unit: TanruUnitSyntax {{ base: {brode}, assignments: [] }},
                    bo_tail: None
                }}) }})
        }})"""
        new = new_root(f"BoundSelbriSyntax {{ leading_selbri: {new_plain}, bo_tail: None }}")
        self.assert_class(old_root(old_unit), new, "pure-plain-bo")

        tagged = old_unit.replace("bo_tense_modal: None", "bo_tense_modal: Some(TagSyntax)")
        self.assertIsNone(COMPARATOR.classify_old_selbri(form(old_root(tagged))))

    def test_diagnostics_are_exact(self) -> None:
        old = {"expectations": {"syntax": {"diagnostics": []}}}
        new = {"expectations": {"syntax": {"diagnostics": ["warning"]}}}
        _, residue = COMPARATOR.compare_fixture(old, new)
        self.assertIn("expectations.syntax.diagnostics", residue)


if __name__ == "__main__":
    unittest.main()
