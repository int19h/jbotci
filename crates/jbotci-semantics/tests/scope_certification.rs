//! Corpus certification of the builder-recorded scope model (jbotci#761).
//!
//! Step 2 of the typed-kernel increment lands the scope structure before any
//! consumer exists, precisely so that certifying it against the derivations the
//! graph already carries is cheap to act on. The structural properties hold
//! everywhere and are model invariants; binder-introduction uniqueness and
//! binder-use enclosure do not, because the record graph itself violates them,
//! and this file pins exactly where so the divergence is a reviewed number
//! rather than a surprise for the host planner.

#[allow(unused_imports)]
use bityzba::{ensures, requires};

use jbotci_dialect::DialectDefinition;
use jbotci_morphology::{
    MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_semantics::completeness::corpus::CORPUS_DOCS;
use jbotci_semantics::model::{
    ScopeUseRole, SemanticGraph, scope_binder_introduction_conflicts, scope_regions_form_one_tree,
    scope_source_orders_are_unique_per_region, scope_unenclosed_binder_uses,
    semantic_reference_sites_match_references, semantic_scope_dependence_divergences,
    semantic_scope_occurrences_match_references, semantic_scope_origins_are_total,
};
use jbotci_semantics::{
    SemanticBuildOptions, build_generated_semantic_graph_with_dictionary_and_options,
};
use jbotci_source::SourceId;
use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};

#[requires(!doc.is_empty())]
#[ensures(!ret.objects.is_empty())]
fn graph_for(doc: &str) -> SemanticGraph {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/phaseb_corpus");
    path.push(format!("{doc}.lojban"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let text = text.trim();
    let dialect = DialectDefinition::default();
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
    let words = segment_words_with_modifiers_with_options_and_source_id(
        text,
        &morphology_options,
        Some(SourceId(format!("<scope:{doc}>"))),
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

/// The structure itself is sound on every corpus document: one rooted tree, one
/// occurrence per reference edge in canonical order, an introduction region for
/// every object, and source-order keys that separate the loci one region orders.
#[test]
#[requires(true)]
#[ensures(true)]
fn recorded_scope_structure_is_sound_corpus_wide() {
    for doc in CORPUS_DOCS {
        let graph = graph_for(doc);
        let scope = &graph.scope;
        assert!(
            scope_regions_form_one_tree(scope.root, &scope.regions),
            "{doc}: the region graph is not one rooted tree"
        );
        assert!(
            semantic_reference_sites_match_references(&graph.objects),
            "{doc}: site enumeration and reference enumeration index different edges"
        );
        assert!(
            semantic_scope_occurrences_match_references(scope, &graph.objects),
            "{doc}: the occurrence multiset is not the graph's reference edges"
        );
        assert!(
            semantic_scope_origins_are_total(graph.root, scope, &graph.objects),
            "{doc}: an object has no recorded introduction region"
        );
        assert!(
            scope_source_orders_are_unique_per_region(&scope.regions, &scope.uses),
            "{doc}: two occurrences in one region share a source-order key"
        );
    }
}

/// Every binder the derivation attributes to a constant lies on that constant's
/// origin path. The converse can fail — the derivation blesses a shared object
/// wherever its traversal first reaches it, which can be shallower than where
/// the object is written — so the shortfall is pinned rather than asserted away.
#[test]
#[requires(true)]
#[ensures(true)]
fn derived_dependence_is_covered_by_recorded_structure() {
    let mut shallower = Vec::new();
    for doc in CORPUS_DOCS {
        let graph = graph_for(doc);
        for (constant, derived, structural) in
            semantic_scope_dependence_divergences(&graph.scope, &graph.objects)
        {
            assert!(
                derived.is_subset(&structural),
                "{doc}: {constant} depends on {derived:?} which its origin path {structural:?} does not carry"
            );
            shallower.push(format!("{doc}:{constant}"));
        }
    }
    assert_eq!(
        shallower,
        Vec::<String>::new(),
        "the frozen corpus has no derivation-shallower constants; a new one needs review"
    );
}

/// Two properties the *record graph* does not have, pinned with their witnesses.
///
/// Binder-introduction uniqueness fails where one variable object is bound by
/// two quantifiers (a quantified argument distributed over connective branches,
/// or a prenex `da` shared by connected statements). Binder-use enclosure fails
/// where a quantifier binds an object other segments already reference — `ro do`
/// quantifies the audience deictic in place. Both are the structural form of
/// failures the host planner reports today; neither can become a model
/// invariant until the record model stops producing them.
#[test]
#[requires(true)]
#[ensures(true)]
fn record_model_scope_conflicts_are_pinned() {
    let mut conflicting = Vec::new();
    let mut unenclosed = Vec::new();
    for doc in CORPUS_DOCS {
        let graph = graph_for(doc);
        let scope = &graph.scope;
        if !scope_binder_introduction_conflicts(&scope.regions).is_empty() {
            conflicting.push(*doc);
        }
        let escaping = scope_unenclosed_binder_uses(&scope.regions, &scope.uses);
        assert!(
            escaping
                .iter()
                .all(|occurrence| occurrence.role == ScopeUseRole::BinderUse),
            "{doc}: only binder uses can escape a binder region"
        );
        if !escaping.is_empty() {
            unenclosed.push(*doc);
        }
    }
    assert_eq!(
        conflicting,
        Vec::<&str>::new(),
        "a corpus document now binds one object at two regions"
    );
    assert_eq!(
        unenclosed,
        Vec::<&str>::new(),
        "a corpus document now uses a binder outside its region"
    );
}

/// The three divergence classes, each with the smallest witness that shows it.
///
/// Measured over the whole fixture corpus (22,311 documents that build), the
/// structure is sound everywhere: no malformed tree, no occurrence mismatch, no
/// missing origin, no source-order collision. The three classes below are the
/// entire remainder — 18 documents bind one object twice, 307 use a binder
/// outside its region, and 37 carry a constant the derivation blessed at a
/// shallower path than the region tree records. Each is a property of the
/// record graph rather than of this model, so each is exhibited rather than
/// asserted away.
#[test]
#[requires(true)]
#[ensures(true)]
fn the_three_divergence_classes_have_pinned_witnesses() {
    // One quantified argument distributed over two connective branches binds the
    // same variable object at both quantifiers.
    let graph = graph_for_text("pe'i ro manti cu morsi gi'e cliva le mi zdani");
    assert!(
        !scope_binder_introduction_conflicts(&graph.scope.regions).is_empty(),
        "a distributed quantifier still binds one object at two regions"
    );

    // `ro do` quantifies the audience deictic in place, so the vocative
    // utterance's own reference to the audience is a use outside the binder.
    let graph = graph_for_text("co'o rodo");
    assert!(
        !scope_unenclosed_binder_uses(&graph.scope.regions, &graph.scope.uses).is_empty(),
        "quantifying a deictic in place still leaves uses outside the binder"
    );

    // The connection claim reaches the question's body formula before the
    // utterance does, so the derivation blesses its elided places at empty
    // scope while the region tree places them inside the question.
    let graph = graph_for_text("ijebo ma ciska");
    let divergences = semantic_scope_dependence_divergences(&graph.scope, &graph.objects);
    assert!(
        !divergences.is_empty(),
        "a shared question body still divides the two records"
    );
    assert!(
        divergences
            .iter()
            .all(|(_, derived, structural)| derived.is_subset(structural)),
        "the derivation never attributes a binder the region tree lacks"
    );
}

#[requires(!text.is_empty())]
#[ensures(!ret.objects.is_empty())]
fn graph_for_text(text: &str) -> SemanticGraph {
    let dialect = DialectDefinition::default();
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
    let words = segment_words_with_modifiers_with_options_and_source_id(
        text,
        &morphology_options,
        Some(SourceId("<scope>".to_owned())),
    )
    .unwrap_or_else(|error| panic!("morphology {text:?}: {error}"));
    let parsed =
        parse_syntax_tree_generated_model_with_source_and_options(&words, text, &syntax_options)
            .unwrap_or_else(|error| panic!("syntax {text:?}: {error}"));
    build_generated_semantic_graph_with_dictionary_and_options(
        &parsed,
        SemanticBuildOptions {
            source_text: Some(text),
            story_time: false,
        },
        jbotci_dictionary_data::english(),
    )
    .unwrap_or_else(|error| panic!("semantics {text:?}: {error}"))
}
