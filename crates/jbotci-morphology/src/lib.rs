//! Lojban morphology model.

mod grammar;
mod segment;

use std::fmt;

use bityzba::{ensures, fields, invariant};
use bityzba::{expensive_ensures, expensive_requires};
pub use jbotci_dialect::{
    CmavoDialectEntry, CmavoDialectTransform, DialectDefinition, DialectFeature,
};
use jbotci_source::{SourceId, SourceLocationError, SourceSpan};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    fn default() -> Self {
        Self::try_from_fields(fields! {
            accept_latin: true,
            accept_cyrillic: true,
            accept_zbalermorna: true,
            cmavo_dialect_entries: Vec::new(),
            cmevla_as_relation_words: false,
            uppercase_marks_stress: true,
            enforce_cgv_ban: true,
        })
        .expect("default morphology options satisfy their invariant")
    }
}

impl MorphologyOptions {
    #[ensures(ret.cmavo_dialect_entries == definition.cmavo_entries)]
    #[ensures(definition.features.contains(&DialectFeature::Cbm) -> ret.cmevla_as_relation_words)]
    #[ensures(definition.features.contains(&DialectFeature::AllowCgv) -> !ret.enforce_cgv_ban)]
    #[ensures(definition.features.contains(&DialectFeature::CaseInsensitive) -> !ret.uppercase_marks_stress)]
    pub fn with_dialect_definition(self, definition: &DialectDefinition) -> Self {
        let mut raw = self.into_raw();
        raw.cmavo_dialect_entries = definition.cmavo_entries.clone();
        if definition.features.contains(&DialectFeature::Cbm) {
            raw.cmevla_as_relation_words = true;
        }
        if definition.features.contains(&DialectFeature::AllowCgv) {
            raw.enforce_cgv_ban = false;
        }
        if definition
            .features
            .contains(&DialectFeature::CaseInsensitive)
        {
            raw.uppercase_marks_stress = false;
        }
        Self::try_from_raw(raw).expect("dialect definition options preserve morphology invariants")
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

#[expensive_requires(left.is_valid())]
#[expensive_requires(right.is_valid())]
pub fn word_with_modifiers_syntax_eq(left: &WordWithModifiers, right: &WordWithModifiers) -> bool {
    match (left, right) {
        (
            WordWithModifiers::BaseWord { word_like: left },
            WordWithModifiers::BaseWord { word_like: right },
        ) => word_like_syntax_eq(left, right),
        (
            WordWithModifiers::StandaloneIndicator {
                indicator: left_indicator,
                nai: left_nai,
            },
            WordWithModifiers::StandaloneIndicator {
                indicator: right_indicator,
                nai: right_nai,
            },
        ) => {
            word_syntax_eq(left_indicator, right_indicator)
                && optional_word_syntax_eq(left_nai.as_deref(), right_nai.as_deref())
        }
        (
            WordWithModifiers::Emphasized {
                bahe: left_bahe,
                word_like: left_word_like,
            },
            WordWithModifiers::Emphasized {
                bahe: right_bahe,
                word_like: right_word_like,
            },
        ) => {
            word_syntax_eq(left_bahe, right_bahe)
                && word_like_syntax_eq(left_word_like, right_word_like)
        }
        (
            WordWithModifiers::WithIndicator {
                base: left_base,
                indicator: left_indicator,
                nai: left_nai,
            },
            WordWithModifiers::WithIndicator {
                base: right_base,
                indicator: right_indicator,
                nai: right_nai,
            },
        ) => {
            word_with_modifiers_syntax_eq(left_base, right_base)
                && word_syntax_eq(left_indicator, right_indicator)
                && optional_word_syntax_eq(left_nai.as_deref(), right_nai.as_deref())
        }
        (WordWithModifiers::NotEof, WordWithModifiers::NotEof) => true,
        _ => false,
    }
}

#[expensive_requires(left.is_valid())]
#[expensive_requires(right.is_valid())]
pub fn word_like_syntax_eq(left: &WordLike, right: &WordLike) -> bool {
    match (left, right) {
        (WordLike::Bare { word: left }, WordLike::Bare { word: right }) => {
            word_syntax_eq(left, right)
        }
        (
            WordLike::ZoQuote {
                zo: left_zo,
                word: left_word,
            },
            WordLike::ZoQuote {
                zo: right_zo,
                word: right_word,
            },
        ) => word_syntax_eq(left_zo, right_zo) && word_syntax_eq(left_word, right_word),
        (
            WordLike::ZoiQuote {
                zoi: left_zoi,
                opening_delimiter: left_opening,
                quoted_text: left_quoted,
                closing_delimiter: left_closing,
            },
            WordLike::ZoiQuote {
                zoi: right_zoi,
                opening_delimiter: right_opening,
                quoted_text: right_quoted,
                closing_delimiter: right_closing,
            },
        ) => {
            word_syntax_eq(left_zoi, right_zoi)
                && word_syntax_eq(left_opening, right_opening)
                && left_quoted == right_quoted
                && word_syntax_eq(left_closing, right_closing)
        }
        (
            WordLike::LohuQuote {
                lohu: left_lohu,
                quoted_words: left_words,
                lehu: left_lehu,
            },
            WordLike::LohuQuote {
                lohu: right_lohu,
                quoted_words: right_words,
                lehu: right_lehu,
            },
        ) => {
            word_syntax_eq(left_lohu, right_lohu)
                && left_words.len() == right_words.len()
                && left_words
                    .iter()
                    .zip(right_words.iter())
                    .all(|(left, right)| word_syntax_eq(left, right))
                && word_syntax_eq(left_lehu, right_lehu)
        }
        (
            WordLike::SingleWordQuote {
                marker: left_marker,
                quoted_text: left_quoted,
            },
            WordLike::SingleWordQuote {
                marker: right_marker,
                quoted_text: right_quoted,
            },
        ) => word_syntax_eq(left_marker, right_marker) && left_quoted == right_quoted,
        (
            WordLike::Letter {
                base: left_base,
                bu: left_bu,
            },
            WordLike::Letter {
                base: right_base,
                bu: right_bu,
            },
        ) => word_like_syntax_eq(left_base, right_base) && word_syntax_eq(left_bu, right_bu),
        (
            WordLike::ZeiLujvo {
                left: left_left,
                zei: left_zei,
                right: left_right,
            },
            WordLike::ZeiLujvo {
                left: right_left,
                zei: right_zei,
                right: right_right,
            },
        ) => {
            word_like_syntax_eq(left_left, right_left)
                && word_syntax_eq(left_zei, right_zei)
                && word_syntax_eq(left_right, right_right)
        }
        _ => false,
    }
}

pub fn word_syntax_eq(left: &Word, right: &Word) -> bool {
    left.kind == right.kind && strip_diacritics(&left.phonemes) == strip_diacritics(&right.phonemes)
}

#[ensures(!ret.is_empty() || text.is_empty())]
pub fn strip_diacritics(text: &str) -> String {
    text.chars().filter_map(strip_diacritic).collect()
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

#[expensive_ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|words| words.iter().all(WordWithModifiers::is_valid)))]
pub fn segment_words_with_modifiers_raw_with_options(
    input: &str,
    options: &MorphologyOptions,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    segment_words_with_modifiers_raw_with_options_and_source_id(input, options, None)
}

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
        WordWithModifiers::StandaloneIndicator { .. } => true,
        WordWithModifiers::Emphasized { word_like, .. } => word_like.is_valid(),
        WordWithModifiers::WithIndicator {
            base,
            indicator: _,
            nai: _,
        } => base.is_valid(),
        WordWithModifiers::NotEof => true,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn word_like_is_valid(word_like: &WordLike) -> bool {
    match word_like {
        WordLike::Bare { .. } | WordLike::ZoQuote { .. } => true,
        WordLike::ZoiQuote { quoted_text, .. } => source_span_is_valid(quoted_text),
        WordLike::LohuQuote { .. } => true,
        WordLike::SingleWordQuote { quoted_text, .. } => source_span_is_valid(quoted_text),
        WordLike::Letter { base, .. } => base.is_valid(),
        WordLike::ZeiLujvo { left, .. } => left.is_valid(),
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn source_span_is_valid(_span: &SourceSpan) -> bool {
    true
}

fn optional_word_syntax_eq(left: Option<&Word>, right: Option<&Word>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => word_syntax_eq(left, right),
        _ => false,
    }
}

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
    fn syntax_equivalence_ignores_spans_and_diacritics_on_words() {
        let mut left = segment_words_with_modifiers("coi").expect("valid morphology");
        let mut right = segment_words_with_modifiers("coi").expect("valid morphology");
        let WordWithModifiers::BaseWord { word_like } = &mut right[0] else {
            panic!("expected base word");
        };
        let WordLike::Bare { word } = word_like.as_mut() else {
            panic!("expected bare word");
        };
        **word = (**word)
            .clone()
            .try_with_fields(fields! {
                phonemes: String::from("coĭ"),
                span: SourceSpan::new(None, 99, 102, 99, 102).expect("valid span"),
            })
            .expect("updated test word remains valid");

        assert!(word_with_modifiers_syntax_eq(
            &left.remove(0),
            &right.remove(0)
        ));
    }

    #[test]
    fn invalid_morphology_options_are_rejected() {
        assert!(
            MorphologyOptions::default()
                .try_with_fields(fields! {
                    cmavo_dialect_entries: vec![CmavoDialectEntry::Expansion {
                        source: "mi".to_owned(),
                        replacement: Vec::new(),
                    }],
                })
                .is_err()
        );
    }

    #[test]
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
