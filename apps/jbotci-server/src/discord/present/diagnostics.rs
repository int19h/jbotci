//! Compact Discord rendering of shared diagnostics.
//!
//! Each diagnostic shows its severity, code and message, the source line its
//! primary label points at with a caret underline in a code block, and its
//! notes. What the message shows is bounded so critical diagnostics always fit
//! the reserved budget, and the complete text is carried alongside it: the
//! assembler attaches that whole text whenever the shown form is an excerpt,
//! so nothing a diagnostic said is lost.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_diagnostics::{Diagnostic, DiagnosticSeverity};

use super::markdown::{
    code_block_in_scope, escape_in_scope, inline_code_in_scope, join_lines_in_scope,
    subtext_in_scope, truncate_units_in_scope,
};
use crate::discord::request::utf16_len;

/// Diagnostics shown in full before the rest is summarized.
pub(crate) const MAX_DETAILED_DIAGNOSTICS: usize = 4;
/// Longest source excerpt line in a caret block.
const MAX_EXCERPT_UNITS: usize = 96;
/// Longest single diagnostic message or note.
const MAX_MESSAGE_UNITS: usize = 300;
/// Notes shown per diagnostic.
const MAX_SHOWN_NOTES: usize = 3;

#[requires(true)]
#[ensures(!ret.is_empty())]
fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Advice => "advice",
    }
}

/// Diagnostics as the message shows them and as they read in full.
#[invariant(!shown.is_empty() && !complete.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderedDiagnostics {
    /// Discord Markdown, bounded to the message's reserved share.
    pub(crate) shown: String,
    /// Every diagnostic in full, as plain text for the attached result.
    pub(crate) complete: String,
    /// Whether `shown` leaves anything out of `complete`.
    pub(crate) is_excerpt: bool,
}

/// Render `diagnostics` against `source`. Returns `None` when there are none.
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (ret.is_some() == !diagnostics.is_empty()))]
pub(crate) fn render_diagnostics_in_scope(
    source: &str,
    diagnostics: &[Diagnostic],
    text_scope: &jbotci_output::budget::TextScope,
) -> Option<RenderedDiagnostics> {
    if diagnostics.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    let mut is_excerpt = false;
    for diagnostic in diagnostics.iter().take(MAX_DETAILED_DIAGNOSTICS) {
        let (rendered, cut) = render_one_in_scope(source, diagnostic, text_scope);
        is_excerpt |= cut;
        lines.push(rendered);
    }
    let remaining = diagnostics.len().saturating_sub(MAX_DETAILED_DIAGNOSTICS);
    if remaining > 0 {
        is_excerpt = true;
        let errors = diagnostics
            .iter()
            .skip(MAX_DETAILED_DIAGNOSTICS)
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
            .count();
        lines.push(subtext_in_scope(
            &text_scope.format(format_args!(
                "{remaining} more diagnostic{} not shown ({errors} error{})",
                if remaining == 1 { "" } else { "s" },
                if errors == 1 { "" } else { "s" }
            )),
            text_scope,
        ));
    }
    let shown = join_lines_in_scope(lines, text_scope);
    let complete = complete_text_in_scope(source, diagnostics, text_scope);
    if text_scope.is_exceeded() {
        return None;
    }
    Some(new!(RenderedDiagnostics {
        shown,
        complete,
        is_excerpt
    }))
}

/// Every diagnostic in full: message, every label with its own source line
/// and caret, and every note, none of them shortened.
#[requires(!diagnostics.is_empty())]
#[ensures(text_scope.is_exceeded() || (!ret.is_empty()))]
fn complete_text_in_scope(
    source: &str,
    diagnostics: &[Diagnostic],
    text_scope: &jbotci_output::budget::TextScope,
) -> String {
    let mut blocks = text_scope.buffer();
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if text_scope.is_exceeded() {
            break;
        }
        if index != 0 {
            blocks.push_str("\n\n");
        }
        let mut lines = vec![text_scope.format(format_args!(
            "{} {}: {}",
            severity_label(diagnostic.severity),
            diagnostic.code,
            diagnostic.message
        ))];
        for label in &diagnostic.labels {
            if let Some((excerpt, _)) = caret_excerpt_in_scope(
                source,
                label.span.byte_start,
                label.span.byte_end,
                &label.message,
                ExcerptLimits::complete(),
                text_scope,
            ) {
                lines.push(excerpt);
            } else if !label.message.is_empty() {
                lines.push(text_scope.copy(&label.message));
            }
        }
        for note in &diagnostic.notes {
            lines.push(text_scope.format(format_args!("note: {note}")));
        }
        blocks.push_str(&text_scope.join(&lines, "\n"));
    }
    blocks.into_piece()
}

/// Counts by severity, for status lines ("2 errors, 1 warning").
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (ret.is_some() == !diagnostics.is_empty()))]
pub(crate) fn summary_in_scope(
    diagnostics: &[Diagnostic],
    text_scope: &jbotci_output::budget::TextScope,
) -> Option<String> {
    if diagnostics.is_empty() {
        return None;
    }
    let count = |severity: DiagnosticSeverity| {
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == severity)
            .count()
    };
    let mut parts = Vec::new();
    for (severity, singular, plural) in [
        (DiagnosticSeverity::Error, "error", "errors"),
        (DiagnosticSeverity::Warning, "warning", "warnings"),
        (DiagnosticSeverity::Advice, "note", "notes"),
    ] {
        let total = count(severity);
        if total > 0 {
            parts.push(text_scope.format(format_args!(
                "{total} {}",
                if total == 1 { singular } else { plural }
            )));
        }
    }
    Some(text_scope.join(&parts, ", "))
}

/// One diagnostic as the message shows it, and whether that left anything out.
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (!ret.0.is_empty()))]
fn render_one_in_scope(
    source: &str,
    diagnostic: &Diagnostic,
    text_scope: &jbotci_output::budget::TextScope,
) -> (String, bool) {
    let (message, mut cut) =
        truncate_units_in_scope(&diagnostic.message, MAX_MESSAGE_UNITS, text_scope);
    cut |= diagnostic.notes.len() > MAX_SHOWN_NOTES;
    let mut lines = vec![text_scope.format(format_args!(
        "**{}** {}: {}",
        severity_label(diagnostic.severity),
        inline_code_in_scope(&diagnostic.code, text_scope),
        escape_in_scope(&message, text_scope)
    ))];
    // Every label is shown, the primary one first: a secondary label says
    // where the error came from, and dropping it would make the message an
    // excerpt of something the reader can already read here.
    let mut labels = diagnostic.labels.iter().collect::<Vec<_>>();
    labels.sort_by_key(|label| !label.primary);
    for label in labels {
        if let Some((excerpt, excerpt_cut)) = caret_excerpt_in_scope(
            source,
            label.span.byte_start,
            label.span.byte_end,
            &label.message,
            ExcerptLimits::shown(),
            text_scope,
        ) {
            cut |= excerpt_cut;
            lines.push(code_block_in_scope(&excerpt, text_scope));
        }
    }
    for note in diagnostic.notes.iter().take(MAX_SHOWN_NOTES) {
        let (note, note_cut) = truncate_units_in_scope(note, MAX_MESSAGE_UNITS, text_scope);
        cut |= note_cut;
        lines.push(subtext_in_scope(&note.replace('\n', " "), text_scope));
    }
    (join_lines_in_scope(lines, text_scope), cut)
}

/// The source line containing `byte_start`, with a caret underline for the
/// labelled range and the label text. `None` if the offsets do not fall on
/// character boundaries of `source`.
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (true))]
fn caret_excerpt_in_scope(
    source: &str,
    byte_start: usize,
    byte_end: usize,
    label: &str,
    limits: ExcerptLimits,
    text_scope: &jbotci_output::budget::TextScope,
) -> Option<(String, bool)> {
    if byte_start > source.len() || byte_end > source.len() || byte_start > byte_end {
        return None;
    }
    if !source.is_char_boundary(byte_start) || !source.is_char_boundary(byte_end) {
        return None;
    }
    let line_start = source[..byte_start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let line_end = source[byte_start..]
        .find('\n')
        .map_or(source.len(), |index| byte_start + index);
    let line = &source[line_start..line_end];
    let prefix_units = utf16_len(&source[line_start..byte_start]);
    let range_end = byte_end.min(line_end);
    let range_units = utf16_len(&source[byte_start..range_end]).max(1);
    // Keep the excerpt short: window around the labelled range.
    let mut cut = false;
    let (shown_line, shown_prefix) = if utf16_len(line) > limits.line_units {
        cut = true;
        let window_start_units = prefix_units.saturating_sub(limits.line_units / 3);
        let mut units = 0;
        let mut start_byte = 0;
        for (index, character) in line.char_indices() {
            if units >= window_start_units {
                start_byte = index;
                break;
            }
            units += character.len_utf16();
            start_byte = index + character.len_utf8();
        }
        let windowed = &line[start_byte..];
        let (windowed, _) = truncate_units_in_scope(windowed, limits.line_units, text_scope);
        let shown_prefix = prefix_units.saturating_sub(units);
        (
            text_scope.format(format_args!("…{windowed}")),
            shown_prefix + 1,
        )
    } else {
        (text_scope.copy(line), prefix_units)
    };
    let caret_units = range_units.min(limits.line_units);
    let mut caret_line = text_scope.buffer_with(&text_scope.repeat(" ", shown_prefix));
    caret_line.push_str(&text_scope.repeat("^", caret_units));
    if !label.is_empty() {
        caret_line.push(' ');
        let (label, label_cut) = truncate_units_in_scope(label, limits.label_units, text_scope);
        cut |= label_cut;
        caret_line.push_str(&label);
    }
    // Tabs would misalign the caret; show them as spaces in the excerpt.
    Some((
        text_scope.format(format_args!(
            "{}\n{caret_line}",
            shown_line.replace('\t', " ")
        )),
        cut,
    ))
}

/// How much of a labelled line an excerpt may show. The message keeps a
/// window; the complete text keeps whole lines and whole labels.
#[invariant(*line_units > 0 && *label_units > 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExcerptLimits {
    line_units: usize,
    label_units: usize,
}

impl ExcerptLimits {
    /// What the message shows.
    #[requires(true)]
    #[ensures(ret.line_units == MAX_EXCERPT_UNITS)]
    fn shown() -> Self {
        new!(ExcerptLimits {
            line_units: MAX_EXCERPT_UNITS,
            label_units: MAX_MESSAGE_UNITS,
        })
    }

    /// Everything, for the attached result.
    #[requires(true)]
    #[ensures(ret.line_units == usize::MAX && ret.label_units == usize::MAX)]
    fn complete() -> Self {
        new!(ExcerptLimits {
            line_units: usize::MAX,
            label_units: usize::MAX,
        })
    }
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
pub(crate) fn render_diagnostics(
    source: &str,
    diagnostics: &[Diagnostic],
) -> Option<RenderedDiagnostics> {
    let text_scope = jbotci_output::budget::TextScope::new(
        jbotci_output::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let result = render_diagnostics_in_scope(source, diagnostics, &text_scope);
    text_scope
        .finish(String::new())
        .expect("unrestricted Discord rendering");
    result
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
fn complete_text(source: &str, diagnostics: &[Diagnostic]) -> String {
    let text_scope = jbotci_output::budget::TextScope::new(
        jbotci_output::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let result = complete_text_in_scope(source, diagnostics, &text_scope);
    text_scope
        .finish(String::new())
        .expect("unrestricted Discord rendering");
    result
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
pub(crate) fn summary(diagnostics: &[Diagnostic]) -> Option<String> {
    let text_scope = jbotci_output::budget::TextScope::new(
        jbotci_output::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let result = summary_in_scope(diagnostics, &text_scope);
    text_scope
        .finish(String::new())
        .expect("unrestricted Discord rendering");
    result
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
fn render_one(source: &str, diagnostic: &Diagnostic) -> (String, bool) {
    let text_scope = jbotci_output::budget::TextScope::new(
        jbotci_output::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let result = render_one_in_scope(source, diagnostic, &text_scope);
    text_scope
        .finish(String::new())
        .expect("unrestricted Discord rendering");
    result
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
fn caret_excerpt(
    source: &str,
    byte_start: usize,
    byte_end: usize,
    label: &str,
    limits: ExcerptLimits,
) -> Option<(String, bool)> {
    let text_scope = jbotci_output::budget::TextScope::new(
        jbotci_output::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let result = caret_excerpt_in_scope(source, byte_start, byte_end, label, limits, &text_scope);
    text_scope
        .finish(String::new())
        .expect("unrestricted Discord rendering");
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::data;
    use jbotci_web_core::analyze_vlasei;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_a_morphology_error_with_a_caret_excerpt() {
        let source = "mi 'klama do";
        let analysis = analyze_vlasei(source, None, None).expect("analysis");
        let rendered = render_diagnostics(source, &analysis.diagnostics).expect("diagnostics");
        assert!(
            rendered.shown.starts_with("**error** `morphology."),
            "{rendered:?}"
        );
        assert!(
            rendered.shown.contains("```\nmi 'klama do\n   ^"),
            "{rendered:?}"
        );
        assert!(!rendered.is_excerpt, "one short diagnostic is shown whole");
        assert!(rendered.complete.contains("morphology."), "{rendered:?}");
        assert_eq!(summary(&analysis.diagnostics).as_deref(), Some("1 error"));
        assert!(render_diagnostics(source, &[]).is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_complete_text_keeps_whole_labels_and_the_message_says_it_cut_them() {
        // A label longer than the message's own bound is shown short there
        // and whole in the complete text, and the diagnostics know they are
        // an excerpt so the assembler attaches it.
        let source = "mi 'klama do";
        let analysis = analyze_vlasei(source, None, None).expect("analysis");
        let long_label = "why this word cannot start here ".repeat(20);
        let diagnostics = analysis
            .diagnostics
            .iter()
            .map(|diagnostic| {
                let mut labels = diagnostic.labels.clone();
                if let Some(label) = labels.first_mut() {
                    *label = label.clone().with_data(data! {
                        message: long_label.clone(),
                    });
                }
                diagnostic.clone().with_data(data! { labels: labels })
            })
            .collect::<Vec<_>>();
        let rendered = render_diagnostics(source, &diagnostics).expect("diagnostics");
        assert!(
            rendered.is_excerpt,
            "the label was shortened for the message"
        );
        assert!(
            !rendered.shown.contains(&long_label),
            "the message keeps its bound"
        );
        assert!(
            rendered.complete.contains(&long_label),
            "the complete text keeps the whole label"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn long_lines_are_windowed_around_the_label() {
        let mut source = "a".repeat(200);
        source.push_str(" 'x");
        let analysis = analyze_vlasei(&source, None, None).expect("analysis");
        let rendered = render_diagnostics(&source, &analysis.diagnostics).expect("diagnostics");
        let block = rendered.shown.split("```").nth(1).expect("code block");
        for line in block.lines() {
            assert!(
                utf16_len(line) <= MAX_EXCERPT_UNITS + 2 + MAX_MESSAGE_UNITS,
                "{line}"
            );
        }
        assert!(block.contains('…'), "{block}");
    }
}
