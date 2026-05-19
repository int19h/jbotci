//! Output format selection and render facade.

mod sexpr;
mod surface;

use bityzba::{invariant, requires};
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

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| !matches!(value, Value::Null)) || ret.is_err())]
pub fn compact_json_value<T: Serialize>(value: &T) -> Result<Value, OutputError> {
    serde_json::to_value(value)
        .map(compact_json_shape)
        .map_err(|source| OutputError::Json(source.to_string()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn compact_json_string<T: Serialize>(value: &T) -> Result<String, OutputError> {
    Ok(format_compact_json_value(&compact_json_value(value)?, 0))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub fn pretty_brackets(tree: &TextSyntax, source: &str) -> Result<String, OutputError> {
    pretty_brackets_with_options(tree, source, BracketRenderOptions::default())
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
                if let Some(value) = compact_legacy_syntax_value(&kind, &object) {
                    return value;
                }
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
            Value::Object(compacted)
        }
        other => other,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_compact_json_value(value: &Value, indent: usize) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            serde_json::to_string(value).expect("serializing JSON scalar cannot fail")
        }
        Value::Array(items) => format_compact_json_array(items, indent),
        Value::Object(object) if is_constructor_object(object) => {
            format_compact_json_constructor(object, indent)
        }
        Value::Object(object) => format_compact_json_object(object, indent),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_compact_json_field_value(value: &Value, field_indent: usize) -> String {
    match value {
        Value::Object(object) if is_constructor_object(object) => {
            format_compact_json_constructor(object, field_indent)
        }
        _ => format_compact_json_value(value, field_indent),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_compact_json_array(items: &[Value], indent: usize) -> String {
    if items.is_empty() {
        return "[]".to_owned();
    }
    if items.iter().all(is_compact_json_scalar) {
        let items = items
            .iter()
            .map(|item| format_compact_json_value(item, indent))
            .collect::<Vec<_>>()
            .join(", ");
        return format!("[{items}]");
    }

    let item_indent = indent + 2;
    let pad = " ".repeat(item_indent);
    let end = " ".repeat(indent);
    let mut output = String::from("[\n");
    for (index, item) in items.iter().enumerate() {
        output.push_str(&pad);
        output.push_str(&format_compact_json_value(item, item_indent));
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
fn format_compact_json_object(object: &serde_json::Map<String, Value>, indent: usize) -> String {
    if object.is_empty() {
        return "{}".to_owned();
    }

    let field_indent = indent + 2;
    let pad = " ".repeat(field_indent);
    let end = " ".repeat(indent);
    let mut output = String::from("{\n");
    for (index, (key, value)) in object.iter().enumerate() {
        output.push_str(&pad);
        output.push_str(&json_string(key));
        output.push_str(": ");
        output.push_str(&format_compact_json_field_value(value, field_indent));
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
) -> String {
    let (constructor, payload) = object.iter().next().expect("constructor object has item");
    let constructor = json_string(constructor);
    match payload {
        Value::Object(fields) if fields.is_empty() => format!("{{{constructor}: {{}}}}"),
        Value::Object(fields) => {
            let field_indent = constructor_indent + 2;
            let pad = " ".repeat(field_indent);
            let end = " ".repeat(constructor_indent);
            let mut output = format!("{{{constructor}: {{\n");
            for (index, (key, value)) in fields.iter().enumerate() {
                output.push_str(&pad);
                output.push_str(&json_string(key));
                output.push_str(": ");
                output.push_str(&format_compact_json_field_value(value, field_indent));
                if index + 1 != fields.len() {
                    output.push(',');
                }
                output.push('\n');
            }
            output.push_str(&end);
            output.push_str("}}");
            output
        }
        other => format!(
            "{{{constructor}: {}}}",
            format_compact_json_value(other, constructor_indent + 2)
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
            if let Some(value) = payload.get("word").or_else(|| payload.get("word_like")) {
                return Some(compact_constructor_value(constructor, value.clone()));
            }
            None
        }
        "BaseWord" => {
            let Value::Object(payload) = payload else {
                return None;
            };
            payload.get("word_like").cloned().map(compact_json_shape)
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
        "StandaloneIndicator" => Some(compact_json_shape(payload.clone())),
        "LojbanText" => Some(compact_json_shape(payload.clone())),
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
fn compact_legacy_syntax_value(
    kind: &str,
    object: &serde_json::Map<String, Value>,
) -> Option<Value> {
    Some(match kind {
        "null" => Value::Null,
        "bool" | "integer" | "text" | "json" => compact_json_shape(object.get("value")?.clone()),
        "word" => compact_json_shape(object.get("word")?.clone()),
        "list" => compact_json_shape(object.get("items")?.clone()),
        "node" => compact_legacy_syntax_node(object.get("node")?.clone()),
        _ => return None,
    })
}

#[requires(true)]
#[ensures(true)]
fn compact_legacy_syntax_node(value: Value) -> Value {
    let Value::Object(mut node) = value else {
        return Value::Null;
    };
    let Some(Value::String(constructor)) = node.remove("constructor") else {
        return Value::Null;
    };
    let fields = node
        .remove("fields")
        .and_then(|fields| match fields {
            Value::Array(fields) => Some(fields),
            _ => None,
        })
        .unwrap_or_default();

    match constructor.as_str() {
        "[]" => Value::Array(Vec::new()),
        "(:)" => compact_legacy_cons(fields),
        "Nothing" => Value::Null,
        "Just" => fields
            .into_iter()
            .find_map(legacy_field_value)
            .map(compact_json_shape)
            .unwrap_or(Value::Null),
        _ => {
            let payload = fields
                .into_iter()
                .filter_map(legacy_named_field)
                .filter_map(|(name, value)| {
                    let value = compact_json_shape(value);
                    (!is_omitted_compact_value(&value)).then_some((name, value))
                })
                .collect::<serde_json::Map<_, _>>();
            Value::Object(
                [(constructor, Value::Object(payload))]
                    .into_iter()
                    .collect(),
            )
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn compact_legacy_cons(fields: Vec<Value>) -> Value {
    let mut values = fields.into_iter().filter_map(legacy_field_value);
    let Some(head) = values.next() else {
        return Value::Array(Vec::new());
    };
    let Some(tail) = values.next() else {
        return Value::Array(vec![compact_json_shape(head)]);
    };
    let mut items = vec![compact_json_shape(head)];
    match compact_json_shape(tail) {
        Value::Array(tail_items) => items.extend(tail_items),
        value if !value.is_null() => items.push(value),
        _ => {}
    }
    Value::Array(items)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(name, _)| !name.is_empty()))]
fn legacy_named_field(field: Value) -> Option<(String, Value)> {
    let Value::Object(mut field) = field else {
        return None;
    };
    let Some(Value::String(name)) = field.remove("name") else {
        return None;
    };
    let value = field.remove("value")?;
    Some((legacy_field_name(&name).to_owned(), value))
}

#[requires(true)]
#[ensures(true)]
fn legacy_field_value(field: Value) -> Option<Value> {
    let Value::Object(mut field) = field else {
        return None;
    };
    field.remove("value")
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn legacy_field_name(name: &str) -> String {
    let mut output = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if index > 0 {
                output.push('_');
            }
            output.extend(ch.to_lowercase());
        } else {
            output.push(ch);
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn compact_constructor_name(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "rafsi" => "Rafsi",
        "hyphen" => "Hyphen",
        "bare" => "Bare",
        "zo-quote" => "ZoQuote",
        "zoi-quote" => "ZoiQuote",
        "lohu-quote" => "LohuQuote",
        "single-word-quote" => "SingleWordQuote",
        "letter" => "Letter",
        "zei-lujvo" => "ZeiLujvo",
        "emphasized" => "Emphasized",
        "with-indicator" => "WithIndicator",
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
    let words = tree
        .clone()
        .words()
        .iter()
        .map(|word| sexpr::leaf(surface::format_word_with_modifiers(word, source)))
        .collect::<Vec<_>>();
    let sexpr = sexpr::node(words);
    Ok(sexpr::render_bracketed_with_options(
        &sexpr::flatten(sexpr),
        options,
    ))
}
