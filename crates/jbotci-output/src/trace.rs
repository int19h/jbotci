use bityzba::{invariant, requires};
use jbotci_diagnostics::{
    TraceEvent, TraceEventKind, TraceFailureBranch, TraceFailureSummary, TracePhase, TraceReport,
};
use owo_colors::OwoColorize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct TraceRenderOptions {
    pub color: bool,
    pub terminal_width: usize,
}

#[requires(options.terminal_width > 0)]
#[ensures(ret.is_empty() || ret.ends_with('\n'))]
pub fn render_trace_report(report: &TraceReport, options: TraceRenderOptions) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "{}\n",
        style_header(
            &format!("trace[{}]", trace_phase_name(report.phase)),
            options.color
        )
    ));
    for event in &report.events {
        push_trace_event(&mut output, event, options.color);
    }
    if report.truncated {
        output.push_str(&format!(
            "  {}\n",
            style_punctuation("... trace limit reached; events truncated", options.color)
        ));
    }
    if let Some(failure) = &report.failure {
        push_failure_summary(&mut output, failure, options.color);
    }
    output
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn trace_phase_name(phase: TracePhase) -> &'static str {
    match phase {
        TracePhase::Morphology => "morphology",
        TracePhase::Syntax => "syntax",
        TracePhase::All => "all",
    }
}

#[requires(true)]
#[ensures(true)]
fn push_trace_event(output: &mut String, event: &TraceEvent, color: bool) {
    let indent = "  ".repeat(event.depth + 1);
    output.push_str(&indent);
    output.push_str(&style_marker(
        trace_event_marker(event.kind),
        event.kind,
        color,
    ));
    output.push(' ');
    output.push_str(&style_construct(&event.label, color));
    output.push_str(&style_punctuation(" @ ", color));
    output.push_str(&style_span(event.byte_start, event.byte_end, color));
    if let Some(detail) = &event.detail {
        output.push_str(&style_punctuation(": ", color));
        output.push_str(detail);
    }
    output.push('\n');
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn trace_event_marker(kind: TraceEventKind) -> &'static str {
    match kind {
        TraceEventKind::ConstructEnter => "->",
        TraceEventKind::ConstructSuccess | TraceEventKind::TerminalSuccess => "ok",
        TraceEventKind::ConstructFailure
        | TraceEventKind::TerminalFailure
        | TraceEventKind::MorphologyFailure => "!!",
        TraceEventKind::TerminalAttempt => "??",
        TraceEventKind::Token => "tok",
        TraceEventKind::Save => "save",
        TraceEventKind::Rewind => "<-",
        TraceEventKind::MorphologyStep => "--",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn style_marker(marker: &str, kind: TraceEventKind, color: bool) -> String {
    if !color {
        return marker.to_owned();
    }
    match kind {
        TraceEventKind::ConstructSuccess | TraceEventKind::TerminalSuccess => {
            marker.green().to_string()
        }
        TraceEventKind::ConstructFailure
        | TraceEventKind::TerminalFailure
        | TraceEventKind::MorphologyFailure => marker.red().to_string(),
        TraceEventKind::Rewind | TraceEventKind::Save => marker.yellow().to_string(),
        _ => marker.bright_black().to_string(),
    }
}

#[requires(true)]
#[ensures(true)]
fn push_failure_summary(output: &mut String, failure: &TraceFailureSummary, color: bool) {
    output.push_str("  ");
    output.push_str(&style_header("farthest failure", color));
    output.push_str(&style_punctuation(" @ ", color));
    output.push_str(&style_span(failure.byte_start, failure.byte_end, color));
    output.push_str(&style_punctuation(": ", color));
    output.push_str(&failure.reason);
    output.push('\n');
    if let Some(context) = &failure.current_context {
        output.push_str("    ");
        output.push_str(&style_keyword("current context", color));
        output.push_str(&style_punctuation(": ", color));
        output.push_str(&style_construct(&context.construct, color));
        output.push_str(&style_punctuation(" @ ", color));
        output.push_str(&style_span(context.byte_start, context.byte_end, color));
        output.push('\n');
    }
    if !failure.branches.is_empty() {
        output.push_str("    ");
        output.push_str(&style_keyword("branches", color));
        output.push_str(&style_punctuation(":", color));
        output.push('\n');
        for branch in &failure.branches {
            push_failure_branch(output, branch, color);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn push_failure_branch(output: &mut String, branch: &TraceFailureBranch, color: bool) {
    output.push_str("      - ");
    if branch.contexts.is_empty() {
        output.push_str(&style_punctuation("<no context>", color));
    } else {
        for (index, context) in branch.contexts.iter().enumerate() {
            if index > 0 {
                output.push_str(&style_punctuation(" > ", color));
            }
            output.push_str(&style_construct(&context.construct, color));
            output.push_str(&style_punctuation(" @ ", color));
            output.push_str(&style_span(context.byte_start, context.byte_end, color));
        }
    }
    if !branch.expected.is_empty() {
        output.push_str(&style_punctuation("; ", color));
        output.push_str(&style_keyword("expected", color));
        output.push_str(&style_punctuation(": ", color));
        output.push_str(&branch.expected.join(", "));
    }
    output.push('\n');
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn style_header(text: &str, color: bool) -> String {
    if color {
        text.bright_white().bold().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn style_construct(text: &str, color: bool) -> String {
    if color {
        text.bright_white().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn style_keyword(text: &str, color: bool) -> String {
    if color {
        text.truecolor(170, 170, 170).to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn style_punctuation(text: &str, color: bool) -> String {
    if color {
        text.bright_black().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(start <= end)]
#[ensures(!ret.is_empty())]
fn style_span(start: usize, end: usize, color: bool) -> String {
    let span = format!("{start}..{end}");
    if color {
        span.bright_black().to_string()
    } else {
        span
    }
}
