"""Generated strict and recovered syntax-model coverage."""

from __future__ import annotations

import ast
import gc
import inspect
import subprocess
import sys
from pathlib import Path

import pytest

import jbotci._native as native
from jbotci import dialect, morphology, source, syntax
from jbotci.syntax import recovered, strict

PACKAGE_ROOT = Path(__file__).resolve().parents[1]


def _union_members(expression: ast.expr) -> tuple[str, ...]:
    if isinstance(expression, ast.Name):
        return (expression.id,)
    assert isinstance(expression, ast.BinOp)
    assert isinstance(expression.op, ast.BitOr)
    return _union_members(expression.left) + _union_members(expression.right)


def _literal_match_args(annotation: ast.expr) -> tuple[str, ...]:
    assert isinstance(annotation, ast.Subscript)
    assert isinstance(annotation.value, ast.Name)
    assert annotation.value.id == "ClassVar"
    tuple_annotation = annotation.slice
    assert isinstance(tuple_annotation, ast.Subscript)
    assert isinstance(tuple_annotation.value, ast.Name)
    assert tuple_annotation.value.id == "tuple"
    elements = (
        tuple_annotation.slice.elts
        if isinstance(tuple_annotation.slice, ast.Tuple)
        else (tuple_annotation.slice,)
    )
    values: list[str] = []
    for element in elements:
        assert isinstance(element, ast.Subscript)
        assert isinstance(element.value, ast.Name)
        assert element.value.id == "Literal"
        assert isinstance(element.slice, ast.Constant)
        assert isinstance(element.slice.value, str)
        values.append(element.slice.value)
    return tuple(values)


def _tokens(text: str) -> tuple[syntax.Token, ...]:
    words = morphology.segment(text)
    return tuple(
        syntax.Token(syntax.PlainWithIndicators(word_like)) for word_like in words
    )


def _token(text: str) -> syntax.Token:
    tokens = _tokens(text)
    assert len(tokens) == 1
    return tokens[0]


def _plain_word(text: str) -> morphology.Word:
    word_like = morphology.segment(text)
    assert len(word_like) == 1
    assert isinstance(word_like[0], morphology.PlainWord)
    return word_like[0].word


def test_schema_inventory_and_every_generated_class_are_exact() -> None:
    assert native._syntax_SCHEMA_MODEL_COUNT == 455
    assert native._syntax_SCHEMA_VARIANT_COUNT == 355
    assert native._syntax_SCHEMA_FIELD_COUNT == 1265
    assert strict.__all__ == native._syntax_STRICT_INVENTORY
    assert recovered.__all__ == native._syntax_RECOVERED_INVENTORY
    assert len(strict.__all__) == len(recovered.__all__) == 810

    namespaces = (
        (strict, native._syntax_STRICT_CONCRETE_INVENTORY),
        (recovered, native._syntax_RECOVERED_CONCRETE_INVENTORY),
    )
    for module, concrete_names in namespaces:
        assert len(concrete_names) == 721
        assert len(set(module.__all__) - set(concrete_names)) == 89
        concrete: list[type[object]] = [
            getattr(module, name) for name in concrete_names
        ]
        assert len(concrete) == 721
        assert all(inspect.isclass(cls) for cls in concrete)

        for cls in concrete:
            assert cls.__module__ == module.__name__
            assert cls.__doc__ is not None and cls.__doc__.strip()
            match_args = getattr(cls, "__match_args__")
            assert isinstance(match_args, tuple)
            assert all(isinstance(field_name, str) for field_name in match_args)
            signature = inspect.signature(cls)
            assert tuple(signature.parameters) == match_args
            assert all(
                parameter.default is inspect.Parameter.empty
                for parameter in signature.parameters.values()
            )
            assert len(set(match_args)) == len(match_args)
            for field_name in match_args:
                assert isinstance(field_name, str)
                member = inspect.getattr_static(cls, field_name)
                assert isinstance(member, property)
                assert member.__doc__ is not None and member.__doc__.strip()


@pytest.mark.parametrize(
    ("module", "stub_name", "inventory", "concrete_inventory"),
    [
        (
            strict,
            "strict.pyi",
            native._syntax_STRICT_INVENTORY,
            native._syntax_STRICT_CONCRETE_INVENTORY,
        ),
        (
            recovered,
            "recovered.pyi",
            native._syntax_RECOVERED_INVENTORY,
            native._syntax_RECOVERED_CONCRETE_INVENTORY,
        ),
    ],
)
def test_generated_stub_inventory_docs_members_and_signatures_are_exact(
    module: object,
    stub_name: str,
    inventory: tuple[str, ...],
    concrete_inventory: tuple[str, ...],
) -> None:
    stub_path = PACKAGE_ROOT / "python" / "jbotci" / "syntax" / stub_name
    tree = ast.parse(stub_path.read_text(encoding="utf-8"), filename=str(stub_path))
    declarations: list[str] = []
    classes: dict[str, ast.ClassDef] = {}
    aliases: dict[str, ast.AnnAssign] = {}
    for statement in tree.body:
        if isinstance(statement, ast.ClassDef):
            declarations.append(statement.name)
            classes[statement.name] = statement
        elif (
            isinstance(statement, ast.AnnAssign)
            and isinstance(statement.target, ast.Name)
            and isinstance(statement.annotation, ast.Name)
            and statement.annotation.id == "TypeAlias"
        ):
            declarations.append(statement.target.id)
            aliases[statement.target.id] = statement

    assert tuple(declarations) == inventory
    assert tuple(classes) == concrete_inventory
    assert set(aliases) == set(inventory) - set(concrete_inventory)

    for class_name, declaration in classes.items():
        runtime_class = getattr(module, class_name)
        assert [ast.unparse(value) for value in declaration.decorator_list] == ["final"]
        assert (
            isinstance(declaration.body[0], ast.Expr)
            and isinstance(declaration.body[0].value, ast.Constant)
            and declaration.body[0].value.value == runtime_class.__doc__
        )
        functions = {
            statement.name: statement
            for statement in declaration.body
            if isinstance(statement, ast.FunctionDef)
        }
        annotations = {
            statement.target.id: statement
            for statement in declaration.body
            if isinstance(statement, ast.AnnAssign)
            and isinstance(statement.target, ast.Name)
        }
        match_args = runtime_class.__match_args__
        assert _literal_match_args(annotations["__match_args__"].annotation) == match_args
        assert ast.unparse(annotations["__hash__"].annotation) == "ClassVar[None]"
        constructor = functions["__new__"]
        assert tuple(argument.arg for argument in constructor.args.args) == (
            "cls",
            *match_args,
        )
        assert constructor.args.defaults == []
        properties = {
            name
            for name, function in functions.items()
            if any(
                isinstance(decorator, ast.Name) and decorator.id == "property"
                for decorator in function.decorator_list
            )
        }
        assert properties == set(match_args)
        assert set(functions) == {
            "__new__",
            *match_args,
            "same_identity",
            "__repr__",
            "__eq__",
        }

    for alias_name, declaration in aliases.items():
        assert declaration.value is not None
        members = _union_members(declaration.value)
        assert members
        assert all(member in classes for member in members)
        assert all(member.startswith(alias_name) for member in members)


def test_manual_syntax_leaf_docs_members_signatures_and_match_args_are_exact() -> None:
    expected: dict[type[object], tuple[str, ...]] = {
        syntax.Token: ("indicators",),
        syntax.PlainWithIndicators: ("word_like",),
        syntax.EmphasizedWithIndicators: ("bahe", "extra_bahe", "word_like"),
        syntax.IndicatorWithIndicators: (
            "base",
            "indicator_bahe",
            "indicator",
            "nai_bahe",
            "nai",
        ),
        syntax.SkippedTokens: ("error_index", "tokens"),
        syntax.MissingRequiredField: ("error_index", "span", "expected"),
    }
    for cls, match_args in expected.items():
        assert cls.__doc__ is not None and cls.__doc__.strip()
        assert cls.__match_args__ == match_args
        signature = inspect.signature(cls)
        assert tuple(signature.parameters) == match_args
        assert all(
            parameter.default is inspect.Parameter.empty
            for parameter in signature.parameters.values()
        )
        for field_name in match_args:
            member = inspect.getattr_static(cls, field_name)
            assert isinstance(member, property)
            assert member.__doc__ is not None and member.__doc__.strip()


def test_unit_product_tuple_variant_nested_identity_and_lifetime() -> None:
    empty = strict.EmptyLinkedSumtiSyntax()
    linked = strict.LinkedSumtiSyntaxEmptyLinkedSumti(empty)

    match linked:
        case strict.LinkedSumtiSyntaxEmptyLinkedSumti(payload):
            assert payload == empty
        case _:
            pytest.fail("closed LinkedSumtiSyntax variant did not pattern-match")

    first = linked.empty_linked_sumti
    second = linked.empty_linked_sumti
    assert first == second == empty
    assert first is not second
    assert first.same_identity(second)
    assert not first.same_identity(empty)

    del linked
    gc.collect()
    assert first == strict.EmptyLinkedSumtiSyntax()
    assert repr(first).startswith("jbotci.syntax.strict.EmptyLinkedSumtiSyntax(")


def test_optional_repeated_nonempty_and_deterministic_projection_cost() -> None:
    ui = _token("ui")
    leading = strict.LeadingIndicatorSyntax(ui, None)
    assert leading.indicator.same_identity(ui)
    assert leading.nai is None
    with pytest.raises(ValueError):
        strict.LeadingIndicatorSyntax(_token("mi"), None)

    niho = _tokens(" ".join(["ni'o"] * 128))
    paragraphs = tuple(
        strict.NihoParagraphSyntax((token,), (), None) for token in niho
    )
    large_tree = strict.TextNihoParagraphsSyntax(paragraphs)
    assert large_tree._debug_projection_count() == 0
    first_projection = large_tree.paragraphs
    assert len(first_projection) == 128
    operations_per_getter = len(first_projection) + 1
    assert large_tree._debug_projection_count() == operations_per_getter
    for getter_count in range(2, 34):
        projected = large_tree.paragraphs
        assert (
            large_tree._debug_projection_count()
            == operations_per_getter * getter_count
        )
        assert all(
            current.same_identity(previous)
            for current, previous in zip(projected, first_projection, strict=True)
        )

    with pytest.raises(ValueError, match="at least one"):
        strict.NihoParagraphSyntax((), (), None)
    with pytest.raises(TypeError):
        strict.NihoParagraphSyntax((object(),), (), None)  # type: ignore[arg-type]


def test_token_owner_bridge_spans_quotations_and_equal_span_siblings() -> None:
    word_like = morphology.segment("zo broda")[0]
    first = syntax.Token(syntax.PlainWithIndicators(word_like))
    second = syntax.Token(syntax.PlainWithIndicators(word_like))

    assert isinstance(first.core_word, morphology.QuotedWord)
    assert first.source_spans == word_like.source_spans
    assert first == second
    assert not first.same_identity(second)
    indicators = first.indicators
    assert isinstance(indicators, syntax.PlainWithIndicators)
    assert indicators.word_like == word_like

    expansion = dialect.DialectDefinition(
        (dialect.CmavoExpansion("coi", ("coi", "ro", "do")),)
    )
    expanded_words = morphology.segment(
        "coi", options=morphology.MorphologyOptions(dialect=expansion)
    )
    expanded_tokens = tuple(
        syntax.Token(syntax.PlainWithIndicators(value)) for value in expanded_words
    )
    assert len(expanded_tokens) == 3
    assert all(
        token.source_spans == expanded_tokens[0].source_spans
        for token in expanded_tokens
    )
    assert all(
        not left.same_identity(right)
        for index, left in enumerate(expanded_tokens)
        for right in expanded_tokens[index + 1 :]
    )


def test_all_with_indicators_variants_validate_and_preserve_typed_children() -> None:
    base_word_like = morphology.segment("mi")[0]
    base = syntax.PlainWithIndicators(base_word_like)
    emphasized = syntax.EmphasizedWithIndicators(
        _plain_word("ba'e"), (), base_word_like
    )
    indicated = syntax.IndicatorWithIndicators(
        base, (), _plain_word("ui"), (), _plain_word("nai")
    )

    assert emphasized.word_like == base_word_like
    assert emphasized.bahe == _plain_word("ba'e")
    assert indicated.base == base
    assert indicated.indicator == _plain_word("ui")
    assert indicated.nai == _plain_word("nai")
    token = syntax.Token(indicated)
    projected = token.indicators
    projected_again = token.indicators
    assert isinstance(projected, syntax.IndicatorWithIndicators)
    assert projected == indicated
    assert projected.same_identity(projected_again)
    assert not projected.same_identity(indicated)

    strict_word = strict.WordTanruUnitSyntax(
        syntax.WithFreeModifiers(indicated, ())
    )
    strict_projected = strict_word.word.value
    strict_projected_again = strict_word.word.value
    assert isinstance(strict_projected, syntax.IndicatorWithIndicators)
    assert strict_projected == indicated
    assert strict_projected.same_identity(strict_projected_again)
    assert not strict_projected.same_identity(indicated)

    recovered_word = recovered.WordTanruUnitSyntax(
        syntax.WithFreeModifiers(syntax.RecoveredValid(indicated), ())
    )
    recovered_projected = recovered_word.word.value.value
    recovered_projected_again = recovered_word.word.value.value
    assert isinstance(recovered_projected, syntax.IndicatorWithIndicators)
    assert recovered_projected == indicated
    assert recovered_projected.same_identity(recovered_projected_again)
    assert not recovered_projected.same_identity(strict_projected)

    del strict_word
    del recovered_word
    gc.collect()
    assert strict_projected.word_like == base_word_like
    assert recovered_projected.word_like == base_word_like

    with pytest.raises(ValueError, match="BAhE"):
        syntax.EmphasizedWithIndicators(_plain_word("mi"), (), base_word_like)
    with pytest.raises(ValueError, match="indicator"):
        syntax.IndicatorWithIndicators(
            base, (), _plain_word("mi"), (), None
        )


def test_with_free_modifiers_and_generated_token_projection_remain_typed() -> None:
    bei = _token("bei")
    decorated = syntax.WithFreeModifiers(bei, [])
    empty = strict.EmptyLinkedSumtiSyntax()
    linked = strict.LinkedSumtiSyntaxEmptyLinkedSumti(empty)
    bei_link = strict.BeiLinkSyntax(decorated, linked)

    assert isinstance(bei_link.bei, syntax.WithFreeModifiers)
    assert bei_link.bei.free_modifiers == ()
    assert isinstance(bei_link.bei.value, syntax.Token)
    assert bei_link.bei.value.same_identity(bei)
    assert bei_link.link == linked
    assert bei_link.bei is not bei_link.bei
    assert bei_link.bei.same_identity(bei_link.bei)


def test_recovered_valid_error_prefix_and_recovery_item_validation() -> None:
    ui = _token("ui")
    empty_span = source.SourceSpan(0, 0, 0, 0)
    missing = syntax.MissingRequiredField(7, empty_span, "UI or CAI")
    skipped = syntax.SkippedTokens(8, [ui])
    assert skipped.tokens[0].same_identity(ui)

    valid = recovered.LeadingIndicatorSyntax(syntax.RecoveredValid(ui), None)
    error = recovered.LeadingIndicatorSyntax(syntax.RecoveredError(missing), None)
    prefix = recovered.LeadingIndicatorSyntax(
        syntax.RecoveredPrefix([skipped], ui), None
    )

    assert isinstance(valid.indicator, syntax.RecoveredValid)
    assert valid.indicator.value.same_identity(ui)
    assert isinstance(error.indicator, syntax.RecoveredError)
    assert error.indicator.error == missing
    assert isinstance(prefix.indicator, syntax.RecoveredPrefix)
    assert prefix.indicator.errors == (skipped,)
    assert prefix.indicator.value.same_identity(ui)
    assert prefix.indicator.same_identity(prefix.indicator)

    with pytest.raises(ValueError, match="at least one"):
        syntax.RecoveredPrefix([], ui)
    with pytest.raises(TypeError, match="SyntaxRecoveryItem"):
        syntax.RecoveredError(object())  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="at least one"):
        syntax.SkippedTokens(0, [])
    with pytest.raises(ValueError, match="must be empty"):
        syntax.MissingRequiredField(0, ui.source_spans[0], "value")


def test_generated_values_reject_mutation_subclassing_and_bad_shapes() -> None:
    empty = strict.EmptyLinkedSumtiSyntax()
    with pytest.raises(AttributeError, match="immutable"):
        empty.extra = 1  # type: ignore[attr-defined]
    with pytest.raises(TypeError):
        hash(empty)
    with pytest.raises(TypeError):
        class InvalidSubclass(strict.EmptyLinkedSumtiSyntax):  # type: ignore[misc]
            pass

    with pytest.raises(TypeError):
        strict.EmptyLinkedSumtiSyntax(object())  # type: ignore[call-arg]
    with pytest.raises(TypeError):
        strict.LinkedSumtiSyntaxEmptyLinkedSumti(object())  # type: ignore[arg-type]

    plain = syntax.PlainWithIndicators(morphology.segment("mi")[0])
    with pytest.raises(AttributeError):
        plain.word_like = morphology.segment("do")[0]  # type: ignore[misc]
    with pytest.raises(TypeError):
        hash(plain)
    with pytest.raises(TypeError):
        class InvalidLeafSubclass(syntax.PlainWithIndicators):  # type: ignore[misc]
            pass


def test_constructible_values_round_trip_structurally_without_owner_aliasing() -> None:
    original_child = strict.EmptyLinkedSumtiSyntax()
    original = strict.LinkedSumtiSyntaxEmptyLinkedSumti(original_child)
    projected_child = original.empty_linked_sumti
    rebuilt = strict.LinkedSumtiSyntaxEmptyLinkedSumti(projected_child)

    assert rebuilt == original
    assert not rebuilt.same_identity(original)
    assert rebuilt.empty_linked_sumti == projected_child
    assert not rebuilt.empty_linked_sumti.same_identity(projected_child)


def test_generated_namespace_imports_outside_repository(tmp_path: Path) -> None:
    result = subprocess.run(
        [
            sys.executable,
            "-c",
            (
                "from jbotci.syntax import strict, recovered; "
                "assert strict.EmptyLinkedSumtiSyntax(); "
                "assert recovered.EmptyLinkedSumtiSyntax()"
            ),
        ],
        cwd=tmp_path,
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stdout + result.stderr
