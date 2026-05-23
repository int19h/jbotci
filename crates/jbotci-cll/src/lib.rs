//! The Complete Lojban Language reference model.

use bityzba::invariant;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[invariant(*chapter > 0)]
#[invariant(!section_number.is_empty())]
#[invariant(!section_id.is_empty())]
#[invariant(example_number.as_ref().is_none_or(|number| !number.is_empty()))]
#[invariant(example_id.as_ref().is_none_or(|id| !id.is_empty()))]
#[invariant(!source_path.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllReference {
    pub chapter: u16,
    pub section_number: String,
    pub section_id: String,
    pub example_number: Option<String>,
    pub example_id: Option<String>,
    pub source_path: String,
}

#[invariant(!lojban.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CllExample {
    pub reference: CllReference,
    pub lojban: String,
    pub gloss_en: Option<String>,
    pub translation_en: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CllError {
    #[error("CLL loading is not implemented yet")]
    NotImplemented,
}
