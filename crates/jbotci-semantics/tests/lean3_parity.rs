//! Byte-parity of this build's `lean3` notation renderer against the frozen
//! Python oracle (Phase-B step 4; research repo `FREEZE-PHASE-B.md`).
//!
//! For each of the 37 frozen corpus documents, the vendored `<doc>.lean3.txt`
//! is the exact output of `python3 render_v5.py <doc>.frozen.json --profile
//! lean3` at the oracle commit `cab176bcce`. This test re-derives each graph
//! from `<doc>.lojban` with *this* build (the same pipeline the completeness
//! tests use — never reading `<doc>.frozen.json`), renders it with the `lean3`
//! profile, and asserts a byte-for-byte match.
//!
//! Because every corpus graph this build produces is byte-identical (in meaning)
//! to the frozen graph the oracle consumed (verified separately, and pinned by
//! the completeness `frozen_divergence_report`), a divergence here is a renderer
//! port defect, not semantic drift — see the PR body's divergence analysis.

#[allow(unused_imports)]
use bityzba::{ensures, requires};

use std::path::PathBuf;

use jbotci_dialect::DialectDefinition;
use jbotci_morphology::{MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id};
use jbotci_semantics::completeness::corpus::CORPUS_DOCS;
use jbotci_semantics::{
    Lean3Config, SemanticBuildOptions, SemanticGraph,
    build_generated_semantic_graph_with_dictionary_and_options, render_lean3,
};
use jbotci_source::SourceId;
use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};
use sha2::{Digest, Sha256};

#[requires(!doc.is_empty() && !suffix.is_empty())]
#[ensures(true)]
fn fixture(doc: &str, suffix: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/phaseb_corpus");
    path.push(format!("{doc}.{suffix}"));
    path
}

/// Re-derive a corpus document's graph from its `.lojban` source with this
/// build (identical pipeline to `tests/completeness.rs::graph_for`).
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

/// The first differing line (1-based) between two texts, for a legible failure.
#[requires(true)]
#[ensures(true)]
fn first_diff(expected: &str, actual: &str) -> Option<(usize, String, String)> {
    for (index, (e, a)) in expected.lines().zip(actual.lines()).enumerate() {
        if e != a {
            return Some((index + 1, e.to_string(), a.to_string()));
        }
    }
    if expected.lines().count() != actual.lines().count() {
        let line = expected.lines().count().min(actual.lines().count()) + 1;
        return Some((line, "<line-count differs>".to_string(), String::new()));
    }
    None
}

/// Assert byte parity between this build's `render_lean3` and the vendored
/// oracle fixtures with `suffix`, under the given `config`.
#[requires(!suffix.is_empty())]
#[ensures(true)]
fn assert_parity(suffix: &str, config: Lean3Config) {
    let mut mismatches: Vec<String> = Vec::new();
    for doc in CORPUS_DOCS {
        let expected = std::fs::read_to_string(fixture(doc, suffix))
            .unwrap_or_else(|error| panic!("read {doc}.{suffix}: {error}"));
        let actual = render_lean3(&graph_for(doc), config);
        if expected != actual {
            match first_diff(&expected, &actual) {
                Some((line, e, a)) => mismatches.push(format!(
                    "{doc}: first diff at line {line}\n    expected: {e}\n    actual:   {a}"
                )),
                None => mismatches.push(format!("{doc}: differs (trailing content)")),
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "{}/{} corpus documents diverge from the frozen lean3 oracle ({suffix}):\n{}",
        mismatches.len(),
        CORPUS_DOCS.len(),
        mismatches.join("\n")
    );
}

/// The default `lean3` profile (provenance off) byte-matches the oracle on all
/// 37 frozen corpus documents.
#[test]
#[requires(true)]
#[ensures(true)]
fn lean3_byte_parity_over_frozen_corpus() {
    assert_parity("lean3.txt", Lean3Config { provenance: false });
}

/// The provenance opt-in (`--provenance`) byte-matches the oracle's
/// `--profile lean3 --provenance` output on all 37 frozen corpus documents.
#[test]
#[requires(true)]
#[ensures(true)]
fn lean3_provenance_byte_parity_over_frozen_corpus() {
    assert_parity("lean3-prov.txt", Lean3Config { provenance: true });
}

/// The pinned aggregate SHA-256 of a fixture set: `sha256( for each doc in
/// lexicographically-sorted `CORPUS_DOCS`: doc-name + '\n' + file bytes )`.
#[requires(!suffix.is_empty())]
#[ensures(true)]
fn aggregate_fixture_hash(suffix: &str) -> String {
    let mut docs: Vec<&&str> = CORPUS_DOCS.iter().collect();
    docs.sort();
    let mut hasher = Sha256::new();
    for doc in docs {
        let bytes = std::fs::read(fixture(doc, suffix))
            .unwrap_or_else(|error| panic!("read {doc}.{suffix}: {error}"));
        hasher.update(doc.as_bytes());
        hasher.update(b"\n");
        hasher.update(&bytes);
    }
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Should-fix 7 (round-1 review, Codex 4): pin the aggregate hash of BOTH
/// vendored fixture sets so the renderer-expected and provenance fixtures cannot
/// silently drift together (e.g. a regeneration that changes both while a stale
/// oracle is in use). If a deliberate oracle change lands, this pin and the
/// `FREEZE-PHASE-B.md` amendment are updated in lockstep.
#[test]
#[requires(true)]
#[ensures(true)]
fn frozen_fixture_aggregate_hashes_are_pinned() {
    assert_eq!(
        aggregate_fixture_hash("lean3.txt"),
        "6ed45e77517acdcf2f819d21064ccb4b77baf4b90fb19e9dacffb6f902c35641",
        "lean3.txt fixture set drifted from the pinned oracle output"
    );
    assert_eq!(
        aggregate_fixture_hash("lean3-prov.txt"),
        "2fbeae6e02fce4b3f4d50b82f8a7fe7b0b47a99b9b117dc3f6ad1940d6478da2",
        "lean3-prov.txt fixture set drifted from the pinned oracle output"
    );
}

/// Blocker-3 regression (round-1 review, kimi 5): a `zoi` quotation whose text
/// carries notation metacharacters (`{ ( ; } )`) renders them safely inside a
/// quoted value and stays byte-identical to the oracle. This is a dedicated
/// hostile-witness fixture, deliberately *outside* [`CORPUS_DOCS`] so it does
/// not perturb the frozen 37-document corpus hash. (The dense-flatten path that
/// the hardening protects is exercised directly in `writer.rs`'s unit tests.)
#[test]
#[requires(true)]
#[ensures(true)]
fn lean3_hostile_witness_regression() {
    let doc = "hostile-quote";
    for (suffix, config) in [
        ("lean3.txt", Lean3Config { provenance: false }),
        ("lean3-prov.txt", Lean3Config { provenance: true }),
    ] {
        let expected = std::fs::read_to_string(fixture(doc, suffix))
            .unwrap_or_else(|error| panic!("read {doc}.{suffix}: {error}"));
        let actual = render_lean3(&graph_for(doc), config);
        assert_eq!(
            expected, actual,
            "{doc} ({suffix}) diverges from the oracle at {:?}",
            first_diff(&expected, &actual)
        );
        // The metacharacters survive inside a quoted value, un-split.
        assert!(actual.contains("QUOTED TEXT: \"zoi gy. a{b(c;d}e .gy\";"));
    }
}
