//! F2LLM window and pooling rules without a device runtime.

use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, invariant, new, requires};

/// Every window reserves its final slot for the end token.
#[invariant(*max_length > 1)]
#[derive(Debug, Clone, Copy)]
pub struct TokenWindowRule {
    max_length: usize,
}

impl TokenWindowRule {
    #[requires(true)]
    #[ensures(ret.is_ok() == (max_length > 1))]
    pub fn new(max_length: usize) -> Result<Self, String> {
        if max_length <= 1 {
            return Err("embedding window length must exceed one token".to_owned());
        }
        Ok(new!(TokenWindowRule {
            max_length: max_length
        }))
    }

    /// Empty input retains one end token so that it can produce an embedding.
    #[requires(true)]
    #[ensures(ret.len() == content.len().div_ceil(self.max_length - 1).max(1))]
    #[expensive_ensures(ret.iter().all(|window| window.len() <= self.max_length && window.last() == Some(&end_token)))]
    pub fn split<T: Copy + Eq>(&self, content: &[T], end_token: T) -> Vec<Vec<T>> {
        if content.is_empty() {
            return vec![vec![end_token]];
        }
        content
            .chunks(self.max_length - 1)
            .map(|chunk| {
                let mut window = Vec::with_capacity(chunk.len() + 1);
                window.extend_from_slice(chunk);
                window.push(end_token);
                window
            })
            .collect()
    }
}

/// Typed failures for embedding-vector validation and pooling.
#[invariant(::NoVectors { vector_count } => *vector_count == 0)]
#[invariant(::DimensionMismatch {
    expected_dimensions,
    actual_dimensions,
} => *expected_dimensions > 0 && expected_dimensions != actual_dimensions)]
#[invariant(::NonFiniteComponent { value_bits, .. } =>
    !f32::from_bits(*value_bits).is_finite())]
#[invariant(::ZeroNorm { norm_bits } => f32::from_bits(*norm_bits) == 0.0)]
#[invariant(::NonFiniteNorm { norm_bits } => !f32::from_bits(*norm_bits).is_finite())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddingVectorError {
    NoVectors {
        vector_count: usize,
    },
    DimensionMismatch {
        expected_dimensions: usize,
        actual_dimensions: usize,
    },
    NonFiniteComponent {
        dimension: usize,
        value_bits: u32,
    },
    ZeroNorm {
        norm_bits: u32,
    },
    NonFiniteNorm {
        norm_bits: u32,
    },
}

impl EmbeddingVectorError {
    #[requires(true)]
    #[ensures(ret == matches!(
        self.as_data(),
        data!(EmbeddingVectorError::ZeroNorm { .. })
    ))]
    pub fn is_zero_norm(&self) -> bool {
        matches!(self.as_data(), data!(EmbeddingVectorError::ZeroNorm { .. }))
    }
}

impl fmt::Display for EmbeddingVectorError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(EmbeddingVectorError::NoVectors { .. }) => {
                formatter.write_str("at least one embedding vector is required for pooling")
            }
            data!(EmbeddingVectorError::DimensionMismatch {
                expected_dimensions,
                actual_dimensions,
            }) => write!(
                formatter,
                "embedding vector has {actual_dimensions} dimensions, expected {expected_dimensions}"
            ),
            data!(EmbeddingVectorError::NonFiniteComponent {
                dimension,
                value_bits,
            }) => write!(
                formatter,
                "embedding vector contains non-finite value {} at dimension {dimension}",
                f32::from_bits(*value_bits)
            ),
            data!(EmbeddingVectorError::ZeroNorm { .. }) => {
                formatter.write_str("embedding vector has zero norm")
            }
            data!(EmbeddingVectorError::NonFiniteNorm { norm_bits }) => write!(
                formatter,
                "embedding vector norm is non-finite: {}",
                f32::from_bits(*norm_bits)
            ),
        }
    }
}

impl std::error::Error for EmbeddingVectorError {}

#[requires(expected_dimensions > 0)]
#[ensures(ret.as_ref().is_ok_and(|magnitude| {
    vector.len() == expected_dimensions
        && vector.iter().all(|value| value.is_finite())
        && magnitude.is_finite()
        && *magnitude > 0.0
}) || ret.is_err())]
pub fn embedding_normalization_magnitude(
    vector: &[f32],
    expected_dimensions: usize,
) -> Result<f32, EmbeddingVectorError> {
    if vector.len() != expected_dimensions {
        return Err(new!(EmbeddingVectorError::DimensionMismatch {
            expected_dimensions,
            actual_dimensions: vector.len(),
        }));
    }
    for (index, value) in vector.iter().enumerate() {
        if !value.is_finite() {
            return Err(new!(EmbeddingVectorError::NonFiniteComponent {
                dimension: index,
                value_bits: value.to_bits(),
            }));
        }
    }
    let magnitude = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if magnitude == 0.0 {
        return Err(new!(EmbeddingVectorError::ZeroNorm {
            norm_bits: magnitude.to_bits(),
        }));
    }
    if !magnitude.is_finite() {
        return Err(new!(EmbeddingVectorError::NonFiniteNorm {
            norm_bits: magnitude.to_bits(),
        }));
    }
    Ok(magnitude)
}

#[requires(vectors.len() == token_counts.len())]
#[requires(token_counts.iter().all(|count| *count > 0))]
#[requires(dimensions > 0)]
#[ensures(ret.as_ref().is_ok_and(|vector| {
    vector.len() == dimensions
        && vector.iter().all(|value| value.is_finite())
        && vector.iter().any(|value| *value != 0.0)
}) || ret.is_err())]
pub fn token_weighted_mean_normalized(
    vectors: &[Vec<f32>],
    token_counts: &[usize],
    dimensions: usize,
) -> Result<Vec<f32>, EmbeddingVectorError> {
    if vectors.is_empty() {
        return Err(new!(EmbeddingVectorError::NoVectors {
            vector_count: vectors.len(),
        }));
    }
    let mut pooled = vec![0.0; dimensions];
    for (vector, token_count) in vectors.iter().zip(token_counts) {
        embedding_normalization_magnitude(vector, dimensions)?;
        for (index, value) in vector.iter().enumerate() {
            pooled[index] += *value * *token_count as f32;
        }
    }
    let divisor = token_counts.iter().sum::<usize>() as f32;
    for value in &mut pooled {
        *value /= divisor;
    }
    let magnitude = embedding_normalization_magnitude(&pooled, dimensions)?;
    normalize_validated_in_place(&mut pooled, magnitude);
    Ok(pooled)
}

#[requires(!vector.is_empty())]
#[requires(vector.iter().all(|value| value.is_finite()))]
#[requires(vector.iter().any(|value| *value != 0.0))]
#[requires(magnitude.is_finite() && magnitude > 0.0)]
#[ensures(
    vector.iter().all(|value| value.is_finite())
        && vector.iter().any(|value| *value != 0.0)
)]
pub fn normalize_validated_in_place(vector: &mut [f32], magnitude: f32) {
    for value in &mut *vector {
        *value /= magnitude;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn window_boundaries_reserve_eos_and_weight_the_short_tail() {
        let rule = TokenWindowRule::new(512).unwrap();
        assert!(TokenWindowRule::new(1).is_err());
        for count in [0, 510, 511, 512, 1021, 1022, 1023] {
            let content = vec![7_u32; count];
            let windows = rule.split(&content, 151645);
            assert_eq!(windows.len(), count.div_ceil(511).max(1));
            assert!(
                windows
                    .iter()
                    .all(|w| w.len() <= 512 && w.last() == Some(&151645))
            );
            assert_eq!(windows.iter().map(|w| w.len() - 1).sum::<usize>(), count);
            assert!(count == 0 || windows.iter().all(|w| w.len() > 1));
        }
        let pooled =
            token_weighted_mean_normalized(&[vec![1.0, 0.0], vec![0.0, 1.0]], &[512, 2], 2)
                .unwrap();
        assert!((pooled[1] / pooled[0] - 2.0 / 512.0).abs() < 1e-7);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mean_pool_normalizes_after_averaging() {
        let pooled = token_weighted_mean_normalized(&[vec![1.0, 0.0], vec![0.0, 1.0]], &[1, 1], 2)
            .expect("valid pooling");
        let expected = 1.0 / 2.0_f32.sqrt();
        assert!((pooled[0] - expected).abs() < 1e-6);
        assert!((pooled[1] - expected).abs() < 1e-6);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mean_pool_rejects_exact_window_cancellation() {
        let error = token_weighted_mean_normalized(&[vec![1.0, 0.0], vec![-1.0, 0.0]], &[1, 1], 2)
            .expect_err("cancelling windows must not produce a successful zero embedding");

        assert!(error.is_zero_norm());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mean_pool_rejects_empty_misdimensioned_and_non_finite_inputs() {
        assert!(
            token_weighted_mean_normalized(&[], &[], 2)
                .unwrap_err()
                .to_string()
                .contains("at least one")
        );
        assert!(
            token_weighted_mean_normalized(&[vec![1.0]], &[1], 2)
                .unwrap_err()
                .to_string()
                .contains("1 dimensions, expected 2")
        );
        assert!(
            token_weighted_mean_normalized(&[vec![1.0, f32::INFINITY]], &[1], 2)
                .unwrap_err()
                .to_string()
                .contains("non-finite value")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedding_vectors_must_be_finite_and_non_zero() {
        assert_eq!(
            embedding_normalization_magnitude(&[0.0, 1.0], 2).unwrap(),
            1.0
        );
        let zero_error = embedding_normalization_magnitude(&[0.0, -0.0], 2).unwrap_err();
        assert!(zero_error.is_zero_norm());
        let nan_error = embedding_normalization_magnitude(&[1.0, f32::NAN], 2).unwrap_err();
        assert_eq!(
            nan_error,
            new!(EmbeddingVectorError::NonFiniteComponent {
                dimension: 1,
                value_bits: f32::NAN.to_bits(),
            })
        );
    }
}
