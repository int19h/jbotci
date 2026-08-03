//! Regenerate the SFN golden files after an intentional output-shape change
//! (jbotci#719): the 48 xml_corpus `*.xml.txt` documents (frozen JSON plus
//! injected binder universes) and the four xml_focused_regressions `*.xml.txt` documents
//! (public typed path, DOC name swapped to the document id).
//!
//! Usage: `regen_goldens <xml-corpus|focused>`; prints one line per written
//! file. Smusni intentionally has no byte-exact golden regeneration mode.

use std::path::PathBuf;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_semantics::notation::render_xml_value_for_tooling;
use jbotci_semantics::{
    SemanticBuildOptions, SemanticGraph, build_generated_semantic_graph_with_dictionary_and_options,
    render_xml,
};
use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};
use serde_json::Value;

#[requires(!text.is_empty())]
#[ensures(true)]
fn graph_for_text(text: &str) -> SemanticGraph {
    let words = jbotci_morphology::segment_words_with_modifiers(text).expect("morphology");
    let parsed = parse_syntax_tree_generated_model_with_source_and_options(
        &words,
        text,
        &ParseOptions::default(),
    )
    .expect("syntax");
    build_generated_semantic_graph_with_dictionary_and_options(
        &parsed,
        SemanticBuildOptions {
            source_text: Some(text),
            story_time: false,
        },
        jbotci_dictionary_data::english(),
    )
    .expect("semantics")
}

#[requires(true)]
#[ensures(true)]
fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[requires(true)]
#[ensures(true)]
fn regen_xml_corpus() {
    let corpus = manifest().join("tests/xml_corpus");
    let binder_universes: Value = serde_json::from_slice(
        &std::fs::read(corpus.join("BINDER_UNIVERSES.json")).expect("binder universes read"),
    )
    .expect("binder universes parse");
    let mut documents: Vec<PathBuf> = std::fs::read_dir(&corpus)
        .expect("corpus directory")
        .filter_map(|entry| {
            let path = entry.expect("corpus entry").path();
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .filter(|name| name.ends_with(".frozen.json"))
                .map(|_| path)
        })
        .collect();
    documents.sort();
    assert_eq!(documents.len(), 48, "the xml corpus must stay at 48 documents");
    for path in documents {
        let document = path
            .file_name()
            .expect("document name")
            .to_string_lossy()
            .into_owned()
            .replace(".frozen.json", "");
        let mut graph: Value = serde_json::from_slice(
            &std::fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
        )
        .expect("frozen graph parses");
        graph["scopeDependenceBinderUniverses"] = binder_universes[document.as_str()].clone();
        let rendered = render_xml_value_for_tooling(graph, &document);
        let target = corpus.join(format!("{document}.xml.txt"));
        std::fs::write(&target, &rendered.output)
            .unwrap_or_else(|error| panic!("write {}: {error}", target.display()));
        println!("xml-corpus {document}");
    }
}

#[requires(true)]
#[ensures(true)]
fn regen_focused() {
    let focused = manifest().join("tests/xml_focused_regressions");
    let cases = [
        (
            "b59",
            "content-first-question-scope",
            "mi djuno lo ka ce'u klama makau",
        ),
        (
            "b60",
            "content-first-question-scope",
            "mi djica lo nu makau klama",
        ),
        ("b61", "referent-sort-abstraction", "mi facki lo ni ma kau clani"),
        ("b62", "sign-quotation", "mi cusku lu ro da klama li'u"),
    ];
    for (document, group, text) in cases {
        let graph = graph_for_text(text);
        // Match the byte-parity test: render with the scope-dependence-first-
        // visit label, then swap the DOC name to the document id.
        let rendered = render_xml(&graph, "<scope-dependence-first-visit>");
        let output = rendered.output.replacen(
            "DOC=\"&lt;scope-dependence-first-visit&gt;\"",
            &format!("DOC=\"{document}\""),
            1,
        );
        let target = focused.join(group).join(format!("{document}.xml.txt"));
        std::fs::write(&target, output)
            .unwrap_or_else(|error| panic!("write {}: {error}", target.display()));
        println!("focused {document}");
    }
}

#[requires(true)]
#[ensures(true)]
fn main() {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| panic!("usage: regen_goldens <xml-corpus|focused>"));
    match mode.as_str() {
        "xml-corpus" => regen_xml_corpus(),
        "focused" => regen_focused(),
        other => panic!("unknown mode: {other}"),
    }
}
