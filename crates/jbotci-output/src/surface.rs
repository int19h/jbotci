use bityzba::{data, invariant, requires};
use jbotci_morphology::{
    GlideMark, MorphologyError, MorphologyOptions, PhonemeRenderOptions, Phonemes, Word, WordKind,
    WordLike, WordLikeData, segment_words_for_display_with_options_and_source_id,
};
use jbotci_orthography::{LojbanScript, render_latin_word_surface_for_script};
use jbotci_syntax::{Token, WithIndicators, WithIndicatorsData};

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Word(..) => true)]
#[invariant(::QuotedWords(..) => true)]
#[invariant(::QuotedText(..) => true)]
enum SurfaceChunk {
    Word(String),
    QuotedWords(Vec<String>),
    QuotedText(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::LojbanWord { .. } => true)]
#[invariant(::VerbatimText { .. } => true)]
enum DisplaySpan {
    LojbanWord {
        kind: WordKind,
        phonemes: Phonemes,
        byte_start: usize,
        byte_end: usize,
    },
    VerbatimText {
        byte_start: usize,
        byte_end: usize,
    },
}

impl DisplaySpan {
    #[requires(true)]
    #[ensures(ret.start <= ret.end)]
    fn byte_range(&self) -> std::ops::Range<usize> {
        match self {
            Self::LojbanWord {
                byte_start,
                byte_end,
                ..
            } => *byte_start..*byte_end,
            Self::VerbatimText {
                byte_start,
                byte_end,
            } => *byte_start..*byte_end,
        }
    }
}

#[requires(true)]
#[ensures(script == LojbanScript::Latin || ret.mark_glides == GlideMark::Breve)]
pub fn phoneme_render_options_for_script(
    script: LojbanScript,
    options: PhonemeRenderOptions,
) -> PhonemeRenderOptions {
    match script {
        LojbanScript::Latin => options,
        LojbanScript::Cyrillic | LojbanScript::Zbalermorna => PhonemeRenderOptions {
            mark_glides: GlideMark::Breve,
            ..options
        },
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rendered| !rendered.is_empty() || text.is_empty()) || ret.is_err())]
pub fn render_lojban_text_for_script(
    text: &str,
    script: LojbanScript,
    options: PhonemeRenderOptions,
) -> Result<String, MorphologyError> {
    render_lojban_text_for_script_with_options(text, script, &MorphologyOptions::default(), options)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|rendered| !rendered.is_empty() || text.is_empty()) || ret.is_err())]
pub fn render_lojban_text_for_script_with_options(
    text: &str,
    script: LojbanScript,
    morphology_options: &MorphologyOptions,
    options: PhonemeRenderOptions,
) -> Result<String, MorphologyError> {
    if script == LojbanScript::Latin {
        return Ok(text.to_owned());
    }
    let words =
        segment_words_for_display_with_options_and_source_id(text, morphology_options, None)?;
    Ok(render_display_words_for_script(
        text,
        script,
        &words,
        phoneme_render_options_for_script(script, options),
    ))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn format_with_indicators_with_options(
    word: &Token,
    source: &str,
    options: PhonemeRenderOptions,
) -> String {
    render_surface_chunks(flatten_with_indicators_surface(
        word.as_indicators(),
        source,
        options,
    ))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn format_word_like_with_options(
    word_like: &WordLike,
    source: &str,
    options: PhonemeRenderOptions,
) -> String {
    render_surface_chunks(flatten_word_like_surface(word_like, source, options))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_compound_with_indicators(word: &Token) -> bool {
    match word.as_indicators().as_data() {
        data!(WithIndicators::Emphasized { .. }) | data!(WithIndicators::WithIndicator { .. }) => {
            true
        }
        data!(WithIndicators::Plain(word_like)) => match word_like.as_data() {
            data!(WordLike::PlainWord(..)) => false,
            data!(WordLike::QuotedWord { .. })
            | data!(WordLike::DelimitedNonLojbanQuote { .. })
            | data!(WordLike::QuotedWords { .. })
            | data!(WordLike::DelimitedWordQuote { .. })
            | data!(WordLike::LerfuWord { .. })
            | data!(WordLike::ZeiCompound { .. }) => true,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn render_display_words_for_script(
    source: &str,
    script: LojbanScript,
    words: &[WordLike],
    options: PhonemeRenderOptions,
) -> String {
    let mut spans = Vec::new();
    collect_display_spans(words, &mut spans);
    spans.sort_by_key(|span| {
        let range = span.byte_range();
        (range.start, range.end)
    });

    let mut output = String::new();
    let mut cursor = 0;
    for span in spans {
        let range = span.byte_range();
        if range.start < cursor || range.end > source.len() {
            continue;
        }
        output.push_str(&render_display_gap_for_script(
            script,
            source.get(cursor..range.start).unwrap_or_default(),
        ));
        match span {
            DisplaySpan::LojbanWord { kind, phonemes, .. } => {
                let latin =
                    render_word_phonemes_without_pause_with_options(kind, &phonemes, options);
                output.push_str(&render_latin_word_surface_for_script(script, kind, &latin));
            }
            DisplaySpan::VerbatimText {
                byte_start,
                byte_end,
            } => output.push_str(source.get(byte_start..byte_end).unwrap_or_default()),
        }
        cursor = range.end;
    }
    output.push_str(&render_display_gap_for_script(
        script,
        source.get(cursor..).unwrap_or_default(),
    ));
    output
}

#[requires(true)]
#[ensures(true)]
fn collect_display_spans(words: &[WordLike], spans: &mut Vec<DisplaySpan>) {
    for word in words {
        collect_word_like_display_spans(word, spans);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_word_like_display_spans(word_like: &WordLike, spans: &mut Vec<DisplaySpan>) {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => push_word_display_span(spans, word),
        data!(WordLike::QuotedWord { zo, word }) => {
            push_word_display_span(spans, zo);
            push_word_display_span(spans, word);
        }
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => {
            push_word_display_span(spans, zoi);
            push_word_display_span(spans, opening_delimiter);
            spans.push(DisplaySpan::VerbatimText {
                byte_start: quoted_text.span.byte_start,
                byte_end: quoted_text.span.byte_end,
            });
            push_word_display_span(spans, closing_delimiter);
        }
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => {
            push_word_display_span(spans, lohu);
            for word in quoted_words {
                push_word_display_span(spans, word);
            }
            push_word_display_span(spans, lehu);
        }
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        }) => {
            push_word_display_span(spans, marker);
            spans.push(DisplaySpan::VerbatimText {
                byte_start: quoted_text.span.byte_start,
                byte_end: quoted_text.span.byte_end,
            });
        }
        data!(WordLike::LerfuWord { base, bu }) => {
            collect_word_like_display_spans(base, spans);
            push_word_display_span(spans, bu);
        }
        data!(WordLike::ZeiCompound { left, zei, right }) => {
            collect_word_like_display_spans(left, spans);
            push_word_display_span(spans, zei);
            push_word_display_span(spans, right);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn push_word_display_span(spans: &mut Vec<DisplaySpan>, word: &Word) {
    spans.push(DisplaySpan::LojbanWord {
        kind: word.kind(),
        phonemes: word.phonemes().clone(),
        byte_start: word.span().byte_start,
        byte_end: word.span().byte_end,
    });
}

#[requires(true)]
#[ensures(true)]
fn render_display_gap_for_script(script: LojbanScript, gap: &str) -> String {
    match script {
        LojbanScript::Latin | LojbanScript::Cyrillic => gap.to_owned(),
        LojbanScript::Zbalermorna => gap
            .chars()
            .map(|ch| if ch == '.' { '\u{ed89}' } else { ch })
            .collect(),
    }
}

#[requires(true)]
#[ensures(true)]
fn flatten_with_indicators_surface(
    word: &WithIndicators<WordLike>,
    source: &str,
    options: PhonemeRenderOptions,
) -> Vec<SurfaceChunk> {
    match word.as_data() {
        data!(WithIndicators::Plain(word_like)) => {
            flatten_word_like_surface(word_like, source, options)
        }
        data!(WithIndicators::Emphasized {
            bahe,
            extra_bahe,
            word_like,
        }) => {
            let mut chunks = vec![SurfaceChunk::Word(render_word(bahe, options))];
            chunks.extend(
                extra_bahe
                    .iter()
                    .map(|bahe| SurfaceChunk::Word(render_word(bahe, options))),
            );
            chunks.extend(flatten_word_like_surface(word_like, source, options));
            chunks
        }
        data!(WithIndicators::WithIndicator {
            base,
            indicator_bahe,
            indicator,
            nai_bahe,
            nai,
        }) => {
            let mut chunks = flatten_with_indicators_surface(base, source, options);
            for bahe in indicator_bahe {
                chunks.push(SurfaceChunk::Word(render_word(bahe, options)));
            }
            chunks.push(SurfaceChunk::Word(render_word(indicator, options)));
            if let Some(nai) = nai {
                for bahe in nai_bahe {
                    chunks.push(SurfaceChunk::Word(render_word(bahe, options)));
                }
                chunks.push(SurfaceChunk::Word(render_word(nai, options)));
            }
            chunks
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn flatten_word_like_surface(
    word_like: &WordLike,
    source: &str,
    options: PhonemeRenderOptions,
) -> Vec<SurfaceChunk> {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => vec![SurfaceChunk::Word(render_word(word, options))],
        data!(WordLike::QuotedWord { zo, word }) => vec![
            SurfaceChunk::Word(render_word(zo, options)),
            SurfaceChunk::QuotedWords(vec![render_word(word, options)]),
        ],
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => vec![
            SurfaceChunk::Word(render_word(zoi, options)),
            SurfaceChunk::Word(render_word_without_pause(opening_delimiter, options)),
            SurfaceChunk::QuotedText(drop_leading_zoi_separator(quoted_text.text.clone())),
            SurfaceChunk::Word(render_word_without_pause(closing_delimiter, options)),
        ],
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => vec![
            SurfaceChunk::Word(render_word(lohu, options)),
            SurfaceChunk::QuotedWords(
                quoted_words
                    .iter()
                    .map(|word| render_word(word, options))
                    .collect(),
            ),
            SurfaceChunk::Word(render_word(lehu, options)),
        ],
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        }) => vec![
            SurfaceChunk::Word(render_word(marker, options)),
            SurfaceChunk::QuotedText(quoted_text.text.clone()),
        ],
        data!(WordLike::LerfuWord { base, bu }) => {
            let mut chunks = flatten_word_like_surface(base, source, options);
            chunks.push(SurfaceChunk::Word(render_word(bu, options)));
            chunks
        }
        data!(WordLike::ZeiCompound { left, zei, right }) => {
            let mut chunks = flatten_word_like_surface(left, source, options);
            chunks.push(SurfaceChunk::Word(render_word(zei, options)));
            chunks.push(SurfaceChunk::Word(render_word(right, options)));
            chunks
        }
    }
}

#[requires(true)]
#[ensures(!ret.starts_with(char::is_whitespace))]
fn drop_leading_zoi_separator(text: String) -> String {
    text.strip_prefix(char::is_whitespace)
        .unwrap_or(&text)
        .to_owned()
}

#[requires(true)]
#[ensures(true)]
fn render_surface_chunks(chunks: Vec<SurfaceChunk>) -> String {
    let rendered = chunks
        .into_iter()
        .map(render_surface_chunk)
        .filter(|chunk| !chunk.is_empty())
        .collect::<Vec<_>>();
    let Some((first, rest)) = rendered.split_first() else {
        return String::new();
    };
    rest.iter().fold(first.clone(), |mut acc, next| {
        if !ends_with_visible_pause_dot(&acc) && !starts_with_visible_pause_dot(next) {
            acc.push('-');
        }
        acc.push_str(next);
        acc
    })
}

#[requires(true)]
#[ensures(true)]
fn render_surface_chunk(chunk: SurfaceChunk) -> String {
    match chunk {
        SurfaceChunk::Word(word) => word,
        SurfaceChunk::QuotedWords(words) => format!("«{}»", words.join(" ")),
        SurfaceChunk::QuotedText(text) => format!("«{text}»"),
    }
}

#[requires(true)]
#[ensures(true)]
fn starts_with_visible_pause_dot(text: &str) -> bool {
    text.chars().next().is_some_and(is_visible_pause_dot)
}

#[requires(true)]
#[ensures(true)]
fn ends_with_visible_pause_dot(text: &str) -> bool {
    text.chars().next_back().is_some_and(is_visible_pause_dot)
}

#[requires(true)]
#[ensures(ret == (ch == '.'))]
fn is_visible_pause_dot(ch: char) -> bool {
    ch == '.'
}

#[requires(true)]
#[ensures(true)]
fn render_word(word: &Word, options: PhonemeRenderOptions) -> String {
    render_visible_word_surface(word, options)
}

#[requires(true)]
#[ensures(true)]
fn render_word_without_pause(word: &Word, options: PhonemeRenderOptions) -> String {
    render_word_phonemes_without_pause_with_options(word.kind(), &word.phonemes(), options)
}

#[requires(!phonemes.as_str().is_empty())]
#[ensures(true)]
pub(crate) fn render_word_phonemes_without_pause(kind: WordKind, phonemes: &Phonemes) -> String {
    render_word_phonemes_without_pause_with_options(kind, phonemes, PhonemeRenderOptions::default())
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_word_phonemes_without_pause_with_options(
    _kind: WordKind,
    phonemes: &Phonemes,
    options: PhonemeRenderOptions,
) -> String {
    phonemes.render(options)
}

#[requires(true)]
#[ensures(true)]
fn render_visible_word_surface(word: &Word, options: PhonemeRenderOptions) -> String {
    let phonemes = word.phonemes();
    let mut rendered =
        render_word_phonemes_without_pause_with_options(word.kind(), &phonemes, options);
    if needs_leading_pause(word) {
        rendered.insert(0, '.');
    }
    if word.kind() == WordKind::Cmevla {
        rendered.push('.');
    }
    rendered
}

#[requires(true)]
#[ensures(true)]
fn needs_leading_pause(word: &Word) -> bool {
    word.kind() == WordKind::Cmevla
        || strip_diacritics(word.phonemes().as_str())
            .chars()
            .next()
            .is_some_and(|ch| matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u'))
}

#[requires(true)]
#[ensures(true)]
fn strip_diacritics(text: &str) -> String {
    text.chars()
        .filter_map(|ch| match ch {
            'á' | 'à' | 'Á' | 'À' => Some('a'),
            'é' | 'è' | 'É' | 'È' => Some('e'),
            'í' | 'ì' | 'ĭ' | 'Ĭ' | 'Í' | 'Ì' => Some('i'),
            'ó' | 'ò' | 'Ó' | 'Ò' => Some('o'),
            'ú' | 'ù' | 'ŭ' | 'Ŭ' | 'Ú' | 'Ù' => Some('u'),
            'ý' | 'ỳ' | 'Ý' | 'Ỳ' => Some('y'),
            '\u{0301}' | '\u{0300}' | '\u{0306}' => None,
            other => Some(other),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::ensures;
    use bityzba::requires;

    use super::*;
    use jbotci_morphology::{GlideMark, StressMark};

    #[requires(true)]
    #[ensures(true)]
    fn display_options() -> PhonemeRenderOptions {
        PhonemeRenderOptions {
            mark_stress: StressMark::None,
            mark_glides: GlideMark::None,
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_cyrillic_glides_from_morphology() {
        let rendered =
            render_lojban_text_for_script("coi", LojbanScript::Cyrillic, display_options())
                .expect("valid Lojban text");

        assert_eq!(rendered, "шой");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_zbalermorna_glides_from_morphology() {
        let rendered =
            render_lojban_text_for_script("coi", LojbanScript::Zbalermorna, display_options())
                .expect("valid Lojban text");

        assert_eq!(rendered, "\u{ed86}\u{eda8}");
        assert_eq!(
            render_lojban_text_for_script("kei", LojbanScript::Zbalermorna, display_options())
                .expect("valid Lojban text"),
            "\u{ed82}\u{eda7}"
        );
        assert_eq!(
            render_lojban_text_for_script("co'i", LojbanScript::Zbalermorna, display_options())
                .expect("valid Lojban text"),
            "\u{ed86}\u{eda3}\u{ed8a}\u{eda2}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn preserves_punctuation_and_protected_quote_payload() {
        let rendered = render_lojban_text_for_script(
            "zoi gy Steve gy .djan.",
            LojbanScript::Cyrillic,
            display_options(),
        )
        .expect("valid Lojban text");

        assert!(rendered.contains("Steve"));
        assert!(!rendered.contains("Стеве"));
        assert!(rendered.ends_with(".джан."));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_zbalermorna_pause_dots_in_lojban_gaps() {
        let rendered =
            render_lojban_text_for_script(".djan.", LojbanScript::Zbalermorna, display_options())
                .expect("valid Lojban text");

        assert_eq!(rendered, "\u{ed89}\u{ed91}\u{ed96}\u{edb0}\u{ed97}\u{ed89}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_display_text_returns_morphology_error() {
        let error =
            render_lojban_text_for_script("hello!", LojbanScript::Cyrillic, display_options())
                .expect_err("invalid Lojban text should not be transliterated loosely");

        assert!(!error.to_string().is_empty());
    }
}
