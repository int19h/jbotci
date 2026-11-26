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
    let source_label = crate::styled::escape_terminal_controls(source_label);
    for diagnostic in diagnostics {
        let remaining =
            ByteLimit::new(output.remaining()).ok_or_else(|| OutputError::Limit { limit })?;
        let message = crate::styled::escape_terminal_controls(&diagnostic.message);
        let code = crate::styled::escape_terminal_controls(&diagnostic.code);
        let notes = diagnostic
            .notes
            .iter()
            .map(|note| crate::styled::escape_terminal_controls(note))
            .collect::<Vec<_>>();
        let label_messages = diagnostic
            .labels
            .iter()
            .map(|label| crate::styled::escape_terminal_controls(&label.message))
            .collect::<Vec<_>>();
        let excerpt = diagnostic_source_excerpt(
            source,
            diagnostic,
            options.terminal_width,
            output.remaining(),
        );
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

#[requires(terminal_width > 0)]
#[ensures(ret.line_start >= 1 && ret.primary_line >= 1 && ret.primary_column >= 1)]
#[ensures(ret.origin <= source.len())]
fn diagnostic_source_excerpt<'source>(
    source: &'source str,
    diagnostic: &Diagnostic,
    terminal_width: usize,
    remaining: usize,
) -> DiagnosticSourceExcerpt<'source> {
    let (origin, end) = annotated_line_bounds(source, diagnostic);
    let line_start = 1 + source[..origin]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    // Anchors exist before the copy and before the escape.
    let anchors = display_anchors(source, diagnostic);
    let (primary_line, primary_column) = primary_anchor_position(&anchors);
    let slice = source.get(origin..end).unwrap_or("");
    let cap = excerpt_copy_cap(terminal_width, remaining);
    if slice.len() <= cap {
        return new!(DiagnosticSourceExcerpt {
            escaped: crate::styled::escape_terminal_source(slice),
            origin,
            line_start,
            primary_line,
            primary_column,
            retained: Vec::new(),
        });
    }
    let retained = retain_annotated_windows(
        source,
        &anchors,
        cap,
        terminal_width,
        primary_line,
        primary_column,
    );
    let origin = retained
        .first()
        .map(|window| window.source_start)
        .unwrap_or(origin);
    let line_start = retained
        .first()
        .map(|window| window.line_number)
        .unwrap_or(line_start);
    new!(DiagnosticSourceExcerpt {
        escaped: crate::styled::escape_terminal_source(""),
        origin,
        line_start,
        primary_line,
        primary_column,
        retained,
    })
}

/// Visible excerpt size. It is also at most the remaining output budget plus one byte.
#[requires(terminal_width > 0)]
#[ensures(ret > 0)]
fn excerpt_copy_cap(terminal_width: usize, remaining: usize) -> usize {
    let visible = terminal_width.saturating_mul(8).max(1);
    let budget = remaining.saturating_add(1).max(1);
    visible.min(budget)
}

#[requires(true)]
#[ensures(ret.0 <= ret.1 && ret.1 <= source.len())]
fn annotated_line_bounds(source: &str, diagnostic: &Diagnostic) -> (usize, usize) {
    if diagnostic.labels.is_empty() {
        return (0, 0);
    }
    let mut start = source.len();
    let mut end = 0usize;
    for label in &diagnostic.labels {
        let span = label_span(label);
        let line_start = line_start_at(source, span.start);
        let line_end = line_end_at(source, span.end);
        start = start.min(line_start);
        end = end.max(line_end);
    }
    if start > end { (0, 0) } else { (start, end) }
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

#[requires(true)]
#[ensures(ret <= source.len())]
fn line_end_at(source: &str, index: usize) -> usize {
    let index = char_boundary_at(source, index);
    match source[index..].find('\n') {
        Some(offset) => index + offset,
        None => source.len(),
    }
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

#[invariant(*source_start <= *source_end)]
struct RawWindow {
    source_start: usize,
    source_end: usize,
    /// One entry is a label index. A merged window keeps every label it holds.
    labels: Vec<usize>,
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
}

/// One copied window before its header is known.
#[invariant(*source_start <= *source_end && *line_number >= 1)]
struct CopiedWindow<'source> {
    escaped: crate::styled::TerminalExcerpt<'source>,
    line_number: usize,
    source_start: usize,
    source_end: usize,
    label_indexes: Vec<usize>,
}

/// Keep a bounded piece of each labelled region.
///
/// Each label already has a display anchor.
/// Windows come from those anchors.
/// An empty anchor stays in the scan.
/// A label whose mark does not fit gets its own location.
/// The primary header stays on the window that holds the primary mark.
#[requires(cap > 0 && terminal_width > 0 && primary_line >= 1 && primary_column >= 1)]
#[ensures(ret.iter().all(|window| window.line_number >= 1 && window.header_line >= 1 && window.header_column >= 1))]
fn retain_annotated_windows<'source>(
    source: &'source str,
    anchors: &[DisplayAnchor],
    cap: usize,
    terminal_width: usize,
    primary_line: usize,
    primary_column: usize,
) -> Vec<RetainedWindow<'source>> {
    let windows = windows_from_anchors(source, anchors, cap, terminal_width);
    let Some(limit) = crate::budget::ByteLimit::new(cap) else {
        return Vec::new();
    };
    let mut budget = crate::budget::SpanBudget::new(limit);
    let mut retained = Vec::new();
    let mut placed = vec![false; anchors.len()];
    for window in windows {
        if window.source_end > source.len()
            || window.source_start > window.source_end
            || !source.is_char_boundary(window.source_start)
            || !source.is_char_boundary(window.source_end)
        {
            continue;
        }
        // A zero-byte line is a real anchor. It does not stop the scan.
        if window.source_start == window.source_end {
            mark_placed(&mut placed, &window.labels);
            retained.push(copied_window(
                source,
                window.source_start,
                window.source_end,
                window.labels.clone(),
            ));
            continue;
        }
        let room = budget.remaining();
        if room == 0 {
            continue;
        }
        let borrowed = &source[window.source_start..window.source_end];
        let take = if borrowed.len() <= room {
            borrowed.len()
        } else {
            bounded_end(borrowed, 0, room)
        };
        if take == 0 {
            continue;
        }
        budget
            .charge(take)
            .expect("the retained excerpt fits in the budget");
        let source_end = window.source_start + take;
        let labels = labels_inside_copy(&window.labels, anchors, window.source_start, source_end);
        if labels.is_empty() {
            continue;
        }
        mark_placed(&mut placed, &labels);
        retained.push(copied_window(
            source,
            window.source_start,
            source_end,
            labels,
        ));
    }
    for anchor in anchors {
        if placed[anchor.label_index] {
            continue;
        }
        // The mark did not fit in a copied window. Show the original location.
        placed[anchor.label_index] = true;
        retained.push(copied_window(
            source,
            anchor.start,
            anchor.start,
            vec![anchor.label_index],
        ));
    }
    let primary_index = chosen_primary_index(anchors);
    retained
        .into_iter()
        .map(|window| {
            let copied = window.into_data();
            let owns_primary =
                primary_index.is_some_and(|index| copied.label_indexes.contains(&index));
            let (header_line, header_column) = if owns_primary {
                (primary_line, primary_column)
            } else {
                anchor_window_header(anchors, &copied.label_indexes, source, copied.source_start)
            };
            new!(RetainedWindow {
                escaped: copied.escaped,
                line_number: copied.line_number,
                header_line,
                header_column,
                source_start: copied.source_start,
                source_end: copied.source_end,
                owns_primary,
                label_indexes: copied.label_indexes,
            })
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn copied_window<'source>(
    source: &'source str,
    source_start: usize,
    source_end: usize,
    label_indexes: Vec<usize>,
) -> CopiedWindow<'source> {
    let line_number = source_line_column(source, source_start).0;
    new!(CopiedWindow {
        escaped: crate::styled::escape_terminal_source(&source[source_start..source_end]),
        line_number,
        source_start,
        source_end,
        label_indexes,
    })
}

#[requires(true)]
#[ensures(true)]
fn mark_placed(placed: &mut [bool], labels: &[usize]) {
    for index in labels {
        if let Some(slot) = placed.get_mut(*index) {
            *slot = true;
        }
    }
}

/// Labels whose anchor start is still inside the copied bytes.
#[requires(true)]
#[ensures(true)]
fn labels_inside_copy(
    labels: &[usize],
    anchors: &[DisplayAnchor],
    source_start: usize,
    source_end: usize,
) -> Vec<usize> {
    labels
        .iter()
        .copied()
        .filter(|index| {
            anchors
                .get(*index)
                .is_some_and(|anchor| anchor_mark_inside(anchor, source_start, source_end))
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn anchor_mark_inside(anchor: &DisplayAnchor, source_start: usize, source_end: usize) -> bool {
    if anchor.start == anchor.end {
        anchor.start >= source_start && anchor.start <= source_end
    } else {
        anchor.start >= source_start && anchor.start < source_end
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
        anchors.push(new!(DisplayAnchor {
            label_index,
            start,
            end,
            line,
            column,
            primary: label.primary,
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

#[requires(cap > 0 && terminal_width > 0)]
#[ensures(true)]
fn windows_from_anchors(
    source: &str,
    anchors: &[DisplayAnchor],
    cap: usize,
    terminal_width: usize,
) -> Vec<RawWindow> {
    if anchors.is_empty() {
        return Vec::new();
    }
    let count = anchors.len();
    let separators = count.saturating_sub(1).saturating_mul(5);
    let body = cap.saturating_sub(separators).max(count);
    // Each label keeps about one terminal line. The byte cap still bounds the copy.
    let share = (body / count).max(1).min(terminal_width);
    let mut windows = Vec::new();
    for anchor in anchors {
        let bounds = label_window(source, anchor.start, anchor.end, share);
        windows.push(new!(RawWindow {
            source_start: bounds.source_start,
            source_end: bounds.source_end,
            labels: vec![anchor.label_index],
        }));
    }
    merge_windows(windows)
}

#[requires(share > 0)]
#[ensures(ret.source_start <= ret.source_end && ret.source_end <= source.len())]
fn label_window(source: &str, start: usize, end: usize, share: usize) -> RawWindow {
    let start = start.min(source.len());
    let end = end.min(source.len()).max(start);
    let line_start = line_start_at(source, start);
    let line_end = line_end_at(source, end);
    if line_end.saturating_sub(line_start) <= share {
        return new!(RawWindow {
            source_start: line_start,
            source_end: line_end,
            labels: Vec::new(),
        });
    }
    if end.saturating_sub(line_start) <= share {
        return new!(RawWindow {
            source_start: line_start,
            source_end: bounded_end(source, line_start, share).min(line_end),
            labels: Vec::new(),
        });
    }
    if line_end.saturating_sub(start) <= share {
        return new!(RawWindow {
            source_start: bounded_start(source, line_end, share).max(line_start),
            source_end: line_end,
            labels: Vec::new(),
        });
    }
    let window_end = bounded_end(source, start, share).min(line_end);
    let window_start = bounded_start(source, window_end, share).max(line_start);
    new!(RawWindow {
        source_start: window_start,
        source_end: window_end,
        labels: Vec::new(),
    })
}

#[requires(true)]
#[ensures(ret <= source.len() && (ret == start || source.is_char_boundary(ret)))]
fn bounded_end(source: &str, start: usize, max_len: usize) -> usize {
    let start = start.min(source.len());
    let mut end = start.saturating_add(max_len).min(source.len());
    while end > start && !source.is_char_boundary(end) {
        end -= 1;
    }
    end
}

#[requires(true)]
#[ensures(ret <= end && (ret == end || source.is_char_boundary(ret)))]
fn bounded_start(source: &str, end: usize, max_len: usize) -> usize {
    let end = end.min(source.len());
    let mut start = end.saturating_sub(max_len);
    while start < end && !source.is_char_boundary(start) {
        start += 1;
    }
    start
}

#[requires(true)]
#[ensures(true)]
fn merge_windows(mut windows: Vec<RawWindow>) -> Vec<RawWindow> {
    windows.sort_by_key(|window| (window.source_start, window.source_end));
    let mut merged: Vec<RawWindow> = Vec::new();
    for window in windows {
        let extend = merged.last().and_then(|last| {
            (window.source_start <= last.source_end)
                .then_some((last.source_start, last.source_end.max(window.source_end)))
        });
        if let Some((source_start, source_end)) = extend {
            let mut labels = merged
                .last()
                .map(|last| last.labels.clone())
                .unwrap_or_default();
            labels.extend(window.labels.iter().copied());
            merged.pop();
            merged.push(new!(RawWindow {
                source_start,
                source_end,
                labels,
            }));
        } else {
            merged.push(window);
        }
    }
    merged
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
                .map(|((_, label), label_message)| {
                    annotation_kind(label.primary)
                        .span(window_label_span(label, window))
                        .label(label_message.as_ref())
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
        crate::styled::reset_excerpt_account();
        let rendered = render_diagnostics("<input>", &source, &[diagnostic.clone()], options)
            .expect("clipped diagnostic");
        let copied = crate::styled::excerpt_copy_bytes();
        let map_len = crate::styled::excerpt_map_len();
        assert!(copied < 1_000, "{copied}");
        assert!(copied * 10 < source.len(), "{copied}");
        assert!(map_len < 1_000, "{map_len}");
        assert!(map_len <= copied + 4, "{map_len} {copied}");
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

        crate::styled::reset_excerpt_account();
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
        let limited_copy = crate::styled::excerpt_copy_bytes();
        let limited_map = crate::styled::excerpt_map_len();
        assert!(limited_copy <= 33, "{limited_copy}");
        assert!(limited_map <= limited_copy + 4, "{limited_map}");
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
        assert!(rendered.contains("mi\\u{9}#"), "{rendered}");
        assert!(location_shows(&rendered, "--> ", 1, column), "{rendered}");
        assert!(!location_shows(&rendered, "--> ", 1, 8), "{rendered}");
        let lines = rendered.lines().collect::<Vec<_>>();
        let source_line = lines
            .iter()
            .find(|line| line.contains("mi\\u{9}#"))
            .expect("excerpt line");
        let text_at = source_line.find("mi\\u{9}#").expect("excerpt column");
        let marked = text_at + "mi\\u{9}".len();
        let caret_line = lines
            .iter()
            .find(|line| line.contains('^'))
            .expect("caret line");
        let caret_at = caret_line.find('^').expect("caret");
        assert_eq!(caret_at, marked, "{rendered}");
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
        let mut saw_eof = false;
        let mut saw_empty_line = false;
        let mut saw_overlap = false;
        let mut saw_distant = false;
        let mut saw_control = false;
        let mut saw_long_line = false;
        let mut saw_final_newline = false;
        let mut saw_no_final_newline = false;
        let mut saw_location_only = false;
        for case in 0..CASES {
            let set = random_label_set(&mut rng, case);
            let cap = excerpt_copy_cap(set.width, usize::MAX);
            let (bound_start, bound_end) = label_bound(&set.source, &set.labels);
            if bound_end.saturating_sub(bound_start) > cap {
                clipped += 1;
            } else {
                uncut += 1;
            }
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
            assert!(
                rendered.len() <= LIMIT,
                "case {case} length {}",
                rendered.len()
            );
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
            for (start, _, message, _) in &set.labels {
                let (label_line, _) = source_line_column(&set.source, *start);
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
                Ok(text) => assert!(text.len() <= 64, "case {case} tight {}", text.len()),
                Err(OutputError::Limit { .. }) => {}
                Err(error) => panic!("case {case} tight failed: {error:?}"),
            }
        }
        assert!(uncut > 0 && clipped > 0, "uncut {uncut} clipped {clipped}");
        assert!(saw_eof && saw_empty_line && saw_overlap && saw_distant);
        assert!(saw_control && saw_long_line && saw_final_newline && saw_no_final_newline);
        assert!(saw_location_only);
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
                if rng.below(2) == 0 {
                    source.push('\n');
                }
                let count = 1 + rng.below(3);
                push_scattered_labels(rng, &source, &mut labels, count);
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
                labels.insert(0, (0, 4, "@@0@@".to_owned(), false));
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

    #[requires(true)]
    #[ensures(true)]
    fn label_bound(source: &str, labels: &[(usize, usize, String, bool)]) -> (usize, usize) {
        let mut start = source.len();
        let mut end = 0usize;
        for (label_start, label_end, _, _) in labels {
            start = start.min(line_start_at(source, *label_start));
            end = end.max(line_end_at(source, *label_end));
        }
        if start > end { (0, 0) } else { (start, end) }
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
        crate::styled::reset_excerpt_account();
        let rendered =
            render_diagnostics("<input>", &source, &[diagnostic], options).expect("two lines");
        let copied = crate::styled::excerpt_copy_bytes();
        assert!(copied < 1_000, "{copied}");
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
                "\\u{9}",
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
