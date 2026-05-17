//! Lojban syntax model and parser facade.

mod grammar;

use jbotci_contracts::{expensive_ensures, expensive_requires};
use jbotci_morphology::{WordWithModifiers, word_with_modifiers_syntax_eq};
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

impl LojbanText {
    pub fn is_valid(&self) -> bool {
        self.leading_nai.iter().all(WordWithModifiers::is_valid)
            && self.leading_cmevla.iter().all(WordWithModifiers::is_valid)
            && self
                .leading_indicators
                .iter()
                .all(WordWithModifiers::is_valid)
            && self
                .leading_free_modifiers
                .iter()
                .all(FreeModifier::is_valid)
            && self
                .leading_connective
                .as_ref()
                .is_none_or(Connective::is_valid)
            && self.paragraphs.iter().all(Paragraph::is_valid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paragraph {
    pub i: Option<WordWithModifiers>,
    pub niho: Vec<WordWithModifiers>,
    pub free_modifiers: Vec<FreeModifier>,
    pub statements: Vec<ParagraphStatement>,
}

impl Paragraph {
    pub fn is_valid(&self) -> bool {
        self.i.as_ref().is_none_or(WordWithModifiers::is_valid)
            && self.niho.iter().all(WordWithModifiers::is_valid)
            && self.free_modifiers.iter().all(FreeModifier::is_valid)
            && self.statements.iter().all(ParagraphStatement::is_valid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParagraphStatement {
    pub i: Option<WordWithModifiers>,
    pub connective: Option<Connective>,
    pub free_modifiers: Vec<FreeModifier>,
    pub statement: Option<Statement>,
}

impl ParagraphStatement {
    pub fn is_valid(&self) -> bool {
        self.i.as_ref().is_none_or(WordWithModifiers::is_valid)
            && self.connective.as_ref().is_none_or(Connective::is_valid)
            && self.free_modifiers.iter().all(FreeModifier::is_valid)
            && self.statement.as_ref().is_none_or(Statement::is_valid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Statement {
    Fragment { fragment: Fragment },
    Placeholder,
}

impl Statement {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Fragment { fragment } => fragment.is_valid(),
            Self::Placeholder => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Fragment {
    Other { words: Vec<WordWithModifiers> },
}

impl Fragment {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Other { words } => words.iter().all(WordWithModifiers::is_valid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FreeModifier {
    Words { words: Vec<WordWithModifiers> },
}

impl FreeModifier {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Words { words } => words.iter().all(WordWithModifiers::is_valid),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Connective {
    Words { words: Vec<WordWithModifiers> },
}

impl Connective {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Words { words } => words.iter().all(WordWithModifiers::is_valid),
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SyntaxError {
    #[error("syntax parsing is not implemented yet")]
    NotImplemented,
    #[error("syntax parse failed at byte {byte_offset}: {reason}")]
    Parse { byte_offset: usize, reason: String },
}

#[expensive_requires(words.iter().all(WordWithModifiers::is_valid))]
#[expensive_ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(LojbanText::is_valid))]
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

impl SyntaxParse {
    pub fn is_valid(&self) -> bool {
        self.parse_tree.is_valid() && self.warnings.iter().all(SyntaxWarning::is_valid)
    }
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

impl SyntaxWarning {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::ExperimentalConstruct {
                construct, anchor, ..
            } => !construct.is_empty() && anchor.is_valid(),
        }
    }
}

#[expensive_requires(words.iter().all(WordWithModifiers::is_valid))]
#[expensive_ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(SyntaxParse::is_valid))]
pub fn parse_syntax_tree(words: &[WordWithModifiers]) -> Result<SyntaxParse, SyntaxError> {
    parse_syntax_tree_with_options(words, &ParseOptions::default())
}

#[expensive_requires(words.iter().all(WordWithModifiers::is_valid))]
#[expensive_ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(SyntaxParse::is_valid))]
pub fn parse_syntax_tree_with_options(
    words: &[WordWithModifiers],
    options: &ParseOptions,
) -> Result<SyntaxParse, SyntaxError> {
    grammar::parse_syntax_tree(words, options)
}

#[expensive_requires(words.iter().all(WordWithModifiers::is_valid))]
#[expensive_ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(SyntaxParse::is_valid))]
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

impl SyntaxTree {
    pub fn is_valid(&self) -> bool {
        self.root.is_valid()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxNode {
    pub constructor: String,
    #[serde(default)]
    pub fields: Vec<SyntaxField>,
}

impl SyntaxNode {
    pub fn is_valid(&self) -> bool {
        !self.constructor.is_empty() && self.fields.iter().all(SyntaxField::is_valid)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxField {
    #[serde(default)]
    pub name: Option<String>,
    pub value: SyntaxValue,
}

impl SyntaxField {
    pub fn is_valid(&self) -> bool {
        self.name.as_ref().is_none_or(|name| !name.is_empty()) && self.value.is_valid()
    }
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
    #[expensive_ensures(ret.is_valid())]
    pub fn node(constructor: impl Into<String>, fields: Vec<SyntaxField>) -> Self {
        Self::Node {
            node: Box::new(SyntaxNode {
                constructor: constructor.into(),
                fields,
            }),
        }
    }

    #[expensive_requires(word.is_valid())]
    #[expensive_ensures(ret.is_valid())]
    pub fn word(word: WordWithModifiers) -> Self {
        Self::Word {
            word: Box::new(word),
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            Self::Null | Self::Bool { .. } | Self::Integer { .. } | Self::Text { .. } => true,
            Self::List { items } => items.iter().all(Self::is_valid),
            Self::Node { node } => node.is_valid(),
            Self::Word { word } => word.is_valid(),
            Self::Json { .. } => true,
        }
    }
}

#[expensive_requires(left.is_valid())]
#[expensive_requires(right.is_valid())]
pub fn syntax_values_equivalent(left: &SyntaxValue, right: &SyntaxValue) -> bool {
    match (left, right) {
        (SyntaxValue::Null, SyntaxValue::Null) => true,
        (SyntaxValue::Bool { value: left }, SyntaxValue::Bool { value: right }) => left == right,
        (SyntaxValue::Integer { value: left }, SyntaxValue::Integer { value: right }) => {
            left == right
        }
        (SyntaxValue::Text { value: left }, SyntaxValue::Text { value: right }) => left == right,
        (SyntaxValue::List { items: left }, SyntaxValue::List { items: right }) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| syntax_values_equivalent(left, right))
        }
        (SyntaxValue::Node { node: left }, SyntaxValue::Node { node: right }) => {
            left.constructor == right.constructor
                && left.fields.len() == right.fields.len()
                && left
                    .fields
                    .iter()
                    .zip(right.fields.iter())
                    .all(|(left, right)| {
                        left.name == right.name
                            && syntax_values_equivalent(&left.value, &right.value)
                    })
        }
        (SyntaxValue::Word { word: left }, SyntaxValue::Word { word: right }) => {
            word_with_modifiers_syntax_eq(left, right)
        }
        (SyntaxValue::Json { value: left }, SyntaxValue::Json { value: right }) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_value_validity_rejects_empty_constructor() {
        let value = SyntaxValue::Node {
            node: Box::new(SyntaxNode {
                constructor: String::new(),
                fields: Vec::new(),
            }),
        };
        assert!(!value.is_valid());
    }

    #[test]
    #[cfg(feature = "expensive_contracts")]
    #[should_panic]
    fn syntax_node_constructor_contract_is_reported() {
        let _ = SyntaxValue::node("", Vec::new());
    }
}
