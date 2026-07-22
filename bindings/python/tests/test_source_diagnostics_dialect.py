from __future__ import annotations

import gc

import pytest

import jbotci._native as native
from jbotci import InvalidInputError, diagnostics, dialect, morphology, source


def test_builtin_dialect_has_no_runtime_constructor() -> None:
    with pytest.raises(TypeError, match=r"^No constructor defined$"):
        dialect.BuiltinDialect()


def test_source_spans_distinguish_unicode_byte_and_character_offsets() -> None:
    source_id = source.SourceId("unicode")
    span = source.source_span_from_char_offsets("éx\n", 0, 1, source_id=source_id)
    assert span.source_id == source_id
    assert span.byte_range == (0, 2)
    assert span.char_range == (0, 1)
    assert span.byte_len == 2
    assert span.char_len == 1
    assert source.source_text_for_span("éx\n", span) == "é"
    assert source.byte_offset_for_char_offset("éx", 1) == 2
    assert source.char_offset_for_byte_offset("éx", 2) == 1


def test_source_span_does_not_invent_endpoint_pairing_constraint() -> None:
    start = source.LineColumn(1, 1)
    span = source.SourceSpan(0, 0, 0, 0, start=start)
    assert span.start == start
    assert span.end is None
    assert span.is_empty()


def test_source_constructors_validate_real_rust_invariants() -> None:
    with pytest.raises(source.SourceLocationException) as zero_line:
        source.LineColumn(0, 1)
    assert zero_line.value.value == source.ZeroLine()
    assert zero_line.value.args == (str(zero_line.value.value),)
    with pytest.raises(source.SourceLocationException) as zero_column:
        source.LineColumn(1, 0)
    assert zero_column.value.value == source.ZeroColumn()

    with pytest.raises(source.SourceLocationException) as byte_range:
        source.SourceSpan(2, 1, 0, 0)
    assert byte_range.value.value == source.ByteRangeInverted(2, 1)
    with pytest.raises(source.SourceLocationException) as char_range:
        source.SourceSpan(0, 0, 2, 1)
    assert char_range.value.value == source.CharRangeInverted(2, 1)

    with pytest.raises(source.DiagnosticSpanException) as byte_boundary:
        source.source_span_from_byte_offsets("é", 1, 2)
    assert byte_boundary.value.value == source.ByteOffsetNotCharBoundary(1)
    with pytest.raises(source.DiagnosticSpanException) as char_boundary:
        source.char_offset_for_byte_offset("é", 1)
    assert char_boundary.value.value == source.ByteOffsetNotCharBoundary(1)

    error: source.SourceLocationError = source.ByteRangeInverted(2, 1)
    # The Rust error enum deliberately preserves arbitrary directly supplied
    # endpoint payloads; only span-producing operations require inversion.
    assert source.ByteRangeInverted(1, 2) == source.ByteRangeInverted(1, 2)
    assert source.CharRangeInverted(1, 1) == source.CharRangeInverted(1, 1)
    match error:
        case source.ByteRangeInverted(start, end):
            assert (start, end) == (2, 1)
        case _:
            pytest.fail("byte-range error did not retain its exact payload")


def test_every_diagnostic_span_variant_and_helper_retains_rust_payload() -> None:
    char_failures = (
        lambda: source.source_span_from_char_offsets("é", 2, 2),
        lambda: source.byte_offset_for_char_offset("é", 2),
    )
    for operation in char_failures:
        with pytest.raises(source.DiagnosticSpanException) as caught:
            operation()
        assert caught.value.value == source.CharOffsetOutOfBounds(2, 1)
        assert caught.value.args == (str(caught.value.value),)
        match caught.value:
            case source.DiagnosticSpanException(
                source.CharOffsetOutOfBounds(offset, source_len)
            ):
                assert (offset, source_len) == (2, 1)
            case _:
                pytest.fail("character-offset payload was not preserved")

    byte_failures = (
        lambda: source.source_span_from_byte_offsets("é", 3, 3),
        lambda: source.char_offset_for_byte_offset("é", 3),
        lambda: source.line_column_for_byte_offset("é", 3),
        lambda: source.source_text_for_span("é", source.SourceSpan(0, 3, 0, 1)),
    )
    for operation in byte_failures:
        with pytest.raises(source.DiagnosticSpanException) as caught:
            operation()
        assert caught.value.value == source.ByteOffsetOutOfBounds(3, 2)
        assert caught.value.args == (str(caught.value.value),)

    boundary_failures = (
        lambda: source.source_span_from_byte_offsets("é", 1, 2),
        lambda: source.char_offset_for_byte_offset("é", 1),
        lambda: source.line_column_for_byte_offset("é", 1),
        lambda: source.source_text_for_span("é", source.SourceSpan(1, 2, 0, 1)),
    )
    for operation in boundary_failures:
        with pytest.raises(source.DiagnosticSpanException) as caught:
            operation()
        assert caught.value.value == source.ByteOffsetNotCharBoundary(1)
        assert caught.value.args == (str(caught.value.value),)

    with pytest.raises(source.DiagnosticSpanException) as char_range:
        source.source_span_from_char_offsets("ab", 2, 1)
    assert char_range.value.value == source.SourceLocation(
        source.CharRangeInverted(2, 1)
    )
    assert char_range.value.value.error == source.CharRangeInverted(2, 1)
    assert str(char_range.value.value) == (
        "invalid source span: character range end 1 precedes start 2"
    )
    assert repr(char_range.value.value) == (
        "jbotci.source.SourceLocation("
        "error=jbotci.source.CharRangeInverted(start=2, end=1))"
    )

    with pytest.raises(source.DiagnosticSpanException) as byte_range:
        source.source_span_from_byte_offsets("ab", 2, 1)
    assert byte_range.value.value == source.SourceLocation(
        source.ByteRangeInverted(2, 1)
    )


def test_source_error_values_and_exceptions_are_immutable_and_final() -> None:
    details: tuple[source.DiagnosticSpanError, ...] = (
        source.CharOffsetOutOfBounds(3, 2),
        source.ByteOffsetOutOfBounds(3, 2),
        source.ByteOffsetNotCharBoundary(1),
        source.SourceLocation(source.ZeroLine()),
    )
    assert repr(details[0]) == (
        "jbotci.source.CharOffsetOutOfBounds(offset=3, source_len=2)"
    )
    assert repr(details[1]) == (
        "jbotci.source.ByteOffsetOutOfBounds(offset=3, source_len=2)"
    )
    assert repr(details[2]) == (
        "jbotci.source.ByteOffsetNotCharBoundary(offset=1)"
    )
    for detail in details:
        with pytest.raises(AttributeError):
            detail.offset = 0  # type: ignore[union-attr]
        with pytest.raises(TypeError):
            type("DerivedDiagnosticSpanValue", (type(detail),), {})

    source_error = source.SourceLocationException(source.ZeroLine())
    diagnostic_error = source.DiagnosticSpanException(details[0])
    for error in (source_error, diagnostic_error):
        with pytest.raises(AttributeError):
            error.args = ("changed",)
        with pytest.raises(AttributeError):
            error.value = source.ZeroColumn()  # type: ignore[assignment]
    with pytest.raises(TypeError):
        type("DerivedSourceLocationException", (source.SourceLocationException,), {})
    with pytest.raises(TypeError):
        type("DerivedDiagnosticSpanException", (source.DiagnosticSpanException,), {})


def test_diagnostic_and_trace_products_are_immutable_typed_values() -> None:
    span = source.source_span_from_char_offsets("mi", 0, 2)
    label = diagnostics.DiagnosticLabel(span, "word", primary=True)
    diagnostic = diagnostics.Diagnostic(
        diagnostics.DiagnosticSeverity.ERROR,
        diagnostics.DiagnosticPhase.MORPHOLOGY,
        "test.code",
        "test message",
        [label],
    )
    assert diagnostic.primary_label == label
    assert diagnostic.labels == (label,)
    assert diagnostic.message_segments
    with pytest.raises(AttributeError):
        diagnostic.code = "changed"  # type: ignore[misc]

    options = diagnostics.TraceOptions(enabled=True, limit=7)
    assert options.enabled
    assert options.limit == 7
    assert options.with_limit(9).limit == 9


def test_complete_trace_and_styled_note_models_round_trip_as_tuples() -> None:
    context = diagnostics.TraceContext("word", 0, 2)
    branch = diagnostics.TraceFailureBranch([context], ["cmavo"])
    failure = diagnostics.TraceFailureSummary(
        0, 2, "expected a word", [branch], context
    )
    event = diagnostics.TraceEvent(
        diagnostics.TracePhase.MORPHOLOGY,
        diagnostics.TraceLevel.DETAILED,
        1,
        diagnostics.TraceEventKind.MORPHOLOGY_FAILURE,
        "word",
        0,
        2,
        "failure",
    )
    report = diagnostics.TraceReport(
        diagnostics.TracePhase.MORPHOLOGY,
        [event],
        truncated=True,
        failure=failure,
    )
    segment = diagnostics.DiagnosticTextSegment(
        diagnostics.DiagnosticTextRole.PLAIN, "note"
    )
    note = diagnostics.DiagnosticStyledNote(
        diagnostics.DiagnosticNoteMode.DETAILED, [segment]
    )

    assert (context.construct, context.byte_start, context.byte_end) == ("word", 0, 2)
    assert report.events == (event,)
    assert report.failure == failure
    assert report.truncated
    assert note.segments == (segment,)
    assert note.mode is diagnostics.DiagnosticNoteMode.DETAILED

    with pytest.raises(InvalidInputError):
        diagnostics.TraceReport(diagnostics.TracePhase.ALL)
    syntax_event = diagnostics.TraceEvent(
        diagnostics.TracePhase.SYNTAX,
        diagnostics.TraceLevel.TOP,
        0,
        diagnostics.TraceEventKind.CONSTRUCT_ENTER,
        "text",
        0,
        0,
    )
    with pytest.raises(InvalidInputError):
        diagnostics.TraceReport(diagnostics.TracePhase.MORPHOLOGY, [syntax_event])
    with pytest.raises(InvalidInputError):
        diagnostics.DiagnosticStyledNote(diagnostics.DiagnosticNoteMode.ALWAYS, [])


def test_diagnostic_domain_operations_delegate_to_rust() -> None:
    assert diagnostics.TracePhase.ALL.includes(diagnostics.TracePhase.SYNTAX)
    assert not diagnostics.TracePhase.MORPHOLOGY.includes(
        diagnostics.TracePhase.SYNTAX
    )
    assert diagnostics.TraceLevel.DETAILED.number() == 2
    assert diagnostics.TraceLevel.from_number(4) is diagnostics.TraceLevel.PRIMITIVES
    with pytest.raises(diagnostics.TraceOptionError) as caught:
        diagnostics.TraceLevel.from_number(0)
    assert caught.value.value == diagnostics.InvalidTraceLevel(0)
    assert diagnostics.TraceOptions(enabled=True).includes(
        diagnostics.TracePhase.MORPHOLOGY
    )
    assert diagnostics.DiagnosticNoteMode.ALWAYS.visible_in(
        diagnostics.DiagnosticDetailMode.SUMMARY
    )


def test_rust_trace_constant_and_structured_errors_are_exact() -> None:
    assert diagnostics.DEFAULT_TRACE_LIMIT is native._diagnostics_DEFAULT_TRACE_LIMIT
    assert diagnostics.DEFAULT_TRACE_LIMIT == 10_000

    for value in (0, 5, 255):
        with pytest.raises(diagnostics.TraceOptionError) as caught:
            diagnostics.TraceLevel.from_number(value)
        detail = caught.value.value
        assert detail == diagnostics.InvalidTraceLevel(value)
        assert hash(detail) == hash(diagnostics.InvalidTraceLevel(value))
        assert detail.value == value
        assert caught.value.args == (str(detail),)
        assert str(detail) == (
            f"invalid trace level {value}; expected 1, 2, 3, or 4"
        )
        assert repr(detail) == (
            f"jbotci.diagnostics.InvalidTraceLevel(value={value})"
        )
        match detail:
            case diagnostics.InvalidTraceLevel(retained):
                assert retained == value
            case _:
                pytest.fail("trace error did not retain its exact value")
        match caught.value:
            case diagnostics.TraceOptionError(
                diagnostics.InvalidTraceLevel(retained)
            ):
                assert retained == value
            case _:
                pytest.fail("trace exception did not expose its typed value")

    for value in (-1, 256):
        with pytest.raises(InvalidInputError, match="between 0 and 255"):
            diagnostics.TraceLevel.from_number(value)
    with pytest.raises(OverflowError):
        diagnostics.TraceLevel.from_number(2**100)
    with pytest.raises(InvalidInputError):
        diagnostics.InvalidTraceLevel(1)

    error = diagnostics.TraceOptionError(diagnostics.InvalidTraceLevel(5))
    assert diagnostics.InvalidTraceLevel.__module__ == "jbotci.diagnostics"
    assert diagnostics.TraceOptionError.__module__ == "jbotci.diagnostics"
    assert diagnostics.TraceOptionError.__qualname__ == "TraceOptionError"
    with pytest.raises(AttributeError):
        error.value = diagnostics.InvalidTraceLevel(6)  # type: ignore[misc]
    with pytest.raises(AttributeError):
        error.args = ("changed",)
    with pytest.raises(TypeError):
        diagnostics.TraceOptionError("invalid")  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        type("DerivedTraceOptionError", (diagnostics.TraceOptionError,), {})
    with pytest.raises(TypeError):
        type("DerivedInvalidTraceLevel", (diagnostics.InvalidTraceLevel,), {})


def test_rust_morphology_constants_are_ordered_immutable_tuples() -> None:
    assert (
        morphology.MORPHOLOGY_TRACE_FILTERS
        is native._morphology_MORPHOLOGY_TRACE_FILTERS
    )
    assert (
        morphology.PERMISSIVE_IGNORABLE_RESERVED_CHARACTERS
        is native._morphology_PERMISSIVE_IGNORABLE_RESERVED_CHARACTERS
    )
    assert isinstance(morphology.MORPHOLOGY_TRACE_FILTERS, tuple)
    assert isinstance(
        morphology.PERMISSIVE_IGNORABLE_RESERVED_CHARACTERS, tuple
    )
    assert all(
        len(character) == 1
        for character in morphology.PERMISSIVE_IGNORABLE_RESERVED_CHARACTERS
    )
    with pytest.raises(TypeError):
        morphology.MORPHOLOGY_TRACE_FILTERS[0] = "changed"  # type: ignore[index]
    with pytest.raises(TypeError):
        morphology.PERMISSIVE_IGNORABLE_RESERVED_CHARACTERS[0] = "x"  # type: ignore[index]


def test_diagnostic_and_trace_children_retain_arc_roots() -> None:
    context = diagnostics.TraceContext("word", 0, 2)
    branch = diagnostics.TraceFailureBranch([context], ["cmavo"])
    failure = diagnostics.TraceFailureSummary(0, 2, "expected", [branch], context)
    event = diagnostics.TraceEvent(
        diagnostics.TracePhase.MORPHOLOGY,
        diagnostics.TraceLevel.DETAILED,
        0,
        diagnostics.TraceEventKind.MORPHOLOGY_FAILURE,
        "word",
        0,
        2,
    )
    report = diagnostics.TraceReport(
        diagnostics.TracePhase.MORPHOLOGY, [event], failure=failure
    )
    located_event = report.events[0]
    located_failure = report.failure
    assert located_failure is not None
    located_branch = located_failure.branches[0]
    located_context = located_branch.contexts[0]
    del report, event, failure, branch, context, located_failure, located_branch
    gc.collect()
    assert located_event.label == "word"
    assert located_context.construct == "word"

    span = source.SourceSpan(0, 2, 0, 2)
    label = diagnostics.DiagnosticLabel(span, "word", primary=True)
    link = diagnostics.VlackuWordLink("klama")
    segment = diagnostics.DiagnosticTextSegment(
        diagnostics.DiagnosticTextRole.SPECIFIC_WORD, "klama", link=link
    )
    styled_note = diagnostics.DiagnosticStyledNote(
        diagnostics.DiagnosticNoteMode.ALWAYS, [segment]
    )
    diagnostic = diagnostics.Diagnostic(
        diagnostics.DiagnosticSeverity.ERROR,
        diagnostics.DiagnosticPhase.MORPHOLOGY,
        "test",
        "failure",
        [label],
        styled_notes=[styled_note],
    )
    located_label = diagnostic.labels[0]
    located_message = diagnostic.message_segments[0]
    located_note = diagnostic.styled_notes[0]
    located_segment = located_note.segments[0]
    located_link = located_segment.link
    assert isinstance(located_link, diagnostics.VlackuWordLink)
    del diagnostic, label, segment, styled_note, located_note, link
    gc.collect()
    assert located_label.message == "word"
    assert located_message.text
    assert located_segment.text == "klama"
    assert located_link.word == "klama"


def test_diagnostic_text_link_payload_variants_support_matching() -> None:
    link: diagnostics.DiagnosticTextLink = diagnostics.CllSectionLink("3.2", "example")
    match link:
        case diagnostics.CllSectionLink(section_id, anchor):
            assert (section_id, anchor) == ("3.2", "example")
        case _:
            pytest.fail("typed CLL link did not match its payload variant")


def test_declarative_dialect_is_separate_and_round_trips() -> None:
    entry = dialect.CmavoSwap("ce'u", "ce")
    definition = dialect.DialectDefinition(
        (entry,),
        (dialect.DialectFeature.CASE_INSENSITIVE,),
    )
    assert definition.cmavo_entries == (entry,)
    assert definition.features == (dialect.DialectFeature.CASE_INSENSITIVE,)
    rendered = dialect.dialect_definition_to_text(definition)
    assert dialect.parse_dialect_definition(rendered) == definition
    assert dialect.DialectFeature.CASE_INSENSITIVE.atom_name() == "CASE-INSENSITIVE"


def test_copied_dialect_inputs_accept_lists_and_tuples_but_return_tuples() -> None:
    entry = dialect.CmavoSwap("ce'u", "ce")
    feature = dialect.DialectFeature.CASE_INSENSITIVE
    from_lists = dialect.DialectDefinition([entry], [feature])
    from_tuples = dialect.DialectDefinition((entry,), (feature,))
    assert from_lists == from_tuples
    assert from_lists.cmavo_entries == (entry,)
    assert from_lists.features == (feature,)
    assert isinstance(from_lists.cmavo_entries, tuple)
    assert isinstance(from_lists.features, tuple)
    assert dialect.cmavo_dialect_entries_to_definition([entry]) == (
        dialect.cmavo_dialect_entries_to_definition((entry,))
    )
    with pytest.raises(TypeError):
        dialect.DialectDefinition(set(), set())  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        dialect.DialectDefinition(
            [object()],  # type: ignore[list-item]
            [feature],
        )
    with pytest.raises(TypeError):
        dialect.cmavo_dialect_entries_to_definition(set())  # type: ignore[arg-type]


def test_builtin_dialect_is_returned_only() -> None:
    with pytest.raises(TypeError):
        dialect.BuiltinDialect()


def test_dialect_entry_validation_precedes_rust_contract_boundary() -> None:
    with pytest.raises(InvalidInputError):
        dialect.CmavoSwap("", "coi")
    with pytest.raises(InvalidInputError):
        dialect.CmavoExpansion("coi", [])
