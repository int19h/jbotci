//! Lojban syntax model and parser facade.

mod grammar;

use bityzba::{data, expensive_invariant, invariant, new};
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

#[expensive_invariant(lojban_text_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LojbanText {
    pub leading_nai: Vec<WordWithModifiers>,
    pub leading_cmevla: Vec<WordWithModifiers>,
    pub leading_indicators: Vec<WordWithModifiers>,
    pub leading_free_modifiers: Vec<FreeModifier>,
    pub leading_connective: Option<Connective>,
    pub paragraphs: Vec<Paragraph>,
}

#[expensive_invariant(paragraph_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paragraph {
    pub i: Option<WordWithModifiers>,
    pub niho: Vec<WordWithModifiers>,
    pub free_modifiers: Vec<FreeModifier>,
    pub statements: Vec<ParagraphStatement>,
}

#[expensive_invariant(paragraph_statement_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParagraphStatement {
    pub i: Option<WordWithModifiers>,
    pub connective: Option<Connective>,
    pub free_modifiers: Vec<FreeModifier>,
    pub statement: Option<Statement>,
}

#[expensive_invariant(statement_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Statement {
    Fragment { fragment: Fragment },
    Placeholder,
}

impl Statement {
    pub fn fragment(fragment: Fragment) -> Self {
        new!(Statement::Fragment { fragment: fragment })
    }

    pub fn placeholder() -> Self {
        new!(Statement::Placeholder)
    }
}

#[expensive_invariant(fragment_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Fragment {
    Other { words: Vec<WordWithModifiers> },
}

impl Fragment {
    pub fn other(words: Vec<WordWithModifiers>) -> Self {
        new!(Fragment::Other { words: words })
    }
}

#[expensive_invariant(free_modifier_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FreeModifier {
    Words { words: Vec<WordWithModifiers> },
}

impl FreeModifier {
    pub fn words(words: Vec<WordWithModifiers>) -> Self {
        new!(FreeModifier::Words { words: words })
    }
}

#[expensive_invariant(connective_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Connective {
    Words { words: Vec<WordWithModifiers> },
}

impl Connective {
    pub fn words(words: Vec<WordWithModifiers>) -> Self {
        new!(Connective::Words { words: words })
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

#[expensive_invariant(syntax_parse_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxParse {
    pub parse_tree: SyntaxValue,
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
/// record field order and avoids treating the tree as an opaque string.
#[expensive_invariant(syntax_tree_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxTree {
    pub root: SyntaxValue,
}

#[invariant(!self.constructor.is_empty(), "syntax constructor must not be empty")]
#[expensive_invariant(syntax_node_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxNode {
    pub constructor: String,
    #[serde(default)]
    pub fields: Vec<SyntaxField>,
}

#[invariant(self.name.as_ref().is_none_or(|name| !name.is_empty()), "syntax field name must not be empty")]
#[expensive_invariant(syntax_field_data_is_valid(self.as_data()))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntaxField {
    #[serde(default)]
    pub name: Option<String>,
    pub value: SyntaxValue,
}

#[expensive_invariant(syntax_value_data_is_valid(self.as_data()))]
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
        new!(SyntaxValue::Null)
    }

    pub fn r#bool(value: bool) -> Self {
        new!(SyntaxValue::Bool { value: value })
    }

    pub fn integer(value: i64) -> Self {
        new!(SyntaxValue::Integer { value: value })
    }

    pub fn text(value: impl Into<String>) -> Self {
        new!(SyntaxValue::Text {
            value: value.into(),
        })
    }

    pub fn list(items: Vec<SyntaxValue>) -> Self {
        new!(SyntaxValue::List { items: items })
    }

    pub fn node(constructor: impl Into<String>, fields: Vec<SyntaxField>) -> Self {
        new!(SyntaxValue::Node {
            node: Box::new(new!(SyntaxNode {
                constructor: constructor.into(),
                fields: fields,
            })),
        })
    }

    pub fn word(word: WordWithModifiers) -> Self {
        new!(SyntaxValue::Word {
            word: Box::new(word),
        })
    }

    pub fn json(value: serde_json::Value) -> Self {
        new!(SyntaxValue::Json { value: value })
    }
}

pub fn syntax_values_equivalent(left: &SyntaxValue, right: &SyntaxValue) -> bool {
    match (left.as_data(), right.as_data()) {
        (data!(SyntaxValue::Null), data!(SyntaxValue::Null)) => true,
        (data!(SyntaxValue::Bool { value: left }), data!(SyntaxValue::Bool { value: right })) => {
            left == right
        }
        (
            data!(SyntaxValue::Integer { value: left }),
            data!(SyntaxValue::Integer { value: right }),
        ) => left == right,
        (data!(SyntaxValue::Text { value: left }), data!(SyntaxValue::Text { value: right })) => {
            left == right
        }
        (data!(SyntaxValue::List { items: left }), data!(SyntaxValue::List { items: right })) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| syntax_values_equivalent(left, right))
        }
        (data!(SyntaxValue::Node { node: left }), data!(SyntaxValue::Node { node: right })) => {
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
        (data!(SyntaxValue::Word { word: left }), data!(SyntaxValue::Word { word: right })) => {
            word_with_modifiers_syntax_eq(left, right)
        }
        (data!(SyntaxValue::Json { value: left }), data!(SyntaxValue::Json { value: right })) => {
            left == right
        }
        _ => false,
    }
}

fn lojban_text_data_is_valid(data: &LojbanTextData) -> bool {
    data.leading_free_modifiers
        .iter()
        .all(|modifier| free_modifier_data_is_valid(modifier.as_data()))
        && data
            .leading_connective
            .as_ref()
            .is_none_or(|connective| connective_data_is_valid(connective.as_data()))
        && data
            .paragraphs
            .iter()
            .all(|paragraph| paragraph_data_is_valid(paragraph.as_data()))
}

fn paragraph_data_is_valid(data: &ParagraphData) -> bool {
    data.free_modifiers
        .iter()
        .all(|modifier| free_modifier_data_is_valid(modifier.as_data()))
        && data
            .statements
            .iter()
            .all(|statement| paragraph_statement_data_is_valid(statement.as_data()))
}

fn paragraph_statement_data_is_valid(data: &ParagraphStatementData) -> bool {
    data.connective
        .as_ref()
        .is_none_or(|connective| connective_data_is_valid(connective.as_data()))
        && data
            .free_modifiers
            .iter()
            .all(|modifier| free_modifier_data_is_valid(modifier.as_data()))
        && data
            .statement
            .as_ref()
            .is_none_or(|statement| statement_data_is_valid(statement.as_data()))
}

fn statement_data_is_valid(data: &StatementData) -> bool {
    match data {
        data!(Statement::Fragment { fragment }) => fragment_data_is_valid(fragment.as_data()),
        data!(Statement::Placeholder) => true,
    }
}

fn fragment_data_is_valid(data: &FragmentData) -> bool {
    match data {
        data!(Fragment::Other { words: _ }) => true,
    }
}

fn free_modifier_data_is_valid(data: &FreeModifierData) -> bool {
    match data {
        data!(FreeModifier::Words { words: _ }) => true,
    }
}

fn connective_data_is_valid(data: &ConnectiveData) -> bool {
    match data {
        data!(Connective::Words { words: _ }) => true,
    }
}

fn syntax_parse_data_is_valid(data: &SyntaxParseData) -> bool {
    syntax_value_data_is_valid(data.parse_tree.as_data())
        && data
            .warnings
            .iter()
            .all(|warning| syntax_warning_data_is_valid(warning.as_data()))
}

fn syntax_warning_data_is_valid(data: &SyntaxWarningData) -> bool {
    match data {
        data!(SyntaxWarning::ExperimentalConstruct { construct, .. }) => !construct.is_empty(),
    }
}

fn syntax_tree_data_is_valid(data: &SyntaxTreeData) -> bool {
    syntax_value_data_is_valid(data.root.as_data())
}

fn syntax_node_data_is_valid(data: &SyntaxNodeData) -> bool {
    data.fields
        .iter()
        .all(|field| syntax_field_data_is_valid(field.as_data()))
}

fn syntax_field_data_is_valid(data: &SyntaxFieldData) -> bool {
    syntax_value_data_is_valid(data.value.as_data())
}

fn syntax_value_data_is_valid(data: &SyntaxValueData) -> bool {
    match data {
        data!(SyntaxValue::Null)
        | data!(SyntaxValue::Bool { .. })
        | data!(SyntaxValue::Integer { .. })
        | data!(SyntaxValue::Text { .. })
        | data!(SyntaxValue::Word { .. })
        | data!(SyntaxValue::Json { .. }) => true,
        data!(SyntaxValue::List { items }) => items
            .iter()
            .all(|item| syntax_value_data_is_valid(item.as_data())),
        data!(SyntaxValue::Node { node }) => syntax_node_data_is_valid(node.as_data()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_value_validity_rejects_empty_constructor() {
        let error = bityzba::try_new!(SyntaxNode {
            constructor: String::new(),
            fields: Vec::new(),
        })
        .expect_err("empty constructor should violate syntax node invariant");

        assert!(
            error
                .to_string()
                .contains("syntax constructor must not be empty")
        );
    }

    #[test]
    fn syntax_field_rejects_empty_name() {
        let error = bityzba::try_new!(SyntaxField {
            name: Some(String::new()),
            value: SyntaxValue::null(),
        })
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
