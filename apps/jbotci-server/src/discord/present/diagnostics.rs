//! Compact Discord rendering of shared diagnostics.
//!
//! Each diagnostic shows its severity, code and message, the source line its
//! primary label points at with a caret underline in a code block, and its
//! notes. The count is bounded so critical diagnostics always fit the reserved
//! budget; the remainder is summarized, never silently dropped.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_diagnostics::{Diagnostic, DiagnosticSeverity};

use super::markdown::{code_block, escape, join_lines, subtext, truncate_units};
use crate::discord::request::utf16_len;

/// Diagnostics shown in full before the rest is summarized.
pub(crate) const MAX_DETAILED_DIAGNOSTICS: usize = 4;
/// Longest source excerpt line in a caret block.
const MAX_EXCERPT_UNITS: usize = 96;
/// Longest single diagnostic message or note.
const MAX_MESSAGE_UNITS: usize = 300;

#[requires(true)]
#[ensures(!ret.is_empty())]
fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Advice => "advice",
    }
}

/// Render `diagnostics` against `source`. Returns `None` when there are none.
#[requires(true)]
#[ensures(ret.is_some() == !diagnostics.is_empty())]
pub(crate) fn render_diagnostics(source: &str, diagnostics: &[Diagnostic]) -> Option<String> {
    if diagnostics.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    for diagnostic in diagnostics.iter().take(MAX_DETAILED_DIAGNOSTICS) {
        lines.push(render_one(source, diagnostic));
    }
    let remaining = diagnostics.len().saturating_sub(MAX_DETAILED_DIAGNOSTICS);
    if remaining > 0 {
        let errors = diagnostics
            .iter()
            .skip(MAX_DETAILED_DIAGNOSTICS)
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
            .count();
        lines.push(subtext(&format!(
            "{remaining} more diagnostic{} not shown ({errors} error{})",
            if remaining == 1 { "" } else { "s" },
            if errors == 1 { "" } else { "s" }
        )));
    }
    Some(join_lines(lines))
}

/// Counts by severity, for status lines ("2 errors, 1 warning").
#[requires(true)]
#[ensures(ret.is_some() == !diagnostics.is_empty())]
pub(crate) fn summary(diagnostics: &[Diagnostic]) -> Option<String> {
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
            parts.push(format!(
                "{total} {}",
                if total == 1 { singular } else { plural }
            ));
        }
    }
    Some(parts.join(", "))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn render_one(source: &str, diagnostic: &Diagnostic) -> String {
    let (message, _) = truncate_units(&diagnostic.message, MAX_MESSAGE_UNITS);
    let mut lines = vec![format!(
        "**{}** `{}`: {}",
        severity_label(diagnostic.severity),
        diagnostic.code.replace('`', ""),
        escape(&message)
    )];
    if let Some(label) = diagnostic
        .labels
        .iter()
        .find(|label| label.primary)
        .or_else(|| diagnostic.labels.first())
        && let Some(excerpt) = caret_excerpt(
            source,
            label.span.byte_start,
            label.span.byte_end,
            &label.message,
        )
    {
        lines.push(code_block(&excerpt));
    }
    for note in diagnostic.notes.iter().take(3) {
        let (note, _) = truncate_units(note, MAX_MESSAGE_UNITS);
        lines.push(subtext(&note.replace('\n', " ")));
    }
    join_lines(lines)
}

/// The source line containing `byte_start`, with a caret underline for the
/// labelled range and the label text. `None` if the offsets do not fall on
/// character boundaries of `source`.
#[requires(true)]
#[ensures(true)]
fn caret_excerpt(source: &str, byte_start: usize, byte_end: usize, label: &str) -> Option<String> {
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
    let (shown_line, shown_prefix) = if utf16_len(line) > MAX_EXCERPT_UNITS {
        let window_start_units = prefix_units.saturating_sub(MAX_EXCERPT_UNITS / 3);
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
        let (windowed, _) = truncate_units(windowed, MAX_EXCERPT_UNITS);
        let shown_prefix = prefix_units.saturating_sub(units);
        (format!("…{windowed}"), shown_prefix + 1)
    } else {
        (line.to_owned(), prefix_units)
    };
    let caret_units = range_units.min(MAX_EXCERPT_UNITS);
    let mut caret_line = " ".repeat(shown_prefix);
    caret_line.push_str(&"^".repeat(caret_units));
    if !label.is_empty() {
        caret_line.push(' ');
        let (label, _) = truncate_units(label, MAX_MESSAGE_UNITS);
        caret_line.push_str(&label);
    }
    // Tabs would misalign the caret; show them as spaces in the excerpt.
    Some(format!("{}\n{caret_line}", shown_line.replace('\t', " ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jbotci_web_core::analyze_vlasei;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_a_morphology_error_with_a_caret_excerpt() {
        let source = "mi 'klama do";
        let analysis = analyze_vlasei(source, None, None).expect("analysis");
        let rendered = render_diagnostics(source, &analysis.diagnostics).expect("diagnostics");
        assert!(rendered.starts_with("**error** `morphology."), "{rendered}");
        assert!(rendered.contains("```\nmi 'klama do\n   ^"), "{rendered}");
        assert_eq!(summary(&analysis.diagnostics).as_deref(), Some("1 error"));
        assert!(render_diagnostics(source, &[]).is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn long_lines_are_windowed_around_the_label() {
        let mut source = "a".repeat(200);
        source.push_str(" 'x");
        let analysis = analyze_vlasei(&source, None, None).expect("analysis");
        let rendered = render_diagnostics(&source, &analysis.diagnostics).expect("diagnostics");
        let block = rendered.split("```").nth(1).expect("code block");
        for line in block.lines() {
            assert!(
                utf16_len(line) <= MAX_EXCERPT_UNITS + 2 + MAX_MESSAGE_UNITS,
                "{line}"
            );
        }
        assert!(block.contains('…'), "{block}");
    }
}
