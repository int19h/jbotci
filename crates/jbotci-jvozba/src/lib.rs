//! Lujvo composition and decomposition.

use bityzba::{invariant, requires};
use jbotci_morphology::Jvopau;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[invariant(!sources.is_empty())]
#[invariant(!parts.is_empty())]
#[invariant(!output.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LujvoPlan {
    pub sources: Vec<LujvoSource>,
    pub parts: Vec<Jvopau>,
    pub output: String,
}

#[invariant(!word.is_empty())]
#[invariant(fixed_rafsi.as_ref().is_none_or(|rafsi| !rafsi.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LujvoSource {
    pub word: String,
    pub fixed_rafsi: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum JvozbaError {
    #[error("jvozba is not implemented yet")]
    NotImplemented,
}

#[requires(true)]
#[ensures(true)]
pub fn compose_lujvo(_sources: &[LujvoSource]) -> Result<LujvoPlan, JvozbaError> {
    Err(JvozbaError::NotImplemented)
}
