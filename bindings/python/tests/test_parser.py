from __future__ import annotations

import enum
import gc
from collections.abc import Callable, Iterator

import pytest

import jbotci
import jbotci._native as native
from jbotci import diagnostics, dialect, morphology, source, syntax


def _expected_tokens(
    expectations: tuple[syntax.SyntaxExpectation, ...],
) -> Iterator[syntax.SyntaxExpectedToken]:
    for expectation in expectations:
        yield from expectation.tokens


def _recovery_items(value: object) -> Iterator[syntax.SyntaxRecoveryItem]:
    """Visit native recovery payloads through the generated projection surface."""

    if isinstance(value, (syntax.SkippedTokens, syntax.MissingRequiredField)):
        yield value
        return
    if isinstance(value, tuple):
        for item in value:
            yield from _recovery_items(item)
        return
    if value is None or isinstance(value, (str, bytes, int, float, bool, enum.Enum)):
        return
    for field in getattr(type(value), "__match_args__", ()):
        yield from _recovery_items(getattr(value, field))


def test_parse_options_use_rust_defaults_and_checked_copying_updates() -> None:
    defaults = syntax.ParseOptions.default()
    constructed = syntax.ParseOptions()

    assert constructed.error_context_depth == defaults.error_context_depth == 1
    assert constructed.max_recovery_errors == defaults.max_recovery_errors == 128
    assert constructed.dialect == defaults.dialect
    assert constructed.trace == defaults.trace

    trace = diagnostics.TraceOptions(
        enabled=True,
        phase=diagnostics.TracePhase.SYNTAX,
    )
    changed = (
        defaults.with_trace(trace)
        .with_error_context_depth(4)
        .with_max_recovery_errors(7)
    )
    assert changed.trace == trace
    assert changed.error_context_depth == 4
    assert changed.max_recovery_errors == 7
    assert defaults.error_context_depth == 1
    assert defaults.max_recovery_errors == 128

    operations: tuple[Callable[[], object], ...] = (
        lambda: syntax.ParseOptions(error_context_depth=-1),
        lambda: syntax.ParseOptions(max_recovery_errors=0),
        lambda: defaults.with_error_context_depth(-1),
        lambda: defaults.with_max_recovery_errors(0),
    )
    for operation in operations:
        with pytest.raises(jbotci.InvalidInputError):
            operation()


def test_native_parser_enum_inventory_is_complete_and_generated() -> None:
    assert syntax.PARSER_ENUM_INVENTORY
    for public_name in syntax.PARSER_ENUM_INVENTORY:
        public_type = getattr(syntax, public_name)
        native_type = getattr(native, f"_syntax_parser_{public_name}")
        assert public_type is native_type
        assert issubclass(public_type, enum.StrEnum)
        assert tuple(public_type.__members__) == tuple(native_type.__members__)
        assert tuple(public_type) == tuple(native_type)


@pytest.mark.parametrize(
    "text",
    (
        "mi tavla do",
        "mi tavla do .i do tavla mi",
        "mi cusku lu do klama li'u",
        "mi cusku lu do klama li'u .i do tavla mi",
    ),
)
def test_root_strict_parse_covers_nested_multi_sentence_and_quotation_text(
    text: str,
) -> None:
    parsed = jbotci.parse(text)

    assert isinstance(parsed, jbotci.ParsedText)
    assert isinstance(parsed.syntax, syntax.SyntaxParse)
    assert isinstance(parsed.parse_tree, syntax.strict.TextSyntaxRegularText)
    assert parsed.source == text
    assert parsed.warnings == ()


def test_low_level_parser_consumes_word_handles_and_strict_success_stays_strict() -> None:
    text = "mi tavla do"
    words = morphology.segment(text)

    direct = syntax.parse_text(words)
    attempt = syntax.parse_syntax_tree_attempt(words, source_text=text)
    assert attempt.succeeded
    assert attempt.error is None
    assert attempt.result is not None
    assert syntax.syntax_tree_eq_ignoring_spans(direct, attempt.result.parse_tree)

    recovery = syntax.parse_syntax_tree_with_recovery(words, source_text=text)
    assert isinstance(recovery, syntax.SyntaxRecoveryParseValid)
    assert isinstance(recovery.parse.parse_tree, syntax.strict.TextSyntaxRegularText)


def test_source_ids_and_non_ascii_spans_remain_attached_to_parser_tokens() -> None:
    parsed = jbotci.parse("mí tavla do", source_id="non-ascii")
    spans = tuple(span for word in parsed.words for span in word.source_spans)

    assert parsed.source_id == source.SourceId("non-ascii")
    assert tuple(span.byte_range for span in spans) == ((0, 3), (4, 9), (10, 12))
    assert tuple(span.char_range for span in spans) == ((0, 2), (3, 8), (9, 11))
    assert all(span.source_id == parsed.source_id for span in spans)

    tokens = syntax.normalize_syntax_tokens(parsed.words)
    assert tuple(token.source_spans[0] for token in tokens) == spans


def test_dialect_expansion_preserves_equal_span_sibling_identity_and_owner() -> None:
    definition = dialect.DialectDefinition(
        (dialect.CmavoExpansion("coi", ("coi", "coi")),)
    )
    parsed = jbotci.parse(
        "coi",
        morphology_options=morphology.MorphologyOptions(dialect=definition),
    )
    assert len(parsed.words) == 2
    assert parsed.words[0].source_spans == parsed.words[1].source_spans

    root = parsed.parse_tree
    assert isinstance(root, syntax.strict.TextSyntaxRegularText)
    first_regular = root.regular_text
    second_regular = root.regular_text
    assert first_regular.same_identity(second_regular)
    free_modifier = first_regular.leading_free_modifiers[0]
    assert isinstance(
        free_modifier, syntax.strict.FreeModifierSyntaxVocativeFreeModifier
    )
    marker_words = free_modifier.vocative_free_modifier.vocative_markers.value
    assert isinstance(
        marker_words, syntax.strict.VocativeMarkerWordsSyntaxCoiVocativeMarkerWords
    )
    markers = marker_words.coi_vocative_marker_words
    first = markers.first_coi
    additional = markers.additional_coi[0].coi
    assert first.source_spans == additional.source_spans
    assert not first.same_identity(additional)
    assert markers.first_coi.same_identity(first)
    assert markers.additional_coi[0].coi.same_identity(additional)


def test_projected_strict_and_recovered_values_retain_owner_lifetimes() -> None:
    parsed = jbotci.parse("mi tavla do", source_id="lifetime")
    root = parsed.parse_tree
    assert isinstance(root, syntax.strict.TextSyntaxRegularText)
    regular = root.regular_text
    paragraphs = regular.paragraphs
    assert paragraphs is not None
    word = parsed.words[0]
    del parsed, root
    gc.collect()

    retained_paragraphs = regular.paragraphs
    assert retained_paragraphs is not None
    assert retained_paragraphs.same_identity(paragraphs)
    assert word.source_spans[0].source_id == source.SourceId("lifetime")

    recovered_result = jbotci.parse_recovered("mi tavla vau vau do")
    recovered_root = recovered_result.parse_tree
    assert isinstance(recovered_root, syntax.recovered.TextSyntaxRegularText)
    recovered_regular = recovered_root.regular_text
    del recovered_result, recovered_root
    gc.collect()

    assert isinstance(recovered_regular, syntax.RecoveredValid)
    assert recovered_regular.value.paragraphs is not None
    assert tuple(_recovery_items(recovered_regular))


def test_warnings_diagnostics_displays_and_syntax_traces_remain_typed() -> None:
    text = "mi'ai klama"
    options = syntax.ParseOptions().with_trace(
        diagnostics.TraceOptions(
            enabled=True,
            phase=diagnostics.TracePhase.SYNTAX,
        )
    )
    parsed = jbotci.parse(text, parse_options=options, source_id="warning")

    assert parsed.syntax_trace is not None
    assert parsed.syntax_trace.events
    assert parsed.syntax_trace.failure is None
    (warning,) = parsed.warnings
    assert warning.kind is syntax.ExperimentalConstruct.EXPERIMENTAL_CMAVO
    assert warning.code == "syntax.warning.experimental-cmavo"
    assert warning.anchor_index == 0
    assert warning.anchor.source_spans[0].char_range == (0, 5)
    diagnostic = warning.to_diagnostic(text, parsed.source_id)
    assert diagnostic.code == warning.code
    assert diagnostic.labels[0].span.char_range == (0, 5)

    tokens = syntax.normalize_syntax_tokens(parsed.words)
    displays = syntax.syntax_warning_displays("warning", text, tokens, parsed.warnings)
    assert displays == (
        syntax.syntax_warning_display("warning", text, tokens, warning),
    )
    (display,) = displays
    assert display.source_label == "warning"
    assert display.kind is warning.kind
    assert display.selection_start == 0
    assert display.selection_length == 5
    assert display.experimental_cmavo == "mi'ai"
    assert "👉" in display.context


def test_strict_failure_exception_matches_non_raising_attempt() -> None:
    text = "mi tavla vau vau do"
    words = morphology.segment(text)
    options = syntax.ParseOptions().with_trace(
        diagnostics.TraceOptions(
            enabled=True,
            phase=diagnostics.TracePhase.SYNTAX,
        )
    )
    attempt = syntax.parse_syntax_tree_attempt(
        words,
        source_text=text,
        options=options,
    )

    assert not attempt.succeeded
    assert attempt.result is None
    assert isinstance(attempt.error, syntax.SyntaxErrorParse)
    assert attempt.trace is not None
    assert attempt.trace.events
    with pytest.raises(syntax.SyntaxError) as caught:
        syntax.parse_syntax_tree(
            words,
            source_text=text,
            source_id=source.SourceId("bad"),
            options=options,
        )

    assert caught.value.value == attempt.error
    assert caught.value.code == attempt.error.code
    assert caught.value.source_id == source.SourceId("bad")
    assert caught.value.trace == attempt.trace
    assert caught.value.diagnostic is not None
    assert caught.value.spans == tuple(
        label.span for label in caught.value.diagnostic.labels
    )


def test_recovered_parse_exposes_exact_error_offsets_and_native_slot_indices() -> None:
    text = "mi tavla vau vau do"
    recovered = syntax.parse_syntax_tree_recovered(
        morphology.segment(text),
        source_text=text,
    )

    assert tuple(
        (error.kind, error.byte_start, error.byte_end)
        for error in recovered.errors
        if isinstance(error, syntax.SyntaxErrorParse)
    ) == (
        (syntax.SyntaxErrorKind.UNEXPECTED_CMAVO, 13, 16),
        (syntax.SyntaxErrorKind.UNEXPECTED_CMAVO, 17, 19),
    )
    slots = tuple(_recovery_items(recovered.parse_tree))
    missing = tuple(
        item for item in slots if isinstance(item, syntax.MissingRequiredField)
    )
    skipped = tuple(item for item in slots if isinstance(item, syntax.SkippedTokens))
    assert {
        (item.error_index, item.span.char_range, item.expected) for item in missing
    } == {
        (0, (13, 13), "bridi_tail_bo_continuation"),
    }
    assert tuple(
        (
            item.error_index,
            tuple(token.source_spans[0].char_range for token in item.tokens),
        )
        for item in skipped
    ) == ((1, ((17, 19),)),)


def test_eof_and_quote_failures_keep_structured_payloads() -> None:
    eof_text = "mi klama le"
    with pytest.raises(syntax.SyntaxError) as caught:
        jbotci.parse(eof_text)
    error = caught.value.value
    assert isinstance(error, syntax.SyntaxErrorParse)
    assert error.kind is syntax.SyntaxErrorKind.INCOMPLETE_SUMTI
    assert (error.byte_start, error.byte_end) == (len(eof_text), len(eof_text))
    assert error.contexts[-1].construct == "description"

    quote_text = "mi cusku zoi gy unclosed"
    with pytest.raises(morphology.MorphologyError) as quote:
        jbotci.parse(quote_text)
    assert isinstance(quote.value.value, morphology.UnterminatedZoiQuote)

    recovered = jbotci.parse_recovered(quote_text)
    assert recovered.morphology_errors
    assert isinstance(recovered.morphology_errors[0], morphology.UnterminatedZoiQuote)


def test_completion_returns_literal_typed_expectations() -> None:
    empty = syntax.expected_continuations_for_text("")
    empty_tokens = tuple(_expected_tokens(empty))
    assert syntax.SyntaxExpectedTokenCmavo(morphology.Cmavo.NAI) in empty_tokens
    assert (
        syntax.SyntaxExpectedTokenWordCategory(syntax.SyntaxWordCategory.CMEVLA)
        in empty_tokens
    )
    assert syntax.SyntaxExpectedTokenSelmaho(morphology.Selmaho.UI) in empty_tokens
    assert (
        syntax.SyntaxExpectedTokenWordCategory(
            syntax.SyntaxWordCategory.SELBRI_WORD
        )
        in empty_tokens
    )
    assert syntax.SyntaxExpectedTokenCmavo(morphology.Cmavo.VAU) not in empty_tokens

    description = syntax.expected_continuations_for_text("mi klama le")
    description_tokens = tuple(_expected_tokens(description))
    assert (
        syntax.SyntaxExpectedTokenWordCategory(
            syntax.SyntaxWordCategory.SELBRI_WORD
        )
        in description_tokens
    )
    assert syntax.SyntaxExpectedTokenCmavo(morphology.Cmavo.I) not in description_tokens


def test_completion_uses_exact_unicode_cursor_prefix_and_checked_timeout() -> None:
    text = "mi 💥 do"
    assert syntax.expected_continuations_at_cursor(text, 3)
    with pytest.raises(morphology.MorphologyError):
        syntax.expected_continuations_at_cursor(text, 4)
    for cursor in (-1, len(text) + 1):
        with pytest.raises(jbotci.InvalidInputError, match="0 <= cursor <= len"):
            syntax.expected_continuations_at_cursor(text, cursor)

    words = morphology.segment("mi klama")
    assert syntax.expected_continuations_with_time_limit(words, 0.0) == ()
    for timeout in (-1.0, float("nan"), float("inf")):
        with pytest.raises(jbotci.InvalidInputError):
            syntax.expected_continuations_with_time_limit(words, timeout)


def test_normalization_partition_structure_and_span_ignoring_equality() -> None:
    text = (
        "mi klama ni'o "
        "mi cusku lu do cadzu i do klama ni'o do tavla li'u ni'o "
        "do cadzu"
    )
    tokens = syntax.normalize_syntax_tokens(morphology.segment(text))
    paragraphs = syntax.partition_syntax_text_units(
        tokens,
        syntax.SyntaxTextUnitGranularity.PARAGRAPH,
    )
    statements = syntax.partition_syntax_text_units(
        tokens,
        syntax.SyntaxTextUnitGranularity.STATEMENT,
    )
    assert tuple((unit.token_start, unit.token_end) for unit in paragraphs) == (
        (0, 2),
        (3, 15),
        (16, 18),
    )
    assert statements == paragraphs

    structure = syntax.syntax_text_structure(tokens)
    assert any(
        isinstance(event, syntax.SyntaxTextStructureEventBoundary)
        and event.kind is syntax.SyntaxTextBoundaryKind.NIHO
        and event.depth == 1
        for event in structure
    )
    assert any(
        isinstance(event, syntax.SyntaxTextStructureEventContainerClose)
        and event.closer is morphology.Cmavo.LIHU
        and event.matched
        for event in structure
    )

    plain = jbotci.parse("mi tavla do").parse_tree
    shifted = jbotci.parse(" mi  tavla do ").parse_tree
    different = jbotci.parse("mi tavla mi").parse_tree
    assert syntax.syntax_tree_eq_ignoring_spans(plain, shifted)
    assert not syntax.syntax_tree_eq_ignoring_spans(plain, different)
