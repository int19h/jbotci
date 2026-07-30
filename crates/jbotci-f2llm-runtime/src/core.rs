//! Target-neutral tokenizer, windowing, quantization-layout, and vector-pooling logic.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_invariant, invariant, new, requires};
use fancy_regex::Regex;
use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;

pub const DEFAULT_MAX_SEQUENCE_LENGTH: usize = 512;

const TOKEN_PATTERN: &str = r"('s|'t|'re|'ve|'m|'ll|'d)|[^\r\n\p{L}\p{N}]?\p{L}+|\p{N}| ?[^\s\p{L}\p{N}]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+";

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct TokenizerArtifact {
    schema_version: u32,
    vocab: HashMap<String, u32>,
    merges: Vec<MergeSpec>,
    special_tokens: SpecialTokens,
}

#[invariant(true)]
#[derive(Debug, Deserialize)]
struct SpecialTokens {
    eos_id: u32,
}

#[invariant(::Text(_) => true)]
#[invariant(::Pair(_) => true)]
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MergeSpec {
    Text(String),
    Pair(Vec<String>),
}

#[invariant(!vocab.is_empty())]
#[invariant(*eos_id > 0)]
#[invariant(byte_encoder.iter().all(|encoded| !encoded.is_empty()))]
#[derive(Debug)]
pub struct QwenByteBpeTokenizer {
    vocab: HashMap<String, u32>,
    eos_id: u32,
    byte_encoder: [String; 256],
    merge_ranks: HashMap<(String, String), usize>,
    cache: RefCell<HashMap<String, Vec<u32>>>,
    pattern: Regex,
}

impl QwenByteBpeTokenizer {
    #[requires(!bytes.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|tokenizer| tokenizer.eos_id() > 0) || ret.is_err())]
    pub fn from_compact_json(bytes: &[u8]) -> Result<Self, String> {
        let artifact: TokenizerArtifact = serde_json::from_slice(bytes)
            .map_err(|error| format!("failed to parse F2LLM tokenizer JSON: {error}"))?;
        if artifact.schema_version != 1 {
            return Err(format!(
                "unsupported F2LLM tokenizer schema version: {}",
                artifact.schema_version
            ));
        }
        if artifact.special_tokens.eos_id == 0 {
            return Err("F2LLM tokenizer eos_id must be positive".to_owned());
        }
        if artifact.vocab.is_empty() {
            return Err("F2LLM tokenizer vocabulary must not be empty".to_owned());
        }
        let mut merge_ranks = HashMap::with_capacity(artifact.merges.len());
        for (rank, merge) in artifact.merges.into_iter().enumerate() {
            if let Some((left, right)) = merge_pair(merge) {
                merge_ranks.insert((left, right), rank);
            }
        }
        let pattern = Regex::new(TOKEN_PATTERN)
            .map_err(|error| format!("failed to compile F2LLM tokenizer regex: {error}"))?;
        Ok(new!(QwenByteBpeTokenizer {
            vocab: artifact.vocab,
            eos_id: artifact.special_tokens.eos_id,
            byte_encoder: bytes_to_unicode(),
            merge_ranks,
            cache: RefCell::new(HashMap::new()),
            pattern: pattern,
        }))
    }

    #[requires(true)]
    #[ensures(ret == self.eos_id)]
    pub fn eos_id(&self) -> u32 {
        self.eos_id
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|ids| !ids.is_empty()) || ret.is_err())]
    pub fn encode_untruncated(&self, text: &str) -> Result<Vec<u32>, String> {
        let normalized = String::from(text).nfc().collect::<String>();
        let mut ids = Vec::new();
        for match_result in self.pattern.find_iter(&normalized) {
            let token_match =
                match_result.map_err(|error| format!("F2LLM tokenizer regex failed: {error}"))?;
            let byte_level = self.byte_level_encode(token_match.as_str());
            ids.extend(self.bpe(&byte_level)?);
        }
        ids.push(self.eos_id);
        Ok(ids)
    }

    #[requires(max_length > 0)]
    #[ensures(ret.as_ref().is_ok_and(|windows| windows.iter().all(|window| !window.is_empty() && window.len() <= max_length)) || ret.is_err())]
    pub fn token_windows(&self, text: &str, max_length: usize) -> Result<Vec<Vec<u32>>, String> {
        let ids = self.encode_untruncated(text)?;
        Ok(ids.chunks(max_length).map(|chunk| chunk.to_vec()).collect())
    }

    #[requires(true)]
    #[ensures(true)]
    fn byte_level_encode(&self, text: &str) -> String {
        let mut encoded = String::new();
        for byte in text.as_bytes() {
            encoded.push_str(&self.byte_encoder[*byte as usize]);
        }
        encoded
    }

    #[requires(true)]
    #[ensures(true)]
    fn bpe(&self, token: &str) -> Result<Vec<u32>, String> {
        if let Some(cached) = self.cache.borrow().get(token) {
            return Ok(cached.clone());
        }
        let mut word = token.chars().map(String::from).collect::<Vec<_>>();
        if word.is_empty() {
            return Ok(Vec::new());
        }
        loop {
            let mut best_rank = usize::MAX;
            let mut best_pair: Option<(String, String)> = None;
            for index in 0..word.len().saturating_sub(1) {
                if let Some(rank) = self
                    .merge_ranks
                    .get(&(word[index].clone(), word[index + 1].clone()))
                {
                    if *rank < best_rank {
                        best_rank = *rank;
                        best_pair = Some((word[index].clone(), word[index + 1].clone()));
                    }
                }
            }
            let Some((left, right)) = best_pair else {
                break;
            };
            let mut next = Vec::with_capacity(word.len());
            let mut index = 0;
            while index < word.len() {
                if index + 1 < word.len() && word[index] == left && word[index + 1] == right {
                    next.push(format!("{left}{right}"));
                    index += 2;
                } else {
                    next.push(word[index].clone());
                    index += 1;
                }
            }
            word = next;
            if word.len() == 1 {
                break;
            }
        }
        let ids = word
            .iter()
            .map(|piece| {
                self.vocab.get(piece).copied().ok_or_else(|| {
                    format!("F2LLM tokenizer piece is missing from vocab: {piece:?}")
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.cache
            .borrow_mut()
            .insert(token.to_owned(), ids.clone());
        Ok(ids)
    }
}

#[requires(groups > 0)]
#[requires(group_size > 0)]
#[ensures(ret == groups * group_size)]
pub(crate) fn q4_padded_row_stride(groups: usize, group_size: usize) -> usize {
    groups * group_size
}

#[requires(rows > 0)]
#[requires(groups > 0)]
#[requires(group_size > 0)]
#[ensures(ret > 0)]
pub(crate) fn q4_packed_byte_len(rows: usize, groups: usize, group_size: usize) -> usize {
    (rows * q4_padded_row_stride(groups, group_size)).div_ceil(2)
}

#[requires(col < groups * group_size)]
#[requires(groups > 0)]
#[requires(group_size > 0)]
#[ensures(ret >= row * groups * group_size)]
pub(crate) fn q4_padded_element_index(
    row: usize,
    col: usize,
    groups: usize,
    group_size: usize,
) -> usize {
    row * q4_padded_row_stride(groups, group_size) + col
}

#[requires(!name.is_empty())]
#[requires(kind == "q4_onnx_gather" || kind == "q4_onnx_matmul")]
#[requires(group_size > 0)]
#[requires(groups > 0)]
#[ensures(true)]
pub(crate) fn validate_q4_tensor_storage(
    name: &str,
    kind: &str,
    shape: [usize; 2],
    group_size: usize,
    groups: usize,
    qweight_byte_length: usize,
    scales_byte_length: usize,
    zero_points_byte_length: usize,
) -> Result<(), String> {
    let expected_groups = shape[1].div_ceil(group_size);
    if groups != expected_groups {
        return Err(format!(
            "{name} groups mismatch: expected {expected_groups}, got {groups}"
        ));
    }
    let expected_qweight = q4_packed_byte_len(shape[0], groups, group_size);
    if qweight_byte_length != expected_qweight {
        return Err(format!(
            "{name} qweight byte length mismatch: expected {expected_qweight}, got {qweight_byte_length}"
        ));
    }
    let expected_quant_params = shape[0] * groups * 4;
    if scales_byte_length != expected_quant_params {
        return Err(format!(
            "{name} scales byte length mismatch: expected {expected_quant_params}, got {scales_byte_length}"
        ));
    }
    let expected_zero_points = if kind == "q4_onnx_gather" {
        (shape[0] * groups).div_ceil(2)
    } else {
        expected_quant_params
    };
    if zero_points_byte_length != expected_zero_points {
        return Err(format!(
            "{name} zero_points byte length mismatch: expected {expected_zero_points}, got {zero_points_byte_length}"
        ));
    }
    Ok(())
}

#[requires(!label.is_empty())]
#[ensures(true)]
pub(crate) fn validate_chunk_layout(
    label: &str,
    byte_length: usize,
    chunks: &[(&str, usize, usize, &str)],
) -> Result<(), String> {
    if byte_length == 0 {
        return Err(format!("{label} byte_length must be positive"));
    }
    if chunks.is_empty() {
        return Err(format!("{label} must contain at least one chunk"));
    }
    let mut covered = 0usize;
    for (url, byte_offset, chunk_byte_length, sha256) in chunks {
        if url.trim().is_empty() {
            return Err(format!("{label} contains a chunk with an empty URL"));
        }
        if *chunk_byte_length == 0 {
            return Err(format!("{label} chunk {url} byte_length must be positive"));
        }
        if *byte_offset % 4 != 0 {
            return Err(format!(
                "{label} chunk {url} byte_offset must be 4-byte aligned"
            ));
        }
        if *byte_offset != covered {
            return Err(format!(
                "{label} chunk {url} starts at {byte_offset}, expected {covered}"
            ));
        }
        validate_sha256_hex(sha256, &format!("{label} chunk {url}"))?;
        covered = covered
            .checked_add(*chunk_byte_length)
            .ok_or_else(|| format!("{label} chunk byte lengths overflow usize"))?;
    }
    if covered != byte_length {
        return Err(format!(
            "{label} chunks cover {covered} bytes, expected {byte_length}"
        ));
    }
    Ok(())
}

#[requires(!name.is_empty())]
#[ensures(true)]
pub(crate) fn validate_sha256_hex(value: &str, name: &str) -> Result<(), String> {
    if value.len() != 64 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        Err(format!("{name} must be a 64-character SHA-256 hex digest"))
    } else {
        Ok(())
    }
}

#[requires(vocab_size > 0)]
#[ensures(true)]
pub(crate) fn validate_token_ids(token_ids: &[u32], vocab_size: usize) -> Result<(), String> {
    let Some((index, token_id)) = token_ids
        .iter()
        .enumerate()
        .find(|(_, token_id)| **token_id as usize >= vocab_size)
    else {
        return Ok(());
    };
    Err(format!(
        "F2LLM tokenizer emitted token id {token_id} at offset {index}, outside vocab size {vocab_size}"
    ))
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
pub(crate) fn embedding_normalization_magnitude(
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

#[invariant(!token_ids.is_empty())]
#[derive(Debug, Clone)]
pub struct TokenWindow {
    pub text_index: usize,
    pub token_ids: Vec<u32>,
}

impl TokenWindow {
    #[requires(!token_ids.is_empty())]
    #[ensures(ret.text_index == text_index)]
    pub fn new(text_index: usize, token_ids: Vec<u32>) -> Self {
        new!(TokenWindow {
            text_index: text_index,
            token_ids: token_ids,
        })
    }
}

#[invariant(!segments.is_empty())]
#[invariant(*total_tokens > 0)]
#[expensive_invariant(
    segments.iter().all(|segment| !segment.token_ids.is_empty())
        && *total_tokens
            == segments
                .iter()
                .map(|segment| segment.token_ids.len())
                .sum::<usize>()
)]
#[derive(Debug, Clone)]
pub struct PackedTokenBatch {
    pub segments: Vec<TokenWindow>,
    pub total_tokens: usize,
}

impl PackedTokenBatch {
    #[requires(true)]
    #[ensures(ret.segments.len() == 1)]
    fn from_window(window: TokenWindow) -> Self {
        let total_tokens = window.token_ids.len();
        new!(PackedTokenBatch {
            segments: vec![window],
            total_tokens: total_tokens,
        })
    }

    #[requires(true)]
    #[ensures(ret.segments.len() == old(self.segments.len()) + 1)]
    fn with_window(self, window: TokenWindow) -> Self {
        let mut data = self.into_data();
        data.total_tokens += window.token_ids.len();
        data.segments.push(window);
        Self::from_data(data)
    }
}

#[requires(budget > 0)]
#[requires(
    windows
        .iter()
        .all(|window| !window.token_ids.is_empty() && window.token_ids.len() <= budget)
)]
#[ensures(ret.iter().all(|batch| batch.total_tokens <= budget))]
pub fn pack_token_windows(windows: &[TokenWindow], budget: usize) -> Vec<PackedTokenBatch> {
    let mut sorted = windows.to_vec();
    sorted.sort_by(|left, right| {
        right
            .token_ids
            .len()
            .cmp(&left.token_ids.len())
            .then_with(|| left.text_index.cmp(&right.text_index))
    });
    let mut batches: Vec<PackedTokenBatch> = Vec::new();
    for window in sorted {
        let window_len = window.token_ids.len();
        let mut best_index = None;
        let mut best_remaining = usize::MAX;
        for (index, batch) in batches.iter().enumerate() {
            if let Some(combined) = batch.total_tokens.checked_add(window_len)
                && combined <= budget
            {
                let remaining = budget - combined;
                if remaining < best_remaining {
                    best_remaining = remaining;
                    best_index = Some(index);
                }
            }
        }
        if let Some(index) = best_index {
            let batch = batches.remove(index).with_window(window);
            batches.insert(index, batch);
        } else {
            batches.push(PackedTokenBatch::from_window(window));
        }
    }
    batches
}

#[requires(dimensions > 0)]
#[ensures(ret.as_ref().is_ok_and(|vector| {
    vector.len() == dimensions
        && vector.iter().all(|value| value.is_finite())
        && vector.iter().any(|value| *value != 0.0)
}) || ret.is_err())]
pub fn mean_pool_normalized(
    vectors: &[Vec<f32>],
    dimensions: usize,
) -> Result<Vec<f32>, EmbeddingVectorError> {
    if vectors.is_empty() {
        return Err(new!(EmbeddingVectorError::NoVectors {
            vector_count: vectors.len(),
        }));
    }
    let mut pooled = vec![0.0; dimensions];
    for vector in vectors {
        embedding_normalization_magnitude(vector, dimensions)?;
        for (index, value) in vector.iter().enumerate() {
            pooled[index] += *value;
        }
    }
    let divisor = vectors.len() as f32;
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
pub(crate) fn normalize_validated_in_place(vector: &mut [f32], magnitude: f32) {
    for value in &mut *vector {
        *value /= magnitude;
    }
}

#[requires(true)]
#[ensures(ret.len() == 256)]
fn bytes_to_unicode() -> [String; 256] {
    let mut bytes = Vec::new();
    for value in 33..=126 {
        bytes.push(value);
    }
    for value in 161..=172 {
        bytes.push(value);
    }
    for value in 174..=255 {
        bytes.push(value);
    }
    let mut chars = bytes.clone();
    let mut next = 0;
    for value in 0..=255 {
        if !bytes.contains(&value) {
            bytes.push(value);
            chars.push(256 + next);
            next += 1;
        }
    }
    let mut encoder: [String; 256] = std::array::from_fn(|_| String::new());
    for (index, byte) in bytes.into_iter().enumerate() {
        let ch = char::from_u32(chars[index] as u32).expect("byte-level token char is valid");
        encoder[byte] = ch.to_string();
    }
    encoder
}

#[requires(true)]
#[ensures(true)]
fn merge_pair(merge: MergeSpec) -> Option<(String, String)> {
    match merge {
        MergeSpec::Text(text) => {
            let mut parts = text.split(' ');
            let left = parts.next()?.to_owned();
            let right = parts.next()?.to_owned();
            Some((left, right))
        }
        MergeSpec::Pair(parts) => {
            if parts.len() < 2 {
                None
            } else {
                Some((parts[0].clone(), parts[1].clone()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn packs_token_windows_by_best_fit_decreasing() {
        assert_eq!(DEFAULT_MAX_SEQUENCE_LENGTH, 512);
        let windows = vec![
            TokenWindow::new(0, vec![1; 200]),
            TokenWindow::new(1, vec![2; 300]),
            TokenWindow::new(2, vec![3; 20]),
        ];
        let batches = pack_token_windows(&windows, 512);
        assert_eq!(batches.len(), 2);
        assert!(batches.iter().all(|batch| batch.total_tokens <= 512));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mean_pool_normalizes_after_averaging() {
        let pooled =
            mean_pool_normalized(&[vec![1.0, 0.0], vec![0.0, 1.0]], 2).expect("valid pooling");
        let expected = 1.0 / 2.0_f32.sqrt();
        assert!((pooled[0] - expected).abs() < 1e-6);
        assert!((pooled[1] - expected).abs() < 1e-6);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mean_pool_rejects_exact_window_cancellation() {
        let error = mean_pool_normalized(&[vec![1.0, 0.0], vec![-1.0, 0.0]], 2)
            .expect_err("cancelling windows must not produce a successful zero embedding");

        assert!(error.is_zero_norm());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mean_pool_rejects_empty_misdimensioned_and_non_finite_inputs() {
        assert!(
            mean_pool_normalized(&[], 2)
                .unwrap_err()
                .to_string()
                .contains("at least one")
        );
        assert!(
            mean_pool_normalized(&[vec![1.0]], 2)
                .unwrap_err()
                .to_string()
                .contains("1 dimensions, expected 2")
        );
        assert!(
            mean_pool_normalized(&[vec![1.0, f32::INFINITY]], 2)
                .unwrap_err()
                .to_string()
                .contains("non-finite value")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tokenizer_matches_byte_bpe_goldens() {
        let tokenizer = tiny_tokenizer();
        assert_eq!(tokenizer.encode_untruncated("hello").unwrap(), vec![8, 999]);
        assert_eq!(
            tokenizer.encode_untruncated("hello world").unwrap(),
            vec![8, 9, 16, 999]
        );
        assert_eq!(
            tokenizer.encode_untruncated("\u{00e9}").unwrap(),
            vec![19, 999]
        );
        assert_eq!(
            tokenizer.encode_untruncated("hello world!").unwrap(),
            vec![8, 9, 16, 20, 999]
        );
        assert_eq!(
            tokenizer.encode_untruncated("hello\n").unwrap(),
            vec![8, 22, 999]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tokenizer_windows_preserve_single_final_eos() {
        let tokenizer = tiny_tokenizer();
        let windows = tokenizer.token_windows("hello world!", 3).unwrap();
        assert_eq!(windows, vec![vec![8, 9, 16], vec![20, 999]]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn q4_padded_index_math_preserves_row_stride_for_non_divisible_shapes() {
        let rows: usize = 2;
        let in_cols: usize = 33;
        let group_size: usize = 32;
        let groups = in_cols.div_ceil(group_size);
        assert_eq!(q4_padded_row_stride(groups, group_size), 64);
        assert_eq!(q4_packed_byte_len(rows, groups, group_size), 64);
        assert_eq!(q4_padded_element_index(0, 32, groups, group_size), 32);
        assert_eq!(q4_padded_element_index(1, 0, groups, group_size), 64);
        assert_ne!(
            q4_padded_element_index(1, 0, groups, group_size),
            rows.saturating_sub(1) * in_cols
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn q4_storage_validation_uses_padded_rows_for_non_divisible_shapes() {
        assert!(
            validate_q4_tensor_storage(
                "test.weight",
                "q4_onnx_matmul",
                [2, 33],
                32,
                2,
                64,
                16,
                16,
            )
            .is_ok()
        );

        let error = validate_q4_tensor_storage(
            "test.weight",
            "q4_onnx_matmul",
            [2, 33],
            32,
            2,
            (2usize * 33).div_ceil(2),
            16,
            16,
        )
        .unwrap_err();
        assert!(error.contains("expected 64"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn q4_storage_validation_rejects_group_mismatch() {
        let error =
            validate_q4_tensor_storage("test.weight", "q4_onnx_matmul", [2, 33], 32, 1, 64, 16, 16)
                .unwrap_err();
        assert!(error.contains("groups mismatch"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn chunk_layout_validation_rejects_misaligned_offsets_and_bad_sha() {
        let good_sha = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let misaligned = [
            ("first.bin", 0usize, 4usize, good_sha),
            ("second.bin", 6usize, 4usize, good_sha),
        ];
        let error = validate_chunk_layout("vectors", 8, &misaligned).unwrap_err();
        assert!(error.contains("4-byte aligned"));

        let bad_sha = [("bad.bin", 0usize, 4usize, "not-a-sha")];
        let error = validate_chunk_layout("vectors", 4, &bad_sha).unwrap_err();
        assert!(error.contains("SHA-256"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn token_ids_are_checked_against_vocab_bounds() {
        assert!(validate_token_ids(&[0, 1, 2], 3).is_ok());
        let error = validate_token_ids(&[0, 3], 3).unwrap_err();
        assert!(error.contains("outside vocab size 3"));
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

    #[requires(true)]
    #[ensures(ret.eos_id() == 999)]
    fn tiny_tokenizer() -> QwenByteBpeTokenizer {
        let json = br#"{
  "schema_version": 1,
  "vocab": {
    "h": 1,
    "e": 2,
    "l": 3,
    "o": 4,
    "he": 5,
    "hel": 6,
    "hell": 7,
    "hello": 8,
    "\u0120": 9,
    "w": 10,
    "r": 11,
    "d": 12,
    "wo": 13,
    "wor": 14,
    "worl": 15,
    "world": 16,
    "\u00c3": 17,
    "\u00a9": 18,
    "\u00c3\u00a9": 19,
    "!": 20,
    ".": 21,
    "\u010a": 22
  },
  "merges": [
    "h e",
    "he l",
    "hel l",
    "hell o",
    "w o",
    "wo r",
    "wor l",
    "worl d",
    "\u00c3 \u00a9"
  ],
  "special_tokens": {
    "eos_id": 999
  }
}"#;
        QwenByteBpeTokenizer::from_compact_json(json).unwrap()
    }
}
