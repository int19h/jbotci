//! Shared source-location types.

use bityzba::{data, invariant, requires};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// Stable identifier for an input source.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceId(pub String);

/// One-indexed line and column in source text.
#[invariant(*line > 0, "line numbers are one-indexed and cannot be zero")]
#[invariant(*column > 0, "column numbers are one-indexed and cannot be zero")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LineColumn {
    pub line: usize,
    pub column: usize,
}

impl LineColumn {
    #[requires(true)]
    #[ensures(true)]
    pub fn new(line: usize, column: usize) -> Result<Self, SourceLocationError> {
        if line == 0 {
            return Err(SourceLocationError::ZeroLine);
        }
        if column == 0 {
            return Err(SourceLocationError::ZeroColumn);
        }
        Ok(Self::from_data(data!(LineColumn { line, column })))
    }
}

/// Half-open source range.
///
/// Both byte and character offsets are stored because Rust string slicing is
/// byte-indexed, while user-facing diagnostics and the v0 corpus use character
/// offsets. Constructors validate only internal range consistency; callers are
/// responsible for deriving offsets from the same source text.
#[invariant(byte_start <= byte_end, "byte range start must not exceed end")]
#[invariant(char_start <= char_end, "character range start must not exceed end")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceSpan {
    pub source_id: Option<SourceId>,
    /// Byte start offset in the source text.
    ///
    /// This is reliable for spans constructed from source text or verbose JSON.
    /// Compact span JSON stores only character offsets, so compact-deserialized
    /// spans mirror character offsets into this field for compatibility; callers
    /// must not use those mirrored byte offsets for slicing non-ASCII text.
    pub byte_start: usize,
    /// Byte end offset in the source text.
    ///
    /// This is reliable for spans constructed from source text or verbose JSON.
    /// Compact span JSON stores only character offsets, so compact-deserialized
    /// spans mirror character offsets into this field for compatibility; callers
    /// must not use those mirrored byte offsets for slicing non-ASCII text.
    pub byte_end: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub start: Option<LineColumn>,
    pub end: Option<LineColumn>,
}

impl Serialize for SourceSpan {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        [self.char_start, self.char_end].serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SourceSpan {
    #[requires(true)]
    #[ensures(true)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[invariant(::Compact(_) => true, "compact wire spans are validated by SourceSpan::new")]
        #[invariant(::Verbose { .. } => true, "verbose wire spans are validated by SourceSpan::new")]
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum EncodedSpan {
            Compact([usize; 2]),
            Verbose {
                source_id: Option<SourceId>,
                byte_start: usize,
                byte_end: usize,
                char_start: usize,
                char_end: usize,
                #[allow(dead_code)]
                start: Option<LineColumn>,
                #[allow(dead_code)]
                end: Option<LineColumn>,
            },
        }

        match EncodedSpan::deserialize(deserializer)? {
            EncodedSpan::Compact([char_start, char_end]) => {
                // Compact spans are serialized as character offsets only. Keep
                // mirroring those values into byte offsets for compatibility,
                // but those byte offsets are unreliable for non-ASCII source.
                SourceSpan::new(None, char_start, char_end, char_start, char_end)
            }
            EncodedSpan::Verbose {
                source_id,
                byte_start,
                byte_end,
                char_start,
                char_end,
                ..
            } => SourceSpan::new(source_id, byte_start, byte_end, char_start, char_end),
        }
        .map_err(serde::de::Error::custom)
    }
}

impl SourceSpan {
    #[requires(true)]
    #[ensures(true)]
    pub fn new(
        source_id: Option<SourceId>,
        byte_start: usize,
        byte_end: usize,
        char_start: usize,
        char_end: usize,
    ) -> Result<Self, SourceLocationError> {
        if byte_end < byte_start {
            return Err(SourceLocationError::ByteRangeInverted {
                start: byte_start,
                end: byte_end,
            });
        }
        if char_end < char_start {
            return Err(SourceLocationError::CharRangeInverted {
                start: char_start,
                end: char_end,
            });
        }
        Ok(Self::from_data(data!(SourceSpan {
            source_id,
            byte_start,
            byte_end,
            char_start,
            char_end,
            start: None,
            end: None,
        })))
    }

    /// Attach optional human-facing line/column endpoints.
    ///
    /// The byte and character offsets remain the source of truth. In
    /// particular, this method intentionally does not require the two
    /// line/column endpoints to be present as a pair: `SourceSpan` has no such
    /// invariant, and callers may enrich either endpoint independently.
    #[requires(true)]
    #[ensures(ret.start == start)]
    #[ensures(ret.end == end)]
    #[ensures(ret.source_id == self.source_id)]
    #[ensures(ret.byte_start == self.byte_start)]
    #[ensures(ret.byte_end == self.byte_end)]
    #[ensures(ret.char_start == self.char_start)]
    #[ensures(ret.char_end == self.char_end)]
    pub fn with_line_columns(self, start: Option<LineColumn>, end: Option<LineColumn>) -> Self {
        self.with_data(data! {
            start: start,
            end: end,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn byte_len(&self) -> usize {
        self.byte_end - self.byte_start
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn char_len(&self) -> usize {
        self.char_end - self.char_start
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn is_empty(&self) -> bool {
        self.byte_start == self.byte_end && self.char_start == self.char_end
    }
}

/// A value with source provenance attached.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spanned<T> {
    pub span: SourceSpan,
    pub value: T,
}

#[invariant(true)]
#[invariant(::ByteRangeInverted => true, "error payloads preserve supplied byte endpoints without independently constraining them")]
#[invariant(::CharRangeInverted => true, "error payloads preserve supplied character endpoints without independently constraining them")]
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SourceLocationError {
    #[error("line numbers are one-indexed and cannot be zero")]
    ZeroLine,
    #[error("column numbers are one-indexed and cannot be zero")]
    ZeroColumn,
    /// Endpoints reported by a failed range operation; direct construction preserves any pair.
    #[error("byte range end {end} precedes start {start}")]
    ByteRangeInverted { start: usize, end: usize },
    /// Endpoints reported by a failed range operation; direct construction preserves any pair.
    #[error("character range end {end} precedes start {start}")]
    CharRangeInverted { start: usize, end: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::requires;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn span_rejects_inverted_ranges() {
        assert!(matches!(
            SourceSpan::new(None, 4, 3, 0, 0),
            Err(SourceLocationError::ByteRangeInverted { .. })
        ));
        assert!(matches!(
            SourceSpan::new(None, 0, 0, 4, 3),
            Err(SourceLocationError::CharRangeInverted { .. })
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn span_line_column_endpoints_remain_independently_optional() {
        let start = LineColumn::new(1, 1).expect("one-indexed location");
        let span = SourceSpan::new(None, 0, 0, 0, 0)
            .expect("empty ordered span")
            .with_line_columns(Some(start), None);

        assert_eq!(span.start, Some(start));
        assert_eq!(span.end, None);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn deserialization_rejects_invalid_spans() {
        let error = serde_json::from_str::<SourceSpan>(
            r#"{
                "source_id": null,
                "byte_start": 4,
                "byte_end": 3,
                "char_start": 0,
                "char_end": 0,
                "start": null,
                "end": null
            }"#,
        )
        .expect_err("inverted byte ranges must be rejected");

        assert!(
            error
                .to_string()
                .contains("byte range end 3 precedes start 4")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn compact_round_trip_preserves_chars_but_not_non_ascii_bytes() {
        let source = "aébc";
        let original = SourceSpan::new(None, 0, 3, 0, 2)
            .expect("test span uses real byte and character offsets");

        let encoded = serde_json::to_string(&original).expect("compact span serializes");
        assert_eq!(encoded, "[0,2]");

        let decoded: SourceSpan =
            serde_json::from_str(&encoded).expect("compact span deserializes");

        assert_eq!((decoded.char_start, decoded.char_end), (0, 2));
        assert_eq!((decoded.byte_start, decoded.byte_end), (0, 2));
        assert_ne!(decoded.byte_end, original.byte_end);
        assert!(
            source.get(decoded.byte_start..decoded.byte_end).is_none(),
            "compact-deserialized byte offsets can point inside a UTF-8 codepoint"
        );
        assert_eq!(
            serde_json::to_string(&decoded).expect("compact span reserializes"),
            encoded
        );
    }
}
