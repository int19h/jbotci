use crate::budget::{BudgetedWriter, ByteLimit, OutputBudget};
use std::borrow::Cow;
use std::io::Write;
use std::ops::Range;

use annotate_snippets::{AnnotationKind, Group, Level, Renderer, Snippet, renderer::DecorStyle};
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
    if diagnostics.is_empty() {
        return Ok(String::new());
    }
    let renderer = diagnostic_renderer(options);
    let mut output = OutputBudget::new(limit).writer();
    let source_label = crate::styled::escape_terminal_field(source_label);
    for diagnostic in diagnostics {
        let remaining =
            ByteLimit::new(output.remaining()).ok_or_else(|| OutputError::Limit { limit })?;
        let message = crate::styled::escape_terminal_field(&diagnostic.message);
        let code = crate::styled::escape_terminal_field(&diagnostic.code);
        let notes = diagnostic
            .notes
            .iter()
            .map(|note| crate::styled::escape_terminal_note(note))
            .collect::<Vec<_>>();
        let label_messages = diagnostic
            .labels
            .iter()
            .map(|label| crate::styled::escape_terminal_field(&label.message))
            .collect::<Vec<_>>();
        let excerpt = diagnostic_source_excerpt(
            source,
            diagnostic,
            options,
            output.remaining(),
            limit,
            source_label.as_ref(),
        )?;
        let group = diagnostic_group(
            source_label.as_ref(),
            &excerpt,
            diagnostic,
            &label_messages,
            message.as_ref(),
            code.as_ref(),
            &notes,
            options,
            remaining,
        )?;
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

#[invariant(*source_start <= *source_end)]
#[invariant(*line_number >= 1 && *header_line >= 1 && *header_column >= 1)]
struct RetainedWindow<'source> {
    escaped: crate::styled::TerminalExcerpt<'source>,
    line_number: usize,
    header_line: usize,
    header_column: usize,
    source_start: usize,
    source_end: usize,
    /// This window holds the primary label. The renderer shows it first.
    owns_primary: bool,
    /// Labels whose display anchor is this window.
    label_indexes: Vec<usize>,
    /// Absolute source byte for each byte of `escaped.text`.
    ///
    /// Empty means the text is the source slice at `source_start`.
    /// [`usize::MAX`] is a cut mark.
    origin: Vec<usize>,
}

#[invariant(*line_start >= 1 && *primary_line >= 1 && *primary_column >= 1)]
struct DiagnosticSourceExcerpt<'source> {
    escaped: crate::styled::TerminalExcerpt<'source>,
    origin: usize,
    line_start: usize,
    /// File position of the primary label. Both paths use it for the main header.
    primary_line: usize,
    primary_column: usize,
    /// Retained pieces of a clipped excerpt. Empty means the excerpt is whole.
    retained: Vec<RetainedWindow<'source>>,
}

#[requires(options.terminal_width > 0)]
#[ensures(ret.as_ref().is_ok_and(|excerpt| excerpt.line_start >= 1 && excerpt.primary_line >= 1 && excerpt.primary_column >= 1) || ret.is_err())]
#[ensures(ret.as_ref().is_ok_and(|excerpt| excerpt.origin <= source.len()) || ret.is_err())]
fn diagnostic_source_excerpt<'source>(
    source: &'source str,
    diagnostic: &Diagnostic,
    options: DiagnosticRenderOptions,
    remaining: usize,
    limit: ByteLimit,
    source_label: &str,
) -> Result<DiagnosticSourceExcerpt<'source>, OutputError> {
    let anchors = display_anchors(source, diagnostic);
    let (primary_line, primary_column) = primary_anchor_position(&anchors);
    if anchors.is_empty() {
        return Ok(new!(DiagnosticSourceExcerpt {
            escaped: crate::styled::escape_terminal_source(""),
            origin: 0,
            line_start: 1,
            primary_line,
            primary_column,
            retained: Vec::new(),
        }));
    }
    let max_copy = remaining.saturating_add(1).max(1);
    let spans = anchors
        .iter()
        .map(|anchor| anchor.start..anchor.end.min(source.len()))
        .collect::<Vec<_>>();
    let (depth, kept) = annotate_snippets::renderer::kept_source_lines(source, &spans);
    let max_line = kept
        .iter()
        .map(|line| line.line_index)
        .chain(anchors.iter().map(|anchor| anchor.line))
        .max()
        .unwrap_or(1);
    let columns =
        annotate_snippets::renderer::code_columns(options.terminal_width, max_line, depth);
    let margin_mark = margin_mark(options.glyphs);
    let whole = whole_unclipped_excerpt(source, &anchors, &kept, columns, margin_mark);
    if let Some((origin, end, line_start)) = whole {
        let slice = source.get(origin..end).unwrap_or("");
        let Some(escaped) = crate::styled::escape_terminal_source_within(slice, max_copy) else {
            return Err(OutputError::Limit { limit });
        };
        return Ok(new!(DiagnosticSourceExcerpt {
            escaped,
            origin,
            line_start,
            primary_line,
            primary_column,
            retained: Vec::new(),
        }));
    }
    let retained = retained_display_windows(
        source,
        &anchors,
        &kept,
        columns,
        margin_mark,
        primary_line,
        primary_column,
        max_copy,
        source_label,
        options.glyphs,
        limit,
    )?;
    let origin = retained
        .first()
        .map(|window| window.source_start)
        .unwrap_or(0);
    let line_start = retained
        .first()
        .map(|window| window.line_number)
        .unwrap_or(1);
    Ok(new!(DiagnosticSourceExcerpt {
        escaped: crate::styled::escape_terminal_source(""),
        origin,
        line_start,
        primary_line,
        primary_column,
        retained,
    }))
}

#[requires(true)]
#[ensures(ret <= source.len())]
fn line_start_at(source: &str, index: usize) -> usize {
    let index = char_boundary_at(source, index);
    match source[..index].rfind('\n') {
        Some(newline) => newline + 1,
        None => 0,
    }
}

#[cfg(test)]
#[requires(true)]
#[ensures(ret <= source.len())]
fn line_end_at(source: &str, index: usize) -> usize {
    line_content_end(source, index)
}

#[requires(true)]
#[ensures(ret <= source.len() && source.is_char_boundary(ret))]
fn char_boundary_at(source: &str, index: usize) -> usize {
    let mut index = index.min(source.len());
    while index > 0 && !source.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// Exclusive end of the visible text on the line that holds `index`.
///
/// CR LF is one line break. The end is the CR byte.
/// A lone CR stays in the line.
#[cfg(test)]
#[requires(true)]
#[ensures(ret <= text.len() && text.is_char_boundary(ret))]
fn line_content_end(text: &str, index: usize) -> usize {
    let index = char_boundary_at(text, index);
    let Some(offset) = text[index..].find('\n') else {
        return text.len();
    };
    let newline = index + offset;
    if newline > 0 && text.as_bytes()[newline - 1] == b'\r' {
        newline - 1
    } else {
        newline
    }
}

/// One label before clipping and before escaping.
///
/// `line` and `column` are the original file position. They start at 1.
/// An empty line is a real anchor. The point at the end of the file is a real anchor.
#[invariant(*start <= *end)]
#[invariant(*line >= 1 && *column >= 1)]
struct DisplayAnchor {
    label_index: usize,
    start: usize,
    end: usize,
    line: usize,
    column: usize,
    primary: bool,
    /// Display columns reserved after this label. The renderer uses the same count.
    label_columns: usize,
}

/// True when this anchor meets the source range of one kept line or one run.
///
/// A point may sit on the exclusive end. A range overlaps when either side does.
#[requires(true)]
#[ensures(true)]
fn anchor_mark_inside(anchor: &DisplayAnchor, source_start: usize, source_end: usize) -> bool {
    if anchor.start == anchor.end {
        anchor.start >= source_start && anchor.start <= source_end
    } else {
        anchor.start < source_end && anchor.end > source_start
    }
}

#[requires(true)]
#[ensures(ret.0 >= 1 && ret.1 >= 1)]
fn source_line_column(source: &str, index: usize) -> (usize, usize) {
    let index = char_boundary_at(source, index.min(source.len()));
    let start = line_start_at(source, index);
    let line = 1 + source[..start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    let column = 1 + source[start..index].chars().count();
    (line.max(1), column.max(1))
}

/// Map every label to its original line and column.
///
/// The map runs before clipping and before escaping.
#[requires(true)]
#[ensures(ret.len() == diagnostic.labels.len())]
#[ensures(ret.iter().all(|anchor| anchor.line >= 1 && anchor.column >= 1 && anchor.start <= anchor.end))]
fn display_anchors(source: &str, diagnostic: &Diagnostic) -> Vec<DisplayAnchor> {
    let mut anchors = Vec::new();
    for (label_index, label) in diagnostic.labels.iter().enumerate() {
        let span = label_span(label);
        let start = char_boundary_at(source, span.start.min(source.len()));
        let end = char_boundary_at(source, span.end.min(source.len()).max(start));
        let (line, column) = source_line_column(source, start);
        let message = crate::styled::escape_terminal_field(&label.message);
        let label_columns = annotate_snippets::renderer::annotation_label_columns(message.as_ref());
        anchors.push(new!(DisplayAnchor {
            label_index,
            start,
            end,
            line,
            column,
            primary: label.primary,
            label_columns,
        }));
    }
    anchors
}

/// File position of the primary anchor. The start is an offset in the source.
#[requires(true)]
#[ensures(ret.0 >= 1 && ret.1 >= 1)]
fn primary_anchor_position(anchors: &[DisplayAnchor]) -> (usize, usize) {
    chosen_primary_index(anchors)
        .map(|index| {
            let anchor = &anchors[index];
            (anchor.line, anchor.column)
        })
        .unwrap_or((1, 1))
}

/// The primary label that starts first in the source.
#[requires(true)]
#[ensures(ret.is_none_or(|index| index < anchors.len()))]
fn chosen_primary_index(anchors: &[DisplayAnchor]) -> Option<usize> {
    let mut best: Option<(usize, usize)> = None;
    for anchor in anchors {
        if !anchor.primary {
            continue;
        }
        let replace = match best {
            None => true,
            Some((best_start, best_index)) => {
                (anchor.start, anchor.label_index) < (best_start, best_index)
            }
        };
        if replace {
            best = Some((anchor.start, anchor.label_index));
        }
    }
    best.map(|(_, index)| index)
}

/// Original position of the earliest anchor in this window.
#[requires(true)]
#[ensures(ret.0 >= 1 && ret.1 >= 1)]
fn anchor_window_header(
    anchors: &[DisplayAnchor],
    labels: &[usize],
    source: &str,
    source_start: usize,
) -> (usize, usize) {
    let mut best: Option<(usize, usize, usize, usize)> = None;
    for index in labels {
        let Some(anchor) = anchors.get(*index) else {
            continue;
        };
        let replace = match best {
            None => true,
            Some((best_line, best_start, _, _)) => {
                (anchor.line, anchor.start) < (best_line, best_start)
            }
        };
        if replace {
            best = Some((anchor.line, anchor.start, anchor.column, anchor.label_index));
        }
    }
    best.map(|(line, _, column, _)| (line, column))
        .unwrap_or_else(|| source_line_column(source, source_start))
}
/// The labelled lines are exactly the lines the renderer keeps, and none of them is cut.
///
/// The returned range is original source. A later render then matches an unclipped diagnostic.
#[requires(true)]
#[ensures(ret.is_none_or(|(origin, end, line)| origin <= end && end <= source.len() && line >= 1))]
fn whole_unclipped_excerpt(
    source: &str,
    anchors: &[DisplayAnchor],
    kept: &[annotate_snippets::renderer::KeptSourceLine],
    columns: usize,
    margin_mark: &str,
) -> Option<(usize, usize, usize)> {
    if kept.is_empty() {
        return None;
    }
    let mut previous = kept[0].line_index;
    for line in kept.iter().skip(1) {
        if line.line_index != previous + 1 {
            return None;
        }
        previous = line.line_index;
    }
    for line in kept {
        let text = source.get(line.start_byte..line.end_byte).unwrap_or("");
        let spans = line_label_spans(anchors, line.start_byte, line.end_byte, text.len());
        let laid = annotate_snippets::renderer::lay_out_line(
            text,
            &spans,
            columns,
            margin_mark,
            crate::styled::expanded_terminal_control,
        );
        if laid.clipped || laid.wide {
            return None;
        }
    }
    Some((
        kept[0].start_byte,
        kept[kept.len() - 1].end_byte,
        kept[0].line_index,
    ))
}

/// Copy the windows the renderer will draw.
///
/// A cut line keeps the renderer's own text, including the mark on each cut edge.
/// The copy, plus the real separator before each later window, stays inside the
/// remaining output budget plus one byte.
#[requires(primary_line >= 1 && primary_column >= 1)]
#[ensures(ret.as_ref().is_ok_and(|windows| windows.iter().all(|window| window.line_number >= 1 && window.header_line >= 1 && window.header_column >= 1)) || ret.is_err())]
fn retained_display_windows<'source>(
    source: &'source str,
    anchors: &[DisplayAnchor],
    kept: &[annotate_snippets::renderer::KeptSourceLine],
    columns: usize,
    margin_mark: &str,
    primary_line: usize,
    primary_column: usize,
    max_copy: usize,
    source_label: &str,
    glyphs: GlyphStyle,
    limit: ByteLimit,
) -> Result<Vec<RetainedWindow<'source>>, OutputError> {
    let mut pieces = Vec::new();
    let mut covered = vec![false; anchors.len()];
    let mut index = 0usize;
    while index < kept.len() {
        let mut end = index + 1;
        while end < kept.len() && kept[end].line_index == kept[end - 1].line_index + 1 {
            end += 1;
        }
        push_run_windows(
            source,
            anchors,
            &kept[index..end],
            columns,
            margin_mark,
            &mut covered,
            &mut pieces,
        );
        index = end;
    }
    for anchor in anchors {
        if covered.get(anchor.label_index).copied().unwrap_or(true) {
            continue;
        }
        covered[anchor.label_index] = true;
        pieces.push(new!(RawPiece {
            text: String::new(),
            origin: Vec::new(),
            source_start: anchor.start.min(source.len()),
            source_end: anchor.start.min(source.len()),
            line_number: anchor.line,
            labels: vec![anchor.label_index],
        }));
    }
    let primary_index = chosen_primary_index(anchors);
    if let Some(primary_at) = pieces
        .iter()
        .position(|piece| primary_index.is_some_and(|index| piece.labels.contains(&index)))
    {
        let primary = pieces.remove(primary_at);
        pieces.insert(0, primary);
    }
    let mut windows = Vec::new();
    let mut accounted = 0usize;
    for piece in pieces {
        let data = piece.into_data();
        let owns_primary = primary_index.is_some_and(|index| data.labels.contains(&index));
        let (header_line, header_column) = if owns_primary {
            (primary_line, primary_column)
        } else {
            anchor_window_header(anchors, &data.labels, source, data.source_start)
        };
        let separator = if windows.is_empty() {
            0
        } else {
            snippet_separator_len(glyphs, source_label, header_line, header_column)
        };
        let room = max_copy.saturating_sub(accounted.saturating_add(separator));
        let escaped = if data.origin.is_empty() {
            let slice = source.get(data.source_start..data.source_end).unwrap_or("");
            let Some(escaped) = crate::styled::escape_terminal_source_within(slice, room) else {
                return Err(OutputError::Limit { limit });
            };
            escaped
        } else if data.text.len() > room {
            return Err(OutputError::Limit { limit });
        } else {
            new!(crate::styled::TerminalExcerpt {
                text: std::borrow::Cow::Owned(data.text),
                map: Vec::new(),
            })
        };
        accounted = accounted
            .saturating_add(separator)
            .saturating_add(escaped.text.len());
        windows.push(new!(RetainedWindow {
            escaped,
            line_number: data.line_number,
            header_line,
            header_column,
            source_start: data.source_start,
            source_end: data.source_end,
            owns_primary,
            label_indexes: data.labels,
            origin: data.origin,
        }));
    }
    Ok(windows)
}

/// One copied window before its header is known.
#[invariant(*source_start <= *source_end && *line_number >= 1)]
struct RawPiece {
    text: String,
    origin: Vec<usize>,
    source_start: usize,
    source_end: usize,
    line_number: usize,
    labels: Vec<usize>,
}

#[requires(true)]
#[ensures(true)]
fn push_run_windows(
    source: &str,
    anchors: &[DisplayAnchor],
    run: &[annotate_snippets::renderer::KeptSourceLine],
    columns: usize,
    margin_mark: &str,
    covered: &mut [bool],
    pieces: &mut Vec<RawPiece>,
) {
    if run.is_empty() {
        return;
    }
    let mut layouts = Vec::new();
    let mut any_cut = false;
    for line in run {
        let text = source.get(line.start_byte..line.end_byte).unwrap_or("");
        let spans = line_label_spans(anchors, line.start_byte, line.end_byte, text.len());
        let laid = annotate_snippets::renderer::lay_out_line(
            text,
            &spans,
            columns,
            margin_mark,
            crate::styled::expanded_terminal_control,
        );
        if laid.clipped || laid.wide {
            any_cut = true;
        }
        layouts.push(laid);
    }
    if !any_cut {
        let labels = labels_on_run(anchors, run);
        if labels.is_empty() {
            return;
        }
        mark_covered(covered, &labels);
        pieces.push(new!(RawPiece {
            text: String::new(),
            origin: Vec::new(),
            source_start: run[0].start_byte,
            source_end: run[run.len() - 1].end_byte,
            line_number: run[0].line_index,
            labels,
        }));
        return;
    }
    for (line, laid) in run.iter().zip(layouts) {
        let labels = labels_on_run(anchors, std::slice::from_ref(line));
        if labels.is_empty() {
            continue;
        }
        if laid.wide && labels.len() > 1 {
            for label in labels {
                push_one_label_window(
                    source,
                    anchors,
                    line,
                    label,
                    columns,
                    margin_mark,
                    covered,
                    pieces,
                );
            }
            continue;
        }
        mark_covered(covered, &labels);
        if laid.clipped || laid.wide {
            pieces.push(clipped_piece(line, laid, labels));
        } else {
            pieces.push(new!(RawPiece {
                text: String::new(),
                origin: Vec::new(),
                source_start: line.start_byte,
                source_end: line.end_byte,
                line_number: line.line_index,
                labels,
            }));
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn push_one_label_window(
    source: &str,
    anchors: &[DisplayAnchor],
    line: &annotate_snippets::renderer::KeptSourceLine,
    label: usize,
    columns: usize,
    margin_mark: &str,
    covered: &mut [bool],
    pieces: &mut Vec<RawPiece>,
) {
    let Some(anchor) = anchors.get(label) else {
        return;
    };
    let text = source.get(line.start_byte..line.end_byte).unwrap_or("");
    let local_start = anchor.start.saturating_sub(line.start_byte).min(text.len());
    let local_end = anchor
        .end
        .saturating_sub(line.start_byte)
        .min(text.len())
        .max(local_start);
    let laid = annotate_snippets::renderer::lay_out_line(
        text,
        &[(local_start, local_end, anchor.label_columns)],
        columns,
        margin_mark,
        crate::styled::expanded_terminal_control,
    );
    if let Some(slot) = covered.get_mut(label) {
        *slot = true;
    }
    pieces.push(clipped_piece(line, laid, vec![label]));
}

#[requires(true)]
#[ensures(true)]
fn clipped_piece(
    line: &annotate_snippets::renderer::KeptSourceLine,
    laid: annotate_snippets::renderer::LaidOutLine,
    labels: Vec<usize>,
) -> RawPiece {
    let origin = laid
        .origin
        .into_iter()
        .map(|byte| {
            if byte == usize::MAX {
                usize::MAX
            } else {
                byte.saturating_add(line.start_byte)
            }
        })
        .collect::<Vec<_>>();
    new!(RawPiece {
        text: laid.text,
        origin,
        source_start: line.start_byte,
        source_end: line.end_byte,
        line_number: line.line_index,
        labels,
    })
}

#[requires(true)]
#[ensures(true)]
fn labels_on_run(
    anchors: &[DisplayAnchor],
    run: &[annotate_snippets::renderer::KeptSourceLine],
) -> Vec<usize> {
    let start = run.first().map(|line| line.start_byte).unwrap_or(0);
    let end = run.last().map(|line| line.end_byte).unwrap_or(start);
    anchors
        .iter()
        .filter(|anchor| anchor_mark_inside(anchor, start, end))
        .map(|anchor| anchor.label_index)
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn line_label_spans(
    anchors: &[DisplayAnchor],
    line_start: usize,
    line_end: usize,
    line_len: usize,
) -> Vec<(usize, usize, usize)> {
    anchors
        .iter()
        .filter(|anchor| anchor_mark_inside(anchor, line_start, line_end))
        .map(|anchor| {
            let start = anchor.start.saturating_sub(line_start).min(line_len);
            let end = anchor
                .end
                .saturating_sub(line_start)
                .min(line_len)
                .max(start);
            (start, end, anchor.label_columns.max(1))
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn mark_covered(covered: &mut [bool], labels: &[usize]) {
    for index in labels {
        if let Some(slot) = covered.get_mut(*index) {
            *slot = true;
        }
    }
}

/// Bytes of the secondary snippet header the renderer writes between windows.
#[requires(line >= 1 && column >= 1)]
#[ensures(ret > 0)]
fn snippet_separator_len(glyphs: GlyphStyle, path: &str, line: usize, column: usize) -> usize {
    let marker = match glyphs {
        GlyphStyle::Unicode => {
            annotate_snippets::renderer::DecorStyle::Unicode.secondary_file_start()
        }
        GlyphStyle::Ascii => annotate_snippets::renderer::DecorStyle::Ascii.secondary_file_start(),
    };
    marker.len() + path.len() + format!(":{line}:{column}").len()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn margin_mark(glyphs: GlyphStyle) -> &'static str {
    match glyphs {
        GlyphStyle::Unicode => annotate_snippets::renderer::DecorStyle::Unicode.margin(),
        GlyphStyle::Ascii => annotate_snippets::renderer::DecorStyle::Ascii.margin(),
    }
}

/// The window that draws this label's text.
///
/// The renderer draws the text at the end of the span. That end can sit on a
/// line break the content range does not include. The window is the latest
/// one that lists this label and still starts at or before that byte.
#[requires(true)]
#[ensures(true)]
fn window_shows_label_text(
    windows: &[RetainedWindow<'_>],
    window: &RetainedWindow<'_>,
    label_index: usize,
    label: &DiagnosticLabel,
) -> bool {
    let span = label_span(label);
    let mark = if span.start == span.end {
        span.start
    } else {
        span.end.saturating_sub(1)
    };
    let mut chosen: Option<&RetainedWindow<'_>> = None;
    for candidate in windows {
        if !candidate.label_indexes.contains(&label_index) {
            continue;
        }
        if candidate.source_start > mark {
            continue;
        }
        let closer = chosen.is_none_or(|current| candidate.source_start >= current.source_start);
        if closer {
            chosen = Some(candidate);
        }
    }
    if chosen.is_none() {
        chosen = windows
            .iter()
            .find(|candidate| candidate.label_indexes.contains(&label_index));
    }
    chosen.is_some_and(|candidate| std::ptr::eq(candidate, window))
}

/// Map a source span through the displayed bytes of a cut window.
///
/// [`usize::MAX`] is a cut mark. A mark between covered bytes stays in the span.
#[requires(true)]
#[ensures(ret.start <= ret.end && ret.end <= origin.len())]
fn map_through_origin(origin: &[usize], start: usize, end: usize) -> Range<usize> {
    let point = start == end;
    let end = end.max(start);
    let mut display_start = None;
    let mut display_end = 0usize;
    for (index, byte) in origin.iter().copied().enumerate() {
        if byte == usize::MAX {
            continue;
        }
        let inside = if point {
            byte == start
        } else {
            byte >= start && byte < end
        };
        if inside {
            if display_start.is_none() {
                display_start = Some(index);
            }
            display_end = index + 1;
        }
    }
    let Some(mut from) = display_start else {
        return 0..0;
    };
    let mut to = display_end.max(from);
    let first_real = origin.iter().copied().find(|byte| *byte != usize::MAX);
    let last_real = origin
        .iter()
        .copied()
        .rev()
        .find(|byte| *byte != usize::MAX);
    if first_real.is_some_and(|byte| start < byte) {
        from = 0;
    }
    if last_real.is_some_and(|byte| end > byte.saturating_add(1)) {
        to = origin.len();
    }
    from.min(origin.len())..to.min(origin.len()).max(from.min(origin.len()))
}

#[requires(!source_label.is_empty())]
#[ensures(true)]
fn diagnostic_group<'diagnostic>(
    source_label: &'diagnostic str,
    excerpt: &'diagnostic DiagnosticSourceExcerpt<'diagnostic>,
    diagnostic: &'diagnostic Diagnostic,
    label_messages: &'diagnostic [Cow<'diagnostic, str>],
    message: &'diagnostic str,
    code: &'diagnostic str,
    notes: &'diagnostic [Cow<'diagnostic, str>],
    options: DiagnosticRenderOptions,
    limit: ByteLimit,
) -> Result<Group<'diagnostic>, OutputError> {
    let mut group = Group::with_title(
        severity_level(diagnostic.severity)
            .primary_title(message)
            .id(code),
    );
    if excerpt.retained.is_empty() {
        let annotations = diagnostic
            .labels
            .iter()
            .zip(label_messages)
            .map(|(label, label_message)| {
                annotation_kind(label.primary)
                    .span(mapped_label_span(label, excerpt))
                    .label(label_message.as_ref())
            })
            .collect::<Vec<_>>();
        group = group.element(
            Snippet::source(excerpt.escaped.text.as_ref())
                .line_start(excerpt.line_start)
                .origin_position(excerpt.primary_line, excerpt.primary_column)
                .path(source_label)
                .annotations(annotations),
        );
    } else {
        let mut order: Vec<usize> = (0..excerpt.retained.len()).collect();
        if let Some(primary_at) = order
            .iter()
            .position(|&index| excerpt.retained[index].owns_primary)
        {
            let index = order.remove(primary_at);
            order.insert(0, index);
        }
        for window_index in order {
            let window = &excerpt.retained[window_index];
            let annotations = diagnostic
                .labels
                .iter()
                .enumerate()
                .zip(label_messages)
                .filter(|((index, _), _)| window.label_indexes.contains(index))
                .map(|((index, label), label_message)| {
                    let mut annotation =
                        annotation_kind(label.primary).span(window_label_span(label, window));
                    if window_shows_label_text(&excerpt.retained, window, index, label) {
                        annotation = annotation.label(label_message.as_ref());
                    }
                    annotation
                })
                .collect::<Vec<_>>();
            group = group.element(
                Snippet::source(window.escaped.text.as_ref())
                    .line_start(window.line_number)
                    .origin_position(window.header_line, window.header_column)
                    .path(source_label)
                    .annotations(annotations),
            );
        }
    }

    for note in notes {
        group = group.element(Level::NOTE.message(note.as_ref()));
    }
    let mut remaining = limit.get();
    for note in &diagnostic.styled_notes {
        if note.mode.visible_in(options.detail) {
            let note_limit =
                ByteLimit::new(remaining).ok_or_else(|| OutputError::Limit { limit })?;
            let rendered = render_styled_note_with_limit(note, options, note_limit)
                .map_err(|error| OutputError::from_render_io(error, limit))?;
            let rendered = crate::styled::normalize_terminal_breaks(&rendered).into_owned();
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
#[ensures(ret.start <= ret.end)]
fn mapped_label_span(
    label: &DiagnosticLabel,
    excerpt: &DiagnosticSourceExcerpt<'_>,
) -> Range<usize> {
    let span = label_span(label);
    apply_escape_map(
        span.start.saturating_sub(excerpt.origin),
        span.end.saturating_sub(excerpt.origin),
        &excerpt.escaped.map,
        excerpt.escaped.text.len(),
    )
}

#[requires(true)]
#[ensures(ret.start <= ret.end)]
fn window_label_span(label: &DiagnosticLabel, window: &RetainedWindow<'_>) -> Range<usize> {
    let span = label_span(label);
    if !window.origin.is_empty() {
        return map_through_origin(&window.origin, span.start, span.end.max(span.start));
    }
    apply_escape_map(
        span.start.saturating_sub(window.source_start),
        span.end.max(span.start).saturating_sub(window.source_start),
        &window.escaped.map,
        window.escaped.text.len(),
    )
}

#[requires(true)]
#[ensures(ret.start <= ret.end && ret.end <= text_len)]
fn apply_escape_map(start: usize, end: usize, map: &[usize], text_len: usize) -> Range<usize> {
    if map.is_empty() {
        let start = start.min(text_len);
        let end = end.min(text_len);
        return start..end.max(start);
    }
    let last = map.len() - 1;
    let start = map[start.min(last)].min(text_len);
    let end = map[end.min(last)].min(text_len);
    start..end.max(start)
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
    let mut index = 0;
    let mut whitespace = None;
    while index < text.len() {
        let character = text[index..].chars().next().expect("utf-8 character");
        let character_len = character.len_utf8();
        if character == '\n' || character == '\r' {
            let (content_end, consumed, ending) =
                if character == '\r' && text.as_bytes().get(index + 1) == Some(&b'\n') {
                    (index, 2, "\r\n")
                } else if character == '\n' {
                    (index, 1, "\n")
                } else {
                    (index, 1, "\r")
                };
            if start != content_end {
                push_soft_run(
                    rendered,
                    line_width,
                    line_text,
                    segment.role,
                    &text[start..content_end],
                    color,
                    wrap_width,
                )?;
            }
            push_hard_break(rendered, line_width, line_text, ending)?;
            index += consumed;
            start = index;
            whitespace = None;
            continue;
        }
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
        index += character_len;
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
    let blank = visible_text.chars().all(char::is_whitespace);
    let escaped = crate::styled::escape_terminal_controls(visible_text.as_ref());
    let run_width = UnicodeWidthStr::width(escaped.as_ref());
    if text.chars().all(char::is_whitespace) {
        if *line_width == 0 || *line_width + run_width <= wrap_width {
            push_visible_run(
                rendered,
                line_width,
                line_text,
                role,
                escaped.as_ref(),
                color,
                run_width,
                blank,
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
        escaped.as_ref(),
        color,
        run_width,
        blank,
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
    blank: bool,
) -> std::io::Result<()> {
    if !color || blank {
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

#[requires(line_ending == "\n" || line_ending == "\r\n" || line_ending == "\r")]
#[ensures(*line_width == 0 && line_text.is_empty() || ret.is_err())]
fn push_hard_break(
    rendered: &mut BudgetedWriter,
    line_width: &mut usize,
    line_text: &mut String,
    line_ending: &str,
) -> std::io::Result<()> {
    rendered.write_all(line_ending.as_bytes())?;
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
    use bityzba::data;
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
    fn an_excerpt_escapes_controls_and_keeps_the_later_span() {
        let source = "ab\u{85}cd\u{7f}";
        let span =
            source_span_from_byte_offsets(Some(SourceId("<input>".to_owned())), source, 4, 6)
                .expect("span on cd");
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Error,
            DiagnosticPhase::Morphology,
            "morphology.quote".to_owned(),
            "bad control".to_owned(),
            vec![DiagnosticLabel::new(span, "here".to_owned(), true)],
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
                glyphs: GlyphStyle::Ascii,
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
        )
        .expect("render excerpt");
        assert!(rendered.contains("ab\\u{85}cd\\u{7f}"), "{rendered}");
        assert!(!rendered.contains('\u{85}'), "{rendered}");
        assert!(!rendered.contains('\u{7f}'), "{rendered}");
        let lines = rendered.lines().collect::<Vec<_>>();
        let source_line = lines
            .iter()
            .find(|line| line.contains("ab\\u{85}cd\\u{7f}"))
            .expect("excerpt line");
        let column = source_line
            .find("ab\\u{85}cd\\u{7f}")
            .expect("excerpt column");
        let marked = column + "ab\\u{85}".len();
        let caret_line = lines
            .iter()
            .find(|line| line.contains('^'))
            .expect("caret line");
        let caret_at = caret_line.find('^').expect("caret");
        assert_eq!(caret_at, marked, "{rendered}");
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
    fn empty_diagnostics_skip_a_long_source() {
        let source = " ".repeat(1_000_000);
        let rendered = render_diagnostics(
            "<input>",
            &source,
            &[],
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
        )
        .expect("empty diagnostics");
        assert_eq!(rendered, "");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_short_limit_stops_a_long_unannotated_source() {
        let mut source = String::from("mi klama\n");
        source.push_str(&" ".repeat(1_000_000));
        source.push('\u{85}');
        let span =
            source_span_from_byte_offsets(Some(SourceId("<input>".to_owned())), &source, 0, 2)
                .expect("span on mi");
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Error,
            DiagnosticPhase::Syntax,
            "syntax.sample".to_owned(),
            "sample".to_owned(),
            vec![DiagnosticLabel::new(span, "here".to_owned(), true)],
            Vec::new(),
            None,
        );
        let options = new!(DiagnosticRenderOptions {
            color: false,
            detail: DiagnosticDetailMode::Summary,
            glyphs: GlyphStyle::Ascii,
            terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
        });
        let rendered = render_diagnostics("<input>", &source, &[diagnostic.clone()], options)
            .expect("short excerpt");
        assert!(rendered.len() < 10_000, "{}", rendered.len());
        assert!(rendered.contains("mi klama"), "{rendered}");
        assert!(!rendered.contains('\u{85}'), "{rendered}");
        let limited = render_diagnostics_with_limit(
            "<input>",
            &source,
            &[diagnostic],
            options,
            ByteLimit::new(32).expect("positive limit"),
        );
        assert!(
            matches!(limited, Err(OutputError::Limit { .. })),
            "{limited:?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_long_annotated_line_keeps_two_distant_labels() {
        let mut source = String::from("mi klama #\u{85}");
        source.push_str(&" ".repeat(100_000));
        source.push_str("do");
        let near =
            source_span_from_byte_offsets(Some(SourceId("<input>".to_owned())), &source, 0, 2)
                .expect("span on mi");
        let far_start = source.len() - 2;
        let far = source_span_from_byte_offsets(
            Some(SourceId("<input>".to_owned())),
            &source,
            far_start,
            source.len(),
        )
        .expect("span on do");
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Error,
            DiagnosticPhase::Syntax,
            "syntax.sample".to_owned(),
            "sample".to_owned(),
            vec![
                DiagnosticLabel::new(near, "near".to_owned(), true),
                DiagnosticLabel::new(far, "far".to_owned(), false),
            ],
            Vec::new(),
            None,
        );
        let options = new!(DiagnosticRenderOptions {
            color: false,
            detail: DiagnosticDetailMode::Summary,
            glyphs: GlyphStyle::Ascii,
            terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
        });
        let rendered = render_diagnostics("<input>", &source, &[diagnostic.clone()], options)
            .expect("clipped diagnostic");
        assert!(rendered.len() < 5_000, "{}", rendered.len());
        assert!(rendered.len() * 10 < source.len(), "{}", rendered.len());
        assert!(rendered.contains("\\u{85}"), "{rendered}");
        assert!(rendered.contains("near"), "{rendered}");
        assert!(rendered.contains("far"), "{rendered}");
        assert!(rendered.contains("do"), "{rendered}");
        assert!(!rendered.contains(&" ".repeat(200)), "{rendered}");
        let far_column = 1 + source[..far_start].chars().count();
        assert!(location_shows(&rendered, "--> ", 1, 1), "{rendered}");
        assert!(
            location_shows(&rendered, "::: ", 1, far_column),
            "{rendered}"
        );
        assert!(source_line_shows(&rendered, 1, "mi klama"), "{rendered}");
        assert!(source_line_shows(&rendered, 1, "do"), "{rendered}");
        assert!(!source_line_shows(&rendered, 3, "do"), "{rendered}");

        let limited = render_diagnostics_with_limit(
            "<input>",
            &source,
            &[diagnostic],
            options,
            ByteLimit::new(32).expect("positive limit"),
        );
        assert!(
            matches!(limited, Err(OutputError::Limit { .. })),
            "{limited:?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_clipped_tail_keeps_the_source_column() {
        let mut source = String::from("mi klama");
        source.push_str(&" ".repeat(100_000));
        source.push('#');
        let mark = source.len();
        source.push('\u{0085}');
        let column = 1 + source[..mark].chars().count();
        assert_eq!(column, 100_010);
        let span = source_span_from_byte_offsets(
            Some(SourceId("<input>".to_owned())),
            &source,
            mark,
            source.len(),
        )
        .expect("span on the control");
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Error,
            DiagnosticPhase::Syntax,
            "syntax.sample".to_owned(),
            "sample".to_owned(),
            vec![DiagnosticLabel::new(span, "here".to_owned(), true)],
            Vec::new(),
            None,
        );
        let rendered = render_diagnostics(
            "<input>",
            &source,
            &[diagnostic],
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::Ascii,
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
        )
        .expect("clipped tail");
        assert!(location_shows(&rendered, "--> ", 1, column), "{rendered}");
        assert!(!location_shows(&rendered, "--> ", 1, 78), "{rendered}");
        assert!(source_line_shows(&rendered, 1, "\\u{85}"), "{rendered}");
        assert!(!rendered.contains(&" ".repeat(200)), "{rendered}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn clipped_windows_keep_each_source_line() {
        let mut source = String::from("NEAR\n");
        for _ in 0..9_999 {
            source.push_str("pad\n");
        }
        let far_start = source.len();
        source.push_str("FAR");
        let far_line = 1 + source[..far_start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count();
        assert_eq!(far_line, 10_001);
        let near =
            source_span_from_byte_offsets(Some(SourceId("<input>".to_owned())), &source, 0, 4)
                .expect("span on NEAR");
        let far = source_span_from_byte_offsets(
            Some(SourceId("<input>".to_owned())),
            &source,
            far_start,
            source.len(),
        )
        .expect("span on FAR");
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Error,
            DiagnosticPhase::Syntax,
            "syntax.sample".to_owned(),
            "sample".to_owned(),
            vec![
                DiagnosticLabel::new(near, "near".to_owned(), true),
                DiagnosticLabel::new(far, "far".to_owned(), false),
            ],
            Vec::new(),
            None,
        );
        let rendered = render_diagnostics(
            "<input>",
            &source,
            &[diagnostic],
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::Ascii,
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
        )
        .expect("two distant lines");
        assert!(location_shows(&rendered, "--> ", 1, 1), "{rendered}");
        assert!(location_shows(&rendered, "::: ", far_line, 1), "{rendered}");
        assert!(source_line_shows(&rendered, 1, "NEAR"), "{rendered}");
        assert!(source_line_shows(&rendered, far_line, "FAR"), "{rendered}");
        assert!(!source_line_shows(&rendered, 3, "FAR"), "{rendered}");
        assert!(!rendered.contains("pad"), "{rendered}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_remote_primary_keeps_its_source_header() {
        let mut source = String::from("NEAR\n");
        for _ in 0..9_999 {
            source.push_str("pad\n");
        }
        let far_start = source.len();
        source.push_str("FAR");
        let far_line = 1 + source[..far_start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count();
        assert_eq!(far_line, 10_001);
        let near = source_span_from_byte_offsets(
            Some(SourceId("<input>".to_owned())),
            &source,
            0,
            source.len(),
        )
        .expect("span from the context");
        let far = source_span_from_byte_offsets(
            Some(SourceId("<input>".to_owned())),
            &source,
            far_start,
            source.len(),
        )
        .expect("span on FAR");
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Error,
            DiagnosticPhase::Syntax,
            "syntax.sample".to_owned(),
            "sample".to_owned(),
            vec![
                DiagnosticLabel::new(far, "far".to_owned(), true),
                DiagnosticLabel::new(near, "near".to_owned(), false),
            ],
            Vec::new(),
            None,
        );
        let rendered = render_diagnostics(
            "<input>",
            &source,
            &[diagnostic],
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::Ascii,
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
        )
        .expect("remote primary");
        assert!(location_shows(&rendered, "--> ", far_line, 1), "{rendered}");
        assert!(!location_shows(&rendered, "--> ", 1, 1), "{rendered}");
        assert!(location_shows(&rendered, "::: ", 1, 1), "{rendered}");
        assert_eq!(rendered.matches("near").count(), 1, "{rendered}");
        assert_eq!(rendered.matches("far").count(), 1, "{rendered}");
        assert!(source_line_shows(&rendered, 1, "NEAR"), "{rendered}");
        assert!(source_line_shows(&rendered, far_line, "FAR"), "{rendered}");
        assert_eq!(caret_row(&rendered, "near"), Some(far_line), "{rendered}");
        assert_eq!(caret_row(&rendered, "far"), Some(far_line), "{rendered}");
        assert!(rendered.len() * 4 < source.len(), "{rendered}");
        assert!(rendered.matches("pad").count() < 30, "{rendered}");
    }

    /// Parser diagnostics for the short ZOI and nelci texts.
    /// An unclipped excerpt equals the visible text of the main renderer.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn render_parser_diagnostics(source: &str) -> String {
        let source_id = SourceId("<input>".to_owned());
        let morphology = jbotci_morphology::segment_words_with_modifiers_recovered(source);
        let mut diagnostics = Vec::new();
        if !morphology.errors.is_empty() {
            for error in &morphology.errors {
                diagnostics.push(
                    error
                        .to_diagnostic(Some(source_id.clone()), source)
                        .expect("morphology span"),
                );
            }
        } else {
            let parsed =
                jbotci_syntax::parse_syntax_tree_with_recovery_with_source_and_options_attempt(
                    &morphology.words,
                    source,
                    &jbotci_syntax::ParseOptions::default(),
                );
            match parsed.result.as_data() {
                data!(jbotci_syntax::SyntaxRecoveryParse::Valid { parse }) => {
                    for warning in &parse.warnings {
                        diagnostics.push(warning.to_diagnostic(Some(source_id.clone()), source));
                    }
                }
                data!(jbotci_syntax::SyntaxRecoveryParse::Recovered { parse }) => {
                    for error in &parse.errors {
                        diagnostics.push(error.to_diagnostic(Some(source_id.clone()), source));
                    }
                    for warning in &parse.warnings {
                        diagnostics.push(warning.to_diagnostic(Some(source_id.clone()), source));
                    }
                }
                data!(jbotci_syntax::SyntaxRecoveryParse::Interrupted) => {
                    panic!("parse interrupted");
                }
            }
        }
        render_diagnostics(
            "<input>",
            source,
            &diagnostics,
            DiagnosticRenderOptions::default(),
        )
        .expect("diagnostic render")
    }

    #[requires(true)]
    #[ensures(true)]
    fn split_gutter(line: &str) -> Option<(&str, &str)> {
        let index = line.find(['|', '│'])?;
        let (gutter, rest) = line.split_at(index);
        let mut body = rest;
        if let Some(stripped) = body.strip_prefix('|') {
            body = stripped;
        } else if let Some(stripped) = body.strip_prefix('│') {
            body = stripped;
        }
        Some((gutter, body))
    }

    #[requires(true)]
    #[ensures(true)]
    fn numbered_source_body<'a>(rendered: &'a str, line_number: usize) -> &'a str {
        rendered
            .lines()
            .find(|line| {
                split_gutter(line)
                    .is_some_and(|(gutter, _)| gutter.trim() == line_number.to_string())
                    && !line.contains('^')
            })
            .unwrap_or_else(|| panic!("numbered source line\n{rendered}"))
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_unclipped_multiline_zoi_excerpt_matches_main() {
        let source = "mi cusku zoi gy. abc\ndef\nghi";
        let rendered = render_parser_diagnostics(source);
        assert_eq!(
            rendered,
            include_str!("../tests/excerpt-fixtures/zoi.stderr"),
            "{rendered}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_unclipped_multiline_context_label_matches_main() {
        let source = "mi nelci lo barda\nzarci ku ku";
        let rendered = render_parser_diagnostics(source);
        assert_eq!(
            rendered,
            include_str!("../tests/excerpt-fixtures/nelci.stderr"),
            "{rendered}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_middle_error_keeps_context_and_marks_both_cut_edges() {
        let source = format!(
            "{}mi tavla kkk do .i {}",
            "mi klama lo zarci .i ".repeat(40),
            "do citka lo plise .i ".repeat(40),
        );
        let rendered = render_parser_diagnostics(&source);
        let body = split_gutter(numbered_source_body(&rendered, 1))
            .expect("gutter")
            .1
            .trim();
        assert!(body.starts_with('…'), "{body}");
        assert!(body.ends_with('…'), "{body}");
        assert!(body.contains("tavla kkk do"), "{body}");
        let source_line = numbered_source_body(&rendered, 1);
        let word = source_line.find("kkk").expect("error word");
        let mark_line = rendered
            .lines()
            .find(|line| line.contains('┯') || line.contains('^'))
            .unwrap_or_else(|| panic!("caret\n{rendered}"));
        let mark_at = mark_line
            .char_indices()
            .find(|(_, mark)| *mark == '┯' || *mark == '^')
            .map(|(index, _)| index)
            .unwrap_or_else(|| panic!("mark\n{rendered}"));
        assert_eq!(
            unicode_width::UnicodeWidthStr::width(&mark_line[..mark_at]),
            unicode_width::UnicodeWidthStr::width(&source_line[..word]),
            "{rendered}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_long_span_marks_each_cut_and_keeps_both_ends() {
        let source = format!("mi nelci lo {}ku ku", "barda ".repeat(150));
        let rendered = render_parser_diagnostics(&source);
        let body = split_gutter(numbered_source_body(&rendered, 1))
            .expect("gutter")
            .1
            .trim();
        assert!(body.starts_with('…'), "{body}");
        assert!(body.ends_with("ku ku"), "{body}");
        assert!(!body.ends_with('…'), "{body}");
        assert!(body.contains("elci lo barda"), "{body}");
        assert!(body.matches('…').count() >= 2, "{body}");
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn excerpt_body(rendered: &str) -> String {
        split_gutter(numbered_source_body(rendered, 1))
            .expect("gutter")
            .1
            .trim()
            .to_owned()
    }

    #[requires(true)]
    #[ensures(true)]
    fn caret_display_column(rendered: &str) -> usize {
        let mark_line = rendered
            .lines()
            .find(|line| line.contains('┯') || line.contains('^'))
            .unwrap_or_else(|| panic!("caret\n{rendered}"));
        let mark_at = mark_line
            .char_indices()
            .find(|(_, mark)| *mark == '┯' || *mark == '^')
            .map(|(index, _)| index)
            .unwrap_or_else(|| panic!("mark\n{rendered}"));
        unicode_width::UnicodeWidthStr::width(&mark_line[..mark_at])
    }

    #[requires(!word.is_empty())]
    #[ensures(true)]
    fn assert_caret_on(rendered: &str, word: &str) {
        let source_line = numbered_source_body(rendered, 1);
        let column = caret_display_column(rendered);
        let found = source_line.match_indices(word).any(|(index, _)| {
            unicode_width::UnicodeWidthStr::width(&source_line[..index]) == column
        });
        assert!(found, "caret column {column} is not on {word}\n{rendered}");
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_caret_is_not_on_the_leading_ellipsis(rendered: &str) {
        let source_line = numbered_source_body(rendered, 1);
        let Some(index) = source_line.find('…') else {
            return;
        };
        let column = unicode_width::UnicodeWidthStr::width(&source_line[..index]);
        assert_ne!(caret_display_column(rendered), column, "{rendered}");
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_combining_marks_follow_bases(text: &str) {
        let mut previous: Option<char> = None;
        for character in text.chars() {
            let width = unicode_width::UnicodeWidthChar::width(character).unwrap_or(1);
            if width == 0 {
                let base = previous.unwrap_or_else(|| panic!("leading combining mark in {text}"));
                assert!(
                    !base.is_whitespace() && base != '…' && base != '.',
                    "combining mark follows {base:?} in {text}"
                );
            }
            previous = Some(character);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_one_fitted_window(rendered: &str, body: &str) {
        assert!(
            unicode_width::UnicodeWidthStr::width(body) <= DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            "{body}"
        );
        assert!(!rendered.contains('⸬'), "{rendered}");
        assert!(!rendered.contains(":::"), "{rendered}");
        assert_combining_marks_follow_bases(body);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_long_span_of_wide_characters_keeps_the_tail() {
        let source = format!("mi nelci lo zoi gy. {} .gy ku ku", "漢字".repeat(80));
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.contains(".gy ku ku"), "{body}");
        assert!(body.starts_with('…'), "{body}");
        assert!(!body.ends_with('…'), "{body}");
        assert!(body.matches('…').count() >= 2, "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "ku");
        assert_caret_is_not_on_the_leading_ellipsis(&rendered);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_long_span_of_emoji_keeps_the_tail() {
        let source = format!("mi nelci lo zoi gy. {} .gy ku ku", "😀".repeat(80));
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.contains(".gy ku ku"), "{body}");
        assert!(body.contains('😀'), "{body}");
        assert!(body.starts_with('…') && !body.ends_with('…'), "{body}");
        assert!(body.matches('…').count() >= 2, "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "ku");
        assert_caret_is_not_on_the_leading_ellipsis(&rendered);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_long_span_of_combining_marks_keeps_one_window() {
        let source = format!("mi nelci lo zoi gy. {} .gy ku ku", "a\u{0301}".repeat(180));
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.contains(".gy ku ku"), "{body}");
        assert!(body.starts_with('…'), "{body}");
        assert!(body.contains('\u{0301}'), "{body}");
        assert!(body.matches('…').count() >= 2, "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "ku");
        assert_caret_is_not_on_the_leading_ellipsis(&rendered);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_middle_error_among_wide_characters_cuts_both_sides() {
        let source = format!(
            "zoi gy. {} .gy mi tavla kkk do .i {}",
            "漢字".repeat(60),
            "do citka lo plise .i ".repeat(30),
        );
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.starts_with("… "), "{body}");
        assert!(body.ends_with('…'), "{body}");
        assert!(body.contains("tavla kkk do"), "{body}");
        assert!(body.contains('字'), "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "kkk");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_middle_error_among_emoji_cuts_both_sides() {
        let source = format!(
            "zoi gy. {} .gy mi tavla kkk do .i {}",
            "😀".repeat(60),
            "do citka lo plise .i ".repeat(30),
        );
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.starts_with("… "), "{body}");
        assert!(body.ends_with('…'), "{body}");
        assert!(body.contains("tavla kkk do"), "{body}");
        assert!(body.contains('😀'), "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "kkk");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_middle_error_among_combining_marks_cuts_both_sides() {
        let source = format!(
            "zoi gy. {} .gy mi tavla kkk do .i {}",
            "a\u{0301}".repeat(180),
            "do citka lo plise .i ".repeat(30),
        );
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.starts_with("…a"), "{body}");
        assert!(body.ends_with('…'), "{body}");
        assert!(body.contains("tavla kkk do"), "{body}");
        assert!(body.contains('\u{0301}'), "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "kkk");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_wide_character_line_cuts_only_the_left() {
        let source = format!("zoi gy. {} .gy kkk", "漢字".repeat(50));
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.starts_with("… "), "{body}");
        assert!(body.ends_with("kkk"), "{body}");
        assert!(!body.ends_with('…'), "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "kkk");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_wide_character_line_cuts_only_the_right() {
        let source = format!("kkk {}", "漢字".repeat(50));
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.starts_with("kkk"), "{body}");
        assert!(body.ends_with('…'), "{body}");
        assert!(!body.starts_with('…'), "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "kkk");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_emoji_line_cuts_only_the_left() {
        let source = format!("zoi gy. {} .gy kkk", "😀".repeat(50));
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.starts_with("… "), "{body}");
        assert!(body.ends_with("kkk") && !body.ends_with('…'), "{body}");
        assert!(body.contains('😀'), "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "kkk");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_emoji_line_cuts_only_the_right() {
        let source = format!("kkk {}", "😀".repeat(50));
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.starts_with("kkk") && body.ends_with('…'), "{body}");
        assert!(body.contains('😀'), "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "kkk");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_combining_mark_line_cuts_only_the_left() {
        let source = format!("zoi gy. {} .gy kkk", "a\u{0301}".repeat(90));
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.starts_with("…a"), "{body}");
        assert!(body.ends_with("kkk") && !body.ends_with('…'), "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "kkk");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_combining_mark_line_cuts_only_the_right() {
        let source = format!("kkk zoi gy. {} .gy", "a\u{0301}".repeat(90));
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.starts_with("kkk") && body.ends_with('…'), "{body}");
        assert!(body.contains('\u{0301}'), "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "kkk");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_stressed_klama_keeps_its_accent_in_a_long_line() {
        let source = format!(
            "{}mi tavla kla\u{0301}ma kkk do .i {}",
            "mi klama lo zarci .i ".repeat(30),
            "do citka lo plise .i ".repeat(30),
        );
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.contains("kla\u{0301}ma"), "{body}");
        assert!(body.starts_with('…') && body.ends_with('…'), "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "kkk");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_stressed_klama_stays_with_its_base_in_a_long_span() {
        let source = format!("mi nelci lo {}kla\u{0301}ma ku ku", "barda ".repeat(150));
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert!(body.contains("kla\u{0301}ma"), "{body}");
        assert!(body.contains("ku ku"), "{body}");
        assert!(body.starts_with('…'), "{body}");
        assert_one_fitted_window(&rendered, &body);
        assert_caret_on(&rendered, "ku");
        assert_caret_is_not_on_the_leading_ellipsis(&rendered);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn single_line_fields_escape_breaks_and_notes_keep_both_words() {
        let source = "mi";
        let span = source_span_from_byte_offsets(
            Some(SourceId("<input>".to_owned())),
            source,
            0,
            source.len(),
        )
        .expect("span on mi");
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Error,
            DiagnosticPhase::Syntax,
            "code\rX".to_owned(),
            "msg\nY".to_owned(),
            vec![DiagnosticLabel::new(span, "lab\rel".to_owned(), true)],
            vec!["note\rline".to_owned()],
            None,
        )
        .with_styled_notes(vec![DiagnosticStyledNote::new(
            DiagnosticNoteMode::Summary,
            vec![DiagnosticTextSegment::new(
                DiagnosticTextRole::Plain,
                "wide\rpart".to_owned(),
            )],
        )]);
        let rendered = render_diagnostics(
            "bad\r\nFAKE.loj",
            source,
            &[diagnostic],
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::Ascii,
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
        )
        .expect("escaped diagnostic");
        assert!(!rendered.contains('\r'), "{rendered:?}");
        assert!(rendered.contains("bad\\u{d}\\u{a}FAKE.loj"), "{rendered:?}");
        assert!(rendered.contains("code\\u{d}X"), "{rendered:?}");
        assert!(rendered.contains("msg\\u{a}Y"), "{rendered:?}");
        assert!(rendered.contains("lab\\u{d}el"), "{rendered:?}");
        assert!(rendered.contains("note"), "{rendered:?}");
        assert!(rendered.contains("line"), "{rendered:?}");
        assert!(rendered.contains("wide"), "{rendered:?}");
        assert!(rendered.contains("part"), "{rendered:?}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_uncut_control_keeps_the_source_column() {
        let source = "mi\t#";
        let hash = source.find('#').expect("hash");
        let column = 1 + source[..hash].chars().count();
        assert_eq!(column, 4);
        let span = source_span_from_byte_offsets(
            Some(SourceId("<input>".to_owned())),
            source,
            hash,
            hash + 1,
        )
        .expect("span on the hash");
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Error,
            DiagnosticPhase::Morphology,
            "morphology.invalid-character".to_owned(),
            "invalid character".to_owned(),
            vec![DiagnosticLabel::new(span, "here".to_owned(), true)],
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
                glyphs: GlyphStyle::Ascii,
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
        )
        .expect("uncut control");
        assert!(rendered.contains('#'), "{rendered}");
        assert!(!rendered.contains("\\u{9}"), "{rendered}");
        assert!(location_shows(&rendered, "--> ", 1, column), "{rendered}");
        assert!(!location_shows(&rendered, "--> ", 1, 8), "{rendered}");
        let lines = rendered.lines().collect::<Vec<_>>();
        let source_line = lines
            .iter()
            .find(|line| line.contains('#') && !line.contains('^'))
            .expect("excerpt line");
        let marked = source_line.find('#').expect("excerpt column");
        let caret_line = lines
            .iter()
            .find(|line| line.contains('^'))
            .expect("caret line");
        let caret_at = caret_line.find('^').expect("caret");
        assert_eq!(caret_at, marked, "{rendered}");
    }

    /// A cut excerpt contains the renderer's margin mark, a folded-line mark,
    /// or a later snippet header. The header is how a copied window marks a
    /// line gap without copying the omitted lines.
    #[requires(true)]
    #[ensures(true)]
    fn rendered_excerpt_is_cut(rendered: &str) -> bool {
        rendered.lines().any(|line| {
            line.contains("...") || line.contains('…') || line.contains(":::") || line.contains('⸬')
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_line_shows(rendered: &str, line_number: usize, text: &str) -> bool {
        rendered.lines().any(|line| {
            line.split('|')
                .next()
                .is_some_and(|gutter| gutter.trim() == line_number.to_string())
                && line.contains(text)
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn location_shows(rendered: &str, marker: &str, line_number: usize, column: usize) -> bool {
        let needle = format!("{marker}<input>:{line_number}:{column}");
        rendered.lines().any(|line| {
            line.match_indices(&needle).any(|(index, _)| {
                line[index + needle.len()..]
                    .chars()
                    .next()
                    .is_none_or(|character| !character.is_ascii_digit())
            })
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_empty_final_line_keeps_the_primary_caret() {
        let source = "zoi gy\n";
        let rendered = render_label_set(
            source,
            &[
                (0, 6, "opening gy".to_owned(), false),
                (
                    source.len(),
                    source.len(),
                    "expected closing delimiter `gy`".to_owned(),
                    true,
                ),
            ],
            DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            usize::MAX,
        )
        .expect("one trailing newline");
        assert!(location_shows(&rendered, "--> ", 2, 1), "{rendered}");
        assert_eq!(
            caret_row(&rendered, "expected closing delimiter `gy`"),
            Some(2),
            "{rendered}"
        );
        assert_eq!(caret_row(&rendered, "opening gy"), Some(1), "{rendered}");
        assert_eq!(
            rendered.matches("expected closing delimiter `gy`").count(),
            1,
            "{rendered}"
        );
        assert_eq!(rendered.matches("opening gy").count(), 1, "{rendered}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ten_thousand_blank_lines_keep_the_eof_label() {
        let mut source = String::from("zoi gy");
        source.push_str(&"\n".repeat(10_000));
        let line = 10_001;
        let end = source.len();
        let rendered = render_label_set(
            &source,
            &[
                (0, 6, "opening gy".to_owned(), false),
                (end, end, "expected closing delimiter `gy`".to_owned(), true),
            ],
            DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            usize::MAX,
        )
        .expect("many trailing newlines");
        assert!(location_shows(&rendered, "--> ", line, 1), "{rendered}");
        assert_eq!(
            caret_row(&rendered, "expected closing delimiter `gy`"),
            Some(line),
            "{rendered}"
        );
        assert_eq!(caret_row(&rendered, "opening gy"), Some(1), "{rendered}");
        assert_eq!(
            rendered.matches("expected closing delimiter `gy`").count(),
            1,
            "{rendered}"
        );
        assert!(rendered.contains("zoi gy"), "{rendered}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_empty_first_label_does_not_hide_a_later_primary() {
        let mut source = String::from("\n");
        for _ in 0..9_999 {
            source.push_str("pad\n");
        }
        let far_start = source.len();
        source.push_str("FAR");
        let far_line = 1 + source[..far_start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count();
        assert_eq!(far_line, 10_001);
        let rendered = render_label_set(
            &source,
            &[
                (0, 0, "empty context".to_owned(), false),
                (far_start, source.len(), "far primary".to_owned(), true),
            ],
            DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            usize::MAX,
        )
        .expect("empty first label");
        assert!(location_shows(&rendered, "--> ", far_line, 1), "{rendered}");
        assert!(!location_shows(&rendered, "--> ", 1, 1), "{rendered}");
        assert_eq!(
            caret_row(&rendered, "far primary"),
            Some(far_line),
            "{rendered}"
        );
        assert_eq!(caret_row(&rendered, "empty context"), Some(1), "{rendered}");
        assert_eq!(rendered.matches("far primary").count(), 1, "{rendered}");
        assert_eq!(rendered.matches("empty context").count(), 1, "{rendered}");
        assert!(source_line_shows(&rendered, far_line, "FAR"), "{rendered}");
    }

    /// 64 label sets from seed `0x1127_0D13_5A11`.
    ///
    /// Widths are 80, 40, and 4.
    /// The invariant run uses a byte limit of 65536.
    /// A second run uses a byte limit of 64.
    /// Shape 0 is one multiline span on a short source.
    /// Shape 6 is one multiline span on a clipped source.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_anchors_keep_the_label_invariants() {
        const CASES: u32 = 64;
        const SEED: u64 = 0x1127_0D13_5A11;
        const LIMIT: usize = 65_536;
        let mut rng = LabelRng { state: SEED };
        let mut uncut = 0usize;
        let mut clipped = 0usize;
        let mut multiline_uncut = 0usize;
        let mut multiline_clipped = 0usize;
        let mut saw_eof = false;
        let mut saw_empty_line = false;
        let mut saw_overlap = false;
        let mut saw_distant = false;
        let mut saw_control = false;
        let mut saw_long_line = false;
        let mut saw_final_newline = false;
        let mut saw_no_final_newline = false;
        let mut saw_location_only = false;
        let mut saw_crlf_only = false;
        let mut saw_lone_cr = false;
        for case in 0..CASES {
            let set = random_label_set(&mut rng, case);
            let multiline = set.labels.iter().any(|(start, end, _, _)| {
                set.source
                    .get(*start..*end)
                    .is_some_and(|text| text.contains('\n'))
            });
            saw_eof |= set.eof_point;
            saw_empty_line |= set.empty_line;
            saw_overlap |= set.overlap;
            saw_distant |= set.distant;
            saw_control |= set.control;
            saw_long_line |= set.long_line;
            saw_location_only |= set.location_only;
            if set.source.ends_with('\n') {
                saw_final_newline = true;
            } else {
                saw_no_final_newline = true;
            }
            let rendered = render_label_set(&set.source, &set.labels, set.width, LIMIT)
                .unwrap_or_else(|error| panic!("case {case} failed: {error:?}"));
            let cut = rendered_excerpt_is_cut(&rendered);
            if cut {
                clipped += 1;
                if multiline {
                    multiline_clipped += 1;
                }
            } else {
                uncut += 1;
                if multiline {
                    multiline_uncut += 1;
                }
            }
            assert!(
                rendered.len() <= LIMIT,
                "case {case} length {}",
                rendered.len()
            );
            let crlf_only =
                set.source.contains("\r\n") && !source_has_lone_carriage_return(&set.source);
            if crlf_only {
                saw_crlf_only = true;
                assert!(
                    !rendered.contains('\u{240d}'),
                    "case {case} shows a carriage return mark\n{rendered}"
                );
            }
            if source_has_lone_carriage_return(&set.source) {
                saw_lone_cr = true;
            }
            let primary = set
                .labels
                .iter()
                .find(|label| label.3)
                .expect("one primary");
            let (line, column) = source_line_column(&set.source, primary.0);
            assert!(
                location_shows(&rendered, "--> ", line, column),
                "case {case} header {line}:{column}\n{rendered}"
            );
            for (start, end, message, _) in &set.labels {
                let end_at = if start == end {
                    *start
                } else {
                    end.saturating_sub(1)
                };
                let (label_line, _) = source_line_column(&set.source, end_at.min(set.source.len()));
                assert_eq!(
                    rendered.matches(message).count(),
                    1,
                    "case {case} label {message}\n{rendered}"
                );
                assert_eq!(
                    caret_row(&rendered, message),
                    Some(label_line),
                    "case {case} caret for {message} wanted line {label_line}\n{rendered}"
                );
            }
            let tight = render_label_set(&set.source, &set.labels, set.width, 64);
            match tight {
                Ok(text) => {
                    assert!(text.len() <= 64, "case {case} tight {}", text.len());
                    if crlf_only {
                        assert!(
                            !text.contains('\u{240d}'),
                            "case {case} tight shows a carriage return mark\n{text}"
                        );
                    }
                }
                Err(OutputError::Limit { .. }) => {}
                Err(error) => panic!("case {case} tight failed: {error:?}"),
            }
        }
        assert!(uncut > 0 && clipped > 0, "uncut {uncut} clipped {clipped}");
        assert!(
            multiline_uncut > 0 && multiline_clipped > 0,
            "multiline uncut {multiline_uncut} clipped {multiline_clipped}"
        );
        assert!(saw_eof && saw_empty_line && saw_overlap && saw_distant);
        assert!(saw_control && saw_long_line && saw_final_newline && saw_no_final_newline);
        assert!(saw_location_only);
        assert!(saw_crlf_only, "no CR LF source without a lone CR");
        assert!(saw_lone_cr, "no lone CR in the generated sources");
    }

    #[invariant(true)]
    struct LabelRng {
        state: u64,
    }

    impl LabelRng {
        #[requires(true)]
        #[ensures(true)]
        fn next(&mut self) -> u64 {
            let mut x = self.state;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.state = x;
            x
        }

        #[requires(n > 0)]
        #[ensures(ret < n)]
        fn below(&mut self, n: usize) -> usize {
            (self.next() as usize) % n
        }
    }

    #[invariant(true)]
    struct LabelSet {
        source: String,
        labels: Vec<(usize, usize, String, bool)>,
        width: usize,
        eof_point: bool,
        empty_line: bool,
        overlap: bool,
        distant: bool,
        control: bool,
        long_line: bool,
        location_only: bool,
    }

    #[requires(true)]
    #[ensures(true)]
    fn random_label_set(rng: &mut LabelRng, case: u32) -> LabelSet {
        let shape = case % 8;
        let width = if shape == 7 {
            4
        } else if case % 2 == 0 {
            80
        } else {
            40
        };
        let mut eof_point = false;
        let mut empty_line = false;
        let mut overlap = false;
        let mut distant = false;
        let mut control = false;
        let mut long_line = false;
        let mut location_only = false;
        let mut source = String::new();
        let mut labels = Vec::new();
        match shape {
            0 => {
                let len = rng.below(60);
                for _ in 0..len {
                    source.push(['a', 'b', ' ', '\n'][rng.below(4)]);
                }
                if !source.contains('\n') {
                    source.push('\n');
                }
                if source.starts_with('\n') || source.is_empty() {
                    source.insert(0, 'a');
                }
                labels.push((0, source.len(), "@@0@@".to_owned(), true));
            }
            1 => {
                source.push_str("zoi gy");
                source.push_str(&"\n".repeat(1 + rng.below(40)));
                labels.push((0, 6, "@@0@@".to_owned(), false));
                labels.push((source.len(), source.len(), "@@1@@".to_owned(), true));
                eof_point = true;
            }
            2 => {
                source.push_str(&"x".repeat(500 + rng.below(700)));
                long_line = true;
                labels.push((0, 1, "@@0@@".to_owned(), false));
                let end = source.len();
                labels.push((end - 1, end, "@@1@@".to_owned(), true));
                overlap = false;
            }
            3 => {
                source.push('\n');
                source.push_str(&"pad\n".repeat(40 + rng.below(30)));
                source.push_str("FAR");
                let far = source.len() - 3;
                labels.push((0, 0, "@@0@@".to_owned(), false));
                labels.push((far, source.len(), "@@1@@".to_owned(), true));
                empty_line = true;
                distant = true;
            }
            4 => {
                source.push_str("mi\t#\n");
                source.push_str(&" \u{85}".repeat(10 + rng.below(20)));
                if rng.below(2) == 0 {
                    source.push('\n');
                }
                control = true;
                push_scattered_labels(rng, &source, &mut labels, 2);
            }
            5 => {
                source.push_str("abcdef");
                source.push_str(&" ".repeat(rng.below(30)));
                labels.push((0, 3, "@@0@@".to_owned(), true));
                labels.push((0, 3, "@@1@@".to_owned(), false));
                labels.push((3, 6, "@@2@@".to_owned(), false));
                overlap = true;
            }
            6 => {
                source.push_str("head\n");
                source.push_str(&"\n".repeat(800 + rng.below(400)));
                source.push_str("tail");
                if rng.below(2) == 0 {
                    source.push('\n');
                    labels.push((source.len(), source.len(), "@@2@@".to_owned(), true));
                    eof_point = true;
                } else {
                    let tail = source.len() - 4;
                    labels.push((tail, source.len(), "@@2@@".to_owned(), true));
                }
                labels.insert(0, (0, source.len(), "@@0@@".to_owned(), false));
                labels.insert(1, (5, 5, "@@1@@".to_owned(), false));
                empty_line = true;
                distant = true;
            }
            _ => {
                source.push_str(&"q".repeat(240));
                long_line = true;
                location_only = true;
                for index in 0..40 {
                    let at = (index * 5).min(source.len());
                    labels.push((at, at, format!("m{index:02}x"), index + 1 == 40));
                }
            }
        }
        if !labels.iter().any(|label| label.3) {
            let last = labels.len() - 1;
            labels[last].3 = true;
        }
        // Case numbers choose the break bytes.
        // One third of the cases use CR LF.
        // One third of the cases insert one lone CR.
        // The other cases keep LF.
        // The choice does not call the random source.
        match case % 3 {
            1 => expand_lf_to_crlf(&mut source, &mut labels),
            2 => insert_lone_carriage_return(&mut source, &mut labels),
            _ => {}
        }
        if labels
            .iter()
            .any(|(start, _, _, _)| *start == source.len() && source.ends_with('\n'))
        {
            eof_point = true;
        }
        LabelSet {
            source,
            labels,
            width,
            eof_point,
            empty_line,
            overlap,
            distant,
            control,
            long_line,
            location_only,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn expand_lf_to_crlf(source: &mut String, labels: &mut [(usize, usize, String, bool)]) {
        if !source.contains('\n') {
            return;
        }
        let mut rewritten = String::with_capacity(source.len().saturating_mul(2));
        let mut map = vec![0usize; source.len() + 1];
        let mut old = 0usize;
        for character in source.chars() {
            let new_at = rewritten.len();
            let width = character.len_utf8();
            if character == '\n' {
                rewritten.push_str("\r\n");
                map[old] = new_at;
            } else {
                rewritten.push(character);
                for byte in 0..width {
                    map[old + byte] = new_at + byte;
                }
            }
            old += width;
        }
        map[source.len()] = rewritten.len();
        *source = rewritten;
        for label in labels.iter_mut() {
            label.0 = map[label.0];
            label.1 = map[label.1];
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn insert_lone_carriage_return(
        source: &mut String,
        labels: &mut [(usize, usize, String, bool)],
    ) {
        let old_len = source.len();
        let at = source
            .char_indices()
            .find(|(_, character)| *character != '\n' && *character != '\r')
            .map(|(index, _)| index)
            .unwrap_or(old_len);
        source.insert(at, '\r');
        if at < old_len {
            for label in labels.iter_mut() {
                if label.0 >= at {
                    label.0 += 1;
                }
                if label.1 >= at {
                    label.1 += 1;
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn source_has_lone_carriage_return(source: &str) -> bool {
        let bytes = source.as_bytes();
        let mut index = 0usize;
        while index < bytes.len() {
            if bytes[index] == b'\r' {
                if bytes.get(index + 1) != Some(&b'\n') {
                    return true;
                }
                index += 2;
            } else {
                index += 1;
            }
        }
        false
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_scattered_labels(
        rng: &mut LabelRng,
        source: &str,
        labels: &mut Vec<(usize, usize, String, bool)>,
        count: usize,
    ) {
        let marks = source
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(source.len()))
            .collect::<Vec<_>>();
        let count = count.max(1).min(6);
        for slot in 0..count {
            let start = char_boundary_at(source, marks[rng.below(marks.len())]);
            let line_end = line_end_at(source, start);
            let end = if line_end > start && rng.below(2) == 0 {
                let step = 1 + rng.below(line_end - start);
                char_boundary_at(source, (start + step).min(line_end)).clamp(start, line_end)
            } else {
                start
            };
            labels.push((start, end, format!("@@{slot}@@"), slot + 1 == count));
        }
    }

    #[requires(width > 0 && limit > 0)]
    #[ensures(true)]
    fn render_label_set(
        source: &str,
        labels: &[(usize, usize, String, bool)],
        width: usize,
        limit: usize,
    ) -> Result<String, OutputError> {
        let mut diagnostic_labels = Vec::new();
        for (start, end, message, primary) in labels {
            let span = source_span_from_byte_offsets(
                Some(SourceId("<input>".to_owned())),
                source,
                *start,
                *end,
            )
            .expect("generated span is on a character boundary");
            diagnostic_labels.push(DiagnosticLabel::new(span, message.clone(), *primary));
        }
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Error,
            DiagnosticPhase::Syntax,
            "syntax.sample".to_owned(),
            "sample".to_owned(),
            diagnostic_labels,
            Vec::new(),
            None,
        );
        render_diagnostics_with_limit(
            "<input>",
            source,
            &[diagnostic],
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::Ascii,
                terminal_width: width,
            }),
            ByteLimit::new(limit).expect("positive limit"),
        )
    }

    /// The numbered source row that holds this label text.
    #[requires(!message.is_empty())]
    #[ensures(true)]
    fn caret_row(rendered: &str, message: &str) -> Option<usize> {
        let mut source_row = None;
        let mut found = None;
        for line in rendered.lines() {
            if let Some((gutter, body)) = line.split_once('|') {
                if let Ok(number) = gutter.trim().parse::<usize>() {
                    if !body.contains('^') {
                        source_row = Some(number);
                    }
                }
            }
            if line.contains(message) {
                found = source_row;
            }
        }
        found
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn line_content_end_stops_before_crlf_and_keeps_a_lone_cr() {
        assert_eq!(line_content_end("a\r\n", 0), 1);
        assert_eq!(line_content_end("a\n", 0), 1);
        assert_eq!(line_content_end("a\r", 0), 2);
        assert_eq!(line_content_end("a\rb\n", 0), 3);
        assert_eq!(line_content_end("\r\n", 0), 0);
        assert_eq!(line_content_end("\r\n", 1), 0);
        assert_eq!(line_content_end("ab\r\ncd", 4), 6);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn crlf_excerpts_omit_the_carriage_return_mark() {
        let one_line = render_label_set(
            "mi ku\r\n",
            &[(3, 5, "@@ku@@".to_owned(), true)],
            80,
            65_536,
        )
        .expect("one line");
        assert!(one_line.contains("mi ku"), "{one_line}");
        assert!(!one_line.contains('\u{240d}'), "{one_line}");
        assert!(!one_line.contains('\r'), "{one_line}");

        let several = render_label_set(
            "mi\r\n\r\nku\r\n",
            &[(6, 8, "@@ku@@".to_owned(), true)],
            80,
            65_536,
        )
        .expect("several lines");
        assert!(several.contains("ku"), "{several}");
        assert!(!several.contains('\u{240d}'), "{several}");

        let multiline = render_label_set(
            "lo\r\nnu\r\n",
            &[(0, 6, "@@all@@".to_owned(), true)],
            80,
            65_536,
        )
        .expect("multiline");
        assert!(!multiline.contains('\u{240d}'), "{multiline}");
        assert!(!multiline.contains('\r'), "{multiline}");
        assert!(multiline.contains("lo"), "{multiline}");
        assert!(multiline.contains("nu"), "{multiline}");
        assert!(multiline.contains("@@all@@"), "{multiline}");
        let lines = multiline.lines().collect::<Vec<_>>();
        let nu_at = lines
            .iter()
            .position(|line| {
                split_gutter(line)
                    .is_some_and(|(gutter, body)| gutter.trim() == "2" && body.contains("nu"))
            })
            .expect("nu source line");
        let marks = lines.get(nu_at + 1).copied().unwrap_or("");
        assert!(marks.contains('^'), "{multiline}");

        let mut clipped = String::from("lo ");
        clipped.push_str(&"x".repeat(80));
        clipped.push_str("\r\nku\r\n");
        let end = clipped.len();
        let window = render_label_set(
            &clipped,
            &[(0, end, "@@span@@".to_owned(), true)],
            20,
            65_536,
        )
        .expect("clipped");
        assert!(!window.contains('\u{240d}'), "{window}");
        assert!(!window.contains('\r'), "{window}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_lone_carriage_return_stays_visible_on_its_line() {
        let rendered = render_label_set("lo\rnu", &[(3, 5, "@@nu@@".to_owned(), true)], 80, 65_536)
            .expect("lone CR");
        assert!(rendered.contains('\u{240d}'), "{rendered}");
        assert!(rendered.contains("lo"), "{rendered}");
        assert!(rendered.contains("nu"), "{rendered}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn distant_labels_skip_the_unannotated_lines() {
        let mut source = String::from("mi #\u{85}\n");
        for _ in 0..2_000 {
            source.push_str("pad\n");
        }
        let far_start = source.len();
        source.push_str("do");
        let near =
            source_span_from_byte_offsets(Some(SourceId("<input>".to_owned())), &source, 0, 2)
                .expect("span on mi");
        let far = source_span_from_byte_offsets(
            Some(SourceId("<input>".to_owned())),
            &source,
            far_start,
            source.len(),
        )
        .expect("span on do");
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Error,
            DiagnosticPhase::Syntax,
            "syntax.sample".to_owned(),
            "sample".to_owned(),
            vec![
                DiagnosticLabel::new(near, "near".to_owned(), true),
                DiagnosticLabel::new(far, "far".to_owned(), false),
            ],
            Vec::new(),
            None,
        );
        let options = new!(DiagnosticRenderOptions {
            color: false,
            detail: DiagnosticDetailMode::Summary,
            glyphs: GlyphStyle::Ascii,
            terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
        });
        let rendered =
            render_diagnostics("<input>", &source, &[diagnostic], options).expect("two lines");
        assert!(rendered.len() < 5_000, "{}", rendered.len());
        assert!(rendered.contains("\\u{85}"), "{rendered}");
        assert!(rendered.contains("do"), "{rendered}");
        assert!(!rendered.contains("pad"), "{rendered}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_excerpt_on_a_later_line_keeps_the_caret() {
        let source = "zz\nab\u{85}cd\u{7f}";
        let span =
            source_span_from_byte_offsets(Some(SourceId("<input>".to_owned())), source, 7, 9)
                .expect("span on cd");
        let diagnostic = Diagnostic::new(
            DiagnosticSeverity::Error,
            DiagnosticPhase::Morphology,
            "morphology.quote".to_owned(),
            "bad control".to_owned(),
            vec![DiagnosticLabel::new(span, "here".to_owned(), true)],
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
                glyphs: GlyphStyle::Ascii,
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
        )
        .expect("later line excerpt");
        assert!(!rendered.contains("zz"), "{rendered}");
        assert!(rendered.contains("ab\\u{85}cd\\u{7f}"), "{rendered}");
        assert!(!rendered.contains('\u{85}'), "{rendered}");
        let lines = rendered.lines().collect::<Vec<_>>();
        let source_line = lines
            .iter()
            .find(|line| line.contains("ab\\u{85}cd\\u{7f}"))
            .expect("excerpt line");
        let column = source_line
            .find("ab\\u{85}cd\\u{7f}")
            .expect("excerpt column");
        let marked = column + "ab\\u{85}".len();
        let caret_line = lines
            .iter()
            .find(|line| line.contains('^'))
            .expect("caret line");
        let caret_at = caret_line.find('^').expect("caret");
        assert_eq!(caret_at, marked, "{rendered}");
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
    fn styled_notes_reset_before_crlf_and_keep_blank_lines_bare() {
        let note = DiagnosticStyledNote::new(
            DiagnosticNoteMode::Summary,
            vec![DiagnosticTextSegment::new(
                DiagnosticTextRole::Construct,
                "hello\r\n\r\nworld".to_owned(),
            )],
        );
        let rendered = render_styled_note(
            &note,
            new!(DiagnosticRenderOptions {
                color: true,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: 80,
            }),
        );

        assert!(
            rendered
                .lines()
                .all(|line| line.is_empty() || line.ends_with("\x1b[39m")),
            "{rendered:?}"
        );
        assert_eq!(rendered.lines().nth(1), Some(""), "{rendered:?}");
        assert!(!rendered.contains("\r\x1b"), "{rendered:?}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn styled_notes_leave_whitespace_lines_and_lone_cr_unstyled() {
        let note = DiagnosticStyledNote::new(
            DiagnosticNoteMode::Summary,
            vec![DiagnosticTextSegment::new(
                DiagnosticTextRole::Construct,
                "hello\n   \n\t\nworld\rwide".to_owned(),
            )],
        );
        let rendered = render_styled_note(
            &note,
            new!(DiagnosticRenderOptions {
                color: true,
                detail: DiagnosticDetailMode::Summary,
                glyphs: GlyphStyle::default(),
                terminal_width: 80,
            }),
        );

        let mut lines = Vec::new();
        let mut rest = rendered.as_str();
        while let Some(index) = rest.find(['\n', '\r']) {
            lines.push(&rest[..index]);
            let width = if rest.as_bytes().get(index) == Some(&b'\r')
                && rest.as_bytes().get(index + 1) == Some(&b'\n')
            {
                2
            } else {
                1
            };
            rest = &rest[index + width..];
        }
        lines.push(rest);
        assert_eq!(
            lines,
            [
                "\x1b[97mhello\x1b[39m",
                "   ",
                "\t",
                "\x1b[97mworld\x1b[39m",
                "\x1b[97mwide\x1b[39m"
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn styled_notes_escape_controls_in_both_modes() {
        let note = DiagnosticStyledNote::new(
            DiagnosticNoteMode::Summary,
            vec![DiagnosticTextSegment::new(
                DiagnosticTextRole::Construct,
                "a\u{1b}b\u{9f}".to_owned(),
            )],
        );
        for color in [false, true] {
            let rendered = render_styled_note(
                &note,
                new!(DiagnosticRenderOptions {
                    color,
                    detail: DiagnosticDetailMode::Summary,
                    glyphs: GlyphStyle::default(),
                    terminal_width: 80,
                }),
            );
            assert!(rendered.contains("a\\u{1b}b\\u{9f}"), "{rendered:?}");
            assert!(!rendered.contains('\u{9f}'), "{rendered:?}");
            if !color {
                assert!(!rendered.contains('\u{1b}'), "{rendered:?}");
            }
        }
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
