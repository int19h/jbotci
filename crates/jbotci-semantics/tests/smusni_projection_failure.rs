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
fn an_unlicensed_higher_order_crossing_is_renderer_backlog() {
    // A defined zei-lujvo description reaches the higher-order crossing
    // boundary. That reason's population is dominated by section-11 designed
    // crossings — an ordinary `lo ka ce'u ...` property reaches it too — so the
    // reason is renderer backlog, not a claim that version 0 cannot express the
    // construction. If the zei-description subcase later proves genuinely
    // unrouted it earns its own reason id under section 14.2's split rule.
    let graph = graph_of("lo abu zei sance cu barda");
    let failed = failed_projection(&graph);
    let crossing = failed
        .failures
        .iter()
        .find(|failure| failure.reason_id == "smusni.projection.higher-order-crossing-unlicensed")
        .unwrap_or_else(|| panic!("{:?}", failed.failures));
    assert_eq!(crossing.failure_class, FailureClass::RouteUnavailable);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn an_unspecified_abstraction_about_a_raised_operand_is_a_tracked_spec_gap() {
    // `tu'a` is the construction section 14.4 names a tracked spec gap: the
    // source withholds which abstraction about the operand is meant, and no
    // version 0 crossing can carry that without inventing content. This is
    // language-design backlog, and section 14.4 forbids counting it toward
    // completeness or approximating it with a plausible predicate.
    let graph = graph_of("mi djica tu'a do");
    let failed = failed_projection(&graph);
    let raised = failed
        .failures
        .iter()
        .find(|failure| failure.reason_id == "smusni.projection.abstraction-about-unspecified")
        .unwrap_or_else(|| panic!("{:?}", failed.failures));
    assert_eq!(raised.failure_class, FailureClass::TrackedSpecGap);
    // The record points at the raising sumti itself rather than at the whole
    // input, and names the raised referent as its failing owner.
    assert!(raised.owner.is_some(), "the record names its failing owner");
    assert_eq!(
        &"mi djica tu'a do"[raised.span.byte_start..raised.span.byte_end],
        "tu'a do"
    );
    // An ordinary event-facet decline is a different boundary with a different
    // reason, so splitting `tu'a` out did not widen the tracked gap.
    assert!(
        failed
            .failures
            .iter()
            .all(|failure| failure.reason_id
                != "smusni.projection.event-facet-reduction-unregistered"),
        "{:?}",
        failed.failures
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn an_ill_scoped_binder_is_an_invalid_graph() {
    // `ro do` quantifies the audience deictic in place, so the vocative
    // utterance's own reference to the audience is evaluated outside the
    // binder's region. That is not a missing route: the graph itself is
    // ill-scoped, and nothing follows about whether smusni can express the
    // corresponding distinction.
    //
    // This is the witness `scope_certification` pins for the same class, and it
    // replaces the stacked question `pau xo ma mo xu` the reconstructing
    // planner used to report here. A question introduces all of its slots at
    // one region and every use of them is inside it, so the record model says
    // that graph is well scoped; only the old reverse-reference reconstruction
    // said otherwise.
    let graph = graph_of("co'o rodo");
    let failed = failed_projection(&graph);
    assert!(classes_of(&failed).contains(&FailureClass::InvalidGraph));
    let ill_scoped = failed
        .failures
        .iter()
        .find(|failure| failure.reason_id == "smusni.projection.binder-does-not-dominate-use")
        .expect("the ill-scoped binder record is present");
    assert_eq!(ill_scoped.failure_class, FailureClass::InvalidGraph);
    // A scope failure names both ends of the edge it describes.
    assert!(ill_scoped.owner.is_some());
    assert!(ill_scoped.use_site.is_some());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn one_object_bound_at_two_scopes_is_an_invalid_graph() {
    // A quantified argument distributed over two connective branches binds the
    // same variable object at both quantifiers, so no single printed binder
    // owns it. `scope_certification` pins this as a property of the record
    // graph rather than a recorder gap, and it is exactly the shape the planner
    // cannot give one lexical home.
    let graph = graph_of("pe'i ro manti cu morsi gi'e cliva le mi zdani");
    let failed = failed_projection(&graph);
    let conflict = failed
        .failures
        .iter()
        .find(|failure| failure.reason_id == "smusni.projection.conflicting-binder-owners")
        .expect("the double-binding record is present");
    assert_eq!(conflict.failure_class, FailureClass::InvalidGraph);
    // The binder is the affected identity; there is no second object, because
    // the conflict is between two scopes rather than between two objects.
    assert!(conflict.owner.is_some());
    assert!(conflict.use_site.is_none());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_description_shared_by_two_acts_has_no_determined_host() {
    // `gy.` resumes `le gerku` in the next sentence, so one reference
    // computation is used in two performed acts. Version 0 hosts an ordinary
    // `Refer` inside its own force segment, and this graph names no discourse
    // host, so the rules do not determine one legal host — section 6.3's
    // registered invalid-graph outcome, not a route the renderer is missing.
    let graph = graph_of("le gerku .i gy. klama");
    let failed = failed_projection(&graph);
    let unhosted = failed
        .failures
        .iter()
        .find(|failure| failure.reason_id == "smusni.projection.dynamic-host-not-unique")
        .expect("the undetermined dynamic host record is present");
    assert_eq!(unhosted.failure_class, FailureClass::InvalidGraph);
    assert!(unhosted.owner.is_some());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn mutually_recursive_shared_values_are_a_tracked_spec_gap() {
    // Three `go'i` back-references over connected statements make the shared
    // values of this text depend on one another in a cycle. `LetRec` binds only
    // groups of inert lambdas, so a cycle through a value that prints no binder
    // has no guarded lexical form at all — the specification's tracked gap,
    // reported here rather than discovered halfway through emission.
    let graph = graph_of("na go'i .ije na'e go'i .ije na'i go'i");
    let failed = failed_projection(&graph);
    let cycle = failed
        .failures
        .iter()
        .find(|failure| failure.reason_id == "smusni.projection.unguarded-or-unrepresentable-scc")
        .expect("the unrepresentable cycle record is present");
    assert_eq!(cycle.failure_class, FailureClass::TrackedSpecGap);
    assert!(cycle.owner.is_some());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_root_that_denotes_no_act_is_a_whole_graph_invalid_graph() {
    // The document body is a `Performable`. A graph rooted at a value has no
    // body position at all, so this is the whole-graph route with no smaller
    // owner. The production builder always roots a text at an utterance or a
    // sequence, so this route is defensive and needs a constructed graph.
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
        "mi djica tu'a do",
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
