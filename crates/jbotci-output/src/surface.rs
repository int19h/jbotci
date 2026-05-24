use bityzba::{data, invariant, requires};
use jbotci_morphology::{
    PhonemeRenderOptions, Phonemes, Word, WordKind, WordLike, WordLikeData, pronunciation_syllables,
};
use jbotci_syntax::WithIndicators;

use crate::OutputError;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Word(_) => true)]
#[invariant(::Text(_) => true)]
enum IpaSurfaceChunk<'word> {
    Word(&'word Word),
    Text(&'word str),
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
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(crate) fn format_words_ipa(words: &[WordLike], source: &str) -> Result<String, OutputError> {
    let chunks = words
        .iter()
        .flat_map(flatten_word_like_ipa)
        .collect::<Vec<_>>();
    render_ipa_surface_chunks(&chunks, source)
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
#[ensures(true)]
fn flatten_word_like_ipa(word_like: &WordLike) -> Vec<IpaSurfaceChunk<'_>> {
    match word_like.as_data() {
        data!(WordLike::Bare(word)) => vec![IpaSurfaceChunk::Word(word)],
        data!(WordLike::ZoQuote { zo, word }) => {
            vec![IpaSurfaceChunk::Word(zo), IpaSurfaceChunk::Word(word)]
        }
        data!(WordLike::ZoiQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => vec![
            IpaSurfaceChunk::Word(zoi),
            IpaSurfaceChunk::Word(opening_delimiter),
            IpaSurfaceChunk::Text(drop_leading_zoi_separator_ref(&quoted_text.text)),
            IpaSurfaceChunk::Word(closing_delimiter),
        ],
        data!(WordLike::LohuQuote {
            lohu,
            quoted_words,
            lehu,
        }) => {
            let mut chunks = vec![IpaSurfaceChunk::Word(lohu)];
            chunks.extend(quoted_words.iter().map(IpaSurfaceChunk::Word));
            chunks.push(IpaSurfaceChunk::Word(lehu));
            chunks
        }
        data!(WordLike::SingleWordQuote {
            marker,
            quoted_text,
        }) => vec![
            IpaSurfaceChunk::Word(marker),
            IpaSurfaceChunk::Text(&quoted_text.text),
        ],
        data!(WordLike::Letter { base, bu }) => {
            let mut chunks = flatten_word_like_ipa(base);
            chunks.push(IpaSurfaceChunk::Word(bu));
            chunks
        }
        data!(WordLike::ZeiLujvo { left, zei, right }) => {
            let mut chunks = flatten_word_like_ipa(left);
            chunks.push(IpaSurfaceChunk::Word(zei));
            chunks.push(IpaSurfaceChunk::Word(right));
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
#[ensures(!ret.starts_with(char::is_whitespace))]
fn drop_leading_zoi_separator_ref(text: &str) -> &str {
    text.strip_prefix(char::is_whitespace).unwrap_or(text)
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
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn render_ipa_surface_chunks(
    chunks: &[IpaSurfaceChunk<'_>],
    source: &str,
) -> Result<String, OutputError> {
    let mut rendered = Vec::new();
    for (index, chunk) in chunks.iter().enumerate() {
        match chunk {
            IpaSurfaceChunk::Word(word) => rendered.push(render_word_ipa(
                word,
                source,
                previous_ipa_word(chunks, index),
                chunks.get(index + 1).is_some(),
            )?),
            IpaSurfaceChunk::Text(text) => {
                if !text.is_empty() {
                    rendered.push((*text).to_owned());
                }
            }
        }
    }
    Ok(rendered.join(" "))
}

#[requires(index <= chunks.len())]
#[ensures(true)]
fn previous_ipa_word<'word>(
    chunks: &'word [IpaSurfaceChunk<'word>],
    index: usize,
) -> Option<&'word Word> {
    if index == 0 {
        return None;
    }
    match chunks.get(index - 1) {
        Some(IpaSurfaceChunk::Word(word)) => Some(*word),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn render_word_ipa(
    word: &Word,
    source: &str,
    previous_word: Option<&Word>,
    has_next_chunk: bool,
) -> Result<String, OutputError> {
    let phonemes = word.phonemes();
    let syllables = pronunciation_syllables(&phonemes).map_err(OutputError::Ipa)?;
    let stress_index = explicit_stress_syllable_index(&syllables)
        .or_else(|| conventional_stress_syllable_index(&syllables));
    let leading_pauses = explicit_leading_pause_count(source, word)
        .max(required_leading_pause_count(word))
        .max(previous_boundary_pause_count(source, previous_word));
    let trailing_pauses = if has_next_chunk {
        0
    } else {
        explicit_trailing_pause_count(source, word)
    };

    let mut rendered = String::new();
    for (index, syllable) in syllables.iter().enumerate() {
        if index > 0 {
            rendered.push('.');
        }
        if stress_index == Some(index) {
            rendered.push('ˈ');
        }
        if index == 0 {
            push_ipa_pauses(&mut rendered, leading_pauses);
        }
        rendered.push_str(&render_ipa_syllable(syllable));
    }
    push_ipa_pauses(&mut rendered, trailing_pauses);
    Ok(rendered)
}

#[requires(true)]
#[ensures(true)]
fn explicit_stress_syllable_index(syllables: &[String]) -> Option<usize> {
    syllables
        .iter()
        .position(|syllable| syllable.chars().any(is_explicit_stress_char))
}

#[requires(true)]
#[ensures(true)]
fn conventional_stress_syllable_index(syllables: &[String]) -> Option<usize> {
    let stressable = syllables
        .iter()
        .enumerate()
        .filter_map(|(index, syllable)| syllable_has_full_vowel(syllable).then_some(index))
        .collect::<Vec<_>>();
    stressable.iter().rev().nth(1).copied()
}

#[requires(true)]
#[ensures(true)]
fn syllable_has_full_vowel(syllable: &str) -> bool {
    syllable
        .chars()
        .any(|value| matches!(strip_vowel_diacritic(value), 'a' | 'e' | 'i' | 'o' | 'u'))
}

#[requires(true)]
#[ensures(true)]
fn is_explicit_stress_char(value: char) -> bool {
    matches!(
        value,
        'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý' | 'à' | 'è' | 'ì' | 'ò' | 'ù' | 'ỳ'
    )
}

#[requires(true)]
#[ensures(true)]
fn render_ipa_syllable(syllable: &str) -> String {
    let mut rendered = String::new();
    for value in syllable.chars() {
        push_ipa_phoneme(&mut rendered, value);
    }
    rendered
}

#[requires(true)]
#[ensures(true)]
fn push_ipa_phoneme(output: &mut String, value: char) {
    match value {
        'a' | 'á' | 'à' => output.push('a'),
        'e' | 'é' | 'è' => output.push('e'),
        'i' | 'í' | 'ì' => output.push('i'),
        'o' | 'ó' | 'ò' => output.push('o'),
        'u' | 'ú' | 'ù' => output.push('u'),
        'y' | 'ý' | 'ỳ' => output.push('ə'),
        'ĭ' => output.push('j'),
        'ŭ' => output.push('w'),
        '\'' => output.push('h'),
        '.' => output.push('ʔ'),
        'c' => output.push('ʃ'),
        'j' => output.push('ʒ'),
        other => output.push(other),
    }
}

#[requires(true)]
#[ensures(true)]
fn strip_vowel_diacritic(value: char) -> char {
    match value {
        'á' | 'à' => 'a',
        'é' | 'è' => 'e',
        'í' | 'ì' | 'ĭ' => 'i',
        'ó' | 'ò' => 'o',
        'ú' | 'ù' | 'ŭ' => 'u',
        'ý' | 'ỳ' => 'y',
        other => other,
    }
}

#[requires(true)]
#[ensures(true)]
fn explicit_leading_pause_count(source: &str, word: &Word) -> usize {
    source
        .as_bytes()
        .get(..word.span().byte_start.min(source.len()))
        .unwrap_or_default()
        .iter()
        .rev()
        .take_while(|value| **value == b'.')
        .count()
}

#[requires(true)]
#[ensures(true)]
fn explicit_trailing_pause_count(source: &str, word: &Word) -> usize {
    source
        .as_bytes()
        .get(word.span().byte_end.min(source.len())..)
        .unwrap_or_default()
        .iter()
        .take_while(|value| **value == b'.')
        .count()
}

#[requires(true)]
#[ensures(ret <= 1)]
fn required_leading_pause_count(word: &Word) -> usize {
    usize::from(word.kind() == WordKind::Cmevla || starts_with_vowel_sound(word))
}

#[requires(true)]
#[ensures(ret <= 1)]
fn previous_boundary_pause_count(source: &str, previous_word: Option<&Word>) -> usize {
    usize::from(previous_word.is_some_and(|word| {
        word.kind() == WordKind::Cmevla || explicit_trailing_pause_count(source, word) > 0
    }))
}

#[requires(true)]
#[ensures(true)]
fn starts_with_vowel_sound(word: &Word) -> bool {
    word.phonemes()
        .as_str()
        .chars()
        .next()
        .map(strip_vowel_diacritic)
        .is_some_and(|value| matches!(value, 'a' | 'e' | 'i' | 'o' | 'u'))
}

#[requires(true)]
#[ensures(true)]
fn push_ipa_pauses(output: &mut String, count: usize) {
    for _ in 0..count {
        output.push('ʔ');
    }
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
