//! Document assembly for experimental smusni S-expressions.

use std::collections::BTreeMap;
use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};

use super::datum::{Datum, print_document};
use super::elaborate::{CompactElaborationData, CompactFallback, elaborate_compact};
use super::planner::{ScopeFailure, ScopeFailureKind, plan_references};
use super::structural::raw_graph_datum;
use crate::model::{DiagnosticSeverity, SemanticGraph, SemanticObjectId};

/// Top-level representation selected by the typed scope planner.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentMode {
    Compact,
    TypedGraph,
}

/// Non-golden corpus measurements returned alongside the document.
///
/// This is the aggregate channel; per-failure detail belongs to
/// [`SmusniRender::diagnostics`]. Objects and edges are separate measurements
/// and must not be conflated: `object_fallbacks` counts graph objects, while
/// `failed_projection_edges` and `fallback_reasons` count failed projection
/// edges. Since one object can decline at two different boundaries, the edge
/// count can exceed the number of distinct failed objects.
#[invariant(*mode == DocumentMode::Compact || *compact_objects == 0)]
#[invariant(*object_fallbacks == 0 || !fallback_reasons.is_empty())]
#[invariant(
    *failed_projection_edges == fallback_reasons.values().sum::<usize>(),
    "the reason aggregate accounts for exactly the failed projection edges"
)]
#[invariant(fallback_reasons.values().all(|count| *count > 0))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmusniRenderStats {
    pub mode: DocumentMode,
    pub compact_objects: usize,
    /// Graph objects that did not reach a compact projection: the distinct
    /// objects whose own projection was declined, or the whole object count
    /// when scope planning rejected compact rendering before elaboration. This
    /// is an object count and never the failed-edge count.
    pub object_fallbacks: usize,
    /// Failed projection edges, which is also the number of
    /// `SmusniDiagnostic::Fallback` records in [`SmusniRender::diagnostics`]
    /// and the sum of `fallback_reasons`.
    pub failed_projection_edges: usize,
    /// Total graph-owned semantic diagnostics of every severity, not warnings
    /// alone.
    pub semantic_diagnostic_count: usize,
    /// One entry per registered reason, counting the failed projection edges it
    /// accounts for.
    pub fallback_reasons: BTreeMap<&'static str, usize>,
}

/// One renderer diagnostic carried beside, never inside, the semantic datum.
///
/// `Fallback` is one record per failed projection edge, not an aggregate: the
/// reason code and message are stable for the failure's typed cause, and the
/// affected graph objects travel as typed evidence. `owner` is the object whose
/// projection or binding failed; `use_site` is the referring object when the
/// failure is about a relationship between two objects rather than one object's
/// own shape.
#[invariant(::Semantic { message, .. } => !message.is_empty())]
#[invariant(::Fallback { reason_id, message, .. } => reason_id.starts_with("smusni.fallback.") && !message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmusniDiagnostic {
    Semantic {
        source_object: SemanticObjectId,
        severity: DiagnosticSeverity,
        message: String,
    },
    Fallback {
        reason_id: &'static str,
        message: &'static str,
        owner: Option<SemanticObjectId>,
        use_site: Option<SemanticObjectId>,
    },
}

impl fmt::Display for SmusniDiagnostic {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(SmusniDiagnostic::Semantic {
                source_object,
                severity,
                message,
            }) => write!(
                formatter,
                "smusni semantic {} on {source_object}: {message}",
                match severity {
                    DiagnosticSeverity::Info => "info",
                    DiagnosticSeverity::Warning => "warning",
                    DiagnosticSeverity::Error => "error",
                }
            ),
            data!(SmusniDiagnostic::Fallback {
                reason_id,
                message,
                owner,
                use_site,
            }) => {
                write!(formatter, "smusni fallback {reason_id}")?;
                if let Some(owner) = owner {
                    write!(formatter, " on {owner}")?;
                }
                if let Some(use_site) = use_site {
                    write!(formatter, " used by {use_site}")?;
                }
                write!(formatter, ": {message}")
            }
        }
    }
}

/// Rendered document plus structural measurements.
#[invariant(!text.is_empty() && text.ends_with('\n') && !text.ends_with("\n\n"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmusniRender {
    pub text: String,
    pub stats: SmusniRenderStats,
    pub diagnostics: Vec<SmusniDiagnostic>,
}

/// Render one graph, appending pre-rendered word-card data inside the document.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.text.ends_with('\n') && !ret.text.ends_with("\n\n"))]
#[ensures(
    ret.diagnostics.iter().filter(|diagnostic| matches!(diagnostic.as_data(), data!(SmusniDiagnostic::Fallback { .. }))).count()
        == ret.stats.failed_projection_edges,
    "every failed projection edge reaches the diagnostic channel as its own record"
)]
pub fn render_document(graph: &SemanticGraph, word_cards: &[Datum]) -> SmusniRender {
    let plan = plan_references(graph);
    let mut fallbacks = plan
        .failures()
        .iter()
        .map(scope_diagnostic)
        .collect::<Vec<_>>();
    let (body, mode, compact_objects, object_fallbacks) = if plan.compact_is_eligible() {
        let elaboration = elaborate_compact(graph, &plan);
        if elaboration.requires_typed_graph() {
            // The planner proved compact eligibility, so `fallbacks` is empty
            // here and the elaborator owns every failed edge. The object
            // statistic counts the distinct objects those edges name, which is
            // deliberately smaller than the edge count when one object declined
            // at more than one boundary.
            fallbacks = elaboration
                .failures
                .iter()
                .map(compact_diagnostic)
                .collect();
            (
                typed_graph_datum(graph, &fallbacks),
                DocumentMode::TypedGraph,
                0,
                elaboration.failed_owners(),
            )
        } else {
            let data!(CompactElaboration {
                body,
                compact_objects,
                ..
            }) = elaboration.into_data();
            (body, DocumentMode::Compact, compact_objects, 0)
        }
    } else {
        // Scope planning rejected compact rendering before any object was
        // projected, so every graph object is rendered raw.
        (
            typed_graph_datum(graph, &fallbacks),
            DocumentMode::TypedGraph,
            0,
            graph.objects.len(),
        )
    };

    let mut children = vec![Datum::unsigned(0), body];
    if !word_cards.is_empty() {
        children.push(Datum::form("Words", word_cards.iter().cloned()));
    }
    let text = print_document(&Datum::form("Smusni", children));
    let stats = new!(SmusniRenderStats {
        mode: mode,
        compact_objects: compact_objects,
        object_fallbacks: object_fallbacks,
        failed_projection_edges: fallbacks.len(),
        semantic_diagnostic_count: graph
            .objects
            .values()
            .map(|object| object.diagnostics().len())
            .sum(),
        fallback_reasons: fallback_reason_counts(&fallbacks),
    });
    let diagnostics = collect_diagnostics(graph, fallbacks);
    new!(SmusniRender {
        text,
        stats,
        diagnostics
    })
}

/// Project one planner scope failure into its per-edge diagnostic record.
#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(SmusniDiagnostic::Fallback { .. })))]
fn scope_diagnostic(failure: &ScopeFailure) -> SmusniDiagnostic {
    new!(SmusniDiagnostic::Fallback {
        reason_id: failure.kind.reason_id(),
        message: failure.kind.message(),
        owner: failure.binder,
        use_site: failure.use_site,
    })
}

/// Project one declined compact projection into its per-edge diagnostic record.
#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(SmusniDiagnostic::Fallback { .. })))]
fn compact_diagnostic(failure: &CompactFallback) -> SmusniDiagnostic {
    new!(SmusniDiagnostic::Fallback {
        reason_id: failure.cause.reason_id(),
        message: failure.cause.message(),
        owner: Some(failure.owner),
        // A declined compact projection is a property of one object's own
        // shape, so there is no second object to name.
        use_site: None,
    })
}

/// Count how many failed edges each registered reason accounts for.
#[requires(fallbacks.iter().all(|diagnostic| matches!(diagnostic.as_data(), data!(SmusniDiagnostic::Fallback { .. }))))]
#[ensures(ret.values().all(|count| *count > 0))]
#[ensures(ret.values().sum::<usize>() == fallbacks.len())]
fn fallback_reason_counts(fallbacks: &[SmusniDiagnostic]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for fallback in fallbacks {
        if let data!(SmusniDiagnostic::Fallback { reason_id, .. }) = fallback.as_data() {
            *counts.entry(*reason_id).or_default() += 1;
        }
    }
    counts
}

/// Collect graph diagnostics and the already ordered per-edge fallback records.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.len() >= old(fallbacks.len()))]
fn collect_diagnostics(
    graph: &SemanticGraph,
    fallbacks: Vec<SmusniDiagnostic>,
) -> Vec<SmusniDiagnostic> {
    let mut diagnostics = graph
        .objects
        .iter()
        .flat_map(|(source_object, object)| {
            object.diagnostics().iter().map(|diagnostic| {
                new!(SmusniDiagnostic::Semantic {
                    source_object: *source_object,
                    severity: diagnostic.severity,
                    message: diagnostic.message.clone(),
                })
            })
        })
        .collect::<Vec<_>>();
    diagnostics.extend(fallbacks);
    diagnostics
}

/// Whole-document graph-faithful representation, labelled with the first
/// registered reason in the ordered failure channel.
#[requires(graph.objects.contains_key(&graph.root))]
#[requires(!fallbacks.is_empty())]
#[ensures(ret.form_head() == Some("TypedGraph"))]
fn typed_graph_datum(graph: &SemanticGraph, fallbacks: &[SmusniDiagnostic]) -> Datum {
    let data!(SmusniDiagnostic::Fallback { reason_id, .. }) = fallbacks[0].as_data() else {
        unreachable!("the fallback channel carries only fallback records")
    };
    Datum::form(
        "TypedGraph",
        [
            Datum::string("SemanticGraph"),
            Datum::string(*reason_id),
            raw_graph_datum(graph),
        ],
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
