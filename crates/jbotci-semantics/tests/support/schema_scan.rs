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

/// A parsed enum: rename policy, tagging, variants, and serialize flag.
///
/// `manual_serialize` means the enum has a hand-written `impl Serialize` (it
/// serializes as a scalar string, e.g. `SemanticSort`), so it has no JSON object
/// shape even if its variants carry data.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub rename: String,
    pub untagged: bool,
    /// Internal-tag field name (`#[serde(tag = "...")]`), if any.
    pub tag: Option<String>,
    /// Adjacent `content` field name (`#[serde(content = "...")]`), if any.
    pub content: Option<String>,
    pub variants: Vec<Variant>,
    pub serialize: bool,
    pub manual_serialize: bool,
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

/// Extract a serde `key = "value"` from an attribute line.
#[requires(!key.is_empty())]
#[ensures(true)]
fn serde_attr_value(attr: &str, key: &str) -> Option<String> {
    let needle = format!("{key} = \"");
    let idx = attr.find(&needle)?;
    let rest = &attr[idx + needle.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

/// Parse a `[pub ] NAME: TYPE` field line (struct fields are `pub`; enum
/// struct-variant fields are not). `prev_has_skip` / `prev_rename` come from the
/// preceding `#[serde(...)]` line; an explicit `rename` wins over camelCase.
#[requires(true)]
#[ensures(true)]
fn parse_field(line: &str, prev_has_skip: bool, prev_rename: Option<&str>) -> Option<Field> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') || trimmed.starts_with("//") {
        return None;
    }
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
    let key = prev_rename.map(str::to_owned).unwrap_or_else(|| camel_case(name));
    Some(Field {
        key,
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
            let manual_serialize = has_manual_serialize(source, &name);
            let serialize = preceding_derives_serialize(&lines, i) || manual_serialize;
            let mut rename = String::new();
            let mut untagged = false;
            let mut tag = None;
            let mut content = None;
            let start = i.saturating_sub(10);
            for attr in &lines[start..i] {
                if let Some(value) = serde_attr_value(attr, "rename_all") {
                    rename = value;
                }
                if let Some(value) = serde_attr_value(attr, "tag") {
                    tag = Some(value);
                }
                if let Some(value) = serde_attr_value(attr, "content") {
                    content = Some(value);
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
                    tag,
                    content,
                    variants,
                    serialize,
                    manual_serialize,
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
    let mut prev_rename: Option<String> = None;
    while i < lines.len() && depth > 0 {
        let line = lines[i];
        // Fields live one level deep and are `[pub ] name: Type` (struct fields
        // are `pub`; enum struct-variant fields are not).
        if depth == 1
            && let Some(field) = parse_field(line, prev_skip, prev_rename.as_deref())
        {
            fields.push(field);
        }
        prev_skip = line.contains("skip_serializing_if");
        prev_rename = serde_attr_value(line, "rename");
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
/// Enums that are not surfaces at all — dispatch/ID/type-tag internals.
const INTERNAL_ENUMS: &[&str] = &[
    "SemanticObject",     // the object dispatch, represented by the 13 Object surfaces
    "SemanticObjectKind", // the `type` tag value
    "SemanticIdPrefix",   // internal to ID strings
    "SemanticOperator",   // internal operator dispatch
];
/// Enums excluded from the *variant-discriminant* surface (a superset of the
/// internals): `RelationLabel` serializes as free-form relation text, and
/// `QuestionSlot` (untagged struct-enum) is inventoried only as a value struct.
const ENUM_EXCLUDE: &[&str] = &[
    "SemanticObject",
    "SemanticObjectKind",
    "SemanticIdPrefix",
    "SemanticOperator",
    "RelationLabel",
    "QuestionSlot",
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
        let mut enums = BTreeMap::new();
        for (name, def) in &model.enums {
            if !def.serialize || INTERNAL_ENUMS.contains(&name.as_str()) {
                continue;
            }
            // Discriminant enums are inventoried by their Rust variant names.
            if !ENUM_EXCLUDE.contains(&name.as_str()) {
                enums.insert(name.clone(), def.variants.iter().map(|v| v.name.clone()).collect());
            }
            // Enums that serialize as a JSON object also contribute a value-struct
            // member set the inventory must cover. Manually-serialized enums
            // (scalar strings, e.g. SemanticSort) never do.
            if !def.manual_serialize {
                if let Some(fields) = enum_object_fields(def) {
                    value_structs.insert(name.clone(), fields);
                }
            }
        }

        SerializedSurface {
            value_structs,
            enums,
            node_keys: model.node_keys,
        }
    }
}

/// The union of struct-variant field keys of an enum.
#[requires(true)]
#[ensures(true)]
fn variant_field_union(def: &EnumDef) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for variant in &def.variants {
        for field in &variant.fields {
            if !keys.contains(&field.key) {
                keys.push(field.key.clone());
            }
        }
    }
    keys
}

/// The JSON object member set an enum serializes to, if any — the fields the
/// inventory must cover beyond the variant discriminants.
///
/// * untagged struct variants (`QuestionSlot`) -> the union of variant fields;
/// * internally tagged (`tag`, maybe `content`) -> the tag key plus either the
///   content key (`IntervalModifier` -> `{kind, value}`) or the variant fields
///   (`ScopeDependence` -> `{kind, mayDependOn}`);
/// * externally tagged with struct variants (`SequenceRelation`) -> the variant
///   fields (nested under the variant key, but the fields must still be listed).
///
/// Untagged/external newtype or unit-only enums have no fixed object shape.
#[requires(true)]
#[ensures(true)]
fn enum_object_fields(def: &EnumDef) -> Option<Vec<String>> {
    let has_struct_variant = def.variants.iter().any(|variant| variant.is_struct);
    if let Some(tag) = &def.tag {
        let mut fields = vec![tag.clone()];
        if let Some(content) = &def.content {
            fields.push(content.clone());
        } else {
            fields.extend(variant_field_union(def));
        }
        return Some(fields);
    }
    if has_struct_variant {
        return Some(variant_field_union(def));
    }
    None
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

        // Non-pub enum-struct-variant fields must be parsed (blocker 2). Before
        // the fix these member sets were empty.
        let members = |name: &str| surface.value_structs.get(name).cloned().unwrap_or_default();
        let has = |name: &str, field: &str| members(name).iter().any(|k| k == field);
        // QuestionSlot (untagged struct-enum) — union of private variant fields.
        for field in ["parameter", "role", "kind", "domain"] {
            assert!(has("QuestionSlot", field), "QuestionSlot missing {field}");
        }
        // ScopeDependence (internal tag "kind", private struct-variant field
        // renamed to mayDependOn) — object shape {kind, mayDependOn}.
        assert!(has("ScopeDependence", "kind") && has("ScopeDependence", "mayDependOn"));
        // IntervalModifier (internal tag "kind", content "value").
        assert!(has("IntervalModifier", "kind") && has("IntervalModifier", "value"));
        // SequenceRelation (external tag) — ParagraphBoundary's inline fields.
        assert!(has("SequenceRelation", "transition") && has("SequenceRelation", "additional"));

        // Roughly the right size (47 value structs + the enum object-shapes).
        assert!(
            surface.value_structs.len() >= 47,
            "expected >=47 value structs, got {}",
            surface.value_structs.len()
        );
    }
}
