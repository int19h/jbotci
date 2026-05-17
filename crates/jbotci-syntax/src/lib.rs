//! Lojban syntax model and parser facade.

mod grammar;

use bityzba::{expensive_invariant, fields, invariant};
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

#[expensive_invariant(lojban_text_raw_is_valid(self.as_raw()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LojbanText {
    pub leading_nai: Vec<WordWithModifiers>,
    pub leading_cmevla: Vec<WordWithModifiers>,
    pub leading_indicators: Vec<WordWithModifiers>,
    pub leading_free_modifiers: Vec<FreeModifier>,
    pub leading_connective: Option<Connective>,
    pub paragraphs: Vec<Paragraph>,
}

#[expensive_invariant(paragraph_raw_is_valid(self.as_raw()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paragraph {
    pub i: Option<WordWithModifiers>,
    pub niho: Vec<WordWithModifiers>,
    pub free_modifiers: Vec<FreeModifier>,
    pub statements: Vec<ParagraphStatement>,
}

#[expensive_invariant(paragraph_statement_raw_is_valid(self.as_raw()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParagraphStatement {
    pub i: Option<WordWithModifiers>,
    pub connective: Option<Connective>,
    pub free_modifiers: Vec<FreeModifier>,
    pub statement: Option<Statement>,
}

#[expensive_invariant(statement_raw_is_valid(self.as_raw()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Statement {
    Fragment { fragment: Fragment },
    Placeholder,
}

impl Statement {
    pub fn fragment(fragment: Fragment) -> Self {
        Self::from_raw(fields!(Statement::Fragment { fragment: fragment }))
    }

    pub fn placeholder() -> Self {
        Self::from_raw(fields!(Statement::Placeholder))
    }
}

#[expensive_invariant(fragment_raw_is_valid(self.as_raw()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Fragment {
    Other { words: Vec<WordWithModifiers> },
}

impl Fragment {
    pub fn other(words: Vec<WordWithModifiers>) -> Self {
        Self::from_raw(fields!(Fragment::Other { words: words }))
    }
}

#[expensive_invariant(free_modifier_raw_is_valid(self.as_raw()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FreeModifier {
    Words { words: Vec<WordWithModifiers> },
}

impl FreeModifier {
    pub fn words(words: Vec<WordWithModifiers>) -> Self {
        Self::from_raw(fields!(FreeModifier::Words { words: words }))
    }
}

#[expensive_invariant(connective_raw_is_valid(self.as_raw()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Connective {
    Words { words: Vec<WordWithModifiers> },
}

impl Connective {
    pub fn words(words: Vec<WordWithModifiers>) -> Self {
        Self::from_raw(fields!(Connective::Words { words: words }))
    }
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

#[expensive_invariant(syntax_parse_raw_is_valid(self.as_raw()))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxParse {
    pub parse_tree: SyntaxValue,
    #[serde(default)]
    pub warnings: Vec<SyntaxWarning>,
}

#[invariant(syntax_warning_raw_is_valid(self.as_raw()))]
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
    pub fn experimental_construct(
        construct: impl Into<String>,
        anchor_index: usize,
        anchor: WordWithModifiers,
    ) -> Self {
        Self::from_raw(fields!(SyntaxWarning::ExperimentalConstruct {
            construct: construct.into(),
            anchor_index: anchor_index,
            anchor: anchor,
        }))
    }
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
#[expensive_invariant(syntax_tree_raw_is_valid(self.as_raw()))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxTree {
    pub root: SyntaxValue,
}

#[invariant(!self.constructor.is_empty(), "syntax constructor must not be empty")]
#[expensive_invariant(syntax_node_raw_is_valid(self.as_raw()))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxNode {
    pub constructor: String,
    #[serde(default)]
    pub fields: Vec<SyntaxField>,
}

#[invariant(self.name.as_ref().is_none_or(|name| !name.is_empty()), "syntax field name must not be empty")]
#[expensive_invariant(syntax_field_raw_is_valid(self.as_raw()))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxField {
    #[serde(default)]
    pub name: Option<String>,
    pub value: SyntaxValue,
}

#[expensive_invariant(syntax_value_raw_is_valid(self.as_raw()))]
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
    pub fn null() -> Self {
        Self::from_raw(fields!(SyntaxValue::Null))
    }

    pub fn r#bool(value: bool) -> Self {
        Self::from_raw(fields!(SyntaxValue::Bool { value: value }))
    }

    pub fn integer(value: i64) -> Self {
        Self::from_raw(fields!(SyntaxValue::Integer { value: value }))
    }

    pub fn text(value: impl Into<String>) -> Self {
        Self::from_raw(fields!(SyntaxValue::Text {
            value: value.into(),
        }))
    }

    pub fn list(items: Vec<SyntaxValue>) -> Self {
        Self::from_raw(fields!(SyntaxValue::List { items: items }))
    }

    pub fn node(constructor: impl Into<String>, fields: Vec<SyntaxField>) -> Self {
        Self::from_raw(fields!(SyntaxValue::Node {
            node: Box::new(SyntaxNode::new(fields! {
                constructor: constructor.into(),
                fields: fields,
            })),
        }))
    }

    pub fn word(word: WordWithModifiers) -> Self {
        Self::from_raw(fields!(SyntaxValue::Word {
            word: Box::new(word),
        }))
    }

    pub fn json(value: serde_json::Value) -> Self {
        Self::from_raw(fields!(SyntaxValue::Json { value: value }))
    }
}

pub fn syntax_values_equivalent(left: &SyntaxValue, right: &SyntaxValue) -> bool {
    match (left.as_raw(), right.as_raw()) {
        (fields!(SyntaxValue::Null), fields!(SyntaxValue::Null)) => true,
        (
            fields!(SyntaxValue::Bool { value: left }),
            fields!(SyntaxValue::Bool { value: right }),
        ) => left == right,
        (
            fields!(SyntaxValue::Integer { value: left }),
            fields!(SyntaxValue::Integer { value: right }),
        ) => left == right,
        (
            fields!(SyntaxValue::Text { value: left }),
            fields!(SyntaxValue::Text { value: right }),
        ) => left == right,
        (
            fields!(SyntaxValue::List { items: left }),
            fields!(SyntaxValue::List { items: right }),
        ) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| syntax_values_equivalent(left, right))
        }
        (fields!(SyntaxValue::Node { node: left }), fields!(SyntaxValue::Node { node: right })) => {
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
        (fields!(SyntaxValue::Word { word: left }), fields!(SyntaxValue::Word { word: right })) => {
            word_with_modifiers_syntax_eq(left, right)
        }
        (
            fields!(SyntaxValue::Json { value: left }),
            fields!(SyntaxValue::Json { value: right }),
        ) => left == right,
        _ => false,
    }
}

fn lojban_text_raw_is_valid(raw: &LojbanTextRaw) -> bool {
    raw.leading_free_modifiers
        .iter()
        .all(|modifier| free_modifier_raw_is_valid(modifier.as_raw()))
        && raw
            .leading_connective
            .as_ref()
            .is_none_or(|connective| connective_raw_is_valid(connective.as_raw()))
        && raw
            .paragraphs
            .iter()
            .all(|paragraph| paragraph_raw_is_valid(paragraph.as_raw()))
}

fn paragraph_raw_is_valid(raw: &ParagraphRaw) -> bool {
    raw.free_modifiers
        .iter()
        .all(|modifier| free_modifier_raw_is_valid(modifier.as_raw()))
        && raw
            .statements
            .iter()
            .all(|statement| paragraph_statement_raw_is_valid(statement.as_raw()))
}

fn paragraph_statement_raw_is_valid(raw: &ParagraphStatementRaw) -> bool {
    raw.connective
        .as_ref()
        .is_none_or(|connective| connective_raw_is_valid(connective.as_raw()))
        && raw
            .free_modifiers
            .iter()
            .all(|modifier| free_modifier_raw_is_valid(modifier.as_raw()))
        && raw
            .statement
            .as_ref()
            .is_none_or(|statement| statement_raw_is_valid(statement.as_raw()))
}

fn statement_raw_is_valid(raw: &StatementRaw) -> bool {
    match raw {
        fields!(Statement::Fragment { fragment }) => fragment_raw_is_valid(fragment.as_raw()),
        fields!(Statement::Placeholder) => true,
    }
}

fn fragment_raw_is_valid(raw: &FragmentRaw) -> bool {
    match raw {
        fields!(Fragment::Other { words: _ }) => true,
    }
}

fn free_modifier_raw_is_valid(raw: &FreeModifierRaw) -> bool {
    match raw {
        fields!(FreeModifier::Words { words: _ }) => true,
    }
}

fn connective_raw_is_valid(raw: &ConnectiveRaw) -> bool {
    match raw {
        fields!(Connective::Words { words: _ }) => true,
    }
}

fn syntax_parse_raw_is_valid(raw: &SyntaxParseRaw) -> bool {
    syntax_value_raw_is_valid(raw.parse_tree.as_raw())
        && raw
            .warnings
            .iter()
            .all(|warning| syntax_warning_raw_is_valid(warning.as_raw()))
}

fn syntax_warning_raw_is_valid(raw: &SyntaxWarningRaw) -> bool {
    match raw {
        fields!(SyntaxWarning::ExperimentalConstruct { construct, .. }) => !construct.is_empty(),
    }
}

fn syntax_tree_raw_is_valid(raw: &SyntaxTreeRaw) -> bool {
    syntax_value_raw_is_valid(raw.root.as_raw())
}

fn syntax_node_raw_is_valid(raw: &SyntaxNodeRaw) -> bool {
    raw.fields
        .iter()
        .all(|field| syntax_field_raw_is_valid(field.as_raw()))
}

fn syntax_field_raw_is_valid(raw: &SyntaxFieldRaw) -> bool {
    syntax_value_raw_is_valid(raw.value.as_raw())
}

fn syntax_value_raw_is_valid(raw: &SyntaxValueRaw) -> bool {
    match raw {
        fields!(SyntaxValue::Null)
        | fields!(SyntaxValue::Bool { .. })
        | fields!(SyntaxValue::Integer { .. })
        | fields!(SyntaxValue::Text { .. })
        | fields!(SyntaxValue::Word { .. })
        | fields!(SyntaxValue::Json { .. }) => true,
        fields!(SyntaxValue::List { items }) => items
            .iter()
            .all(|item| syntax_value_raw_is_valid(item.as_raw())),
        fields!(SyntaxValue::Node { node }) => syntax_node_raw_is_valid(node.as_raw()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_value_validity_rejects_empty_constructor() {
        let error = SyntaxNode::try_from_raw(fields!(SyntaxNode {
            constructor: String::new(),
            fields: Vec::new(),
        }))
        .expect_err("empty constructor should violate syntax node invariant");

        assert!(
            error
                .to_string()
                .contains("syntax constructor must not be empty")
        );
    }

    #[test]
    fn syntax_field_rejects_empty_name() {
        let error = SyntaxField::try_from_raw(fields!(SyntaxField {
            name: Some(String::new()),
            value: SyntaxValue::null(),
        }))
        .expect_err("empty field name should violate syntax field invariant");

        assert!(
            error
                .to_string()
                .contains("syntax field name must not be empty")
        );
    }

    #[test]
    #[should_panic]
    fn syntax_node_constructor_contract_is_reported() {
        let _ = SyntaxValue::node("", Vec::new());
    }
}
