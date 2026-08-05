//! Corpus-driven and source-driven verification of the completeness inventory.
//!
//! Phase-B step 2 (research repo `DESIGN-RECORD.md`). Four checks:
//!
//! * `model_surface_lower_bound` — *corpus-independent*: scans the model source
//!   for every serialized struct/enum and its members and fails if any is
//!   missing from the inventory. This is what makes the inventory provably
//!   complete against the model rather than only against what the corpus
//!   happens to exercise.
//! * `witness_verification` — resolves each `Witness::Corpus` pointer through
//!   the model type graph, so a witness cannot pass on a coincidental equal
//!   string carried by an *unrelated* field: the path must land on the claimed
//!   surface/field.
//! * `drift_guard` — every serde edge this build emits over the corpus is
//!   covered by an inventoried field entry (and no witnessed entry is stale).
//! * `frozen_divergence_report` — structural + scalar comparison of this build's
//!   graph against the frozen JSON, pinned to a baseline so regressions surface.
//!
//! Graphs are re-derived by this build from the vendored `<doc>.lojban`.

#[allow(unused_imports)]
use bityzba::{data, ensures, requires};

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use jbotci_dialect::DialectDefinition;
use jbotci_morphology::{
    MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_semantics::completeness::corpus::CORPUS_DOCS;
use jbotci_semantics::completeness::model::{DispositionData, EntryKind, SurfaceCategory};
use jbotci_semantics::completeness::{
    baseline_contract_for, baseline_disposition, render_field_inventory, source_link_surfaces,
};
use jbotci_semantics::{
    SemanticBuildOptions, SemanticGraph, build_generated_semantic_graph_with_dictionary_and_options,
};
use jbotci_source::SourceId;
use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};

#[path = "support/schema_scan.rs"]
mod schema_scan;
#[path = "support/type_graph.rs"]
mod type_graph;

// ---------------------------------------------------------------------------
// Pipeline: re-derive a graph and walk its serialized coordinates.
// ---------------------------------------------------------------------------

#[requires(!doc.is_empty() && !suffix.is_empty())]
#[ensures(true)]
fn fixture(doc: &str, suffix: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/phaseb_corpus");
    path.push(format!("{doc}.{suffix}"));
    path
}

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
    let words = segment_words_with_modifiers_with_options_and_source_id(
        text,
        &morphology_options,
        source_id,
    )
    .unwrap_or_else(|error| panic!("morphology {doc}: {error}"));
    let parsed =
        parse_syntax_tree_generated_model_with_source_and_options(&words, text, &syntax_options)
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

/// The Rust node type a serialized object belongs to. The JSON `type` tag
/// collapses Referent/Eventuality/Sign onto `"referent"`; the ID sort prefix
/// disambiguates, mirroring construction.
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

#[requires(true)]
#[ensures(!ret.is_empty() || key.is_empty())]
fn canonical_key(key: &str) -> String {
    let is_place = key.len() > 1
        && key.starts_with('x')
        && key[1..].chars().all(|character| character.is_ascii_digit());
    if is_place {
        "x*".to_owned()
    } else {
        key.to_owned()
    }
}

/// A serialized graph as a `serde_json::Value`.
#[requires(!doc.is_empty())]
#[ensures(true)]
fn graph_json(doc: &str) -> serde_json::Value {
    let json = graph_for(doc).to_json_string(0).expect("serialize graph");
    serde_json::from_str(&json).expect("parse graph json")
}

/// The `"<NodeType>:<path>"` coordinate set and the scalar values seen at each.
#[requires(true)]
#[ensures(true)]
fn walk_document(
    value: &serde_json::Value,
) -> (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>) {
    let objects = value
        .get("objects")
        .and_then(serde_json::Value::as_object)
        .expect("graph objects");
    let mut coords = BTreeSet::new();
    let mut values: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (id, object) in objects {
        let json_type = object
            .get("type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let node = node_type(id, json_type);
        walk_value(&node, "", object, &mut coords, &mut values);
    }
    (coords, values)
}

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
                if child.is_null() {
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

#[requires(true)]
#[ensures(true)]
fn corpus_surface() -> (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>) {
    let mut coords = BTreeSet::new();
    let mut values: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for doc in CORPUS_DOCS {
        let (doc_coords, doc_values) = walk_document(&graph_json(doc));
        coords.extend(doc_coords);
        for (key, set) in doc_values {
            values.entry(key).or_default().extend(set);
        }
    }
    (coords, values)
}

// ---------------------------------------------------------------------------
// drift_guard
// ---------------------------------------------------------------------------

#[test]
#[requires(true)]
#[ensures(true)]
fn drift_guard() {
    let (corpus_coords, _values) = corpus_surface();
    let graph = type_graph::TypeGraph::from_source();

    // Every emitted coordinate resolves to a `(type, field)` in the type graph,
    // and that pair is inventoried as a field entry. Both are required: an
    // unresolvable coordinate means the type graph (hence the resolver) is
    // incomplete; an un-inventoried pair means the model gained a serde field
    // the inventory does not list.
    let inventory_pairs: BTreeSet<(String, String)> = render_field_inventory()
        .entries()
        .iter()
        .filter(|entry| {
            matches!(
                entry.kind,
                EntryKind::Discriminator | EntryKind::Field | EntryKind::VariantField
            )
        })
        .filter(|entry| {
            matches!(
                entry.surface.category,
                SurfaceCategory::Object | SurfaceCategory::ValueStruct
            )
        })
        .map(|entry| (entry.surface.name.to_owned(), entry.field.to_owned()))
        .collect();

    let mut unresolved: Vec<String> = Vec::new();
    let mut uninventoried: Vec<(String, String)> = Vec::new();
    for coord in &corpus_coords {
        match graph.resolve(coord) {
            None => unresolved.push(coord.clone()),
            Some(resolved) => {
                let pair = (resolved.owner, resolved.key);
                if !inventory_pairs.contains(&pair) {
                    uninventoried.push(pair);
                }
            }
        }
    }
    unresolved.sort();
    unresolved.dedup();
    uninventoried.sort();
    uninventoried.dedup();

    assert!(
        unresolved.is_empty(),
        "{} emitted coordinate(s) the type graph cannot resolve (resolver incomplete): {:?}",
        unresolved.len(),
        &unresolved[..unresolved.len().min(20)]
    );
    assert!(
        uninventoried.is_empty(),
        "this build emits {} serde field(s) no inventory entry covers: {:?}",
        uninventoried.len(),
        &uninventoried[..uninventoried.len().min(20)]
    );
}

// ---------------------------------------------------------------------------
// witness_verification (type-directed; ties path -> claimed surface)
// ---------------------------------------------------------------------------

#[test]
#[requires(true)]
#[ensures(true)]
fn witness_verification() {
    let inventory = render_field_inventory();
    let graph = type_graph::TypeGraph::from_source();
    let mut cache: BTreeMap<&str, (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>)> =
        BTreeMap::new();
    let mut failures: Vec<String> = Vec::new();

    for entry in inventory.entries() {
        let Some((doc, path, expect)) = entry.witness.corpus() else {
            continue;
        };
        if entry.surface.category == SurfaceCategory::Document {
            continue; // envelope facts point at the graph root, not an object path
        }
        // (1) The path must resolve, through the type graph, to the claimed
        //     surface/field — an equal string at an unrelated field cannot
        //     "witness" this entry. Derived facts are skipped: their `field` is a
        //     synthetic fact name, not a JSON key, so only presence is checked.
        if entry.kind != EntryKind::DerivedFact
            && !graph.path_matches_entry(
                path,
                entry.surface.category,
                entry.surface.name,
                entry.field,
            )
        {
            failures.push(format!(
                "{:?} {}::{} — witness path {path} does not resolve to this surface/field",
                entry.surface.category, entry.surface.name, entry.field
            ));
            continue;
        }
        // (1b) A variant witness must assert the variant's own discriminant value
        //      (not just that the field is of the enum type) — otherwise a variant
        //      could borrow a sibling's occurrence at the same field.
        if entry.kind == EntryKind::EnumVariant && expect.value() != Some(entry.field) {
            failures.push(format!(
                "Enum {}::{} — variant witness must expect discriminant {:?}, got {:?}",
                entry.surface.name,
                entry.field,
                entry.field,
                expect.value()
            ));
            continue;
        }
        // (2) The coordinate is actually populated (or carries the value).
        let surface = cache
            .entry(doc)
            .or_insert_with(|| walk_document(&graph_json(doc)));
        let (coords, values) = surface;
        let present = match expect.value() {
            None => coords.contains(path),
            Some(expected) => values.get(path).is_some_and(|set| set.contains(expected)),
        };
        if !present {
            failures.push(format!(
                "{:?} {}::{} — {path} not populated in {doc} (expect value {:?})",
                entry.surface.category,
                entry.surface.name,
                entry.field,
                expect.value()
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} witness pointer(s) did not resolve soundly:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// model_surface_lower_bound (corpus-independent)
// ---------------------------------------------------------------------------

#[test]
#[requires(true)]
#[ensures(true)]
fn model_surface_lower_bound() {
    let scan = schema_scan::SerializedSurface::from_source();
    let inventory = render_field_inventory();

    let mut inv_members: BTreeSet<(String, String)> = BTreeSet::new();
    let mut inv_surfaces: BTreeSet<String> = BTreeSet::new();
    for entry in inventory.entries() {
        inv_surfaces.insert(entry.surface.name.to_owned());
        inv_members.insert((entry.surface.name.to_owned(), entry.field.to_owned()));
    }

    let mut missing: Vec<String> = Vec::new();

    for (name, fields) in &scan.value_structs {
        if !inv_surfaces.contains(name) {
            missing.push(format!("missing ValueStruct surface: {name}"));
            continue;
        }
        for field in fields {
            if !inv_members.contains(&(name.clone(), field.clone())) {
                missing.push(format!("missing field {name}::{field}"));
            }
        }
    }
    for (name, variants) in &scan.enums {
        if !inv_surfaces.contains(name) {
            missing.push(format!("missing Enum surface: {name}"));
            continue;
        }
        for variant in variants {
            if !inv_members.contains(&(name.clone(), variant.clone())) {
                missing.push(format!("missing variant {name}::{variant}"));
            }
        }
    }
    for (node, keys) in &scan.node_keys {
        for key in keys {
            if !inv_members.contains(&(node.clone(), key.clone())) {
                missing.push(format!("missing object field {node}::{key}"));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "model surface not fully inventoried ({} gap(s)):\n{}",
        missing.len(),
        missing.join("\n")
    );
}

// ---------------------------------------------------------------------------
// node_type disambiguation pin (should-fix 10)
// ---------------------------------------------------------------------------

#[test]
#[requires(true)]
#[ensures(true)]
fn node_type_scheme_is_exercised() {
    let (coords, _) = corpus_surface();
    assert!(
        coords.iter().any(|coord| coord.starts_with("Eventuality:")),
        "no Eventuality: coordinate — the referent/eventuality/sign ID-prefix scheme is unexercised"
    );
    assert!(
        coords.iter().any(|coord| coord.starts_with("Sign:")),
        "no Sign: coordinate — the referent/eventuality/sign ID-prefix scheme is unexercised"
    );
    assert!(coords.iter().any(|coord| coord.starts_with("Referent:")));
}

// ---------------------------------------------------------------------------
// frozen_divergence_report (structural + scalar, pinned to a baseline)
// ---------------------------------------------------------------------------

/// Corpus documents currently diverging from the frozen oracle graph. Pinned so
/// a regression fails rather than scrolling past in the report.
const FROZEN_DIVERGENCE_BASELINE: usize = 0;

/// A JSON scalar looks like a semantic-object ID (`entity:1`, `eventuality/locution:25`).
#[requires(true)]
#[ensures(true)]
fn is_object_id(text: &str) -> bool {
    text.split_once(':').is_some_and(|(prefix, index)| {
        !prefix.is_empty()
            && prefix.chars().all(|c| c.is_ascii_lowercase() || c == '/')
            && !index.is_empty()
            && index.chars().all(|c| c.is_ascii_digit())
    })
}

/// True when a coordinate lands on a source-provenance value (`SemanticSource`
/// or `SourceByteSpan`), by type — not by a `:source` substring, which would
/// wrongly catch semantic fields like `Connector.source` (a String) or
/// `RelationMetadata:sourceWords`.
#[requires(true)]
#[ensures(true)]
fn is_provenance_coord(graph: &type_graph::TypeGraph, coord: &str) -> bool {
    graph.resolve(coord).is_some_and(|resolved| {
        matches!(resolved.owner.as_str(), "SemanticSource" | "SourceByteSpan")
    })
}

#[test]
#[requires(true)]
#[ensures(true)]
fn frozen_divergence_report() {
    let graph = type_graph::TypeGraph::from_source();
    let mut diverging = 0usize;
    for doc in CORPUS_DOCS {
        let built = graph_json(doc);
        let frozen_text = std::fs::read_to_string(fixture(doc, "frozen.json"))
            .unwrap_or_else(|error| panic!("read {doc}.frozen.json: {error}"));
        let frozen: serde_json::Value =
            serde_json::from_str(&frozen_text).expect("parse frozen json");

        let (built_coords, built_values) = walk_document(&built);
        let (frozen_coords, frozen_values) = walk_document(&frozen);

        let only_built: Vec<&String> = built_coords.difference(&frozen_coords).collect();
        let only_frozen: Vec<&String> = frozen_coords.difference(&built_coords).collect();

        // Scalar divergence, ID-agnostic and source-excluded (IDs renumber between
        // builds; source text is input-derived and equal by construction).
        let mut value_diffs: Vec<String> = Vec::new();
        for coord in built_coords.intersection(&frozen_coords) {
            // Exclude source provenance by *type* (span bytes / source text are
            // input-derived and equal by construction), not by substring.
            if is_provenance_coord(&graph, coord) {
                continue;
            }
            let normalize = |set: Option<&BTreeSet<String>>| -> BTreeSet<String> {
                set.map(|s| {
                    s.iter()
                        .map(|v| {
                            if is_object_id(v) {
                                "<ID>".to_owned()
                            } else {
                                v.clone()
                            }
                        })
                        .collect()
                })
                .unwrap_or_default()
            };
            let built_set = normalize(built_values.get(coord));
            let frozen_set = normalize(frozen_values.get(coord));
            if built_set != frozen_set {
                value_diffs.push(format!(
                    "    {coord}: this={built_set:?} frozen={frozen_set:?}"
                ));
            }
        }

        if !only_built.is_empty() || !only_frozen.is_empty() || !value_diffs.is_empty() {
            diverging += 1;
            eprintln!("[{doc}] divergence:");
            if !only_built.is_empty() {
                eprintln!("    only in this build: {only_built:?}");
            }
            if !only_frozen.is_empty() {
                eprintln!("    only in frozen: {only_frozen:?}");
            }
            for line in &value_diffs {
                eprintln!("{line}");
            }
        }
    }
    eprintln!(
        "frozen-divergence report: {diverging} of {} documents diverge",
        CORPUS_DOCS.len()
    );
    assert_eq!(
        diverging, FROZEN_DIVERGENCE_BASELINE,
        "frozen divergence changed from the pinned baseline; if this build's graph intentionally \
         changed, update FROZEN_DIVERGENCE_BASELINE with justification"
    );
}

// ---------------------------------------------------------------------------
// provenance_is_type_based (SF4) — the excluded set is the SemanticSource-typed
// source fields, derived from the type graph, not a hand list that can drift.
// ---------------------------------------------------------------------------

#[test]
#[requires(true)]
#[ensures(true)]
fn provenance_is_type_based() {
    let graph = type_graph::TypeGraph::from_source();
    let inventory = render_field_inventory();
    let links = source_link_surfaces();
    let mut mismatches: Vec<String> = Vec::new();
    let mut saw_source = false;
    for entry in inventory.entries() {
        if entry.field != "source" {
            continue;
        }
        saw_source = true;
        let policy_says_provenance = links.contains(&entry.surface.name);
        let type_says_provenance =
            graph.field_type(entry.surface.name, "source") == Some("SemanticSource");
        if policy_says_provenance != type_says_provenance {
            mismatches.push(format!(
                "{}::source — policy provenance={policy_says_provenance} but type graph says {:?}",
                entry.surface.name,
                graph.field_type(entry.surface.name, "source")
            ));
        }
    }
    assert!(saw_source, "no source fields inventoried?");
    assert!(
        mismatches.is_empty(),
        "provenance policy disagrees with the type graph:\n{}",
        mismatches.join("\n")
    );
    // The load-bearing distinction: Connector.source is a typed ConnectorSource
    // (surface word vs implicit juxtaposition), not SemanticSource provenance —
    // so it is NOT in the source-link set.
    assert_eq!(
        graph.field_type("Connector", "source"),
        Some("ConnectorSource")
    );
    assert!(!links.contains(&"Connector"));
}

// ---------------------------------------------------------------------------
// contract_disagreements_are_flagged (SF3) — registered-vs-baseline mismatch.
// ---------------------------------------------------------------------------

#[test]
#[requires(true)]
#[ensures(true)]
fn contract_disagreements_are_flagged() {
    use jbotci_semantics::completeness::{CompletenessContract, Disposition};

    let inventory = render_field_inventory();
    let baseline = baseline_contract_for(&inventory);

    // A renderer that agrees everywhere has no disagreements.
    let faithful = baseline_contract_for(&inventory);
    assert!(faithful.disagreements(&baseline).is_empty());

    // A renderer that flips one entry's disposition is caught.
    let target = inventory
        .entries()
        .iter()
        .find(|entry| {
            matches!(
                baseline_disposition(entry).as_data(),
                data!(Disposition::DirectLowering { .. })
            )
        })
        .expect("inventory has a rendered entry")
        .key();
    let mut deviating = CompletenessContract::new();
    for entry in inventory.entries() {
        let disposition = if entry.key() == target {
            Disposition::typed_fallback(
                "test-only disagreement",
                "Content",
                "SemanticObject",
                "smusni.fallback.test",
            )
        } else {
            baseline_disposition(entry)
        };
        deviating.register(entry.key(), disposition);
    }
    let disagreements = deviating.disagreements(&baseline);
    assert_eq!(disagreements, vec![target]);
}
