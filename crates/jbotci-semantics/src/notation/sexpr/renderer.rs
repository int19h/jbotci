//! Document assembly for experimental smusni S-expressions.

use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};

use super::datum::{Datum, print_document};
use super::elaborate::elaborate_compact;
use super::planner::{ReferencePlan, ScopeFailureKind, plan_references};
use super::structural::{definition_datum, reference_datum};
use crate::model::SemanticGraph;

/// Top-level representation selected by the typed scope planner.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentMode {
    Compact,
    TypedGraph,
}

/// Non-golden corpus measurements returned alongside the document.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmusniRenderStats {
    pub mode: DocumentMode,
    pub compact_objects: usize,
    pub object_fallbacks: usize,
    pub warning_count: usize,
    pub fallback_reasons: BTreeMap<&'static str, usize>,
}

/// Rendered document plus structural measurements.
#[invariant(!text.is_empty() && text.ends_with('\n') && !text.ends_with("\n\n"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmusniRender {
    pub text: String,
    pub stats: SmusniRenderStats,
}

/// Render one graph, appending pre-rendered word-card data inside the document.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.text.ends_with('\n') && !ret.text.ends_with("\n\n"))]
pub fn render_document(graph: &SemanticGraph, word_cards: &[Datum]) -> SmusniRender {
    let plan = plan_references(graph);
    let (body, warnings, stats) = if plan.compact_is_eligible() {
        render_compact_document(graph, &plan)
    } else {
        render_typed_graph(graph, &plan)
    };

    let mut children = vec![Datum::Unsigned(0), body];
    if !warnings.is_empty() {
        children.push(Datum::form("Warnings", warnings));
    }
    if !word_cards.is_empty() {
        children.push(Datum::form("Words", word_cards.iter().cloned()));
    }
    let text = print_document(&Datum::form("Smusni", children));
    new!(SmusniRender { text, stats })
}

/// Compact semantic projection with typed object fallbacks where an exact
/// human-readable form cannot be proved faithful.
#[requires(graph.objects.contains_key(&graph.root))]
#[requires(plan.compact_is_eligible())]
#[ensures(true)]
fn render_compact_document(
    graph: &SemanticGraph,
    plan: &ReferencePlan,
) -> (Datum, Vec<Datum>, SmusniRenderStats) {
    debug_assert!(plan.compact_is_eligible());
    let elaboration = elaborate_compact(graph, plan);
    (
        elaboration.body,
        elaboration.warnings,
        SmusniRenderStats {
            mode: DocumentMode::Compact,
            compact_objects: elaboration.compact_objects,
            object_fallbacks: elaboration.object_fallbacks,
            warning_count: graph
                .objects
                .values()
                .map(|object| object.diagnostics().len())
                .sum(),
            fallback_reasons: elaboration.fallback_reasons,
        },
    )
}

/// Whole-document graph-faithful fallback selected only for named scope
/// planning failures.
#[requires(graph.objects.contains_key(&graph.root))]
#[requires(!plan.compact_is_eligible())]
#[ensures(true)]
fn render_typed_graph(
    graph: &SemanticGraph,
    plan: &ReferencePlan,
) -> (Datum, Vec<Datum>, SmusniRenderStats) {
    let mut children = vec![Datum::form("Root", [reference_datum(graph.root)])];
    children.extend(
        graph
            .objects
            .iter()
            .map(|(id, object)| definition_datum(*id, object)),
    );
    let mut fallback_reasons = BTreeMap::new();
    for failure in plan.failures() {
        *fallback_reasons.entry(failure.kind.label()).or_default() += 1;
    }
    (
        Datum::form("TypedGraph", children),
        Vec::new(),
        SmusniRenderStats {
            mode: DocumentMode::TypedGraph,
            compact_objects: 0,
            object_fallbacks: graph.objects.len(),
            warning_count: graph
                .objects
                .values()
                .map(|object| object.diagnostics().len())
                .sum(),
            fallback_reasons,
        },
    )
}

/// Keep the complete named failure table linked into this renderer even when a
/// corpus happens not to exercise all classes.
#[requires(true)]
#[ensures(ret.len() == 6)]
pub fn scope_failure_labels() -> [&'static str; 6] {
    [
        ScopeFailureKind::MultipleBinderOwners.label(),
        ScopeFailureKind::BinderDoesNotEncloseUse.label(),
        ScopeFailureKind::ScopeDependencyWithoutEnclosingBinder.label(),
        ScopeFailureKind::UnrepresentableCycle.label(),
        ScopeFailureKind::DefinitionSiteDoesNotDominateUse.label(),
        ScopeFailureKind::DeclarationPlanningDidNotConverge.label(),
    ]
}
