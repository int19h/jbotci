//! Lojban morphology model.

mod grammar;
mod segment;
mod syntax_eq;

use std::fmt;

use bityzba::{data, ensures, invariant, new, requires};
pub use jbotci_dialect::{
    CmavoDialectEntry, CmavoDialectTransform, DialectDefinition, DialectFeature,
};
use jbotci_source::{SourceId, SourceLocationError, SourceSpan};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use syntax_eq::{strip_diacritics, word_like_syntax_eq, word_syntax_eq};

#[invariant(self.cmavo_dialect_entries.iter().all(CmavoDialectEntry::is_valid), "cmavo dialect entries must be normalized and internally valid")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MorphologyOptions {
    pub accept_latin: bool,
    pub accept_cyrillic: bool,
    pub accept_zbalermorna: bool,
    pub cmavo_dialect_entries: Vec<CmavoDialectEntry>,
    pub cmevla_as_relation_words: bool,
    pub uppercase_marks_stress: bool,
    pub enforce_cgv_ban: bool,
}

impl Default for MorphologyOptions {
    #[requires(true)]
    #[ensures(true)]
    fn default() -> Self {
        new!(MorphologyOptions {
            accept_latin: true,
            accept_cyrillic: true,
            accept_zbalermorna: true,
            cmavo_dialect_entries: Vec::new(),
            cmevla_as_relation_words: false,
            uppercase_marks_stress: true,
            enforce_cgv_ban: true,
        })
    }
}

impl MorphologyOptions {
    #[ensures(ret.cmavo_dialect_entries == definition.cmavo_entries)]
    #[ensures(definition.features.contains(&DialectFeature::Cbm) -> ret.cmevla_as_relation_words)]
    #[ensures(definition.features.contains(&DialectFeature::AllowCgv) -> !ret.enforce_cgv_ban)]
    #[ensures(definition.features.contains(&DialectFeature::CaseInsensitive) -> !ret.uppercase_marks_stress)]
    #[requires(true)]
    pub fn with_dialect_definition(self, definition: &DialectDefinition) -> Self {
        let cmevla_as_relation_words = self.cmevla_as_relation_words;
        let enforce_cgv_ban = self.enforce_cgv_ban;
        let uppercase_marks_stress = self.uppercase_marks_stress;
        self.with_data(data! {
            cmavo_dialect_entries: definition.cmavo_entries.clone(),
            cmevla_as_relation_words: cmevla_as_relation_words
                || definition.features.contains(&DialectFeature::Cbm),
            enforce_cgv_ban: enforce_cgv_ban
                && !definition.features.contains(&DialectFeature::AllowCgv),
            uppercase_marks_stress: uppercase_marks_stress
                && !definition.features.contains(&DialectFeature::CaseInsensitive),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub enum WordKind {
    #[serde(rename = "cmavo")]
    Cmavo,
    #[serde(rename = "gismu")]
    Gismu,
    #[serde(rename = "lujvo")]
    Lujvo,
    #[serde(rename = "fu'ivla")]
    Fuhivla,
    #[serde(rename = "cmevla")]
    Cmevla,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[invariant(true)]
pub enum LujvoSegment {
    Rafsi { text: String },
    Hyphen { text: String },
}

impl LujvoSegment {
    #[ensures(!ret.is_empty())]
    #[requires(true)]
    pub fn text(&self) -> &str {
        match self {
            Self::Rafsi { text } | Self::Hyphen { text } => text,
        }
    }
}

impl fmt::Display for WordKind {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Cmavo => "cmavo",
            Self::Gismu => "gismu",
            Self::Lujvo => "lujvo",
            Self::Fuhivla => "fu'ivla",
            Self::Cmevla => "cmevla",
        };
        f.write_str(text)
    }
}

#[invariant(!self.phonemes.is_empty(), "word phoneme text must not be empty")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Word {
    pub kind: WordKind,
    pub phonemes: String,
    pub span: SourceSpan,
    pub surface_override: Option<String>,
    pub dialect_transform: Option<CmavoDialectTransform>,
}

impl fmt::Display for Word {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.phonemes)
    }
}

impl Word {
    #[requires(true)]
    #[ensures(!ret.is_empty() || self.phonemes.is_empty())]
    pub fn canonical_phonemes(&self) -> String {
        canonicalize_text(&self.phonemes)
    }

    #[requires(true)]
    #[ensures(ret == (self.kind == WordKind::Cmavo))]
    pub fn is_cmavo(&self) -> bool {
        self.kind == WordKind::Cmavo
    }

    #[requires(true)]
    #[ensures(ret == matches!(self.kind, WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla))]
    pub fn is_brivla(&self) -> bool {
        matches!(
            self.kind,
            WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla
        )
    }

    #[requires(true)]
    #[ensures(ret == (self.kind == WordKind::Cmevla))]
    pub fn is_cmevla(&self) -> bool {
        self.kind == WordKind::Cmevla
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    pub fn is_cmavo_text(&self, text: &str) -> bool {
        self.is_cmavo() && canonical_text_eq(&self.phonemes, text)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn selmaho(&self) -> Option<&'static str> {
        if self.is_cmavo() {
            selmaho(&self.phonemes)
        } else {
            None
        }
    }
}

#[invariant(word_like_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum WordLike {
    Bare(Box<Word>),
    ZoQuote {
        zo: Box<Word>,
        word: Box<Word>,
    },
    ZoiQuote {
        zoi: Box<Word>,
        opening_delimiter: Box<Word>,
        quoted_text: SourceSpan,
        closing_delimiter: Box<Word>,
    },
    LohuQuote {
        lohu: Box<Word>,
        quoted_words: Vec<Word>,
        lehu: Box<Word>,
    },
    SingleWordQuote {
        marker: Box<Word>,
        quoted_text: SourceSpan,
    },
    Letter {
        base: Box<WordLike>,
        bu: Box<Word>,
    },
    ZeiLujvo {
        left: Box<WordLike>,
        zei: Box<Word>,
        right: Box<Word>,
    },
}

impl WordLike {
    #[requires(true)]
    #[ensures(true)]
    pub fn bare(word: Word) -> Self {
        new!(WordLike::Bare(Box::new(word)))
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn zo_quote(zo: Word, word: Word) -> Self {
        new!(WordLike::ZoQuote {
            zo: Box::new(zo),
            word: Box::new(word),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn zoi_quote(
        zoi: Word,
        opening_delimiter: Word,
        quoted_text: SourceSpan,
        closing_delimiter: Word,
    ) -> Self {
        new!(WordLike::ZoiQuote {
            zoi: Box::new(zoi),
            opening_delimiter: Box::new(opening_delimiter),
            quoted_text: quoted_text,
            closing_delimiter: Box::new(closing_delimiter),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn lohu_quote(lohu: Word, quoted_words: Vec<Word>, lehu: Word) -> Self {
        new!(WordLike::LohuQuote {
            lohu: Box::new(lohu),
            quoted_words: quoted_words,
            lehu: Box::new(lehu),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn single_word_quote(marker: Word, quoted_text: SourceSpan) -> Self {
        new!(WordLike::SingleWordQuote {
            marker: Box::new(marker),
            quoted_text: quoted_text,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn letter(base: WordLike, bu: Word) -> Self {
        new!(WordLike::Letter {
            base: Box::new(base),
            bu: Box::new(bu),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn zei_lujvo(left: WordLike, zei: Word, right: Word) -> Self {
        new!(WordLike::ZeiLujvo {
            left: Box::new(left),
            zei: Box::new(zei),
            right: Box::new(right),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visible_base_word(&self) -> Option<&Word> {
        match self.as_data() {
            data!(WordLike::Bare(word)) => Some(word),
            data!(WordLike::ZoQuote { zo, .. }) => Some(zo),
            data!(WordLike::ZoiQuote { zoi, .. }) => Some(zoi),
            data!(WordLike::LohuQuote { lohu, .. }) => Some(lohu),
            data!(WordLike::SingleWordQuote { marker, .. }) => Some(marker),
            data!(WordLike::Letter { base, .. }) => base.visible_base_word(),
            data!(WordLike::ZeiLujvo { left, .. }) => left.visible_base_word(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visible_selmaho(&self) -> Option<&'static str> {
        visible_selmaho(self)
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    pub fn visible_cmavo_is(&self, text: &str) -> bool {
        match self.as_data() {
            data!(WordLike::Bare(word)) => word.is_cmavo_text(text),
            _ => false,
        }
    }

    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), data!(WordLike::Bare(word)) if word.is_brivla()))]
    pub fn is_brivla(&self) -> bool {
        matches!(self.as_data(), data!(WordLike::Bare(word)) if word.is_brivla())
    }

    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), data!(WordLike::Bare(word)) if word.is_cmevla()))]
    pub fn is_cmevla(&self) -> bool {
        matches!(self.as_data(), data!(WordLike::Bare(word)) if word.is_cmevla())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
    pub fn byte_range(&self) -> Option<std::ops::Range<usize>> {
        word_like_byte_range(self)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn source_spans(&self) -> Vec<&SourceSpan> {
        let mut spans = Vec::new();
        self.source_spans_into(&mut spans);
        spans
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn source_spans_into<'a>(&'a self, out: &mut Vec<&'a SourceSpan>) {
        match self.as_data() {
            data!(WordLike::Bare(word)) => out.push(&word.span),
            data!(WordLike::ZoQuote { zo, word }) => {
                out.push(&zo.span);
                out.push(&word.span);
            }
            data!(WordLike::ZoiQuote {
                zoi,
                opening_delimiter,
                quoted_text,
                closing_delimiter,
            }) => {
                out.push(&zoi.span);
                out.push(&opening_delimiter.span);
                out.push(quoted_text);
                out.push(&closing_delimiter.span);
            }
            data!(WordLike::LohuQuote {
                lohu,
                quoted_words,
                lehu,
            }) => {
                out.push(&lohu.span);
                for word in quoted_words {
                    out.push(&word.span);
                }
                out.push(&lehu.span);
            }
            data!(WordLike::SingleWordQuote {
                marker,
                quoted_text,
            }) => {
                out.push(&marker.span);
                out.push(quoted_text);
            }
            data!(WordLike::Letter { base, bu }) => {
                base.source_spans_into(out);
                out.push(&bu.span);
            }
            data!(WordLike::ZeiLujvo { left, zei, right }) => {
                left.source_spans_into(out);
                out.push(&zei.span);
                out.push(&right.span);
            }
        }
    }
}

impl<'de> Deserialize<'de> for WordLike {
    #[requires(true)]
    #[ensures(true)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        word_like_from_json(serde_json::Value::deserialize(deserializer)?)
            .map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for WordLike {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(WordLike::Bare(word)) => write!(f, "{word}"),
            data!(WordLike::ZoQuote { zo, word }) => write!(f, "{zo}-<<{word}>>"),
            data!(WordLike::ZoiQuote {
                zoi,
                opening_delimiter,
                quoted_text,
                closing_delimiter,
            }) => write!(
                f,
                "{zoi}-{opening_delimiter}-<{} chars>-{closing_delimiter}",
                quoted_text.char_len()
            ),
            data!(WordLike::LohuQuote {
                lohu,
                quoted_words,
                lehu,
            }) => {
                write!(f, "{lohu}-<<")?;
                for (index, word) in quoted_words.iter().enumerate() {
                    if index > 0 {
                        f.write_str(" ")?;
                    }
                    write!(f, "{word}")?;
                }
                write!(f, ">>-{lehu}")
            }
            data!(WordLike::SingleWordQuote {
                marker,
                quoted_text,
            }) => write!(f, "{marker}-<{} chars>", quoted_text.char_len()),
            data!(WordLike::Letter { base, bu }) => write!(f, "{base}-{bu}"),
            data!(WordLike::ZeiLujvo { left, zei, right }) => {
                write!(f, "{left}-{zei}-{right}")
            }
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
pub fn map_word_like_spans<F>(word_like: WordLike, map_span: &F) -> Result<WordLike, String>
where
    F: Fn(SourceSpan) -> Result<SourceSpan, String>,
{
    Ok(match word_like.into_data() {
        data!(WordLike::Bare(word)) => WordLike::bare(map_word_spans(*word, map_span)?),
        data!(WordLike::ZoQuote { zo, word }) => WordLike::zo_quote(
            map_word_spans(*zo, map_span)?,
            map_word_spans(*word, map_span)?,
        ),
        data!(WordLike::ZoiQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => WordLike::zoi_quote(
            map_word_spans(*zoi, map_span)?,
            map_word_spans(*opening_delimiter, map_span)?,
            map_span(quoted_text)?,
            map_word_spans(*closing_delimiter, map_span)?,
        ),
        data!(WordLike::LohuQuote {
            lohu,
            quoted_words,
            lehu,
        }) => WordLike::lohu_quote(
            map_word_spans(*lohu, map_span)?,
            quoted_words
                .into_iter()
                .map(|word| map_word_spans(word, map_span))
                .collect::<Result<Vec<_>, _>>()?,
            map_word_spans(*lehu, map_span)?,
        ),
        data!(WordLike::SingleWordQuote {
            marker,
            quoted_text,
        }) => {
            WordLike::single_word_quote(map_word_spans(*marker, map_span)?, map_span(quoted_text)?)
        }
        data!(WordLike::Letter { base, bu }) => WordLike::letter(
            map_word_like_spans(*base, map_span)?,
            map_word_spans(*bu, map_span)?,
        ),
        data!(WordLike::ZeiLujvo { left, zei, right }) => WordLike::zei_lujvo(
            map_word_like_spans(*left, map_span)?,
            map_word_spans(*zei, map_span)?,
            map_word_spans(*right, map_span)?,
        ),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
pub fn map_word_spans<F>(word: Word, map_span: &F) -> Result<Word, String>
where
    F: Fn(SourceSpan) -> Result<SourceSpan, String>,
{
    let data = word.into_data();
    Ok(Word::from_data(data!(Word {
        span: map_span(data.span)?,
        ..data
    })))
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
pub enum MorphologyError {
    #[error("unsupported morphology at character {char_offset}: `{word}` ({reason})")]
    Unsupported {
        char_offset: usize,
        word: String,
        reason: String,
    },
    #[error("invalid morphology at character {char_offset}: `{word}` ({reason})")]
    Invalid {
        char_offset: usize,
        word: String,
        reason: String,
    },
    #[error("unterminated ZOI quote, expected closing delimiter `{delimiter}`")]
    UnterminatedZoiQuote {
        char_offset: usize,
        delimiter: String,
    },
    #[error("invalid source span: {0}")]
    SourceSpan(#[from] SourceLocationError),
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers(input: &str) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        None,
    )
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_with_options(
    input: &str,
    options: &MorphologyOptions,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id(input, options, None)
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_with_source_id(
    input: &str,
    source_id: SourceId,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        Some(source_id),
    )
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_with_options_and_source_id(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordLike>, MorphologyError> {
    grammar::segment_words_with_modifiers(input, options, source_id)
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_raw(input: &str) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_raw_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        None,
    )
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_raw_with_source_id(
    input: &str,
    source_id: SourceId,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_raw_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        Some(source_id),
    )
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_raw_with_options(
    input: &str,
    options: &MorphologyOptions,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_raw_with_options_and_source_id(input, options, None)
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_raw_with_options_and_source_id(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordLike>, MorphologyError> {
    grammar::segment_words_with_modifiers_raw(input, options, source_id)
}

#[cfg_attr(not(test), allow(dead_code))]
#[requires(true)]
#[ensures(true)]
fn word_like_data_is_valid(word_like: &WordLikeData) -> bool {
    match word_like {
        data!(WordLike::Bare(..)) => true,
        data!(WordLike::ZoQuote { zo, .. }) => zo.selmaho() == Some("ZO"),
        data!(WordLike::ZoiQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => {
            zoi.selmaho() == Some("ZOI")
                && opening_delimiter.span.byte_end <= quoted_text.byte_start
                && quoted_text.byte_end <= closing_delimiter.span.byte_start
        }
        data!(WordLike::LohuQuote { lohu, lehu, .. }) => {
            lohu.is_cmavo_text("lo'u") && lehu.is_cmavo_text("le'u")
        }
        data!(WordLike::SingleWordQuote { marker, .. }) => is_single_word_quote_marker(marker),
        data!(WordLike::Letter { base, bu }) => {
            word_like_data_is_valid(base.as_data()) && bu.is_cmavo_text("bu")
        }
        data!(WordLike::ZeiLujvo { left, zei, .. }) => {
            word_like_data_is_valid(left.as_data()) && zei.is_cmavo_text("zei")
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn is_single_word_quote_marker(word: &Word) -> bool {
    canonical_text_eq(&word.phonemes, "zo'oi")
        || canonical_text_eq(&word.phonemes, "la'oi")
        || canonical_text_eq(&word.phonemes, "ra'oi")
        || canonical_text_eq(&word.phonemes, "me'oi")
        || canonical_text_eq(&word.phonemes, "go'oi")
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn word_like_from_json(value: serde_json::Value) -> Result<WordLike, String> {
    let mut object = json_object(value)?;
    if let Some(kind) = object.remove("kind") {
        let kind = json_string(kind)?;
        return match kind.as_str() {
            "bare" => Ok(WordLike::bare(word_field(&mut object, "word")?)),
            "zo-quote" => Ok(WordLike::zo_quote(
                word_field(&mut object, "zo")?,
                word_field(&mut object, "word")?,
            )),
            "zoi-quote" => Ok(WordLike::zoi_quote(
                word_field(&mut object, "zoi")?,
                word_field(&mut object, "opening_delimiter")?,
                source_span_field(&mut object, "quoted_text")?,
                word_field(&mut object, "closing_delimiter")?,
            )),
            "lohu-quote" => Ok(WordLike::lohu_quote(
                word_field(&mut object, "lohu")?,
                words_field(&mut object, "quoted_words")?,
                word_field(&mut object, "lehu")?,
            )),
            "single-word-quote" => Ok(WordLike::single_word_quote(
                word_field(&mut object, "marker")?,
                source_span_field(&mut object, "quoted_text")?,
            )),
            "letter" => Ok(WordLike::letter(
                word_like_field(&mut object, "base")?,
                word_field(&mut object, "bu")?,
            )),
            "zei-lujvo" => Ok(WordLike::zei_lujvo(
                word_like_field(&mut object, "left")?,
                word_field(&mut object, "zei")?,
                word_field(&mut object, "right")?,
            )),
            other => Err(format!("unknown word-like kind `{other}`")),
        };
    }
    let (constructor, payload) = single_constructor(object)?;
    let mut payload = json_object(payload)?;
    match constructor.as_str() {
        "BaseWord" => word_like_field(&mut payload, "word_like"),
        "Emphasized" => word_like_field(&mut payload, "word_like"),
        "WithIndicator" => word_like_field(&mut payload, "base"),
        "StandaloneIndicator" => Ok(WordLike::bare(word_field(&mut payload, "indicator")?)),
        "NotEof" => Ok(WordLike::bare(new!(Word {
            kind: WordKind::Cmavo,
            phonemes: String::from("fa'o"),
            span: SourceSpan::new(None, 0, 0, 0, 0).expect("valid empty span"),
            surface_override: None,
            dialect_transform: None,
        }))),
        "Bare" => Ok(WordLike::bare(word_payload(payload)?)),
        "ZoQuote" => Ok(WordLike::zo_quote(
            word_field(&mut payload, "zo")?,
            word_field(&mut payload, "word")?,
        )),
        "ZoiQuote" => Ok(WordLike::zoi_quote(
            word_field(&mut payload, "zoi")?,
            word_field(&mut payload, "opening_delimiter")?,
            source_span_field(&mut payload, "quoted_text")?,
            word_field(&mut payload, "closing_delimiter")?,
        )),
        "LohuQuote" => Ok(WordLike::lohu_quote(
            word_field(&mut payload, "lohu")?,
            words_field(&mut payload, "quoted_words")?,
            word_field(&mut payload, "lehu")?,
        )),
        "SingleWordQuote" => Ok(WordLike::single_word_quote(
            word_field(&mut payload, "marker")?,
            source_span_field(&mut payload, "quoted_text")?,
        )),
        "Letter" => Ok(WordLike::letter(
            word_like_field(&mut payload, "base")?,
            word_field(&mut payload, "bu")?,
        )),
        "ZeiLujvo" => Ok(WordLike::zei_lujvo(
            word_like_field(&mut payload, "left")?,
            word_field(&mut payload, "zei")?,
            word_field(&mut payload, "right")?,
        )),
        other => Err(format!("unknown word-like constructor `{other}`")),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn word_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<Word, String> {
    serde_json::from_value(required_field(object, name)?)
        .map_err(|error| format!("invalid word field `{name}`: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn word_payload(mut object: serde_json::Map<String, serde_json::Value>) -> Result<Word, String> {
    if object.contains_key("word") {
        return word_field(&mut object, "word");
    }
    serde_json::from_value(serde_json::Value::Object(object))
        .map_err(|error| format!("invalid word payload: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn words_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<Vec<Word>, String> {
    let Some(value) = object.remove(name) else {
        return Ok(Vec::new());
    };
    serde_json::from_value(value)
        .map_err(|error| format!("invalid word list field `{name}`: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn source_span_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<SourceSpan, String> {
    serde_json::from_value(required_field(object, name)?)
        .map_err(|error| format!("invalid source span field `{name}`: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn word_like_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<WordLike, String> {
    word_like_from_json(required_field(object, name)?)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn required_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<serde_json::Value, String> {
    object
        .remove(name)
        .ok_or_else(|| format!("missing field `{name}`"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn json_object(
    value: serde_json::Value,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    match value {
        serde_json::Value::Object(object) => Ok(object),
        other => Err(format!("expected object, got {other}")),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn json_string(value: serde_json::Value) -> Result<String, String> {
    match value {
        serde_json::Value::String(text) => Ok(text),
        other => Err(format!("expected string, got {other}")),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn single_constructor(
    object: serde_json::Map<String, serde_json::Value>,
) -> Result<(String, serde_json::Value), String> {
    if object.len() != 1 {
        return Err(format!(
            "expected single constructor key, got {}",
            object.len()
        ));
    }
    Ok(object.into_iter().next().expect("object has one item"))
}

#[ensures(!ret.is_empty() || text.is_empty())]
#[requires(true)]
pub fn canonicalize_text(text: &str) -> String {
    text.chars()
        .filter(|value| *value != ',')
        .flat_map(strip_diacritic)
        .flat_map(char::to_lowercase)
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub fn canonical_text_eq(left: &str, right: &str) -> bool {
    left.chars()
        .filter(|value| *value != ',')
        .flat_map(strip_diacritic)
        .flat_map(char::to_lowercase)
        .eq(right
            .chars()
            .filter(|value| *value != ',')
            .flat_map(strip_diacritic)
            .flat_map(char::to_lowercase))
}

#[requires(true)]
#[ensures(ret -> !text.is_empty())]
pub fn canonical_text_is_all(text: &str, expected: char) -> bool {
    let mut saw_char = false;
    for value in text
        .chars()
        .filter(|value| *value != ',')
        .flat_map(strip_diacritic)
        .flat_map(char::to_lowercase)
    {
        if value != expected {
            return false;
        }
        saw_char = true;
    }
    saw_char
}

#[requires(true)]
#[ensures(true)]
pub fn strip_diacritics_eq(left: &str, right: &str) -> bool {
    left.chars()
        .filter_map(strip_diacritic)
        .eq(right.chars().filter_map(strip_diacritic))
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

#[requires(true)]
#[ensures(true)]
pub fn selmaho(cmavo: &str) -> Option<&'static str> {
    match canonicalize_text(cmavo).as_str() {
        "a" | "e" | "ji" | "o" | "u" => Some("A"),
        "ba" | "ca" | "pu" => Some("PU"),
        "ba'e" | "za'e" => Some("BAhE"),
        "be" => Some("BE"),
        "bei" => Some("BEI"),
        "be'o" => Some("BEhO"),
        "by" | "cy" | "dy" | "fy" | "gy" | "jy" | "ky" | "ly" | "my" | "ny" | "py" | "ry"
        | "sy" | "ty" | "vy" | "xy" | "y'y" | "zy" => Some("BY"),
        "cai" | "cu'i" | "pei" | "ru'e" | "sai" => Some("CAI"),
        "ce'e" => Some("CEhE"),
        "co" => Some("CO"),
        "coi" | "co'o" | "je'e" | "ju'i" | "mi'e" | "mu'o" | "ta'a" => Some("COI"),
        "cu" => Some("CU"),
        "da" | "de" | "di" | "do" | "fo'a" | "fo'e" | "fo'i" | "fo'o" | "fo'u" | "ko" | "ko'a"
        | "ko'e" | "ko'i" | "ko'o" | "ko'u" | "ma" | "mi" | "ra" | "ri" | "ru" | "ta" | "ti"
        | "tu" | "vo'a" | "vo'e" | "vo'i" | "vo'o" | "vo'u" | "zo'e" | "zu'i" => Some("KOhA"),
        "fa" | "fai" | "fe" | "fi" | "fi'a" | "fo" | "fu" => Some("FA"),
        "fa'o" => Some("FAhO"),
        "fu'a" => Some("FUhA"),
        "ge'a" | "pa'i" | "re'a" | "sa'i" | "sa'o" | "su'i" | "te'a" | "va'a" | "vu'u" | "cu'a"
        | "de'o" | "fe'a" | "fe'i" | "fu'u" | "ju'u" | "ne'o" | "pi'a" | "pi'i" | "ri'o" => {
            Some("VUhU")
        }
        "goi" | "ne" | "no'u" | "pe" | "po" | "po'e" | "po'u" => Some("GOI"),
        "i" => Some("I"),
        "ja" | "je" | "jo" | "ju" => Some("JA"),
        "jei" | "ka" | "li'i" | "mu'e" | "ni" | "nu" | "pu'u" | "si'o" | "su'u" | "za'i"
        | "zu'o" => Some("NU"),
        "jo'i" => Some("JOhI"),
        "joi" | "ce" | "ce'o" | "fa'u" | "jo'e" | "jo'u" | "ju'e" | "ku'a" | "pi'u" => Some("JOI"),
        "la" | "lai" | "la'i" => Some("LA"),
        "le" | "lei" | "le'e" | "le'i" | "lo" | "loi" | "lo'e" | "lo'i" => Some("LE"),
        "li" | "me'o" => Some("LI"),
        "la'e" | "lu'a" | "lu'e" | "lu'i" | "lu'o" | "tu'a" | "vu'i" => Some("LAhE"),
        "li'u" => Some("LIhU"),
        "lo'o" => Some("LOhO"),
        "ni'o" => Some("NIhO"),
        "lu" => Some("LU"),
        "tu'e" => Some("TUhE"),
        "to" => Some("TO"),
        "toi" => Some("TOI"),
        "zo" | "ma'oi" => Some("ZO"),
        "zoi" | "la'o" | "mu'oi" => Some("ZOI"),
        "lo'u" => Some("LOhU"),
        "le'u" => Some("LEhU"),
        "bu" => Some("BU"),
        "zei" => Some("ZEI"),
        "na" | "ja'a" => Some("NA"),
        "nai" => Some("NAI"),
        "na'e" | "je'a" | "no'e" | "to'e" => Some("NAhE"),
        "noi" | "poi" | "voi" => Some("NOI"),
        "pa" | "re" | "ci" | "vo" | "mu" | "xa" | "ze" | "bi" | "so" | "no" | "pi" => Some("PA"),
        "pe'e" => Some("PEhE"),
        "sa" => Some("SA"),
        "se" | "te" | "ve" | "xe" => Some("SE"),
        "si" => Some("SI"),
        "su" => Some("SU"),
        "va" | "vi" | "vu" => Some("VA"),
        "vau" => Some("VAU"),
        "vei" => Some("VEI"),
        "ve'o" => Some("VEhO"),
        "xi" => Some("XI"),
        "y" => Some("Y"),
        "za" | "zi" | "zu" => Some("ZI"),
        "zo'u" => Some("ZOhU"),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub fn visible_selmaho(word_like: &WordLike) -> Option<&'static str> {
    match word_like.as_data() {
        data!(WordLike::Bare(word)) => word.selmaho(),
        data!(WordLike::ZoQuote { .. }) => Some("ZO"),
        data!(WordLike::ZoiQuote { zoi, .. }) => zoi.selmaho(),
        data!(WordLike::LohuQuote { .. }) => Some("LOhU"),
        data!(WordLike::SingleWordQuote { marker, .. }) => marker.selmaho(),
        data!(WordLike::Letter { .. }) => Some("BU"),
        data!(WordLike::ZeiLujvo { .. }) => Some("ZEI"),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
fn word_like_byte_range(word_like: &WordLike) -> Option<std::ops::Range<usize>> {
    match word_like.as_data() {
        data!(WordLike::Bare(word)) => Some(word.span.byte_start..word.span.byte_end),
        data!(WordLike::ZoQuote { zo, word }) => Some(zo.span.byte_start..word.span.byte_end),
        data!(WordLike::ZoiQuote {
            zoi,
            closing_delimiter,
            ..
        }) => Some(zoi.span.byte_start..closing_delimiter.span.byte_end),
        data!(WordLike::LohuQuote { lohu, lehu, .. }) => {
            Some(lohu.span.byte_start..lehu.span.byte_end)
        }
        data!(WordLike::SingleWordQuote {
            marker,
            quoted_text
        }) => Some(marker.span.byte_start..quoted_text.byte_end),
        data!(WordLike::Letter { base, bu }) => {
            word_like_byte_range(base).map(|range| range.start..bu.span.byte_end.max(range.end))
        }
        data!(WordLike::ZeiLujvo { left, right, .. }) => {
            word_like_byte_range(left).map(|range| range.start..right.span.byte_end.max(range.end))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::requires;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn default_options_enforce_cgv_ban() {
        assert!(MorphologyOptions::default().enforce_cgv_ban);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn segments_simple_cmavo_and_gismu() {
        let words = segment_words_with_modifiers("mi klama do").expect("valid morphology");
        assert_eq!(words.len(), 3);
        assert_eq!(
            base_word(&words[0]).map(|word| word.kind),
            Some(WordKind::Cmavo)
        );
        assert_eq!(
            base_word(&words[0]).map(|word| word.phonemes.as_str()),
            Some("mi")
        );
        assert_eq!(
            base_word(&words[1]).map(|word| word.kind),
            Some(WordKind::Gismu)
        );
        assert_eq!(
            base_word(&words[1]).map(|word| word.phonemes.as_str()),
            Some("klama")
        );
        assert_eq!(
            base_word(&words[2]).map(|word| word.kind),
            Some(WordKind::Cmavo)
        );
        assert_eq!(
            base_word(&words[2]).map(|word| word.span.char_start),
            Some(9)
        );
        assert_eq!(
            base_word(&words[2]).map(|word| word.span.char_end),
            Some(11)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn splits_adjacent_cmavo() {
        let words = segment_words_with_modifiers("mimi").expect("valid morphology");
        let phonemes: Vec<_> = words
            .iter()
            .map(|word| base_word(word).expect("base word").phonemes.as_str())
            .collect();
        assert_eq!(phonemes, ["mi", "mi"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn marks_cmavo_glides() {
        let words = segment_words_with_modifiers_raw("coi .ui").expect("valid morphology");
        let phonemes: Vec<_> = words
            .iter()
            .map(|word| base_word(word).expect("base word").phonemes.as_str())
            .collect();
        assert_eq!(phonemes, ["coĭ", "ŭi"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_cbm_dialect_to_morphology_options() {
        let dialect = jbotci_dialect::parse_dialect_definition("(cbm)").expect("dialect");
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);
        let words = segment_words_with_modifiers_with_options("mi .alis. do sa broda", &options)
            .expect("valid morphology");
        let phonemes: Vec<_> = words
            .iter()
            .map(|word| base_word(word).expect("base word").phonemes.as_str())
            .collect();
        assert_eq!(phonemes, ["mi", "broda"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_allow_cgv_dialect_to_morphology_options() {
        let dialect = jbotci_dialect::parse_dialect_definition("(allow-cgv)").expect("dialect");
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);
        let words = segment_words_with_modifiers_with_options("la siatl.", &options)
            .expect("valid morphology");
        assert_eq!(
            base_word(&words[1]).map(|word| word.phonemes.as_str()),
            Some("sĭatl")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_case_insensitive_dialect_to_morphology_options() {
        let dialect =
            jbotci_dialect::parse_dialect_definition("(case-insensitive)").expect("dialect");
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);
        let words = segment_words_with_modifiers_with_options("NALSELTRO", &options)
            .expect("valid morphology");
        assert_eq!(
            base_word(&words[0]).map(|word| word.phonemes.as_str()),
            Some("nalseltro")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_combined_dialect_formula_to_morphology_options() {
        let dialect = jbotci_dialect::parse_dialect_definition("(allow-cgv case-insensitive)")
            .expect("dialect");
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);
        let words = segment_words_with_modifiers_with_options("la ITALIAS.", &options)
            .expect("valid morphology");
        assert_eq!(
            base_word(&words[1]).map(|word| word.phonemes.as_str()),
            Some("italĭas")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_equivalence_ignores_spans_and_diacritics_on_words() {
        let mut left = segment_words_with_modifiers("coi").expect("valid morphology");
        let mut right = segment_words_with_modifiers("coi").expect("valid morphology");
        let word = match right[0].as_data() {
            data!(WordLike::Bare(word)) => (**word).clone(),
            _ => panic!("expected bare word"),
        };
        right[0] = WordLike::bare(word.with_data(data! {
            phonemes: String::from("coĭ"),
            span: SourceSpan::new(None, 99, 102, 99, 102).expect("valid span"),
        }));

        assert!(word_like_syntax_eq(&left.remove(0), &right.remove(0)));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_morphology_options_are_rejected() {
        let panic = std::panic::catch_unwind(|| {
            let _ = MorphologyOptions::default().with_data(data! {
                cmavo_dialect_entries: vec![CmavoDialectEntry::Expansion {
                    source: "mi".to_owned(),
                    replacement: Vec::new(),
                }],
            });
        });
        assert!(panic.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_deserialization_rejects_invalid_words() {
        let error = serde_json::from_str::<Word>(
            r#"{
                "kind": "cmavo",
                "phonemes": "",
                "span": {
                    "source_id": null,
                    "byte_start": 0,
                    "byte_end": 0,
                    "char_start": 0,
                    "char_end": 0,
                    "start": null,
                    "end": null
                },
                "surface_override": null,
                "dialect_transform": null
            }"#,
        )
        .expect_err("empty phoneme text must be rejected");

        assert!(
            error
                .to_string()
                .contains("word phoneme text must not be empty")
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn base_word(word: &WordLike) -> Option<&Word> {
        match word.as_data() {
            data!(WordLike::Bare(word)) => Some(word),
            _ => None,
        }
    }
}
