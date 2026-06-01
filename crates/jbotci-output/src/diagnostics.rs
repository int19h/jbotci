use std::ops::Range;

use annotate_snippets::{
    Annotation, AnnotationKind, Group, Level, Renderer, Snippet, renderer::DecorStyle,
};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_diagnostics::{
    Diagnostic, DiagnosticDetailMode, DiagnosticLabel, DiagnosticSeverity, DiagnosticStyledNote,
    DiagnosticTextRole, DiagnosticTextSegment,
};
use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthStr;

use crate::{GlyphStyle, OutputError};

pub const DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH: usize = 80;

const DIAGNOSTIC_NOTE_PREFIX_WIDTH: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct DiagnosticRenderOptions {
    pub color: bool,
    pub detail: DiagnosticDetailMode,
    pub glyphs: GlyphStyle,
    pub terminal_width: usize,
}

impl Default for DiagnosticRenderOptions {
    #[requires(true)]
    #[ensures(!ret.color)]
    #[ensures(ret.glyphs == GlyphStyle::default())]
    #[ensures(ret.terminal_width == DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH)]
    fn default() -> Self {
        Self {
            color: false,
            detail: DiagnosticDetailMode::Summary,
            glyphs: GlyphStyle::default(),
            terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
        }
    }
}

#[requires(!source_label.is_empty())]
#[requires(options.terminal_width > 0)]
#[ensures(diagnostics.is_empty() -> ret.as_ref().is_ok_and(String::is_empty))]
#[ensures(!diagnostics.is_empty() -> ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub fn render_diagnostics(
    source_label: &str,
    source: &str,
    diagnostics: &[Diagnostic],
    options: DiagnosticRenderOptions,
) -> Result<String, OutputError> {
    let renderer = diagnostic_renderer(options);
    let mut output = String::new();
    for diagnostic in diagnostics {
        let group = diagnostic_group(source_label, source, diagnostic, options);
        let report = [group];
        let rendered = renderer.render(&report);
        output.push_str(&rendered);
        if !rendered.ends_with('\n') {
            output.push('\n');
        }
    }
    Ok(output)
}

#[requires(!source_label.is_empty())]
#[requires(options.terminal_width > 0)]
#[ensures(true)]
fn diagnostic_group<'diagnostic>(
    source_label: &'diagnostic str,
    source: &'diagnostic str,
    diagnostic: &'diagnostic Diagnostic,
    options: DiagnosticRenderOptions,
) -> Group<'diagnostic> {
    let annotations = diagnostic
        .labels
        .iter()
        .map(annotation_for_label)
        .collect::<Vec<_>>();
    let snippet = Snippet::source(source)
        .line_start(1)
        .path(source_label)
        .annotations(annotations);
    let mut group = Group::with_title(
        severity_level(diagnostic.severity)
            .primary_title(&diagnostic.message)
            .id(&diagnostic.code),
    )
    .element(snippet);

    for note in &diagnostic.notes {
        group = group.element(Level::NOTE.message(note));
    }
    for note in &diagnostic.styled_notes {
        if note.mode.visible_in(options.detail) {
            group = group.element(Level::NOTE.message(render_styled_note(note, options)));
        }
    }
    group
}

#[requires(options.terminal_width > 0)]
#[ensures(true)]
fn diagnostic_renderer(options: DiagnosticRenderOptions) -> Renderer {
    let renderer = if options.color {
        Renderer::styled()
    } else {
        Renderer::plain()
    };
    let decor_style = match options.glyphs {
        GlyphStyle::Unicode => DecorStyle::Unicode,
        GlyphStyle::Ascii => DecorStyle::Ascii,
    };
    renderer
        .decor_style(decor_style)
        .term_width(options.terminal_width)
}

#[requires(true)]
#[ensures(true)]
fn annotation_for_label(label: &DiagnosticLabel) -> Annotation<'_> {
    annotation_kind(label.primary)
        .span(label_span(label))
        .label(&label.message)
}

#[requires(true)]
#[ensures(true)]
fn annotation_kind(primary: bool) -> AnnotationKind {
    if primary {
        AnnotationKind::Primary
    } else {
        AnnotationKind::Context
    }
}

#[requires(true)]
#[ensures(true)]
fn severity_level(severity: DiagnosticSeverity) -> Level<'static> {
    match severity {
        DiagnosticSeverity::Error => Level::ERROR,
        DiagnosticSeverity::Warning => Level::WARNING,
        DiagnosticSeverity::Advice => Level::HELP,
    }
}

#[requires(!note.segments.is_empty())]
#[requires(options.terminal_width > 0)]
#[ensures(!ret.is_empty())]
fn render_styled_note(note: &DiagnosticStyledNote, options: DiagnosticRenderOptions) -> String {
    let wrap_width = diagnostic_note_wrap_width(options.terminal_width);
    let mut rendered = String::new();
    let mut line_width = 0;
    let mut line_text = String::new();
    for segment in &note.segments {
        render_wrapped_segment(
            &mut rendered,
            &mut line_width,
            &mut line_text,
            segment,
            options.color,
            wrap_width,
        );
    }
    rendered
}

#[requires(!segment.text.is_empty())]
#[requires(wrap_width > 0)]
#[ensures(true)]
fn render_wrapped_segment(
    rendered: &mut String,
    line_width: &mut usize,
    line_text: &mut String,
    segment: &DiagnosticTextSegment,
    color: bool,
    wrap_width: usize,
) {
    let mut run = String::new();
    let mut run_is_whitespace = false;
    for character in segment.text.chars() {
        if character == '\n' {
            flush_text_run(
                rendered,
                line_width,
                line_text,
                segment.role,
                &mut run,
                color,
                wrap_width,
            );
            push_hard_break(rendered, line_width, line_text);
        } else if character.is_whitespace() {
            if !run.is_empty() && !run_is_whitespace {
                flush_text_run(
                    rendered,
                    line_width,
                    line_text,
                    segment.role,
                    &mut run,
                    color,
                    wrap_width,
                );
            }
            run_is_whitespace = true;
            run.push(character);
        } else {
            if !run.is_empty() && run_is_whitespace {
                flush_text_run(
                    rendered,
                    line_width,
                    line_text,
                    segment.role,
                    &mut run,
                    color,
                    wrap_width,
                );
            }
            run_is_whitespace = false;
            run.push(character);
        }
    }
    flush_text_run(
        rendered,
        line_width,
        line_text,
        segment.role,
        &mut run,
        color,
        wrap_width,
    );
}

#[requires(wrap_width > 0)]
#[ensures(run.is_empty())]
fn flush_text_run(
    rendered: &mut String,
    line_width: &mut usize,
    line_text: &mut String,
    role: DiagnosticTextRole,
    run: &mut String,
    color: bool,
    wrap_width: usize,
) {
    if run.is_empty() {
        return;
    }
    push_soft_run(
        rendered, line_width, line_text, role, run, color, wrap_width,
    );
    run.clear();
}

#[requires(!text.is_empty())]
#[requires(wrap_width > 0)]
#[ensures(true)]
fn push_soft_run(
    rendered: &mut String,
    line_width: &mut usize,
    line_text: &mut String,
    role: DiagnosticTextRole,
    text: &str,
    color: bool,
    wrap_width: usize,
) {
    let run_width = rendered_segment_width(role, text, color);
    if text.chars().all(char::is_whitespace) {
        if *line_width == 0 || *line_width + run_width <= wrap_width {
            push_visible_run(
                rendered, line_width, line_text, role, text, color, run_width,
            );
        } else {
            push_auto_break(rendered, line_width, line_text);
        }
        return;
    }

    if *line_width > 0
        && *line_width + run_width > wrap_width
        && !can_trail_previous_run(role, text)
    {
        push_auto_break(rendered, line_width, line_text);
    }
    push_visible_run(
        rendered, line_width, line_text, role, text, color, run_width,
    );
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn can_trail_previous_run(role: DiagnosticTextRole, text: &str) -> bool {
    role == DiagnosticTextRole::Punctuation
        && text
            .chars()
            .all(|character| matches!(character, ',' | ';' | ':' | ')' | ']' | '}'))
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn push_visible_run(
    rendered: &mut String,
    line_width: &mut usize,
    line_text: &mut String,
    role: DiagnosticTextRole,
    text: &str,
    color: bool,
    width: usize,
) {
    rendered.push_str(&render_styled_segment(role, text, color));
    *line_width += width;
    line_text.push_str(&visible_segment_text(role, text, color));
}

#[requires(true)]
#[ensures(*line_width == 0)]
#[ensures(line_text.is_empty())]
fn push_hard_break(rendered: &mut String, line_width: &mut usize, line_text: &mut String) {
    rendered.push('\n');
    *line_width = 0;
    line_text.clear();
}

#[requires(true)]
#[requires(!line_text.is_empty())]
#[ensures(!line_text.is_empty())]
fn push_auto_break(rendered: &mut String, line_width: &mut usize, line_text: &mut String) {
    let indent = continuation_indent(line_text);
    rendered.push('\n');
    rendered.push_str(&indent);
    *line_width = UnicodeWidthStr::width(indent.as_str());
    *line_text = indent;
}

#[requires(!text.is_empty())]
#[ensures(!ret.is_empty())]
fn render_styled_segment(role: DiagnosticTextRole, text: &str, color: bool) -> String {
    let visible_text = visible_segment_text(role, text, color);
    if !color {
        return visible_text;
    }
    match role {
        DiagnosticTextRole::Construct => visible_text.bright_white().to_string(),
        DiagnosticTextRole::SpecificWord => visible_text.bright_cyan().italic().to_string(),
        DiagnosticTextRole::Selmaho => visible_text.bright_cyan().to_string(),
        DiagnosticTextRole::WordCategory => visible_text.bright_green().to_string(),
        DiagnosticTextRole::Keyword => visible_text.truecolor(170, 170, 170).to_string(),
        DiagnosticTextRole::Punctuation => visible_text.bright_black().to_string(),
        DiagnosticTextRole::Plain => visible_text,
    }
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn rendered_segment_width(role: DiagnosticTextRole, text: &str, color: bool) -> usize {
    UnicodeWidthStr::width(visible_segment_text(role, text, color).as_str())
}

#[requires(!text.is_empty())]
#[ensures(!ret.is_empty())]
fn visible_segment_text(role: DiagnosticTextRole, text: &str, color: bool) -> String {
    match (color, role) {
        (false, DiagnosticTextRole::SpecificWord) => format!("{{{text}}}"),
        (true, DiagnosticTextRole::WordCategory) => text.to_lowercase(),
        _ => text.to_owned(),
    }
}

#[requires(!line_text.is_empty())]
#[ensures(!ret.is_empty())]
fn continuation_indent(line_text: &str) -> String {
    let leading_spaces_len = line_text
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == b' ')
        .count();
    let leading_spaces = &line_text[..leading_spaces_len];
    let rest = &line_text[leading_spaces_len..];
    if rest.starts_with("- ") {
        format!("{leading_spaces}  ")
    } else if !leading_spaces.is_empty() {
        leading_spaces.to_owned()
    } else {
        "  ".to_owned()
    }
}

#[requires(terminal_width > 0)]
#[ensures(ret > 0)]
fn diagnostic_note_wrap_width(terminal_width: usize) -> usize {
    terminal_width
        .saturating_sub(DIAGNOSTIC_NOTE_PREFIX_WIDTH)
        .max(1)
}

#[requires(true)]
#[ensures(ret.start <= ret.end)]
fn label_span(label: &DiagnosticLabel) -> Range<usize> {
    label.span.byte_start..label.span.byte_end
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
                "FIhOI bridi/subbridi adverbial term".to_owned(),
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
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            },
        )
        .expect("render diagnostic");

        assert!(rendered.contains("warning"));
        assert!(rendered.contains("syntax.warning.experimental-fihoi-adverbial"));
        assert!(rendered.contains("<input>"));
        assert!(rendered.contains("fi'oi"));
        assert!(rendered.contains("FIhOI bridi/subbridi adverbial term"));
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
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            },
        )
        .expect("render diagnostic");

        assert!(rendered.contains("\x1b["));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ascii_decor_style_preserves_source_snippet() {
        let source = "mi kláma fi'oi broda";
        let rendered = render_diagnostics(
            "<input>",
            source,
            &[warning_diagnostic(source)],
            DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::Ascii,
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            },
        )
        .expect("render ASCII diagnostic");

        assert!(rendered.contains("--> <input>:"));
        assert!(!rendered.contains('╭'));
        assert!(rendered.contains("mi kláma fi'oi broda"));
        assert!(rendered.contains("^^^^^"));
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
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
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
                    "needs one of:\n- selbri".to_owned(),
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
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
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
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
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
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            },
        )
        .expect("color diagnostic");
        assert!(rendered.contains("\x1b["));
        assert!(rendered.contains("lo"));
        assert!(!rendered.contains("{lo}"));
        assert!(rendered.contains("\x1b[3m"));
        assert!(!rendered.contains("\x1b[4m"));
        assert!(rendered.contains("\x1b[96m"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn styled_segment_visible_text_matches_color_policy() {
        assert_eq!(
            visible_segment_text(DiagnosticTextRole::WordCategory, "KOhA SUMTI", false),
            "KOhA SUMTI"
        );
        assert_eq!(
            visible_segment_text(DiagnosticTextRole::WordCategory, "KOhA SUMTI", true),
            "koha sumti"
        );
        assert_eq!(
            visible_segment_text(DiagnosticTextRole::SpecificWord, "lo", false),
            "{lo}"
        );
        assert_eq!(
            visible_segment_text(DiagnosticTextRole::SpecificWord, "lo", true),
            "lo"
        );
        assert_eq!(
            visible_segment_text(DiagnosticTextRole::Selmaho, "GAhO", true),
            "GAhO"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn styled_segment_ansi_roles_match_color_policy() {
        let word = render_styled_segment(DiagnosticTextRole::SpecificWord, "lo", true);
        assert!(word.contains("\x1b[3m"));
        assert!(word.contains("\x1b[96m"));
        assert!(!word.contains("\x1b[4m"));

        let selmaho = render_styled_segment(DiagnosticTextRole::Selmaho, "GAhO", true);
        assert!(selmaho.contains("\x1b[96m"));

        let category = render_styled_segment(DiagnosticTextRole::WordCategory, "BRIVLA", true);
        assert!(category.contains("\x1b[92m"));
        assert!(category.contains("brivla"));
        assert!(!category.contains("BRIVLA"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn term_width_elides_long_single_line_source() {
        let source = "mi klama lo mutce mutce mutce mutce mutce mutce mutce mutce zdani";
        let start = source.find("zdani").expect("highlight word");
        let end = start + "zdani".len();
        let span =
            source_span_from_byte_offsets(Some(SourceId("<input>".to_owned())), source, start, end)
                .expect("highlight span");
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Warning,
            DiagnosticPhase::Syntax,
            "syntax.warning.test".to_owned(),
            "test warning".to_owned(),
            vec![DiagnosticLabel::new(span, "end label".to_owned(), true)],
            Vec::new(),
            None,
        );
        let rendered = render_diagnostics(
            "<input>",
            source,
            &[diagnostic],
            DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: 44,
            },
        )
        .expect("render long source diagnostic");

        assert!(rendered.contains('…'));
        assert!(rendered.contains("zdani"));
        assert!(!rendered.contains(source));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn styled_summary_notes_wrap_before_styling() {
        let note = DiagnosticStyledNote::new(
            DiagnosticNoteMode::Summary,
            vec![
                DiagnosticTextSegment::new(
                    DiagnosticTextRole::Plain,
                    "expected one of: ".to_owned(),
                ),
                DiagnosticTextSegment::new(DiagnosticTextRole::SpecificWord, "lo".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ", ".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::SpecificWord, "le".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ", ".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::WordCategory, "BRIVLA".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ", ".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Selmaho, "GAhO".to_owned()),
            ],
        );
        let rendered = render_styled_note(
            &note,
            DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: 36,
            },
        );

        assert!(rendered.contains('\n'));
        assert!(rendered.contains("{lo}"));
        assert!(rendered.contains("BRIVLA"));
        assert!(!rendered.contains("\x1b["));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn styled_detailed_bullets_wrap_with_continuation_indent() {
        let note = DiagnosticStyledNote::new(
            DiagnosticNoteMode::Detailed,
            vec![
                DiagnosticTextSegment::new(DiagnosticTextRole::Plain, "needs one of:\n".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, "- ".to_owned()),
                DiagnosticTextSegment::new(
                    DiagnosticTextRole::Construct,
                    "free modifier".to_owned(),
                ),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " (".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::SpecificWord, "lo'ai".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ", ".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::SpecificWord, "sa'ai".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ", ".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::SpecificWord, "le'ai".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " or ".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Selmaho, "XI".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ")".to_owned()),
            ],
        );
        let rendered = render_styled_note(
            &note,
            DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Detailed,
                glyphs: GlyphStyle::default(),
                terminal_width: 38,
            },
        );

        assert!(rendered.contains("\n  "));
        assert!(rendered.contains("- free modifier"));
        assert!(rendered.contains("{lo'ai}"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn non_ascii_byte_span_renders_source_text() {
        let source = "mi kláma fi'oi broda";
        let start = source.find("kláma").expect("non-ASCII word");
        let end = start + "kláma".len();
        let span =
            source_span_from_byte_offsets(Some(SourceId("<input>".to_owned())), source, start, end)
                .expect("non-ASCII byte span");
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Warning,
            DiagnosticPhase::Syntax,
            "syntax.warning.test".to_owned(),
            "test warning".to_owned(),
            vec![DiagnosticLabel::new(
                span,
                "non-ASCII label".to_owned(),
                true,
            )],
            Vec::new(),
            None,
        );
        let rendered = render_diagnostics(
            "<input>",
            source,
            &[diagnostic],
            DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            },
        )
        .expect("render non-ASCII diagnostic");

        assert!(rendered.contains("kláma"));
        assert!(rendered.contains("non-ASCII label"));
    }
}
