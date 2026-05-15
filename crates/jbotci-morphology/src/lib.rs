//! Lojban morphology model.

use jbotci_source::SourceSpan;
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
    #[error("morphology parsing is not implemented yet")]
    NotImplemented,
}

pub fn segment_words_with_modifiers(
    _input: &str,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    Err(MorphologyError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_options_enforce_cgv_ban() {
        assert!(MorphologyOptions::default().enforce_cgv_ban);
    }
}
