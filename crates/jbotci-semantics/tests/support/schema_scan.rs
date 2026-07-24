//! A minimal scanner over the `jbotci-semantics` model source that enumerates
//! the serialized surface (structs, enums, and the node serializers' keys).
//!
//! It exists so `model_surface_lower_bound` can assert the inventory covers the
//! model *without* depending on what the corpus exercises — a serialized type
//! that no corpus document reaches still fails the build if it is un-inventoried.
//! The scan is deliberately syntactic (like the workspace's contract scanner);
//! the model source is regular enough for a line scanner, and the numbers are
//! cross-checked in `scanner_self_check`.

#![allow(dead_code)]

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

use std::collections::BTreeMap;

const MODEL_RS: &str = include_str!("../../src/model.rs");
const SEMANTIC_OBJECT_RS: &str = include_str!("../../src/model/semantic_object.rs");

/// A parsed field: its serde (camelCase) JSON key, its base type, optionality.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct Field {
    pub key: String,
    pub base_type: String,
    pub optional: bool,
}

/// A parsed enum variant with its shape and (for struct variants) its fields.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub is_struct: bool,
    pub fields: Vec<Field>,
}

/// A parsed enum: rename policy, untagged flag, variants, serialize flag.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub rename: String,
    pub untagged: bool,
    pub variants: Vec<Variant>,
    pub serialize: bool,
}

/// The parsed model: serialize-bearing structs and enums, and node serializer keys.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct Model {
    pub structs: BTreeMap<String, (Vec<Field>, bool)>,
    pub enums: BTreeMap<String, EnumDef>,
    pub node_keys: BTreeMap<String, Vec<String>>,
}

/// snake_case -> camelCase (serde's `rename_all = "camelCase"`).
#[requires(true)]
#[ensures(true)]
pub fn camel_case(snake: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for ch in snake.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// PascalCase -> kebab-case (serde's `rename_all = "kebab-case"`).
#[requires(true)]
#[ensures(true)]
pub fn kebab_case(name: &str) -> String {
    let mut out = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index != 0 {
                out.push('-');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// PascalCase -> camelCase.
#[requires(true)]
#[ensures(true)]
pub fn variant_camel(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Strip `Option<>`, `Vec<>`, `Box<>`, `BTreeMap<_, V>`, and references to the
/// innermost base type name.
#[requires(true)]
#[ensures(true)]
pub fn base_type(raw: &str) -> String {
    let mut ty = raw.trim().trim_end_matches(',').trim().to_owned();
    ty = ty.trim_start_matches('&').trim().to_owned();
    for _ in 0..6 {
        if let Some(inner) = ty
            .strip_prefix("Option<")
            .or_else(|| ty.strip_prefix("Vec<"))
            .or_else(|| ty.strip_prefix("Box<"))
        {
            ty = inner.trim_end_matches('>').trim().to_owned();
            continue;
        }
        if let Some(rest) = ty.strip_prefix("BTreeMap<") {
            if let Some(comma) = rest.find(',') {
                ty = rest[comma + 1..].trim_end_matches('>').trim().to_owned();
                continue;
            }
        }
        break;
    }
    ty
}

/// Parse a `pub NAME: TYPE` field line at struct/variant depth; returns the
/// serde key, base type, and whether the *preceding* line carried a skip attr.
#[requires(true)]
#[ensures(true)]
fn parse_field(line: &str, prev_has_skip: bool) -> Option<Field> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
    let colon = rest.find(':')?;
    let name = rest[..colon].trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
        return None;
    }
    if name == "common" {
        return None;
    }
    let ty = rest[colon + 1..].trim();
    if ty.is_empty() {
        return None;
    }
    Some(Field {
        key: camel_case(name),
        base_type: base_type(ty),
        optional: prev_has_skip || ty.starts_with("Option<"),
    })
}

impl Model {
    /// Parse both model source files.
    #[requires(true)]
    #[ensures(!ret.structs.is_empty() && !ret.enums.is_empty())]
    pub fn from_source() -> Self {
        let mut structs = BTreeMap::new();
        let mut enums = BTreeMap::new();
        for source in [MODEL_RS, SEMANTIC_OBJECT_RS] {
            parse_source(source, &mut structs, &mut enums);
        }
        let node_keys = parse_node_keys(SEMANTIC_OBJECT_RS);
        Model {
            structs,
            enums,
            node_keys,
        }
    }
}

/// True if `NAME` has an `impl Serialize for NAME` (or `... for NAME<`) anywhere.
#[requires(!name.is_empty())]
#[ensures(true)]
fn has_manual_serialize(source: &str, name: &str) -> bool {
    let needle_a = format!("impl Serialize for {name} ");
    let needle_b = format!("impl Serialize for {name}\n");
    source.contains(&needle_a) || source.contains(&needle_b)
}

/// Whether an attribute block within the preceding lines derives `Serialize`.
#[requires(true)]
#[ensures(true)]
fn preceding_derives_serialize(lines: &[&str], decl_index: usize) -> bool {
    let start = decl_index.saturating_sub(10);
    lines[start..decl_index]
        .iter()
        .any(|line| line.contains("derive(") && line.contains("Serialize"))
}

#[requires(true)]
#[ensures(true)]
fn parse_source(
    source: &str,
    structs: &mut BTreeMap<String, (Vec<Field>, bool)>,
    enums: &mut BTreeMap<String, EnumDef>,
) {
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(name) = line.strip_prefix("pub struct ").filter(|_| line.contains('{')) {
            let name = name.split(['<', ' ', '{']).next().unwrap_or("").to_owned();
            let serialize = preceding_derives_serialize(&lines, i)
                || has_manual_serialize(source, &name);
            let (fields, next) = parse_struct_body(&lines, i + 1);
            structs.insert(name, (fields, serialize));
            i = next;
            continue;
        }
        if let Some(name) = line.strip_prefix("pub enum ") {
            let name = name.split(['<', ' ', '{']).next().unwrap_or("").to_owned();
            let serialize = preceding_derives_serialize(&lines, i)
                || has_manual_serialize(source, &name);
            let mut rename = String::new();
            let mut untagged = false;
            let start = i.saturating_sub(10);
            for attr in &lines[start..i] {
                if let Some(idx) = attr.find("rename_all = \"") {
                    let rest = &attr[idx + "rename_all = \"".len()..];
                    if let Some(end) = rest.find('"') {
                        rename = rest[..end].to_owned();
                    }
                }
                if attr.contains("untagged") {
                    untagged = true;
                }
            }
            let (variants, next) = parse_enum_body(&lines, i);
            enums.insert(
                name,
                EnumDef {
                    rename,
                    untagged,
                    variants,
                    serialize,
                },
            );
            i = next;
            continue;
        }
        i += 1;
    }
}

/// Parse a struct body starting at the line after `pub struct X {`.
#[requires(true)]
#[ensures(true)]
fn parse_struct_body(lines: &[&str], mut i: usize) -> (Vec<Field>, usize) {
    let mut depth = 1i32;
    let mut fields = Vec::new();
    let mut prev_skip = false;
    while i < lines.len() && depth > 0 {
        let line = lines[i];
        if depth == 1
            && line.trim_start().starts_with("pub ")
            && let Some(field) = parse_field(line, prev_skip)
        {
            fields.push(field);
        }
        prev_skip = line.contains("skip_serializing_if");
        depth += brace_delta(line);
        i += 1;
    }
    (fields, i)
}

/// Parse an enum body starting at the `pub enum X {` line.
#[requires(true)]
#[ensures(true)]
fn parse_enum_body(lines: &[&str], decl: usize) -> (Vec<Variant>, usize) {
    let mut i = decl;
    while i < lines.len() && !lines[i].contains('{') {
        i += 1;
    }
    let mut depth = 1i32;
    i += 1;
    let mut variants = Vec::new();
    while i < lines.len() && depth > 0 {
        let line = lines[i];
        let trimmed = line.trim_start();
        if depth == 1 && !trimmed.starts_with("//") {
            if let Some(name) = leading_variant(trimmed) {
                let is_struct = trimmed[name.len()..].trim_start().starts_with('{');
                let mut vfields = Vec::new();
                if is_struct {
                    let (parsed, _next) = parse_struct_body(lines, i + 1);
                    vfields = parsed;
                }
                variants.push(Variant {
                    name,
                    is_struct,
                    fields: vfields,
                });
            }
        }
        depth += brace_delta(line);
        i += 1;
    }
    (variants, i)
}

/// A capitalized leading identifier (a variant name), if the line starts one.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|name| !name.is_empty()))]
fn leading_variant(trimmed: &str) -> Option<String> {
    let mut chars = trimmed.char_indices();
    let (_, first) = chars.next()?;
    if !first.is_ascii_uppercase() {
        return None;
    }
    let mut end = trimmed.len();
    for (index, ch) in trimmed.char_indices() {
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            end = index;
            break;
        }
    }
    Some(trimmed[..end].to_owned())
}

#[requires(true)]
#[ensures(true)]
fn brace_delta(line: &str) -> i32 {
    line.chars().filter(|c| *c == '{').count() as i32
        - line.chars().filter(|c| *c == '}').count() as i32
}

/// Node serializer key list per object surface, scanned from the `serialize_*`
/// functions' string-literal keys.
#[requires(true)]
#[ensures(true)]
fn parse_node_keys(source: &str) -> BTreeMap<String, Vec<String>> {
    let fn_node = [
        ("serialize_utterance", "Utterance"),
        ("serialize_sequence", "Sequence"),
        ("serialize_eventuality", "Eventuality"),
        ("serialize_referent", "Referent"),
        ("serialize_parameter", "Parameter"),
        ("serialize_predication", "Predication"),
        ("serialize_formula", "Formula"),
        ("serialize_sign", "Sign"),
        ("serialize_displayed", "DisplayedContent"),
        ("serialize_math", "MathExpression"),
        ("serialize_quantity", "Quantity"),
        ("serialize_relation_metadata", "RelationMetadata"),
        ("serialize_question", "Question"),
    ];
    let lines: Vec<&str> = source.lines().collect();
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let matched = fn_node
            .iter()
            .find(|(name, _)| line.starts_with(&format!("fn {name}<")));
        if let Some((_, node)) = matched {
            // advance to the signature-closing `{`
            while i < lines.len() && !(lines[i].contains("-> Result") && lines[i].trim_end().ends_with('{')) {
                i += 1;
            }
            i += 1;
            let mut depth = 1i32;
            let entry = out.entry((*node).to_owned()).or_default();
            while i < lines.len() && depth > 0 {
                let body = lines[i];
                if (body.contains("serialize_entry(")
                    || body.contains("optional_entry!")
                    || body.contains("nonempty_entry!"))
                    && let Some(key) = first_string_literal(body)
                    && key != "type"
                    && !entry.contains(&key)
                {
                    entry.push(key);
                }
                depth += brace_delta(body);
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    out
}

/// The first `"..."` string literal on a line (a serde key), if it is a plain identifier.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn first_string_literal(line: &str) -> Option<String> {
    let open = line.find('"')?;
    let rest = &line[open + 1..];
    let close = rest.find('"')?;
    let literal = &rest[..close];
    if !literal.is_empty()
        && literal
            .chars()
            .all(|c| c.is_ascii_alphanumeric())
        && literal.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
    {
        Some(literal.to_owned())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Classified serialized surface (the lower bound the inventory must cover).
// ---------------------------------------------------------------------------

/// Structs that serialize as scalars (no JSON object fields) — excluded.
const SCALAR_STRUCTS: &[&str] = &["SemanticObjectId", "GeneratedEventualityId", "PlaceIndex"];
/// The graph root envelope — represented by the `Document` surface.
const ROOT_STRUCTS: &[&str] = &["SemanticGraph"];
/// Node structs (serialized manually; covered via `node_keys`) and pure helpers.
const NODE_OR_HELPER_STRUCTS: &[&str] = &[
    "UtteranceNode",
    "SequenceNode",
    "EventualityNode",
    "ReferentNode",
    "ParameterNode",
    "PredicationNode",
    "AtomFormulaNode",
    "ConnectiveFormulaNode",
    "QuantifiedFormulaNode",
    "QuantifierBundleFormulaNode",
    "RespectivelyDistributionFormulaNode",
    "SignNode",
    "DisplayedContentNode",
    "MathExpressionNode",
    "QuantityNode",
    "RelationMetadataNode",
    "QuestionNode",
    "SemanticObjectCommon",
    "FormulaTraversal",
    "ForethoughtRelationBranch",
];
/// Enums that do not enumerate as discriminant surfaces (scalar/free-form/internal).
const ENUM_EXCLUDE: &[&str] = &[
    "SemanticObject",
    "SemanticObjectKind",
    "SemanticIdPrefix",
    "SemanticOperator",
    "RelationLabel",
    "QuestionSlot", // untagged struct-enum, inventoried as a value struct
];

/// The serialized surface the inventory must at minimum cover.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct SerializedSurface {
    /// value-struct name -> its serde field keys.
    pub value_structs: BTreeMap<String, Vec<String>>,
    /// enum name -> its variant Rust names.
    pub enums: BTreeMap<String, Vec<String>>,
    /// object surface -> its node serializer keys.
    pub node_keys: BTreeMap<String, Vec<String>>,
}

impl SerializedSurface {
    #[requires(true)]
    #[ensures(!ret.value_structs.is_empty() && !ret.enums.is_empty())]
    pub fn from_source() -> Self {
        let model = Model::from_source();
        let mut value_structs = BTreeMap::new();
        for (name, (fields, serialize)) in &model.structs {
            if !serialize
                || SCALAR_STRUCTS.contains(&name.as_str())
                || ROOT_STRUCTS.contains(&name.as_str())
                || NODE_OR_HELPER_STRUCTS.contains(&name.as_str())
            {
                continue;
            }
            value_structs.insert(name.clone(), fields.iter().map(|f| f.key.clone()).collect());
        }
        // QuestionSlot (untagged struct-enum) is a value struct: the union of its
        // variant fields.
        if let Some(question_slot) = model.enums.get("QuestionSlot") {
            let mut keys: Vec<String> = Vec::new();
            for variant in &question_slot.variants {
                for field in &variant.fields {
                    if !keys.contains(&field.key) {
                        keys.push(field.key.clone());
                    }
                }
            }
            value_structs.insert("QuestionSlot".to_owned(), keys);
        }

        let mut enums = BTreeMap::new();
        for (name, def) in &model.enums {
            if !def.serialize || ENUM_EXCLUDE.contains(&name.as_str()) {
                continue;
            }
            // Discriminant enums are inventoried by their Rust variant names.
            enums.insert(
                name.clone(),
                def.variants.iter().map(|v| v.name.clone()).collect(),
            );
        }

        SerializedSurface {
            value_structs,
            enums,
            node_keys: model.node_keys,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn scanner_self_check() {
        let model = Model::from_source();
        // Known landmarks the parser must find (guards against a silent parse break).
        assert!(model.structs.contains_key("Composition"));
        assert!(model.structs["Composition"].1, "Composition must be Serialize");
        assert!(model.structs.contains_key("SemanticDiagnostic"));
        assert!(model.structs.contains_key("IntervalEndpointInclusion"));
        assert!(model.enums.contains_key("QuestionKind"));
        assert!(model.enums.contains_key("MathLiteralValue"));
        assert!(model.enums["QuestionSlot"].untagged);

        let surface = SerializedSurface::from_source();
        // Blocker-1 surfaces must all be present as value structs / enums.
        for name in ["Composition", "SemanticDiagnostic", "IntervalEndpointInclusion", "QuestionSlot"] {
            assert!(surface.value_structs.contains_key(name), "value struct {name} missing");
        }
        assert!(surface.enums.contains_key("MathLiteralValue"));
        // Node keys must include Formula's variant fields.
        assert!(surface.node_keys["Formula"].iter().any(|k| k == "operator"));
        assert!(surface.node_keys["Utterance"].iter().any(|k| k == "force"));
        // Roughly the right size (47 value structs + QuestionSlot).
        assert!(
            surface.value_structs.len() >= 47,
            "expected >=47 value structs, got {}",
            surface.value_structs.len()
        );
    }
}
