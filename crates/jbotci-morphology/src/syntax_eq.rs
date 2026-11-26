use bityzba::data;
#[allow(unused_imports)]
use bityzba::ensures;
use bityzba::requires;

use crate::{Verbatim, Word, WordLike, WordLikeData, folded_lojban_diacritics_eq};

#[requires(true)]
#[ensures(true)]
pub fn word_like_syntax_eq(left: &WordLike, right: &WordLike) -> bool {
    let mut stack = vec![(left, right)];
    while let Some((left, right)) = stack.pop() {
        if !syntax_step(&mut stack, left, right) {
            return false;
        }
    }
    true
}

#[requires(true)]
#[ensures(true)]
fn syntax_step<'a>(
    stack: &mut Vec<(&'a WordLike, &'a WordLike)>,
    left: &'a WordLike,
    right: &'a WordLike,
) -> bool {
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
            data!(WordLike::SelmahoQuotedWord {
                mahoi: left_mahoi,
                word: left_word,
            }),
            data!(WordLike::SelmahoQuotedWord {
                mahoi: right_mahoi,
                word: right_word,
            }),
        ) => word_syntax_eq(left_mahoi, right_mahoi) && word_syntax_eq(left_word, right_word),
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
                && verbatim_syntax_eq(left_quoted, right_quoted)
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
            data!(WordLike::ReplacementQuote {
                runs: left,
                lehai: lc
            }),
            data!(WordLike::ReplacementQuote {
                runs: right,
                lehai: rc
            }),
        ) => {
            if left.len() != right.len() || !word_syntax_eq(lc, rc) {
                return false;
            }
            for (left_run, right_run) in left.iter().zip(right) {
                if !word_syntax_eq(&left_run.marker, &right_run.marker)
                    || left_run.words.len() != right_run.words.len()
                {
                    return false;
                }
                for (left_word, right_word) in left_run.words.iter().zip(&right_run.words) {
                    stack.push((left_word.word.as_ref(), right_word.word.as_ref()));
                }
            }
            true
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
        ) => {
            word_syntax_eq(left_marker, right_marker)
                && verbatim_syntax_eq(left_quoted, right_quoted)
        }
        (
            data!(WordLike::LerfuWord {
                base: left_base,
                bu: left_bu,
            }),
            data!(WordLike::LerfuWord {
                base: right_base,
                bu: right_bu,
            }),
        ) => {
            if !word_syntax_eq(left_bu, right_bu) {
                return false;
            }
            stack.push((left_base.as_ref(), right_base.as_ref()));
            true
        }
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
            if !word_syntax_eq(left_zei, right_zei) || !word_syntax_eq(left_right, right_right) {
                return false;
            }
            stack.push((left_left.as_ref(), right_left.as_ref()));
            true
        }
        _ => false,
    }
}

#[requires(true)]
#[ensures(ret == (left.text == right.text))]
fn verbatim_syntax_eq(left: &Verbatim, right: &Verbatim) -> bool {
    left.text == right.text
}

#[requires(true)]
#[ensures(true)]
pub fn word_syntax_eq(left: &Word, right: &Word) -> bool {
    if left.kind() != right.kind() {
        return false;
    }
    match (left.phonemes_ref(), right.phonemes_ref()) {
        (Some(left), Some(right)) => folded_lojban_diacritics_eq(left.as_str(), right.as_str()),
        _ => folded_lojban_diacritics_eq(left.phonemes().as_str(), right.phonemes().as_str()),
    }
}
