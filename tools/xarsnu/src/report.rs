//! Deterministic offline rendering of validated transcript records.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Write as _};
use std::path::Path;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

use crate::protocol::{
    ListenerFlowAbandonReasonData, ProtocolEventData, ProtocolRunOutcomeData, TurnForfeitReasonData,
};
use crate::{
    AbortKind, DiagnosticCategory, ListenerFlowAbandonReason, ProtocolRunOutcome,
    ScenarioConfigError, ScenarioInstance, TaskStatus, TranscriptError, TranscriptRecord,
    TurnForfeitReason, UsageTotals, read_transcript,
};

/// Read, validate, and render one transcript without consulting any external service.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|report| !report.is_empty()) || ret.is_err())]
pub fn report_file(path: &Path) -> Result<String, TranscriptError> {
    let records = read_transcript(path)?;
    Ok(render_report(&records))
}

/// Read, validate, and render only the reshareable visible dialog.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|dialog| !dialog.is_empty()) || ret.is_err())]
pub fn dialog_file(path: &Path) -> Result<String, DialogReportError> {
    let records =
        read_transcript(path).map_err(|source| DialogReportError::Transcript { source })?;
    render_dialog_document(&records).map_err(|source| DialogReportError::Scenario { source })
}

/// A transcript could not be rendered as a standalone dialog document.
#[invariant(::Transcript { .. } => true)]
#[invariant(::Scenario { .. } => true)]
#[derive(Debug)]
pub enum DialogReportError {
    Transcript { source: TranscriptError },
    Scenario { source: ScenarioConfigError },
}

impl fmt::Display for DialogReportError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transcript { source } => source.fmt(formatter),
            Self::Scenario { source } => {
                write!(formatter, "invalid transcript scenario snapshot: {source}")
            }
        }
    }
}

impl Error for DialogReportError {
    #[requires(true)]
    #[ensures(ret.is_some())]
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transcript { source } => Some(source),
            Self::Scenario { source } => Some(source),
        }
    }
}

#[requires(!records.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|dialog| dialog.starts_with("# xarsnu dialog")) || ret.is_err())]
fn render_dialog_document(records: &[TranscriptRecord]) -> Result<String, ScenarioConfigError> {
    let header = match records[0].event.as_data() {
        bityzba::data!(ProtocolEvent::RunStarted { header }) => header,
        _ => unreachable!("validated transcripts begin with a run header"),
    };
    let scenario = ScenarioInstance::from_toml(&header.scenario_instance_toml)?;
    let mut dialog = format!("# xarsnu dialog — {}\n\n", scenario.id());
    write!(dialog, "*scenario {} — ", header.config.scenario)
        .expect("writing to String cannot fail");
    for (index, participant) in header.config.participants.iter().enumerate() {
        if index > 0 {
            dialog.push_str(", ");
        }
        write!(dialog, "{}: {}", participant.name, participant.model)
            .expect("writing to String cannot fail");
    }
    dialog.push_str("*\n\n");
    render_dialog_entries(&mut dialog, records);
    Ok(dialog)
}

#[requires(!records.is_empty())]
#[ensures(report.contains("## Dialog"))]
fn render_dialog_section(report: &mut String, records: &[TranscriptRecord]) {
    report.push_str("\n## Dialog\n\n");
    render_dialog_entries(report, records);
}

#[requires(!records.is_empty())]
#[ensures(report.len() >= old(report.len()))]
fn render_dialog_entries(report: &mut String, records: &[TranscriptRecord]) {
    let mut has_entry = false;
    for record in records {
        let is_entry = matches!(
            record.event.as_data(),
            bityzba::data!(ProtocolEvent::MessagePosted { .. })
                | bityzba::data!(ProtocolEvent::TurnForfeited { .. })
                | bityzba::data!(ProtocolEvent::AnswerSubmitted { .. })
                | bityzba::data!(ProtocolEvent::CheckerOutcome { .. })
                | bityzba::data!(ProtocolEvent::RunAborted { .. })
        );
        if !is_entry {
            continue;
        }
        if has_entry {
            report.push('\n');
        }
        match record.event.as_data() {
            bityzba::data!(ProtocolEvent::MessagePosted {
                speaker,
                message,
                ..
            }) => {
                writeln!(report, "**{speaker}:** {}", message.text)
                    .expect("writing to String cannot fail");
            }
            bityzba::data!(ProtocolEvent::TurnForfeited {
                turn_number,
                speaker,
                ..
            }) => {
                writeln!(report, "*({speaker} forfeited turn {turn_number})*")
                    .expect("writing to String cannot fail");
            }
            bityzba::data!(ProtocolEvent::AnswerSubmitted { participant, .. }) => {
                writeln!(report, "*({participant} submitted an answer)*")
                    .expect("writing to String cannot fail");
            }
            bityzba::data!(ProtocolEvent::CheckerOutcome { outcome, .. }) => {
                writeln!(report, "*(checker: {})*", task_status_name(outcome.status))
                    .expect("writing to String cannot fail");
            }
            bityzba::data!(ProtocolEvent::RunAborted { record }) => {
                writeln!(report, "*(run aborted: {})*", abort_reason(record.kind))
                    .expect("writing to String cannot fail");
            }
            _ => unreachable!("dialog entry kinds were filtered above"),
        }
        has_entry = true;
    }
}

/// Render a human-review document from an already validated event sequence.
#[requires(!records.is_empty())]
#[ensures(!ret.is_empty())]
pub(crate) fn render_report(records: &[TranscriptRecord]) -> String {
    let mut report = String::from("# xarsnu run report\n\n");
    let mut summary = ReportSummary::default();

    for record in records {
        match record.event.as_data() {
            bityzba::data!(ProtocolEvent::RunStarted { header }) => {
                writeln!(report, "- Transcript schema: {}", header.schema_version)
                    .expect("writing to String cannot fail");
                writeln!(report, "- Scenario reference: `{}`", header.config.scenario)
                    .expect("writing to String cannot fail");
                writeln!(
                    report,
                    "- Gate format: `{}`",
                    tersmu_format_name(header.config.tersmu_format)
                )
                .expect("writing to String cannot fail");
                report.push_str("- Models:\n");
                for participant in &header.config.participants {
                    writeln!(
                        report,
                        "  - `{}`: `{}` (temperature {})",
                        participant.name, participant.model, participant.temperature
                    )
                    .expect("writing to String cannot fail");
                    summary
                        .usage_by_participant
                        .entry(participant.name.clone())
                        .or_default();
                }
                render_dialog_section(&mut report, records);
                report.push_str(
                    "\n<details><summary>Scenario instance snapshot</summary>\n\n```toml\n",
                );
                report.push_str(&header.scenario_instance_toml);
                if !header.scenario_instance_toml.ends_with('\n') {
                    report.push('\n');
                }
                report.push_str("```\n\n</details>\n\n");
            }
            bityzba::data!(ProtocolEvent::TurnStarted {
                turn_number,
                speaker,
            }) => {
                writeln!(report, "## Turn {turn_number} — `{speaker}`\n")
                    .expect("writing to String cannot fail");
            }
            bityzba::data!(ProtocolEvent::IntentRegistered {
                turn_number,
                speaker,
                meaning_en,
                revision,
                revision_number,
            }) => {
                writeln!(
                    report,
                    "### Intent{}\n\nParticipant: `{speaker}`\n",
                    if *revision { " revision" } else { "" }
                )
                .expect("writing to String cannot fail");
                quote(&mut report, meaning_en);
                report.push('\n');
                summary.intents.insert(*turn_number, meaning_en.clone());
                if *revision {
                    summary.intent_revisions += 1;
                    writeln!(report, "Revision number: {revision_number}\n")
                        .expect("writing to String cannot fail");
                }
            }
            bityzba::data!(ProtocolEvent::CandidateSubmitted {
                turn_number,
                speaker,
                text,
                attempt,
            }) => {
                writeln!(
                    report,
                    "### Parse attempt {attempt}\n\nParticipant: `{speaker}`\n"
                )
                .expect("writing to String cannot fail");
                quote(&mut report, text);
                report.push('\n');
                *summary.parse_attempts.entry(*turn_number).or_default() += 1;
            }
            bityzba::data!(ProtocolEvent::CandidateRejected {
                diagnostic_category,
                diagnostics,
                ..
            }) => {
                writeln!(
                    report,
                    "**Gate result:** rejected ({})\n\nDiagnostics (verbatim):\n",
                    diagnostic_category_name(*diagnostic_category)
                )
                .expect("writing to String cannot fail");
                quote(&mut report, diagnostics);
                report.push('\n');
                *summary
                    .diagnostic_categories
                    .entry(*diagnostic_category)
                    .or_default() += 1;
            }
            bityzba::data!(ProtocolEvent::CandidateAccepted { message, .. }) => {
                report.push_str("**Gate result:** accepted\n\ntersmu rendering (verbatim):\n\n");
                quote_bytes(&mut report, &message.tersmu_rendering);
                report.push('\n');
            }
            bityzba::data!(ProtocolEvent::MeaningConfirmed {
                matches,
                paraphrase_en,
                discrepancies,
                ..
            }) => {
                writeln!(
                    report,
                    "### Sender confirmation\n\nVerdict: **{}**\n\nParaphrase:\n",
                    if *matches { "match" } else { "mismatch" }
                )
                .expect("writing to String cannot fail");
                quote(&mut report, paraphrase_en);
                if let Some(discrepancies) = discrepancies {
                    report.push_str("\nDiscrepancies:\n\n");
                    quote(&mut report, discrepancies);
                }
                report.push('\n');
                if !matches {
                    summary.confirm_mismatches += 1;
                }
            }
            bityzba::data!(ProtocolEvent::MessagePosted { message, .. }) => {
                report.push_str("### Posted message\n\nLojban:\n\n");
                quote(&mut report, &message.text);
                report.push_str("\ntersmu rendering:\n\n");
                quote_bytes(&mut report, &message.tersmu_rendering);
                report.push('\n');
            }
            bityzba::data!(ProtocolEvent::BlindInterpretationRecorded {
                turn_number,
                listener,
                interpretation_en,
                ..
            }) => {
                writeln!(report, "### Blind interpretation — `{listener}`\n")
                    .expect("writing to String cannot fail");
                quote(&mut report, interpretation_en);
                report.push('\n');
                summary.blind_turns.insert(*turn_number);
            }
            bityzba::data!(ProtocolEvent::TersmuRevealed {
                listener,
                message,
                ..
            }) => {
                writeln!(report, "### tersmu revealed to `{listener}`\n")
                    .expect("writing to String cannot fail");
                quote_bytes(&mut report, &message.tersmu_rendering);
                report.push('\n');
            }
            bityzba::data!(ProtocolEvent::Acknowledged {
                turn_number,
                listener,
                final_understanding_en,
                discrepancies,
                ..
            }) => {
                writeln!(
                    report,
                    "### Acknowledgment — `{listener}`\n\nFinal understanding:\n"
                )
                .expect("writing to String cannot fail");
                quote(&mut report, final_understanding_en);
                if let Some(discrepancies) = discrepancies {
                    report.push_str("\nRecorded discrepancies:\n\n");
                    quote(&mut report, discrepancies);
                    summary.discrepancy_acknowledgments.push((
                        *turn_number,
                        listener.clone(),
                        discrepancies.to_owned(),
                    ));
                }
                report.push('\n');
            }
            bityzba::data!(ProtocolEvent::ReferenceToolCompleted {
                participant,
                tool_name,
                arguments,
                result,
                succeeded,
                ..
            }) => {
                writeln!(
                    report,
                    "### Reference tool `{tool_name}` — `{participant}`\n\nStatus: **{}**\n\nArguments:\n",
                    if *succeeded { "success" } else { "failure" }
                )
                .expect("writing to String cannot fail");
                quote(&mut report, arguments);
                report.push_str("\nResult:\n\n");
                quote(&mut report, result);
                report.push('\n');
            }
            bityzba::data!(ProtocolEvent::ReferenceLookupRepeated {
                participant,
                tool_name,
                repeat_number,
                remaining_calls,
                ..
            }) => {
                writeln!(
                    report,
                    "### Repeated reference lookup — `{participant}` / `{tool_name}`\n\nExact-query occurrence: **{repeat_number}**; reference calls remaining in phase: **{remaining_calls}**.\n"
                )
                .expect("writing to String cannot fail");
                summary.reference_repeats += 1;
            }
            bityzba::data!(ProtocolEvent::ReferenceCallBudgetExhausted {
                participant,
                maximum,
                ..
            }) => {
                writeln!(
                    report,
                    "### Reference-call budget exhausted — `{participant}`\n\nPhase maximum: **{maximum}**; reference tools withdrawn.\n"
                )
                .expect("writing to String cannot fail");
                summary.reference_budgets_exhausted += 1;
            }
            bityzba::data!(ProtocolEvent::ReferenceResearchNudge {
                participant,
                consecutive_calls,
                message,
                ..
            }) => {
                writeln!(
                    report,
                    "### Reference-research nudge — `{participant}`\n\nConsecutive reference calls: **{consecutive_calls}**\n\nCorrection:\n"
                )
                .expect("writing to String cannot fail");
                quote(&mut report, message);
                report.push('\n');
                summary.reference_nudges += 1;
            }
            bityzba::data!(ProtocolEvent::ProseRejected {
                participant,
                attempt,
                maximum_attempts,
                ..
            }) => {
                writeln!(
                    report,
                    "### Auto-mode prose rejected — `{participant}`\n\nProtocol action attempt **{attempt}** of **{maximum_attempts}** returned prose instead of a tool call.\n"
                )
                .expect("writing to String cannot fail");
                summary.prose_rejections += 1;
            }
            bityzba::data!(ProtocolEvent::ListenerFlowAbandoned {
                listener,
                reason,
                ..
            }) => {
                writeln!(
                    report,
                    "### Listener flow abandoned — `{listener}`\n\nReason: {}\n",
                    listener_abandon_reason(reason)
                )
                .expect("writing to String cannot fail");
                summary.listener_flows_abandoned += 1;
            }
            bityzba::data!(ProtocolEvent::ProtocolError {
                participant,
                tool_name,
                message,
                ..
            }) => {
                writeln!(
                    report,
                    "### Protocol error — `{participant}` / `{tool_name}`\n"
                )
                .expect("writing to String cannot fail");
                quote(&mut report, message);
                report.push('\n');
                summary.protocol_errors += 1;
            }
            bityzba::data!(ProtocolEvent::TurnForfeited {
                speaker,
                reason,
                ..
            }) => {
                writeln!(
                    report,
                    "### Turn forfeited — `{speaker}`\n\nReason: {}\n",
                    forfeit_reason(reason)
                )
                .expect("writing to String cannot fail");
                summary.forfeits += 1;
            }
            bityzba::data!(ProtocolEvent::AnswerSubmitted {
                participant,
                answer,
                ..
            }) => {
                writeln!(report, "### Scenario answer — `{participant}`\n\n```json")
                    .expect("writing to String cannot fail");
                report.push_str(
                    &serde_json::to_string_pretty(answer)
                        .expect("validated scenario answers serialize"),
                );
                report.push_str("\n```\n\n");
            }
            bityzba::data!(ProtocolEvent::CheckerOutcome { outcome, .. }) => {
                writeln!(
                    report,
                    "### Scenario checker\n\nAggregate status: **{}**\n",
                    task_status_name(outcome.status)
                )
                .expect("writing to String cannot fail");
                summary.task_status = Some(outcome.status);
                summary.task_outcomes.clear();
                for participant in &outcome.participants {
                    summary
                        .task_outcomes
                        .insert(participant.participant.clone(), participant.correct);
                }
            }
            bityzba::data!(ProtocolEvent::ThinkingRecorded {
                participant,
                trace,
                ..
            }) => {
                writeln!(report, "### Thinking — `{participant}`\n")
                    .expect("writing to String cannot fail");
                if let Some(reasoning) = &trace.reasoning {
                    quote(&mut report, reasoning);
                }
                if let Some(reasoning_details) = &trace.reasoning_details {
                    if trace.reasoning.is_some() {
                        report.push('\n');
                    }
                    report.push_str("reasoning_details (verbatim JSON):\n\n");
                    let serialized = serde_json::to_string_pretty(reasoning_details)
                        .expect("provider reasoning details serialize to JSON");
                    quote(&mut report, &serialized);
                }
                report.push('\n');
            }
            bityzba::data!(ProtocolEvent::UsageRecorded {
                participant,
                usage,
                ..
            }) => {
                writeln!(
                    report,
                    "### API usage — `{participant}`\n\n{} prompt + {} completion = {} tokens; ${:.6}; {} cached, {} cache-write tokens\n",
                    usage.prompt_tokens,
                    usage.completion_tokens,
                    usage.total_tokens,
                    usage.cost,
                    usage.cached_tokens.unwrap_or(0),
                    usage.cache_write_tokens.unwrap_or(0),
                )
                .expect("writing to String cannot fail");
                if usage.reasoning_present || usage.reasoning_tokens.is_some() {
                    writeln!(
                        report,
                        "Reasoning field present: {}; reasoning tokens: {}\n",
                        usage.reasoning_present,
                        usage.reasoning_tokens.map_or_else(
                            || "not reported".to_owned(),
                            |tokens| tokens.to_string(),
                        ),
                    )
                    .expect("writing to String cannot fail");
                }
                summary
                    .usage_by_participant
                    .entry(participant.clone())
                    .or_default()
                    .record(usage);
                summary.run_usage.record(usage);
            }
            bityzba::data!(ProtocolEvent::RunAborted { record }) => {
                writeln!(
                    report,
                    "### Run aborted\n\nCost budget ${:.6}; actual cost ${:.6}.\n",
                    record.max_cost_usd, record.actual_cost_usd
                )
                .expect("writing to String cannot fail");
                summary.aborts += 1;
            }
            bityzba::data!(ProtocolEvent::RunFinished { outcome }) => {
                writeln!(
                    report,
                    "## Run finished\n\nOutcome: **{}** after {} turn(s).\n",
                    run_outcome_name(outcome),
                    outcome.turns()
                )
                .expect("writing to String cannot fail");
            }
            bityzba::data!(ProtocolEvent::RunFailed { failure }) => {
                writeln!(
                    report,
                    "## Runtime failure\n\nCall site: **{}**\n\nTurn: {}\n",
                    failure.call_site.as_str(),
                    failure.turn_number,
                )
                .expect("writing to String cannot fail");
                if let Some(participant) = &failure.participant {
                    writeln!(report, "Participant: `{participant}`\n")
                        .expect("writing to String cannot fail");
                }
                report.push_str("Error:\n\n");
                quote(&mut report, &failure.message);
                report.push('\n');
                summary.runtime_failures += 1;
            }
        }
    }

    render_summary(&mut report, &summary);
    report
}

#[invariant(true, "all counters and totals start at zero and only increase")]
#[derive(Debug, Default)]
struct ReportSummary {
    parse_attempts: BTreeMap<usize, usize>,
    diagnostic_categories: BTreeMap<DiagnosticCategory, usize>,
    intent_revisions: usize,
    confirm_mismatches: usize,
    intents: BTreeMap<usize, String>,
    blind_turns: BTreeSet<usize>,
    discrepancy_acknowledgments: Vec<(usize, String, String)>,
    reference_repeats: usize,
    reference_budgets_exhausted: usize,
    reference_nudges: usize,
    prose_rejections: usize,
    listener_flows_abandoned: usize,
    protocol_errors: usize,
    forfeits: usize,
    aborts: usize,
    runtime_failures: usize,
    task_status: Option<TaskStatus>,
    task_outcomes: BTreeMap<String, Option<bool>>,
    usage_by_participant: BTreeMap<String, UsageTotals>,
    run_usage: UsageTotals,
}

#[requires(true)]
#[ensures(report.contains("## Summary"))]
fn render_summary(report: &mut String, summary: &ReportSummary) {
    report.push_str("## Summary\n\n### Task outcomes\n\n");
    if let Some(status) = summary.task_status {
        writeln!(report, "Aggregate: **{}**", task_status_name(status))
            .expect("writing to String cannot fail");
    } else {
        report.push_str("Aggregate: **not recorded**\n");
    }
    for (participant, correct) in &summary.task_outcomes {
        let verdict = match correct {
            Some(true) => "correct",
            Some(false) => "incorrect",
            None => "not required",
        };
        writeln!(report, "- `{participant}`: {verdict}").expect("writing to String cannot fail");
    }

    report.push_str("\n### Parse attempts\n\n");
    for (turn, attempts) in &summary.parse_attempts {
        writeln!(report, "- Turn {turn}: {attempts}").expect("writing to String cannot fail");
    }
    for category in [
        DiagnosticCategory::Morphology,
        DiagnosticCategory::Syntax,
        DiagnosticCategory::Other,
    ] {
        writeln!(
            report,
            "- {} failures: {}",
            diagnostic_category_name(category),
            summary
                .diagnostic_categories
                .get(&category)
                .copied()
                .unwrap_or(0)
        )
        .expect("writing to String cannot fail");
    }

    writeln!(
        report,
        "\n### Revisions and mismatches\n\n- Intent revisions: {}\n- Confirmation mismatches: {}",
        summary.intent_revisions, summary.confirm_mismatches
    )
    .expect("writing to String cannot fail");

    writeln!(
        report,
        "\n### Reference-loop mitigations\n\n- Memoized repeats: {}\n- Phase budgets exhausted: {}\n- Idle-research nudges: {}",
        summary.reference_repeats,
        summary.reference_budgets_exhausted,
        summary.reference_nudges
    )
    .expect("writing to String cannot fail");

    report.push_str("\n### Divergence flags\n\n");
    let intent_turns = summary.intents.keys().copied().collect::<BTreeSet<_>>();
    for turn in summary.blind_turns.intersection(&intent_turns) {
        writeln!(
            report,
            "- Turn {turn}: sender intent and blind interpretation are both recorded; review them side by side."
        )
        .expect("writing to String cannot fail");
    }
    for (turn, listener, discrepancies) in &summary.discrepancy_acknowledgments {
        writeln!(
            report,
            "- Turn {turn}: `{listener}` acknowledged with discrepancies: {discrepancies}"
        )
        .expect("writing to String cannot fail");
    }
    if summary.blind_turns.is_empty() && summary.discrepancy_acknowledgments.is_empty() {
        report.push_str("- None recorded.\n");
    }

    writeln!(
        report,
        "\n### Protocol stops\n\n- Auto-mode prose rejections: {}\n- Listener flows abandoned: {}\n- Protocol errors: {}\n- Forfeits: {}\n- Budget aborts: {}\n- Runtime failures: {}",
        summary.prose_rejections,
        summary.listener_flows_abandoned,
        summary.protocol_errors,
        summary.forfeits,
        summary.aborts,
        summary.runtime_failures
    )
    .expect("writing to String cannot fail");

    report.push_str("\n### Usage\n\n");
    for (participant, usage) in &summary.usage_by_participant {
        writeln!(
            report,
            "- `{participant}`: {} prompt + {} completion = {} tokens; ${:.6}",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens, usage.cost_usd
        )
        .expect("writing to String cannot fail");
        render_cache_observability(report, usage);
    }
    writeln!(
        report,
        "- Run total: {} prompt + {} completion = {} tokens; ${:.6}",
        summary.run_usage.prompt_tokens,
        summary.run_usage.completion_tokens,
        summary.run_usage.total_tokens,
        summary.run_usage.cost_usd
    )
    .expect("writing to String cannot fail");
    render_cache_observability(report, &summary.run_usage);
}

#[requires(true)]
#[ensures(report.contains("Cache efficiency:") && report.contains("Call hit rate:"))]
fn render_cache_observability(report: &mut String, usage: &UsageTotals) {
    writeln!(
        report,
        "  - Cache totals: {} cached tokens; {} cache-write tokens",
        usage.cached_tokens, usage.cache_write_tokens
    )
    .expect("writing to String cannot fail");
    writeln!(
        report,
        "  - Cache efficiency: {} ({} / {} prompt tokens)",
        percentage(usage.cache_efficiency()),
        usage.cached_tokens,
        usage.prompt_tokens,
    )
    .expect("writing to String cannot fail");
    writeln!(
        report,
        "  - Call hit rate: {} ({} / {} provider calls)",
        percentage(usage.cache_hit_rate()),
        usage.cache_hit_calls,
        usage.provider_calls,
    )
    .expect("writing to String cannot fail");
    if usage.reasoning_calls > 0 || usage.reasoning_tokens > 0 {
        writeln!(
            report,
            "  - Reasoning totals: {} tokens across {} provider calls",
            usage.reasoning_tokens, usage.reasoning_calls,
        )
        .expect("writing to String cannot fail");
    }
}

#[requires(rate.is_none_or(|value| value.is_finite() && value >= 0.0))]
#[ensures(!ret.is_empty())]
fn percentage(rate: Option<f64>) -> String {
    rate.map_or_else(
        || "n/a".to_owned(),
        |value| format!("{:.2}%", value * 100.0),
    )
}

#[requires(true)]
#[ensures(!report.is_empty())]
fn quote(report: &mut String, text: &str) {
    for line in text.lines() {
        writeln!(report, "> {line}").expect("writing to String cannot fail");
    }
    if text.is_empty() {
        report.push_str(">\n");
    }
}

#[requires(true)]
#[ensures(!report.is_empty())]
fn quote_bytes(report: &mut String, bytes: &[u8]) {
    quote(report, &String::from_utf8_lossy(bytes));
}

#[requires(true)]
#[ensures(!ret.is_empty())]
const fn diagnostic_category_name(category: DiagnosticCategory) -> &'static str {
    match category {
        DiagnosticCategory::Morphology => "morphology",
        DiagnosticCategory::Syntax => "syntax",
        DiagnosticCategory::Other => "other",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
const fn task_status_name(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Success => "success",
        TaskStatus::Partial => "partial",
        TaskStatus::Failure => "failure",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
const fn abort_reason(kind: AbortKind) -> &'static str {
    match kind {
        AbortKind::CostBudgetExceeded => "cost budget exceeded",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
const fn tersmu_format_name(format: crate::TersmuFormat) -> &'static str {
    match format {
        crate::TersmuFormat::TreeProj => "tree+proj",
        crate::TersmuFormat::Tree => "tree",
        crate::TersmuFormat::Json => "json",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn run_outcome_name(outcome: &ProtocolRunOutcome) -> &'static str {
    match outcome.as_data() {
        bityzba::data!(ProtocolRunOutcome::Completed { .. }) => "completed",
        bityzba::data!(ProtocolRunOutcome::ScenarioCompleted { .. }) => "scenario completed",
        bityzba::data!(ProtocolRunOutcome::BudgetAborted { .. }) => "budget aborted",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn forfeit_reason(reason: &TurnForfeitReason) -> String {
    match reason.as_data() {
        bityzba::data!(TurnForfeitReason::ParseAttempts { maximum }) => {
            format!("parse-attempt cap ({maximum})")
        }
        bityzba::data!(TurnForfeitReason::IntentRevisions { maximum }) => {
            format!("intent-revision cap ({maximum})")
        }
        bityzba::data!(TurnForfeitReason::ProtocolProseResponses { maximum_attempts }) => {
            format!("automatic tool-call attempts exhausted ({maximum_attempts})")
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn listener_abandon_reason(reason: &ListenerFlowAbandonReason) -> String {
    match reason.as_data() {
        bityzba::data!(ListenerFlowAbandonReason::ProtocolProseResponses { maximum_attempts }) => {
            format!("automatic tool-call attempts exhausted ({maximum_attempts})")
        }
    }
}
