//! Semantic search abstractions.

pub mod vlacku;

use std::fmt;

use bityzba::{contract_trait, data, invariant, requires};
use serde::{Deserialize, Serialize};

#[invariant(!model.is_empty())]
#[invariant(*dimensions == values.len())]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Embedding {
    pub model: String,
    pub dimensions: usize,
    pub values: Vec<f32>,
}

#[invariant(score.is_finite())]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit<T> {
    pub item: T,
    pub score: f32,
}

#[contract_trait]
pub trait VectorSearchIndex<T> {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|hits| hits.len() <= limit) || ret.is_err())]
    fn search(&self, query: &Embedding, limit: usize) -> Result<Vec<SearchHit<T>>, SearchError>;
}

#[invariant(::DimensionMismatch { expected, actual } => expected != actual)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchError {
    DimensionMismatch { expected: usize, actual: usize },
}

impl fmt::Display for SearchError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(SearchError::DimensionMismatch { expected, actual }) => {
                write!(
                    formatter,
                    "embedding dimension mismatch: expected {expected}, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for SearchError {}
