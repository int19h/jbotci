"""Inspect recovery, source spans, structured errors, warnings, and traces."""

from __future__ import annotations

import jbotci
from jbotci import diagnostics, morphology, source, syntax


def main() -> None:
    """Exercise diagnostic products through public typed APIs."""
    trace = diagnostics.TraceOptions(
        enabled=True,
        level=diagnostics.TraceLevel.TOP,
        phase=diagnostics.TracePhase.ALL,
    )
    parsed = jbotci.parse(
        "mi tavla do",
        morphology_options=morphology.MorphologyOptions(trace=trace),
        parse_options=syntax.ParseOptions(trace=trace),
    )
    assert parsed.morphology_trace is not None
    assert parsed.syntax_trace is not None
    assert isinstance(parsed.warnings, tuple)

    span = source.source_span_from_char_offsets("coi do", 0, 3)
    assert source.source_text_for_span("coi do", span) == "coi"
    try:
        source.source_span_from_char_offsets("coi", 0, 4)
    except source.DiagnosticSpanException as error:
        assert isinstance(error.value, source.CharOffsetOutOfBounds)
    else:
        raise AssertionError("out-of-range source offset unexpectedly succeeded")

    recovered = jbotci.parse_recovered("mi tavla vau vau do")
    assert recovered.syntax_errors
    print(
        f"{len(recovered.syntax_errors)} recovered syntax errors; "
        f"trace phases: {parsed.morphology_trace.phase.value}, "
        f"{parsed.syntax_trace.phase.value}"
    )


if __name__ == "__main__":
    main()
