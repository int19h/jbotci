//! IPA rendering and ALINE-style sound similarity.

use std::cmp::Ordering;
use std::sync::LazyLock;

#[allow(unused_imports)]
use bityzba::expensive_invariant;
use bityzba::{data, invariant, new, requires};
use jbotci_morphology::{
    LeadingPauseVowelMode, Phonemes, Word, WordKind, WordLike, WordLikeData,
    pronunciation_syllables, segment_words_with_modifiers, word_needs_leading_pause,
};
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Morphology { .. } => true)]
#[invariant(::NoPronounceableWords { .. } => true)]
#[invariant(::UnsupportedSegment { .. } => true)]
#[invariant(::EmptyQuery => true)]
#[invariant(::EmptyBracketedIpa => true)]
#[invariant(::NestedBrackets => true)]
#[invariant(::MissingClosingBracket => true)]
#[invariant(::MissingOpeningBracket => true)]
#[invariant(::PartialBracketedQuery => true)]
#[invariant(::Syllabification { .. } => true)]
pub enum PhoneticError {
    #[error("{message}")]
    Morphology { message: String },
    #[error("no pronounceable words in `{input}`")]
    NoPronounceableWords { input: String },
    #[error("Unsupported IPA segment near `{near}` for ALINE sound search.")]
    UnsupportedSegment { near: String },
    #[error("Sound search requires at least one IPA segment.")]
    EmptyQuery,
    #[error("Bracketed IPA input must not be empty.")]
    EmptyBracketedIpa,
    #[error("IPA input must use one pair of brackets around the whole query.")]
    NestedBrackets,
    #[error("IPA input starts with `[` but does not end with `]`.")]
    MissingClosingBracket,
    #[error("IPA input ends with `]` but does not start with `[`.")]
    MissingOpeningBracket,
    #[error("Use `[ ... ]` around the whole IPA query.")]
    PartialBracketedQuery,
    #[error("{message}")]
    Syllabification { message: String },
}

#[invariant((self.as_data().0 as usize) < IPA_SEGMENT_SYMBOLS.len())]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IpaSegmentId(u16);

impl IpaSegmentId {
    #[requires(true)]
    #[ensures(true)]
    pub const fn from_static_index(index: u16) -> Self {
        assert!(
            (index as usize) < IPA_SEGMENT_SYMBOLS.len(),
            "static IPA segment id must index IPA_SEGMENT_SYMBOLS"
        );
        Self(data!(IpaSegmentId(index)))
    }

    #[requires(true)]
    #[ensures((ret as usize) < IPA_SEGMENT_SYMBOLS.len())]
    pub fn get(self) -> u16 {
        self.as_data().0
    }
}

#[invariant(!segments.is_empty())]
#[invariant(self_similarity.is_finite())]
#[invariant(*self_similarity > 0.0)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IpaTokenSequenceView<'a> {
    pub segments: &'a [IpaSegmentId],
    pub self_similarity: f64,
}

impl<'a> IpaTokenSequenceView<'a> {
    #[requires(!segments.is_empty())]
    #[requires(self_similarity.is_finite())]
    #[requires(self_similarity > 0.0)]
    #[ensures(ret.segment_count() == segments.len())]
    pub fn new(segments: &'a [IpaSegmentId], self_similarity: f64) -> Self {
        new!(IpaTokenSequenceView {
            segments,
            self_similarity,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub const fn from_static_parts(segments: &'a [IpaSegmentId], self_similarity: f64) -> Self {
        assert!(
            !segments.is_empty(),
            "static IPA token sequence must contain at least one segment"
        );
        assert!(
            self_similarity.is_finite(),
            "static IPA token sequence self-similarity must be finite"
        );
        assert!(
            self_similarity > 0.0,
            "static IPA token sequence self-similarity must be positive"
        );
        Self(data!(IpaTokenSequenceView {
            segments,
            self_similarity,
        }))
    }

    #[requires(true)]
    #[ensures(ret == self.segments.len())]
    pub fn segment_count(self) -> usize {
        self.segments.len()
    }
}

#[invariant(!segments.is_empty())]
#[invariant(self_similarity.is_finite())]
#[invariant(*self_similarity > 0.0)]
#[expensive_invariant(*self_similarity == aline_raw_similarity(segments, segments))]
#[derive(Debug, Clone, PartialEq)]
pub struct IpaTokenSequence {
    segments: Vec<IpaSegmentId>,
    self_similarity: f64,
}

impl IpaTokenSequence {
    #[requires(true)]
    #[ensures(ret == self.segments.len())]
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    #[requires(true)]
    #[ensures(ret.len() == self.segments.len())]
    pub fn segments(&self) -> &[IpaSegmentId] {
        &self.segments
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn self_similarity(&self) -> f64 {
        self.self_similarity
    }

    #[requires(true)]
    #[ensures(ret.segment_count() == self.segment_count())]
    pub fn view(&self) -> IpaTokenSequenceView<'_> {
        IpaTokenSequenceView::new(&self.segments, self.self_similarity)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct IpaTokenizedText {
    pub ipa: String,
    pub token_sequence: IpaTokenSequence,
}

#[derive(Debug, Default, Clone, PartialEq)]
#[invariant(true)]
pub struct AlineSimilarityScratch {
    previous_previous: Vec<f64>,
    previous: Vec<f64>,
    current: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[invariant(true)]
struct AlineFeatures {
    is_consonant: bool,
    syllabic_value: f64,
    place_value: f64,
    manner_value: f64,
    voice_value: f64,
    nasal_value: f64,
    retroflex_value: f64,
    lateral_value: f64,
    aspirated_value: f64,
    high_value: f64,
    back_value: f64,
    round_value: f64,
    long_value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Syllabic => true)]
#[invariant(::Place => true)]
#[invariant(::Manner => true)]
#[invariant(::Voice => true)]
#[invariant(::Nasal => true)]
#[invariant(::Retroflex => true)]
#[invariant(::Lateral => true)]
#[invariant(::Aspirated => true)]
#[invariant(::High => true)]
#[invariant(::Back => true)]
#[invariant(::Round => true)]
#[invariant(::Long => true)]
enum AlineFeature {
    Syllabic,
    Place,
    Manner,
    Voice,
    Nasal,
    Retroflex,
    Lateral,
    Aspirated,
    High,
    Back,
    Round,
    Long,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Word(_) => true)]
#[invariant(::Text(_) => true)]
enum IpaSurfaceChunk<'word> {
    Word(&'word Word),
    Text(&'word str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct IpaRenderedWord {
    body: String,
    leading_pause_required: bool,
    trailing_pause_required: bool,
}

const ALINE_SKIP_SCORE: f64 = -10.0;
const ALINE_SUBSTITUTION_CEILING: f64 = 35.0;
const ALINE_EXPANSION_CEILING: f64 = 45.0;
const ALINE_VOWEL_PENALTY: f64 = 10.0;

const BILABIAL_SYMBOLS: &[&str] = &["p", "b", "m", "ʙ", "β", "ɸ", "B"];
const LABIODENTAL_SYMBOLS: &[&str] = &["f", "v", "ʋ", "ɱ"];
const DENTAL_SYMBOLS: &[&str] = &["θ", "ð"];
const ALVEOLAR_SYMBOLS: &[&str] = &["t", "d", "n", "s", "z", "r", "l", "ɹ", "ɾ", "ɬ", "ɮ"];
const RETROFLEX_SYMBOLS: &[&str] = &["ʈ", "ɖ", "ɳ", "ʂ", "ʐ", "ɻ", "ɽ"];
const PALATO_ALVEOLAR_SYMBOLS: &[&str] = &["ʃ", "ʒ"];
const PALATAL_SYMBOLS: &[&str] = &["j", "c", "ɟ", "ɲ", "ç", "ʝ"];
const VELAR_SYMBOLS: &[&str] = &["k", "g", "x", "ɣ", "ŋ", "w", "ɰ"];
const UVULAR_SYMBOLS: &[&str] = &["q", "ɢ", "χ", "ʁ", "ʀ", "ɴ", "N", "R"];
const PHARYNGEAL_SYMBOLS: &[&str] = &["ħ", "ʕ"];
const GLOTTAL_SYMBOLS: &[&str] = &["h", "ɦ", "ʔ"];
const AFFRICATE_SYMBOLS: &[&str] = &["t͡ʃ", "d͡ʒ", "tʃ", "dʒ", "ts", "dz"];
const TRILL_SYMBOLS: &[&str] = &["r", "ʀ", "ʙ", "R", "B"];
const TAP_SYMBOLS: &[&str] = &["ɾ", "ɽ"];
const APPROXIMANT_SYMBOLS: &[&str] = &["j", "w", "ʋ", "ɹ", "ɻ", "ɰ"];
const FRICATIVE_SYMBOLS: &[&str] = &[
    "ɸ", "β", "f", "v", "θ", "ð", "s", "z", "ʃ", "ʒ", "ʂ", "ʐ", "ç", "ʝ", "x", "ɣ", "χ", "ʁ", "ħ",
    "ʕ", "h", "ɦ", "ɬ", "ɮ",
];
const VOICED_CONSONANT_SYMBOLS: &[&str] = &[
    "b", "d", "ɖ", "ɟ", "g", "ɢ", "m", "ɱ", "n", "ɳ", "ɲ", "ŋ", "ɴ", "N", "ʙ", "B", "r", "ʀ", "R",
    "ɾ", "ɽ", "β", "v", "ð", "z", "ʒ", "ʐ", "ʝ", "ɣ", "ʁ", "ʕ", "ɦ", "ɮ", "ʋ", "ɹ", "ɻ", "ɰ", "j",
    "w", "l", "d͡ʒ", "dʒ", "dz",
];
const NASAL_SYMBOLS: &[&str] = &["m", "ɱ", "n", "ɳ", "ɲ", "ŋ", "ɴ", "N"];
const LATERAL_SYMBOLS: &[&str] = &["l", "ɬ", "ɮ"];
const HIGH_VOWEL_SYMBOLS: &[&str] = &["i", "y", "ɨ", "ʉ", "ɯ", "u", "I", "U"];
const MID_VOWEL_SYMBOLS: &[&str] = &[
    "e", "ø", "ɘ", "ɵ", "ɤ", "o", "ə", "ɛ", "œ", "ɜ", "ɞ", "ʌ", "ɔ", "E", "O",
];
const FRONT_VOWEL_SYMBOLS: &[&str] = &["i", "y", "e", "ø", "ɛ", "œ", "æ", "a", "ɶ", "I", "E"];
const CENTRAL_VOWEL_SYMBOLS: &[&str] = &["ɨ", "ʉ", "ɘ", "ɵ", "ə", "ɜ", "ɞ", "ɐ", "ä"];
const ROUNDED_VOWEL_SYMBOLS: &[&str] = &[
    "y", "ʉ", "u", "ø", "ɵ", "o", "œ", "ɞ", "ɔ", "ɶ", "ɒ", "U", "O",
];

const IPA_SEGMENT_SYMBOLS: &[&str] = &[
    "p", "b", "m", "ʙ", "β", "ɸ", "B", "f", "v", "ʋ", "ɱ", "θ", "ð", "t", "d", "n", "s", "z", "r",
    "l", "ɹ", "ɾ", "ɬ", "ɮ", "ʈ", "ɖ", "ɳ", "ʂ", "ʐ", "ɻ", "ɽ", "ʃ", "ʒ", "j", "c", "ɟ", "ɲ", "ç",
    "ʝ", "k", "g", "x", "ɣ", "ŋ", "w", "ɰ", "q", "ɢ", "χ", "ʁ", "ʀ", "ɴ", "N", "R", "ħ", "ʕ", "h",
    "ɦ", "ʔ", "t͡ʃ", "d͡ʒ", "tʃ", "dʒ", "ts", "dz", "i", "y", "ɨ", "ʉ", "ɯ", "u", "I", "U", "e", "ø",
    "ɘ", "ɵ", "ɤ", "o", "ə", "ɛ", "œ", "ɜ", "ɞ", "ʌ", "ɔ", "E", "O", "æ", "ɐ", "a", "ɶ", "ä", "ɑ",
    "ɒ", "iː", "yː", "ɨː", "ʉː", "ɯː", "uː", "Iː", "Uː", "eː", "øː", "ɘː", "ɵː", "ɤː", "oː", "əː",
    "ɛː", "œː", "ɜː", "ɞː", "ʌː", "ɔː", "Eː", "Oː", "æː", "ɐː", "aː", "ɶː", "äː", "ɑː", "ɒː",
];

const CONSONANT_RELEVANT_FEATURES: &[AlineFeature] = &[
    AlineFeature::Syllabic,
    AlineFeature::Manner,
    AlineFeature::Voice,
    AlineFeature::Nasal,
    AlineFeature::Retroflex,
    AlineFeature::Lateral,
    AlineFeature::Aspirated,
    AlineFeature::Place,
];

const VOWEL_RELEVANT_FEATURES: &[AlineFeature] = &[
    AlineFeature::Syllabic,
    AlineFeature::Nasal,
    AlineFeature::Retroflex,
    AlineFeature::High,
    AlineFeature::Back,
    AlineFeature::Round,
    AlineFeature::Long,
];

static IPA_SEGMENT_FEATURES: LazyLock<Vec<AlineFeatures>> = LazyLock::new(|| {
    IPA_SEGMENT_SYMBOLS
        .iter()
        .map(|symbol| derive_aline_features(symbol))
        .collect()
});

static IPA_SEGMENT_NORMALIZED_SYMBOLS: LazyLock<Vec<String>> = LazyLock::new(|| {
    IPA_SEGMENT_SYMBOLS
        .iter()
        .map(|symbol| normalize_ipa_query(symbol))
        .collect()
});

static ALINE_PAIR_FEATURE_DIFFERENCES: LazyLock<Vec<f64>> = LazyLock::new(|| {
    let segment_count = IPA_SEGMENT_SYMBOLS.len();
    let mut differences = Vec::with_capacity(segment_count * segment_count);
    for left in IPA_SEGMENT_FEATURES.iter().copied() {
        for right in IPA_SEGMENT_FEATURES.iter().copied() {
            differences.push(feature_difference_from_features(left, right));
        }
    }
    differences
});

static ALINE_VOWEL_PENALTIES: LazyLock<Vec<f64>> = LazyLock::new(|| {
    IPA_SEGMENT_FEATURES
        .iter()
        .map(|features| {
            if features.is_consonant {
                0.0
            } else {
                ALINE_VOWEL_PENALTY
            }
        })
        .collect()
});

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|sequence| sequence.segment_count() > 0) || ret.is_err())]
pub fn sound_query_to_token_sequence(raw_query: &str) -> Result<IpaTokenSequence, PhoneticError> {
    let ipa = sound_query_to_ipa(raw_query)?;
    tokenize_ipa_text(&ipa)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.trim().is_empty()) || ret.is_err())]
pub fn sound_query_to_ipa(raw_query: &str) -> Result<String, PhoneticError> {
    match bracketed_ipa_query(raw_query)? {
        Some(ipa) => Ok(ipa),
        None => lojban_text_to_ipa(raw_query),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.trim().is_empty()) || ret.is_err())]
pub fn lojban_text_to_ipa(raw_text: &str) -> Result<String, PhoneticError> {
    let words =
        segment_words_with_modifiers(raw_text).map_err(|error| PhoneticError::Morphology {
            message: error.to_string(),
        })?;
    ipa_morphology_text(&words, raw_text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.ipa.trim().is_empty()) || ret.is_err())]
pub fn lojban_text_to_tokenized_ipa(raw_text: &str) -> Result<IpaTokenizedText, PhoneticError> {
    let ipa = lojban_text_to_ipa(raw_text)?;
    let token_sequence = tokenize_ipa_text(&ipa)?;
    Ok(IpaTokenizedText {
        ipa,
        token_sequence,
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn ipa_morphology_text(words: &[WordLike], source: &str) -> Result<String, PhoneticError> {
    let chunks = words
        .iter()
        .flat_map(flatten_word_like_ipa)
        .collect::<Vec<_>>();
    if chunks.is_empty() {
        return Err(PhoneticError::NoPronounceableWords {
            input: source.to_owned(),
        });
    }
    render_ipa_surface_chunks(&chunks, source)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|sequence| sequence.segment_count() > 0) || ret.is_err())]
pub fn tokenize_ipa_text(text: &str) -> Result<IpaTokenSequence, PhoneticError> {
    let mut segments = Vec::new();
    let normalized = normalize_ipa_query(text);
    let mut remaining = normalized.trim();
    while !remaining.is_empty() {
        let Some(first) = remaining.chars().next() else {
            break;
        };
        if is_ipa_boundary(first) {
            remaining = &remaining[first.len_utf8()..];
            continue;
        }
        if is_ipa_ignored_modifier(first) {
            remaining = &remaining[first.len_utf8()..];
            continue;
        }
        let Some((segment_id, segment_length)) = match_longest_segment(remaining) else {
            return Err(PhoneticError::UnsupportedSegment {
                near: remaining.chars().take(12).collect::<String>(),
            });
        };
        segments.push(segment_id);
        remaining = &remaining[segment_length..];
        while let Some(modifier) = remaining.chars().next() {
            if is_ipa_ignored_modifier(modifier) {
                remaining = &remaining[modifier.len_utf8()..];
            } else {
                break;
            }
        }
    }
    if segments.is_empty() {
        Err(PhoneticError::EmptyQuery)
    } else {
        Ok(make_token_sequence(segments))
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_ipa_query(text: &str) -> String {
    text.nfd()
        .filter_map(|value| match value {
            '\u{0261}' => Some('g'),
            value if is_ipa_tie_bar(value) => None,
            value => Some(value),
        })
        .collect()
}

#[requires(true)]
#[ensures((0.0..=1.0).contains(&ret))]
pub fn aline_phonetic_similarity(
    source: IpaTokenSequenceView<'_>,
    target: IpaTokenSequenceView<'_>,
) -> f64 {
    let mut scratch = AlineSimilarityScratch::default();
    aline_phonetic_similarity_with_scratch(source, target, &mut scratch)
}

#[requires(true)]
#[ensures((0.0..=1.0).contains(&ret))]
pub fn aline_phonetic_similarity_with_scratch(
    source: IpaTokenSequenceView<'_>,
    target: IpaTokenSequenceView<'_>,
    scratch: &mut AlineSimilarityScratch,
) -> f64 {
    let raw_similarity =
        aline_raw_similarity_with_scratch(source.segments, target.segments, scratch);
    let normalizer = source.self_similarity + target.self_similarity;
    (2.0 * raw_similarity / normalizer).clamp(0.0, 1.0)
}

#[requires(true)]
#[ensures(matches!(ret, Ordering::Less | Ordering::Equal | Ordering::Greater))]
pub fn compare_similarity_then_index(left: (usize, f64), right: (usize, f64)) -> Ordering {
    right
        .1
        .partial_cmp(&left.1)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.0.cmp(&right.0))
}

#[requires(true)]
#[ensures(ret.is_some_and(|symbol| !symbol.is_empty()))]
pub fn ipa_segment_symbol(id: IpaSegmentId) -> Option<&'static str> {
    Some(IPA_SEGMENT_SYMBOLS[id.get() as usize])
}

#[requires(true)]
#[ensures(true)]
fn bracketed_ipa_query(raw_query: &str) -> Result<Option<String>, PhoneticError> {
    let trimmed = raw_query.trim();
    let starts = trimmed.starts_with('[');
    let ends = trimmed.ends_with(']');
    match (starts, ends) {
        (true, true) => {
            let inner = trimmed[1..trimmed.len() - 1].trim();
            if inner.is_empty() {
                Err(PhoneticError::EmptyBracketedIpa)
            } else if inner.contains('[') || inner.contains(']') {
                Err(PhoneticError::NestedBrackets)
            } else {
                Ok(Some(inner.to_owned()))
            }
        }
        (true, false) => Err(PhoneticError::MissingClosingBracket),
        (false, true) => Err(PhoneticError::MissingOpeningBracket),
        (false, false) if trimmed.contains('[') || trimmed.contains(']') => {
            Err(PhoneticError::PartialBracketedQuery)
        }
        (false, false) => Ok(None),
    }
}

#[requires(true)]
#[ensures(true)]
fn is_ipa_boundary(value: char) -> bool {
    value.is_whitespace()
        || matches!(value, '.' | '/' | '|' | '‖' | 'ˈ' | 'ˌ')
        || ('\u{02E5}'..='\u{02E9}').contains(&value)
}

#[requires(true)]
#[ensures(true)]
fn is_ipa_tie_bar(value: char) -> bool {
    matches!(value, '\u{0361}' | '\u{035C}')
}

#[requires(true)]
#[ensures(true)]
fn is_ipa_ignored_modifier(value: char) -> bool {
    matches!(value, '\u{0300}'..='\u{036F}' | '\u{02B0}'..='\u{02FF}')
        && !is_ipa_boundary(value)
        && !is_ipa_tie_bar(value)
}

#[requires(!remaining.is_empty())]
#[ensures(ret.as_ref().is_some_and(|(_id, length)| *length > 0) || ret.is_none())]
fn match_longest_segment(remaining: &str) -> Option<(IpaSegmentId, usize)> {
    IPA_SEGMENT_NORMALIZED_SYMBOLS
        .iter()
        .enumerate()
        .filter(|(_, symbol)| remaining.starts_with(symbol.as_str()))
        .max_by(|(_, left), (_, right)| left.len().cmp(&right.len()).then_with(|| right.cmp(left)))
        .and_then(|(index, _)| u16::try_from(index).ok())
        .map(|index| {
            let id = new!(IpaSegmentId(index));
            let normalized_symbol = &IPA_SEGMENT_NORMALIZED_SYMBOLS[id.get() as usize];
            (id, normalized_symbol.len())
        })
}

#[requires(!segments.is_empty())]
#[ensures(ret.segment_count() > 0)]
fn make_token_sequence(segments: Vec<IpaSegmentId>) -> IpaTokenSequence {
    let self_similarity = aline_raw_similarity(&segments, &segments);
    new!(IpaTokenSequence {
        segments,
        self_similarity,
    })
}

#[requires(!source.is_empty())]
#[requires(!target.is_empty())]
#[ensures(true)]
fn aline_raw_similarity(source: &[IpaSegmentId], target: &[IpaSegmentId]) -> f64 {
    let mut scratch = AlineSimilarityScratch::default();
    aline_raw_similarity_with_scratch(source, target, &mut scratch)
}

#[requires(!source.is_empty())]
#[requires(!target.is_empty())]
#[ensures(true)]
fn aline_raw_similarity_with_scratch(
    source: &[IpaSegmentId],
    target: &[IpaSegmentId],
    scratch: &mut AlineSimilarityScratch,
) -> f64 {
    let row_width = target.len() + 1;
    scratch.previous_previous.resize(row_width, 0.0);
    scratch.previous.resize(row_width, 0.0);
    scratch.current.resize(row_width, 0.0);
    scratch.previous.fill(0.0);

    let mut has_previous_previous = false;
    let mut best: f64 = 0.0;
    for source_index in 0..source.len() {
        scratch.current[0] = 0.0;
        for target_index in 1..=target.len() {
            let delete_source = scratch.previous[target_index] + ALINE_SKIP_SCORE;
            let insert_target = scratch.current[target_index - 1] + ALINE_SKIP_SCORE;
            let substitute = scratch.previous[target_index - 1]
                + substitution_score(source[source_index], target[target_index - 1]);
            let compress_source = if has_previous_previous && source_index > 0 {
                scratch.previous_previous[target_index - 1]
                    + expansion_score(
                        target[target_index - 1],
                        source[source_index - 1],
                        source[source_index],
                    )
            } else {
                0.0
            };
            let expand_target = if target_index > 1 {
                scratch.previous[target_index - 2]
                    + expansion_score(
                        source[source_index],
                        target[target_index - 2],
                        target[target_index - 1],
                    )
            } else {
                0.0
            };
            let cell = delete_source
                .max(insert_target)
                .max(substitute)
                .max(compress_source)
                .max(expand_target)
                .max(0.0);
            scratch.current[target_index] = cell;
            best = best.max(cell);
        }
        std::mem::swap(&mut scratch.previous_previous, &mut scratch.previous);
        std::mem::swap(&mut scratch.previous, &mut scratch.current);
        has_previous_previous = true;
    }
    best
}

#[requires(true)]
#[ensures(true)]
fn substitution_score(left: IpaSegmentId, right: IpaSegmentId) -> f64 {
    ALINE_SUBSTITUTION_CEILING
        - feature_difference(left, right)
        - vowel_penalty(left)
        - vowel_penalty(right)
}

#[requires(true)]
#[ensures(true)]
fn expansion_score(
    single: IpaSegmentId,
    first_second: IpaSegmentId,
    second_second: IpaSegmentId,
) -> f64 {
    ALINE_EXPANSION_CEILING
        - feature_difference(single, first_second)
        - feature_difference(single, second_second)
        - vowel_penalty(single)
        - vowel_penalty(first_second).max(vowel_penalty(second_second))
}

#[requires(true)]
#[ensures(true)]
fn feature_difference(left: IpaSegmentId, right: IpaSegmentId) -> f64 {
    let segment_count = IPA_SEGMENT_SYMBOLS.len();
    let index = (left.get() as usize) * segment_count + (right.get() as usize);
    // The ALINE tables are built from IPA_SEGMENT_SYMBOLS, and IpaSegmentId
    // guarantees every id indexes that same symbol table.
    ALINE_PAIR_FEATURE_DIFFERENCES[index]
}

#[requires(true)]
#[ensures(true)]
fn feature_difference_from_features(
    left_features: AlineFeatures,
    right_features: AlineFeatures,
) -> f64 {
    relevant_features(left_features, right_features)
        .iter()
        .map(|feature| {
            (feature_value(*feature, left_features) - feature_value(*feature, right_features)).abs()
                * feature_salience(*feature)
        })
        .sum()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn relevant_features(left: AlineFeatures, right: AlineFeatures) -> &'static [AlineFeature] {
    if left.is_consonant || right.is_consonant {
        CONSONANT_RELEVANT_FEATURES
    } else {
        VOWEL_RELEVANT_FEATURES
    }
}

#[requires(true)]
#[ensures(true)]
fn feature_value(feature: AlineFeature, values: AlineFeatures) -> f64 {
    match feature {
        AlineFeature::Syllabic => values.syllabic_value,
        AlineFeature::Place => values.place_value,
        AlineFeature::Manner => values.manner_value,
        AlineFeature::Voice => values.voice_value,
        AlineFeature::Nasal => values.nasal_value,
        AlineFeature::Retroflex => values.retroflex_value,
        AlineFeature::Lateral => values.lateral_value,
        AlineFeature::Aspirated => values.aspirated_value,
        AlineFeature::High => values.high_value,
        AlineFeature::Back => values.back_value,
        AlineFeature::Round => values.round_value,
        AlineFeature::Long => values.long_value,
    }
}

#[requires(true)]
#[ensures(ret > 0.0)]
fn feature_salience(feature: AlineFeature) -> f64 {
    match feature {
        AlineFeature::Syllabic => 5.0,
        AlineFeature::Voice => 10.0,
        AlineFeature::Lateral => 10.0,
        AlineFeature::High => 5.0,
        AlineFeature::Manner => 50.0,
        AlineFeature::Long => 1.0,
        AlineFeature::Place => 40.0,
        AlineFeature::Nasal => 10.0,
        AlineFeature::Aspirated => 5.0,
        AlineFeature::Retroflex => 10.0,
        AlineFeature::Round => 5.0,
        AlineFeature::Back => 5.0,
    }
}

#[requires(true)]
#[ensures(true)]
fn vowel_penalty(segment: IpaSegmentId) -> f64 {
    ALINE_VOWEL_PENALTIES[segment.get() as usize]
}

#[requires(true)]
#[ensures(true)]
fn segment_features(segment: IpaSegmentId) -> AlineFeatures {
    IPA_SEGMENT_FEATURES[segment.get() as usize]
}

#[requires(!symbol.is_empty())]
#[ensures(true)]
fn derive_aline_features(symbol: &str) -> AlineFeatures {
    let base_symbol = strip_length_mark(symbol);
    let is_consonant = !all_short_vowel_symbols().contains(&base_symbol);
    AlineFeatures {
        is_consonant,
        syllabic_value: if is_consonant { 0.0 } else { 1.0 },
        place_value: derive_place_value(base_symbol, is_consonant),
        manner_value: derive_manner_value(base_symbol, is_consonant),
        voice_value: derive_voice_value(base_symbol, is_consonant),
        nasal_value: flag(NASAL_SYMBOLS.contains(&base_symbol)),
        retroflex_value: flag(RETROFLEX_SYMBOLS.contains(&base_symbol)),
        lateral_value: flag(LATERAL_SYMBOLS.contains(&base_symbol)),
        aspirated_value: flag(symbol.ends_with('ʰ')),
        high_value: derive_high_value(base_symbol),
        back_value: derive_back_value(base_symbol),
        round_value: flag(ROUNDED_VOWEL_SYMBOLS.contains(&base_symbol)),
        long_value: flag(symbol != base_symbol),
    }
}

#[requires(true)]
#[ensures(matches!(ret, 0.0 | 1.0))]
fn flag(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

#[requires(!symbol.is_empty())]
#[ensures(!ret.is_empty())]
fn strip_length_mark(symbol: &str) -> &str {
    symbol.strip_suffix('ː').unwrap_or(symbol)
}

#[requires(true)]
#[ensures(true)]
fn derive_place_value(symbol: &str, is_consonant: bool) -> f64 {
    if !is_consonant {
        -1.0
    } else if BILABIAL_SYMBOLS.contains(&symbol) {
        1.0
    } else if LABIODENTAL_SYMBOLS.contains(&symbol) {
        0.95
    } else if DENTAL_SYMBOLS.contains(&symbol) {
        0.9
    } else if ALVEOLAR_SYMBOLS.contains(&symbol) {
        0.85
    } else if RETROFLEX_SYMBOLS.contains(&symbol) {
        0.8
    } else if PALATO_ALVEOLAR_SYMBOLS.contains(&symbol) {
        0.75
    } else if PALATAL_SYMBOLS.contains(&symbol) {
        0.7
    } else if VELAR_SYMBOLS.contains(&symbol) {
        0.6
    } else if UVULAR_SYMBOLS.contains(&symbol) {
        0.5
    } else if PHARYNGEAL_SYMBOLS.contains(&symbol) {
        0.3
    } else if GLOTTAL_SYMBOLS.contains(&symbol) {
        0.1
    } else {
        0.5
    }
}

#[requires(true)]
#[ensures(true)]
fn derive_manner_value(symbol: &str, is_consonant: bool) -> f64 {
    if !is_consonant {
        vowel_manner_value(symbol)
    } else if TRILL_SYMBOLS.contains(&symbol) {
        0.7
    } else if TAP_SYMBOLS.contains(&symbol) {
        0.65
    } else if APPROXIMANT_SYMBOLS.contains(&symbol) {
        0.6
    } else if AFFRICATE_SYMBOLS.contains(&symbol) {
        0.9
    } else if FRICATIVE_SYMBOLS.contains(&symbol) {
        0.8
    } else {
        1.0
    }
}

#[requires(true)]
#[ensures(true)]
fn derive_voice_value(symbol: &str, is_consonant: bool) -> f64 {
    if !is_consonant || VOICED_CONSONANT_SYMBOLS.contains(&symbol) {
        1.0
    } else {
        0.0
    }
}

#[requires(true)]
#[ensures(true)]
fn vowel_manner_value(symbol: &str) -> f64 {
    if HIGH_VOWEL_SYMBOLS.contains(&symbol) {
        0.4
    } else if MID_VOWEL_SYMBOLS.contains(&symbol) {
        0.2
    } else {
        0.0
    }
}

#[requires(true)]
#[ensures(true)]
fn derive_high_value(symbol: &str) -> f64 {
    if HIGH_VOWEL_SYMBOLS.contains(&symbol) {
        1.0
    } else if MID_VOWEL_SYMBOLS.contains(&symbol) {
        0.5
    } else {
        0.0
    }
}

#[requires(true)]
#[ensures(true)]
fn derive_back_value(symbol: &str) -> f64 {
    if FRONT_VOWEL_SYMBOLS.contains(&symbol) {
        1.0
    } else if CENTRAL_VOWEL_SYMBOLS.contains(&symbol) {
        0.5
    } else {
        0.0
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn all_short_vowel_symbols() -> &'static [&'static str] {
    &[
        "i", "y", "ɨ", "ʉ", "ɯ", "u", "I", "U", "e", "ø", "ɘ", "ɵ", "ɤ", "o", "ə", "ɛ", "œ", "ɜ",
        "ɞ", "ʌ", "ɔ", "E", "O", "æ", "ɐ", "a", "ɶ", "ä", "ɑ", "ɒ",
    ]
}

#[requires(true)]
#[ensures(true)]
fn flatten_word_like_ipa(word_like: &WordLike) -> Vec<IpaSurfaceChunk<'_>> {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => vec![IpaSurfaceChunk::Word(word)],
        data!(WordLike::QuotedWord { zo, word }) => {
            vec![IpaSurfaceChunk::Word(zo), IpaSurfaceChunk::Word(word)]
        }
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => vec![
            IpaSurfaceChunk::Word(zoi),
            IpaSurfaceChunk::Word(opening_delimiter),
            IpaSurfaceChunk::Text(drop_leading_zoi_separator_ref(&quoted_text.text)),
            IpaSurfaceChunk::Word(closing_delimiter),
        ],
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => {
            let mut chunks = vec![IpaSurfaceChunk::Word(lohu)];
            chunks.extend(quoted_words.iter().map(IpaSurfaceChunk::Word));
            chunks.push(IpaSurfaceChunk::Word(lehu));
            chunks
        }
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        }) => vec![
            IpaSurfaceChunk::Word(marker),
            IpaSurfaceChunk::Text(&quoted_text.text),
        ],
        data!(WordLike::LerfuWord { base, bu }) => {
            let mut chunks = flatten_word_like_ipa(base);
            chunks.push(IpaSurfaceChunk::Word(bu));
            chunks
        }
        data!(WordLike::ZeiCompound { left, zei, right }) => {
            let mut chunks = flatten_word_like_ipa(left);
            chunks.push(IpaSurfaceChunk::Word(zei));
            chunks.push(IpaSurfaceChunk::Word(right));
            chunks
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn drop_leading_zoi_separator_ref(text: &str) -> &str {
    text.strip_prefix(' ').unwrap_or(text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn render_ipa_surface_chunks(
    chunks: &[IpaSurfaceChunk<'_>],
    source: &str,
) -> Result<String, PhoneticError> {
    let mut rendered = Vec::new();
    let mut previous_word: Option<IpaRenderedWord> = None;
    for chunk in chunks {
        match chunk {
            IpaSurfaceChunk::Word(word) => {
                let word = render_word_ipa(word, source)?;
                let pause_before = previous_word
                    .as_ref()
                    .is_some_and(|previous| previous.trailing_pause_required)
                    || (previous_word.is_some() && word.leading_pause_required);
                let body = if pause_before {
                    ipa_body_with_leading_pause(&word.body)
                } else {
                    word.body.clone()
                };
                rendered.push(body);
                previous_word = Some(word);
            }
            IpaSurfaceChunk::Text(text) => {
                if !text.is_empty() {
                    rendered.push((*text).to_owned());
                }
                previous_word = None;
            }
        }
    }
    Ok(rendered.join(" "))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rendered| !rendered.body.is_empty()) || ret.is_err())]
fn render_word_ipa(word: &Word, source: &str) -> Result<IpaRenderedWord, PhoneticError> {
    let phonemes = word.phonemes();
    let body = if word.kind() == WordKind::Cmevla {
        render_cmevla_ipa_body(&phonemes)
    } else {
        render_syllabified_ipa_body(&pronunciation_syllables(&phonemes).map_err(|error| {
            PhoneticError::Syllabification {
                message: error.to_string(),
            }
        })?)
    };
    Ok(IpaRenderedWord {
        body,
        leading_pause_required: explicit_leading_pause_count(source, word) > 0
            || required_leading_pause_count(word) > 0,
        trailing_pause_required: explicit_trailing_pause_count(source, word) > 0
            || word.kind() == WordKind::Cmevla,
    })
}

#[requires(!body.is_empty())]
#[ensures(!ret.is_empty())]
fn ipa_body_with_leading_pause(body: &str) -> String {
    body.strip_prefix('ˈ')
        .map(|rest| format!("ˈʔ{rest}"))
        .unwrap_or_else(|| format!("ʔ{body}"))
}

#[requires(!phonemes.as_str().is_empty())]
#[ensures(!ret.is_empty())]
fn render_cmevla_ipa_body(phonemes: &Phonemes) -> String {
    let text = phonemes.as_str();
    if text.contains(',') {
        let syllables = text
            .split(',')
            .filter(|syllable| !syllable.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if !syllables.is_empty() {
            return render_syllabified_ipa_body(&syllables);
        }
    }
    if text.chars().any(is_explicit_stress_char) {
        return render_unsyllabified_cmevla_ipa(text);
    }
    match pronunciation_syllables(phonemes) {
        Ok(syllables) => render_syllabified_ipa_body(&syllables),
        Err(_) => render_unsyllabified_cmevla_ipa(text),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty() || syllables.is_empty())]
fn render_syllabified_ipa_body(syllables: &[String]) -> String {
    let stress_index = explicit_stress_syllable_index(syllables)
        .or_else(|| conventional_stress_syllable_index(syllables));

    let mut rendered = String::new();
    for (index, syllable) in syllables.iter().enumerate() {
        if index > 0 {
            rendered.push('.');
        }
        if stress_index == Some(index) {
            rendered.push('ˈ');
        }
        rendered.push_str(&render_ipa_syllable(syllable));
    }
    rendered
}

#[requires(!text.is_empty())]
#[ensures(!ret.is_empty())]
fn render_unsyllabified_cmevla_ipa(text: &str) -> String {
    let mut rendered = String::new();
    for value in text.chars() {
        if is_explicit_stress_char(value) {
            rendered.push('ˈ');
        }
        push_ipa_phoneme(&mut rendered, value);
    }
    rendered
}

#[requires(true)]
#[ensures(true)]
fn explicit_stress_syllable_index(syllables: &[String]) -> Option<usize> {
    syllables
        .iter()
        .position(|syllable| syllable.chars().any(is_explicit_stress_char))
}

#[requires(true)]
#[ensures(true)]
fn conventional_stress_syllable_index(syllables: &[String]) -> Option<usize> {
    let stressable = syllables
        .iter()
        .enumerate()
        .filter_map(|(index, syllable)| syllable_has_full_vowel(syllable).then_some(index))
        .collect::<Vec<_>>();
    stressable.iter().rev().nth(1).copied()
}

#[requires(true)]
#[ensures(true)]
fn syllable_has_full_vowel(syllable: &str) -> bool {
    syllable
        .chars()
        .any(|value| matches!(strip_vowel_diacritic(value), 'a' | 'e' | 'i' | 'o' | 'u'))
}

#[requires(true)]
#[ensures(true)]
fn is_explicit_stress_char(value: char) -> bool {
    matches!(
        value,
        'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý' | 'à' | 'è' | 'ì' | 'ò' | 'ù' | 'ỳ'
    )
}

#[requires(true)]
#[ensures(true)]
fn render_ipa_syllable(syllable: &str) -> String {
    let mut rendered = String::new();
    for value in syllable.chars() {
        push_ipa_phoneme(&mut rendered, value);
    }
    rendered
}

#[requires(true)]
#[ensures(true)]
fn push_ipa_phoneme(output: &mut String, value: char) {
    match value {
        'a' | 'á' | 'à' => output.push('a'),
        'e' | 'é' | 'è' => output.push('e'),
        'i' | 'í' | 'ì' => output.push('i'),
        'o' | 'ó' | 'ò' => output.push('o'),
        'u' | 'ú' | 'ù' => output.push('u'),
        'y' | 'ý' | 'ỳ' => output.push('ə'),
        'ĭ' => output.push('j'),
        'ŭ' => output.push('w'),
        '\'' => output.push('h'),
        '.' => output.push('ʔ'),
        'c' => output.push('ʃ'),
        'j' => output.push('ʒ'),
        other => output.push(other),
    }
}

#[requires(true)]
#[ensures(true)]
fn strip_vowel_diacritic(value: char) -> char {
    match value {
        'á' | 'à' => 'a',
        'é' | 'è' => 'e',
        'í' | 'ì' | 'ĭ' => 'i',
        'ó' | 'ò' => 'o',
        'ú' | 'ù' | 'ŭ' => 'u',
        'ý' | 'ỳ' => 'y',
        other => other,
    }
}

#[requires(true)]
#[ensures(true)]
fn explicit_leading_pause_count(source: &str, word: &Word) -> usize {
    source
        .as_bytes()
        .get(..word.span().byte_start.min(source.len()))
        .unwrap_or_default()
        .iter()
        .rev()
        .take_while(|value| **value == b'.')
        .count()
}

#[requires(true)]
#[ensures(true)]
fn explicit_trailing_pause_count(source: &str, word: &Word) -> usize {
    source
        .as_bytes()
        .get(word.span().byte_end.min(source.len())..)
        .unwrap_or_default()
        .iter()
        .take_while(|value| **value == b'.')
        .count()
}

#[requires(true)]
#[ensures(ret <= 1)]
fn required_leading_pause_count(word: &Word) -> usize {
    usize::from(word_needs_leading_pause(
        word,
        LeadingPauseVowelMode::FoldedVowels,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::requires;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bracketed_ipa_and_lojban_sound_queries_tokenize_to_the_same_word() {
        let bracketed = sound_query_to_token_sequence("[ˈkla.ma]").expect("bracketed IPA");
        let lojban = sound_query_to_token_sequence("klama").expect("Lojban query");

        assert_eq!(bracketed.segment_count(), 5);
        assert_eq!(lojban.segment_count(), 5);
        assert_eq!(
            aline_phonetic_similarity(bracketed.view(), lojban.view()),
            1.0
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn aline_tokenizer_prefers_long_affricate_segments() {
        let tied_affricate = tokenize_ipa_text("t͡ʃa").expect("tie-bar affricate");
        let plain_affricate = tokenize_ipa_text("tʃa").expect("plain affricate");
        let separated = tokenize_ipa_text("t.ʃ.a").expect("separated segments");

        assert_eq!(tied_affricate.segment_count(), 2);
        assert_eq!(plain_affricate.segment_count(), 2);
        assert_eq!(separated.segment_count(), 3);
        assert!(
            aline_phonetic_similarity(tied_affricate.view(), plain_affricate.view())
                > aline_phonetic_similarity(tied_affricate.view(), separated.view())
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn aline_tokenizer_accepts_gimfihi_ipa_normalization_inputs() {
        let normalized = tokenize_ipa_text("ɡátʰ").expect("normalized IPA input");
        let plain = tokenize_ipa_text("gat").expect("plain IPA input");
        let tied = tokenize_ipa_text("d͡ʒa").expect("tie-bar affricate");
        let untied = tokenize_ipa_text("dʒa").expect("untied affricate");

        assert_eq!(normalized.segment_count(), 3);
        assert_eq!(
            aline_phonetic_similarity(normalized.view(), plain.view()),
            1.0
        );
        assert_eq!(aline_phonetic_similarity(tied.view(), untied.view()), 1.0);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nfd_unstable_ipa_segments_match_their_table_entries() {
        for symbol in ["ç", "ä", "äː"] {
            let sequence = tokenize_ipa_text(symbol).expect("IPA segment tokenizes");
            let [segment] = sequence.segments() else {
                panic!("{symbol} should tokenize as exactly one IPA segment");
            };

            assert_eq!(ipa_segment_symbol(*segment), Some(symbol));
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn aline_similarity_reuses_scratch_without_changing_scores() {
        let long = sound_query_to_token_sequence("klama").expect("longer query");
        let short = sound_query_to_token_sequence("ka").expect("shorter query");
        let different = sound_query_to_token_sequence("coi").expect("different query");
        let mut scratch = AlineSimilarityScratch::default();

        assert_eq!(
            aline_phonetic_similarity_with_scratch(long.view(), short.view(), &mut scratch),
            aline_phonetic_similarity(long.view(), short.view())
        );
        assert_eq!(
            aline_phonetic_similarity_with_scratch(short.view(), long.view(), &mut scratch),
            aline_phonetic_similarity(short.view(), long.view())
        );
        assert_eq!(
            aline_phonetic_similarity_with_scratch(different.view(), long.view(), &mut scratch),
            aline_phonetic_similarity(different.view(), long.view())
        );
        assert_eq!(
            aline_phonetic_similarity_with_scratch(long.view(), long.view(), &mut scratch),
            1.0
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn aline_similarity_sorts_descending_with_dictionary_order_tie_breaks() {
        let mut scored = vec![(2, 0.5), (1, 0.8), (0, 0.8), (3, 0.2)];
        scored.sort_by(|left, right| compare_similarity_then_index(*left, *right));

        assert_eq!(scored, vec![(0, 0.8), (1, 0.8), (2, 0.5), (3, 0.2)]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_standard_lojban_ipa() {
        assert_eq!(lojban_text_to_ipa("klama").expect("IPA"), "ˈkla.ma");
        assert_eq!(lojban_text_to_ipa("coi").expect("IPA"), "ʃoj");
    }
}
