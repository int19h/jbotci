//! Semantic search abstractions.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[bityzba::invariant(true)]
pub struct Embedding {
    pub model: String,
    pub dimensions: usize,
    pub values: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[bityzba::invariant(true)]
pub struct SearchHit<T> {
    pub item: T,
    pub score: f32,
}

#[bityzba::contract_trait]
pub trait VectorSearchIndex<T> {
    #[bityzba::requires(true)]
    #[bityzba::ensures(true)]
    fn search(&self, query: &Embedding, limit: usize) -> Result<Vec<SearchHit<T>>, SearchError>;
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[bityzba::invariant(true)]
pub enum SearchError {
    #[error("semantic search is not implemented yet")]
    NotImplemented,
    #[error("embedding dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
}
