//! Structural renderers for recovered morphology and syntax results.

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, invariant, new, requires};
use jbotci_morphology::{MorphologyError, RecoveredMorphologySegmentation};
use jbotci_source::SourceSpan;
use jbotci_syntax::{RecoveredSyntaxParse, SyntaxError};
use jbotci_tree::{FieldRef, RecoveryItemState, RecoveryProjection, TreeVisitor};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::budget::ByteLimit;
use crate::json;
use crate::tree::{self, RecoveryTreeError, TreeValue};
use crate::{
    BracketRenderOptions, BracketSourceConstruct, BracketSourceRange, Indentation,
    JsonRenderOptions, OutputError, TreeRenderOptions, brackets, sexpr,
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
    max_bytes: &crate::budget::OutputBudget,
) -> Result<RecoveryRenderItem, OutputError> {
    // The original span is still a borrow here. An oversized span copies nothing.
    let Some(borrowed) = source.get(span.byte_start..span.byte_end) else {
        return Err(OutputError::Recovery(format!(
            "recovery span {}..{} is outside the source text",
            span.byte_start, span.byte_end
        )));
    };
    // Recovery text is written as a JSON string. Charge that escaped length.
    let Some(bytes) =
        crate::json_writer::json_string_body_len_within(borrowed, max_bytes.remaining())
    else {
        return Err(OutputError::Limit {
            limit: max_bytes.limit(),
        });
    };
    let text = max_bytes
        .copy_accounted(borrowed, bytes)
        .map_err(|error| OutputError::Limit { limit: error.limit })?;
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
    max_bytes: &crate::budget::OutputBudget,
) -> Result<RecoveryRenderItem, OutputError> {
    let (diagnostic_code, expected) = match &errors[error_index] {
        MorphologyError::Invalid { kind, .. } => (
            kind.code().to_owned(),
            vec![errors[error_index].diagnostic_message()],
        ),
        MorphologyError::UnterminatedZoiQuote { delimiter, .. } => (
            "morphology.unterminated-zoi-quote".to_owned(),
            vec![format!("closing delimiter `{delimiter}`")],
        ),
        MorphologyError::DialectExpansion(failure) => (
            "morphology.dialect-expansion-limit".to_owned(),
            vec![failure.to_string()],
        ),
        MorphologyError::SourceSpan(_) => (
            "morphology.source-span".to_owned(),
            vec!["valid source span".to_owned()],
        ),
        MorphologyError::Interrupted => return Err(OutputError::Interrupted),
    };
    recovery_item_from_span(
        error_index,
        errors.len(),
        diagnostic_code,
        expected,
        span.clone(),
        source,
        max_bytes,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn syntax_recovery_item<E: RecoveryItemState + Serialize>(
    item: &E,
    errors: &[SyntaxError],
    source: &str,
    max_bytes: &crate::budget::OutputBudget,
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
        SyntaxError::Interrupted => return Err(OutputError::Interrupted),
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
        max_bytes,
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
    max_bytes: &crate::budget::OutputBudget,
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
                item: morphology_recovery_item(
                    error_index,
                    &recovered.errors,
                    span,
                    source,
                    max_bytes,
                )?,
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
    pretty_recovered_morphology_brackets_with_limit(
        recovered,
        source,
        options,
        ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
        &jbotci_interrupt::CancellationToken::never(),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn pretty_recovered_morphology_brackets_with_limit(
    recovered: &RecoveredMorphologySegmentation,
    source: &str,
    options: BracketRenderOptions,
    limit: ByteLimit,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> Result<String, OutputError> {
    if recovered.errors.is_empty() {
        return crate::pretty_morphology_brackets_with_limit(
            &recovered.words,
            source,
            options,
            limit,
            cancellation,
        );
    }
    let leaves = crate::budget::OutputBudget::new(limit);
    let context = brackets::BracketContext {
        options,
        max_leaf_bytes: leaves.clone(),
    };
    let children = recovered_morphology_sequence(recovered, source, &leaves)?
        .into_iter()
        .map(|item| match item.into_data() {
            data!(RecoveredMorphologySequenceItem::Valid { word }) => {
                brackets::word_like_brackets(word, &context)
            }
            data!(RecoveredMorphologySequenceItem::Error { item }) => {
                Ok(recovery_error_sexpr(&item))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let value = sexpr::flatten(sexpr::node(children));
    sexpr::render_bracketed_with_limit(&value, options, limit, cancellation)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn pretty_recovered_morphology_tree_with_options(
    recovered: &RecoveredMorphologySegmentation,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    pretty_recovered_morphology_tree_with_limit(
        recovered,
        source,
        options,
        ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
        &jbotci_interrupt::CancellationToken::never(),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn pretty_recovered_morphology_tree_with_limit(
    recovered: &RecoveredMorphologySegmentation,
    source: &str,
    options: TreeRenderOptions,
    limit: ByteLimit,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> Result<String, OutputError> {
    if recovered.errors.is_empty() {
        return crate::pretty_morphology_tree_with_limit(
            &recovered.words,
            source,
            options,
            limit,
            cancellation,
        );
    }
    let leaves = crate::budget::OutputBudget::new(limit);
    let mut values = Vec::new();
    for item in recovered_morphology_sequence(recovered, source, &leaves)? {
        values.push(match item.into_data() {
            data!(RecoveredMorphologySequenceItem::Valid { word }) => {
                tree::morphology_tree_value(word, source, options, &leaves)?
            }
            data!(RecoveredMorphologySequenceItem::Error { item }) => {
                recovery_error_tree_value(&item)
            }
        });
    }
    tree::render_tree_value_with_limit(
        &tree::collapse_value(TreeValue::Collection(values)),
        options,
        None,
        limit,
        cancellation,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn compact_recovered_morphology_json_string_with_options(
    recovered: &RecoveredMorphologySegmentation,
    source: &str,
    options: JsonRenderOptions,
) -> Result<String, OutputError> {
    compact_recovered_morphology_json_string_with_limit(
        recovered,
        source,
        options,
        ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn compact_recovered_morphology_json_string_with_limit(
    recovered: &RecoveredMorphologySegmentation,
    source: &str,
    options: JsonRenderOptions,
    limit: ByteLimit,
) -> Result<String, OutputError> {
    compact_recovered_morphology_json_string_with_limits(recovered, source, options, limit, limit)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= text_limit.get()) || ret.is_err())]
pub fn compact_recovered_morphology_json_string_with_limits(
    recovered: &RecoveredMorphologySegmentation,
    source: &str,
    options: JsonRenderOptions,
    text_limit: ByteLimit,
    projection_limit: ByteLimit,
) -> Result<String, OutputError> {
    if recovered.errors.is_empty() {
        return crate::compact_morphology_json_string_with_limits(
            &recovered.words,
            options,
            text_limit,
            projection_limit,
        );
    }
    // Serde can only report a string. Check before projection so a cancelled
    // segmentation stays `OutputError::Interrupted` instead of a JSON error.
    if recovered.errors.iter().any(MorphologyError::is_interrupted) {
        return Err(OutputError::Interrupted);
    }
    let projection = new!(RecoveredMorphologyProjection { recovered, source });
    let mut value = crate::compact_json_value_with_limit(&projection, projection_limit)?;
    json::render_phoneme_fields_in_json_value(&mut value, options.phonemes);
    crate::render_compact_json_value_with_limit(&value, options, text_limit)
}

#[invariant(recovered.errors.len() == recovered.error_regions.len())]
struct RecoveredMorphologyProjection<'a> {
    recovered: &'a RecoveredMorphologySegmentation,
    source: &'a str,
}
impl Serialize for RecoveredMorphologyProjection<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{Error, SerializeSeq};
        let mut sequence = serializer.serialize_seq(None)?;
        let mut words = self.recovered.words.iter().peekable();
        let mut errors = self.recovered.error_regions.iter().enumerate().peekable();
        while words.peek().is_some() || errors.peek().is_some() {
            let word_start = words
                .peek()
                .and_then(|word| word.byte_range())
                .map(|range| range.start);
            let error_start = errors.peek().map(|(_, span)| span.byte_start);
            #[cfg(test)]
            json::record_projection_visit();
            if error_start.is_some() && (word_start.is_none() || error_start < word_start) {
                let (index, span) = errors.next().expect("peeked recovery region");
                let error = &self.recovered.errors[index];
                let code = match error {
                    MorphologyError::Invalid { kind, .. } => kind.code(),
                    MorphologyError::UnterminatedZoiQuote { .. } => {
                        "morphology.unterminated-zoi-quote"
                    }
                    MorphologyError::DialectExpansion(_) => "morphology.dialect-expansion-limit",
                    MorphologyError::SourceSpan(_) => "morphology.source-span",
                    // `compact_recovered_morphology_json_string_with_limits`
                    // returns `OutputError::Interrupted` before serialize.
                    MorphologyError::Interrupted => {
                        return Err(S::Error::custom(
                            "interrupted morphology reached the JSON projection",
                        ));
                    }
                };
                let text = self
                    .source
                    .get(span.byte_start..span.byte_end)
                    .ok_or_else(|| S::Error::custom("recovery span is outside the source"))?;
                let fields = new!(MorphologyRecoveryErrorProjection {
                    span: [span.char_start, span.char_end],
                    text,
                    expected: ExpectedMorphologyError(error),
                    error_index: index,
                    diagnostic_code: code,
                });
                sequence.serialize_element(&MorphologyErrorConstructor { error: fields })?;
            } else {
                sequence.serialize_element(words.next().expect("peeked word"))?;
            }
        }
        sequence.end()
    }
}

#[invariant(span[0] <= span[1] && !diagnostic_code.is_empty())]
#[derive(Serialize)]
struct MorphologyRecoveryErrorProjection<'a> {
    span: [usize; 2],
    text: &'a str,
    expected: ExpectedMorphologyError<'a>,
    error_index: usize,
    diagnostic_code: &'static str,
}
#[invariant(true)]
#[derive(Serialize)]
struct MorphologyErrorConstructor<'a> {
    #[serde(rename = "Error")]
    error: MorphologyRecoveryErrorProjection<'a>,
}
#[invariant(true)]
struct ExpectedMorphologyError<'a>(&'a MorphologyError);
impl Serialize for ExpectedMorphologyError<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // collect_str writes the diagnostic directly into serde_json's bounded sink.
        struct_display_expected(self.0, serializer)
    }
}
#[requires(true)]
#[ensures(true)]
fn struct_display_expected<S: serde::Serializer>(
    error: &MorphologyError,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeSeq;
    let mut sequence = serializer.serialize_seq(Some(1))?;
    sequence.serialize_element(&MorphologyExpectation(error))?;
    sequence.end()
}
#[invariant(true)]
struct MorphologyExpectation<'a>(&'a MorphologyError);
impl Serialize for MorphologyExpectation<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
impl std::fmt::Display for MorphologyExpectation<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            MorphologyError::Invalid { .. } => {
                let message = self.0.diagnostic_message();
                formatter.write_str(&message)
            }
            MorphologyError::UnterminatedZoiQuote { delimiter, .. } => {
                write!(formatter, "closing delimiter `{delimiter}`")
            }
            MorphologyError::DialectExpansion(failure) => write!(formatter, "{failure}"),
            MorphologyError::SourceSpan(_) => formatter.write_str("valid source span"),
            MorphologyError::Interrupted => formatter.write_str("computation interrupted"),
        }
    }
}

#[requires(indent.is_none_or(|indent| indent.spaces() == 0))]
#[ensures(ret.as_ref().is_ok_and(|text| text.ends_with('\n')) || ret.is_err())]
pub fn pretty_recovered_morphology_raw(
    recovered: &RecoveredMorphologySegmentation,
    source: &str,
    indent: Option<Indentation>,
) -> Result<String, OutputError> {
    if recovered.errors.is_empty() {
        return Ok(debug_output(&recovered.words, indent));
    }
    Ok(debug_output(
        &recovered_morphology_sequence(
            recovered,
            source,
            &crate::budget::OutputBudget::unlimited(),
        )?,
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

#[cfg(test)]
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

#[requires(indent.is_none_or(|indent| indent.spaces() == 0))]
#[ensures(ret.ends_with('\n'))]
fn debug_output(value: &impl std::fmt::Debug, indent: Option<Indentation>) -> String {
    if indent == Some(Indentation::compact()) {
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
        max_leaf_bytes: &crate::budget::OutputBudget,
    ) -> Self {
        Self {
            context: brackets::BracketContext {
                options,
                max_leaf_bytes: max_leaf_bytes.clone(),
            },
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
        match brackets::word(token, &self.context) {
            Ok(value) => self.push(value),
            Err(error) => self.render_error = Some(error),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: RecoveryItemState + Serialize>(&mut self, item: &'tree E) {
        if !self.recovery_projection.include(item) {
            return;
        }
        match syntax_recovery_item(item, self.errors, self.source, &self.context.max_leaf_bytes) {
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
    pretty_recovered_syntax_brackets_with_limit(
        recovered,
        source,
        options,
        ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
        &jbotci_interrupt::CancellationToken::never(),
    )
}

/// Render recovered brackets until `cancellation` is set.
///
/// The recovered tree visitor cannot abort mid-walk. The flag is read before
/// the walk and again when the walk returns; a set flag discards the s-expression
/// instead of rendering it. The string writer also reads the flag at each node.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn pretty_recovered_syntax_brackets_with_limit(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: BracketRenderOptions,
    limit: ByteLimit,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> Result<String, OutputError> {
    if cancellation.is_cancelled() {
        return Err(OutputError::Interrupted);
    }
    // The conversion takes the tree by value. Each token is a shared pointer.
    // The quote text stays in that pointer. The leaf copy uses the byte limit.
    if let Ok(valid) = recovered.parse_tree.as_ref().clone().try_into_valid() {
        return crate::pretty_generated_model_brackets_with_limit(
            &valid,
            source,
            options,
            limit,
            cancellation,
        );
    }
    let leaves = crate::budget::OutputBudget::new(limit);
    let mut visitor = RecoveredBracketBuilder::new(source, &recovered.errors, options, &leaves);
    jbotci_syntax::generated_model::recovered::TreeNode::visit_in_order(
        recovered.parse_tree.as_ref(),
        &mut visitor,
    );
    if cancellation.is_cancelled() {
        return Err(OutputError::Interrupted);
    }
    let value = sexpr::flatten(visitor.finish()?);
    sexpr::render_bracketed_with_limit(&value, options, limit, cancellation)
}

#[requires(true)]
#[ensures(!recovered.errors.is_empty() || ret.is_ok())]
#[expensive_ensures(ret.as_ref().is_ok_and(|fragments| !fragments.is_empty() || recovered.errors.is_empty()) || ret.is_err())]
pub fn pretty_recovered_syntax_bracket_source_fragments_with_options(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: BracketRenderOptions,
) -> Result<Vec<crate::BracketSourceFragment>, OutputError> {
    let (brackets, _) =
        pretty_recovered_syntax_source_fragment_pair_with_options(recovered, source, options)?;
    Ok(brackets)
}

#[requires(true)]
#[ensures(!recovered.errors.is_empty() || ret.is_ok())]
#[expensive_ensures(ret.as_ref().is_ok_and(|(brackets, plain)| {
    (!brackets.is_empty() || recovered.errors.is_empty())
        && (!plain.is_empty() || recovered.errors.is_empty())
}) || ret.is_err())]
pub fn pretty_recovered_syntax_source_fragment_pair_with_options(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: BracketRenderOptions,
) -> Result<
    (
        Vec<crate::BracketSourceFragment>,
        Vec<crate::BracketSourceFragment>,
    ),
    OutputError,
> {
    if let Ok(valid) = recovered.parse_tree.as_ref().clone().try_into_valid() {
        return crate::brackets::pretty_generated_model_source_fragment_pair_with_options(
            &valid, source, options,
        );
    }
    let value = recovered_flat_bracket_sexpr(recovered, source, options)?;
    Ok((
        sexpr::render_bracketed_source_fragments_with_options(&value, options),
        sexpr::render_plain_source_fragments_with_options(&value, options),
    ))
}

#[requires(true)]
#[ensures(true)]
fn recovered_flat_bracket_sexpr(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: BracketRenderOptions,
) -> Result<sexpr::SExpr, OutputError> {
    let leaves = crate::budget::OutputBudget::unlimited();
    let mut visitor = RecoveredBracketBuilder::new(source, &recovered.errors, options, &leaves);
    jbotci_syntax::generated_model::recovered::TreeNode::visit_in_order(
        recovered.parse_tree.as_ref(),
        &mut visitor,
    );
    Ok(sexpr::flatten(visitor.finish()?))
}

#[requires(true)]
#[ensures(!recovered.errors.is_empty() || ret.is_ok())]
#[expensive_ensures(ret.as_ref().is_ok_and(|fragments| !fragments.is_empty() || recovered.errors.is_empty()) || ret.is_err())]
pub fn pretty_recovered_syntax_plain_source_fragments_with_options(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: BracketRenderOptions,
) -> Result<Vec<crate::BracketSourceFragment>, OutputError> {
    let (_, plain) =
        pretty_recovered_syntax_source_fragment_pair_with_options(recovered, source, options)?;
    Ok(plain)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn pretty_recovered_syntax_plain_with_limit(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: BracketRenderOptions,
    limit: ByteLimit,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> Result<String, OutputError> {
    if cancellation.is_cancelled() {
        return Err(OutputError::Interrupted);
    }
    // The conversion takes the tree by value. Each token is a shared pointer.
    // The quote text stays in that pointer. The leaf copy uses the byte limit.
    if let Ok(valid) = recovered.parse_tree.as_ref().clone().try_into_valid() {
        return crate::pretty_generated_model_plain_with_limit(
            &valid,
            source,
            options,
            limit,
            cancellation,
        );
    }
    let leaves = crate::budget::OutputBudget::new(limit);
    let mut visitor = RecoveredBracketBuilder::new(source, &recovered.errors, options, &leaves);
    jbotci_syntax::generated_model::recovered::TreeNode::visit_in_order(
        recovered.parse_tree.as_ref(),
        &mut visitor,
    );
    if cancellation.is_cancelled() {
        return Err(OutputError::Interrupted);
    }
    let value = sexpr::flatten(visitor.finish()?);
    sexpr::render_plain_with_limit(&value, options, limit, cancellation)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn pretty_recovered_syntax_tree_with_options(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    pretty_recovered_syntax_tree_with_limit(
        recovered,
        source,
        options,
        ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn pretty_recovered_syntax_tree_with_limit(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: TreeRenderOptions,
    limit: ByteLimit,
) -> Result<String, OutputError> {
    tree::pretty_recovered_generated_model_tree_with_limit(recovered, source, options, limit)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn compact_recovered_syntax_json_string_with_options(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: JsonRenderOptions,
) -> Result<String, OutputError> {
    compact_recovered_syntax_json_string_with_limits(
        recovered,
        source,
        options,
        ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
        ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn compact_recovered_syntax_json_string_with_limits(
    recovered: &RecoveredSyntaxParse,
    source: &str,
    options: JsonRenderOptions,
    limit: ByteLimit,
    projection_limit: ByteLimit,
) -> Result<String, OutputError> {
    if let Ok(valid) = recovered.parse_tree.as_ref().clone().try_into_valid() {
        return crate::compact_generated_model_json_string_with_limits(
            &valid,
            options,
            limit,
            projection_limit,
        );
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
    let tree = tree::recovered_generated_model_tree_value(
        recovered,
        source,
        tree_options,
        &crate::budget::OutputBudget::new(projection_limit),
    )?;
    let value = recovered_tree_json_value(&tree);
    // One charge for the whole document. A leaf that fits can still overflow it.
    // The text limit stays on the writer below.
    if projection_limit.get() != usize::MAX {
        crate::budget::bounded_json_size(&value, projection_limit)
            .map_err(|error| OutputError::from_json(error, projection_limit))?;
    }
    crate::render_compact_json_value_with_limit(&value, options, limit)
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
        TreeValue::Verbatim { text, span, .. } => {
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

#[requires(indent.is_none_or(|indent| indent.spaces() == 0))]
#[ensures(ret.ends_with('\n'))]
pub fn pretty_recovered_syntax_raw(
    recovered: &RecoveredSyntaxParse,
    indent: Option<Indentation>,
) -> String {
    match recovered.parse_tree.as_ref().clone().try_into_valid() {
        Ok(valid) => debug_output(&valid, indent),
        Err(_) => debug_output(recovered.parse_tree.as_ref(), indent),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jbotci_syntax::ParseOptions;

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
        .expect("an uninterrupted recovered parse returns a tree")
    }

    #[requires(true)]
    #[ensures(true)]
    fn parse_generated_syntax(source: &str) -> Box<jbotci_syntax::generated_model::TextSyntax> {
        let words = jbotci_morphology::segment_words_with_modifiers(source)
            .expect("generated syntax probes use valid morphology");
        jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            source,
            &ParseOptions::default(),
        )
        .expect("an uninterrupted generated parse returns a tree")
    }

    #[requires(true)]
    #[ensures(true)]
    fn plain_with_options(source: &str, options: BracketRenderOptions) -> String {
        crate::pretty_generated_model_plain_with_limit(
            &parse_generated_syntax(source),
            source,
            options,
            crate::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("plain output")
    }

    #[requires(true)]
    #[ensures(true)]
    fn fragment_contains_role(
        fragment: &crate::BracketSourceFragment,
        role: crate::BracketSourceFragmentRole,
    ) -> bool {
        match fragment {
            crate::BracketSourceFragment::Text { role: actual, .. } => *actual == role,
            crate::BracketSourceFragment::Span { children, .. } => children
                .iter()
                .any(|child| fragment_contains_role(child, role)),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn fragment_text(fragment: &crate::BracketSourceFragment) -> String {
        match fragment {
            crate::BracketSourceFragment::Text { text, .. } => text.clone(),
            crate::BracketSourceFragment::Span { children, .. } => children
                .iter()
                .map(fragment_text)
                .collect::<Vec<_>>()
                .join(""),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn plain_uses_the_same_normalized_words_and_elided_terminators_as_brackets() {
        let options = BracketRenderOptions {
            show_elided: true,
            ..BracketRenderOptions::default()
        };
        let source = "mi klama lo zarci";
        let tree = parse_generated_syntax(source);
        let brackets = crate::pretty_generated_model_brackets_with_limit(
            &tree,
            source,
            options,
            crate::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("brackets");
        assert_eq!(brackets, "(mi [kláma {lo zárci ku} vau])");
        assert_eq!(
            plain_with_options(source, options),
            "mi kláma lo zárci ku vau"
        );
        assert_eq!(
            plain_with_options("mi klama lo nu do klama", options),
            "mi kláma lo nu do kláma vau kei ku vau"
        );

        let fragments = crate::pretty_generated_model_plain_source_fragments_with_options(
            &tree, source, options,
        )
        .expect("plain fragments");
        assert_eq!(
            fragments
                .iter()
                .map(fragment_text)
                .collect::<Vec<_>>()
                .join(""),
            "mi kláma lo zárci ku vau"
        );
        assert!(fragments.iter().any(|fragment| matches!(
            fragment,
            crate::BracketSourceFragment::Span { range: Some(_), .. }
        )));
        assert!(fragments.iter().any(|fragment| {
            fragment_contains_role(fragment, crate::BracketSourceFragmentRole::Elided)
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn plain_honors_stress_glide_script_quotes_and_paragraphs() {
        assert_eq!(
            plain_with_options(
                "mi klama lo zarci",
                BracketRenderOptions {
                    phonemes: jbotci_morphology::PhonemeRenderOptions {
                        mark_stress: crate::StressMark::None,
                        mark_glides: crate::GlideMark::None,
                    },
                    ..BracketRenderOptions::default()
                }
            ),
            "mi klama lo zarci"
        );
        assert_eq!(
            plain_with_options("coi", BracketRenderOptions::default()),
            "coĭ"
        );
        assert_eq!(
            plain_with_options(
                "coi",
                BracketRenderOptions {
                    phonemes: jbotci_morphology::PhonemeRenderOptions {
                        mark_glides: crate::GlideMark::None,
                        ..jbotci_morphology::PhonemeRenderOptions::default()
                    },
                    ..BracketRenderOptions::default()
                }
            ),
            "coi"
        );
        assert_eq!(
            plain_with_options(
                "coi",
                BracketRenderOptions {
                    script: crate::LojbanScript::Zbalermorna,
                    ..BracketRenderOptions::default()
                }
            ),
            "\u{ed86}\u{eda8}"
        );
        assert_eq!(
            plain_with_options("zo. mi klama", BracketRenderOptions::default()),
            "zo mi kláma"
        );
        assert_eq!(
            plain_with_options(
                "mi klama lo zarci .i do djica lo nu pinxe lo djacu",
                BracketRenderOptions::default()
            ),
            "mi kláma lo zárci .i do djíca lo nu pínxe lo djácu"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn plain_color_resets_every_styled_leaf_and_every_line() {
        let color = BracketRenderOptions {
            color: true,
            ..BracketRenderOptions::default()
        };
        let elided = BracketRenderOptions {
            show_elided: true,
            ..color
        };
        for (source, options) in [
            ("coi", color),
            ("mi klama lo zarci", color),
            ("mi klama lo zarci", elided),
        ] {
            let rendered = plain_with_options(source, options);
            assert!(rendered.ends_with("\x1b[0m"), "{rendered:?}");
            for (index, _) in rendered.match_indices(' ') {
                assert!(rendered[..index].ends_with("\x1b[0m"), "{rendered:?}");
            }
        }
        assert_eq!(plain_with_options("coi", color), "\x1b[35mcoĭ\x1b[0m");

        let source = "mi ku .i do";
        let recovered = parse_recovered_syntax(source);
        let rendered = pretty_recovered_syntax_plain_with_limit(
            &recovered,
            source,
            color,
            crate::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("colored recovered plain");
        assert!(rendered.ends_with("\x1b[0m"), "{rendered:?}");
        for (index, _) in rendered.match_indices(' ') {
            assert!(rendered[..index].ends_with("\x1b[0m"), "{rendered:?}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colored_multiline_quotes_reset_bracket_and_plain_lines() {
        let options = BracketRenderOptions {
            color: true,
            ..BracketRenderOptions::default()
        };
        for source in [
            "zoi gy hello\nworld gy",
            "zoi gy hello\n\nworld gy",
            "zoi gy hello\r\nworld gy",
            "la'o gy hello\nworld gy",
        ] {
            let tree = parse_generated_syntax(source);
            let bracketed = crate::pretty_generated_model_brackets_with_limit(
                &tree,
                source,
                options,
                crate::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
                &jbotci_interrupt::CancellationToken::never(),
            )
            .expect("colored multiline quote brackets");
            let plain = plain_with_options(source, options);
            for rendered in [&bracketed, &plain] {
                assert!(
                    rendered
                        .lines()
                        .all(|line| line.is_empty() || line.ends_with("\x1b[0m")),
                    "{source:?}: {rendered:?}"
                );
                assert!(rendered.contains("hello\x1b[0m"), "{source:?}");
                assert!(rendered.contains("world"), "{source:?}");
            }
            if source == "zoi gy hello\n\nworld gy" {
                assert_eq!(plain.lines().nth(1), Some(""), "{plain:?}");
                assert_eq!(bracketed.lines().nth(1), Some(""), "{bracketed:?}");
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_multiline_error_restores_strikethrough_in_brackets_and_plain() {
        let source = "mi ku\nku .i do";
        let recovered = parse_recovered_syntax(source);
        assert!(!recovered.errors.is_empty());
        let options = BracketRenderOptions {
            color: true,
            ..BracketRenderOptions::default()
        };
        let bracketed = pretty_recovered_syntax_brackets_with_options(&recovered, source, options)
            .expect("multiline error brackets");
        let plain = pretty_recovered_syntax_plain_with_limit(
            &recovered,
            source,
            options,
            crate::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("multiline error plain");
        for rendered in [&bracketed, &plain] {
            assert!(
                rendered.contains("\n\x1b[91m\x1b[9mku\x1b[29m"),
                "{rendered:?}"
            );
            assert_eq!(rendered.matches('‼').count(), 2, "{rendered:?}");
            assert!(
                rendered
                    .lines()
                    .all(|line| line.is_empty() || line.ends_with("\x1b[0m")),
                "{rendered:?}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colored_quotes_keep_whitespace_and_cr_lines_unstyled() {
        let color = BracketRenderOptions {
            color: true,
            ..BracketRenderOptions::default()
        };
        let cases = [
            ("zoi gy hello\n   \nworld gy", "hello\n   \nworld"),
            ("zoi gy hello\n\r\r\nworld gy", "hello\n\r\r\nworld"),
            ("zoi gy hello\rworld gy", "hello\rworld"),
            ("la'o gy hello\n\t\nworld gy", "hello\n\t\nworld"),
        ];
        for (source, payload) in cases {
            let tree = parse_generated_syntax(source);
            let bracketed = crate::pretty_generated_model_brackets_with_limit(
                &tree,
                source,
                color,
                crate::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
                &jbotci_interrupt::CancellationToken::never(),
            )
            .expect("colored quote brackets");
            let plain = plain_with_options(source, color);
            let plain_brackets = crate::pretty_generated_model_brackets_with_limit(
                &tree,
                source,
                BracketRenderOptions::default(),
                crate::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
                &jbotci_interrupt::CancellationToken::never(),
            )
            .expect("plain quote brackets");
            let plain_text = plain_with_options(source, BracketRenderOptions::default());
            for rendered in [&bracketed, &plain] {
                assert_colored_break_lines(rendered);
                assert!(rendered.contains("hello"), "{source:?}: {rendered:?}");
                assert!(rendered.contains("world"), "{source:?}: {rendered:?}");
            }
            for rendered in [&plain_brackets, &plain_text] {
                assert!(rendered.contains(payload), "{source:?}: {rendered:?}");
                assert!(!rendered.contains('\u{1b}'), "{source:?}: {rendered:?}");
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_control_in_a_quote_is_a_visible_escape() {
        let source = "mi nelci zoi gy.a\u{1b}b\u{80}.gy";
        let tree = parse_generated_syntax(source);
        let cancel = jbotci_interrupt::CancellationToken::never();
        let limit = crate::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit");
        for color in [false, true] {
            let options = BracketRenderOptions {
                color,
                ..BracketRenderOptions::default()
            };
            let bracketed = crate::pretty_generated_model_brackets_with_limit(
                &tree, source, options, limit, &cancel,
            )
            .expect("control quote brackets");
            let plain = crate::pretty_generated_model_plain_with_limit(
                &tree, source, options, limit, &cancel,
            )
            .expect("control quote plain");
            for rendered in [&bracketed, &plain] {
                assert!(rendered.contains("a\\u{1b}b\\u{80}"), "{rendered:?}");
                assert!(!rendered.contains('\u{80}'), "{rendered:?}");
                if !color {
                    assert!(!rendered.contains('\u{1b}'), "{rendered:?}");
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_megabyte_quote_stops_at_32_bytes() {
        // A long run of letters makes the vowel parser recurse until the
        // stack overflows. `#` is one byte, it is not a letter, and it is
        // not a word separator, so this quote stays one megabyte of text.
        let source = format!("zoi gy.{}.gy", "#".repeat(1_000_000));
        let words =
            jbotci_morphology::segment_words_with_modifiers(&source).expect("a quote segments");
        let generated = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            &source,
            &ParseOptions::default(),
        )
        .expect("a quote parses");
        let recovered = jbotci_syntax::parse_syntax_tree_recovered_with_source_and_options(
            &words,
            &source,
            &ParseOptions::default(),
        )
        .expect("a recovered quote parses");
        let limit = crate::budget::ByteLimit::new(32).expect("positive limit");
        let cancel = jbotci_interrupt::CancellationToken::never();
        let options = BracketRenderOptions::default();
        let tree_options = crate::TreeRenderOptions::default();
        let leaves = crate::budget::OutputBudget::new(limit);
        let projections = [
            crate::brackets::generated_model_bracket_sexpr(
                &generated, &source, options, &leaves, &cancel,
            )
            .map(|_| ()),
            crate::tree::projected_generated_syntax_tree_value_with_index(
                &generated,
                &source,
                tree_options,
                &leaves,
                None,
            )
            .map(|_| ()),
            crate::tree::morphology_tree_value(&words[0], &source, tree_options, &leaves)
                .map(|_| ()),
        ];
        for projection in projections {
            assert!(
                matches!(projection, Err(crate::OutputError::Limit { .. })),
                "{projection:?}"
            );
        }
        for color in [false, true] {
            let options = BracketRenderOptions {
                color,
                ..BracketRenderOptions::default()
            };
            let results = [
                crate::pretty_generated_model_brackets_with_limit(
                    &generated, &source, options, limit, &cancel,
                ),
                crate::pretty_generated_model_plain_with_limit(
                    &generated, &source, options, limit, &cancel,
                ),
                pretty_recovered_syntax_brackets_with_limit(
                    &recovered, &source, options, limit, &cancel,
                ),
                pretty_recovered_syntax_plain_with_limit(
                    &recovered, &source, options, limit, &cancel,
                ),
                crate::pretty_morphology_brackets_with_limit(
                    &words, &source, options, limit, &cancel,
                ),
            ];
            for result in results {
                assert!(
                    matches!(result, Err(crate::OutputError::Limit { .. })),
                    "{color}: {result:?}"
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_projection_limit_does_not_shorten_a_quote() {
        let source = "mi ku .i mi nelci zoi gy.abcdefghijklmnop.gy";
        let recovered = parse_recovered_syntax(source);
        let options = JsonRenderOptions::default();
        let text_limit = ByteLimit::new(8_192).expect("positive text limit");
        let projection_limit = ByteLimit::new(8_192).expect("positive projection limit");
        let json = compact_recovered_syntax_json_string_with_limits(
            &recovered,
            source,
            options,
            text_limit,
            projection_limit,
        )
        .expect("both limits hold the quote");
        let value: Value = serde_json::from_str(&json).expect("json");
        let verbatim = verbatim_fields(&value).expect("verbatim");
        assert_eq!(verbatim["text"], "abcdefghijklmnop");
        assert_eq!(verbatim["span"], serde_json::json!([25, 41]));
        let small_projection = compact_recovered_syntax_json_string_with_limits(
            &recovered,
            source,
            options,
            text_limit,
            ByteLimit::new(4).expect("positive projection limit"),
        );
        assert!(
            matches!(small_projection, Err(OutputError::Limit { .. })),
            "{small_projection:?}"
        );
        let small_text = compact_recovered_syntax_json_string_with_limits(
            &recovered,
            source,
            options,
            ByteLimit::new(8).expect("positive text limit"),
            projection_limit,
        );
        assert!(
            matches!(small_text, Err(OutputError::Limit { .. })),
            "{small_text:?}"
        );

        let trimmed = "mi ku .i mi nelci zoi gy.abcd efgh.gy";
        let trimmed_recovered = parse_recovered_syntax(trimmed);
        let trimmed_small = compact_recovered_syntax_json_string_with_limits(
            &trimmed_recovered,
            trimmed,
            options,
            text_limit,
            ByteLimit::new(4).expect("positive projection limit"),
        );
        assert!(
            matches!(trimmed_small, Err(OutputError::Limit { .. })),
            "{trimmed_small:?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_short_limit_rejects_a_long_recovery_span() {
        let mut source = String::from("mi ku");
        source.push_str(&" ".repeat(1_000_000));
        source.push_str("ku .i do");
        let recovered = parse_recovered_syntax(&source);
        assert!(!recovered.errors.is_empty());
        let limit = ByteLimit::new(32).expect("positive limit");
        let text_limit = ByteLimit::new(8_192).expect("positive text limit");
        let cancel = jbotci_interrupt::CancellationToken::never();
        let options = BracketRenderOptions::default();
        let plain =
            pretty_recovered_syntax_plain_with_limit(&recovered, &source, options, limit, &cancel);
        assert!(matches!(plain, Err(OutputError::Limit { .. })), "{plain:?}");
        let brackets = pretty_recovered_syntax_brackets_with_limit(
            &recovered, &source, options, limit, &cancel,
        );
        assert!(
            matches!(brackets, Err(OutputError::Limit { .. })),
            "{brackets:?}"
        );
        let tree = pretty_recovered_syntax_tree_with_limit(
            &recovered,
            &source,
            TreeRenderOptions::default(),
            limit,
        );
        assert!(matches!(tree, Err(OutputError::Limit { .. })), "{tree:?}");
        let json = compact_recovered_syntax_json_string_with_limits(
            &recovered,
            &source,
            JsonRenderOptions::default(),
            text_limit,
            limit,
        );
        assert!(matches!(json, Err(OutputError::Limit { .. })), "{json:?}");
    }

    #[requires(true)]
    #[ensures(true)]
    fn verbatim_fields(value: &Value) -> Option<&Map<String, Value>> {
        match value {
            Value::Object(object) => {
                if let Some(Value::Object(verbatim)) = object.get("Verbatim") {
                    return Some(verbatim);
                }
                object.values().find_map(verbatim_fields)
            }
            Value::Array(items) => items.iter().find_map(verbatim_fields),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_oversized_spaced_quote_is_not_shortened() {
        let source = format!("mi nelci zoi gy.a{}b.gy", " ".repeat(8_192));
        assert_quote_limit(&source, ByteLimit::new(4_096).expect("positive tree limit"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_cut_inside_spaces_does_not_hide_the_overflow() {
        let source = format!("mi nelci zoi gy.a{}b.gy", " ".repeat(4_096));
        assert_quote_limit(&source, ByteLimit::new(4_096).expect("positive tree limit"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_shared_leaf_budget_stops_when_later_text_no_longer_fits() {
        let source = "mi ku .i mi nelci zoi gy.abcdefghijklmnop.gy";
        let recovered = parse_recovered_syntax(source);
        assert!(!recovered.errors.is_empty());
        // The recovery text and the quote share one budget. Sixteen bytes
        // hold either charge alone and do not hold both.
        let tight = tree::recovered_generated_model_tree_value(
            &recovered,
            source,
            TreeRenderOptions::default(),
            &crate::budget::OutputBudget::new(ByteLimit::new(16).expect("positive leaf budget")),
        );
        assert!(matches!(tight, Err(OutputError::Limit { .. })), "{tight:?}");
        let tree = tree::recovered_generated_model_tree_value(
            &recovered,
            source,
            TreeRenderOptions::default(),
            &crate::budget::OutputBudget::new(ByteLimit::new(64).expect("positive leaf budget")),
        )
        .expect("recovery text and quote fit");
        let rendered = format!("{tree:?}");
        assert!(rendered.contains("abcdefghijklmnop"), "{rendered}");
        let json = compact_recovered_syntax_json_string_with_limits(
            &recovered,
            source,
            JsonRenderOptions::default(),
            ByteLimit::new(8_192).expect("positive text limit"),
            ByteLimit::new(16).expect("positive projection limit"),
        );
        assert!(matches!(json, Err(OutputError::Limit { .. })), "{json:?}");
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_quote_limit(source: &str, limit: ByteLimit) {
        let words =
            jbotci_morphology::segment_words_with_modifiers(source).expect("quote segments");
        let generated = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            source,
            &ParseOptions::default(),
        )
        .expect("quote parses");
        let recovered = jbotci_syntax::parse_syntax_tree_recovered_with_source_and_options(
            &words,
            source,
            &ParseOptions::default(),
        )
        .expect("recovered quote parses");
        let cancel = jbotci_interrupt::CancellationToken::never();
        let options = BracketRenderOptions::default();
        let tree_options = TreeRenderOptions::default();
        let results = [
            crate::pretty_generated_model_tree_with_limit(
                &generated,
                source,
                tree_options,
                limit,
                &cancel,
            ),
            crate::pretty_generated_model_brackets_with_limit(
                &generated, source, options, limit, &cancel,
            ),
            crate::pretty_generated_model_plain_with_limit(
                &generated, source, options, limit, &cancel,
            ),
            pretty_recovered_syntax_tree_with_limit(&recovered, source, tree_options, limit),
            pretty_recovered_syntax_brackets_with_limit(
                &recovered, source, options, limit, &cancel,
            ),
            pretty_recovered_syntax_plain_with_limit(&recovered, source, options, limit, &cancel),
            crate::pretty_morphology_tree_with_limit(&words, source, tree_options, limit, &cancel),
            crate::pretty_morphology_brackets_with_limit(&words, source, options, limit, &cancel),
        ];
        for result in results {
            assert!(
                matches!(result, Err(OutputError::Limit { .. })),
                "{result:?}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colored_recovered_error_keeps_the_whitespace_line_unstyled() {
        let source = "mi ku\n   \nku .i do";
        let recovered = parse_recovered_syntax(source);
        assert!(!recovered.errors.is_empty());
        let color = BracketRenderOptions {
            color: true,
            ..BracketRenderOptions::default()
        };
        let bracketed = pretty_recovered_syntax_brackets_with_options(&recovered, source, color)
            .expect("colored error brackets");
        let plain = pretty_recovered_syntax_plain_with_limit(
            &recovered,
            source,
            color,
            crate::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("colored error plain");
        let plain_brackets = pretty_recovered_syntax_brackets_with_options(
            &recovered,
            source,
            BracketRenderOptions::default(),
        )
        .expect("plain error brackets");
        for rendered in [&bracketed, &plain] {
            assert_colored_break_lines(rendered);
            assert!(rendered.contains("\x1b[9m"), "{rendered:?}");
            assert!(rendered.contains('‼'), "{rendered:?}");
        }
        assert!(plain_brackets.contains("\n   \n"), "{plain_brackets:?}");
        assert!(!plain_brackets.contains('\u{1b}'), "{plain_brackets:?}");
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_colored_break_lines(rendered: &str) {
        for line in split_all_breaks(rendered) {
            let visible = strip_ansi(line);
            if visible.chars().all(char::is_whitespace) {
                assert!(!line.contains('\u{1b}'), "{line:?} in {rendered:?}");
            } else if line.contains('\u{1b}') {
                assert!(line.ends_with("\x1b[0m"), "{line:?} in {rendered:?}");
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn split_all_breaks(text: &str) -> Vec<&str> {
        let mut lines = Vec::new();
        let bytes = text.as_bytes();
        let mut start = 0;
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'\n' || bytes[index] == b'\r' {
                lines.push(&text[start..index]);
                if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                    index += 2;
                } else {
                    index += 1;
                }
                start = index;
            } else {
                index += 1;
            }
        }
        lines.push(&text[start..]);
        lines
    }

    #[requires(true)]
    #[ensures(true)]
    fn strip_ansi(text: &str) -> String {
        let mut visible = String::new();
        let mut chars = text.chars();
        while let Some(character) = chars.next() {
            if character == '\u{1b}' {
                for next in chars.by_ref() {
                    if next == 'm' {
                        break;
                    }
                }
            } else {
                visible.push(character);
            }
        }
        visible
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

        let raw = pretty_recovered_morphology_raw(&recovered, source, Some(Indentation::compact()))
            .expect("raw");
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
                pretty_recovered_morphology_raw(
                    &recovered_morphology,
                    source,
                    Some(Indentation::compact())
                )
                .expect("recovered morphology raw"),
                debug_output(&words, Some(Indentation::compact())),
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
                pretty_recovered_syntax_raw(&recovered, Some(Indentation::compact())),
                debug_output(strict.as_ref(), Some(Indentation::compact())),
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

#[requires(indent.is_none_or(|indent| indent.spaces() == 0))]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn pretty_recovered_syntax_raw_with_limit(
    recovered: &RecoveredSyntaxParse,
    indent: Option<Indentation>,
    limit: ByteLimit,
) -> Result<String, OutputError> {
    match recovered.parse_tree.as_ref().clone().try_into_valid() {
        Ok(valid) => raw_with_limit(&valid, indent, limit),
        Err(_) => raw_with_limit(recovered.parse_tree.as_ref(), indent, limit),
    }
}

#[requires(indent.is_none_or(|indent| indent.spaces() == 0))]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn pretty_recovered_morphology_raw_with_limit(
    recovered: &RecoveredMorphologySegmentation,
    source: &str,
    indent: Option<Indentation>,
    limit: ByteLimit,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> Result<String, OutputError> {
    // Debug formatting of an already-built list has no inner checkpoint.
    // The load after it is the one a late flag must turn into an interrupt.
    cancellation.check().map_err(|_| OutputError::Interrupted)?;
    let rendered = if recovered.errors.is_empty() {
        raw_with_limit(&recovered.words, indent, limit)?
    } else {
        raw_with_limit(
            &recovered_morphology_sequence(
                recovered,
                source,
                &crate::budget::OutputBudget::new(limit),
            )?,
            indent,
            limit,
        )?
    };
    cancellation.check().map_err(|_| OutputError::Interrupted)?;
    Ok(rendered)
}

#[requires(indent.is_none_or(|indent| indent.spaces() == 0))]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn raw_with_limit(
    value: &impl std::fmt::Debug,
    indent: Option<Indentation>,
    limit: ByteLimit,
) -> Result<String, OutputError> {
    let rendered = if indent == Some(Indentation::compact()) {
        crate::budget::bounded_format(format_args!("{value:?}\n"), limit)
    } else {
        crate::budget::bounded_format(format_args!("{value:#?}\n"), limit)
    };
    rendered.map_err(|error| OutputError::from_render_io(error, limit))
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
pub(crate) fn recovered_morphology_sequence_for_json_test(
    recovered: &RecoveredMorphologySegmentation,
    source: &str,
) -> Value {
    Value::Array(
        recovered_morphology_sequence(recovered, source, &crate::budget::OutputBudget::unlimited())
            .unwrap()
            .into_iter()
            .map(|item| match item.into_data() {
                data!(RecoveredMorphologySequenceItem::Valid { word }) => {
                    json::morphology_word_like_value(word, crate::PhonemeRenderOptions::default())
                }
                data!(RecoveredMorphologySequenceItem::Error { item }) => {
                    recovery_error_json_value(&item)
                }
            })
            .collect(),
    )
}
