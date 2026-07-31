//! Fixture-derived regression coverage for the XML representation boundary.
//!
//! The exhaustive check deliberately attempts every repository fixture.  It is
//! not a sample: every input whose strict morphology, syntax, and semantic
//! construction succeed must render twice, account every non-waived surface,
//! and parse as XML.

use std::collections::BTreeSet;
use std::path::PathBuf;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_morphology::{
    MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_semantics::{
    SemanticBuildOptions, SemanticGraph, XML_DECLARED_WAIVERS,
    build_generated_semantic_graph_with_dictionary_and_options, render_xml,
};
use jbotci_source::SourceId;
use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};
#[cfg(feature = "expensive_contracts")]
use rayon::prelude::*;
use sha2::{Digest, Sha256};
#[cfg(feature = "expensive_contracts")]
use xtask_common::fixtures::load_fixture_tree;

#[invariant(!text.is_empty())]
#[invariant(typed_reason.as_ref().is_none_or(|reason| !reason.is_empty()))]
#[derive(Debug)]
struct ReviewerRegression {
    text: String,
    typed_reason: Option<String>,
}

#[invariant(!id.is_empty())]
#[invariant(reasons.iter().all(|reason| !reason.is_empty()))]
#[cfg(feature = "expensive_contracts")]
#[derive(Debug)]
struct FixtureRenderResult {
    id: String,
    reasons: BTreeSet<String>,
}

#[requires(true)]
#[ensures(ret.iter().all(|case| !case.text.is_empty()))]
fn reviewer_regressions() -> Vec<ReviewerRegression> {
    [
        (
            ".a'enai do ranji bacru",
            None,
        ),
        ("li xo jei do curve", None),
        (
            "doi mo do'udai",
            Some("NON-CANONICAL-GROUND"),
        ),
        (
            "ka'i da",
            Some("NON-COMPACT-REFERENT"),
        ),
        (
            "ru'i ku zo'u la ke barda bruna ke'e ku zgana do",
            Some("NON-COMPACT-NAME-DESCRIPTOR"),
        ),
        (
            "mi bai ke ge klama le zarci gi cadzu le bisli ke'e",
            None,
        ),
        (
            ".iri'abo mi milxe leka tatpi kei ca",
            None,
        ),
        (
            "le glico bangu cu cfika bangu .iki'ubo ra se pilno le lisri ciska",
            Some("NON-DERIVABLE-GENERATED-CONTENT"),
        ),
        (
            "sa pu tcidu da poi srana le terfrica be zo y'ybu bei zo xy",
            None,
        ),
        (
            "su'oda zo'u mi prami da .ije naku do prami da",
            Some("BINDER-DOES-NOT-ENCLOSE-USE"),
        ),
        (
            "ta'e na tcidu le cfika",
            None,
        ),
        (
            "ba'e mi noi cmima jy.",
            Some("NON-CANONICAL-GROUND"),
        ),
        (
            "vei ci .a vo ve'o prenu cu klama le zarci",
            Some("MULTIPLE-BINDER-OWNERS"),
        ),
        (
            "zo'epe mi pu xe .ei klama le spita fu le mi fetsi ca le cerni .ibabo mi klama .ei lo mikce .ibabo mi xe .ei klama lo bi'u mikce le ckule fu pa le mi panzi .ije xruti xe klama .ei .ibabo xe .ei klama le zdani le ckule fu le re panzi .ibabo xe .ei klama lo drata mikce le zdani fu pa le panzi .ibabo te gusta le vancysanmi .ibabo co'e li'o .i a'anai .oi",
            Some("NON-DERIVABLE-GENERATED-CONTENT"),
        ),
    ]
    .into_iter()
    .map(|(text, typed_reason)| {
        new!(ReviewerRegression {
            text: text.to_owned(),
            typed_reason: typed_reason.map(str::to_owned),
        })
    })
    .collect()
}

#[requires(!source.is_empty())]
#[ensures(ret.is_err() || ret.as_ref().is_ok_and(|graph| graph.objects.contains_key(&graph.root)))]
fn graph_for_source(source: &str) -> Result<SemanticGraph, String> {
    let dialect = jbotci_dialect::DialectDefinition::default();
    graph_for_source_and_dialect(source, &dialect, "<xml-regression>")
}

#[requires(!source_label.is_empty())]
#[ensures(ret.is_err() || ret.as_ref().is_ok_and(|graph| graph.objects.contains_key(&graph.root)))]
fn graph_for_source_and_dialect(
    source: &str,
    dialect: &jbotci_dialect::DialectDefinition,
    source_label: &str,
) -> Result<SemanticGraph, String> {
    let morphology_options = MorphologyOptions::default().with_dialect_definition(dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(dialect);
    let words = segment_words_with_modifiers_with_options_and_source_id(
        source,
        &morphology_options,
        Some(SourceId(source_label.to_owned())),
    )
    .map_err(|error| format!("morphology: {error}"))?;
    let syntax =
        parse_syntax_tree_generated_model_with_source_and_options(&words, source, &syntax_options)
            .map_err(|error| format!("syntax: {error}"))?;
    build_generated_semantic_graph_with_dictionary_and_options(
        &syntax,
        SemanticBuildOptions {
            source_text: Some(source),
            story_time: false,
        },
        jbotci_dictionary_data::english(),
    )
    .map_err(|error| format!("semantics: {error}"))
}

#[requires(!document_name.is_empty())]
#[ensures(ret.iter().all(|kind| !kind.is_empty()))]
fn assert_render_contract(graph: &SemanticGraph, document_name: &str) -> BTreeSet<String> {
    let first = render_xml(graph, document_name);
    let second = render_xml(graph, document_name);
    assert_eq!(
        first, second,
        "{document_name}: XML render is not deterministic"
    );
    assert!(
        first.omissions.iter().all(|omission| omission
            .waiver
            .is_some_and(|family| { XML_DECLARED_WAIVERS.contains(&family) })),
        "{document_name}: XML left an unwaived semantic occurrence: {:?}",
        first.omissions,
    );
    let document = roxmltree::Document::parse(&first.output)
        .unwrap_or_else(|error| panic!("{document_name}: malformed XML: {error}"));
    let root = document.root_element();
    assert_eq!(root.tag_name().name(), "SFN", "{document_name}");
    let typed = root.attribute("FORM") == Some("TYPED-GRAPH");
    if !typed {
        let generic_nodes: Vec<String> = document
            .descendants()
            .filter(|node| {
                node.is_element()
                    && matches!(
                        node.tag_name().name(),
                        "EXTRA" | "FIELD" | "LIST" | "ITEM" | "RECORD" | "UNKNOWN"
                    )
            })
            .map(|node| match node.attribute("NAME") {
                Some(field) => format!("{}[{field}]", node.tag_name().name()),
                None => node.tag_name().name().to_owned(),
            })
            .collect();
        assert!(
            generic_nodes.is_empty(),
            "{document_name}: known compact semantics reached generic scaffolding: {generic_nodes:?}"
        );
    }
    let reasons: BTreeSet<String> = document
        .descendants()
        .filter(|node| node.has_tag_name("INCOMPATIBILITY"))
        .map(|node| {
            node.attribute("KIND")
                .unwrap_or_else(|| panic!("{document_name}: reason lacks KIND"))
                .to_owned()
        })
        .collect();
    assert_eq!(
        typed,
        !reasons.is_empty(),
        "{document_name}: form/reason mismatch"
    );
    reasons
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reviewer_failures_select_the_expected_form_and_f2_is_structured() {
    for case in reviewer_regressions() {
        let graph =
            graph_for_source(&case.text).unwrap_or_else(|error| panic!("{:?}: {error}", case.text));
        let reasons =
            assert_render_contract(&graph, &format!("<reviewer-regression:{}>", case.text));
        match &case.typed_reason {
            Some(reason) => assert!(
                reasons.contains(reason),
                "{:?}: expected {reason}, got {reasons:?}",
                case.text,
            ),
            None => assert!(
                reasons.is_empty(),
                "{:?}: compact regression unexpectedly selected typed graph: {reasons:?}",
                case.text,
            ),
        }
    }

    let graph = graph_for_source("ui dai mi klama").expect("F2 input must build");
    let rendered = render_xml(&graph, "<f2>");
    let descriptor_omissions = rendered
        .omissions
        .iter()
        .filter(|omission| {
            omission.waiver == Some(jbotci_semantics::XmlWaiverFamily::DescriptorWord)
        })
        .count();
    assert_eq!(descriptor_omissions, 1);
    assert!(
        rendered
            .output
            .contains("descriptor *.word provenance (1 field)")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn content_first_question_scope_outputs_are_byte_pinned() {
    let cases = [
        (
            "b59",
            "mi djuno lo ka ce'u klama makau",
            "333d657bb822e83c89b28d4e358a3d2fe2e629f1238cbc5e101c330dec9ea9c6",
            "content-first-question-scope/b59.frozen.json",
            include_str!(
                "../crates/jbotci-semantics/tests/xml_focused_regressions/content-first-question-scope/b59.frozen.json"
            ),
            include_str!(
                "../crates/jbotci-semantics/tests/xml_focused_regressions/content-first-question-scope/b59.xml.txt"
            ),
            1,
        ),
        (
            "b60",
            "mi djica lo nu makau klama",
            "8291b682a0673c28111138daaf85878b9c3ec5487a767b219bdf1fe75386227e",
            "content-first-question-scope/b60.frozen.json",
            include_str!(
                "../crates/jbotci-semantics/tests/xml_focused_regressions/content-first-question-scope/b60.frozen.json"
            ),
            include_str!(
                "../crates/jbotci-semantics/tests/xml_focused_regressions/content-first-question-scope/b60.xml.txt"
            ),
            1,
        ),
        (
            "b61",
            "mi facki lo ni ma kau clani",
            "23f61eacc63c40d895c9246b1097d8c21794b1f3a607c7c1e36558077fb18c58",
            "referent-sort-abstraction/b61.frozen.json",
            include_str!(
                "../crates/jbotci-semantics/tests/xml_focused_regressions/referent-sort-abstraction/b61.frozen.json"
            ),
            include_str!(
                "../crates/jbotci-semantics/tests/xml_focused_regressions/referent-sort-abstraction/b61.xml.txt"
            ),
            1,
        ),
        (
            "b62",
            "mi cusku lu ro da klama li'u",
            "ff1eec3b104dcd978d3da8b24fbea591166dc699fe0baa23f9c308c4be30ef5a",
            "sign-quotation/b62.frozen.json",
            include_str!(
                "../crates/jbotci-semantics/tests/xml_focused_regressions/sign-quotation/b62.frozen.json"
            ),
            include_str!(
                "../crates/jbotci-semantics/tests/xml_focused_regressions/sign-quotation/b62.xml.txt"
            ),
            0,
        ),
    ];

    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("crates/jbotci-semantics/tests/xml_focused_regressions");
    let expected_frozen_json: BTreeSet<PathBuf> =
        cases.iter().map(|case| fixture_root.join(case.3)).collect();
    let mut actual_frozen_json = BTreeSet::new();
    for group in std::fs::read_dir(&fixture_root)
        .unwrap_or_else(|error| panic!("read {}: {error}", fixture_root.display()))
    {
        let group = group.expect("focused fixture group entry must be readable");
        if !group
            .file_type()
            .expect("focused fixture group type must be readable")
            .is_dir()
        {
            continue;
        }
        for fixture in std::fs::read_dir(group.path())
            .unwrap_or_else(|error| panic!("read {}: {error}", group.path().display()))
        {
            let fixture = fixture.expect("focused fixture entry must be readable");
            if fixture
                .file_name()
                .to_string_lossy()
                .ends_with(".frozen.json")
            {
                actual_frozen_json.insert(fixture.path());
            }
        }
    }
    assert_eq!(
        actual_frozen_json, expected_frozen_json,
        "every focused frozen JSON fixture must be consumed by this byte-parity test",
    );

    for (
        document,
        source,
        expected_hash,
        _frozen_path,
        frozen_json,
        prototype_xml,
        expected_embedded_questions,
    ) in cases
    {
        let graph = graph_for_source(source)
            .unwrap_or_else(|error| panic!("{source:?} must build and validate: {error}"));
        let frozen_graph: serde_json::Value = serde_json::from_str(frozen_json)
            .unwrap_or_else(|error| panic!("{document} frozen JSON must parse: {error}"));
        assert_eq!(
            serde_json::to_value(&graph).expect("generated graph serializes"),
            frozen_graph,
            "{source:?} generated graph differs from pinned JSON",
        );
        let rendered = render_xml(&graph, "<scope-dependence-first-visit>");
        assert!(!rendered.output.contains("FORM=\"TYPED-GRAPH\""));
        assert_eq!(
            rendered.output.matches("<EMBEDDED-QUESTIONS>").count(),
            expected_embedded_questions,
        );
        assert!(!rendered.output.contains("SAME-FOR-ALL=\"true\""));
        assert!(!rendered.output.contains("POSSIBLY-DIFFERENT-PER=\""));
        let parsed = roxmltree::Document::parse(&rendered.output)
            .unwrap_or_else(|error| panic!("{document} XML must parse: {error}"));
        let generic_nodes: Vec<String> = parsed
            .descendants()
            .filter(|node| {
                node.is_element()
                    && matches!(
                        node.tag_name().name(),
                        "EXTRA" | "FIELD" | "LIST" | "ITEM" | "RECORD" | "UNKNOWN"
                    )
            })
            .map(|node| match node.attribute("NAME") {
                Some(field) => format!("{}[{field}]", node.tag_name().name()),
                None => node.tag_name().name().to_owned(),
            })
            .collect();
        assert!(
            generic_nodes.is_empty(),
            "{document}: focused compact output reached generic scaffolding: {generic_nodes:?}"
        );
        assert_eq!(
            rendered.output.replacen(
                "DOC=\"&lt;scope-dependence-first-visit&gt;\"",
                &format!("DOC=\"{document}\""),
                1
            ),
            prototype_xml,
            "{source:?} product/prototype XML differs beyond DOC",
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(rendered.output.as_bytes())),
            expected_hash,
            "{source:?} XML bytes changed",
        );
    }
}

#[cfg(feature = "expensive_contracts")]
#[test]
#[requires(true)]
#[ensures(true)]
fn every_semantically_valid_repository_fixture_satisfies_the_xml_contract() {
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let fixtures = load_fixture_tree(&fixture_root).expect("repository fixtures must load");
    let fixture_count = fixtures.len();
    let mut results: Vec<FixtureRenderResult> = fixtures
        .par_iter()
        .filter_map(|fixture| {
            let test_case = &fixture.test_case;
            let dialect = test_case
                .dialect_definition()
                .unwrap_or_else(|error| panic!("{} dialect: {error}", test_case.id));
            let graph = graph_for_source_and_dialect(
                &test_case.lojban,
                &dialect,
                &format!("<fixture:{}>", test_case.id),
            )
            .ok()?;
            let reasons = assert_render_contract(&graph, &test_case.id);
            Some(new!(FixtureRenderResult {
                id: test_case.id.clone(),
                reasons,
            }))
        })
        .collect();
    results.sort_by(|left, right| left.id.cmp(&right.id));
    let semantic_graphs = results.len();
    let mut compact_graphs = 0usize;
    let mut typed_graphs = 0usize;
    let mut observed_reasons = BTreeSet::new();

    for result in results {
        if result.reasons.is_empty() {
            compact_graphs += 1;
        } else {
            typed_graphs += 1;
            observed_reasons.extend(result.reasons.iter().cloned());
        }
    }

    println!(
        "fixtures={fixture_count} semantic_graphs={semantic_graphs} compact={compact_graphs} typed={typed_graphs} reasons={observed_reasons:?}"
    );
    assert!(fixture_count > 20_000, "fixture tree unexpectedly narrowed");
    assert!(
        semantic_graphs > 10_000,
        "semantic coverage unexpectedly narrowed"
    );
    assert!(compact_graphs > 0, "compact form was not exercised");
    assert!(typed_graphs > 0, "typed graph form was not exercised");
    for required in [
        "BINDER-DOES-NOT-ENCLOSE-USE",
        "MULTIPLE-BINDER-OWNERS",
        "NON-CANONICAL-GROUND",
        "NON-COMPACT-FIELD-SHAPE",
        "NON-COMPACT-NAME-DESCRIPTOR",
        "NON-COMPACT-REFERENT",
        "NON-DERIVABLE-GENERATED-CONTENT",
        "REPEATED-SINGLE-USE-EMISSION",
        "SCOPE-DEPENDENCY-WITHOUT-ENCLOSING-BINDER",
        "UNREPRESENTABLE-CYCLE",
    ] {
        assert!(
            observed_reasons.contains(required),
            "fixture corpus did not exercise {required}"
        );
    }
}
