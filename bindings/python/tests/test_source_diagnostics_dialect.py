from __future__ import annotations

import pytest

from jbotci import InvalidInputError, diagnostics, dialect, source


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
    with pytest.raises(InvalidInputError):
        source.LineColumn(0, 1)
    with pytest.raises(InvalidInputError):
        source.SourceSpan(2, 1, 0, 0)
    with pytest.raises(InvalidInputError):
        source.source_span_from_byte_offsets("é", 1, 2)
    with pytest.raises(InvalidInputError):
        source.char_offset_for_byte_offset("é", 1)

    error: source.SourceLocationError = source.ByteRangeInverted(2, 1)
    match error:
        case source.ByteRangeInverted(start, end):
            assert (start, end) == (2, 1)
        case _:
            pytest.fail("byte-range error did not retain its exact payload")


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
    with pytest.raises(InvalidInputError):
        diagnostics.DiagnosticStyledNote(diagnostics.DiagnosticNoteMode.ALWAYS, [])


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


def test_dialect_entry_validation_precedes_rust_contract_boundary() -> None:
    with pytest.raises(InvalidInputError):
        dialect.CmavoSwap("", "coi")
    with pytest.raises(InvalidInputError):
        dialect.CmavoExpansion("coi", [])
