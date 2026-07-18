//! Structural renderers for recovered morphology and syntax results.

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, invariant, new, requires};
use jbotci_morphology::{MorphologyError, RecoveredMorphologySegmentation};
use jbotci_source::SourceSpan;
use jbotci_syntax::{RecoveredSyntaxParse, SyntaxError};
use jbotci_tree::{FieldRef, RecoveryItemState, RecoveryProjection, TreeVisitor};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::json;
use crate::tree::{self, RecoveryTreeError, TreeValue};
use crate::{
    BracketRenderOptions, BracketSourceConstruct, BracketSourceRange, JsonRenderOptions,
    OutputError, TreeRenderOptions, brackets, sexpr,
};

#[invariant(error_index < diagnostic_count, "recovery error indices must resolve to a diagnostic")]
#[invariant(!diagnostic_code.is_empty(), "recovery errors must identify their diagnostic")]
#[invariant(!expected.is_empty(), "recovery errors must describe what was expected")]
#[invariant(span.is_empty() == text.is_empty(), "zero-width recovery markers have no source text")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecoveryRenderItem {
    pub(crate) error_index: usize,
    pub(crate) diagnostic_count: usize,
    pub(crate) diagnostic_code: String,
    pub(crate) expected: Vec<String>,
    pub(crate) span: SourceSpan,
    pub(crate) text: String,
}

#[invariant(::Valid { .. } => true)]
#[invariant(::Error { item } => item.error_index < item.diagnostic_count)]
#[derive(Debug)]
enum RecoveredMorphologySequenceItem<'tree> {
    Valid {
        word: &'tree jbotci_morphology::WordLike,
    },
    Error {
        item: RecoveryRenderItem,
    },
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn recovery_item_from_span(
    error_index: usize,
    diagnostic_count: usize,
    diagnostic_code: String,
    expected: Vec<String>,
    span: SourceSpan,
    source: &str,
) -> Result<RecoveryRenderItem, OutputError> {
    let text = source
        .get(span.byte_start..span.byte_end)
        .ok_or_else(|| {
            OutputError::Recovery(format!(
                "recovery span {}..{} is outside the source text",
                span.byte_start, span.byte_end
            ))
        })?
        .to_owned();
    if error_index >= diagnostic_count {
        return Err(OutputError::Recovery(format!(
            "recovery error index {error_index} is outside {diagnostic_count} diagnostics"
        )));
    }
    if diagnostic_code.is_empty() || expected.is_empty() || span.is_empty() != text.is_empty() {
        return Err(OutputError::Recovery(
            "recovery item metadata violates renderer invariants".to_owned(),
        ));
    }
    Ok(new!(RecoveryRenderItem {
        error_index,
        diagnostic_count,
        diagnostic_code,
        expected,
        span,
        text,
    }))
}

#[requires(error_index < errors.len())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn morphology_recovery_item(
    error_index: usize,
    errors: &[MorphologyError],
    span: &SourceSpan,
    source: &str,
) -> Result<RecoveryRenderItem, OutputError> {
    let (diagnostic_code, expected) = match &errors[error_index] {
        MorphologyError::Invalid { kind, .. } => {
            (kind.code().to_owned(), vec![kind.message().to_owned()])
        }
        MorphologyError::UnterminatedZoiQuote { delimiter, .. } => (
            "morphology.unterminated-zoi-quote".to_owned(),
            vec![format!("closing delimiter `{delimiter}`")],
        ),
        MorphologyError::SourceSpan(_) => (
            "morphology.source-span".to_owned(),
            vec!["valid source span".to_owned()],
        ),
    };
    recovery_item_from_span(
        error_index,
        errors.len(),
        diagnostic_code,
        expected,
        span.clone(),
        source,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn syntax_recovery_item<E: RecoveryItemState + Serialize>(
    item: &E,
    errors: &[SyntaxError],
    source: &str,
) -> Result<RecoveryRenderItem, OutputError> {
    let error_index = item.recovery_error_index().ok_or_else(|| {
        OutputError::Recovery("syntax recovery item has no diagnostic index".to_owned())
    })?;
    let error = errors.get(error_index).ok_or_else(|| {
        OutputError::Recovery(format!(
            "syntax recovery error index {error_index} is outside {} diagnostics",
            errors.len()
        ))
    })?;
    let (diagnostic_code, expected) = match error {
        SyntaxError::NotImplemented => (
            "syntax.not-implemented".to_owned(),
            vec!["implemented syntax parser".to_owned()],
        ),
        SyntaxError::Parse { kind, expected, .. } => (
            kind.code().to_owned(),
            if expected.is_empty() {
                vec![kind.message().to_owned()]
            } else {
                expected.clone()
            },
        ),
    };
    let span = recovery_item_span(item).ok_or_else(|| {
        OutputError::Recovery("syntax recovery item has no source span".to_owned())
    })?;
    recovery_item_from_span(
        error_index,
        errors.len(),
        diagnostic_code,
        expected,
        span,
        source,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|span| span.byte_start <= span.byte_end && span.char_start <= span.char_end))]
fn recovery_item_span(item: &impl RecoveryItemState) -> Option<SourceSpan> {
    let mut result: Option<SourceSpan> = None;
    item.visit_source_spans(&mut |span| {
        result = Some(match result.take() {
            None => span.clone(),
            Some(existing) => SourceSpan::new(
                existing
                    .source_id
                    .clone()
                    .or_else(|| span.source_id.clone()),
                existing.byte_start.min(span.byte_start),
                existing.byte_end.max(span.byte_end),
                existing.char_start.min(span.char_start),
                existing.char_end.max(span.char_end),
            )
            .expect("unions of valid source spans remain ordered"),
        });
    });
    result
}

#[requires(recovered.errors.len() == recovered.error_regions.len())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn recovered_morphology_sequence<'tree>(
    recovered: &'tree RecoveredMorphologySegmentation,
    source: &str,
) -> Result<Vec<RecoveredMorphologySequenceItem<'tree>>, OutputError> {
    let mut words = recovered.words.iter().peekable();
    let mut errors = recovered.error_regions.iter().enumerate().peekable();
    let mut sequence = Vec::with_capacity(recovered.words.len() + recovered.errors.len());
    while words.peek().is_some() || errors.peek().is_some() {
        let word_start = words.peek().and_then(|word| {
            word.source_spans()
                .into_iter()
                .map(|span| span.byte_start)
                .min()
        });
        let error_start = errors.peek().map(|(_, span)| span.byte_start);
        if error_start.is_some() && (word_start.is_none() || error_start < word_start) {
            let (error_index, span) = errors.next().expect("peeked error exists");
            sequence.push(new!(RecoveredMorphologySequenceItem::Error {
                item: morphology_recovery_item(error_index, &recovered.errors, span, source)?,
            }));
        } else {
            sequence.push(new!(RecoveredMorphologySequenceItem::Valid {
                word: words.next().expect("peeked word exists"),
            }));
        }
    }
    Ok(sequence)
}

#[requires(true)]
#[ensures(recovered.words.is_empty() || ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn pretty_recovered_morphology_brackets_with_options(
    recovered: &RecoveredMorphologySegmentation,
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    if recovered.errors.is_empty() {
        return crate::pretty_morphology_brackets_with_options(&recovered.words, source, options);
    }
    let context = brackets::BracketContext { options };
    let children = recovered_morphology_sequence(recovered, source)?
        .into_iter()
        .map(|item| match item.into_data() {
            data!(RecoveredMorphologySequenceItem::Valid { word }) => {
                brackets::word_like_brackets(word, &context)
            }
            data!(RecoveredMorphologySequenceItem::Error { item }) => recovery_error_sexpr(&item),
        })
        .collect();
    let value = sexpr::flatten(sexpr::node(children));
    Ok(sexpr::render_bracketed_with_options(&value, options))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn pretty_recovered_morphology_tree_with_options(
    recovered: &RecoveredMorphologySegmentation,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    if recovered.errors.is_empty() {
        return crate::pretty_morphology_tree_with_options(&recovered.words, source, options);
    }
    let values = recovered_morphology_sequence(recovered, source)?
        .into_iter()
        .map(|item| match item.into_data() {
            data!(RecoveredMorphologySequenceItem::Valid { word }) => {
                tree::morphology_tree_value(word, source, options)
            }
            data!(RecoveredMorphologySequenceItem::Error { item }) => {
                recovery_error_tree_value(&item)
            }
        })
        .collect();
    Ok(tree::render_tree_value_with_options(
        &tree::collapse_value(TreeValue::Collection(values)),
        options,
        None,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn compact_recovered_morphology_json_string_with_options(
    recovered: &RecoveredMorphologySegmentation,
    source: &str,
    options: JsonRenderOptions,
) -> Result<String, OutputError> {
    if recovered.errors.is_empty() {
        return crate::compact_morphology_json_string_with_options(&recovered.words, options);
    }
    let items = recovered_morphology_sequence(recovered, source)?
        .into_iter()
        .map(|item| match item.into_data() {
            data!(RecoveredMorphologySequenceItem::Valid { word }) => {
                json::morphology_word_like_value(word, options.phonemes)
            }
            data!(RecoveredMorphologySequenceItem::Error { item }) => {
                recovery_error_json_value(&item)
            }
        })
        .collect();
    Ok(crate::format_compact_json_value(
        &Value::Array(items),
        0,
        options,
    ))
}

#[requires(indent.is_none_or(|indent| indent == 0))]
#[ensures(ret.as_ref().is_ok_and(|text| text.ends_with('\n')) || ret.is_err())]
pub fn pretty_recovered_morphology_raw(
    recovered: &RecoveredMorphologySegmentation,
    source: &str,
    indent: Option<usize>,
) -> Result<String, OutputError> {
    if recovered.errors.is_empty() {
        return Ok(debug_output(&recovered.words, indent));
    }
    Ok(debug_output(
        &recovered_morphology_sequence(recovered, source)?,
        indent,
    ))
}

#[requires(true)]
#[ensures(true)]
fn recovery_error_sexpr(item: &RecoveryRenderItem) -> sexpr::SExpr {
    sexpr::error_leaf_with_range(
        item.text.clone(),
        Some(new!(BracketSourceRange {
            byte_start: item.span.byte_start,
            byte_end: item.span.byte_end,
        })),
    )
}

#[requires(true)]
#[ensures(true)]
fn recovery_error_tree_value(item: &RecoveryRenderItem) -> TreeValue {
    TreeValue::Error {
        error: new!(RecoveryTreeError {
            text: item.text.clone(),
            span: Some((item.span.char_start, item.span.char_end)),
            error_index: item.error_index,
            diagnostic_count: item.diagnostic_count,
            diagnostic_code: item.diagnostic_code.clone(),
            expected: item.expected.clone(),
        }),
    }
}

#[requires(true)]
#[ensures(matches!(ret, Value::Object(_)))]
fn recovery_error_json_value(item: &RecoveryRenderItem) -> Value {
    let mut fields = Map::new();
    fields.insert(
        "span".to_owned(),
        Value::Array(vec![item.span.char_start.into(), item.span.char_end.into()]),
    );
    fields.insert("text".to_owned(), Value::String(item.text.clone()));
    fields.insert(
        "expected".to_owned(),
        Value::Array(item.expected.iter().cloned().map(Value::String).collect()),
    );
    fields.insert("error_index".to_owned(), item.error_index.into());
    fields.insert(
        "diagnostic_code".to_owned(),
        Value::String(item.diagnostic_code.clone()),
    );
    json::constructor_value("Error", Value::Object(fields))
}

#[requires(indent.is_none_or(|indent| indent == 0))]
#[ensures(ret.ends_with('\n'))]
fn debug_output(value: &impl std::fmt::Debug, indent: Option<usize>) -> String {
    if indent == Some(0) {
        format!("{value:?}\n")
    } else {
        format!("{value:#?}\n")
    }
}

#[invariant(true)]
#[derive(Debug)]
struct RecoveredBracketFrame {
    children: Vec<sexpr::SExpr>,
    constructs: Vec<BracketSourceConstruct>,
}

#[invariant(true)]
#[derive(Debug)]
struct RecoveredBracketBuilder<'source, 'errors> {
    context: brackets::BracketContext,
    source: &'source str,
    errors: &'errors [SyntaxError],
    stack: Vec<RecoveredBracketFrame>,
    root: Option<sexpr::SExpr>,
    render_error: Option<OutputError>,
    recovery_projection: RecoveryProjection,
}

impl<'source, 'errors> RecoveredBracketBuilder<'source, 'errors> {
    #[requires(true)]
    #[ensures(ret.source == source)]
    #[ensures(ret.errors.as_ptr() == errors.as_ptr() && ret.errors.len() == errors.len())]
    #[ensures(ret.stack.is_empty() && ret.root.is_none() && ret.render_error.is_none())]
    fn new(
        source: &'source str,
        errors: &'errors [SyntaxError],
        options: BracketRenderOptions,
    ) -> Self {
        Self {
            context: brackets::BracketContext { options },
            source,
            errors,
            stack: Vec::new(),
            root: None,
            render_error: None,
            recovery_projection: RecoveryProjection::default(),
        }
    }

    #[requires(self.stack.is_empty())]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn finish(self) -> Result<sexpr::SExpr, OutputError> {
        if let Some(error) = self.render_error {
            return Err(error);
        }
        self.root.ok_or_else(|| {
            OutputError::Recovery("recovered bracket walk produced no root".to_owned())
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn push(&mut self, value: sexpr::SExpr) {
        if let Some(frame) = self.stack.last_mut() {
            frame.children.push(value);
        } else {
            self.root = Some(value);
        }
    }

    #[requires(!self.stack.is_empty())]
    #[ensures(true)]
    fn pop(&mut self) {
        let mut frame = self.stack.pop().expect("entered bracket frame exists");
        frame.children.sort_by_key(|child| {
            sexpr::expr_range(child)
                .map(|range| range.byte_start)
                .unwrap_or(usize::MAX)
        });
        self.push(sexpr::node_with_constructs(
            frame.children,
            frame.constructs,
        ));
    }
}

impl<'tree> TreeVisitor<'tree> for RecoveredBracketBuilder<'_, '_> {
    type Node = jbotci_syntax::generated_model::recovered::NodeRef<'tree>;
    type Atom = jbotci_syntax::generated_model::recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.stack.push(RecoveredBracketFrame {
            children: Vec::new(),
            constructs: brackets::bracket_source_construct(node.constructor_name())
                .into_iter()
                .collect(),
        });
    }

    #[requires(!self.stack.is_empty())]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        self.pop();
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, _field: FieldRef) {
        self.stack.push(RecoveredBracketFrame {
            children: Vec::new(),
            constructs: Vec::new(),
        });
    }

    #[requires(!self.stack.is_empty())]
    #[ensures(true)]
    fn exit_field(&mut self, _field: FieldRef) {
        self.pop();
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.stack.push(RecoveredBracketFrame {
            children: Vec::new(),
            constructs: Vec::new(),
        });
    }

    #[requires(!self.stack.is_empty())]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        self.pop();
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let jbotci_syntax::generated_model::recovered::AtomRef::Token(token) = atom;
        self.recovery_projection.separate();
        self.push(brackets::word(token, &self.context));
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: RecoveryItemState + Serialize>(&mut self, item: &'tree E) {
        if !self.recovery_projection.include(item) {
            return;
        }
        match syntax_recovery_item(item, self.errors, self.source) {
            Ok(item) => self.push(recovery_error_sexpr(&item)),
            Err(error) => self.render_error = Some(error),
        }
    }
}

#[requires(true)]
#[ensures(!recovered.errors.is_empty() || ret.is_ok())]
#[expensive_ensures(ret.as_ref().is_ok_and(|text| !text.is_empty() || recovered.errors.is_empty()) || ret.is_err())]
pub fn pretty_recovered_syntax_brackets_with_options(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    if let Ok(valid) = recovered.parse_tree.as_ref().clone().try_into_valid() {
        return crate::pretty_generated_model_brackets_with_options(&valid, source, options);
    }
    let mut visitor = RecoveredBracketBuilder::new(source, &recovered.errors, options);
    jbotci_syntax::generated_model::recovered::TreeNode::visit_in_order(
        recovered.parse_tree.as_ref(),
        &mut visitor,
    );
    let value = sexpr::flatten(visitor.finish()?);
    Ok(sexpr::render_bracketed_with_options(&value, options))
}

#[requires(true)]
#[ensures(!recovered.errors.is_empty() || ret.is_ok())]
#[expensive_ensures(ret.as_ref().is_ok_and(|fragments| !fragments.is_empty() || recovered.errors.is_empty()) || ret.is_err())]
pub fn pretty_recovered_syntax_bracket_source_fragments_with_options(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: BracketRenderOptions,
) -> Result<Vec<crate::BracketSourceFragment>, OutputError> {
    if let Ok(valid) = recovered.parse_tree.as_ref().clone().try_into_valid() {
        return crate::pretty_bracket_source_fragments_with_options(&valid, source, options);
    }
    let mut visitor = RecoveredBracketBuilder::new(source, &recovered.errors, options);
    jbotci_syntax::generated_model::recovered::TreeNode::visit_in_order(
        recovered.parse_tree.as_ref(),
        &mut visitor,
    );
    let value = sexpr::flatten(visitor.finish()?);
    Ok(sexpr::render_bracketed_source_fragments_with_options(
        &value, options,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn pretty_recovered_syntax_tree_with_options(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    tree::pretty_recovered_generated_model_tree_with_options(recovered, source, options)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn compact_recovered_syntax_json_string_with_options(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: JsonRenderOptions,
) -> Result<String, OutputError> {
    if let Ok(valid) = recovered.parse_tree.as_ref().clone().try_into_valid() {
        return crate::compact_generated_model_json_string_with_options(&valid, options);
    }
    let tree_options = TreeRenderOptions {
        color: false,
        indent: options.indent,
        phonemes: options.phonemes,
        glyphs: crate::GlyphStyle::Unicode,
        show_spans: true,
        show_refs: false,
        decompose_lujvo: false,
        show_elided: options.show_elided,
    };
    let tree = tree::recovered_generated_model_tree_value(recovered, source, tree_options)?;
    let value = recovered_tree_json_value(&tree);
    Ok(crate::format_compact_json_value(&value, 0, options))
}

#[requires(true)]
#[ensures(true)]
fn recovered_tree_json_value(value: &TreeValue) -> Value {
    match value {
        TreeValue::Node(node) => {
            let has_labelled_entries = node.entries.iter().any(|entry| entry.label.is_some());
            let payload = if !has_labelled_entries {
                let mut primary = node
                    .entries
                    .iter()
                    .map(|entry| recovered_tree_json_value(&entry.value))
                    .collect::<Vec<_>>();
                match primary.len() {
                    0 => Value::Object(Map::new()),
                    1 => primary.pop().expect("length checked"),
                    _ => Value::Array(primary),
                }
            } else {
                let mut fields = Map::new();
                let mut primary = node
                    .entries
                    .iter()
                    .filter(|entry| entry.label.is_none())
                    .map(|entry| recovered_tree_json_value(&entry.value))
                    .collect::<Vec<_>>();
                for entry in &node.entries {
                    if let Some(label) = entry.label {
                        fields.insert(label.to_owned(), recovered_tree_json_value(&entry.value));
                    } else if !primary.is_empty() {
                        insert_primary_json_field(&mut fields, &mut primary);
                    }
                }
                Value::Object(fields)
            };
            json::constructor_value(node.constructor, payload)
        }
        TreeValue::Collection(items) => {
            Value::Array(items.iter().map(recovered_tree_json_value).collect())
        }
        TreeValue::Syntax { value, .. } => recovered_tree_json_value(value),
        TreeValue::Word {
            constructor,
            phonemes,
            span,
            elided,
        } => {
            let mut fields = Map::new();
            fields.insert("phonemes".to_owned(), Value::String(phonemes.clone()));
            if let Some((start, end)) = span {
                fields.insert(
                    "span".to_owned(),
                    Value::Array(vec![(*start).into(), (*end).into()]),
                );
            }
            if *elided {
                fields.insert("elided".to_owned(), Value::Bool(true));
            }
            json::constructor_value(constructor, Value::Object(fields))
        }
        TreeValue::Verbatim { text, span } => {
            let mut fields = Map::new();
            fields.insert("text".to_owned(), Value::String(text.clone()));
            if let Some((start, end)) = span {
                fields.insert(
                    "span".to_owned(),
                    Value::Array(vec![(*start).into(), (*end).into()]),
                );
            }
            json::constructor_value("Verbatim", Value::Object(fields))
        }
        TreeValue::Error { error } => {
            let mut fields = Map::new();
            if let Some((start, end)) = error.span {
                fields.insert(
                    "span".to_owned(),
                    Value::Array(vec![start.into(), end.into()]),
                );
            }
            fields.insert("text".to_owned(), Value::String(error.text.clone()));
            fields.insert(
                "expected".to_owned(),
                Value::Array(error.expected.iter().cloned().map(Value::String).collect()),
            );
            fields.insert("error_index".to_owned(), error.error_index.into());
            fields.insert(
                "diagnostic_code".to_owned(),
                Value::String(error.diagnostic_code.clone()),
            );
            json::constructor_value("Error", Value::Object(fields))
        }
        TreeValue::Text(text) => Value::String(text.clone()),
        TreeValue::Span {
            char_start,
            char_end,
            ..
        } => Value::Array(vec![(*char_start).into(), (*char_end).into()]),
    }
}

#[requires(true)]
#[ensures(primary.is_empty())]
fn insert_primary_json_field(fields: &mut Map<String, Value>, primary: &mut Vec<Value>) {
    if primary.is_empty() {
        return;
    }
    let value = if primary.len() == 1 {
        primary.pop().expect("length checked")
    } else {
        Value::Array(std::mem::take(primary))
    };
    fields.insert("value".to_owned(), value);
}

#[requires(indent.is_none_or(|indent| indent == 0))]
#[ensures(ret.ends_with('\n'))]
pub fn pretty_recovered_syntax_raw(
    recovered: &RecoveredSyntaxParse,
    indent: Option<usize>,
) -> String {
    match recovered.parse_tree.as_ref().clone().try_into_valid() {
        Ok(valid) => debug_output(&valid, indent),
        Err(_) => debug_output(recovered.parse_tree.as_ref(), indent),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jbotci_syntax::{ParseOptions, SyntaxErrorKind};

    #[requires(true)]
    #[ensures(true)]
    fn parse_recovered_syntax(source: &str) -> RecoveredSyntaxParse {
        let words = jbotci_morphology::segment_words_with_modifiers(source)
            .expect("syntax recovery probes use valid morphology");
        jbotci_syntax::parse_syntax_tree_recovered_with_source_and_options(
            &words,
            source,
            &ParseOptions::default(),
        )
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_valid_empty_text_has_empty_bracket_renderings() {
        let source = "le broda sa le si";
        let recovered = parse_recovered_syntax(source);
        assert!(recovered.errors.is_empty());
        assert_eq!(
            pretty_recovered_syntax_brackets_with_options(
                &recovered,
                source,
                BracketRenderOptions::default(),
            )
            .expect("recovered syntax brackets"),
            ""
        );
        assert_eq!(
            pretty_recovered_syntax_bracket_source_fragments_with_options(
                &recovered,
                source,
                BracketRenderOptions::default(),
            )
            .expect("recovered syntax bracket source fragments"),
            Vec::new()
        );
    }

    #[requires(true)]
    #[ensures(ret.as_object().is_some())]
    fn recovered_syntax_json(source: &str, recovered: &RecoveredSyntaxParse) -> Value {
        serde_json::from_str(
            &compact_recovered_syntax_json_string_with_options(
                recovered,
                source,
                JsonRenderOptions::default(),
            )
            .expect("recovered syntax JSON"),
        )
        .expect("valid recovered syntax JSON")
    }

    #[requires(true)]
    #[ensures(true)]
    fn collect_error_objects<'value>(
        value: &'value Value,
        errors: &mut Vec<&'value Map<String, Value>>,
    ) {
        match value {
            Value::Object(object) => {
                if let Some(Value::Object(error)) = object.get("Error") {
                    errors.push(error);
                    return;
                }
                for child in object.values() {
                    collect_error_objects(child, errors);
                }
            }
            Value::Array(items) => {
                for item in items {
                    collect_error_objects(item, errors);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn count_error_source_fragments(fragments: &[crate::BracketSourceFragment]) -> usize {
        fragments
            .iter()
            .map(|fragment| match fragment {
                crate::BracketSourceFragment::Text { role, .. } => {
                    usize::from(*role == crate::BracketSourceFragmentRole::Error)
                }
                crate::BracketSourceFragment::Span { children, .. } => {
                    count_error_source_fragments(children)
                }
            })
            .sum()
    }

    #[requires(true)]
    #[ensures(true)]
    fn count_construct_source_fragments(
        fragments: &[crate::BracketSourceFragment],
        expected: crate::BracketSourceConstruct,
    ) -> usize {
        fragments
            .iter()
            .map(|fragment| match fragment {
                crate::BracketSourceFragment::Text { .. } => 0,
                crate::BracketSourceFragment::Span {
                    constructs,
                    children,
                    ..
                } => {
                    usize::from(constructs.contains(&expected))
                        + count_construct_source_fragments(children, expected)
                }
            })
            .sum()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bracket_source_fragments_retain_filterable_grammar_constructs() {
        for source in ["mi viska le mlatu", "mi ku i do viska le mlatu"] {
            let recovered = parse_recovered_syntax(source);
            let fragments = pretty_recovered_syntax_bracket_source_fragments_with_options(
                &recovered,
                source,
                BracketRenderOptions::default(),
            )
            .expect("bracket source fragments");

            assert!(
                count_construct_source_fragments(&fragments, crate::BracketSourceConstruct::Sumti,)
                    > 0,
                "sumti boundaries must survive flattening for {source:?}",
            );
            assert!(
                count_construct_source_fragments(
                    &fragments,
                    crate::BracketSourceConstruct::BridiTail,
                ) > 0,
                "bridi-tail boundaries must survive flattening for {source:?}",
            );
        }
    }

    #[requires(true)]
    #[ensures(!ret.0.is_empty() && ret.1.0 <= ret.1.1)]
    fn syntax_error_code_and_span(error: &SyntaxError) -> (&'static str, (usize, usize)) {
        match error {
            SyntaxError::NotImplemented => ("syntax.not-implemented", (0, 0)),
            SyntaxError::Parse {
                kind,
                byte_start,
                byte_end,
                ..
            } => (kind.code(), (*byte_start, *byte_end)),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_mi_ku_i_do_preserves_both_statements_and_the_error_position() {
        let source = "mi ku i do";
        let recovered = parse_recovered_syntax(source);

        let brackets = pretty_recovered_syntax_brackets_with_options(
            &recovered,
            source,
            BracketRenderOptions::default(),
        )
        .expect("brackets");
        assert_eq!(brackets, "([mi ‼ku‼] [.i do])");

        let mut bracket_builder = RecoveredBracketBuilder::new(
            source,
            &recovered.errors,
            BracketRenderOptions::default(),
        );
        jbotci_syntax::generated_model::recovered::TreeNode::visit_in_order(
            recovered.parse_tree.as_ref(),
            &mut bracket_builder,
        );
        let bracket_tree = bracket_builder.finish().expect("bracket tree");
        let mut error_leaves = Vec::new();
        collect_error_sexprs(&bracket_tree, &mut error_leaves);
        assert_eq!(error_leaves.len(), 1);
        let (text, range) = error_leaves[0];
        assert_eq!(text, "ku");
        assert_eq!(
            range.map(|range| (range.byte_start, range.byte_end)),
            Some((3, 5))
        );

        let tree = pretty_recovered_syntax_tree_with_options(
            &recovered,
            source,
            TreeRenderOptions {
                show_refs: false,
                show_spans: true,
                ..TreeRenderOptions::default()
            },
        )
        .expect("tree");
        let mi = tree.find("Cmavo @[0‥2) \"mi\"").expect("initial mi");
        let error = tree.find("Error @[3‥5) \"ku\"").expect("ku error");
        let following = tree
            .find("ParagraphStatement @[6‥10)")
            .expect("following statement");
        let do_word = tree.find("Cmavo @[8‥10) \"do\"").expect("following do");
        assert!(tree.starts_with("ParagraphStatementSequence"));
        assert!(
            mi < error && error < following && following < do_word,
            "{tree}"
        );

        let json = recovered_syntax_json(source, &recovered);
        let root = json
            .get("ParagraphStatementSequence")
            .and_then(Value::as_object)
            .expect("paragraph statement sequence");
        let initial = root
            .get("value")
            .and_then(Value::as_array)
            .expect("initial statement values");
        assert_eq!(initial[0]["Cmavo"]["phonemes"], "mi");
        assert_eq!(initial[1]["Error"]["span"], serde_json::json!([3, 5]));
        assert_eq!(initial[1]["Error"]["text"], "ku");
        let following = root["following"].as_array().expect("following statements");
        assert_eq!(following.len(), 1);
        assert_eq!(
            following[0]["ParagraphStatement"]["i"]["Cmavo"]["phonemes"],
            "i"
        );
        assert_eq!(
            following[0]["ParagraphStatement"]["value"]["Cmavo"]["phonemes"],
            "do"
        );

        let raw = pretty_recovered_syntax_raw(&recovered, Some(0));
        assert!(raw.starts_with("RegularText("), "{raw}");
        assert!(raw.contains("Prefix(RecoveredPrefix { errors: [SkippedTokens { error_index: 0"));
        assert!(raw.contains("byte_start: 3, byte_end: 5"));
        assert!(raw.contains("FollowingParagraphStatementSyntax"));
        assert!(raw.contains("text: \"mi\""));
        assert!(raw.contains("text: \"do\""));
    }

    #[requires(true)]
    #[ensures(true)]
    fn collect_error_sexprs<'expr>(
        expr: &'expr sexpr::SExpr,
        errors: &mut Vec<(&'expr str, Option<BracketSourceRange>)>,
    ) {
        match expr {
            sexpr::SExpr::Leaf {
                text,
                range,
                role: sexpr::LeafRole::Error,
            } => errors.push((text, *range)),
            sexpr::SExpr::Leaf { .. } => {}
            sexpr::SExpr::Node { children, .. } => {
                for child in children {
                    collect_error_sexprs(child, errors);
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_eof_error_keeps_prefix_and_zero_width_markers() {
        let source = "mi viska lo";
        let recovered = parse_recovered_syntax(source);
        assert!(matches!(
            recovered.errors.as_slice(),
            [SyntaxError::Parse {
                kind: SyntaxErrorKind::IncompleteSumti,
                byte_start: 11,
                byte_end: 11,
                ..
            }]
        ));

        let brackets = pretty_recovered_syntax_brackets_with_options(
            &recovered,
            source,
            BracketRenderOptions::default(),
        )
        .expect("brackets");
        assert_eq!(brackets, "(mi [víska {lo ‼‼}])");

        let source_fragments = pretty_recovered_syntax_bracket_source_fragments_with_options(
            &recovered,
            source,
            BracketRenderOptions::default(),
        )
        .expect("bracket source fragments");
        assert_eq!(count_error_source_fragments(&source_fragments), 1);

        let tree = pretty_recovered_syntax_tree_with_options(
            &recovered,
            source,
            TreeRenderOptions {
                show_refs: false,
                show_spans: true,
                ..TreeRenderOptions::default()
            },
        )
        .expect("tree");
        assert!(tree.starts_with("BridiWithLeadingTerms @[0‥11)"), "{tree}");
        let mi = tree.find("Cmavo @[0‥2) \"mi\"").expect("mi prefix");
        let viska = tree.find("Gismu @[3‥8) \"víska\"").expect("viska prefix");
        let lo = tree.find("Cmavo @[9‥11) \"lo\"").expect("lo prefix");
        let marker = tree.find("Error @[11‥11) \"\"").expect("missing marker");
        assert!(mi < viska && viska < lo && lo < marker, "{tree}");
        assert_eq!(tree.matches("Error @[11‥11) \"\"").count(), 1, "{tree}");

        let json = recovered_syntax_json(source, &recovered);
        assert_eq!(
            json["BridiWithLeadingTerms"]["leading_terms"][0]["Cmavo"]["phonemes"],
            "mi"
        );
        assert_eq!(
            json["BridiWithLeadingTerms"]["bridi_tail"]["SelbriSimpleBridiTail"]["selbri"]["Gismu"]
                ["phonemes"],
            "víska"
        );
        let mut errors = Vec::new();
        collect_error_objects(&json, &mut errors);
        assert_eq!(errors.len(), 1);
        assert!(
            errors
                .iter()
                .all(|error| error["span"] == serde_json::json!([11, 11]))
        );
        assert!(errors.iter().all(|error| error["text"] == ""));
        assert!(errors.iter().all(|error| error["error_index"] == 0));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_two_errors_link_json_to_diagnostics_in_source_order() {
        let source = "mi ku i do ku i mi klama";
        let recovered = parse_recovered_syntax(source);
        assert_eq!(recovered.errors.len(), 2);

        let brackets = pretty_recovered_syntax_brackets_with_options(
            &recovered,
            source,
            BracketRenderOptions::default(),
        )
        .expect("brackets");
        assert_eq!(brackets, "([mi ‼ku‼] [{.i do} ‼ku‼ {.i (mi kláma)}])");

        let tree = pretty_recovered_syntax_tree_with_options(
            &recovered,
            source,
            TreeRenderOptions {
                show_refs: false,
                show_spans: true,
                ..TreeRenderOptions::default()
            },
        )
        .expect("tree");
        let first = tree.find("Error @[3‥5) \"ku\"").expect("first error");
        let do_word = tree.find("Cmavo @[8‥10) \"do\"").expect("middle statement");
        let second = tree.find("Error @[11‥13) \"ku\"").expect("second error");
        let klama = tree
            .find("Gismu @[19‥24) \"kláma\"")
            .expect("final statement");
        assert!(
            first < do_word && do_word < second && second < klama,
            "{tree}"
        );

        let json = recovered_syntax_json(source, &recovered);
        let mut errors = Vec::new();
        collect_error_objects(&json, &mut errors);
        assert_eq!(errors.len(), 2);
        for (source_order, error) in errors.iter().enumerate() {
            let error_index = error["error_index"].as_u64().expect("error index") as usize;
            assert_eq!(error_index, source_order);
            let (code, (start, end)) = syntax_error_code_and_span(&recovered.errors[error_index]);
            assert_eq!(error["diagnostic_code"], code);
            assert_eq!(error["span"], serde_json::json!([start, end]));
            assert_eq!(error["text"], "ku");
        }
        assert_eq!(
            json["ParagraphStatementSequence"]["following"][2]["ParagraphStatement"]["value"]["BridiWithLeadingTerms"]
                ["bridi_tail"]["Gismu"]["phonemes"],
            "kláma"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_morphology_marks_invalid_region_between_valid_words() {
        let source = "mi @@@ do";
        let recovered = jbotci_morphology::segment_words_with_modifiers_recovered(source);

        let brackets = pretty_recovered_morphology_brackets_with_options(
            &recovered,
            source,
            BracketRenderOptions::default(),
        )
        .expect("morphology brackets");
        assert_eq!(brackets, "(mi ‼@@@ ‼ do)");

        let tree = pretty_recovered_morphology_tree_with_options(
            &recovered,
            source,
            TreeRenderOptions::default(),
        )
        .expect("morphology tree");
        let mi = tree.find("Cmavo \"mi\"").expect("mi");
        let error = tree.find("Error \"@@@ \"").expect("error");
        let do_word = tree.find("Cmavo \"do\"").expect("do");
        assert!(mi < error && error < do_word, "{tree}");

        let json: Value = serde_json::from_str(
            &compact_recovered_morphology_json_string_with_options(
                &recovered,
                source,
                JsonRenderOptions::default(),
            )
            .expect("morphology JSON"),
        )
        .expect("valid morphology JSON");
        let items = json.as_array().expect("morphology sequence");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["PlainWord"]["Cmavo"]["phonemes"], "mi");
        assert_eq!(items[1]["Error"]["span"], serde_json::json!([3, 7]));
        assert_eq!(items[1]["Error"]["text"], "@@@ ");
        assert_eq!(items[1]["Error"]["error_index"], 0);
        assert_eq!(items[2]["PlainWord"]["Cmavo"]["phonemes"], "do");

        let raw = pretty_recovered_morphology_raw(&recovered, source, Some(0)).expect("raw");
        let mi = raw.find("text: \"mi\"").expect("raw mi");
        let error = raw.find("text: \"@@@ \"").expect("raw error");
        let do_word = raw.find("text: \"do\"").expect("raw do");
        assert!(mi < error && error < do_word, "{raw}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn valid_recovered_renderers_are_byte_identical_to_strict_renderers() {
        for source in [
            "mi klama",
            "lo nu mi klama",
            "to coi",
            "zoi gy hello world gy",
        ] {
            let words =
                jbotci_morphology::segment_words_with_modifiers(source).expect("morphology");
            let recovered_morphology =
                jbotci_morphology::segment_words_with_modifiers_recovered(source);
            assert!(recovered_morphology.errors.is_empty(), "{source}");

            assert_eq!(
                pretty_recovered_morphology_brackets_with_options(
                    &recovered_morphology,
                    source,
                    BracketRenderOptions::default(),
                ),
                crate::pretty_morphology_brackets_with_options(
                    &words,
                    source,
                    BracketRenderOptions::default(),
                ),
                "morphology brackets: {source}",
            );
            assert_eq!(
                pretty_recovered_morphology_tree_with_options(
                    &recovered_morphology,
                    source,
                    TreeRenderOptions::default(),
                ),
                crate::pretty_morphology_tree_with_options(
                    &words,
                    source,
                    TreeRenderOptions::default(),
                ),
                "morphology tree: {source}",
            );
            assert_eq!(
                compact_recovered_morphology_json_string_with_options(
                    &recovered_morphology,
                    source,
                    JsonRenderOptions::default(),
                ),
                crate::compact_morphology_json_string_with_options(
                    &words,
                    JsonRenderOptions::default(),
                ),
                "morphology JSON: {source}",
            );
            assert_eq!(
                pretty_recovered_morphology_raw(&recovered_morphology, source, Some(0))
                    .expect("recovered morphology raw"),
                debug_output(&words, Some(0)),
                "morphology raw: {source}",
            );

            let strict = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
                &words,
                source,
                &ParseOptions::default(),
            )
            .expect("strict syntax");
            let recovered = parse_recovered_syntax(source);
            assert!(recovered.errors.is_empty(), "{source}");

            let bracket_options = BracketRenderOptions::default();
            assert_eq!(
                pretty_recovered_syntax_brackets_with_options(&recovered, source, bracket_options,),
                crate::pretty_generated_model_brackets_with_options(
                    &strict,
                    source,
                    bracket_options,
                ),
                "syntax brackets: {source}",
            );
            let tree_options = TreeRenderOptions {
                show_refs: false,
                ..TreeRenderOptions::default()
            };
            assert_eq!(
                pretty_recovered_syntax_tree_with_options(&recovered, source, tree_options),
                crate::pretty_generated_model_tree_with_options(&strict, source, tree_options),
                "syntax tree: {source}",
            );
            assert_eq!(
                compact_recovered_syntax_json_string_with_options(
                    &recovered,
                    source,
                    JsonRenderOptions::default(),
                ),
                crate::compact_generated_model_json_string_with_options(
                    &strict,
                    JsonRenderOptions::default(),
                ),
                "syntax JSON: {source}",
            );
            assert_eq!(
                pretty_recovered_syntax_raw(&recovered, Some(0)),
                debug_output(strict.as_ref(), Some(0)),
                "syntax raw: {source}",
            );
        }

        let source = "to coi";
        let recovered = parse_recovered_syntax(source);
        let color_options = BracketRenderOptions {
            color: true,
            show_elided: true,
            ..BracketRenderOptions::default()
        };
        let current =
            pretty_recovered_syntax_brackets_with_options(&recovered, source, color_options)
                .expect("color brackets");
        let legacy = current
            .replace("\x1b[3m", "\x1b[9m")
            .replace("\x1b[23m", "\x1b[29m");
        let mechanically_restyled = legacy
            .replace("\x1b[9m", "\x1b[3m")
            .replace("\x1b[29m", "\x1b[23m");
        assert_eq!(mechanically_restyled, current);
        assert!(current.contains("\x1b[3mtoi\x1b[23m"), "{current:?}");
        assert!(!current.contains("\x1b[9m"), "{current:?}");
    }
}
