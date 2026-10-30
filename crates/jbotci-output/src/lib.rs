#![recursion_limit = "1024"]

//! Output format selection and render facade.

mod brackets;
pub use brackets::{
    pretty_generated_model_brackets_with_limit, pretty_morphology_brackets_with_limit,
};
pub mod budget;
mod diagnostics;
mod indentation;
mod json;
mod json_writer;
mod markdown;
mod places;
mod recovered;
mod references;
mod sexpr;
mod surface;
mod trace;
mod tree;
mod unicode_math;

#[allow(unused_imports)]
use bityzba::expensive_ensures;
use bityzba::{invariant, requires};
pub use diagnostics::{
    DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH, DiagnosticRenderOptions, render_diagnostics,
    render_diagnostics_with_limit,
};
pub use indentation::{Indentation, InvalidIndentation};
pub use jbotci_diagnostics::DiagnosticDetailMode;
use jbotci_morphology::WordLike;
pub use jbotci_morphology::{GlideMark, PhonemeRenderOptions, StressMark};
pub use jbotci_orthography::LojbanScript;
use jbotci_syntax::TextSyntax;
pub use markdown::{
    DEFAULT_MARKDOWN_LINK_BASE, render_diagnostic_text_segments_markdown,
    render_diagnostic_text_segments_markdown_with_link_base, render_vlacku_card_markdown,
    render_vlacku_card_markdown_with_link_base, render_vlacku_cards_markdown,
    render_vlacku_cards_markdown_with_link_base, render_vlacku_decomposition_markdown,
    render_vlacku_headword_markdown,
};
pub use places::{
    DefinitionPlaceMap, IndexedPlaceSpan, format_definition_line_with_indexed_places,
    format_definition_line_with_indexed_places_in_scope, format_notes_line_with_indexed_places,
    format_notes_line_with_indexed_places_in_scope, indexed_place_spans_for_definition_line,
    indexed_place_spans_for_notes_line,
};
pub use recovered::{
    compact_recovered_morphology_json_string_with_limit,
    compact_recovered_morphology_json_string_with_limits,
    compact_recovered_morphology_json_string_with_options,
    compact_recovered_syntax_json_string_with_limits,
    compact_recovered_syntax_json_string_with_options,
    pretty_recovered_morphology_brackets_with_limit,
    pretty_recovered_morphology_brackets_with_options, pretty_recovered_morphology_raw,
    pretty_recovered_morphology_raw_with_limit, pretty_recovered_morphology_tree_with_limit,
    pretty_recovered_morphology_tree_with_options,
    pretty_recovered_syntax_bracket_source_fragments_with_options,
    pretty_recovered_syntax_brackets_with_limit, pretty_recovered_syntax_brackets_with_options,
    pretty_recovered_syntax_error_region_brackets_with_options, pretty_recovered_syntax_raw,
    pretty_recovered_syntax_raw_with_limit, pretty_recovered_syntax_tree_with_limit,
    pretty_recovered_syntax_tree_with_options, raw_with_limit,
};
pub use references::{
    ReferenceAnnotationSource, ReferenceAnnotationSourceData, ReferenceAnnotations,
    ReferenceDisplayModel, ReferenceName, ReferenceSlotName, RichReferenceAnnotation,
    RichReferenceAnnotations, generated_reference_slot_name_for_place_slot,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use surface::{
    LujvoFragmentKind, LujvoFragmentRenderError, phoneme_render_options_for_script,
    render_lojban_text_for_script, render_lojban_text_for_script_with_options,
    render_lujvo_fragment_for_script,
};
use thiserror::Error;
pub use trace::{TraceRenderOptions, render_trace_report};
pub use tree::{GeneratedReferenceDisplay, generated_reference_display};
pub use tree::{
    pretty_generated_model_tree_with_limit, pretty_generated_model_tree_with_reference_display,
    pretty_morphology_tree_with_limit,
};

#[invariant(byte_start <= byte_end, "bracket source byte range must be ordered")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BracketSourceRange {
    pub byte_start: usize,
    pub byte_end: usize,
}

#[invariant(true)]
#[invariant(::Text { .. } => true)]
#[invariant(::Span { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum BracketSourceFragment {
    Text {
        text: String,
        range: Option<BracketSourceRange>,
        role: BracketSourceFragmentRole,
    },
    Span {
        range: Option<BracketSourceRange>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        constructs: Vec<BracketSourceConstruct>,
        children: Vec<BracketSourceFragment>,
    },
}

/// Grammar constructs whose boundaries survive bracket-tree flattening.
///
/// This metadata is deliberately independent of any decoration glyphs. Editor
/// integrations can filter the tree-anchored fragment hierarchy and let a
/// decoration profile decide what, if anything, to draw at each boundary.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BracketSourceConstruct {
    Sumti,
    BridiTail,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BracketSourceFragmentRole {
    #[default]
    Normal,
    Elided,
    Error,
}

impl BracketSourceFragmentRole {
    #[requires(true)]
    #[ensures(ret == matches!(self, Self::Normal))]
    pub fn is_normal(self) -> bool {
        matches!(self, Self::Normal)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Diagnostic(..) => true)]
#[invariant(::Json(..) => true)]
#[invariant(::Ipa(..) => true)]
#[invariant(::References(..) => true)]
#[invariant(::Recovery(..) => true)]
#[invariant(::Render(..) => true)]
#[invariant(::Limit { .. } => true)]
pub enum OutputError {
    #[error("The generated output exceeds the {}-byte limit.", limit.get())]
    Limit { limit: budget::ByteLimit },
    #[error("failed to render output: {0}")]
    Render(String),
    #[error("failed to render diagnostic: {0}")]
    Diagnostic(String),
    #[error("failed to encode compact JSON: {0}")]
    Json(String),
    #[error("failed to render IPA: {0}")]
    Ipa(String),
    #[error("failed to analyze semantic references: {0}")]
    References(#[from] jbotci_semantics::references::ReferenceAnalysisError),
    #[error("failed to render recovered parse: {0}")]
    Recovery(String),
}

impl OutputError {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_render_io(error: std::io::Error, limit: budget::ByteLimit) -> Self {
        if error.kind() == std::io::ErrorKind::FileTooLarge {
            Self::Limit { limit }
        } else {
            Self::Render(error.to_string())
        }
    }
    #[requires(true)]
    #[ensures(true)]
    fn from_json(error: serde_json::Error, limit: budget::ByteLimit) -> Self {
        if error.io_error_kind() == Some(std::io::ErrorKind::FileTooLarge) {
            Self::Limit { limit }
        } else {
            Self::Json(error.to_string())
        }
    }
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
    pub script: LojbanScript,
    pub glyphs: GlyphStyle,
    pub decompose_lujvo: bool,
    pub insert_hair_space: bool,
    pub show_elided: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct JsonRenderOptions {
    pub indent: Indentation,
    pub phonemes: PhonemeRenderOptions,
    pub show_elided: bool,
    pub color: bool,
}

impl Default for JsonRenderOptions {
    #[requires(true)]
    #[ensures(ret.indent == Indentation::default())]
    #[ensures(ret.phonemes == PhonemeRenderOptions::default())]
    #[ensures(!ret.show_elided)]
    #[ensures(!ret.color)]
    fn default() -> Self {
        Self {
            indent: crate::Indentation::default(),
            phonemes: PhonemeRenderOptions::default(),
            show_elided: false,
            color: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct TreeRenderOptions {
    pub color: bool,
    pub indent: Indentation,
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
    #[ensures(ret.indent == Indentation::default())]
    #[ensures(ret.phonemes == PhonemeRenderOptions::default())]
    #[ensures(ret.glyphs == GlyphStyle::default())]
    #[ensures(!ret.show_spans)]
    #[ensures(!ret.show_refs)]
    #[ensures(!ret.decompose_lujvo)]
    #[ensures(!ret.show_elided)]
    fn default() -> Self {
        Self {
            color: false,
            indent: crate::Indentation::default(),
            phonemes: PhonemeRenderOptions::default(),
            glyphs: GlyphStyle::default(),
            show_spans: false,
            show_refs: false,
            decompose_lujvo: false,
            show_elided: false,
        }
    }
}

/// Serialize a projection before its JSON buffer can exceed the limit.
#[requires(true)]
#[ensures(true)]
pub fn compact_json_value_with_limit<T: Serialize>(
    value: &T,
    limit: budget::ByteLimit,
) -> Result<Value, OutputError> {
    let bytes =
        budget::bounded_json(value, limit).map_err(|error| OutputError::from_json(error, limit))?;
    let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
    deserializer.disable_recursion_limit();
    Value::deserialize(serde_stacker::Deserializer::new(&mut deserializer))
        .map(compact_json_shape)
        .map_err(|error| OutputError::Json(error.to_string()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn render_json_value_with_limit(
    value: &Value,
    options: JsonRenderOptions,
    limit: budget::ByteLimit,
) -> Result<String, OutputError> {
    json_writer::render(value, 0, options, false, limit)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn render_compact_json_value_with_limit(
    value: &Value,
    options: JsonRenderOptions,
    limit: budget::ByteLimit,
) -> Result<String, OutputError> {
    json_writer::render(value, 0, options, true, limit)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= text_limit.get()) || ret.is_err())]
pub fn compact_json_string_with_limits<T: Serialize>(
    value: &T,
    options: JsonRenderOptions,
    text_limit: budget::ByteLimit,
    projection_limit: budget::ByteLimit,
) -> Result<String, OutputError> {
    render_compact_json_value_with_limit(
        &compact_json_value_with_limit(value, projection_limit)?,
        options,
        text_limit,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= text_limit.get()) || ret.is_err())]
pub fn compact_generated_model_json_string_with_limits(
    tree: &jbotci_syntax::generated_model::TextSyntax,
    options: JsonRenderOptions,
    text_limit: budget::ByteLimit,
    projection_limit: budget::ByteLimit,
) -> Result<String, OutputError> {
    let mut value = compact_json_value_with_limit(tree, projection_limit)?;
    if options.show_elided {
        json::insert_generated_model_elided_terminators(&mut value, options);
    }
    json::render_phoneme_fields_in_json_value(&mut value, options.phonemes);
    render_compact_json_value_with_limit(&value, options, text_limit)
}

#[requires(true)]
#[ensures(true)]
pub fn compact_json_value<T: Serialize>(value: &T) -> Result<Value, OutputError> {
    let mut bytes = Vec::new();
    let mut serializer = serde_json::Serializer::new(&mut bytes);
    let serializer = serde_stacker::Serializer::new(&mut serializer);
    value
        .serialize(serializer)
        .map_err(|source| OutputError::Json(source.to_string()))?;
    let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
    deserializer.disable_recursion_limit();
    let deserializer = serde_stacker::Deserializer::new(&mut deserializer);
    Value::deserialize(deserializer)
        .map(compact_json_shape)
        .map_err(|source| OutputError::Json(source.to_string()))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_json_value_with_options(value: &Value, options: JsonRenderOptions) -> String {
    format_standard_json_value(value, 0, options)
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
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn compact_morphology_json_string_with_limit(
    words: &[WordLike],
    options: JsonRenderOptions,
    limit: budget::ByteLimit,
) -> Result<String, OutputError> {
    compact_morphology_json_string_with_limits(words, options, limit, limit)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= text_limit.get()) || ret.is_err())]
pub fn compact_morphology_json_string_with_limits(
    words: &[WordLike],
    options: JsonRenderOptions,
    text_limit: budget::ByteLimit,
    projection_limit: budget::ByteLimit,
) -> Result<String, OutputError> {
    let mut value = json::morphology_projection_with_limit(words, projection_limit)?;
    json::render_phoneme_fields_in_json_value(&mut value, options.phonemes);
    render_compact_json_value_with_limit(&value, options, text_limit)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn ipa_morphology_text(words: &[WordLike], source: &str) -> Result<String, OutputError> {
    jbotci_phonetic::ipa_morphology_text(words, source)
        .map_err(|error| OutputError::Ipa(error.to_string()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| !matches!(value, Value::Null)) || ret.is_err())]
pub fn compact_generated_model_json_value(
    tree: &jbotci_syntax::generated_model::TextSyntax,
) -> Result<Value, OutputError> {
    compact_json_value(tree)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn compact_generated_model_json_string_with_options(
    tree: &jbotci_syntax::generated_model::TextSyntax,
    options: JsonRenderOptions,
) -> Result<String, OutputError> {
    let mut value = compact_json_value(tree)?;
    if options.show_elided {
        json::insert_generated_model_elided_terminators(&mut value, options);
    }
    json::render_phoneme_fields_in_json_value(&mut value, options.phonemes);
    Ok(format_compact_json_value(&value, 0, options))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn pretty_morphology_tree_with_options(
    words: &[WordLike],
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    tree::pretty_morphology_tree_with_options(words, source, options)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn pretty_tree_with_options(
    tree: &TextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    tree::pretty_generated_model_tree_with_options(tree, source, options)
}

#[doc(hidden)]
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn pretty_generated_model_tree_with_options(
    tree: &jbotci_syntax::generated_model::TextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    tree::pretty_generated_model_tree_with_options(tree, source, options)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn pretty_generated_model_raw_tree_with_options(
    tree: &jbotci_syntax::generated_model::TextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    tree::pretty_generated_model_raw_tree_with_options(tree, source, options)
}

#[doc(hidden)]
#[requires(true)]
#[ensures(ret.is_ok())]
#[expensive_ensures(ret.as_ref().is_ok_and(|text| text.is_empty() == brackets::generated_text_is_empty(tree)) || ret.is_err())]
pub fn pretty_generated_model_brackets_with_options(
    tree: &jbotci_syntax::generated_model::TextSyntax,
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    brackets::pretty_generated_model_brackets_with_options(tree, source, options)
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
    json_writer::render(
        value,
        indent,
        options,
        true,
        budget::ByteLimit::new(usize::MAX).expect("positive limit"),
    )
    .expect("unrestricted JSON rendering")
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_standard_json_value(value: &Value, indent: usize, options: JsonRenderOptions) -> String {
    json_writer::render(
        value,
        indent,
        options,
        false,
        budget::ByteLimit::new(usize::MAX).expect("positive limit"),
    )
    .expect("unrestricted JSON rendering")
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
    let field = json::constructor_single_payload_field(constructor)?;
    single_payload_field(constructor, payload, field)
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
#[ensures(ret.is_ok())]
#[expensive_ensures(ret.as_ref().is_ok_and(|text| text.is_empty() == brackets::generated_text_is_empty(tree)) || ret.is_err())]
pub fn pretty_brackets_with_options(
    tree: &TextSyntax,
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    brackets::pretty_generated_model_brackets_with_options(tree, source, options)
}

#[requires(true)]
#[ensures(ret.is_ok())]
#[expensive_ensures(ret.as_ref().is_ok_and(|fragments| fragments.is_empty() == brackets::generated_text_is_empty(tree)) || ret.is_err())]
pub fn pretty_bracket_source_fragments_with_options(
    tree: &TextSyntax,
    source: &str,
    options: BracketRenderOptions,
) -> Result<Vec<BracketSourceFragment>, OutputError> {
    brackets::pretty_generated_model_bracket_source_fragments_with_options(tree, source, options)
}

#[requires(true)]
#[ensures(words.is_empty() || ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn pretty_morphology_brackets_with_options(
    words: &[WordLike],
    source: &str,
    options: BracketRenderOptions,
) -> Result<String, OutputError> {
    brackets::pretty_morphology_brackets_with_options(words, source, options)
}

#[cfg(test)]
mod tests {
    use bityzba::{data, requires};
    use jbotci_dictionary::WordType;
    use jbotci_morphology::{
        Word, WordKind, WordLike, WordLikeData, pronunciation_syllables,
        segment_words_with_modifiers,
    };
    use jbotci_source::SourceSpan;
    use jbotci_syntax::{
        ParseOptions, parse_syntax_tree, parse_syntax_tree_generated_model_with_source_and_options,
    };

    use super::*;

    #[requires(true)]
    #[bityzba::ensures(ret.as_ref().is_none_or(|phonemes| !phonemes.as_str().is_empty()))]
    fn cmavo_phonemes(text: &str) -> Option<jbotci_morphology::Phonemes> {
        let normalized = jbotci_morphology::normalize_cmavo_form(text)?;
        jbotci_morphology::Cmavo::from_text(&normalized)?;
        jbotci_morphology::Phonemes::from_canonical(normalized).ok()
    }

    const DICTIONARY_PARSE_XFAILS: &str = include_str!("../tests/dictionary_parse_xfails.tsv");
    const DICTIONARY_CATEGORY_EXCEPTIONS: &str =
        include_str!("../tests/dictionary_category_exceptions.tsv");
    const DICTIONARY_UNSYLLABIFIABLE_CMEVLA: &str =
        include_str!("../tests/dictionary_unsyllabifiable_cmevla.tsv");
    const DICTIONARY_IPA_XFAILS: &str = include_str!("../tests/dictionary_ipa_xfails.tsv");

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
    fn compact_json_value_allows_root_null() {
        let value = compact_json_value(&Option::<usize>::None).expect("serialize null");

        assert_eq!(value, Value::Null);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_v0_ipa_examples() {
        let cases = [
            ("klama", "ˈkla.ma"),
            ("tavla", "ˈta.vla"),
            ("coi", "ʃoj"),
            ("i", "i"),
            ("oi", "oj"),
            ("ui", "wi"),
            ("ie", "je"),
            ("ba'e", "ˈba.he"),
            ("e'u bridi", "ˈe.hu ˈbri.di"),
            ("la alis", "la ˈʔa.lis"),
            (".alis.", "ˈa.lis"),
            ("i la diskord", "i la ʔdiskord"),
            (".armstrong.", "armstrong"),
            ("bastn.", "bas.tn"),
            (".finyks.", "finəks"),
            ("i la diskord jdice", "i la ʔdiskord ˈʔʒdi.ʃe"),
            ("diskord i", "diskord ʔi"),
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
            ("bolxada", "bol.ˈxa.da"),
            ("dikyjvo", "ˈdi.kə.ʒvo"),
            ("bisydja", "ˈbi.sə.dʒa"),
            ("cidjrspageti", "ʃid.ʒr.spa.ˈge.ti"),
            ("jetcybolxada", "ʒe.tʃə.bol.ˈxa.da"),
            ("cipnrstrigi", "ʃip.nr.ˈstri.gi"),
            ("cabrspreso", "ʃa.br.ˈspre.so"),
            ("bolstropfedo", "bol.strop.ˈfe.do"),
            ("ciskrpeuedji", "ʃis.kr.pe.ˈwe.dʒi"),
            ("bangrsfe'enska", "ban.gr.sfe.ˈhen.ska"),
        ];

        for (source, expected) in cases {
            assert_eq!(render_ipa(source), expected, "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_cmevla_ipa_boundaries_conservatively() {
        let cases = [
            ("t", "t"),
            (".t", "t"),
            ("t.", "t"),
            (".t.", "t"),
            ("la t.", "la ʔt"),
            (".alis.", "ˈa.lis"),
            ("la .alis.", "la ˈʔa.lis"),
            ("deLEZ.", "delˈez"),
            ("nu,IORK.", "nu.ˈjork"),
            ("diskord i", "diskord ʔi"),
            ("i la diskord", "i la ʔdiskord"),
        ];

        for (source, expected) in cases {
            let rendered = render_ipa(source);
            assert_eq!(rendered, expected, "{source}");
            assert!(!rendered.contains("ʔʔ"), "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn y_initial_by_words_render_written_leading_pauses() {
        let source = "y ybu y'y y'y bu";
        assert_eq!(
            render_morphology_brackets_with_script(source, LojbanScript::Latin),
            "([.y bu] .y'y [.y'y bu])"
        );
        assert_eq!(render_ipa("mi ybu y'y"), "mi ʔə bu ʔə.hə");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_entries_parse_match_categories_and_render_ipa() {
        let mut failures = Vec::new();
        for entry in jbotci_dictionary_data::english().entries() {
            let parsed = match direct_dictionary_entry_words(entry.word, entry.word_type) {
                Some(words) => words,
                None => match segment_words_with_modifiers(entry.word) {
                    Ok(words) => words,
                    Err(error) => {
                        if dictionary_parse_xfail_reason(entry.word, entry.word_type).is_none() {
                            failures.push(format!(
                                "{} [{}] failed morphology without xfail: {error:?}",
                                entry.word,
                                entry.word_type.as_str()
                            ));
                        }
                        continue;
                    }
                },
            };

            if let Some(reason) = dictionary_parse_xfail_reason(entry.word, entry.word_type) {
                failures.push(format!(
                    "{} [{}] parsed despite parse xfail: {reason}",
                    entry.word,
                    entry.word_type.as_str()
                ));
            }

            match (
                dictionary_category_exception_reason(entry.word, entry.word_type),
                dictionary_category_matches(entry.word_type, &parsed),
            ) {
                (Some(reason), true) => failures.push(format!(
                    "{} [{}] still has stale category exception: {reason}",
                    entry.word,
                    entry.word_type.as_str()
                )),
                (None, false) => failures.push(format!(
                    "{} [{}] parsed with unexpected morphology structure",
                    entry.word,
                    entry.word_type.as_str()
                )),
                _ => {}
            }

            let ipa = ipa_morphology_text(&parsed, entry.word);
            let ipa_rendered = ipa.as_ref().is_ok_and(|ipa| !ipa.is_empty());
            match ipa {
                Ok(ipa)
                    if !ipa.is_empty()
                        && dictionary_ipa_xfail_reason(entry.word, entry.word_type).is_none() => {}
                Ok(ipa) if !ipa.is_empty() => failures.push(format!(
                    "{} [{}] rendered IPA despite IPA xfail: {}",
                    entry.word,
                    entry.word_type.as_str(),
                    dictionary_ipa_xfail_reason(entry.word, entry.word_type)
                        .expect("IPA xfail exists")
                )),
                Ok(_) => failures.push(format!(
                    "{} [{}] rendered empty IPA",
                    entry.word,
                    entry.word_type.as_str()
                )),
                Err(error) => {
                    if dictionary_ipa_xfail_reason(entry.word, entry.word_type).is_none() {
                        failures.push(format!(
                            "{} [{}] failed IPA rendering: {error}",
                            entry.word,
                            entry.word_type.as_str()
                        ));
                    }
                }
            }

            assert_dictionary_words_syllabify(
                entry.word,
                entry.word_type,
                &parsed,
                ipa_rendered,
                &mut failures,
            );
        }

        assert!(
            failures.is_empty(),
            "{} dictionary failures:\n{}",
            failures.len(),
            failures.iter().cloned().collect::<Vec<_>>().join("\n")
        );
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|words| !words.is_empty()))]
    fn direct_dictionary_entry_words(word: &str, word_type: WordType) -> Option<Vec<WordLike>> {
        if !matches!(
            word_type,
            WordType::Cmavo | WordType::ExperimentalCmavo | WordType::ObsoleteCmavo
        ) || word.chars().any(char::is_whitespace)
        {
            return None;
        }
        let phonemes = cmavo_phonemes(word)?;
        let span = SourceSpan::new(None, 0, word.len(), 0, word.chars().count())
            .expect("dictionary word span is valid");
        Some(vec![WordLike::bare(Word::from_kind(
            WordKind::Cmavo,
            phonemes,
            span,
        ))])
    }

    /// `ipa_rendered` reports whether the entry produced IPA at all.
    ///
    /// `dictionary_ipa_xfails.tsv` serves both consumers of a failed
    /// pronunciation: an entry that renders no IPA, and an entry whose
    /// non-cmevla words do not syllabify. The staleness check therefore has to
    /// see both, or an entry listed for the first reason reads as a stale
    /// exception for the second.
    #[requires(true)]
    #[ensures(true)]
    fn assert_dictionary_words_syllabify(
        entry_word: &str,
        word_type: WordType,
        parsed: &[WordLike],
        ipa_rendered: bool,
        failures: &mut Vec<String>,
    ) {
        let mut unsyllabifiable_cmevla_found = false;
        let mut unsyllabifiable_non_cmevla_found = false;
        for word in plain_words(parsed) {
            match word.kind() {
                WordKind::Cmevla => {
                    if pronunciation_syllables(&word.phonemes()).is_err() {
                        unsyllabifiable_cmevla_found = true;
                        if dictionary_unsyllabifiable_cmevla_reason(entry_word, word_type).is_none()
                        {
                            failures.push(format!(
                                "{entry_word} [{}] contains unsyllabifiable cmevla `{}` without allowlist reason",
                                word_type.as_str(),
                                word.phonemes().as_str()
                            ));
                        }
                    }
                }
                _ => {
                    if let Err(error) = pronunciation_syllables(&word.phonemes()) {
                        unsyllabifiable_non_cmevla_found = true;
                        if dictionary_ipa_xfail_reason(entry_word, word_type).is_none() {
                            failures.push(format!(
                                "{entry_word} [{}] contains non-cmevla `{}` that failed strict syllabification: {error}",
                                word_type.as_str(),
                                word.phonemes().as_str()
                            ));
                        }
                    }
                }
            }
        }
        if dictionary_unsyllabifiable_cmevla_reason(entry_word, word_type).is_some()
            && !unsyllabifiable_cmevla_found
        {
            failures.push(format!(
                "{entry_word} [{}] still has stale unsyllabifiable-cmevla exception",
                word_type.as_str()
            ));
        }
        if dictionary_ipa_xfail_reason(entry_word, word_type).is_some()
            && !unsyllabifiable_non_cmevla_found
            && ipa_rendered
        {
            failures.push(format!(
                "{entry_word} [{}] still has stale non-cmevla syllable exception",
                word_type.as_str()
            ));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn dictionary_category_matches(word_type: WordType, words: &[WordLike]) -> bool {
        match word_type {
            WordType::Gismu | WordType::ExperimentalGismu => {
                single_plain_word_kind(words) == Some(WordKind::Gismu)
            }
            WordType::Lujvo => single_plain_word_kind(words) == Some(WordKind::Lujvo),
            WordType::Fuivla | WordType::ObsoleteFuivla => {
                single_plain_word_kind(words) == Some(WordKind::Fuhivla)
            }
            WordType::Cmevla | WordType::ObsoleteCmevla => {
                single_plain_word_kind(words) == Some(WordKind::Cmevla)
            }
            WordType::Cmavo | WordType::ExperimentalCmavo | WordType::ObsoleteCmavo => {
                single_plain_word_kind(words) == Some(WordKind::Cmavo)
            }
            WordType::CmavoCompound => {
                words.len() >= 2
                    && words.iter().all(|word| {
                        single_plain_word_kind(std::slice::from_ref(word)) == Some(WordKind::Cmavo)
                    })
            }
            WordType::BuLetteral => words.len() == 1 && contains_lerfu_word(&words[0]),
            WordType::ZeiLujvo | WordType::ObsoleteZeiLujvo => {
                words.len() == 1 && contains_zei_compound(&words[0])
            }
            WordType::Phrase => {
                words.len() >= 2
                    || words.first().is_some_and(|word| {
                        // Magic words such as BU and ZEI fold multiple written words into
                        // one structured morphology node.
                        !matches!(word.as_data(), data!(WordLike::PlainWord(_)))
                    })
            }
            // Lensisku's untyped catch-all claims no morphological category,
            // so there is nothing for the entry's morphology to contradict.
            WordType::Nalvla => true,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn single_plain_word_kind(words: &[WordLike]) -> Option<WordKind> {
        let [word] = words else {
            return None;
        };
        match word.as_data() {
            data!(WordLike::PlainWord(word)) => Some(word.kind()),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn contains_lerfu_word(word: &WordLike) -> bool {
        match word.as_data() {
            data!(WordLike::LerfuWord { .. }) => true,
            data!(WordLike::ZeiCompound { left, .. }) => contains_lerfu_word(left),
            _ => false,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn contains_zei_compound(word: &WordLike) -> bool {
        match word.as_data() {
            data!(WordLike::ZeiCompound { .. }) => true,
            data!(WordLike::LerfuWord { base, .. }) => contains_zei_compound(base),
            _ => false,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn plain_words(words: &[WordLike]) -> Vec<&Word> {
        let mut plain = Vec::new();
        for word in words {
            append_plain_words(word, &mut plain);
        }
        plain
    }

    #[requires(true)]
    #[ensures(true)]
    fn append_plain_words<'word>(word: &'word WordLike, plain: &mut Vec<&'word Word>) {
        match word.as_data() {
            data!(WordLike::PlainWord(word)) => plain.push(word),
            data!(WordLike::QuotedWord { zo, word }) => {
                plain.push(zo);
                plain.push(word);
            }
            data!(WordLike::SelmahoQuotedWord { mahoi, word }) => {
                plain.push(mahoi);
                plain.push(word);
            }
            data!(WordLike::DelimitedNonLojbanQuote {
                zoi,
                opening_delimiter,
                closing_delimiter,
                ..
            }) => {
                plain.push(zoi);
                plain.push(opening_delimiter);
                plain.push(closing_delimiter);
            }
            data!(WordLike::QuotedWords {
                lohu,
                quoted_words,
                lehu,
            }) => {
                plain.push(lohu);
                plain.extend(quoted_words.iter());
                plain.push(lehu);
            }
            data!(WordLike::ReplacementQuote { runs, lehai }) => {
                for run in runs {
                    plain.push(&run.marker);
                    for word in &run.words {
                        append_plain_words(&word.word, plain);
                    }
                }
                plain.push(lehai);
            }
            data!(WordLike::DelimitedWordQuote { marker, .. }) => plain.push(marker),
            data!(WordLike::LerfuWord { base, bu }) => {
                append_plain_words(base, plain);
                plain.push(bu);
            }
            data!(WordLike::ZeiCompound { left, zei, right }) => {
                append_plain_words(left, plain);
                plain.push(zei);
                plain.push(right);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn dictionary_parse_xfail_reason(_word: &str, _word_type: WordType) -> Option<&'static str> {
        dictionary_exception_reason(DICTIONARY_PARSE_XFAILS, _word, _word_type)
    }

    #[requires(true)]
    #[ensures(true)]
    fn dictionary_category_exception_reason(
        _word: &str,
        _word_type: WordType,
    ) -> Option<&'static str> {
        dictionary_exception_reason(DICTIONARY_CATEGORY_EXCEPTIONS, _word, _word_type)
    }

    #[requires(true)]
    #[ensures(true)]
    fn dictionary_unsyllabifiable_cmevla_reason(
        word: &str,
        word_type: WordType,
    ) -> Option<&'static str> {
        dictionary_exception_reason(DICTIONARY_UNSYLLABIFIABLE_CMEVLA, word, word_type)
    }

    #[requires(true)]
    #[ensures(true)]
    fn dictionary_ipa_xfail_reason(word: &str, word_type: WordType) -> Option<&'static str> {
        dictionary_exception_reason(DICTIONARY_IPA_XFAILS, word, word_type)
    }

    #[requires(true)]
    #[ensures(true)]
    fn dictionary_exception_reason(
        exceptions: &'static str,
        word: &str,
        word_type: WordType,
    ) -> Option<&'static str> {
        exceptions.lines().find_map(|line| {
            let mut fields = line.splitn(3, '\t');
            let exception_word = fields.next()?;
            let exception_type = fields.next()?;
            let reason = fields.next()?;
            (exception_word == word && exception_type == word_type.as_str()).then_some(reason)
        })
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
        assert!(
            !pretty_tree_with_options(&parsed, source, TreeRenderOptions::default())
                .expect("tree")
                .contains("vau")
        );
        assert!(
            !pretty_brackets_with_options(&parsed, source, BracketRenderOptions::default())
                .expect("brackets")
                .contains("vau")
        );
        assert!(
            !compact_generated_model_json_string_with_options(
                &parsed,
                JsonRenderOptions::default()
            )
            .expect("json")
            .contains("\"elided\"")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_model_tree_renders_leading_i_statement_marker() {
        let source = "i mi klama";
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        let parsed = parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            source,
            &ParseOptions::default(),
        )
        .expect("valid syntax");
        let regular_text = parsed.as_ref();
        assert_eq!(regular_text.leading_i_statements.len(), 1);
        let tree = pretty_generated_model_tree_with_options(
            parsed.as_ref(),
            source,
            TreeRenderOptions {
                show_spans: true,
                ..TreeRenderOptions::default()
            },
        )
        .expect("tree");

        assert!(tree.contains("i: Cmavo @[0‥1) \"i\""), "{tree}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn text_renderers_show_representative_elided_terminators() {
        let tree = render_tree_with_elided("mi klama");
        assert!(tree.contains("vau: Cmavo @[8‥8) \"vau\""), "{tree}");

        let description = render_tree_with_elided("le broda");
        assert!(
            description.contains("ku: Cmavo @[8‥8) \"ku\""),
            "{description}"
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
            colorized_brackets.contains("\x1b[3mtoi\x1b[23m"),
            "{colorized_brackets:?}"
        );
        assert!(!colorized_brackets.contains("\x1b[9m"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bracket_script_option_renders_word_leaves() {
        assert_eq!(
            render_morphology_brackets_with_script("mi klama", LojbanScript::Cyrillic),
            "(ми кла\u{0301}ма)"
        );
        assert_eq!(
            render_morphology_brackets_with_script("mi klama", LojbanScript::Zbalermorna),
            "(\u{ed87}\u{eda2} \u{ed82}\u{ed84}\u{eda0}\u{ed98}\u{ed87}\u{eda0})"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bracket_script_option_leaves_quote_payload_text_verbatim() {
        let cyrillic =
            render_morphology_brackets_with_script("zoi gy raw_payload gy", LojbanScript::Cyrillic);
        assert!(cyrillic.contains("raw_payload"), "{cyrillic}");
        assert!(cyrillic.contains("зой"), "{cyrillic}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn json_renderer_marks_elided_terminators_with_zero_length_spans() {
        let parsed = parse("li pa");
        let json = compact_generated_model_json_string_with_options(
            &parsed,
            JsonRenderOptions {
                show_elided: true,
                ..JsonRenderOptions::default()
            },
        )
        .expect("json");
        let value = serde_json::from_str(&json).expect("valid json");
        assert!(has_elided_cmavo(&value, "lo'o", [5, 5]), "{json}");
        assert!(has_elided_cmavo(&value, "vau", [5, 5]), "{json}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn json_formatter_colorizes_typed_roles() {
        let options = JsonRenderOptions {
            indent: crate::Indentation::compact(),
            color: true,
            ..JsonRenderOptions::default()
        };
        let value = serde_json::json!({
            "field": { "Bridi": {} },
            "string": "value",
        });
        let standard = render_json_value_with_options(&value, options);

        assert!(
            standard.contains("\x1b[32m\"field\"\x1b[39m"),
            "{standard:?}"
        );
        assert!(
            standard.contains("\x1b[94m\"Bridi\"\x1b[39m"),
            "{standard:?}"
        );
        assert!(
            standard.contains("\x1b[33m\"value\"\x1b[39m"),
            "{standard:?}"
        );
        assert!(standard.contains("\x1b[90m{\x1b[39m"), "{standard:?}");
        assert!(standard.contains("\x1b[90m}\x1b[39m"), "{standard:?}");
        assert!(!standard.contains("\x1b[36m"));

        let compact = format_compact_json_value(&serde_json::json!({ "Bridi": {} }), 0, options);
        assert!(compact.contains("\x1b[94m\"Bridi\"\x1b[39m"), "{compact:?}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn standard_json_formatter_matches_serde_without_color() {
        let value = serde_json::json!({
            "array": [1, true, null, "value"],
            "object": { "field": "text" },
        });

        assert_eq!(
            render_json_value_with_options(
                &value,
                JsonRenderOptions {
                    indent: crate::Indentation::compact(),
                    ..JsonRenderOptions::default()
                },
            ),
            serde_json::to_string(&value).expect("value serializes")
        );
        assert_eq!(
            render_json_value_with_options(
                &value,
                JsonRenderOptions {
                    indent: crate::Indentation::default(),
                    ..JsonRenderOptions::default()
                },
            ),
            serde_json::to_string_pretty(&value).expect("value serializes")
        );
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

    #[requires(!source.is_empty())]
    #[ensures(!ret.is_empty())]
    fn render_morphology_brackets_with_script(source: &str, script: LojbanScript) -> String {
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        pretty_morphology_brackets_with_options(
            &words,
            source,
            BracketRenderOptions {
                script,
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

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn ipa_morphology_text_with_limit(
    words: &[WordLike],
    source: &str,
    limit: budget::ByteLimit,
) -> Result<String, OutputError> {
    jbotci_phonetic::ipa_morphology_text_with_limit(words, source, limit).map_err(|error| {
        match error {
            jbotci_phonetic::PhoneticError::OutputLimit { .. } => OutputError::Limit { limit },
            error => OutputError::Ipa(error.to_string()),
        }
    })
}

#[cfg(test)]
mod bounded_output_tests {
    use super::*;
    use budget::ByteLimit;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_projection_refuses_before_visiting_the_complete_result() {
        let source = "mi ".repeat(1333);
        let words = jbotci_morphology::segment_words_with_modifiers(&source).unwrap();
        let small = ByteLimit::new(64).unwrap();
        let projection = ByteLimit::new(4096).unwrap();
        crate::json::reset_projection_visits();
        let result = compact_morphology_json_string_with_limits(
            &words,
            JsonRenderOptions::default(),
            small,
            projection,
        );
        assert!(matches!(result, Err(OutputError::Limit { .. })));
        let visits = crate::json::projection_visits();
        assert!(
            visits > 0 && visits < words.len(),
            "visited {visits} of {}",
            words.len()
        );
        let source = format!("{source}@");
        let recovered = jbotci_morphology::segment_words_with_modifiers_recovered(&source);
        assert!(!recovered.errors.is_empty());
        crate::json::reset_projection_visits();
        let result = compact_recovered_morphology_json_string_with_limits(
            &recovered,
            &source,
            JsonRenderOptions::default(),
            small,
            projection,
        );
        assert!(matches!(result, Err(OutputError::Limit { .. })));
        let visits = crate::json::projection_visits();
        assert!(
            visits > 0 && visits < recovered.words.len(),
            "visited {visits} of {}",
            recovered.words.len()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bounded_morphology_projection_preserves_strict_and_recovered_output() {
        let unlimited = ByteLimit::new(usize::MAX).unwrap();
        for source in [
            "mi klama",
            "zo broda cu melbi",
            "zoi gy hello gy",
            "mi broda zei brode",
            "mi @ do",
        ] {
            let recovered = jbotci_morphology::segment_words_with_modifiers_recovered(source);
            let options = JsonRenderOptions::default();
            let expected = if recovered.errors.is_empty() {
                compact_morphology_json_string_with_options(&recovered.words, options).unwrap()
            } else {
                let items = crate::recovered::recovered_morphology_sequence_for_json_test(
                    &recovered, source,
                );
                render_compact_json_value_with_limit(&items, options, unlimited).unwrap()
            };
            let actual = compact_recovered_morphology_json_string_with_limits(
                &recovered, source, options, unlimited, unlimited,
            )
            .unwrap();
            assert_eq!(actual, expected, "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_formats_refuse_one_byte_below_the_exact_output() {
        let source = "mi klama lo zarci";
        let words = jbotci_morphology::segment_words_with_modifiers(source).expect("morphology");
        for indent in [
            Indentation::compact(),
            Indentation::try_from(8).expect("bounded indentation"),
        ] {
            let tree_options = TreeRenderOptions {
                indent,
                ..TreeRenderOptions::default()
            };
            let json_options = JsonRenderOptions {
                indent,
                ..JsonRenderOptions::default()
            };
            let raw_indent = (indent == Indentation::compact()).then_some(indent);
            let unlimited = ByteLimit::new(usize::MAX).unwrap();
            let outputs = [
                compact_morphology_json_string_with_limits(
                    &words,
                    json_options,
                    unlimited,
                    unlimited,
                )
                .unwrap(),
                pretty_morphology_tree_with_limit(&words, source, tree_options, unlimited).unwrap(),
                pretty_morphology_brackets_with_limit(
                    &words,
                    source,
                    BracketRenderOptions::default(),
                    unlimited,
                )
                .unwrap(),
                ipa_morphology_text_with_limit(&words, source, unlimited).unwrap(),
                raw_with_limit(&words, raw_indent, unlimited).unwrap(),
            ];
            for (format, expected) in outputs.iter().enumerate() {
                for bytes in [expected.len() - 1, expected.len()] {
                    let limit = ByteLimit::new(bytes).unwrap();
                    let result = match format {
                        0 => compact_morphology_json_string_with_limits(
                            &words,
                            json_options,
                            limit,
                            unlimited,
                        ),
                        1 => pretty_morphology_tree_with_limit(&words, source, tree_options, limit),
                        2 => pretty_morphology_brackets_with_limit(
                            &words,
                            source,
                            BracketRenderOptions::default(),
                            limit,
                        ),
                        3 => ipa_morphology_text_with_limit(&words, source, limit),
                        _ => raw_with_limit(&words, raw_indent, limit),
                    };
                    if bytes == expected.len() {
                        assert_eq!(result.unwrap(), *expected);
                    } else {
                        assert!(
                            matches!(result, Err(OutputError::Limit { .. })),
                            "format={format}: {result:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn escaped_utf8_json_keeps_the_typed_limit_error() {
        let value = serde_json::json!({"text": "😀\"\n\\"});
        let options = JsonRenderOptions::default();
        let expected = render_json_value_with_options(&value, options);
        assert_eq!(
            render_json_value_with_limit(&value, options, ByteLimit::new(expected.len()).unwrap())
                .unwrap(),
            expected
        );
        assert!(matches!(
            render_json_value_with_limit(
                &value,
                options,
                ByteLimit::new(expected.len() - 1).unwrap()
            ),
            Err(OutputError::Limit { .. })
        ));
    }
}
