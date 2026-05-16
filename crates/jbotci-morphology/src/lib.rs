//! Lojban morphology model.

mod segment;

use jbotci_source::{SourceId, SourceLocationError, SourceSpan};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    fn default() -> Self {
        Self {
            accept_latin: true,
            accept_cyrillic: true,
            accept_zbalermorna: true,
            cmavo_dialect_entries: Vec::new(),
            cmevla_as_relation_words: false,
            uppercase_marks_stress: true,
            enforce_cgv_ban: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CmavoDialectEntry {
    Swap {
        left: String,
        right: String,
    },
    Expansion {
        source: String,
        replacement: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CmavoDialectTransform {
    pub source_text: String,
    pub target_text: String,
    pub group_key: String,
    pub output_index: usize,
    pub output_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
pub enum LujvoSegment {
    Rafsi { text: String },
    Hyphen { text: String },
}

impl LujvoSegment {
    pub fn text(&self) -> &str {
        match self {
            Self::Rafsi { text } | Self::Hyphen { text } => text,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Word {
    pub kind: WordKind,
    pub phonemes: String,
    pub span: SourceSpan,
    pub surface_override: Option<String>,
    pub dialect_transform: Option<CmavoDialectTransform>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum WordLike {
    Bare {
        word: Box<Word>,
    },
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum WordWithModifiers {
    BaseWord {
        word_like: Box<WordLike>,
    },
    StandaloneIndicator {
        indicator: Box<Word>,
        nai: Option<Box<Word>>,
    },
    Emphasized {
        bahe: Box<Word>,
        word_like: Box<WordLike>,
    },
    WithIndicator {
        base: Box<WordWithModifiers>,
        indicator: Box<Word>,
        nai: Option<Box<Word>>,
    },
    NotEof,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
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
    #[error("invalid source span: {0}")]
    SourceSpan(#[from] SourceLocationError),
}

pub fn segment_words_with_modifiers(
    input: &str,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        None,
    )
}

pub fn segment_words_with_modifiers_with_options(
    input: &str,
    options: &MorphologyOptions,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id(input, options, None)
}

pub fn segment_words_with_modifiers_with_source_id(
    input: &str,
    source_id: SourceId,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        Some(source_id),
    )
}

pub fn segment_words_with_modifiers_with_options_and_source_id(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    segment::segment_words_with_modifiers(input, options, source_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_enforce_cgv_ban() {
        assert!(MorphologyOptions::default().enforce_cgv_ban);
    }

    #[test]
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
    fn splits_adjacent_cmavo() {
        let words = segment_words_with_modifiers("mimi").expect("valid morphology");
        let phonemes: Vec<_> = words
            .iter()
            .map(|word| base_word(word).expect("base word").phonemes.as_str())
            .collect();
        assert_eq!(phonemes, ["mi", "mi"]);
    }

    #[test]
    fn marks_cmavo_glides() {
        let words = segment_words_with_modifiers("coi .ui").expect("valid morphology");
        let phonemes: Vec<_> = words
            .iter()
            .map(|word| base_word(word).expect("base word").phonemes.as_str())
            .collect();
        assert_eq!(phonemes, ["coĭ", "ŭi"]);
    }

    fn base_word(word: &WordWithModifiers) -> Option<&Word> {
        match word {
            WordWithModifiers::BaseWord { word_like } => match word_like.as_ref() {
                WordLike::Bare { word } => Some(word),
                _ => None,
            },
            _ => None,
        }
    }
}
