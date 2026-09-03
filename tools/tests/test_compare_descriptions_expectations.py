"""The comparer's transcribed quantifier shapes, checked against the grammar itself.

`compare-descriptions-expectations.py` rewrites the BASELINE tree, so its one tree class is keyed
on a baseline node's name and its exact baseline field tuples, and produces the node HEAD spells.
Both sides are hand-transcriptions of the `rule … -> struct` bodies in
`crates/jbotci-syntax/src/grammar/generated.rs` — at `ARCHIVE_COMMIT` for the old side and at HEAD
for the new one. A transcription that drifts from either grammar fails OPEN in a way the report
cannot show: a class keyed on a field tuple the baseline never had simply never fires, and the
fixtures it should have classified land in manual residue looking like ordinary population instead
of like a broken comparer. Epoch 6b lost 639 fixtures to exactly that, so the transcriptions are
re-derived here rather than asserted.

The remaining tests cover what no transcription can: that the re-typing refuses every mex the
baseline quantifier route cannot form, so a genuine raw mex is never laundered into a baseline
one; that the warning class accepts only the removal of `experimental-zantufa-mex` and nothing
else; and the retired-shape invariant on the regenerated tree, which is what holds epoch 9's
deleted head-connective route deleted.
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

SCRIPT_PATH = Path(__file__).parents[1] / "compare-descriptions-expectations.py"
SPEC = importlib.util.spec_from_file_location("descriptions_comparator", SCRIPT_PATH)
assert SPEC is not None
assert SPEC.loader is not None
COMPARATOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = COMPARATOR
SPEC.loader.exec_module(COMPARATOR)

Form = COMPARATOR.Form

RULE = re.compile(r'^    rule "[^"]*" (\w+)\([^)]*\) -> struct \{\n(.*?)\n    \}', re.S | re.M)
ENUM = re.compile(r'^    rule "[^"]*" (\w+)\([^)]*\) -> enum \{\n(.*?)\n    \}', re.S | re.M)
# A gated field is a field exactly as an ungated one is: the Debug rendering carries it either
# way, so the field tuple the comparer matches has to see it.
FIELD = re.compile(r"^        (?:when feature\(\w+\) )?field (\w+)(?::[^<]*)? <-", re.M)
ARM = re.compile(r"^        (?:when feature\(\w+\) )?(\w+),$", re.M)


def _rule_name(struct: str) -> str:
    """The DSL's snake-case rule name for a Debug struct name."""
    return re.sub(r"(?<!^)(?=[A-Z])", "_", struct[: -len("Syntax")]).lower()


def _struct_fields(source: str) -> dict[str, tuple[str, ...]]:
    return {name: tuple(FIELD.findall(body)) for name, body in RULE.findall(source)}


def _enum_arms(source: str) -> dict[str, tuple[str, ...]]:
    return {name: tuple(ARM.findall(body)) for name, body in ENUM.findall(source)}


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


def _word() -> Form:
    """A stand-in leaf: the class carries these across without looking inside them."""
    return Form(name="WithFreeModifiers", fields=(("value", "word"), ("free_modifiers", [])))


NONE = Form(name="None")


def _operand(simple: Form, links: list | None = None, grouped: Form | None = None) -> Form:
    """One `mekso_operand`, with its continuation slots empty unless a test fills them."""
    return Form(
        name=COMPARATOR.OPERAND_ARM,
        args=(
            Form(
                name=COMPARATOR.OPERAND_STRUCT,
                fields=(
                    (
                        "connected_expression",
                        Form(
                            name=COMPARATOR.CHAIN_ARM,
                            args=(
                                Form(
                                    name=COMPARATOR.CHAIN_STRUCT,
                                    fields=(
                                        ("first", Form(name=COMPARATOR.SIMPLE_ARM, args=(simple,))),
                                        ("links", links if links is not None else []),
                                    ),
                                ),
                            ),
                        ),
                    ),
                    ("grouped_continuation", grouped if grouped is not None else NONE),
                ),
            ),
        ),
    )


def _infix(operand: Form, tail: Form | None = None, continuations: list | None = None) -> Form:
    return Form(
        name=COMPARATOR.INFIX_ARM,
        args=(
            Form(
                name=COMPARATOR.INFIX_STRUCT,
                fields=(
                    (
                        "first_expression",
                        Form(
                            name=COMPARATOR.PRECEDENCE_STRUCT,
                            fields=(
                                ("left_expression", operand),
                                ("tail", tail if tail is not None else NONE),
                            ),
                        ),
                    ),
                    ("continuations", continuations if continuations is not None else []),
                ),
            ),
        ),
    )


def _priority(mekso: Form) -> Form:
    return Form(
        name=COMPARATOR.ZPRI_ARM,
        args=(Form(name=COMPARATOR.ZPRI_STRUCT, args=(mekso,)),),
    )


def _number_operand(payload: str = "pa-run") -> Form:
    return Form(
        name=COMPARATOR.NUMBER_ARM,
        args=(Form(name=COMPARATOR.NUMBER_STRUCT, args=(payload,)),),
    )


def _parenthesized_operand() -> Form:
    return Form(
        name=COMPARATOR.PARENTHESIZED_ARM,
        args=(
            Form(
                name=COMPARATOR.PARENTHESIZED_STRUCT,
                fields=(
                    ("vei", _word()),
                    ("inner_expression", "inner"),
                    ("veho", NONE),
                ),
            ),
        ),
    )


def _rewrite(old: Form) -> tuple[object, set[str]]:
    classes: set[str] = set()
    return COMPARATOR.rewrite_node(old, "", classes), classes


class TranscribedShapeTest(unittest.TestCase):
    def setUp(self) -> None:
        self.head = _struct_fields(_grammar_at_head())
        self.archive = _struct_fields(_grammar_at_archive_commit())
        self.head_arms = _enum_arms(_grammar_at_head())
        self.archive_arms = _enum_arms(_grammar_at_archive_commit())

    def test_extraction_finds_the_productions_the_class_names(self) -> None:
        # A regex that stopped matching would make every assertion below vacuous.
        for struct in (
            COMPARATOR.ZPRI_STRUCT,
            COMPARATOR.INFIX_STRUCT,
            COMPARATOR.OPERAND_STRUCT,
            COMPARATOR.PARENTHESIZED_STRUCT,
            COMPARATOR.MEKSO_QUANTIFIER_STRUCT,
        ):
            with self.subTest(struct=struct):
                self.assertIn(_rule_name(struct), self.archive)
                self.assertIn(_rule_name(struct), self.head)

    def test_the_old_mex_spine_field_tuples_are_the_baseline_ones(self) -> None:
        # Each tuple is the exact one the macro rendered at the archive commit; a drift here is
        # what would make the class silently stop firing.
        self.assertEqual(
            self.archive[_rule_name(COMPARATOR.INFIX_STRUCT)], COMPARATOR.INFIX_FIELDS
        )
        self.assertEqual(
            self.archive[_rule_name(COMPARATOR.OPERAND_STRUCT)], COMPARATOR.OPERAND_FIELDS
        )
        self.assertEqual(
            self.archive[_rule_name(COMPARATOR.PARENTHESIZED_STRUCT)],
            COMPARATOR.PARENTHESIZED_FIELDS,
        )

    def test_the_precedence_and_chain_tuples_match_the_baseline(self) -> None:
        self.assertEqual(
            self.archive[_rule_name(COMPARATOR.PRECEDENCE_STRUCT)], COMPARATOR.PRECEDENCE_FIELDS
        )

    def test_the_new_mekso_quantifier_tuple_is_the_head_one(self) -> None:
        self.assertEqual(
            self.head[_rule_name(COMPARATOR.MEKSO_QUANTIFIER_STRUCT)], ("vei", "mekso", "veho")
        )

    def test_the_quantifier_sum_still_holds_both_targets_of_the_retyping(self) -> None:
        arms = self.head_arms["quantifier"]
        self.assertIn("pa_run_quantifier", arms)
        self.assertIn("mekso_quantifier", arms)

    def test_every_retired_shape_is_gone_from_the_head_grammar(self) -> None:
        head = _grammar_at_head()
        for shape in COMPARATOR.RETIRED_SHAPES:
            with self.subTest(shape=shape):
                self.assertNotIn(_rule_name(shape + "Syntax"), self.head)
                self.assertNotIn(f"{shape}Syntax", head)


class RetypingTest(unittest.TestCase):
    def test_a_lone_number_operand_returns_to_the_baseline_pa_run(self) -> None:
        rewritten, classes = _rewrite(_priority(_infix(_operand(_number_operand()))))
        self.assertEqual(classes, {COMPARATOR.CLASS_QUANTIFIER_RETYPING})
        self.assertEqual(rewritten, Form(name=COMPARATOR.PA_RUN_ARM, args=("pa-run",)))

    def test_a_lone_parenthesized_operand_returns_to_the_baseline_vei_form(self) -> None:
        rewritten, classes = _rewrite(_priority(_infix(_operand(_parenthesized_operand()))))
        self.assertEqual(classes, {COMPARATOR.CLASS_QUANTIFIER_RETYPING})
        self.assertEqual(
            rewritten,
            Form(
                name=COMPARATOR.MEKSO_QUANTIFIER_ARM,
                args=(
                    Form(
                        name=COMPARATOR.MEKSO_QUANTIFIER_STRUCT,
                        fields=(("vei", _word()), ("mekso", "inner"), ("veho", NONE)),
                    ),
                ),
            ),
        )

    def test_a_non_empty_continuation_slot_is_a_genuine_raw_mex(self) -> None:
        # Each of these is a mex the baseline quantifier route cannot form, so the candidate keeps
        # its priority ownership and any change to it is residue rather than a re-typing.
        for label, candidate in (
            ("infix continuation", _infix(_operand(_number_operand()), continuations=["su'i"])),
            ("precedence tail", _infix(_operand(_number_operand()), tail=Form(name="Tail"))),
            ("operand link", _infix(_operand(_number_operand(), links=["joi"]))),
            ("grouped continuation", _infix(_operand(_number_operand(), grouped=Form(name="Ke")))),
        ):
            with self.subTest(label=label):
                rewritten, classes = _rewrite(_priority(candidate))
                self.assertEqual(classes, set())
                self.assertEqual(rewritten, _priority(candidate))

    def test_an_operand_that_is_neither_baseline_surface_is_left_alone(self) -> None:
        forethought = Form(name="ForethoughtCallMekso", args=("call",))
        rewritten, classes = _rewrite(_priority(_infix(_operand(forethought))))
        self.assertEqual(classes, set())
        self.assertEqual(rewritten, _priority(_infix(_operand(forethought))))

    def test_a_reading_other_than_infix_is_left_alone(self) -> None:
        # Only the baseline `infix_mekso` reading classifies; the Zantufa readings are held to
        # Zantufa-only surfaces elsewhere and keep their raw-mex ownership by construction.
        for arm in ("ZantufaPriorityMex", "ReinterpretZantufaMex", "ReversePolishMekso"):
            with self.subTest(arm=arm):
                candidate = _priority(Form(name=arm, args=("payload",)))
                rewritten, classes = _rewrite(candidate)
                self.assertEqual(classes, set())
                self.assertEqual(rewritten, candidate)

    def test_a_node_that_is_not_the_priority_route_is_left_alone(self) -> None:
        candidate = Form(name="ZantufaRawMeksoQuantifier", args=("mex",))
        rewritten, classes = _rewrite(candidate)
        self.assertEqual(classes, set())
        self.assertEqual(rewritten, candidate)


class WarningRemovalTest(unittest.TestCase):
    MEX = {"severity": "warning", "code": COMPARATOR.ZANTUFA_MEX_CODE, "byte-span": [0, 2]}
    OTHER = {
        "severity": "warning",
        "code": "syntax.warning.experimental-zantufa-selbri-relative-placement",
        "byte-span": [9, 12],
    }
    ERROR = {"severity": "error", "code": "syntax.unexpected-cmavo", "byte-span": [3, 5]}

    def test_removing_only_the_mex_warnings_is_the_class(self) -> None:
        self.assertTrue(
            COMPARATOR.quantifier_warnings_removed([self.MEX, self.OTHER], [self.OTHER])
        )
        self.assertTrue(COMPARATOR.quantifier_warnings_removed([self.MEX], []))

    def test_an_unchanged_list_is_not_the_class(self) -> None:
        # The class must have something to explain; an equal list never reaches it, and asserting
        # it here keeps a future refactor from making the predicate vacuously true.
        self.assertFalse(COMPARATOR.quantifier_warnings_removed([self.OTHER], [self.OTHER]))

    def test_any_other_diagnostic_moving_takes_the_fixture_out_of_the_class(self) -> None:
        moved = dict(self.OTHER, **{"byte-span": [10, 13]})
        self.assertFalse(COMPARATOR.quantifier_warnings_removed([self.MEX, self.OTHER], [moved]))
        self.assertFalse(
            COMPARATOR.quantifier_warnings_removed([self.MEX, self.OTHER], [self.OTHER, self.ERROR])
        )
        self.assertFalse(COMPARATOR.quantifier_warnings_removed([self.MEX, self.ERROR], []))

    def test_order_is_part_of_the_expectation(self) -> None:
        self.assertFalse(
            COMPARATOR.quantifier_warnings_removed(
                [self.MEX, self.OTHER, self.ERROR], [self.ERROR, self.OTHER]
            )
        )


class RetiredShapeTest(unittest.TestCase):
    def test_a_regenerated_tree_may_not_contain_a_retired_node(self) -> None:
        for shape in COMPARATOR.RETIRED_SHAPES:
            with self.subTest(shape=shape):
                with self.assertRaises(COMPARATOR.Divergence):
                    COMPARATOR.assert_no_retired_shapes(f"SumtiBase({shape}({shape}Syntax {{ }}))", "raw")

    def test_a_longer_name_ending_in_a_retired_one_is_not_a_hit(self) -> None:
        # A substring test would report a node that merely ENDS with a retired name, hiding the
        # class and inventing residue; the boundary is what makes the check usable.
        COMPARATOR.assert_no_retired_shapes(
            "ExpDescriptionConnectionSumtiSyntax { }", "raw"
        )


if __name__ == "__main__":
    unittest.main()
