//! Deterministic offline rendering of validated transcript records.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

use crate::protocol::{ProtocolEventData, ProtocolRunOutcomeData, TurnForfeitReasonData};
use crate::{
    DiagnosticCategory, ProtocolRunOutcome, TaskStatus, TranscriptError, TranscriptRecord,
    TurnForfeitReason, UsageTotals, read_transcript,
};

/// Read, validate, and render one transcript without consulting any external service.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|report| !report.is_empty()) || ret.is_err())]
pub fn report_file(path: &Path) -> Result<String, TranscriptError> {
    let records = read_transcript(path)?;
    Ok(render_report(&records))
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
            bityzba::data!(ProtocolEvent::UsageRecorded {
                participant,
                usage,
                ..
            }) => {
                writeln!(
                    report,
                    "### API usage — `{participant}`\n\n{} prompt + {} completion = {} tokens; ${:.6}\n",
                    usage.prompt_tokens,
                    usage.completion_tokens,
                    usage.total_tokens,
                    usage.cost
                )
                .expect("writing to String cannot fail");
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
    protocol_errors: usize,
    forfeits: usize,
    aborts: usize,
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
        "\n### Protocol stops\n\n- Protocol errors: {}\n- Forfeits: {}\n- Budget aborts: {}",
        summary.protocol_errors, summary.forfeits, summary.aborts
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
    }
}
