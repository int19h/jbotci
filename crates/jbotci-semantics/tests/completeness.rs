//! Corpus-driven verification of the render-field completeness inventory.
//!
//! Phase-B step 2 (research repo `DESIGN-RECORD.md`). Every graph here is
//! re-derived from the vendored `<doc>.lojban` by *this* jbotci build — never
//! read from the frozen `<doc>.frozen.json`. Three checks:
//!
//! * `drift_guard` — every serde coordinate this build emits over the corpus is
//!   covered by an inventoried field entry, and no witnessed field entry points
//!   at a coordinate the build no longer emits.
//! * `witness_verification` — every `Witness::Corpus` pointer resolves: its
//!   field is populated (or its variant value present) in the named document.
//! * `frozen_divergence_report` — compares this build's graph against the frozen
//!   JSON and *reports* (never fails on) structural or value divergences.

#[allow(unused_imports)]
use bityzba::{ensures, requires};

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use jbotci_dialect::DialectDefinition;
use jbotci_morphology::{MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id};
use jbotci_semantics::completeness::corpus::CORPUS_DOCS;
use jbotci_semantics::completeness::model::{EntryKind, SurfaceCategory};
use jbotci_semantics::completeness::render_field_inventory;
use jbotci_semantics::{
    SemanticBuildOptions, SemanticGraph, build_generated_semantic_graph_with_dictionary_and_options,
};
use jbotci_source::SourceId;
use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};

/// Path to a vendored corpus fixture.
#[requires(!doc.is_empty() && !suffix.is_empty())]
#[ensures(true)]
fn fixture(doc: &str, suffix: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/phaseb_corpus");
    path.push(format!("{doc}.{suffix}"));
    path
}

/// Re-derive the semantic graph for a corpus document with this build.
#[requires(!doc.is_empty())]
#[ensures(true)]
fn graph_for(doc: &str) -> SemanticGraph {
    let text = std::fs::read_to_string(fixture(doc, "lojban"))
        .unwrap_or_else(|error| panic!("read {doc}.lojban: {error}"));
    let text = text.trim();
    let dialect = DialectDefinition::default();
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
    let source_id = Some(SourceId(format!("<phaseb:{doc}>")));
    let words =
        segment_words_with_modifiers_with_options_and_source_id(text, &morphology_options, source_id)
            .unwrap_or_else(|error| panic!("morphology {doc}: {error}"));
    let parsed = parse_syntax_tree_generated_model_with_source_and_options(&words, text, &syntax_options)
        .unwrap_or_else(|error| panic!("syntax {doc}: {error}"));
    build_generated_semantic_graph_with_dictionary_and_options(
        &parsed,
        SemanticBuildOptions {
            source_text: Some(text),
            story_time: false,
        },
        jbotci_dictionary_data::english(),
    )
    .unwrap_or_else(|error| panic!("semantics {doc}: {error}"))
}

/// The Rust node type a serialized object belongs to.
///
/// The JSON `type` tag collapses `Referent`/`Eventuality`/`Sign` onto
/// `"referent"`; the ID sort prefix (`eventuality*`, `sign`, else entity-like)
/// disambiguates — mirroring how the objects are constructed.
#[requires(!id.is_empty())]
#[ensures(!ret.is_empty())]
fn node_type(id: &str, json_type: &str) -> String {
    if json_type != "referent" {
        let mapped = match json_type {
            "utterance" => "Utterance",
            "sequence" => "Sequence",
            "parameter" => "Parameter",
            "predication" => "Predication",
            "formula" => "Formula",
            "displayedContent" => "DisplayedContent",
            "mathExpression" => "MathExpression",
            "quantity" => "Quantity",
            "relationMetadata" => "RelationMetadata",
            "question" => "Question",
            other => other,
        };
        return mapped.to_owned();
    }
    let prefix = id.split(':').next().unwrap_or("");
    if prefix.starts_with("eventuality") {
        "Eventuality".to_owned()
    } else if prefix == "sign" {
        "Sign".to_owned()
    } else {
        "Referent".to_owned()
    }
}

/// Canonicalize a JSON map key: numbered places (`x1`, `x2`, ...) collapse to
/// `x*` so per-place arguments share one coordinate.
#[requires(true)]
#[ensures(!ret.is_empty() || key.is_empty())]
fn canonical_key(key: &str) -> String {
    let is_place = key.len() > 1
        && key.starts_with('x')
        && key[1..].chars().all(|character| character.is_ascii_digit());
    if is_place { "x*".to_owned() } else { key.to_owned() }
}

/// What one corpus document exercises: the set of `"<NodeType>:<path>"`
/// coordinates, and the scalar values seen at each coordinate.
#[requires(true)]
#[ensures(true)]
fn walk_document(graph: &SemanticGraph) -> (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>) {
    let json = graph.to_json_string(0).expect("serialize graph");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse graph json");
    let objects = value
        .get("objects")
        .and_then(serde_json::Value::as_object)
        .expect("graph objects");
    let mut coords = BTreeSet::new();
    let mut values: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (id, object) in objects {
        let json_type = object.get("type").and_then(serde_json::Value::as_str).unwrap_or("");
        let node = node_type(id, json_type);
        walk_value(&node, "", object, &mut coords, &mut values);
    }
    (coords, values)
}

/// Recursively record coordinates and scalar values under `prefix`.
#[requires(!node.is_empty())]
#[ensures(true)]
fn walk_value(
    node: &str,
    prefix: &str,
    value: &serde_json::Value,
    coords: &mut BTreeSet<String>,
    values: &mut BTreeMap<String, BTreeSet<String>>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                if prefix.is_empty() && key == "type" {
                    continue;
                }
                let canonical = canonical_key(key);
                let path = if prefix.is_empty() {
                    canonical
                } else {
                    format!("{prefix}.{canonical}")
                };
                coords.insert(format!("{node}:{path}"));
                walk_value(node, &path, child, coords, values);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                walk_value(node, prefix, item, coords, values);
            }
        }
        scalar => {
            if prefix.is_empty() {
                return;
            }
            let rendered = match scalar {
                serde_json::Value::String(text) => text.clone(),
                serde_json::Value::Bool(flag) => flag.to_string(),
                serde_json::Value::Number(number) => number.to_string(),
                _ => return,
            };
            values
                .entry(format!("{node}:{prefix}"))
                .or_default()
                .insert(rendered);
        }
    }
}

/// Reduce a `"<NodeType>:<path>"` coordinate to a *serde edge*
/// `(parent-context, leaf)` — the struct-occurrence-invariant unit the drift
/// guard compares. A value struct (e.g. `SemanticSource`) recurs at many paths
/// (`source`, `assignedNames.source`, `arguments.x*.source`, ...) but always
/// emits the same edges (`source -> span`, `span -> byteStart`), so edges make
/// one inventory entry per struct field sufficient.
///
/// * A depth-1 object field yields `("OBJECT", field)` — node-agnostic, so the
///   shared `SemanticObjectCommon` fields (`source`, `diagnostics`) match under
///   every node type.
/// * A deeper field yields `(immediate-parent-segment, leaf)`.
/// * A dynamic map-instance coordinate (leaf `x*`, a per-place `ArgumentValue`)
///   is not a serde field and returns `None`.
#[requires(true)]
#[ensures(true)]
fn edge_of(coord: &str) -> Option<(String, String)> {
    let path = coord.split_once(':').map_or(coord, |(_, path)| path);
    let segments: Vec<&str> = path.split('.').collect();
    let (leaf, parent) = match segments.as_slice() {
        [] => return None,
        [only] => (*only, "OBJECT"),
        [.., parent, leaf] => (*leaf, *parent),
    };
    if leaf == "x*" {
        return None;
    }
    Some((parent.to_owned(), leaf.to_owned()))
}

/// Union of the corpus coordinate sets and value maps across all 37 documents.
#[requires(true)]
#[ensures(true)]
fn corpus_surface() -> (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>) {
    let mut coords = BTreeSet::new();
    let mut values: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for doc in CORPUS_DOCS {
        let (doc_coords, doc_values) = walk_document(&graph_for(doc));
        coords.extend(doc_coords);
        for (key, set) in doc_values {
            values.entry(key).or_default().extend(set);
        }
    }
    (coords, values)
}

/// Every serde field this build emits over the corpus is inventoried, and no
/// witnessed field entry points at a coordinate the build no longer emits.
#[test]
#[requires(true)]
#[ensures(true)]
fn drift_guard() {
    let (corpus_coords, _values) = corpus_surface();
    let corpus_edges: BTreeSet<(String, String)> =
        corpus_coords.iter().filter_map(|coord| edge_of(coord)).collect();

    let inventory = render_field_inventory();
    let inventory_edges: BTreeSet<(String, String)> = inventory
        .entries()
        .iter()
        .filter(|entry| entry.kind == EntryKind::Field)
        .filter(|entry| {
            matches!(entry.surface.category, SurfaceCategory::Object | SurfaceCategory::ValueStruct)
        })
        .filter_map(|entry| entry.witness.corpus().and_then(|(_, path, _)| edge_of(path)))
        .collect();

    let emitted_but_uninventoried: Vec<&(String, String)> =
        corpus_edges.difference(&inventory_edges).collect();
    let inventoried_but_stale: Vec<&(String, String)> =
        inventory_edges.difference(&corpus_edges).collect();

    assert!(
        emitted_but_uninventoried.is_empty(),
        "this build emits {} serde edge(s) no inventory field entry covers: {:?}",
        emitted_but_uninventoried.len(),
        &emitted_but_uninventoried[..emitted_but_uninventoried.len().min(20)]
    );
    assert!(
        inventoried_but_stale.is_empty(),
        "{} witnessed field entr(ies) name a serde edge this build no longer emits: {:?}",
        inventoried_but_stale.len(),
        &inventoried_but_stale[..inventoried_but_stale.len().min(20)]
    );
}

/// Every corpus witness pointer resolves in its named document.
#[test]
#[requires(true)]
#[ensures(true)]
fn witness_verification() {
    let inventory = render_field_inventory();
    // Cache each witness document's surface once.
    let mut cache: BTreeMap<&str, (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>)> =
        BTreeMap::new();
    let mut failures: Vec<String> = Vec::new();

    for entry in inventory.entries() {
        let Some((doc, path, expect)) = entry.witness.corpus() else {
            continue;
        };
        // Document-level facts point at the graph envelope, not an object path.
        if entry.surface.category == SurfaceCategory::Document {
            continue;
        }
        let surface = cache
            .entry(doc)
            .or_insert_with(|| walk_document(&graph_for(doc)));
        let (coords, values) = surface;
        let present = match expect.value() {
            None => coords.contains(path),
            Some(expected) => values.get(path).is_some_and(|set| set.contains(expected)),
        };
        if !present {
            failures.push(format!(
                "{}::{} -> {path} in {doc} (expect value {:?})",
                entry.surface.name,
                entry.field,
                expect.value()
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} witness pointer(s) did not resolve:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Report (never fail on) divergences between this build's graph and the frozen
/// corpus JSON: object-count-by-type, root, and coordinate-set differences.
#[test]
#[requires(true)]
#[ensures(true)]
fn frozen_divergence_report() {
    let mut diverging = 0usize;
    for doc in CORPUS_DOCS {
        let built = graph_for(doc);
        let (built_coords, _) = walk_document(&built);

        let frozen_text = std::fs::read_to_string(fixture(doc, "frozen.json"))
            .unwrap_or_else(|error| panic!("read {doc}.frozen.json: {error}"));
        let frozen: serde_json::Value =
            serde_json::from_str(&frozen_text).expect("parse frozen json");
        let frozen_coords = frozen_coordinate_set(&frozen);

        let only_built: Vec<&String> = built_coords.difference(&frozen_coords).collect();
        let only_frozen: Vec<&String> = frozen_coords.difference(&built_coords).collect();
        if !only_built.is_empty() || !only_frozen.is_empty() {
            diverging += 1;
            eprintln!("[{doc}] coordinate divergence:");
            if !only_built.is_empty() {
                eprintln!("    only in this build: {only_built:?}");
            }
            if !only_frozen.is_empty() {
                eprintln!("    only in frozen: {only_frozen:?}");
            }
        }
    }
    // Reported, not asserted: divergences are a report item per the task.
    eprintln!("frozen-divergence report: {diverging} of {} documents diverge", CORPUS_DOCS.len());
}

/// The coordinate set of a frozen graph, walked exactly like this build's.
#[requires(true)]
#[ensures(true)]
fn frozen_coordinate_set(frozen: &serde_json::Value) -> BTreeSet<String> {
    let mut coords = BTreeSet::new();
    let mut values: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    if let Some(objects) = frozen.get("objects").and_then(serde_json::Value::as_object) {
        for (id, object) in objects {
            let json_type = object.get("type").and_then(serde_json::Value::as_str).unwrap_or("");
            let node = node_type(id, json_type);
            walk_value(&node, "", object, &mut coords, &mut values);
        }
    }
    coords
}
