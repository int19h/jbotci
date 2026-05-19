//! Lojban syntax model and parser facade.

mod grammar;

use std::fmt;

use bityzba::{data, invariant, new, requires};
use jbotci_dialect::DialectDefinition;
use jbotci_morphology::{Word, WordLike};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod ast {
    pub use crate::grammar::ast::*;
}
use ast::TextSyntax;

#[invariant(indicator_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Indicator {
    pub indicator: Box<Word>,
    pub nai: Option<Box<Word>>,
}

impl Indicator {
    #[requires(true)]
    #[ensures(true)]
    pub fn new(indicator: Word, nai: Option<Word>) -> Self {
        new!(Indicator {
            indicator: Box::new(indicator),
            nai: nai.map(Box::new),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn words(&self) -> Vec<WordWithModifiers> {
        let mut words = vec![WordWithModifiers::bare(WordLike::bare(
            (*self.indicator).clone(),
        ))];
        if let Some(nai) = &self.nai {
            words.push(WordWithModifiers::bare(WordLike::bare((**nai).clone())));
        }
        words
    }
}

#[requires(true)]
#[ensures(true)]
fn indicator_data_is_valid(indicator: &IndicatorData) -> bool {
    is_indicator_word(&indicator.indicator)
        && indicator
            .nai
            .as_deref()
            .is_none_or(|nai| nai.is_cmavo_text("nai"))
}

#[invariant(word_with_modifiers_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WordWithModifiers {
    Bare(Box<WordLike>),
    Emphasized {
        bahe: Box<Word>,
        word_like: Box<WordLike>,
    },
    WithIndicator {
        base: Box<WordWithModifiers>,
        indicator: Box<Word>,
        nai: Option<Box<Word>>,
    },
}

impl WordWithModifiers {
    #[requires(true)]
    #[ensures(true)]
    pub fn bare(word_like: WordLike) -> Self {
        new!(WordWithModifiers::Bare(Box::new(word_like)))
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
    pub fn word_like(&self) -> Option<&WordLike> {
        match self.as_data() {
            data!(WordWithModifiers::Bare(word_like))
            | data!(WordWithModifiers::Emphasized { word_like, .. }) => Some(word_like),
            data!(WordWithModifiers::WithIndicator { base, .. }) => base.word_like(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn visible_word(&self) -> Option<&Word> {
        self.word_like().and_then(WordLike::visible_base_word)
    }
}

impl fmt::Display for WordWithModifiers {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(WordWithModifiers::Bare(word_like)) => write!(f, "{word_like}"),
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
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn word_with_modifiers_data_is_valid(word: &WordWithModifiersData) -> bool {
    match word {
        data!(WordWithModifiers::Bare(..)) => true,
        data!(WordWithModifiers::Emphasized { bahe, .. }) => bahe.selmaho() == Some("BAhE"),
        data!(WordWithModifiers::WithIndicator { indicator, nai, .. }) => {
            is_indicator_word(indicator)
                && nai.as_deref().is_none_or(|nai| nai.is_cmavo_text("nai"))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn is_indicator_word(word: &Word) -> bool {
    word.kind == jbotci_morphology::WordKind::Cmavo
        && (crate::grammar::tokens::UI_WORDS.contains(&word.canonical_phonemes().as_str())
            || crate::grammar::tokens::CAI_WORDS.contains(&word.canonical_phonemes().as_str())
            || word.canonical_phonemes() == "y")
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[invariant(true)]
pub struct TraceOptions {
    pub level: u8,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[invariant(true)]
pub struct ParseOptions {
    pub trace: TraceOptions,
    pub dialect: DialectDefinition,
}

impl ParseOptions {
    #[requires(true)]
    #[ensures(ret.dialect == *definition)]
    pub fn with_dialect_definition(mut self, definition: &DialectDefinition) -> Self {
        self.dialect = definition.clone();
        self
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LojbanText {
    pub leading_nai: Vec<WordWithModifiers>,
    pub leading_cmevla: Vec<WordWithModifiers>,
    pub leading_indicators: Vec<Indicator>,
    pub leading_free_modifiers: Vec<FreeModifier>,
    pub leading_connective: Option<Connective>,
    pub paragraphs: Vec<Paragraph>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paragraph {
    pub i: Option<WordWithModifiers>,
    pub niho: Vec<WordWithModifiers>,
    pub free_modifiers: Vec<FreeModifier>,
    pub statements: Vec<ParagraphStatement>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParagraphStatement {
    pub i: Option<WordWithModifiers>,
    pub connective: Option<Connective>,
    pub free_modifiers: Vec<FreeModifier>,
    pub statement: Option<Statement>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Statement {
    Fragment { fragment: Fragment },
    Placeholder,
}

impl Statement {
    #[requires(true)]
    #[ensures(true)]
    pub fn fragment(fragment: Fragment) -> Self {
        Statement::Fragment { fragment }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn placeholder() -> Self {
        Statement::Placeholder
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Fragment {
    Other { words: Vec<WordWithModifiers> },
}

impl Fragment {
    #[requires(true)]
    #[ensures(true)]
    pub fn other(words: Vec<WordWithModifiers>) -> Self {
        Fragment::Other { words }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FreeModifier {
    Words { words: Vec<WordWithModifiers> },
}

impl FreeModifier {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(words: Vec<WordWithModifiers>) -> Self {
        FreeModifier::Words { words }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Connective {
    Words { words: Vec<WordWithModifiers> },
}

impl Connective {
    #[requires(true)]
    #[ensures(true)]
    pub fn words(words: Vec<WordWithModifiers>) -> Self {
        Connective::Words { words }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
pub enum SyntaxError {
    #[error("syntax parsing is not implemented yet")]
    NotImplemented,
    #[error("syntax parse failed at byte {byte_offset}: {reason}")]
    Parse { byte_offset: usize, reason: String },
}

#[requires(true)]
#[ensures(true)]
pub fn parse_text(words: &[WordLike], options: &ParseOptions) -> Result<LojbanText, SyntaxError> {
    grammar::parse_text(words, options)
}

#[requires(true)]
#[ensures(true)]
pub fn parse_raw_text(
    words: &[WordLike],
    options: &ParseOptions,
) -> Result<ast::TextSyntax, SyntaxError> {
    grammar::parse_raw_text(words, options)
}

#[invariant(syntax_parse_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SyntaxParse {
    pub parse_tree: TextSyntax,
    #[serde(default)]
    pub warnings: Vec<SyntaxWarning>,
}

#[invariant(syntax_warning_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SyntaxWarning {
    ExperimentalConstruct {
        construct: String,
        anchor_index: usize,
        anchor: WordWithModifiers,
    },
}

impl SyntaxWarning {
    #[requires(true)]
    #[ensures(true)]
    pub fn experimental_construct(
        construct: impl Into<String>,
        anchor_index: usize,
        anchor: WordWithModifiers,
    ) -> Self {
        new!(SyntaxWarning::ExperimentalConstruct {
            construct: construct.into(),
            anchor_index: anchor_index,
            anchor: anchor,
        })
    }
}

#[requires(true)]
#[ensures(true)]
pub fn parse_syntax_tree(words: &[WordLike]) -> Result<SyntaxParse, SyntaxError> {
    parse_syntax_tree_with_options(words, &ParseOptions::default())
}

#[requires(true)]
#[ensures(true)]
pub fn parse_syntax_tree_with_options(
    words: &[WordLike],
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    grammar::parse_syntax_tree(words, options)
}

#[requires(true)]
#[ensures(true)]
pub fn parse_syntax_tree_with_source_and_options(
    words: &[WordLike],
    source: &str,
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    grammar::parse_syntax_tree_with_source(words, Some(source), options)
}

#[requires(true)]
#[ensures(true)]
fn syntax_parse_data_is_valid(data: &SyntaxParseData) -> bool {
    data.warnings
        .iter()
        .all(|warning| syntax_warning_data_is_valid(warning.as_data()))
}

#[requires(true)]
#[ensures(true)]
fn syntax_warning_data_is_valid(data: &SyntaxWarningData) -> bool {
    match data {
        data!(SyntaxWarning::ExperimentalConstruct { construct, .. }) => !construct.is_empty(),
    }
}
