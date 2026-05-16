use chumsky::error::Rich;
use chumsky::prelude::*;
use chumsky::span::{SimpleSpan, Spanned};
use jbotci_source::{SourceId, SourceSpan};

use crate::{MorphologyError, MorphologyOptions, Word, WordKind, WordLike, WordWithModifiers};

type MorphExtra<'src> = extra::Err<Rich<'src, char>>;
type ZoiExtra<'src> = extra::Full<Rich<'src, char>, (), ZoiPrefix>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZoiPrefix {
    zoi: Word,
    opening_delimiter: Word,
    options: MorphologyOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZoiBody {
    quoted_text: SourceSpan,
    closing_delimiter: Word,
}

pub(crate) fn segment_words_with_modifiers(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    parser(input, options.clone(), source_id)
        .parse(input)
        .into_result()
        .map_err(|errors| morphology_error(input, errors))
}

fn parser<'src>(
    input: &'src str,
    options: MorphologyOptions,
    source_id: Option<SourceId>,
) -> impl Parser<'src, &'src str, Vec<WordWithModifiers>, MorphExtra<'src>> {
    choice((
        zoi_quote(input, options.clone(), source_id.clone()),
        zo_quote(input, options.clone(), source_id.clone()),
        non_magic_bare_word(input, options, source_id).map(base_word),
    ))
    .padded_by(separators())
    .repeated()
    .collect::<Vec<_>>()
    .then_ignore(end())
}

fn zo_quote<'src>(
    input: &'src str,
    options: MorphologyOptions,
    source_id: Option<SourceId>,
) -> impl Parser<'src, &'src str, WordWithModifiers, MorphExtra<'src>> {
    cmavo_marker(input, options.clone(), source_id.clone(), "zo", "ZO")
        .then_ignore(required_separator())
        .then(bare_word(input, options, source_id))
        .map(|(zo, word)| {
            base_word_like(WordLike::ZoQuote {
                zo: Box::new(zo),
                word: Box::new(word),
            })
        })
}

fn zoi_quote<'src>(
    input: &'src str,
    options: MorphologyOptions,
    source_id: Option<SourceId>,
) -> impl Parser<'src, &'src str, WordWithModifiers, MorphExtra<'src>> {
    let delimiter_options = options.clone();
    cmavo_marker(input, options.clone(), source_id.clone(), "zoĭ", "ZOI")
        .then_ignore(required_separator())
        .then(bare_word(input, options, source_id.clone()))
        .map(move |(zoi, opening_delimiter)| ZoiPrefix {
            zoi,
            opening_delimiter,
            options: delimiter_options.clone(),
        })
        .then_ignore(required_separator())
        .then_with_ctx(zoi_body(input, source_id))
        .map(|(prefix, body)| {
            base_word_like(WordLike::ZoiQuote {
                zoi: Box::new(prefix.zoi),
                opening_delimiter: Box::new(prefix.opening_delimiter),
                quoted_text: body.quoted_text,
                closing_delimiter: Box::new(body.closing_delimiter),
            })
        })
}

fn cmavo_marker<'src>(
    input: &'src str,
    options: MorphologyOptions,
    source_id: Option<SourceId>,
    phonemes: &'static str,
    label: &'static str,
) -> impl Parser<'src, &'src str, Word, MorphExtra<'src>> {
    bare_word(input, options, source_id).try_map(move |word, span| {
        if word.kind == WordKind::Cmavo && word.phonemes == phonemes {
            Ok(word)
        } else {
            Err(Rich::custom(span, format!("expected {label} marker")))
        }
    })
}

fn bare_word<'src>(
    input: &'src str,
    options: MorphologyOptions,
    source_id: Option<SourceId>,
) -> impl Parser<'src, &'src str, Word, MorphExtra<'src>> {
    raw_word().try_map(move |(raw, span), _| {
        word_from_raw(input, raw, span, &options, source_id.clone())
            .map_err(|reason| Rich::custom(span, reason))
    })
}

fn non_magic_bare_word<'src>(
    input: &'src str,
    options: MorphologyOptions,
    source_id: Option<SourceId>,
) -> impl Parser<'src, &'src str, Word, MorphExtra<'src>> {
    bare_word(input, options, source_id).try_map(|word, span| {
        if is_magic_marker(&word) {
            Err(Rich::custom(
                span,
                format!("expected {} marker to form a quote", word.phonemes.as_str()),
            ))
        } else {
            Ok(word)
        }
    })
}

fn is_magic_marker(word: &Word) -> bool {
    word.kind == WordKind::Cmavo && matches!(word.phonemes.as_str(), "zo" | "zoĭ")
}

fn zoi_body<'src>(
    input: &'src str,
    source_id: Option<SourceId>,
) -> impl Parser<'src, &'src str, ZoiBody, ZoiExtra<'src>> {
    custom::<_, &'src str, ZoiBody, ZoiExtra<'src>>(move |inp| {
        let before = inp.cursor();
        let start_span: SimpleSpan = inp.span_since(&before);
        let body_start = start_span.start;
        let prefix: ZoiPrefix = inp.ctx().clone();
        let Some(close) = find_zoi_close(input, body_start, &prefix, source_id.clone()) else {
            return Err(Rich::custom(
                inp.span_from(&before..),
                format!(
                    "expected closing ZOI delimiter `{}`",
                    prefix.opening_delimiter.phonemes
                ),
            ));
        };

        for _ in input[body_start..close.closing_delimiter.span.byte_end].chars() {
            inp.skip();
        }

        Ok(ZoiBody {
            quoted_text: close.quoted_text,
            closing_delimiter: close.closing_delimiter,
        })
    })
}

fn raw_word<'src>()
-> impl Parser<'src, &'src str, (&'src str, SimpleSpan), MorphExtra<'src>> + Clone {
    any()
        .filter(|value: &char| !crate::segment::is_separator(*value))
        .repeated()
        .at_least(1)
        .to_slice()
        .spanned()
        .map(|spanned: Spanned<&'src str, SimpleSpan>| (spanned.inner, spanned.span))
}

fn required_separator<'src>() -> impl Parser<'src, &'src str, (), MorphExtra<'src>> + Clone {
    any()
        .filter(|value: &char| crate::segment::is_separator(*value))
        .repeated()
        .at_least(1)
        .ignored()
}

fn separators<'src>() -> impl Parser<'src, &'src str, (), MorphExtra<'src>> + Clone {
    any()
        .filter(|value: &char| crate::segment::is_separator(*value))
        .repeated()
        .ignored()
}

fn base_word(word: Word) -> WordWithModifiers {
    base_word_like(WordLike::Bare {
        word: Box::new(word),
    })
}

fn base_word_like(word_like: WordLike) -> WordWithModifiers {
    WordWithModifiers::BaseWord {
        word_like: Box::new(word_like),
    }
}

fn word_from_raw(
    input: &str,
    raw: &str,
    span: SimpleSpan,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Word, String> {
    let normalized = crate::segment::normalize_word_with_options(raw, options);
    if normalized.is_empty() {
        return Err("no valid morphology characters".to_owned());
    }

    let (kind, phonemes) = if let Some(kind) =
        crate::segment::classify_fast_simple_word(raw, &normalized)
    {
        (kind, normalized)
    } else if crate::segment::is_simple_cmevla(&normalized) {
        (WordKind::Cmevla, normalized)
    } else if let Some(cmavo) = crate::segment::parse_cmavo_form(&normalized) {
        (WordKind::Cmavo, cmavo)
    } else {
        return Err("the Chumsky spike currently supports only simple cmavo, cmevla, gismu, and fast-path lujvo".to_owned());
    };

    Ok(Word {
        kind,
        phonemes,
        span: source_span(input, source_id, span.start, span.end)?,
        surface_override: None,
        dialect_transform: None,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZoiClose {
    quoted_text: SourceSpan,
    closing_delimiter: Word,
}

fn find_zoi_close(
    input: &str,
    body_start: usize,
    prefix: &ZoiPrefix,
    source_id: Option<SourceId>,
) -> Option<ZoiClose> {
    let mut cursor = body_start;
    while cursor < input.len() {
        let word_start = cursor;
        while cursor < input.len() {
            let value = input[cursor..].chars().next()?;
            if crate::segment::is_separator(value) {
                break;
            }
            cursor += value.len_utf8();
        }
        let word_end = cursor;
        if word_start < word_end {
            let raw = &input[word_start..word_end];
            let span = SimpleSpan::from(word_start..word_end);
            if let Ok(closing_delimiter) =
                word_from_raw(input, raw, span, &prefix.options, source_id.clone())
                && closing_delimiter.kind == prefix.opening_delimiter.kind
                && closing_delimiter.phonemes == prefix.opening_delimiter.phonemes
            {
                return Some(ZoiClose {
                    quoted_text: source_span(
                        input,
                        source_id,
                        body_start,
                        trim_trailing_separators(input, body_start, word_start),
                    )
                    .ok()?,
                    closing_delimiter,
                });
            }
        }
        while cursor < input.len() {
            let value = input[cursor..].chars().next()?;
            if !crate::segment::is_separator(value) {
                break;
            }
            cursor += value.len_utf8();
        }
    }
    None
}

fn trim_trailing_separators(input: &str, start: usize, end: usize) -> usize {
    let mut trimmed_end = end;
    while start < trimmed_end {
        let Some((offset, value)) = input[start..trimmed_end].char_indices().next_back() else {
            break;
        };
        if !crate::segment::is_separator(value) {
            break;
        }
        trimmed_end = start + offset;
    }
    trimmed_end
}

fn source_span(
    input: &str,
    source_id: Option<SourceId>,
    byte_start: usize,
    byte_end: usize,
) -> Result<SourceSpan, String> {
    SourceSpan::new(
        source_id,
        byte_start,
        byte_end,
        char_offset(input, byte_start),
        char_offset(input, byte_end),
    )
    .map_err(|error| error.to_string())
}

fn char_offset(input: &str, byte_offset: usize) -> usize {
    input[..byte_offset].chars().count()
}

fn morphology_error(input: &str, errors: Vec<Rich<'_, char>>) -> MorphologyError {
    let Some(error) = errors.into_iter().next() else {
        return MorphologyError::Invalid {
            char_offset: 0,
            word: String::new(),
            reason: "unknown Chumsky morphology error".to_owned(),
        };
    };
    let span = error.span();
    MorphologyError::Invalid {
        char_offset: char_offset(input, span.start),
        word: input
            .get(span.start..span.end)
            .unwrap_or_default()
            .to_owned(),
        reason: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_ordinary_sentence() {
        let words =
            segment_words_with_modifiers("mi klama do", &MorphologyOptions::default(), None)
                .expect("valid morphology");

        assert_eq!(bare_phonemes(&words), ["mi", "klama", "do"]);
        assert_eq!(bare_span(&words[1]).map(|span| span.byte_start), Some(3));
        assert_eq!(bare_span(&words[1]).map(|span| span.byte_end), Some(8));
    }

    #[test]
    fn parses_zo_quote_as_one_wordlike() {
        let words = segment_words_with_modifiers("zo si", &MorphologyOptions::default(), None)
            .expect("valid morphology");

        assert_eq!(words.len(), 1);
        let WordWithModifiers::BaseWord { word_like } = &words[0] else {
            panic!("expected base word");
        };
        let WordLike::ZoQuote { zo, word } = word_like.as_ref() else {
            panic!("expected ZO quote");
        };
        assert_eq!(zo.phonemes, "zo");
        assert_eq!(word.phonemes, "si");
    }

    #[test]
    fn parses_zoi_quote_as_one_wordlike() {
        let words =
            segment_words_with_modifiers("zoi gy broda gy", &MorphologyOptions::default(), None)
                .expect("valid morphology");

        assert_eq!(words.len(), 1);
        let WordWithModifiers::BaseWord { word_like } = &words[0] else {
            panic!("expected base word");
        };
        let WordLike::ZoiQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        } = word_like.as_ref()
        else {
            panic!("expected ZOI quote");
        };
        assert_eq!(zoi.phonemes, "zoĭ");
        assert_eq!(opening_delimiter.phonemes, "gy");
        assert_eq!(opening_delimiter.span.byte_start, 4);
        assert_eq!(opening_delimiter.span.byte_end, 6);
        assert_eq!(quoted_text.byte_start, 7);
        assert_eq!(quoted_text.byte_end, 12);
        assert_eq!(closing_delimiter.phonemes, "gy");
        assert_eq!(closing_delimiter.span.byte_start, 13);
        assert_eq!(closing_delimiter.span.byte_end, 15);
    }

    #[test]
    fn reports_unclosed_zoi_quote() {
        let error =
            segment_words_with_modifiers("zoi gy broda", &MorphologyOptions::default(), None)
                .expect_err("unclosed ZOI should fail");

        assert!(error.to_string().contains("expected closing ZOI delimiter"));
    }

    fn bare_phonemes(words: &[WordWithModifiers]) -> Vec<&str> {
        words
            .iter()
            .map(|word| bare_word(word).expect("bare word").phonemes.as_str())
            .collect()
    }

    fn bare_span(word: &WordWithModifiers) -> Option<&SourceSpan> {
        bare_word(word).map(|word| &word.span)
    }

    fn bare_word(word: &WordWithModifiers) -> Option<&Word> {
        match word {
            WordWithModifiers::BaseWord { word_like } => match word_like.as_ref() {
                WordLike::Bare { word } => Some(word),
                _ => None,
            },
            _ => None,
        }
    }
}
