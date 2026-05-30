//! Output format selection and render facade.

mod brackets;
mod diagnostics;
mod json;
mod places;
mod references;
mod sexpr;
mod surface;
mod trace;
mod tree;

use bityzba::{invariant, requires};
pub use diagnostics::{
    DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH, DiagnosticRenderOptions, render_diagnostics,
};
pub use jbotci_diagnostics::DiagnosticDetailMode;
use jbotci_morphology::WordLike;
pub use jbotci_morphology::{GlideMark, PhonemeRenderOptions, StressMark};
use jbotci_syntax::ast::TextSyntax;
pub use places::format_definition_or_notes_line_with_indexed_places;
pub use references::{
    ReferenceAnnotations, ReferenceDisplayModel, ReferenceName, ReferenceSlotName,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
pub use trace::{TraceRenderOptions, render_trace_report};
pub use tree::reference_display_model_for_syntax_tree;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct BracketSourceRange {
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
#[invariant(true)]
#[invariant(::Text { .. } => true)]
#[invariant(::Span { .. } => true)]
pub enum BracketSourceFragment {
    Text {
        text: String,
        range: Option<BracketSourceRange>,
        elided: bool,
    },
    Span {
        range: Option<BracketSourceRange>,
        children: Vec<BracketSourceFragment>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputBase {
    Compact,
    Ipa,
    Tree,
    Raw,
    Camxes,
    Svg,
    Gloss,
    Xml,
    MermaidFlowchart,
    MermaidBlock,
    Markdown,
    Lean,
    Paraphrase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFeature {
    WordKind,
    Definitions,
    Color,
    CompactXml,
    Gloss,
    LeanPrelude,
    LeanUnicode,
    LeanSyntheticNames,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct OutputFormat {
    pub base: OutputBase,
    pub features: Vec<OutputFeature>,
}

impl Default for OutputFormat {
    #[requires(true)]
    #[ensures(ret.base == OutputBase::Compact && ret.features.is_empty())]
    fn default() -> Self {
        Self {
            base: OutputBase::Compact,
            features: Vec::new(),
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Diagnostic(..) => true)]
#[invariant(::Json(..) => true)]
#[invariant(::Ipa(..) => true)]
#[invariant(::References(..) => true)]
pub enum OutputError {
    #[error("output rendering is not implemented yet")]
    NotImplemented,
    #[error("failed to render diagnostic: {0}")]
    Diagnostic(String),
    #[error("failed to encode compact JSON: {0}")]
    Json(String),
    #[error("failed to render IPA: {0}")]
    Ipa(String),
    #[error("failed to analyze semantic references: {0}")]
    References(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum GlyphStyle {
    #[default]
    Unicode,
    Ascii,
}

impl GlyphStyle {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn arrow(self) -> &'static str {
        match self {
            Self::Unicode => "→",
            Self::Ascii => "->",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn slot_open(self) -> &'static str {
        match self {
            Self::Unicode => "⟨",
            Self::Ascii => "<",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn slot_close(self) -> &'static str {
        match self {
            Self::Unicode => "⟩",
            Self::Ascii => ">",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn span_leader(self) -> &'static str {
        match self {
            Self::Unicode => "‥",
            Self::Ascii => "..",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn lujvo_separator(self) -> &'static str {
        match self {
            Self::Unicode => "·",
            Self::Ascii => "~",
        }
    }

    #[requires(value > 0)]
    #[ensures(!ret.is_empty())]
    pub fn numeric_suffix(self, value: usize) -> String {
        match self {
            Self::Unicode => subscript_number(value),
            Self::Ascii => value.to_string(),
        }
    }
}

#[requires(value > 0)]
#[ensures(!ret.is_empty())]
fn subscript_number(value: usize) -> String {
    value.to_string().chars().map(subscript_digit).collect()
}

#[requires(character.is_ascii_digit())]
#[ensures(true)]
fn subscript_digit(character: char) -> char {
    match character {
        '0' => '₀',
        '1' => '₁',
        '2' => '₂',
        '3' => '₃',
        '4' => '₄',
        '5' => '₅',
        '6' => '₆',
        '7' => '₇',
        '8' => '₈',
        '9' => '₉',
        _ => unreachable!("requires ASCII digit"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[invariant(true)]
pub struct BracketRenderOptions {
    pub color: bool,
    pub phonemes: PhonemeRenderOptions,
    pub glyphs: GlyphStyle,
    pub decompose_lujvo: bool,
    pub insert_hair_space: bool,
    pub show_elided: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct JsonRenderOptions {
    pub indent: usize,
    pub phonemes: PhonemeRenderOptions,
    pub show_elided: bool,
}

impl Default for JsonRenderOptions {
    #[requires(true)]
    #[ensures(ret.indent == 2)]
    #[ensures(ret.phonemes == PhonemeRenderOptions::default())]
    #[ensures(!ret.show_elided)]
    fn default() -> Self {
        Self {
            indent: 2,
            phonemes: PhonemeRenderOptions::default(),
            show_elided: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct TreeRenderOptions {
    pub color: bool,
    pub indent: usize,
    pub phonemes: PhonemeRenderOptions,
    pub glyphs: GlyphStyle,
    pub show_spans: bool,
    pub show_refs: bool,
    pub decompose_lujvo: bool,
    pub show_elided: bool,
}

impl Default for TreeRenderOptions {
    #[requires(true)]
    #[ensures(!ret.color)]
    #[ensures(ret.indent == 2)]
    #[ensures(ret.phonemes == PhonemeRenderOptions::default())]
    #[ensures(ret.glyphs == GlyphStyle::default())]
    #[ensures(!ret.show_spans)]
    #[ensures(!ret.show_refs)]
    #[ensures(!ret.decompose_lujvo)]
    #[ensures(!ret.show_elided)]
    fn default() -> Self {
        Self {
            color: false,
            indent: 2,
            phonemes: PhonemeRenderOptions::default(),
            glyphs: GlyphStyle::default(),
            show_spans: false,
            show_refs: false,
            decompose_lujvo: false,
            show_elided: false,
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| !matches!(value, Value::Null)) || ret.is_err())]
pub fn compact_json_value<T: Serialize>(value: &T) -> Result<Value, OutputError> {
    let mut bytes = Vec::new();
    let mut serializer = serde_json::Serializer::new(&mut bytes);
    let serializer = serde_stacker::Serializer::new(&mut serializer);
    value
        .serialize(serializer)
        .map_err(|source| OutputError::Json(source.to_string()))?;
    serde_json::from_slice(&bytes)
        .map(compact_json_shape)
        .map_err(|source| OutputError::Json(source.to_string()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn compact_json_string<T: Serialize>(value: &T) -> Result<String, OutputError> {
    compact_json_string_with_options(value, JsonRenderOptions::default())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn compact_json_string_with_options<T: Serialize>(
    value: &T,
    options: JsonRenderOptions,
) -> Result<String, OutputError> {
    Ok(format_compact_json_value(
        &compact_json_value(value)?,
        0,
        options,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| !matches!(value, Value::Null)) || ret.is_err())]
pub fn compact_morphology_json_value(words: &[WordLike]) -> Result<Value, OutputError> {
    Ok(json::morphology_json_value(
        words,
        PhonemeRenderOptions::default(),
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn compact_morphology_json_string_with_options(
    words: &[WordLike],
    options: JsonRenderOptions,
) -> Result<String, OutputError> {
    Ok(format_compact_json_value(
        &json::morphology_json_value(words, options.phonemes),
        0,
        options,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn ipa_morphology_text(words: &[WordLike], source: &str) -> Result<String, OutputError> {
    surface::format_words_ipa(words, source)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| !matches!(value, Value::Null)) || ret.is_err())]
pub fn compact_syntax_json_value(tree: &TextSyntax) -> Result<Value, OutputError> {
    Ok(json::syntax_json_value(tree, JsonRenderOptions::default()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn compact_syntax_json_string_with_options(
    tree: &TextSyntax,
    options: JsonRenderOptions,
) -> Result<String, OutputError> {
    Ok(format_compact_json_value(
        &json::syntax_json_value(tree, options),
        0,
        options,
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub fn pretty_brackets(tree: &TextSyntax, source: &str) -> Result<String, OutputError> {
    pretty_brackets_with_options(tree, source, BracketRenderOptions::default())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub fn pretty_tree(tree: &TextSyntax, source: &str) -> Result<String, OutputError> {
    pretty_tree_with_options(tree, source, TreeRenderOptions::default())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub fn pretty_morphology_tree(words: &[WordLike], source: &str) -> Result<String, OutputError> {
    pretty_morphology_tree_with_options(words, source, TreeRenderOptions::default())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub fn pretty_morphology_tree_with_options(
    words: &[WordLike],
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    tree::pretty_morphology_tree_with_options(words, source, options)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub fn pretty_tree_with_options(
    tree: &TextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    tree::pretty_tree_with_options(tree, source, options)
}

#[requires(true)]
#[ensures(true)]
fn compact_json_shape(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(compact_json_shape)
                .filter(|value| !is_omitted_compact_value(value))
                .collect(),
        ),
        Value::Object(mut object) => {
            if let Some(span) = compact_span_object(&object) {
                return span;
            }
            if let Some(value) = compact_constructor_object(&object) {
                return value;
            }
            if let Some(Value::String(kind)) = object.remove("kind") {
                if let Some(constructor) = compact_constructor_name(&kind) {
                    let payload = object
                        .into_iter()
                        .filter_map(|(key, value)| {
                            let value = compact_json_shape(value);
                            (!is_omitted_compact_value(&value)).then_some((key, value))
                        })
                        .collect();
                    return Value::Object(
                        [(constructor.to_owned(), Value::Object(payload))]
                            .into_iter()
                            .collect(),
                    );
                }
                object.insert("kind".to_owned(), Value::String(kind));
            }
            let mut compacted = object
                .into_iter()
                .filter_map(|(key, value)| {
                    let value = compact_json_shape(value);
                    (!is_omitted_compact_value(&value)).then_some((key, value))
                })
                .collect::<serde_json::Map<_, _>>();
            if let Some(leading_indicators) = compacted.remove("leading_indicators") {
                let leading_indicators = compact_leading_indicators(leading_indicators);
                if !is_omitted_compact_value(&leading_indicators) {
                    compacted.insert("leading_indicators".to_owned(), leading_indicators);
                }
            }
            if let Some(value) = compact_constructor_object(&compacted) {
                return value;
            }
            Value::Object(compacted)
        }
        other => other,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_compact_json_value(value: &Value, indent: usize, options: JsonRenderOptions) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            serde_json::to_string(value).expect("serializing JSON scalar cannot fail")
        }
        Value::Array(items) => format_compact_json_array(items, indent, options),
        Value::Object(object) if is_constructor_object(object) => {
            format_compact_json_constructor(object, indent, options)
        }
        Value::Object(object) => format_compact_json_object(object, indent, options),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_compact_json_field_value(
    value: &Value,
    field_indent: usize,
    options: JsonRenderOptions,
) -> String {
    match value {
        Value::Object(object) if is_constructor_object(object) => {
            format_compact_json_constructor(object, field_indent, options)
        }
        _ => format_compact_json_value(value, field_indent, options),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_compact_json_array(items: &[Value], indent: usize, options: JsonRenderOptions) -> String {
    if items.is_empty() {
        return "[]".to_owned();
    }
    if options.indent == 0 || items.iter().all(is_compact_json_scalar) {
        let separator = if options.indent == 0 { "," } else { ", " };
        let items = items
            .iter()
            .map(|item| format_compact_json_value(item, indent, options))
            .collect::<Vec<_>>()
            .join(separator);
        return format!("[{items}]");
    }

    let item_indent = indent + options.indent;
    let pad = " ".repeat(item_indent);
    let end = " ".repeat(indent);
    let mut output = String::from("[\n");
    for (index, item) in items.iter().enumerate() {
        output.push_str(&pad);
        output.push_str(&format_compact_json_value(item, item_indent, options));
        if index + 1 != items.len() {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str(&end);
    output.push(']');
    output
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_compact_json_object(
    object: &serde_json::Map<String, Value>,
    indent: usize,
    options: JsonRenderOptions,
) -> String {
    if object.is_empty() {
        return "{}".to_owned();
    }
    if options.indent == 0 {
        let fields = object
            .iter()
            .map(|(key, value)| {
                format!(
                    "{}:{}",
                    json_string(key),
                    format_compact_json_field_value(value, indent, options)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        return format!("{{{fields}}}");
    }

    let field_indent = indent + options.indent;
    let pad = " ".repeat(field_indent);
    let end = " ".repeat(indent);
    let mut output = String::from("{\n");
    for (index, (key, value)) in object.iter().enumerate() {
        output.push_str(&pad);
        output.push_str(&json_string(key));
        output.push_str(": ");
        output.push_str(&format_compact_json_field_value(
            value,
            field_indent,
            options,
        ));
        if index + 1 != object.len() {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str(&end);
    output.push('}');
    output
}

#[requires(is_constructor_object(object))]
#[ensures(!ret.is_empty())]
fn format_compact_json_constructor(
    object: &serde_json::Map<String, Value>,
    constructor_indent: usize,
    options: JsonRenderOptions,
) -> String {
    let (constructor, payload) = object.iter().next().expect("constructor object has item");
    let constructor = json_string(constructor);
    match payload {
        Value::Object(fields) if fields.is_empty() && options.indent == 0 => {
            format!("{{{constructor}:{{}}}}")
        }
        Value::Object(fields) if fields.is_empty() => format!("{{{constructor}: {{}}}}"),
        Value::Object(fields) if options.indent == 0 => {
            let fields = fields
                .iter()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        json_string(key),
                        format_compact_json_field_value(value, constructor_indent, options)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{constructor}:{{{fields}}}}}")
        }
        Value::Object(fields) => {
            let field_indent = constructor_indent + options.indent;
            let pad = " ".repeat(field_indent);
            let end = " ".repeat(constructor_indent);
            let mut output = format!("{{{constructor}: {{\n");
            for (index, (key, value)) in fields.iter().enumerate() {
                output.push_str(&pad);
                output.push_str(&json_string(key));
                output.push_str(": ");
                output.push_str(&format_compact_json_field_value(
                    value,
                    field_indent,
                    options,
                ));
                if index + 1 != fields.len() {
                    output.push(',');
                }
                output.push('\n');
            }
            output.push_str(&end);
            output.push_str("}}");
            output
        }
        other if options.indent == 0 => format!(
            "{{{constructor}:{}}}",
            format_compact_json_value(other, constructor_indent, options)
        ),
        other => format!(
            "{{{constructor}: {}}}",
            format_compact_json_value(other, constructor_indent + options.indent, options)
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn is_constructor_object(object: &serde_json::Map<String, Value>) -> bool {
    object.len() == 1
        && object
            .keys()
            .next()
            .is_some_and(|key| key.chars().next().is_some_and(char::is_uppercase))
}

#[requires(true)]
#[ensures(true)]
fn is_compact_json_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

#[requires(true)]
#[ensures(ret.starts_with('"') && ret.ends_with('"'))]
fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing JSON string cannot fail")
}

#[requires(true)]
#[ensures(true)]
fn compact_leading_indicators(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .flat_map(compact_leading_indicator)
                .filter(|value| !is_omitted_compact_value(value))
                .collect(),
        ),
        other => other,
    }
}

#[requires(true)]
#[ensures(true)]
fn compact_leading_indicator(value: Value) -> Vec<Value> {
    let value = compact_json_shape(value);
    let Value::Object(object) = &value else {
        return vec![value];
    };
    if object.len() != 1 {
        return vec![value];
    }
    let Some((constructor, payload)) = object.iter().next() else {
        return vec![value];
    };
    if constructor != "WithIndicator" {
        return vec![value];
    }
    let Value::Object(payload) = payload else {
        return vec![value];
    };
    let Some(base) = payload.get("base") else {
        return vec![value];
    };
    let mut indicators = compact_leading_indicator(base.clone());
    if indicators.is_empty() || !indicators.iter().all(is_indicator_record) {
        return vec![value];
    }
    let mut current = serde_json::Map::new();
    if let Some(indicator) = payload.get("indicator") {
        current.insert(
            "indicator".to_owned(),
            compact_json_shape(indicator.clone()),
        );
    }
    if let Some(nai) = payload.get("nai") {
        let nai = compact_json_shape(nai.clone());
        if !is_omitted_compact_value(&nai) {
            current.insert("nai".to_owned(), nai);
        }
    }
    indicators.push(Value::Object(current));
    indicators
}

#[requires(true)]
#[ensures(true)]
fn is_indicator_record(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(object) if object.contains_key("indicator")
    )
}

#[requires(true)]
#[ensures(true)]
fn compact_constructor_object(object: &serde_json::Map<String, Value>) -> Option<Value> {
    if object.len() != 1 {
        return None;
    }
    let (constructor, payload) = object.iter().next()?;
    match constructor.as_str() {
        "Bare" => {
            let Value::Object(payload) = payload else {
                return None;
            };
            if let Some(value) = payload.get("word") {
                return Some(compact_constructor_value(constructor, value.clone()));
            }
            None
        }
        "GekSentence" => single_payload_field(constructor, payload, "gek_sentence"),
        "Argument" => single_payload_field(constructor, payload, "argument"),
        "BeiLink" => single_payload_field(constructor, payload, "bei_only_links"),
        "RelativeClause" => single_payload_field(constructor, payload, "relative_clauses"),
        "MathExpression" => single_payload_field(constructor, payload, "math_expression"),
        "Relation" => single_payload_field(constructor, payload, "relation"),
        "Descriptor" => single_payload_field(constructor, payload, "descriptor"),
        "ConnectedDescriptor" => single_payload_field(constructor, payload, "connected_descriptor"),
        "Base" => single_payload_field(constructor, payload, "word"),
        "Abstraction" => single_payload_field(constructor, payload, "abstraction"),
        "Compound" => single_payload_field(constructor, payload, "units"),
        "Wrapped" => single_payload_field(constructor, payload, "relation"),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn single_payload_field(constructor: &str, payload: &Value, field: &str) -> Option<Value> {
    let Value::Object(payload) = payload else {
        return None;
    };
    if payload.len() != 1 {
        return None;
    }
    payload
        .get(field)
        .cloned()
        .map(|value| compact_constructor_value(constructor, value))
}

#[requires(true)]
#[ensures(true)]
fn compact_constructor_value(constructor: &str, payload: Value) -> Value {
    Value::Object(
        [(constructor.to_owned(), compact_json_shape(payload))]
            .into_iter()
            .collect(),
    )
}

#[requires(true)]
#[ensures(true)]
fn compact_span_object(object: &serde_json::Map<String, Value>) -> Option<Value> {
    if !object.contains_key("source_id")
        || !object.contains_key("byte_start")
        || !object.contains_key("byte_end")
        || !object.contains_key("char_start")
        || !object.contains_key("char_end")
    {
        return None;
    }
    let char_start = object.get("char_start")?.as_u64()?;
    let char_end = object.get("char_end")?.as_u64()?;
    Some(Value::Array(vec![char_start.into(), char_end.into()]))
}

#[requires(true)]
#[ensures(true)]
fn compact_constructor_name(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "rafsi" => "Rafsi",
        "hyphen" => "Hyphen",
        _ => return None,
    })
}

#[requires(true)]
#[ensures(true)]
fn is_omitted_compact_value(value: &Value) -> bool {
    matches!(value, Value::Null) || matches!(value, Value::Array(items) if items.is_empty())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub fn pretty_brackets_with_options(
    tree: &TextSyntax,
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    brackets::pretty_brackets_with_options(tree, source, options)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|fragments| !fragments.is_empty()))]
pub fn pretty_bracket_source_fragments_with_options(
    tree: &TextSyntax,
    source: &str,
    options: BracketRenderOptions,
) -> Result<Vec<BracketSourceFragment>, OutputError> {
    brackets::pretty_bracket_source_fragments_with_options(tree, source, options)
}

#[requires(true)]
#[ensures(words.is_empty() || ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub fn pretty_morphology_brackets_with_options(
    words: &[WordLike],
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    brackets::pretty_morphology_brackets_with_options(words, source, options)
}

#[cfg(test)]
mod tests {
    use bityzba::requires;
    use jbotci_morphology::segment_words_with_modifiers;
    use jbotci_syntax::parse_syntax_tree;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn glyph_style_maps_unicode_and_ascii_tokens() {
        assert_eq!(GlyphStyle::Unicode.arrow(), "→");
        assert_eq!(GlyphStyle::Unicode.slot_open(), "⟨");
        assert_eq!(GlyphStyle::Unicode.slot_close(), "⟩");
        assert_eq!(GlyphStyle::Unicode.span_leader(), "‥");
        assert_eq!(GlyphStyle::Unicode.numeric_suffix(12), "₁₂");
        assert_eq!(GlyphStyle::Unicode.lujvo_separator(), "·");

        assert_eq!(GlyphStyle::Ascii.arrow(), "->");
        assert_eq!(GlyphStyle::Ascii.slot_open(), "<");
        assert_eq!(GlyphStyle::Ascii.slot_close(), ">");
        assert_eq!(GlyphStyle::Ascii.span_leader(), "..");
        assert_eq!(GlyphStyle::Ascii.numeric_suffix(12), "12");
        assert_eq!(GlyphStyle::Ascii.lujvo_separator(), "~");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_v0_ipa_examples() {
        let cases = [
            ("klama", "ˈkla.ma"),
            ("tavla", "ˈta.vla"),
            ("coi", "ʃoj"),
            ("i", "ʔi"),
            ("oi", "ʔoj"),
            ("ui", "ʔwi"),
            ("ie", "ʔje"),
            ("ba'e", "ˈba.he"),
            ("e'u bridi", "ˈʔe.hu ˈbri.di"),
            ("la alis", "la ˈʔa.lis"),
            (".alis.", "ˈʔa.lisʔ"),
            ("i la diskord", "ʔi la ˈʔdi.skord"),
            (".armstrong.", "ˈʔa.rm.strongʔ"),
            ("bastn.", "ʔbas.tnʔ"),
            (".finyks.", "ʔfi.nəksʔ"),
            ("i la diskord jdice", "ʔi la ˈʔdi.skord ˈʔʒdi.ʃe"),
            ("diskord i", "ˈʔdi.skord ʔi"),
            (
                "nicte je xekri je blanu .i oi lo ca skari cu slabu",
                "ˈni.ʃte ʒe ˈxe.kri ʒe ˈbla.nu ʔi ʔoj lo ʃa ˈska.ri ʃu ˈsla.bu",
            ),
            ("mi .ui", "mi ʔwi"),
            ("zo si", "zo si"),
            ("lo'u mi le'u", "ˈlo.hu mi ˈle.hu"),
            ("mi bu", "mi bu"),
            ("mi zei do", "mi zej do"),
            ("zoi gy raw_payload gy", "zoj gə raw_payload gə"),
        ];

        for (source, expected) in cases {
            assert_eq!(render_ipa(source), expected, "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ipa_output_reports_empty_pronounceable_text() {
        for source in ["si", "fa'o"] {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let error = ipa_morphology_text(&words, source).expect_err("empty IPA rejected");
            assert!(
                error.to_string().contains("no pronounceable words"),
                "{error}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn default_gentufa_rendering_does_not_show_elided_terminators() {
        let source = "mi klama";
        let parsed = parse(source);
        assert!(!pretty_tree(&parsed, source).expect("tree").contains("vau"));
        assert!(
            !pretty_brackets(&parsed, source)
                .expect("brackets")
                .contains("vau")
        );
        assert!(
            !compact_syntax_json_string_with_options(&parsed, JsonRenderOptions::default())
                .expect("json")
                .contains("\"elided\"")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn text_renderers_show_representative_elided_terminators() {
        let tree = render_tree_with_elided("mi klama");
        assert!(tree.contains("vau: Cmavo @[8‥8) \"vau\""), "{tree}");

        let descriptor = render_tree_with_elided("le broda");
        assert!(
            descriptor.contains("ku: Cmavo @[8‥8) \"ku\""),
            "{descriptor}"
        );

        let abstraction = render_tree_with_elided("lo nu mi klama");
        assert!(
            abstraction.contains("kei: Cmavo @[14‥14) \"kei\""),
            "{abstraction}"
        );
        assert!(
            abstraction.contains("ku: Cmavo @[14‥14) \"ku\""),
            "{abstraction}"
        );

        let free_modifier = render_brackets_with_elided("to coi");
        assert!(free_modifier.contains("toi"), "{free_modifier}");

        let mekso = render_tree_with_elided("li pa");
        assert!(mekso.contains("boi: Cmavo @[5‥5) \"boi\""), "{mekso}");
        assert!(mekso.contains("loho: Cmavo @[5‥5) \"lo'o\""), "{mekso}");

        let colorized_tree = render_tree_with_elided_and_color("mi klama");
        assert!(
            colorized_tree.contains("\x1b[9m\"vau\"\x1b[29m"),
            "{colorized_tree:?}"
        );

        let colorized_brackets = render_brackets_with_elided_and_color("to coi");
        assert!(
            colorized_brackets.contains("\x1b[9mtoi\x1b[29m"),
            "{colorized_brackets:?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn json_renderer_marks_elided_terminators_with_zero_length_spans() {
        let parsed = parse("li pa");
        let json = compact_syntax_json_string_with_options(
            &parsed,
            JsonRenderOptions {
                show_elided: true,
                ..JsonRenderOptions::default()
            },
        )
        .expect("json");
        let value = serde_json::from_str(&json).expect("valid json");
        assert!(has_elided_cmavo(&value, "boi", [5, 5]), "{json}");
        assert!(has_elided_cmavo(&value, "lo'o", [5, 5]), "{json}");
    }

    #[requires(!source.is_empty())]
    #[ensures(!ret.is_empty())]
    fn render_ipa(source: &str) -> String {
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        ipa_morphology_text(&words, source).expect("IPA output")
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn parse(source: &str) -> TextSyntax {
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        parse_syntax_tree(&words)
            .expect("valid syntax")
            .parse_tree
            .as_ref()
            .clone()
    }

    #[requires(!source.is_empty())]
    #[ensures(!ret.is_empty())]
    fn render_tree_with_elided(source: &str) -> String {
        render_tree_with_elided_options(source, false)
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn render_tree_with_elided_and_color(source: &str) -> String {
        render_tree_with_elided_options(source, true)
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn render_tree_with_elided_options(source: &str, color: bool) -> String {
        let parsed = parse(source);
        pretty_tree_with_options(
            &parsed,
            source,
            TreeRenderOptions {
                color,
                show_elided: true,
                show_spans: true,
                ..TreeRenderOptions::default()
            },
        )
        .expect("tree")
    }

    #[requires(!source.is_empty())]
    #[ensures(!ret.is_empty())]
    fn render_brackets_with_elided(source: &str) -> String {
        render_brackets_with_elided_options(source, false)
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn render_brackets_with_elided_and_color(source: &str) -> String {
        render_brackets_with_elided_options(source, true)
    }

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn render_brackets_with_elided_options(source: &str, color: bool) -> String {
        let parsed = parse(source);
        pretty_brackets_with_options(
            &parsed,
            source,
            BracketRenderOptions {
                color,
                show_elided: true,
                ..BracketRenderOptions::default()
            },
        )
        .expect("brackets")
    }

    #[requires(true)]
    #[ensures(true)]
    fn has_elided_cmavo(value: &Value, phonemes: &str, span: [usize; 2]) -> bool {
        match value {
            Value::Object(object) => {
                object.get("phonemes").and_then(Value::as_str) == Some(phonemes)
                    && object.get("elided").and_then(Value::as_bool) == Some(true)
                    && object
                        .get("span")
                        .is_some_and(|value| span_matches(value, span))
                    || object
                        .values()
                        .any(|value| has_elided_cmavo(value, phonemes, span))
            }
            Value::Array(items) => items
                .iter()
                .any(|value| has_elided_cmavo(value, phonemes, span)),
            _ => false,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn span_matches(value: &Value, span: [usize; 2]) -> bool {
        let Value::Array(items) = value else {
            return false;
        };
        items.len() == 2
            && items[0].as_u64() == Some(span[0] as u64)
            && items[1].as_u64() == Some(span[1] as u64)
    }
}
