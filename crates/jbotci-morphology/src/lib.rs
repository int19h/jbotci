//! Lojban morphology model.

mod grammar;
mod segment;

use std::fmt;

use contracts::{ensures, requires};
use jbotci_contracts::expensive_ensures;
pub use jbotci_dialect::{
    CmavoDialectEntry, CmavoDialectTransform, DialectDefinition, DialectFeature,
};
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
    #[ensures(ret.is_valid())]
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

impl MorphologyOptions {
    #[ensures(ret -> self.cmavo_dialect_entries.iter().all(CmavoDialectEntry::is_valid))]
    pub fn is_valid(&self) -> bool {
        self.cmavo_dialect_entries
            .iter()
            .all(CmavoDialectEntry::is_valid)
    }

    #[requires(self.is_valid())]
    #[requires(definition.is_valid())]
    #[ensures(ret.is_valid())]
    #[ensures(ret.cmavo_dialect_entries == definition.cmavo_entries)]
    #[ensures(definition.features.contains(&DialectFeature::Cbm) -> ret.cmevla_as_relation_words)]
    #[ensures(definition.features.contains(&DialectFeature::AllowCgv) -> !ret.enforce_cgv_ban)]
    #[ensures(definition.features.contains(&DialectFeature::CaseInsensitive) -> !ret.uppercase_marks_stress)]
    pub fn with_dialect_definition(mut self, definition: &DialectDefinition) -> Self {
        self.cmavo_dialect_entries = definition.cmavo_entries.clone();
        if definition.features.contains(&DialectFeature::Cbm) {
            self.cmevla_as_relation_words = true;
        }
        if definition.features.contains(&DialectFeature::AllowCgv) {
            self.enforce_cgv_ban = false;
        }
        if definition
            .features
            .contains(&DialectFeature::CaseInsensitive)
        {
            self.uppercase_marks_stress = false;
        }
        self
    }
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
    #[ensures(!ret.is_empty())]
    pub fn text(&self) -> &str {
        match self {
            Self::Rafsi { text } | Self::Hyphen { text } => text,
        }
    }
}

impl fmt::Display for WordKind {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Word {
    pub kind: WordKind,
    pub phonemes: String,
    pub span: SourceSpan,
    pub surface_override: Option<String>,
    pub dialect_transform: Option<CmavoDialectTransform>,
}

impl Word {
    #[ensures(ret -> !self.phonemes.is_empty())]
    #[ensures(ret -> source_span_is_valid(&self.span))]
    #[ensures(ret -> self.dialect_transform.as_ref().is_none_or(CmavoDialectTransform::is_valid))]
    pub fn is_valid(&self) -> bool {
        !self.phonemes.is_empty()
            && source_span_is_valid(&self.span)
            && self
                .dialect_transform
                .as_ref()
                .is_none_or(CmavoDialectTransform::is_valid)
    }
}

impl fmt::Display for Word {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.phonemes)
    }
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

impl WordLike {
    #[expensive_ensures(ret -> word_like_is_valid(self))]
    pub fn is_valid(&self) -> bool {
        word_like_is_valid(self)
    }
}

impl fmt::Display for WordLike {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bare { word } => write!(f, "{word}"),
            Self::ZoQuote { zo, word } => write!(f, "{zo}-<<{word}>>"),
            Self::ZoiQuote {
                zoi,
                opening_delimiter,
                quoted_text,
                closing_delimiter,
            } => write!(
                f,
                "{zoi}-{opening_delimiter}-<{} chars>-{closing_delimiter}",
                quoted_text.char_len()
            ),
            Self::LohuQuote {
                lohu,
                quoted_words,
                lehu,
            } => {
                write!(f, "{lohu}-<<")?;
                for (index, word) in quoted_words.iter().enumerate() {
                    if index > 0 {
                        f.write_str(" ")?;
                    }
                    write!(f, "{word}")?;
                }
                write!(f, ">>-{lehu}")
            }
            Self::SingleWordQuote {
                marker,
                quoted_text,
            } => write!(f, "{marker}-<{} chars>", quoted_text.char_len()),
            Self::Letter { base, bu } => write!(f, "{base}-{bu}"),
            Self::ZeiLujvo { left, zei, right } => write!(f, "{left}-{zei}-{right}"),
        }
    }
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

impl WordWithModifiers {
    #[expensive_ensures(ret -> word_with_modifiers_is_valid(self))]
    pub fn is_valid(&self) -> bool {
        word_with_modifiers_is_valid(self)
    }
}

impl fmt::Display for WordWithModifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BaseWord { word_like } => write!(f, "{word_like}"),
            Self::StandaloneIndicator { indicator, nai } => {
                write!(f, "{indicator}")?;
                if let Some(nai) = nai {
                    write!(f, "-{nai}")?;
                }
                Ok(())
            }
            Self::Emphasized { bahe, word_like } => write!(f, "{bahe}-{word_like}"),
            Self::WithIndicator {
                base,
                indicator,
                nai,
            } => {
                write!(f, "{base}-{indicator}")?;
                if let Some(nai) = nai {
                    write!(f, "-{nai}")?;
                }
                Ok(())
            }
            Self::NotEof => f.write_str("<not-eof>"),
        }
    }
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

#[expensive_ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|words| words.iter().all(WordWithModifiers::is_valid)))]
pub fn segment_words_with_modifiers(
    input: &str,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        None,
    )
}

#[requires(options.is_valid())]
#[expensive_ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|words| words.iter().all(WordWithModifiers::is_valid)))]
pub fn segment_words_with_modifiers_with_options(
    input: &str,
    options: &MorphologyOptions,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id(input, options, None)
}

#[expensive_ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|words| words.iter().all(WordWithModifiers::is_valid)))]
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

#[requires(options.is_valid())]
#[expensive_ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|words| words.iter().all(WordWithModifiers::is_valid)))]
pub fn segment_words_with_modifiers_with_options_and_source_id(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    grammar::segment_words_with_modifiers(input, options, source_id)
}

#[expensive_ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|words| words.iter().all(WordWithModifiers::is_valid)))]
pub fn segment_words_with_modifiers_raw(
    input: &str,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    segment_words_with_modifiers_raw_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        None,
    )
}

#[expensive_ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|words| words.iter().all(WordWithModifiers::is_valid)))]
pub fn segment_words_with_modifiers_raw_with_source_id(
    input: &str,
    source_id: SourceId,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    segment_words_with_modifiers_raw_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        Some(source_id),
    )
}

#[requires(options.is_valid())]
#[expensive_ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|words| words.iter().all(WordWithModifiers::is_valid)))]
pub fn segment_words_with_modifiers_raw_with_options(
    input: &str,
    options: &MorphologyOptions,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    segment_words_with_modifiers_raw_with_options_and_source_id(input, options, None)
}

#[requires(options.is_valid())]
#[expensive_ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|words| words.iter().all(WordWithModifiers::is_valid)))]
pub fn segment_words_with_modifiers_raw_with_options_and_source_id(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    grammar::segment_words_with_modifiers_raw(input, options, source_id)
}

#[cfg_attr(not(test), allow(dead_code))]
fn word_with_modifiers_is_valid(word: &WordWithModifiers) -> bool {
    match word {
        WordWithModifiers::BaseWord { word_like } => word_like.is_valid(),
        WordWithModifiers::StandaloneIndicator { indicator, nai } => {
            indicator.is_valid() && nai.as_ref().is_none_or(|word| word.is_valid())
        }
        WordWithModifiers::Emphasized { bahe, word_like } => {
            bahe.is_valid() && word_like.is_valid()
        }
        WordWithModifiers::WithIndicator {
            base,
            indicator,
            nai,
        } => {
            base.is_valid()
                && indicator.is_valid()
                && nai.as_ref().is_none_or(|word| word.is_valid())
        }
        WordWithModifiers::NotEof => true,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn word_like_is_valid(word_like: &WordLike) -> bool {
    match word_like {
        WordLike::Bare { word } => word.is_valid(),
        WordLike::ZoQuote { zo, word } => zo.is_valid() && word.is_valid(),
        WordLike::ZoiQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        } => {
            zoi.is_valid()
                && opening_delimiter.is_valid()
                && source_span_is_valid(quoted_text)
                && closing_delimiter.is_valid()
        }
        WordLike::LohuQuote {
            lohu,
            quoted_words,
            lehu,
        } => lohu.is_valid() && quoted_words.iter().all(Word::is_valid) && lehu.is_valid(),
        WordLike::SingleWordQuote {
            marker,
            quoted_text,
        } => marker.is_valid() && source_span_is_valid(quoted_text),
        WordLike::Letter { base, bu } => base.is_valid() && bu.is_valid(),
        WordLike::ZeiLujvo { left, zei, right } => {
            left.is_valid() && zei.is_valid() && right.is_valid()
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn source_span_is_valid(span: &SourceSpan) -> bool {
    span.byte_start <= span.byte_end && span.char_start <= span.char_end
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
        let words = segment_words_with_modifiers_raw("coi .ui").expect("valid morphology");
        let phonemes: Vec<_> = words
            .iter()
            .map(|word| base_word(word).expect("base word").phonemes.as_str())
            .collect();
        assert_eq!(phonemes, ["coĭ", "ŭi"]);
    }

    #[test]
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
    #[should_panic]
    fn invalid_morphology_options_contract_is_reported() {
        let options = MorphologyOptions {
            cmavo_dialect_entries: vec![CmavoDialectEntry::Expansion {
                source: "mi".to_owned(),
                replacement: Vec::new(),
            }],
            ..MorphologyOptions::default()
        };
        let _ = segment_words_with_modifiers_with_options("mi", &options);
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
