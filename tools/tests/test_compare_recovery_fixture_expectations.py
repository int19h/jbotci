from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path
from typing import Any


SCRIPT_PATH = (
    Path(__file__).parents[1] / "compare-recovery-fixture-expectations.py"
)
SPEC = importlib.util.spec_from_file_location("recovery_fixture_comparator", SCRIPT_PATH)
assert SPEC is not None
assert SPEC.loader is not None
COMPARATOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = COMPARATOR
SPEC.loader.exec_module(COMPARATOR)


JsonObject = dict[str, Any]


def diagnostic(start: int, end: int, source_text: str) -> JsonObject:
    return {
        "severity": "error",
        "code": "syntax.unexpected-cmavo",
        "byte-span": [start, end],
        "source-text": source_text,
        "message": "unexpected cmavo",
    }


def fixture(
    valid_tokens: list[str],
    recovery_items: list[JsonObject],
    diagnostics: list[JsonObject] | None = None,
) -> JsonObject:
    recovered_diagnostics = diagnostics or [diagnostic(3, 5, "ku")]
    return {
        "id": "comparator-test",
        "lojban": "mi ku do ti",
        "expectations": {
            "syntax": {
                "status": "failure",
                "diagnostics": [diagnostic(3, 5, "ku")],
                "recovered": {
                    "status": "failure",
                    "diagnostics": recovered_diagnostics,
                    "tree": {
                        "valid-tokens": valid_tokens,
                        "recovery-items": recovery_items,
                    },
                },
            }
        },
    }


def baseline_fixture() -> JsonObject:
    return fixture(
        ["mi"],
        [
            {
                "kind": "invalid",
                "error-index": 0,
                "byte-spans": [[3, 5], [6, 8], [9, 11]],
            }
        ],
    )


def unwind_candidate() -> JsonObject:
    return fixture(
        ["mi", "do"],
        [
            {
                "kind": "invalid",
                "error-index": 0,
                "byte-spans": [[3, 5], [9, 11]],
            },
            {
                "kind": "missing",
                "error-index": 0,
                "byte-spans": [[6, 6]],
            },
        ],
    )


class RecoveryFixtureComparatorTests(unittest.TestCase):
    def classify(self, candidate: JsonObject) -> Any:
        return COMPARATOR.classify_changed_fixture(
            Path("fixture.toml"), baseline_fixture(), candidate
        )

    def test_accepts_only_conserving_unwind_shape(self) -> None:
        classification = self.classify(unwind_candidate())

        self.assertEqual(classification.category, "recovery-unwind")
        self.assertEqual(classification.reasons, ())
        self.assertEqual(classification.old_invalid_token_count, 3)
        self.assertEqual(classification.new_invalid_token_count, 2)
        self.assertEqual(classification.old_missing_item_count, 0)
        self.assertEqual(classification.new_missing_item_count, 1)

    def test_rejects_a_narrowed_invalid_token_span(self) -> None:
        candidate = unwind_candidate()
        candidate["expectations"]["syntax"]["recovered"]["tree"]["recovery-items"][0][
            "byte-spans"
        ][0] = [4, 5]

        classification = self.classify(candidate)

        self.assertEqual(classification.category, "residue")
        self.assertIn(
            "candidate invalid tokens are not an exact ordered subset of baseline",
            classification.reasons,
        )

    def test_rejects_a_lost_token_projection(self) -> None:
        candidate = unwind_candidate()
        candidate["expectations"]["syntax"]["recovered"]["tree"][
            "valid-tokens"
        ] = ["mi"]

        classification = self.classify(candidate)

        self.assertEqual(classification.category, "residue")
        self.assertIn(
            "valid and invalid token projections do not conserve token count",
            classification.reasons,
        )

    def test_rejects_finer_slots_without_a_missing_item(self) -> None:
        candidate = fixture(
            ["mi", "do"],
            [
                {
                    "kind": "invalid",
                    "error-index": 0,
                    "byte-spans": [[3, 5]],
                },
                {
                    "kind": "invalid",
                    "error-index": 0,
                    "byte-spans": [[9, 11]],
                },
            ],
        )

        classification = self.classify(candidate)

        self.assertEqual(classification.category, "residue")
        self.assertIn(
            "candidate does not synthesize additional missing items",
            classification.reasons,
        )

    def test_rejects_a_missing_item_without_a_matching_invalid_slot(self) -> None:
        candidate = unwind_candidate()
        candidate_diagnostics = [
            diagnostic(3, 5, "ku"),
            {
                "severity": "error",
                "code": "syntax.incomplete-sumti",
                "byte-span": [6, 6],
                "source-text": "",
                "message": "incomplete sumti",
            },
        ]
        recovered = candidate["expectations"]["syntax"]["recovered"]
        recovered["diagnostics"] = candidate_diagnostics
        recovered["tree"]["recovery-items"][1]["error-index"] = 1

        classification = self.classify(candidate)

        self.assertEqual(classification.category, "residue")
        self.assertIn(
            "synthesized missing items do not share a diagnostic with an invalid item",
            classification.reasons,
        )


if __name__ == "__main__":
    unittest.main()
