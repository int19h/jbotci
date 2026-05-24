//! Structured source diagnostics shared by parsers, renderers, and fixtures.

use bityzba::{invariant, new, requires};
use jbotci_source::{LineColumn, SourceId, SourceLocationError, SourceSpan};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Advice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticPhase {
    Morphology,
    Syntax,
}

#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticLabel {
    pub span: SourceSpan,
    pub message: String,
    pub primary: bool,
}

impl DiagnosticLabel {
    #[requires(!message.is_empty())]
    #[ensures(true)]
    pub fn new(span: SourceSpan, message: String, primary: bool) -> Self {
        new!(DiagnosticLabel {
            span,
            message,
            primary,
        })
    }
}

#[invariant(!code.is_empty())]
#[invariant(!message.is_empty())]
#[invariant(!labels.is_empty())]
#[invariant(labels.iter().any(|label| label.primary))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub phase: DiagnosticPhase,
    pub code: String,
    pub message: String,
    pub labels: Vec<DiagnosticLabel>,
    pub notes: Vec<String>,
    pub word_index: Option<usize>,
}

impl Diagnostic {
    #[requires(!code.is_empty())]
    #[requires(!message.is_empty())]
    #[requires(!labels.is_empty())]
    #[requires(labels.iter().any(|label| label.primary))]
    #[ensures(true)]
    pub fn new(
        severity: DiagnosticSeverity,
        phase: DiagnosticPhase,
        code: String,
        message: String,
        labels: Vec<DiagnosticLabel>,
        notes: Vec<String>,
        word_index: Option<usize>,
    ) -> Self {
        new!(Diagnostic {
            severity,
            phase,
            code,
            message,
            labels,
            notes,
            word_index,
        })
    }

    #[requires(true)]
    #[ensures(ret.primary)]
    pub fn primary_label(&self) -> &DiagnosticLabel {
        self.labels
            .iter()
            .find(|label| label.primary)
            .expect("diagnostic invariant guarantees a primary label")
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::CharOffsetOutOfBounds => true)]
#[invariant(::ByteOffsetOutOfBounds => true)]
#[invariant(::ByteOffsetNotCharBoundary => true)]
#[invariant(::SourceLocation(..) => true)]
pub enum DiagnosticSpanError {
    #[error("character offset {offset} exceeds source character length {source_len}")]
    CharOffsetOutOfBounds { offset: usize, source_len: usize },
    #[error("byte offset {offset} exceeds source byte length {source_len}")]
    ByteOffsetOutOfBounds { offset: usize, source_len: usize },
    #[error("byte offset {offset} is not a UTF-8 character boundary")]
    ByteOffsetNotCharBoundary { offset: usize },
    #[error("invalid source span: {0}")]
    SourceLocation(#[from] SourceLocationError),
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|span| span.char_start == char_start) || ret.is_err())]
pub fn source_span_from_char_offsets(
    source_id: Option<SourceId>,
    source: &str,
    char_start: usize,
    char_end: usize,
) -> Result<SourceSpan, DiagnosticSpanError> {
    let source_len = source.chars().count();
    if char_start > source_len {
        return Err(DiagnosticSpanError::CharOffsetOutOfBounds {
            offset: char_start,
            source_len,
        });
    }
    if char_end > source_len {
        return Err(DiagnosticSpanError::CharOffsetOutOfBounds {
            offset: char_end,
            source_len,
        });
    }
    let byte_start = byte_offset_for_char_offset(source, char_start)?;
    let byte_end = byte_offset_for_char_offset(source, char_end)?;
    SourceSpan::new(source_id, byte_start, byte_end, char_start, char_end)
        .map_err(DiagnosticSpanError::SourceLocation)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|span| span.byte_start == byte_start) || ret.is_err())]
pub fn source_span_from_byte_offsets(
    source_id: Option<SourceId>,
    source: &str,
    byte_start: usize,
    byte_end: usize,
) -> Result<SourceSpan, DiagnosticSpanError> {
    validate_byte_offset(source, byte_start)?;
    validate_byte_offset(source, byte_end)?;
    let char_start = char_offset_for_byte_offset(source, byte_start)?;
    let char_end = char_offset_for_byte_offset(source, byte_end)?;
    SourceSpan::new(source_id, byte_start, byte_end, char_start, char_end)
        .map_err(DiagnosticSpanError::SourceLocation)
}

#[requires(char_offset <= source.chars().count())]
#[ensures(ret.as_ref().is_ok_and(|offset| *offset <= source.len()) || ret.is_err())]
pub fn byte_offset_for_char_offset(
    source: &str,
    char_offset: usize,
) -> Result<usize, DiagnosticSpanError> {
    let source_len = source.chars().count();
    if char_offset > source_len {
        return Err(DiagnosticSpanError::CharOffsetOutOfBounds {
            offset: char_offset,
            source_len,
        });
    }
    Ok(source
        .char_indices()
        .map(|(index, _)| index)
        .nth(char_offset)
        .unwrap_or(source.len()))
}

#[requires(byte_offset <= source.len())]
#[requires(source.is_char_boundary(byte_offset))]
#[ensures(ret.as_ref().is_ok_and(|offset| *offset <= source.chars().count()) || ret.is_err())]
pub fn char_offset_for_byte_offset(
    source: &str,
    byte_offset: usize,
) -> Result<usize, DiagnosticSpanError> {
    validate_byte_offset(source, byte_offset)?;
    Ok(source[..byte_offset].chars().count())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|line_column| line_column.line > 0 && line_column.column > 0) || ret.is_err())]
pub fn line_column_for_byte_offset(
    source: &str,
    byte_offset: usize,
) -> Result<LineColumn, DiagnosticSpanError> {
    validate_byte_offset(source, byte_offset)?;
    let mut line = 1usize;
    let mut column = 1usize;
    for ch in source[..byte_offset].chars() {
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    LineColumn::new(line, column).map_err(DiagnosticSpanError::SourceLocation)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| source.contains(text) || text.is_empty()) || ret.is_err())]
pub fn source_text_for_span(
    source: &str,
    span: &SourceSpan,
) -> Result<String, DiagnosticSpanError> {
    validate_byte_offset(source, span.byte_start)?;
    validate_byte_offset(source, span.byte_end)?;
    Ok(source[span.byte_start..span.byte_end].to_owned())
}

#[requires(true)]
#[ensures(ret.is_ok() -> offset <= source.len())]
fn validate_byte_offset(source: &str, offset: usize) -> Result<(), DiagnosticSpanError> {
    if offset > source.len() {
        return Err(DiagnosticSpanError::ByteOffsetOutOfBounds {
            offset,
            source_len: source.len(),
        });
    }
    if !source.is_char_boundary(offset) {
        return Err(DiagnosticSpanError::ByteOffsetNotCharBoundary { offset });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::requires;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn converts_non_ascii_character_offsets_to_byte_spans() {
        let source = "coi gleki ĭa";
        let span = source_span_from_char_offsets(None, source, 10, 12).expect("span");
        assert_eq!(span.byte_start, 10);
        assert_eq!(span.byte_end, 13);
        assert_eq!(
            source_text_for_span(source, &span).expect("source text"),
            "ĭa"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_byte_offsets_inside_a_codepoint() {
        let error = source_span_from_byte_offsets(None, "ĭa", 1, 2).expect_err("bad boundary");
        assert!(matches!(
            error,
            DiagnosticSpanError::ByteOffsetNotCharBoundary { offset: 1 }
        ));
    }
}
