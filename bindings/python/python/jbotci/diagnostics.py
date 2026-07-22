"""Immutable diagnostics, styled text, and parser trace products."""

from collections.abc import Sequence
from typing import Final, TypeAlias, final

from ._errors import _StructuredError
from . import _native as _rust
from ._native import (
    _diagnostics_CllSectionLink as CllSectionLink,
    _diagnostics_Diagnostic as Diagnostic,
    _diagnostics_DiagnosticDetailMode as DiagnosticDetailMode,
    _diagnostics_DiagnosticLabel as DiagnosticLabel,
    _diagnostics_DiagnosticNoteMode as DiagnosticNoteMode,
    _diagnostics_DiagnosticPhase as DiagnosticPhase,
    _diagnostics_DiagnosticSeverity as DiagnosticSeverity,
    _diagnostics_DiagnosticStyledNote as DiagnosticStyledNote,
    _diagnostics_DiagnosticTextRole as DiagnosticTextRole,
    _diagnostics_DiagnosticTextSegment as DiagnosticTextSegment,
    _diagnostics_EbnfRuleLink as EbnfRuleLink,
    _diagnostics_TraceContext as TraceContext,
    _diagnostics_TraceEvent as TraceEvent,
    _diagnostics_TraceEventKind as TraceEventKind,
    _diagnostics_TraceFailureBranch as TraceFailureBranch,
    _diagnostics_TraceFailureSummary as TraceFailureSummary,
    _diagnostics_TraceFilter as TraceFilter,
    _diagnostics_TraceLevel as TraceLevel,
    _diagnostics_InvalidTraceLevel as InvalidTraceLevel,
    _diagnostics_TraceOptions as TraceOptions,
    _diagnostics_TracePhase as TracePhase,
    _diagnostics_TraceReport as TraceReport,
    _diagnostics_VlackuWordLink as VlackuWordLink,
    _diagnostics_diagnostic_note_mode_visible_in as _rust_diagnostic_note_mode_visible_in,
    _diagnostics_diagnostic_text_segments as _rust_diagnostic_text_segments,
    _diagnostics_diagnostic_text_segments_text as _rust_diagnostic_text_segments_text,
    _diagnostics_trace_level_from_number as _rust_trace_level_from_number,
    _diagnostics_trace_level_number as _rust_trace_level_number,
    _diagnostics_trace_phase_includes as _rust_trace_phase_includes,
)

DEFAULT_TRACE_LIMIT: Final[int] = _rust._diagnostics_DEFAULT_TRACE_LIMIT

DiagnosticTextLink: TypeAlias = VlackuWordLink | CllSectionLink | EbnfRuleLink


@final
class TraceOptionError(_StructuredError[InvalidTraceLevel]):
    """Invalid trace configuration retaining its exact Rust error value."""

    __slots__ = ()

    def __init__(self, value: InvalidTraceLevel) -> None:
        if not isinstance(value, InvalidTraceLevel):
            raise TypeError("value must be an InvalidTraceLevel")
        super().__init__(value)

    def __init_subclass__(cls) -> None:
        raise TypeError("TraceOptionError is final")


def _trace_phase_includes(self: TracePhase, phase: TracePhase) -> bool:
    """Return whether this trace-phase selector includes the requested phase."""

    return _rust_trace_phase_includes(self, phase)


def _trace_level_number(self: TraceLevel) -> int:
    """Return this trace level's stable one-based number."""

    return _rust_trace_level_number(self)


def _trace_level_from_number(value: int) -> TraceLevel:
    """Return the exact trace level for a stable one-based number."""

    return _rust_trace_level_from_number(value)


def _diagnostic_note_mode_visible_in(
    self: DiagnosticNoteMode, detail: DiagnosticDetailMode
) -> bool:
    """Return whether this note mode is visible at the requested detail level."""

    return _rust_diagnostic_note_mode_visible_in(self, detail)


# Rust supplies these operations on fieldless enums. The exact enum classes are
# constructed per interpreter from Rust metadata, so attach thin methods after
# construction while delegating all domain behavior back to Rust.
setattr(TracePhase, "includes", _trace_phase_includes)
setattr(TraceLevel, "number", _trace_level_number)
setattr(TraceLevel, "from_number", staticmethod(_trace_level_from_number))
setattr(DiagnosticNoteMode, "visible_in", _diagnostic_note_mode_visible_in)


def diagnostic_text_segments(text: str) -> tuple[DiagnosticTextSegment, ...]:
    """Parse inline diagnostic markup into immutable styled segments."""

    return _rust_diagnostic_text_segments(text)


def diagnostic_text_segments_text(
    segments: Sequence[DiagnosticTextSegment],
) -> str:
    """Concatenate styled diagnostic segments into their plain text."""

    return _rust_diagnostic_text_segments_text(segments)

__all__: tuple[str, ...] = (
    "DEFAULT_TRACE_LIMIT",
    "DiagnosticSeverity",
    "DiagnosticPhase",
    "TracePhase",
    "TraceLevel",
    "InvalidTraceLevel",
    "TraceOptionError",
    "TraceEventKind",
    "DiagnosticDetailMode",
    "DiagnosticNoteMode",
    "DiagnosticTextRole",
    "TraceFilter",
    "TraceOptions",
    "TraceEvent",
    "TraceContext",
    "TraceFailureBranch",
    "TraceFailureSummary",
    "TraceReport",
    "VlackuWordLink",
    "CllSectionLink",
    "EbnfRuleLink",
    "DiagnosticTextLink",
    "DiagnosticTextSegment",
    "DiagnosticStyledNote",
    "DiagnosticLabel",
    "Diagnostic",
    "diagnostic_text_segments",
    "diagnostic_text_segments_text",
)
