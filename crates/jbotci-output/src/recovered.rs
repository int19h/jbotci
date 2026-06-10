//! Structural renderers for recovered parse trees.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_morphology::PhonemeRenderOptions;
use jbotci_tree::{FieldRef, TreeVisitor};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::json::{self, JsonEntry};
use crate::tree::{self, TreeEntry, TreeNode, TreeValue};
use crate::{BracketRenderOptions, OutputError, TreeRenderOptions, brackets, sexpr};

#[requires(true)]
#[ensures(true)]
pub(crate) fn morphology_json_value(
    words: &[jbotci_morphology::tree::recovered::WordLike],
    phonemes: PhonemeRenderOptions,
) -> Result<Value, OutputError> {
    Ok(Value::Array(
        words
            .iter()
            .map(|word_like| recovered_morphology_json_value(word_like, phonemes))
            .collect(),
    ))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn syntax_json_value(
    syntax: &jbotci_syntax::tree::recovered::TextSyntax,
    options: crate::JsonRenderOptions,
) -> Result<Value, OutputError> {
    let mut visitor = RecoveredSyntaxJsonBuilder::new(options);
    jbotci_syntax::tree::recovered::TreeNode::visit_in_order(syntax, &mut visitor);
    Ok(visitor.finish())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(crate) fn morphology_tree_with_options(
    words: &[jbotci_morphology::tree::recovered::WordLike],
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    let value = TreeValue::Collection(
        words
            .iter()
            .map(|word_like| recovered_morphology_tree_value(word_like, source, options))
            .collect(),
    );
    Ok(tree::render_plain_tree_value_with_options(value, options))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(crate) fn syntax_tree_with_options(
    syntax: &jbotci_syntax::tree::recovered::TextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    let mut visitor = RecoveredSyntaxTreeBuilder::new(source, options);
    jbotci_syntax::tree::recovered::TreeNode::visit_in_order(syntax, &mut visitor);
    Ok(tree::render_plain_tree_value_with_options(
        visitor.finish(),
        options,
    ))
}

#[requires(true)]
#[ensures(words.is_empty() || ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(crate) fn morphology_brackets(
    words: &[jbotci_morphology::tree::recovered::WordLike],
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    let context = brackets::BracketContext { source, options };
    let sexpr = sexpr::node(
        words
            .iter()
            .map(|word_like| recovered_morphology_word_like_brackets(word_like, &context))
            .collect(),
    );
    Ok(sexpr::render_bracketed_with_options(
        &sexpr::flatten(sexpr),
        options,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(crate) fn syntax_brackets(
    syntax: &jbotci_syntax::tree::recovered::TextSyntax,
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    let context = brackets::BracketContext { source, options };
    let sexpr = recovered_text_syntax_brackets(syntax, &context);
    Ok(sexpr::render_bracketed_with_options(
        &sexpr::flatten(sexpr),
        options,
    ))
}

type MorphologySlot<T> = jbotci_morphology::tree::recovered::Recovered<T>;
type SyntaxSlot<T> = jbotci_syntax::tree::recovered::Recovered<T>;

#[requires(true)]
#[ensures(true)]
fn recovered_morphology_word_like_brackets(
    value: &jbotci_morphology::tree::recovered::WordLike,
    source: &brackets::BracketContext<'_>,
) -> sexpr::SExpr {
    if let Ok(valid) = value.clone().try_into_valid() {
        return brackets::word_like_brackets(&valid, source);
    }

    match value {
        jbotci_morphology::tree::recovered::WordLike::PlainWord(word) => {
            recovered_morphology_word_brackets(word, source)
        }
        jbotci_morphology::tree::recovered::WordLike::QuotedWord { zo, word } => sexpr::node(vec![
            recovered_morphology_word_brackets(zo, source),
            recovered_morphology_word_brackets(word, source),
        ]),
        jbotci_morphology::tree::recovered::WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        } => sexpr::node(vec![
            recovered_morphology_word_brackets(zoi, source),
            recovered_morphology_word_brackets(opening_delimiter, source),
            recovered_morphology_verbatim_brackets(quoted_text),
            recovered_morphology_word_brackets(closing_delimiter, source),
        ]),
        jbotci_morphology::tree::recovered::WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        } => {
            let mut children = vec![recovered_morphology_word_brackets(lohu, source)];
            children.extend(
                quoted_words
                    .iter()
                    .map(|word| recovered_morphology_word_brackets(word, source)),
            );
            children.push(recovered_morphology_word_brackets(lehu, source));
            sexpr::node(children)
        }
        jbotci_morphology::tree::recovered::WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        } => sexpr::node(vec![
            recovered_morphology_word_brackets(marker, source),
            recovered_morphology_verbatim_brackets(quoted_text),
        ]),
        jbotci_morphology::tree::recovered::WordLike::LerfuWord { base, bu } => sexpr::node(vec![
            recovered_morphology_word_like_slot_brackets(base, source),
            recovered_morphology_word_brackets(bu, source),
        ]),
        jbotci_morphology::tree::recovered::WordLike::ZeiCompound { left, zei, right } => {
            sexpr::node(vec![
                recovered_morphology_word_like_slot_brackets(left, source),
                recovered_morphology_word_brackets(zei, source),
                recovered_morphology_word_brackets(right, source),
            ])
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_morphology_word_like_slot_brackets(
    value: &MorphologySlot<jbotci_morphology::tree::recovered::WordLike>,
    source: &brackets::BracketContext<'_>,
) -> sexpr::SExpr {
    match value {
        jbotci_tree::Recovered::Valid(word_like) => {
            recovered_morphology_word_like_brackets(word_like, source)
        }
        jbotci_tree::Recovered::Error(item) => recovery_error_bracket_leaf(item),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_morphology_word_brackets(
    value: &MorphologySlot<jbotci_morphology::tree::recovered::Word>,
    source: &brackets::BracketContext<'_>,
) -> sexpr::SExpr {
    match value {
        jbotci_tree::Recovered::Valid(word) => match word.clone().try_into_valid() {
            Ok(valid) => brackets::word_leaf(&valid, source),
            Err(error) => recovery_error_bracket_leaf(&error.item),
        },
        jbotci_tree::Recovered::Error(item) => recovery_error_bracket_leaf(item),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_morphology_verbatim_brackets(
    value: &MorphologySlot<jbotci_morphology::tree::recovered::Verbatim>,
) -> sexpr::SExpr {
    match value {
        jbotci_tree::Recovered::Valid(verbatim) => match verbatim.clone().try_into_valid() {
            Ok(valid) => brackets::quoted_text_leaf(&valid),
            Err(error) => recovery_error_bracket_leaf(&error.item),
        },
        jbotci_tree::Recovered::Error(item) => recovery_error_bracket_leaf(item),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_text_syntax_brackets(
    value: &jbotci_syntax::tree::recovered::TextSyntax,
    source: &brackets::BracketContext<'_>,
) -> sexpr::SExpr {
    let mut children = Vec::new();
    children.extend(
        value
            .leading_nai
            .iter()
            .map(|word| recovered_syntax_token_brackets(word, source)),
    );
    children.extend(
        value
            .leading_cmevla
            .iter()
            .map(|word| recovered_syntax_token_brackets(word, source)),
    );
    children.extend(
        value
            .leading_indicators
            .iter()
            .map(recovered_syntax_fallback_slot_brackets),
    );
    children.extend(
        value
            .leading_free_modifiers
            .iter()
            .map(recovered_syntax_fallback_slot_brackets),
    );
    if let Some(connective) = &value.leading_connective {
        children.push(recovered_syntax_fallback_slot_brackets(connective));
    }
    children.extend(
        value
            .paragraphs
            .iter()
            .map(|paragraph| recovered_syntax_paragraph_slot_brackets(paragraph, source)),
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn recovered_syntax_paragraph_slot_brackets(
    value: &SyntaxSlot<jbotci_syntax::tree::recovered::ParagraphSyntax>,
    source: &brackets::BracketContext<'_>,
) -> sexpr::SExpr {
    match value {
        jbotci_tree::Recovered::Valid(paragraph) => match paragraph.clone().try_into_valid() {
            Ok(valid) => brackets::paragraph(&valid, source),
            Err(_) => recovered_syntax_paragraph_brackets(paragraph, source),
        },
        jbotci_tree::Recovered::Error(item) => recovery_error_bracket_leaf(item),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_syntax_paragraph_brackets(
    value: &jbotci_syntax::tree::recovered::ParagraphSyntax,
    source: &brackets::BracketContext<'_>,
) -> sexpr::SExpr {
    let mut children = Vec::new();
    if let Some(i) = &value.i {
        children.push(recovered_syntax_token_brackets(i, source));
    }
    children.extend(
        value
            .niho
            .iter()
            .map(|word| recovered_syntax_token_brackets(word, source)),
    );
    children.extend(
        value
            .free_modifiers
            .iter()
            .map(recovered_syntax_fallback_slot_brackets),
    );
    children.extend(
        value
            .statements
            .iter()
            .map(|statement| recovered_syntax_paragraph_statement_slot_brackets(statement, source)),
    );
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn recovered_syntax_paragraph_statement_slot_brackets(
    value: &SyntaxSlot<jbotci_syntax::tree::recovered::ParagraphStatementSyntax>,
    source: &brackets::BracketContext<'_>,
) -> sexpr::SExpr {
    match value {
        jbotci_tree::Recovered::Valid(statement) => match statement.clone().try_into_valid() {
            Ok(valid) => brackets::paragraph_statement(&valid, source),
            Err(_) => recovered_syntax_paragraph_statement_brackets(statement, source),
        },
        jbotci_tree::Recovered::Error(item) => recovery_error_bracket_leaf(item),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_syntax_paragraph_statement_brackets(
    value: &jbotci_syntax::tree::recovered::ParagraphStatementSyntax,
    source: &brackets::BracketContext<'_>,
) -> sexpr::SExpr {
    let mut children = Vec::new();
    if let Some(i) = &value.i {
        children.push(recovered_syntax_token_brackets(i, source));
    }
    if let Some(connective) = &value.connective {
        children.push(recovered_syntax_fallback_slot_brackets(connective));
    }
    children.extend(
        value
            .free_modifiers
            .iter()
            .map(recovered_syntax_fallback_slot_brackets),
    );
    if let Some(statement) = &value.statement {
        children.push(recovered_syntax_statement_slot_brackets(statement, source));
    }
    sexpr::node(children)
}

#[requires(true)]
#[ensures(true)]
fn recovered_syntax_statement_slot_brackets(
    value: &SyntaxSlot<jbotci_syntax::tree::recovered::StatementSyntax>,
    source: &brackets::BracketContext<'_>,
) -> sexpr::SExpr {
    match value {
        jbotci_tree::Recovered::Valid(statement) => match statement.clone().try_into_valid() {
            Ok(valid) => brackets::statement_syntax(&valid, source),
            Err(_) => recovered_syntax_source_words_brackets(value, source),
        },
        jbotci_tree::Recovered::Error(item) => recovery_error_bracket_leaf(item),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_syntax_source_words_brackets<'source, T>(
    value: &T,
    source: &'source brackets::BracketContext<'source>,
) -> sexpr::SExpr
where
    T: jbotci_syntax::tree::recovered::TreeNode,
{
    let mut visitor = RecoveredSyntaxWordBracketVisitor {
        source,
        children: Vec::new(),
    };
    jbotci_syntax::tree::recovered::TreeNode::visit_in_order(value, &mut visitor);
    sexpr::node(visitor.children)
}

#[derive(Debug)]
#[invariant(true)]
struct RecoveredSyntaxWordBracketVisitor<'source> {
    source: &'source brackets::BracketContext<'source>,
    children: Vec<sexpr::SExpr>,
}

impl<'tree> TreeVisitor<'tree> for RecoveredSyntaxWordBracketVisitor<'_> {
    type Node = jbotci_syntax::tree::recovered::NodeRef<'tree>;
    type Atom = jbotci_syntax::tree::recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let value = match atom {
            jbotci_syntax::tree::recovered::AtomRef::Token(token) => {
                brackets::word(token, self.source)
            }
            jbotci_syntax::tree::recovered::AtomRef::Word(word) => {
                brackets::word_leaf(word, self.source)
            }
        };
        self.children.push(value);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: Serialize>(&mut self, item: &'tree E) {
        self.children.push(recovery_error_bracket_leaf(item));
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_syntax_token_brackets(
    value: &SyntaxSlot<jbotci_syntax::Token>,
    source: &brackets::BracketContext<'_>,
) -> sexpr::SExpr {
    match value {
        jbotci_tree::Recovered::Valid(token) => brackets::word(token, source),
        jbotci_tree::Recovered::Error(item) => recovery_error_bracket_leaf(item),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_syntax_fallback_slot_brackets<T: Serialize>(value: &SyntaxSlot<T>) -> sexpr::SExpr {
    match value {
        jbotci_tree::Recovered::Valid(value) => {
            bracket_sexpr(&recovered_json_by_serde(value, false))
        }
        jbotci_tree::Recovered::Error(item) => recovery_error_bracket_leaf(item),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_morphology_tree_value(
    word_like: &jbotci_morphology::tree::recovered::WordLike,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut visitor = RecoveredMorphologyTreeBuilder::new(source, options);
    jbotci_morphology::tree::recovered::TreeNode::visit_in_order(word_like, &mut visitor);
    visitor.finish()
}

#[requires(true)]
#[ensures(true)]
fn recovered_morphology_json_value(
    word_like: &jbotci_morphology::tree::recovered::WordLike,
    phonemes: PhonemeRenderOptions,
) -> Value {
    let mut visitor = RecoveredMorphologyJsonBuilder::new(phonemes);
    jbotci_morphology::tree::recovered::TreeNode::visit_in_order(word_like, &mut visitor);
    visitor.finish()
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Node => true)]
#[invariant(::Field => true)]
#[invariant(::Collection => true)]
enum RecoveredTreeFrame {
    Node {
        constructor: &'static str,
        entries: Vec<TreeEntry>,
    },
    Field {
        name: Option<&'static str>,
        primary: bool,
        values: Vec<TreeValue>,
        nested_entries: Vec<TreeEntry>,
    },
    Collection {
        items: Vec<TreeValue>,
    },
}

#[derive(Debug)]
#[invariant(true)]
struct RecoveredMorphologyTreeBuilder<'source> {
    source: &'source str,
    options: TreeRenderOptions,
    stack: Vec<RecoveredTreeFrame>,
    root: Option<TreeValue>,
}

impl<'source> RecoveredMorphologyTreeBuilder<'source> {
    #[requires(true)]
    #[ensures(ret.source == source)]
    fn new(source: &'source str, options: TreeRenderOptions) -> Self {
        Self {
            source,
            options,
            stack: Vec::new(),
            root: None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> TreeValue {
        self.root
            .expect("recovered morphology tree walk produced a root")
    }
}

impl<'tree> TreeVisitor<'tree> for RecoveredMorphologyTreeBuilder<'_> {
    type Node = jbotci_morphology::tree::recovered::NodeRef<'tree>;
    type Atom = jbotci_morphology::tree::recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.stack.push(RecoveredTreeFrame::Node {
            constructor: node.constructor_name(),
            entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        let Some(RecoveredTreeFrame::Node {
            constructor,
            entries,
        }) = self.stack.pop()
        else {
            panic!("recovered morphology tree walker exited a node without entering it");
        };
        let value = tree::morphology_node_value(constructor, &entries, self.options).unwrap_or(
            TreeValue::Node(TreeNode {
                constructor,
                entries,
            }),
        );
        push_tree_value(&mut self.stack, &mut self.root, value);
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: FieldRef) {
        self.stack.push(RecoveredTreeFrame::Field {
            name: field.name,
            primary: field.primary,
            values: Vec::new(),
            nested_entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: FieldRef) {
        exit_tree_field(&mut self.stack, &mut self.root, |_| false);
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.stack
            .push(RecoveredTreeFrame::Collection { items: Vec::new() });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        exit_tree_sequence(&mut self.stack, &mut self.root);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let value = match atom {
            jbotci_morphology::tree::recovered::AtomRef::Phonemes(phonemes) => {
                TreeValue::Text(phonemes.render(self.options.phonemes))
            }
            jbotci_morphology::tree::recovered::AtomRef::String(text) => {
                TreeValue::Text(text.clone())
            }
            jbotci_morphology::tree::recovered::AtomRef::SourceSpan(span) => {
                tree::source_span_value(span)
            }
        };
        push_tree_value(&mut self.stack, &mut self.root, value);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: Serialize>(&mut self, item: &'tree E) {
        push_tree_value(
            &mut self.stack,
            &mut self.root,
            recovery_error_tree_value(item, self.options.show_spans, Some(self.source)),
        );
    }
}

#[derive(Debug)]
#[invariant(true)]
struct RecoveredSyntaxTreeBuilder<'source> {
    source: &'source str,
    options: TreeRenderOptions,
    stack: Vec<RecoveredTreeFrame>,
    root: Option<TreeValue>,
}

impl<'source> RecoveredSyntaxTreeBuilder<'source> {
    #[requires(true)]
    #[ensures(ret.source == source)]
    fn new(source: &'source str, options: TreeRenderOptions) -> Self {
        Self {
            source,
            options,
            stack: Vec::new(),
            root: None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> TreeValue {
        self.root
            .expect("recovered syntax tree walk produced a root")
    }
}

impl<'tree> TreeVisitor<'tree> for RecoveredSyntaxTreeBuilder<'_> {
    type Node = jbotci_syntax::tree::recovered::NodeRef<'tree>;
    type Atom = jbotci_syntax::tree::recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.stack.push(RecoveredTreeFrame::Node {
            constructor: json::syntax_constructor_name(node.constructor_name()),
            entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        let Some(RecoveredTreeFrame::Node {
            constructor,
            entries,
        }) = self.stack.pop()
        else {
            panic!("recovered syntax tree walker exited a node without entering it");
        };
        push_tree_value(
            &mut self.stack,
            &mut self.root,
            TreeValue::Node(TreeNode {
                constructor,
                entries,
            }),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: FieldRef) {
        self.stack.push(RecoveredTreeFrame::Field {
            name: field.name,
            primary: field.primary,
            values: Vec::new(),
            nested_entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: FieldRef) {
        exit_tree_field(
            &mut self.stack,
            &mut self.root,
            recovered_syntax_primary_field,
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.stack
            .push(RecoveredTreeFrame::Collection { items: Vec::new() });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        exit_tree_sequence(&mut self.stack, &mut self.root);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let value = match atom {
            jbotci_syntax::tree::recovered::AtomRef::Token(token) => {
                tree::with_indicators_tree_value(token.as_indicators(), self.source, self.options)
            }
            jbotci_syntax::tree::recovered::AtomRef::Word(word) => {
                tree::word_tree_value(word, self.source, self.options)
            }
        };
        push_tree_value(&mut self.stack, &mut self.root, value);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: Serialize>(&mut self, item: &'tree E) {
        push_tree_value(
            &mut self.stack,
            &mut self.root,
            recovery_error_tree_value(item, self.options.show_spans, Some(self.source)),
        );
    }
}

#[requires(true)]
#[ensures(true)]
fn push_tree_value(
    stack: &mut [RecoveredTreeFrame],
    root: &mut Option<TreeValue>,
    value: TreeValue,
) {
    match stack.last_mut() {
        Some(RecoveredTreeFrame::Field { values, .. }) => values.push(value),
        Some(RecoveredTreeFrame::Collection { items }) => items.push(value),
        Some(RecoveredTreeFrame::Node { entries, .. }) => {
            entries.push(TreeEntry { label: None, value });
        }
        None => *root = Some(value),
    }
}

#[requires(true)]
#[ensures(true)]
fn exit_tree_field(
    stack: &mut Vec<RecoveredTreeFrame>,
    root: &mut Option<TreeValue>,
    extra_primary: impl Fn(Option<&'static str>) -> bool,
) {
    let Some(RecoveredTreeFrame::Field {
        name,
        primary,
        values,
        nested_entries,
    }) = stack.pop()
    else {
        panic!("recovered tree walker exited a field without entering it");
    };
    if values.is_empty() && nested_entries.is_empty() {
        return;
    }
    if primary || name.is_none() || extra_primary(name) {
        for value in values {
            push_tree_value_in_order(stack, root, value);
        }
    } else {
        let value = if values.len() == 1 {
            values.into_iter().next().expect("length checked")
        } else {
            TreeValue::Collection(values)
        };
        push_tree_entry(stack, TreeEntry { label: name, value });
    }
    for entry in nested_entries {
        push_tree_entry(stack, entry);
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_syntax_primary_field(name: Option<&'static str>) -> bool {
    matches!(
        name,
        Some(
            "base"
                | "bridi_tail"
                | "cmavo"
                | "expression"
                | "expressions"
                | "first"
                | "inner"
                | "inner_expression"
                | "inner_operator"
                | "inner_selbri"
                | "inner_statement"
                | "inner_subbridi"
                | "inner_sumti"
                | "inner_term"
                | "inner_unit"
                | "mekso"
                | "mekso_operator"
                | "number"
                | "paragraphs"
                | "parts"
                | "selbri"
                | "statement"
                | "statements"
                | "subbridi"
                | "sumti"
                | "tag"
                | "tanru_unit"
                | "text"
        )
    )
}

#[requires(true)]
#[ensures(true)]
fn push_tree_value_in_order(
    stack: &mut [RecoveredTreeFrame],
    root: &mut Option<TreeValue>,
    value: TreeValue,
) {
    match value {
        TreeValue::Collection(items) => {
            for item in items {
                push_tree_value(stack, root, item);
            }
        }
        value => push_tree_value(stack, root, value),
    }
}

#[requires(true)]
#[ensures(true)]
fn push_tree_entry(stack: &mut [RecoveredTreeFrame], entry: TreeEntry) {
    match stack.last_mut() {
        Some(RecoveredTreeFrame::Node { entries, .. }) => entries.push(entry),
        Some(RecoveredTreeFrame::Field { nested_entries, .. }) => nested_entries.push(entry),
        Some(RecoveredTreeFrame::Collection { .. }) | None => {
            panic!("recovered tree labelled field has no containing node")
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn exit_tree_sequence(stack: &mut Vec<RecoveredTreeFrame>, root: &mut Option<TreeValue>) {
    let Some(RecoveredTreeFrame::Collection { items }) = stack.pop() else {
        panic!("recovered tree walker exited a collection without entering it");
    };
    if !items.is_empty() {
        push_tree_value(stack, root, TreeValue::Collection(items));
    }
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
#[invariant(::Node => true)]
#[invariant(::Field => true)]
#[invariant(::Sequence => true)]
enum RecoveredJsonFrame<N> {
    Node {
        node: N,
        entries: Vec<JsonEntry>,
    },
    Field {
        name: Option<&'static str>,
        values: Vec<Value>,
        nested_entries: Vec<JsonEntry>,
    },
    Sequence {
        items: Vec<Value>,
    },
}

#[derive(Debug)]
#[invariant(true)]
struct RecoveredMorphologyJsonBuilder {
    phonemes: PhonemeRenderOptions,
    stack: Vec<RecoveredJsonFrame<MorphologyJsonNodeInfo>>,
    root: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct MorphologyJsonNodeInfo {
    constructor: &'static str,
    variant: bool,
}

impl RecoveredMorphologyJsonBuilder {
    #[requires(true)]
    #[ensures(ret.phonemes == phonemes)]
    fn new(phonemes: PhonemeRenderOptions) -> Self {
        Self {
            phonemes,
            stack: Vec::new(),
            root: None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> Value {
        self.root
            .expect("recovered morphology JSON walk produced a root")
    }
}

impl<'tree> TreeVisitor<'tree> for RecoveredMorphologyJsonBuilder {
    type Node = jbotci_morphology::tree::recovered::NodeRef<'tree>;
    type Atom = jbotci_morphology::tree::recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.stack.push(RecoveredJsonFrame::Node {
            node: MorphologyJsonNodeInfo {
                constructor: node.constructor_name(),
                variant: node.is_variant(),
            },
            entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        let Some(RecoveredJsonFrame::Node { node, entries }) = self.stack.pop() else {
            panic!("recovered morphology JSON walker exited a node without entering it");
        };
        push_json_value(
            &mut self.stack,
            &mut self.root,
            json::node_value(node.constructor, node.variant, entries),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: FieldRef) {
        self.stack.push(RecoveredJsonFrame::Field {
            name: field.name,
            values: Vec::new(),
            nested_entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: FieldRef) {
        exit_json_field(&mut self.stack, &mut self.root);
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.stack
            .push(RecoveredJsonFrame::Sequence { items: Vec::new() });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        exit_json_sequence(&mut self.stack, &mut self.root);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let value = match atom {
            jbotci_morphology::tree::recovered::AtomRef::Phonemes(phonemes) => {
                Value::String(phonemes.render(self.phonemes))
            }
            jbotci_morphology::tree::recovered::AtomRef::String(text) => {
                Value::String(text.clone())
            }
            jbotci_morphology::tree::recovered::AtomRef::SourceSpan(span) => span_json_value(span),
        };
        push_json_value(&mut self.stack, &mut self.root, value);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: Serialize>(&mut self, item: &'tree E) {
        push_json_value(
            &mut self.stack,
            &mut self.root,
            recovery_error_json_value(item),
        );
    }
}

#[derive(Debug)]
#[invariant(true)]
struct RecoveredSyntaxJsonBuilder {
    options: crate::JsonRenderOptions,
    stack: Vec<RecoveredJsonFrame<SyntaxJsonNodeInfo>>,
    root: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct SyntaxJsonNodeInfo {
    constructor: &'static str,
    variant: bool,
}

impl RecoveredSyntaxJsonBuilder {
    #[requires(true)]
    #[ensures(ret.options == options)]
    fn new(options: crate::JsonRenderOptions) -> Self {
        Self {
            options,
            stack: Vec::new(),
            root: None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> Value {
        self.root
            .expect("recovered syntax JSON walk produced a root")
    }
}

impl<'tree> TreeVisitor<'tree> for RecoveredSyntaxJsonBuilder {
    type Node = jbotci_syntax::tree::recovered::NodeRef<'tree>;
    type Atom = jbotci_syntax::tree::recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.stack.push(RecoveredJsonFrame::Node {
            node: SyntaxJsonNodeInfo {
                constructor: json::syntax_constructor_name(node.constructor_name()),
                variant: node.is_variant(),
            },
            entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        let Some(RecoveredJsonFrame::Node { node, entries }) = self.stack.pop() else {
            panic!("recovered syntax JSON walker exited a node without entering it");
        };
        push_json_value(
            &mut self.stack,
            &mut self.root,
            json::node_value(node.constructor, node.variant, entries),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: FieldRef) {
        self.stack.push(RecoveredJsonFrame::Field {
            name: field.name,
            values: Vec::new(),
            nested_entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: FieldRef) {
        exit_json_field(&mut self.stack, &mut self.root);
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.stack
            .push(RecoveredJsonFrame::Sequence { items: Vec::new() });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        exit_json_sequence(&mut self.stack, &mut self.root);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let value = match atom {
            jbotci_syntax::tree::recovered::AtomRef::Token(token) => {
                json::with_indicators_value(token.as_indicators(), self.options.phonemes)
            }
            jbotci_syntax::tree::recovered::AtomRef::Word(word) => {
                json::morphology_word_value(word, self.options.phonemes)
            }
        };
        push_json_value(&mut self.stack, &mut self.root, value);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: Serialize>(&mut self, item: &'tree E) {
        push_json_value(
            &mut self.stack,
            &mut self.root,
            recovery_error_json_value(item),
        );
    }
}

#[requires(true)]
#[ensures(true)]
fn push_json_value<N>(stack: &mut [RecoveredJsonFrame<N>], root: &mut Option<Value>, value: Value) {
    match stack.last_mut() {
        Some(RecoveredJsonFrame::Node { entries, .. }) => {
            entries.push(JsonEntry { label: None, value });
        }
        Some(RecoveredJsonFrame::Field { values, .. }) => values.push(value),
        Some(RecoveredJsonFrame::Sequence { items }) => items.push(value),
        None => *root = Some(value),
    }
}

#[requires(true)]
#[ensures(true)]
fn exit_json_field<N>(stack: &mut Vec<RecoveredJsonFrame<N>>, root: &mut Option<Value>) {
    let Some(RecoveredJsonFrame::Field {
        name,
        values,
        nested_entries,
    }) = stack.pop()
    else {
        panic!("recovered JSON walker exited a field without entering it");
    };
    let Some(value) = json::field_value(values, nested_entries) else {
        return;
    };
    if let Some(label) = name {
        push_json_entry(
            stack,
            JsonEntry {
                label: Some(label),
                value,
            },
        );
    } else {
        push_json_value(stack, root, value);
    }
}

#[requires(true)]
#[ensures(true)]
fn push_json_entry<N>(stack: &mut [RecoveredJsonFrame<N>], entry: JsonEntry) {
    match stack.last_mut() {
        Some(RecoveredJsonFrame::Node { entries, .. }) => entries.push(entry),
        Some(RecoveredJsonFrame::Field { nested_entries, .. }) => nested_entries.push(entry),
        Some(RecoveredJsonFrame::Sequence { .. }) | None => {
            panic!("recovered JSON walker produced a labelled field outside a node or field")
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn exit_json_sequence<N>(stack: &mut Vec<RecoveredJsonFrame<N>>, root: &mut Option<Value>) {
    let Some(RecoveredJsonFrame::Sequence { items }) = stack.pop() else {
        panic!("recovered JSON walker exited a sequence without entering it");
    };
    if !items.is_empty() {
        push_json_value(stack, root, Value::Array(items));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct RecoveryRenderItem {
    text: Option<String>,
    expected: Vec<String>,
    diagnostic_code: String,
    span: Option<RecoveryRenderSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct RecoveryRenderSpan {
    byte_start: usize,
    byte_end: usize,
    char_start: usize,
    char_end: usize,
}

#[requires(true)]
#[ensures(true)]
fn recovery_render_item<E: Serialize>(item: &E, source: Option<&str>) -> RecoveryRenderItem {
    let value = serde_json::to_value(item).unwrap_or(Value::Null);
    let object = value.as_object();
    let text = object
        .and_then(|object| object.get("text"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let expected = object
        .and_then(|object| object.get("expected"))
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| vec!["valid syntax".to_owned()]);
    let diagnostic_code = object
        .and_then(|object| object.get("diagnostic_code"))
        .and_then(Value::as_str)
        .unwrap_or("recovery.error")
        .to_owned();
    let span = object
        .and_then(|object| object.get("span"))
        .and_then(|span| recovery_render_span(span, source));
    RecoveryRenderItem {
        text,
        expected,
        diagnostic_code,
        span,
    }
}

#[requires(true)]
#[ensures(true)]
fn recovery_render_span(value: &Value, source: Option<&str>) -> Option<RecoveryRenderSpan> {
    if let Some(values) = value.as_array() {
        let [char_start, char_end] = values.as_slice() else {
            return None;
        };
        let char_start = char_start.as_u64()?.try_into().ok()?;
        let char_end = char_end.as_u64()?.try_into().ok()?;
        if let Some(source) = source
            && let Ok(span) = jbotci_diagnostics::source_span_from_char_offsets(
                None, source, char_start, char_end,
            )
        {
            return Some(RecoveryRenderSpan {
                byte_start: span.byte_start,
                byte_end: span.byte_end,
                char_start: span.char_start,
                char_end: span.char_end,
            });
        }
        return Some(RecoveryRenderSpan {
            byte_start: char_start,
            byte_end: char_end,
            char_start,
            char_end,
        });
    }

    let object = value.as_object()?;
    Some(RecoveryRenderSpan {
        byte_start: object.get("byte_start")?.as_u64()?.try_into().ok()?,
        byte_end: object.get("byte_end")?.as_u64()?.try_into().ok()?,
        char_start: object.get("char_start")?.as_u64()?.try_into().ok()?,
        char_end: object.get("char_end")?.as_u64()?.try_into().ok()?,
    })
}

#[requires(true)]
#[ensures(true)]
fn recovery_error_tree_value<E: Serialize>(
    item: &E,
    include_span: bool,
    source: Option<&str>,
) -> TreeValue {
    let item = recovery_render_item(item, source);
    TreeValue::Error {
        text: item.text.unwrap_or_default(),
        span: if include_span {
            item.span.map(|span| (span.char_start, span.char_end))
        } else {
            None
        },
    }
}

#[requires(true)]
#[ensures(matches!(ret, Value::Object(_)))]
fn recovery_error_json_value<E: Serialize>(item: &E) -> Value {
    let item = recovery_render_item(item, None);
    let mut fields = Map::new();
    if let Some(span) = item.span {
        fields.insert(
            "span".to_owned(),
            Value::Array(vec![span.char_start.into(), span.char_end.into()]),
        );
    }
    fields.insert(
        "text".to_owned(),
        item.text.map(Value::String).unwrap_or(Value::Null),
    );
    fields.insert(
        "expected".to_owned(),
        Value::Array(item.expected.into_iter().map(Value::String).collect()),
    );
    fields.insert(
        "diagnostic_code".to_owned(),
        Value::String(item.diagnostic_code),
    );
    json::constructor_value("Error", Value::Object(fields))
}

#[requires(true)]
#[ensures(true)]
fn recovery_error_bracket_leaf<E: Serialize>(item: &E) -> sexpr::SExpr {
    let item = recovery_render_item(item, None);
    sexpr::error_leaf(item.text.unwrap_or_default())
}

#[requires(true)]
#[ensures(true)]
fn span_json_value(span: &jbotci_source::SourceSpan) -> Value {
    Value::Array(vec![span.char_start.into(), span.char_end.into()])
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn bracket_value(value: &Value, options: BracketRenderOptions) -> String {
    let sexpr = bracket_sexpr(value);
    sexpr::render_bracketed_with_options(&sexpr::flatten(sexpr), options)
}

#[requires(true)]
#[ensures(true)]
fn bracket_sexpr(value: &Value) -> sexpr::SExpr {
    match value {
        Value::Object(object) if object.len() == 1 && object.contains_key("Error") => {
            let text = object
                .get("Error")
                .and_then(Value::as_object)
                .and_then(|object| object.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("");
            sexpr::leaf(format!("‼{text}‼"))
        }
        Value::Object(object) if object.get("kind").and_then(Value::as_str) == Some("error") => {
            let text = object
                .get("value")
                .and_then(Value::as_object)
                .and_then(|object| object.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("");
            sexpr::leaf(format!("‼{text}‼"))
        }
        Value::Object(object) if object.get("kind").and_then(Value::as_str) == Some("valid") => {
            object
                .get("value")
                .map(bracket_sexpr)
                .unwrap_or_else(|| sexpr::node(Vec::new()))
        }
        Value::Object(object) if object.len() == 1 => {
            let (constructor, child) = object.iter().next().expect("checked object length");
            sexpr::node(vec![sexpr::leaf(constructor.clone()), bracket_sexpr(child)])
        }
        Value::Object(object) => sexpr::node(
            object
                .iter()
                .map(|(key, value)| {
                    sexpr::node(vec![sexpr::leaf(key.clone()), bracket_sexpr(value)])
                })
                .collect(),
        ),
        Value::Array(items) => sexpr::node(items.iter().map(bracket_sexpr).collect()),
        Value::String(text) => sexpr::leaf(text.clone()),
        Value::Number(number) => sexpr::leaf(number.to_string()),
        Value::Bool(value) => sexpr::leaf(value.to_string()),
        Value::Null => sexpr::node(Vec::new()),
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_json_by_serde<T: Serialize + ?Sized>(value: &T, include_spans: bool) -> Value {
    serde_json::to_value(value)
        .map(|value| collapse_recovered_serde_value(value, include_spans))
        .unwrap_or(Value::Null)
}

#[requires(true)]
#[ensures(true)]
fn collapse_recovered_serde_value(value: Value, include_spans: bool) -> Value {
    match value {
        Value::Object(mut object) => {
            if object.get("kind").and_then(Value::as_str) == Some("valid") {
                return object
                    .remove("value")
                    .map(|value| collapse_recovered_serde_value(value, include_spans))
                    .unwrap_or(Value::Null);
            }
            if object.get("kind").and_then(Value::as_str) == Some("error") {
                return recovery_error_json_value_from_serde(object.remove("value"), include_spans);
            }
            Value::Object(
                object
                    .into_iter()
                    .filter(|(key, _)| include_spans || key != "span")
                    .map(|(key, value)| (key, collapse_recovered_serde_value(value, include_spans)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| collapse_recovered_serde_value(item, include_spans))
                .collect(),
        ),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => value,
    }
}

#[requires(true)]
#[ensures(matches!(ret, Value::Object(_)))]
fn recovery_error_json_value_from_serde(value: Option<Value>, include_span: bool) -> Value {
    let mut object = value
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    let mut fields = Map::new();
    if include_span && let Some(span) = object.remove("span") {
        fields.insert("span".to_owned(), span);
    }
    fields.insert(
        "text".to_owned(),
        object.remove("text").unwrap_or(Value::Null),
    );
    fields.insert(
        "expected".to_owned(),
        object
            .remove("expected")
            .unwrap_or_else(|| Value::Array(Vec::new())),
    );
    fields.insert(
        "diagnostic_code".to_owned(),
        object
            .remove("diagnostic_code")
            .unwrap_or_else(|| Value::String("recovery.error".to_owned())),
    );
    json::constructor_value("Error", Value::Object(fields))
}
