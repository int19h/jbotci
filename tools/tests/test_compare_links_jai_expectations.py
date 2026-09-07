"""Synthetic comparer mutations; these are not measured corpus or parser-winner results."""

import copy
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

from tools import links_jai_ledger as ledger
from tools.rust_debug import DebugParser, NumberLiteral, exact_equal


SCRIPT = Path(__file__).parents[1] / "compare-links-jai-expectations.py"
SPEC = importlib.util.spec_from_file_location("tools.links_jai_expectations", SCRIPT)
COMPARER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = COMPARER
SPEC.loader.exec_module(COMPARER)
JAI = COMPARER.jai
Form = JAI.Form


def debug(value):
    """Serialize synthetic Debug test data, never parser-produced expectations."""
    if isinstance(value, Form):
        if value.fields is not None:
            return value.name + " { " + ", ".join(f"{key}: {debug(child)}" for key, child in value.fields) + " }"
        if value.args is not None:
            return value.name + "(" + ", ".join(debug(child) for child in value.args) + ")"
        return value.name
    if isinstance(value, list):
        return "[" + ", ".join(debug(child) for child in value) + "]"
    return json.dumps(value, ensure_ascii=False)


class ComparerTests(unittest.TestCase):
    def fixture(self):
        return {"id": "synthetic.case", "lojban": "mi broda", "tags": ["links-jai-epoch"],
                "expectations": {"syntax": {"status": "success", "raw": "synthetic strict tree",
                                             "diagnostics": []}}}

    def test_exact_fixture_identity_has_no_delta(self):
        fixture = self.fixture()
        delta = COMPARER.compare_pair(fixture, copy.deepcopy(fixture), "case.toml")
        self.assertEqual(delta.changed, ())

    def test_scalar_type_changes_in_pinned_surfaces_are_not_identity(self):
        for before, after in [(True, 1), (False, 0), (1, 1.0), (-0.0, 0.0)]:
            for first, second in [(before, after), (after, before)]:
                with self.subTest(first=first, second=second):
                    old = self.fixture()
                    old["expectations"]["syntax"]["tree"] = {"nested": [first]}
                    new = copy.deepcopy(old)
                    new["expectations"]["syntax"]["tree"]["nested"] = [second]
                    delta = COMPARER.compare_pair(old, new, "case.toml")
                    self.assertEqual(delta.changed, ("syntax.tree",))

    def test_metadata_scalar_types_are_exact(self):
        for before, after in [(True, 1), (False, 0), (1, 1.0), (-0.0, 0.0)]:
            with self.subTest(before=before, after=after):
                old = self.fixture()
                old["provenance"] = [{"synthetic-value": before}]
                new = copy.deepcopy(old)
                new["provenance"][0]["synthetic-value"] = after
                with self.assertRaisesRegex(ledger.ValidationError, "metadata changed"):
                    COMPARER.compare_pair(old, new, "case.toml")

    def test_source_id_dialect_and_prose_changes_fail(self):
        for field, value in [("id", "other"), ("lojban", "mi brode"), ("dialect", "(zantufa)"),
                             ("provenance", [{"kind": "other", "description": "edited prose"}])]:
            with self.subTest(field=field):
                old = self.fixture()
                new = copy.deepcopy(old)
                new[field] = value
                with self.assertRaises(ledger.ValidationError):
                    COMPARER.compare_pair(old, new, "case.toml")

    def test_tree_and_diagnostics_are_full_surfaces(self):
        old = self.fixture()
        new = copy.deepcopy(old)
        new["expectations"]["syntax"]["raw"] = {"sha256": "a" * 64}
        new["expectations"]["syntax"]["diagnostics"] = [{"severity": "warning", "code": "synthetic"}]
        delta = COMPARER.compare_pair(old, new, "case.toml")
        self.assertEqual(delta.changed, ("syntax.diagnostics", "syntax.raw"))

    def test_removing_raw_on_failure_stays_visible(self):
        old = self.fixture()
        new = copy.deepcopy(old)
        new["expectations"]["syntax"] = {"status": "failure", "diagnostics": [{"severity": "error"}]}
        delta = COMPARER.compare_pair(old, new, "case.toml")
        self.assertEqual(delta.changed, ("syntax.diagnostics", "syntax.raw", "syntax.status"))

    def test_new_witnesses_require_tag_status_raw_and_complete_diagnostics(self):
        COMPARER.require_new_pins(self.fixture(), "case.toml")
        for mutation in ["tag", "raw", "diagnostics", "pending", "false-success"]:
            with self.subTest(mutation=mutation):
                fixture = self.fixture()
                syntax = fixture["expectations"]["syntax"]
                if mutation == "tag":
                    fixture["tags"] = []
                elif mutation in {"raw", "diagnostics"}:
                    del syntax[mutation]
                elif mutation == "pending":
                    syntax["status"] = "pending"
                else:
                    syntax["diagnostics"] = [{"severity": "error"}]
                with self.assertRaises(ledger.ValidationError):
                    COMPARER.require_new_pins(fixture, "case.toml")

    def test_recovered_changes_are_not_silently_mechanical(self):
        old = self.fixture()
        old["expectations"]["syntax"]["recovered"] = {"raw": {"sha256": "a" * 64}}
        new = copy.deepcopy(old)
        new["expectations"]["syntax"]["recovered"]["raw"] = {"sha256": "b" * 64}
        delta = COMPARER.compare_pair(old, new, "case.toml")
        self.assertEqual(delta.changed, ("syntax.recovered.raw",))
        disposition = {"class": "d1-recovery-withdrawal", "surfaces": ["syntax.recovered.raw"],
                       "reason": "synthetic disposition test"}
        COMPARER.review_delta(delta, disposition, "c-a")
        disposition["surfaces"] = []
        with self.assertRaises(ledger.ValidationError):
            COMPARER.review_delta(delta, disposition, "c-a")

    def test_strict_failure_pins_errors_but_has_no_tree(self):
        fixture = self.fixture()
        syntax = fixture["expectations"]["syntax"]
        syntax["status"] = "failure"
        syntax["diagnostics"] = [{"severity": "error", "code": "synthetic.error",
                                 "byte-span": [0, 2], "source-text": "mi"}]
        with self.assertRaisesRegex(ledger.ValidationError, "strict failure cannot have"):
            COMPARER.require_new_pins(fixture, "case.toml")
        del syntax["raw"]
        COMPARER.require_new_pins(fixture, "case.toml")

    def test_manual_class_cannot_hide_other_surfaces_or_premature_mechanisms(self):
        delta = COMPARER.FixtureDelta("synthetic.case", "case.toml", ("syntax.raw",), False)
        for category in ["manual", "d1-recovery-withdrawal", "d3-jai-exception"]:
            with self.subTest(category=category):
                with self.assertRaises(ledger.ValidationError):
                    COMPARER.review_delta(delta, {"class": category, "surfaces": ["syntax.raw"],
                                                  "reason": "synthetic"}, "c-a")

    def roots(self):
        temporary = tempfile.TemporaryDirectory(prefix="links-jai-comparer-unit-")
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        baseline, candidate = root / "base", root / "candidate"
        for directory in [baseline, candidate]:
            (directory / "tests/fixtures").mkdir(parents=True)
            (directory / "tests/fixtures/base.toml").write_text(
                'id = "synthetic.base"\nlojban = "mi broda"\n'
                '[expectations.syntax]\nstatus = "success"\nraw = "synthetic tree"\ndiagnostics = []\n',
                encoding="utf-8")
        (candidate / "tests/fixtures/new.toml").write_text(
            'id = "synthetic.new"\nlojban = "mi brode"\ntags = ["links-jai-epoch"]\n'
            '[expectations.syntax]\nstatus = "success"\nraw = "synthetic tree"\ndiagnostics = []\n',
            encoding="utf-8")
        return baseline, candidate

    def test_population_pairs_paths_and_ids_and_requires_nonzero_witnesses(self):
        baseline, candidate = self.roots()
        for root in [baseline, candidate]:
            profile = root / "tests/fixtures/profiles/all.toml"
            profile.parent.mkdir()
            profile.write_text('name = "all"\n', encoding="utf-8")
        report = COMPARER.compare_roots(baseline, candidate, "c-a", {})
        self.assertEqual(report["baseline_fixtures"], 1)
        self.assertEqual(report["unchanged_existing_fixtures"], 1)
        self.assertEqual(report["epoch_new"], ["synthetic.new"])
        (candidate / "tests/fixtures/base.toml").unlink()
        with self.assertRaisesRegex(ledger.ValidationError, "unpaired baseline"):
            COMPARER.compare_roots(baseline, candidate, "c-a", {})

    def test_untagged_new_and_duplicate_ids_fail(self):
        baseline, candidate = self.roots()
        path = candidate / "tests/fixtures/new.toml"
        original = path.read_text()
        for changed in [original.replace('tags = ["links-jai-epoch"]', 'tags = []'),
                        original.replace("synthetic.new", "synthetic.base")]:
            with self.subTest(changed=changed):
                path.write_text(changed, encoding="utf-8")
                with self.assertRaises(ledger.ValidationError):
                    COMPARER.compare_roots(baseline, candidate, "c-a", {})

    def test_undeclared_and_unused_dispositions_fail(self):
        baseline, candidate = self.roots()
        with self.assertRaises(ledger.ValidationError):
            COMPARER.compare_roots(baseline, candidate, "c-a", {"synthetic.base": {}})
        path = candidate / "tests/fixtures/base.toml"
        path.write_text(path.read_text().replace("synthetic tree", "different tree"), encoding="utf-8")
        with self.assertRaisesRegex(ledger.ValidationError, "undispositioned"):
            COMPARER.compare_roots(baseline, candidate, "c-a", {})

    def test_jai_mapping_requires_exact_full_tree_and_unchanged_other_facets(self):
        old = self.fixture()
        word = JAI.arm("WordTanruUnit", JAI.arm("WordTanruUnitSyntax", "synthetic word at 12..17"))
        old_node = JAI.product("JaiModalTanruUnitSyntax", jai="synthetic JAI at 9..12",
                               tense_modal=JAI.NONE, inner_unit=word)
        new_node = JAI.product("JaiModalTanruUnitSyntax", jai="synthetic JAI at 9..12",
                               tense_modal=JAI.NONE, inner_unit=JAI.shared_atom(word))
        old["expectations"]["syntax"]["raw"] = debug(old_node)
        new = copy.deepcopy(old)
        new["expectations"]["syntax"]["raw"] = debug(new_node)
        delta = COMPARER.compare_pair(old, new, "case.toml")
        self.assertEqual(COMPARER.mechanical_jai(old, new, delta, "c-c"), 1)
        self.assertEqual(COMPARER.mechanical_jai(old, new, delta, "c-a"), 0)
        for mutation in ["word", "outside", "diagnostics", "failure", "regression", "recovered"]:
            with self.subTest(mutation=mutation):
                before, after = copy.deepcopy(old), copy.deepcopy(new)
                syntax = after["expectations"]["syntax"]
                if mutation in {"word", "outside"}:
                    syntax["raw"] = syntax["raw"].replace("12..17" if mutation == "word" else "9..12", "13..18")
                elif mutation == "diagnostics":
                    syntax["diagnostics"] = [{"severity": "warning"}]
                elif mutation == "failure":
                    syntax["status"] = "failure"
                elif mutation == "regression":
                    before["tags"].append("regression-baseline")
                    after["tags"].append("regression-baseline")
                else:
                    syntax["recovered"] = {"raw": "different recovered tree"}
                changed = COMPARER.compare_pair(before, after, "case.toml")
                self.assertEqual(COMPARER.mechanical_jai(before, after, changed, "c-c"), 0)

    def test_jai_mapping_preserves_scalar_types_and_numeric_literals(self):
        old_node = JAI.product("JaiModalTanruUnitSyntax", jai="synthetic JAI",
            tense_modal=JAI.NONE, inner_unit=JAI.arm("WordTanruUnit",
                JAI.arm("WordTanruUnitSyntax", "synthetic word")))
        mapped, count = JAI.rewrite(old_node)
        self.assertEqual(count, 1)
        for left, right in [("true", "1"), ("false", "0"), ("1", "1.0"),
                            ("-0.0", "0.0"), ("1.0000000000000001", "1.0"),
                            ("9007199254740993.0", "9007199254740992.0"),
                            ('"1"', "1")]:
            for before, after in [(left, right), (right, left)]:
                with self.subTest(before=before, after=after):
                    old = self.fixture()
                    old["expectations"]["syntax"]["raw"] = (
                        f"Envelope {{ unrelated: Some([{before}]), jai: {debug(old_node)} }}")
                    new = copy.deepcopy(old)
                    new["expectations"]["syntax"]["raw"] = (
                        f"Envelope {{ unrelated: Some([{before}]), jai: {debug(mapped)} }}")
                    delta = COMPARER.compare_pair(old, new, "case.toml")
                    self.assertEqual(COMPARER.mechanical_jai(old, new, delta, "c-c"), 1)
                    new["expectations"]["syntax"]["raw"] = (
                        f"Envelope {{ unrelated: Some([{after}]), jai: {debug(mapped)} }}")
                    delta = COMPARER.compare_pair(old, new, "case.toml")
                    self.assertEqual(COMPARER.mechanical_jai(old, new, delta, "c-c"), 0)


class DebugStructureTests(unittest.TestCase):
    def test_literal_mode_is_opt_in_and_does_not_change_the_legacy_reader(self):
        for text, value in [("1", 1), ("-0.0", -0.0), ("1.0", 1.0),
                            ("1.0000000000000001", 1.0)]:
            with self.subTest(text=text):
                legacy = DebugParser(text).parse()
                self.assertIs(type(legacy), type(value))
                self.assertEqual(legacy, value)
                self.assertEqual(DebugParser(text, preserve_number_literals=True).parse(),
                                 NumberLiteral(text))
        self.assertIs(DebugParser("true", preserve_number_literals=True).parse(), True)

    def test_exact_equality_preserves_node_field_sequence_and_scalar_shapes(self):
        unequal = [
            (Form("Node"), Form("Other")),
            (Form("Node"), Form("Node", fields=())),
            (Form("Node", fields=()), Form("Node", args=())),
            (JAI.product("Node", first=1, last=2), JAI.product("Node", last=2, first=1)),
            (JAI.arm("Node", [True]), JAI.arm("Node", [1])),
            (JAI.arm("Node", (1,)), JAI.arm("Node", (1.0,))),
            ([1], (1,)), ([1], [1, 2]),
            (NumberLiteral("1"), "1"), (NumberLiteral("1"), NumberLiteral("1.0")),
        ]
        for left, right in unequal:
            with self.subTest(left=left, right=right):
                self.assertFalse(exact_equal(left, right))
                self.assertFalse(exact_equal(right, left))
                self.assertTrue(exact_equal(left, copy.deepcopy(left)))
        self.assertTrue(exact_equal({"first": [1], "last": True}, {"last": True, "first": [1]}))
        self.assertFalse(exact_equal({"nested": [float("nan")]}, {"nested": [float("nan")]}))

    def test_literal_reader_only_ignores_debug_layout_not_number_spelling(self):
        before = DebugParser('Node { span: (1, 2), flag: true }', preserve_number_literals=True).parse()
        after = DebugParser('Node{span:(1,2,),flag:true,}', preserve_number_literals=True).parse()
        self.assertTrue(exact_equal(before, after))
        for changed in ['Node { span: (1.0, 2), flag: true }',
                        'Node { span: (1, 2), flag: 1 }',
                        'Node { span: (1, 2), flag: "true" }']:
            self.assertFalse(exact_equal(before, DebugParser(changed, preserve_number_literals=True).parse()))


class JaiTranscriptionTests(unittest.TestCase):
    def word(self, text="synthetic brivla"):
        return JAI.arm("WordTanruUnit", JAI.arm("WordTanruUnitSyntax", text))

    def converted(self, se, inner):
        return JAI.arm("ConvertedJaiInnerTanruUnit", JAI.product(
            "ConvertedJaiInnerTanruUnitSyntax", se=se, inner_unit=inner))

    def scalar(self, inner):
        return JAI.arm("ScalarNegatedJaiInnerTanruUnit", JAI.product(
            "ScalarNegatedJaiInnerTanruUnitSyntax", nahe="synthetic NAhE", inner_unit=inner))

    def group(self, units, tails=()):
        leading = JAI.product("TanruJaiInnerSelbriSyntax", first_unit=units[0], additional_units=list(units[1:]))
        continuations = [JAI.product("ConnectedJaiInnerSelbriContinuationSyntax", connective="synthetic JEK",
            trailing_selbri=JAI.product("TanruJaiInnerSelbriSyntax", first_unit=right[0],
                                       additional_units=list(right[1:]))) for right in tails]
        connected = JAI.product("ConnectedJaiInnerSelbriSyntax", leading_selbri=leading, continuations=continuations)
        return JAI.arm("GroupedJaiInnerTanruUnit", JAI.product("GroupedJaiInnerTanruUnitSyntax",
            ke="synthetic KE", selbri=connected, kehe=JAI.NONE))

    def test_every_frozen_arm_is_explicit_and_same_products_are_preserved_whole(self):
        self.assertEqual(len(JAI.OLD_ARMS), 11)
        for name in JAI.SAME_PRODUCTS:
            with self.subTest(name=name):
                old = JAI.arm(name, JAI.arm(f"{name}Syntax", "synthetic complete product"))
                self.assertEqual(JAI.map_inner(old), JAI.shared_atom(old))
        for old in [JAI.arm("ProBridiTanruUnit", Form("ProBridiTanruUnitSyntax")),
                    JAI.arm("UnknownNewArm", self.word()), Form("WordTanruUnit"),
                    self.converted("se", JAI.arm("ProBridiTanruUnit", Form("ProBridiTanruUnitSyntax")))]:
            with self.assertRaises(JAI.ManualShape):
                JAI.map_inner(old)

    def test_conversion_order_and_scalar_boundary_are_not_flattened_together(self):
        word = self.word()
        self.assertEqual(JAI.map_inner(self.converted("se", self.converted("te", word))),
                         JAI.shared_atom(word, ["se", "te"]))
        old = self.converted("se", self.scalar(self.converted("te", word)))
        expected = JAI.shared_atom(JAI.arm("ScalarNegatedTanruUnit", JAI.product(
            "ScalarNegatedTanruUnitSyntax", nahe="synthetic NAhE",
            inner_unit=JAI.arm("TanruUnitAtom", JAI.shared_atom(word, ["te"])) )), ["se"])
        self.assertEqual(JAI.map_inner(old), expected)

    def test_group_pure_adjacency_and_pure_connections_map_but_mixed_precedence_does_not(self):
        left, right = self.word("left"), self.word("right")
        for old in [self.group([left]), self.group([left, right]), self.group([left], [[right]])]:
            mapped = JAI.map_inner(old)
            names = [node.name for node in JAI.nodes(mapped)]
            self.assertIn("GroupedTanruUnitSyntax", names)
            self.assertNotIn("GroupedJaiInnerTanruUnitSyntax", names)
        for old in [self.group([left, right], [[right]]), self.group([left], [[left, right]])]:
            with self.assertRaisesRegex(JAI.ManualShape, "mixes"):
                JAI.map_inner(old)

    def test_mapping_is_field_order_exact_and_not_a_blanket_name_substitution(self):
        old = JAI.arm("ConvertedJaiInnerTanruUnit", JAI.product(
            "ConvertedJaiInnerTanruUnitSyntax", inner_unit=self.word(), se="se"))
        with self.assertRaises(JAI.ManualShape):
            JAI.map_inner(old)
        unrelated = JAI.product("Unrelated", inner_unit=self.word(), text="JaiModalTanruUnitSyntax")
        self.assertEqual(JAI.rewrite(unrelated), (unrelated, 0))

    def test_transcribed_field_orders_match_the_actual_base_and_shared_grammar(self):
        import re
        import subprocess

        root = Path(__file__).parents[2]
        path = "crates/jbotci-syntax/src/grammar/generated.rs"
        base = subprocess.check_output(["git", "show", f"{COMPARER.SEMANTIC_BASE}:{path}"], cwd=root, text=True)
        current = (root / path).read_text()
        rule = re.compile(r'^    rule "[^"]*" (\w+)\([^)]*\) -> struct \{\n(.*?)\n    \}', re.S | re.M)
        field = re.compile(r'^        field (\w+) <-', re.M)
        before = {name: tuple(field.findall(body)) for name, body in rule.findall(base)}
        shared = {name: tuple(field.findall(body)) for name, body in rule.findall(current)}
        for name, expected in {
            "jai_modal_tanru_unit": JAI.JAI_FIELDS,
            "converted_jai_inner_tanru_unit": JAI.CONVERSION_FIELDS,
            "scalar_negated_jai_inner_tanru_unit": JAI.SCALAR_FIELDS,
            "grouped_jai_inner_tanru_unit": JAI.GROUP_FIELDS,
            "connected_jai_inner_selbri": JAI.OLD_CONNECTED_FIELDS,
            "tanru_jai_inner_selbri": JAI.OLD_TANRU_FIELDS,
        }.items():
            self.assertEqual(before[name], expected, name)
        for name, expected in {
            "tanru_unit_atom": JAI.ATOM_FIELDS,
            "scalar_negated_tanru_unit": JAI.SCALAR_FIELDS,
            "grouped_tanru_unit": JAI.GROUP_FIELDS,
            "linked_tanru_unit": ("base", "linkargs"),
            "tanru_unit": ("base", "assignments"),
            "plain_bo_tanru_unit": ("leading_unit", "bo_tail"),
            "bound_selbri": ("leading_selbri", "bo_tail"),
            "connected_selbri": ("leading_selbri", "continuations"),
            "simple_connected_selbri_continuation": ("connective", "trailing_selbri"),
            "tanru_selbri": ("first_selbri", "additional_selbri"),
        }.items():
            self.assertEqual(shared[name], expected, name)


if __name__ == "__main__":
    unittest.main()
