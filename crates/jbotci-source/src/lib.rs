//! Shared source-location types.

use contracts::ensures;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Stable identifier for an input source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceId(pub String);

/// One-indexed line and column in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct LineColumn {
    pub line: usize,
    pub column: usize,
}

impl LineColumn {
    #[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(LineColumn::is_valid))]
    pub fn new(line: usize, column: usize) -> Result<Self, SourceLocationError> {
        if line == 0 {
            return Err(SourceLocationError::ZeroLine);
        }
        if column == 0 {
            return Err(SourceLocationError::ZeroColumn);
        }
        Ok(Self { line, column })
    }

    pub const fn is_valid(&self) -> bool {
        self.line > 0 && self.column > 0
    }
}

impl<'de> Deserialize<'de> for LineColumn {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct UncheckedLineColumn {
            line: usize,
            column: usize,
        }

        let unchecked = UncheckedLineColumn::deserialize(deserializer)?;
        Self::new(unchecked.line, unchecked.column).map_err(D::Error::custom)
    }
}

/// Half-open source range.
///
/// Both byte and character offsets are stored because Rust string slicing is
/// byte-indexed, while user-facing diagnostics and the v0 corpus use character
/// offsets. Constructors validate only internal range consistency; callers are
/// responsible for deriving offsets from the same source text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct SourceSpan {
    pub source_id: Option<SourceId>,
    pub byte_start: usize,
    pub byte_end: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub start: Option<LineColumn>,
    pub end: Option<LineColumn>,
}

impl SourceSpan {
    #[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(SourceSpan::is_valid))]
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
        Ok(Self {
            source_id,
            byte_start,
            byte_end,
            char_start,
            char_end,
            start: None,
            end: None,
        })
    }

    pub fn is_valid(&self) -> bool {
        self.byte_start <= self.byte_end
            && self.char_start <= self.char_end
            && self.start.as_ref().is_none_or(LineColumn::is_valid)
            && self.end.as_ref().is_none_or(LineColumn::is_valid)
    }

    pub const fn byte_len(&self) -> usize {
        self.byte_end - self.byte_start
    }

    pub const fn char_len(&self) -> usize {
        self.char_end - self.char_start
    }

    pub const fn is_empty(&self) -> bool {
        self.byte_start == self.byte_end && self.char_start == self.char_end
    }
}

impl<'de> Deserialize<'de> for SourceSpan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct UncheckedSourceSpan {
            source_id: Option<SourceId>,
            byte_start: usize,
            byte_end: usize,
            char_start: usize,
            char_end: usize,
            start: Option<LineColumn>,
            end: Option<LineColumn>,
        }

        let unchecked = UncheckedSourceSpan::deserialize(deserializer)?;
        let mut span = Self::new(
            unchecked.source_id,
            unchecked.byte_start,
            unchecked.byte_end,
            unchecked.char_start,
            unchecked.char_end,
        )
        .map_err(D::Error::custom)?;
        span.start = unchecked.start;
        span.end = unchecked.end;
        if span.is_valid() {
            Ok(span)
        } else {
            Err(D::Error::custom("invalid source span"))
        }
    }
}

/// A value with source provenance attached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spanned<T> {
    pub span: SourceSpan,
    pub value: T,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
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

    #[test]
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

        assert!(error.to_string().contains("byte range end"));
    }
}
