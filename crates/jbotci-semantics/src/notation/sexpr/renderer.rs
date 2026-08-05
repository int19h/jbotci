//! Document assembly for experimental smusni S-expressions.

use std::collections::BTreeMap;
use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};

use super::datum::{Datum, print_document};
use super::elaborate::elaborate_compact;
use super::planner::{ReferencePlan, ScopeFailureKind, plan_references};
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
#[invariant(*mode == DocumentMode::Compact || *compact_objects == 0)]
#[invariant(*object_fallbacks == 0 || !fallback_reasons.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmusniRenderStats {
    pub mode: DocumentMode,
    pub compact_objects: usize,
    pub object_fallbacks: usize,
    pub warning_count: usize,
    pub fallback_reasons: BTreeMap<&'static str, usize>,
}

/// One renderer diagnostic carried beside, never inside, the semantic datum.
#[invariant(::Semantic { message, .. } => !message.is_empty())]
#[invariant(::Fallback { reason_id, occurrences } => reason_id.starts_with("smusni.fallback.") && *occurrences > 0)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmusniDiagnostic {
    Semantic {
        source_object: SemanticObjectId,
        severity: DiagnosticSeverity,
        message: String,
    },
    Fallback {
        reason_id: &'static str,
        occurrences: usize,
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
                occurrences,
            }) => write!(
                formatter,
                "smusni fallback: {reason_id} ({occurrences} occurrence{})",
                if *occurrences == 1 { "" } else { "s" },
            ),
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
pub fn render_document(graph: &SemanticGraph, word_cards: &[Datum]) -> SmusniRender {
    let plan = plan_references(graph);
    let (body, stats) = if plan.compact_is_eligible() {
        let elaboration = elaborate_compact(graph, &plan);
        if elaboration.requires_typed_graph {
            render_typed_graph_with_reasons(graph, &plan, elaboration.fallback_reasons)
        } else {
            assemble_compact_document(graph, elaboration)
        }
    } else {
        render_typed_graph(graph, &plan)
    };

    let mut children = vec![Datum::unsigned(0), body];
    if !word_cards.is_empty() {
        children.push(Datum::form("Words", word_cards.iter().cloned()));
    }
    let text = print_document(&Datum::form("Smusni", children));
    let diagnostics = collect_diagnostics(graph, &stats.fallback_reasons);
    new!(SmusniRender {
        text,
        stats,
        diagnostics
    })
}

/// Collect graph diagnostics and renderer fallbacks once in stable order.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.len() >= fallback_reasons.len())]
fn collect_diagnostics(
    graph: &SemanticGraph,
    fallback_reasons: &BTreeMap<&'static str, usize>,
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
    diagnostics.extend(fallback_reasons.iter().map(|(reason_id, occurrences)| {
        new!(SmusniDiagnostic::Fallback {
            reason_id: *reason_id,
            occurrences: *occurrences,
        })
    }));
    diagnostics
}

/// Assemble a compact elaboration after its local typed fallbacks have proved
/// that no binder-bearing object requires whole-graph scope.
#[requires(graph.objects.contains_key(&graph.root))]
#[requires(!elaboration.requires_typed_graph)]
#[ensures(true)]
fn assemble_compact_document(
    graph: &SemanticGraph,
    elaboration: super::elaborate::CompactElaboration,
) -> (Datum, SmusniRenderStats) {
    (
        elaboration.body,
        new!(SmusniRenderStats {
            mode: DocumentMode::Compact,
            compact_objects: elaboration.compact_objects,
            object_fallbacks: elaboration.object_fallbacks,
            warning_count: graph
                .objects
                .values()
                .map(|object| object.diagnostics().len())
                .sum(),
            fallback_reasons: elaboration.fallback_reasons
        }),
    )
}

/// Whole-document graph-faithful fallback selected only for named scope
/// planning failures.
#[requires(graph.objects.contains_key(&graph.root))]
#[requires(!plan.compact_is_eligible())]
#[ensures(true)]
fn render_typed_graph(graph: &SemanticGraph, plan: &ReferencePlan) -> (Datum, SmusniRenderStats) {
    render_typed_graph_with_reasons(graph, plan, BTreeMap::new())
}

/// Whole-document fallback with any exact elaboration-time scope reason
/// retained alongside planner failures.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(true)]
fn render_typed_graph_with_reasons(
    graph: &SemanticGraph,
    plan: &ReferencePlan,
    mut fallback_reasons: BTreeMap<&'static str, usize>,
) -> (Datum, SmusniRenderStats) {
    for failure in plan.failures() {
        *fallback_reasons
            .entry(failure.kind.reason_id())
            .or_default() += 1;
    }
    let reason_id = plan
        .failures()
        .first()
        .map(|failure| failure.kind.reason_id())
        .or_else(|| fallback_reasons.keys().copied().next())
        .expect("whole-document fallback always has a registered cause");
    (
        Datum::form(
            "TypedGraph",
            [
                Datum::string("SemanticGraph"),
                Datum::string(reason_id),
                raw_graph_datum(graph),
            ],
        ),
        new!(SmusniRenderStats {
            mode: DocumentMode::TypedGraph,
            compact_objects: 0,
            object_fallbacks: graph.objects.len(),
            warning_count: graph
                .objects
                .values()
                .map(|object| object.diagnostics().len())
                .sum(),
            fallback_reasons: fallback_reasons
        }),
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
