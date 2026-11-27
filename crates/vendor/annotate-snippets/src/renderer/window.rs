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
    pub clipped: bool,
    /// The laid-out line is still wider than the code column.
    ///
    /// Two labels that far apart need separate snippets. One long span does not:
    /// the middle replacement brings it back to the code column.
    pub wide: bool,
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
    let room = right.saturating_sub(left);
    let mut skipped = 0usize;
    let mut taken = 0usize;
    let mut started = false;
    let mut code = String::new();
    let mut code_origin = Vec::new();
    for_each_char(line, expand, true, |origin, ch| {
        let width = char_width(ch);
        if !started {
            if skipped < left {
                skipped += width;
                return true;
            }
            started = true;
        }
        if taken + width > room {
            return false;
        }
        taken += width;
        push_char(&mut code, &mut code_origin, origin, ch);
        true
    });
    if skipped > left {
        left = skipped;
    }
    let cut_left = margin.was_cut_left();
    let cut_right = display_len > right;
    let padding = str_width(margin_mark);
    let (skip_left, _left_width) = prefix_covering(&code, padding, cut_left);
    let rest = &code[skip_left..];
    let rest_origin = &code_origin[skip_left..];
    let skip_right = if cut_right {
        suffix_chars(rest, padding)
    } else {
        0
    };
    let mut cells = Vec::<(char, usize)>::new();
    if cut_left {
        for ch in margin_mark.chars() {
            cells.push((ch, usize::MAX));
        }
    }
    let rest_chars = rest.chars().count();
    let keep_chars = rest_chars.saturating_sub(skip_right);
    for (index, ch) in rest.chars().enumerate() {
        if index >= keep_chars {
            break;
        }
        let byte = rest
            .char_indices()
            .nth(index)
            .map(|(offset, _)| offset)
            .unwrap_or(0);
        let origin = rest_origin.get(byte).copied().unwrap_or(usize::MAX);
        cells.push((ch, origin));
    }
    if cut_right {
        for ch in margin_mark.chars() {
            cells.push((ch, usize::MAX));
        }
    }
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
        if start >= end || start >= cells.len() {
            continue;
        }
        let end = min(end, cells.len());
        let mut next = Vec::with_capacity(cells.len() - (end - start) + margin_mark.chars().count());
        next.extend_from_slice(&cells[..start]);
        for ch in margin_mark.chars() {
            next.push((ch, usize::MAX));
        }
        next.extend_from_slice(&cells[end..]);
        cells = next;
        cut_middle = true;
    }
    let mut text = String::new();
    let mut origin = Vec::new();
    for (ch, byte) in cells {
        push_char(&mut text, &mut origin, byte, ch);
    }
    let wide = str_width(&text) > column_width;
    LaidOutLine {
        text,
        origin,
        clipped: cut_left || cut_right || cut_middle,
        wide,
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
    let end = match source[start..].find('\n') {
        Some(relative) => {
            let newline = start + relative;
            if newline > start && source.as_bytes()[newline - 1] == b'\r' {
                newline - 1
            } else {
                newline
            }
        }
        None => source.len(),
    };
    (start, end)
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

fn prefix_covering(text: &str, padding: usize, enabled: bool) -> (usize, usize) {
    if !enabled {
        return (0, 0);
    }
    let mut bytes = 0usize;
    let mut width = 0usize;
    for ch in text.chars() {
        width += char_width(ch);
        bytes += ch.len_utf8();
        if width >= padding {
            break;
        }
    }
    (bytes, width)
}

fn suffix_chars(text: &str, padding: usize) -> usize {
    let mut count = 0usize;
    let mut width = 0usize;
    for ch in text.chars().rev() {
        width += char_width(ch);
        count += 1;
        if width >= padding {
            break;
        }
    }
    count
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
        assert!(!laid.wide);
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
}
