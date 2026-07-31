//! Fixture-derived regression coverage for the XML representation boundary.
//!
//! The exhaustive check deliberately attempts every repository fixture.  It is
//! not a sample: every input whose strict morphology, syntax, and semantic
//! construction succeed must render twice, account every non-waived surface,
//! and parse as XML.

use std::collections::BTreeSet;
#[cfg(feature = "expensive_contracts")]
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
        (
            "li xo jei do curve",
            Some("SCOPE-DEPENDENCY-WITHOUT-ENCLOSING-BINDER"),
        ),
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
        let reasons = assert_render_contract(&graph, "<reviewer-regression>");
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
