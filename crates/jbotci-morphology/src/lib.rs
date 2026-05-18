//! Lojban morphology model.

mod grammar;
mod segment;
mod syntax_eq;

use std::fmt;

use bityzba::{data, ensures, expensive_invariant, invariant, new, requires};
pub use jbotci_dialect::{
    CmavoDialectEntry, CmavoDialectTransform, DialectDefinition, DialectFeature,
};
use jbotci_source::{SourceId, SourceLocationError, SourceSpan};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use syntax_eq::{
    strip_diacritics, word_like_syntax_eq, word_syntax_eq, word_with_modifiers_syntax_eq,
};

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

#[expensive_invariant(word_like_data_is_valid(self.as_data()))]
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
    #[requires(true)]
    #[ensures(true)]
    pub fn bare(word: Word) -> Self {
        new!(WordLike::Bare {
            word: Box::new(word),
        })
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
}

impl fmt::Display for WordLike {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(WordLike::Bare { word }) => write!(f, "{word}"),
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

#[expensive_invariant(word_with_modifiers_data_is_valid(self.as_data()))]
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
    #[requires(true)]
    #[ensures(true)]
    pub fn base_word(word_like: WordLike) -> Self {
        new!(WordWithModifiers::BaseWord {
            word_like: Box::new(word_like),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn standalone_indicator(indicator: Word, nai: Option<Word>) -> Self {
        new!(WordWithModifiers::StandaloneIndicator {
            indicator: Box::new(indicator),
            nai: nai.map(Box::new),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn emphasized(bahe: Word, word_like: WordLike) -> Self {
        new!(WordWithModifiers::Emphasized {
            bahe: Box::new(bahe),
            word_like: Box::new(word_like),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn with_indicator(base: WordWithModifiers, indicator: Word, nai: Option<Word>) -> Self {
        new!(WordWithModifiers::WithIndicator {
            base: Box::new(base),
            indicator: Box::new(indicator),
            nai: nai.map(Box::new),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn not_eof() -> Self {
        new!(WordWithModifiers::NotEof)
    }
}

impl fmt::Display for WordWithModifiers {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(WordWithModifiers::BaseWord { word_like }) => write!(f, "{word_like}"),
            data!(WordWithModifiers::StandaloneIndicator { indicator, nai }) => {
                write!(f, "{indicator}")?;
                if let Some(nai) = nai {
                    write!(f, "-{nai}")?;
                }
                Ok(())
            }
            data!(WordWithModifiers::Emphasized { bahe, word_like }) => {
                write!(f, "{bahe}-{word_like}")
            }
            data!(WordWithModifiers::WithIndicator {
                base,
                indicator,
                nai,
            }) => {
                write!(f, "{base}-{indicator}")?;
                if let Some(nai) = nai {
                    write!(f, "-{nai}")?;
                }
                Ok(())
            }
            data!(WordWithModifiers::NotEof) => f.write_str("<not-eof>"),
        }
    }
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
    #[error("invalid source span: {0}")]
    SourceSpan(#[from] SourceLocationError),
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers(
    input: &str,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
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
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id(input, options, None)
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_with_options_and_source_id(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    grammar::segment_words_with_modifiers(input, options, source_id)
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_raw(
    input: &str,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
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
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
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
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    segment_words_with_modifiers_raw_with_options_and_source_id(input, options, None)
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_raw_with_options_and_source_id(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    grammar::segment_words_with_modifiers_raw(input, options, source_id)
}

#[cfg_attr(not(test), allow(dead_code))]
#[requires(true)]
#[ensures(true)]
fn word_with_modifiers_data_is_valid(word: &WordWithModifiersData) -> bool {
    match word {
        data!(WordWithModifiers::BaseWord { word_like }) => {
            word_like_data_is_valid(word_like.as_data())
        }
        data!(WordWithModifiers::StandaloneIndicator { .. }) => true,
        data!(WordWithModifiers::Emphasized { word_like, .. }) => {
            word_like_data_is_valid(word_like.as_data())
        }
        data!(WordWithModifiers::WithIndicator {
            base,
            indicator: _,
            nai: _,
        }) => word_with_modifiers_data_is_valid(base.as_data()),
        data!(WordWithModifiers::NotEof) => true,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[requires(true)]
#[ensures(true)]
fn word_like_data_is_valid(word_like: &WordLikeData) -> bool {
    match word_like {
        data!(WordLike::Bare { .. }) | data!(WordLike::ZoQuote { .. }) => true,
        data!(WordLike::ZoiQuote { quoted_text, .. }) => source_span_is_valid(quoted_text),
        data!(WordLike::LohuQuote { .. }) => true,
        data!(WordLike::SingleWordQuote { quoted_text, .. }) => source_span_is_valid(quoted_text),
        data!(WordLike::Letter { base, .. }) => word_like_data_is_valid(base.as_data()),
        data!(WordLike::ZeiLujvo { left, .. }) => word_like_data_is_valid(left.as_data()),
    }
}

#[cfg_attr(not(test), allow(dead_code))]
#[requires(true)]
#[ensures(true)]
fn source_span_is_valid(_span: &SourceSpan) -> bool {
    true
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
            data!(WordWithModifiers::BaseWord { word_like }) => match word_like.as_data() {
                data!(WordLike::Bare { word }) => (**word).clone(),
                _ => panic!("expected bare word"),
            },
            _ => panic!("expected base word"),
        };
        right[0] = WordWithModifiers::base_word(WordLike::bare(word.with_data(data! {
            phonemes: String::from("coĭ"),
            span: SourceSpan::new(None, 99, 102, 99, 102).expect("valid span"),
        })));

        assert!(word_with_modifiers_syntax_eq(
            &left.remove(0),
            &right.remove(0)
        ));
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
    fn base_word(word: &WordWithModifiers) -> Option<&Word> {
        match word.as_data() {
            data!(WordWithModifiers::BaseWord { word_like }) => match word_like.as_data() {
                data!(WordLike::Bare { word }) => Some(word),
                _ => None,
            },
            _ => None,
        }
    }
}
