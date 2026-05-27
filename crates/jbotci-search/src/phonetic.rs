use std::cmp::Ordering;

use bityzba::{invariant, requires};
use jbotci_morphology::segment_words_with_modifiers;
use jbotci_output::ipa_morphology_text;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Message(_) => true)]
pub enum PhoneticError {
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
struct IpaSegmentVector {
    symbol: String,
    features: AlineFeatures,
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct IpaTokenSequence {
    segments: Vec<IpaSegmentVector>,
    self_similarity: f64,
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

const ALINE_SKIP_SCORE: f64 = -10.0;
const ALINE_SUBSTITUTION_CEILING: f64 = 35.0;
const ALINE_EXPANSION_CEILING: f64 = 45.0;
const ALINE_VOWEL_PENALTY: f64 = 10.0;

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|sequence| sequence.segment_count() > 0) || ret.is_err())]
pub fn sound_query_to_token_sequence(raw_query: &str) -> Result<IpaTokenSequence, PhoneticError> {
    let ipa = sound_query_to_ipa(raw_query)?;
    let tokenized = tokenize_ipa_text(&ipa)?;
    if tokenized.segment_count() == 0 {
        Err(PhoneticError::Message(
            "Sound search requires at least one IPA segment.".to_owned(),
        ))
    } else {
        Ok(tokenized)
    }
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
    let words = segment_words_with_modifiers(raw_text)
        .map_err(|error| PhoneticError::Message(error.to_string()))?;
    ipa_morphology_text(&words, raw_text).map_err(|error| PhoneticError::Message(error.to_string()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|sequence| sequence.segment_count() > 0) || ret.is_err())]
pub fn tokenize_ipa_text(text: &str) -> Result<IpaTokenSequence, PhoneticError> {
    let mut segments = Vec::new();
    let mut remaining = text.trim();
    while !remaining.is_empty() {
        let Some(first) = remaining.chars().next() else {
            break;
        };
        if is_ipa_boundary(first) {
            remaining = &remaining[first.len_utf8()..];
            continue;
        }
        let Some(segment_text) = match_longest_segment(remaining) else {
            return Err(PhoneticError::Message(format!(
                "Unsupported IPA segment near `{}` for ALINE sound search.",
                remaining.chars().take(12).collect::<String>()
            )));
        };
        let next_index = segment_text.len();
        segments.push(make_segment(segment_text));
        remaining = &remaining[next_index..];
    }
    if segments.is_empty() {
        Err(PhoneticError::Message(
            "Sound search requires at least one IPA segment.".to_owned(),
        ))
    } else {
        Ok(make_token_sequence(segments))
    }
}

#[requires(true)]
#[ensures((0.0..=1.0).contains(&ret))]
pub fn aline_phonetic_similarity(source: &IpaTokenSequence, target: &IpaTokenSequence) -> f64 {
    let raw_similarity = aline_raw_similarity(&source.segments, &target.segments);
    let normalizer = source.self_similarity + target.self_similarity;
    if normalizer <= 0.0 {
        0.0
    } else {
        (2.0 * raw_similarity / normalizer).clamp(0.0, 1.0)
    }
}

impl IpaTokenSequence {
    #[requires(true)]
    #[ensures(ret == self.segments.len())]
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }
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
                Err(PhoneticError::Message(
                    "Bracketed IPA input must not be empty.".to_owned(),
                ))
            } else if inner.contains('[') || inner.contains(']') {
                Err(PhoneticError::Message(
                    "IPA input must use one pair of brackets around the whole query.".to_owned(),
                ))
            } else {
                Ok(Some(inner.to_owned()))
            }
        }
        (true, false) => Err(PhoneticError::Message(
            "IPA input starts with `[` but does not end with `]`.".to_owned(),
        )),
        (false, true) => Err(PhoneticError::Message(
            "IPA input ends with `]` but does not start with `[`.".to_owned(),
        )),
        (false, false) if trimmed.contains('[') || trimmed.contains(']') => Err(
            PhoneticError::Message("Use `[ ... ]` around the whole IPA query.".to_owned()),
        ),
        (false, false) => Ok(None),
    }
}

#[requires(true)]
#[ensures(true)]
fn is_ipa_boundary(value: char) -> bool {
    value.is_whitespace() || matches!(value, '.' | '/' | 'ˈ' | 'ˌ')
}

#[requires(!remaining.is_empty())]
#[ensures(ret.as_ref().is_some_and(|segment| !segment.is_empty()) || ret.is_none())]
fn match_longest_segment(remaining: &str) -> Option<&'static str> {
    all_segment_symbols()
        .into_iter()
        .filter(|symbol| remaining.starts_with(symbol))
        .max_by(|left, right| left.len().cmp(&right.len()).then_with(|| right.cmp(left)))
}

#[requires(!symbol.is_empty())]
#[ensures(ret.symbol == symbol)]
fn make_segment(symbol: &str) -> IpaSegmentVector {
    IpaSegmentVector {
        symbol: symbol.to_owned(),
        features: derive_aline_features(symbol),
    }
}

#[requires(!segments.is_empty())]
#[ensures(ret.segment_count() > 0)]
fn make_token_sequence(segments: Vec<IpaSegmentVector>) -> IpaTokenSequence {
    let self_similarity = aline_raw_similarity(&segments, &segments);
    IpaTokenSequence {
        segments,
        self_similarity,
    }
}

#[requires(!source.is_empty())]
#[requires(!target.is_empty())]
#[ensures(true)]
fn aline_raw_similarity(source: &[IpaSegmentVector], target: &[IpaSegmentVector]) -> f64 {
    let mut previous_previous: Option<Vec<f64>> = None;
    let mut previous = vec![0.0; target.len() + 1];
    let mut best: f64 = 0.0;
    for source_index in 0..source.len() {
        let mut current = vec![0.0; target.len() + 1];
        for target_index in 1..=target.len() {
            let delete_source = previous[target_index] + ALINE_SKIP_SCORE;
            let insert_target = current[target_index - 1] + ALINE_SKIP_SCORE;
            let substitute = previous[target_index - 1]
                + substitution_score(&source[source_index], &target[target_index - 1]);
            let compress_source = previous_previous
                .as_ref()
                .filter(|_| source_index > 0)
                .map_or(0.0, |row| {
                    row[target_index - 1]
                        + expansion_score(
                            &target[target_index - 1],
                            &source[source_index - 1],
                            &source[source_index],
                        )
                });
            let expand_target = if target_index > 1 {
                previous[target_index - 2]
                    + expansion_score(
                        &source[source_index],
                        &target[target_index - 2],
                        &target[target_index - 1],
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
            current[target_index] = cell;
            best = best.max(cell);
        }
        previous_previous = Some(previous);
        previous = current;
    }
    best
}

#[requires(true)]
#[ensures(true)]
fn substitution_score(left: &IpaSegmentVector, right: &IpaSegmentVector) -> f64 {
    ALINE_SUBSTITUTION_CEILING
        - feature_difference(left, right)
        - vowel_penalty(left)
        - vowel_penalty(right)
}

#[requires(true)]
#[ensures(true)]
fn expansion_score(
    single: &IpaSegmentVector,
    first_second: &IpaSegmentVector,
    second_second: &IpaSegmentVector,
) -> f64 {
    ALINE_EXPANSION_CEILING
        - feature_difference(single, first_second)
        - feature_difference(single, second_second)
        - vowel_penalty(single)
        - vowel_penalty(first_second).max(vowel_penalty(second_second))
}

#[requires(true)]
#[ensures(true)]
fn feature_difference(left: &IpaSegmentVector, right: &IpaSegmentVector) -> f64 {
    relevant_features(left, right)
        .into_iter()
        .map(|feature| {
            (feature_value(feature, left.features) - feature_value(feature, right.features)).abs()
                * feature_salience(feature)
        })
        .sum()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn relevant_features(left: &IpaSegmentVector, right: &IpaSegmentVector) -> Vec<AlineFeature> {
    if left.features.is_consonant || right.features.is_consonant {
        vec![
            AlineFeature::Syllabic,
            AlineFeature::Manner,
            AlineFeature::Voice,
            AlineFeature::Nasal,
            AlineFeature::Retroflex,
            AlineFeature::Lateral,
            AlineFeature::Aspirated,
            AlineFeature::Place,
        ]
    } else {
        vec![
            AlineFeature::Syllabic,
            AlineFeature::Nasal,
            AlineFeature::Retroflex,
            AlineFeature::High,
            AlineFeature::Back,
            AlineFeature::Round,
            AlineFeature::Long,
        ]
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
fn vowel_penalty(segment: &IpaSegmentVector) -> f64 {
    if segment.features.is_consonant {
        0.0
    } else {
        ALINE_VOWEL_PENALTY
    }
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
        nasal_value: flag(nasal_symbols().contains(&base_symbol)),
        retroflex_value: flag(retroflex_symbols().contains(&base_symbol)),
        lateral_value: flag(lateral_symbols().contains(&base_symbol)),
        aspirated_value: flag(symbol.ends_with('ʰ')),
        high_value: derive_high_value(base_symbol),
        back_value: derive_back_value(base_symbol),
        round_value: flag(rounded_vowel_symbols().contains(&base_symbol)),
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
    } else if bilabial_symbols().contains(&symbol) {
        1.0
    } else if labiodental_symbols().contains(&symbol) {
        0.95
    } else if dental_symbols().contains(&symbol) {
        0.9
    } else if alveolar_symbols().contains(&symbol) {
        0.85
    } else if retroflex_symbols().contains(&symbol) {
        0.8
    } else if palato_alveolar_symbols().contains(&symbol) {
        0.75
    } else if palatal_symbols().contains(&symbol) {
        0.7
    } else if velar_symbols().contains(&symbol) {
        0.6
    } else if uvular_symbols().contains(&symbol) {
        0.5
    } else if pharyngeal_symbols().contains(&symbol) {
        0.3
    } else if glottal_symbols().contains(&symbol) {
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
    } else if trill_symbols().contains(&symbol) {
        0.7
    } else if tap_symbols().contains(&symbol) {
        0.65
    } else if approximant_symbols().contains(&symbol) {
        0.6
    } else if affricate_symbols().contains(&symbol) {
        0.9
    } else if fricative_symbols().contains(&symbol) {
        0.8
    } else {
        1.0
    }
}

#[requires(true)]
#[ensures(true)]
fn derive_voice_value(symbol: &str, is_consonant: bool) -> f64 {
    if !is_consonant || voiced_consonant_symbols().contains(&symbol) {
        1.0
    } else {
        0.0
    }
}

#[requires(true)]
#[ensures(true)]
fn vowel_manner_value(symbol: &str) -> f64 {
    if high_vowel_symbols().contains(&symbol) {
        0.4
    } else if mid_vowel_symbols().contains(&symbol) {
        0.2
    } else {
        0.0
    }
}

#[requires(true)]
#[ensures(true)]
fn derive_high_value(symbol: &str) -> f64 {
    if high_vowel_symbols().contains(&symbol) {
        1.0
    } else if mid_vowel_symbols().contains(&symbol) {
        0.5
    } else {
        0.0
    }
}

#[requires(true)]
#[ensures(true)]
fn derive_back_value(symbol: &str) -> f64 {
    if front_vowel_symbols().contains(&symbol) {
        1.0
    } else if central_vowel_symbols().contains(&symbol) {
        0.5
    } else {
        0.0
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn all_segment_symbols() -> Vec<&'static str> {
    let mut symbols = consonant_symbols();
    symbols.extend(all_short_vowel_symbols());
    symbols.extend(
        all_short_vowel_symbols()
            .into_iter()
            .filter_map(long_vowel_symbol),
    );
    symbols
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn consonant_symbols() -> Vec<&'static str> {
    [
        bilabial_symbols(),
        labiodental_symbols(),
        dental_symbols(),
        alveolar_symbols(),
        retroflex_symbols(),
        palato_alveolar_symbols(),
        palatal_symbols(),
        velar_symbols(),
        uvular_symbols(),
        pharyngeal_symbols(),
        glottal_symbols(),
        affricate_symbols(),
    ]
    .concat()
}

#[requires(true)]
#[ensures(true)]
fn long_vowel_symbol(symbol: &str) -> Option<&'static str> {
    match symbol {
        "i" => Some("iː"),
        "y" => Some("yː"),
        "ɨ" => Some("ɨː"),
        "ʉ" => Some("ʉː"),
        "ɯ" => Some("ɯː"),
        "u" => Some("uː"),
        "I" => Some("Iː"),
        "U" => Some("Uː"),
        "e" => Some("eː"),
        "ø" => Some("øː"),
        "ɘ" => Some("ɘː"),
        "ɵ" => Some("ɵː"),
        "ɤ" => Some("ɤː"),
        "o" => Some("oː"),
        "ə" => Some("əː"),
        "ɛ" => Some("ɛː"),
        "œ" => Some("œː"),
        "ɜ" => Some("ɜː"),
        "ɞ" => Some("ɞː"),
        "ʌ" => Some("ʌː"),
        "ɔ" => Some("ɔː"),
        "E" => Some("Eː"),
        "O" => Some("Oː"),
        "æ" => Some("æː"),
        "ɐ" => Some("ɐː"),
        "a" => Some("aː"),
        "ɶ" => Some("ɶː"),
        "ä" => Some("äː"),
        "ɑ" => Some("ɑː"),
        "ɒ" => Some("ɒː"),
        _ => None,
    }
}

macro_rules! symbols {
    ($name:ident, [$($value:literal),+ $(,)?]) => {
        #[requires(true)]
        #[ensures(!ret.is_empty())]
        fn $name() -> Vec<&'static str> {
            vec![$($value),+]
        }
    };
}

symbols!(bilabial_symbols, ["p", "b", "m", "ʙ", "β", "ɸ", "B"]);
symbols!(labiodental_symbols, ["f", "v", "ʋ", "ɱ"]);
symbols!(dental_symbols, ["θ", "ð"]);
symbols!(
    alveolar_symbols,
    ["t", "d", "n", "s", "z", "r", "l", "ɹ", "ɾ", "ɬ", "ɮ"]
);
symbols!(retroflex_symbols, ["ʈ", "ɖ", "ɳ", "ʂ", "ʐ", "ɻ", "ɽ"]);
symbols!(palato_alveolar_symbols, ["ʃ", "ʒ"]);
symbols!(palatal_symbols, ["j", "c", "ɟ", "ɲ", "ç", "ʝ"]);
symbols!(velar_symbols, ["k", "g", "x", "ɣ", "ŋ", "w", "ɰ"]);
symbols!(uvular_symbols, ["q", "ɢ", "χ", "ʁ", "ʀ", "ɴ", "N", "R"]);
symbols!(pharyngeal_symbols, ["ħ", "ʕ"]);
symbols!(glottal_symbols, ["h", "ɦ", "ʔ"]);
symbols!(trill_symbols, ["r", "ʀ", "ʙ", "R", "B"]);
symbols!(tap_symbols, ["ɾ", "ɽ"]);
symbols!(approximant_symbols, ["j", "w", "ʋ", "ɹ", "ɻ", "ɰ"]);
symbols!(affricate_symbols, ["t͡ʃ", "d͡ʒ", "tʃ", "dʒ", "ts", "dz"]);
symbols!(
    fricative_symbols,
    [
        "ɸ", "β", "f", "v", "θ", "ð", "s", "z", "ʃ", "ʒ", "ʂ", "ʐ", "ç", "ʝ", "x", "ɣ", "χ", "ʁ",
        "ħ", "ʕ", "h", "ɦ", "ɬ", "ɮ"
    ]
);
symbols!(
    voiced_consonant_symbols,
    [
        "b", "d", "ɖ", "ɟ", "g", "ɢ", "m", "ɱ", "n", "ɳ", "ɲ", "ŋ", "ɴ", "N", "ʙ", "B", "r", "ʀ",
        "R", "ɾ", "ɽ", "β", "v", "ð", "z", "ʒ", "ʐ", "ʝ", "ɣ", "ʁ", "ʕ", "ɦ", "ɮ", "ʋ", "ɹ", "ɻ",
        "ɰ", "j", "w", "l", "d͡ʒ", "dʒ", "dz"
    ]
);
symbols!(nasal_symbols, ["m", "ɱ", "n", "ɳ", "ɲ", "ŋ", "ɴ", "N"]);
symbols!(lateral_symbols, ["l", "ɬ", "ɮ"]);
symbols!(high_vowel_symbols, ["i", "y", "ɨ", "ʉ", "ɯ", "u", "I", "U"]);
symbols!(
    mid_vowel_symbols,
    [
        "e", "ø", "ɘ", "ɵ", "ɤ", "o", "ə", "ɛ", "œ", "ɜ", "ɞ", "ʌ", "ɔ", "E", "O"
    ]
);
symbols!(low_vowel_symbols, ["æ", "ɐ", "a", "ɶ", "ä", "ɑ", "ɒ"]);
symbols!(
    front_vowel_symbols,
    ["i", "y", "e", "ø", "ɛ", "œ", "æ", "a", "ɶ", "I", "E"]
);
symbols!(
    central_vowel_symbols,
    ["ɨ", "ʉ", "ɘ", "ɵ", "ə", "ɜ", "ɞ", "ɐ", "ä"]
);
symbols!(
    rounded_vowel_symbols,
    [
        "y", "ʉ", "u", "ø", "ɵ", "o", "œ", "ɞ", "ɔ", "ɶ", "ɒ", "U", "O"
    ]
);

#[requires(true)]
#[ensures(!ret.is_empty())]
fn all_short_vowel_symbols() -> Vec<&'static str> {
    [
        high_vowel_symbols(),
        mid_vowel_symbols(),
        low_vowel_symbols(),
    ]
    .concat()
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
        assert_eq!(aline_phonetic_similarity(&bracketed, &lojban), 1.0);
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
            aline_phonetic_similarity(&tied_affricate, &plain_affricate)
                > aline_phonetic_similarity(&tied_affricate, &separated)
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
}
