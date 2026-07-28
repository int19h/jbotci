//! Byte-parity of this build's `smusni` notation renderer against the frozen
//! Python oracle (Phase-B step 4; research repo `FREEZE-PHASE-B.md`).
//!
//! For each of the 48 frozen corpus documents, the vendored `<doc>.smusni.txt`
//! is the exact output of `python3 render_v5.py <doc>.frozen.json --profile
//! lean3` at oracle commit `c6004a1bc4dda0c9d27cef188e21402d64f36d30`
//! (jbotci#652). The corpus comprises the original 37 documents, the two
//! jbotci#620 witnesses, and five new
//! discriminant-verified question witnesses, plus four tagged-argument
//! witnesses. `lean3` is the research
//! repo's historical profile name for the product's `smusni` notation. This test
//! re-derives each graph
//! from `<doc>.lojban` with *this* build (the same pipeline the completeness
//! tests use — never reading `<doc>.frozen.json`), renders it with the `smusni`
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
use jbotci_morphology::{
    MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_semantics::completeness::corpus::CORPUS_DOCS;
use jbotci_semantics::{
    SemanticBuildOptions, SemanticGraph, SmusniConfig,
    build_generated_semantic_graph_with_dictionary_and_options, render_smusni,
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

/// Assert byte parity between this build's `render_smusni` and the vendored
/// oracle fixtures with `suffix`, under the given `config`.
#[requires(!suffix.is_empty())]
#[ensures(true)]
fn assert_parity(suffix: &str, config: SmusniConfig) {
    let mut mismatches: Vec<String> = Vec::new();
    for doc in CORPUS_DOCS {
        let expected = std::fs::read_to_string(fixture(doc, suffix))
            .unwrap_or_else(|error| panic!("read {doc}.{suffix}: {error}"));
        let actual = render_smusni(&graph_for(doc), config);
        assert_no_standalone_modal_word(&actual);
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
        "{}/{} corpus documents diverge from the frozen smusni oracle ({suffix}):\n{}",
        mismatches.len(),
        CORPUS_DOCS.len(),
        mismatches.join("\n")
    );
}

/// The default `smusni` profile (provenance off) byte-matches the oracle on all
/// 48 frozen corpus documents.
#[test]
#[requires(true)]
#[ensures(true)]
fn smusni_byte_parity_over_frozen_corpus() {
    assert_parity("smusni.txt", SmusniConfig { provenance: false });
}

/// The provenance opt-in (`--provenance`) byte-matches the oracle's
/// `--profile lean3 --provenance` output on all 48 frozen corpus documents.
#[test]
#[requires(true)]
#[ensures(true)]
fn smusni_provenance_byte_parity_over_frozen_corpus() {
    assert_parity("smusni-prov.txt", SmusniConfig { provenance: true });
}

/// The pinned aggregate SHA-256 of a fixture set: `sha256( for each doc in
/// lexicographically-sorted `CORPUS_DOCS`: doc-name + '\n' + file bytes )`.
#[requires(!suffix.is_empty())]
#[ensures(true)]
fn aggregate_fixture_hash(suffix: &str) -> String {
    aggregate_fixture_hash_for(CORPUS_DOCS.iter().copied(), suffix)
}

#[requires(!suffix.is_empty())]
#[ensures(true)]
fn aggregate_fixture_hash_for<'a>(docs: impl Iterator<Item = &'a str>, suffix: &str) -> String {
    let mut docs: Vec<&str> = docs.collect();
    docs.sort();
    let mut hasher = Sha256::new();
    for doc in docs {
        let bytes = std::fs::read(fixture(doc, suffix))
            .unwrap_or_else(|error| panic!("read {doc}.{suffix}: {error}"));
        hasher.update(doc.as_bytes());
        hasher.update(b"\n");
        hasher.update(&bytes);
    }
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The 44 documents that predate jbotci#652 retain their exact fixture bytes.
/// This pins the stability claim independently of the aggregate over the four
/// newly appended witnesses.
#[test]
#[requires(true)]
#[ensures(true)]
fn preexisting_fixture_bytes_are_unchanged() {
    let preexisting = CORPUS_DOCS
        .iter()
        .copied()
        .filter(|doc| !doc.starts_with("modal-"));
    assert_eq!(
        aggregate_fixture_hash_for(preexisting.clone(), "smusni.txt"),
        "0dbc1d7f96c49217382b9602c62001b5f3ae35c919b7a681e8dbe0b1e216a93e"
    );
    assert_eq!(
        aggregate_fixture_hash_for(preexisting, "smusni-prov.txt"),
        "d6fb701f5855382f3ddf7d6026951e5f4c74ae16bef6919968c515dce17a4e8a"
    );
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
        aggregate_fixture_hash("smusni.txt"),
        "455109a8f08252344f7865603e6063417d8adabb169031de87618b24b9810243",
        "smusni.txt fixture set drifted from the pinned oracle output"
    );
    assert_eq!(
        aggregate_fixture_hash("smusni-prov.txt"),
        "a53661c41f7768cf25e5e9f669d2ab42b99048d5aaddb8a4b0f08f437ef4c66c",
        "smusni-prov.txt fixture set drifted from the pinned oracle output"
    );
}

/// Blocker-3 regression (round-1 review, kimi 5): a `zoi` quotation whose text
/// carries notation metacharacters (`{ ( ; } )`) renders them safely inside a
/// quoted value and stays byte-identical to the oracle. This is a dedicated
/// hostile-witness fixture, deliberately *outside* [`CORPUS_DOCS`] so it does
/// not perturb the frozen corpus hash. (The dense-flatten path that
/// the hardening protects is exercised directly in `writer.rs`'s unit tests.)
#[test]
#[requires(true)]
#[ensures(true)]
fn smusni_hostile_witness_regression() {
    let doc = "hostile-quote";
    for (suffix, config) in [
        ("smusni.txt", SmusniConfig { provenance: false }),
        ("smusni-prov.txt", SmusniConfig { provenance: true }),
    ] {
        let expected = std::fs::read_to_string(fixture(doc, suffix))
            .unwrap_or_else(|error| panic!("read {doc}.{suffix}: {error}"));
        let actual = render_smusni(&graph_for(doc), config);
        assert_eq!(
            expected,
            actual,
            "{doc} ({suffix}) diverges from the oracle at {:?}",
            first_diff(&expected, &actual)
        );
        // The metacharacters survive inside a quoted value, un-split.
        assert!(actual.contains("QUOTED TEXT: \"zoi gy. a{b(c;d}e .gy\";"));
    }
}

/// jbotci#620 regression: a relation-question predication embedded under a
/// description with an indirect question (`lo se jalge cu mo kau` — the exact
/// live-traffic sentence that crashed the renderer). Its `mo` predication
/// carries `relationParameter` instead of a lexical `relation`, and it sits
/// beside an ordinary `jalge` predication, so this one document exercises both
/// relation shapes. Kept *outside* [`CORPUS_DOCS`] (like `hostile-quote`) so it
/// does not perturb the frozen corpus hash; `ti-mo` is the minimal in-corpus
/// witness the completeness inventory points at. Generated by the oracle at
/// commit `7e9c722`.
#[test]
#[requires(true)]
#[ensures(true)]
fn smusni_relation_question_indirect_regression() {
    let doc = "relation-question-indirect";
    for (suffix, config) in [
        ("smusni.txt", SmusniConfig { provenance: false }),
        ("smusni-prov.txt", SmusniConfig { provenance: true }),
    ] {
        let expected = std::fs::read_to_string(fixture(doc, suffix))
            .unwrap_or_else(|error| panic!("read {doc}.{suffix}: {error}"));
        let actual = render_smusni(&graph_for(doc), config);
        assert_eq!(
            expected,
            actual,
            "{doc} ({suffix}) diverges from the oracle at {:?}",
            first_diff(&expected, &actual)
        );
        // Both relation shapes render: the bound relation-question parameter as
        // a `VALUE <id>` reference, and the ordinary lexical relation beside it.
        assert!(actual.contains("RELATION: VALUE "));
        assert!(actual.contains("RELATION: jalge;"));
        assert!(actual.contains("QUESTION qu"));
        assert!(actual.contains("KIND: RELATION;"));
        assert!(!actual.contains("UNKNOWN question_"));
    }
}

/// jbotci#622 regression: the corpus witnesses collectively exercise every
/// currently builder-reachable question kind, both modes, homogeneous and typed
/// slots, and the optional focus/presupposed-answer fields. Byte parity above is
/// the oracle proof; these assertions make the intended discriminants explicit.
#[test]
#[requires(true)]
#[ensures(true)]
fn smusni_question_record_shapes_are_explicit() {
    let multiple = render_smusni(
        &graph_for("question-multiple-domains"),
        SmusniConfig { provenance: false },
    );
    for wording in [
        "KIND: MULTIPLE;",
        "KIND: TRUTH;",
        "KIND: QUANTITY;",
        "KIND: ARGUMENT;",
        "KIND: RELATION;",
        "DOMAIN: Argumentbundle;",
        "SLOTS (",
    ] {
        assert!(multiple.contains(wording), "missing `{wording}`");
    }

    let connective = render_smusni(
        &graph_for("question-connective"),
        SmusniConfig { provenance: false },
    );
    assert!(connective.contains("KIND: CONNECTIVE;"));

    let tense = render_smusni(
        &graph_for("question-tense"),
        SmusniConfig { provenance: false },
    );
    assert!(tense.contains("KIND: TENSE;"));

    let math_operator = render_smusni(
        &graph_for("question-math-operator"),
        SmusniConfig { provenance: false },
    );
    assert!(math_operator.contains("KIND: MATH OPERATOR;"));

    let indirect = render_smusni(
        &graph_for("question-indirect-presupposed"),
        SmusniConfig { provenance: false },
    );
    for wording in [
        "KIND: ARGUMENT;",
        "MODE: INDIRECT;",
        "FOCUS: ",
        "PRESUPPOSED ANSWER: ",
    ] {
        assert!(indirect.contains(wording), "missing `{wording}`");
    }

    let place = render_smusni(
        &graph_for("mi-klama-fia"),
        SmusniConfig { provenance: false },
    );
    assert!(place.contains("KIND: PLACE;"));
    assert!(place.contains("PLACE QUESTIONS ("));

    for rendered in [multiple, connective, tense, math_operator, indirect, place] {
        assert!(rendered.contains("QUESTION qu"));
        assert!(!rendered.contains("UNKNOWN question_"));
        assert!(!rendered.contains("renderer-support(\"question\")"));
    }
}

#[requires(true)]
#[ensures(true)]
fn assert_no_standalone_modal_word(rendered: &str) {
    assert!(
        !rendered
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|word| word.eq_ignore_ascii_case("modal")),
        "human-facing smusni must not use the standalone word `modal`:\n{rendered}"
    );
}

/// jbotci#652 regression: predicate and formula tags are ordinary keyword
/// entries after numbered places in the existing ARGS sequence. Surface BAI
/// and FIhO spellings stay provenance-only, and eventuality-level tags use the
/// same entry shape.
#[test]
#[requires(true)]
#[ensures(true)]
fn smusni_tagged_argument_entries_are_explicit_and_terminology_neutral() {
    let fronted = render_smusni(
        &graph_for("modal-fronted-vao"),
        SmusniConfig { provenance: false },
    );
    assert!(fronted.contains("[vanbi]: ("));
    assert!(!fronted.contains("[va'o]:"));
    assert!(!fronted.contains("INTRODUCED BY"));

    let tail = render_smusni(
        &graph_for("modal-tail-sepio"),
        SmusniConfig { provenance: false },
    );
    for wording in [
        "[pilno]: (",
        "[1]: REFERENCE DENOTATION r11;",
        "[2]: REFERENCE DENOTATION r7;",
        "[3]: REFERENCE DENOTATION r6;",
    ] {
        assert!(tail.contains(wording), "missing `{wording}`");
    }
    assert!(!tail.contains("se pi'o"));

    let fiho = render_smusni(
        &graph_for("modal-fiho-selpilno"),
        SmusniConfig { provenance: false },
    );
    assert!(fiho.contains("[f16]: REFERENCE DENOTATION r6;"));
    assert!(!fiho.contains("fi'o"));

    let eventuality = render_smusni(
        &graph_for("modal-eventuality-fragment"),
        SmusniConfig { provenance: false },
    );
    assert!(eventuality.contains("ARGS ("));
    assert!(eventuality.contains("[vanbi]: ("));

    let provenance = render_smusni(
        &graph_for("modal-fiho-selpilno"),
        SmusniConfig { provenance: true },
    );
    assert!(provenance.contains("INTRODUCED BY: fi'o;"));
    assert!(provenance.contains("CONSTRUCT: \"tagged-argument\";"));

    for rendered in [fronted, tail, fiho, eventuality, provenance] {
        assert!(!rendered.contains("MODAL ARGUMENTS"));
        assert_no_standalone_modal_word(&rendered);
    }
}
