//! The one structured smusni projection-failure envelope.
//!
//! Specification section 16.1 makes diagnostics and statistics host-neutral
//! structured data, and forbids a host from recovering structure by parsing
//! rendered text back. This module is that structure: every host profile — the
//! command line, the HTTP problem document, and the MCP tool error — presents
//! this same value, and none of them reparses another's presentation.

use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_diagnostics::{
    Diagnostic, DiagnosticLabel, DiagnosticPhase, DiagnosticSeverity, source_span_from_byte_offsets,
};
use jbotci_semantics::{FailureSpanSource, SmusniProjectionFailed};
use jbotci_source::SourceId;
use serde::Serialize;

use crate::TersmuFormat;

/// The stable top-level code of every smusni projection failure.
pub const SMUSNI_PROJECTION_FAILED_CODE: &str = "smusni-projection-failed";

/// The documented transport safety limit on returned records.
///
/// A transport that truncates reports the total, the returned count, and that
/// truncation occurred, so a consumer never mistakes a bounded list for the
/// whole one. The internal failure always holds every record in order.
pub const TRANSPORT_RECORD_LIMIT: usize = 100;

/// One projection failure in its transport-facing shape.
#[invariant(!reason_id.is_empty() && !message.is_empty() && !failure_class.is_empty())]
#[invariant(span.byte_start <= span.byte_end)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionFailureRecord {
    /// Registered id from the closed `smusni.projection.` namespace.
    pub reason_id: String,
    /// The section-16.2 class, from this reason's registry row.
    pub failure_class: String,
    /// Always `error`; a projection failure has no other severity.
    pub severity: DiagnosticSeverity,
    /// The stable message fixed by the failure's typed cause.
    pub message: String,
    /// The primary span, already resolved by the section-16.2 attribution
    /// order when the record was created.
    pub span: ProjectionFailureSpan,
    /// Which attribution rule chose the span.
    pub span_source: String,
    /// The failing owner's graph identity, when the graph supplies one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// The use site's graph identity, when the failure names two objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_site: Option<String>,
}

/// A byte range in the original Lojban input.
#[invariant(byte_start <= byte_end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionFailureSpan {
    pub byte_start: usize,
    pub byte_end: usize,
}

/// The section-16.1 statistics that survive into every host profile.
#[invariant(*failed_projection_edges == failure_reasons.values().sum::<usize>())]
#[invariant(*failing_owners <= *failed_projection_edges)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionStatistics {
    pub failed_projection_edges: usize,
    pub failing_owners: usize,
    pub semantic_diagnostic_count: usize,
    pub failure_reasons: BTreeMap<String, usize>,
}

/// The complete structured failure envelope.
#[invariant(!code.is_empty() && !format.is_empty())]
#[invariant(*returned == diagnostics.len() && *returned <= *total)]
#[invariant(*truncated == (*returned < *total))]
#[invariant(*total > 0, "a failed projection reports at least one record")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionFailureEnvelope {
    /// Stable machine-readable code, the same in every transport.
    pub code: String,
    /// The format the caller actually asked for. An explicit smusni request is
    /// never silently answered in another format, so this always echoes it.
    pub format: String,
    pub diagnostics: Vec<ProjectionFailureRecord>,
    /// How many records the projection produced in total.
    pub total: usize,
    /// How many of them this envelope carries.
    pub returned: usize,
    /// Whether `diagnostics` is a truncation of the total.
    pub truncated: bool,
    pub statistics: ProjectionStatistics,
}

/// Build the complete envelope from one failed projection.
///
/// Every record is carried in deterministic order. A transport that imposes a
/// display limit applies [`ProjectionFailureEnvelope::limited`] to this value;
/// the internal failure always holds the whole list.
#[requires(!failed.failures.is_empty())]
#[ensures(ret.total == failed.failures.len())]
#[ensures(!ret.truncated)]
pub fn projection_failure_envelope(
    failed: &SmusniProjectionFailed,
    format: TersmuFormat,
) -> ProjectionFailureEnvelope {
    let total = failed.failures.len();
    let diagnostics = failed
        .failures
        .iter()
        .map(|failure| {
            new!(ProjectionFailureRecord {
                reason_id: failure.reason_id.to_owned(),
                failure_class: failure.failure_class.name().to_owned(),
                severity: DiagnosticSeverity::Error,
                message: failure.message.to_owned(),
                span: new!(ProjectionFailureSpan {
                    byte_start: failure.span.byte_start,
                    byte_end: failure.span.byte_end,
                }),
                span_source: span_source_name(failure.span_source).to_owned(),
                owner: failure.owner.map(|owner| owner.to_string()),
                use_site: failure.use_site.map(|use_site| use_site.to_string()),
            })
        })
        .collect::<Vec<_>>();
    let statistics = new!(ProjectionStatistics {
        failed_projection_edges: failed.stats.failed_projection_edges,
        failing_owners: failed.stats.failing_owners,
        semantic_diagnostic_count: failed.stats.semantic_diagnostic_count,
        failure_reasons: failed
            .stats
            .failure_reasons
            .iter()
            .map(|(reason, count)| ((*reason).to_owned(), *count))
            .collect(),
    });
    new!(ProjectionFailureEnvelope {
        code: SMUSNI_PROJECTION_FAILED_CODE.to_owned(),
        format: format.wire_name().to_owned(),
        diagnostics,
        total,
        returned: total,
        truncated: false,
        statistics,
    })
}

impl ProjectionFailureEnvelope {
    /// Truncate the carried records to a host's documented display limit,
    /// keeping the total, the returned count, and the truncation flag exact.
    #[requires(limit.is_none_or(|limit| limit > 0))]
    #[ensures(ret.total == old(self.total))]
    #[ensures(ret.truncated == (ret.returned < ret.total))]
    pub fn limited(self, limit: Option<usize>) -> Self {
        let total = self.total;
        let Some(limit) = limit.filter(|limit| *limit < self.diagnostics.len()) else {
            return self;
        };
        let mut data = self.into_data();
        data.diagnostics.truncate(limit);
        data.returned = limit;
        data.truncated = limit < total;
        Self::from_data(data)
    }

    /// How many records this envelope did not carry.
    #[requires(true)]
    #[ensures(ret == self.total - self.returned)]
    pub fn omitted(&self) -> usize {
        self.total - self.returned
    }
}

/// Convert the envelope's records into the standard labelled diagnostics the
/// rest of the toolchain renders.
///
/// The rendered source is the original Lojban input. A record whose span was
/// attributed to the whole input (section 16.2 rule 4) is labelled over the
/// whole text here, because this layer is the first one that owns it; a span
/// that cannot be located in the source at all is clamped to the input rather
/// than dropped, so no record is ever silently lost.
#[requires(true)]
#[ensures(ret.len() == envelope.diagnostics.len())]
pub fn projection_failure_diagnostics(
    envelope: &ProjectionFailureEnvelope,
    source_id: Option<SourceId>,
    source: &str,
) -> Vec<Diagnostic> {
    envelope
        .diagnostics
        .iter()
        .map(|record| {
            let (byte_start, byte_end) =
                if record.span_source == span_source_name(FailureSpanSource::WholeInput) {
                    (0, source.len())
                } else {
                    (
                        clamp_to_boundary(source, record.span.byte_start),
                        clamp_to_boundary(source, record.span.byte_end),
                    )
                };
            let span =
                source_span_from_byte_offsets(source_id.clone(), source, byte_start, byte_end)
                    .expect("clamped offsets are valid character boundaries of this source");
            let mut notes = vec![format!("failure class: {}", record.failure_class)];
            if let Some(owner) = &record.owner {
                notes.push(format!("failing owner: {owner}"));
            }
            if let Some(use_site) = &record.use_site {
                notes.push(format!("use site: {use_site}"));
            }
            Diagnostic::new(
                DiagnosticSeverity::Error,
                DiagnosticPhase::SemanticProjection,
                record.reason_id.clone(),
                record.message.clone(),
                vec![DiagnosticLabel::new(
                    span,
                    "this content has no smusni projection".to_owned(),
                    true,
                )],
                notes,
                None,
            )
        })
        .collect()
}

/// The stable wire spelling of one attribution rule.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn span_source_name(span_source: FailureSpanSource) -> &'static str {
    match span_source {
        FailureSpanSource::Owner => "owner",
        FailureSpanSource::UseSite => "use-site",
        FailureSpanSource::Ancestor => "ancestor",
        FailureSpanSource::WholeInput => "whole-input",
    }
}

/// Move an offset onto the nearest character boundary at or below it, bounded
/// by the source length.
#[requires(true)]
#[ensures(ret <= source.len() && source.is_char_boundary(ret))]
fn clamp_to_boundary(source: &str, offset: usize) -> usize {
    let mut offset = offset.min(source.len());
    while !source.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}
