#[allow(unused_imports)]
use bityzba::{ensures, requires};

use crate::WordKind;

use super::text_chars;

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn is_fast_plain_lujvo_char(value: char) -> bool {
    is_fast_vowel(value) || is_fast_consonant(value) || value == 'y'
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn is_fast_vowel(value: char) -> bool {
    matches!(value, 'a' | 'e' | 'i' | 'o' | 'u')
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
pub(super) fn is_fast_initial_pair_chars(first: char, second: char) -> bool {
    INITIAL_PAIRS.contains(&format!("{first}{second}").as_str())
}

pub(super) const INITIAL_PAIRS: &[&str] = &[
    "bl", "br", "cf", "ck", "cl", "cm", "cn", "cp", "cr", "ct", "dj", "dr", "dz", "fl", "fr", "gl",
    "gr", "jb", "jd", "jg", "jm", "jv", "kl", "kr", "ml", "mr", "pl", "pr", "sf", "sk", "sl", "sm",
    "sn", "sp", "sr", "st", "tc", "tr", "ts", "vl", "vr", "xl", "xr", "zb", "zd", "zg", "zm", "zv",
];

#[requires(true)]
#[ensures(true)]
pub(super) fn is_fast_permissible_consonant_pair(first: char, second: char) -> bool {
    matches!(fast_consonant_pair_class(first, second), Some(1 | 2))
}

#[requires(true)]
#[ensures(true)]
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
