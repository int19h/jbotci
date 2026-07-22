"""Source identifiers, half-open spans, and Unicode-aware offset helpers."""

from typing import TypeAlias, final

from ._errors import _StructuredError
from ._native import (
    _source_ByteOffsetNotCharBoundary as ByteOffsetNotCharBoundary,
    _source_ByteOffsetOutOfBounds as ByteOffsetOutOfBounds,
    _source_LineColumn as LineColumn,
    _source_ByteRangeInverted as ByteRangeInverted,
    _source_CharRangeInverted as CharRangeInverted,
    _source_CharOffsetOutOfBounds as CharOffsetOutOfBounds,
    _source_SourceId as SourceId,
    _source_SourceSpan as SourceSpan,
    _source_SourceLocation as SourceLocation,
    _source_ZeroColumn as ZeroColumn,
    _source_ZeroLine as ZeroLine,
    _source_byte_offset_for_char_offset,
    _source_char_offset_for_byte_offset,
    _source_line_column_for_byte_offset,
    _source_source_span_from_byte_offsets,
    _source_source_span_from_char_offsets,
    _source_source_text_for_span,
)

SourceLocationError: TypeAlias = (
    ZeroLine | ZeroColumn | ByteRangeInverted | CharRangeInverted
)
DiagnosticSpanError: TypeAlias = (
    CharOffsetOutOfBounds
    | ByteOffsetOutOfBounds
    | ByteOffsetNotCharBoundary
    | SourceLocation
)


@final
class SourceLocationException(_StructuredError[SourceLocationError]):
    """Invalid source location retaining its exact Rust error value."""

    __slots__ = ()

    def __init__(self, value: SourceLocationError) -> None:
        if not isinstance(
            value, (ZeroLine, ZeroColumn, ByteRangeInverted, CharRangeInverted)
        ):
            raise TypeError("value must be a SourceLocationError variant")
        super().__init__(value)

    def __init_subclass__(cls) -> None:
        raise TypeError("SourceLocationException is final")


@final
class DiagnosticSpanException(_StructuredError[DiagnosticSpanError]):
    """Invalid diagnostic span retaining its exact Rust error value."""

    __slots__ = ()

    def __init__(self, value: DiagnosticSpanError) -> None:
        if not isinstance(
            value,
            (
                CharOffsetOutOfBounds,
                ByteOffsetOutOfBounds,
                ByteOffsetNotCharBoundary,
                SourceLocation,
            ),
        ):
            raise TypeError("value must be a DiagnosticSpanError variant")
        super().__init__(value)

    def __init_subclass__(cls) -> None:
        raise TypeError("DiagnosticSpanException is final")


def source_span_from_char_offsets(
    source: str,
    char_start: int,
    char_end: int,
    *,
    source_id: SourceId | None = None,
) -> SourceSpan:
    """Build a source span from validated Unicode-scalar offsets."""

    return _source_source_span_from_char_offsets(
        source, char_start, char_end, source_id=source_id
    )


def source_span_from_byte_offsets(
    source: str,
    byte_start: int,
    byte_end: int,
    *,
    source_id: SourceId | None = None,
) -> SourceSpan:
    """Build a source span from validated UTF-8 boundary offsets."""

    return _source_source_span_from_byte_offsets(
        source, byte_start, byte_end, source_id=source_id
    )


def byte_offset_for_char_offset(source: str, char_offset: int) -> int:
    """Convert a Unicode-scalar offset to its UTF-8 byte offset."""

    return _source_byte_offset_for_char_offset(source, char_offset)


def char_offset_for_byte_offset(source: str, byte_offset: int) -> int:
    """Convert a UTF-8 boundary offset to its Unicode-scalar offset."""

    return _source_char_offset_for_byte_offset(source, byte_offset)


def line_column_for_byte_offset(source: str, byte_offset: int) -> LineColumn:
    """Return the one-based source coordinates at a UTF-8 boundary."""

    return _source_line_column_for_byte_offset(source, byte_offset)


def source_text_for_span(source: str, span: SourceSpan) -> str:
    """Slice source text using a span after validating its byte range."""

    return _source_source_text_for_span(source, span)

__all__: tuple[str, ...] = (
    "SourceId",
    "LineColumn",
    "SourceSpan",
    "ZeroLine",
    "ZeroColumn",
    "ByteRangeInverted",
    "CharRangeInverted",
    "SourceLocationError",
    "SourceLocationException",
    "CharOffsetOutOfBounds",
    "ByteOffsetOutOfBounds",
    "ByteOffsetNotCharBoundary",
    "SourceLocation",
    "DiagnosticSpanError",
    "DiagnosticSpanException",
    "source_span_from_char_offsets",
    "source_span_from_byte_offsets",
    "byte_offset_for_char_offset",
    "char_offset_for_byte_offset",
    "line_column_for_byte_offset",
    "source_text_for_span",
)
