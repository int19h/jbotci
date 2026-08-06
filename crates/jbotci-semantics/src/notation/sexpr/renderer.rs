//! Document assembly and the fallible projection result for smusni v0.
//!
//! Projection either succeeds with exactly one `(Smusni 0 ...)` document or
//! fails with a nonempty ordered list of registered projection errors and no
//! document at all (specification section 16.1). The failure path deliberately
//! returns as soon as planning or elaboration has collected its failed edges:
//! nothing is serialized, so a graph that cannot be projected never pays for a
//! whole-graph capture.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};

use super::datum::{Datum, print_document};
use super::elaborate::{CompactElaborationData, CompactFallback, elaborate_compact};
use super::planner::{ScopeFailure, ScopeFailureKind, plan_references};
use crate::completeness::model::FailureClass;
use crate::model::{
    DiagnosticSeverity, SemanticGraph, SemanticObjectId, SemanticObjectKind, SourceByteSpan,
};
use crate::notation::registry::registered_failure_class;

/// Object kinds whose projection can inhabit the document's `Performable`
/// body position. Any other root denotes a value rather than an act, so the
/// graph has no document body and fails at the whole graph.
const PERFORMABLE_ROOT_KINDS: &[SemanticObjectKind] =
    &[SemanticObjectKind::Utterance, SemanticObjectKind::Sequence];

/// Registered reason for a graph whose root denotes no performable act.
const ROOT_NOT_PERFORMABLE_REASON: &str = "smusni.projection.graph.root-not-performable";

/// Stable message of the root-not-performable failure.
const ROOT_NOT_PERFORMABLE_MESSAGE: &str = "the graph root denotes a value rather than a performable act, so this graph has no document body";

/// Non-golden corpus measurements returned with either result.
///
/// This is the aggregate channel; per-failure detail belongs to
/// [`SmusniProjectionFailed::failures`]. The retained measurements are exactly
/// the three specification section 16.1 names: the failed-edge count, the
/// per-reason counts, and the number of distinct failing owners.
#[invariant(
    *failed_projection_edges == failure_reasons.values().sum::<usize>(),
    "the reason aggregate accounts for exactly the failed projection edges"
)]
#[invariant(failure_reasons.values().all(|count| *count > 0))]
#[invariant(*failing_owners <= *failed_projection_edges)]
#[invariant(*compact_objects == 0 || *failed_projection_edges == 0, "a rendered document has no failed edge")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmusniRenderStats {
    /// Graph objects that reached a compact projection. Zero on the failure
    /// path, because no object is projected once the render is known to fail.
    pub compact_objects: usize,
    /// Failed projection edges, which is also the number of records in
    /// [`SmusniProjectionFailed::failures`] and the sum of `failure_reasons`.
    pub failed_projection_edges: usize,
    /// Distinct semantic owners those failed edges name. One owner can decline
    /// at more than one boundary, so this is at most the edge count.
    pub failing_owners: usize,
    /// Total graph-owned semantic diagnostics of every severity, not warnings
    /// alone.
    pub semantic_diagnostic_count: usize,
    /// One entry per registered reason, counting the failed projection edges it
    /// accounts for.
    pub failure_reasons: BTreeMap<&'static str, usize>,
}

/// One nonfatal semantic diagnostic carried beside, never inside, the datum.
///
/// These are the graph's own attached diagnostics. They are deliberately not
/// projection errors: this implementation does not promote a graph-attached
/// diagnostic of severity `error` into a projection failure, because a graph
/// that carries one still has a faithful projection when every edge projects.
#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmusniDiagnostic {
    pub source_object: SemanticObjectId,
    pub severity: DiagnosticSeverity,
    pub message: String,
}

impl fmt::Display for SmusniDiagnostic {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "smusni semantic {} on {}: {}",
            match self.severity {
                DiagnosticSeverity::Info => "info",
                DiagnosticSeverity::Warning => "warning",
                DiagnosticSeverity::Error => "error",
            },
            self.source_object,
            self.message
        )
    }
}

/// Which section-16.2 attribution rule produced a failure record's span.
///
/// The rules are tried in order, so this also says how much provenance the
/// graph actually supplied. `WholeInput` is the genuinely source-free case: a
/// host that owns the original text presents the whole input rather than the
/// renderer's best approximation of it.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureSpanSource {
    /// Rule 1: the primary span of the failed edge's owner.
    Owner,
    /// Rule 2: the span of the use site.
    UseSite,
    /// Rule 3: the nearest source-bearing semantic ancestor.
    Ancestor,
    /// Rule 4: no source-bearing object was reachable, so the record names the
    /// whole input and carries its identities as notes.
    WholeInput,
}

/// One registered projection failure.
///
/// The record is complete at creation: its span is already resolved by the
/// section-16.2 attribution order, so late formatting never has to rediscover
/// provenance. Severity is fixed at `error` by the specification and is
/// therefore not a field.
#[invariant(reason_id.starts_with("smusni.projection.") && !message.is_empty())]
#[invariant(span.byte_start <= span.byte_end)]
#[invariant(*span_source == FailureSpanSource::Owner -> owner.is_some())]
#[invariant(*span_source == FailureSpanSource::UseSite -> use_site.is_some())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmusniProjectionFailure {
    /// Registered id from the closed `smusni.projection.` namespace.
    pub reason_id: &'static str,
    /// Stable message fixed by the failure's typed cause.
    pub message: &'static str,
    /// The section-16.2 class, taken from this reason's registry row.
    pub failure_class: FailureClass,
    /// The resolved primary span, chosen when this record was created.
    pub span: SourceByteSpan,
    /// Which attribution rule chose `span`.
    pub span_source: FailureSpanSource,
    /// The object whose projection or binding failed, when the graph has one.
    pub owner: Option<SemanticObjectId>,
    /// The referring object, when the failure is about a relationship between
    /// two objects rather than one object's own shape.
    pub use_site: Option<SemanticObjectId>,
}

impl SmusniProjectionFailure {
    /// Every projection failure is an error; section 16.1 fixes this.
    #[requires(true)]
    #[ensures(ret == DiagnosticSeverity::Error)]
    pub fn severity(&self) -> DiagnosticSeverity {
        DiagnosticSeverity::Error
    }
}

impl fmt::Display for SmusniProjectionFailure {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.reason_id, self.message)
    }
}

/// A successful projection: one complete document, its nonfatal diagnostics,
/// and its statistics.
#[invariant(!text.is_empty() && text.ends_with('\n') && !text.ends_with("\n\n"))]
#[invariant(stats.failed_projection_edges == 0, "a rendered document has no failed edge")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmusniRender {
    pub text: String,
    pub stats: SmusniRenderStats,
    pub diagnostics: Vec<SmusniDiagnostic>,
}

/// A failed projection: no document, a nonempty ordered failure list, the
/// nonfatal diagnostics collected before the failure, and the statistics.
#[invariant(!failures.is_empty(), "a failed projection reports at least one failure")]
#[invariant(stats.failed_projection_edges == failures.len())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmusniProjectionFailed {
    pub failures: Vec<SmusniProjectionFailure>,
    pub diagnostics: Vec<SmusniDiagnostic>,
    pub stats: SmusniRenderStats,
}

/// The two disjoint projection results of specification section 16.1.
pub type SmusniProjection = Result<SmusniRender, SmusniProjectionFailed>;

/// Project one graph, appending pre-rendered word-card data inside the
/// document.
///
/// The word cards are built by the caller and only reach the document on the
/// success path, so a failed projection never carries a `Words` section.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.as_ref().is_ok_and(|render| render.text.ends_with('\n')) || ret.is_err())]
#[ensures(
    ret.as_ref().err().is_none_or(|failed| failed.failures.len() == failed.stats.failed_projection_edges),
    "every failed projection edge reaches the failure channel as its own record"
)]
pub fn render_document(graph: &SemanticGraph, word_cards: &[Datum]) -> SmusniProjection {
    let spans = SpanResolver::new(graph);
    let diagnostics = collect_diagnostics(graph);
    let semantic_diagnostic_count = diagnostics.len();

    // The document body is a `Performable`. A root that denotes a value has no
    // body position at all, so this is a whole-graph failure with no smaller
    // owner and it supersedes any edge inside the graph.
    if !PERFORMABLE_ROOT_KINDS.contains(&graph.objects[&graph.root].object_kind()) {
        let failure = spans.resolve(
            ROOT_NOT_PERFORMABLE_REASON,
            ROOT_NOT_PERFORMABLE_MESSAGE,
            Some(graph.root),
            None,
        );
        return Err(failed(
            vec![failure],
            diagnostics,
            semantic_diagnostic_count,
        ));
    }

    let plan = plan_references(graph);
    if !plan.compact_is_eligible() {
        // Scope planning proved that no lexical compact tree represents this
        // graph. Return here: nothing is elaborated and nothing is serialized.
        let failures = plan
            .failures()
            .iter()
            .map(|failure| scope_failure(&spans, failure))
            .collect::<Vec<_>>();
        return Err(failed(failures, diagnostics, semantic_diagnostic_count));
    }

    let elaboration = elaborate_compact(graph, &plan);
    if elaboration.requires_typed_graph() {
        let failures = elaboration
            .failures
            .iter()
            .map(|failure| compact_failure(&spans, failure))
            .collect::<Vec<_>>();
        return Err(failed(failures, diagnostics, semantic_diagnostic_count));
    }

    let data!(CompactElaboration {
        body,
        compact_objects,
        ..
    }) = elaboration.into_data();
    let mut children = vec![Datum::unsigned(0), body];
    if !word_cards.is_empty() {
        children.push(Datum::form("Words", word_cards.iter().cloned()));
    }
    let text = print_document(&Datum::form("Smusni", children));
    let stats = new!(SmusniRenderStats {
        compact_objects: compact_objects,
        failed_projection_edges: 0,
        failing_owners: 0,
        semantic_diagnostic_count: semantic_diagnostic_count,
        failure_reasons: BTreeMap::new(),
    });
    Ok(new!(SmusniRender {
        text,
        stats,
        diagnostics
    }))
}

/// Assemble the failed result from an already ordered failure channel.
#[requires(!failures.is_empty())]
#[ensures(ret.failures.len() == old(failures.len()))]
fn failed(
    failures: Vec<SmusniProjectionFailure>,
    diagnostics: Vec<SmusniDiagnostic>,
    semantic_diagnostic_count: usize,
) -> SmusniProjectionFailed {
    let stats = new!(SmusniRenderStats {
        compact_objects: 0,
        failed_projection_edges: failures.len(),
        failing_owners: failures
            .iter()
            .filter_map(|failure| failure.owner)
            .collect::<BTreeSet<_>>()
            .len(),
        semantic_diagnostic_count: semantic_diagnostic_count,
        failure_reasons: failure_reason_counts(&failures),
    });
    new!(SmusniProjectionFailed {
        failures,
        diagnostics,
        stats
    })
}

/// Project one planner scope failure into its per-edge failure record.
#[requires(true)]
#[ensures(ret.reason_id == failure.kind.reason_id())]
fn scope_failure(spans: &SpanResolver<'_>, failure: &ScopeFailure) -> SmusniProjectionFailure {
    spans.resolve(
        failure.kind.reason_id(),
        failure.kind.message(),
        failure.binder,
        failure.use_site,
    )
}

/// Project one declined compact projection into its per-edge failure record.
#[requires(true)]
#[ensures(ret.reason_id == failure.cause.reason_id())]
fn compact_failure(spans: &SpanResolver<'_>, failure: &CompactFallback) -> SmusniProjectionFailure {
    spans.resolve(
        failure.cause.reason_id(),
        failure.cause.message(),
        Some(failure.owner),
        // A declined compact projection is a property of one object's own
        // shape, so there is no second object to name.
        None,
    )
}

/// Count how many failed edges each registered reason accounts for.
#[requires(true)]
#[ensures(ret.values().all(|count| *count > 0))]
#[ensures(ret.values().sum::<usize>() == failures.len())]
fn failure_reason_counts(failures: &[SmusniProjectionFailure]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for failure in failures {
        *counts.entry(failure.reason_id).or_default() += 1;
    }
    counts
}

/// Collect the graph's own attached diagnostics in stable object order.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(true)]
fn collect_diagnostics(graph: &SemanticGraph) -> Vec<SmusniDiagnostic> {
    graph
        .objects
        .iter()
        .flat_map(|(source_object, object)| {
            object.diagnostics().iter().map(|diagnostic| {
                new!(SmusniDiagnostic {
                    source_object: *source_object,
                    severity: diagnostic.severity,
                    message: diagnostic.message.clone(),
                })
            })
        })
        .collect()
}

/// Resolves one failure's primary span by the section-16.2 attribution order.
///
/// Rule 3 needs an ancestor. The planner and elaborator do not retain the
/// exact ancestor they held, so this walks the graph's own reverse-reference
/// edges outward from the owner in identity order and takes the first
/// source-bearing object it reaches. That is deterministic for a given graph,
/// which is what section 17 requires of the failure order and its spans.
#[invariant(true)]
#[derive(Debug)]
struct SpanResolver<'graph> {
    graph: &'graph SemanticGraph,
    parents: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    whole_input: SourceByteSpan,
}

impl<'graph> SpanResolver<'graph> {
    #[requires(graph.objects.contains_key(&graph.root))]
    #[ensures(true)]
    fn new(graph: &'graph SemanticGraph) -> Self {
        let mut parents: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> = BTreeMap::new();
        let mut references = Vec::new();
        let mut whole_input: Option<SourceByteSpan> = None;
        for (id, object) in &graph.objects {
            references.clear();
            object.references_into(&mut references);
            for target in references.iter().copied() {
                parents.entry(target).or_default().insert(*id);
            }
            if let Some(source) = object.source() {
                whole_input = Some(match whole_input {
                    None => source.span,
                    Some(current) => SourceByteSpan {
                        byte_start: current.byte_start.min(source.span.byte_start),
                        byte_end: current.byte_end.max(source.span.byte_end),
                    },
                });
            }
        }
        // A graph with no source-bearing object anywhere still needs a span; an
        // empty one at the start of the input is the honest answer, and the
        // record says so through `FailureSpanSource::WholeInput`.
        let whole_input = whole_input.unwrap_or(SourceByteSpan {
            byte_start: 0,
            byte_end: 0,
        });
        Self {
            graph,
            parents,
            whole_input,
        }
    }

    /// The span an object declares for itself, if any.
    #[requires(true)]
    #[ensures(true)]
    fn own_span(&self, id: SemanticObjectId) -> Option<SourceByteSpan> {
        self.graph
            .objects
            .get(&id)
            .and_then(|object| object.source())
            .map(|source| source.span)
    }

    /// The nearest source-bearing ancestor of `id`, in breadth-first identity
    /// order over the reverse-reference edges.
    #[requires(true)]
    #[ensures(true)]
    fn ancestor_span(&self, id: SemanticObjectId) -> Option<SourceByteSpan> {
        let mut seen = BTreeSet::from([id]);
        let mut pending = VecDeque::from([id]);
        while let Some(current) = pending.pop_front() {
            let Some(parents) = self.parents.get(&current) else {
                continue;
            };
            for parent in parents.iter().copied() {
                if !seen.insert(parent) {
                    continue;
                }
                if let Some(span) = self.own_span(parent) {
                    return Some(span);
                }
                pending.push_back(parent);
            }
        }
        None
    }

    /// Build one complete failure record, resolving its span now.
    #[requires(reason_id.starts_with("smusni.projection.") && !message.is_empty())]
    #[ensures(ret.reason_id == reason_id && ret.message == message)]
    fn resolve(
        &self,
        reason_id: &'static str,
        message: &'static str,
        owner: Option<SemanticObjectId>,
        use_site: Option<SemanticObjectId>,
    ) -> SmusniProjectionFailure {
        let (span, span_source) = owner
            .and_then(|owner| self.own_span(owner))
            .map(|span| (span, FailureSpanSource::Owner))
            .or_else(|| {
                use_site
                    .and_then(|use_site| self.own_span(use_site))
                    .map(|span| (span, FailureSpanSource::UseSite))
            })
            .or_else(|| {
                owner
                    .or(use_site)
                    .and_then(|id| self.ancestor_span(id))
                    .map(|span| (span, FailureSpanSource::Ancestor))
            })
            .unwrap_or((self.whole_input, FailureSpanSource::WholeInput));
        new!(SmusniProjectionFailure {
            reason_id,
            message,
            failure_class: registered_failure_class(reason_id),
            span,
            span_source,
            owner,
            use_site,
        })
    }
}

/// Keep the complete named failure table linked into this renderer even when a
/// corpus happens not to exercise all classes.
#[requires(true)]
#[ensures(ret.len() == 7)]
pub fn scope_failure_labels() -> [&'static str; 7] {
    [
        ScopeFailureKind::MultipleBinderOwners.label(),
        ScopeFailureKind::BinderDoesNotEncloseUse.label(),
        ScopeFailureKind::ScopeDependencyBinderUnowned.label(),
        ScopeFailureKind::ScopeDependencyWithoutEnclosingBinder.label(),
        ScopeFailureKind::UnrepresentableCycle.label(),
        ScopeFailureKind::DefinitionSiteDoesNotDominateUse.label(),
        ScopeFailureKind::DeclarationPlanningDidNotConverge.label(),
    ]
}
