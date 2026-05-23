//! Shared source-location types.

use bityzba::{data, invariant, requires};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// Stable identifier for an input source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[invariant(true)]
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
    pub byte_start: usize,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct Spanned<T> {
    pub span: SourceSpan,
    pub value: T,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::ByteRangeInverted => true)]
#[invariant(::CharRangeInverted => true)]
pub enum SourceLocationError {
    #[error("line numbers are one-indexed and cannot be zero")]
    ZeroLine,
    #[error("column numbers are one-indexed and cannot be zero")]
    ZeroColumn,
    #[error("byte range end {end} precedes start {start}")]
    ByteRangeInverted { start: usize, end: usize },
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
}
