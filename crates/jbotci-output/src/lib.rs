//! Output format selection and render facade.

mod brackets;
mod sexpr;
mod surface;
mod tree;

use bityzba::{invariant, requires};
use jbotci_morphology::{Word, WordLike};
use jbotci_source::SourceSpan;
use jbotci_syntax::WithIndicators;
use jbotci_syntax::ast::TextSyntax;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
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
#[invariant(true)]
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
pub enum OutputError {
    #[error("output rendering is not implemented yet")]
    NotImplemented,
    #[error("invalid syntax tree for bracket rendering: {0}")]
    InvalidSyntaxTree(String),
    #[error("failed to encode compact JSON: {0}")]
    Json(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[invariant(true)]
pub struct BracketRenderOptions {
    pub color: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct JsonRenderOptions {
    pub indent: usize,
}

impl Default for JsonRenderOptions {
    #[requires(true)]
    #[ensures(ret.indent == 2)]
    fn default() -> Self {
        Self { indent: 2 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct TreeRenderOptions {
    pub color: bool,
    pub indent: usize,
}

impl Default for TreeRenderOptions {
    #[requires(true)]
    #[ensures(!ret.color)]
    #[ensures(ret.indent == 2)]
    fn default() -> Self {
        Self {
            color: false,
            indent: 2,
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
#[ensures(true)]
fn compact_value_sexpr(value: &Value, source: &str) -> Result<sexpr::SExpr, OutputError> {
    if is_omitted_compact_value(value) {
        return Ok(sexpr::empty_node());
    }
    if let Some(word) = compact_word(value) {
        let word = WithIndicators::bare(WordLike::bare(word));
        return Ok(sexpr::leaf(surface::format_with_indicators(&word, source)));
    }
    if let Some(word) = compact_with_indicators_surface(value, source)? {
        return Ok(sexpr::leaf(word));
    }
    if let Some(sexpr) = compact_special_constructor_sexpr(value, source)? {
        return Ok(sexpr);
    }
    if let Some(sexpr) = compact_text_root_sexpr(value, source)? {
        return Ok(sexpr);
    }
    if let Some(indicator) = compact_indicator_surface(value, source)? {
        return Ok(sexpr::leaf(indicator));
    }
    if let Some(sexpr) = compact_connective_record_sexpr(value, source)? {
        return Ok(sexpr);
    }
    match value {
        Value::Array(items) if is_compact_with_free_modifiers_array(items) => items
            .iter()
            .map(|value| compact_value_sexpr(value, source))
            .collect::<Result<Vec<_>, _>>()
            .map(sexpr::splice),
        Value::Array(items) => compact_values_sexpr(items.iter(), source),
        Value::Object(object) => compact_values_sexpr(object.values(), source),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            Ok(sexpr::empty_node())
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn compact_connective_record_sexpr(
    value: &Value,
    source: &str,
) -> Result<Option<sexpr::SExpr>, OutputError> {
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    if !object.contains_key("cmavo") || !object.contains_key("kind") {
        return Ok(None);
    }
    compact_named_fields_sexpr(value, source, &["se", "nahe", "na", "cmavo", "nai"])
}

#[requires(true)]
#[ensures(true)]
fn compact_indicator_surface(value: &Value, source: &str) -> Result<Option<String>, OutputError> {
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    if !object.contains_key("indicator") {
        return Ok(None);
    }
    let Some(mut rendered) = object
        .get("indicator")
        .map(|value| compact_with_indicators_surface(value, source))
        .transpose()?
        .flatten()
    else {
        return Ok(None);
    };
    if let Some(nai) = object.get("nai").and_then(compact_word) {
        let nai = WithIndicators::bare(WordLike::bare(nai));
        rendered.push('-');
        rendered.push_str(&surface::format_with_indicators(&nai, source));
    }
    Ok(Some(rendered))
}

#[requires(true)]
#[ensures(true)]
fn compact_text_root_sexpr(
    value: &Value,
    source: &str,
) -> Result<Option<sexpr::SExpr>, OutputError> {
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    if !object.contains_key("paragraphs")
        && !object.contains_key("leading_indicators")
        && !object.contains_key("leading_free_modifiers")
        && !object.contains_key("leading_connective")
        && !object.contains_key("leading_nai")
        && !object.contains_key("leading_cmevla")
    {
        return Ok(None);
    }
    let mut children = Vec::new();
    for field in ["leading_nai", "leading_cmevla"] {
        if let Some(value) = object.get(field) {
            children.push(compact_value_sexpr(value, source)?);
        }
    }
    if let Some(value) = object.get("leading_indicators") {
        children.push(compact_leading_indicators_sexpr(value, source)?);
    }
    for field in ["leading_free_modifiers", "leading_connective", "paragraphs"] {
        if let Some(value) = object.get(field) {
            children.push(compact_value_sexpr(value, source)?);
        }
    }
    Ok(Some(sexpr::node(children)))
}

#[requires(true)]
#[ensures(true)]
fn compact_leading_indicators_sexpr(
    value: &Value,
    source: &str,
) -> Result<sexpr::SExpr, OutputError> {
    let Some(items) = value.as_array() else {
        return compact_value_sexpr(value, source);
    };
    let indicators = items
        .iter()
        .map(|item| compact_indicator_surface(item, source))
        .collect::<Result<Option<Vec<_>>, _>>()?;
    let Some(indicators) = indicators else {
        return compact_values_sexpr(items.iter(), source);
    };
    Ok(sexpr::leaf(indicators.join("-")))
}

#[requires(true)]
#[ensures(true)]
fn is_compact_with_free_modifiers_array(items: &[Value]) -> bool {
    items.len() > 1 && items[1..].iter().all(is_compact_free_modifier)
}

#[requires(true)]
#[ensures(true)]
fn is_compact_free_modifier(value: &Value) -> bool {
    let Some((constructor, _)) = compact_constructor(value) else {
        return false;
    };
    matches!(
        constructor,
        "Sei" | "To" | "Xi" | "Mai" | "Soi" | "Vocative" | "Replacement"
    )
}

#[requires(true)]
#[ensures(true)]
fn compact_special_constructor_sexpr(
    value: &Value,
    source: &str,
) -> Result<Option<sexpr::SExpr>, OutputError> {
    let Some((constructor, payload)) = compact_constructor(value) else {
        return Ok(None);
    };
    Ok(match constructor {
        "Composite" => payload
            .as_object()
            .and_then(|fields| fields.get("leaves"))
            .map(|leaves| compact_value_sexpr(leaves, source))
            .transpose()?,
        "Binary" => compact_named_fields_sexpr(
            payload,
            source,
            &["left_expression", "operator", "right_expression"],
        )?,
        "Descriptor" => compact_named_fields_sexpr(
            payload,
            source,
            &[
                "outer_quantifier",
                "descriptor",
                "tail_elements",
                "relation",
                "relative_clauses",
                "ku",
            ],
        )?,
        "Connective" => {
            compact_named_fields_sexpr(payload, source, &["se", "nahe", "na", "cmavo", "nai"])?
        }
        "Connected" if compact_has_field(payload, "leading_statement") => {
            compact_named_fields_sexpr(
                payload,
                source,
                &["leading_statement", "i", "connective", "trailing_statement"],
            )?
        }
        "Connected" if compact_has_field(payload, "leading_argument") => {
            compact_named_fields_sexpr(
                payload,
                source,
                &["leading_argument", "connective", "trailing_argument"],
            )?
        }
        "Connected" if compact_has_field(payload, "leading_relation") => {
            compact_named_fields_sexpr(
                payload,
                source,
                &["leading_relation", "connective", "trailing_relation"],
            )?
        }
        "PreIConnected" if compact_has_field(payload, "leading_statement") => {
            compact_named_fields_sexpr(
                payload,
                source,
                &["leading_statement", "connective", "i", "trailing_statement"],
            )?
        }
        "Lohu" if compact_field_contains_constructor(payload, "lohu", "LohuQuote") => payload
            .as_object()
            .and_then(|fields| fields.get("lohu"))
            .map(|lohu| compact_value_sexpr(lohu, source))
            .transpose()?,
        "Zoi" if compact_field_contains_constructor(payload, "zoi", "ZoiQuote") => payload
            .as_object()
            .and_then(|fields| fields.get("zoi"))
            .map(|zoi| compact_value_sexpr(zoi, source))
            .transpose()?,
        "Zo" if compact_field_contains_constructor(payload, "zo", "ZoQuote") => payload
            .as_object()
            .and_then(|fields| fields.get("zo"))
            .map(|zo| compact_value_sexpr(zo, source))
            .transpose()?,
        "Laho" if compact_field_contains_constructor(payload, "laho", "ZoiQuote") => payload
            .as_object()
            .and_then(|fields| fields.get("laho"))
            .map(|laho| compact_value_sexpr(laho, source))
            .transpose()?,
        _ => None,
    })
}

#[requires(!field.is_empty())]
#[ensures(true)]
fn compact_has_field(value: &Value, field: &str) -> bool {
    value
        .as_object()
        .is_some_and(|object| object.contains_key(field))
}

#[requires(!fields.is_empty())]
#[ensures(true)]
fn compact_named_fields_sexpr(
    value: &Value,
    source: &str,
    fields: &[&str],
) -> Result<Option<sexpr::SExpr>, OutputError> {
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    fields
        .iter()
        .filter_map(|field| object.get(*field))
        .map(|value| compact_value_sexpr(value, source))
        .collect::<Result<Vec<_>, _>>()
        .map(sexpr::node)
        .map(Some)
}

#[requires(!field.is_empty())]
#[requires(!constructor.is_empty())]
#[ensures(true)]
fn compact_field_contains_constructor(value: &Value, field: &str, constructor: &str) -> bool {
    value
        .as_object()
        .and_then(|fields| fields.get(field))
        .is_some_and(|value| compact_contains_constructor(value, constructor))
}

#[requires(!constructor.is_empty())]
#[ensures(true)]
fn compact_contains_constructor(value: &Value, constructor: &str) -> bool {
    if compact_constructor(value).is_some_and(|(current, _)| current == constructor) {
        return true;
    }
    match value {
        Value::Array(items) => items
            .iter()
            .any(|item| compact_contains_constructor(item, constructor)),
        Value::Object(object) => object
            .values()
            .any(|item| compact_contains_constructor(item, constructor)),
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn compact_values_sexpr<'value>(
    values: impl Iterator<Item = &'value Value>,
    source: &str,
) -> Result<sexpr::SExpr, OutputError> {
    values
        .map(|value| compact_value_sexpr(value, source))
        .collect::<Result<Vec<_>, _>>()
        .map(sexpr::node)
}

#[requires(true)]
#[ensures(true)]
fn compact_with_indicators_surface(
    value: &Value,
    source: &str,
) -> Result<Option<String>, OutputError> {
    Ok(compact_with_indicators(value, source)?
        .map(|word| surface::format_with_indicators(&word, source)))
}

#[requires(true)]
#[ensures(true)]
fn compact_with_indicators(
    value: &Value,
    source: &str,
) -> Result<Option<WithIndicators<WordLike>>, OutputError> {
    let Some((constructor, payload)) = compact_constructor(value) else {
        return Ok(None);
    };
    Ok(match constructor {
        "Bare" => compact_word_like(payload, source)?.map(WithIndicators::bare),
        "Emphasized" => {
            let Some(payload) = payload.as_object() else {
                return Ok(None);
            };
            let Some(bahe) = payload.get("bahe").and_then(compact_word) else {
                return Ok(None);
            };
            let Some(word_like) = payload
                .get("word_like")
                .map(|value| compact_word_like(value, source))
                .transpose()?
                .flatten()
            else {
                return Ok(None);
            };
            Some(WithIndicators::emphasized(bahe, word_like))
        }
        "WithIndicator" => {
            let Some(payload) = payload.as_object() else {
                return Ok(None);
            };
            let Some(base) = payload
                .get("base")
                .map(|value| compact_with_indicators(value, source))
                .transpose()?
                .flatten()
            else {
                return Ok(None);
            };
            let Some(indicator) = payload.get("indicator").and_then(compact_word) else {
                return Ok(None);
            };
            let nai = payload.get("nai").and_then(compact_word);
            Some(WithIndicators::with_indicator(base, indicator, nai))
        }
        _ => None,
    })
}

#[requires(true)]
#[ensures(true)]
fn compact_word_like(value: &Value, source: &str) -> Result<Option<WordLike>, OutputError> {
    if let Some(word) = compact_word(value) {
        return Ok(Some(WordLike::bare(word)));
    }
    let Some((constructor, payload)) = compact_constructor(value) else {
        return Ok(None);
    };
    Ok(match constructor {
        "Bare" => compact_word_like(payload, source)?,
        "ZoQuote" => {
            let Some(payload) = payload.as_object() else {
                return Ok(None);
            };
            let Some(zo) = payload.get("zo").and_then(compact_word) else {
                return Ok(None);
            };
            let Some(word) = payload.get("word").and_then(compact_word) else {
                return Ok(None);
            };
            Some(WordLike::zo_quote(zo, word))
        }
        "ZoiQuote" => {
            let Some(payload) = payload.as_object() else {
                return Ok(None);
            };
            let Some(zoi) = payload.get("zoi").and_then(compact_word) else {
                return Ok(None);
            };
            let Some(opening_delimiter) = payload.get("opening_delimiter").and_then(compact_word)
            else {
                return Ok(None);
            };
            let Some(quoted_text) = payload
                .get("quoted_text")
                .and_then(|value| compact_span(value, source))
            else {
                return Ok(None);
            };
            let Some(closing_delimiter) = payload.get("closing_delimiter").and_then(compact_word)
            else {
                return Ok(None);
            };
            Some(WordLike::zoi_quote(
                zoi,
                opening_delimiter,
                quoted_text,
                closing_delimiter,
            ))
        }
        "LohuQuote" => {
            let Some(payload) = payload.as_object() else {
                return Ok(None);
            };
            let Some(lohu) = payload.get("lohu").and_then(compact_word) else {
                return Ok(None);
            };
            let quoted_words = payload
                .get("quoted_words")
                .and_then(Value::as_array)
                .map(|words| words.iter().filter_map(compact_word).collect::<Vec<_>>())
                .unwrap_or_default();
            let Some(lehu) = payload.get("lehu").and_then(compact_word) else {
                return Ok(None);
            };
            Some(WordLike::lohu_quote(lohu, quoted_words, lehu))
        }
        "SingleWordQuote" => {
            let Some(payload) = payload.as_object() else {
                return Ok(None);
            };
            let Some(marker) = payload.get("marker").and_then(compact_word) else {
                return Ok(None);
            };
            let Some(quoted_text) = payload
                .get("quoted_text")
                .and_then(|value| compact_span(value, source))
            else {
                return Ok(None);
            };
            Some(WordLike::single_word_quote(marker, quoted_text))
        }
        "Letter" => {
            let Some(payload) = payload.as_object() else {
                return Ok(None);
            };
            let Some(base) = payload
                .get("base")
                .map(|value| compact_word_like(value, source))
                .transpose()?
                .flatten()
            else {
                return Ok(None);
            };
            let Some(bu) = payload.get("bu").and_then(compact_word) else {
                return Ok(None);
            };
            Some(WordLike::letter(base, bu))
        }
        "ZeiLujvo" => {
            let Some(payload) = payload.as_object() else {
                return Ok(None);
            };
            let Some(left) = payload
                .get("left")
                .map(|value| compact_word_like(value, source))
                .transpose()?
                .flatten()
            else {
                return Ok(None);
            };
            let Some(zei) = payload.get("zei").and_then(compact_word) else {
                return Ok(None);
            };
            let Some(right) = payload.get("right").and_then(compact_word) else {
                return Ok(None);
            };
            Some(WordLike::zei_lujvo(left, zei, right))
        }
        _ => None,
    })
}

#[requires(true)]
#[ensures(true)]
fn compact_constructor(value: &Value) -> Option<(&str, &Value)> {
    let object = value.as_object()?;
    if object.len() != 1 {
        return None;
    }
    object
        .iter()
        .next()
        .map(|(key, value)| (key.as_str(), value))
}

#[requires(true)]
#[ensures(true)]
fn compact_word(value: &Value) -> Option<Word> {
    let object = value.as_object()?;
    if !object.contains_key("kind") || !object.contains_key("phonemes") {
        return None;
    }
    serde_json::from_value(value.clone()).ok()
}

#[requires(true)]
#[ensures(true)]
fn compact_span(value: &Value, source: &str) -> Option<SourceSpan> {
    let chars = value.as_array()?;
    if chars.len() != 2 {
        return None;
    }
    let char_start = usize::try_from(chars.first()?.as_u64()?).ok()?;
    let char_end = usize::try_from(chars.get(1)?.as_u64()?).ok()?;
    let byte_start = byte_offset_for_char(source, char_start)?;
    let byte_end = byte_offset_for_char(source, char_end)?;
    SourceSpan::new(None, byte_start, byte_end, char_start, char_end).ok()
}

#[requires(true)]
#[ensures(true)]
fn byte_offset_for_char(source: &str, char_offset: usize) -> Option<usize> {
    if char_offset == source.chars().count() {
        return Some(source.len());
    }
    source.char_indices().nth(char_offset).map(|(byte, _)| byte)
}
