"""Source identifiers, half-open spans, and Unicode-aware offset helpers."""

from typing import TypeAlias

from ._native import (
    _source_LineColumn as LineColumn,
    _source_ByteRangeInverted as ByteRangeInverted,
    _source_CharRangeInverted as CharRangeInverted,
    _source_SourceId as SourceId,
    _source_SourceSpan as SourceSpan,
    _source_ZeroColumn as ZeroColumn,
    _source_ZeroLine as ZeroLine,
    _source_byte_offset_for_char_offset as byte_offset_for_char_offset,
    _source_char_offset_for_byte_offset as char_offset_for_byte_offset,
    _source_line_column_for_byte_offset as line_column_for_byte_offset,
    _source_source_span_from_byte_offsets as source_span_from_byte_offsets,
    _source_source_span_from_char_offsets as source_span_from_char_offsets,
    _source_source_text_for_span as source_text_for_span,
)

SourceLocationError: TypeAlias = (
    ZeroLine | ZeroColumn | ByteRangeInverted | CharRangeInverted
)

__all__: tuple[str, ...] = (
    "SourceId",
    "LineColumn",
    "SourceSpan",
    "ZeroLine",
    "ZeroColumn",
    "ByteRangeInverted",
    "CharRangeInverted",
    "SourceLocationError",
    "source_span_from_char_offsets",
    "source_span_from_byte_offsets",
    "byte_offset_for_char_offset",
    "char_offset_for_byte_offset",
    "line_column_for_byte_offset",
    "source_text_for_span",
)
