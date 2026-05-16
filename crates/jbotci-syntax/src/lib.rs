//! Lojban syntax model and parser facade.

mod grammar;

use jbotci_morphology::WordWithModifiers;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TraceOptions {
    pub level: u8,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ParseOptions {
    pub trace: TraceOptions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LojbanText {
    pub leading_nai: Vec<WordWithModifiers>,
    pub leading_cmevla: Vec<WordWithModifiers>,
    pub leading_indicators: Vec<WordWithModifiers>,
    pub leading_free_modifiers: Vec<FreeModifier>,
    pub leading_connective: Option<Connective>,
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paragraph {
    pub i: Option<WordWithModifiers>,
    pub niho: Vec<WordWithModifiers>,
    pub free_modifiers: Vec<FreeModifier>,
    pub statements: Vec<ParagraphStatement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParagraphStatement {
    pub i: Option<WordWithModifiers>,
    pub connective: Option<Connective>,
    pub free_modifiers: Vec<FreeModifier>,
    pub statement: Option<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Statement {
    Fragment { fragment: Fragment },
    Placeholder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Fragment {
    Other { words: Vec<WordWithModifiers> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FreeModifier {
    Words { words: Vec<WordWithModifiers> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Connective {
    Words { words: Vec<WordWithModifiers> },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SyntaxError {
    #[error("syntax parsing is not implemented yet")]
    NotImplemented,
    #[error("syntax parse failed at byte {byte_offset}: {reason}")]
    Parse { byte_offset: usize, reason: String },
}

pub fn parse_text(
    words: &[WordWithModifiers],
    options: &ParseOptions,
) -> Result<LojbanText, SyntaxError> {
    grammar::parse_text(words, options)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxParse {
    pub parse_tree: SyntaxValue,
    #[serde(default)]
    pub warnings: Vec<SyntaxWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SyntaxWarning {
    ExperimentalConstruct {
        construct: String,
        anchor_index: usize,
        anchor: WordWithModifiers,
    },
}

pub fn parse_syntax_tree(words: &[WordWithModifiers]) -> Result<SyntaxParse, SyntaxError> {
    parse_syntax_tree_with_options(words, &ParseOptions::default())
}

pub fn parse_syntax_tree_with_options(
    words: &[WordWithModifiers],
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    grammar::parse_syntax_tree(words, options)
}

pub fn parse_syntax_tree_with_source_and_options(
    words: &[WordWithModifiers],
    source: &str,
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    grammar::parse_syntax_tree_with_source(words, Some(source), options)
}

/// Lossless fixture representation for v0 syntax trees.
///
/// The parser port will eventually use the strongly typed parse-tree structs
/// directly. Until then, v0 exports syntax expectations as constructor records:
/// every node has a constructor name and an ordered field list. This preserves
/// record field order and avoids treating the raw tree as an opaque string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxTree {
    pub root: SyntaxValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxNode {
    pub constructor: String,
    #[serde(default)]
    pub fields: Vec<SyntaxField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxField {
    #[serde(default)]
    pub name: Option<String>,
    pub value: SyntaxValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SyntaxValue {
    Null,
    Bool { value: bool },
    Integer { value: i64 },
    Text { value: String },
    List { items: Vec<SyntaxValue> },
    Node { node: Box<SyntaxNode> },
    Word { word: Box<WordWithModifiers> },
    Json { value: serde_json::Value },
}

impl SyntaxValue {
    pub fn node(constructor: impl Into<String>, fields: Vec<SyntaxField>) -> Self {
        Self::Node {
            node: Box::new(SyntaxNode {
                constructor: constructor.into(),
                fields,
            }),
        }
    }

    pub fn word(word: WordWithModifiers) -> Self {
        Self::Word {
            word: Box::new(word),
        }
    }
}
