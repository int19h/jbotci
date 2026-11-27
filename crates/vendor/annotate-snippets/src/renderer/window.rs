//! The source text a diagnostic snippet actually draws.
//!
//! The numbers that trim a line live in [`Margin`] and in the long-span
//! replacement used by `render_source_line`. This module calls those paths.

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::{max, min};
use core::ops::Range;

use super::margin::Margin;
use super::render::{
    char_width, code_column_width, long_span_pad, num_decimal_digits, output_replacement, str_width,
};
use super::source_map::SourceMap;
use crate::{AnnotationKind, renderer::num_overlap};

/// A replacement for one source character before the line is trimmed.
///
/// The bytes are the text the snippet source will hold. They are ASCII.
#[derive(Clone, Copy, Debug)]
pub struct ExpandedControl {
    pub text: [u8; 6],
    pub len: u8,
}

/// One source line that survives folding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeptSourceLine {
    pub line_index: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// One displayed source line.
///
/// `origin[i]` is the byte offset in the input line for output byte `i`.
/// [`usize::MAX`] marks a byte inserted as a cut mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaidOutLine {
    pub text: String,
    pub origin: Vec<usize>,
    /// The renderer drops or replaces some of this line.
    ///
    /// A line wider than the code column stays one window until a cut actually happens.
    /// The renderer leaves a single-line span uncut through twice the code column.
    pub clipped: bool,
}

/// Display columns the renderer reserves after a span for its label.
///
/// This is the label width plus one column, the same value `render_source_line` uses.
pub fn annotation_label_columns(label: &str) -> usize {
    str_width(label).saturating_add(1).max(1)
}

/// Code columns left after the line-number gutter.
pub fn code_columns(term_width: usize, max_line_number: usize, multiline_depth: usize) -> usize {
    code_column_width(
        term_width,
        num_decimal_digits(max_line_number.max(1)),
        multiline_depth,
    )
}

/// Lines the renderer keeps for `spans`.
///
/// Offsets are bytes of `source`. A folded gap is a missing line, not an
/// ellipsis inside this list. `multiline_depth` is the renderer's depth.
pub fn kept_source_lines(
    source: &str,
    spans: &[Range<usize>],
) -> (usize, Vec<KeptSourceLine>) {
    let map = SourceMap::new(source, 1);
    let annotations = spans
        .iter()
        .map(|span| AnnotationKind::Primary.span(span.start..span.end.min(source.len().saturating_add(1))))
        .collect();
    let (depth, lines) = map.annotated_lines(annotations, true);
    let kept = lines
        .into_iter()
        .map(|line| {
            let (start_byte, end_byte) = line_byte_range(source, line.line_index);
            KeptSourceLine {
                line_index: line.line_index,
                start_byte,
                end_byte,
            }
        })
        .collect();
    (depth, kept)
}

/// Lay out one source line the way `draw_line` and the long-span replacement do.
///
/// Each span is `(start, end, label_columns)` in bytes of `line`.
/// `label_columns` is the label's display width plus one, matching the renderer.
/// `expand` replaces a character before the trim. `None` keeps the character.
pub fn lay_out_line(
    line: &str,
    spans: &[(usize, usize, usize)],
    column_width: usize,
    margin_mark: &str,
    expand: fn(char) -> Option<ExpandedControl>,
) -> LaidOutLine {
    let display_len = stream_width(line, expand);
    let whitespace = leading_whitespace(line, expand);
    let mut span_left = usize::MAX;
    let mut span_right = 0usize;
    let mut label_right = 0usize;
    let mut display_spans = Vec::with_capacity(spans.len());
    for &(start, end, label_columns) in spans {
        let (start_col, mut end_col) = span_columns(line, start, end, expand);
        if start_col == end_col {
            end_col = end_col.saturating_add(1);
        }
        span_left = min(span_left, start_col);
        span_left = min(span_left, end_col);
        span_right = max(span_right, start_col);
        span_right = max(span_right, end_col);
        label_right = max(label_right, end_col.saturating_add(label_columns));
        display_spans.push((start_col, end_col));
    }
    if span_left == usize::MAX {
        span_left = 0;
    }
    let margin = Margin::new(
        whitespace,
        span_left,
        span_right,
        label_right,
        column_width,
        display_len,
    );
    let mut left = margin.left(display_len);
    let right = margin.right(display_len);
    let mut glyphs = Vec::new();
    for_each_char(line, expand, true, |origin, ch| {
        glyphs.push(Glyph {
            ch,
            origin,
            width: char_width(ch),
        });
        true
    });
    let mut cells = group_cells(&glyphs);
    let mut skipped = 0usize;
    let mut start_cell = 0usize;
    while start_cell < cells.len() && skipped < left {
        skipped += cells[start_cell].width;
        start_cell += 1;
    }
    // A wide character that crosses `left` moves the edge forward.
    // The kept columns and the later cut math both use that edge.
    if skipped > left {
        left = skipped;
    }
    let room = right.saturating_sub(left);
    let mut taken = 0usize;
    let mut end_cell = start_cell;
    while end_cell < cells.len() {
        let width = cells[end_cell].width;
        if taken + width > room {
            break;
        }
        taken += width;
        end_cell += 1;
    }
    cells = cells[start_cell..end_cell].to_vec();
    let cut_left = margin.was_cut_left();
    let cut_right = display_len > right;
    let padding = str_width(margin_mark);
    let mut left_drop = 0usize;
    let mut left_consumed = 0usize;
    if cut_left {
        for cell in &cells {
            if left_consumed >= padding {
                break;
            }
            left_consumed += cell.width;
            left_drop += 1;
        }
    }
    let mut right_drop = 0usize;
    if cut_right {
        let mut consumed = 0usize;
        for cell in cells.iter().rev() {
            if consumed >= padding || left_drop + right_drop >= cells.len() {
                break;
            }
            consumed += cell.width;
            right_drop += 1;
        }
    }
    let content_end = cells.len().saturating_sub(right_drop);
    let content_start = left_drop.min(content_end);
    let mut marked = Vec::with_capacity(cells.len());
    if cut_left {
        push_mark(&mut glyphs, &mut marked, margin_mark);
        // `draw_line` places the rest at the consumed display column.
        // A one-column mark that replaces a wider character leaves that gap.
        let gap = left_consumed.saturating_sub(padding);
        for _ in 0..gap {
            push_glyph(
                &mut glyphs,
                &mut marked,
                Glyph {
                    ch: ' ',
                    origin: usize::MAX,
                    width: 1,
                },
            );
        }
    }
    marked.extend_from_slice(&cells[content_start..content_end]);
    if cut_right {
        push_mark(&mut glyphs, &mut marked, margin_mark);
    }
    cells = marked;
    let mut overlap = vec![false; display_spans.len()];
    for (index, span) in display_spans.iter().enumerate() {
        for (other_index, other) in display_spans.iter().enumerate() {
            if other_index > 1 && num_overlap(other.0, other.1, span.0, span.1, false) {
                overlap[index] = true;
                overlap[other_index] = true;
            }
        }
    }
    let margin_width = str_width(margin_mark);
    let mut cut_middle = false;
    let mut cuts = Vec::new();
    for (index, &(start_col, end_col)) in display_spans.iter().enumerate() {
        if overlap[index] {
            continue;
        }
        let width = end_col.saturating_sub(start_col);
        let Some(pad) = long_span_pad(width, column_width, margin_width) else {
            continue;
        };
        let Some(mark_start) = start_col.checked_add(pad) else {
            continue;
        };
        let Some(mark_end) = end_col.checked_sub(pad) else {
            continue;
        };
        let start = mark_start.saturating_sub(left);
        let end = mark_end.saturating_sub(left);
        if start < end {
            cuts.push((start, end));
        }
    }
    if !cuts.is_empty() {
        let mut col = 0usize;
        let mut next = Vec::with_capacity(cells.len());
        let mut open: Option<usize> = None;
        for cell in &cells {
            let cell_end = col + cell.width;
            let covered = cuts.iter().position(|&(start, end)| {
                cell.width > 0 && col >= start && cell_end <= end
            });
            if let Some(index) = covered {
                if open != Some(index) {
                    push_mark(&mut glyphs, &mut next, margin_mark);
                    open = Some(index);
                    cut_middle = true;
                }
            } else {
                open = None;
                next.push(*cell);
            }
            col = cell_end;
        }
        cells = next;
    }
    let mut text = String::new();
    let mut origin = Vec::new();
    for cell in &cells {
        for glyph in glyphs.iter().skip(cell.start).take(cell.len) {
            push_char(&mut text, &mut origin, glyph.origin, glyph.ch);
        }
    }
    LaidOutLine {
        text,
        origin,
        clipped: cut_left || cut_right || cut_middle,
    }
}

struct Glyph {
    ch: char,
    origin: usize,
    width: usize,
}

#[derive(Clone, Copy)]
struct Cell {
    start: usize,
    len: usize,
    width: usize,
}

fn group_cells(glyphs: &[Glyph]) -> Vec<Cell> {
    let mut cells = Vec::new();
    let mut index = 0usize;
    while index < glyphs.len() {
        let start = index;
        let width = glyphs[index].width;
        index += 1;
        if width > 0 {
            while index < glyphs.len() && glyphs[index].width == 0 {
                index += 1;
            }
        }
        cells.push(Cell {
            start,
            len: index - start,
            width,
        });
    }
    cells
}

fn push_glyph(glyphs: &mut Vec<Glyph>, cells: &mut Vec<Cell>, glyph: Glyph) {
    let start = glyphs.len();
    let width = glyph.width;
    glyphs.push(glyph);
    cells.push(Cell {
        start,
        len: 1,
        width,
    });
}

fn push_mark(glyphs: &mut Vec<Glyph>, cells: &mut Vec<Cell>, margin_mark: &str) {
    for ch in margin_mark.chars() {
        push_glyph(
            glyphs,
            cells,
            Glyph {
                ch,
                origin: usize::MAX,
                width: char_width(ch),
            },
        );
    }
}

fn line_byte_range(source: &str, line_index: usize) -> (usize, usize) {
    let mut start = 0usize;
    let mut number = 1usize;
    while number < line_index {
        let Some(relative) = source[start..].find('\n') else {
            return (source.len(), source.len());
        };
        start += relative + 1;
        number += 1;
    }
    let Some(relative) = source[start..].find('\n') else {
        return (start, source.len());
    };
    let newline = start + relative;
    // A line with text ends before its break, so CR LF stays out of the copy.
    // An empty line is only that break. The range includes the break, or the
    // next render has no line to draw and the label moves to an earlier line.
    let empty = newline == start
        || (newline > start && source.as_bytes()[newline - 1] == b'\r' && newline - 1 == start);
    if empty {
        return (start, newline + 1);
    }
    if source.as_bytes()[newline - 1] == b'\r' {
        return (start, newline - 1);
    }
    (start, newline)
}

fn stream_width(line: &str, expand: fn(char) -> Option<ExpandedControl>) -> usize {
    let mut width = 0usize;
    for_each_char(line, expand, true, |_origin, ch| {
        width += char_width(ch);
        true
    });
    width
}

fn leading_whitespace(line: &str, expand: fn(char) -> Option<ExpandedControl>) -> usize {
    let mut columns = 0usize;
    let mut saw_text = false;
    for_each_char(line, expand, true, |_origin, ch| {
        if saw_text {
            return false;
        }
        if ch.is_whitespace() {
            columns += char_width(ch);
            true
        } else {
            saw_text = true;
            false
        }
    });
    if saw_text { columns } else { 0 }
}

fn span_columns(
    line: &str,
    start: usize,
    end: usize,
    expand: fn(char) -> Option<ExpandedControl>,
) -> (usize, usize) {
    let start = min(start, line.len());
    let end = min(end, line.len()).max(start);
    let mut column = 0usize;
    let mut start_col = None;
    let mut end_col = None;
    for (index, ch) in line.char_indices() {
        if start_col.is_none() && index >= start {
            start_col = Some(column);
        }
        if end_col.is_none() && index >= end {
            end_col = Some(column);
        }
        column += expanded_width(ch, expand, false);
    }
    (start_col.unwrap_or(column), end_col.unwrap_or(column))
}

fn expanded_width(ch: char, expand: fn(char) -> Option<ExpandedControl>, normalized: bool) -> usize {
    if let Some(expanded) = expand(ch) {
        let bytes = &expanded.text[..expanded.len as usize];
        let text = core::str::from_utf8(bytes).unwrap_or("");
        return str_width(text);
    }
    if normalized {
        if let Some(replacement) = output_replacement(ch) {
            return str_width(replacement);
        }
    }
    char_width(ch)
}

fn for_each_char(
    line: &str,
    expand: fn(char) -> Option<ExpandedControl>,
    normalized: bool,
    mut visit: impl FnMut(usize, char) -> bool,
) {
    for (index, ch) in line.char_indices() {
        if let Some(expanded) = expand(ch) {
            let bytes = &expanded.text[..expanded.len as usize];
            let text = core::str::from_utf8(bytes).unwrap_or("");
            for out in text.chars() {
                if !visit(index, out) {
                    return;
                }
            }
            continue;
        }
        if normalized {
            if let Some(replacement) = output_replacement(ch) {
                for out in replacement.chars() {
                    if !visit(index, out) {
                        return;
                    }
                }
                continue;
            }
        }
        if !visit(index, ch) {
            return;
        }
    }
}

fn push_char(text: &mut String, origin: &mut Vec<usize>, byte: usize, ch: char) {
    let mut buffer = [0u8; 4];
    let encoded = ch.encode_utf8(&mut buffer);
    text.push_str(encoded);
    for _ in 0..encoded.len() {
        origin.push(byte);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnnotationKind, Level, Renderer, Snippet, renderer::DecorStyle};

    fn none_expand(_ch: char) -> Option<ExpandedControl> {
        None
    }

    fn code_body(rendered: &str) -> String {
        rendered
            .lines()
            .find_map(|line| {
                let (gutter, body) = line.split_once('│').or_else(|| line.split_once('|'))?;
                gutter
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .map(|_| body.trim_start().to_owned())
            })
            .unwrap_or_default()
    }

    #[test]
    fn a_short_line_is_not_clipped() {
        let line = "mi klama";
        let laid = lay_out_line(line, &[(0, 2, 5)], 70, "…", none_expand);
        assert!(!laid.clipped);
        assert_eq!(laid.text, line);
    }

    #[test]
    fn a_middle_label_matches_the_renderer() {
        let line = format!(
            "{}mi tavla kkk do .i {}",
            "mi klama lo zarci .i ".repeat(8),
            "do citka lo plise .i ".repeat(8)
        );
        let start = line.find("kkk").expect("error word");
        let columns = code_columns(80, 1, 0);
        let laid = lay_out_line(&line, &[(start, start + 3, 2)], columns, "…", none_expand);
        let rendered = Renderer::plain()
            .decor_style(DecorStyle::Unicode)
            .term_width(80)
            .render(&[crate::Group::with_title(Level::ERROR.primary_title("err")).element(
                Snippet::source(&line)
                    .line_start(1)
                    .annotation(AnnotationKind::Primary.span(start..start + 3).label("x")),
            )]);
        let body = code_body(&rendered);
        assert!(laid.clipped, "{laid:?}\n{rendered}");
        assert!(
            laid.text.starts_with('…') && laid.text.ends_with('…'),
            "{}\n{body}",
            laid.text
        );
        assert!(laid.text.contains("kkk"), "{}", laid.text);
        assert_eq!(laid.text, body, "renderer:\n{rendered}");
    }

    #[test]
    fn a_long_span_matches_the_renderer() {
        let line = format!("mi nelci lo {} ku ku", "barda ".repeat(80));
        let start = line.find("lo ").expect("span start");
        let end = line.len() - " ku".len();
        let columns = code_columns(80, 1, 0);
        let laid = lay_out_line(&line, &[(start, end, 2)], columns, "…", none_expand);
        let rendered = Renderer::plain()
            .decor_style(DecorStyle::Unicode)
            .term_width(80)
            .render(&[crate::Group::with_title(Level::ERROR.primary_title("err")).element(
                Snippet::source(&line)
                    .line_start(1)
                    .annotation(AnnotationKind::Primary.span(start..end).label("x")),
            )]);
        let body = code_body(&rendered);
        assert!(laid.clipped, "{laid:?}\n{rendered}");
        assert_eq!(laid.text, body, "renderer:\n{rendered}");
    }

    #[test]
    fn a_wide_character_on_the_left_cut_keeps_the_alignment_space() {
        let line = "漢".repeat(40);
        let start = line.len() - "漢".len();
        let columns = code_columns(80, 1, 0);
        let laid = lay_out_line(
            &line,
            &[(start, line.len(), 2)],
            columns,
            "…",
            none_expand,
        );
        let rendered = Renderer::plain()
            .decor_style(DecorStyle::Unicode)
            .term_width(80)
            .render(&[crate::Group::with_title(Level::ERROR.primary_title("err")).element(
                Snippet::source(&line)
                    .line_start(1)
                    .annotation(AnnotationKind::Primary.span(start..line.len()).label("x")),
            )]);
        let body = code_body(&rendered);
        assert!(
            laid.text.starts_with("… "),
            "missing alignment space in {}\n{body}",
            laid.text
        );
        assert_eq!(laid.text, body, "renderer:\n{rendered}");
    }

    #[test]
    fn a_wide_character_that_straddles_the_left_edge_matches_the_renderer() {
        let line = "漢".repeat(80);
        let han = "漢".len();
        let start = 40 * han;
        let columns = code_columns(78, 1, 0);
        let laid = lay_out_line(
            &line,
            &[(start, start + han, 2)],
            columns,
            "…",
            none_expand,
        );
        let rendered = Renderer::plain()
            .decor_style(DecorStyle::Unicode)
            .term_width(78)
            .render(&[crate::Group::with_title(Level::ERROR.primary_title("err")).element(
                Snippet::source(&line)
                    .line_start(1)
                    .annotation(AnnotationKind::Primary.span(start..start + han).label("x")),
            )]);
        let body = code_body(&rendered);
        assert_eq!(laid.text, body, "renderer:\n{rendered}");
        assert!(laid.text.contains('漢'), "{}", laid.text);
    }

    #[test]
    fn combining_marks_stay_with_their_base() {
        let line = format!("mi nelci lo zoi gy. {} .gy ku ku", "a\u{0301}".repeat(180));
        let start = line.find("lo ").expect("span");
        let end = line.len();
        let columns = code_columns(80, 1, 0);
        let laid = lay_out_line(&line, &[(start, end, 2)], columns, "…", none_expand);
        assert!(laid.text.contains(".gy ku ku"), "{}", laid.text);
        let mut previous_width = 1usize;
        for ch in laid.text.chars() {
            let width = char_width(ch);
            if width == 0 {
                assert!(previous_width > 0, "orphan mark in {}", laid.text);
            }
            previous_width = width;
        }
    }

    fn render_body(line: &str, start: usize, end: usize, term_width: usize, ascii: bool) -> String {
        let decor = if ascii {
            DecorStyle::Ascii
        } else {
            DecorStyle::Unicode
        };
        let rendered = Renderer::plain()
            .decor_style(decor)
            .term_width(term_width)
            .render(&[crate::Group::with_title(Level::ERROR.primary_title("err")).element(
                Snippet::source(line)
                    .line_start(1)
                    .annotation(AnnotationKind::Primary.span(start..end).label("x")),
            )]);
        code_body(&rendered)
    }

    fn mapped_span(text: &str, origin: &[usize], start: usize, end: usize) -> Option<(usize, usize)> {
        let mut mapped_start = None;
        let mut mapped_end = 0usize;
        let mut byte = 0usize;
        for ch in text.chars() {
            let len = ch.len_utf8();
            let source = origin.get(byte).copied().unwrap_or(usize::MAX);
            if source != usize::MAX && source >= start && source < end {
                if mapped_start.is_none() {
                    mapped_start = Some(byte);
                }
                mapped_end = byte + len;
            }
            byte += len;
        }
        mapped_start.map(|mapped| (mapped, mapped_end))
    }

    fn assert_marks_follow_bases(text: &str) {
        let mut previous: Option<char> = None;
        for ch in text.chars() {
            if char_width(ch) == 0 {
                let base = previous.unwrap_or_else(|| panic!("leading mark in {text}"));
                assert!(
                    char_width(base) > 0 && base != '…' && base != '.' && !base.is_whitespace(),
                    "mark after {base:?} in {text}"
                );
            }
            previous = Some(ch);
        }
    }

    #[test]
    fn a_pass_one_window_renders_unchanged() {
        let term_width = 80;
        let columns = code_columns(term_width, 1, 0);
        for span_width in 65..=152 {
            for skew in 0..4 {
                for ascii in [false, true] {
                    let margin = if ascii { "..." } else { "…" };
                    let prefix = format!("{}{}", "a".repeat(skew), "漢".repeat(6));
                    let span = "b".repeat(span_width);
                    let line = format!("{prefix}{span} miamy\u{0301}\u{0308}z漢a");
                    let start = prefix.len();
                    let end = start + span.len();
                    let laid = lay_out_line(&line, &[(start, end, 2)], columns, margin, none_expand);
                    let body = render_body(&line, start, end, term_width, ascii);
                    assert_eq!(
                        laid.text, body,
                        "span {span_width} skew {skew} ascii {ascii}\n{body}"
                    );
                    assert_marks_follow_bases(&laid.text);
                    let (mapped_start, mapped_end) =
                        mapped_span(&laid.text, &laid.origin, start, end).unwrap_or_else(|| {
                            panic!(
                                "span disappeared at width {span_width} skew {skew} ascii {ascii}: {}",
                                laid.text
                            )
                        });
                    assert!(mapped_end > mapped_start, "{}", laid.text);
                    let again = render_body(&laid.text, mapped_start, mapped_end, term_width, ascii);
                    assert_eq!(
                        again, laid.text,
                        "second pass span {span_width} skew {skew} ascii {ascii}\n{again}"
                    );
                    assert_marks_follow_bases(&again);
                }
            }
        }
    }
}
