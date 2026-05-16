use jbotci_source::{SourceId, SourceSpan};

use crate::{MorphologyError, MorphologyOptions, Word, WordKind, WordLike, WordWithModifiers};

#[derive(Debug, Clone, Copy)]
struct SourceChar {
    byte_offset: usize,
    value: char,
}

#[derive(Debug)]
struct Segmenter<'a> {
    input: &'a str,
    options: &'a MorphologyOptions,
    source_id: Option<SourceId>,
    chars: Vec<SourceChar>,
    index: usize,
}

pub fn segment_words_with_modifiers(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    Segmenter::new(input, options, source_id).segment()
}

impl<'a> Segmenter<'a> {
    fn new(input: &'a str, options: &'a MorphologyOptions, source_id: Option<SourceId>) -> Self {
        Self {
            input,
            options,
            source_id,
            chars: input
                .char_indices()
                .map(|(byte_offset, value)| SourceChar { byte_offset, value })
                .collect(),
            index: 0,
        }
    }

    fn segment(mut self) -> Result<Vec<WordWithModifiers>, MorphologyError> {
        let mut words = Vec::new();
        loop {
            self.skip_leading_noise();
            if self.index == self.chars.len() {
                return Ok(words);
            }
            words.push(self.next_word()?);
        }
    }

    fn next_word(&mut self) -> Result<WordWithModifiers, MorphologyError> {
        let start = self.index;
        let end = self.candidate_end(start);
        let raw = self.slice(start, end);
        let normalized = normalize_word_with_options(raw, self.options);
        if normalized.is_empty() {
            return Err(MorphologyError::Invalid {
                char_offset: start,
                word: raw.to_owned(),
                reason: "no valid morphology characters".to_owned(),
            });
        }

        if let Some(kind) = classify_fast_simple_word(raw, &normalized) {
            self.index = end;
            return self.word_with_modifiers(start, end, kind, normalized);
        }

        if is_simple_cmevla(&normalized) {
            self.index = end;
            return self.word_with_modifiers(start, end, WordKind::Cmevla, normalized);
        }

        if let Some(cmavo) = self.cmavo_prefix(start, end) {
            self.index = cmavo.end;
            return self.word_with_modifiers(start, cmavo.end, WordKind::Cmavo, cmavo.phonemes);
        }

        Err(MorphologyError::Unsupported {
            char_offset: start,
            word: raw.to_owned(),
            reason: "the initial Rust port currently supports plain Latin cmavo, cmevla, gismu, and fast-path lujvo only".to_owned(),
        })
    }

    fn word_with_modifiers(
        &self,
        start: usize,
        end: usize,
        kind: WordKind,
        phonemes: String,
    ) -> Result<WordWithModifiers, MorphologyError> {
        let word = Word {
            kind,
            phonemes,
            span: SourceSpan::new(
                self.source_id.clone(),
                self.byte_offset(start),
                self.byte_offset(end),
                start,
                end,
            )?,
            surface_override: None,
            dialect_transform: None,
        };
        Ok(WordWithModifiers::BaseWord {
            word_like: Box::new(WordLike::Bare {
                word: Box::new(word),
            }),
        })
    }

    fn cmavo_prefix(&self, start: usize, end: usize) -> Option<CmavoPrefix> {
        ((start + 1)..=end).find_map(|prefix_end| {
            let phonemes = parse_cmavo_form(&normalize_word_with_options(
                self.slice(start, prefix_end),
                self.options,
            ))?;
            if self.cmavo_boundary_ok(prefix_end, end) {
                Some(CmavoPrefix {
                    end: prefix_end,
                    phonemes,
                })
            } else {
                None
            }
        })
    }

    fn cmavo_boundary_ok(&self, prefix_end: usize, candidate_end: usize) -> bool {
        if prefix_end == candidate_end {
            return true;
        }
        let remainder =
            normalize_word_with_options(self.slice(prefix_end, candidate_end), self.options);
        !starts_with_nucleus(&text_chars(&remainder), 0)
            && self.candidate_starts_with_supported_word(prefix_end, candidate_end)
    }

    fn candidate_starts_with_supported_word(&self, start: usize, end: usize) -> bool {
        let raw = self.slice(start, end);
        let normalized = normalize_word_with_options(raw, self.options);
        classify_fast_simple_word(raw, &normalized).is_some()
            || is_simple_cmevla(&normalized)
            || ((start + 1)..=end).any(|prefix_end| {
                parse_cmavo_form(&normalize_word_with_options(
                    self.slice(start, prefix_end),
                    self.options,
                ))
                .is_some()
            })
    }

    fn candidate_end(&self, start: usize) -> usize {
        let mut end = start;
        while end < self.chars.len() && !is_separator(self.chars[end].value) {
            end += 1;
        }
        end
    }

    fn skip_leading_noise(&mut self) {
        while self.index < self.chars.len()
            && (is_separator(self.chars[self.index].value) || self.chars[self.index].value == ',')
        {
            self.index += 1;
        }
    }

    fn slice(&self, start: usize, end: usize) -> &'a str {
        &self.input[self.byte_offset(start)..self.byte_offset(end)]
    }

    fn byte_offset(&self, index: usize) -> usize {
        self.chars
            .get(index)
            .map_or(self.input.len(), |source_char| source_char.byte_offset)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CmavoPrefix {
    end: usize,
    phonemes: String,
}

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

pub(crate) fn normalize_word_with_options(raw: &str, options: &MorphologyOptions) -> String {
    raw.chars()
        .filter_map(|value| normalize_char(value, options))
        .collect()
}

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

fn is_valid_normalized_char(value: char) -> bool {
    is_vowel(value)
        || is_consonant(value)
        || matches!(value, 'y' | 'ý' | '\'' | ',' | 'ĭ' | 'ŭ' | '0'..='9')
}

fn text_chars(text: &str) -> Vec<char> {
    text.chars().collect()
}

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

fn parse_initial(chars: &[char], start: usize, end: usize) -> Option<String> {
    let initial: String = chars.get(start..end)?.iter().collect();
    let valid_shape = match end - start {
        0 => true,
        1 => is_consonant(chars[start]),
        2 => INITIAL_PAIRS.contains(&initial.as_str()),
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

fn parse_diphthong(chars: &[char], start: usize) -> Option<(String, usize)> {
    let first = *chars.get(start)?;
    let second = *chars.get(start + 1)?;
    let semivowel = match (base_vowel(first)?, second) {
        ('a', 'i' | 'í' | 'ĭ') | ('e', 'i' | 'í' | 'ĭ') | ('o', 'i' | 'í' | 'ĭ') => 'ĭ',
        ('a', 'u' | 'ú' | 'ŭ') => 'ŭ',
        _ => return None,
    };
    let end = start + 2;
    if starts_with_nucleus(chars, end) {
        return None;
    }
    Some((format!("{}{}", normalize_vowel(first), semivowel), end))
}

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

fn starts_with_nucleus(chars: &[char], start: usize) -> bool {
    if start >= chars.len() {
        return false;
    }
    parse_diphthong(chars, start).is_some()
        || chars
            .get(start)
            .is_some_and(|value| is_vowel(*value) || matches!(*value, 'y' | 'ý'))
}

fn starts_with_cluster(chars: &[char], start: usize) -> bool {
    chars
        .get(start)
        .zip(chars.get(start + 1))
        .is_some_and(|(first, second)| is_consonant(*first) && is_consonant(*second))
}

pub(crate) fn is_simple_cmevla(normalized: &str) -> bool {
    let chars = text_chars(normalized);
    chars.last().is_some_and(|last| is_consonant(*last))
        && chars.first().is_some_and(|first| *first != '\'')
        && chars.iter().all(|value| {
            is_consonant(*value)
                || is_vowel(*value)
                || matches!(*value, 'y' | 'ý' | '\'' | ',' | '0'..='9')
        })
}

fn is_vowel(value: char) -> bool {
    base_vowel(value).is_some()
}

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

pub(crate) fn classify_fast_simple_word(raw_word: &str, normalized_word: &str) -> Option<WordKind> {
    if raw_word.is_empty() || normalized_word.is_empty() {
        return None;
    }
    if !raw_word.chars().all(is_fast_raw_word_char) {
        return None;
    }
    if is_fast_simple_gismu(normalized_word) {
        Some(WordKind::Gismu)
    } else if is_fast_simple_lujvo(normalized_word) {
        Some(WordKind::Lujvo)
    } else {
        None
    }
}

fn is_fast_raw_word_char(value: char) -> bool {
    matches!(
        value,
        'a' | 'b'
            | 'c'
            | 'd'
            | 'e'
            | 'f'
            | 'g'
            | 'i'
            | 'j'
            | 'k'
            | 'l'
            | 'm'
            | 'n'
            | 'o'
            | 'p'
            | 'r'
            | 's'
            | 't'
            | 'u'
            | 'v'
            | 'x'
            | 'y'
            | 'z'
            | '\''
    )
}

fn is_fast_simple_gismu(word: &str) -> bool {
    let chars = text_chars(word);
    match &chars[..] {
        [a, b, c, d, e] => {
            (is_fast_consonant(*a)
                && is_fast_vowel(*b)
                && is_fast_consonant(*c)
                && is_fast_consonant(*d)
                && is_fast_vowel(*e)
                && is_fast_permissible_consonant_pair(*c, *d))
                || (is_fast_initial_pair_chars(*a, *b)
                    && is_fast_vowel(*c)
                    && is_fast_consonant(*d)
                    && is_fast_vowel(*e))
        }
        _ => false,
    }
}

fn is_fast_simple_lujvo(word: &str) -> bool {
    if word.chars().count() <= 5 || !word.chars().all(is_fast_plain_lujvo_char) {
        return false;
    }
    let chars = text_chars(word);
    let split = chars.len() - 5;
    let prefix: String = chars[..split].iter().collect();
    let suffix: String = chars[split..].iter().collect();
    if !is_fast_simple_gismu(&suffix) {
        return false;
    }
    let Some(mut chunks) = fast_simple_rafsi_chunks(&prefix) else {
        return false;
    };
    if chunks.is_empty() {
        return false;
    }
    chunks.push(suffix);
    fast_rafsi_boundaries_are_valid(&chunks)
        && !is_fast_tosmabru_failure(&chunks[..chunks.len() - 1], chunks.last().expect("suffix"))
}

fn is_fast_plain_lujvo_char(value: char) -> bool {
    is_fast_vowel(value) || is_fast_consonant(value) || value == 'y'
}

fn fast_simple_rafsi_chunks(word: &str) -> Option<Vec<String>> {
    if word.is_empty() {
        return Some(Vec::new());
    }
    let chars = text_chars(word);
    if chars.len() < 3 {
        return None;
    }
    let chunk: String = chars[..3].iter().collect();
    if !is_fast_short_rafsi(&chunk) {
        return None;
    }
    let rest: String = chars[3..].iter().collect();
    let mut chunks = vec![chunk];
    chunks.extend(fast_simple_rafsi_chunks(&rest)?);
    Some(chunks)
}

fn is_fast_short_rafsi(rafsi: &str) -> bool {
    let chars = text_chars(rafsi);
    match &chars[..] {
        [a, b, c] => {
            (is_fast_consonant(*a) && is_fast_vowel(*b) && is_fast_consonant(*c))
                || (is_fast_initial_pair_chars(*a, *b) && is_fast_vowel(*c))
        }
        _ => false,
    }
}

fn fast_rafsi_boundaries_are_valid(parts: &[String]) -> bool {
    parts.windows(2).all(|window| {
        let left = &window[0];
        let right = &window[1];
        let Some(left_tail) = left.chars().last() else {
            return true;
        };
        let Some(right_head) = right.chars().next() else {
            return true;
        };
        if is_fast_consonant(left_tail) && is_fast_consonant(right_head) {
            is_fast_permissible_consonant_pair(left_tail, right_head)
                && !(left_tail == 'n'
                    && (right.starts_with("ts")
                        || right.starts_with("tc")
                        || right.starts_with("dz")
                        || right.starts_with("dj")))
        } else {
            true
        }
    })
}

fn is_fast_tosmabru_failure(prefix_chunks: &[String], suffix: &str) -> bool {
    if !prefix_chunks
        .iter()
        .all(|chunk| fast_syllables_pattern(chunk).as_deref() == Some("CVC"))
    {
        return false;
    }
    let suffix_chars = text_chars(suffix);
    match &suffix_chars[..] {
        [_c1, _v1, c2, c3, _v2] => {
            is_fast_initial_pair_chars(*c2, *c3) && {
                let mut parts = prefix_chunks.to_vec();
                parts.push(suffix.to_owned());
                parts.windows(2).all(|window| {
                    let left_tail = window[0].chars().last();
                    let right_head = window[1].chars().next();
                    left_tail
                        .zip(right_head)
                        .is_some_and(|(left, right)| is_fast_initial_pair_chars(left, right))
                })
            }
        }
        _ => false,
    }
}

fn fast_syllables_pattern(text: &str) -> Option<String> {
    text.chars()
        .map(|value| {
            if is_fast_vowel(value) {
                Some('V')
            } else if is_fast_consonant(value) {
                Some('C')
            } else if value == 'y' {
                Some('Y')
            } else if value == '\'' {
                Some('\'')
            } else {
                None
            }
        })
        .collect()
}

fn is_fast_vowel(value: char) -> bool {
    matches!(value, 'a' | 'e' | 'i' | 'o' | 'u')
}

fn is_fast_consonant(value: char) -> bool {
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

fn is_fast_initial_pair_chars(first: char, second: char) -> bool {
    INITIAL_PAIRS.contains(&format!("{first}{second}").as_str())
}

const INITIAL_PAIRS: &[&str] = &[
    "bl", "br", "cf", "ck", "cl", "cm", "cn", "cp", "cr", "ct", "dj", "dr", "dz", "fl", "fr", "gl",
    "gr", "jb", "jd", "jg", "jm", "jv", "kl", "kr", "ml", "mr", "pl", "pr", "sf", "sk", "sl", "sm",
    "sn", "sp", "sr", "st", "tc", "tr", "ts", "vl", "vr", "xl", "xr", "zb", "zd", "zg", "zm", "zv",
];

fn is_fast_permissible_consonant_pair(first: char, second: char) -> bool {
    matches!(fast_consonant_pair_class(first, second), Some(1 | 2))
}

fn fast_consonant_pair_class(first: char, second: char) -> Option<u8> {
    let first_index = FAST_CONSONANT_ORDER.find(first)?;
    let second_index = FAST_CONSONANT_ORDER.find(second)?;
    FAST_PAIR_MATRIX
        .get(first_index)
        .and_then(|row| row.get(second_index))
        .copied()
}

const FAST_CONSONANT_ORDER: &str = "rlnmbvdgjzscxktfp";

const FAST_PAIR_MATRIX: [[u8; 17]; 17] = [
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
