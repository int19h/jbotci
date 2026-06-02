use bityzba::{data, ensures, requires};

use crate::{Word, WordLike, WordLikeData};

#[requires(true)]
#[ensures(true)]
pub fn word_like_syntax_eq(left: &WordLike, right: &WordLike) -> bool {
    match (left.as_data(), right.as_data()) {
        (data!(WordLike::PlainWord(left)), data!(WordLike::PlainWord(right))) => {
            word_syntax_eq(left, right)
        }
        (
            data!(WordLike::QuotedWord {
                zo: left_zo,
                word: left_word,
            }),
            data!(WordLike::QuotedWord {
                zo: right_zo,
                word: right_word,
            }),
        ) => word_syntax_eq(left_zo, right_zo) && word_syntax_eq(left_word, right_word),
        (
            data!(WordLike::DelimitedNonLojbanQuote {
                zoi: left_zoi,
                opening_delimiter: left_opening,
                quoted_text: left_quoted,
                closing_delimiter: left_closing,
            }),
            data!(WordLike::DelimitedNonLojbanQuote {
                zoi: right_zoi,
                opening_delimiter: right_opening,
                quoted_text: right_quoted,
                closing_delimiter: right_closing,
            }),
        ) => {
            word_syntax_eq(left_zoi, right_zoi)
                && word_syntax_eq(left_opening, right_opening)
                && left_quoted == right_quoted
                && word_syntax_eq(left_closing, right_closing)
        }
        (
            data!(WordLike::QuotedWords {
                lohu: left_lohu,
                quoted_words: left_words,
                lehu: left_lehu,
            }),
            data!(WordLike::QuotedWords {
                lohu: right_lohu,
                quoted_words: right_words,
                lehu: right_lehu,
            }),
        ) => {
            word_syntax_eq(left_lohu, right_lohu)
                && left_words.len() == right_words.len()
                && left_words
                    .iter()
                    .zip(right_words.iter())
                    .all(|(left, right)| word_syntax_eq(left, right))
                && word_syntax_eq(left_lehu, right_lehu)
        }
        (
            data!(WordLike::DelimitedWordQuote {
                marker: left_marker,
                quoted_text: left_quoted,
            }),
            data!(WordLike::DelimitedWordQuote {
                marker: right_marker,
                quoted_text: right_quoted,
            }),
        ) => word_syntax_eq(left_marker, right_marker) && left_quoted == right_quoted,
        (
            data!(WordLike::LerfuWord {
                base: left_base,
                bu: left_bu,
            }),
            data!(WordLike::LerfuWord {
                base: right_base,
                bu: right_bu,
            }),
        ) => word_like_syntax_eq(left_base, right_base) && word_syntax_eq(left_bu, right_bu),
        (
            data!(WordLike::ZeiCompound {
                left: left_left,
                zei: left_zei,
                right: left_right,
            }),
            data!(WordLike::ZeiCompound {
                left: right_left,
                zei: right_zei,
                right: right_right,
            }),
        ) => {
            word_like_syntax_eq(left_left, right_left)
                && word_syntax_eq(left_zei, right_zei)
                && word_syntax_eq(left_right, right_right)
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
pub fn word_syntax_eq(left: &Word, right: &Word) -> bool {
    left.kind() == right.kind()
        && strip_diacritics(left.phonemes().as_str()) == strip_diacritics(right.phonemes().as_str())
}

#[ensures(!ret.is_empty() || text.is_empty())]
#[requires(true)]
pub fn strip_diacritics(text: &str) -> String {
    text.chars().filter_map(strip_diacritic).collect()
}

#[requires(true)]
#[ensures(true)]
fn strip_diacritic(value: char) -> Option<char> {
    Some(match value {
        'á' | 'à' | 'Á' | 'À' => 'a',
        'é' | 'è' | 'É' | 'È' => 'e',
        'í' | 'ì' | 'ĭ' | 'Ĭ' | 'Í' | 'Ì' => 'i',
        'ó' | 'ò' | 'Ó' | 'Ò' => 'o',
        'ú' | 'ù' | 'ŭ' | 'Ŭ' | 'Ú' | 'Ù' => 'u',
        'ý' | 'ỳ' | 'Ý' | 'Ỳ' => 'y',
        '\u{0301}' | '\u{0300}' | '\u{0306}' => return None,
        other => other,
    })
}
