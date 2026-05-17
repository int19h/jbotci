//! The Complete Lojban Language reference model.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[bityzba::invariant(true)]
pub struct CllReference {
    pub chapter: u16,
    pub section_number: String,
    pub section_id: String,
    pub example_number: Option<String>,
    pub example_id: Option<String>,
    pub source_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[bityzba::invariant(true)]
pub struct CllExample {
    pub reference: CllReference,
    pub lojban: String,
    pub gloss_en: Option<String>,
    pub translation_en: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[bityzba::invariant(true)]
pub enum CllError {
    #[error("CLL loading is not implemented yet")]
    NotImplemented,
}
