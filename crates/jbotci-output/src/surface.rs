use bityzba::{data, invariant, requires};
use jbotci_morphology::{PhonemeRenderOptions, Phonemes, Word, WordKind, WordLike, WordLikeData};
use jbotci_syntax::WithIndicators;

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

#[requires(true)]
#[ensures(true)]
pub(crate) fn format_with_indicators_with_options(
    word: &WithIndicators<WordLike>,
    source: &str,
    options: PhonemeRenderOptions,
) -> String {
    render_surface_chunks(flatten_with_indicators_surface(word, source, options))
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
pub(crate) fn is_compound_with_indicators(word: &WithIndicators<WordLike>) -> bool {
    match word {
        WithIndicators::Emphasized { .. } | WithIndicators::WithIndicator { .. } => true,
        WithIndicators::Bare(word_like) => match word_like.as_data() {
            data!(WordLike::Bare(..)) => false,
            data!(WordLike::ZoQuote { .. })
            | data!(WordLike::ZoiQuote { .. })
            | data!(WordLike::LohuQuote { .. })
            | data!(WordLike::SingleWordQuote { .. })
            | data!(WordLike::Letter { .. })
            | data!(WordLike::ZeiLujvo { .. }) => true,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn flatten_with_indicators_surface(
    word: &WithIndicators<WordLike>,
    source: &str,
    options: PhonemeRenderOptions,
) -> Vec<SurfaceChunk> {
    match word {
        WithIndicators::Bare(word_like) => flatten_word_like_surface(word_like, source, options),
        WithIndicators::Emphasized { bahe, word_like } => {
            let mut chunks = vec![SurfaceChunk::Word(render_word(bahe, options))];
            chunks.extend(flatten_word_like_surface(word_like, source, options));
            chunks
        }
        WithIndicators::WithIndicator {
            base,
            indicator,
            nai,
        } => {
            let mut chunks = flatten_with_indicators_surface(base, source, options);
            chunks.push(SurfaceChunk::Word(render_word(indicator, options)));
            if let Some(nai) = nai {
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
        data!(WordLike::Bare(word)) => vec![SurfaceChunk::Word(render_word(word, options))],
        data!(WordLike::ZoQuote { zo, word }) => vec![
            SurfaceChunk::Word(render_word(zo, options)),
            SurfaceChunk::QuotedWords(vec![render_word(word, options)]),
        ],
        data!(WordLike::ZoiQuote {
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
        data!(WordLike::LohuQuote {
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
        data!(WordLike::SingleWordQuote {
            marker,
            quoted_text,
        }) => vec![
            SurfaceChunk::Word(render_word(marker, options)),
            SurfaceChunk::QuotedText(quoted_text.text.clone()),
        ],
        data!(WordLike::Letter { base, bu }) => {
            let mut chunks = flatten_word_like_surface(base, source, options);
            chunks.push(SurfaceChunk::Word(render_word(bu, options)));
            chunks
        }
        data!(WordLike::ZeiLujvo { left, zei, right }) => {
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
