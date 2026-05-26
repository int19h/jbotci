//! Lojban semantic model and builder facade.

pub mod references;

use bityzba::{invariant, requires};
use jbotci_source::SourceSpan;
use jbotci_syntax::TextSyntax;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct SemanticText {
    pub source: Option<SourceSpan>,
    pub leading_modifiers: Vec<ScopedModifier>,
    pub paragraphs: Vec<SemanticParagraph>,
    pub trailing_modifiers: Vec<ScopedModifier>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct SemanticParagraph {
    pub source: Option<SourceSpan>,
    pub statements: Vec<SemanticStatement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct SemanticStatement {
    pub source: Option<SourceSpan>,
    pub content: StatementContent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[invariant(true)]
pub enum StatementContent {
    Empty,
    Placeholder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct ScopedModifier {
    pub source: Option<SourceSpan>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::NotImplemented => true)]
pub enum SemanticsError {
    #[error("semantic analysis is not implemented yet")]
    NotImplemented,
}

#[requires(true)]
#[ensures(true)]
pub fn build_semantic_text(_syntax: &TextSyntax) -> Result<SemanticText, SemanticsError> {
    Err(SemanticsError::NotImplemented)
}
