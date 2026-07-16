use std::sync::Arc;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_source::{SourceId, SourceSpan};

/// Column-unit encoding for zero-based editor positions.
#[invariant(true, "all position encodings are valid")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PositionEncoding {
    /// UTF-8 code units (bytes).
    Utf8,
    /// UTF-16 code units, the default encoding in LSP clients.
    Utf16,
    /// UTF-32 code units (Unicode scalar values).
    Utf32,
}

/// A zero-based line and column position in a caller-selected encoding.
#[invariant(true, "every zero-based line and column pair is a valid clamped input")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    #[requires(true)]
    #[ensures(ret.line == line && ret.column == column)]
    pub fn new(line: usize, column: usize) -> Self {
        Position { line, column }
    }
}

/// An ordered, half-open pair of editor positions.
#[invariant(start <= end, "position ranges must be ordered")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionRange {
    pub start: Position,
    pub end: Position,
}

impl PositionRange {
    #[requires(start <= end)]
    #[ensures(ret.start == start && ret.end == end)]
    pub fn new(start: Position, end: Position) -> Self {
        new!(PositionRange { start, end })
    }
}

/// Equivalent offsets at a Unicode scalar boundary in one document.
#[invariant(true, "cross-encoding consistency is relative to the owning LineIndex")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextOffsets {
    pub byte: usize,
    pub char: usize,
    pub utf16: usize,
}

#[invariant(byte_start <= byte_end && byte_end <= byte_next)]
#[invariant(char_start <= char_end && char_end <= char_next)]
#[invariant(utf16_start <= utf16_end && utf16_end <= utf16_next)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct LineInfo {
    byte_start: usize,
    byte_end: usize,
    byte_next: usize,
    char_start: usize,
    char_end: usize,
    char_next: usize,
    utf16_start: usize,
    utf16_end: usize,
    utf16_next: usize,
}

/// Immutable cross-encoding index for a source document.
///
/// Construction scans the document once. Queries binary-search the line table and
/// scan at most one line, without allocating. Lines recognize LF, CRLF, and lone CR.
/// Positions use zero-based lines and columns. A line beyond the document clamps to
/// document end; a column beyond a line clamps before its terminator. Offsets inside
/// a UTF-8 scalar, and UTF-16 columns inside a surrogate pair, clamp backward to the
/// preceding scalar boundary. Bytes between CR and LF in CRLF map to the preceding
/// line end because that interior boundary has no distinct editor position.
#[invariant(!lines.is_empty(), "every document has at least one line")]
#[invariant(lines.first().is_some_and(|line| line.byte_start == 0 && line.char_start == 0 && line.utf16_start == 0))]
#[invariant(lines.last().is_some_and(|line| line.byte_next == text.len() && line.char_next == *char_len && line.utf16_next == *utf16_len))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineIndex {
    text: Arc<str>,
    lines: Vec<LineInfo>,
    char_len: usize,
    utf16_len: usize,
}

impl LineIndex {
    /// Build an index in O(text length), sharing ownership of the indexed text.
    #[requires(true)]
    #[ensures(ret.line_count() > 0)]
    #[ensures(ret.char_len() <= ret.byte_len())]
    #[ensures(ret.char_len() <= ret.utf16_len())]
    pub fn new(text: Arc<str>) -> Self {
        let mut lines = Vec::new();
        let mut byte_start = 0;
        let mut char_start = 0;
        let mut utf16_start = 0;
        let mut char_offset = 0;
        let mut utf16_offset = 0;
        let mut chars = text.char_indices().peekable();

        while let Some((byte_offset, character)) = chars.next() {
            if !matches!(character, '\r' | '\n') {
                char_offset += 1;
                utf16_offset += character.len_utf16();
                continue;
            }

            let char_end = char_offset;
            let utf16_end = utf16_offset;
            char_offset += 1;
            utf16_offset += 1;
            let mut byte_next = byte_offset + character.len_utf8();
            if character == '\r' && chars.peek().is_some_and(|(_, next)| *next == '\n') {
                let (lf_byte_offset, lf) = chars.next().expect("peeked LF must remain available");
                char_offset += 1;
                utf16_offset += 1;
                byte_next = lf_byte_offset + lf.len_utf8();
            }
            lines.push(new!(LineInfo {
                byte_start,
                byte_end: byte_offset,
                byte_next,
                char_start,
                char_end,
                char_next: char_offset,
                utf16_start,
                utf16_end,
                utf16_next: utf16_offset,
            }));
            byte_start = byte_next;
            char_start = char_offset;
            utf16_start = utf16_offset;
        }

        lines.push(new!(LineInfo {
            byte_start,
            byte_end: text.len(),
            byte_next: text.len(),
            char_start,
            char_end: char_offset,
            char_next: char_offset,
            utf16_start,
            utf16_end: utf16_offset,
            utf16_next: utf16_offset,
        }));

        new!(LineIndex {
            text,
            lines,
            char_len: char_offset,
            utf16_len: utf16_offset,
        })
    }

    #[requires(true)]
    #[ensures(ret.len() == self.byte_len())]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[requires(true)]
    #[ensures(ret == self.text.len())]
    pub fn byte_len(&self) -> usize {
        self.text.len()
    }

    #[requires(true)]
    #[ensures(ret == self.char_len)]
    pub fn char_len(&self) -> usize {
        self.char_len
    }

    #[requires(true)]
    #[ensures(ret == self.utf16_len)]
    pub fn utf16_len(&self) -> usize {
        self.utf16_len
    }

    #[requires(true)]
    #[ensures(ret > 0)]
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Resolve a byte offset, clamping document overflow and partial scalars backward.
    #[requires(true)]
    #[ensures(ret.byte <= self.byte_len() && ret.char <= self.char_len() && ret.utf16 <= self.utf16_len())]
    pub fn offsets_for_byte(&self, byte_offset: usize) -> TextOffsets {
        let target = byte_offset.min(self.byte_len());
        let line = self.line_for_byte(target);
        self.offsets_within_line(
            line,
            target - line.byte_start,
            PositionEncoding::Utf8,
            line.byte_next,
        )
    }

    /// Resolve a Unicode scalar offset, clamping document overflow to document end.
    #[requires(true)]
    #[ensures(ret.byte <= self.byte_len() && ret.char <= self.char_len() && ret.utf16 <= self.utf16_len())]
    pub fn offsets_for_char(&self, char_offset: usize) -> TextOffsets {
        let target = char_offset.min(self.char_len());
        let line = self.line_for_char(target);
        self.offsets_within_line(
            line,
            target - line.char_start,
            PositionEncoding::Utf32,
            line.byte_next,
        )
    }

    /// Resolve a UTF-16 offset, clamping overflow and split surrogate pairs backward.
    #[requires(true)]
    #[ensures(ret.byte <= self.byte_len() && ret.char <= self.char_len() && ret.utf16 <= self.utf16_len())]
    pub fn offsets_for_utf16(&self, utf16_offset: usize) -> TextOffsets {
        let target = utf16_offset.min(self.utf16_len());
        let line = self.line_for_utf16(target);
        self.offsets_within_line(
            line,
            target - line.utf16_start,
            PositionEncoding::Utf16,
            line.byte_next,
        )
    }

    #[requires(true)]
    #[ensures(ret <= self.char_len())]
    pub fn byte_to_char_offset(&self, byte_offset: usize) -> usize {
        self.offsets_for_byte(byte_offset).char
    }

    #[requires(true)]
    #[ensures(ret <= self.utf16_len())]
    pub fn byte_to_utf16_offset(&self, byte_offset: usize) -> usize {
        self.offsets_for_byte(byte_offset).utf16
    }

    #[requires(true)]
    #[ensures(ret <= self.byte_len())]
    pub fn char_to_byte_offset(&self, char_offset: usize) -> usize {
        self.offsets_for_char(char_offset).byte
    }

    #[requires(true)]
    #[ensures(ret <= self.utf16_len())]
    pub fn char_to_utf16_offset(&self, char_offset: usize) -> usize {
        self.offsets_for_char(char_offset).utf16
    }

    #[requires(true)]
    #[ensures(ret <= self.byte_len())]
    pub fn utf16_to_byte_offset(&self, utf16_offset: usize) -> usize {
        self.offsets_for_utf16(utf16_offset).byte
    }

    #[requires(true)]
    #[ensures(ret <= self.char_len())]
    pub fn utf16_to_char_offset(&self, utf16_offset: usize) -> usize {
        self.offsets_for_utf16(utf16_offset).char
    }

    #[requires(true)]
    #[ensures(ret.line < self.line_count())]
    pub fn position_for_byte(&self, byte_offset: usize, encoding: PositionEncoding) -> Position {
        self.position_for_offsets(self.offsets_for_byte(byte_offset), encoding)
    }

    #[requires(true)]
    #[ensures(ret.line < self.line_count())]
    pub fn position_for_char(&self, char_offset: usize, encoding: PositionEncoding) -> Position {
        self.position_for_offsets(self.offsets_for_char(char_offset), encoding)
    }

    #[requires(true)]
    #[ensures(ret.line < self.line_count())]
    pub fn position_for_utf16(&self, utf16_offset: usize, encoding: PositionEncoding) -> Position {
        self.position_for_offsets(self.offsets_for_utf16(utf16_offset), encoding)
    }

    /// Resolve a position to all three offsets with the documented clamping rules.
    #[requires(true)]
    #[ensures(ret.byte <= self.byte_len() && ret.char <= self.char_len() && ret.utf16 <= self.utf16_len())]
    pub fn offsets_for_position(
        &self,
        position: Position,
        encoding: PositionEncoding,
    ) -> TextOffsets {
        let Some(line) = self.lines.get(position.line) else {
            return self.document_end();
        };
        self.offsets_within_line(line, position.column, encoding, line.byte_end)
    }

    #[requires(true)]
    #[ensures(ret <= self.byte_len())]
    pub fn byte_offset_for_position(
        &self,
        position: Position,
        encoding: PositionEncoding,
    ) -> usize {
        self.offsets_for_position(position, encoding).byte
    }

    #[requires(true)]
    #[ensures(ret <= self.char_len())]
    pub fn char_offset_for_position(
        &self,
        position: Position,
        encoding: PositionEncoding,
    ) -> usize {
        self.offsets_for_position(position, encoding).char
    }

    #[requires(true)]
    #[ensures(ret <= self.utf16_len())]
    pub fn utf16_offset_for_position(
        &self,
        position: Position,
        encoding: PositionEncoding,
    ) -> usize {
        self.offsets_for_position(position, encoding).utf16
    }

    /// Resolve a pipeline span without consulting its unpopulated line/column fields.
    /// UTF-8 positions use the span's byte offsets; UTF-16 and UTF-32 positions use
    /// its character offsets.
    #[requires(true)]
    #[ensures(ret.start <= ret.end)]
    pub fn positions_for_span(
        &self,
        span: &SourceSpan,
        encoding: PositionEncoding,
    ) -> PositionRange {
        let (start, end) = match encoding {
            PositionEncoding::Utf8 => (
                self.position_for_byte(span.byte_start, encoding),
                self.position_for_byte(span.byte_end, encoding),
            ),
            PositionEncoding::Utf16 | PositionEncoding::Utf32 => (
                self.position_for_char(span.char_start, encoding),
                self.position_for_char(span.char_end, encoding),
            ),
        };
        new!(PositionRange { start, end })
    }

    /// Convert an ordered position pair back to a span carrying byte and char offsets.
    #[requires(positions.start <= positions.end)]
    #[ensures(ret.byte_start <= ret.byte_end && ret.char_start <= ret.char_end)]
    pub fn span_for_positions(
        &self,
        positions: &PositionRange,
        encoding: PositionEncoding,
        source_id: Option<SourceId>,
    ) -> SourceSpan {
        let start = self.offsets_for_position(positions.start, encoding);
        let end = self.offsets_for_position(positions.end, encoding);
        SourceSpan::new(source_id, start.byte, end.byte, start.char, end.char)
            .expect("ordered clamped positions must produce ordered offsets")
    }

    #[requires(true)]
    #[ensures(ret.byte <= self.byte_len() && ret.char <= self.char_len() && ret.utf16 <= self.utf16_len())]
    fn offsets_within_line(
        &self,
        line: &LineInfo,
        target_units: usize,
        encoding: PositionEncoding,
        byte_limit: usize,
    ) -> TextOffsets {
        let mut byte = line.byte_start;
        let mut char = line.char_start;
        let mut utf16 = line.utf16_start;
        let mut consumed_units = 0usize;

        for character in self.text[line.byte_start..byte_limit].chars() {
            let character_units = match encoding {
                PositionEncoding::Utf8 => character.len_utf8(),
                PositionEncoding::Utf16 => character.len_utf16(),
                PositionEncoding::Utf32 => 1,
            };
            if consumed_units.saturating_add(character_units) > target_units {
                break;
            }
            consumed_units += character_units;
            byte += character.len_utf8();
            char += 1;
            utf16 += character.len_utf16();
        }

        TextOffsets { byte, char, utf16 }
    }

    #[requires(offsets.byte <= self.byte_len() && offsets.char <= self.char_len() && offsets.utf16 <= self.utf16_len())]
    #[ensures(ret.line < self.line_count())]
    fn position_for_offsets(&self, offsets: TextOffsets, encoding: PositionEncoding) -> Position {
        let line_number = self.line_number_for_byte(offsets.byte);
        let line = &self.lines[line_number];
        let column = if offsets.byte >= line.byte_end {
            match encoding {
                PositionEncoding::Utf8 => line.byte_end - line.byte_start,
                PositionEncoding::Utf16 => line.utf16_end - line.utf16_start,
                PositionEncoding::Utf32 => line.char_end - line.char_start,
            }
        } else {
            match encoding {
                PositionEncoding::Utf8 => offsets.byte - line.byte_start,
                PositionEncoding::Utf16 => offsets.utf16 - line.utf16_start,
                PositionEncoding::Utf32 => offsets.char - line.char_start,
            }
        };
        Position {
            line: line_number,
            column,
        }
    }

    #[requires(byte_offset <= self.byte_len())]
    #[ensures(ret < self.line_count())]
    fn line_number_for_byte(&self, byte_offset: usize) -> usize {
        self.lines
            .partition_point(|line| line.byte_start <= byte_offset)
            .saturating_sub(1)
    }

    #[requires(byte_offset <= self.byte_len())]
    #[ensures(ret.byte_start <= byte_offset && byte_offset <= ret.byte_next)]
    fn line_for_byte(&self, byte_offset: usize) -> &LineInfo {
        &self.lines[self.line_number_for_byte(byte_offset)]
    }

    #[requires(char_offset <= self.char_len())]
    #[ensures(ret.char_start <= char_offset && char_offset <= ret.char_next)]
    fn line_for_char(&self, char_offset: usize) -> &LineInfo {
        let line_number = self
            .lines
            .partition_point(|line| line.char_start <= char_offset)
            .saturating_sub(1);
        &self.lines[line_number]
    }

    #[requires(utf16_offset <= self.utf16_len())]
    #[ensures(ret.utf16_start <= utf16_offset && utf16_offset <= ret.utf16_next)]
    fn line_for_utf16(&self, utf16_offset: usize) -> &LineInfo {
        let line_number = self
            .lines
            .partition_point(|line| line.utf16_start <= utf16_offset)
            .saturating_sub(1);
        &self.lines[line_number]
    }

    #[requires(true)]
    #[ensures(ret.byte == self.byte_len() && ret.char == self.char_len() && ret.utf16 == self.utf16_len())]
    fn document_end(&self) -> TextOffsets {
        TextOffsets {
            byte: self.byte_len(),
            char: self.char_len(),
            utf16: self.utf16_len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    #[requires(true)]
    #[ensures(ret.first() == Some(&(0, 0, 0)))]
    fn scalar_boundaries(source: &str) -> Vec<(usize, usize, usize)> {
        let mut boundaries = vec![(0, 0, 0)];
        let mut chars = 0;
        let mut utf16 = 0;
        for (byte, character) in source.char_indices() {
            chars += 1;
            utf16 += character.len_utf16();
            boundaries.push((byte + character.len_utf8(), chars, utf16));
        }
        boundaries
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_round_trip_properties(source: &str) {
        let index = LineIndex::new(Arc::from(source));
        for (byte, char, utf16) in scalar_boundaries(source) {
            let expected = TextOffsets { byte, char, utf16 };
            assert_eq!(
                index.offsets_for_byte(byte),
                expected,
                "byte boundary in {source:?}"
            );
            assert_eq!(
                index.offsets_for_char(char),
                expected,
                "char boundary in {source:?}"
            );
            assert_eq!(
                index.offsets_for_utf16(utf16),
                expected,
                "UTF-16 boundary in {source:?}"
            );
            assert_eq!(index.byte_to_char_offset(byte), char);
            assert_eq!(index.byte_to_utf16_offset(byte), utf16);
            assert_eq!(index.char_to_byte_offset(char), byte);
            assert_eq!(index.char_to_utf16_offset(char), utf16);
            assert_eq!(index.utf16_to_byte_offset(utf16), byte);
            assert_eq!(index.utf16_to_char_offset(utf16), char);

            let crlf_interior = byte > 0
                && byte < source.len()
                && source.as_bytes()[byte - 1] == b'\r'
                && source.as_bytes()[byte] == b'\n';
            for encoding in [
                PositionEncoding::Utf8,
                PositionEncoding::Utf16,
                PositionEncoding::Utf32,
            ] {
                let position = index.position_for_byte(byte, encoding);
                assert_eq!(index.position_for_char(char, encoding), position);
                assert_eq!(index.position_for_utf16(utf16, encoding), position);
                let resolved = index.offsets_for_position(position, encoding);
                assert_eq!(index.position_for_byte(resolved.byte, encoding), position);
                if !crlf_interior {
                    assert_eq!(resolved, expected, "position round trip in {source:?}");
                }
            }
        }

        for (byte, character) in source.char_indices() {
            if character.len_utf8() > 1 {
                assert_eq!(index.offsets_for_byte(byte + 1).byte, byte);
            }
            if character.len_utf16() == 2 {
                let utf16 = source[..byte].encode_utf16().count();
                assert_eq!(index.offsets_for_utf16(utf16 + 1).byte, byte);
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_mixed_unicode_documents_round_trip_in_all_encodings() {
        let atoms = ["a", "«", "·", "𝙰", "🙂", "\n", "\r", "\r\n"];
        for seed in 0..96usize {
            let mut source = String::from("a«·𝙰🙂\r\n");
            let mut state = seed.wrapping_add(1);
            for _ in 0..24 {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                source.push_str(atoms[state % atoms.len()]);
            }
            source.push_str("\ra\n«·𝙰🙂");
            assert_round_trip_properties(&source);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn positions_clamp_to_line_and_document_ends() {
        let index = LineIndex::new(Arc::from("a\r\nb\rc\n"));
        assert_eq!(index.line_count(), 4);
        assert_eq!(
            index.offsets_for_position(Position::new(0, usize::MAX), PositionEncoding::Utf16),
            TextOffsets {
                byte: 1,
                char: 1,
                utf16: 1
            }
        );
        assert_eq!(
            index.offsets_for_position(Position::new(99, 7), PositionEncoding::Utf8),
            TextOffsets {
                byte: 7,
                char: 7,
                utf16: 7
            }
        );
        assert_eq!(
            index.position_for_byte(2, PositionEncoding::Utf16),
            Position::new(0, 1),
            "the CRLF interior maps to the preceding line end"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_encoded_columns_clamp_to_scalar_boundaries() {
        let index = LineIndex::new(Arc::from("«𝙰"));
        assert_eq!(
            index.offsets_for_position(Position::new(0, 1), PositionEncoding::Utf8),
            TextOffsets {
                byte: 0,
                char: 0,
                utf16: 0
            }
        );
        assert_eq!(
            index.offsets_for_position(Position::new(0, 2), PositionEncoding::Utf16),
            TextOffsets {
                byte: 2,
                char: 1,
                utf16: 1
            }
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn source_spans_round_trip_without_line_column_metadata() {
        let index = LineIndex::new(Arc::from("a𝙰\r\n🙂"));
        let span = SourceSpan::new(None, 1, 5, 1, 2).expect("ordered test span");
        assert!(span.start.is_none() && span.end.is_none());
        for encoding in [
            PositionEncoding::Utf8,
            PositionEncoding::Utf16,
            PositionEncoding::Utf32,
        ] {
            let positions = index.positions_for_span(&span, encoding);
            assert_eq!(index.span_for_positions(&positions, encoding, None), span);
        }
    }
}
