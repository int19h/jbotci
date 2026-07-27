"""Place-assignment and discourse-reference binding witnesses."""

from __future__ import annotations

import ast
import gc
import subprocess
import sys
import threading
import tomllib
from collections.abc import Iterator
from pathlib import Path
from typing import cast

import pytest

import jbotci
import jbotci._native as native
from jbotci import dialect, syntax
from jbotci.semantics import references
from jbotci.syntax import strict
from jbotci.syntax._runtime import _SyntaxNode

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
WORKSPACE_ROOT = PACKAGE_ROOT.parents[1]


# These are existing Rust reference-projection fixtures, selected to exercise
# ordinary and omitted numbered places; FA, SE, JAI, modal, connective, tanru,
# and CO propagation; all PlaceSlot payload shapes; ri/ra/ru, ke'a, go'i,
# GOI/CEI/prenex bindings; vague and unresolved targets; and nested syntax
# inside abstractions, relative clauses, free modifiers, and multiple
# paragraphs. The assertions below read the oracle values from the fixtures;
# they do not duplicate resolver expectations in Python.
REFERENCE_FIXTURE_PATHS: tuple[Path, ...] = tuple(
    WORKSPACE_ROOT / relative
    for relative in (
        "tests/fixtures/muplis/collection-18/942-canonical.toml",
        "tests/fixtures/corpus/camxes/15306.toml",
        "tests/fixtures/corpus/camxes/4499.toml",
        (
            "tests/fixtures/adhoc/v0/warnings/standard-no-warning/"
            "standard-jai-relation-unit.toml"
        ),
        "tests/fixtures/corpus/camxes/9016.toml",
        "tests/fixtures/corpus/camxes/20439.toml",
        "tests/fixtures/corpus/camxes/10180.toml",
        "tests/fixtures/corpus/camxes/14445.toml",
        "tests/fixtures/corpus/camxes/6175.toml",
        (
            "tests/fixtures/adhoc/v0/warnings/zantufa/"
            "zantufa-jai-tag-term.toml"
        ),
        "tests/fixtures/corpus/camxes/7646.toml",
        "tests/fixtures/corpus/camxes/8287.toml",
        "tests/fixtures/corpus/camxes/4741.toml",
        "tests/fixtures/corpus/camxes/12666.toml",
        "tests/fixtures/corpus/camxes/6013.toml",
        "tests/fixtures/corpus/camxes/17613.toml",
        "tests/fixtures/corpus/camxes/6263.toml",
        "tests/fixtures/corpus/camxes/14042.toml",
        "tests/fixtures/muplis/collection-18/1650-canonical.toml",
        "tests/fixtures/corpus/camxes/9830.toml",
        "tests/fixtures/corpus/camxes/6800.toml",
        "tests/fixtures/cll/chapter-06/section-6.11/c6e11d1.toml",
        (
            "tests/fixtures/adhoc/v0/syntax/basic/"
            "su-erases-to-previous-niho.toml"
        ),
    )
)

STRUCTURAL_FIXTURE_PATHS: tuple[Path, ...] = (
    WORKSPACE_ROOT
    / "tests/fixtures/muplis/collection-18/942-canonical.toml",
    WORKSPACE_ROOT
    / "tests/fixtures/adhoc/v0/syntax/basic/number-expression.toml",
    WORKSPACE_ROOT / "tests/fixtures/corpus/camxes/9830.toml",
    WORKSPACE_ROOT / "tests/fixtures/corpus/camxes/6800.toml",
    WORKSPACE_ROOT
    / "tests/fixtures/cll/chapter-06/section-6.11/c6e11d1.toml",
    WORKSPACE_ROOT
    / "tests/fixtures/adhoc/v0/syntax/basic/su-erases-to-previous-niho.toml",
)


def _fixture_fields(path: Path) -> tuple[str, str, str | None]:
    loaded = cast(object, tomllib.loads(path.read_text(encoding="utf-8")))
    if not isinstance(loaded, dict):
        raise AssertionError(f"{path} is not a TOML table")
    fixture = cast(dict[str, object], loaded)
    text = fixture.get("lojban")
    dialect_formula = fixture.get("dialect")
    expectations = fixture.get("expectations")
    if (
        not isinstance(text, str)
        or not isinstance(expectations, dict)
        or not (
            isinstance(dialect_formula, str) or dialect_formula is None
        )
    ):
        raise AssertionError(f"{path} has an unexpected fixture shape")
    semantics = cast(dict[str, object], expectations).get("semantics")
    if not isinstance(semantics, dict):
        raise AssertionError(f"{path} has no semantics table")
    refs = cast(dict[str, object], semantics).get("refs")
    if not isinstance(refs, dict):
        raise AssertionError(f"{path} has no reference projection")
    raw = cast(dict[str, object], refs).get("raw")
    if not isinstance(raw, str):
        raise AssertionError(f"{path} has no successful reference projection")
    return text, raw, dialect_formula


def _analyze_fixture(path: Path) -> references.ReferenceAnalysis:
    text, _, dialect_formula = _fixture_fields(path)
    options = None
    if dialect_formula is not None:
        options = syntax.ParseOptions.default().with_dialect(
            dialect.parse_dialect_definition(dialect_formula)
        )
    return jbotci.analyze(text, parse_options=options).reference_analysis


def _walk_syntax(value: object) -> Iterator[_SyntaxNode]:
    if isinstance(value, _SyntaxNode):
        yield value
    if isinstance(value, tuple):
        for item in value:
            yield from _walk_syntax(item)
    elif type(value).__module__.startswith("jbotci.syntax"):
        for field in getattr(type(value), "__match_args__", ()):
            yield from _walk_syntax(getattr(value, field))


@pytest.mark.parametrize(
    "fixture_path",
    REFERENCE_FIXTURE_PATHS,
    ids=lambda path: path.stem,
)
def test_typed_projection_matches_existing_rust_fixture_oracle(
    fixture_path: Path,
) -> None:
    """The typed projection and canonical JSON exactly match the Rust oracle."""

    _, expected_json, _ = _fixture_fields(fixture_path)
    analysis = _analyze_fixture(fixture_path)

    assert analysis.fixture_projection_json() == expected_json
    assert (
        analysis.fixture_projection()
        == references._fixture_projection_from_json(expected_json)
    )
    assert references.fixture_projection(analysis) == analysis.fixture_projection()
    assert (
        references.fixture_projection_json(analysis)
        == analysis.fixture_projection_json()
    )


def test_fixture_set_witnesses_payload_structural_kinds() -> None:
    projections = tuple(
        _analyze_fixture(path).fixture_projection()
        for path in REFERENCE_FIXTURE_PATHS
    )
    propagations = {
        type(frame.propagation)
        for projection in projections
        for frame in projection.frames
    }
    slots = {
        type(assignment.slot)
        for projection in projections
        for assignment in projection.assignments
    }
    targets = {
        type(edge.target)
        for projection in projections
        for edge in projection.references
    }

    assert propagations == {
        references.FixtureNoPlaceFramePropagation,
        references.FixtureForwardPlaceFramePropagation,
        references.FixtureConversionPlaceFramePropagation,
        references.FixtureJaiPlaceFramePropagation,
        references.FixtureConnectiveBranchesPlaceFramePropagation,
        references.FixtureCompoundPlaceFramePropagation,
        references.FixtureCoPlaceFramePropagation,
    }
    assert slots == {
        references.FixtureNumberedPlaceSlot,
        references.FixtureModalPlaceSlot,
        references.FixturePlaceQuestionPlaceSlot,
        references.FixtureFaiPlaceSlot,
    }
    assert {
        references.FixtureResolvedNodeReferenceTarget,
        references.FixtureUnresolvedReferenceTarget,
        references.FixtureVagueReferenceTarget,
    } <= targets


def test_index_round_trips_every_node_and_every_typed_id_family() -> None:
    seen_typed_families: set[str] = set()

    for fixture_path in STRUCTURAL_FIXTURE_PATHS:
        analysis = _analyze_fixture(fixture_path)
        index = analysis.syntax_index
        nodes = tuple(_walk_syntax(analysis.syntax))
        raw_ids = tuple(index.id_of(node) for node in nodes)

        assert all(raw_id is not None for raw_id in raw_ids)
        concrete_ids = cast(tuple[references.RawSyntaxNodeId, ...], raw_ids)
        assert {raw_id.value for raw_id in concrete_ids} == set(
            range(index.node_count())
        )

        for node, raw_id in zip(nodes, concrete_ids, strict=True):
            projected = index.node(raw_id)
            metadata = index.metadata(raw_id)
            assert projected is not None
            assert projected == node
            assert projected is not node
            assert metadata is not None
            assert metadata.id == raw_id
            assert metadata.preorder == raw_id.value
            assert metadata.leaf_start <= metadata.leaf_end
            assert index.id_of(projected) == raw_id

            if metadata.parent is None:
                assert metadata.depth == 0
            else:
                parent_metadata = index.metadata(metadata.parent)
                assert parent_metadata is not None
                assert parent_metadata.depth + 1 == metadata.depth
                assert parent_metadata.preorder < metadata.preorder

            if isinstance(node, strict.TextSyntax):
                text_id = index.text_node_id(node)
                assert text_id is not None
                assert text_id.raw_id == raw_id
                seen_typed_families.add("text")
            if isinstance(node, strict.ParagraphSyntax):
                paragraph_id = index.paragraph_node_id(node)
                assert paragraph_id is not None
                assert paragraph_id.raw_id == raw_id
                seen_typed_families.add("paragraph")
            if isinstance(node, strict.StatementSyntax):
                statement_id = index.statement_node_id(node)
                assert statement_id is not None
                assert statement_id.raw_id == raw_id
                seen_typed_families.add("statement")
            if isinstance(node, strict.BridiSyntax):
                bridi_id = index.bridi_node_id(node)
                assert bridi_id is not None
                assert bridi_id.raw_id == raw_id
                seen_typed_families.add("bridi")
            if isinstance(node, strict.BridiTailSyntax):
                bridi_tail_id = index.bridi_tail_node_id(node)
                assert bridi_tail_id is not None
                assert bridi_tail_id.raw_id == raw_id
                seen_typed_families.add("bridi-tail")
            if isinstance(node, strict.SelbriSyntax):
                selbri_id = index.selbri_node_id(node)
                assert selbri_id is not None
                assert selbri_id.raw_id == raw_id
                seen_typed_families.add("selbri")
            if isinstance(node, strict.TanruUnitSyntax):
                tanru_unit_id = index.tanru_unit_node_id(node)
                assert tanru_unit_id is not None
                assert tanru_unit_id.raw_id == raw_id
                seen_typed_families.add("tanru-unit")
            if isinstance(node, strict.TermSyntax):
                term_id = index.term_node_id(node)
                assert term_id is not None
                assert term_id.raw_id == raw_id
                seen_typed_families.add("term")
            if isinstance(node, strict.SumtiSyntax):
                sumti_id = index.sumti_node_id(node)
                assert sumti_id is not None
                assert sumti_id.raw_id == raw_id
                seen_typed_families.add("sumti")
            if isinstance(node, strict.FreeModifierSyntax):
                free_modifier_id = index.free_modifier_node_id(node)
                assert free_modifier_id is not None
                assert free_modifier_id.raw_id == raw_id
                seen_typed_families.add("free-modifier")
            if isinstance(node, strict.AbstractionTanruUnitSyntax):
                abstraction_id = index.abstraction_node_id(node)
                assert abstraction_id is not None
                assert abstraction_id.raw_id == raw_id
                seen_typed_families.add("abstraction")
            if isinstance(node, strict.MeksoSyntax):
                mekso_id = index.mekso_node_id(node)
                assert mekso_id is not None
                assert mekso_id.raw_id == raw_id
                seen_typed_families.add("mekso")
            if isinstance(node, strict.MeksoOperatorSyntax):
                mekso_operator_id = index.mekso_operator_node_id(node)
                assert mekso_operator_id is not None
                assert mekso_operator_id.raw_id == raw_id
                seen_typed_families.add("mekso-operator")

    runtime_typed_families = {
        name.removesuffix("_node_id").replace("_", "-")
        for name in dir(_analyze_fixture(STRUCTURAL_FIXTURE_PATHS[0]).syntax_index)
        if name.endswith("_node_id")
    }
    assert seen_typed_families == runtime_typed_families


def test_root_metadata_preserves_exact_spans_parent_order_and_depth() -> None:
    analysis = jbotci.analyze("mi tavla do").reference_analysis
    index = analysis.syntax_index
    root = index.root()
    metadata = index.metadata(root.raw_id)

    assert index.text_node_id(analysis.syntax) == root
    assert metadata is not None
    assert metadata.parent is None
    assert metadata.preorder == root.value
    assert metadata.depth == 0
    assert metadata.first_source_span is not None
    assert metadata.last_source_span is not None
    assert (
        metadata.first_source_span.byte_start,
        metadata.first_source_span.byte_end,
    ) == (0, 2)
    assert (
        metadata.last_source_span.byte_start,
        metadata.last_source_span.byte_end,
    ) == (9, 11)


def test_every_place_and_discourse_query_matches_owned_records() -> None:
    analysis = jbotci.analyze(
        "mi tavla do .i ko'a goi le broda cu klama .i ko'a cadzu"
    ).reference_analysis
    places = analysis.place_analysis
    frames = places.frames()
    assignments = places.assignments()

    assert frames
    assert assignments
    assert analysis.discourse_references.edges()
    assert isinstance(frames, tuple)
    assert isinstance(assignments, tuple)
    assert isinstance(analysis.discourse_references.edges(), tuple)

    frame_nodes = {frame.node for frame in frames}
    frame_nodes.add(analysis.syntax_index.root().raw_id)
    for node in frame_nodes:
        expected_frame_ids = tuple(
            frame.id for frame in frames if frame.node == node
        )
        assert places.frames_for_node(node) == expected_frame_ids
        for frame_id in expected_frame_ids:
            assert places.frame(frame_id) in frames

    for frame in frames:
        assert places.frame(frame.id) == frame
        expected_assignment_ids = tuple(
            assignment.id
            for assignment in assignments
            if assignment.frame == frame.id
        )
        assert (
            places.assignments_for_frame(frame.id)
            == expected_assignment_ids
        )

    for assignment in assignments:
        assert places.assignment(assignment.id) == assignment
        expected_sumti = tuple(
            candidate.id
            for candidate in assignments
            if candidate.sumti == assignment.sumti
        )
        assert places.assignments_for_sumti(assignment.sumti) == expected_sumti
        if assignment.term is not None:
            expected_term = tuple(
                candidate.id
                for candidate in assignments
                if candidate.term == assignment.term
            )
            assert places.assignments_for_term(assignment.term) == expected_term

        expected_slot = tuple(
            candidate.id
            for candidate in assignments
            if candidate.frame == assignment.frame
            and candidate.slot == assignment.slot
        )
        assert (
            places.assignments_for_frame_slot(
                assignment.frame, assignment.slot
            )
            == expected_slot
        )
        expected_first = None
        if expected_slot:
            first_assignment = places.assignment(expected_slot[0])
            assert first_assignment is not None
            expected_first = first_assignment.sumti
        assert (
            places.first_argument_for_place(
                assignment.frame, assignment.slot
            )
            == expected_first
        )


def test_analysis_and_every_nested_wrapper_retain_the_original_owner() -> None:
    parsed = jbotci.parse("mi tavla do .i ra cadzu")
    tree = parsed.parse_tree
    analysis = references.analyze_references(parsed)
    index = analysis.syntax_index
    node = index.node(index.root().raw_id)
    frame = analysis.place_analysis.frames()[0]
    assignment = analysis.place_analysis.assignments()[0]
    edge = analysis.discourse_references.edges()[0]
    target = edge.target

    del tree
    del parsed
    del analysis
    gc.collect()

    assert node is not None
    assert index.id_of(node) == index.root().raw_id
    assert frame.kind in references.PlaceFrameKind
    assert frame.propagation is not None
    assert assignment.sumti.value >= 0
    assert edge.rule in references.ReferenceRule
    assert isinstance(target, references.VagueReferenceTarget)
    assert target.kind in references.VagueReferenceKind


def test_analysis_scoped_ids_reject_cross_analysis_use() -> None:
    first = jbotci.analyze("mi tavla do").reference_analysis
    second = jbotci.analyze("mi tavla do").reference_analysis
    first_root = first.syntax_index.root()
    second_root = second.syntax_index.root()
    first_frame = first.place_analysis.frames()[0]
    second_frame = second.place_analysis.frames()[0]
    first_assignment = first.place_analysis.assignments()[0]

    assert first_root.value == second_root.value
    assert first_root != second_root
    assert len({first_root, second_root}) == 2
    assert first_frame.id.value == second_frame.id.value
    assert first_frame.id != second_frame.id

    with pytest.raises(jbotci.InvalidInputError):
        second.syntax_index.node(first_root.raw_id)
    with pytest.raises(jbotci.InvalidInputError):
        second.syntax_index.metadata(first_root.raw_id)
    with pytest.raises(jbotci.InvalidInputError):
        second.place_analysis.frames_for_node(first_root.raw_id)
    with pytest.raises(jbotci.InvalidInputError):
        second.place_analysis.frame(first_frame.id)
    with pytest.raises(jbotci.InvalidInputError):
        second.place_analysis.assignment(first_assignment.id)
    with pytest.raises(jbotci.InvalidInputError):
        second.place_analysis.assignments_for_sumti(first_assignment.sumti)
    with pytest.raises(jbotci.InvalidInputError):
        second.syntax_index.id_of(first.syntax)


def test_ids_are_distinct_immutable_non_integer_types() -> None:
    analysis = jbotci.analyze("mi tavla do").reference_analysis
    root = analysis.syntax_index.root()
    frame = analysis.place_analysis.frames()[0]
    assignment = analysis.place_analysis.assignments()[0]

    assert not isinstance(root, int)
    assert not isinstance(root.raw_id, int)
    assert len({root.__class__, root.raw_id.__class__}) == 2
    assert len({frame.id.__class__, assignment.id.__class__}) == 2
    with pytest.raises(TypeError):
        analysis.syntax_index.node(root.value)  # type: ignore[arg-type]
    with pytest.raises(AttributeError):
        root.value = 1  # type: ignore[misc]


def test_payload_values_follow_rust_equality_and_hashability() -> None:
    analysis = jbotci.analyze("mi tavla do .i ra cadzu").reference_analysis
    frame = analysis.place_analysis.frames()[0]
    edge = analysis.discourse_references.edges()[0]

    first_propagation = frame.propagation
    second_propagation = frame.propagation
    assert first_propagation == second_propagation
    assert first_propagation is not second_propagation
    with pytest.raises(TypeError):
        hash(first_propagation)

    first_target = edge.target
    second_target = edge.target
    assert first_target == second_target
    assert first_target is not second_target
    with pytest.raises(TypeError):
        hash(first_target)


def test_high_level_analyze_retains_parse_products_without_tersmu_surface() -> None:
    result = jbotci.analyze("mi tavla do", source_id="reference-test")

    assert isinstance(result, jbotci.AnalyzedText)
    assert result.parsed.syntax is result.syntax
    assert result.parsed.words is result.words
    assert result.parse_tree == result.reference_analysis.syntax
    assert result.warnings == result.syntax.warnings
    assert result.reference_analysis.syntax_index.node_count() > 0
    for forbidden in ("model", "builder", "render_tree", "render_tree_proj"):
        assert not hasattr(result, forbidden)
        assert not hasattr(result.reference_analysis, forbidden)
        assert forbidden not in references.__all__


def test_reference_analysis_releases_gil_during_core_analysis() -> None:
    parsed = jbotci.parse(".i ".join(("mi tavla do",) * 100))
    ready = threading.Event()
    stop = threading.Event()
    counter = 0

    def worker() -> None:
        nonlocal counter
        ready.set()
        while not stop.is_set():
            counter += 1
            if counter % 100 == 0:
                threading.Event().wait(0)

    thread = threading.Thread(target=worker)
    thread.start()
    assert ready.wait(timeout=1)
    previous_interval = sys.getswitchinterval()
    try:
        sys.setswitchinterval(1.0)
        before = counter
        references.analyze_references(parsed)
        after = counter
    finally:
        sys.setswitchinterval(previous_interval)
        stop.set()
        thread.join(timeout=1)

    assert after > before


def test_runtime_reference_inventory_matches_composed_stub_inventory() -> None:
    inventory = native._references_RUNTIME_INVENTORY
    assert inventory == tuple(
        name for name in native.__all__ if name.startswith("_references_")
    )

    stub_path = PACKAGE_ROOT / "python" / "jbotci" / "_native.pyi"
    tree = ast.parse(stub_path.read_text(encoding="utf-8"), filename=str(stub_path))
    declarations: set[str] = set()
    for declaration in tree.body:
        if isinstance(declaration, (ast.ClassDef, ast.FunctionDef)):
            declarations.add(declaration.name)
        elif (
            isinstance(declaration, ast.AnnAssign)
            and isinstance(declaration.target, ast.Name)
        ):
            declarations.add(declaration.target.id)
    assert set(inventory) == {
        name for name in declarations if name.startswith("_references_")
    }

    for name in inventory:
        value = getattr(native, name)
        if isinstance(value, type):
            assert getattr(references, name.removeprefix("_references_")) is value


def test_error_hierarchy_is_closed_and_structured() -> None:
    assert issubclass(
        references.MissingRootNodeError, references.ReferenceAnalysisError
    )
    with pytest.raises(TypeError):
        references.ReferenceAnalysisError(
            cast(references.ReferenceAnalysisErrorValue, object())
        )

    with pytest.raises(TypeError):

        class InvalidReferenceError(references.ReferenceAnalysisError):
            pass


def test_installed_reference_import_works_outside_repository(
    tmp_path: Path,
) -> None:
    result = subprocess.run(
        [
            sys.executable,
            "-c",
            (
                "import jbotci; "
                "from jbotci.semantics import references; "
                "result = jbotci.analyze('mi tavla do'); "
                "analysis = references.analyze_references(result.syntax); "
                "assert analysis.syntax_index.root().value >= 0; "
                "assert analysis.place_analysis.assignments()"
            ),
        ],
        cwd=tmp_path,
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stdout + result.stderr
