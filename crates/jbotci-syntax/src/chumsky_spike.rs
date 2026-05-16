use std::ops::Range;

use chumsky::error::{Rich, RichReason};
use chumsky::input::{Input, MappedInput};
use chumsky::prelude::*;
use chumsky::span::{SimpleSpan, Spanned};
use jbotci_morphology::{WordKind, WordLike, WordWithModifiers};

use crate::{SpikeSentence, SyntaxError};

type Span = SimpleSpan;
type Token = WordWithModifiers;
type SpannedToken = Spanned<Token, Span>;
type ParserInput<'tokens> = MappedInput<'tokens, Token, Span, &'tokens [SpannedToken]>;
type ParseExtra<'tokens> = extra::Err<Rich<'tokens, Token, Span>>;

pub(crate) fn parse_mi_relation_do(
    words: &[WordWithModifiers],
) -> Result<SpikeSentence, SyntaxError> {
    let tokens = spanned_tokens(words);
    let eoi_offset = tokens.last().map_or(0, |token| token.span.end);

    parser()
        .parse(
            tokens
                .as_slice()
                .split_spanned(SimpleSpan::from(eoi_offset..eoi_offset)),
        )
        .into_result()
        .map_err(syntax_error)
}

fn parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, SpikeSentence, ParseExtra<'tokens>> {
    group((
        token_matching("cmavo `mi`", is_bare_mi),
        token_matching("relation word", is_bare_relation_word),
        token_matching("cmavo `do`", is_bare_do),
    ))
    .then_ignore(end())
    .map(|(subject, relation, object)| SpikeSentence {
        subject,
        relation,
        object,
    })
}

fn token_matching<'tokens>(
    label: &'static str,
    predicate: fn(&WordWithModifiers) -> bool,
) -> impl Parser<'tokens, ParserInput<'tokens>, WordWithModifiers, ParseExtra<'tokens>> {
    any().try_map(move |word: WordWithModifiers, span| {
        if predicate(&word) {
            Ok(word)
        } else {
            Err(Rich::custom(span, format!("expected {label}")))
        }
    })
}

fn spanned_tokens(words: &[WordWithModifiers]) -> Vec<SpannedToken> {
    words
        .iter()
        .cloned()
        .map(|word| {
            let range = word_byte_range(&word).unwrap_or(0..0);
            Spanned {
                inner: word,
                span: SimpleSpan::from(range),
            }
        })
        .collect()
}

fn word_byte_range(word: &WordWithModifiers) -> Option<Range<usize>> {
    match word {
        WordWithModifiers::BaseWord { word_like } => word_like_byte_range(word_like.as_ref()),
        WordWithModifiers::StandaloneIndicator { indicator, nai } => {
            Some(indicator.span.byte_start..nai.as_ref().unwrap_or(indicator).span.byte_end)
        }
        WordWithModifiers::Emphasized { bahe, word_like } => word_like_byte_range(word_like)
            .map(|range| bahe.span.byte_start.min(range.start)..bahe.span.byte_end.max(range.end)),
        WordWithModifiers::WithIndicator {
            base,
            indicator,
            nai,
        } => word_byte_range(base).map(|range| {
            range.start
                ..nai
                    .as_ref()
                    .unwrap_or(indicator)
                    .span
                    .byte_end
                    .max(range.end)
        }),
        WordWithModifiers::NotEof => None,
    }
}

fn word_like_byte_range(word_like: &WordLike) -> Option<Range<usize>> {
    match word_like {
        WordLike::Bare { word } => Some(word.span.byte_start..word.span.byte_end),
        WordLike::ZoQuote { zo, word } => Some(zo.span.byte_start..word.span.byte_end),
        WordLike::ZoiQuote {
            zoi,
            closing_delimiter,
            ..
        } => Some(zoi.span.byte_start..closing_delimiter.span.byte_end),
        WordLike::LohuQuote { lohu, lehu, .. } => Some(lohu.span.byte_start..lehu.span.byte_end),
        WordLike::SingleWordQuote {
            marker,
            quoted_text,
        } => Some(marker.span.byte_start..quoted_text.byte_end),
        WordLike::Letter { base, bu } => {
            word_like_byte_range(base).map(|range| range.start..bu.span.byte_end.max(range.end))
        }
        WordLike::ZeiLujvo { left, right, .. } => {
            word_like_byte_range(left).map(|range| range.start..right.span.byte_end.max(range.end))
        }
    }
}

fn is_bare_mi(word: &WordWithModifiers) -> bool {
    bare_word_kind_and_phonemes(word) == Some((WordKind::Cmavo, "mi"))
}

fn is_bare_do(word: &WordWithModifiers) -> bool {
    bare_word_kind_and_phonemes(word) == Some((WordKind::Cmavo, "do"))
}

fn is_bare_relation_word(word: &WordWithModifiers) -> bool {
    bare_word_kind_and_phonemes(word).is_some_and(|(kind, _)| {
        matches!(kind, WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla)
    })
}

fn bare_word_kind_and_phonemes(word: &WordWithModifiers) -> Option<(WordKind, &str)> {
    let WordWithModifiers::BaseWord { word_like } = word else {
        return None;
    };
    let WordLike::Bare { word } = word_like.as_ref() else {
        return None;
    };
    Some((word.kind, word.phonemes.as_str()))
}

fn syntax_error(errors: Vec<Rich<'_, WordWithModifiers, Span>>) -> SyntaxError {
    let Some(error) = errors.into_iter().next() else {
        return SyntaxError::Parse {
            byte_offset: 0,
            reason: "unknown Chumsky syntax error".to_owned(),
        };
    };

    let reason = match error.reason() {
        RichReason::Custom(message) => message.to_string(),
        _ => format!("{error:?}"),
    };

    SyntaxError::Parse {
        byte_offset: error.span().start,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use jbotci_morphology::{
        WordLike, WordWithModifiers, segment_words_with_modifiers_chumsky_spike,
    };

    use super::*;

    #[test]
    fn parses_mi_relation_do_token_stream() {
        let words =
            segment_words_with_modifiers_chumsky_spike("mi klama do").expect("valid morphology");

        let sentence = parse_mi_relation_do(&words).expect("valid syntax");

        let relation = bare_word(&sentence.relation).expect("bare relation word");
        assert_eq!(relation.phonemes, "klama");
        assert_eq!(relation.span.byte_start, 3);
        assert_eq!(relation.span.byte_end, 8);
    }

    #[test]
    fn reports_expected_relation_word() {
        let words = segment_words_with_modifiers_chumsky_spike("mi do").expect("valid morphology");

        let error = parse_mi_relation_do(&words).expect_err("syntax should fail");

        assert!(error.to_string().contains("expected relation word"));
    }

    fn bare_word(word: &WordWithModifiers) -> Option<&jbotci_morphology::Word> {
        match word {
            WordWithModifiers::BaseWord { word_like } => match word_like.as_ref() {
                WordLike::Bare { word } => Some(word),
                _ => None,
            },
            _ => None,
        }
    }
}
