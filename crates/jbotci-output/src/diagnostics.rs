use std::ops::Range;

use ariadne::{Color, Config, IndexType, Label, Report, ReportKind, Source};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_diagnostics::{
    Diagnostic, DiagnosticDetailMode, DiagnosticLabel, DiagnosticSeverity, DiagnosticStyledNote,
    DiagnosticTextRole,
};
use owo_colors::OwoColorize;

use crate::OutputError;

type AriadneSpan = (String, Range<usize>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct DiagnosticRenderOptions {
    pub color: bool,
    pub detail: DiagnosticDetailMode,
}

impl Default for DiagnosticRenderOptions {
    #[requires(true)]
    #[ensures(!ret.color)]
    fn default() -> Self {
        Self {
            color: false,
            detail: DiagnosticDetailMode::Summary,
        }
    }
}

#[requires(!source_label.is_empty())]
#[ensures(diagnostics.is_empty() -> ret.as_ref().is_ok_and(String::is_empty))]
#[ensures(!diagnostics.is_empty() -> ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn render_diagnostics(
    source_label: &str,
    source: &str,
    diagnostics: &[Diagnostic],
    options: DiagnosticRenderOptions,
) -> Result<String, OutputError> {
    let mut output = Vec::new();
    for diagnostic in diagnostics {
        let report = diagnostic_report(source_label, diagnostic, options);
        report
            .write(
                (source_label.to_owned(), Source::from(source.to_owned())),
                &mut output,
            )
            .map_err(|error| OutputError::Diagnostic(error.to_string()))?;
    }
    String::from_utf8(output).map_err(|error| OutputError::Diagnostic(error.to_string()))
}

#[requires(!source_label.is_empty())]
#[ensures(true)]
fn diagnostic_report(
    source_label: &str,
    diagnostic: &Diagnostic,
    options: DiagnosticRenderOptions,
) -> Report<'static, AriadneSpan> {
    let primary = diagnostic.primary_label();
    let mut builder = Report::build(
        report_kind(diagnostic.severity),
        ariadne_span(source_label, primary),
    )
    .with_config(
        Config::default()
            .with_color(options.color)
            .with_index_type(IndexType::Byte),
    )
    .with_code(&diagnostic.code)
    .with_message(&diagnostic.message);

    for label in &diagnostic.labels {
        builder = builder.with_label(
            Label::new(ariadne_span(source_label, label))
                .with_message(&label.message)
                .with_color(label_color(diagnostic.severity, label.primary)),
        );
    }
    for note in &diagnostic.notes {
        builder = builder.with_note(note);
    }
    for note in &diagnostic.styled_notes {
        if note.mode.visible_in(options.detail) {
            builder = builder.with_note(render_styled_note(note, options.color));
        }
    }
    builder.finish()
}

#[requires(!note.segments.is_empty())]
#[ensures(!ret.is_empty())]
fn render_styled_note(note: &DiagnosticStyledNote, color: bool) -> String {
    let mut rendered = String::new();
    for segment in &note.segments {
        rendered.push_str(&render_styled_segment(segment.role, &segment.text, color));
    }
    rendered
}

#[requires(!text.is_empty())]
#[ensures(!ret.is_empty())]
fn render_styled_segment(role: DiagnosticTextRole, text: &str, color: bool) -> String {
    if !color {
        return match role {
            DiagnosticTextRole::SpecificWord => format!("{{{text}}}"),
            _ => text.to_owned(),
        };
    }
    match role {
        DiagnosticTextRole::Construct => text.bright_white().to_string(),
        DiagnosticTextRole::SpecificWord => text.magenta().underline().to_string(),
        DiagnosticTextRole::Selmaho => text.magenta().to_string(),
        DiagnosticTextRole::WordCategory => text.bright_blue().to_string(),
        DiagnosticTextRole::Keyword => text.truecolor(170, 170, 170).to_string(),
        DiagnosticTextRole::Punctuation => text.bright_black().to_string(),
        DiagnosticTextRole::Plain => text.to_owned(),
    }
}

#[requires(true)]
#[ensures(matches!(ret, ReportKind::Error | ReportKind::Warning | ReportKind::Advice))]
fn report_kind(severity: DiagnosticSeverity) -> ReportKind<'static> {
    match severity {
        DiagnosticSeverity::Error => ReportKind::Error,
        DiagnosticSeverity::Warning => ReportKind::Warning,
        DiagnosticSeverity::Advice => ReportKind::Advice,
    }
}

#[requires(true)]
#[ensures(true)]
fn label_color(severity: DiagnosticSeverity, primary: bool) -> Color {
    match (severity, primary) {
        (DiagnosticSeverity::Error, true) => Color::Red,
        (DiagnosticSeverity::Warning, true) => Color::Yellow,
        (DiagnosticSeverity::Advice, true) => Color::Cyan,
        (_, false) => Color::Blue,
    }
}

#[requires(!source_label.is_empty())]
#[ensures(ret.1.start <= ret.1.end)]
fn ariadne_span(source_label: &str, label: &DiagnosticLabel) -> AriadneSpan {
    (
        source_label.to_owned(),
        label.span.byte_start..label.span.byte_end,
    )
}

#[cfg(test)]
mod tests {
    use jbotci_diagnostics::{
        DiagnosticNoteMode, DiagnosticPhase, DiagnosticSeverity, DiagnosticStyledNote,
        DiagnosticTextRole, DiagnosticTextSegment, source_span_from_byte_offsets,
    };
    use jbotci_source::SourceId;

    use super::*;

    #[requires(true)]
    #[ensures(!ret.labels.is_empty())]
    fn warning_diagnostic(source: &str) -> Diagnostic {
        let span =
            source_span_from_byte_offsets(Some(SourceId("<input>".to_owned())), source, 9, 14)
                .expect("test span is valid");
        Diagnostic::new(
            DiagnosticSeverity::Warning,
            DiagnosticPhase::Syntax,
            "syntax.warning.experimental-fihoi-adverbial".to_owned(),
            "experimental syntax".to_owned(),
            vec![DiagnosticLabel::new(
                span,
                "FIhOI bridi/subsentence adverbial term".to_owned(),
                true,
            )],
            vec!["syntax accepted with warning".to_owned()],
            Some(2),
        )
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_plain_warning_with_source_label_and_span_text() {
        let source = "mi klama fi'oi broda";
        let rendered = render_diagnostics(
            "<input>",
            source,
            &[warning_diagnostic(source)],
            DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
            },
        )
        .expect("render diagnostic");

        assert!(rendered.contains("Warning"));
        assert!(rendered.contains("syntax.warning.experimental-fihoi-adverbial"));
        assert!(rendered.contains("<input>"));
        assert!(rendered.contains("fi'oi"));
        assert!(rendered.contains("FIhOI bridi/subsentence adverbial term"));
        assert!(rendered.contains("syntax accepted with warning"));
        assert!(!rendered.contains("\x1b["));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_color_when_requested() {
        let source = "mi klama fi'oi broda";
        let rendered = render_diagnostics(
            "<input>",
            source,
            &[warning_diagnostic(source)],
            DiagnosticRenderOptions {
                color: true,
                detail: DiagnosticDetailMode::Summary,
            },
        )
        .expect("render diagnostic");

        assert!(rendered.contains("\x1b["));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn empty_diagnostics_render_empty_text() {
        let rendered = render_diagnostics(
            "<input>",
            "coi",
            &[],
            DiagnosticRenderOptions {
                color: true,
                detail: DiagnosticDetailMode::Summary,
            },
        )
        .expect("render diagnostics");

        assert_eq!(rendered, "");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn styled_notes_respect_detail_mode_and_plain_word_braces() {
        let source = "mi klama fi'oi broda";
        let diagnostic = warning_diagnostic(source).with_styled_notes(vec![
            DiagnosticStyledNote::new(
                DiagnosticNoteMode::Summary,
                vec![
                    DiagnosticTextSegment::new(
                        DiagnosticTextRole::Plain,
                        "expected one of: ".to_owned(),
                    ),
                    DiagnosticTextSegment::new(DiagnosticTextRole::SpecificWord, "lo".to_owned()),
                ],
            ),
            DiagnosticStyledNote::new(
                DiagnosticNoteMode::Detailed,
                vec![DiagnosticTextSegment::new(
                    DiagnosticTextRole::Plain,
                    "needs one of:\n- relation".to_owned(),
                )],
            ),
        ]);

        let summary = render_diagnostics(
            "<input>",
            source,
            std::slice::from_ref(&diagnostic),
            DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
            },
        )
        .expect("summary diagnostic");
        assert!(summary.contains("expected one of: {lo}"));
        assert!(!summary.contains("needs one of:"));

        let detailed = render_diagnostics(
            "<input>",
            source,
            &[diagnostic],
            DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Detailed,
            },
        )
        .expect("detailed diagnostic");
        assert!(detailed.contains("needs one of:"));
        assert!(!detailed.contains("expected one of: {lo}"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn styled_notes_color_specific_words_without_plain_braces() {
        let source = "mi klama fi'oi broda";
        let diagnostic =
            warning_diagnostic(source).with_styled_notes(vec![DiagnosticStyledNote::new(
                DiagnosticNoteMode::Summary,
                vec![DiagnosticTextSegment::new(
                    DiagnosticTextRole::SpecificWord,
                    "lo".to_owned(),
                )],
            )]);

        let rendered = render_diagnostics(
            "<input>",
            source,
            &[diagnostic],
            DiagnosticRenderOptions {
                color: true,
                detail: DiagnosticDetailMode::Summary,
            },
        )
        .expect("color diagnostic");
        assert!(rendered.contains("\x1b["));
        assert!(rendered.contains("lo"));
        assert!(!rendered.contains("{lo}"));
    }
}
