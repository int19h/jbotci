use crate::budget::{BudgetedWriter, ByteLimit, OutputBudget};
use std::borrow::Cow;
use std::io::Write;
use std::ops::Range;

use annotate_snippets::{
    Annotation, AnnotationKind, Group, Level, Renderer, Snippet, renderer::DecorStyle,
};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_diagnostics::{
    Diagnostic, DiagnosticDetailMode, DiagnosticLabel, DiagnosticSeverity, DiagnosticStyledNote,
    DiagnosticTextRole, DiagnosticTextSegment,
};
use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthStr;

use crate::{GlyphStyle, OutputError};

pub const DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH: usize = 80;

#[invariant(*terminal_width > 0, "diagnostic render width must be positive")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        new!(DiagnosticRenderOptions {
            color: false,
            detail: DiagnosticDetailMode::Summary,
            glyphs: GlyphStyle::default(),
            terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
        })
    }
}

#[requires(!source_label.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|text| text.is_empty() == diagnostics.is_empty()) || ret.is_err())]
pub fn render_diagnostics(
    source_label: &str,
    source: &str,
    diagnostics: &[Diagnostic],
    options: DiagnosticRenderOptions,
) -> Result<String, OutputError> {
    render_diagnostics_with_limit(
        source_label,
        source,
        diagnostics,
        options,
        ByteLimit::new(usize::MAX).expect("positive limit"),
    )
}

#[requires(!source_label.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
pub fn render_diagnostics_with_limit(
    source_label: &str,
    source: &str,
    diagnostics: &[Diagnostic],
    options: DiagnosticRenderOptions,
    limit: ByteLimit,
) -> Result<String, OutputError> {
    let renderer = diagnostic_renderer(options);
    let mut output = OutputBudget::new(limit).writer();
    for diagnostic in diagnostics {
        let remaining =
            ByteLimit::new(output.remaining()).ok_or_else(|| OutputError::Limit { limit })?;
        let group = diagnostic_group(source_label, source, diagnostic, options, remaining)?;
        renderer
            .render_into(&[group], &mut output, remaining.get())
            .map_err(|_| OutputError::Limit { limit })?;
        if output.last_byte() != Some(b'\n') {
            output
                .write_all(b"\n")
                .map_err(|error| OutputError::from_render_io(error, limit))?;
        }
    }
    Ok(output
        .into_string()
        .expect("diagnostic rendering writes UTF-8"))
}

#[requires(!source_label.is_empty())]
#[ensures(true)]
fn diagnostic_group<'diagnostic>(
    source_label: &'diagnostic str,
    source: &'diagnostic str,
    diagnostic: &'diagnostic Diagnostic,
    options: DiagnosticRenderOptions,
    limit: ByteLimit,
) -> Result<Group<'diagnostic>, OutputError> {
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
    let mut remaining = limit.get();
    for note in &diagnostic.styled_notes {
        if note.mode.visible_in(options.detail) {
            let note_limit =
                ByteLimit::new(remaining).ok_or_else(|| OutputError::Limit { limit })?;
            let rendered = render_styled_note_with_limit(note, options, note_limit)
                .map_err(|error| OutputError::from_render_io(error, limit))?;
            remaining -= rendered.len();
            group = group.element(Level::NOTE.message(rendered));
        }
    }
    Ok(group)
}

#[requires(true)]
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

#[cfg(test)]
#[requires(!note.segments.is_empty())]
#[ensures(!ret.is_empty())]
fn render_styled_note(note: &DiagnosticStyledNote, options: DiagnosticRenderOptions) -> String {
    render_styled_note_with_limit(
        note,
        options,
        ByteLimit::new(usize::MAX).expect("positive limit"),
    )
    .expect("unrestricted diagnostic note")
}

#[requires(!note.segments.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
fn render_styled_note_with_limit(
    note: &DiagnosticStyledNote,
    options: DiagnosticRenderOptions,
    limit: ByteLimit,
) -> std::io::Result<String> {
    let mut rendered = OutputBudget::new(limit).writer();
    let wrap_width = diagnostic_note_wrap_width(options.terminal_width);
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
        )?;
    }
    Ok(rendered.into_string().expect("diagnostic notes are UTF-8"))
}

#[requires(!segment.text.is_empty() && wrap_width > 0)]
#[ensures(true)]
fn render_wrapped_segment(
    rendered: &mut BudgetedWriter,
    line_width: &mut usize,
    line_text: &mut String,
    segment: &DiagnosticTextSegment,
    color: bool,
    wrap_width: usize,
) -> std::io::Result<()> {
    let text = &segment.text;
    let mut start = 0;
    let mut whitespace = None;
    for (index, character) in text.char_indices() {
        if character == '\n' {
            if start != index {
                push_soft_run(
                    rendered,
                    line_width,
                    line_text,
                    segment.role,
                    &text[start..index],
                    color,
                    wrap_width,
                )?;
            }
            push_hard_break(rendered, line_width, line_text)?;
            start = index + 1;
            whitespace = None;
        } else {
            let next_whitespace = character.is_whitespace();
            if whitespace.is_some_and(|old| old != next_whitespace) {
                push_soft_run(
                    rendered,
                    line_width,
                    line_text,
                    segment.role,
                    &text[start..index],
                    color,
                    wrap_width,
                )?;
                start = index;
            }
            whitespace = Some(next_whitespace);
        }
    }
    if start != text.len() {
        push_soft_run(
            rendered,
            line_width,
            line_text,
            segment.role,
            &text[start..],
            color,
            wrap_width,
        )?;
    }
    Ok(())
}

#[requires(!text.is_empty() && wrap_width > 0)]
#[ensures(true)]
fn push_soft_run(
    rendered: &mut BudgetedWriter,
    line_width: &mut usize,
    line_text: &mut String,
    role: DiagnosticTextRole,
    text: &str,
    color: bool,
    wrap_width: usize,
) -> std::io::Result<()> {
    let visible_text = visible_segment_text(role, text, color, rendered.remaining())?;
    let run_width = UnicodeWidthStr::width(visible_text.as_ref());
    if text.chars().all(char::is_whitespace) {
        if *line_width == 0 || *line_width + run_width <= wrap_width {
            push_visible_run(
                rendered,
                line_width,
                line_text,
                role,
                visible_text.as_ref(),
                color,
                run_width,
            )?;
        } else {
            push_auto_break(rendered, line_width, line_text)?;
        }
        return Ok(());
    }
    if *line_width > 0
        && *line_width + run_width > wrap_width
        && !can_trail_previous_run(role, text)
    {
        push_auto_break(rendered, line_width, line_text)?;
    }
    push_visible_run(
        rendered,
        line_width,
        line_text,
        role,
        visible_text.as_ref(),
        color,
        run_width,
    )
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn can_trail_previous_run(role: DiagnosticTextRole, text: &str) -> bool {
    role == DiagnosticTextRole::Punctuation
        && text
            .chars()
            .all(|character| matches!(character, ',' | ';' | ':' | ')' | ']' | '}'))
}

#[requires(!visible_text.is_empty())]
#[ensures(true)]
fn push_visible_run(
    rendered: &mut BudgetedWriter,
    line_width: &mut usize,
    line_text: &mut String,
    role: DiagnosticTextRole,
    visible_text: &str,
    color: bool,
    width: usize,
) -> std::io::Result<()> {
    if !color {
        rendered.write_all(visible_text.as_bytes())?;
    } else {
        match role {
            DiagnosticTextRole::Construct => write!(rendered, "{}", visible_text.bright_white())?,
            DiagnosticTextRole::SpecificWord => {
                write!(rendered, "{}", visible_text.bright_cyan().italic())?
            }
            DiagnosticTextRole::Selmaho => write!(rendered, "{}", visible_text.bright_cyan())?,
            DiagnosticTextRole::WordCategory => {
                write!(rendered, "{}", visible_text.bright_green())?
            }
            DiagnosticTextRole::Keyword => {
                write!(rendered, "{}", visible_text.truecolor(170, 170, 170))?
            }
            DiagnosticTextRole::Punctuation => write!(rendered, "{}", visible_text.bright_black())?,
            DiagnosticTextRole::Plain => rendered.write_all(visible_text.as_bytes())?,
        }
    }
    *line_width += width;
    line_text.push_str(visible_text);
    Ok(())
}

#[requires(true)]
#[ensures(*line_width == 0 && line_text.is_empty() || ret.is_err())]
fn push_hard_break(
    rendered: &mut BudgetedWriter,
    line_width: &mut usize,
    line_text: &mut String,
) -> std::io::Result<()> {
    rendered.write_all(b"\n")?;
    *line_width = 0;
    line_text.clear();
    Ok(())
}

#[requires(!line_text.is_empty())]
#[ensures(!line_text.is_empty())]
fn push_auto_break(
    rendered: &mut BudgetedWriter,
    line_width: &mut usize,
    line_text: &mut String,
) -> std::io::Result<()> {
    let indent = continuation_indent(line_text);
    rendered.write_all(b"\n")?;
    rendered.write_all(indent.as_bytes())?;
    *line_width = UnicodeWidthStr::width(indent.as_str());
    *line_text = indent;
    Ok(())
}

#[requires(!text.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|text| !text.as_ref().is_empty()) || ret.is_err())]
fn visible_segment_text<'text>(
    role: DiagnosticTextRole,
    text: &'text str,
    color: bool,
    remaining: usize,
) -> std::io::Result<Cow<'text, str>> {
    match (color, role) {
        (false, DiagnosticTextRole::SpecificWord) => {
            let limit = ByteLimit::new(remaining).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::FileTooLarge,
                    "diagnostic byte limit reached",
                )
            })?;
            Ok(Cow::Owned(crate::budget::bounded_format(
                format_args!("{{{text}}}"),
                limit,
            )?))
        }
        (true, DiagnosticTextRole::WordCategory) => {
            let limit = ByteLimit::new(remaining).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::FileTooLarge,
                    "diagnostic byte limit reached",
                )
            })?;
            let mut lowercase = OutputBudget::new(limit).writer();
            for character in text.chars().flat_map(char::to_lowercase) {
                let mut bytes = [0; 4];
                lowercase.write_all(character.encode_utf8(&mut bytes).as_bytes())?;
            }
            Ok(Cow::Owned(
                lowercase.into_string().expect("lowercase text is UTF-8"),
            ))
        }
        _ => Ok(Cow::Borrowed(text)),
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
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
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
            new!(DiagnosticRenderOptions {
                color: true,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
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
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::Ascii,
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
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
            new!(DiagnosticRenderOptions {
                color: true,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
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
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
        )
        .expect("summary diagnostic");
        assert!(summary.contains("expected one of: {lo}"));
        assert!(!summary.contains("needs one of:"));

        let detailed = render_diagnostics(
            "<input>",
            source,
            &[diagnostic],
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Detailed,
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
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
            new!(DiagnosticRenderOptions {
                color: true,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
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
            visible_segment_text(
                DiagnosticTextRole::WordCategory,
                "KOhA SUMTI",
                false,
                usize::MAX
            )
            .unwrap(),
            "KOhA SUMTI"
        );
        assert_eq!(
            visible_segment_text(
                DiagnosticTextRole::WordCategory,
                "KOhA SUMTI",
                true,
                usize::MAX
            )
            .unwrap(),
            "koha sumti"
        );
        assert_eq!(
            visible_segment_text(DiagnosticTextRole::SpecificWord, "lo", false, usize::MAX)
                .unwrap(),
            "{lo}"
        );
        assert_eq!(
            visible_segment_text(DiagnosticTextRole::SpecificWord, "lo", true, usize::MAX).unwrap(),
            "lo"
        );
        assert_eq!(
            visible_segment_text(DiagnosticTextRole::Selmaho, "GAhO", true, usize::MAX).unwrap(),
            "GAhO"
        );
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
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: 44,
            }),
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
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: 36,
            }),
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
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Detailed,
                glyphs: GlyphStyle::default(),
                terminal_width: 38,
            }),
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
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
        )
        .expect("render non-ASCII diagnostic");

        assert!(rendered.contains("kláma"));
        assert!(rendered.contains("non-ASCII label"));
    }
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exhausted_diagnostic_preparation_keeps_the_typed_quota_error() {
        for (role, color) in [
            (DiagnosticTextRole::SpecificWord, false),
            (DiagnosticTextRole::WordCategory, true),
        ] {
            assert_eq!(
                visible_segment_text(role, "lo", color, 0)
                    .unwrap_err()
                    .kind(),
                std::io::ErrorKind::FileTooLarge
            );
        }
    }
}
