//! Lujvo composition and decomposition.

use bityzba::{invariant, requires};
use jbotci_dictionary::Dictionary;
use jbotci_morphology::{
    Jvopau, Phonemes, WordLike, canonicalize_text, segment_words_with_modifiers,
};
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

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LujvoDecomposition<'a> {
    pub segments: Vec<LujvoSegmentInfo<'a>>,
    pub source_words: Vec<&'a str>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LujvoSegmentInfo<'a> {
    pub segment: Jvopau,
    pub source: Option<&'a str>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::NotImplemented => true)]
pub enum JvozbaError {
    #[error("jvozba is not implemented yet")]
    NotImplemented,
}

#[requires(true)]
#[ensures(true)]
pub fn compose_lujvo(_sources: &[LujvoSource]) -> Result<LujvoPlan, JvozbaError> {
    Err(JvozbaError::NotImplemented)
}

#[requires(true)]
#[ensures(true)]
pub fn decompose_lujvo_like<'a>(
    dictionary: &Dictionary<'a>,
    raw_word: &str,
) -> Option<LujvoDecomposition<'a>> {
    let normalized = normalize_lujvo_like_input(raw_word);
    if normalized.is_empty() {
        return None;
    }

    let parts =
        morphology_lujvo_parts(&normalized).or_else(|| fallback_lujvo_parts(&normalized))?;
    let segments = parts
        .into_iter()
        .map(|segment| segment_with_source(dictionary, segment))
        .collect::<Vec<_>>();
    let source_words = segments
        .iter()
        .filter_map(|segment| match &segment.segment {
            Jvopau::Rafsi(_) => segment.source,
            Jvopau::Hyphen(_) => None,
        })
        .collect::<Vec<_>>();
    let rafsi_count = segments
        .iter()
        .filter(|segment| matches!(segment.segment, Jvopau::Rafsi(_)))
        .count();

    if rafsi_count >= 2 && source_words.len() == rafsi_count {
        Some(LujvoDecomposition {
            segments,
            source_words,
        })
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_lujvo_like_input(raw_word: &str) -> String {
    let apostrophe_normalized = raw_word
        .trim()
        .trim_matches('.')
        .chars()
        .map(normalize_apostrophe)
        .collect::<String>();
    canonicalize_text(&apostrophe_normalized)
}

#[requires(true)]
#[ensures(true)]
fn normalize_apostrophe(value: char) -> char {
    match value {
        '\'' | 'h' | 'H' | '’' | '\u{a78b}' | '\u{a78c}' | '\u{2bb}' | '\u{2bf}' | '\u{2b0}'
        | '\u{2d2}' => '\'',
        other => other,
    }
}

#[requires(true)]
#[ensures(true)]
fn morphology_lujvo_parts(normalized: &str) -> Option<Vec<Jvopau>> {
    let words = segment_words_with_modifiers(normalized).ok()?;
    let [word_like] = words.as_slice() else {
        return None;
    };
    let word = word_like.bare_word()?;
    let parts = word.lujvo_parts()?;
    Some(parts.iter().cloned().collect())
}

#[requires(true)]
#[ensures(true)]
fn segment_with_source<'a>(dictionary: &Dictionary<'a>, segment: Jvopau) -> LujvoSegmentInfo<'a> {
    let source = match &segment {
        Jvopau::Rafsi(phonemes) => dictionary
            .lookup_rafsi(phonemes.as_str())
            .next()
            .map(|matched| matched.entry.word),
        Jvopau::Hyphen(_) => None,
    };
    LujvoSegmentInfo { segment, source }
}

#[requires(true)]
#[ensures(true)]
fn fallback_lujvo_parts(normalized: &str) -> Option<Vec<Jvopau>> {
    let parts = sloppy_decompose(normalized)?;
    let rafsi_parts = parts
        .iter()
        .filter_map(|part| match part {
            RawLujvoSegment::Rafsi(text) => Some(text.clone()),
            RawLujvoSegment::Hyphen(_) => None,
        })
        .collect::<Vec<_>>();
    let bonded = bond_rafsis(&rafsi_parts)?;
    if bonded.concat() == normalized {
        Some(
            parts
                .into_iter()
                .filter_map(|part| match part {
                    RawLujvoSegment::Rafsi(text) => Some(Jvopau::rafsi(phonemes(text)?)),
                    RawLujvoSegment::Hyphen(text) => Some(Jvopau::hyphen(phonemes(text)?)),
                })
                .collect(),
        )
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Rafsi(_) => true)]
#[invariant(::Hyphen(_) => true)]
enum RawLujvoSegment {
    Rafsi(String),
    Hyphen(String),
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn phonemes(text: String) -> Option<Phonemes> {
    Phonemes::from_canonical(text).ok()
}

#[requires(true)]
#[ensures(true)]
fn sloppy_decompose(normalized: &str) -> Option<Vec<RawLujvoSegment>> {
    sloppy_decompose_from(Vec::new(), normalized)
}

#[requires(true)]
#[ensures(true)]
fn sloppy_decompose_from(
    mut acc: Vec<RawLujvoSegment>,
    remaining: &str,
) -> Option<Vec<RawLujvoSegment>> {
    if remaining.is_empty() {
        acc.reverse();
        return Some(acc);
    }

    if should_drop_hyphen(&acc, remaining) {
        let (hyphen, rest) = split_char_at(remaining, 1)?;
        acc.push(RawLujvoSegment::Hyphen(hyphen.to_owned()));
        return sloppy_decompose_from(acc, rest);
    }

    if has_head_syllable(remaining, "CVV") && has_vowel_pair_after_initial(remaining) {
        let (rafsi, rest) = split_char_at(remaining, 3)?;
        acc.push(RawLujvoSegment::Rafsi(rafsi.to_owned()));
        return sloppy_decompose_from(acc, rest);
    }

    if split_char_at(remaining, 4)
        .and_then(|(prefix, _)| syllables_pattern(prefix))
        .as_deref()
        == Some("CV'V")
    {
        let (rafsi, rest) = split_char_at(remaining, 4)?;
        acc.push(RawLujvoSegment::Rafsi(rafsi.to_owned()));
        return sloppy_decompose_from(acc, rest);
    }

    if has_head_syllable(remaining, "CVCCY") || has_head_syllable(remaining, "CCVCY") {
        let (rafsi, rest_with_hyphen) = split_char_at(remaining, 4)?;
        let (_, rest) = split_char_at(rest_with_hyphen, 1)?;
        acc.push(RawLujvoSegment::Rafsi(rafsi.to_owned()));
        acc.push(RawLujvoSegment::Hyphen("y".to_owned()));
        return sloppy_decompose_from(acc, rest);
    }

    if matches!(
        syllables_pattern(remaining).as_deref(),
        Some("CVCCV" | "CCVCV")
    ) {
        acc.push(RawLujvoSegment::Rafsi(remaining.to_owned()));
        acc.reverse();
        return Some(acc);
    }

    if has_head_syllable(remaining, "CVC") || has_head_syllable(remaining, "CCV") {
        let (rafsi, rest) = split_char_at(remaining, 3)?;
        acc.push(RawLujvoSegment::Rafsi(rafsi.to_owned()));
        return sloppy_decompose_from(acc, rest);
    }

    None
}

#[requires(true)]
#[ensures(true)]
fn split_char_at(text: &str, count: usize) -> Option<(&str, &str)> {
    let byte_index = text
        .char_indices()
        .nth(count)
        .map(|(index, _)| index)
        .unwrap_or(text.len());
    if text.chars().count() < count {
        None
    } else {
        Some(text.split_at(byte_index))
    }
}

#[requires(true)]
#[ensures(true)]
fn should_drop_hyphen(acc: &[RawLujvoSegment], remaining: &str) -> bool {
    previous_is_rafsi(acc)
        && (remaining.starts_with('y')
            || remaining.starts_with("nr")
            || (remaining.starts_with('r') && has_head_syllable(remaining, "C")))
}

#[requires(true)]
#[ensures(true)]
fn previous_is_rafsi(acc: &[RawLujvoSegment]) -> bool {
    matches!(acc.last(), Some(RawLujvoSegment::Rafsi(_)))
}

#[requires(true)]
#[ensures(true)]
fn has_head_syllable(text: &str, pattern: &str) -> bool {
    split_char_at(text, pattern.chars().count())
        .and_then(|(prefix, _)| syllables_pattern(prefix))
        .is_some_and(|actual| actual == pattern)
}

#[requires(true)]
#[ensures(true)]
fn has_vowel_pair_after_initial(text: &str) -> bool {
    split_char_at(text, 3)
        .map(|(prefix, _)| {
            prefix
                .chars()
                .skip(1)
                .collect::<String>()
                .as_str()
                .to_owned()
        })
        .is_some_and(|pair| matches!(pair.as_str(), "ai" | "ei" | "oi" | "au"))
}

#[requires(true)]
#[ensures(true)]
fn bond_rafsis(rafsis: &[String]) -> Option<Vec<String>> {
    if rafsis.len() < 2 {
        return None;
    }
    let first = rafsis.first()?.clone();
    let second = rafsis.get(1)?;
    let mut bonded = vec![first.clone()];
    if should_insert_cvv_hyphen(&first, second, rafsis.len()) {
        bonded.push(if second.starts_with('r') {
            "n".to_owned()
        } else {
            "r".to_owned()
        });
    }
    for pair in rafsis.windows(2) {
        let previous = &pair[0];
        let next = &pair[1];
        if needs_y_hyphen(previous, next) {
            bonded.push("y".to_owned());
        }
        bonded.push(next.clone());
    }
    if tosmabru(&bonded) {
        bonded.insert(1, "y".to_owned());
    }
    Some(bonded)
}

#[requires(true)]
#[ensures(true)]
fn needs_y_hyphen(previous: &str, next: &str) -> bool {
    let previous_pattern = syllables_pattern(previous);
    let previous_tail = previous.chars().last();
    let next_head = next.chars().next();
    matches!(previous_pattern.as_deref(), Some("CVCC" | "CCVC"))
        || matches!(
            (previous_tail, next_head),
            (Some(left), Some(right))
                if is_consonant(left)
                    && is_consonant(right)
                    && permissible_consonant_pair(left, right) == Some(0)
        )
        || (previous_tail == Some('n')
            && (next.starts_with("ts")
                || next.starts_with("tc")
                || next.starts_with("dz")
                || next.starts_with("dj")))
}

#[requires(true)]
#[ensures(true)]
fn should_insert_cvv_hyphen(first_rafsi: &str, second: &str, rafsi_count: usize) -> bool {
    matches!(
        syllables_pattern(first_rafsi).as_deref(),
        Some("CVV" | "CV'V")
    ) && (rafsi_count > 2 || syllables_pattern(second).as_deref() != Some("CCV"))
}

#[requires(true)]
#[ensures(true)]
fn tosmabru(parts: &[String]) -> bool {
    let Some(last_part) = parts.last() else {
        return false;
    };
    if is_cmevla(last_part) {
        return false;
    }
    if let Some(y_index) = parts.iter().position(|part| part == "y") {
        let heads = &parts[..y_index];
        return heads.len() > 1
            && heads
                .iter()
                .all(|part| syllables_pattern(part).as_deref() == Some("CVC"))
            && heads
                .windows(2)
                .all(|pair| consonant_pair_is_rank_two(&pair[0], &pair[1]));
    }
    if syllables_pattern(last_part).as_deref() == Some("CVCCV") {
        let chars = last_part.chars().collect::<Vec<_>>();
        if chars.len() >= 4
            && is_consonant(chars[2])
            && is_consonant(chars[3])
            && permissible_consonant_pair(chars[2], chars[3]) == Some(2)
        {
            let heads = &parts[..parts.len().saturating_sub(1)];
            return !heads.is_empty()
                && heads
                    .iter()
                    .all(|part| syllables_pattern(part).as_deref() == Some("CVC"))
                && parts
                    .windows(2)
                    .all(|pair| consonant_pair_is_rank_two(&pair[0], &pair[1]));
        }
    }
    false
}

#[requires(true)]
#[ensures(true)]
fn consonant_pair_is_rank_two(left: &str, right: &str) -> bool {
    matches!(
        (left.chars().last(), right.chars().next()),
        (Some(left_tail), Some(right_head))
            if is_consonant(left_tail)
                && is_consonant(right_head)
                && permissible_consonant_pair(left_tail, right_head) == Some(2)
    )
}

#[requires(true)]
#[ensures(true)]
fn syllables_pattern(text: &str) -> Option<String> {
    text.chars().map(classify_syllable_char).collect()
}

#[requires(true)]
#[ensures(true)]
fn classify_syllable_char(value: char) -> Option<char> {
    if is_vowel(value) {
        Some('V')
    } else if is_consonant(value) {
        Some('C')
    } else if value == '\'' {
        Some('\'')
    } else if value == 'y' {
        Some('Y')
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn is_vowel(value: char) -> bool {
    matches!(value, 'a' | 'e' | 'i' | 'o' | 'u')
}

#[requires(true)]
#[ensures(true)]
fn is_consonant(value: char) -> bool {
    matches!(
        value,
        'b' | 'c'
            | 'd'
            | 'f'
            | 'g'
            | 'j'
            | 'k'
            | 'l'
            | 'm'
            | 'n'
            | 'p'
            | 'r'
            | 's'
            | 't'
            | 'v'
            | 'x'
            | 'z'
    )
}

#[requires(true)]
#[ensures(true)]
fn is_cmevla(text: &str) -> bool {
    text.chars()
        .last()
        .is_some_and(|value| !matches!(value, 'a' | 'e' | 'i' | 'o' | 'u' | 'y' | '\''))
}

#[requires(true)]
#[ensures(true)]
fn permissible_consonant_pair(first: char, second: char) -> Option<i32> {
    let consonant_order = "rlnmbvdgjzscxktfp";
    let first_index = consonant_order.chars().position(|value| value == first)?;
    let second_index = consonant_order.chars().position(|value| value == second)?;
    PAIR_MATRIX
        .get(first_index)
        .and_then(|row| row.get(second_index))
        .copied()
}

const PAIR_MATRIX: [[i32; 17]; 17] = [
    [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [2, 2, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1],
    [2, 2, 1, 1, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
    [2, 2, 1, 1, 1, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
    [2, 1, 1, 1, 1, 1, 0, 1, 2, 2, 0, 0, 0, 0, 0, 0, 0],
    [2, 2, 1, 1, 1, 1, 1, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0],
    [1, 1, 1, 2, 2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [1, 1, 1, 2, 2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    [2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 2, 2, 2],
    [2, 2, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 2],
    [2, 2, 1, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1],
    [2, 2, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1],
    [2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 2, 2, 1, 1, 0, 1, 1],
    [2, 2, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 1],
    [2, 2, 1, 1, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0],
];

#[requires(true)]
#[ensures(true)]
pub fn word_like_type_key(word_like: &WordLike) -> Option<&'static str> {
    let word = word_like.bare_word()?;
    Some(match word.kind() {
        jbotci_morphology::WordKind::Cmavo => "cmavo",
        jbotci_morphology::WordKind::Gismu => "gismu",
        jbotci_morphology::WordKind::Lujvo => "lujvo",
        jbotci_morphology::WordKind::Fuhivla => "fu'ivla",
        jbotci_morphology::WordKind::Cmevla => "cmevla",
    })
}
