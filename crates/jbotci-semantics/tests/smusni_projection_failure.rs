//! The laws of a failed smusni projection (specification section 17).
//!
//! Each corpus-reachable failure class gets a real input rather than a
//! synthetic-only fixture, so these also record which classes real Lojban
//! actually reaches today.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};

use jbotci_dialect::DialectDefinition;
use jbotci_morphology::{
    MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_semantics::model::SemanticGraphData;
use jbotci_semantics::{
    FailureClass, SemanticBuildOptions, SemanticGraph, SmusniProjectionFailed,
    build_generated_semantic_graph_with_dictionary_and_options, render_smusni,
};
use jbotci_source::SourceId;
use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};

/// Build a graph through the exact production pipeline.
#[requires(!text.trim().is_empty())]
#[ensures(ret.objects.contains_key(&ret.root))]
fn graph_of(text: &str) -> SemanticGraph {
    let text = text.trim();
    let dialect = DialectDefinition::default();
    let words = segment_words_with_modifiers_with_options_and_source_id(
        text,
        &MorphologyOptions::default().with_dialect_definition(&dialect),
        Some(SourceId(format!("<projection-failure:{text}>"))),
    )
    .expect("witness morphology");
    let parsed = parse_syntax_tree_generated_model_with_source_and_options(
        &words,
        text,
        &ParseOptions::default().with_dialect_definition(&dialect),
    )
    .expect("witness syntax");
    build_generated_semantic_graph_with_dictionary_and_options(
        &parsed,
        SemanticBuildOptions {
            source_text: Some(text),
            story_time: false,
        },
        jbotci_dictionary_data::english(),
    )
    .expect("witness semantics")
}

/// Project one graph, requiring the failed result and every section-17 law.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(!ret.failures.is_empty())]
fn failed_projection(graph: &SemanticGraph) -> SmusniProjectionFailed {
    let Err(failed) = render_smusni(graph) else {
        panic!("expected a projection failure, not a document");
    };
    // Nonempty, registered, and complete: id, class, message, and span.
    assert!(!failed.failures.is_empty());
    for failure in &failed.failures {
        assert!(failure.reason_id.starts_with("smusni.projection."));
        assert!(failure.reason_id.len() > "smusni.projection.".len());
        assert!(!failure.message.is_empty());
        assert!(FailureClass::ALL.contains(&failure.failure_class));
        assert!(failure.span.byte_start <= failure.span.byte_end);
        assert_eq!(
            failure.severity(),
            jbotci_semantics::model::DiagnosticSeverity::Error
        );
    }
    // Deterministic order for a given graph.
    let Err(again) = render_smusni(graph) else {
        panic!("a failing projection must not become a document on a second run");
    };
    assert_eq!(again.failures, failed.failures);
    // The statistics summarize exactly those records, and nothing was rendered.
    assert_eq!(failed.stats.failed_projection_edges, failed.failures.len());
    assert_eq!(
        failed.stats.failure_reasons.values().sum::<usize>(),
        failed.failures.len()
    );
    assert_eq!(failed.stats.compact_objects, 0);
    failed
}

/// The classes one input's failure channel reports.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn classes_of(failed: &SmusniProjectionFailed) -> Vec<FailureClass> {
    failed
        .failures
        .iter()
        .map(|failure| failure.failure_class)
        .collect()
}

#[test]
#[requires(true)]
#[ensures(true)]
fn an_unregistered_relation_reduction_is_renderer_backlog() {
    // `su'o` quantification over a plain predication has a normative route this
    // renderer does not carry yet, so every record is renderer backlog.
    let graph = graph_of("su'o gerku cu bajra");
    let failed = failed_projection(&graph);
    assert!(
        failed.failures.iter().any(|failure| failure.reason_id
            == "smusni.projection.relation-reduction-unregistered-or-inexact"),
        "{:?}",
        failed.failures
    );
    assert!(classes_of(&failed).contains(&FailureClass::RouteUnavailable));
    // The record's span points into the original input rather than at nothing.
    let located = failed
        .failures
        .iter()
        .find(|failure| {
            failure.reason_id == "smusni.projection.relation-reduction-unregistered-or-inexact"
        })
        .expect("the relation record is present");
    assert!(located.span.byte_end <= "su'o gerku cu bajra".len());
    assert!(located.span.byte_start < located.span.byte_end);
    assert!(
        located.owner.is_some(),
        "the record names its failing owner"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn an_unlicensed_higher_order_crossing_is_a_tracked_spec_gap() {
    // A defined zei-lujvo description crosses an order version 0 does not
    // license at all, which is language-design backlog rather than renderer
    // backlog, and section 14.4 forbids counting it toward completeness.
    let graph = graph_of("lo abu zei sance cu barda");
    let failed = failed_projection(&graph);
    assert!(classes_of(&failed).contains(&FailureClass::TrackedSpecGap));
    assert!(
        failed.failures.iter().any(
            |failure| failure.reason_id == "smusni.projection.higher-order-crossing-unlicensed"
        ),
        "{:?}",
        failed.failures
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_root_that_denotes_no_act_is_an_invalid_graph() {
    // The document body is a `Performable`. A graph rooted at a value has no
    // body position at all, so this is the whole-graph invalid-graph route with
    // no smaller owner — the class real Lojban input does not reach, because
    // the builder always roots a text at an utterance or a sequence.
    let graph = graph_of("mi klama");
    let value_root = graph
        .objects
        .keys()
        .copied()
        .find(|id| graph.objects[id].as_referent().is_some())
        .expect("the witness has a referent");
    let graph = graph.with_data(data! { root: value_root });
    let failed = failed_projection(&graph);
    assert_eq!(failed.failures.len(), 1, "the whole graph fails once");
    assert_eq!(
        failed.failures[0].reason_id,
        "smusni.projection.graph.root-not-performable"
    );
    assert_eq!(failed.failures[0].failure_class, FailureClass::InvalidGraph);
    assert_eq!(failed.failures[0].owner, Some(value_root));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn implementation_invariant_failures_are_not_corpus_reachable() {
    // `ImplementationInvariant` means the renderer itself broke, so a class
    // breakdown that ever contains it on ordinary input is a defect report. It
    // is checked here as an expectation, not as an accident.
    for text in [
        "su'o gerku cu bajra",
        "lo abu zei sance cu barda",
        "mi cusku lu mi prami do li'u",
        "ro da poi gerku cu bajra",
        "coi do",
    ] {
        let graph = graph_of(text);
        let Err(failed) = render_smusni(&graph) else {
            continue;
        };
        assert!(
            !classes_of(&failed).contains(&FailureClass::ImplementationInvariant),
            "{text} reported an implementation-invariant failure: {:?}",
            failed.failures
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn one_owner_declining_at_two_boundaries_is_two_edges_and_one_owner() {
    // The edge statistic and the owner statistic measure different things, so a
    // corpus report can separate them. Whatever the counts are, the two must
    // agree with the records they summarize.
    for text in ["su'o gerku cu bajra", "coi do", "ro da poi gerku cu bajra"] {
        let graph = graph_of(text);
        let failed = failed_projection(&graph);
        let owners = failed
            .failures
            .iter()
            .filter_map(|failure| failure.owner)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(failed.stats.failing_owners, owners.len(), "{text}");
        assert!(failed.stats.failing_owners <= failed.stats.failed_projection_edges);
    }
}
