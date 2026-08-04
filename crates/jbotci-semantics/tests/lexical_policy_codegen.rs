//! Rejection and determinism tests for the offline #746 generator.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use sha2::{Digest, Sha256};

#[path = "../codegen/lexical_scope_policy.rs"]
mod codegen;

use codegen::{CodegenErrorKind, GeneratorInputs, GeneratorPaths};

const SOURCE: &[u8] = include_bytes!("../data/sources/lojban-org/oblique_keywords.txt");
const SOURCE_METADATA: &str =
    include_str!("../data/sources/lojban-org/oblique_keywords.metadata.toml");
const POLICIES: &str = include_str!("../data/lexical-scope-policies.toml");
const WITNESSES: &[u8] = include_bytes!("../data/smusni-draft9-must-compact.txt");
const DICTIONARY: &[u8] = include_bytes!("../../jbotci-dictionary-data/data/dictionary-en.json");
const DICTIONARY_METADATA: &str =
    include_str!("../../jbotci-dictionary-data/data/dictionary-en.metadata.toml");

#[invariant(true)]
#[derive(Debug, Clone)]
struct OwnedInputs {
    source: Vec<u8>,
    source_metadata: String,
    policies: String,
    witnesses: Vec<u8>,
    dictionary: Vec<u8>,
    dictionary_metadata: String,
}

impl OwnedInputs {
    #[requires(true)]
    #[ensures(ret.source.len() == SOURCE.len() && ret.witnesses.len() == WITNESSES.len())]
    fn reviewed() -> Self {
        Self {
            source: SOURCE.to_vec(),
            source_metadata: SOURCE_METADATA.to_owned(),
            policies: POLICIES.to_owned(),
            witnesses: WITNESSES.to_vec(),
            dictionary: DICTIONARY.to_vec(),
            dictionary_metadata: DICTIONARY_METADATA.to_owned(),
        }
    }

    #[requires(true)]
    #[ensures(ret.source.len() == self.source.len())]
    fn view(&self) -> GeneratorInputs<'_> {
        GeneratorInputs {
            source: &self.source,
            source_metadata: &self.source_metadata,
            policies: &self.policies,
            witnesses: &self.witnesses,
            dictionary: &self.dictionary,
            dictionary_metadata: &self.dictionary_metadata,
        }
    }
}

#[requires(haystack.windows(needle.len()).any(|window| window == needle))]
#[ensures(ret < haystack.len())]
fn find_bytes(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("precondition requires needle")
}

#[requires(text.lines().any(|line| line.starts_with(prefix)))]
#[ensures(ret.lines().any(|line| line == replacement))]
fn replace_first_line(text: &str, prefix: &str, replacement: &str) -> String {
    let mut replaced = false;
    text.lines()
        .map(|line| {
            if !replaced && line.starts_with(prefix) {
                replaced = true;
                replacement
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[requires(text.lines().filter(|line| line.starts_with(prefix)).count() > selected)]
#[ensures(ret.lines().filter(|line| *line == replacement).count() >= 1)]
fn replace_selected_line(text: &str, prefix: &str, selected: usize, replacement: &str) -> String {
    let mut matching_index = 0usize;
    text.lines()
        .map(|line| {
            if line.starts_with(prefix) {
                let replace = matching_index == selected;
                matching_index += 1;
                if replace {
                    return replacement;
                }
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[requires(true)]
#[ensures(true)]
fn assert_rejected(inputs: &OwnedInputs, expected: CodegenErrorKind) {
    let error = codegen::generate(inputs.view()).expect_err("mutation must be rejected");
    assert_eq!(error.kind, expected, "{}", error.message);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vendored_source_and_witness_registry_have_exact_reviewed_bytes() {
    assert_eq!(SOURCE.len(), 79_293);
    assert_eq!(SOURCE.iter().filter(|byte| **byte == b'\n').count(), 3_542);
    assert!(SOURCE.ends_with(b"\r\n"));
    assert!(
        SOURCE
            .iter()
            .enumerate()
            .all(|(index, byte)| *byte != b'\n' || index > 0 && SOURCE[index - 1] == b'\r')
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(SOURCE)),
        "355786cfd049063c92514fac2d417fc4966df7749dc17d7cfb49bd903fb6a2cb"
    );
    assert_eq!(WITNESSES.iter().filter(|byte| **byte == b'\n').count(), 18);
    assert_eq!(
        format!("{:x}", Sha256::digest(WITNESSES)),
        "850e960656599617673acbbbad5fe0384c681f8ab4587aa234b6c82e9478106b"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn generation_is_deterministic_from_the_reviewed_offline_inputs() {
    let inputs = OwnedInputs::reviewed();
    let first = codegen::generate(inputs.view()).expect("reviewed inputs generate");
    let second = codegen::generate(inputs.view()).expect("same inputs regenerate");
    assert_eq!(first, second);
    assert!(!first.is_empty());

    let mut prose_only = inputs.clone();
    prose_only.policies = replace_first_line(
        &prose_only.policies,
        "rationale = ",
        "rationale = \"Nonempty review prose is not executable policy metadata.\"",
    );
    assert_eq!(
        codegen::generate(prose_only.view()).expect("nonempty review prose remains valid"),
        first,
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn every_row_accepts_each_other_closed_policy_and_emits_only_that_change() {
    let production = [
        "Extensional",
        "Intensional",
        "Extensional",
        "Extensional",
        "Extensional",
        "Extensional",
        "Extensional",
    ];
    let alternatives = ["Extensional", "Intensional", "Opaque"];
    for (selected, original) in production.into_iter().enumerate() {
        for alternative in alternatives
            .into_iter()
            .filter(|alternative| *alternative != original)
        {
            let mut inputs = OwnedInputs::reviewed();
            inputs.policies = replace_selected_line(
                &inputs.policies,
                "policy = ",
                selected,
                &format!("policy = \"{alternative}\""),
            );
            let generated = String::from_utf8(
                codegen::generate(inputs.view()).expect("closed alternative policy generates"),
            )
            .expect("generated Rust is UTF-8");
            let mut expected = [6usize, 1, 0];
            let original_index = alternatives
                .iter()
                .position(|policy| policy == &original)
                .expect("production policy is closed");
            let alternative_index = alternatives
                .iter()
                .position(|policy| policy == &alternative)
                .expect("alternative policy is closed");
            expected[original_index] -= 1;
            expected[alternative_index] += 1;
            for (policy, expected_count) in alternatives.iter().zip(expected) {
                assert_eq!(
                    generated
                        .matches(&format!("policy: ScopePolicy::{policy}"))
                        .count(),
                    expected_count,
                );
            }
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn rejects_source_envelope_and_whole_file_drift_independently() {
    let mut metadata = OwnedInputs::reviewed();
    metadata.source_metadata =
        metadata
            .source_metadata
            .replacen("https://www.lojban.org/", "https://example.invalid/", 1);
    assert_rejected(&metadata, CodegenErrorKind::SourceMetadata);

    let mut structure = OwnedInputs::reviewed();
    let key = find_bytes(&structure.source, b"bacru1;");
    structure.source[key] = b'z';
    assert_rejected(&structure, CodegenErrorKind::WholeSourceAudit);

    let mut duplicate = OwnedInputs::reviewed();
    let second = find_bytes(&duplicate.source, b"bacru2;");
    duplicate.source[second + "bacru".len()] = b'1';
    assert_rejected(&duplicate, CodegenErrorKind::Duplicate);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn rejects_io_parse_witness_and_dictionary_metadata_failures() {
    let missing = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data/does-not-exist-lexical-policy-746");
    let paths = GeneratorPaths {
        source: missing.clone(),
        source_metadata: missing.clone(),
        policies: missing.clone(),
        witnesses: missing.clone(),
        dictionary: missing.clone(),
        dictionary_metadata: missing.clone(),
        output: missing,
    };
    let error = codegen::generate_from_paths(&paths).expect_err("missing source must fail I/O");
    assert_eq!(error.kind, CodegenErrorKind::Io);

    let mut parse = OwnedInputs::reviewed();
    parse.policies = "[[relation_place]\n".to_owned();
    assert_rejected(&parse, CodegenErrorKind::Parse);

    let mut witnesses = OwnedInputs::reviewed();
    witnesses.witnesses[0] = b'd';
    assert_rejected(&witnesses, CodegenErrorKind::Coverage);

    let mut dictionary_metadata = OwnedInputs::reviewed();
    dictionary_metadata.dictionary_metadata = dictionary_metadata.dictionary_metadata.replacen(
        "ba268ad701f8f44656ea4b17a1fd9539cfc1a3c523d0bdf581a44e3e93bb412f",
        "0a268ad701f8f44656ea4b17a1fd9539cfc1a3c523d0bdf581a44e3e93bb412f",
        1,
    );
    assert_rejected(&dictionary_metadata, CodegenErrorKind::LexicalIdentity);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn rejects_lexical_identity_fingerprint_and_place_range_drift() {
    let mut identity = OwnedInputs::reviewed();
    identity.policies =
        identity
            .policies
            .replacen("word_type = \"gismu\"", "word_type = \"lujvo\"", 1);
    assert_rejected(&identity, CodegenErrorKind::LexicalIdentity);

    let mut fingerprint = OwnedInputs::reviewed();
    let row = find_bytes(&fingerprint.dictionary, b"\"word\": \"blabi\"");
    let definition = row
        + find_bytes(&fingerprint.dictionary[row..], b"\"definition\": \"")
        + b"\"definition\": \"".len();
    let changed = fingerprint.dictionary[definition..]
        .iter()
        .position(|byte| byte.is_ascii_alphabetic())
        .expect("blabi definition contains text")
        + definition;
    fingerprint.dictionary[changed] = if fingerprint.dictionary[changed] == b'x' {
        b'y'
    } else {
        b'x'
    };
    assert_rejected(&fingerprint, CodegenErrorKind::Fingerprint);

    let mut range = OwnedInputs::reviewed();
    range.policies = range
        .policies
        .replacen("attested_arity = 1", "attested_arity = 2", 1);
    assert_rejected(&range, CodegenErrorKind::KeyRange);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn rejects_duplicate_closed_enum_and_evidence_failures() {
    let mut duplicate = OwnedInputs::reviewed();
    let first_child = duplicate
        .policies
        .find("[[relation_place.family_policy]]")
        .expect("first child");
    let second_outer = duplicate.policies[first_child..]
        .find("[[relation_place]]")
        .expect("second outer")
        + first_child;
    let child = duplicate.policies[first_child..second_outer].to_owned();
    duplicate.policies.insert_str(second_outer, &child);
    assert_rejected(&duplicate, CodegenErrorKind::Duplicate);

    let mut closed = OwnedInputs::reviewed();
    closed.policies =
        closed
            .policies
            .replacen("policy = \"Extensional\"", "policy = \"Unverified\"", 1);
    assert_rejected(&closed, CodegenErrorKind::ClosedEnum);

    let mut family = OwnedInputs::reviewed();
    family.policies = family.policies.replacen(
        "dynamic_family = \"ref-comp-referents-entity\"",
        "dynamic_family = \"ref-comp-referents-sign\"",
        1,
    );
    assert_rejected(&family, CodegenErrorKind::ClosedEnum);

    let mut evidence = OwnedInputs::reviewed();
    evidence.policies = replace_first_line(&evidence.policies, "rationale = ", "rationale = \"\"");
    assert_rejected(&evidence, CodegenErrorKind::Evidence);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn rejects_missing_or_orphan_coverage_and_noncanonical_order() {
    let mut coverage = OwnedInputs::reviewed();
    coverage.policies = coverage.policies.replacen("must:10", "must:18", 1);
    assert_rejected(&coverage, CodegenErrorKind::Coverage);

    let mut missing = OwnedInputs::reviewed();
    let final_outer = missing
        .policies
        .rfind("[[relation_place]]")
        .expect("final outer row");
    missing.policies.truncate(final_outer);
    assert_rejected(&missing, CodegenErrorKind::Coverage);

    let mut orphan = OwnedInputs::reviewed();
    orphan.policies = orphan
        .policies
        .replacen("relation = \"blabi\"", "relation = \"clabi\"", 1);
    assert_rejected(&orphan, CodegenErrorKind::Coverage);

    let mut order = OwnedInputs::reviewed();
    order.policies = order
        .policies
        .replacen("relation = \"blabi\"", "relation = \"zlabi\"", 1);
    assert_rejected(&order, CodegenErrorKind::NonDeterministicOrder);
}
