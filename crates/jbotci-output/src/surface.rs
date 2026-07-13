use bityzba::{data, invariant, requires};
use jbotci_morphology::{
    GlideMark, LeadingPauseContext, LeadingPauseVowelMode, MorphologyError, MorphologyOptions,
    PhonemeRenderOptions, Phonemes, Word, WordKind, WordLike, WordLikeData,
    segment_words_for_display_with_options_and_source_id, word_needs_leading_pause_in_context,
};
use jbotci_orthography::{LojbanScript, render_latin_word_surface_for_script};

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
pub(crate) fn format_word_with_options(word: &Word, options: PhonemeRenderOptions) -> String {
    format_word_with_options_in_context(word, options, LeadingPauseContext::IndependentWord)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn format_word_with_options_in_context(
    word: &Word,
    options: PhonemeRenderOptions,
    context: LeadingPauseContext,
) -> String {
    render_word(word, options, context)
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
        data!(WordLike::SelmahoQuotedWord { mahoi, word }) => {
            push_word_display_span(spans, mahoi);
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
fn render_word(word: &Word, options: PhonemeRenderOptions, context: LeadingPauseContext) -> String {
    render_visible_word_surface(word, options, context)
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
fn render_visible_word_surface(
    word: &Word,
    options: PhonemeRenderOptions,
    context: LeadingPauseContext,
) -> String {
    let phonemes = word.phonemes();
    let mut rendered =
        render_word_phonemes_without_pause_with_options(word.kind(), &phonemes, options);
    if word_needs_leading_pause_in_context(word, LeadingPauseVowelMode::FoldedVowels, context) {
        rendered.insert(0, '.');
    }
    if word.kind() == WordKind::Cmevla {
        rendered.push('.');
    }
    rendered
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
