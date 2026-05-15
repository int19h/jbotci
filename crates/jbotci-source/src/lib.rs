//! Shared source-location types.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable identifier for an input source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceId(pub String);

/// One-indexed line and column in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LineColumn {
    pub line: usize,
    pub column: usize,
}

impl LineColumn {
    pub const fn new(line: usize, column: usize) -> Result<Self, SourceLocationError> {
        if line == 0 {
            return Err(SourceLocationError::ZeroLine);
        }
        if column == 0 {
            return Err(SourceLocationError::ZeroColumn);
        }
        Ok(Self { line, column })
    }
}

/// Half-open source range.
///
/// Both byte and character offsets are stored because Rust string slicing is
/// byte-indexed, while user-facing diagnostics and the v0 corpus use character
/// offsets. Constructors validate only internal range consistency; callers are
/// responsible for deriving offsets from the same source text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
}
