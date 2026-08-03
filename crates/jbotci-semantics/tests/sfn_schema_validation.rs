//! Schema regression tests for the SFN-XML v0 schema
//! (`resources/sfn-xml-v0.xsd`, jbotci#709): every document the renderer can
//! emit — compact form with and without the structured word-card WORDS
//! section — must validate, and documents that break the schema's key
//! constraints must be rejected.
//!
//! Three layers:
//!
//! * LIVE witnesses: the exact adversarial-review witnesses rendered by this
//!   build — `ti mi zei do` (DEICTIC-REFERENCE GROUND-REF in the body plus
//!   the mi-zei-do VARIABLE-CONTEXT card), `lo mlatu cu abu zei barda` (the
//!   `<LETTER TEXT="abu"/>` leaf), and `lo skamymlatu cu barda` both with
//!   and without word cards.
//! * Frozen corpus: all 48 `tests/xml_corpus/*.xml.txt` documents.
//! * Mutation rejection: a `VERSION` other than the fixed `"0"` and a
//!   dangling `COMPONENT WORD=` keyref must FAIL validation.
//!
//! Validation shells out to `xmllint --noout --schema`; when xmllint is not
//! on PATH the tests print a note and skip instead of failing (CI has it).

#[allow(unused_imports)]
use bityzba::{ensures, requires};

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use jbotci_morphology::segment_words_with_modifiers;
use jbotci_semantics::notation::word_cards::build_word_cards;
use jbotci_semantics::{
    SemanticBuildOptions, SemanticGraph,
    build_generated_semantic_graph_with_dictionary_and_options, render_xml_with_word_cards,
};
use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};

/// The semantics pipeline shared by the live witnesses (identical to the
/// `graph_for_text` helper in `notation::render`'s tests).
#[requires(!text.is_empty())]
#[ensures(true)]
fn graph_for_text(text: &str, words: &[jbotci_morphology::WordLike]) -> SemanticGraph {
    let parsed = parse_syntax_tree_generated_model_with_source_and_options(
        words,
        text,
        &ParseOptions::default(),
    )
    .unwrap_or_else(|error| panic!("syntax {text}: {error}"));
    build_generated_semantic_graph_with_dictionary_and_options(
        &parsed,
        SemanticBuildOptions {
            source_text: Some(text),
            story_time: false,
        },
        jbotci_dictionary_data::english(),
    )
    .unwrap_or_else(|error| panic!("semantics {text}: {error}"))
}

/// Render one text as canonical SFN-XML with its structured word cards.
#[requires(!text.is_empty() && !document_name.is_empty())]
#[ensures(ret.ends_with('\n'))]
fn render_with_defs(text: &str, document_name: &str) -> String {
    let words = segment_words_with_modifiers(text)
        .unwrap_or_else(|error| panic!("morphology {text}: {error}"));
    let graph = graph_for_text(text, &words);
    let cards = build_word_cards(jbotci_dictionary_data::english(), &words);
    render_xml_with_word_cards(&graph, document_name, &cards)
        .into_data()
        .output
}

/// Render one text as canonical SFN-XML without word cards.
#[requires(!text.is_empty() && !document_name.is_empty())]
#[ensures(ret.ends_with('\n'))]
fn render_without_defs(text: &str, document_name: &str) -> String {
    let words = segment_words_with_modifiers(text)
        .unwrap_or_else(|error| panic!("morphology {text}: {error}"));
    let graph = graph_for_text(text, &words);
    render_xml_with_word_cards(&graph, document_name, &[])
        .into_data()
        .output
}

#[requires(true)]
#[ensures(true)]
fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/sfn-xml-v0.xsd")
}

/// Whether xmllint is on PATH; the tests print a note and skip when it is
/// not (CI always has it).
#[requires(true)]
#[ensures(true)]
fn xmllint_available() -> bool {
    Command::new("xmllint")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// Feed one document to `xmllint --noout --schema <xsd> -` over stdin:
/// `Ok(())` when it validates, `Err(stderr)` when it does not. `None` means
/// xmllint could not be spawned (not on PATH), which the availability probe
/// rules out beforehand.
#[requires(true)]
#[ensures(true)]
fn xmllint_validate(document: &str) -> Option<Result<(), String>> {
    let mut child = Command::new("xmllint")
        .arg("--noout")
        .arg("--schema")
        .arg(schema_path())
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    // xmllint may close stdin early on a fatal error; a short write then
    // still surfaces as the non-zero exit status below.
    let _ = child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(document.as_bytes());
    let output = child.wait_with_output().expect("xmllint was spawned");
    Some(if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    })
}

/// Assert one document validates against the schema.
#[requires(!label.is_empty())]
#[ensures(true)]
fn assert_valid(label: &str, document: &str) {
    match xmllint_validate(document) {
        Some(Ok(())) => {}
        Some(Err(stderr)) => panic!("{label} must validate against the schema:\n{stderr}"),
        None => unreachable!("xmllint availability was probed by the test"),
    }
}

/// Assert one document is REJECTED by the schema.
#[requires(!label.is_empty())]
#[ensures(true)]
fn assert_invalid(label: &str, document: &str) {
    match xmllint_validate(document) {
        Some(Ok(())) => panic!("{label} must be rejected by the schema but validated"),
        Some(Err(_)) => {}
        None => unreachable!("xmllint availability was probed by the test"),
    }
}

/// The exact review witnesses (#709): deictic ground references, the
/// VARIABLE-CONTEXT word card, the LETTER leaf, and the no-cards shape all
/// produce schema-valid documents.
#[test]
#[requires(true)]
#[ensures(true)]
fn live_witnesses_validate_against_the_schema() {
    if !xmllint_available() {
        eprintln!("skipping live_witnesses_validate_against_the_schema: xmllint is not on PATH");
        return;
    }

    // DEICTIC-REFERENCE GROUND-REF in the body (ti) plus the mi-zei-do
    // VARIABLE-CONTEXT card.
    let deictic = render_with_defs("ti mi zei do", "schema-witness-deictic");
    assert!(
        deictic.contains("GROUND-REF="),
        "the witness must exercise DEICTIC-REFERENCE GROUND-REF:\n{deictic}"
    );
    assert!(
        deictic.contains("<VARIABLE-CONTEXT ROLE=\"SPEAKER\"/>")
            && deictic.contains("<VARIABLE-CONTEXT ROLE=\"AUDIENCE\"/>"),
        "the witness must exercise the mi-zei-do VARIABLE-CONTEXT card:\n{deictic}"
    );
    assert_valid("ti mi zei do (with defs)", &deictic);

    // The `<LETTER TEXT="abu"/>` leaf inside a zei-compound card.
    let letter = render_with_defs("lo mlatu cu abu zei barda", "schema-witness-letter");
    assert!(
        letter.contains("<LETTER TEXT=\"abu\"/>"),
        "the witness must exercise the LETTER leaf:\n{letter}"
    );
    assert_valid("lo mlatu cu abu zei barda (with defs)", &letter);

    // A nonce-lujvo card with a COMPONENT-referencing composition, and the
    // same document without word cards.
    let nonce = render_with_defs("lo skamymlatu cu barda", "schema-witness-nonce");
    assert!(
        nonce.contains("<COMPONENT WORD=\"mlatu\"/>"),
        "the witness must exercise the COMPONENT keyref:\n{nonce}"
    );
    assert_valid("lo skamymlatu cu barda (with defs)", &nonce);
    let plain = render_without_defs("lo skamymlatu cu barda", "schema-witness-plain");
    assert_valid("lo skamymlatu cu barda (without defs)", &plain);
}

/// Every frozen corpus document validates against the schema.
#[test]
#[requires(true)]
#[ensures(true)]
fn frozen_xml_corpus_validates_against_the_schema() {
    if !xmllint_available() {
        eprintln!(
            "skipping frozen_xml_corpus_validates_against_the_schema: xmllint is not on PATH"
        );
        return;
    }
    let corpus_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/xml_corpus");
    let mut documents: Vec<(String, String)> = Vec::new();
    for entry in std::fs::read_dir(&corpus_dir).expect("the xml_corpus directory exists") {
        let path = entry.expect("a readable directory entry").path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name.ends_with(".xml.txt") {
            let document = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {file_name}: {error}"));
            documents.push((file_name.to_owned(), document));
        }
    }
    documents.sort_by(|(left, _), (right, _)| left.cmp(right));
    assert_eq!(
        documents.len(),
        48,
        "the frozen XML corpus holds exactly 48 documents"
    );
    let mut failures = String::new();
    for (file_name, document) in &documents {
        if let Some(Err(stderr)) = xmllint_validate(document) {
            failures.push_str(&format!("--- {file_name} ---\n{stderr}\n"));
        }
    }
    assert!(
        failures.is_empty(),
        "frozen corpus documents must validate against the schema:\n{failures}"
    );
}

/// Schema mutations must be rejected: a VERSION other than the fixed "0",
/// and a COMPONENT WORD= referencing no card (dangling keyref). The
/// unmutated document is the positive control.
#[test]
#[requires(true)]
#[ensures(true)]
fn mutated_documents_are_rejected_by_the_schema() {
    if !xmllint_available() {
        eprintln!("skipping mutated_documents_are_rejected_by_the_schema: xmllint is not on PATH");
        return;
    }
    let document = render_with_defs("lo skamymlatu cu barda", "schema-mutation-control");
    // Positive control: the unmutated document validates.
    assert_valid("unmutated positive control", &document);

    assert!(
        document.contains("VERSION=\"0\""),
        "the render must carry the fixed VERSION for the mutation to be meaningful"
    );
    let wrong_version = document.replacen("VERSION=\"0\"", "VERSION=\"999\"", 1);
    assert_invalid("VERSION=\"999\" mutation", &wrong_version);

    assert!(
        document.contains("<COMPONENT WORD=\"mlatu\"/>"),
        "the render must carry the COMPONENT keyref for the mutation to be meaningful"
    );
    let dangling_component = document.replacen(
        "<COMPONENT WORD=\"mlatu\"/>",
        "<COMPONENT WORD=\"missing-card\"/>",
        1,
    );
    assert_invalid(
        "dangling COMPONENT WORD keyref mutation",
        &dangling_component,
    );
}
