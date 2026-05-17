//! Lujvo composition and decomposition.

use bityzba::{invariant, requires};
use jbotci_morphology::LujvoSegment;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct LujvoPlan {
    pub sources: Vec<LujvoSource>,
    pub segments: Vec<LujvoSegment>,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct LujvoSource {
    pub word: String,
    pub fixed_rafsi: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
pub enum JvozbaError {
    #[error("jvozba is not implemented yet")]
    NotImplemented,
}

#[requires(true)]
#[ensures(true)]
pub fn compose_lujvo(_sources: &[LujvoSource]) -> Result<LujvoPlan, JvozbaError> {
    Err(JvozbaError::NotImplemented)
}
