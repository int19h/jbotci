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
#[invariant(*line_count >= 1)]
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
    /// One boundary table per joined line. Empty means the text is the raw slice.
    bounds: Vec<LineBound>,
    /// Source lines in this window, including a bridging line.
    line_count: usize,
    /// Byte ranges of synthetic cut marks in `escaped.text`.
    marks: Vec<Range<usize>>,
    /// Byte ranges of unstyled alignment gaps in `escaped.text`.
    gaps: Vec<Range<usize>>,
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
    // An uncut shown-line set is one raw snippet, including a folded gap.
    // The renderer draws that gap.
    if let Some((origin, end, line_start)) =
        whole_unclipped_excerpt(source, &anchors, &kept, columns, margin_mark)
    {
        let slice = source.get(origin..end).unwrap_or("");
        // The renderer draws this text. A second escape would change an uncut line.
        if slice.len() > max_copy {
            return Err(OutputError::Limit { limit });
        }
        return Ok(new!(DiagnosticSourceExcerpt {
            escaped: raw_terminal_excerpt(slice),
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
/// Original excerpt text. The renderer applies its own replacements.
#[requires(true)]
#[ensures(ret.map.is_empty() && ret.text.len() == text.len())]
fn raw_terminal_excerpt(text: &str) -> crate::styled::TerminalExcerpt<'_> {
    new!(crate::styled::TerminalExcerpt {
        text: std::borrow::Cow::Borrowed(text),
        map: Vec::new(),
    })
}

/// The labelled lines are one shown-line set, and the shared margin cuts none of them.
///
/// The margin is the one the renderer uses for the whole set.
/// A folded gap stays inside the range. The renderer draws the fold.
/// The returned range is original source.
#[requires(true)]
#[ensures(ret.is_none_or(|(origin, end, line)| origin <= end && end <= source.len() && line >= 1))]
fn whole_unclipped_excerpt(
    source: &str,
    anchors: &[DisplayAnchor],
    kept: &[annotate_snippets::renderer::KeptSourceLine],
    columns: usize,
    margin_mark: &str,
) -> Option<(usize, usize, usize)> {
    let first = kept.first()?;
    let last = kept.last()?;
    let origin = first.start_byte;
    let end = last.end_byte;
    let line_start = first.line_index;
    if origin > end || end > source.len() {
        return None;
    }
    let text = source.get(origin..end).unwrap_or("");
    let spans = run_label_spans(anchors, origin, end, kept.len() == 1);
    let laid = annotate_snippets::renderer::lay_out_run(
        text,
        &spans,
        columns,
        margin_mark,
        crate::styled::expanded_terminal_control,
    );
    if laid.len() != kept.len() || laid.iter().any(|line| line.laid.clipped) {
        return None;
    }
    Some((origin, end, line_start))
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
    // One shown-line set is one snippet. A larger gap is a fold inside it.
    push_run_windows(
        source,
        anchors,
        kept,
        columns,
        margin_mark,
        &mut covered,
        &mut pieces,
    );
    for anchor in anchors {
        if covered.get(anchor.label_index).copied().unwrap_or(true) {
            continue;
        }
        covered[anchor.label_index] = true;
        pieces.push(new!(RawPiece {
            text: String::new(),
            bounds: Vec::new(),
            marks: Vec::new(),
            gaps: Vec::new(),
            source_start: anchor.start.min(source.len()),
            source_end: anchor.start.min(source.len()),
            line_number: anchor.line,
            line_count: 1,
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
        let escaped = if data.bounds.is_empty() {
            let slice = source.get(data.source_start..data.source_end).unwrap_or("");
            if slice.len() > room {
                return Err(OutputError::Limit { limit });
            }
            raw_terminal_excerpt(slice)
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
            bounds: data.bounds,
            line_count: data.line_count,
            marks: data.marks,
            gaps: data.gaps,
        }));
    }
    Ok(windows)
}

/// One copied window before its header is known.
#[invariant(*source_start <= *source_end && *line_number >= 1 && *line_count >= 1)]
struct RawPiece {
    text: String,
    bounds: Vec<LineBound>,
    marks: Vec<Range<usize>>,
    gaps: Vec<Range<usize>>,
    source_start: usize,
    source_end: usize,
    line_number: usize,
    line_count: usize,
    labels: Vec<usize>,
}

/// Source bytes of one prepared line, mapped like an escape table.
///
/// `map` has one entry per source byte plus the exclusive end.
/// Entry `i` is the window byte where source byte `source_start + i` begins.
#[invariant(*source_end >= *source_start)]
#[invariant(map.len() == source_end.saturating_sub(*source_start).saturating_add(1))]
struct LineBound {
    source_start: usize,
    source_end: usize,
    map: Vec<usize>,
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
    let run_start = run[0].start_byte;
    let run_end = run[run.len() - 1].end_byte;
    let text = source.get(run_start..run_end).unwrap_or("");
    let spans = run_label_spans(anchors, run_start, run_end, run.len() == 1);
    let laid_lines = annotate_snippets::renderer::lay_out_run(
        text,
        &spans,
        columns,
        margin_mark,
        crate::styled::expanded_terminal_control,
    );
    let mut layouts = Vec::new();
    let mut any_cut = false;
    for line in run {
        let relative = line
            .line_index
            .saturating_sub(run[0].line_index)
            .saturating_add(1);
        let laid = laid_lines
            .iter()
            .find(|item| item.line_index == relative)
            .map(|item| item.laid.clone())
            .unwrap_or_else(|| annotate_snippets::renderer::LaidOutLine {
                text: String::new(),
                origin: Vec::new(),
                marks: Vec::new(),
                gaps: Vec::new(),
                clipped: false,
            });
        if laid.clipped {
            any_cut = true;
        }
        layouts.push(laid);
    }
    let labels = labels_on_run(anchors, run);
    if labels.is_empty() {
        return;
    }
    mark_covered(covered, &labels);
    if !any_cut {
        pieces.push(new!(RawPiece {
            text: String::new(),
            bounds: Vec::new(),
            marks: Vec::new(),
            gaps: Vec::new(),
            source_start: run[0].start_byte,
            source_end: run[run.len() - 1].end_byte,
            line_number: run[0].line_index,
            line_count: run.len().max(1),
            labels,
        }));
        return;
    }
    // One snippet for the run. Every line keeps the planned window.
    // A later render does not trim or escape that window again.
    pieces.push(joined_run_piece(source, anchors, run, layouts, labels));
}

#[requires(run.len() == layouts.len())]
#[ensures(!ret.bounds.is_empty() && ret.source_start <= ret.source_end && ret.line_number >= 1 && ret.line_count >= 1)]
fn joined_run_piece(
    source: &str,
    anchors: &[DisplayAnchor],
    run: &[annotate_snippets::renderer::KeptSourceLine],
    layouts: Vec<annotate_snippets::renderer::LaidOutLine>,
    labels: Vec<usize>,
) -> RawPiece {
    let mut text = String::new();
    let mut bounds = Vec::new();
    let mut marks = Vec::new();
    let mut gaps = Vec::new();
    let last = run.len().saturating_sub(1);
    for (index, (line, laid)) in run.iter().zip(layouts).enumerate() {
        if index > 0 {
            // The bytes between shown lines are the fold. The renderer draws it.
            // An empty shown line already consumed its break, so the slice is empty.
            let previous = &run[index - 1];
            let gap = source.get(previous.end_byte..line.start_byte).unwrap_or("");
            if gap.is_empty() {
                text.push('\n');
            } else {
                text.push_str(gap);
            }
        }
        // An uncut line uses the same laid-out text as a cut line.
        // That text is the escaped form the budget charges.
        let base = text.len();
        text.push_str(&laid.text);
        let content = laid_line_content(source, line);
        let cover_break = index == last && span_covers_trailing_break(source, anchors, line);
        let (map, append_newline) = source_boundary(&content, &laid.origin, base, cover_break);
        if append_newline {
            text.push('\n');
        }
        let source_start = line.start_byte;
        let source_end = source_start + content.len() + usize::from(cover_break);
        bounds.push(new!(LineBound {
            source_start,
            source_end,
            map,
        }));
        for range in laid.marks {
            marks.push(base + range.start..base + range.end);
        }
        for range in laid.gaps {
            gaps.push(base + range.start..base + range.end);
        }
    }
    let line_number = run.first().map(|line| line.line_index).unwrap_or(1);
    let source_start = bounds.first().map(|line| line.source_start).unwrap_or(0);
    let source_end = bounds
        .last()
        .map(|line| line.source_end)
        .unwrap_or(source_start);
    new!(RawPiece {
        text,
        bounds,
        marks,
        gaps,
        source_start,
        source_end,
        line_number,
        line_count: run.len().max(1),
        labels,
    })
}

/// The line text the layout origins refer to. A kept empty line includes its break.
#[requires(true)]
#[ensures(ret.len() <= source.len())]
fn laid_line_content<'a>(
    source: &'a str,
    line: &annotate_snippets::renderer::KeptSourceLine,
) -> &'a str {
    let end = line.end_byte.min(source.len());
    let start = line.start_byte.min(end);
    let slice = source.get(start..end).unwrap_or("");
    if slice.as_bytes().last() == Some(&b'\n') {
        let without_line_feed = &slice[..slice.len() - 1];
        if without_line_feed.is_empty() || without_line_feed == "\r" {
            return "";
        }
    }
    slice
}

/// The multiline span consumes the break after this kept line.
#[requires(true)]
#[ensures(!ret || anchors.iter().any(|anchor| anchor.start < anchor.end))]
fn span_covers_trailing_break(
    source: &str,
    anchors: &[DisplayAnchor],
    line: &annotate_snippets::renderer::KeptSourceLine,
) -> bool {
    let break_at = line.end_byte;
    if break_at == line.start_byte {
        return false;
    }
    let Some(byte) = source.as_bytes().get(break_at).copied() else {
        return false;
    };
    if byte != b'\n' && byte != b'\r' {
        return false;
    }
    anchors.iter().any(|anchor| {
        anchor.start != anchor.end && anchor.start < break_at && anchor.end > break_at
    })
}

/// Map one laid-out line the way an escape table maps raw text.
///
/// The table has `content.len() + 1` entries, plus one when `trailing_newline`
/// is set. A cut byte points at its mark. A point at the exclusive end points
/// at the display end when the right side is not cut.
/// The bool is true when the caller must append the trailing break.
#[requires(true)]
#[ensures(ret.0.len() == content.len() + usize::from(trailing_newline) + 1)]
fn source_boundary(
    content: &str,
    origin: &[usize],
    display_base: usize,
    trailing_newline: bool,
) -> (Vec<usize>, bool) {
    let content_len = content.len();
    let source_len = content_len + usize::from(trailing_newline);
    let mut map = vec![display_base; source_len + 1];
    let mut glyph_at = vec![None; content_len + 1];
    let mut index = 0usize;
    while index < origin.len() {
        let src = origin[index];
        let start = index;
        while index < origin.len() && origin[index] == src {
            index += 1;
        }
        if src != usize::MAX && src <= content_len && glyph_at[src].is_none() {
            glyph_at[src] = Some(display_base + start);
        }
    }
    let left_mark = if origin.first().copied() == Some(usize::MAX) {
        Some(display_base)
    } else {
        None
    };
    let mut right_rel = origin.len();
    while right_rel > 0 && origin[right_rel - 1] == usize::MAX {
        right_rel -= 1;
    }
    let right_mark = if right_rel < origin.len() && right_rel > 0 {
        Some(display_base + right_rel)
    } else {
        None
    };
    let display_end = display_base + origin.len();
    let first_real = glyph_at.iter().position(|slot| slot.is_some());
    let last_real = glyph_at.iter().rposition(|slot| slot.is_some());
    for (start, ch) in content.char_indices() {
        let next = start + ch.len_utf8();
        let before_visible = match first_real {
            None => true,
            Some(first) => start < first,
        };
        if let Some(at) = glyph_at[start] {
            let rel = at - display_base;
            let mut end = rel;
            while end < origin.len() && origin[end] == start {
                end += 1;
            }
            let glyph_end = display_base + end;
            for slot in map.iter_mut().take(next).skip(start) {
                *slot = at;
            }
            if next <= source_len {
                map[next] = glyph_end;
            }
        } else if before_visible {
            let at = left_mark.unwrap_or(display_base);
            for slot in map.iter_mut().take(next).skip(start) {
                *slot = at;
            }
            if next <= source_len {
                map[next] = at;
            }
        } else {
            let at = if last_real.is_some_and(|last| start > last) {
                right_mark.unwrap_or(display_end)
            } else {
                middle_mark_display(start, origin, &glyph_at, display_base)
            };
            for slot in map.iter_mut().take(next).skip(start) {
                *slot = at;
            }
            if next <= source_len {
                map[next] = at;
            }
        }
    }
    let append_newline = trailing_newline && right_mark.is_none();
    if trailing_newline {
        let newline_at = content_len;
        if let Some(at) = right_mark {
            map[newline_at] = at;
            map[newline_at + 1] = at;
        } else {
            map[newline_at] = display_end;
            map[newline_at + 1] = display_end + 1;
        }
    } else if let Some(at) = right_mark {
        map[content_len] = at;
    } else {
        map[content_len] = display_end;
    }
    (map, append_newline)
}

#[requires(source_byte <= glyph_at.len())]
#[ensures(ret >= display_base)]
fn middle_mark_display(
    source_byte: usize,
    origin: &[usize],
    glyph_at: &[Option<usize>],
    display_base: usize,
) -> usize {
    let prev = glyph_at
        .get(..source_byte)
        .and_then(|slots| slots.iter().rposition(|slot| slot.is_some()));
    let prev_end = prev
        .and_then(|byte| {
            let at = glyph_at.get(byte).and_then(|slot| *slot)?;
            let rel = at - display_base;
            let mut end = rel;
            while end < origin.len() && origin[end] == byte {
                end += 1;
            }
            Some(end)
        })
        .unwrap_or(0);
    let next_start = glyph_at
        .get(source_byte + 1..)
        .and_then(|slots| slots.iter().position(|slot| slot.is_some()))
        .map(|offset| source_byte + 1 + offset)
        .and_then(|byte| glyph_at.get(byte).and_then(|slot| *slot))
        .map(|at| at - display_base)
        .unwrap_or(origin.len());
    let mut index = prev_end;
    while index < next_start && index < origin.len() {
        if origin[index] == usize::MAX {
            return display_base + index;
        }
        index += 1;
    }
    display_base + prev_end.min(origin.len())
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
#[ensures(ret.iter().all(|(start, end, _columns)| *start <= *end))]
fn run_label_spans(
    anchors: &[DisplayAnchor],
    run_start: usize,
    run_end: usize,
    single_line: bool,
) -> Vec<(usize, usize, usize)> {
    let len = run_end.saturating_sub(run_start);
    let mut spans = Vec::new();
    for anchor in anchors {
        if !anchor_mark_inside(anchor, run_start, run_end) {
            continue;
        }
        let crosses = anchor.start < run_start || anchor.end > run_end;
        if single_line && crosses && anchor.start != anchor.end {
            if anchor.start < run_start && anchor.end > run_end {
                continue;
            }
            if anchor.start >= run_start && anchor.end > run_end {
                let start = anchor.start.saturating_sub(run_start).min(len);
                spans.push((start, start, 0));
                continue;
            }
            if anchor.start < run_start && anchor.end <= run_end {
                let end = anchor.end.saturating_sub(run_start).min(len);
                spans.push((end, end, anchor.label_columns.max(1)));
                continue;
            }
        }
        let start = anchor.start.saturating_sub(run_start).min(len);
        let end = anchor.end.saturating_sub(run_start).min(len).max(start);
        spans.push((start, end, anchor.label_columns.max(1)));
    }
    spans
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

/// Map a source span through each prepared line's boundary table.
#[requires(true)]
#[ensures(ret.start <= ret.end && ret.end <= text_len)]
fn map_through_bounds(
    bounds: &[LineBound],
    text_len: usize,
    start: usize,
    end: usize,
) -> Range<usize> {
    if start == end {
        return map_point(bounds, text_len, start);
    }
    let end = end.max(start);
    let mut from = None;
    let mut to = 0usize;
    for line in bounds {
        if end <= line.source_start || start >= line.source_end {
            continue;
        }
        let last = line.map.len().saturating_sub(1);
        let local_start = start.max(line.source_start) - line.source_start;
        let local_end = end.min(line.source_end) - line.source_start;
        let mapped_from = line.map[local_start.min(last)].min(text_len);
        let mapped_to = line.map[local_end.min(last)].min(text_len);
        if from.is_none() {
            from = Some(mapped_from);
        }
        to = to.max(mapped_to);
    }
    let from = from.unwrap_or_else(|| map_point(bounds, text_len, start).start);
    let to = to.max(from).min(text_len);
    from.min(text_len)..to
}

/// A point stays zero-width. The end of an uncut line is the display end.
#[requires(true)]
#[ensures(ret.start == ret.end && ret.end <= text_len)]
fn map_point(bounds: &[LineBound], text_len: usize, point: usize) -> Range<usize> {
    for line in bounds {
        if point >= line.source_start && point < line.source_end {
            let index = point - line.source_start;
            let at = line.map.get(index).copied().unwrap_or(0).min(text_len);
            return at..at;
        }
    }
    if let Some(line) = bounds.iter().rev().find(|line| point == line.source_end) {
        let at = line.map.last().copied().unwrap_or(0).min(text_len);
        return at..at;
    }
    if let Some(line) = bounds.first() {
        if point < line.source_start {
            let at = line.map.first().copied().unwrap_or(0).min(text_len);
            return at..at;
        }
    }
    if let Some(line) = bounds.last() {
        let at = line.map.last().copied().unwrap_or(0).min(text_len);
        return at..at;
    }
    0..0
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
                .filter_map(|((index, label), label_message)| {
                    let span = mapped_window_span(label, window)?;
                    let mut annotation = annotation_kind(label.primary).span(span);
                    if window_shows_label_text(&excerpt.retained, window, index, label) {
                        annotation = annotation.label(label_message.as_ref());
                    }
                    Some(annotation)
                })
                .collect::<Vec<_>>();
            let mut snippet = Snippet::source(window.escaped.text.as_ref())
                .line_start(window.line_number)
                .origin_position(window.header_line, window.header_column)
                .path(source_label)
                .annotations(annotations);
            if !window.bounds.is_empty() {
                snippet = snippet.prepared_window(window.marks.clone(), window.gaps.clone());
            }
            group = group.element(snippet);
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

/// The label's bytes in this window. A pass-through span on one line is absent.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|span| span.start <= span.end))]
fn mapped_window_span(
    label: &DiagnosticLabel,
    window: &RetainedWindow<'_>,
) -> Option<Range<usize>> {
    let span = label_span(label);
    let mut start = span.start;
    let mut end = span.end.max(start);
    let crosses = start < window.source_start || end > window.source_end;
    if window.line_count == 1 && crosses && start != end {
        if start < window.source_start && end > window.source_end {
            return None;
        }
        if start >= window.source_start && end > window.source_end {
            end = start;
        } else if start < window.source_start && end <= window.source_end {
            start = end;
        }
    }
    let text_len = window.escaped.text.len();
    if window.bounds.is_empty() {
        return Some(apply_escape_map(
            start.saturating_sub(window.source_start),
            end.saturating_sub(window.source_start),
            &window.escaped.map,
            text_len,
        ));
    }
    Some(map_through_bounds(&window.bounds, text_len, start, end))
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
    use unicode_width::UnicodeWidthStr;

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
        assert!(rendered.contains('\u{85}'), "{rendered}");
        assert!(rendered.contains('␡'), "{rendered}");
        assert!(!rendered.contains("\\u{85}"), "{rendered}");
        assert!(!rendered.contains('\u{7f}'), "{rendered}");
        let lines = rendered.lines().collect::<Vec<_>>();
        let source_line = lines
            .iter()
            .find(|line| line.contains('\u{85}') && line.contains("cd"))
            .expect("excerpt line");
        let marked = UnicodeWidthStr::width(&source_line[..source_line.find("cd").expect("cd")]);
        let caret_line = lines
            .iter()
            .find(|line| line.contains('^'))
            .expect("caret line");
        let caret_at = caret_line.find('^').expect("caret");
        assert_eq!(
            UnicodeWidthStr::width(&caret_line[..caret_at]),
            marked,
            "{rendered}"
        );
    }

    /// `go` plus BEL, ESC, DEL, NEL, CSI, and APC, then ` ku`.
    /// The escaped form is 40 bytes. The raw line is 14 bytes.
    const ESCAPED_CONTROL_LINE: &str = "go\\u{7}\\u{1b}\\u{7f}\\u{85}\\u{9b}\\u{9f} ku";

    #[requires(true)]
    #[ensures(ret.len() == 14)]
    fn raw_control_line() -> String {
        format!("go\u{7}\u{1b}\u{7f}\u{85}\u{9b}\u{9f} ku")
    }

    #[requires(end <= source.len())]
    #[ensures(!ret.labels.is_empty())]
    fn span_diagnostic(source: &str, end: usize) -> Diagnostic {
        let span =
            source_span_from_byte_offsets(Some(SourceId("<input>".to_owned())), source, 0, end)
                .expect("span");
        Diagnostic::new(
            DiagnosticSeverity::Error,
            DiagnosticPhase::Syntax,
            "syntax.sample".to_owned(),
            "sample".to_owned(),
            vec![DiagnosticLabel::new(span, "here".to_owned(), true)],
            Vec::new(),
            None,
        )
    }

    #[requires(true)]
    #[ensures(ret.terminal_width == DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH && !ret.color)]
    fn ascii_excerpt_options() -> DiagnosticRenderOptions {
        new!(DiagnosticRenderOptions {
            color: false,
            detail: DiagnosticDetailMode::Summary,
            glyphs: GlyphStyle::Ascii,
            terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
        })
    }

    /// The uncut line is escaped text. The budget accepts that text and rejects
    /// a budget that only covers the raw line.
    #[requires(span_end <= source.len() && (line_index == 0 || line_index == 1))]
    #[ensures(true)]
    fn assert_prepared_control_charge(source: &str, span_end: usize, line_index: usize) {
        let diagnostic = span_diagnostic(source, span_end);
        let options = ascii_excerpt_options();
        let limit = ByteLimit::new(usize::MAX).expect("positive limit");
        let excerpt =
            diagnostic_source_excerpt(source, &diagnostic, options, usize::MAX, limit, "<input>")
                .expect("excerpt");
        assert_eq!(excerpt.retained.len(), 1, "one prepared window");
        let window = &excerpt.retained[0];
        assert!(!window.bounds.is_empty(), "prepared text");
        let lines: Vec<&str> = window.escaped.text.lines().collect();
        assert!(lines.len() >= 2, "{}", window.escaped.text);
        assert_eq!(
            lines[line_index], ESCAPED_CONTROL_LINE,
            "{}",
            window.escaped.text
        );
        assert_eq!(ESCAPED_CONTROL_LINE.len(), 40);
        assert!(!lines[line_index].contains("..."), "uncut line was cut");
        let cut_index = 1 - line_index;
        assert!(
            lines[cut_index].contains("..."),
            "cut line missing mark: {lines:?}"
        );
        for control in [
            '\u{0007}', '\u{001b}', '\u{007f}', '\u{0085}', '\u{009b}', '\u{009f}',
        ] {
            assert!(
                !window.escaped.text.contains(control),
                "raw {control:?} in {}",
                window.escaped.text
            );
        }
        let charged = window.escaped.text.len();
        // The raw line is 14 bytes. Its escaped form is 40 bytes.
        let raw_gap = 40 - 14;
        let under = diagnostic_source_excerpt(
            source,
            &diagnostic,
            options,
            charged - raw_gap,
            limit,
            "<input>",
        );
        assert!(
            matches!(under, Err(OutputError::Limit { .. })),
            "raw-sized budget accepted the escaped window"
        );
        let short =
            diagnostic_source_excerpt(source, &diagnostic, options, charged - 2, limit, "<input>");
        assert!(
            matches!(short, Err(OutputError::Limit { .. })),
            "one byte under the escaped length was accepted"
        );
        let fits =
            diagnostic_source_excerpt(source, &diagnostic, options, charged - 1, limit, "<input>")
                .expect("escaped length fits");
        assert_eq!(fits.retained.len(), 1);
        assert_eq!(
            fits.retained[0].escaped.text.as_ref(),
            window.escaped.text.as_ref()
        );
        assert_eq!(fits.retained[0].escaped.text.len(), charged);

        let rendered =
            render_diagnostics("<input>", source, &[diagnostic], options).expect("render");
        for control in [
            '\u{0007}', '\u{001b}', '\u{007f}', '\u{0085}', '\u{009b}', '\u{009f}',
        ] {
            assert!(!rendered.contains(control), "{rendered:?}");
        }
        let body = rendered_source_text(&rendered, line_index + 1);
        assert_eq!(body, ESCAPED_CONTROL_LINE, "{rendered}");
    }

    #[requires(true)]
    #[ensures(true)]
    fn rendered_source_text(rendered: &str, line_number: usize) -> String {
        let body = split_gutter(numbered_source_body(rendered, line_number))
            .expect("gutter")
            .1;
        let mut rest = body.trim();
        loop {
            let trimmed = rest.trim_start();
            let Some(mark) = trimmed.chars().next() else {
                return String::new();
            };
            if !matches!(mark, '|' | '│' | '┃' | '/' | '┏') {
                return trimmed.to_owned();
            }
            rest = &trimmed[mark.len_utf8()..];
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_uncut_line_before_a_cut_charges_the_escaped_controls() {
        // The span stops at the start of the long line. A span over every
        // column of that line is not a cut.
        let mut source = raw_control_line();
        source.push('\n');
        let span_end = source.len() + 4;
        source.push_str(&"a".repeat(200));
        assert_prepared_control_charge(&source, span_end, 0);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_uncut_line_after_a_cut_charges_the_escaped_controls() {
        let mut source = "a".repeat(200);
        source.push('\n');
        source.push_str(&raw_control_line());
        assert_prepared_control_charge(&source, source.len(), 1);
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
        // Two short labels do not cut the line. The renderer draws the whole gap.
        let rendered = render_diagnostics("<input>", &source, &[diagnostic.clone()], options)
            .expect("unclipped diagnostic");
        assert!(
            rendered.len() > source.len(),
            "rendered {} source {}",
            rendered.len(),
            source.len()
        );
        assert!(rendered.contains('\u{85}'), "missing control");
        assert!(!rendered.contains("\\u{85}"), "uncut line was pre-escaped");
        assert!(rendered.contains("near"), "missing near label");
        assert!(rendered.contains("far"), "missing far label");
        assert!(rendered.contains("do"), "missing far word");
        assert!(!rendered.contains(":::"), "unexpected second window");
        assert!(
            location_shows(&rendered, "--> ", 1, 1),
            "primary location missing"
        );
        assert!(
            source_line_shows(&rendered, 1, "mi klama"),
            "near word missing from line 1"
        );
        assert!(
            source_line_shows(&rendered, 1, "do"),
            "far word missing from line 1"
        );
        assert!(
            !source_line_shows(&rendered, 3, "do"),
            "far word moved to another snippet"
        );

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
        assert!(!rendered.contains(":::"), "{rendered}");
        assert!(rendered.contains("..."), "{rendered}");
        assert!(source_line_shows(&rendered, 1, "NEAR"), "{rendered}");
        assert!(source_line_shows(&rendered, far_line, "FAR"), "{rendered}");
        assert!(!source_line_shows(&rendered, 3, "FAR"), "{rendered}");
        assert!(rendered.matches("pad").count() < 30, "{rendered}");
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
        assert!(!rendered.contains(":::"), "{rendered}");
        assert!(rendered.contains("..."), "{rendered}");
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
        render_parser_diagnostics_with(source, GlyphStyle::default())
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn render_parser_diagnostics_with(source: &str, glyphs: GlyphStyle) -> String {
        render_parser_diagnostics_colored(source, glyphs, false)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn parser_diagnostics(source: &str) -> Vec<Diagnostic> {
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
        diagnostics
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn render_parser_diagnostics_colored(source: &str, glyphs: GlyphStyle, color: bool) -> String {
        render_diagnostics(
            "<input>",
            source,
            &parser_diagnostics(source),
            new!(DiagnosticRenderOptions {
                color,
                detail: DiagnosticDetailMode::Summary,
                glyphs,
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
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
    fn an_unclipped_multi_label_line_in_the_width_band_stays_one_window() {
        let source = format!("lo {}ku ku", "barda ".repeat(15));
        let rendered = render_parser_diagnostics(&source);
        let body = excerpt_body(&rendered);
        assert_eq!(body, source, "{rendered}");
        assert!(!rendered.contains('⸬'), "{rendered}");
        assert!(!rendered.contains(":::"), "{rendered}");
        assert!(
            rendered.contains("while parsing term connection"),
            "{rendered}"
        );
        assert_caret_on(&rendered, "ku");
    }

    #[requires(true)]
    #[ensures(ret > 0)]
    fn numbered_line_width(rendered: &str, line_number: usize) -> usize {
        unicode_width::UnicodeWidthStr::width(numbered_source_body(rendered, line_number))
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_single_snippet(rendered: &str) {
        assert!(!rendered.contains('⸬'), "{rendered}");
        assert!(!rendered.contains(":::"), "{rendered}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_line_that_ends_with_the_margin_mark_stays_within_the_terminal() {
        for total in 76usize..=80 {
            let ascii_source = format!(
                "kkk {}...",
                &("mi klama ".repeat(20))[..total.saturating_sub(7)]
            );
            let ascii = render_parser_diagnostics_with(&ascii_source, GlyphStyle::Ascii);
            assert!(
                numbered_line_width(&ascii, 1) <= DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
                "{ascii}"
            );
            assert_single_snippet(&ascii);
            let ascii_body = excerpt_body(&ascii);
            if (77..=79).contains(&total) {
                assert!(ascii_body.ends_with("kla..."), "{ascii_body}");
            }

            let unicode_source = format!("kkk zoi gy. {}…", "a".repeat(total.saturating_sub(13)));
            let unicode = render_parser_diagnostics(&unicode_source);
            assert!(
                numbered_line_width(&unicode, 1) <= DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
                "{unicode}"
            );
            assert_single_snippet(&unicode);
            assert!(excerpt_body(&unicode).ends_with('…'), "{unicode}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_wide_span_ascii_tail_keeps_three_letters_before_the_mark() {
        let source = format!(
            "zoi gy. {} .gy {} {}...",
            "漢".repeat(10),
            "k".repeat(100),
            "a".repeat(5)
        );
        let rendered = render_parser_diagnostics_with(&source, GlyphStyle::Ascii);
        let body = excerpt_body(&rendered);
        // The shared margin keeps this span wider than the terminal, as main does.
        assert!(body.ends_with("aaa..."), "{body}");
        assert!(!body.contains("aaaaa"), "{body}");
        assert_single_snippet(&rendered);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_cut_mark_uses_the_line_number_color() {
        let source = format!("kkk zoi gy. {}…", "a".repeat(77usize.saturating_sub(13)));
        let rendered = render_parser_diagnostics_colored(&source, GlyphStyle::Unicode, true);
        assert!(
            rendered.contains("\u{1b}[1m\u{1b}[94m…\u{1b}[0m"),
            "{rendered:?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn an_indented_second_line_keeps_the_first_line_in_one_snippet() {
        let spaces = format!("{}mi nelci lo\nku ku", " ".repeat(27));
        let spaced = render_parser_diagnostics(&spaces);
        assert_single_snippet(&spaced);
        assert!(
            numbered_source_body(&spaced, 1).contains(&format!("{}mi nelci", " ".repeat(27))),
            "{spaced}"
        );
        assert!(
            numbered_source_body(&spaced, 2).contains("ku ku"),
            "{spaced}"
        );

        let tabs = format!("{}mi nelci lo\nku ku", "\t".repeat(7));
        let tabbed = render_parser_diagnostics(&tabs);
        assert_single_snippet(&tabbed);
        assert!(
            numbered_source_body(&tabbed, 1).contains(&format!("{}mi nelci", " ".repeat(28))),
            "{tabbed}"
        );
        assert!(
            numbered_source_body(&tabbed, 2).contains("ku ku"),
            "{tabbed}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_long_multiline_start_stays_in_one_snippet() {
        let source = format!("mi cusku zoi gy. {}\ndef", "a".repeat(90));
        let rendered = render_parser_diagnostics(&source);
        assert_single_snippet(&rendered);
        let body = excerpt_body(&rendered);
        assert!(body.contains("mi cusku zoi"), "{body}");
        assert!(body.ends_with('…'), "{body}");
        assert!(
            numbered_line_width(&rendered, 1) <= DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            "{rendered}"
        );
        assert!(
            numbered_source_body(&rendered, 2).contains("def"),
            "{rendered}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_parser_line_after_a_cut_charges_the_escaped_controls() {
        let source = format!("mi cusku zoi gy. {}\ndef\u{85}\u{9f}", "a".repeat(90));
        let escaped_tail = "def\\u{85}\\u{9f}";
        assert_eq!(escaped_tail.len(), 15);
        let options = new!(DiagnosticRenderOptions {
            color: false,
            detail: DiagnosticDetailMode::Summary,
            glyphs: GlyphStyle::Unicode,
            terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
        });
        let limit = ByteLimit::new(usize::MAX).expect("positive limit");
        let mut charged_window = None;
        for diagnostic in parser_diagnostics(&source) {
            let excerpt = diagnostic_source_excerpt(
                &source,
                &diagnostic,
                options,
                usize::MAX,
                limit,
                "<input>",
            )
            .expect("excerpt");
            for window in &excerpt.retained {
                if window.escaped.text.contains(escaped_tail) {
                    charged_window = Some(window.escaped.text.clone());
                }
            }
        }
        let text = charged_window.expect("prepared window with the escaped tail");
        let tail = text
            .lines()
            .find(|line| line.contains("def"))
            .expect("def line");
        assert_eq!(tail, escaped_tail, "{text}");
        assert!(!tail.contains('…'), "{tail}");
        assert!(!text.contains('\u{85}'), "{text:?}");
        assert!(!text.contains('\u{9f}'), "{text:?}");
        assert!(!text.contains('\u{9b}'), "{text:?}");
        let charged = text.len();
        // U+0085 and U+009F are 4 raw bytes and 12 escaped bytes.
        let raw_gap = 12 - 4;
        let diagnostic = parser_diagnostics(&source)
            .into_iter()
            .find(|diagnostic| {
                diagnostic_source_excerpt(
                    &source,
                    diagnostic,
                    options,
                    usize::MAX,
                    limit,
                    "<input>",
                )
                .ok()
                .is_some_and(|excerpt| {
                    excerpt
                        .retained
                        .iter()
                        .any(|window| window.escaped.text.contains(escaped_tail))
                })
            })
            .expect("diagnostic");
        let under = diagnostic_source_excerpt(
            &source,
            &diagnostic,
            options,
            charged - raw_gap,
            limit,
            "<input>",
        );
        assert!(
            matches!(under, Err(OutputError::Limit { .. })),
            "raw-sized budget accepted the escaped window"
        );
        let short =
            diagnostic_source_excerpt(&source, &diagnostic, options, charged - 2, limit, "<input>");
        assert!(
            matches!(short, Err(OutputError::Limit { .. })),
            "one byte under the escaped length was accepted"
        );
        let fits =
            diagnostic_source_excerpt(&source, &diagnostic, options, charged - 1, limit, "<input>")
                .expect("escaped length fits");
        assert!(
            fits.retained
                .iter()
                .any(|window| window.escaped.text.as_ref() == text.as_ref())
        );
        let rendered =
            render_diagnostics("<input>", &source, &parser_diagnostics(&source), options)
                .expect("render");
        let body = rendered_source_text(&rendered, 2);
        assert_eq!(body, escaped_tail, "{rendered}");
        assert!(!rendered.contains('\u{85}'), "{rendered:?}");
        assert!(!rendered.contains('\u{9f}'), "{rendered:?}");
        assert!(!rendered.contains('\u{9b}'), "{rendered:?}");
        let colored = render_parser_diagnostics_colored(&source, GlyphStyle::Unicode, true);
        assert!(colored.contains(escaped_tail), "{colored:?}");
        assert!(!colored.contains('\u{85}'), "{colored:?}");
        assert!(!colored.contains('\u{9f}'), "{colored:?}");
        assert!(!colored.contains('\u{9b}'), "{colored:?}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_short_line_before_a_long_span_stays_left_cut() {
        let source = format!("mi nelci lo\n{}ku ku", "barda ".repeat(25));
        let rendered = render_parser_diagnostics(&source);
        assert_single_snippet(&rendered);
        let first = excerpt_body(&rendered);
        assert!(first.contains("elci lo"), "{first}");
        assert!(first.contains('…'), "{first}");
        let second = numbered_source_body(&rendered, 2);
        assert!(second.ends_with("ku ku"), "{second}");
        assert_eq!(second.matches('…').count(), 1, "{second}");
        let ellipsis = second.find('…').expect("left cut");
        let word = second.find("barda").expect("barda");
        assert!(ellipsis < word, "{second}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn overlapping_labels_on_a_clipped_line_in_the_width_band_stay_one_window() {
        let source = format!(
            "zoi gy. {} .gy {} miamy\u{0301}\u{0308}z {}",
            "漢".repeat(10),
            "k".repeat(100),
            "do citka lo plise .i ".repeat(5)
        );
        let rendered = render_parser_diagnostics(&source);
        assert!(!rendered.contains('⸬'), "{rendered}");
        assert!(!rendered.contains(":::"), "{rendered}");
        let body = excerpt_body(&rendered);
        assert!(body.starts_with('…') && body.contains(".gy "), "{body}");
        assert!(body.matches('k').count() >= 100, "{body}");
        assert!(!body.contains('漢'), "{body}");
        assert!(
            !body.contains("…\u{0301}") && !body.contains("…\u{0308}"),
            "{body}"
        );
        assert_combining_marks_follow_bases(&body);
        let ascii = render_parser_diagnostics_with(&source, GlyphStyle::Ascii);
        assert!(!ascii.contains(":::"), "{ascii}");
        let ascii_body = excerpt_body(&ascii);
        // The ASCII mark is three columns, so it consumes `.gy` on this straddle.
        assert!(ascii_body.starts_with("..."), "{ascii_body}");
        assert!(ascii_body.matches('k').count() >= 100, "{ascii_body}");
        assert!(!ascii_body.contains('漢'), "{ascii_body}");
        assert!(
            !ascii_body.contains("...\u{0301}") && !ascii_body.contains("...\u{0308}"),
            "{ascii_body}"
        );
        assert_combining_marks_follow_bases(&ascii_body);
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
        assert!(rendered.contains('\u{85}'), "{rendered}");
        assert!(!rendered.contains("\\u{85}"), "{rendered}");
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
        assert!(rendered.contains('\u{85}'), "{rendered}");
        assert!(rendered.contains('␡'), "{rendered}");
        assert!(!rendered.contains("\\u{85}"), "{rendered}");
        let lines = rendered.lines().collect::<Vec<_>>();
        let source_line = lines
            .iter()
            .find(|line| line.contains('\u{85}') && line.contains("cd"))
            .expect("excerpt line");
        let marked = UnicodeWidthStr::width(&source_line[..source_line.find("cd").expect("cd")]);
        let caret_line = lines
            .iter()
            .find(|line| line.contains('^'))
            .expect("caret line");
        let caret_at = caret_line.find('^').expect("caret");
        assert_eq!(
            UnicodeWidthStr::width(&caret_line[..caret_at]),
            marked,
            "{rendered}"
        );
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

    #[requires(labels.iter().all(|(start, end, message, _)| {
        *start <= *end && *end <= source.len() && !message.is_empty()
    }))]
    #[ensures(!ret.is_empty())]
    fn render_carbon(source: &str, labels: &[(usize, usize, &str, bool)], ascii: bool) -> String {
        let mut diagnostic_labels = Vec::new();
        for (start, end, message, primary) in labels {
            let span = source_span_from_byte_offsets(
                Some(SourceId("<input>".to_owned())),
                source,
                *start,
                *end,
            )
            .expect("span is on a character boundary");
            diagnostic_labels.push(DiagnosticLabel::new(span, (*message).to_owned(), *primary));
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
        render_diagnostics(
            "<input>",
            source,
            &[diagnostic],
            new!(DiagnosticRenderOptions {
                color: false,
                detail: DiagnosticDetailMode::Summary,
                glyphs: if ascii {
                    GlyphStyle::Ascii
                } else {
                    GlyphStyle::Unicode
                },
                terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
            }),
        )
        .expect("render")
    }

    #[requires(true)]
    #[ensures(true)]
    fn gutter_body(line: &str) -> Option<&str> {
        line.split_once('│')
            .or_else(|| line.split_once('|'))
            .map(|(_, body)| body)
    }

    #[requires(true)]
    #[ensures(true)]
    fn numbered_body<'a>(lines: &'a [&'a str], number: usize) -> Option<&'a str> {
        lines.iter().find_map(|line| {
            let (gutter, body) = line.split_once('│').or_else(|| line.split_once('|'))?;
            (gutter.trim() == number.to_string()).then_some(body)
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn display_columns(text: &str) -> usize {
        UnicodeWidthStr::width(text)
    }

    #[requires(true)]
    #[ensures(true)]
    fn first_mark_column(line: &str) -> Option<usize> {
        let mut width = 0usize;
        for ch in line.chars() {
            if matches!(ch, '^' | '-' | '━' | '─' | '┯' | '┬' | '╿') {
                return Some(width);
            }
            width += UnicodeWidthStr::width(ch.encode_utf8(&mut [0; 4]));
        }
        None
    }

    #[requires(true)]
    #[ensures(true)]
    fn corner_column(line: &str) -> Option<usize> {
        let mut width = 0usize;
        for ch in line.chars() {
            if matches!(ch, '┛' | '┘') {
                return Some(width);
            }
            width += UnicodeWidthStr::width(ch.encode_utf8(&mut [0; 4]));
        }
        None
    }

    #[requires(true)]
    #[ensures(true)]
    fn mark_count(body: &str) -> usize {
        body.chars()
            .filter(|ch| {
                matches!(
                    ch,
                    '^' | '-' | '_' | '━' | '─' | '┯' | '┬' | '╿' | '┛' | '┘'
                )
            })
            .count()
    }

    #[requires(true)]
    #[ensures(true)]
    fn max_display_line(rendered: &str) -> usize {
        rendered.lines().map(display_columns).max().unwrap_or(0)
    }

    /// Display column of a margin mark that has a real glyph on both sides.
    /// A mark at either cut edge is not interior.
    #[requires(!mark.is_empty())]
    #[ensures(true)]
    fn interior_mark_column(body: &str, mark: &str) -> Option<usize> {
        let mut seen = 0usize;
        let mut index = 0usize;
        let mut saw_glyph = false;
        for ch in body.chars() {
            let mark_here = index + mark.len() <= body.len() && body[index..].starts_with(mark);
            if saw_glyph && mark_here {
                let after = &body[index + mark.len()..];
                if after
                    .chars()
                    .any(|next| !next.is_whitespace() && !mark.starts_with(next))
                {
                    return Some(seen);
                }
            }
            if !ch.is_whitespace() && !mark.starts_with(ch) {
                saw_glyph = true;
            }
            seen += display_columns(ch.encode_utf8(&mut [0; 4]));
            index += ch.len_utf8();
        }
        None
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn carbon_blank_comment_and_delimiter_bridges_stay_in_one_snippet() {
        for (source, middle) in [
            ("mi nelci lo\n\nku ku", ""),
            ("mi cusku zoi gy. abc\n//x\nghi", "//x"),
            ("mi cusku zoi gy. abc\n(\nghi", "("),
            ("mi cusku zoi gy. abc\n{\nghi", "{"),
            ("mi cusku zoi gy. abc\n[\nghi", "["),
        ] {
            let rendered = render_carbon(source, &[(0, source.len(), "here", true)], false);
            assert_eq!(rendered.matches("error[").count(), 1, "{rendered}");
            assert!(!rendered.contains('⸬'), "{rendered}");
            assert!(!rendered.contains(":::"), "{rendered}");
            assert!(!rendered_excerpt_is_cut(&rendered), "{rendered}");
            let lines: Vec<_> = rendered.lines().collect();
            assert!(numbered_body(&lines, 1).is_some(), "{rendered}");
            assert!(numbered_body(&lines, 2).is_some(), "{rendered}");
            assert!(numbered_body(&lines, 3).is_some(), "{rendered}");
            if !middle.is_empty() {
                let body = numbered_body(&lines, 2).expect("bridge");
                assert!(body.contains(middle), "{rendered}");
            }
            let first = lines
                .iter()
                .position(|line| numbered_body(&[*line], 1).is_some())
                .expect("line 1");
            let third = lines
                .iter()
                .position(|line| numbered_body(&[*line], 3).is_some())
                .expect("line 3");
            assert!(first < third, "{rendered}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn carbon_a_larger_gap_stays_one_snippet_with_the_fold() {
        let source = "mi nelci lo\n\n\nku ku";
        let rendered = render_carbon(source, &[(0, source.len(), "here", true)], false);
        assert_eq!(rendered.matches("error[").count(), 1, "{rendered}");
        assert!(!rendered.contains('⸬'), "{rendered}");
        assert!(rendered.contains('‡'), "{rendered}");
        let lines: Vec<_> = rendered.lines().collect();
        let first = lines
            .iter()
            .position(|line| numbered_body(&[*line], 1).is_some())
            .expect("line 1");
        let fourth = lines
            .iter()
            .position(|line| numbered_body(&[*line], 4).is_some())
            .expect("line 4");
        assert!(first < fourth, "{rendered}");
        assert!(numbered_body(&lines, 2).is_none(), "{rendered}");
        assert!(numbered_body(&lines, 3).is_none(), "{rendered}");

        let ascii = render_carbon(source, &[(0, source.len(), "here", true)], true);
        assert_eq!(ascii.matches("error[").count(), 1, "{ascii}");
        assert!(!ascii.contains(":::"), "{ascii}");
        assert!(ascii.lines().any(|line| line.starts_with("...")), "{ascii}");
        let ascii_lines: Vec<_> = ascii.lines().collect();
        let ascii_first = ascii_lines
            .iter()
            .position(|line| numbered_body(&[*line], 1).is_some())
            .expect("ascii line 1");
        let ascii_fourth = ascii_lines
            .iter()
            .position(|line| numbered_body(&[*line], 4).is_some())
            .expect("ascii line 4");
        assert!(ascii_first < ascii_fourth, "{ascii}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn carbon_point_after_a_left_cut_follows_the_visible_end() {
        let source = "mi klama lo zarci .i ".repeat(5) + "lo";
        let point = source.len();
        let rendered = render_carbon(&source, &[(point, point, "ATEND", true)], false);
        let lines: Vec<_> = rendered.lines().collect();
        let source_line = lines
            .iter()
            .copied()
            .find(|line| line.contains("lo") && line.contains('…'))
            .expect("window");
        let lo_at = source_line.rfind("lo").expect("lo");
        let after_lo = display_columns(&source_line[..lo_at + 2]);
        let caret_line = lines
            .iter()
            .copied()
            .find(|line| first_mark_column(line).is_some())
            .expect("caret");
        let caret = first_mark_column(caret_line).expect("col");
        let ellipsis = display_columns(&source_line[..source_line.find('…').expect("ellipsis")]);
        assert_ne!(caret, ellipsis, "{rendered}");
        assert_eq!(caret, after_lo, "{rendered}");
        assert_eq!(mark_count(caret_line), 1, "{rendered}");
        assert!(max_display_line(&rendered) <= 80, "{rendered}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn carbon_point_past_a_right_cut_lands_on_the_right_mark() {
        for mark in ["…", "..."] {
            let line = "a".repeat(160);
            let columns = annotate_snippets::renderer::code_columns(80, 1, 0);
            let laid = annotate_snippets::renderer::lay_out_line(
                &line,
                &[(0, 4, 2)],
                columns,
                mark,
                crate::styled::expanded_terminal_control,
            );
            assert!(laid.clipped, "{}", laid.text);
            assert!(
                UnicodeWidthStr::width(laid.text.as_str()) <= columns,
                "{}",
                laid.text
            );
            let (map, append) = source_boundary(&line, &laid.origin, 0, false);
            assert!(!append, "{}", laid.text);
            let mark_at = laid.text.rfind(mark).expect("right mark");
            let last_real = laid
                .origin
                .iter()
                .copied()
                .filter(|byte| *byte != usize::MAX)
                .max()
                .expect("visible byte");
            assert!(last_real + 1 < line.len(), "{}", laid.text);
            assert_eq!(map[last_real + 1], mark_at, "{}", laid.text);
            assert_eq!(map[line.len()], mark_at, "{}", laid.text);
            assert_ne!(map[0], mark_at, "{}", laid.text);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn carbon_points_on_wide_tab_and_escaped_glyphs_stay_one_column() {
        let accented = "mi kláma";
        let accent = accented.find('á').expect("accent");
        let accented_render = render_carbon(accented, &[(accent, accent, "AT", true)], false);
        let lines: Vec<_> = accented_render.lines().collect();
        let source_line = lines
            .iter()
            .copied()
            .find(|line| line.contains('á'))
            .expect("source");
        let accent_col = display_columns(&source_line[..source_line.find('á').expect("á")]);
        let caret_line = lines
            .iter()
            .copied()
            .find(|line| first_mark_column(line).is_some())
            .expect("caret");
        assert_eq!(
            first_mark_column(caret_line),
            Some(accent_col),
            "{accented_render}"
        );
        assert_eq!(mark_count(caret_line), 1, "{accented_render}");
        assert_point_stays_on_glyph_start(accented, accent, 'á'.len_utf8());

        let tabbed = "mi\tlo";
        let tab_render = render_carbon(tabbed, &[(2, 2, "AT", true)], false);
        let tab_lines: Vec<_> = tab_render.lines().collect();
        let tab_line = tab_lines
            .iter()
            .copied()
            .find(|line| line.contains("mi") && line.contains("lo"))
            .expect("source");
        let spaces_at = tab_line.find("mi").expect("mi") + 2;
        assert!(tab_line[spaces_at..].starts_with("    "), "{tab_render}");
        let tab_caret = tab_lines
            .iter()
            .copied()
            .find(|line| first_mark_column(line).is_some())
            .expect("caret");
        assert_eq!(
            first_mark_column(tab_caret),
            Some(display_columns(&tab_line[..spaces_at])),
            "{tab_render}"
        );
        assert_eq!(mark_count(tab_caret), 1, "{tab_render}");
        assert_point_stays_on_glyph_start(tabbed, 2, 1);

        let control = "mi\u{7}lo";
        let control_render = render_carbon(control, &[(2, 2, "AT", true)], false);
        assert!(control_render.contains('␇'), "{control_render}");
        assert!(!control_render.contains("\\u{7}"), "{control_render}");
        let control_lines: Vec<_> = control_render.lines().collect();
        let control_line = control_lines
            .iter()
            .copied()
            .find(|line| line.contains('␇'))
            .expect("source");
        let picture_at = control_line.find('␇').expect("picture");
        let control_caret = control_lines
            .iter()
            .copied()
            .find(|line| first_mark_column(line).is_some())
            .expect("caret");
        assert_eq!(
            first_mark_column(control_caret),
            Some(display_columns(&control_line[..picture_at])),
            "{control_render}"
        );
        assert_eq!(mark_count(control_caret), 1, "{control_render}");
        assert_point_stays_on_glyph_start(control, 2, 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn carbon_trailing_newline_keeps_the_end_column_past_the_last_word() {
        let source = format!("mi cusku zoi gy. {}\ndef\n", "a".repeat(90));
        let rendered = render_carbon(&source, &[(0, source.len(), "here", true)], false);
        let lines: Vec<_> = rendered.lines().collect();
        let def_line = lines
            .iter()
            .copied()
            .find(|line| numbered_body(&[*line], 2).is_some_and(|body| body.contains("def")))
            .expect("def");
        let def_end = def_line.rfind("def").expect("def") + 3;
        let past = display_columns(&def_line[..def_end]);
        let mark_line = lines
            .iter()
            .copied()
            .find(|line| corner_column(line).is_some())
            .expect("end mark");
        assert_eq!(corner_column(mark_line), Some(past), "{rendered}");
        assert!(max_display_line(&rendered) <= 80, "{rendered}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn carbon_one_line_multiline_ends_stay_one_column() {
        let mut start_only = format!("mi cusku zoi gy. {}", "a".repeat(90));
        start_only.push('\n');
        start_only.push_str(&"\n".repeat(98));
        start_only.push_str("def");
        let start_render =
            render_carbon(&start_only, &[(9, start_only.len(), "here", true)], false);
        assert!(max_display_line(&start_render) <= 80, "{start_render}");
        assert!(!start_render.contains('⸬'), "{start_render}");
        assert!(start_render.contains('‡'), "{start_render}");
        let start_lines: Vec<_> = start_render.lines().collect();
        let first_line = start_lines
            .iter()
            .copied()
            .find(|line| numbered_body(&[*line], 1).is_some())
            .expect("line 1");
        let first = gutter_body(first_line).expect("body");
        assert!(!first.contains(&"a".repeat(90)), "{start_render}");
        let first_at = start_lines
            .iter()
            .position(|line| numbered_body(&[*line], 1).is_some())
            .expect("line 1");
        let end_at = start_lines
            .iter()
            .position(|line| numbered_body(&[*line], 100).is_some())
            .expect("line 100");
        assert!(first_at < end_at, "{start_render}");
        assert!(numbered_body(&start_lines, 2).is_none(), "{start_render}");
        let underline = start_lines[first_at + 1];
        assert!(
            underline.contains('┏') && underline.contains('┛'),
            "{start_render}"
        );
        let close = corner_column(underline).expect("start hook");
        let ellipsis = display_columns(&first_line[..first_line.find('…').expect("cut")]);
        assert!(close < ellipsis, "{start_render}");
        assert!(
            mark_count(gutter_body(underline).unwrap_or("")) < 20,
            "{start_render}"
        );

        let mut end_only = String::from("mi cusku zoi gy. abc\n");
        end_only.push_str(&"\n".repeat(98));
        end_only.push_str(&"d".repeat(120));
        let end_render = render_carbon(&end_only, &[(0, end_only.len(), "here", true)], false);
        assert!(!end_render.contains('⸬'), "{end_render}");
        assert!(end_render.contains('‡'), "{end_render}");
        let end_lines: Vec<_> = end_render.lines().collect();
        let end_line = end_lines
            .iter()
            .copied()
            .find(|line| numbered_body(&[*line], 100).is_some())
            .expect("line 100");
        let end_body = gutter_body(end_line).expect("body");
        assert!(end_body.contains('┃'), "{end_render}");
        assert!(end_body.contains(&"d".repeat(120)), "{end_render}");
        let end_row = end_lines
            .iter()
            .position(|line| *line == end_line)
            .expect("row");
        let close = end_lines[end_row + 1];
        assert!(close.contains('┗') && close.contains('┛'), "{end_render}");
        let last_d = end_line.rfind('d').expect("d");
        let d_column = display_columns(&end_line[..last_d]);
        assert_eq!(corner_column(close), Some(d_column), "{end_render}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn carbon_a_pass_through_span_does_not_widen_the_middle_line() {
        let mut source = String::from("mi cusku zoi gy. abc\n");
        source.push_str(&"\n".repeat(10));
        let middle = source.len();
        source.push_str(&"a".repeat(150));
        let point = middle + 40;
        source.push('\n');
        source.push_str(&"\n".repeat(10));
        source.push_str("def");
        let rendered = render_carbon(
            &source,
            &[
                (0, source.len(), "CROSS", false),
                (point, point, "POINT", true),
            ],
            false,
        );
        assert!(max_display_line(&rendered) <= 80, "{rendered}");
        let lines: Vec<_> = rendered.lines().collect();
        let window = lines
            .iter()
            .copied()
            .find(|line| {
                gutter_body(line).is_some_and(|body| body.contains('a') && body.contains('…'))
            })
            .expect("cut window");
        let body = gutter_body(window).expect("body");
        assert!(!body.contains(&"a".repeat(150)), "{rendered}");
        let window_at = lines.iter().position(|line| *line == window).expect("row");
        let underline = gutter_body(lines[window_at + 1]).unwrap_or("");
        assert!(mark_count(underline) <= 3, "{rendered}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn carbon_long_span_underlines_keep_the_middle_mark() {
        for ascii in [false, true] {
            let mark = if ascii { "..." } else { "…" };
            let keys = "k".repeat(200);
            let key_render = render_carbon(&keys, &[(0, keys.len(), "here", true)], ascii);
            assert_middle_mark_lines(&key_render, mark);

            let words = format!("mi nelci lo {}ku ku", "barda ".repeat(150));
            let word_render = render_carbon(&words, &[(0, words.len(), "here", true)], ascii);
            assert_middle_mark_lines(&word_render, mark);

            let wide = format!("mi nelci lo {}", "漢".repeat(80));
            let wide_render = render_carbon(&wide, &[(0, wide.len(), "here", true)], ascii);
            assert_middle_mark_lines(&wide_render, mark);
        }
    }

    #[requires(!mark.is_empty())]
    #[ensures(true)]
    fn assert_middle_mark_lines(rendered: &str, mark: &str) {
        let lines: Vec<_> = rendered.lines().collect();
        let source_at = lines
            .iter()
            .position(|line| numbered_body(&[*line], 1).is_some_and(|body| body.contains(mark)))
            .expect("cut source");
        let source = lines[source_at];
        let underline = lines.get(source_at + 1).copied().expect("underline");
        let column = interior_mark_column(source, mark).expect("interior mark");
        assert_eq!(
            interior_mark_column(underline, mark),
            Some(column),
            "{rendered}"
        );
        assert!(max_display_line(rendered) <= 80, "{rendered}");
    }

    #[requires(glyph_bytes > 0 && point + glyph_bytes <= text.len())]
    #[ensures(true)]
    fn assert_point_stays_on_glyph_start(text: &str, point: usize, glyph_bytes: usize) {
        let columns = annotate_snippets::renderer::code_columns(80, 1, 0);
        let laid = annotate_snippets::renderer::lay_out_line(
            text,
            &[(point, point, 2)],
            columns,
            "…",
            crate::styled::expanded_terminal_control,
        );
        let (map, append) = source_boundary(text, &laid.origin, 0, false);
        assert!(!append, "{}", laid.text);
        let bound = new!(LineBound {
            source_start: 0,
            source_end: text.len(),
            map: map.clone(),
        });
        let mapped = map_point(&[bound], laid.text.len(), point);
        assert_eq!(mapped.start, mapped.end, "{}", laid.text);
        assert_eq!(mapped.start, map[point], "{}", laid.text);
        assert!(map[point + glyph_bytes] > map[point], "{}", laid.text);
    }
}
