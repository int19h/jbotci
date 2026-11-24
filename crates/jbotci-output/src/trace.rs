use bityzba::{invariant, requires};
use jbotci_diagnostics::{
    TraceEvent, TraceEventKind, TraceFailureBranch, TraceFailureSummary, TracePhase, TraceReport,
};
use owo_colors::OwoColorize;

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceRenderOptions {
    pub color: bool,
}

#[requires(true)]
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
        output.push_str(&crate::styled::escape_terminal_controls(detail));
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
    output.push_str(&crate::styled::escape_terminal_controls(&failure.reason));
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
        output.push_str(&crate::styled::escape_terminal_controls(
            &branch.expected.join(", "),
        ));
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
    let text = crate::styled::escape_terminal_controls(text);
    if color {
        text.as_ref().bright_white().to_string()
    } else {
        text.into_owned()
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

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::new;
    use jbotci_diagnostics::{
        TraceContext, TraceEvent, TraceEventKind, TraceFailureBranch, TraceFailureSummary,
        TraceLevel, TracePhase, TraceReport,
    };

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn trace_text_escapes_controls_in_both_modes() {
        let report = new!(TraceReport {
            phase: TracePhase::Syntax,
            events: vec![new!(TraceEvent {
                phase: TracePhase::Syntax,
                level: TraceLevel::Top,
                depth: 0,
                kind: TraceEventKind::Token,
                label: "a\u{1b}b".to_owned(),
                byte_start: 0,
                byte_end: 1,
                detail: Some("c\u{9f}".to_owned()),
            })],
            truncated: false,
            failure: Some(new!(TraceFailureSummary {
                byte_start: 0,
                byte_end: 1,
                reason: "d\t".to_owned(),
                branches: vec![TraceFailureBranch {
                    contexts: vec![TraceContext::new("e\u{1b}".to_owned(), 0, 1)],
                    expected: vec!["f\u{80}".to_owned()],
                }],
                current_context: None,
            })),
        });
        for color in [false, true] {
            let rendered = render_trace_report(&report, TraceRenderOptions { color });
            assert!(rendered.contains("a\\u{1b}b"), "{rendered:?}");
            assert!(rendered.contains("c\\u{9f}"), "{rendered:?}");
            assert!(rendered.contains("d\\u{9}"), "{rendered:?}");
            assert!(rendered.contains("e\\u{1b}"), "{rendered:?}");
            assert!(rendered.contains("f\\u{80}"), "{rendered:?}");
            assert!(!rendered.contains('\u{9f}'), "{rendered:?}");
            assert!(!rendered.contains('\u{80}'), "{rendered:?}");
            if !color {
                assert!(!rendered.contains('\u{1b}'), "{rendered:?}");
            }
        }
    }
}
