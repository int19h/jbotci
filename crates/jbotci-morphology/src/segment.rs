use std::ops::Range;

use bityzba::{ensures, invariant, new, requires};
use vec1::Vec1;

use crate::{LujvoPart, MorphologyErrorKind, MorphologyOptions, Phonemes, WordKind};

mod fast;
pub(crate) use fast::classify_fast_simple_word;
use fast::{
    is_fast_experimental_permissible_consonant_pair, is_fast_initial_pair_chars,
    is_fast_permissible_consonant_pair,
};

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_separator(value: char) -> bool {
    value.is_whitespace()
        || matches!(
            value,
            '.' | '?'
                | '!'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '<'
                | '>'
                | ';'
                | ':'
                | '-'
                | '"'
                | '\u{00ab}'
                | '\u{00bb}'
                | '\u{201c}'
                | '\u{201d}'
                | '\u{2018}'
                | '\u{27e8}'
                | '\u{27e9}'
                | '\u{2997}'
                | '\u{2998}'
                | '\u{2045}'
                | '\u{2046}'
                | '\u{2987}'
                | '\u{2988}'
                | '\u{27e6}'
                | '\u{27e7}'
                | '\u{2989}'
                | '\u{298a}'
                | '\u{27ea}'
                | '\u{27eb}'
        )
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn normalize_word_with_options(raw: &str, options: &MorphologyOptions) -> String {
    raw.chars()
        .filter_map(|value| normalize_char(value, options))
        .collect()
}

#[invariant(is_valid_normalized_char(self.value))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedSourceChar {
    pub source_index: usize,
    pub value: char,
}

#[invariant(self.start < self.end, "morphology violations must cover a non-empty source range")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MorphologyViolation {
    pub kind: MorphologyErrorKind,
    pub start: usize,
    pub end: usize,
}

impl MorphologyViolation {
    #[requires(start < end)]
    #[ensures(ret.kind == kind)]
    #[ensures(ret.start == start)]
    #[ensures(ret.end == end)]
    fn new(kind: MorphologyErrorKind, start: usize, end: usize) -> Self {
        new!(MorphologyViolation {
            kind: kind,
            start: start,
            end: end,
        })
    }
}

#[invariant(self.start < self.end, "source ranges must cover a non-empty span")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceRange {
    pub start: usize,
    pub end: usize,
}

impl SourceRange {
    #[requires(start < end)]
    #[ensures(ret.start == start)]
    #[ensures(ret.end == end)]
    fn new(start: usize, end: usize) -> Self {
        new!(SourceRange {
            start: start,
            end: end,
        })
    }
}

#[requires(true)]
#[ensures(ret.iter().all(|item| is_valid_normalized_char(item.value)))]
pub(crate) fn normalize_source_chars(
    chars: impl IntoIterator<Item = (usize, char)>,
    options: &MorphologyOptions,
) -> Vec<NormalizedSourceChar> {
    chars
        .into_iter()
        .filter_map(|(source_index, value)| {
            normalize_char(value, options).map(|normalized| {
                new!(NormalizedSourceChar {
                    source_index: source_index,
                    value: normalized,
                })
            })
        })
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|violation| violation.start < violation.end))]
pub(crate) fn first_morphology_violation(
    chars: &[NormalizedSourceChar],
) -> Option<MorphologyViolation> {
    let values = normalized_values(chars);
    let range = invalid_apostrophe_range(&values)
        .map(|range| (MorphologyErrorKind::InvalidApostrophe, range))
        .or_else(|| {
            digit_apostrophe_range(&values)
                .map(|range| (MorphologyErrorKind::DigitApostrophe, range))
        })
        .or_else(|| {
            digit_followed_by_nucleus_range(&values)
                .map(|range| (MorphologyErrorKind::DigitVowel, range))
        })
        .or_else(|| {
            breve_not_glide_range(&values).map(|range| (MorphologyErrorKind::BreveNotGlide, range))
        })
        .or_else(|| {
            geminated_consonant_range(&values)
                .map(|range| (MorphologyErrorKind::GeminatedConsonant, range))
        })
        .or_else(|| y_hiatus_range(&values).map(|range| (MorphologyErrorKind::YHiatus, range)))
        .or_else(|| {
            vowel_hiatus_range(&values).map(|range| (MorphologyErrorKind::VowelHiatus, range))
        })
        .or_else(|| {
            voicing_mismatch_range(&values)
                .map(|range| (MorphologyErrorKind::VoicingMismatch, range))
        })
        .or_else(|| {
            forbidden_consonant_triple_range(&values)
                .map(|range| (MorphologyErrorKind::ForbiddenConsonantTriple, range))
        })
        .or_else(|| {
            forbidden_consonant_pair_range(&values)
                .map(|range| (MorphologyErrorKind::ForbiddenConsonantPair, range))
        })
        .or_else(|| {
            slinkuhi_slice(&values, 0, values.len())
                .then_some((MorphologyErrorKind::Slinkuhi, 0..values.len()))
        });
    range.and_then(|(kind, range)| violation_from_normalized_range(chars, kind, range))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end))]
pub(crate) fn cgv_source_range(chars: &[NormalizedSourceChar]) -> Option<SourceRange> {
    let values = normalized_values(chars);
    cgv_range(&values).and_then(|range| source_range_from_normalized_range(chars, range))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end))]
pub(crate) fn experimental_mz_source_range(chars: &[NormalizedSourceChar]) -> Option<SourceRange> {
    let values = normalized_values(chars);
    mz_pair_range(&values).and_then(|range| source_range_from_normalized_range(chars, range))
}

#[requires(true)]
#[ensures(ret.len() == chars.len())]
fn normalized_values(chars: &[NormalizedSourceChar]) -> Vec<char> {
    chars.iter().map(|value| value.value).collect()
}

#[requires(range.start < range.end)]
#[requires(range.end <= chars.len())]
#[ensures(ret.as_ref().is_some_and(|violation| violation.start < violation.end))]
fn violation_from_normalized_range(
    chars: &[NormalizedSourceChar],
    kind: MorphologyErrorKind,
    range: Range<usize>,
) -> Option<MorphologyViolation> {
    let range = source_range_from_normalized_range(chars, range)?;
    Some(MorphologyViolation::new(kind, range.start, range.end))
}

#[requires(range.start < range.end)]
#[requires(range.end <= chars.len())]
#[ensures(ret.as_ref().is_some_and(|range| range.start < range.end))]
fn source_range_from_normalized_range(
    chars: &[NormalizedSourceChar],
    range: Range<usize>,
) -> Option<SourceRange> {
    let start = chars.get(range.start)?.source_index;
    let end = chars.get(range.end - 1)?.source_index + 1;
    (start < end).then(|| SourceRange::new(start, end))
}

#[requires(true)]
#[ensures(ret == normalize_char(value, options).is_some())]
pub(crate) fn is_normalizable_word_char(value: char, options: &MorphologyOptions) -> bool {
    normalize_char(value, options).is_some()
}

#[ensures(ret.as_ref().is_none_or(|(_, phonemes)| !phonemes.is_empty()))]
#[requires(true)]
pub(crate) fn classify_word_with_options(
    raw_word: &str,
    normalized_word: &str,
    options: &MorphologyOptions,
) -> Option<(WordKind, String)> {
    if let Some(kind) = classify_fast_simple_word(raw_word, normalized_word) {
        return Some((kind, canonicalize_brivla_phonemes(normalized_word)));
    }

    let stripped = normalized_word.replace(',', "");
    if stripped.is_empty() {
        return None;
    }

    let normalized_chars = text_chars(normalized_word);
    let blocks_brivla = blocks_word_shape(&normalized_chars);

    if !blocks_brivla && is_gismu(&stripped) {
        return Some((
            WordKind::Gismu,
            canonicalize_brivla_phonemes(normalized_word),
        ));
    }

    if !blocks_brivla && is_lujvo(&stripped) {
        return Some((
            WordKind::Lujvo,
            canonicalize_brivla_phonemes(normalized_word),
        ));
    }

    if !blocks_brivla && is_fuhivla_shape(&stripped) {
        return Some((
            WordKind::Fuhivla,
            canonicalize_brivla_phonemes(normalized_word),
        ));
    }

    if is_cmevla_with_options(normalized_word, options) {
        return Some((
            WordKind::Cmevla,
            mark_predictable_stress(&canonicalize_word_phonemes(normalized_word)),
        ));
    }

    None
}

#[ensures(!ret.is_empty() || normalized_word.is_empty())]
#[requires(true)]
pub(crate) fn canonicalize_word_phonemes(normalized_word: &str) -> String {
    let chars: Vec<char> = normalized_word.chars().collect();
    let mut out = String::new();
    for (index, value) in chars.iter().copied().enumerate() {
        if value == ',' {
            if chars
                .get(index + 1)
                .is_some_and(|_| starts_glide(&chars, index + 1))
            {
                out.push(value);
            }
            continue;
        }
        let output = if is_i_semivowel(&chars, index) {
            'ĭ'
        } else if is_u_semivowel(&chars, index) {
            'ŭ'
        } else {
            normalize_vowel(value)
        };
        out.push(output);
    }
    out
}

#[ensures(!ret.is_empty() || normalized_word.is_empty())]
#[requires(true)]
fn canonicalize_brivla_phonemes(normalized_word: &str) -> String {
    mark_predictable_stress(
        &canonicalize_word_phonemes(normalized_word)
            .chars()
            .filter(|value| *value != ',')
            .collect::<String>(),
    )
}

#[ensures(!ret.is_empty() || phonemes.is_empty())]
#[requires(true)]
fn mark_predictable_stress(phonemes: &str) -> String {
    if has_explicit_stress(phonemes) {
        return phonemes.to_owned();
    }
    let stressable = phonemes
        .char_indices()
        .filter_map(|(index, ch)| is_full_vowel(ch).then_some(index))
        .collect::<Vec<_>>();
    let Some(&stress_index) = stressable.iter().rev().nth(1) else {
        return phonemes.to_owned();
    };
    let mut out = String::with_capacity(phonemes.len() + 1);
    for (index, ch) in phonemes.char_indices() {
        if index == stress_index {
            out.push(acute_vowel(ch));
        } else {
            out.push(ch);
        }
    }
    out
}

#[requires(true)]
#[ensures(true)]
fn has_explicit_stress(phonemes: &str) -> bool {
    phonemes
        .chars()
        .any(|ch| matches!(ch, 'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý'))
}

#[requires(true)]
#[ensures(true)]
fn is_full_vowel(ch: char) -> bool {
    matches!(
        ch,
        'a' | 'e' | 'i' | 'o' | 'u' | 'á' | 'é' | 'í' | 'ó' | 'ú'
    )
}

#[requires(true)]
#[ensures(true)]
fn acute_vowel(ch: char) -> char {
    match ch {
        'a' | 'á' => 'á',
        'e' | 'é' => 'é',
        'i' | 'í' => 'í',
        'o' | 'ó' => 'ó',
        'u' | 'ú' => 'ú',
        other => other,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|parts| !parts.is_empty()))]
pub(crate) fn parse_lujvo_parts(word: &str) -> Option<Vec1<LujvoPart>> {
    let chars = text_chars(word);
    if chars.len() <= 3 || !chars.iter().all(|value| is_lujvo_char(*value)) {
        return None;
    }
    Vec1::try_from_vec(lujvo_parts_from(&chars, 0, false)?).ok()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|syllables| syllables.iter().all(|syllable| !syllable.is_empty())))]
pub(crate) fn pronunciation_syllable_texts(phonemes: &str) -> Option<Vec<String>> {
    let chars = phonemes
        .chars()
        .filter(|value| *value != ',')
        .collect::<Vec<_>>();
    if chars.is_empty() {
        return None;
    }
    pronunciation_syllable_texts_from(&chars, 0, chars.len())
        .or_else(|| fallback_pronunciation_syllable_texts(&chars))
}

#[requires(true)]
#[ensures(true)]
fn normalize_char(value: char, options: &MorphologyOptions) -> Option<char> {
    let normalized = match value {
        '\'' | 'h' | 'H' | '\u{2019}' | '\u{a78b}' | '\u{a78c}' | '\u{02bb}' | '\u{02bf}'
        | '\u{02b0}' | '\u{02d2}' => '\'',
        'A' => {
            return Some(if options.uppercase_marks_stress {
                'á'
            } else {
                'a'
            });
        }
        'E' => {
            return Some(if options.uppercase_marks_stress {
                'é'
            } else {
                'e'
            });
        }
        'I' => {
            return Some(if options.uppercase_marks_stress {
                'í'
            } else {
                'i'
            });
        }
        'O' => {
            return Some(if options.uppercase_marks_stress {
                'ó'
            } else {
                'o'
            });
        }
        'U' => {
            return Some(if options.uppercase_marks_stress {
                'ú'
            } else {
                'u'
            });
        }
        'Y' => {
            return Some(if options.uppercase_marks_stress {
                'ý'
            } else {
                'y'
            });
        }
        'Á' | 'À' | 'à' => 'á',
        'É' | 'È' | 'è' => 'é',
        'Í' | 'Ì' | 'ì' => 'í',
        'Ó' | 'Ò' | 'ò' => 'ó',
        'Ú' | 'Ù' | 'ù' => 'ú',
        'Ý' | 'Ỳ' | 'ỳ' => 'ý',
        'Ĭ' => 'ĭ',
        'Ŭ' => 'ŭ',
        _ => value.to_ascii_lowercase(),
    };
    if is_valid_normalized_char(normalized) {
        Some(normalized)
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn is_valid_normalized_char(value: char) -> bool {
    is_vowel(value)
        || is_consonant(value)
        || matches!(value, 'y' | 'ý' | '\'' | ',' | 'ĭ' | 'ŭ' | '0'..='9')
}

#[requires(true)]
#[ensures(true)]
fn text_chars(text: &str) -> Vec<char> {
    text.chars().collect()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_cmavo_form(text: &str) -> Option<String> {
    let chars = text_chars(text);
    if chars.is_empty() {
        return None;
    }
    if chars.iter().all(|value| matches!(value, 'y' | 'ý')) {
        return Some(text.to_owned());
    }
    if chars.len() == 1 && chars[0].is_ascii_digit() {
        return Some(digit_to_cmavo(chars[0]).to_owned());
    }
    parse_cmavo_form_main(&chars)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn starts_with_cvcy_lujvo(text: &str) -> bool {
    let chars = text_chars(text);
    starts_with_cvcy_lujvo_chars(&chars, 0)
}

#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
#[requires(true)]
fn parse_cmavo_form_main(chars: &[char]) -> Option<String> {
    if chars.first().is_some_and(|value| *value == '\'') || starts_with_cluster(chars, 0) {
        return None;
    }
    for (onset, after_onset) in parse_onsets(chars, 0) {
        if let Some(rest) = parse_cmavo_form_tail(chars, after_onset) {
            return Some(onset + &rest);
        }
    }
    None
}

#[requires(start <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
fn parse_cmavo_form_tail(chars: &[char], start: usize) -> Option<String> {
    for (nucleus, after_nucleus) in parse_nuclei(chars, start) {
        if after_nucleus == chars.len() {
            return Some(nucleus);
        }
        if chars.get(after_nucleus) == Some(&'\'')
            && let Some(rest) = parse_cmavo_form_tail(chars, after_nucleus + 1)
        {
            return Some(format!("{nucleus}'{rest}"));
        }
    }
    None
}

#[requires(start <= chars.len())]
#[ensures(ret.iter().all(|(_, end)| *end >= start && *end <= chars.len()))]
fn parse_onsets(chars: &[char], start: usize) -> Vec<(String, usize)> {
    let mut onsets = Vec::new();
    if let Some((glide, end)) = parse_glide(chars, start) {
        onsets.push((glide, end));
    }
    for end in (start..=chars.len()).rev() {
        if let Some(initial) = parse_initial(chars, start, end) {
            onsets.push((initial, end));
        }
    }
    onsets
}

#[requires(start <= end && end <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|value| value.chars().count() == end - start))]
fn parse_initial(chars: &[char], start: usize, end: usize) -> Option<String> {
    let initial: String = chars.get(start..end)?.iter().collect();
    let valid_shape = match end - start {
        0 => true,
        1 => is_consonant(chars[start]),
        2 => is_fast_initial_pair_chars(chars[start], chars[start + 1]),
        _ => false,
    };
    if !valid_shape {
        return None;
    }
    if end < chars.len() && (is_consonant(chars[end]) || parse_glide(chars, end).is_some()) {
        return None;
    }
    Some(initial)
}

#[requires(start <= chars.len())]
#[ensures(ret.iter().all(|(_, end)| *end > start && *end <= chars.len()))]
fn parse_nuclei(chars: &[char], start: usize) -> Vec<(String, usize)> {
    let mut nuclei = Vec::new();
    if let Some((diphthong, end)) = parse_diphthong(chars, start) {
        nuclei.push((diphthong, end));
    }
    if let Some((vowel, end)) = parse_single_vowel(chars, start) {
        nuclei.push((vowel, end));
    }
    nuclei
}

#[requires(start <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|(_, end)| *end > start && *end <= chars.len()))]
fn parse_diphthong(chars: &[char], start: usize) -> Option<(String, usize)> {
    let first = *chars.get(start)?;
    let second = *chars.get(start + 1)?;
    let semivowel = match (base_vowel(first)?, second) {
        ('a', 'i' | 'í' | 'ĭ') | ('e', 'i' | 'í' | 'ĭ') | ('o', 'i' | 'í' | 'ĭ') => 'ĭ',
        ('a', 'u' | 'ú' | 'ŭ') => 'ŭ',
        _ => return None,
    };
    let end = start + 2;
    if next_non_comma_index(chars, end)
        .is_some_and(|next| matches_diphthong_semivowel(chars[next], semivowel))
    {
        return None;
    }
    if starts_with_nucleus(chars, end) {
        return None;
    }
    Some((format!("{}{}", normalize_vowel(first), semivowel), end))
}

#[requires(true)]
#[ensures(true)]
fn matches_diphthong_semivowel(value: char, semivowel: char) -> bool {
    match semivowel {
        'ĭ' => matches!(value, 'i' | 'í' | 'ĭ'),
        'ŭ' => matches!(value, 'u' | 'ú' | 'ŭ'),
        _ => false,
    }
}

#[requires(start <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|(_, end)| *end == start + 1))]
fn parse_single_vowel(chars: &[char], start: usize) -> Option<(String, usize)> {
    let value = *chars.get(start)?;
    if value == 'y' || value == 'ý' {
        let end = start + 1;
        if starts_with_nucleus(chars, end) {
            return None;
        }
        return Some((value.to_string(), end));
    }
    if !is_vowel(value) {
        return None;
    }
    let end = start + 1;
    if starts_with_nucleus(chars, end) {
        return None;
    }
    Some((normalize_vowel(value).to_string(), end))
}

#[requires(start <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|(_, end)| *end > start && *end <= chars.len()))]
fn parse_glide(chars: &[char], start: usize) -> Option<(String, usize)> {
    let value = *chars.get(start)?;
    let glide = match value {
        'i' | 'í' | 'ĭ' => 'ĭ',
        'u' | 'ú' | 'ŭ' => 'ŭ',
        _ => return None,
    };
    if starts_with_nucleus(chars, start + 1) {
        Some((glide.to_string(), start + 1))
    } else {
        None
    }
}

#[requires(start <= chars.len())]
#[ensures(true)]
fn starts_with_nucleus(chars: &[char], start: usize) -> bool {
    if start >= chars.len() {
        return false;
    }
    parse_diphthong(chars, start).is_some() || parse_single_vowel(chars, start).is_some()
}

#[requires(start <= chars.len())]
#[ensures(true)]
fn starts_with_cluster(chars: &[char], start: usize) -> bool {
    chars
        .get(start)
        .zip(chars.get(start + 1))
        .is_some_and(|(first, second)| is_consonant(*first) && is_consonant(*second))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_cmevla_with_options(normalized: &str, _options: &MorphologyOptions) -> bool {
    let chars = text_chars(normalized);
    chars.last().is_some_and(|last| is_consonant(*last))
        && chars.first().is_some_and(|first| *first != '\'')
        && !blocks_word_shape(&chars)
        && !has_forbidden_consonant_triple(&chars)
        && !has_forbidden_consonant_pair(&chars)
        && !has_digit_followed_by_nucleus(&chars)
        && !has_vowel_hiatus(&chars)
        && chars.iter().all(|value| {
            is_consonant(*value)
                || is_vowel(*value)
                || matches!(*value, 'y' | 'ý' | '\'' | ',' | '0'..='9')
        })
}

#[requires(true)]
#[ensures(true)]
fn blocks_word_shape(chars: &[char]) -> bool {
    has_invalid_apostrophe(chars)
        || has_digit_followed_by_nucleus(chars)
        || has_geminated_consonant(chars)
        || has_y_hiatus(chars)
        || has_vowel_hiatus(chars)
        || has_voicing_mismatch(chars)
        || has_forbidden_consonant_triple(chars)
        || has_forbidden_consonant_pair(chars)
}

#[requires(true)]
#[ensures(true)]
fn has_voicing_mismatch(chars: &[char]) -> bool {
    voicing_mismatch_range(chars).is_some()
}

#[requires(true)]
#[ensures(true)]
fn has_invalid_apostrophe(chars: &[char]) -> bool {
    invalid_apostrophe_range(chars).is_some()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= chars.len()))]
fn invalid_apostrophe_range(chars: &[char]) -> Option<Range<usize>> {
    chars.iter().enumerate().find_map(|(index, value)| {
        (*value == '\''
            && (!previous_non_comma(chars, index)
                .is_some_and(|(_, previous)| can_precede_apostrophe(previous))
                || !starts_with_nucleus(chars, index + 1)))
        .then_some(index..index + 1)
    })
}

#[requires(true)]
#[ensures(true)]
fn can_precede_apostrophe(value: char) -> bool {
    is_vowel(value) || is_y(value) || matches!(value, 'ĭ' | 'ŭ')
}

#[requires(true)]
#[ensures(true)]
fn has_geminated_consonant(chars: &[char]) -> bool {
    geminated_consonant_range(chars).is_some()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= chars.len()))]
fn geminated_consonant_range(chars: &[char]) -> Option<Range<usize>> {
    chars.iter().enumerate().find_map(|(index, value)| {
        if !is_consonant(*value) {
            return None;
        }
        let next = next_non_comma_index(chars, index + 1)?;
        (chars[next] == *value).then_some(index..next + 1)
    })
}

#[requires(true)]
#[ensures(true)]
fn has_forbidden_consonant_triple(chars: &[char]) -> bool {
    forbidden_consonant_triple_range(chars).is_some()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= chars.len()))]
fn forbidden_consonant_triple_range(chars: &[char]) -> Option<Range<usize>> {
    let mut first = None;
    let mut second = None;
    for (index, value) in chars.iter().copied().enumerate() {
        if !is_consonant(value) {
            first = None;
            second = None;
            continue;
        }
        if matches!(
            (first, second, value),
            (Some('n'), Some('d'), 'j' | 'z') | (Some('n'), Some('t'), 'c' | 's')
        ) {
            return Some(index - 2..index + 1);
        }
        first = second;
        second = Some(value);
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn has_forbidden_consonant_pair(chars: &[char]) -> bool {
    forbidden_consonant_pair_range(chars).is_some()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= chars.len()))]
fn forbidden_consonant_pair_range(chars: &[char]) -> Option<Range<usize>> {
    chars.iter().enumerate().find_map(|(index, value)| {
        if !is_consonant(*value) {
            return None;
        }
        let next = next_non_comma_index(chars, index + 1)?;
        (is_consonant(chars[next])
            && !mz_pair(*value, chars[next])
            && !is_fast_permissible_consonant_pair(*value, chars[next]))
        .then_some(index..next + 1)
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= chars.len()))]
fn mz_pair_range(chars: &[char]) -> Option<Range<usize>> {
    chars.iter().enumerate().find_map(|(index, value)| {
        if !is_consonant(*value) {
            return None;
        }
        let next = next_non_comma_index(chars, index + 1)?;
        mz_pair(*value, chars[next]).then_some(index..next + 1)
    })
}

#[requires(true)]
#[ensures(ret == (first == 'm' && second == 'z'))]
fn mz_pair(first: char, second: char) -> bool {
    first == 'm' && second == 'z'
}

#[requires(true)]
#[ensures(true)]
fn has_digit_followed_by_nucleus(chars: &[char]) -> bool {
    digit_followed_by_nucleus_range(chars).is_some()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= chars.len()))]
fn digit_followed_by_nucleus_range(chars: &[char]) -> Option<Range<usize>> {
    chars.iter().enumerate().find_map(|(index, value)| {
        if !value.is_ascii_digit() {
            return None;
        }
        let next = next_non_comma_index(chars, index + 1)?;
        starts_with_nucleus(chars, next).then_some(index..nucleus_end_for_span(chars, next))
    })
}

#[requires(true)]
#[ensures(true)]
fn has_y_hiatus(chars: &[char]) -> bool {
    y_hiatus_range(chars).is_some()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= chars.len()))]
fn y_hiatus_range(chars: &[char]) -> Option<Range<usize>> {
    chars.iter().enumerate().find_map(|(index, value)| {
        if !is_y(*value) {
            return None;
        }
        let next = next_non_comma_index(chars, index + 1)?;
        (!is_y(chars[next]) && starts_with_nucleus(chars, next))
            .then_some(index..nucleus_end_for_span(chars, next))
    })
}

#[requires(true)]
#[ensures(true)]
fn is_vowel(value: char) -> bool {
    base_vowel(value).is_some()
}

#[requires(true)]
#[ensures(true)]
fn base_vowel(value: char) -> Option<char> {
    match value {
        'a' | 'á' => Some('a'),
        'e' | 'é' => Some('e'),
        'i' | 'í' => Some('i'),
        'o' | 'ó' => Some('o'),
        'u' | 'ú' => Some('u'),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_vowel(value: char) -> char {
    match value {
        'á' => 'á',
        'é' => 'é',
        'í' => 'í',
        'ó' => 'ó',
        'ú' => 'ú',
        _ => base_vowel(value).unwrap_or(value),
    }
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
fn is_i_semivowel(chars: &[char], index: usize) -> bool {
    matches!(chars.get(index).copied(), Some('i' | 'í' | 'ĭ'))
        && (is_diphthong_semivowel(chars, index, 'i') || starts_glide(chars, index))
}

#[requires(true)]
#[ensures(true)]
fn is_u_semivowel(chars: &[char], index: usize) -> bool {
    matches!(chars.get(index).copied(), Some('u' | 'ú' | 'ŭ'))
        && (is_diphthong_semivowel(chars, index, 'u') || starts_glide(chars, index))
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn is_diphthong_semivowel(chars: &[char], index: usize, semivowel: char) -> bool {
    let Some((_, previous)) = previous_non_comma(chars, index) else {
        return false;
    };
    if next_starts_nucleus(chars, index + 1) {
        return false;
    }
    matches!(
        (base_vowel(previous), semivowel),
        (Some('a'), 'i') | (Some('e'), 'i') | (Some('o'), 'i') | (Some('a'), 'u')
    )
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn starts_glide(chars: &[char], index: usize) -> bool {
    matches!(
        chars.get(index).copied(),
        Some('i' | 'í' | 'ĭ' | 'u' | 'ú' | 'ŭ')
    ) && next_starts_nucleus(chars, index + 1)
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn next_starts_nucleus(chars: &[char], mut index: usize) -> bool {
    while chars.get(index) == Some(&',') {
        index += 1;
    }
    starts_with_nucleus(chars, index)
}

#[requires(index <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|(found, _)| *found < index))]
fn previous_non_comma(chars: &[char], index: usize) -> Option<(usize, char)> {
    let mut cursor = index;
    while cursor > 0 {
        cursor -= 1;
        let value = chars[cursor];
        if value != ',' {
            return Some((cursor, value));
        }
    }
    None
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= chars.len()))]
fn cgv_range(chars: &[char]) -> Option<Range<usize>> {
    for (index, value) in chars.iter().copied().enumerate() {
        if !matches!(value, 'i' | 'í' | 'ĭ' | 'u' | 'ú' | 'ŭ') || !starts_glide(chars, index) {
            continue;
        }
        if let Some((previous_index, previous)) = previous_non_comma(chars, index)
            && is_consonant(previous)
        {
            return Some(previous_index..glide_end_for_span(chars, index));
        }
    }
    None
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= chars.len()))]
fn digit_apostrophe_range(chars: &[char]) -> Option<Range<usize>> {
    chars.iter().enumerate().find_map(|(index, value)| {
        if !value.is_ascii_digit() {
            return None;
        }
        let next = next_non_comma_index(chars, index + 1)?;
        (chars[next] == '\'').then_some(index..next + 1)
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= chars.len()))]
fn breve_not_glide_range(chars: &[char]) -> Option<Range<usize>> {
    chars.iter().enumerate().find_map(|(index, value)| {
        let invalid = match *value {
            'ĭ' => !is_i_semivowel(chars, index),
            'ŭ' => !is_u_semivowel(chars, index),
            _ => false,
        };
        invalid.then_some(index..index + 1)
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= chars.len()))]
fn voicing_mismatch_range(chars: &[char]) -> Option<Range<usize>> {
    chars.iter().enumerate().find_map(|(index, value)| {
        let first_voicing = obstruent_voicing(*value)?;
        let next = next_non_comma_index(chars, index + 1)?;
        let second_voicing = obstruent_voicing(chars[next])?;
        (first_voicing != second_voicing).then_some(index..next + 1)
    })
}

#[requires(true)]
#[ensures(true)]
fn obstruent_voicing(value: char) -> Option<bool> {
    match value {
        'b' | 'd' | 'g' | 'j' | 'v' | 'z' => Some(true),
        'c' | 'f' | 'k' | 'p' | 's' | 't' | 'x' => Some(false),
        _ => None,
    }
}

#[requires(start < chars.len())]
#[ensures(ret > start && ret <= chars.len())]
fn nucleus_end_for_span(chars: &[char], start: usize) -> usize {
    raw_diphthong_end(chars, start)
        .map(|(_, end)| end)
        .unwrap_or(start + 1)
}

#[requires(start < chars.len())]
#[ensures(ret > start && ret <= chars.len())]
fn glide_end_for_span(chars: &[char], start: usize) -> usize {
    next_non_comma_index(chars, start + 1)
        .filter(|next| starts_with_nucleus(chars, *next))
        .map(|next| nucleus_end_for_span(chars, next))
        .unwrap_or(start + 1)
}

#[requires(true)]
#[ensures(true)]
fn is_gismu(word: &str) -> bool {
    let chars = text_chars(word);
    match &chars[..] {
        [a, b, c, d, e] => {
            (is_consonant(*a)
                && is_vowel(*b)
                && is_consonant(*c)
                && is_consonant(*d)
                && is_vowel(*e)
                && is_fast_experimental_permissible_consonant_pair(*c, *d))
                || (is_fast_initial_pair_chars(*a, *b)
                    && is_vowel(*c)
                    && is_consonant(*d)
                    && is_vowel(*e))
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_lujvo(word: &str) -> bool {
    parse_lujvo_parts(word).is_some()
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn lujvo_from(chars: &[char], index: usize, has_initial_rafsi: bool) -> bool {
    if index >= chars.len() {
        return false;
    }
    if has_initial_rafsi && is_lujvo_core(chars, index) {
        return true;
    }
    if !has_initial_rafsi && is_lujvo_final_rafsi_alone(chars, index) {
        return true;
    }
    initial_rafsi_ends(chars, index)
        .into_iter()
        .any(|end| end > index && lujvo_from(chars, end, true))
}

#[requires(index <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|parts| !parts.is_empty()))]
fn lujvo_parts_from(
    chars: &[char],
    index: usize,
    has_initial_rafsi: bool,
) -> Option<Vec<LujvoPart>> {
    if index >= chars.len() {
        return None;
    }
    if has_initial_rafsi && is_lujvo_core(chars, index) {
        return Some(vec![rafsi_part(chars, index, chars.len())?]);
    }
    if !has_initial_rafsi && is_lujvo_final_rafsi_alone(chars, index) {
        return Some(vec![rafsi_part(chars, index, chars.len())?]);
    }
    for end in initial_rafsi_ends(chars, index) {
        if end <= index {
            continue;
        }
        let Some(mut rest) = lujvo_parts_from(chars, end, true) else {
            continue;
        };
        let mut parts = initial_rafsi_parts(chars, index, end)?;
        parts.append(&mut rest);
        return Some(parts);
    }
    None
}

#[requires(start < end && end <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|parts| !parts.is_empty()))]
fn initial_rafsi_parts(chars: &[char], start: usize, end: usize) -> Option<Vec<LujvoPart>> {
    if let Some(hyphen_start) = (start + 1..end).find(|index| is_rafsi_hyphen_start(chars, *index))
    {
        return Some(vec![
            rafsi_part(chars, start, hyphen_start)?,
            hyphen_part(chars, hyphen_start, end)?,
        ]);
    }
    Some(vec![rafsi_part(chars, start, end)?])
}

#[requires(index < chars.len())]
#[ensures(true)]
fn is_rafsi_hyphen_start(chars: &[char], index: usize) -> bool {
    chars.get(index).is_some_and(|value| is_y(*value))
        || (chars.get(index) == Some(&'\'')
            && chars.get(index + 1).is_some_and(|value| is_y(*value)))
}

#[requires(start < end && end <= chars.len())]
#[ensures(true)]
fn rafsi_part(chars: &[char], start: usize, end: usize) -> Option<LujvoPart> {
    phonemes_part(chars, start, end).map(LujvoPart::rafsi)
}

#[requires(start < end && end <= chars.len())]
#[ensures(true)]
fn hyphen_part(chars: &[char], start: usize, end: usize) -> Option<LujvoPart> {
    phonemes_part(chars, start, end).map(LujvoPart::hyphen)
}

#[requires(start < end && end <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|phonemes| !phonemes.as_str().is_empty()))]
fn phonemes_part(chars: &[char], start: usize, end: usize) -> Option<Phonemes> {
    Phonemes::from_canonical(chars[start..end].iter().collect()).ok()
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn starts_with_cvcy_lujvo_chars(chars: &[char], index: usize) -> bool {
    let Some(base_end) = cvc_rafsi_end(chars, index) else {
        return false;
    };
    if !chars.get(base_end).is_some_and(|value| is_y(*value)) {
        return false;
    }
    let mut after_hyphen = base_end + 1;
    if chars.get(after_hyphen) == Some(&'\'') {
        after_hyphen += 1;
    }
    lujvo_from(chars, after_hyphen, true)
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn is_lujvo_core(chars: &[char], index: usize) -> bool {
    is_gismu_slice(chars, index, chars.len())
        || is_short_final_rafsi_slice(chars, index, chars.len())
        || is_cvv_final_rafsi_slice(chars, index, chars.len())
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn is_lujvo_final_rafsi_alone(chars: &[char], index: usize) -> bool {
    is_cvv_final_rafsi_slice(chars, index, chars.len())
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn initial_rafsi_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    ends.extend(extended_rafsi_ends(chars, index));
    ends.extend(y_rafsi_ends(chars, index));
    ends.extend(y_less_rafsi_ends(chars, index));
    ends.sort_unstable_by(|left, right| right.cmp(left));
    ends.dedup();
    ends
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn extended_rafsi_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    for head_end in brivla_head_ends(chars, index) {
        if chars.get(head_end) == Some(&'\'')
            && chars.get(head_end + 1).is_some_and(|value| is_y(*value))
        {
            let mut end = head_end + 2;
            if chars.get(end) == Some(&'\'') {
                end += 1;
            }
            ends.push(end);
        }
    }
    ends
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end < chars.len()))]
fn brivla_head_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    for end in (index + 1)..chars.len() {
        if starts_with_onset(chars, index)
            && end > index
            && chars[index..end]
                .iter()
                .any(|value| is_vowel(*value) || is_y(*value))
            && !is_cmavo_slice(chars, index, end)
            && !slinkuhi_slice(chars, index, end)
        {
            ends.push(end);
        }
    }
    ends
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn slinkuhi_slice(chars: &[char], start: usize, end: usize) -> bool {
    start < end && is_consonant(chars[start]) && rafsi_string_slice(chars, start + 1, end)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn rafsi_string_slice(chars: &[char], start: usize, end: usize) -> bool {
    rafsi_string_from(chars, start, end)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn rafsi_string_from(chars: &[char], start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }
    if rafsi_string_ending(chars, start, end) {
        return true;
    }
    y_less_rafsi_ends(chars, start)
        .into_iter()
        .any(|next| next > start && next <= end && rafsi_string_from(chars, next, end))
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn rafsi_string_ending(chars: &[char], start: usize, end: usize) -> bool {
    is_gismu_slice(chars, start, end)
        || is_cvv_final_rafsi_slice(chars, start, end)
        || y_less_rafsi_ends(chars, start)
            .into_iter()
            .any(|mid| mid > start && mid < end && is_short_final_rafsi_slice(chars, mid, end))
        || y_rafsi_slice(chars, start, end)
        || hy_rafsi_slice(chars, start, end)
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn y_rafsi_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    for base_end in long_rafsi_ends(chars, index)
        .into_iter()
        .chain(cvc_rafsi_end(chars, index))
    {
        if chars.get(base_end).is_some_and(|value| is_y(*value)) {
            let mut end = base_end + 1;
            if chars.get(end) == Some(&'\'') {
                end += 1;
            }
            ends.push(end);
        }
    }
    ends
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn y_rafsi_slice(chars: &[char], start: usize, end: usize) -> bool {
    long_rafsi_ends(chars, start)
        .into_iter()
        .chain(cvc_rafsi_end(chars, start))
        .any(|base_end| rafsi_hyphen_end(chars, base_end) == Some(end))
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn y_less_rafsi_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    if let Some(end) = cvc_rafsi_end(chars, index) {
        ends.push(end);
    }
    if let Some(end) = ccv_rafsi_end(chars, index) {
        ends.push(end);
    }
    ends.extend(cvv_rafsi_ends(chars, index));
    ends
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn hy_rafsi_slice(chars: &[char], start: usize, end: usize) -> bool {
    long_rafsi_ends(chars, start).into_iter().any(|base_end| {
        chars.get(base_end).is_some_and(|value| is_vowel(*value))
            && hy_rafsi_hyphen_end(chars, base_end + 1) == Some(end)
    }) || ccv_rafsi_end(chars, start)
        .into_iter()
        .chain(cvv_rafsi_ends(chars, start))
        .any(|base_end| hy_rafsi_hyphen_end(chars, base_end) == Some(end))
}

#[requires(index <= chars.len())]
#[ensures(ret.is_none_or(|end| end > index && end <= chars.len()))]
fn rafsi_hyphen_end(chars: &[char], index: usize) -> Option<usize> {
    if chars.get(index).is_some_and(|value| is_y(*value)) {
        let mut end = index + 1;
        if chars.get(end) == Some(&'\'') {
            end += 1;
        }
        Some(end)
    } else {
        None
    }
}

#[requires(index <= chars.len())]
#[ensures(ret.is_none_or(|end| end > index && end <= chars.len()))]
fn hy_rafsi_hyphen_end(chars: &[char], index: usize) -> Option<usize> {
    if chars.get(index) == Some(&'\'') && chars.get(index + 1).is_some_and(|value| is_y(*value)) {
        let mut end = index + 2;
        if chars.get(end) == Some(&'\'') {
            end += 1;
        }
        Some(end)
    } else {
        None
    }
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn long_rafsi_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    if index + 4 <= chars.len()
        && is_fast_initial_pair_chars(chars[index], chars[index + 1])
        && is_vowel(chars[index + 2])
        && is_consonant(chars[index + 3])
    {
        ends.push(index + 4);
    }
    if index + 4 <= chars.len()
        && is_consonant(chars[index])
        && is_vowel(chars[index + 1])
        && is_consonant(chars[index + 2])
        && is_consonant(chars[index + 3])
    {
        ends.push(index + 4);
    }
    ends
}

#[requires(index <= chars.len())]
#[ensures(ret.is_none_or(|end| end > index && end <= chars.len()))]
fn cvc_rafsi_end(chars: &[char], index: usize) -> Option<usize> {
    (index + 3 <= chars.len()
        && is_consonant(chars[index])
        && is_vowel(chars[index + 1])
        && is_consonant(chars[index + 2]))
    .then_some(index + 3)
}

#[requires(index <= chars.len())]
#[ensures(ret.is_none_or(|end| end > index && end <= chars.len()))]
fn ccv_rafsi_end(chars: &[char], index: usize) -> Option<usize> {
    (index + 3 <= chars.len()
        && is_fast_initial_pair_chars(chars[index], chars[index + 1])
        && is_vowel(chars[index + 2]))
    .then_some(index + 3)
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn cvv_rafsi_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    if index < chars.len() && is_consonant(chars[index]) {
        for vowel_end in vowel_pair_ends(chars, index + 1) {
            ends.push(vowel_end);
            if chars.get(vowel_end) == Some(&'r')
                || (chars.get(vowel_end) == Some(&'n') && chars.get(vowel_end + 1) == Some(&'r'))
            {
                ends.push(vowel_end + 1);
            }
        }
    }
    ends
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn vowel_pair_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    if index + 3 <= chars.len()
        && is_vowel(chars[index])
        && chars[index + 1] == '\''
        && is_vowel(chars[index + 2])
    {
        ends.push(index + 3);
    }
    if index + 2 <= chars.len() && is_diphthong_pair(chars[index], chars[index + 1]) {
        ends.push(index + 2);
    }
    ends
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn is_gismu_slice(chars: &[char], start: usize, end: usize) -> bool {
    end > start && is_gismu(&chars[start..end].iter().collect::<String>())
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn is_short_final_rafsi_slice(chars: &[char], start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }
    if end == start + 3
        && is_consonant(chars[start])
        && is_diphthong_pair(chars[start + 1], chars[start + 2])
    {
        return true;
    }
    if end == start + 3
        && is_fast_initial_pair_chars(chars[start], chars[start + 1])
        && is_vowel(chars[start + 2])
    {
        return true;
    }
    end == start + 4
        && is_consonant(chars[start])
        && is_vowel(chars[start + 1])
        && chars[start + 2] == '\''
        && is_vowel(chars[start + 3])
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn is_cvv_final_rafsi_slice(chars: &[char], start: usize, end: usize) -> bool {
    if start >= end || !is_consonant(chars[start]) {
        return false;
    }
    vowel_pair_ends(chars, start + 1)
        .into_iter()
        .any(|vowel_end| vowel_end == end)
}

#[requires(true)]
#[ensures(true)]
fn is_fuhivla_shape(word: &str) -> bool {
    let chars = text_chars(word);
    is_fuhivla_shape_slice(&chars, 0, chars.len())
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn is_fuhivla_shape_slice(chars: &[char], start: usize, end: usize) -> bool {
    if end <= start
        || end - start < 4
        || !chars[end - 1..end].iter().all(|value| is_vowel(*value))
        || chars[start..end]
            .iter()
            .filter(|value| is_vowel(**value))
            .count()
            < 2
        || chars[start..end].iter().any(|value| is_y(*value))
        || has_vowel_hiatus(&chars[start..end])
    {
        return false;
    }
    if rafsi_string_slice(chars, start, end)
        || slinkuhi_slice(chars, start, end)
        || invalid_initial_rafsi_continuation(chars, start, end)
        || invalid_vowel_initial_fuhivla_shape(chars, start, end)
    {
        return false;
    }
    if !starts_with_valid_word_onset(chars, start) {
        return false;
    }
    let slice = &chars[start..end];
    slice.iter().any(|value| is_consonant(*value))
        && has_consonant_cluster(slice)
        && !is_cmavo_slice(chars, start, end)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn invalid_initial_rafsi_continuation(chars: &[char], start: usize, end: usize) -> bool {
    let prefix_end = cvc_rafsi_end(chars, start).or_else(|| ccv_rafsi_end(chars, start));
    let Some(prefix_end) = prefix_end else {
        return false;
    };
    bad_lujvo_prefix_continuation(chars, prefix_end, end)
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn bad_lujvo_prefix_continuation(chars: &[char], index: usize, end: usize) -> bool {
    if index >= end {
        return false;
    }
    if chars[index] == 'r' {
        return starts_t_l(chars, index + 1, end) || starts_n_liquid(chars, index + 1, end);
    }
    starts_affricate_liquid(chars, index, end)
        || starts_jr_vowel(chars, index, end)
        || starts_consonantal_then_forbidden_initial(chars, index, end)
        || starts_gn_vowel(chars, index, end)
        || starts_cgv_sequence(chars, index, end)
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn starts_forbidden_initial_pair(chars: &[char], index: usize, end: usize) -> bool {
    index + 1 < end
        && is_consonant(chars[index])
        && is_consonant(chars[index + 1])
        && !is_fast_initial_pair_chars(chars[index], chars[index + 1])
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn starts_n_liquid(chars: &[char], index: usize, end: usize) -> bool {
    index + 1 < end && chars[index] == 'n' && is_liquid(chars[index + 1])
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn starts_t_l(chars: &[char], index: usize, end: usize) -> bool {
    index + 1 < end && chars[index] == 't' && chars[index + 1] == 'l'
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn starts_affricate_liquid(chars: &[char], index: usize, end: usize) -> bool {
    index + 2 < end
        && matches!(
            (chars[index], chars[index + 1]),
            ('d', 'j' | 'z') | ('t', 'c' | 's')
        )
        && is_liquid(chars[index + 2])
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn starts_jr_vowel(chars: &[char], index: usize, end: usize) -> bool {
    index + 2 < end && chars[index] == 'j' && chars[index + 1] == 'r' && is_vowel(chars[index + 2])
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn starts_consonantal_then_forbidden_initial(chars: &[char], index: usize, end: usize) -> bool {
    index + 3 < end
        && is_consonant(chars[index])
        && is_syllabic(chars[index + 1])
        && starts_forbidden_initial_pair(chars, index + 2, end)
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn starts_gn_vowel(chars: &[char], index: usize, end: usize) -> bool {
    index + 2 < end && chars[index] == 'g' && chars[index + 1] == 'n' && is_vowel(chars[index + 2])
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn starts_cgv_sequence(chars: &[char], index: usize, end: usize) -> bool {
    index < end && is_consonant(chars[index]) && starts_glide(chars, index + 1)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn invalid_vowel_initial_fuhivla_shape(chars: &[char], start: usize, end: usize) -> bool {
    chars
        .get(start)
        .is_some_and(|value| is_vowel(*value) || matches!(value, 'ĭ' | 'ŭ'))
        && !parse_fuhivla_shape(chars, start, end)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn parse_fuhivla_shape(chars: &[char], start: usize, end: usize) -> bool {
    brivla_head_ends_for_fuhivla(chars, start, end)
        .into_iter()
        .any(|head_end| {
            stressed_syllable_ends_for_fuhivla(chars, head_end, end)
                .into_iter()
                .any(|stressed_end| consonantal_chain_then_final(chars, stressed_end, end))
        })
}

#[requires(start <= end && end <= chars.len())]
#[ensures(ret.iter().all(|head_end| *head_end >= start && *head_end <= end))]
fn brivla_head_ends_for_fuhivla(chars: &[char], start: usize, end: usize) -> Vec<usize> {
    if chars.get(start) == Some(&'\'') || !starts_with_onset(chars, start) {
        return Vec::new();
    }
    let mut ends = vec![start];
    let mut stack = vec![start];
    while let Some(index) = stack.pop() {
        for syllable_end in unstressed_syllable_ends_for_fuhivla(chars, index, end) {
            if syllable_end > index && !ends.contains(&syllable_end) {
                ends.push(syllable_end);
                stack.push(syllable_end);
            }
        }
    }
    ends.sort_unstable();
    ends
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.iter().all(|syllable_end| *syllable_end > index && *syllable_end <= end))]
fn unstressed_syllable_ends_for_fuhivla(chars: &[char], index: usize, end: usize) -> Vec<usize> {
    let mut ends: Vec<usize> = syllable_ends(chars, index, end)
        .into_iter()
        .filter(|syllable_end| {
            !syllable_has_explicit_stress(chars, index, *syllable_end)
                && !consonantal_chain_then_final(chars, *syllable_end, end)
        })
        .collect();
    ends.extend(consonantal_syllable_ends(chars, index, end));
    ends.sort_unstable();
    ends.dedup();
    ends
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.iter().all(|syllable_end| *syllable_end > index && *syllable_end <= end))]
fn stressed_syllable_ends_for_fuhivla(chars: &[char], index: usize, end: usize) -> Vec<usize> {
    syllable_ends(chars, index, end)
        .into_iter()
        .filter(|syllable_end| {
            syllable_has_explicit_stress(chars, index, *syllable_end)
                || consonantal_chain_then_final(chars, *syllable_end, end)
        })
        .collect()
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn consonantal_chain_then_final(chars: &[char], index: usize, end: usize) -> bool {
    if final_syllable_slice(chars, index, end) {
        return true;
    }
    consonantal_syllable_ends(chars, index, end)
        .into_iter()
        .any(|next| next > index && consonantal_chain_then_final(chars, next, end))
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn final_syllable_slice(chars: &[char], start: usize, end: usize) -> bool {
    brivla_onset_ends(chars, start)
        .into_iter()
        .filter(|onset_end| !chars.get(*onset_end).is_some_and(|value| is_y(*value)))
        .any(|onset_end| {
            parse_nuclei(chars, onset_end)
                .into_iter()
                .any(|(_, nucleus_end)| {
                    nucleus_end == end && !syllable_has_explicit_stress(chars, start, end)
                })
        })
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.iter().all(|syllable_end| *syllable_end > index && *syllable_end <= end))]
fn syllable_ends(chars: &[char], index: usize, end: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    for onset_end in brivla_onset_ends(chars, index) {
        if chars.get(onset_end).is_some_and(|value| is_y(*value)) {
            continue;
        }
        for (_, nucleus_end) in parse_nuclei(chars, onset_end) {
            if nucleus_end <= end {
                ends.push(nucleus_end);
                ends.extend(coda_ends(chars, nucleus_end, end));
            }
        }
    }
    ends.sort_unstable();
    ends.dedup();
    ends
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|syllables| syllables.iter().all(|syllable| !syllable.is_empty())))]
fn pronunciation_syllable_texts_from(
    chars: &[char],
    index: usize,
    end: usize,
) -> Option<Vec<String>> {
    if index == end {
        return Some(Vec::new());
    }
    let mut candidate_ends = syllable_ends(chars, index, end);
    candidate_ends.extend(consonantal_syllable_ends(chars, index, end));
    candidate_ends.sort_unstable();
    candidate_ends.dedup();
    for candidate_end in candidate_ends {
        let candidate_end = if candidate_end == end {
            final_consonantal_syllable_start(chars, index)
                .filter(|split| *split > index)
                .unwrap_or(candidate_end)
        } else {
            candidate_end
        };
        if candidate_end <= index || candidate_end > end {
            continue;
        }
        let Some(mut rest) = pronunciation_syllable_texts_from(chars, candidate_end, end) else {
            continue;
        };
        rest.insert(0, chars[index..candidate_end].iter().collect());
        return Some(rest);
    }
    None
}

#[requires(!chars.is_empty())]
#[ensures(ret.as_ref().is_none_or(|syllables| syllables.iter().all(|syllable| !syllable.is_empty())))]
fn fallback_pronunciation_syllable_texts(chars: &[char]) -> Option<Vec<String>> {
    let mut syllables: Vec<String> = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if let Some(end) = best_fallback_syllable_end(chars, index) {
            syllables.push(chars[index..end].iter().collect());
            index = end;
        } else if let Some(last) = syllables.last_mut() {
            last.extend(chars[index..].iter().copied());
            break;
        } else {
            return None;
        }
    }
    (!syllables.is_empty()).then_some(syllables)
}

#[requires(index < chars.len())]
#[ensures(ret.is_none_or(|end| end > index && end <= chars.len()))]
fn best_fallback_syllable_end(chars: &[char], index: usize) -> Option<usize> {
    if consonantal_pronunciation_syllable_slice(chars, index, chars.len()) {
        return Some(chars.len());
    }
    for end in consonantal_syllable_ends(chars, index, chars.len()) {
        if has_later_pronunciation_nucleus(chars, end) {
            return Some(end);
        }
    }
    let next_nucleus = next_pronunciation_nucleus_start(chars, index)?;
    let nucleus_end = pronunciation_nucleus_end(chars, next_nucleus)?;
    for onset_start in fallback_onset_starts_before_nucleus(chars, index, next_nucleus) {
        if onset_start > index
            && consonantal_pronunciation_syllable_slice(chars, index, onset_start)
        {
            return Some(onset_start);
        }
    }
    if !has_later_pronunciation_nucleus(chars, nucleus_end) {
        if let Some(final_start) = final_consonantal_syllable_start(chars, nucleus_end)
            && final_start > index
        {
            return Some(final_start);
        }
        return Some(chars.len());
    }
    let next_start = next_pronunciation_syllable_start(chars, nucleus_end);
    (next_start > index).then_some(next_start)
}

#[requires(index <= nucleus_start && nucleus_start < chars.len())]
#[ensures(ret.iter().all(|start| *start >= index && *start <= nucleus_start))]
fn fallback_onset_starts_before_nucleus(
    chars: &[char],
    index: usize,
    nucleus_start: usize,
) -> Vec<usize> {
    (index..=nucleus_start)
        .filter(|start| valid_pronunciation_onset_slice(chars, *start, nucleus_start))
        .collect()
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn consonantal_pronunciation_syllable_slice(chars: &[char], start: usize, end: usize) -> bool {
    end > start + 1
        && is_consonant(chars[start])
        && chars[start + 1..end]
            .iter()
            .all(|value| is_syllabic(*value))
}

#[requires(index <= chars.len())]
#[ensures(ret.is_none_or(|start| start >= index && start < chars.len()))]
fn final_consonantal_syllable_start(chars: &[char], index: usize) -> Option<usize> {
    (index..chars.len())
        .find(|start| consonantal_pronunciation_syllable_slice(chars, *start, chars.len()))
}

#[requires(index <= chars.len())]
#[ensures(ret.is_none_or(|found| found >= index && found < chars.len()))]
fn next_pronunciation_nucleus_start(chars: &[char], index: usize) -> Option<usize> {
    (index..chars.len()).find(|candidate| is_pronunciation_nucleus_start(chars, *candidate))
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn has_later_pronunciation_nucleus(chars: &[char], index: usize) -> bool {
    next_pronunciation_nucleus_start(chars, index).is_some()
}

#[requires(index < chars.len())]
#[ensures(ret.is_none_or(|end| end > index && end <= chars.len()))]
fn pronunciation_nucleus_end(chars: &[char], index: usize) -> Option<usize> {
    if let Some((_, end)) = raw_diphthong_end(chars, index) {
        return Some(end);
    }
    is_pronunciation_nucleus_start(chars, index).then_some(index + 1)
}

#[requires(index < chars.len())]
#[ensures(true)]
fn is_pronunciation_nucleus_start(chars: &[char], index: usize) -> bool {
    matches!(
        chars.get(index).copied(),
        Some('a' | 'á' | 'e' | 'é' | 'i' | 'í' | 'o' | 'ó' | 'u' | 'ú' | 'y' | 'ý')
    ) || raw_diphthong_end(chars, index).is_some()
}

#[requires(nucleus_end <= chars.len())]
#[ensures(ret >= nucleus_end && ret <= chars.len())]
fn next_pronunciation_syllable_start(chars: &[char], nucleus_end: usize) -> usize {
    let Some(next_nucleus) = next_pronunciation_nucleus_start(chars, nucleus_end) else {
        return chars.len();
    };
    for start in nucleus_end..next_nucleus {
        if valid_pronunciation_onset_slice(chars, start, next_nucleus) {
            return start;
        }
    }
    next_nucleus
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn valid_pronunciation_onset_slice(chars: &[char], start: usize, end: usize) -> bool {
    if start == end {
        return true;
    }
    if end == start + 1 {
        return is_consonant(chars[start]) || chars[start] == '\'';
    }
    if end == start + 3
        && chars[start] == 's'
        && is_consonant(chars[start + 1])
        && is_liquid(chars[start + 2])
    {
        return true;
    }
    starts_with_onset(chars, start) && end > start
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.iter().all(|syllable_end| *syllable_end > index && *syllable_end <= end))]
fn consonantal_syllable_ends(chars: &[char], index: usize, end: usize) -> Vec<usize> {
    if index >= end
        || !is_consonant(chars[index])
        || !chars
            .get(index + 1)
            .is_some_and(|value| is_syllabic(*value))
    {
        return Vec::new();
    }
    coda_ends(chars, index + 1, end)
        .into_iter()
        .filter(|coda_end| *coda_end > index + 1)
        .collect()
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.iter().all(|coda_end| *coda_end >= index && *coda_end <= end))]
fn coda_ends(chars: &[char], index: usize, end: usize) -> Vec<usize> {
    let mut ends = vec![index];
    if index < end
        && is_consonant(chars[index])
        && !starts_any_syllable(chars, index, end)
        && starts_any_syllable(chars, index + 1, end)
    {
        ends.push(index + 1);
    }
    ends
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn starts_any_syllable(chars: &[char], index: usize, end: usize) -> bool {
    if index >= end {
        return false;
    }
    if !consonantal_syllable_ends(chars, index, end).is_empty() {
        return true;
    }
    brivla_onset_ends(chars, index)
        .into_iter()
        .filter(|onset_end| !chars.get(*onset_end).is_some_and(|value| is_y(*value)))
        .any(|onset_end| {
            parse_nuclei(chars, onset_end)
                .into_iter()
                .any(|(_, nucleus_end)| nucleus_end <= end)
        })
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|onset_end| *onset_end >= index && *onset_end <= chars.len()))]
fn brivla_onset_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends: Vec<usize> = parse_onsets(chars, index)
        .into_iter()
        .map(|(_, end)| end)
        .collect();
    if chars.get(index) == Some(&'\'') {
        ends.push(index + 1);
    }
    ends.sort_unstable_by(|left, right| right.cmp(left));
    ends.dedup();
    ends
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn syllable_has_explicit_stress(chars: &[char], start: usize, end: usize) -> bool {
    chars[start..end].iter().any(|value| {
        matches!(
            value,
            'á' | 'é' | 'í' | 'ó' | 'ú' | 'Á' | 'É' | 'Í' | 'Ó' | 'Ú'
        )
    })
}

#[requires(true)]
#[ensures(ret == matches!(value, 'l' | 'm' | 'n' | 'r'))]
fn is_syllabic(value: char) -> bool {
    matches!(value, 'l' | 'm' | 'n' | 'r')
}

#[requires(true)]
#[ensures(true)]
fn has_consonant_cluster(chars: &[char]) -> bool {
    chars
        .windows(2)
        .any(|pair| is_consonant(pair[0]) && is_consonant(pair[1]))
}

#[requires(true)]
#[ensures(true)]
fn has_vowel_hiatus(chars: &[char]) -> bool {
    vowel_hiatus_range(chars).is_some()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= chars.len()))]
fn vowel_hiatus_range(chars: &[char]) -> Option<Range<usize>> {
    for index in 0..chars.len() {
        if !is_vowel(chars[index]) {
            continue;
        }
        if starts_repeated_glide_diphthong_sequence(chars, index) {
            let next = next_non_comma_index(chars, index + 1).unwrap_or(index + 1);
            return Some(index..nucleus_end_for_span(chars, next));
        }
        if starts_glide(chars, index) {
            continue;
        }
        if parse_diphthong(chars, index).is_some() {
            continue;
        }
        if next_non_comma_index(chars, index + 1).is_some_and(|next| starts_glide(chars, next)) {
            continue;
        }
        if let Some(next) = next_non_comma_index(chars, index + 1)
            && starts_with_nucleus(chars, next)
        {
            return Some(index..nucleus_end_for_span(chars, next));
        }
    }
    None
}

#[requires(start <= chars.len())]
#[ensures(ret.is_none_or(|(_, end)| end > start && end <= chars.len()))]
fn raw_diphthong_end(chars: &[char], start: usize) -> Option<(char, usize)> {
    let first = base_vowel(*chars.get(start)?)?;
    let second = *chars.get(start + 1)?;
    let semivowel = match (first, second) {
        ('a', 'i' | 'í' | 'ĭ') | ('e', 'i' | 'í' | 'ĭ') | ('o', 'i' | 'í' | 'ĭ') => 'ĭ',
        ('a', 'u' | 'ú' | 'ŭ') => 'ŭ',
        _ => return None,
    };
    Some((semivowel, start + 2))
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn starts_repeated_glide_diphthong_sequence(chars: &[char], index: usize) -> bool {
    let semivowel = match chars.get(index).copied() {
        Some('i' | 'í' | 'ĭ') => 'ĭ',
        Some('u' | 'ú' | 'ŭ') => 'ŭ',
        _ => return false,
    };
    next_non_comma_index(chars, index + 1)
        .and_then(|nucleus_start| raw_diphthong_end(chars, nucleus_start))
        .and_then(|(diphthong_semivowel, diphthong_end)| {
            (diphthong_semivowel == semivowel)
                .then(|| next_non_comma_index(chars, diphthong_end))
                .flatten()
        })
        .is_some_and(|next| matches_diphthong_semivowel(chars[next], semivowel))
}

#[requires(index <= chars.len())]
#[ensures(ret.is_none_or(|found| found >= index && found < chars.len()))]
fn next_non_comma_index(chars: &[char], mut index: usize) -> Option<usize> {
    while chars.get(index) == Some(&',') {
        index += 1;
    }
    (index < chars.len()).then_some(index)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn is_cmavo_slice(chars: &[char], start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }
    parse_cmavo_form(&chars[start..end].iter().collect::<String>()).is_some()
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn starts_with_onset(chars: &[char], index: usize) -> bool {
    index <= chars.len()
        && (index == chars.len()
            || is_vowel(chars[index])
            || is_y(chars[index])
            || is_consonant(chars[index])
            || matches!(chars[index], '\'' | 'ĭ' | 'ŭ'))
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn starts_with_valid_word_onset(chars: &[char], index: usize) -> bool {
    let Some(first) = chars.get(index).copied() else {
        return false;
    };
    if is_vowel(first) || is_y(first) || matches!(first, 'ĭ' | 'ŭ') {
        return true;
    }
    if !is_consonant(first) {
        return false;
    }
    let Some(second) = chars.get(index + 1).copied() else {
        return true;
    };
    if !is_consonant(second) {
        return true;
    }
    if chars
        .get(index + 2)
        .is_some_and(|value| is_consonant(*value))
    {
        valid_three_consonant_initial(chars, index)
            && !chars
                .get(index + 3)
                .is_some_and(|value| is_consonant(*value))
    } else {
        is_fast_initial_pair_chars(first, second)
    }
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn valid_three_consonant_initial(chars: &[char], index: usize) -> bool {
    let (Some(first), Some(second), Some(third)) = (
        chars.get(index).copied(),
        chars.get(index + 1).copied(),
        chars.get(index + 2).copied(),
    ) else {
        return false;
    };
    is_sibilant(first) && is_other_consonant(second) && is_liquid(third)
}

#[requires(true)]
#[ensures(true)]
fn is_sibilant(value: char) -> bool {
    matches!(value, 'c' | 's' | 'j' | 'z')
}

#[requires(true)]
#[ensures(true)]
fn is_other_consonant(value: char) -> bool {
    matches!(
        value,
        'p' | 't' | 'k' | 'f' | 'x' | 'b' | 'd' | 'g' | 'v' | 'm' | 'n'
    )
}

#[requires(true)]
#[ensures(true)]
fn is_liquid(value: char) -> bool {
    matches!(value, 'l' | 'r')
}

#[requires(true)]
#[ensures(true)]
fn is_lujvo_char(value: char) -> bool {
    is_consonant(value) || is_vowel(value) || is_y(value) || matches!(value, '\'' | 'ĭ' | 'ŭ')
}

#[requires(true)]
#[ensures(true)]
fn is_diphthong_pair(first: char, second: char) -> bool {
    matches!(
        (base_vowel(first), base_semivowel(second)),
        (Some('a'), Some('i'))
            | (Some('e'), Some('i'))
            | (Some('o'), Some('i'))
            | (Some('a'), Some('u'))
    )
}

#[requires(true)]
#[ensures(true)]
fn base_semivowel(value: char) -> Option<char> {
    match value {
        'i' | 'í' | 'ĭ' => Some('i'),
        'u' | 'ú' | 'ŭ' => Some('u'),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_y(value: char) -> bool {
    matches!(value, 'y' | 'ý')
}

#[requires(true)]
#[ensures(true)]
fn digit_to_cmavo(value: char) -> &'static str {
    match value {
        '0' => "no",
        '1' => "pa",
        '2' => "re",
        '3' => "ci",
        '4' => "vo",
        '5' => "mu",
        '6' => "xa",
        '7' => "ze",
        '8' => "bi",
        '9' => "so",
        _ => "",
    }
}
