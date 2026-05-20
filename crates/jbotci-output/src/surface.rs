use bityzba::{data, invariant, requires};
use jbotci_morphology::{Word, WordKind, WordLike, WordLikeData};
use jbotci_source::SourceSpan;
use jbotci_syntax::WithIndicators;

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum SurfaceChunk {
    Word(String),
    QuotedWords(Vec<Word>),
    QuotedText(String),
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn format_with_indicators(word: &WithIndicators<WordLike>, source: &str) -> String {
    render_surface_chunks(flatten_with_indicators_surface(word, source))
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
) -> Vec<SurfaceChunk> {
    match word {
        WithIndicators::Bare(word_like) => flatten_word_like_surface(word_like, source),
        WithIndicators::Emphasized { bahe, word_like } => {
            let mut chunks = vec![SurfaceChunk::Word(render_word(bahe))];
            chunks.extend(flatten_word_like_surface(word_like, source));
            chunks
        }
        WithIndicators::WithIndicator {
            base,
            indicator,
            nai,
        } => {
            let mut chunks = flatten_with_indicators_surface(base, source);
            chunks.push(SurfaceChunk::Word(render_word(indicator)));
            if let Some(nai) = nai {
                chunks.push(SurfaceChunk::Word(render_word(nai)));
            }
            chunks
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn flatten_word_like_surface(word_like: &WordLike, source: &str) -> Vec<SurfaceChunk> {
    match word_like.as_data() {
        data!(WordLike::Bare(word)) => vec![SurfaceChunk::Word(render_word(word))],
        data!(WordLike::ZoQuote { zo, word }) => vec![
            SurfaceChunk::Word(render_word(zo)),
            SurfaceChunk::QuotedWords(vec![(**word).clone()]),
        ],
        data!(WordLike::ZoiQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => vec![
            SurfaceChunk::Word(render_word(zoi)),
            SurfaceChunk::Word(render_word_without_pause(opening_delimiter)),
            SurfaceChunk::QuotedText(drop_leading_zoi_separator(source_slice(
                source,
                quoted_text,
            ))),
            SurfaceChunk::Word(render_word_without_pause(closing_delimiter)),
        ],
        data!(WordLike::LohuQuote {
            lohu,
            quoted_words,
            lehu,
        }) => vec![
            SurfaceChunk::Word(render_word(lohu)),
            SurfaceChunk::QuotedWords(quoted_words.clone()),
            SurfaceChunk::Word(render_word(lehu)),
        ],
        data!(WordLike::SingleWordQuote {
            marker,
            quoted_text,
        }) => vec![
            SurfaceChunk::Word(render_word(marker)),
            SurfaceChunk::QuotedText(source_slice(source, quoted_text)),
        ],
        data!(WordLike::Letter { base, bu }) => {
            let mut chunks = flatten_word_like_surface(base, source);
            chunks.push(SurfaceChunk::Word(render_word(bu)));
            chunks
        }
        data!(WordLike::ZeiLujvo { left, zei, right }) => {
            let mut chunks = flatten_word_like_surface(left, source);
            chunks.push(SurfaceChunk::Word(render_word(zei)));
            chunks.push(SurfaceChunk::Word(render_word(right)));
            chunks
        }
    }
}

#[requires(span.byte_start <= span.byte_end)]
#[ensures(true)]
fn source_slice(source: &str, span: &SourceSpan) -> String {
    source
        .get(span.byte_start..span.byte_end)
        .unwrap_or_default()
        .to_owned()
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
        SurfaceChunk::QuotedWords(words) => format!(
            "«{}»",
            words.iter().map(render_word).collect::<Vec<_>>().join(" ")
        ),
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
fn render_word(word: &Word) -> String {
    if let Some(surface_override) = &word.surface_override {
        return surface_override.clone();
    }
    render_visible_word_surface(word)
}

#[requires(true)]
#[ensures(true)]
fn render_word_without_pause(word: &Word) -> String {
    if let Some(surface_override) = &word.surface_override {
        return surface_override.clone();
    }
    match word.kind {
        WordKind::Cmavo | WordKind::Cmevla => strip_stress_accents(&add_diacritics(&word.phonemes)),
        WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla => add_diacritics(&word.phonemes),
    }
}

#[requires(true)]
#[ensures(true)]
fn render_visible_word_surface(word: &Word) -> String {
    let mut rendered = match word.kind {
        WordKind::Cmavo => {
            mark_falling_diphthong_glides(&strip_stress_accents(&add_diacritics(&word.phonemes)))
        }
        WordKind::Cmevla => strip_stress_accents(&add_diacritics(&word.phonemes)),
        WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla => add_diacritics(&word.phonemes),
    };
    if needs_leading_pause(word) {
        rendered.insert(0, '.');
    }
    if word.kind == WordKind::Cmevla {
        rendered.push('.');
    }
    rendered
}

#[requires(true)]
#[ensures(true)]
fn needs_leading_pause(word: &Word) -> bool {
    word.kind == WordKind::Cmevla
        || strip_diacritics(&word.phonemes)
            .chars()
            .next()
            .is_some_and(|ch| matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u'))
}

#[requires(true)]
#[ensures(true)]
fn add_diacritics(text: &str) -> String {
    mark_stress(&normalize_uppercase_stress(text))
}

#[requires(true)]
#[ensures(true)]
fn normalize_uppercase_stress(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            'A' | 'Á' | 'À' | 'à' => 'á',
            'E' | 'É' | 'È' | 'è' => 'é',
            'I' | 'Í' | 'Ì' | 'ì' => 'í',
            'O' | 'Ó' | 'Ò' | 'ò' => 'ó',
            'U' | 'Ú' | 'Ù' | 'ù' => 'ú',
            'Y' | 'Ý' | 'Ỳ' | 'ỳ' => 'ý',
            other => other,
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn mark_falling_diphthong_glides(text: &str) -> String {
    let mut rendered = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match (ch, chars.peek().copied()) {
            ('a', Some('i')) | ('e', Some('i')) | ('o', Some('i')) => {
                rendered.push(ch);
                rendered.push('ĭ');
                chars.next();
            }
            ('a', Some('u')) => {
                rendered.push('a');
                rendered.push('ŭ');
                chars.next();
            }
            _ => rendered.push(ch),
        }
    }
    rendered
}

#[requires(true)]
#[ensures(true)]
fn mark_stress(text: &str) -> String {
    if has_explicit_stress(text) {
        return text.to_owned();
    }
    let stressable = text
        .char_indices()
        .filter_map(|(index, ch)| is_full_vowel(ch).then_some(index))
        .collect::<Vec<_>>();
    let Some(&stress_index) = stressable.iter().rev().nth(1) else {
        return text.to_owned();
    };
    let mut rendered = String::with_capacity(text.len() + 1);
    for (index, ch) in text.char_indices() {
        if index == stress_index {
            rendered.push(acute_vowel(ch));
        } else {
            rendered.push(ch);
        }
    }
    rendered
}

#[requires(true)]
#[ensures(true)]
fn has_explicit_stress(text: &str) -> bool {
    text.chars().any(|ch| {
        matches!(
            ch,
            'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý' | 'à' | 'è' | 'ì' | 'ò' | 'ù' | 'ỳ'
        )
    })
}

#[requires(true)]
#[ensures(true)]
fn is_full_vowel(ch: char) -> bool {
    matches!(
        ch,
        'a' | 'e' | 'i' | 'o' | 'u' | 'á' | 'é' | 'í' | 'ó' | 'ú' | 'à' | 'è' | 'ì' | 'ò' | 'ù'
    )
}

#[requires(true)]
#[ensures(true)]
fn acute_vowel(ch: char) -> char {
    match ch {
        'a' | 'á' | 'à' => 'á',
        'e' | 'é' | 'è' => 'é',
        'i' | 'í' | 'ì' => 'í',
        'o' | 'ó' | 'ò' => 'ó',
        'u' | 'ú' | 'ù' => 'ú',
        other => other,
    }
}

#[requires(true)]
#[ensures(true)]
fn strip_stress_accents(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            'á' | 'à' => 'a',
            'é' | 'è' => 'e',
            'í' | 'ì' => 'i',
            'ó' | 'ò' => 'o',
            'ú' | 'ù' => 'u',
            'ý' | 'ỳ' => 'y',
            other => other,
        })
        .collect()
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
