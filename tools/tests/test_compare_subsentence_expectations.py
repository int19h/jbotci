"""The comparer's transcribed subsentence shapes, checked against the grammar itself.

`compare-subsentence-expectations.py` rewrites the BASELINE tree, so every one of its classes is
keyed on a baseline node's name and its exact baseline field tuple, and produces the node HEAD
spells.  Both sides are hand-transcriptions of the `rule … -> struct` bodies in
`crates/jbotci-syntax/src/grammar/generated.rs` — at `ARCHIVE_COMMIT` for the old side and at
HEAD for the new one.  A transcription that drifts from either grammar fails open in a way the
report cannot show: a class keyed on a field tuple the baseline never had simply never fires,
and the fixtures it should have classified land in manual residue looking like ordinary
population instead of like a broken comparer.  Epoch 6b lost 639 fixtures to exactly that, so
the transcriptions are re-derived here rather than asserted.

The remaining tests cover what no transcription can: the two body re-typings, which have to
refuse the shapes that are acceptance flips rather than re-typings; the rejection-diagnostic
class, which is a whole-fixture predicate rather than a tree rewrite; the retired-shape
invariant on the regenerated tree; and the completeness check on epoch-new witnesses.
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

SCRIPT_PATH = Path(__file__).parents[1] / "compare-subsentence-expectations.py"
SPEC = importlib.util.spec_from_file_location("subsentence_comparator", SCRIPT_PATH)
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
FIELD = re.compile(r"^        (?:when feature\(\w+\) )?field (\w+) <-", re.M)
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
    """A stand-in leaf: the classes carry these across without looking inside them."""
    return Form(name="WithFreeModifiers", fields=(("value", "word"), ("free_modifiers", [])))


def _bridi_statement(payload: str = "bridi") -> Form:
    return Form(
        name=COMPARATOR.STATEMENT_BASE_ARM,
        args=(
            Form(
                name=COMPARATOR.BRIDI_STATEMENT_ARM,
                args=(Form(name=COMPARATOR.BRIDI_STATEMENT_STRUCT, args=(payload,)),),
            ),
        ),
    )


def _prenex_statement(terms: list, inner: Form) -> Form:
    return Form(
        name=COMPARATOR.STATEMENT_BASE_ARM,
        args=(
            Form(
                name=COMPARATOR.PRENEX_STATEMENT_ARM,
                args=(
                    Form(
                        name=COMPARATOR.PRENEX_STATEMENT_STRUCT,
                        fields=(
                            ("prenex_terms", terms),
                            ("zohu", "zohu"),
                            ("inner_statement", inner),
                        ),
                    ),
                ),
            ),
        ),
    )


class TranscribedShapeTest(unittest.TestCase):
    def setUp(self) -> None:
        self.head = _struct_fields(_grammar_at_head())
        self.archive = _struct_fields(_grammar_at_archive_commit())
        self.head_arms = _enum_arms(_grammar_at_head())
        self.archive_arms = _enum_arms(_grammar_at_archive_commit())

    def test_extraction_finds_the_productions_every_class_names(self) -> None:
        # A regex that stopped matching would make every assertion below vacuous.
        for struct in (
            COMPARATOR.SOI_OLD_STRUCT,
            COMPARATOR.FIHOI_OLD_STRUCT,
            COMPARATOR.PRENEX_STATEMENT_STRUCT,
            *(product for product, _, _ in COMPARATOR.ZANTUFA_RELATIVE_PRODUCTS.values()),
        ):
            with self.subTest(struct=struct):
                self.assertIn(_rule_name(struct), self.archive)
        for struct in (
            COMPARATOR.SOI_NEW_STRUCT,
            COMPARATOR.FIHOI_NEW_STRUCT,
            *(product for product, _, _ in COMPARATOR.ZANTUFA_RELATIVE_PRODUCTS.values()),
        ):
            with self.subTest(struct=struct):
                self.assertIn(_rule_name(struct), self.head)

    def test_the_zantufa_relative_arms_keep_their_fields_and_gain_one_sum(self) -> None:
        # The wrapper class is licensed by the two arms moving from `bridi_relative_clause`
        # into a sum of their own, with their own field tuples untouched.
        for arm, (struct, fields, body) in COMPARATOR.ZANTUFA_RELATIVE_PRODUCTS.items():
            with self.subTest(arm=arm):
                self.assertEqual(self.archive[_rule_name(struct)], fields)
                self.assertEqual(self.head[_rule_name(struct)], fields)
                self.assertIn(body, fields)
                rule = _rule_name(arm + "Syntax")
                self.assertIn(rule, self.archive_arms["bridi_relative_clause"])
                self.assertNotIn(rule, self.head_arms["bridi_relative_clause"])
                self.assertIn(
                    rule,
                    self.head_arms[_rule_name(COMPARATOR.RELATIVE_WRAPPER_ARM + "Syntax")],
                )
        self.assertIn(
            "statement_relative_clause", self.head_arms["bridi_relative_clause"]
        )

    def test_the_soi_arm_splits_into_three_at_every_leaf_inventory(self) -> None:
        self.assertEqual(self.archive[_rule_name(COMPARATOR.SOI_OLD_STRUCT)], COMPARATOR.SOI_OLD_FIELDS)
        self.assertEqual(
            self.head[_rule_name(COMPARATOR.SOI_NEW_STRUCT)], ("soi", "subsentence", "sehu")
        )
        for parent in COMPARATOR.ADVERBIAL_PARENTS:
            with self.subTest(parent=parent):
                arms = self.archive_arms[_rule_name(parent)]
                self.assertIn(_rule_name(COMPARATOR.SOI_OLD_ARM + "Syntax"), arms)
                self.assertIn(_rule_name(COMPARATOR.FIHOI_OLD_ARM + "Syntax"), arms)
                head_arms = self.head_arms[_rule_name(parent)]
                self.assertNotIn(_rule_name(COMPARATOR.SOI_OLD_ARM + "Syntax"), head_arms)
                self.assertNotIn(_rule_name(COMPARATOR.FIHOI_OLD_ARM + "Syntax"), head_arms)
                for arm in (
                    _rule_name(COMPARATOR.SOI_NEW_ARM + "Syntax"),
                    _rule_name(COMPARATOR.FIHOI_NEW_ARM + "Syntax"),
                    "zantufa_xoi_adverbial_term",
                ):
                    self.assertIn(arm, head_arms)

    def test_the_fihoi_arm_keeps_its_fihau_and_stops_eliding_it(self) -> None:
        self.assertEqual(
            self.archive[_rule_name(COMPARATOR.FIHOI_OLD_STRUCT)], COMPARATOR.FIHOI_OLD_FIELDS
        )
        self.assertEqual(
            self.head[_rule_name(COMPARATOR.FIHOI_NEW_STRUCT)], ("fihoi", "subsentence", "fihau")
        )
        head = _grammar_at_head()
        body = re.search(
            rf'rule "[^"]*" {_rule_name(COMPARATOR.FIHOI_NEW_STRUCT)}\([^)]*\) -> struct \{{(.*?)\n    \}}',
            head,
            re.S,
        )
        assert body is not None
        # An explicit FIhAU is what selects the arm, which is why the field is not optional.
        self.assertIn("field fihau <- cmavo(Fihau)", body.group(1))
        self.assertNotIn("field fihau <- opt(", body.group(1))

    def test_the_prenex_statement_fields_are_the_ones_the_retyping_reads(self) -> None:
        self.assertEqual(
            self.archive[_rule_name(COMPARATOR.PRENEX_STATEMENT_STRUCT)],
            COMPARATOR.PRENEX_STATEMENT_FIELDS,
        )
        # Both targets keep the same three roles under their own names, which is what makes the
        # rewrite a re-typing rather than a reshaping.
        self.assertEqual(
            self.head["prenex_subbridi"], ("prenex_terms", "zohu", "inner_subbridi")
        )
        self.assertEqual(
            self.head["zantufa_relative_prenex_statement"],
            ("prenex_terms", "zohu", "inner_statement"),
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


class BodyRetypingTest(unittest.TestCase):
    """The two re-typings, and the shapes they must refuse."""

    def test_a_bridi_body_retypes_into_both_families(self) -> None:
        self.assertEqual(
            COMPARATOR._relative_body(_bridi_statement()),
            Form(
                name="ZantufaRelativeStatementBase",
                args=(
                    Form(
                        name="ZantufaRelativeBridiStatement",
                        args=(Form(name="ZantufaRelativeBridiStatementSyntax", args=("bridi",)),),
                    ),
                ),
            ),
        )
        self.assertEqual(
            COMPARATOR._subsentence_body(_bridi_statement()),
            Form(
                name="BridiSubbridi",
                args=(Form(name="BridiSubbridiSyntax", args=("bridi",)),),
            ),
        )

    def test_a_nonempty_prenex_retypes_and_recurses(self) -> None:
        old = _prenex_statement(["da"], _bridi_statement())
        new = COMPARATOR._subsentence_body(old)
        self.assertEqual(new.name, "PrenexSubbridi")
        inner = dict(new.args[0].fields)
        self.assertEqual(inner["prenex_terms"], ["da"])
        self.assertEqual(inner["inner_subbridi"].name, "BridiSubbridi")

    def test_an_empty_prenex_is_refused_by_both_families(self) -> None:
        # Zantufa's prenex requires terms and camxes-exp's subsentence admits an empty one that
        # the shared statement never reached from these positions, so an empty run on the
        # baseline side is a body whose owner moved rather than a re-typing.
        old = _prenex_statement([], _bridi_statement())
        for retype in (COMPARATOR._relative_body, COMPARATOR._subsentence_body):
            with self.subTest(retype=retype.__name__):
                with self.assertRaises(COMPARATOR.BodyRetypeError):
                    retype(old)

    def test_an_i_connected_body_is_refused(self) -> None:
        # `IStatementConnection` at the default profile is an acceptance flip, not a re-typing.
        old = Form(name="IStatementConnection", args=("connection",))
        for retype in (COMPARATOR._relative_body, COMPARATOR._subsentence_body):
            with self.subTest(retype=retype.__name__):
                with self.assertRaises(COMPARATOR.BodyRetypeError):
                    retype(old)

    def test_a_tuhe_group_retypes_only_into_the_relative_family(self) -> None:
        old = Form(
            name=COMPARATOR.STATEMENT_BASE_ARM,
            args=(Form(name=COMPARATOR.TEXT_GROUP_STATEMENT_ARM, args=("group",)),),
        )
        self.assertEqual(
            COMPARATOR._relative_body(old).args[0].name, COMPARATOR.TEXT_GROUP_STATEMENT_ARM
        )
        with self.assertRaises(COMPARATOR.BodyRetypeError):
            COMPARATOR._subsentence_body(old)


class ArmRewriteTest(unittest.TestCase):
    """The node rewrites, including the two owner moves they must refuse."""

    def _rewrite(self, node):
        classes: set[str] = set()
        return COMPARATOR.rewrite_node(node, "expectations.syntax.raw", classes), classes

    def test_the_relative_wrapper_fires_only_on_the_two_zantufa_arms(self) -> None:
        for arm in COMPARATOR.ZANTUFA_RELATIVE_ARMS:
            with self.subTest(arm=arm):
                node = Form(name="BridiRelativeClause", args=(Form(name=arm, args=("payload",)),))
                new, classes = self._rewrite(node)
                self.assertEqual(new.args[0].name, COMPARATOR.RELATIVE_WRAPPER_ARM)
                self.assertEqual(classes, {COMPARATOR.CLASS_RELATIVE_WRAPPER})
        baseline = Form(
            name="BridiRelativeClause",
            args=(Form(name="RestrictiveBridiRelativeClause", args=("payload",)),),
        )
        new, classes = self._rewrite(baseline)
        self.assertIs(new, baseline)
        self.assertEqual(classes, set())

    def test_the_fihoi_split_refuses_an_elided_fihau(self) -> None:
        payload = Form(
            name=COMPARATOR.FIHOI_OLD_STRUCT,
            fields=(
                ("fihoi", _word()),
                ("statement", _bridi_statement()),
                ("fihau", Form(name="None")),
            ),
        )
        with self.assertRaises(COMPARATOR.Divergence):
            self._rewrite(Form(name=COMPARATOR.FIHOI_OLD_ARM, args=(payload,)))

    def test_the_fihoi_split_unwraps_an_explicit_fihau(self) -> None:
        payload = Form(
            name=COMPARATOR.FIHOI_OLD_STRUCT,
            fields=(
                ("fihoi", _word()),
                ("statement", _bridi_statement()),
                ("fihau", Form(name="Some", args=("fihau",))),
            ),
        )
        new, classes = self._rewrite(Form(name=COMPARATOR.FIHOI_OLD_ARM, args=(payload,)))
        self.assertEqual(new.name, COMPARATOR.FIHOI_NEW_ARM)
        self.assertEqual(dict(new.args[0].fields)["fihau"], "fihau")
        self.assertEqual(classes, {COMPARATOR.CLASS_FIHOI_SPLIT})

    def test_the_soi_split_wraps_the_arm_and_retypes_the_body(self) -> None:
        payload = Form(
            name=COMPARATOR.SOI_OLD_STRUCT,
            fields=(
                ("soi", _word()),
                ("statement", _bridi_statement()),
                ("sehu", Form(name="None")),
            ),
        )
        new, classes = self._rewrite(Form(name=COMPARATOR.SOI_OLD_ARM, args=(payload,)))
        self.assertEqual(new.name, COMPARATOR.SOI_NEW_ARM)
        self.assertEqual(new.args[0].name, COMPARATOR.SOI_NEW_WRAPPER)
        self.assertEqual(dict(new.args[0].args[0].fields)["subsentence"].name, "BridiSubbridi")
        self.assertEqual(classes, {COMPARATOR.CLASS_SOI_SPLIT})

    def test_a_statement_width_soi_body_is_an_owner_move(self) -> None:
        # An I-connected body moves to the Zantufa XOI arm, which is an owner change and has to
        # take an individual disposition rather than a class.
        payload = Form(
            name=COMPARATOR.SOI_OLD_STRUCT,
            fields=(
                ("soi", _word()),
                ("statement", Form(name="IStatementConnection", args=("connection",))),
                ("sehu", Form(name="None")),
            ),
        )
        with self.assertRaises(COMPARATOR.Divergence):
            self._rewrite(Form(name=COMPARATOR.SOI_OLD_ARM, args=(payload,)))


class RejectionDiagnosticClassTest(unittest.TestCase):
    """The one class that is a whole-fixture predicate rather than a tree rewrite."""

    def _document(self, status: str, diagnostics: list[dict]) -> dict:
        return {"expectations": {"syntax": {"status": status, "diagnostics": diagnostics}}}

    def _error(self, code: str) -> dict:
        return {"severity": "error", "code": code, "byte-span": [0, 2]}

    def _classify(self, old: dict, new: dict) -> bool:
        old_leaves = dict(COMPARATOR.leaves(old))
        new_leaves = dict(COMPARATOR.leaves(new))
        return COMPARATOR.is_rejection_diagnostic_reclassification(
            old, new, old_leaves, new_leaves
        )

    def test_a_moved_error_frontier_classifies(self) -> None:
        self.assertTrue(
            self._classify(
                self._document("failure", [self._error("syntax.unexpected-end")]),
                self._document("failure", [self._error("syntax.incomplete-selbri")]),
            )
        )

    def test_an_acceptance_flip_does_not(self) -> None:
        self.assertFalse(
            self._classify(
                self._document("failure", [self._error("syntax.unexpected-end")]),
                self._document("success", []),
            )
        )

    def test_a_warning_appearing_does_not(self) -> None:
        warned = {
            "expectations": {
                "syntax": {
                    "status": "failure",
                    "diagnostics": [
                        {"severity": "warning", "code": "w", "byte-span": [0, 2]},
                        self._error("syntax.incomplete-selbri"),
                    ],
                }
            }
        }
        self.assertFalse(
            self._classify(
                self._document("failure", [self._error("syntax.unexpected-end")]), warned
            )
        )

    def test_another_leaf_moving_does_not(self) -> None:
        old = self._document("failure", [self._error("syntax.unexpected-end")])
        new = self._document("failure", [self._error("syntax.incomplete-selbri")])
        old["expectations"]["morphology"] = {"status": "success"}
        new["expectations"]["morphology"] = {"status": "failure"}
        self.assertFalse(self._classify(old, new))

    def test_an_unchanged_fixture_does_not_classify(self) -> None:
        same = self._document("failure", [self._error("syntax.unexpected-end")])
        self.assertFalse(self._classify(same, same))


class RetiredShapeInvariantTest(unittest.TestCase):
    """The retired-shape check reads the regenerated TEXT, so it has to read identifiers."""

    def test_a_retired_node_is_reported_in_both_debug_renderings(self) -> None:
        for tree in (
            "Foo { bar: SoiAdverbialTerm(SoiAdverbialTermSyntax { sehu: None }) }",
            "Foo { bar: FihoiAdverbialTermSyntax { fihau: None } }",
        ):
            with self.subTest(tree=tree):
                with self.assertRaises(COMPARATOR.Divergence):
                    COMPARATOR.assert_no_retired_shapes(tree, "expectations.syntax.raw")

    def test_the_splits_own_output_is_not_the_shape_it_retires(self) -> None:
        # `ExpSoiAdverbialTermSyntax` and `FihoiProposalAdverbialTermSyntax` both END with a
        # retired name.  A substring test would report the split's own products as the nodes the
        # epoch deletes, which both hides the classes and invents residue.
        COMPARATOR.assert_no_retired_shapes(
            "ExpSoiAdverbialTerm(ExpSoiAdverbialTermSyntax(ExpSoiSubsentenceAdverbialSyntax { }))"
            " FihoiProposalAdverbialTerm(FihoiProposalAdverbialTermSyntax { })",
            "expectations.syntax.raw",
        )


class EpochNewDiagnosticsPinTest(unittest.TestCase):
    """The completeness check on epoch-new witnesses, exercised on a synthetic tree.

    Nothing classifies an epoch-new witness, so the pin it carries is the whole audit.  This
    epoch moves warnings between three adverbial arms and two relative routes, so a witness that
    omits the key would leave a construct free to stop warning with no expectation moving.
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
                "adhoc/warned.toml": 'id = "warned"\nlojban = "mi broda xoi mi brode"\n\n'
                "[expectations.syntax]\n"
                'status = "success"\n'
                'raw = "tree"\n'
                'diagnostics = [{ severity = "warning", code = "x", byte-span = [0, 2],'
                ' source-text = "mi", message = "m" }]\n',
                "adhoc/silent.toml": 'id = "silent"\nlojban = "mi broda soi mi brode"\n\n'
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
                "adhoc/unpinned.toml": 'id = "unpinned"\nlojban = "broda no\'oi mi brode"\n\n'
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
