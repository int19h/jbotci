use std::cell::RefCell;
use std::collections::HashMap;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
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

#[invariant(true)]
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
        let mut merge_ranks = HashMap::with_capacity(artifact.merges.len());
        for (rank, merge) in artifact.merges.into_iter().enumerate() {
            if let Some((left, right)) = merge_pair(merge) {
                merge_ranks.insert((left, right), rank);
            }
        }
        Ok(Self {
            vocab: artifact.vocab,
            eos_id: artifact.special_tokens.eos_id,
            byte_encoder: bytes_to_unicode(),
            merge_ranks,
            cache: RefCell::new(HashMap::new()),
            pattern: Regex::new(TOKEN_PATTERN)
                .map_err(|error| format!("failed to compile F2LLM tokenizer regex: {error}"))?,
        })
    }

    #[requires(true)]
    #[ensures(ret == self.eos_id)]
    pub fn eos_id(&self) -> u32 {
        self.eos_id
    }

    #[requires(max_length > 0)]
    #[ensures(ret.as_ref().is_ok_and(|ids| ids.len() <= max_length) || ret.is_err())]
    pub fn encode_truncated(&self, text: &str, max_length: usize) -> Result<Vec<u32>, String> {
        let mut ids = self.encode_untruncated(text)?;
        if ids.len() > max_length {
            ids.truncate(max_length);
            if let Some(last) = ids.last_mut() {
                *last = self.eos_id;
            }
        }
        Ok(ids)
    }

    #[requires(true)]
    #[ensures(!ret.as_ref().is_ok_and(|ids| ids.is_empty()))]
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
    #[ensures(ret.as_ref().is_ok() || ret.is_err())]
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

#[invariant(true)]
#[derive(Debug, Clone)]
pub struct TokenWindow {
    pub text_index: usize,
    pub token_ids: Vec<u32>,
}

#[invariant(true)]
#[derive(Debug, Clone)]
pub struct PackedTokenBatch {
    pub segments: Vec<TokenWindow>,
    pub total_tokens: usize,
}

#[requires(budget > 0)]
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
            if batch.total_tokens + window_len <= budget {
                let remaining = budget - (batch.total_tokens + window_len);
                if remaining < best_remaining {
                    best_remaining = remaining;
                    best_index = Some(index);
                }
            }
        }
        if let Some(index) = best_index {
            batches[index].total_tokens += window_len;
            batches[index].segments.push(window);
        } else {
            batches.push(PackedTokenBatch {
                total_tokens: window_len,
                segments: vec![window],
            });
        }
    }
    batches
}

#[requires(!vectors.is_empty())]
#[requires(vectors.iter().all(|vector| vector.len() == dimensions))]
#[ensures(ret.len() == dimensions)]
pub fn mean_pool_normalized(vectors: &[Vec<f32>], dimensions: usize) -> Vec<f32> {
    let mut pooled = vec![0.0; dimensions];
    for vector in vectors {
        for (index, value) in vector.iter().enumerate() {
            pooled[index] += *value;
        }
    }
    let divisor = vectors.len() as f32;
    for value in &mut pooled {
        *value /= divisor;
    }
    normalize_in_place(&mut pooled);
    pooled
}

#[requires(true)]
#[ensures(true)]
pub fn normalize_in_place(vector: &mut [f32]) {
    let sum = vector.iter().map(|value| value * value).sum::<f32>();
    let magnitude = sum.sqrt();
    if magnitude == 0.0 {
        return;
    }
    for value in vector {
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
            TokenWindow {
                text_index: 0,
                token_ids: vec![1; 200],
            },
            TokenWindow {
                text_index: 1,
                token_ids: vec![2; 300],
            },
            TokenWindow {
                text_index: 2,
                token_ids: vec![3; 20],
            },
        ];
        let batches = pack_token_windows(&windows, 512);
        assert_eq!(batches.len(), 2);
        assert!(batches.iter().all(|batch| batch.total_tokens <= 512));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mean_pool_normalizes_after_averaging() {
        let pooled = mean_pool_normalized(&[vec![1.0, 0.0], vec![0.0, 1.0]], 2);
        let expected = 1.0 / 2.0_f32.sqrt();
        assert!((pooled[0] - expected).abs() < 1e-6);
        assert!((pooled[1] - expected).abs() < 1e-6);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tokenizer_matches_byte_bpe_goldens() {
        let tokenizer = tiny_tokenizer();
        assert_eq!(
            tokenizer.encode_truncated("hello", 8).unwrap(),
            vec![8, 999]
        );
        assert_eq!(
            tokenizer.encode_truncated("hello world", 8).unwrap(),
            vec![8, 9, 16, 999]
        );
        assert_eq!(
            tokenizer.encode_truncated("\u{00e9}", 8).unwrap(),
            vec![19, 999]
        );
        assert_eq!(
            tokenizer.encode_truncated("hello world!", 3).unwrap(),
            vec![8, 9, 999]
        );
        assert_eq!(
            tokenizer.encode_truncated("hello\n", 8).unwrap(),
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
