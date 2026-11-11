use std::ops::Range;
use std::rc::Rc;

use bityzba::{data, invariant, new, requires};
use vec1::Vec1;

use crate::{
    Cmavo, LujvoParseExpectation, LujvoPart, MorphologyErrorDetail, MorphologyErrorDetailData,
    MorphologyErrorKind, MorphologyOptions, Phonemes, Selmaho, WordKind,
};

mod phonotactics;
mod recognition_cache;
pub use phonotactics::ConsonantPairClass;
pub(crate) use phonotactics::{
    consonant_pair_class, experimental_permissible_consonant_pair, initial_pair_chars,
    permissible_consonant_pair,
};
pub(crate) use recognition_cache::MorphologyRecognitionCache;

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_separator(value: char) -> bool {
    value.is_whitespace()
        || crate::is_period_character(value)
        || matches!(
            value,
            '?' | '!'
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
            Some(event) => match event.as_data() {
                data!(NormalizedCharEvent::Emit {
                    value: normalized_value,
                    implicit_apostrophe_vowel,
                }) => {
                    push_normalized_value(
                        &mut normalized,
                        &mut previous_implicit_apostrophe_vowel,
                        source_index,
                        source_index + 1,
                        value,
                        *normalized_value,
                        *implicit_apostrophe_vowel,
                    );
                }
                data!(NormalizedCharEvent::EmitText { text }) => {
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
                data!(NormalizedCharEvent::StressPrevious) => {
                    stress_last_normalized_char(&mut normalized);
                }
                data!(NormalizedCharEvent::StressPreviousVowel) => {
                    stress_last_normalized_vowel_char(&mut normalized);
                }
                data!(NormalizedCharEvent::Ignore) => {
                    previous_implicit_apostrophe_vowel = false;
                }
            },
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

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(_, phonemes)| !phonemes.is_empty()))]
pub(crate) fn classify_word(normalized_word: &str) -> Option<(WordKind, String)> {
    let stripped = normalized_word.replace(',', "");
    if stripped.is_empty() {
        return None;
    }

    let normalized_chars = text_chars(normalized_word);
    let stripped_chars = text_chars(&stripped);
    let mut cache = MorphologyRecognitionCache::new(stripped_chars.len());
    classify_word_with_cache(
        normalized_word,
        &normalized_chars,
        &stripped_chars,
        0,
        stripped_chars.len(),
        &mut cache,
    )
}

#[requires(stripped_start <= stripped_end && stripped_end <= stripped_chars.len())]
#[requires(cache.char_count == stripped_chars.len())]
#[ensures(ret.as_ref().is_none_or(|(_, phonemes)| !phonemes.is_empty()))]
pub(crate) fn classify_word_with_cache(
    normalized_word: &str,
    normalized_chars: &[char],
    stripped_chars: &[char],
    stripped_start: usize,
    stripped_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Option<(WordKind, String)> {
    if cache.is_cancelled() {
        return None;
    }

    let stripped_prefix = &stripped_chars[stripped_start..stripped_end];
    if stripped_prefix.is_empty() {
        return None;
    }
    let blocks_brivla = blocks_word_shape(normalized_chars);

    if !blocks_brivla && is_gismu_chars(stripped_prefix) {
        return Some((
            WordKind::Gismu,
            canonicalize_brivla_phonemes(normalized_word),
        ));
    }

    if !blocks_brivla
        && is_lujvo_slice_with_cache(stripped_chars, stripped_start, stripped_end, cache)
    {
        return Some((
            WordKind::Lujvo,
            canonicalize_brivla_phonemes(normalized_word),
        ));
    }

    if !blocks_brivla
        && is_fuhivla_shape_slice_with_cache(stripped_chars, stripped_start, stripped_end, cache)
    {
        return Some((
            WordKind::Fuhivla,
            canonicalize_brivla_phonemes(normalized_word),
        ));
    }

    if is_cmevla_text(normalized_word) {
        return Some((
            WordKind::Cmevla,
            mark_predictable_stress(&canonicalize_word_phonemes(normalized_word)),
        ));
    }

    None
}

#[requires(true)]
#[ensures(!ret.is_empty() || normalized_word.is_empty())]
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

#[requires(true)]
#[ensures(!ret.is_empty() || normalized_word.is_empty())]
fn canonicalize_brivla_phonemes(normalized_word: &str) -> String {
    mark_predictable_stress(
        &canonicalize_word_phonemes(normalized_word)
            .chars()
            .filter(|value| *value != ',')
            .collect::<String>(),
    )
}

#[requires(true)]
#[ensures(!ret.is_empty() || phonemes.is_empty())]
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
        if let Some((_, end)) = parse_diphthong_end(&chars, index) {
            if let Some(byte_start) = byte_starts.get(index).copied() {
                starts.push(byte_start);
            }
            index = end;
        } else if let Some(end) = parse_single_vowel_end(&chars, index) {
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
#[ensures(ret.iter().all(|parts| !parts.is_empty()))]
pub(crate) fn parse_cmevla_lujvo_part_candidates_with_canonical_phonemes(
    shape_word: &str,
    canonical_word: &str,
) -> Vec<Vec<LujvoPart>> {
    let shape_chars = text_chars(shape_word);
    let canonical_chars = text_chars(canonical_word);
    if shape_chars.len() != canonical_chars.len() {
        return Vec::new();
    }
    analyze_cmevla_lujvo_part_range_candidates_chars(&shape_chars)
        .into_iter()
        .filter_map(|ranges| lujvo_ranges_to_parts(&canonical_chars, ranges))
        .collect()
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
    /// The continuation this rafsi still needs.
    /// A stressed extended rafsi expects a short final rafsi.
    /// An unstressed initial rafsi expects another initial rafsi or a final part.
    expected: LujvoParseExpectation,
    /// True when that rafsi reaches the end of the span.
    /// No input remains, so the lujvo is incomplete.
    incomplete: bool,
}

impl LujvoParseFailure {
    #[requires(true)]
    #[ensures(ret.index == index)]
    #[ensures(ret.expected == expected)]
    #[ensures(!ret.incomplete)]
    fn new(index: usize, expected: LujvoParseExpectation) -> Self {
        LujvoParseFailure {
            index: index,
            expected: expected,
            incomplete: false,
        }
    }

    #[requires(true)]
    #[ensures(ret.index == index)]
    #[ensures(ret.expected == expected)]
    #[ensures(ret.incomplete)]
    fn incomplete(index: usize, expected: LujvoParseExpectation) -> Self {
        LujvoParseFailure {
            index: index,
            expected: expected,
            incomplete: true,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn to_detail(self, chars: &[char]) -> Option<MorphologyErrorDetail> {
        if self.index == 0 || self.index > chars.len() {
            return None;
        }
        let parsed_prefix = chars[..self.index].iter().collect::<String>();
        if self.incomplete {
            Some(new!(MorphologyErrorDetail::IncompleteLujvo {
                parsed_prefix: parsed_prefix,
                expected: self.expected,
            }))
        } else {
            Some(new!(MorphologyErrorDetail::InvalidLujvo {
                parsed_prefix: Some(parsed_prefix),
                expected: self.expected,
            }))
        }
    }
}

#[invariant(self.start < self.end, "lujvo part ranges must cover a non-empty span")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LujvoPartRange {
    kind: LujvoPartRangeKind,
    start: usize,
    end: usize,
}

impl LujvoPartRange {
    #[requires(start < end)]
    #[ensures(ret.kind == LujvoPartRangeKind::Rafsi)]
    #[ensures(ret.start == start)]
    #[ensures(ret.end == end)]
    fn rafsi(start: usize, end: usize) -> Self {
        new!(LujvoPartRange {
            kind: LujvoPartRangeKind::Rafsi,
            start: start,
            end: end,
        })
    }

    #[requires(start < end)]
    #[ensures(ret.kind == LujvoPartRangeKind::Hyphen)]
    #[ensures(ret.start == start)]
    #[ensures(ret.end == end)]
    fn hyphen(start: usize, end: usize) -> Self {
        new!(LujvoPartRange {
            kind: LujvoPartRangeKind::Hyphen,
            start: start,
            end: end,
        })
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LujvoPartRangeKind {
    Rafsi,
    Hyphen,
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|parts| !parts.is_empty()) || ret.is_err())]
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
#[ensures(ret.as_ref().is_ok_and(|ranges| !ranges.is_empty()) || ret.is_err())]
fn analyze_lujvo_part_ranges_chars(
    chars: &[char],
) -> Result<Vec1<LujvoPartRange>, LujvoParseFailure> {
    let mut cache = MorphologyRecognitionCache::new(chars.len());
    analyze_lujvo_part_ranges_chars_with_cache(chars, &mut cache)
}

#[requires(cache.char_count == chars.len())]
#[ensures(ret.as_ref().is_ok_and(|ranges| !ranges.is_empty()) || ret.is_err())]
fn analyze_lujvo_part_ranges_chars_with_cache(
    chars: &[char],
    cache: &mut MorphologyRecognitionCache,
) -> Result<Vec1<LujvoPartRange>, LujvoParseFailure> {
    if cache.is_cancelled() {
        return Err(LujvoParseFailure::new(
            0,
            LujvoParseExpectation::InitialOrStandaloneFinalRafsi,
        ));
    }

    let mut failure = None;
    if chars.len() <= 3 || !chars.iter().all(|value| is_lujvo_char(*value)) {
        record_lujvo_failure(&mut failure, 0, false);
        return Err(failure.expect("initial lujvo failure was recorded"));
    }
    if let Some(ranges) =
        lujvo_part_ranges_from_with_cache(chars, 0, chars.len(), false, &mut failure, cache)
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
fn merge_lujvo_failure(
    failure: &mut Option<LujvoParseFailure>,
    candidate: Option<LujvoParseFailure>,
) {
    if let Some(candidate) = candidate
        && failure.is_none_or(|current| candidate.index > current.index)
    {
        *failure = Some(candidate);
    }
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

#[requires(true)]
#[ensures(true)]
pub(crate) fn classify_lujvo_rafsi(text: &str) -> crate::ValsiLujvoRafsiKind {
    let chars = text_chars(text);
    if chars.is_empty() {
        return crate::ValsiLujvoRafsiKind::Unknown;
    }
    let end = chars.len();
    if is_cultural_rafsi_slice(&chars, 0, end) {
        return crate::ValsiLujvoRafsiKind::Cultural;
    }
    if is_gismu_slice(&chars, 0, end) {
        return crate::ValsiLujvoRafsiKind::Gismu;
    }
    if is_fuhivla_shape_slice(&chars, 0, end) {
        return crate::ValsiLujvoRafsiKind::Fuhivla;
    }
    if cvc_rafsi_end(&chars, 0) == Some(end) {
        return crate::ValsiLujvoRafsiKind::Cvc;
    }
    if ccv_rafsi_end(&chars, 0) == Some(end) {
        return crate::ValsiLujvoRafsiKind::Ccv;
    }
    if cvv_rafsi_ends(&chars, 0)
        .into_iter()
        .any(|rafsi_end| rafsi_end == end)
    {
        return crate::ValsiLujvoRafsiKind::Cvv;
    }
    if long_rafsi_ends(&chars, 0)
        .into_iter()
        .any(|rafsi_end| rafsi_end == end)
    {
        return crate::ValsiLujvoRafsiKind::Long;
    }
    if rafsi_string_slice(&chars, 0, end) {
        return crate::ValsiLujvoRafsiKind::Extended;
    }
    crate::ValsiLujvoRafsiKind::Unknown
}

#[requires(true)]
#[ensures(!ret || !text.is_empty())]
pub(crate) fn is_lujvo_rafsi_fragment(text: &str) -> bool {
    if classify_lujvo_rafsi(text) != crate::ValsiLujvoRafsiKind::Unknown {
        return true;
    }

    let mut chars = text_chars(text);
    if chars.is_empty() {
        return false;
    }

    // Extended and cultural fu'ivla rafsi are the fu'ivla stem with its final
    // vowel removed. Test that exact segmenter shape rule with each possible
    // final vowel; unlike jvozba's RafsiShape, this also covers consonant-final
    // pieces such as `spaget`, `kulnrfars`, and `akt`.
    //
    // Each trial appends a different vowel, so the five strings are not one
    // candidate and cannot share a table. A sparse cache starts empty, so
    // these trials do not allocate a square.
    for final_vowel in ['a', 'e', 'i', 'o', 'u'] {
        chars.push(final_vowel);
        let is_fuhivla = is_fuhivla_shape_slice(&chars, 0, chars.len());
        chars.pop();
        if is_fuhivla {
            return true;
        }
    }
    false
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn classify_fuhivla_stage(text: &str) -> crate::ValsiFuhivlaStage {
    let chars = text_chars(text);
    if chars.is_empty() || !is_fuhivla_shape_slice(&chars, 0, chars.len()) {
        return crate::ValsiFuhivlaStage::Unknown;
    }
    if has_stage3_classifier(&chars) {
        crate::ValsiFuhivlaStage::Stage3
    } else {
        crate::ValsiFuhivlaStage::Stage4
    }
}

#[requires(true)]
#[ensures(true)]
fn has_stage3_classifier(chars: &[char]) -> bool {
    let end = chars.len();
    // One cache for every classifier trial. A fresh cache per trial would
    // repeat the same candidate's recognizer from scratch.
    let mut cache = MorphologyRecognitionCache::new(chars.len());
    for classifier_end in y_rafsi_ends(chars, 0)
        .into_iter()
        .chain(hy_rafsi_ends(chars, 0))
        .chain(rn_hyphenated_classifier_ends(chars, 0))
    {
        if classifier_end < end
            && is_fuhivla_shape_slice_with_cache(chars, classifier_end, end, &mut cache)
        {
            return true;
        }
    }
    false
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn rn_hyphenated_classifier_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    for base_end in long_rafsi_ends(chars, index)
        .into_iter()
        .chain(cvc_rafsi_end(chars, index))
        .chain(ccv_rafsi_end(chars, index))
        .chain(cvv_rafsi_base_ends(chars, index))
    {
        if let Some(end) = r_hyphen_end(chars, base_end).or_else(|| n_hyphen_end(chars, base_end)) {
            ends.push(end);
        }
    }
    ends
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

#[invariant(::Emit { value, .. } => is_valid_normalized_char(*value))]
#[invariant(::EmitText { text } => !text.is_empty() && text.chars().all(is_valid_normalized_char))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        return Some(new!(NormalizedCharEvent::StressPrevious));
    }
    if options.accept_zbalermorna && is_zbalermorna_stress_mark(value) {
        return Some(new!(NormalizedCharEvent::StressPreviousVowel));
    }
    if options.accept_zbalermorna && is_zbalermorna_ignored_mark(value) {
        return Some(new!(NormalizedCharEvent::Ignore));
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
        return Some(new!(NormalizedCharEvent::EmitText { text: normalized }));
    }
    None
}

#[requires(is_valid_normalized_char(value))]
#[ensures(matches!(ret.as_data(), data!(NormalizedCharEvent::Emit { value: emitted, implicit_apostrophe_vowel }) if *emitted == value && *implicit_apostrophe_vowel == insert_implicit_apostrophe))]
fn normalized_emit(value: char, insert_implicit_apostrophe: bool) -> NormalizedCharEvent {
    new!(NormalizedCharEvent::Emit {
        value,
        implicit_apostrophe_vowel: insert_implicit_apostrophe,
    })
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
pub(crate) fn is_valid_normalized_char(value: char) -> bool {
    is_vowel(value)
        || is_consonant(value)
        || matches!(value, 'y' | 'ý' | '\'' | ',' | 'ĭ' | 'ŭ' | '0'..='9')
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn text_chars(text: &str) -> Vec<char> {
    text.chars().collect()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn parse_cmavo_form(text: &str) -> Option<String> {
    let chars = text_chars(text);
    parse_cmavo_form_chars(&chars)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
fn parse_cmavo_form_chars(chars: &[char]) -> Option<String> {
    let mut normalized = String::new();
    let mut output = Some(&mut normalized);
    parse_cmavo_form_chars_into(chars, &mut output)?;
    Some(normalized)
}

#[requires(true)]
#[ensures(true)]
fn matches_cmavo_form_chars(chars: &[char]) -> bool {
    let mut output = None;
    parse_cmavo_form_chars_into(chars, &mut output).is_some()
}

#[requires(true)]
#[ensures(true)]
fn parse_cmavo_form_chars_into(chars: &[char], output: &mut Option<&mut String>) -> Option<()> {
    if chars.is_empty() {
        return None;
    }
    if chars.iter().all(|value| matches!(value, 'y' | 'ý')) {
        append_cmavo_chars(output, chars);
        return Some(());
    }
    if chars.len() == 1 && chars[0].is_ascii_digit() {
        append_cmavo_str(output, digit_to_cmavo(chars[0])?);
        return Some(());
    }
    parse_cmavo_form_main_into(chars, output)
}

#[requires(true)]
#[ensures(true)]
fn append_cmavo_chars(output: &mut Option<&mut String>, chars: &[char]) {
    if let Some(text) = output.as_mut() {
        text.extend(chars.iter().copied());
    }
}

#[requires(true)]
#[ensures(true)]
fn append_cmavo_str(output: &mut Option<&mut String>, value: &str) {
    if let Some(text) = output.as_mut() {
        text.push_str(value);
    }
}

#[requires(true)]
#[ensures(true)]
fn append_cmavo_char(output: &mut Option<&mut String>, value: char) {
    if let Some(text) = output.as_mut() {
        text.push(value);
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_cmavo_form_main_into(chars: &[char], output: &mut Option<&mut String>) -> Option<()> {
    if chars.first().is_some_and(|value| *value == '\'') || starts_with_cluster(chars, 0) {
        return None;
    }
    for onset in parse_onsets(chars, 0) {
        if with_cmavo_output_rollback(output, |output| {
            append_cmavo_str(output, &onset.normalized);
            parse_cmavo_form_tail_into(chars, onset.end, output)
        })
        .is_some()
        {
            return Some(());
        }
    }
    None
}

#[requires(start <= chars.len())]
#[ensures(true)]
fn parse_cmavo_form_tail_into(
    chars: &[char],
    start: usize,
    output: &mut Option<&mut String>,
) -> Option<()> {
    if let Some((semivowel, after_nucleus)) = parse_diphthong_end(chars, start) {
        if after_nucleus == chars.len() {
            append_cmavo_diphthong_nucleus(output, chars[start], semivowel);
            return Some(());
        }
        if chars.get(after_nucleus) == Some(&'\'')
            && with_cmavo_output_rollback(output, |output| {
                append_cmavo_diphthong_nucleus(output, chars[start], semivowel);
                append_cmavo_char(output, '\'');
                parse_cmavo_form_tail_into(chars, after_nucleus + 1, output)
            })
            .is_some()
        {
            return Some(());
        }
    }
    if let Some(after_nucleus) = parse_single_vowel_end(chars, start) {
        let value = chars[start];
        if after_nucleus == chars.len() {
            append_cmavo_single_nucleus(output, value);
            return Some(());
        }
        if chars.get(after_nucleus) == Some(&'\'')
            && with_cmavo_output_rollback(output, |output| {
                append_cmavo_single_nucleus(output, value);
                append_cmavo_char(output, '\'');
                parse_cmavo_form_tail_into(chars, after_nucleus + 1, output)
            })
            .is_some()
        {
            return Some(());
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn append_cmavo_diphthong_nucleus(output: &mut Option<&mut String>, first: char, semivowel: char) {
    append_cmavo_char(output, normalize_vowel(first));
    append_cmavo_char(output, semivowel);
}

#[requires(true)]
#[ensures(true)]
fn append_cmavo_single_nucleus(output: &mut Option<&mut String>, value: char) {
    let nucleus = if is_y(value) {
        value
    } else {
        normalize_vowel(value)
    };
    append_cmavo_char(output, nucleus);
}

#[requires(true)]
#[ensures(true)]
fn with_cmavo_output_rollback(
    output: &mut Option<&mut String>,
    parse: impl FnOnce(&mut Option<&mut String>) -> Option<()>,
) -> Option<()> {
    let original_len = output.as_ref().map(|text| text.len());
    if parse(output).is_some() {
        return Some(());
    }
    if let Some(original_len) = original_len
        && let Some(text) = output.as_mut()
    {
        text.truncate(original_len);
    }
    None
}

#[invariant(self.start <= self.end)]
#[invariant(self.normalized.chars().count() == self.end - self.start)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedOnset {
    start: usize,
    end: usize,
    normalized: String,
}

#[requires(start <= chars.len())]
#[ensures(ret.iter().all(|onset| onset.start == start && onset.end >= start && onset.end <= chars.len()))]
fn parse_onsets(chars: &[char], start: usize) -> Vec<ParsedOnset> {
    let mut onsets = Vec::new();
    if let Some(end) = parse_glide_end(chars, start) {
        let glide = match chars[start] {
            'i' | 'í' | 'ĭ' => 'ĭ',
            'u' | 'ú' | 'ŭ' => 'ŭ',
            _ => unreachable!("parse_glide_end only accepts glide starts"),
        };
        onsets.push(new!(ParsedOnset {
            start: start,
            end: end,
            normalized: glide.to_string(),
        }));
    }
    for end in (start..=max_initial_end(chars, start)).rev() {
        if let Some(initial) = parse_initial(chars, start, end) {
            onsets.push(new!(ParsedOnset {
                start: start,
                end: end,
                normalized: initial,
            }));
        }
    }
    onsets
}

#[requires(start <= end && end <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|value| value.chars().count() == end - start))]
fn parse_initial(chars: &[char], start: usize, end: usize) -> Option<String> {
    parse_initial_end(chars, start, end)?;
    Some(chars[start..end].iter().collect())
}

#[requires(start <= chars.len())]
#[ensures(ret >= start && ret <= chars.len())]
fn max_initial_end(chars: &[char], start: usize) -> usize {
    chars.len().min(start + 3)
}

#[requires(start <= chars.len())]
#[requires(ends.iter().all(|end| *end <= chars.len()))]
#[ensures(ends.iter().all(|end| *end <= chars.len()))]
fn append_parse_onset_ends(chars: &[char], start: usize, ends: &mut Vec<usize>) {
    if let Some(end) = parse_glide_end(chars, start) {
        ends.push(end);
    }
    for end in (start..=max_initial_end(chars, start)).rev() {
        if parse_initial_end(chars, start, end).is_some() {
            ends.push(end);
        }
    }
}

#[requires(start <= chars.len())]
#[requires(target_end <= chars.len())]
#[ensures(true)]
fn has_parse_onset_end(chars: &[char], start: usize, target_end: usize) -> bool {
    if parse_glide_end(chars, start) == Some(target_end) {
        return true;
    }
    target_end >= start
        && target_end <= max_initial_end(chars, start)
        && parse_initial_end(chars, start, target_end).is_some()
}

#[requires(start <= chars.len())]
#[ensures(true)]
fn has_parse_onset(chars: &[char], start: usize) -> bool {
    parse_glide_end(chars, start).is_some()
        || (start..=max_initial_end(chars, start))
            .rev()
            .any(|end| parse_initial_end(chars, start, end).is_some())
}

#[requires(start <= end && end <= chars.len())]
#[ensures(ret.is_none_or(|value| value == end))]
fn parse_initial_end(chars: &[char], start: usize, end: usize) -> Option<usize> {
    if !valid_initial_shape(chars, start, end) {
        return None;
    }
    if end < chars.len() && (is_consonant(chars[end]) || starts_glide(chars, end)) {
        return None;
    }
    Some(end)
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
#[requires(ends.iter().all(|end| *end <= chars.len()))]
#[ensures(ends.iter().all(|end| *end <= chars.len()))]
fn append_parse_nucleus_ends(chars: &[char], start: usize, ends: &mut Vec<usize>) {
    if let Some((_, end)) = parse_diphthong_end(chars, start) {
        ends.push(end);
    }
    if let Some(end) = parse_single_vowel_end(chars, start) {
        ends.push(end);
    }
}

#[requires(start <= chars.len())]
#[requires(target_end <= chars.len())]
#[ensures(true)]
fn has_parse_nucleus_end(chars: &[char], start: usize, target_end: usize) -> bool {
    parse_diphthong_end(chars, start).is_some_and(|(_, end)| end == target_end)
        || parse_single_vowel_end(chars, start) == Some(target_end)
}

#[requires(start <= chars.len())]
#[requires(limit <= chars.len())]
#[ensures(true)]
fn has_parse_nucleus_end_at_or_before(chars: &[char], start: usize, limit: usize) -> bool {
    parse_diphthong_end(chars, start).is_some_and(|(_, end)| end <= limit)
        || parse_single_vowel_end(chars, start).is_some_and(|end| end <= limit)
}

#[requires(start <= chars.len())]
#[ensures(ret.is_none_or(|(_, end)| end > start && end <= chars.len()))]
pub(crate) fn parse_diphthong_end(chars: &[char], start: usize) -> Option<(char, usize)> {
    let (semivowel, end) = raw_diphthong_end(chars, start)?;
    if next_non_comma_index(chars, end)
        .is_some_and(|next| matches_diphthong_semivowel(chars[next], semivowel))
    {
        return None;
    }
    if starts_with_nucleus(chars, end) {
        return None;
    }
    Some((semivowel, end))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn matches_diphthong_semivowel(value: char, semivowel: char) -> bool {
    match semivowel {
        'ĭ' => matches!(value, 'i' | 'í' | 'ĭ'),
        'ŭ' => matches!(value, 'u' | 'ú' | 'ŭ'),
        _ => false,
    }
}

#[requires(start <= chars.len())]
#[ensures(ret.is_none_or(|end| end == start + 1))]
pub(crate) fn parse_single_vowel_end(chars: &[char], start: usize) -> Option<usize> {
    let value = *chars.get(start)?;
    if value == 'y' || value == 'ý' {
        let end = start + 1;
        if starts_with_nucleus(chars, end) {
            return None;
        }
        return Some(end);
    }
    if !is_vowel(value) && !matches!(value, 'ĭ' | 'ŭ') {
        return None;
    }
    let end = start + 1;
    if starts_with_nucleus(chars, end) {
        return None;
    }
    Some(end)
}

#[requires(start <= chars.len())]
#[ensures(ret.is_none_or(|(_, end)| end > start && end <= chars.len()))]
pub(crate) fn parse_explicit_stress_nucleus_end(
    chars: &[char],
    start: usize,
) -> Option<(bool, usize)> {
    if let Some(end) = parse_explicit_stress_diphthong_end(chars, start) {
        return Some((true, end));
    }
    let value = *chars.get(start)?;
    parse_explicit_stress_single_vowel_end(chars, start)
        .map(|end| (!matches!(value, 'y' | 'ý'), end))
}

#[requires(start <= chars.len())]
#[ensures(ret.is_none_or(|end| end > start && end <= chars.len()))]
fn parse_explicit_stress_diphthong_end(chars: &[char], start: usize) -> Option<usize> {
    let (semivowel, end) = raw_diphthong_end(chars, start)?;
    if next_non_comma_index(chars, end)
        .is_some_and(|next| matches_diphthong_semivowel(chars[next], semivowel))
    {
        return None;
    }
    if starts_with_pause_required_nucleus(chars, end) {
        return None;
    }
    Some(end)
}

#[requires(start <= chars.len())]
#[ensures(ret.is_none_or(|end| end == start + 1))]
fn parse_explicit_stress_single_vowel_end(chars: &[char], start: usize) -> Option<usize> {
    let value = *chars.get(start)?;
    if value == 'y' || value == 'ý' {
        let end = start + 1;
        if starts_with_pause_required_nucleus(chars, end) {
            return None;
        }
        return Some(end);
    }
    if !is_vowel(value) && !matches!(value, 'ĭ' | 'ŭ') {
        return None;
    }
    let end = start + 1;
    if starts_with_pause_required_nucleus(chars, end) {
        return None;
    }
    Some(end)
}

#[requires(start <= chars.len())]
#[ensures(ret.is_none_or(|end| end > start && end <= chars.len()))]
fn parse_glide_end(chars: &[char], start: usize) -> Option<usize> {
    starts_glide(chars, start).then_some(start + 1)
}

#[requires(start <= chars.len())]
#[ensures(true)]
pub(crate) fn starts_with_nucleus(chars: &[char], start: usize) -> bool {
    if start >= chars.len() {
        return false;
    }
    parse_diphthong_end(chars, start).is_some() || parse_single_vowel_end(chars, start).is_some()
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
pub(crate) fn is_cmevla_text(normalized: &str) -> bool {
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
pub(crate) fn is_vowel(value: char) -> bool {
    base_vowel(value).is_some()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn base_vowel(value: char) -> Option<char> {
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
pub(crate) fn normalize_vowel(value: char) -> char {
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
fn is_gismu_chars(chars: &[char]) -> bool {
    match chars {
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

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn lujvo_from_until_with_cache(
    chars: &[char],
    index: usize,
    end: usize,
    has_initial_rafsi: bool,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.lujvo_from(index, end, has_initial_rafsi) {
        return result;
    }
    // This checker only needs a yes or no. The failure record belongs to the
    // part-range analysis that builds the diagnostic.
    let mut ignored_failure = None;
    let result = if index >= end {
        false
    } else if has_initial_rafsi
        && lujvo_core_part_ranges_until(chars, index, end, &mut ignored_failure, cache).is_some()
    {
        true
    } else if !has_initial_rafsi
        && lujvo_core_part_ranges_until(chars, index, end, &mut ignored_failure, cache)
            .is_some_and(|ranges| ranges.len() > 1)
    {
        true
    } else if !has_initial_rafsi && is_lujvo_final_rafsi_alone_until(chars, index, end) {
        true
    } else {
        initial_rafsi_ends_until_with_cache(chars, index, end, cache)
            .into_iter()
            .any(|next| next > index && lujvo_from_until_with_cache(chars, next, end, true, cache))
    };
    cache.record_lujvo_from(index, end, has_initial_rafsi, result);
    result
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.as_ref().is_none_or(|ranges| !ranges.is_empty()))]
#[ensures(ret.as_ref().is_none_or(|ranges| ranges.iter().all(|range| range.start < range.end && range.end <= end)))]
fn lujvo_part_ranges_from_with_cache(
    chars: &[char],
    index: usize,
    end: usize,
    has_initial_rafsi: bool,
    failure: &mut Option<LujvoParseFailure>,
    cache: &mut MorphologyRecognitionCache,
) -> Option<Vec<LujvoPartRange>> {
    if cache.is_cancelled() {
        return None;
    }

    if let Some(memo) = cache.lujvo_part_ranges(index, end, has_initial_rafsi) {
        merge_lujvo_failure(failure, memo.failure);
        return memo.ranges.as_ref().map(|ranges| ranges.to_vec());
    }
    let mut local_failure = None;
    let result = lujvo_part_ranges_uncached(
        chars,
        index,
        end,
        has_initial_rafsi,
        &mut local_failure,
        cache,
    );
    let ranges = result
        .as_ref()
        .map(|ranges| Rc::<[LujvoPartRange]>::from(ranges.as_slice()));
    cache.record_lujvo_part_ranges(index, end, has_initial_rafsi, ranges, local_failure);
    merge_lujvo_failure(failure, local_failure);
    result
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.as_ref().is_none_or(|ranges| !ranges.is_empty()))]
#[ensures(ret.as_ref().is_none_or(|ranges| ranges.iter().all(|range| range.start < range.end && range.end <= end)))]
fn lujvo_part_ranges_uncached(
    chars: &[char],
    index: usize,
    end: usize,
    has_initial_rafsi: bool,
    failure: &mut Option<LujvoParseFailure>,
    cache: &mut MorphologyRecognitionCache,
) -> Option<Vec<LujvoPartRange>> {
    if index >= end {
        record_lujvo_failure(failure, index, has_initial_rafsi);
        return None;
    }
    if has_initial_rafsi
        && let Some(ranges) = lujvo_core_part_ranges_until(chars, index, end, failure, cache)
    {
        return Some(ranges);
    }
    if !has_initial_rafsi
        && let Some(ranges) = lujvo_core_part_ranges_until(chars, index, end, failure, cache)
        && ranges.len() > 1
    {
        return Some(ranges);
    }
    if !has_initial_rafsi && is_lujvo_final_rafsi_alone_until(chars, index, end) {
        return Some(vec![LujvoPartRange::rafsi(index, end)]);
    }
    for next in initial_rafsi_ends_until_with_cache(chars, index, end, cache) {
        if next <= index {
            continue;
        }
        // The rafsi consumed this span and ends on y. No input remains after
        // that y. The lujvo is incomplete. Stress comes from the parser
        // predicate in this span. A stressed initial rafsi needs a short final
        // rafsi. An unstressed initial rafsi can take another initial rafsi or
        // a brivla core. A tail after the y stays on the invalid-lujvo path.
        if next == end && initial_rafsi_reaches_end_on_y(chars, index, end, cache) {
            let expected = if stressed_y_ending_rafsi_in_context(chars, index, end, cache) {
                LujvoParseExpectation::ShortFinalRafsi
            } else {
                LujvoParseExpectation::FinalOrInitialRafsi
            };
            merge_lujvo_failure(failure, Some(LujvoParseFailure::incomplete(end, expected)));
            continue;
        }
        let Some(mut rest) =
            lujvo_part_ranges_from_with_cache(chars, next, end, true, failure, cache)
        else {
            continue;
        };
        let mut ranges = initial_rafsi_ranges(chars, index, next)?;
        ranges.append(&mut rest);
        return Some(ranges);
    }
    record_lujvo_failure(failure, index, has_initial_rafsi);
    None
}

#[requires(!ranges.is_empty())]
#[requires(ranges.iter().all(|range| range.start < range.end && range.end <= chars.len()))]
#[ensures(ret.as_ref().is_none_or(|parts| !parts.is_empty()))]
fn lujvo_ranges_to_parts(chars: &[char], ranges: Vec<LujvoPartRange>) -> Option<Vec<LujvoPart>> {
    ranges
        .into_iter()
        .map(|range| match range.kind {
            LujvoPartRangeKind::Rafsi => rafsi_part(chars, range.start, range.end),
            LujvoPartRangeKind::Hyphen => hyphen_part(chars, range.start, range.end),
        })
        .collect()
}

#[requires(true)]
#[ensures(ret.iter().all(|ranges| !ranges.is_empty()))]
fn analyze_cmevla_lujvo_part_range_candidates_chars(chars: &[char]) -> Vec<Vec<LujvoPartRange>> {
    if chars.len() <= 3 || !chars.iter().all(|value| is_lujvo_char(*value)) {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    collect_cmevla_lujvo_part_ranges_from(chars, 0, &mut Vec::new(), &mut candidates);
    unique_cmevla_lujvo_part_range_candidates(candidates)
}

#[requires(candidates.iter().all(|ranges| !ranges.is_empty()))]
#[ensures(ret.iter().all(|ranges| !ranges.is_empty()))]
fn unique_cmevla_lujvo_part_range_candidates(
    candidates: Vec<Vec<LujvoPartRange>>,
) -> Vec<Vec<LujvoPartRange>> {
    let mut unique = Vec::new();
    for candidate in candidates {
        if !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }
    unique
}

#[requires(index <= chars.len())]
#[requires(acc.iter().all(|range| range.start < range.end && range.end <= chars.len()))]
#[requires(out.iter().all(|ranges| !ranges.is_empty()))]
#[ensures(out.iter().all(|ranges| !ranges.is_empty()))]
fn collect_cmevla_lujvo_part_ranges_from(
    chars: &[char],
    index: usize,
    acc: &mut Vec<LujvoPartRange>,
    out: &mut Vec<Vec<LujvoPartRange>>,
) {
    if index == chars.len() {
        if cmevla_lujvo_ranges_match_bonded(chars, acc) {
            out.push(acc.clone());
        }
        return;
    }

    for ranges in cmevla_lujvo_range_candidates(chars, index, acc) {
        let Some(next) = ranges.last().map(|range| range.end) else {
            continue;
        };
        let previous_len = acc.len();
        acc.extend(ranges);
        collect_cmevla_lujvo_part_ranges_from(chars, next, acc, out);
        acc.truncate(previous_len);
    }
}

#[requires(index < chars.len())]
#[requires(acc.iter().all(|range| range.start < range.end && range.end <= chars.len()))]
#[ensures(ret.iter().all(|ranges| !ranges.is_empty()))]
fn cmevla_lujvo_range_candidates(
    chars: &[char],
    index: usize,
    acc: &[LujvoPartRange],
) -> Vec<Vec<LujvoPartRange>> {
    let mut candidates = Vec::new();
    if acc
        .last()
        .is_some_and(|range| range.kind == LujvoPartRangeKind::Rafsi)
        && let Some(hyphen) = cmevla_lujvo_standalone_hyphen_range(chars, index)
    {
        candidates.push(vec![hyphen]);
    }

    candidates.extend(
        cmevla_lujvo_cvv_rafsi_ends(chars, index)
            .into_iter()
            .map(|end| vec![LujvoPartRange::rafsi(index, end)]),
    );

    if let Some(ranges) = cmevla_lujvo_y_hyphenated_rafsi_ranges(chars, index) {
        candidates.push(ranges);
    }

    if let Some(range) = cmevla_lujvo_final_long_vowel_range(chars, index) {
        candidates.push(vec![range]);
    }
    if let Some(range) = cmevla_lujvo_final_long_consonant_range(chars, index) {
        candidates.push(vec![range]);
    }

    if let Some(end) = cvc_rafsi_end(chars, index) {
        candidates.push(vec![LujvoPartRange::rafsi(index, end)]);
    }
    if let Some(end) = ccv_rafsi_end(chars, index) {
        candidates.push(vec![LujvoPartRange::rafsi(index, end)]);
    }

    candidates
}

#[requires(index <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|range| range.start == index && range.end > index && range.end <= chars.len()))]
fn cmevla_lujvo_standalone_hyphen_range(chars: &[char], index: usize) -> Option<LujvoPartRange> {
    rafsi_hyphen_end(chars, index)
        .or_else(|| r_hyphen_end(chars, index))
        .or_else(|| n_hyphen_end(chars, index))
        .map(|end| LujvoPartRange::hyphen(index, end))
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn cmevla_lujvo_cvv_rafsi_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = cvv_rafsi_base_ends(chars, index);
    ends.sort_by_key(|end| (*end, *end - index));
    ends
}

#[requires(index <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|ranges| !ranges.is_empty()))]
fn cmevla_lujvo_y_hyphenated_rafsi_ranges(
    chars: &[char],
    index: usize,
) -> Option<Vec<LujvoPartRange>> {
    long_rafsi_ends(chars, index)
        .into_iter()
        .chain(cvc_rafsi_end(chars, index))
        .find_map(|base_end| {
            rafsi_hyphen_end(chars, base_end).map(|end| {
                vec![
                    LujvoPartRange::rafsi(index, base_end),
                    LujvoPartRange::hyphen(base_end, end),
                ]
            })
        })
}

#[requires(index <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|range| range.start == index && range.end == chars.len()))]
fn cmevla_lujvo_final_long_vowel_range(chars: &[char], index: usize) -> Option<LujvoPartRange> {
    is_gismu_slice(chars, index, chars.len()).then_some(LujvoPartRange::rafsi(index, chars.len()))
}

#[requires(index <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|range| range.start == index && range.end == chars.len()))]
fn cmevla_lujvo_final_long_consonant_range(chars: &[char], index: usize) -> Option<LujvoPartRange> {
    long_rafsi_ends(chars, index)
        .into_iter()
        .any(|end| end == chars.len())
        .then_some(LujvoPartRange::rafsi(index, chars.len()))
}

#[requires(ranges.iter().all(|range| range.start < range.end && range.end <= chars.len()))]
#[ensures(true)]
fn cmevla_lujvo_ranges_match_bonded(chars: &[char], ranges: &[LujvoPartRange]) -> bool {
    let rafsi_texts = cmevla_lujvo_rafsi_texts(chars, ranges);
    if rafsi_texts.len() < 2 {
        return false;
    }
    let Some(bonded) = crate::bond_rafsis(&rafsi_texts) else {
        return false;
    };
    cmevla_lujvo_raw_ranges_match_bonded(chars, ranges, &bonded)
}

#[requires(ranges.iter().all(|range| range.start < range.end && range.end <= chars.len()))]
#[ensures(ret.iter().all(|text| !text.is_empty()))]
fn cmevla_lujvo_rafsi_texts(chars: &[char], ranges: &[LujvoPartRange]) -> Vec<String> {
    ranges
        .iter()
        .filter(|range| range.kind == LujvoPartRangeKind::Rafsi)
        .map(|range| chars[range.start..range.end].iter().collect())
        .collect()
}

/// Whether the written cmevla spells exactly what bonding its rafsi
/// produces, so the split is the word's own composition rather than a
/// phonotactically possible one.
///
/// The one licensed difference is the `r`/`n` bonding hyphen a lujvo would
/// need before an `r`-initial rafsi: a cmevla ends in a consonant, so its
/// shape already holds the word together and that hyphen may go unwritten.
/// `bau` and `rok` bond as `bau` + `n` + `rok`, yet the written `baurok` is
/// still those two rafsi. Everything else — including a plain chain such as
/// `fek` + `lat` that needs no hyphen at all — must match the bonded
/// spelling letter for letter.
#[requires(ranges.iter().all(|range| range.start < range.end && range.end <= chars.len()))]
#[ensures(true)]
fn cmevla_lujvo_raw_ranges_match_bonded(
    chars: &[char],
    ranges: &[LujvoPartRange],
    bonded: &[String],
) -> bool {
    let mut range_index = 0;
    for bonded_part in bonded {
        if ranges
            .get(range_index)
            .is_some_and(|range| cmevla_lujvo_range_text_is(chars, range, bonded_part))
        {
            range_index += 1;
            continue;
        }
        if crate::is_bonding_hyphen(bonded_part)
            && ranges.get(range_index).is_some_and(|range| {
                range.kind == LujvoPartRangeKind::Rafsi && chars.get(range.start) == Some(&'r')
            })
        {
            continue;
        }
        return false;
    }
    range_index == ranges.len()
}

#[requires(range.start < range.end && range.end <= chars.len())]
#[ensures(true)]
fn cmevla_lujvo_range_text_is(chars: &[char], range: &LujvoPartRange, text: &str) -> bool {
    chars[range.start..range.end]
        .iter()
        .copied()
        .eq(text.chars())
}

#[requires(start < end && end <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|ranges| !ranges.is_empty()))]
#[ensures(ret.as_ref().is_none_or(|ranges| ranges.iter().all(|range| range.start < range.end && range.end <= end)))]
fn initial_rafsi_ranges(chars: &[char], start: usize, end: usize) -> Option<Vec<LujvoPartRange>> {
    if let Some(hyphen_start) = (start + 1..end).find(|index| is_rafsi_hyphen_start(chars, *index))
    {
        return Some(vec![
            LujvoPartRange::rafsi(start, hyphen_start),
            LujvoPartRange::hyphen(hyphen_start, end),
        ]);
    }
    if let Some(hyphen_start) = cvv_rafsi_hyphen_start_for_initial_end(chars, start, end) {
        return Some(vec![
            LujvoPartRange::rafsi(start, hyphen_start),
            LujvoPartRange::hyphen(hyphen_start, end),
        ]);
    }
    Some(vec![LujvoPartRange::rafsi(start, end)])
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

#[cfg(test)]
#[requires(index <= chars.len())]
#[ensures(true)]
fn starts_with_cvcy_lujvo_chars(chars: &[char], index: usize) -> bool {
    let mut cache = MorphologyRecognitionCache::new(chars.len());
    starts_with_cvcy_lujvo_chars_with_cache(chars, index, chars.len(), &mut cache)
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
pub(crate) fn starts_with_cvcy_lujvo_chars_with_cache(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    let Some(base_end) = cvc_rafsi_end(chars, index) else {
        return false;
    };
    if base_end >= end || !chars.get(base_end).is_some_and(|value| is_y(*value)) {
        return false;
    }
    let mut after_hyphen = base_end + 1;
    if chars.get(after_hyphen) == Some(&'\'') {
        after_hyphen += 1;
    }
    after_hyphen <= end && lujvo_from_until_with_cache(chars, after_hyphen, end, true, cache)
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.as_ref().is_none_or(|ranges| !ranges.is_empty()))]
#[ensures(ret.as_ref().is_none_or(|ranges| ranges.iter().all(|range| range.start < range.end && range.end <= end)))]
fn lujvo_core_part_ranges_until(
    chars: &[char],
    index: usize,
    end: usize,
    failure: &mut Option<LujvoParseFailure>,
    cache: &mut MorphologyRecognitionCache,
) -> Option<Vec<LujvoPartRange>> {
    if is_gismu_slice(chars, index, end)
        || is_short_final_rafsi_slice(chars, index, end)
        || is_cvv_final_rafsi_slice(chars, index, end)
        || is_fuhivla_shape_slice_with_cache(chars, index, end, cache)
    {
        return Some(vec![LujvoPartRange::rafsi(index, end)]);
    }
    // brivla_core <- ... / stressed_initial_rafsi short_final_rafsi
    if let Some(split) =
        stressed_initial_rafsi_short_final_split_with_cache(chars, index, end, cache)
    {
        let mut ranges = initial_rafsi_ranges(chars, index, split)?;
        ranges.push(LujvoPartRange::rafsi(split, end));
        return Some(ranges);
    }
    record_stressed_extended_short_final_failure(chars, index, end, failure, cache);
    None
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn record_stressed_extended_short_final_failure(
    chars: &[char],
    index: usize,
    end: usize,
    failure: &mut Option<LujvoParseFailure>,
    cache: &mut MorphologyRecognitionCache,
) {
    // brivla_core <- ... / stressed_initial_rafsi short_final_rafsi.
    // stressed_initial_rafsi is a stressed extended rafsi, a stressed y-rafsi,
    // or a stressed y-less rafsi. The split search above returns only a match
    // whose tail is a short final. When that search fails, keep the furthest
    // stressed initial rafsi. Input after it is a bad tail. No input after it
    // means the short final is missing. Stress uses the parser predicate in
    // this span. A rafsi shape alone does not decide stress.
    let Some(split) = stressed_initial_rafsi_failure_split(chars, index, end, cache) else {
        return;
    };
    let candidate = if split == end {
        LujvoParseFailure::incomplete(split, LujvoParseExpectation::ShortFinalRafsi)
    } else {
        LujvoParseFailure::new(split, LujvoParseExpectation::ShortFinalRafsi)
    };
    merge_lujvo_failure(failure, Some(candidate));
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.is_none_or(|split| split > index && split <= end))]
fn stressed_initial_rafsi_failure_split(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Option<usize> {
    if cache.is_cancelled() {
        return None;
    }
    let mut splits = stressed_extended_rafsi_ends_until_with_cache(chars, index, end, cache)
        .into_iter()
        .filter(|split| chars.get(split - 1).is_some_and(|value| is_y(*value)))
        .collect::<Vec<_>>();
    splits.extend(stressed_y_rafsi_ends_in_context(chars, index, end, cache));
    // A stressed y-less rafsi has no y. The short-final note names the break
    // after y, so this alternative applies only when the rafsi reaches the
    // end and the short final is missing. A tail, as in `klá'y`, stays off
    // this path.
    splits.extend(
        stressed_y_less_rafsi_ends_in_context(chars, index, end, cache)
            .into_iter()
            .filter(|split| *split == end),
    );
    splits
        .into_iter()
        .filter(|split| {
            *split > index && *split <= end && !is_short_final_rafsi_slice(chars, *split, end)
        })
        .max()
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn stressed_y_ending_rafsi_in_context(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() || index >= end {
        return false;
    }
    // stressed_extended_rafsi and stressed_y_rafsi end on y.
    // The vowel stress predicate runs in this span. The rafsi shape is not
    // evidence that the rafsi is unstressed.
    stressed_extended_rafsi_slice_with_cache(chars, index, end, end, cache)
        || stressed_y_rafsi_ends_in_context(chars, index, end, cache).contains(&end)
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.iter().all(|next| *next > index && *next <= end))]
fn stressed_y_rafsi_ends_in_context(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Vec<usize> {
    // stressed_y_rafsi <- (stressed_long_rafsi / stressed_CVC_rafsi) y
    if cache.is_cancelled() {
        return Vec::new();
    }
    let mut ends = Vec::new();
    for base_end in stressed_long_rafsi_ends_in_context(chars, index, end, cache) {
        if chars.get(base_end).is_some_and(|value| is_y(*value)) && base_end + 1 <= end {
            ends.push(base_end + 1);
        }
    }
    if let Some(base_end) = stressed_cvc_rafsi_end_in_context(chars, index, end, cache)
        && chars.get(base_end).is_some_and(|value| is_y(*value))
        && base_end + 1 <= end
    {
        ends.push(base_end + 1);
    }
    ends
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.iter().all(|next| *next > index && *next <= end))]
fn stressed_y_less_rafsi_ends_in_context(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Vec<usize> {
    // stressed_y_less_rafsi <- stressed_CVC_rafsi !y / stressed_CCV_rafsi / stressed_CVV_rafsi
    if cache.is_cancelled() {
        return Vec::new();
    }
    let mut ends = Vec::new();
    if let Some(cvc_end) = stressed_cvc_rafsi_end_in_context(chars, index, end, cache)
        && !chars.get(cvc_end).is_some_and(|value| is_y(*value))
    {
        ends.push(cvc_end);
    }
    if index + 3 <= end
        && initial_pair_chars(chars[index], chars[index + 1])
        && is_vowel(chars[index + 2])
        && vowel_is_stressed_in_context(chars, index + 2, end, cache)
    {
        ends.push(index + 3);
    }
    ends.extend(stressed_cvv_rafsi_ends_in_context(chars, index, end, cache));
    ends
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.iter().all(|next| *next > index && *next <= end))]
fn stressed_long_rafsi_ends_in_context(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Vec<usize> {
    // stressed_long_rafsi <- initial_pair stressed_vowel consonant
    //                      / consonant stressed_vowel consonant consonant
    if cache.is_cancelled() {
        return Vec::new();
    }
    let mut ends = Vec::new();
    if index + 4 <= end
        && initial_pair_chars(chars[index], chars[index + 1])
        && is_vowel(chars[index + 2])
        && is_consonant(chars[index + 3])
        && vowel_is_stressed_in_context(chars, index + 2, end, cache)
    {
        ends.push(index + 4);
    }
    if index + 4 <= end
        && is_consonant(chars[index])
        && is_vowel(chars[index + 1])
        && is_consonant(chars[index + 2])
        && is_consonant(chars[index + 3])
        && vowel_is_stressed_in_context(chars, index + 1, end, cache)
    {
        ends.push(index + 4);
    }
    ends
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.is_none_or(|next| next > index && next <= end))]
fn stressed_cvc_rafsi_end_in_context(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Option<usize> {
    // stressed_CVC_rafsi <- consonant stressed_vowel consonant
    (index + 3 <= end
        && is_consonant(chars[index])
        && is_vowel(chars[index + 1])
        && is_consonant(chars[index + 2])
        && vowel_is_stressed_in_context(chars, index + 1, end, cache))
    .then_some(index + 3)
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.iter().all(|next| *next > index && *next <= end))]
fn stressed_cvv_rafsi_ends_in_context(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Vec<usize> {
    // stressed_CVV_rafsi <- consonant (unstressed_vowel h stressed_vowel / stressed_diphthong) r_hyphen?
    if cache.is_cancelled() || index >= end || !is_consonant(chars[index]) {
        return Vec::new();
    }
    let mut ends = Vec::new();
    if index + 4 <= end
        && is_vowel(chars[index + 1])
        && chars[index + 2] == '\''
        && is_vowel(chars[index + 3])
        && !vowel_is_stressed_in_context(chars, index + 1, end, cache)
        && vowel_is_stressed_in_context(chars, index + 3, end, cache)
    {
        push_rafsi_with_optional_r_hyphen(&mut ends, chars, index + 4, end);
    }
    if index + 3 <= end
        && is_diphthong_pair(chars[index + 1], chars[index + 2])
        && nucleus_is_stressed_in_context(chars, index + 1, index + 3, end, cache)
    {
        push_rafsi_with_optional_r_hyphen(&mut ends, chars, index + 3, end);
    }
    ends
}

#[requires(base_end <= end && end <= chars.len())]
#[ensures(ends.iter().all(|next| *next >= base_end && *next <= end))]
fn push_rafsi_with_optional_r_hyphen(
    ends: &mut Vec<usize>,
    chars: &[char],
    base_end: usize,
    end: usize,
) {
    ends.push(base_end);
    if let Some(hyphen_end) =
        r_hyphen_end(chars, base_end).or_else(|| n_hyphen_end(chars, base_end))
        && hyphen_end <= end
    {
        ends.push(hyphen_end);
    }
}

#[requires(vowel_index < context_end && context_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn vowel_is_stressed_in_context(
    chars: &[char],
    vowel_index: usize,
    context_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    // stressed_vowel <- &stressed vowel / vowel &stress
    nucleus_is_stressed_in_context(chars, vowel_index, vowel_index + 1, context_end, cache)
}

#[requires(index < end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn initial_rafsi_reaches_end_on_y(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    // The end came from a rafsi recognizer. y-rafsi, hy-rafsi, brivla rafsi,
    // and fu'ivla rafsi all end on y. A y-less rafsi does not.
    y_rafsi_slice(chars, index, end)
        || hy_rafsi_slice(chars, index, end)
        || extended_rafsi_ends_until_with_cache(chars, index, end, cache).contains(&end)
        || stressed_extended_rafsi_ends_until_with_cache(chars, index, end, cache).contains(&end)
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn is_lujvo_final_rafsi_alone_until(chars: &[char], index: usize, end: usize) -> bool {
    is_cvv_final_rafsi_slice(chars, index, end)
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.iter().all(|next| *next > index && *next <= end))]
fn initial_rafsi_ends_until_with_cache(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Vec<usize> {
    if cache.is_cancelled() {
        return Vec::new();
    }

    if let Some(ends) = cache.initial_rafsi_ends(index, end) {
        return ends.to_vec();
    }
    let mut ends = Vec::new();
    let extended = extended_rafsi_ends_until_with_cache(chars, index, end, cache);
    ends.extend(extended.iter().copied());
    ends.extend(
        y_rafsi_ends(chars, index)
            .into_iter()
            .filter(|next| *next <= end),
    );
    if !any_extended_rafsi_starts_until_with_cache(chars, index, end, cache) {
        ends.extend(y_less_rafsi_ends(chars, index).into_iter().filter(|next| {
            *next <= end && !any_extended_rafsi_starts_until_with_cache(chars, *next, end, cache)
        }));
    }

    let mut preferred_ends = Vec::new();
    for end in ends {
        if !preferred_ends.contains(&end) {
            preferred_ends.push(end);
        }
    }
    cache.record_initial_rafsi_ends(index, end, &preferred_ends);
    preferred_ends
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn any_extended_rafsi_starts_until_with_cache(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.any_extended_rafsi_starts(index, end) {
        return result;
    }
    let result = index < end
        && (is_fuhivla_shape_slice_with_cache(chars, index, end, cache)
            || !extended_rafsi_ends_until_with_cache(chars, index, end, cache).is_empty()
            || !stressed_extended_rafsi_ends_until_with_cache(chars, index, end, cache).is_empty());
    cache.record_any_extended_rafsi_starts(index, end, result);
    result
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.iter().all(|next| *next > index && *next <= end))]
fn stressed_extended_rafsi_ends_until_with_cache(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Vec<usize> {
    if cache.is_cancelled() {
        return Vec::new();
    }

    if let Some(ends) = cache.stressed_extended_rafsi_ends(index, end) {
        return ends.to_vec();
    }
    let ends = ((index + 1)..=end)
        .filter(|next| stressed_extended_rafsi_slice_with_cache(chars, index, *next, end, cache))
        .collect::<Vec<_>>();
    cache.record_stressed_extended_rafsi_ends(index, end, &ends);
    ends
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.iter().all(|next| *next > index && *next <= end))]
fn extended_rafsi_ends_until_with_cache(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Vec<usize> {
    if cache.is_cancelled() {
        return Vec::new();
    }

    if let Some(ends) = cache.extended_rafsi_ends(index, end) {
        return ends.to_vec();
    }
    let mut ends = Vec::new();
    for base_end in fuhivla_rafsi_base_ends_until_with_cache(chars, index, end, cache) {
        if let Some(next) = rafsi_hyphen_end(chars, base_end)
            && next <= end
        {
            ends.push(next);
        }
    }
    // brivla_rafsi <- &(syllable consonantal_syllable* syllable) brivla_head h y h?
    // The `&` is at the rafsi start and reads to the classified end.
    if brivla_rafsi_two_syllable_lookahead(chars, index, end) {
        for base_end in brivla_head_ends_until_with_cache(chars, index, end, end, cache)
            .iter()
            .copied()
        {
            if chars[index..base_end].iter().any(|value| is_y(*value)) {
                continue;
            }
            if let Some(next) = hy_rafsi_hyphen_end(chars, base_end)
                && next <= end
            {
                ends.push(next);
            }
        }
    }
    cache.record_extended_rafsi_ends(index, end, &ends);
    ends
}

#[requires(index <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn brivla_rafsi_two_syllable_lookahead(chars: &[char], index: usize, lookahead_end: usize) -> bool {
    // &(syllable consonantal_syllable* syllable)
    // `syllable` excludes a `y` nucleus. Try the second vowel syllable before
    // a consonantal syllable, so the consonantal syllable cannot consume it.
    if index >= lookahead_end {
        return false;
    }
    syllable_ends(chars, index, lookahead_end, lookahead_end)
        .into_iter()
        .any(|first_end| syllable_after_consonantal_chain(chars, first_end, lookahead_end))
}

#[requires(index <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn syllable_after_consonantal_chain(chars: &[char], index: usize, lookahead_end: usize) -> bool {
    if syllable_ends(chars, index, lookahead_end, lookahead_end)
        .iter()
        .any(|syllable_end| *syllable_end > index)
    {
        return true;
    }
    consonantal_syllable_ends(chars, index, lookahead_end, lookahead_end)
        .into_iter()
        .any(|next| next > index && syllable_after_consonantal_chain(chars, next, lookahead_end))
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.is_none_or(|split| split > start && split < end))]
fn stressed_initial_rafsi_short_final_split_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Option<usize> {
    if cache.is_cancelled() {
        return None;
    }

    (start + 1..end).rev().find(|split| {
        is_short_final_rafsi_slice(chars, *split, end)
            && stressed_initial_rafsi_slice_with_cache(chars, start, *split, end, cache)
    })
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn stressed_initial_rafsi_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    stressed_extended_rafsi_slice_with_cache(chars, start, end, lookahead_end, cache)
        || stressed_y_rafsi_slice(chars, start, end)
        || stressed_y_less_rafsi_slice(chars, start, end)
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn stressed_extended_rafsi_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    stressed_brivla_rafsi_slice_with_cache(chars, start, end, lookahead_end, cache)
        || stressed_fuhivla_rafsi_slice_with_cache(chars, start, end, lookahead_end, cache)
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

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn stressed_brivla_rafsi_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    // stressed_brivla_rafsi <- &unstressed_syllable brivla_head stressed_syllable h y
    // The two-syllable lookahead is only on unstressed `brivla_rafsi`.
    // This rule keeps `&unstressed_syllable`, which reads to the pause, not to `'`.
    if end < start + 3
        || chars.get(end - 2) != Some(&'\'')
        || !chars.get(end - 1).is_some_and(|value| is_y(*value))
        || unstressed_syllable_ends_for_fuhivla(chars, start, lookahead_end, lookahead_end)
            .is_empty()
    {
        return false;
    }
    let stress_end = end - 2;
    brivla_head_ends_until_with_cache(chars, start, stress_end, lookahead_end, cache)
        .iter()
        .copied()
        .filter(|head_end| *head_end < stress_end)
        .any(|head_end| {
            stressed_syllable_ends_for_fuhivla(chars, head_end, stress_end, lookahead_end)
                .into_iter()
                .any(|syllable_end| syllable_end == stress_end)
        })
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn stressed_fuhivla_rafsi_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    // stressed_fuhivla_rafsi <- fuhivla_head stressed_syllable consonantal_syllable* !h onset y
    // `end` is the `y`. Lookaheads read to `lookahead_end`, where pause applies.
    // The onset and the `y` stay inside this rafsi.
    if end < start + 2 || !chars.get(end - 1).is_some_and(|value| is_y(*value)) {
        return false;
    }
    let base_end = end - 1;
    fuhivla_head_ends_until_with_cache(chars, start, base_end, lookahead_end, cache)
        .iter()
        .copied()
        .filter(|head_end| *head_end < base_end)
        .any(|head_end| {
            stressed_syllable_ends_for_fuhivla(chars, head_end, base_end, lookahead_end)
                .into_iter()
                .any(|stressed_end| {
                    consonantal_chain_then_onset(chars, stressed_end, base_end, lookahead_end)
                })
        })
}

#[requires(index <= span_end && span_end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn consonantal_chain_then_onset(
    chars: &[char],
    index: usize,
    span_end: usize,
    lookahead_end: usize,
) -> bool {
    if chars.get(index) != Some(&'\'') && has_parse_onset_end(chars, index, span_end) {
        return true;
    }
    consonantal_syllable_ends(chars, index, span_end, lookahead_end)
        .into_iter()
        .any(|next| {
            next > index && consonantal_chain_then_onset(chars, next, span_end, lookahead_end)
        })
}

#[requires(index <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end < lookahead_end))]
fn fuhivla_rafsi_base_ends_until_with_cache(
    chars: &[char],
    index: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Vec<usize> {
    if cache.is_cancelled() {
        return Vec::new();
    }

    if let Some(ends) = cache.fuhivla_rafsi_base_ends(index, lookahead_end) {
        return ends.to_vec();
    }
    let mut ends = Vec::new();
    for base_end in (index + 1)..lookahead_end {
        if rafsi_hyphen_end(chars, base_end).is_some()
            && fuhivla_rafsi_base_slice_with_cache(chars, index, base_end, lookahead_end, cache)
        {
            ends.push(base_end);
        }
    }
    cache.record_fuhivla_rafsi_base_ends(index, lookahead_end, &ends);
    ends
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn fuhivla_rafsi_base_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    // fuhivla_rafsi <- &unstressed_syllable fuhivla_head !h onset y h?
    // `&unstressed_syllable` and `!rafsi_string` read to `lookahead_end`.
    // The head stops before the onset. The onset stays inside this base.
    if start >= end
        || chars[start..end].iter().any(|value| is_y(*value))
        || unstressed_syllable_ends_for_fuhivla(chars, start, lookahead_end, lookahead_end)
            .is_empty()
    {
        return false;
    }
    fuhivla_head_ends_until_with_cache(chars, start, end, lookahead_end, cache)
        .iter()
        .copied()
        .filter(|head_end| *head_end > start && *head_end < end)
        .any(|head_end| {
            chars.get(head_end) != Some(&'\'') && has_parse_onset_end(chars, head_end, end)
        })
}

#[requires(index <= span_end && span_end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.iter().all(|head_end| *head_end >= index && *head_end <= span_end))]
fn brivla_head_ends_until_with_cache(
    chars: &[char],
    index: usize,
    span_end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Rc<[usize]> {
    if cache.is_cancelled() {
        return Rc::from([]);
    }

    if let Some(ends) = cache.brivla_head_ends(index, span_end, lookahead_end) {
        return ends;
    }
    // brivla_head <- !cmavo !slinkuhi !h &onset unstressed_syllable*
    // The three predicates are lookaheads. They read to `lookahead_end`.
    // The syllables themselves stop at `span_end`.
    let ends = if index > span_end
        || chars.get(index) == Some(&'\'')
        || !starts_with_onset(chars, index)
        || cmavo_word_slice_with_cache(chars, index, lookahead_end, cache)
        || slinkuhi_slice_with_cache(chars, index, lookahead_end, cache)
    {
        Vec::new()
    } else {
        brivla_head_ends_without_lookahead(chars, index, span_end, lookahead_end)
    };
    cache.record_brivla_head_ends(index, span_end, lookahead_end, &ends)
}

#[requires(index <= span_end && span_end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.iter().all(|head_end| *head_end >= index && *head_end <= span_end))]
fn fuhivla_head_ends_until_with_cache(
    chars: &[char],
    index: usize,
    span_end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Rc<[usize]> {
    if cache.is_cancelled() {
        return Rc::from([]);
    }

    if let Some(ends) = cache.fuhivla_head_ends(index, span_end, lookahead_end) {
        return ends;
    }
    // fuhivla_head <- !rafsi_string brivla_head
    // `!rafsi_string` reads past the head, through a following `y`.
    if rafsi_string_starts_slice_with_cache(chars, index, lookahead_end, cache) {
        cache.record_fuhivla_head_ends(index, span_end, lookahead_end, &[])
    } else {
        let shared =
            brivla_head_ends_until_with_cache(chars, index, span_end, lookahead_end, cache);
        cache.record_fuhivla_head_ends_shared(index, span_end, lookahead_end, shared)
    }
}

#[requires(index <= span_end && span_end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(ret.iter().all(|head_end| *head_end >= index && *head_end <= span_end))]
fn brivla_head_ends_without_lookahead(
    chars: &[char],
    index: usize,
    span_end: usize,
    lookahead_end: usize,
) -> Vec<usize> {
    let mut ends = vec![index];
    let mut stack = vec![index];
    while let Some(start) = stack.pop() {
        for syllable_end in
            unstressed_syllable_ends_for_fuhivla(chars, start, span_end, lookahead_end)
        {
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
    let mut cache = MorphologyRecognitionCache::new(chars.len());
    slinkuhi_slice_with_cache(chars, start, end, &mut cache)
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn slinkuhi_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.slinkuhi(start, end) {
        return result;
    }
    let result = start < end
        && !rafsi_string_starts_slice_with_cache(chars, start, end, cache)
        && is_consonant(chars[start])
        && rafsi_string_starts_slice_with_cache(chars, start + 1, end, cache);
    cache.record_slinkuhi(start, end, result);
    result
}

#[cfg(test)]
#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn rafsi_string_starts_slice(chars: &[char], start: usize, end: usize) -> bool {
    let mut cache = MorphologyRecognitionCache::new(chars.len());
    rafsi_string_starts_slice_with_cache(chars, start, end, &mut cache)
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn rafsi_string_starts_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.rafsi_string_starts(start, end) {
        return result;
    }
    let result = start < end
        && ((start + 1)..=end).any(|word_end| {
            rafsi_string_slice_for_lookahead_with_cache(chars, start, word_end, end, cache)
        });
    cache.record_rafsi_string_starts(start, end, result);
    result
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn rafsi_string_slice(chars: &[char], start: usize, end: usize) -> bool {
    let mut cache = MorphologyRecognitionCache::new(chars.len());
    rafsi_string_slice_for_lookahead_with_cache(chars, start, end, end, &mut cache)
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn rafsi_string_slice_for_lookahead_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    // The lookahead span is already the third key of `rafsi_string_from`.
    // A second table stored the same bool again.
    rafsi_string_from_with_cache(chars, start, end, lookahead_end, cache)
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn rafsi_string_from_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.rafsi_string_from(start, end, lookahead_end) {
        return result;
    }
    let result = if start >= end {
        false
    } else {
        let cursor = greedy_landing_cursor(chars, start, end, lookahead_end, cache);
        rafsi_string_ending_with_cache(chars, cursor, end, lookahead_end, cache)
    };
    cache.record_rafsi_string_from(start, end, lookahead_end, result);
    result
}

/// Cursor where the greedy y-less walk from `start` stops under `end`.
///
/// For one lookahead, the first unstressed y-less end that fits is stable as
/// the cap grows. The walk capped at the lookahead is therefore a chain. A
/// shorter cap follows that chain until the next jump would pass the cap. The
/// only departure is a lower-priority end that still fits under the shorter cap.
#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret >= start && ret <= end)]
fn greedy_landing_cursor(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> usize {
    let chain = greedy_max_chain(chars, start, lookahead_end, cache);
    let index = chain
        .partition_point(|cursor| *cursor <= end)
        .saturating_sub(1);
    let cursor = chain[index];
    if let Some(&jump) = chain.get(index + 1)
        && jump > end
        && let Some(shorter) = greedy_shorter_rafsi_end(chars, cursor, end, lookahead_end, cache)
    {
        return greedy_landing_cursor(chars, shorter, end, lookahead_end, cache);
    }
    cursor
}

#[requires(start <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.first().copied() == Some(start))]
#[ensures(ret.windows(2).all(|pair| pair[0] < pair[1] && pair[1] <= lookahead_end))]
fn greedy_max_chain(
    chars: &[char],
    start: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Rc<[usize]> {
    if let Some(chain) = cache.greedy_rafsi_chain(start, lookahead_end) {
        return chain;
    }
    let mut cursors = vec![start];
    let mut cursor = start;
    while cursor < lookahead_end {
        let Some(next) =
            greedy_y_less_rafsi_end(chars, cursor, lookahead_end, lookahead_end, cache)
        else {
            break;
        };
        cursor = next;
        cursors.push(cursor);
    }
    cache.record_greedy_rafsi_chain(start, lookahead_end, &cursors)
}

#[requires(index <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.is_none_or(|next| next > index && next <= end))]
fn greedy_shorter_rafsi_end(
    chars: &[char],
    index: usize,
    end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Option<usize> {
    let mut passed_primary = false;
    for next in y_less_rafsi_ends(chars, index) {
        if next > lookahead_end
            || !y_less_rafsi_is_unstressed_in_context(chars, index, next, lookahead_end, cache)
        {
            continue;
        }
        if !passed_primary {
            passed_primary = true;
            continue;
        }
        if next <= end {
            return Some(next);
        }
    }
    None
}

#[requires(index <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.is_none_or(|next| next > index && next <= end))]
fn greedy_y_less_rafsi_end(
    chars: &[char],
    index: usize,
    end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> Option<usize> {
    y_less_rafsi_ends(chars, index).into_iter().find(|next| {
        *next <= end
            && y_less_rafsi_is_unstressed_in_context(chars, index, *next, lookahead_end, cache)
    })
}

#[requires(start <= end && end <= context_end && context_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn y_less_rafsi_is_unstressed_in_context(
    chars: &[char],
    start: usize,
    end: usize,
    context_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if let Some(result) = cache.y_less_rafsi_unstressed(start, end, context_end) {
        return result;
    }
    let result = if cvc_rafsi_end(chars, start) == Some(end) {
        !nucleus_is_stressed_in_context(chars, start + 1, start + 2, context_end, cache)
    } else if ccv_rafsi_end(chars, start) == Some(end) {
        !nucleus_is_stressed_in_context(chars, start + 2, start + 3, context_end, cache)
    } else {
        cvv_rafsi_is_unstressed_in_context(chars, start, end, context_end, cache)
    };
    cache.record_y_less_rafsi_unstressed(start, end, context_end, result);
    result
}

#[requires(start <= end && end <= context_end && context_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn cvv_rafsi_is_unstressed_in_context(
    chars: &[char],
    start: usize,
    end: usize,
    context_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if let Some(result) = cache.cvv_rafsi_unstressed(start, end, context_end) {
        return result;
    }
    let result = cvv_rafsi_is_unstressed_uncached(chars, start, end, context_end, cache);
    cache.record_cvv_rafsi_unstressed(start, end, context_end, result);
    result
}

#[requires(start <= end && end <= context_end && context_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn cvv_rafsi_is_unstressed_uncached(
    chars: &[char],
    start: usize,
    end: usize,
    context_end: usize,
    cache: &mut MorphologyRecognitionCache,
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
            return !nucleus_is_stressed_in_context(
                chars,
                start + 1,
                start + 2,
                context_end,
                cache,
            ) && !nucleus_is_stressed_in_context(
                chars,
                start + 3,
                start + 4,
                context_end,
                cache,
            );
        }
        if vowel_end == start + 3 {
            return !nucleus_is_stressed_in_context(
                chars,
                start + 1,
                vowel_end,
                context_end,
                cache,
            );
        }
    }
    false
}

#[requires(start < end && end <= context_end && context_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn nucleus_is_stressed_in_context(
    chars: &[char],
    start: usize,
    end: usize,
    context_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if let Some(result) = cache.nucleus_is_stressed(start, end, context_end) {
        return result;
    }
    let result = syllable_has_explicit_stress(chars, start, end)
        || nucleus_has_implicit_stress(chars, end, context_end, cache);
    cache.record_nucleus_is_stressed(start, end, context_end, result);
    result
}

#[requires(index <= context_end && context_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn nucleus_has_implicit_stress(
    chars: &[char],
    index: usize,
    context_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if let Some(result) = cache.nucleus_has_implicit_stress(index, context_end) {
        return result;
    }
    let result = nucleus_has_implicit_stress_uncached(chars, index, context_end, cache);
    cache.record_nucleus_has_implicit_stress(index, context_end, result);
    result
}

#[requires(index <= context_end && context_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn nucleus_has_implicit_stress_uncached(
    chars: &[char],
    index: usize,
    context_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
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
        if let Some(glide_end) = parse_glide_end(chars, cursor)
            && glide_end <= context_end
        {
            stack.push(glide_end);
        }
    }

    after_consonants_or_glides
        .into_iter()
        .any(|cursor| stress_tail_reaches_boundary(chars, cursor, context_end, cache))
}

#[requires(index <= context_end && context_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn stress_tail_reaches_boundary(
    chars: &[char],
    index: usize,
    context_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if let Some(result) = cache.stress_tail_reaches_boundary(index, context_end) {
        return result;
    }
    let result = stress_tail_reaches_boundary_uncached(chars, index, context_end);
    cache.record_stress_tail_reaches_boundary(index, context_end, result);
    result
}

#[requires(index <= context_end && context_end <= chars.len())]
#[ensures(true)]
fn stress_tail_reaches_boundary_uncached(chars: &[char], index: usize, context_end: usize) -> bool {
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
            syllable_ends(chars, syllable_start, context_end, context_end)
                .into_iter()
                .any(|syllable_end| syllable_end == context_end)
        })
    })
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn initial_pair_y_ending_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    // rafsi_string end: stressed_y_less_rafsi? initial_pair y
    // The pair is the two consonants immediately before `y`.
    // A stressed CCV rafsi ends in a vowel, so it cannot eat that pair.
    if end - start < 3 || !chars.get(end - 1).is_some_and(|value| is_y(*value)) {
        return false;
    }
    let pair_start = end - 3;
    if !initial_pair_chars(chars[pair_start], chars[pair_start + 1]) {
        return false;
    }
    if pair_start == start {
        return true;
    }
    y_less_rafsi_ends(chars, start).into_iter().any(|mid| {
        mid == pair_start
            && !y_less_rafsi_is_unstressed_in_context(chars, start, mid, lookahead_end, cache)
    })
}

#[requires(start <= end && end <= lookahead_end && lookahead_end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn rafsi_string_ending_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    lookahead_end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.rafsi_string_ending(start, end, lookahead_end) {
        return result;
    }
    let result = (is_gismu_slice_for_rafsi_string(chars, start, end, lookahead_end)
        && post_word_slice_with_cache(chars, end, lookahead_end, cache))
        || (is_cvv_final_rafsi_slice_for_rafsi_string(chars, start, end, lookahead_end)
            && post_word_slice_with_cache(chars, end, lookahead_end, cache))
        || y_less_rafsi_ends(chars, start).into_iter().any(|mid| {
            mid > start
                && mid < end
                && is_short_final_rafsi_slice_for_rafsi_string(chars, mid, end, lookahead_end)
                && post_word_slice_with_cache(chars, end, lookahead_end, cache)
        })
        || y_rafsi_slice(chars, start, end)
        || hy_rafsi_slice(chars, start, end)
        || initial_pair_y_ending_with_cache(chars, start, end, lookahead_end, cache);
    cache.record_rafsi_string_ending(start, end, lookahead_end, result);
    result
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
        || stressed_syllable_has_implicit_stress(chars, syllable_end, context_end)
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
        && !stressed_syllable_has_implicit_stress(chars, end, context_end)
}

#[cfg(test)]
#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn post_word_slice(chars: &[char], index: usize, end: usize) -> bool {
    let mut cache = MorphologyRecognitionCache::new(chars.len());
    post_word_slice_with_cache(chars, index, end, &mut cache)
}

#[requires(index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn post_word_slice_with_cache(
    chars: &[char],
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.post_word(index, end) {
        return result;
    }
    let result = match next_non_comma_index(chars, index) {
        None => true,
        Some(next) => {
            next >= end
                || (!starts_with_pause_required_nucleus(chars, next)
                    && lojban_word_slice_with_cache(chars, next, end, cache))
        }
    };
    cache.record_post_word(index, end, result);
    result
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
fn is_cultural_rafsi_slice(chars: &[char], start: usize, end: usize) -> bool {
    if end <= start {
        return false;
    }
    let Some(base_end) = rafsi_hyphen_start(chars, start, end) else {
        return false;
    };
    let base = &chars[start..base_end];
    (base.len() == 5
        && initial_pair_chars(base[0], base[1])
        && is_vowel(base[2])
        && is_vowel(base[3])
        && is_consonant(base[4]))
        || (base.len() == 6
            && initial_pair_chars(base[0], base[1])
            && is_vowel(base[2])
            && base[3] == '\''
            && is_vowel(base[4])
            && is_consonant(base[5]))
}

#[requires(start <= end && end <= chars.len())]
#[ensures(ret.is_none_or(|index| start < index && index < end))]
fn rafsi_hyphen_start(chars: &[char], start: usize, end: usize) -> Option<usize> {
    (start + 1..end).find(|index| is_rafsi_hyphen_start(chars, *index))
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
    ends.extend(initial_cvv_rafsi_ends(chars, index));
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
        .chain(cvv_rafsi_base_ends(chars, index))
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
    cvv_rafsi_base_ends(chars, index)
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn initial_cvv_rafsi_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    for base_end in cvv_rafsi_base_ends(chars, index) {
        if let Some(end) = r_hyphen_end(chars, base_end).or_else(|| n_hyphen_end(chars, base_end)) {
            ends.push(end);
        }
        ends.push(base_end);
    }
    ends
}

#[requires(index <= chars.len())]
#[ensures(ret.iter().all(|end| *end > index && *end <= chars.len()))]
fn cvv_rafsi_base_ends(chars: &[char], index: usize) -> Vec<usize> {
    let mut ends = Vec::new();
    if index < chars.len() && is_consonant(chars[index]) {
        ends.extend(vowel_pair_ends(chars, index + 1));
    }
    ends
}

#[requires(start < end && end <= chars.len())]
#[ensures(ret.is_none_or(|hyphen_start| start < hyphen_start && hyphen_start < end))]
fn cvv_rafsi_hyphen_start_for_initial_end(
    chars: &[char],
    start: usize,
    end: usize,
) -> Option<usize> {
    cvv_rafsi_base_ends(chars, start)
        .into_iter()
        .find(|base_end| {
            r_hyphen_end(chars, *base_end) == Some(end)
                || n_hyphen_end(chars, *base_end) == Some(end)
        })
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
    end > start && is_gismu_chars(&chars[start..end])
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

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn is_fuhivla_shape_slice(chars: &[char], start: usize, end: usize) -> bool {
    let mut cache = MorphologyRecognitionCache::new(chars.len());
    is_fuhivla_shape_slice_with_cache(chars, start, end, &mut cache)
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn is_fuhivla_shape_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    is_fuhivla_shape_slice_with_y_policy_with_cache(chars, start, end, false, cache)
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn fuhivla_shape_slice_rejected_by_y(chars: &[char], start: usize, end: usize) -> bool {
    let mut cache = MorphologyRecognitionCache::new(chars.len());
    fuhivla_shape_slice_rejected_by_y_with_cache(chars, start, end, &mut cache)
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn fuhivla_shape_slice_rejected_by_y_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    start < end
        && chars[start..end].iter().any(|value| is_y(*value))
        && is_fuhivla_shape_slice_with_y_policy_with_cache(chars, start, end, true, cache)
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn is_fuhivla_shape_slice_with_y_policy_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    allows_y: bool,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.fuhivla_shape_y(start, end, allows_y) {
        return result;
    }
    // The checks stay in the original order. Each one can call back into this
    // cycle, so a later check must not run when an earlier check already failed.
    let result = if end <= start
        || end - start < 4
        || !ends_with_vocalic_nucleus(chars, start, end)
        || count_vocalic_nuclei(chars, start, end) < 2
        || (!allows_y && chars[start..end].iter().any(|value| is_y(*value)))
        || has_vowel_hiatus(&chars[start..end])
        || !parse_fuhivla_shape_with_cache(chars, start, end, cache)
        || rafsi_string_slice_for_lookahead_with_cache(chars, start, end, end, cache)
        || slinkuhi_slice_with_cache(chars, start, end, cache)
        || has_blocking_cmavo_prefix_slice_with_cache(chars, start, end, cache)
        || invalid_initial_rafsi_continuation(chars, start, end)
        || invalid_vowel_initial_fuhivla_shape_with_cache(chars, start, end, cache)
        || !starts_with_valid_word_onset(chars, start)
    {
        false
    } else {
        let slice = &chars[start..end];
        slice.iter().any(|value| is_consonant(*value))
            && has_consonant_cluster(slice)
            && !is_cmavo_slice(chars, start, end)
    };
    cache.record_fuhivla_shape_y(start, end, allows_y, result);
    result
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn has_blocking_cmavo_prefix_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if start >= end {
        return false;
    }
    cmavo_word_slice_with_cache(chars, start, end, cache)
}

#[requires(prefix_start <= prefix_end && prefix_end <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn cmavo_post_word_slice_with_cache(
    chars: &[char],
    prefix_start: usize,
    prefix_end: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    // `mut` is used on wasm32, where the closure is called in place. Native
    // code moves the closure into `stacker::maybe_grow`.
    #[allow(unused_mut)]
    let mut recognize = || {
        if let Some(result) = cache.cmavo_post_word(prefix_start, prefix_end, end) {
            return result;
        }
        let result = match next_non_comma_index(chars, prefix_end) {
            None => true,
            Some(index) if index >= end => true,
            Some(index) => {
                if starts_with_pause_required_nucleus(chars, index)
                    && !indicator_cmavo_boundary_slice_with_cache(
                        chars,
                        prefix_start,
                        prefix_end,
                        index,
                        end,
                        cache,
                    )
                {
                    false
                } else {
                    lojban_word_slice_with_cache(chars, index, end, cache)
                }
            }
        };
        cache.record_cmavo_post_word(prefix_start, prefix_end, end, result);
        result
    };
    // The post-word check calls `lojban_word` on the next word and stays on
    // the stack until that word returns. Grow the native stack when less than
    // 64 KiB remains. wasm32 has no auxiliary stack, so the call stays on the
    // host stack.
    #[cfg(not(target_arch = "wasm32"))]
    {
        return stacker::maybe_grow(64 * 1024, 1024 * 1024, recognize);
    }
    #[cfg(target_arch = "wasm32")]
    {
        recognize()
    }
}

#[requires(prefix_start <= prefix_end && prefix_end <= index && index <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn indicator_cmavo_boundary_slice_with_cache(
    chars: &[char],
    prefix_start: usize,
    prefix_end: usize,
    index: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.indicator_boundary(prefix_start, prefix_end, index, end) {
        return result;
    }
    let result = is_indicator_cmavo_slice(chars, prefix_start, prefix_end)
        && !starts_with_nucleus(chars, index)
        && indicator_cmavo_starts_slice_with_cache(chars, index, end, cache);
    cache.record_indicator_boundary(prefix_start, prefix_end, index, end, result);
    result
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn indicator_cmavo_starts_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.indicator_cmavo_starts(start, end) {
        return result;
    }
    let result = start < end
        && ((start + 1)..=end).any(|word_end| {
            is_indicator_cmavo_slice(chars, start, word_end)
                && cmavo_post_word_slice_with_cache(chars, start, word_end, end, cache)
        });
    cache.record_indicator_cmavo_starts(start, end, result);
    result
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn is_indicator_cmavo_slice(chars: &[char], start: usize, end: usize) -> bool {
    if start >= end {
        return false;
    }
    parse_cmavo_form_chars(&chars[start..end])
        .and_then(|phonemes| Cmavo::from_text(&phonemes))
        .is_some_and(|cmavo| cmavo.is_selmaho(Selmaho::Ui) || cmavo.is_selmaho(Selmaho::Cai))
}

#[cfg(test)]
#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn lojban_word_slice(chars: &[char], start: usize, end: usize) -> bool {
    let mut cache = MorphologyRecognitionCache::new(chars.len());
    lojban_word_slice_with_cache(chars, start, end, &mut cache)
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn lojban_word_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    // `mut` is used on wasm32, where the closure is called in place. Native
    // code moves the closure into `stacker::maybe_grow`.
    #[allow(unused_mut)]
    let mut recognize = || {
        if let Some(result) = cache.lojban_word(start, end) {
            return result;
        }
        let result = start < end
            && (is_cmevla_slice(chars, start, end)
                || cmavo_word_slice_with_cache(chars, start, end, cache)
                || brivla_word_slice_with_cache(chars, start, end, cache));
        cache.record_lojban_word(start, end, result);
        result
    };
    // Classifying one word calls back into `cmavo_word` for the same span and
    // for each later word. Grow the native stack when less than 64 KiB remains.
    // wasm32 has no auxiliary stack, so the call stays on the host stack.
    #[cfg(not(target_arch = "wasm32"))]
    {
        return stacker::maybe_grow(64 * 1024, 1024 * 1024, recognize);
    }
    #[cfg(target_arch = "wasm32")]
    {
        recognize()
    }
}

#[cfg(test)]
#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn cmavo_word_slice(chars: &[char], start: usize, end: usize) -> bool {
    let mut cache = MorphologyRecognitionCache::new(chars.len());
    cmavo_word_slice_with_cache(chars, start, end, &mut cache)
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn cmavo_word_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    // `mut` is used on wasm32, where the closure is called in place. Native
    // code moves the closure into `stacker::maybe_grow`.
    #[allow(unused_mut)]
    let mut recognize = || {
        if let Some(result) = cache.cmavo_word(start, end) {
            return result;
        }
        let result = if start >= end
            || is_cmevla_slice(chars, start, end)
            || starts_with_cvcy_lujvo_chars_with_cache(chars, start, end, cache)
        {
            false
        } else {
            ((start + 1)..=end).any(|word_end| {
                is_cmavo_slice(chars, start, word_end)
                    && cmavo_post_word_slice_with_cache(chars, start, word_end, end, cache)
            })
        };
        cache.record_cmavo_word(start, end, result);
        result
    };
    // Each accepted cmavo calls `cmavo_post_word`, which recognizes the next
    // word before this call returns. Grow the native stack when less than 64
    // KiB remains. wasm32 has no auxiliary stack, so the call stays on the
    // host stack.
    #[cfg(not(target_arch = "wasm32"))]
    {
        return stacker::maybe_grow(64 * 1024, 1024 * 1024, recognize);
    }
    #[cfg(target_arch = "wasm32")]
    {
        recognize()
    }
}

#[cfg(test)]
#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn brivla_word_slice(chars: &[char], start: usize, end: usize) -> bool {
    let mut cache = MorphologyRecognitionCache::new(chars.len());
    brivla_word_slice_with_cache(chars, start, end, &mut cache)
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn brivla_word_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.brivla_word(start, end) {
        return result;
    }
    let result = !cmavo_word_slice_with_cache(chars, start, end, cache)
        && (is_gismu_slice(chars, start, end)
            || is_lujvo_slice_with_cache(chars, start, end, cache)
            || is_fuhivla_shape_slice_with_cache(chars, start, end, cache));
    cache.record_brivla_word(start, end, result);
    result
}

#[cfg(test)]
#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn is_lujvo_slice(chars: &[char], start: usize, end: usize) -> bool {
    let mut cache = MorphologyRecognitionCache::new(chars.len());
    is_lujvo_slice_with_cache(chars, start, end, &mut cache)
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn is_lujvo_slice_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.lujvo_slice(start, end) {
        return result;
    }
    let result = chars[start..end].iter().all(|value| is_lujvo_char(*value))
        && !cmavo_word_slice_with_cache(chars, start, end, cache)
        && !is_fuhivla_shape_slice_with_cache(chars, start, end, cache)
        && lujvo_from_until_with_cache(chars, start, end, false, cache);
    cache.record_lujvo_slice(start, end, result);
    result
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
pub(crate) fn starts_with_pause_required_nucleus(chars: &[char], start: usize) -> bool {
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
        if let Some((_, nucleus_end)) = parse_diphthong_end(chars, index)
            && nucleus_end <= end
        {
            count += 1;
            index = nucleus_end;
        } else if let Some(nucleus_end) = parse_single_vowel_end(chars, index)
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
    // A glide after this `r` stays on the experimental consonant-glide warning.
    // Camxes rejects that glide by `r <- comma* [rR] !h !glide !r`.
    // `kanria` is the case. The continuation is `r` and then a glide.
    // `starts_cgv_sequence` would reject it, so the warning would not run.
    // `kanrtlti` does not use this return. There `tl` is a consonantal syllable.
    // The onset parser already bans `tl` as an onset.
    if chars[index] == 'r' {
        return false;
    }
    // A consonant and then a glide matches `!glide` on that consonant.
    // `glide <- (i / u) &nucleus`.
    // `j` or `g` before that glide is the same test. `cmajria` and `cmagnia`
    // take the warning. `cmajriti` and `cmagniti` have a vowel, not a glide.
    starts_cgv_sequence(chars, index, end)
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn starts_cgv_sequence(chars: &[char], index: usize, end: usize) -> bool {
    index < end && is_consonant(chars[index]) && starts_glide(chars, index + 1)
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn invalid_vowel_initial_fuhivla_shape_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    chars
        .get(start)
        .is_some_and(|value| is_vowel(*value) || matches!(value, 'ĭ' | 'ŭ'))
        && !parse_fuhivla_shape_with_cache(chars, start, end, cache)
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn parse_fuhivla_shape_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.fuhivla_shape(start, end) {
        return result;
    }
    let result = parse_fuhivla_shape_with_head_policy_with_cache(
        chars,
        start,
        end,
        FuhivlaHeadPolicy::Standard,
        cache,
    ) || parse_experimental_cgv_fuhivla_shape_with_cache(chars, start, end, cache);
    cache.record_fuhivla_shape(start, end, result);
    result
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn parse_experimental_cgv_fuhivla_shape_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.experimental_cgv(start, end) {
        return result;
    }
    let result = has_cgv_sequence_slice(chars, start, end)
        && parse_fuhivla_shape_with_head_policy_with_cache(
            chars,
            start,
            end,
            FuhivlaHeadPolicy::ExperimentalCgv,
            cache,
        );
    cache.record_experimental_cgv(start, end, result);
    result
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn has_cgv_sequence_slice(chars: &[char], start: usize, end: usize) -> bool {
    cgv_range(&chars[start..end]).is_some()
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(true)]
fn parse_fuhivla_shape_with_head_policy_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    head_policy: FuhivlaHeadPolicy,
    cache: &mut MorphologyRecognitionCache,
) -> bool {
    if cache.is_cancelled() {
        return false;
    }

    if let Some(result) = cache.fuhivla_head_policy(start, end, head_policy) {
        return result;
    }
    let result = fuhivla_head_ends_for_fuhivla_with_cache(chars, start, end, head_policy, cache)
        .iter()
        .copied()
        .any(|head_end| {
            stressed_syllable_ends_for_fuhivla(chars, head_end, end, end)
                .into_iter()
                .any(|stressed_end| consonantal_chain_then_final(chars, stressed_end, end))
        });
    cache.record_fuhivla_head_policy(start, end, head_policy, result);
    result
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum FuhivlaHeadPolicy {
    Standard,
    ExperimentalCgv,
}

#[requires(start <= end && end <= chars.len())]
#[requires(cache.char_count == chars.len())]
#[ensures(ret.iter().all(|head_end| *head_end >= start && *head_end <= end))]
fn fuhivla_head_ends_for_fuhivla_with_cache(
    chars: &[char],
    start: usize,
    end: usize,
    head_policy: FuhivlaHeadPolicy,
    cache: &mut MorphologyRecognitionCache,
) -> Rc<[usize]> {
    if cache.is_cancelled() {
        return Rc::from([]);
    }

    match head_policy {
        FuhivlaHeadPolicy::Standard => {
            fuhivla_head_ends_until_with_cache(chars, start, end, end, cache)
        }
        FuhivlaHeadPolicy::ExperimentalCgv => {
            brivla_head_ends_until_with_cache(chars, start, end, end, cache)
        }
    }
}

#[requires(index <= span_end && span_end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(ret.iter().all(|syllable_end| *syllable_end > index && *syllable_end <= span_end))]
fn unstressed_syllable_ends_for_fuhivla(
    chars: &[char],
    index: usize,
    span_end: usize,
    lookahead_end: usize,
) -> Vec<usize> {
    let mut ends: Vec<usize> = syllable_ends(chars, index, span_end, lookahead_end)
        .into_iter()
        .filter(|syllable_end| {
            !syllable_has_explicit_stress(chars, index, *syllable_end)
                && !stressed_syllable_has_implicit_stress(chars, *syllable_end, lookahead_end)
        })
        .collect();
    ends.extend(consonantal_syllable_ends(
        chars,
        index,
        span_end,
        lookahead_end,
    ));
    ends.sort_unstable();
    ends.dedup();
    ends
}

#[requires(index <= span_end && span_end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(ret.iter().all(|syllable_end| *syllable_end > index && *syllable_end <= span_end))]
fn stressed_syllable_ends_for_fuhivla(
    chars: &[char],
    index: usize,
    span_end: usize,
    lookahead_end: usize,
) -> Vec<usize> {
    syllable_ends(chars, index, span_end, lookahead_end)
        .into_iter()
        .filter(|syllable_end| {
            syllable_has_explicit_stress(chars, index, *syllable_end)
                || stressed_syllable_has_implicit_stress(chars, *syllable_end, lookahead_end)
        })
        .collect()
}

#[requires(syllable_end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn stressed_syllable_has_implicit_stress(
    chars: &[char],
    syllable_end: usize,
    lookahead_end: usize,
) -> bool {
    // Camxes: stressed_syllable <- &stressed syllable / syllable &stress.
    // stress <- (consonant / glide)* h? y? syllable pause.
    // `lookahead_end` is that pause. The coda stays in this syllable.
    // The tail is one final syllable, or consonantal syllables and then one
    // final syllable. `h? y?` covers an apostrophe or a y before that syllable.
    if final_syllable_slice(chars, syllable_end, lookahead_end) {
        return true;
    }
    if consonantal_syllable_ends(chars, syllable_end, lookahead_end, lookahead_end)
        .into_iter()
        .any(|next| next > syllable_end && consonantal_chain_then_final(chars, next, lookahead_end))
    {
        return true;
    }
    stress_tail_through_hyphen(chars, syllable_end, lookahead_end)
}

#[requires(syllable_end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn stress_tail_through_hyphen(chars: &[char], syllable_end: usize, lookahead_end: usize) -> bool {
    // `(consonant / glide)* h? y? syllable pause`, and the tail uses `h` or `y`.
    let mut cursors = vec![syllable_end];
    let mut seen = vec![syllable_end];
    let mut read = 0;
    while read < cursors.len() {
        let cursor = cursors[read];
        read += 1;
        if cursor < lookahead_end && is_consonant(chars[cursor]) {
            let next = cursor + 1;
            if !seen.contains(&next) {
                seen.push(next);
                cursors.push(next);
            }
        }
        if let Some(glide_end) = parse_glide_end(chars, cursor)
            && glide_end <= lookahead_end
            && !seen.contains(&glide_end)
        {
            seen.push(glide_end);
            cursors.push(glide_end);
        }
    }
    cursors
        .into_iter()
        .any(|cursor| hyphen_then_final_syllable(chars, cursor, lookahead_end))
}

#[requires(index <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn hyphen_then_final_syllable(chars: &[char], index: usize, lookahead_end: usize) -> bool {
    let mut syllable_starts = Vec::new();
    if index < lookahead_end && chars[index] == '\'' {
        let after_h = index + 1;
        syllable_starts.push(after_h);
        if after_h < lookahead_end && chars.get(after_h).is_some_and(|value| is_y(*value)) {
            syllable_starts.push(after_h + 1);
        }
    } else if index < lookahead_end && chars.get(index).is_some_and(|value| is_y(*value)) {
        syllable_starts.push(index + 1);
    } else {
        return false;
    }
    syllable_starts.into_iter().any(|syllable_start| {
        syllable_start <= lookahead_end
            && syllable_ends(chars, syllable_start, lookahead_end, lookahead_end)
                .into_iter()
                .any(|syllable_end| syllable_end == lookahead_end)
    })
}

#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn consonantal_chain_then_final(chars: &[char], index: usize, end: usize) -> bool {
    if final_syllable_slice(chars, index, end) {
        return true;
    }
    consonantal_syllable_ends(chars, index, end, end)
        .into_iter()
        .any(|next| next > index && consonantal_chain_then_final(chars, next, end))
}

#[requires(start <= end && end <= chars.len())]
#[ensures(true)]
fn final_syllable_slice(chars: &[char], start: usize, end: usize) -> bool {
    !syllable_has_explicit_stress(chars, start, end)
        && brivla_onset_has_nucleus_end(chars, start, end, true)
}

#[requires(index <= span_end && span_end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(ret.iter().all(|syllable_end| *syllable_end > index && *syllable_end <= span_end))]
fn syllable_ends(
    chars: &[char],
    index: usize,
    span_end: usize,
    lookahead_end: usize,
) -> Vec<usize> {
    let mut ends = Vec::new();
    let mut onset_ends = Vec::with_capacity(6);
    let mut nucleus_ends = Vec::with_capacity(2);
    append_brivla_onset_ends_unsorted(chars, index, &mut onset_ends);
    for onset_end in onset_ends.iter().copied() {
        if chars.get(onset_end).is_some_and(|value| is_y(*value)) {
            continue;
        }
        nucleus_ends.clear();
        append_parse_nucleus_ends(chars, onset_end, &mut nucleus_ends);
        for nucleus_end in nucleus_ends.iter().copied() {
            // syllable <- onset !y nucleus coda?
            // The nucleus is inside the span. Coda lookahead reads further.
            if nucleus_end <= span_end {
                ends.push(nucleus_end);
                ends.extend(coda_ends(chars, nucleus_end, span_end, lookahead_end));
            }
        }
    }
    ends.sort_unstable();
    ends.dedup();
    ends
}

/// Both syllable flags at one index.
///
/// The sentinel at `end` is `(false, false)`. A consonant whose next character
/// is inside the span and whose onset check fails depends on the next index.
/// Every other index is closed: the onset check is bounded, and the flags do
/// not read further. The walk therefore stops at the end of that run.
#[invariant(true)]
#[derive(Clone, Copy, Debug)]
struct SyllableFlags {
    starts_any: bool,
    coda_takes_next: bool,
}

#[requires(index <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(true)]
fn syllable_flags_at(chars: &[char], index: usize, lookahead_end: usize) -> SyllableFlags {
    let end = lookahead_end;
    if index >= end {
        return SyllableFlags {
            starts_any: false,
            coda_takes_next: false,
        };
    }
    let mut pending = Vec::new();
    let mut cursor = index;
    let terminal = loop {
        if cursor >= end {
            break SyllableFlags {
                starts_any: false,
                coda_takes_next: false,
            };
        }
        recognition_cache::note_syllable_step();
        let onset = brivla_onset_has_nucleus_end_at_or_before(chars, cursor, end);
        let consonant = is_consonant(chars[cursor]);
        let next_inside = cursor + 1 < end;
        if !consonant || onset || !next_inside {
            break SyllableFlags {
                starts_any: onset,
                coda_takes_next: false,
            };
        }
        pending.push(cursor);
        cursor += 1;
    };
    let mut next_flags = terminal;
    for source in pending.into_iter().rev() {
        let next_is_syllabic = is_syllabic(chars[source + 1]);
        let consonantal = next_is_syllabic && next_flags.coda_takes_next;
        let starts_any = consonantal;
        next_flags = SyllableFlags {
            starts_any: starts_any,
            coda_takes_next: !starts_any && next_flags.starts_any,
        };
    }
    next_flags
}

#[requires(index <= span_end && span_end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(ret.iter().all(|syllable_end| *syllable_end > index && *syllable_end <= span_end))]
fn consonantal_syllable_ends(
    chars: &[char],
    index: usize,
    span_end: usize,
    lookahead_end: usize,
) -> Vec<usize> {
    if index >= span_end
        || !is_consonant(chars[index])
        || !chars
            .get(index + 1)
            .is_some_and(|value| is_syllabic(*value))
    {
        return Vec::new();
    }
    // consonantal_syllable <- consonant &syllabic coda
    // `coda_ends(index + 1)` is `[index + 1]` plus maybe `[index + 2]`.
    // The filter keeps only the second of those. The coda consonant stays in
    // the span. `&any_syllable` reads to `lookahead_end`, so a later `y`
    // nucleus still counts.
    if index + 2 <= span_end && syllable_flags_at(chars, index + 1, lookahead_end).coda_takes_next {
        vec![index + 2]
    } else {
        Vec::new()
    }
}

#[requires(index <= span_end && span_end <= lookahead_end && lookahead_end <= chars.len())]
#[ensures(ret.iter().all(|coda_end| *coda_end >= index && *coda_end <= span_end))]
fn coda_ends(chars: &[char], index: usize, span_end: usize, lookahead_end: usize) -> Vec<usize> {
    let mut ends = Vec::with_capacity(2);
    ends.push(index);
    // coda <- !any_syllable consonant &any_syllable / syllabic? consonant? &pause
    // The consonant stays in the span. Both syllable tests read to the lookahead.
    if index < span_end && syllable_flags_at(chars, index, lookahead_end).coda_takes_next {
        ends.push(index + 1);
    }
    ends
}

#[cfg(test)]
#[requires(index <= end && end <= chars.len())]
#[ensures(true)]
fn starts_any_syllable(chars: &[char], index: usize, end: usize) -> bool {
    if index >= end {
        return false;
    }
    syllable_flags_at(chars, index, end).starts_any
}

#[requires(index <= chars.len())]
#[requires(ends.iter().all(|end| *end <= chars.len()))]
#[ensures(ends.iter().all(|end| *end <= chars.len()))]
fn append_brivla_onset_ends_unsorted(chars: &[char], index: usize, ends: &mut Vec<usize>) {
    append_parse_onset_ends(chars, index, ends);
    if let Some(end) = experimental_cgv_brivla_onset_end(chars, index) {
        ends.push(end);
    }
    if chars.get(index) == Some(&'\'') {
        ends.push(index + 1);
    }
}

#[requires(index <= chars.len())]
#[requires(nucleus_end <= chars.len())]
#[ensures(true)]
fn brivla_onset_has_nucleus_end(
    chars: &[char],
    index: usize,
    nucleus_end: usize,
    skip_y_onset: bool,
) -> bool {
    if let Some(onset_end) = parse_glide_end(chars, index)
        && brivla_onset_candidate_has_nucleus_end(chars, onset_end, nucleus_end, skip_y_onset)
    {
        return true;
    }
    for onset_end in (index..=max_initial_end(chars, index)).rev() {
        if parse_initial_end(chars, index, onset_end).is_some()
            && brivla_onset_candidate_has_nucleus_end(chars, onset_end, nucleus_end, skip_y_onset)
        {
            return true;
        }
    }
    if let Some(onset_end) = experimental_cgv_brivla_onset_end(chars, index)
        && brivla_onset_candidate_has_nucleus_end(chars, onset_end, nucleus_end, skip_y_onset)
    {
        return true;
    }
    chars.get(index) == Some(&'\'')
        && brivla_onset_candidate_has_nucleus_end(chars, index + 1, nucleus_end, skip_y_onset)
}

#[requires(onset_end <= chars.len())]
#[requires(nucleus_end <= chars.len())]
#[ensures(true)]
fn brivla_onset_candidate_has_nucleus_end(
    chars: &[char],
    onset_end: usize,
    nucleus_end: usize,
    skip_y_onset: bool,
) -> bool {
    (!skip_y_onset || !chars.get(onset_end).is_some_and(|value| is_y(*value)))
        && has_parse_nucleus_end(chars, onset_end, nucleus_end)
}

#[requires(index <= chars.len())]
#[requires(limit <= chars.len())]
#[ensures(true)]
fn brivla_onset_has_nucleus_end_at_or_before(chars: &[char], index: usize, limit: usize) -> bool {
    if let Some(onset_end) = parse_glide_end(chars, index)
        && has_parse_nucleus_end_at_or_before(chars, onset_end, limit)
    {
        return true;
    }
    for onset_end in (index..=max_initial_end(chars, index)).rev() {
        if parse_initial_end(chars, index, onset_end).is_some()
            && has_parse_nucleus_end_at_or_before(chars, onset_end, limit)
        {
            return true;
        }
    }
    if let Some(onset_end) = experimental_cgv_brivla_onset_end(chars, index)
        && has_parse_nucleus_end_at_or_before(chars, onset_end, limit)
    {
        return true;
    }
    chars.get(index) == Some(&'\'') && has_parse_nucleus_end_at_or_before(chars, index + 1, limit)
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
        if parse_diphthong_end(chars, index).is_some() {
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
    // The PEG spells each vowel as `comma* [aA]` and so on, so a comma between
    // the two halves of a falling diphthong does not break it - it only marks
    // where the writer wants the syllable read. Glide detection already skips
    // commas; the two have to agree, or `zo,is.` would be rejected as hiatus
    // while `zoi,s.` is accepted.
    let second_index = next_non_comma_index(chars, start + 1)?;
    let semivowel = match (first, chars[second_index]) {
        ('a', 'i' | 'í' | 'ĭ') | ('e', 'i' | 'í' | 'ĭ') | ('o', 'i' | 'í' | 'ĭ') => 'ĭ',
        ('a', 'u' | 'ú' | 'ŭ') => 'ŭ',
        _ => return None,
    };
    Some((semivowel, second_index + 1))
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
pub(crate) fn next_non_comma_index(chars: &[char], mut index: usize) -> Option<usize> {
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
    matches_cmavo_form_chars(&chars[start..end])
}

#[requires(index <= chars.len())]
#[ensures(true)]
fn starts_with_onset(chars: &[char], index: usize) -> bool {
    index <= chars.len()
        && (has_parse_onset(chars, index)
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
pub(crate) fn digit_to_cmavo(value: char) -> Option<&'static str> {
    Some(match value {
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
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fuhivla_rafsi_lookahead_matches_camxes() {
        // Camxes morphology, not the full text grammar. A foreign word is a
        // rejection. Uppercase vowels are camxes stress. Acutes are the same
        // stress in jbotci.
        let accepted = [
            "kanbrtybra",
            "kánbrtybra",
            "KAnbrtybra",
            "spagetybra",
            "spagétybra",
            "spaGEtybra",
            "cmacrntybroda",
            "cmácrntybroda",
            "cmAcrntybroda",
            "klama'ybra",
            "klamá'ybra",
            "klamA'ybra",
            "klama'ybroda",
            "kanbrti'ybroda",
            "kaunbrti",
            "cmarnteti",
            "kanbrbrti",
            "kanybriti",
            "cmajria",
            "cmagnia",
            "cmajriti",
            "cmagniti",
            "gerkrxuiybra",
            "gerkrxuipybra",
            "gerkrxuipykla",
            "gerkrxuipybroda",
            "zbasu'ybroda",
            "kanbrma'ybroda",
            "blanu'ybroda",
        ];
        let expected = [
            "Lujvo:rafsi:kánbrt+hyphen:y+rafsi:bra",
            "Lujvo:rafsi:kánbrt+hyphen:y+rafsi:bra",
            "Lujvo:rafsi:kánbrt+hyphen:y+rafsi:bra",
            "Lujvo:rafsi:spa+rafsi:gét+hyphen:y+rafsi:bra",
            "Lujvo:rafsi:spa+rafsi:gét+hyphen:y+rafsi:bra",
            "Lujvo:rafsi:spa+rafsi:gét+hyphen:y+rafsi:bra",
            "Lujvo:rafsi:cmacrnt+hyphen:y+rafsi:bróda",
            "Lujvo:rafsi:cmácrnt+hyphen:y+rafsi:bro | Cmavo:da",
            "Lujvo:rafsi:cmácrnt+hyphen:y+rafsi:bro | Cmavo:da",
            "Lujvo:rafsi:klamá+hyphen:'y+rafsi:bra",
            "Lujvo:rafsi:klamá+hyphen:'y+rafsi:bra",
            "Lujvo:rafsi:klamá+hyphen:'y+rafsi:bra",
            "Lujvo:rafsi:klama+hyphen:'y+rafsi:bróda",
            "Lujvo:rafsi:kanbrti+hyphen:'y+rafsi:bróda",
            "Fuhivla:káŭnbrti",
            "Fuhivla:cmarntéti",
            "Fuhivla:kánbrbrti",
            "Lujvo:rafsi:kan+hyphen:y+rafsi:bríti",
            "Fuhivla:cmájrĭa",
            "Fuhivla:cmágnĭa",
            "Fuhivla:cmajríti",
            "Fuhivla:cmagníti",
            "Lujvo:rafsi:gerkrxúĭ+hyphen:y+rafsi:bra",
            "Lujvo:rafsi:gerkrxŭíp+hyphen:y+rafsi:bra",
            "Lujvo:rafsi:gerkrxŭíp+hyphen:y+rafsi:kla",
            "Lujvo:rafsi:gerkrxŭip+hyphen:y+rafsi:bróda",
            "Lujvo:rafsi:zbasu+hyphen:'y+rafsi:bróda",
            "Lujvo:rafsi:kanbrma+hyphen:'y+rafsi:bróda",
            "Lujvo:rafsi:blanu+hyphen:'y+rafsi:bróda",
        ];
        for (source, expected) in accepted.iter().zip(expected) {
            assert_eq!(morphology_classes(source), expected, "{source}");
            let marked = stress_marked_output(source);
            assert_eq!(
                morphology_classes(&marked),
                expected,
                "{source} read back as {marked}"
            );
        }
        // `kan` + `br` + `y` is `stressed_y_less_rafsi? initial_pair y`.
        // Camxes rejects the word. `klaMAybra` has no apostrophe, so `' y`
        // does not match, and camxes rejects it too.
        // A brivla rafsi before `'y` needs two vowel syllables. One vowel
        // syllable, with or without a following consonantal syllable, fails
        // the lookahead. Stress on the first syllable fails
        // `&unstressed_syllable` on `stressed_brivla_rafsi`.
        // `gerkrxuipyte` keeps the stressed fu'ivla rafsi and still fails,
        // because `te` is not a short final rafsi.
        for source in [
            "kanbrybra",
            "kánbrybra",
            "KAnbrybra",
            "klaMAybra",
            "cmania",
            "kandia",
            "gerkrxuipyte",
            "gerkrxuipy",
            "kla'ybroda",
            "klá'ybroda",
            "bra'ybroda",
            "brá'ybroda",
            "spa'ykanbrti",
            "spá'ykanbrti",
            "ka'ybra",
            "ká'ybra",
            "kanbr'ybroda",
            "kánbr'ybroda",
            "kláma'ybroda",
            "zbásu'ybroda",
            "kánbrma'ybroda",
        ] {
            let attempt = crate::segment_words_with_modifiers_with_options_and_source_id_attempt(
                source,
                &MorphologyOptions::default(),
                None,
            );
            assert!(attempt.into_data().result.is_err(), "{source} is rejected");
        }
        // Camxes rejects these (`r <- comma* [rR] !h !glide !r` and
        // `n <- comma* [nN] !h !glide !n !affricate`).
        // jbotci keeps the experimental consonant-glide warning.
        // Camxes rejects `gerkrxuipybra`: `u` is not a nucleus before `i`, and
        // `i` is not a glide before `p`, so no syllable starts at `x`.
        // The experimental consonant-glide onset accepts `xui`. The a692
        // parser already accepted this word with the same warning.
        for (source, start, end, text) in [
            ("cmajria", 4, 7, "ria"),
            ("cmagnia", 4, 7, "nia"),
            ("gerkrxuipybra", 5, 8, "xui"),
            ("gerkrxuipykla", 5, 8, "xui"),
            ("gerkrxuipybroda", 5, 8, "xui"),
        ] {
            let attempt = crate::segment_words_with_modifiers_with_options_and_source_id_attempt(
                source,
                &MorphologyOptions::default(),
                None,
            );
            let data = attempt.into_data();
            assert!(data.result.is_ok(), "{source}");
            assert_eq!(data.warnings.len(), 1, "{source}");
            assert_eq!(
                data.warnings[0].kind,
                crate::MorphologyWarningKind::ExperimentalCgv,
                "{source}"
            );
            assert_eq!(data.warnings[0].char_start, start, "{source}");
            assert_eq!(data.warnings[0].char_end, end, "{source}");
            assert_eq!(data.warnings[0].text, text, "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stressed_extended_rafsi_keeps_short_final_failure() {
        // The stressed extended rafsi is recognized. The tail is not a
        // `short_final_rafsi`. The diagnostic names that prefix and the tail.
        for (source, prefix, suffix) in [
            ("gerkrxuipyte", "gerkrxuipy", "te"),
            ("kanbrtyte", "kanbrty", "te"),
        ] {
            let error = crate::segment_words_with_modifiers_with_options_and_source_id(
                source,
                &MorphologyOptions::default(),
                None,
            )
            .expect_err(source);
            let crate::MorphologyError::Invalid {
                kind,
                char_start,
                char_end,
                text,
                detail,
                ..
            } = &error
            else {
                panic!("{source} is an invalid-word error, got {error:?}");
            };
            assert_eq!(*kind, MorphologyErrorKind::InvalidLujvo, "{source}");
            assert_eq!(
                (*char_start, *char_end),
                (0, source.chars().count()),
                "{source}"
            );
            assert_eq!(text, source, "{source}");
            assert_eq!(&text[prefix.len()..], suffix, "{source}");
            let expected = new!(MorphologyErrorDetail::InvalidLujvo {
                parsed_prefix: Some(prefix.to_owned()),
                expected: LujvoParseExpectation::ShortFinalRafsi,
            });
            assert_eq!(detail.as_ref(), Some(&expected), "{source}");
            let diagnostic = error.to_diagnostic(None, source).expect(source);
            assert_eq!(diagnostic.code, "morphology.invalid-lujvo", "{source}");
            let note: String = diagnostic
                .styled_notes
                .iter()
                .flat_map(|note| note.segments.iter().map(|segment| segment.text.as_str()))
                .collect();
            assert!(
                note.contains(prefix) && note.contains("a short final rafsi after the `y`"),
                "{source}: {note}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stressed_rafsi_ending_at_y_is_incomplete_lujvo() {
        // A stressed initial rafsi ends on y. The missing part is a short
        // final rafsi. `spagéty` is `spa` plus the stressed y-rafsi `géty`.
        // `bródy` is a stressed long rafsi plus y.
        for (source, prefix) in [
            ("spagéty", "spagéty"),
            ("bródy", "bródy"),
            ("kánbrty", "kánbrty"),
            ("klamá'y", "klamá'y"),
            ("Only", "ónly"),
        ] {
            assert_incomplete_lujvo(source, prefix, LujvoParseExpectation::ShortFinalRafsi);
        }
        // An unstressed y-ending initial rafsi ends the span. The parser can
        // still read another initial rafsi or a final part. No syllable
        // follows the final y, so implicit stress does not apply. `kanbrty`
        // uses this path. `kánbrty` is the stressed form.
        for (source, prefix) in [
            ("kanbrty", "kanbrty"),
            ("spagety", "spagety"),
            ("only", "only"),
            ("probably", "probably"),
            ("brody", "brody"),
            ("klama'y", "klama'y"),
        ] {
            assert_incomplete_lujvo(source, prefix, LujvoParseExpectation::FinalOrInitialRafsi);
        }
        // `kla'y` has one syllable before apostrophe-y. That is a hy-rafsi in
        // rafsi_string, not a brivla rafsi, so the lujvo parser does not reach
        // the missing short final. `ka'y` is a cmavo.
        for source in ["kla'y", "klá'y"] {
            let error = crate::segment_words_with_modifiers_with_options_and_source_id(
                source,
                &MorphologyOptions::default(),
                None,
            )
            .expect_err(source);
            let crate::MorphologyError::Invalid { kind, .. } = &error else {
                panic!("{source} is an invalid-word error, got {error:?}");
            };
            assert_eq!(*kind, MorphologyErrorKind::UnrecognizedWord, "{source}");
        }
        assert!(
            crate::segment_words_with_modifiers_with_options_and_source_id(
                "ka'y",
                &MorphologyOptions::default(),
                None,
            )
            .is_ok(),
            "ka'y is a cmavo"
        );
        // A pause ends the word on the y. A comma inside one candidate is
        // removed before the lujvo parse, so a tail after the comma remains.
        assert_incomplete_lujvo(
            "kánbrty.",
            "kánbrty",
            LujvoParseExpectation::ShortFinalRafsi,
        );
        assert_incomplete_lujvo(
            "kánbrty,",
            "kánbrty",
            LujvoParseExpectation::ShortFinalRafsi,
        );
        assert_incomplete_lujvo(
            "kánbrty broda",
            "kánbrty",
            LujvoParseExpectation::ShortFinalRafsi,
        );
        assert_short_final_failure("kanbrty,te", "kanbrty", "te");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn corpus_sentences_keep_incomplete_lujvo_spans() {
        let incomplete = [
            (
                "to purci le nu go'i kei fa le nu do cpacu le tasty food toi",
                45,
                50,
                "tasty",
            ),
            ("Only one broda.", 0, 4, "Only"),
            ("only one selbri", 0, 4, "only"),
            ("i like simplicity", 7, 17, "simplicity"),
            (
                "leka tsali, leka {dexterity}, leka renvi, leka selnelci, leka {manipulation}",
                18,
                27,
                "dexterity",
            ),
            (
                "leka tsali, leka {dexterity}, leka renvi, leka selnelci, leka jitro, leka melbi, leka zgana, leka besna, leka pesysutra",
                18,
                27,
                "dexterity",
            ),
            (
                ".iku'i le preti cu kakne le nu vasru loi raply lerfu",
                41,
                46,
                "raply",
            ),
            ("probably more likely", 0, 8, "probably"),
            ("xirma prami, probably", 13, 21, "probably"),
            (
                "probably more like le stuzi be tu'a le ckire",
                0,
                8,
                "probably",
            ),
            ("uu mi na tavla la ny joi la omny", 28, 32, "omny"),
            ("probably noroi klama", 0, 8, "probably"),
            ("probably me le re lumci kumfa", 0, 8, "probably"),
            ("probably me re le lumci kumfa", 0, 8, "probably"),
            ("probably any size", 0, 8, "probably"),
            ("only one bite", 0, 4, "only"),
        ];
        for (source, start, end, word) in incomplete {
            let error = crate::segment_words_with_modifiers_with_options_and_source_id(
                source,
                &MorphologyOptions::default(),
                None,
            )
            .expect_err(source);
            let crate::MorphologyError::Invalid {
                kind,
                char_start,
                char_end,
                text,
                ..
            } = &error
            else {
                panic!("{source} is an invalid-word error, got {error:?}");
            };
            assert_eq!(*kind, MorphologyErrorKind::IncompleteLujvo, "{source}");
            assert_eq!(
                (*char_start, *char_end, text.as_str()),
                (start, end, word),
                "{source}"
            );
            // A capital letter stresses the first vowel. `Only` takes the
            // stressed path. The other words have no stress mark. They stay
            // on the unstressed path.
            let continuation = if word == "Only" {
                LujvoParseExpectation::ShortFinalRafsi
            } else {
                LujvoParseExpectation::FinalOrInitialRafsi
            };
            let diagnostic = error.to_diagnostic(None, source).expect(source);
            assert_eq!(
                diagnostic.message,
                continuation.incomplete_headline(),
                "{source}"
            );
            assert_eq!(diagnostic.code, "morphology.incomplete-lujvo", "{source}");
        }
        let unchanged = [
            (
                "some japanese guy?",
                14,
                17,
                "guy",
                MorphologyErrorKind::UnrecognizedWord,
            ),
            (
                "doi p1nky .i xu do xamgu tavla fo la lojban?",
                4,
                9,
                "p1nky",
                MorphologyErrorKind::UnrecognizedWord,
            ),
        ];
        for (source, start, end, word, expected_kind) in unchanged {
            let error = crate::segment_words_with_modifiers_with_options_and_source_id(
                source,
                &MorphologyOptions::default(),
                None,
            )
            .expect_err(source);
            let crate::MorphologyError::Invalid {
                kind,
                char_start,
                char_end,
                text,
                ..
            } = &error
            else {
                panic!("{source} is an invalid-word error, got {error:?}");
            };
            assert_eq!(*kind, expected_kind, "{source}");
            assert_eq!(
                (*char_start, *char_end, text.as_str()),
                (start, end, word),
                "{source}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn y_ending_rafsi_with_a_short_final_stays_lujvo() {
        for source in [
            "onlybra",
            "Onlybra",
            "klama'ybroda",
            "kanbrtybra",
            "brodybra",
            "probablybroda",
            "brodyklama",
            "klama'yklama",
        ] {
            let words = crate::segment_words_with_modifiers_with_options_and_source_id(
                source,
                &MorphologyOptions::default(),
                None,
            )
            .unwrap_or_else(|error| panic!("{source} stays a word: {error:?}"));
            assert_eq!(words.len(), 1, "{source}");
            let data!(crate::WordLike::PlainWord(word)) = words[0].as_data() else {
                panic!("{source} is one plain word, got {words:?}");
            };
            assert!(word.lujvo_parts().is_some(), "{source} is a lujvo");
        }
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn assert_incomplete_lujvo(
        source: &str,
        prefix: &str,
        expected_continuation: LujvoParseExpectation,
    ) {
        let error = crate::segment_words_with_modifiers_with_options_and_source_id(
            source,
            &MorphologyOptions::default(),
            None,
        )
        .expect_err(source);
        let crate::MorphologyError::Invalid { kind, detail, .. } = &error else {
            panic!("{source} is an invalid-word error, got {error:?}");
        };
        assert_eq!(*kind, MorphologyErrorKind::IncompleteLujvo, "{source}");
        let expected = new!(MorphologyErrorDetail::IncompleteLujvo {
            parsed_prefix: prefix.to_owned(),
            expected: expected_continuation,
        });
        assert_eq!(detail.as_ref(), Some(&expected), "{source}");
        let diagnostic = error.to_diagnostic(None, source).expect(source);
        assert_eq!(diagnostic.code, "morphology.incomplete-lujvo", "{source}");
        assert_eq!(
            diagnostic.message,
            expected_continuation.incomplete_headline(),
            "{source}"
        );
        let note: String = diagnostic
            .styled_notes
            .iter()
            .flat_map(|note| note.segments.iter().map(|segment| segment.text.as_str()))
            .collect();
        let detail_text = match expected_continuation {
            LujvoParseExpectation::ShortFinalRafsi => "needs a short final rafsi after the `y`",
            LujvoParseExpectation::FinalOrInitialRafsi => {
                "needs a final rafsi or another initial rafsi after the `y`"
            }
            LujvoParseExpectation::InitialOrStandaloneFinalRafsi => {
                panic!("{source} is not an initial-only failure")
            }
        };
        assert!(
            note.contains(prefix) && note.contains(detail_text),
            "{source}: {note}"
        );
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn assert_short_final_failure(source: &str, prefix: &str, suffix: &str) {
        let error = crate::segment_words_with_modifiers_with_options_and_source_id(
            source,
            &MorphologyOptions::default(),
            None,
        )
        .expect_err(source);
        let crate::MorphologyError::Invalid {
            kind, text, detail, ..
        } = &error
        else {
            panic!("{source} is an invalid-word error, got {error:?}");
        };
        assert_eq!(*kind, MorphologyErrorKind::InvalidLujvo, "{source}");
        assert!(text.contains(suffix), "{source} text {text}");
        let expected = new!(MorphologyErrorDetail::InvalidLujvo {
            parsed_prefix: Some(prefix.to_owned()),
            expected: LujvoParseExpectation::ShortFinalRafsi,
        });
        assert_eq!(detail.as_ref(), Some(&expected), "{source}");
    }

    #[requires(!source.is_empty())]
    #[ensures(!ret.is_empty())]
    fn morphology_classes(source: &str) -> String {
        let attempt = crate::segment_words_with_modifiers_with_options_and_source_id_attempt(
            source,
            &MorphologyOptions::default(),
            None,
        );
        let data = attempt.into_data();
        let Ok(words) = data.result else {
            return "ERR".to_owned();
        };
        words
            .iter()
            .map(|word| match word.as_data() {
                data!(crate::WordLike::PlainWord(word)) => {
                    let body = if let Some(parts) = word.lujvo_parts() {
                        parts
                            .iter()
                            .map(|part| match part {
                                LujvoPart::Rafsi(phonemes) => {
                                    format!("rafsi:{}", phonemes.as_str())
                                }
                                LujvoPart::Hyphen(phonemes) => {
                                    format!("hyphen:{}", phonemes.as_str())
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("+")
                    } else {
                        word.phonemes().as_str().to_owned()
                    };
                    format!("{:?}:{body}", word.kind())
                }
                _ => "other".to_owned(),
            })
            .collect::<Vec<_>>()
            .join(" | ")
    }

    #[requires(!source.is_empty())]
    #[ensures(!ret.is_empty())]
    fn stress_marked_output(source: &str) -> String {
        let attempt = crate::segment_words_with_modifiers_with_options_and_source_id_attempt(
            source,
            &MorphologyOptions::default(),
            None,
        );
        let words = attempt
            .into_data()
            .result
            .unwrap_or_else(|_| panic!("{source}"));
        words
            .iter()
            .map(|word| match word.as_data() {
                data!(crate::WordLike::PlainWord(word)) => word.phonemes().as_str().to_owned(),
                _ => panic!("{source}"),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn segment_and_lujvo_pair_matrices_are_equivalent() {
        for first in phonotactics::CONSONANT_ORDER_FOR_TEST.chars() {
            for second in phonotactics::CONSONANT_ORDER_FOR_TEST.chars() {
                assert_eq!(
                    phonotactics::consonant_pair_class_for_test(first, second),
                    crate::lujvo::consonant_pair_class(first, second),
                    "{first}{second}"
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_recognition_cache_starts_with_no_slots() {
        for char_count in [0, 1, 5, 32, 4_000] {
            let cache = MorphologyRecognitionCache::new(char_count);

            assert_eq!(cache.char_count, char_count);
            assert_eq!(
                cache.allocated_slots(),
                0,
                "char_count={char_count} allocated a table before any key was stored"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cached_lujvo_analysis_matches_wrapper_results() {
        let cases = [
            "coi",
            "klama",
            "jetcybolxada",
            "spageti",
            "alis",
            "xlaglymlu",
            "jbaugri",
            "coi klama",
            "zgikemfi'inalka'esefsysajyke'ejvekemsefsyda'atoiflike'ejvejagborkemjilryjvesefsyborxenze'a",
        ];

        for source in cases {
            let chars = text_chars(source);
            let expected = analyze_lujvo_part_ranges_chars(&chars);
            let mut cache = MorphologyRecognitionCache::new(chars.len());
            let actual = analyze_lujvo_part_ranges_chars_with_cache(&chars, &mut cache);

            assert_eq!(actual, expected, "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cached_slice_helpers_match_wrapper_results() {
        let cases = [
            "coi",
            "klama",
            "jetcybolxada",
            "spageti",
            "alis",
            "xlaglymlu",
            "jbaugri",
            "coi klama",
            "jbojevysofkemsuzgugje'ake'eborkemfaipaltrusi'oke'ekemgubyseltru",
        ];

        for source in cases {
            let chars = text_chars(source);
            for (start, end) in selected_ranges(chars.len()) {
                assert_cached_slice_helpers_match_wrappers(source, &chars, start, end);
            }
        }
    }

    #[requires(start <= end && end <= chars.len())]
    #[ensures(true)]
    fn assert_cached_slice_helpers_match_wrappers(
        source: &str,
        chars: &[char],
        start: usize,
        end: usize,
    ) {
        let mut cache = MorphologyRecognitionCache::new(chars.len());
        assert_eq!(
            cmavo_word_slice_with_cache(chars, start, end, &mut cache),
            cmavo_word_slice(chars, start, end),
            "{source} {start}..{end} cmavo"
        );
        assert_eq!(
            brivla_word_slice_with_cache(chars, start, end, &mut cache),
            brivla_word_slice(chars, start, end),
            "{source} {start}..{end} brivla"
        );
        assert_eq!(
            is_lujvo_slice_with_cache(chars, start, end, &mut cache),
            is_lujvo_slice(chars, start, end),
            "{source} {start}..{end} lujvo"
        );
        assert_eq!(
            is_fuhivla_shape_slice_with_cache(chars, start, end, &mut cache),
            is_fuhivla_shape_slice(chars, start, end),
            "{source} {start}..{end} fuhivla"
        );
        assert_eq!(
            lojban_word_slice_with_cache(chars, start, end, &mut cache),
            lojban_word_slice(chars, start, end),
            "{source} {start}..{end} word"
        );
        assert_eq!(
            slinkuhi_slice_with_cache(chars, start, end, &mut cache),
            slinkuhi_slice(chars, start, end),
            "{source} {start}..{end} slinkuhi"
        );
        assert_eq!(
            rafsi_string_starts_slice_with_cache(chars, start, end, &mut cache),
            rafsi_string_starts_slice(chars, start, end),
            "{source} {start}..{end} rafsi string"
        );
        assert_eq!(
            post_word_slice_with_cache(chars, start, end, &mut cache),
            post_word_slice(chars, start, end),
            "{source} {start}..{end} post-word"
        );
        assert_eq!(
            starts_with_cvcy_lujvo_chars_with_cache(chars, start, end, &mut cache),
            starts_with_cvcy_lujvo_chars(&chars[..end], start),
            "{source} {start}..{end} CVC-y lujvo"
        );
    }

    #[requires(true)]
    #[ensures(ret.iter().all(|(start, end)| start <= end && *end <= char_count))]
    fn selected_ranges(char_count: usize) -> Vec<(usize, usize)> {
        let mut ranges = vec![(0, char_count), (char_count, char_count)];
        if char_count > 0 {
            ranges.push((0, 1));
            ranges.push((1, char_count));
        }
        if char_count > 3 {
            ranges.push((0, 3));
            ranges.push((2, char_count));
        }
        if char_count > 8 {
            ranges.push((3, 8));
        }
        ranges.sort_unstable();
        ranges.dedup();
        ranges
    }

    /// Body executions for `cíniconulová` repeated `groups` times.
    ///
    /// The repeated piece is 12 characters. `tcíniconuloválsicu` is 18
    /// characters, not 12. The cap is the key space: 26 two-index slots, 8
    /// three-index maps, and 1 four-index map. A body runs once per stored
    /// key, so the count cannot pass that space. Growth from one size to the
    /// next stays inside four times the four-index ratio.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stressed_cmavo_groups_execute_polynomially_many_recognizer_bodies() {
        let mut previous_bodies = 0u64;
        let mut previous_n = 0u64;
        for groups in 1..=3 {
            let input = "cíniconulová".repeat(groups);
            let char_count = input.chars().count();
            let cap = recognition_key_space(char_count);
            let counter = super::recognition_cache::RecognitionBodyCount::begin(cap);
            let words = crate::segment_words_for_display_with_options_and_source_id(
                &input,
                &crate::MorphologyOptions::default(),
                None,
            )
            .unwrap_or_else(|error| panic!("{input}: {error}"));
            let bodies = counter.get();
            assert!(!words.is_empty(), "{input}");
            assert!(
                bodies <= cap,
                "groups={groups} chars={char_count} bodies={bodies} cap={cap}"
            );
            assert!(
                bodies > previous_bodies,
                "groups={groups} bodies={bodies} previous={previous_bodies}"
            );
            if previous_bodies > 0 {
                let n = u64::try_from(char_count).unwrap_or(u64::MAX);
                assert!(
                    bodies.saturating_mul(previous_n.saturating_pow(4))
                        <= previous_bodies
                            .saturating_mul(n.saturating_pow(4))
                            .saturating_mul(4),
                    "groups={groups} bodies={bodies} previous={previous_bodies} n={n} previous_n={previous_n}"
                );
            }
            previous_bodies = bodies;
            previous_n = u64::try_from(char_count).unwrap_or(u64::MAX);
        }
    }

    /// Issue #1083. The acute marks are explicit stress. A brivla has one
    /// stress, on the penultimate syllable (CLL 4.3), and cmavo need none
    /// (CLL 4.2). Each stressed piece here is a five-letter gismu shape with
    /// that stress, and the pieces between them are cmavo. The same breaks
    /// are what `vlasei` already returned for the shorter words before this
    /// change. The apostrophe in the long word is U+2019.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn explicit_stress_segments_the_reported_words() {
        assert_eq!(
            display_words("nolotcíniconuloválsicu"),
            [
                "cmavo:no",
                "cmavo:lo",
                "gismu:tcíni",
                "cmavo:co",
                "cmavo:nu",
                "cmavo:lo",
                "gismu:válsi",
                "cmavo:cu",
            ]
        );
        assert_eq!(
            display_words("danoroidjúnolotcíniconuloválsicunu\u{2019}osárcu"),
            [
                "cmavo:da",
                "cmavo:no",
                "cmavo:ro\u{12d}",
                "gismu:djúno",
                "cmavo:lo",
                "gismu:tcíni",
                "cmavo:co",
                "cmavo:nu",
                "cmavo:lo",
                "gismu:válsi",
                "cmavo:cu",
                "cmavo:nu'o",
                "gismu:sárcu",
            ]
        );
    }

    #[requires(!input.is_empty())]
    #[ensures(!ret.is_empty())]
    fn display_words(input: &str) -> Vec<String> {
        crate::segment_words_for_display_with_options_and_source_id(
            input,
            &crate::MorphologyOptions::default(),
            None,
        )
        .unwrap_or_else(|error| panic!("{input}: {error}"))
        .iter()
        .map(|word| word.to_string())
        .collect()
    }

    /// 26 two-index slots, 8 three-index maps, and 1 four-index map.
    ///
    /// Eighteen of the two-index slots are plain maps. Four maps carry a flag
    /// with two values (`has_initial_rafsi` on `lujvo_from` and on
    /// `lujvo_part_ranges`, `allows_y`, and `FuhivlaHeadPolicy`). Those eight
    /// slots make 26 with the plain maps. `brivla_head_ends` and
    /// `fuhivla_head_ends` are three-index keys. The extra index is the
    /// lookahead end. `m = char_count + 1`.
    #[requires(char_count < usize::MAX)]
    #[ensures(ret >= 26)]
    fn recognition_key_space(char_count: usize) -> u64 {
        let m = u64::try_from(char_count)
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        let m2 = m.saturating_mul(m);
        let m3 = m2.saturating_mul(m);
        let m4 = m3.saturating_mul(m);
        m2.saturating_mul(26)
            .saturating_add(m3.saturating_mul(8))
            .saturating_add(m4)
    }

    /// Hash capacity is at most about twice the stored keys, plus spare
    /// buckets across the 31 maps.
    #[requires(char_count < usize::MAX)]
    #[ensures(ret >= recognition_key_space(char_count))]
    fn recognition_slot_cap(char_count: usize) -> u64 {
        recognition_key_space(char_count)
            .saturating_mul(2)
            .saturating_add(1024)
    }

    /// Slots a nested dense cache allocated for `lo` repeated `char_count / 2`
    /// times. Each suffix kept 18 squares. The shared region has to stay
    /// under that sum.
    #[requires(char_count > 0)]
    #[ensures(ret >= 18)]
    fn nested_dense_slots(char_count: usize) -> u64 {
        let mut total = 0u64;
        let mut len = char_count;
        while len > 0 {
            let stride = u64::try_from(len).unwrap_or(u64::MAX).saturating_add(1);
            total = total.saturating_add(stride.saturating_mul(stride).saturating_mul(18));
            if len < 2 {
                break;
            }
            len -= 2;
        }
        total
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn coda_scan_matches_the_old_recurrence_and_stays_linear() {
        for length in 1..=14 {
            let chars = vec!['l'; length];
            let (old_ends, old_calls, old_starts) = legacy_coda_ends(&chars, 0, chars.len());
            let start_steps = super::recognition_cache::SyllableStepCount::begin(length as u64);
            assert_eq!(
                starts_any_syllable(&chars, 0, chars.len()),
                old_starts,
                "length {length}"
            );
            assert!(
                start_steps.get() <= length as u64,
                "length {length} start steps={}",
                start_steps.get()
            );
            drop(start_steps);
            let steps = super::recognition_cache::SyllableStepCount::begin(length as u64);
            let new_ends = coda_ends(&chars, 0, chars.len(), chars.len());
            assert_eq!(new_ends, old_ends, "length {length}");
            assert!(
                steps.get() <= length as u64,
                "length {length} steps={} cap={length}",
                steps.get()
            );
            assert!(
                old_calls > length as u64 || length < 4,
                "length {length} old calls {old_calls}"
            );
        }
        for (length, published) in [(10u64, 143u64), (13, 609), (16, 2_583), (19, 10_945)] {
            let chars = vec!['l'; usize::try_from(length).unwrap_or(0)];
            let (_ends, calls, _starts) = legacy_coda_ends(&chars, 0, chars.len());
            assert_eq!(calls, published, "length {length}");
            assert!(published > length, "length {length} published {published}");
        }
        let chars = vec!['l'; 64];
        let steps = super::recognition_cache::SyllableStepCount::begin(chars.len() as u64);
        let ends = coda_ends(&chars, 0, chars.len(), chars.len());
        assert_eq!(ends.first().copied(), Some(0));
        assert!(ends.len() <= 2, "{ends:?}");
        assert!(steps.get() <= chars.len() as u64, "steps={}", steps.get());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syllabic_consonant_run_counts_inner_scan_steps() {
        for reps in [4usize, 8, 12] {
            let input = format!("ba{}a", "rl".repeat(reps));
            let char_count = input.chars().count();
            let body_cap = recognition_key_space(char_count);
            let step_cap = recognition_key_space(char_count);
            let bodies = super::recognition_cache::RecognitionBodyCount::begin(body_cap);
            let steps = super::recognition_cache::SyllableStepCount::begin(step_cap);
            let words = crate::segment_words_for_display_with_options_and_source_id(
                &input,
                &crate::MorphologyOptions::default(),
                None,
            );
            assert!(
                bodies.get() <= body_cap,
                "reps={reps} bodies={} cap={body_cap}",
                bodies.get()
            );
            assert!(
                steps.get() <= step_cap,
                "reps={reps} chars={char_count} steps={} cap={step_cap}",
                steps.get()
            );
            // Camxes accepts this run as one fu'ivla.
            // The split is `bar`, then `lr` once for each remaining pair, then `la`.
            // The counts above bound that scan.
            let words = words.unwrap_or_else(|error| panic!("{input}: {error:?}"));
            assert_eq!(words.len(), 1, "{input}");
            let word = words[0]
                .bare_word()
                .unwrap_or_else(|| panic!("{input}: {words:?}"));
            assert_eq!(word.kind(), crate::WordKind::Fuhivla, "{input}");
            assert_eq!(
                word.phonemes().as_str(),
                format!("bá{}a", "rl".repeat(reps)),
                "{input}"
            );
            let mut expected_syllables = Vec::with_capacity(reps + 1);
            expected_syllables.push("bar");
            expected_syllables.extend(std::iter::repeat_n("lr", reps - 1));
            expected_syllables.push("la");
            let got_syllables =
                pronunciation_syllable_texts(&input).unwrap_or_else(|| panic!("{input} syllables"));
            assert_eq!(
                got_syllables.iter().map(String::as_str).collect::<Vec<_>>(),
                expected_syllables,
                "{input}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn repeated_lo_reuses_one_sparse_region() {
        for copies in [20usize, 40, 80] {
            let input = "lo".repeat(copies);
            let char_count = input.chars().count();
            let body_cap = recognition_key_space(char_count);
            let slot_cap = recognition_slot_cap(char_count);
            let dense = nested_dense_slots(char_count);
            let peak = super::recognition_cache::RecognitionSlotPeak::begin();
            let bodies = super::recognition_cache::RecognitionBodyCount::begin(body_cap);
            let words = display_words(&input);
            assert_eq!(words.len(), copies, "{input}");
            assert!(words.iter().all(|word| word == "cmavo:lo"), "{words:?}");
            assert!(
                bodies.get() <= body_cap,
                "copies={copies} bodies={} cap={body_cap}",
                bodies.get()
            );
            assert!(
                peak.get() <= slot_cap,
                "copies={copies} slots={} cap={slot_cap}",
                peak.get()
            );
            assert!(
                peak.get() < dense,
                "copies={copies} slots={} nested dense slots={dense}",
                peak.get()
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejected_vowel_run_allocates_no_dense_square() {
        for copies in [64usize, 256] {
            let input = format!("b{}", "a".repeat(copies));
            let char_count = input.chars().count();
            let square = u64::try_from(char_count)
                .unwrap_or(u64::MAX)
                .saturating_add(1)
                .saturating_pow(2);
            let peak = super::recognition_cache::RecognitionSlotPeak::begin();
            let _result = crate::segment_words_for_display_with_options_and_source_id(
                &input,
                &crate::MorphologyOptions::default(),
                None,
            );
            assert!(
                peak.get() < square,
                "chars={char_count} slots={} one square={square}",
                peak.get()
            );
        }
    }

    /// Inner coda scans on repeated `lo` and `lo'a`.
    ///
    /// Each query visits the syllabic-consonant run only. These strings have
    /// no such run, so doubling the length keeps the step count inside five
    /// times the previous count.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn repeated_lo_and_loa_scan_quadratically() {
        for piece in ["lo", "lo'a"] {
            let mut previous_steps = 0u64;
            let mut previous_n = 0u64;
            let copies = if piece == "lo" {
                [40usize, 80, 160]
            } else {
                [20, 40, 80]
            };
            for copies in copies {
                let input = piece.repeat(copies);
                let n = u64::try_from(input.chars().count()).unwrap_or(u64::MAX);
                let cap = n.saturating_mul(n).saturating_mul(64);
                let steps = super::recognition_cache::SyllableStepCount::begin(cap);
                let words = display_words(&input);
                let got = steps.get();
                assert_eq!(words.len(), copies, "{input}");
                assert!(
                    words.iter().all(|word| word == &format!("cmavo:{piece}")),
                    "{words:?}"
                );
                assert!(got > 0, "{piece} copies={copies} scanned no indexes");
                assert!(
                    got <= cap,
                    "{piece} copies={copies} chars={n} steps={got} cap={cap}"
                );
                if previous_steps > 0 {
                    assert!(
                        got.saturating_mul(previous_n.saturating_pow(2))
                            .saturating_mul(4)
                            <= previous_steps
                                .saturating_mul(n.saturating_pow(2))
                                .saturating_mul(5),
                        "{piece} steps={got} previous={previous_steps} n={n} previous_n={previous_n}"
                    );
                }
                previous_steps = got;
                previous_n = n;
            }
        }
    }

    /// 3,200 copies of `lo`, then 800 copies of `lo'a`, on a 256 KiB thread.
    ///
    /// A Rust stack overflow aborts the process. The segmentation runs in a
    /// child process, so an abort fails this test and leaves the test binary
    /// alive. Native morphology grows the stack at each per-word call. On the
    /// head before that growth, this child aborts.
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    #[requires(true)]
    #[ensures(true)]
    fn long_unspaced_cmavo_chain_finishes_on_a_small_stack() {
        const CHILD: &str = "JBOTCI_MORPHOLOGY_SMALL_STACK_CHILD";
        if std::env::var_os(CHILD).is_some() {
            let lo = segment_on_small_stack("lo".repeat(3_200));
            assert_eq!(lo.len(), 3_200);
            assert!(lo.iter().all(|word| word == "cmavo:lo"));
            let loa = segment_on_small_stack("lo'a".repeat(800));
            assert_eq!(loa.len(), 800);
            assert!(loa.iter().all(|word| word == "cmavo:lo'a"));
            return;
        }
        let exe = std::env::current_exe().unwrap_or_else(|error| panic!("{error}"));
        let output = std::process::Command::new(exe)
            .args([
                "--exact",
                "segment::tests::long_unspaced_cmavo_chain_finishes_on_a_small_stack",
                "--test-threads=1",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap_or_else(|error| panic!("{error}"));
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success()
                && stdout.contains("long_unspaced_cmavo_chain_finishes_on_a_small_stack"),
            "small-stack child status: {}\n{stdout}\n{stderr}",
            output.status
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[requires(!input.is_empty())]
    #[ensures(!ret.is_empty())]
    fn segment_on_small_stack(input: String) -> Vec<String> {
        let handle = std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(move || display_words(&input))
            .unwrap_or_else(|error| panic!("{error}"));
        handle
            .join()
            .unwrap_or_else(|_| panic!("small-stack thread panicked"))
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fuhivla_head_miss_shares_the_brivla_allocation() {
        let chars = ['z'];
        let mut cache = MorphologyRecognitionCache::new(chars.len());
        let shared =
            fuhivla_head_ends_until_with_cache(&chars, 0, chars.len(), chars.len(), &mut cache);
        let brivla = cache
            .brivla_head_ends(0, chars.len(), chars.len())
            .unwrap_or_else(|| panic!("brivla head list is missing"));
        let fuhivla = cache
            .fuhivla_head_ends(0, chars.len(), chars.len())
            .unwrap_or_else(|| panic!("fuhivla head list is missing"));
        assert!(Rc::ptr_eq(&shared, &brivla));
        assert!(Rc::ptr_eq(&fuhivla, &brivla));
    }

    /// The pre-scan recurrence, counted at each entry to `coda_ends`.
    ///
    /// The numbers at lengths 10, 13, 16, and 19 are the ones measured on the
    /// dense-table head. This copy exists so the scan can be compared with that
    /// recurrence on inputs small enough for the recurrence to finish.
    #[requires(index <= end && end <= chars.len())]
    #[ensures(true)]
    fn legacy_coda_ends(chars: &[char], index: usize, end: usize) -> (Vec<usize>, u64, bool) {
        #[requires(index <= end && end <= chars.len())]
        #[ensures(true)]
        fn coda(chars: &[char], index: usize, end: usize, calls: &mut u64) -> Vec<usize> {
            *calls = calls.saturating_add(1);
            let mut ends = vec![index];
            if index < end
                && is_consonant(chars[index])
                && !starts(chars, index, end, calls)
                && starts(chars, index + 1, end, calls)
            {
                ends.push(index + 1);
            }
            ends
        }
        #[requires(end <= chars.len())]
        #[ensures(true)]
        fn starts(chars: &[char], index: usize, end: usize, calls: &mut u64) -> bool {
            if index >= end {
                return false;
            }
            if !consonantal(chars, index, end, calls).is_empty() {
                return true;
            }
            brivla_onset_has_nucleus_end_at_or_before(chars, index, end)
        }
        #[requires(end <= chars.len())]
        #[ensures(true)]
        fn consonantal(chars: &[char], index: usize, end: usize, calls: &mut u64) -> Vec<usize> {
            if index >= end
                || !is_consonant(chars[index])
                || !chars
                    .get(index + 1)
                    .is_some_and(|value| is_syllabic(*value))
            {
                return Vec::new();
            }
            coda(chars, index + 1, end, calls)
                .into_iter()
                .filter(|coda_end| *coda_end > index + 1)
                .collect()
        }
        let mut calls = 0;
        let ends = coda(chars, index, end, &mut calls);
        // `starts` calls `coda` again. The published counts above are the first
        // `coda` entry total, so keep that total before the extra lookup.
        let call_count = calls;
        let starts_here = starts(chars, index, end, &mut calls);
        (ends, call_count, starts_here)
    }

    /// Issue #1093. A consonantal syllable can follow a stressed syllable that
    /// ends in a consonant. It can also stand before the stress, after an open
    /// syllable or a closed one. A pair such as `tl` can be that syllable even
    /// when it is not a valid onset. The lists are the camxes syllable splits.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn consonantal_syllable_fuhivla_match_camxes() {
        let accepted: &[(&str, &str, &str, &[&str])] = &[
            ("kanbrti", "Fuhivla", "kánbrti", &["kan", "br", "ti"]),
            ("tanbrti", "Fuhivla", "tánbrti", &["tan", "br", "ti"]),
            ("kanbrta", "Fuhivla", "kánbrta", &["kan", "br", "ta"]),
            ("tarbrtu", "Fuhivla", "tárbrtu", &["tar", "br", "tu"]),
            (
                "kanbrtana",
                "Fuhivla",
                "kanbrtána",
                &["kan", "br", "ta", "na"],
            ),
            ("kanbrata", "Lujvo", "kanbráta", &["kan", "bra", "ta"]),
            (
                "cmacrntegrale",
                "Fuhivla",
                "cmacrntegrále",
                &["cmac", "rn", "te", "gra", "le"],
            ),
            (
                "tarmrnintegrale",
                "Fuhivla",
                "tarmrnintegrále",
                &["tar", "mr", "nin", "te", "gra", "le"],
            ),
            ("cmacrnte", "Fuhivla", "cmácrnte", &["cmac", "rn", "te"]),
            (
                "kanbrmrti",
                "Fuhivla",
                "kánbrmrti",
                &["kan", "br", "mr", "ti"],
            ),
            (
                "kanbrlnti",
                "Fuhivla",
                "kánbrlnti",
                &["kan", "br", "ln", "ti"],
            ),
            (
                "kanbrmrtana",
                "Fuhivla",
                "kanbrmrtána",
                &["kan", "br", "mr", "ta", "na"],
            ),
            ("kánbrti", "Fuhivla", "kánbrti", &["kán", "br", "ti"]),
            (
                "cmacrntegrále",
                "Fuhivla",
                "cmacrntegrále",
                &["cmac", "rn", "te", "grá", "le"],
            ),
            (
                "tamrnitegrale",
                "Fuhivla",
                "tamrnitegrále",
                &["ta", "mr", "ni", "te", "gra", "le"],
            ),
            (
                "kabrnitegrale",
                "Fuhivla",
                "kabrnitegrále",
                &["ka", "br", "ni", "te", "gra", "le"],
            ),
            (
                "kanrnlati",
                "Fuhivla",
                "kanrnláti",
                &["kan", "rn", "la", "ti"],
            ),
            ("cmajriti", "Fuhivla", "cmajríti", &["cmaj", "ri", "ti"]),
            ("cmagniti", "Fuhivla", "cmagníti", &["cmag", "ni", "ti"]),
            (
                "cmatclatana",
                "Fuhivla",
                "cmatclatána",
                &["cmat", "cla", "ta", "na"],
            ),
            (
                "cidjrspageti",
                "Fuhivla",
                "cidjrspagéti",
                &["cid", "jr", "spa", "ge", "ti"],
            ),
            (
                "barlrlrlrla",
                "Fuhivla",
                "bárlrlrlrla",
                &["bar", "lr", "lr", "lr", "la"],
            ),
            ("kanrtlti", "Fuhivla", "kánrtlti", &["ka", "nr", "tl", "ti"]),
            (
                "kanrtltana",
                "Fuhivla",
                "kanrtltána",
                &["ka", "nr", "tl", "ta", "na"],
            ),
            ("kartlti", "Fuhivla", "kártlti", &["kar", "tl", "ti"]),
            ("cmartlti", "Fuhivla", "cmártlti", &["cmar", "tl", "ti"]),
            (
                "tarmrtlti",
                "Fuhivla",
                "tármrtlti",
                &["tar", "mr", "tl", "ti"],
            ),
            ("kandlti", "Fuhivla", "kándlti", &["kan", "dl", "ti"]),
            (
                "kanbrtlti",
                "Fuhivla",
                "kánbrtlti",
                &["kan", "br", "tl", "ti"],
            ),
            ("kanmlti", "Fuhivla", "kánmlti", &["kan", "ml", "ti"]),
            ("kanrdlti", "Fuhivla", "kánrdlti", &["ka", "nr", "dl", "ti"]),
            ("kadbmti", "Fuhivla", "kádbmti", &["kad", "bm", "ti"]),
            (
                "kadbmtana",
                "Fuhivla",
                "kadbmtána",
                &["kad", "bm", "ta", "na"],
            ),
            ("kadbnti", "Fuhivla", "kádbnti", &["kad", "bn", "ti"]),
            (
                "kadbntana",
                "Fuhivla",
                "kadbntána",
                &["kad", "bn", "ta", "na"],
            ),
            ("kabdlti", "Fuhivla", "kábdlti", &["kab", "dl", "ti"]),
            (
                "kabdltana",
                "Fuhivla",
                "kabdltána",
                &["kab", "dl", "ta", "na"],
            ),
            ("kabdmti", "Fuhivla", "kábdmti", &["kab", "dm", "ti"]),
            (
                "kabdmtana",
                "Fuhivla",
                "kabdmtána",
                &["kab", "dm", "ta", "na"],
            ),
            ("kabdnti", "Fuhivla", "kábdnti", &["kab", "dn", "ti"]),
            (
                "kabdntana",
                "Fuhivla",
                "kabdntána",
                &["kab", "dn", "ta", "na"],
            ),
            ("kacfmti", "Fuhivla", "kácfmti", &["kac", "fm", "ti"]),
            (
                "kacfmtana",
                "Fuhivla",
                "kacfmtána",
                &["kac", "fm", "ta", "na"],
            ),
            ("kacfnti", "Fuhivla", "kácfnti", &["kac", "fn", "ti"]),
            (
                "kacfntana",
                "Fuhivla",
                "kacfntána",
                &["kac", "fn", "ta", "na"],
            ),
            ("kabgmti", "Fuhivla", "kábgmti", &["kab", "gm", "ti"]),
            (
                "kabgmtana",
                "Fuhivla",
                "kabgmtána",
                &["kab", "gm", "ta", "na"],
            ),
            ("kabgnti", "Fuhivla", "kábgnti", &["kab", "gn", "ti"]),
            (
                "kabgntana",
                "Fuhivla",
                "kabgntána",
                &["kab", "gn", "ta", "na"],
            ),
            ("kabjlti", "Fuhivla", "kábjlti", &["kab", "jl", "ti"]),
            (
                "kabjltana",
                "Fuhivla",
                "kabjltána",
                &["kab", "jl", "ta", "na"],
            ),
            ("kabjnti", "Fuhivla", "kábjnti", &["kab", "jn", "ti"]),
            (
                "kabjntana",
                "Fuhivla",
                "kabjntána",
                &["kab", "jn", "ta", "na"],
            ),
            ("kabjrti", "Fuhivla", "kábjrti", &["kab", "jr", "ti"]),
            (
                "kabjrtana",
                "Fuhivla",
                "kabjrtána",
                &["kab", "jr", "ta", "na"],
            ),
            ("kackmti", "Fuhivla", "káckmti", &["kac", "km", "ti"]),
            (
                "kackmtana",
                "Fuhivla",
                "kackmtána",
                &["kac", "km", "ta", "na"],
            ),
            ("kacknti", "Fuhivla", "kácknti", &["kac", "kn", "ti"]),
            (
                "kackntana",
                "Fuhivla",
                "kackntána",
                &["kac", "kn", "ta", "na"],
            ),
            ("kablmti", "Fuhivla", "káblmti", &["kab", "lm", "ti"]),
            (
                "kablmtana",
                "Fuhivla",
                "kablmtána",
                &["kab", "lm", "ta", "na"],
            ),
            ("kablnti", "Fuhivla", "káblnti", &["kab", "ln", "ti"]),
            (
                "kablntana",
                "Fuhivla",
                "kablntána",
                &["kab", "ln", "ta", "na"],
            ),
            ("kablrti", "Fuhivla", "káblrti", &["kab", "lr", "ti"]),
            (
                "kablrtana",
                "Fuhivla",
                "kablrtána",
                &["kab", "lr", "ta", "na"],
            ),
            ("kabmnti", "Fuhivla", "kábmnti", &["kab", "mn", "ti"]),
            (
                "kabmntana",
                "Fuhivla",
                "kabmntána",
                &["kab", "mn", "ta", "na"],
            ),
            ("kabnlti", "Fuhivla", "kábnlti", &["kab", "nl", "ti"]),
            (
                "kabnltana",
                "Fuhivla",
                "kabnltána",
                &["kab", "nl", "ta", "na"],
            ),
            ("kabnmti", "Fuhivla", "kábnmti", &["kab", "nm", "ti"]),
            (
                "kabnmtana",
                "Fuhivla",
                "kabnmtána",
                &["kab", "nm", "ta", "na"],
            ),
            ("kabnrti", "Fuhivla", "kábnrti", &["kab", "nr", "ti"]),
            (
                "kabnrtana",
                "Fuhivla",
                "kabnrtána",
                &["kab", "nr", "ta", "na"],
            ),
            ("kacpmti", "Fuhivla", "kácpmti", &["kac", "pm", "ti"]),
            (
                "kacpmtana",
                "Fuhivla",
                "kacpmtána",
                &["kac", "pm", "ta", "na"],
            ),
            ("kacpnti", "Fuhivla", "kácpnti", &["kac", "pn", "ti"]),
            (
                "kacpntana",
                "Fuhivla",
                "kacpntána",
                &["kac", "pn", "ta", "na"],
            ),
            ("kabrlti", "Fuhivla", "kábrlti", &["kab", "rl", "ti"]),
            (
                "kabrltana",
                "Fuhivla",
                "kabrltána",
                &["kab", "rl", "ta", "na"],
            ),
            ("kabrmti", "Fuhivla", "kábrmti", &["kab", "rm", "ti"]),
            (
                "kabrmtana",
                "Fuhivla",
                "kabrmtána",
                &["kab", "rm", "ta", "na"],
            ),
            ("kabrnti", "Fuhivla", "kábrnti", &["kab", "rn", "ti"]),
            (
                "kabrntana",
                "Fuhivla",
                "kabrntána",
                &["kab", "rn", "ta", "na"],
            ),
            ("kactlti", "Fuhivla", "káctlti", &["kac", "tl", "ti"]),
            (
                "kactltana",
                "Fuhivla",
                "kactltána",
                &["kac", "tl", "ta", "na"],
            ),
            ("kactmti", "Fuhivla", "káctmti", &["kac", "tm", "ti"]),
            (
                "kactmtana",
                "Fuhivla",
                "kactmtána",
                &["kac", "tm", "ta", "na"],
            ),
            ("kactnti", "Fuhivla", "káctnti", &["kac", "tn", "ti"]),
            (
                "kactntana",
                "Fuhivla",
                "kactntána",
                &["kac", "tn", "ta", "na"],
            ),
            ("kabvmti", "Fuhivla", "kábvmti", &["kab", "vm", "ti"]),
            (
                "kabvmtana",
                "Fuhivla",
                "kabvmtána",
                &["kab", "vm", "ta", "na"],
            ),
            ("kabvnti", "Fuhivla", "kábvnti", &["kab", "vn", "ti"]),
            (
                "kabvntana",
                "Fuhivla",
                "kabvntána",
                &["kab", "vn", "ta", "na"],
            ),
            ("kafxmti", "Fuhivla", "káfxmti", &["kaf", "xm", "ti"]),
            (
                "kafxmtana",
                "Fuhivla",
                "kafxmtána",
                &["kaf", "xm", "ta", "na"],
            ),
            ("kafxnti", "Fuhivla", "káfxnti", &["kaf", "xn", "ti"]),
            (
                "kafxntana",
                "Fuhivla",
                "kafxntána",
                &["kaf", "xn", "ta", "na"],
            ),
            ("kabzlti", "Fuhivla", "kábzlti", &["kab", "zl", "ti"]),
            (
                "kabzltana",
                "Fuhivla",
                "kabzltána",
                &["kab", "zl", "ta", "na"],
            ),
            ("kabznti", "Fuhivla", "kábznti", &["kab", "zn", "ti"]),
            (
                "kabzntana",
                "Fuhivla",
                "kabzntána",
                &["kab", "zn", "ta", "na"],
            ),
            ("kabzrti", "Fuhivla", "kábzrti", &["kab", "zr", "ti"]),
            (
                "kabzrtana",
                "Fuhivla",
                "kabzrtána",
                &["kab", "zr", "ta", "na"],
            ),
        ];
        for &(word, kind, phonemes, syllables) in accepted {
            let (got_kind, got_phonemes) = classify_word(word).unwrap_or_else(|| panic!("{word}"));
            assert_eq!(format!("{got_kind:?}"), kind, "{word}");
            assert_eq!(got_phonemes, phonemes, "{word}");
            let got_syllables =
                pronunciation_syllable_texts(word).unwrap_or_else(|| panic!("{word} syllables"));
            assert_eq!(
                got_syllables.iter().map(String::as_str).collect::<Vec<_>>(),
                syllables,
                "{word}"
            );
            let (again_kind, again_phonemes) = classify_word(&got_phonemes)
                .unwrap_or_else(|| panic!("{word} stress mark {got_phonemes}"));
            assert_eq!(format!("{again_kind:?}"), kind, "{word} stress mark");
            assert_eq!(again_phonemes, got_phonemes, "{word} stress mark");
            if kind == "Lujvo" {
                assert_eq!(
                    morphology_classes(word),
                    morphology_classes(&got_phonemes),
                    "{word} stress mark split"
                );
            }
        }
        // Camxes rejects these two. jbotci accepts them with the experimental
        // consonant-glide warning. See `fuhivla_rafsi_lookahead_matches_camxes`.
        for word in ["cmajria", "cmagnia"] {
            let (got_kind, _) = classify_word(word).unwrap_or_else(|| panic!("{word}"));
            assert_eq!(got_kind, WordKind::Fuhivla, "{word}");
        }
        for word in [
            "kanbrtí",
            "cmácrntegrale",
            "brlnti",
            "kanbxti",
            "banrtlinganu",
            "bangrtlingana",
            "kartlati",
            "kannrti",
        ] {
            assert!(classify_word(word).is_none(), "{word}");
        }
    }
}
