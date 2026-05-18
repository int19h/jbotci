use bityzba::{ensures, requires};

use crate::{MorphologyOptions, WordKind};

mod fast;
pub(crate) use fast::classify_fast_simple_word;
use fast::{INITIAL_PAIRS, is_fast_initial_pair_chars, is_fast_permissible_consonant_pair};

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
    let blocks_brivla = blocks_word_shape(&normalized_chars, options);

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
            canonicalize_word_phonemes(normalized_word),
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
    canonicalize_word_phonemes(normalized_word)
        .chars()
        .filter(|value| *value != ',')
        .collect()
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
    if starts_with_nucleus(chars, end) {
        return None;
    }
    Some((format!("{}{}", normalize_vowel(first), semivowel), end))
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
pub(crate) fn is_cmevla_with_options(normalized: &str, options: &MorphologyOptions) -> bool {
    let chars = text_chars(normalized);
    chars.last().is_some_and(|last| is_consonant(*last))
        && chars.first().is_some_and(|first| *first != '\'')
        && !blocks_word_shape(&chars, options)
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
fn blocks_word_shape(chars: &[char], options: &MorphologyOptions) -> bool {
    has_invalid_apostrophe(chars)
        || has_geminated_consonant(chars)
        || has_y_hiatus(chars)
        || (options.enforce_cgv_ban && contains_cgv(chars))
}

#[requires(true)]
#[ensures(true)]
fn has_invalid_apostrophe(chars: &[char]) -> bool {
    chars.iter().enumerate().any(|(index, value)| {
        *value == '\''
            && (!previous_non_comma(chars, index)
                .is_some_and(|(_, previous)| can_precede_apostrophe(previous))
                || !starts_with_nucleus(chars, index + 1))
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
    chars.iter().enumerate().any(|(index, value)| {
        is_consonant(*value)
            && next_non_comma_index(chars, index + 1).is_some_and(|next| chars[next] == *value)
    })
}

#[requires(true)]
#[ensures(true)]
fn has_forbidden_consonant_triple(chars: &[char]) -> bool {
    consonant_runs(chars).iter().any(|run| {
        run.windows(3)
            .any(|triple| matches!(triple, ['n', 'd', 'j' | 'z'] | ['n', 't', 'c' | 's']))
    })
}

#[requires(true)]
#[ensures(ret.iter().all(|run| run.iter().all(|value| is_consonant(*value))))]
fn consonant_runs(chars: &[char]) -> Vec<Vec<char>> {
    let mut runs = Vec::new();
    let mut current = Vec::new();
    for value in chars.iter().copied() {
        if is_consonant(value) {
            current.push(value);
        } else if !current.is_empty() {
            runs.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        runs.push(current);
    }
    runs
}

#[requires(true)]
#[ensures(true)]
fn has_forbidden_consonant_pair(chars: &[char]) -> bool {
    chars.iter().enumerate().any(|(index, value)| {
        is_consonant(*value)
            && next_non_comma_index(chars, index + 1).is_some_and(|next| {
                is_consonant(chars[next])
                    && !is_fast_permissible_consonant_pair(*value, chars[next])
            })
    })
}

#[requires(true)]
#[ensures(true)]
fn has_digit_followed_by_nucleus(chars: &[char]) -> bool {
    chars.iter().enumerate().any(|(index, value)| {
        value.is_ascii_digit()
            && next_non_comma_index(chars, index + 1)
                .is_some_and(|next| starts_with_nucleus(chars, next))
    })
}

#[requires(true)]
#[ensures(true)]
fn has_y_hiatus(chars: &[char]) -> bool {
    chars.iter().enumerate().any(|(index, value)| {
        is_y(*value)
            && next_non_comma_index(chars, index + 1)
                .is_some_and(|next| !is_y(chars[next]) && starts_with_nucleus(chars, next))
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
#[ensures(true)]
fn contains_cgv(chars: &[char]) -> bool {
    for (index, value) in chars.iter().copied().enumerate() {
        if !matches!(value, 'i' | 'í' | 'ĭ' | 'u' | 'ú' | 'ŭ') || !starts_glide(chars, index) {
            continue;
        }
        if previous_non_comma(chars, index).is_some_and(|(_, previous)| is_consonant(previous)) {
            return true;
        }
    }
    false
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
                && is_fast_permissible_consonant_pair(*c, *d))
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
    let chars = text_chars(word);
    if chars.len() <= 3 || !chars.iter().all(|value| is_lujvo_char(*value)) {
        return false;
    }
    lujvo_from(&chars, 0, false)
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
    let slice = &chars[start..end];
    if rafsi_string_slice(chars, start, end) || slinkuhi_slice(chars, start, end) {
        return false;
    }
    if !starts_with_valid_word_onset(chars, start) {
        return false;
    }
    slice.iter().any(|value| is_consonant(*value))
        && has_consonant_cluster(slice)
        && !is_cmavo_slice(chars, start, end)
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
    for index in 0..chars.len() {
        if !is_vowel(chars[index]) {
            continue;
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
        if next_starts_nucleus(chars, index + 1) {
            return true;
        }
    }
    false
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
