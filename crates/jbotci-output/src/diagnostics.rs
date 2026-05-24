use std::ops::Range;

use ariadne::{Color, Config, IndexType, Label, Report, ReportKind, Source};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_diagnostics::{Diagnostic, DiagnosticLabel, DiagnosticSeverity};

use crate::OutputError;

type AriadneSpan = (String, Range<usize>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct DiagnosticRenderOptions {
    pub color: bool,
}

impl Default for DiagnosticRenderOptions {
    #[requires(true)]
    #[ensures(!ret.color)]
    fn default() -> Self {
        Self { color: false }
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
    builder.finish()
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
    use jbotci_diagnostics::{DiagnosticPhase, DiagnosticSeverity, source_span_from_byte_offsets};
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
            DiagnosticRenderOptions { color: false },
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
            DiagnosticRenderOptions { color: true },
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
            DiagnosticRenderOptions { color: true },
        )
        .expect("render diagnostics");

        assert_eq!(rendered, "");
    }
}
