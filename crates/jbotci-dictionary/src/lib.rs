//! Dictionary model.

use jbotci_morphology::WordKind;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dictionary {
    pub entries: Vec<WordEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordEntry {
    pub word: String,
    pub kind: WordKind,
    pub rafsi: Vec<String>,
    pub definitions: Vec<Definition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Definition {
    pub language: String,
    pub text: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DictionaryError {
    #[error("dictionary loading is not implemented yet")]
    NotImplemented,
}
