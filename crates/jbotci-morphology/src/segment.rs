use std::ops::Range;

use bityzba::{ensures, invariant, new, requires};
use vec1::Vec1;

use crate::{
    Cmavo, LujvoParseExpectation, LujvoPart, MorphologyErrorDetail, MorphologyErrorDetailData,
    MorphologyErrorKind, MorphologyOptions, Phonemes, Selmaho, WordKind,
};

mod phonotactics;
use phonotactics::{
    experimental_permissible_consonant_pair, initial_pair_chars, permissible_consonant_pair,
};

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_separator(value: char) -> bool {
    value.is_whitespace()
        || is_cyrillic_period(value)
        || is_zbalermorna_period(value)
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
    normalize_source_chars(raw.chars().enumerate(), options)
        .into_iter()
        .map(|value| value.value)
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().is_some_and(|text| text.chars().all(is_valid_normalized_char)) || ret.is_none())]
pub(crate) fn normalize_word_checked_with_options(
    raw: &str,
    options: &MorphologyOptions,
) -> Option<String> {
    Some(
        normalize_source_chars_checked(raw.chars().enumerate(), options)
            .ok()?
            .into_iter()
            .map(|value| value.value)
            .collect(),
    )
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn first_unnormalizable_word_char(
    raw: &str,
    options: &MorphologyOptions,
) -> Option<(usize, char)> {
    normalize_source_chars_checked(raw.chars().enumerate(), options)
        .err()
        .map(|error| (error.source_index, error.source_value))
}

#[invariant(self.source_start <= self.source_end)]
#[invariant(is_valid_normalized_char(self.value))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedSourceChar {
    pub source_start: usize,
    pub source_end: usize,
    pub source_value: char,
    pub value: char,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NormalizationError {
    pub source_index: usize,
    pub source_value: char,
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
    normalize_source_chars_with_mode(chars, options, false)
        .expect("unchecked normalization skips unnormalizable source characters")
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|chars| chars.iter().all(|item| is_valid_normalized_char(item.value))) || ret.is_err())]
pub(crate) fn normalize_source_chars_checked(
    chars: impl IntoIterator<Item = (usize, char)>,
    options: &MorphologyOptions,
) -> Result<Vec<NormalizedSourceChar>, NormalizationError> {
    normalize_source_chars_with_mode(chars, options, true)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|chars| chars.iter().all(|item| is_valid_normalized_char(item.value))) || ret.is_err())]
fn normalize_source_chars_with_mode(
    chars: impl IntoIterator<Item = (usize, char)>,
    options: &MorphologyOptions,
    checked: bool,
) -> Result<Vec<NormalizedSourceChar>, NormalizationError> {
    let source_chars = chars.into_iter().collect::<Vec<_>>();
    let mut normalized = Vec::new();
    let mut previous_implicit_apostrophe_vowel = false;
    let mut cursor = 0;
    while cursor < source_chars.len() {
        let (source_index, value) = source_chars[cursor];
        if options.accept_zbalermorna && is_zbalermorna_attitudinal_shorthand(value) {
            if let Some((payload_index, payload_value)) = source_chars.get(cursor + 1).copied()
                && let Some(payload) = normalize_zbalermorna_shorthand_payload(payload_value)
            {
                push_normalized_text(
                    &mut normalized,
                    &mut previous_implicit_apostrophe_vowel,
                    payload_index,
                    payload_index + 1,
                    payload_value,
                    payload,
                    false,
                );
                push_normalized_value(
                    &mut normalized,
                    &mut previous_implicit_apostrophe_vowel,
                    source_index,
                    payload_index + 1,
                    value,
                    '\'',
                    false,
                );
                cursor += 2;
                continue;
            }
            if checked {
                return Err(NormalizationError {
                    source_index,
                    source_value: value,
                });
            }
            previous_implicit_apostrophe_vowel = false;
            cursor += 1;
            continue;
        }
        match normalize_char_event(value, options) {
            Some(NormalizedCharEvent::Emit {
                value: normalized_value,
                implicit_apostrophe_vowel,
            }) => {
                push_normalized_value(
                    &mut normalized,
                    &mut previous_implicit_apostrophe_vowel,
                    source_index,
                    source_index + 1,
                    value,
                    normalized_value,
                    implicit_apostrophe_vowel,
                );
            }
            Some(NormalizedCharEvent::EmitText { text }) => {
                push_normalized_text(
                    &mut normalized,
                    &mut previous_implicit_apostrophe_vowel,
                    source_index,
                    source_index + 1,
                    value,
                    text,
                    false,
                );
            }
            Some(NormalizedCharEvent::StressPrevious) => {
                stress_last_normalized_char(&mut normalized);
            }
            Some(NormalizedCharEvent::StressPreviousVowel) => {
                stress_last_normalized_vowel_char(&mut normalized);
            }
            Some(NormalizedCharEvent::Ignore) => {
                previous_implicit_apostrophe_vowel = false;
            }
            None => {
                if checked {
                    return Err(NormalizationError {
                        source_index,
                        source_value: value,
                    });
                }
                previous_implicit_apostrophe_vowel = false;
            }
        }
        cursor += 1;
    }
    Ok(normalized)
}

#[requires(is_valid_normalized_char(value))]
#[requires(source_start <= source_end)]
#[ensures(true)]
fn push_normalized_value(
    normalized: &mut Vec<NormalizedSourceChar>,
    previous_implicit_apostrophe_vowel: &mut bool,
    source_start: usize,
    source_end: usize,
    source_value: char,
    value: char,
    implicit_apostrophe_vowel: bool,
) {
    if *previous_implicit_apostrophe_vowel && implicit_apostrophe_vowel {
        normalized.push(new!(NormalizedSourceChar {
            source_start: source_start,
            source_end: source_start,
            source_value: '\'',
            value: '\''
        }));
    }
    normalized.push(new!(NormalizedSourceChar {
        source_start: source_start,
        source_end: source_end,
        source_value: source_value,
        value: value,
    }));
    *previous_implicit_apostrophe_vowel = implicit_apostrophe_vowel;
}

#[requires(text.chars().all(is_valid_normalized_char))]
#[requires(source_start <= source_end)]
#[ensures(true)]
fn push_normalized_text(
    normalized: &mut Vec<NormalizedSourceChar>,
    previous_implicit_apostrophe_vowel: &mut bool,
    source_start: usize,
    source_end: usize,
    source_value: char,
    text: &str,
    implicit_apostrophe_vowel: bool,
) {
    for value in text.chars() {
        push_normalized_value(
            normalized,
            previous_implicit_apostrophe_vowel,
            source_start,
            source_end,
            source_value,
            value,
            implicit_apostrophe_vowel,
        );
    }
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
            hard_breve_not_glide_range(chars)
                .map(|range| (MorphologyErrorKind::BreveNotGlide, range))
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
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end))]
pub(crate) fn latin_breve_not_glide_source_range(
    chars: &[NormalizedSourceChar],
) -> Option<SourceRange> {
    latin_breve_not_glide_range(chars)
        .and_then(|range| source_range_from_normalized_range(chars, range))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end))]
pub(crate) fn required_breve_not_glide_source_range(
    chars: &[NormalizedSourceChar],
) -> Option<SourceRange> {
    hard_breve_not_glide_range(chars)
        .and_then(|range| source_range_from_normalized_range(chars, range))
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
    let slice = chars.get(range)?;
    let start = slice.iter().map(|value| value.source_start).min()?;
    let end = slice.iter().map(|value| value.source_end).max()?;
    (start < end).then(|| SourceRange::new(start, end))
}

#[ensures(ret.as_ref().is_none_or(|(_, phonemes)| !phonemes.is_empty()))]
#[requires(true)]
pub(crate) fn classify_word_with_options(
    normalized_word: &str,
    options: &MorphologyOptions,
) -> Option<(WordKind, String)> {
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
            out.push(value);
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
    let stressable = stressable_nucleus_byte_starts(phonemes);
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
fn stressable_nucleus_byte_starts(phonemes: &str) -> Vec<usize> {
    let chars = text_chars(phonemes);
    let byte_starts = phonemes
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let mut starts = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == ',' {
            index += 1;
            continue;
        }
        if let Some((_, end)) = parse_diphthong(&chars, index) {
            if let Some(byte_start) = byte_starts.get(index).copied() {
                starts.push(byte_start);
            }
            index = end;
        } else if let Some((_, end)) = parse_single_vowel(&chars, index) {
            if !matches!(chars[index], 'y' | 'ý')
                && let Some(byte_start) = byte_starts.get(index).copied()
            {
                starts.push(byte_start);
            }
            index = end;
        } else {
            index += 1;
        }
    }
    starts
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
    match analyze_lujvo_parts(word) {
        Ok(parts) => Some(parts),
        Err(_) => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|parts| !parts.is_empty()))]
pub(crate) fn parse_lujvo_parts_with_canonical_phonemes(
    shape_word: &str,
    canonical_word: &str,
) -> Option<Vec1<LujvoPart>> {
    let shape_chars = text_chars(shape_word);
    let canonical_chars = text_chars(canonical_word);
    if shape_chars.len() != canonical_chars.len() {
        return None;
    }
    let ranges = analyze_lujvo_part_ranges_chars(&shape_chars).ok()?;
    let parts = lujvo_ranges_to_parts(&canonical_chars, ranges.into_iter().collect())?;
    Vec1::try_from_vec(parts).ok()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn invalid_lujvo_error_detail(word: &str) -> Option<MorphologyErrorDetail> {
    let chars = text_chars(word);
    if !chars.iter().any(|value| is_y(*value)) {
        return None;
    }
    let Err(failure) = analyze_lujvo_parts_chars(&chars) else {
        return None;
    };
    failure.to_detail(&chars)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn fuhivla_y_error_detail(word: &str) -> Option<MorphologyErrorDetail> {
    let chars = text_chars(word);
    fuhivla_shape_slice_rejected_by_y(&chars, 0, chars.len())
        .then_some(new!(MorphologyErrorDetail::FuhivlaContainsY))
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LujvoParseFailure {
    index: usize,
    expected: LujvoParseExpectation,
}

impl LujvoParseFailure {
    #[requires(true)]
    #[ensures(ret.index == index)]
    #[ensures(ret.expected == expected)]
    fn new(index: usize, expected: LujvoParseExpectation) -> Self {
        LujvoParseFailure {
            index: index,
            expected: expected,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn to_detail(self, chars: &[char]) -> Option<MorphologyErrorDetail> {
        if self.index == 0 || self.index > chars.len() {
            return None;
        }
        Some(new!(MorphologyErrorDetail::InvalidLujvo {
            parsed_prefix: Some(chars[..self.index].iter().collect()),
            expected: self.expected,
        }))
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|parts| !parts.is_empty()) || ret.as_ref().is_err())]
fn analyze_lujvo_parts(word: &str) -> Result<Vec1<LujvoPart>, LujvoParseFailure> {
    let chars = text_chars(word);
    analyze_lujvo_parts_chars(&chars)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|parts| !parts.is_empty()) || ret.as_ref().is_err())]
fn analyze_lujvo_parts_chars(chars: &[char]) -> Result<Vec1<LujvoPart>, LujvoParseFailure> {
    let ranges = analyze_lujvo_part_ranges_chars(chars)?;
    let parts = lujvo_ranges_to_parts(chars, ranges.into_iter().collect()).ok_or_else(|| {
        LujvoParseFailure::new(0, LujvoParseExpectation::InitialOrStandaloneFinalRafsi)
    })?;
    Vec1::try_from_vec(parts).map_err(|_| {
        LujvoParseFailure::new(0, LujvoParseExpectation::InitialOrStandaloneFinalRafsi)
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|ranges| !ranges.is_empty()) || ret.as_ref().is_err())]
fn analyze_lujvo_part_ranges_chars(
    chars: &[char],
) -> Result<Vec1<Range<usize>>, LujvoParseFailure> {
    let mut failure = None;
    if chars.len() <= 3 || !chars.iter().all(|value| is_lujvo_char(*value)) {
        record_lujvo_failure(&mut failure, 0, false);
        return Err(failure.expect("initial lujvo failure was recorded"));
    }
    if let Some(ranges) = lujvo_part_ranges_from(chars, 0, false, &mut failure)
        && let Ok(ranges) = Vec1::try_from_vec(ranges)
    {
        return Ok(ranges);
    }
    Err(failure.unwrap_or_else(|| {
        LujvoParseFailure::new(0, LujvoParseExpectation::InitialOrStandaloneFinalRafsi)
    }))
}

#[requires(true)]
#[ensures(true)]
fn record_lujvo_failure(
    failure: &mut Option<LujvoParseFailure>,
    index: usize,
    has_initial_rafsi: bool,
) {
    let expected = if has_initial_rafsi {
        LujvoParseExpectation::FinalOrInitialRafsi
    } else {
        LujvoParseExpectation::InitialOrStandaloneFinalRafsi
    };
    let candidate = LujvoParseFailure::new(index, expected);
    if failure.is_none_or(|current| candidate.index > current.index) {
        *failure = Some(candidate);
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|syllables| syllables.iter().all(|syllable| !syllable.is_empty())))]
pub(crate) fn pronunciation_syllable_texts(phonemes: &str) -> Option<Vec<String>> {
    let chars = pronunciation_chars(phonemes)?;
    if chars.is_empty() {
        return None;
    }
    strict_pronunciation_syllable_texts(&chars)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct PronunciationChar {
    original: char,
    annotated: char,
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|chars| {
    chars.iter().all(|value| value.original != ',' && value.annotated != ',')
}))]
fn pronunciation_chars(phonemes: &str) -> Option<Vec<PronunciationChar>> {
    let mut chars = phonemes
        .chars()
        .filter(|value| *value != ',')
        .map(|original| {
            pronunciation_base_char(original).map(|annotated| PronunciationChar {
                original,
                annotated,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    mark_pronunciation_glides(&mut chars)?;
    Some(chars)
}

#[requires(chars.iter().all(|value| value.annotated != ','))]
#[ensures(ret.as_ref().is_some_and(|syllables| syllables.iter().all(|syllable| !syllable.is_empty())) || ret.is_none())]
fn strict_pronunciation_syllable_texts(chars: &[PronunciationChar]) -> Option<Vec<String>> {
    if !chars
        .iter()
        .any(|value| is_pronunciation_vowel(value.annotated))
    {
        return None;
    }
    let ranges = split_pronunciation_after_nuclei(chars);
    let mut syllables = Vec::new();
    for (index, range) in ranges.iter().enumerate() {
        let piece = &chars[range.clone()];
        if index == ranges.len() - 1
            && !piece
                .iter()
                .any(|value| is_pronunciation_vowel(value.annotated))
        {
            apply_pronunciation_coda(&mut syllables, piece, None)?;
            continue;
        }
        if piece
            .last()
            .is_some_and(|value| is_pronunciation_offglide(value.annotated))
            && (piece.len() < 2 || !is_pronunciation_vowel(piece[piece.len() - 2].annotated))
        {
            return None;
        }
        let nucleus_len = if piece
            .last()
            .is_some_and(|value| is_pronunciation_offglide(value.annotated))
        {
            2
        } else {
            1
        };
        let onset = &piece[..piece.len().checked_sub(nucleus_len)?];
        let nucleus = pronunciation_original_text(&piece[piece.len() - nucleus_len..]);
        if onset.is_empty() {
            if index != 0 {
                return None;
            }
            syllables.push(nucleus);
            continue;
        }
        if onset.len() == 1 && onset[0].annotated == '\'' {
            if index == 0 {
                return None;
            }
            syllables.push(format!("'{}", nucleus));
            continue;
        }
        if onset.len() == 1 && is_pronunciation_onglide(onset[0].annotated) {
            let mut syllable = String::with_capacity(onset[0].original.len_utf8() + nucleus.len());
            syllable.push(onset[0].original);
            syllable.push_str(&nucleus);
            syllables.push(syllable);
            continue;
        }
        if pronunciation_hard_onset(onset) {
            let mut syllable = pronunciation_original_text(onset);
            syllable.push_str(&nucleus);
            syllables.push(syllable);
            continue;
        }
        if onset.iter().any(|value| value.annotated == '\'') {
            return None;
        }
        split_pronunciation_onset(&mut syllables, onset, &nucleus)?;
    }
    Some(syllables)
}

#[requires(chars.iter().all(|value| value.annotated != ','))]
#[ensures(ret.iter().all(|range| range.start < range.end && range.end <= chars.len()))]
fn split_pronunciation_after_nuclei(chars: &[PronunciationChar]) -> Vec<Range<usize>> {
    let mut pieces = Vec::new();
    let mut start = 0;
    for index in 0..chars.len() {
        let current = chars[index].annotated;
        let next = chars.get(index + 1).map(|value| value.annotated);
        if (is_pronunciation_vowel(current) && !next.is_some_and(is_pronunciation_offglide))
            || is_pronunciation_offglide(current)
        {
            pieces.push(start..index + 1);
            start = index + 1;
        }
    }
    if start < chars.len() {
        pieces.push(start..chars.len());
    }
    pieces
}

#[requires(true)]
#[ensures(ret.is_none_or(|_| {
    chars.iter().all(|value| {
        is_consonant(value.annotated)
            || is_pronunciation_vowel(value.annotated)
            || matches!(value.annotated, 'q' | 'w' | 'ĭ' | 'ŭ' | '\'')
    })
}))]
fn mark_pronunciation_glides(chars: &mut [PronunciationChar]) -> Option<()> {
    for index in (0..chars.len()).rev() {
        let value = chars[index].annotated;
        if !matches!(value, 'i' | 'u') {
            continue;
        }
        let (onglide, offglide) = if value == 'i' {
            ('q', 'ĭ')
        } else {
            ('w', 'ŭ')
        };
        if chars
            .get(index + 1)
            .is_some_and(|next| is_pronunciation_vowel(next.annotated))
        {
            chars[index].annotated = onglide;
            continue;
        }
        if index == 0 {
            continue;
        }
        let previous = chars[index - 1].annotated;
        if !matches!(previous, 'a' | 'e' | 'o' | 'y') {
            continue;
        }
        if !pronunciation_diphthong(previous, value) {
            return None;
        }
        chars[index].annotated = offglide;
        if chars
            .get(index + 1)
            .is_some_and(|next| next.annotated == onglide)
        {
            return None;
        }
    }
    Some(())
}

#[requires(true)]
#[ensures(true)]
fn pronunciation_diphthong(first: char, second: char) -> bool {
    matches!((first, second), ('a' | 'e' | 'o', 'i') | ('a', 'u'))
}

#[requires(chars.iter().all(|value| value.original != ','))]
#[ensures(!ret.is_empty() || chars.is_empty())]
fn pronunciation_original_text(chars: &[PronunciationChar]) -> String {
    chars.iter().map(|value| value.original).collect()
}

#[requires(true)]
#[ensures(ret.is_some() == (is_consonant(value) || pronunciation_base_vowel(value).is_some() || matches!(value, 'ĭ' | 'ŭ' | '\'')))]
fn pronunciation_base_char(value: char) -> Option<char> {
    pronunciation_base_vowel(value)
        .or_else(|| is_consonant(value).then_some(value))
        .or_else(|| matches!(value, 'ĭ').then_some('i'))
        .or_else(|| matches!(value, 'ŭ').then_some('u'))
        .or_else(|| matches!(value, '\'').then_some('\''))
}

#[requires(true)]
#[ensures(ret.is_none_or(|value| matches!(value, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')))]
fn pronunciation_base_vowel(value: char) -> Option<char> {
    match value {
        'a' | 'á' => Some('a'),
        'e' | 'é' => Some('e'),
        'i' | 'í' => Some('i'),
        'o' | 'ó' => Some('o'),
        'u' | 'ú' => Some('u'),
        'y' | 'ý' => Some('y'),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret == matches!(value, 'a' | 'e' | 'i' | 'o' | 'u' | 'y'))]
fn is_pronunciation_vowel(value: char) -> bool {
    matches!(value, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
}

#[requires(true)]
#[ensures(ret == matches!(value, 'q' | 'w'))]
fn is_pronunciation_onglide(value: char) -> bool {
    matches!(value, 'q' | 'w')
}

#[requires(true)]
#[ensures(ret == matches!(value, 'ĭ' | 'ŭ'))]
fn is_pronunciation_offglide(value: char) -> bool {
    matches!(value, 'ĭ' | 'ŭ')
}

#[requires(true)]
#[ensures(true)]
fn pronunciation_hard_onset(chars: &[PronunciationChar]) -> bool {
    match chars {
        [] => true,
        [value] => is_consonant(value.annotated),
        [first, second] => {
            initial_pair_chars(first.annotated, second.annotated)
                || (is_consonant(first.annotated) && is_pronunciation_onglide(second.annotated))
        }
        [first, second, third] => {
            (initial_pair_chars(first.annotated, second.annotated)
                && is_pronunciation_onglide(third.annotated))
                || is_sibilant(first.annotated)
                    && initial_pair_chars(first.annotated, second.annotated)
                    && initial_pair_chars(second.annotated, third.annotated)
                    && is_liquid(third.annotated)
        }
        [first, second, third, fourth] => {
            valid_three_consonant_initial(&[first.annotated, second.annotated, third.annotated], 0)
                && is_pronunciation_onglide(fourth.annotated)
        }
        _ => false,
    }
}

#[requires(chars.iter().all(|value| is_consonant(value.annotated)))]
#[ensures(ret.as_ref().is_none_or(|(_, syllables)| syllables.iter().all(|syllable| !syllable.is_empty())))]
fn parse_previous_pronunciation_coda(
    chars: &[PronunciationChar],
) -> Option<(Option<PronunciationChar>, Vec<String>)> {
    if chars.is_empty() {
        return Some((None, Vec::new()));
    }
    if (chars.len() - 1).is_multiple_of(2)
        && is_consonant(chars[0].annotated)
        && let Some(syllables) = pronunciation_consonantal_syllables(&chars[1..])
    {
        return Some((Some(chars[0]), syllables));
    }
    if chars.len().is_multiple_of(2)
        && let Some(syllables) = pronunciation_consonantal_syllables(chars)
    {
        return Some((None, syllables));
    }
    None
}

#[requires(chars.len().is_multiple_of(2))]
#[ensures(ret.as_ref().is_none_or(|syllables| syllables.iter().all(|syllable| !syllable.is_empty())))]
fn pronunciation_consonantal_syllables(chars: &[PronunciationChar]) -> Option<Vec<String>> {
    let mut syllables = Vec::new();
    for chunk in chars.chunks(2) {
        let [first, second] = chunk else {
            return None;
        };
        if is_consonant(first.annotated)
            && is_syllabic(second.annotated)
            && first.annotated != second.annotated
        {
            syllables.push(pronunciation_original_text(chunk));
        } else {
            return None;
        }
    }
    Some(syllables)
}

#[requires(chars.iter().all(|value| is_consonant(value.annotated) || is_pronunciation_onglide(value.annotated) || value.annotated == '\''))]
#[ensures(true)]
fn apply_pronunciation_coda(
    syllables: &mut Vec<String>,
    chars: &[PronunciationChar],
    next_consonant: Option<PronunciationChar>,
) -> Option<()> {
    if next_consonant.is_none()
        && chars
            .iter()
            .any(|value| is_pronunciation_onglide(value.annotated) || value.annotated == '\'')
    {
        return None;
    }
    let (coda, consonantal_syllables) = parse_previous_pronunciation_coda(chars)?;
    if let Some(coda) = coda {
        let next = consonantal_syllables
            .first()
            .and_then(|syllable| syllable.chars().next())
            .or(next_consonant.map(|value| value.annotated));
        if let Some(next) = next
            && !pronunciation_pair_permissible(coda.annotated, next)
        {
            return None;
        }
        let previous = syllables.last_mut()?;
        previous.push(coda.original);
    }
    syllables.extend(consonantal_syllables);
    Some(())
}

#[requires(!onset.is_empty())]
#[requires(nucleus.chars().count() <= 2)]
#[ensures(true)]
fn split_pronunciation_onset(
    syllables: &mut Vec<String>,
    onset: &[PronunciationChar],
    nucleus: &str,
) -> Option<()> {
    let (suffix_len, hard_onset) = (1..=onset.len().min(4)).rev().find_map(|suffix_len| {
        let hard_onset = &onset[onset.len() - suffix_len..];
        if !pronunciation_hard_onset(hard_onset) {
            return None;
        }
        let prefix = &onset[..onset.len() - suffix_len];
        let (coda, consonantal_syllables) = parse_previous_pronunciation_coda(prefix)?;
        if consonantal_syllables.is_empty()
            && let (Some(coda), Some(first)) = (coda, hard_onset.first())
            && !pronunciation_pair_permissible(coda.annotated, first.annotated)
        {
            return None;
        }
        if !consonantal_syllables.is_empty()
            && let (Some(previous), Some(first)) = (prefix.last(), hard_onset.first())
            && !pronunciation_pair_permissible(previous.annotated, first.annotated)
        {
            return None;
        }
        let triple_prefix = if consonantal_syllables.is_empty() {
            coda
        } else {
            prefix.last().copied()
        };
        if let Some(previous) = triple_prefix
            && hard_onset.len() >= 2
            && forbidden_consonant_triple_chars(
                previous.annotated,
                hard_onset[0].annotated,
                hard_onset[1].annotated,
            )
        {
            return None;
        }
        Some((suffix_len, hard_onset))
    })?;
    let prefix = &onset[..onset.len() - suffix_len];
    apply_pronunciation_coda(syllables, prefix, hard_onset.first().copied())?;
    let mut syllable = pronunciation_original_text(hard_onset);
    syllable.push_str(nucleus);
    syllables.push(syllable);
    Some(())
}

#[requires(true)]
#[ensures(ret == experimental_permissible_consonant_pair(first, second))]
fn pronunciation_pair_permissible(first: char, second: char) -> bool {
    experimental_permissible_consonant_pair(first, second)
}

#[requires(true)]
#[ensures(ret == matches!((first, second, third), ('n', 'd', 'j' | 'z') | ('n', 't', 'c' | 's')))]
fn forbidden_consonant_triple_chars(first: char, second: char, third: char) -> bool {
    matches!(
        (first, second, third),
        ('n', 'd', 'j' | 'z') | ('n', 't', 'c' | 's')
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(::Emit { .. } => true)]
#[invariant(::EmitText { .. } => true)]
#[invariant(::StressPrevious => true)]
#[invariant(::StressPreviousVowel => true)]
#[invariant(::Ignore => true)]
enum NormalizedCharEvent {
    Emit {
        value: char,
        implicit_apostrophe_vowel: bool,
    },
    EmitText {
        text: &'static str,
    },
    StressPrevious,
    StressPreviousVowel,
    Ignore,
}

#[requires(true)]
#[ensures(true)]
fn normalize_char_event(value: char, options: &MorphologyOptions) -> Option<NormalizedCharEvent> {
    if is_combining_stress_mark(value) {
        return Some(NormalizedCharEvent::StressPrevious);
    }
    if options.accept_zbalermorna && is_zbalermorna_stress_mark(value) {
        return Some(NormalizedCharEvent::StressPreviousVowel);
    }
    if options.accept_zbalermorna && is_zbalermorna_ignored_mark(value) {
        return Some(NormalizedCharEvent::Ignore);
    }
    if options.accept_latin && is_latin_apostrophe(value) {
        return Some(normalized_emit('\'', false));
    }
    if options.accept_cyrillic && is_cyrillic_apostrophe(value) {
        return Some(normalized_emit('\'', false));
    }
    if value.is_ascii_digit() {
        return Some(normalized_emit(value, false));
    }
    if options.accept_latin
        && let Some(normalized) = normalize_latin_char(value, options)
    {
        return Some(normalized_emit(normalized, false));
    }
    if options.accept_cyrillic
        && let Some((normalized, implicit_apostrophe_vowel)) =
            normalize_cyrillic_char(value, options)
    {
        return Some(normalized_emit(normalized, implicit_apostrophe_vowel));
    }
    if options.accept_zbalermorna
        && let Some(normalized) = normalize_zbalermorna_text(value)
    {
        return Some(NormalizedCharEvent::EmitText { text: normalized });
    }
    None
}

#[requires(is_valid_normalized_char(value))]
#[ensures(matches!(ret, NormalizedCharEvent::Emit { value: emitted, implicit_apostrophe_vowel } if emitted == value && implicit_apostrophe_vowel == insert_implicit_apostrophe))]
fn normalized_emit(value: char, insert_implicit_apostrophe: bool) -> NormalizedCharEvent {
    NormalizedCharEvent::Emit {
        value,
        implicit_apostrophe_vowel: insert_implicit_apostrophe,
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_latin_char(value: char, options: &MorphologyOptions) -> Option<char> {
    let normalized = match value {
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
    is_valid_normalized_char(normalized).then_some(normalized)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(value, _)| is_valid_normalized_char(*value)))]
fn normalize_cyrillic_char(value: char, options: &MorphologyOptions) -> Option<(char, bool)> {
    match value {
        'а' => Some(('a', true)),
        'А' => Some((stressable_uppercase_vowel('a', options), true)),
        'е' | 'э' | 'є' => Some(('e', true)),
        'Е' | 'Э' | 'Є' => Some((stressable_uppercase_vowel('e', options), true)),
        'и' | 'і' => Some(('i', true)),
        'И' | 'І' => Some((stressable_uppercase_vowel('i', options), true)),
        'о' => Some(('o', true)),
        'О' => Some((stressable_uppercase_vowel('o', options), true)),
        'у' => Some(('u', true)),
        'У' => Some((stressable_uppercase_vowel('u', options), true)),
        'ъ' | 'ы' | 'ә' => Some(('y', true)),
        'Ъ' | 'Ы' | 'Ә' => Some((stressable_uppercase_vowel('y', options), true)),
        'й' | 'Й' | 'ј' | 'Ј' => Some(('ĭ', false)),
        'ў' | 'Ў' => Some(('ŭ', false)),
        'б' | 'Б' => Some(('b', false)),
        'ш' | 'Ш' | 'щ' | 'Щ' => Some(('c', false)),
        'д' | 'Д' => Some(('d', false)),
        'ф' | 'Ф' => Some(('f', false)),
        'г' | 'Г' | 'ґ' | 'Ґ' => Some(('g', false)),
        'ж' | 'Ж' => Some(('j', false)),
        'к' | 'К' => Some(('k', false)),
        'л' | 'Л' => Some(('l', false)),
        'м' | 'М' => Some(('m', false)),
        'н' | 'Н' => Some(('n', false)),
        'п' | 'П' => Some(('p', false)),
        'р' | 'Р' => Some(('r', false)),
        'с' | 'С' => Some(('s', false)),
        'т' | 'Т' => Some(('t', false)),
        'в' | 'В' => Some(('v', false)),
        'х' | 'Х' => Some(('x', false)),
        'з' | 'З' => Some(('z', false)),
        ',' => Some((',', false)),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| text.chars().all(is_valid_normalized_char)))]
fn normalize_zbalermorna_text(value: char) -> Option<&'static str> {
    match value {
        '\u{ed80}' => Some("p"),
        '\u{ed81}' => Some("t"),
        '\u{ed82}' => Some("k"),
        '\u{ed83}' => Some("f"),
        '\u{ed84}' => Some("l"),
        '\u{ed85}' => Some("s"),
        '\u{ed86}' => Some("c"),
        '\u{ed87}' => Some("m"),
        '\u{ed88}' => Some("x"),
        '\u{ed8a}' => Some("'"),
        '\u{ed90}' => Some("b"),
        '\u{ed91}' => Some("d"),
        '\u{ed92}' => Some("g"),
        '\u{ed93}' => Some("v"),
        '\u{ed94}' => Some("r"),
        '\u{ed95}' => Some("z"),
        '\u{ed96}' => Some("j"),
        '\u{ed97}' => Some("n"),
        '\u{ed9a}' => Some(","),
        '\u{eda0}' | '\u{edb0}' => Some("a"),
        '\u{eda1}' | '\u{edb1}' => Some("e"),
        '\u{eda2}' | '\u{edb2}' => Some("i"),
        '\u{eda3}' | '\u{edb3}' => Some("o"),
        '\u{eda4}' | '\u{edb4}' => Some("u"),
        '\u{eda5}' | '\u{edb5}' => Some("y"),
        '\u{eda6}' => Some("aĭ"),
        '\u{eda7}' => Some("eĭ"),
        '\u{eda8}' => Some("oĭ"),
        '\u{eda9}' => Some("aŭ"),
        '\u{edaa}' => Some("ĭ"),
        '\u{edab}' => Some("ŭ"),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| text.chars().all(is_valid_normalized_char)))]
fn normalize_zbalermorna_shorthand_payload(value: char) -> Option<&'static str> {
    match value {
        '\u{eda0}'..='\u{eda9}' | '\u{edb0}'..='\u{edb5}' => normalize_zbalermorna_text(value),
        _ => None,
    }
}

#[requires(matches!(value, 'a' | 'e' | 'i' | 'o' | 'u' | 'y'))]
#[ensures(true)]
fn stressable_uppercase_vowel(value: char, options: &MorphologyOptions) -> char {
    if options.uppercase_marks_stress {
        stress_vowel(value).unwrap_or(value)
    } else {
        value
    }
}

#[requires(true)]
#[ensures(true)]
fn is_latin_apostrophe(value: char) -> bool {
    matches!(
        value,
        '\'' | 'h'
            | 'H'
            | '\u{2019}'
            | '\u{a78b}'
            | '\u{a78c}'
            | '\u{02bb}'
            | '\u{02bf}'
            | '\u{02b0}'
            | '\u{02d2}'
    )
}

#[requires(true)]
#[ensures(true)]
fn is_cyrillic_apostrophe(value: char) -> bool {
    matches!(value, 'һ' | 'Һ')
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_cyrillic_period(value: char) -> bool {
    matches!(value, 'ӏ' | 'Ӏ')
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_zbalermorna_period(value: char) -> bool {
    matches!(value, '\u{ed89}')
}

#[requires(true)]
#[ensures(ret == (value == '\u{ed8b}'))]
fn is_zbalermorna_attitudinal_shorthand(value: char) -> bool {
    value == '\u{ed8b}'
}

#[requires(true)]
#[ensures(true)]
fn is_combining_stress_mark(value: char) -> bool {
    matches!(value, '\u{0301}' | '\u{0300}')
}

#[requires(true)]
#[ensures(true)]
fn is_zbalermorna_stress_mark(value: char) -> bool {
    matches!(value, '\u{ed98}')
}

#[requires(true)]
#[ensures(true)]
fn is_zbalermorna_ignored_mark(value: char) -> bool {
    matches!(value, '\u{ed8c}' | '\u{ed99}' | '\u{ed9b}')
}

#[requires(true)]
#[ensures(true)]
fn stress_last_normalized_char(chars: &mut [NormalizedSourceChar]) {
    if let Some(last) = chars.last_mut()
        && let Some(stressed) = stress_vowel(last.value)
    {
        *last = new!(NormalizedSourceChar {
            source_start: last.source_start,
            source_end: last.source_end,
            source_value: last.source_value,
            value: stressed,
        });
    }
}

#[requires(true)]
#[ensures(true)]
fn stress_last_normalized_vowel_char(chars: &mut [NormalizedSourceChar]) {
    if let Some(last) = chars
        .iter_mut()
        .rev()
        .find(|source_char| stress_vowel(source_char.value).is_some())
        && let Some(stressed) = stress_vowel(last.value)
    {
        *last = new!(NormalizedSourceChar {
            source_start: last.source_start,
            source_end: last.source_end,
            source_value: last.source_value,
            value: stressed,
        });
    }
}

#[requires(true)]
#[ensures(true)]
fn stress_vowel(value: char) -> Option<char> {
    match value {
        'a' | 'á' => Some('á'),
        'e' | 'é' => Some('é'),
        'i' | 'í' => Some('í'),
        'o' | 'ó' => Some('ó'),
        'u' | 'ú' => Some('ú'),
        'y' | 'ý' => Some('ý'),
        _ => None,
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
    if !valid_initial_shape(chars, start, end) {
        return None;
    }
    if end < chars.len() && (is_consonant(chars[end]) || parse_glide(chars, end).is_some()) {
        return None;
    }
    Some(initial)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn valid_initial_shape(chars: &[char], start: usize, end: usize) -> bool {
    match end - start {
        0 => true,
        1 => is_consonant(chars[start]),
        2 => initial_pair_chars(chars[start], chars[start + 1]),
        3 => valid_three_consonant_initial(chars, start),
        _ => false,
    }
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
    if !is_vowel(value) && !matches!(value, 'ĭ' | 'ŭ') {
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
                || matches!(*value, 'ĭ' | 'ŭ')
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
            && !permissible_consonant_pair(*value, chars[next]))
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
        'ĭ' => 'i',
        'ŭ' => 'u',
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
fn latin_breve_not_glide_range(chars: &[NormalizedSourceChar]) -> Option<Range<usize>> {
    let values = normalized_values(chars);
    breve_not_glide_range(&values).filter(|range| {
        chars
            .get(range.start)
            .is_some_and(|source| is_latin_breve_source(source.source_value))
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start < range.end && range.end <= chars.len()))]
fn hard_breve_not_glide_range(chars: &[NormalizedSourceChar]) -> Option<Range<usize>> {
    let values = normalized_values(chars);
    breve_not_glide_range(&values).filter(|range| {
        chars
            .get(range.start)
            .is_some_and(|source| !is_latin_breve_source(source.source_value))
    })
}

#[requires(true)]
#[ensures(ret == matches!(value, 'ĭ' | 'Ĭ' | 'ŭ' | 'Ŭ'))]
fn is_latin_breve_source(value: char) -> bool {
    matches!(value, 'ĭ' | 'Ĭ' | 'ŭ' | 'Ŭ')
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
                && experimental_permissible_consonant_pair(*c, *d))
                || (initial_pair_chars(*a, *b) && is_vowel(*c) && is_consonant(*d) && is_vowel(*e))
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_lujvo(word: &str) -> bool {
    let chars = text_chars(word);
    !cmavo_word_slice(&chars, 0, chars.len())
        && !is_fuhivla_shape_slice(&chars, 0, chars.len())
        && parse_lujvo_parts(word).is_some()
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn lujvo_from(chars: &[char], index: usize, has_initial_rafsi: bool) -> bool {
    if index >= chars.len() {
        return false;
    }
    if has_initial_rafsi && lujvo_core_part_ranges(chars, index).is_some() {
        return true;
    }
    if !has_initial_rafsi
        && lujvo_core_part_ranges(chars, index).is_some_and(|ranges| ranges.len() > 1)
    {
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
#[ensures(ret.as_ref().is_none_or(|ranges| !ranges.is_empty()))]
#[ensures(ret.as_ref().is_none_or(|ranges| ranges.iter().all(|range| range.start < range.end && range.end <= chars.len())))]
fn lujvo_part_ranges_from(
    chars: &[char],
    index: usize,
    has_initial_rafsi: bool,
    failure: &mut Option<LujvoParseFailure>,
) -> Option<Vec<Range<usize>>> {
    if index >= chars.len() {
        record_lujvo_failure(failure, index, has_initial_rafsi);
        return None;
    }
    if has_initial_rafsi && let Some(ranges) = lujvo_core_part_ranges(chars, index) {
        return Some(ranges);
    }
    if !has_initial_rafsi
        && let Some(ranges) = lujvo_core_part_ranges(chars, index)
        && ranges.len() > 1
    {
        return Some(ranges);
    }
    if !has_initial_rafsi && is_lujvo_final_rafsi_alone(chars, index) {
        return Some(vec![index..chars.len()]);
    }
    for end in initial_rafsi_ends(chars, index) {
        if end <= index {
            continue;
        }
        let Some(mut rest) = lujvo_part_ranges_from(chars, end, true, failure) else {
            continue;
        };
        let mut ranges = initial_rafsi_ranges(chars, index, end)?;
        ranges.append(&mut rest);
        return Some(ranges);
    }
    record_lujvo_failure(failure, index, has_initial_rafsi);
    None
}

#[requires(!ranges.is_empty())]
#[requires(ranges.iter().all(|range| range.start < range.end && range.end <= chars.len()))]
#[ensures(ret.as_ref().is_none_or(|parts| !parts.is_empty()))]
fn lujvo_ranges_to_parts(chars: &[char], ranges: Vec<Range<usize>>) -> Option<Vec<LujvoPart>> {
    ranges
        .into_iter()
        .map(|range| {
            if is_rafsi_hyphen_start(chars, range.start) {
                hyphen_part(chars, range.start, range.end)
            } else {
                rafsi_part(chars, range.start, range.end)
            }
        })
        .collect()
}

#[requires(start < end && end <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|ranges| !ranges.is_empty()))]
#[ensures(ret.as_ref().is_none_or(|ranges| ranges.iter().all(|range| range.start < range.end && range.end <= end)))]
fn initial_rafsi_ranges(chars: &[char], start: usize, end: usize) -> Option<Vec<Range<usize>>> {
    if let Some(hyphen_start) = (start + 1..end).find(|index| is_rafsi_hyphen_start(chars, *index))
    {
        return Some(vec![start..hyphen_start, hyphen_start..end]);
    }
    Some(vec![start..end])
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
    lujvo_core_part_ranges(chars, index).is_some()
}

#[requires(index <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|ranges| !ranges.is_empty()))]
#[ensures(ret.as_ref().is_none_or(|ranges| ranges.iter().all(|range| range.start < range.end && range.end <= chars.len())))]
fn lujvo_core_part_ranges(chars: &[char], index: usize) -> Option<Vec<Range<usize>>> {
    if is_gismu_slice(chars, index, chars.len())
        || is_short_final_rafsi_slice(chars, index, chars.len())
        || is_cvv_final_rafsi_slice(chars, index, chars.len())
        || is_fuhivla_shape_slice(chars, index, chars.len())
    {
        return Some(vec![index..chars.len()]);
    }
    let split = stressed_initial_rafsi_short_final_split(chars, index, chars.len())?;
    let mut ranges = initial_rafsi_ranges(chars, index, split)?;
    ranges.push(split..chars.len());
    Some(ranges)
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
    let extended = extended_rafsi_ends(chars, index);
    ends.extend(extended.iter().copied());
    ends.extend(y_rafsi_ends(chars, index));
    if !any_extended_rafsi_starts(chars, index) {
        ends.extend(
            y_less_rafsi_ends(chars, index)
                .into_iter()
                .filter(|end| !any_extended_rafsi_starts(chars, *end)),
        );
    }

    let mut preferred_ends = Vec::new();
    for end in ends {
        if !preferred_ends.contains(&end) {
            preferred_ends.push(end);
        }
    }
    preferred_ends
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn any_extended_rafsi_starts(chars: &[char], index: usize) -> bool {
    index < chars.len()
        && (is_fuhivla_shape_slice(chars, index, chars.len())
            || !extended_rafsi_ends(chars, index).is_empty()
            || !stressed_extended_rafsi_ends(chars, index).is_empty())
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn stressed_extended_rafsi_ends(chars: &[char], index: usize) -> Vec<usize> {
    ((index + 1)..=chars.len())
        .filter(|end| stressed_extended_rafsi_slice(chars, index, *end))
        .collect()
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn extended_rafsi_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    for base_end in fuhivla_rafsi_base_ends(chars, index) {
        if let Some(end) = rafsi_hyphen_end(chars, base_end) {
            ends.push(end);
        }
    }
    for base_end in brivla_head_ends(chars, index) {
        if chars[index..base_end].iter().any(|value| is_y(*value)) {
            continue;
        }
        if let Some(end) = hy_rafsi_hyphen_end(chars, base_end) {
            ends.push(end);
        }
    }
    ends
}

#[requires(start <= end && end <= chars.len())]
#[ensures(ret.is_none_or(|split| split > start && split < end))]
fn stressed_initial_rafsi_short_final_split(
    chars: &[char],
    start: usize,
    end: usize,
) -> Option<usize> {
    (start + 1..end).rev().find(|split| {
        is_short_final_rafsi_slice(chars, *split, end)
            && stressed_initial_rafsi_slice(chars, start, *split)
    })
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn stressed_initial_rafsi_slice(chars: &[char], start: usize, end: usize) -> bool {
    stressed_extended_rafsi_slice(chars, start, end)
        || stressed_y_rafsi_slice(chars, start, end)
        || stressed_y_less_rafsi_slice(chars, start, end)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn stressed_extended_rafsi_slice(chars: &[char], start: usize, end: usize) -> bool {
    stressed_brivla_rafsi_slice(chars, start, end)
        || stressed_fuhivla_rafsi_slice(chars, start, end)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn stressed_y_rafsi_slice(chars: &[char], start: usize, end: usize) -> bool {
    y_rafsi_slice(chars, start, end)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn stressed_y_less_rafsi_slice(chars: &[char], start: usize, end: usize) -> bool {
    y_less_rafsi_ends(chars, start)
        .into_iter()
        .any(|rafsi_end| rafsi_end == end)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn stressed_brivla_rafsi_slice(chars: &[char], start: usize, end: usize) -> bool {
    if end < start + 3
        || chars.get(end - 2) != Some(&'\'')
        || !chars.get(end - 1).is_some_and(|value| is_y(*value))
    {
        return false;
    }
    let stress_end = end - 2;
    brivla_head_ends_until(chars, start, stress_end)
        .into_iter()
        .filter(|head_end| *head_end < stress_end)
        .any(|head_end| {
            stressed_syllable_ends_for_fuhivla(chars, head_end, stress_end)
                .into_iter()
                .any(|syllable_end| syllable_end == stress_end)
        })
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn stressed_fuhivla_rafsi_slice(chars: &[char], start: usize, end: usize) -> bool {
    if end < start + 2 || !chars.get(end - 1).is_some_and(|value| is_y(*value)) {
        return false;
    }
    let base_end = end - 1;
    fuhivla_head_ends_until(chars, start, base_end)
        .into_iter()
        .filter(|head_end| *head_end < base_end)
        .any(|head_end| {
            stressed_syllable_ends_for_fuhivla(chars, head_end, base_end)
                .into_iter()
                .any(|stressed_end| consonantal_chain_then_onset(chars, stressed_end, base_end))
        })
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn consonantal_chain_then_onset(chars: &[char], index: usize, end: usize) -> bool {
    if chars.get(index) != Some(&'\'')
        && parse_onsets(chars, index)
            .into_iter()
            .any(|(_, onset_end)| onset_end == end)
    {
        return true;
    }
    consonantal_syllable_ends(chars, index, end, SyllablePolicy::Brivla)
        .into_iter()
        .any(|next| next > index && consonantal_chain_then_onset(chars, next, end))
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end < chars.len()))]
fn fuhivla_rafsi_base_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    for base_end in (index + 1)..chars.len() {
        if rafsi_hyphen_end(chars, base_end).is_some()
            && fuhivla_rafsi_base_slice(chars, index, base_end, chars.len())
        {
            ends.push(base_end);
        }
    }
    ends
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn fuhivla_rafsi_base_slice(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
) -> bool {
    if start >= end
        || chars[start..end].iter().any(|value| is_y(*value))
        || unstressed_syllable_ends_for_fuhivla(chars, start, end).is_empty()
    {
        return false;
    }
    fuhivla_head_ends_until(chars, start, lookahead_end)
        .into_iter()
        .filter(|head_end| *head_end > start && *head_end < end)
        .any(|head_end| {
            chars.get(head_end) != Some(&'\'')
                && parse_onsets(chars, head_end)
                    .into_iter()
                    .any(|(_, onset_end)| onset_end == end)
        })
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end >= index && *end <= chars.len()))]
fn brivla_head_ends(chars: &[char], index: usize) -> Vec<usize> {
    brivla_head_ends_until(chars, index, chars.len())
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.iter().all(|head_end| *head_end >= index && *head_end <= end))]
fn brivla_head_ends_until(chars: &[char], index: usize, end: usize) -> Vec<usize> {
    if index > end
        || chars.get(index) == Some(&'\'')
        || !starts_with_onset(chars, index)
        || cmavo_word_slice(chars, index, end)
        || slinkuhi_slice(chars, index, end)
    {
        return Vec::new();
    }

    brivla_head_ends_without_lookahead(chars, index, end)
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.iter().all(|head_end| *head_end >= index && *head_end <= end))]
fn fuhivla_head_ends_until(chars: &[char], index: usize, end: usize) -> Vec<usize> {
    if rafsi_string_starts_slice(chars, index, end) {
        return Vec::new();
    }
    brivla_head_ends_until(chars, index, end)
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.iter().all(|head_end| *head_end >= index && *head_end <= end))]
fn brivla_head_ends_without_lookahead(chars: &[char], index: usize, end: usize) -> Vec<usize> {
    let mut ends = vec![index];
    let mut stack = vec![index];
    while let Some(start) = stack.pop() {
        for syllable_end in unstressed_syllable_ends_for_fuhivla(chars, start, end) {
            if syllable_end > start && !ends.contains(&syllable_end) {
                ends.push(syllable_end);
                stack.push(syllable_end);
            }
        }
    }
    ends.sort_unstable();
    ends
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn slinkuhi_slice(chars: &[char], start: usize, end: usize) -> bool {
    start < end
        && !rafsi_string_starts_slice(chars, start, end)
        && is_consonant(chars[start])
        && rafsi_string_starts_slice(chars, start + 1, end)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn rafsi_string_starts_slice(chars: &[char], start: usize, end: usize) -> bool {
    start < end
        && ((start + 1)..=end)
            .any(|word_end| rafsi_string_slice_for_lookahead(chars, start, word_end, end))
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn rafsi_string_slice(chars: &[char], start: usize, end: usize) -> bool {
    rafsi_string_slice_for_lookahead(chars, start, end, end)
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn rafsi_string_slice_for_lookahead(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
) -> bool {
    rafsi_string_from(chars, start, end, lookahead_end)
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn rafsi_string_from(chars: &[char], start: usize, end: usize, lookahead_end: usize) -> bool {
    if start >= end {
        return false;
    }
    let mut cursor = start;
    while cursor < end {
        let Some(next) = greedy_y_less_rafsi_end(chars, cursor, end, lookahead_end) else {
            break;
        };
        cursor = next;
    }
    rafsi_string_ending(chars, cursor, end, lookahead_end)
}

#[requires(index <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(ret.is_none_or(|next| next > index && next <= end))]
fn greedy_y_less_rafsi_end(
    chars: &[char],
    index: usize,
    end: usize,
    lookahead_end: usize,
) -> Option<usize> {
    y_less_rafsi_ends(chars, index).into_iter().find(|next| {
        *next <= end && y_less_rafsi_is_unstressed_in_context(chars, index, *next, lookahead_end)
    })
}

#[requires(start <= end && end <= context_end && context_end <= chars.len())]
#[ensures(true)]
fn y_less_rafsi_is_unstressed_in_context(
    chars: &[char],
    start: usize,
    end: usize,
    context_end: usize,
) -> bool {
    if cvc_rafsi_end(chars, start) == Some(end) {
        return !nucleus_is_stressed_in_context(chars, start + 1, start + 2, context_end);
    }
    if ccv_rafsi_end(chars, start) == Some(end) {
        return !nucleus_is_stressed_in_context(chars, start + 2, start + 3, context_end);
    }
    cvv_rafsi_is_unstressed_in_context(chars, start, end, context_end)
}

#[requires(start <= end && end <= context_end && context_end <= chars.len())]
#[ensures(true)]
fn cvv_rafsi_is_unstressed_in_context(
    chars: &[char],
    start: usize,
    end: usize,
    context_end: usize,
) -> bool {
    if start >= end || !is_consonant(chars[start]) {
        return false;
    }
    for vowel_end in vowel_pair_ends(chars, start + 1) {
        let rafsi_end = if end == vowel_end {
            Some(vowel_end)
        } else if r_hyphen_end(chars, vowel_end) == Some(end) {
            Some(end)
        } else if n_hyphen_end(chars, vowel_end) == Some(end) {
            Some(end)
        } else {
            None
        };
        if rafsi_end.is_none() {
            continue;
        }
        if chars.get(start + 2) == Some(&'\'') && vowel_end == start + 4 {
            return !nucleus_is_stressed_in_context(chars, start + 1, start + 2, context_end)
                && !nucleus_is_stressed_in_context(chars, start + 3, start + 4, context_end);
        }
        if vowel_end == start + 3 {
            return !nucleus_is_stressed_in_context(chars, start + 1, vowel_end, context_end);
        }
    }
    false
}

#[requires(start < end && end <= context_end && context_end <= chars.len())]
#[ensures(true)]
fn nucleus_is_stressed_in_context(
    chars: &[char],
    start: usize,
    end: usize,
    context_end: usize,
) -> bool {
    syllable_has_explicit_stress(chars, start, end)
        || nucleus_has_implicit_stress(chars, end, context_end)
}

#[requires(index <= context_end && context_end <= chars.len())]
#[ensures(true)]
fn nucleus_has_implicit_stress(chars: &[char], index: usize, context_end: usize) -> bool {
    let mut stack = vec![index];
    let mut after_consonants_or_glides = Vec::new();
    while let Some(cursor) = stack.pop() {
        if after_consonants_or_glides.contains(&cursor) {
            continue;
        }
        after_consonants_or_glides.push(cursor);
        if cursor < context_end && is_consonant(chars[cursor]) {
            stack.push(cursor + 1);
        }
        if let Some((_, glide_end)) = parse_glide(chars, cursor)
            && glide_end <= context_end
        {
            stack.push(glide_end);
        }
    }

    after_consonants_or_glides
        .into_iter()
        .any(|cursor| stress_tail_reaches_boundary(chars, cursor, context_end))
}

#[requires(index <= context_end && context_end <= chars.len())]
#[ensures(true)]
fn stress_tail_reaches_boundary(chars: &[char], index: usize, context_end: usize) -> bool {
    let mut after_h = vec![index];
    if index < context_end && chars.get(index) == Some(&'\'') {
        after_h.push(index + 1);
    }
    after_h.into_iter().any(|cursor| {
        let mut after_y = vec![cursor];
        if cursor < context_end && chars.get(cursor).is_some_and(|value| is_y(*value)) {
            after_y.push(cursor + 1);
        }
        after_y.into_iter().any(|syllable_start| {
            syllable_ends(chars, syllable_start, context_end, SyllablePolicy::Brivla)
                .into_iter()
                .any(|syllable_end| syllable_end == context_end)
        })
    })
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn rafsi_string_ending(chars: &[char], start: usize, end: usize, lookahead_end: usize) -> bool {
    (is_gismu_slice_for_rafsi_string(chars, start, end, lookahead_end)
        && post_word_slice(chars, end, lookahead_end))
        || (is_cvv_final_rafsi_slice_for_rafsi_string(chars, start, end, lookahead_end)
            && post_word_slice(chars, end, lookahead_end))
        || y_less_rafsi_ends(chars, start).into_iter().any(|mid| {
            mid > start
                && mid < end
                && is_short_final_rafsi_slice_for_rafsi_string(chars, mid, end, lookahead_end)
                && post_word_slice(chars, end, lookahead_end)
        })
        || y_rafsi_slice(chars, start, end)
        || hy_rafsi_slice(chars, start, end)
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn is_gismu_slice_for_rafsi_string(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
) -> bool {
    if end != start + 5 {
        return false;
    }
    let final_start = if initial_pair_chars(chars[start], chars[start + 1])
        && is_vowel(chars[start + 2])
        && is_consonant(chars[start + 3])
        && is_vowel(chars[start + 4])
    {
        start + 3
    } else if is_consonant(chars[start])
        && is_vowel(chars[start + 1])
        && is_consonant(chars[start + 2])
        && is_consonant(chars[start + 3])
        && is_vowel(chars[start + 4])
        && experimental_permissible_consonant_pair(chars[start + 2], chars[start + 3])
    {
        start + 3
    } else {
        return false;
    };
    syllable_is_stressed_in_context(chars, start, final_start, lookahead_end)
        && final_syllable_slice_in_context(chars, final_start, end, lookahead_end)
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn is_cvv_final_rafsi_slice_for_rafsi_string(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
) -> bool {
    end == start + 4
        && is_consonant(chars[start])
        && is_vowel(chars[start + 1])
        && chars[start + 2] == '\''
        && is_vowel(chars[start + 3])
        && syllable_is_stressed_in_context(chars, start, start + 2, lookahead_end)
        && final_syllable_slice_in_context(chars, start + 3, end, lookahead_end)
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn is_short_final_rafsi_slice_for_rafsi_string(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
) -> bool {
    is_short_final_rafsi_slice(chars, start, end)
        && final_syllable_slice_in_context(chars, start, end, lookahead_end)
}

#[requires(start <= syllable_end && syllable_end <= context_end && context_end <= chars.len())]
#[ensures(true)]
fn syllable_is_stressed_in_context(
    chars: &[char],
    start: usize,
    syllable_end: usize,
    context_end: usize,
) -> bool {
    syllable_has_explicit_stress(chars, start, syllable_end)
        || stressed_syllable_has_implicit_stress(chars, start, syllable_end, context_end)
}

#[requires(start <= end && end <= context_end && context_end <= chars.len())]
#[ensures(true)]
fn final_syllable_slice_in_context(
    chars: &[char],
    start: usize,
    end: usize,
    context_end: usize,
) -> bool {
    final_syllable_slice(chars, start, end)
        && !stressed_syllable_has_implicit_stress(chars, start, end, context_end)
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn post_word_slice(chars: &[char], index: usize, end: usize) -> bool {
    let Some(next) = next_non_comma_index(chars, index) else {
        return true;
    };
    next >= end
        || (!starts_with_pause_required_nucleus(chars, next) && lojban_word_slice(chars, next, end))
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
    if !y_rafsi_ends(chars, index).is_empty() || !hy_rafsi_ends(chars, index).is_empty() {
        return Vec::new();
    }

    let mut ends = Vec::new();
    if let Some(end) = cvc_rafsi_end(chars, index) {
        ends.push(end);
    }
    if let Some(end) = ccv_rafsi_end(chars, index) {
        ends.push(end);
    }
    ends.extend(cvv_rafsi_ends(chars, index));
    ends.retain(|end| chars.get(*end) != Some(&'\''));
    ends
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn hy_rafsi_slice(chars: &[char], start: usize, end: usize) -> bool {
    hy_rafsi_ends(chars, start)
        .into_iter()
        .any(|hy_rafsi_end| hy_rafsi_end == end)
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn hy_rafsi_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    for base_end in long_rafsi_ends(chars, index) {
        if chars.get(base_end).is_some_and(|value| is_vowel(*value))
            && let Some(end) = hy_rafsi_hyphen_end(chars, base_end + 1)
        {
            ends.push(end);
        }
    }
    for base_end in ccv_rafsi_end(chars, index)
        .into_iter()
        .chain(cvv_rafsi_ends(chars, index))
    {
        if let Some(end) = hy_rafsi_hyphen_end(chars, base_end) {
            ends.push(end);
        }
    }
    ends
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
        && initial_pair_chars(chars[index], chars[index + 1])
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
        && initial_pair_chars(chars[index], chars[index + 1])
        && is_vowel(chars[index + 2]))
    .then_some(index + 3)
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn cvv_rafsi_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    if index < chars.len() && is_consonant(chars[index]) {
        for vowel_end in vowel_pair_ends(chars, index + 1) {
            if let Some(end) =
                r_hyphen_end(chars, vowel_end).or_else(|| n_hyphen_end(chars, vowel_end))
            {
                ends.push(end);
            }
            ends.push(vowel_end);
        }
    }
    ends
}

#[requires(index <= chars.len())]
#[ensures(ret.is_none_or(|end| end == index + 1 && end <= chars.len()))]
fn r_hyphen_end(chars: &[char], index: usize) -> Option<usize> {
    (chars.get(index) == Some(&'r')
        && chars
            .get(index + 1)
            .is_some_and(|value| is_consonant(*value)))
    .then_some(index + 1)
}

#[requires(index <= chars.len())]
#[ensures(ret.is_none_or(|end| end == index + 1 && end <= chars.len()))]
fn n_hyphen_end(chars: &[char], index: usize) -> Option<usize> {
    (chars.get(index) == Some(&'n') && chars.get(index + 1) == Some(&'r')).then_some(index + 1)
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
        && initial_pair_chars(chars[start], chars[start + 1])
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
    is_fuhivla_shape_slice_with_y_policy(chars, start, end, false)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn fuhivla_shape_slice_rejected_by_y(chars: &[char], start: usize, end: usize) -> bool {
    start < end
        && chars[start..end].iter().any(|value| is_y(*value))
        && is_fuhivla_shape_slice_with_y_policy(chars, start, end, true)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn is_fuhivla_shape_slice_with_y_policy(
    chars: &[char],
    start: usize,
    end: usize,
    allows_y: bool,
) -> bool {
    if end <= start
        || end - start < 4
        || !ends_with_vocalic_nucleus(chars, start, end)
        || count_vocalic_nuclei(chars, start, end) < 2
        || (!allows_y && chars[start..end].iter().any(|value| is_y(*value)))
        || has_vowel_hiatus(&chars[start..end])
        || !parse_fuhivla_shape(chars, start, end)
    {
        return false;
    }
    if rafsi_string_slice(chars, start, end)
        || slinkuhi_slice(chars, start, end)
        || has_blocking_cmavo_prefix_slice(chars, start, end)
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
fn has_blocking_cmavo_prefix_slice(chars: &[char], start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }
    cmavo_word_slice(chars, start, end)
}

#[requires(prefix_start <= prefix_end && prefix_end <= end && end <= chars.len())]
#[ensures(true)]
fn cmavo_post_word_slice(
    chars: &[char],
    prefix_start: usize,
    prefix_end: usize,
    end: usize,
) -> bool {
    let Some(index) = next_non_comma_index(chars, prefix_end) else {
        return true;
    };
    if index >= end {
        return true;
    }
    if starts_with_pause_required_nucleus(chars, index)
        && !indicator_cmavo_boundary_slice(chars, prefix_start, prefix_end, index, end)
    {
        return false;
    }
    lojban_word_slice(chars, index, end)
}

#[requires(prefix_start <= prefix_end && prefix_end <= index && index <= end && end <= chars.len())]
#[ensures(true)]
fn indicator_cmavo_boundary_slice(
    chars: &[char],
    prefix_start: usize,
    prefix_end: usize,
    index: usize,
    end: usize,
) -> bool {
    is_indicator_cmavo_slice(chars, prefix_start, prefix_end)
        && !starts_with_nucleus(chars, index)
        && indicator_cmavo_starts_slice(chars, index, end)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn indicator_cmavo_starts_slice(chars: &[char], start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }
    ((start + 1)..=end).any(|word_end| {
        is_indicator_cmavo_slice(chars, start, word_end)
            && cmavo_post_word_slice(chars, start, word_end, end)
    })
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn is_indicator_cmavo_slice(chars: &[char], start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }
    parse_cmavo_form(&chars[start..end].iter().collect::<String>())
        .and_then(|phonemes| Cmavo::from_text(&phonemes))
        .is_some_and(|cmavo| cmavo.is_selmaho(Selmaho::Ui) || cmavo.is_selmaho(Selmaho::Cai))
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn lojban_word_slice(chars: &[char], start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }
    is_cmevla_slice(chars, start, end)
        || cmavo_word_slice(chars, start, end)
        || brivla_word_slice(chars, start, end)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn cmavo_word_slice(chars: &[char], start: usize, end: usize) -> bool {
    if start >= end
        || is_cmevla_slice(chars, start, end)
        || starts_with_cvcy_lujvo_chars(chars, start)
    {
        return false;
    }
    ((start + 1)..=end).any(|word_end| {
        is_cmavo_slice(chars, start, word_end) && cmavo_post_word_slice(chars, start, word_end, end)
    })
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn brivla_word_slice(chars: &[char], start: usize, end: usize) -> bool {
    !cmavo_word_slice(chars, start, end)
        && (is_gismu_slice(chars, start, end)
            || is_lujvo_slice(chars, start, end)
            || is_fuhivla_shape_slice(chars, start, end))
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn is_lujvo_slice(chars: &[char], start: usize, end: usize) -> bool {
    let slice = &chars[start..end];
    slice.iter().all(|value| is_lujvo_char(*value))
        && !cmavo_word_slice(chars, start, end)
        && !is_fuhivla_shape_slice(chars, start, end)
        && lujvo_from(slice, 0, false)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn is_cmevla_slice(chars: &[char], start: usize, end: usize) -> bool {
    start < end
        && chars[end - 1].is_ascii_lowercase()
        && is_consonant(chars[end - 1])
        && !blocks_word_shape(&chars[start..end])
}

#[requires(start <= chars.len())]
#[ensures(true)]
fn starts_with_pause_required_nucleus(chars: &[char], start: usize) -> bool {
    let Some(start) = next_non_comma_index(chars, start) else {
        return false;
    };
    starts_with_nucleus(chars, start)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn ends_with_vocalic_nucleus(chars: &[char], start: usize, end: usize) -> bool {
    start < end
        && chars
            .get(end - 1)
            .is_some_and(|value| is_vowel(*value) || matches!(*value, 'ĭ' | 'ŭ'))
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn count_vocalic_nuclei(chars: &[char], start: usize, end: usize) -> usize {
    let mut count = 0;
    let mut index = start;
    while index < end {
        if let Some((_, nucleus_end)) = parse_diphthong(chars, index)
            && nucleus_end <= end
        {
            count += 1;
            index = nucleus_end;
        } else if let Some((_, nucleus_end)) = parse_single_vowel(chars, index)
            && nucleus_end <= end
        {
            count += 1;
            index = nucleus_end;
        } else {
            index += 1;
        }
    }
    count
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
        && !initial_pair_chars(chars[index], chars[index + 1])
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
    parse_fuhivla_shape_with_head_policy(chars, start, end, FuhivlaHeadPolicy::Standard)
        || parse_experimental_cgv_fuhivla_shape(chars, start, end)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn parse_experimental_cgv_fuhivla_shape(chars: &[char], start: usize, end: usize) -> bool {
    has_cgv_sequence_slice(chars, start, end)
        && parse_fuhivla_shape_with_head_policy(
            chars,
            start,
            end,
            FuhivlaHeadPolicy::ExperimentalCgv,
        )
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn has_cgv_sequence_slice(chars: &[char], start: usize, end: usize) -> bool {
    cgv_range(&chars[start..end]).is_some()
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn parse_fuhivla_shape_with_head_policy(
    chars: &[char],
    start: usize,
    end: usize,
    head_policy: FuhivlaHeadPolicy,
) -> bool {
    fuhivla_head_ends_for_fuhivla(chars, start, end, head_policy)
        .into_iter()
        .any(|head_end| {
            stressed_syllable_ends_for_fuhivla(chars, head_end, end)
                .into_iter()
                .any(|stressed_end| consonantal_chain_then_final(chars, stressed_end, end))
        })
}

#[invariant(::Standard => true)]
#[invariant(::ExperimentalCgv => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FuhivlaHeadPolicy {
    Standard,
    ExperimentalCgv,
}

#[requires(start <= end && end <= chars.len())]
#[ensures(ret.iter().all(|head_end| *head_end >= start && *head_end <= end))]
fn fuhivla_head_ends_for_fuhivla(
    chars: &[char],
    start: usize,
    end: usize,
    head_policy: FuhivlaHeadPolicy,
) -> Vec<usize> {
    match head_policy {
        FuhivlaHeadPolicy::Standard => fuhivla_head_ends_until(chars, start, end),
        FuhivlaHeadPolicy::ExperimentalCgv => brivla_head_ends_until(chars, start, end),
    }
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.iter().all(|syllable_end| *syllable_end > index && *syllable_end <= end))]
fn unstressed_syllable_ends_for_fuhivla(chars: &[char], index: usize, end: usize) -> Vec<usize> {
    let mut ends: Vec<usize> = syllable_ends(chars, index, end, SyllablePolicy::Brivla)
        .into_iter()
        .filter(|syllable_end| {
            !syllable_has_explicit_stress(chars, index, *syllable_end)
                && !stressed_syllable_has_implicit_stress(chars, index, *syllable_end, end)
        })
        .collect();
    ends.extend(consonantal_syllable_ends(
        chars,
        index,
        end,
        SyllablePolicy::Brivla,
    ));
    ends.sort_unstable();
    ends.dedup();
    ends
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.iter().all(|syllable_end| *syllable_end > index && *syllable_end <= end))]
fn stressed_syllable_ends_for_fuhivla(chars: &[char], index: usize, end: usize) -> Vec<usize> {
    syllable_ends(chars, index, end, SyllablePolicy::Brivla)
        .into_iter()
        .filter(|syllable_end| {
            syllable_has_explicit_stress(chars, index, *syllable_end)
                || stressed_syllable_has_implicit_stress(chars, index, *syllable_end, end)
        })
        .collect()
}

#[requires(start <= syllable_end && syllable_end <= end && end <= chars.len())]
#[ensures(true)]
fn stressed_syllable_has_implicit_stress(
    chars: &[char],
    start: usize,
    syllable_end: usize,
    end: usize,
) -> bool {
    if final_syllable_slice(chars, syllable_end, end) {
        return true;
    }
    syllable_can_end_without_coda(chars, start, syllable_end)
        && consonantal_syllable_ends(chars, syllable_end, end, SyllablePolicy::Brivla)
            .into_iter()
            .any(|next| next > syllable_end && consonantal_chain_then_final(chars, next, end))
}

#[requires(start <= syllable_end && syllable_end <= chars.len())]
#[ensures(true)]
fn syllable_can_end_without_coda(chars: &[char], start: usize, syllable_end: usize) -> bool {
    brivla_onset_ends(chars, start)
        .into_iter()
        .filter(|onset_end| !chars.get(*onset_end).is_some_and(|value| is_y(*value)))
        .any(|onset_end| {
            parse_nuclei(chars, onset_end)
                .into_iter()
                .any(|(_, nucleus_end)| nucleus_end == syllable_end)
        })
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn consonantal_chain_then_final(chars: &[char], index: usize, end: usize) -> bool {
    if final_syllable_slice(chars, index, end) {
        return true;
    }
    consonantal_syllable_ends(chars, index, end, SyllablePolicy::Brivla)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(::Brivla => true)]
enum SyllablePolicy {
    Brivla,
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.iter().all(|syllable_end| *syllable_end > index && *syllable_end <= end))]
fn syllable_ends(chars: &[char], index: usize, end: usize, policy: SyllablePolicy) -> Vec<usize> {
    let mut ends = Vec::new();
    for onset_end in brivla_onset_ends(chars, index) {
        if chars.get(onset_end).is_some_and(|value| is_y(*value)) {
            continue;
        }
        for (_, nucleus_end) in parse_nuclei(chars, onset_end) {
            if nucleus_end <= end {
                ends.push(nucleus_end);
                ends.extend(coda_ends(chars, nucleus_end, end, policy));
            }
        }
    }
    ends.sort_unstable();
    ends.dedup();
    ends
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.iter().all(|syllable_end| *syllable_end > index && *syllable_end <= end))]
fn consonantal_syllable_ends(
    chars: &[char],
    index: usize,
    end: usize,
    policy: SyllablePolicy,
) -> Vec<usize> {
    if index >= end
        || !is_consonant(chars[index])
        || !chars
            .get(index + 1)
            .is_some_and(|value| is_syllabic(*value))
    {
        return Vec::new();
    }
    coda_ends(chars, index + 1, end, policy)
        .into_iter()
        .filter(|coda_end| *coda_end > index + 1)
        .collect()
}

#[requires(index <= end && end <= chars.len())]
#[ensures(ret.iter().all(|coda_end| *coda_end >= index && *coda_end <= end))]
fn coda_ends(chars: &[char], index: usize, end: usize, policy: SyllablePolicy) -> Vec<usize> {
    let mut ends = vec![index];
    if index < end
        && is_consonant(chars[index])
        && !starts_any_syllable(chars, index, end, policy)
        && starts_any_syllable(chars, index + 1, end, policy)
    {
        ends.push(index + 1);
    }
    ends
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn starts_any_syllable(chars: &[char], index: usize, end: usize, policy: SyllablePolicy) -> bool {
    if index >= end {
        return false;
    }
    if !consonantal_syllable_ends(chars, index, end, policy).is_empty() {
        return true;
    }
    brivla_onset_ends(chars, index)
        .into_iter()
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
    if let Some(end) = experimental_cgv_brivla_onset_end(chars, index) {
        ends.push(end);
    }
    if chars.get(index) == Some(&'\'') {
        ends.push(index + 1);
    }
    ends.sort_unstable_by(|left, right| right.cmp(left));
    ends.dedup();
    ends
}

#[requires(index <= chars.len())]
#[ensures(ret.is_none_or(|end| end > index && end <= chars.len()))]
fn experimental_cgv_brivla_onset_end(chars: &[char], index: usize) -> Option<usize> {
    let max_initial_end = (index + 3).min(chars.len());
    (index + 1..=max_initial_end).rev().find_map(|initial_end| {
        (valid_initial_shape(chars, index, initial_end) && starts_glide(chars, initial_end))
            .then(|| next_non_comma_index(chars, initial_end + 1))
            .flatten()
    })
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
        if !is_vowel(chars[index]) && !matches!(chars[index], 'ĭ' | 'ŭ') {
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
        && (!parse_onsets(chars, index).is_empty()
            || experimental_cgv_brivla_onset_end(chars, index).is_some())
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
        initial_pair_chars(first, second)
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
    is_sibilant(first)
        && initial_pair_chars(first, second)
        && initial_pair_chars(second, third)
        && is_liquid(third)
}

#[requires(true)]
#[ensures(true)]
fn is_sibilant(value: char) -> bool {
    matches!(value, 'c' | 's' | 'j' | 'z')
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
