//! Adapted from [styled_buffer]
//!
//! [styled_buffer]: https://github.com/rust-lang/rust/blob/894f7a4ba6554d3797404bbf550d9919df060b97/compiler/rustc_errors/src/styled_buffer.rs

use alloc::{vec, vec::Vec};
use core::fmt::{self, Write};

use crate::renderer::stylesheet::Stylesheet;
use crate::renderer::ElementStyle;
use crate::Level;

#[derive(Debug)]
pub(crate) struct StyledBuffer {
    lines: Vec<Vec<StyledChar>>,
    limit: usize,
    bytes: usize,
    exceeded: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StyledChar {
    ch: char,
    style: ElementStyle,
}

impl StyledChar {
    pub(crate) const SPACE: Self = StyledChar::new(' ', ElementStyle::NoStyle);

    pub(crate) const fn new(ch: char, style: ElementStyle) -> StyledChar {
        StyledChar { ch, style }
    }
}

impl StyledBuffer {
    pub(crate) fn new(limit: usize) -> StyledBuffer {
        StyledBuffer {
            lines: vec![],
            limit,
            bytes: 0,
            exceeded: false,
        }
    }

    fn set_bytes(&mut self, bytes: Option<usize>) -> bool {
        match bytes {
            Some(bytes) if bytes <= self.limit && !self.exceeded => {
                self.bytes = bytes;
                true
            }
            _ => {
                self.exceeded = true;
                false
            }
        }
    }

    fn ensure_lines(&mut self, line: usize) -> bool {
        if self.exceeded {
            return false;
        }
        if line >= self.lines.len() {
            let Some(count) = line.checked_add(1) else {
                self.exceeded = true;
                return false;
            };
            let extra_newlines = line.saturating_sub(self.lines.len().saturating_sub(1));
            if !self.set_bytes(self.bytes.checked_add(extra_newlines)) {
                return false;
            }
            self.lines.resize(count, Vec::new());
        }
        true
    }

    pub(crate) fn render(
        &self,
        level: &Level<'_>,
        stylesheet: &Stylesheet,
        str: &mut impl Write,
    ) -> Result<(), fmt::Error> {
        if self.exceeded {
            return Err(fmt::Error);
        }
        for (i, line) in self.lines.iter().enumerate() {
            let mut current_style = stylesheet.none;
            for StyledChar { ch, style } in line {
                let ch_style = style.color_spec(level, stylesheet);
                if ch_style != current_style {
                    if !line.is_empty() {
                        write!(str, "{current_style:#}")?;
                    }
                    current_style = ch_style;
                    write!(str, "{current_style}")?;
                }
                write!(str, "{ch}")?;
            }
            write!(str, "{current_style:#}")?;
            if i != self.lines.len() - 1 {
                writeln!(str)?;
            }
        }
        Ok(())
    }

    /// Sets `chr` with `style` for given `line`, `col`.
    /// If `line` does not exist in our buffer, adds empty lines up to the given
    /// and fills the last line with unstyled whitespace.
    pub(crate) fn putc(&mut self, line: usize, col: usize, chr: char, style: ElementStyle) {
        if !self.ensure_lines(line) {
            return;
        }
        let Some(count) = col.checked_add(1) else {
            self.exceeded = true;
            return;
        };
        let padding = count.saturating_sub(self.lines[line].len());
        let replaced = self.lines[line].get(col).map_or(1, |old| old.ch.len_utf8());
        let bytes = self
            .bytes
            .checked_add(padding)
            .and_then(|bytes| bytes.checked_sub(replaced))
            .and_then(|bytes| bytes.checked_add(chr.len_utf8()));
        if !self.set_bytes(bytes) {
            return;
        }
        if padding > 0 {
            self.lines[line].resize(count, StyledChar::SPACE);
        }
        self.lines[line][col] = StyledChar::new(chr, style);
    }

    /// Sets `string` with `style` for given `line`, starting from `col`.
    /// If `line` does not exist in our buffer, adds empty lines up to the given
    /// and fills the last line with unstyled whitespace.
    pub(crate) fn puts(&mut self, line: usize, col: usize, string: &str, style: ElementStyle) {
        for (offset, c) in string.chars().enumerate() {
            self.putc(line, col + offset, c, style);
        }
    }

    /// For given `line` inserts `string` with `style` after old content of that line,
    /// adding lines if needed
    pub(crate) fn append(&mut self, line: usize, string: &str, style: ElementStyle) {
        if line >= self.lines.len() {
            self.puts(line, 0, string, style);
        } else {
            let col = self.lines[line].len();
            self.puts(line, col, string, style);
        }
    }

    pub(crate) fn replace(&mut self, line: usize, start: usize, end: usize, string: &str) {
        if self.exceeded || line >= self.lines.len() || start == end {
            return;
        }
        // If the replacement range would be out of bounds, do nothing, as we
        // can't replace things that don't exist.
        if start > self.lines[line].len() || end > self.lines[line].len() {
            return;
        };
        let removed = self.lines[line][start..end]
            .iter()
            .map(|cell| cell.ch.len_utf8())
            .sum::<usize>();
        let bytes = self
            .bytes
            .checked_sub(removed)
            .and_then(|bytes| bytes.checked_add(string.len()));
        if !self.set_bytes(bytes) {
            return;
        }
        self.lines[line].splice(
            start..end,
            string
                .chars()
                .map(|c| StyledChar::new(c, ElementStyle::LineNumber)),
        );
    }

    /// For given `line` inserts `string` with `style` before old content of that line,
    /// adding lines if needed
    pub(crate) fn prepend(&mut self, line: usize, string: &str, style: ElementStyle) {
        if !self.ensure_lines(line) {
            return;
        }
        let string_len = string.chars().count();

        if !self.lines[line].is_empty() {
            if !self.set_bytes(self.bytes.checked_add(string_len)) {
                return;
            }
            // Push the old content over to make room for new content.
            for _ in 0..string_len {
                self.lines[line].insert(0, StyledChar::SPACE);
            }
        }

        self.puts(line, 0, string, style);
    }

    pub(crate) fn num_lines(&self) -> usize {
        self.lines.len()
    }

    /// Set `style` for `line`, `col_start..col_end` range if:
    /// 1. That line and column range exist in `StyledBuffer`
    /// 2. `overwrite` is `true` or existing style is `Style::NoStyle` or `Style::Quotation`
    pub(crate) fn set_style_range(
        &mut self,
        line: usize,
        col_start: usize,
        col_end: usize,
        style: ElementStyle,
        overwrite: bool,
    ) {
        for col in col_start..col_end {
            self.set_style(line, col, style, overwrite);
        }
    }

    /// Set `style` for `line`, `col` if:
    /// 1. That line and column exist in `StyledBuffer`
    /// 2. `overwrite` is `true` or existing style is `Style::NoStyle` or `Style::Quotation`
    pub(crate) fn set_style(
        &mut self,
        line: usize,
        col: usize,
        style: ElementStyle,
        overwrite: bool,
    ) {
        if let Some(ref mut line) = self.lines.get_mut(line) {
            if let Some(StyledChar { style: s, .. }) = line.get_mut(col) {
                if overwrite || matches!(s, ElementStyle::NoStyle | ElementStyle::Quotation) {
                    *s = style;
                }
            }
        }
    }
}

#[cfg(test)]
mod bounded_tests {
    use super::*;

    #[test]
    fn layout_counts_utf8_and_refuses_growth_at_the_boundary() {
        let mut buffer = StyledBuffer::new(5);
        buffer.putc(0, 0, '😀', ElementStyle::NoStyle);
        buffer.putc(0, 1, 'a', ElementStyle::NoStyle);
        assert_eq!(buffer.bytes, 5);
        assert!(!buffer.exceeded);
        let capacity = buffer.lines[0].capacity();
        buffer.putc(0, 2, 'b', ElementStyle::NoStyle);
        assert!(buffer.exceeded);
        assert_eq!(buffer.lines[0].len(), 2);
        assert_eq!(buffer.lines[0].capacity(), capacity);
    }

    #[test]
    fn layout_refuses_padding_before_it_resizes_a_line() {
        let mut buffer = StyledBuffer::new(1);
        buffer.putc(0, 1000, 'x', ElementStyle::NoStyle);
        assert!(buffer.exceeded);
        assert_eq!(buffer.lines[0].capacity(), 0);
        let mut buffer = StyledBuffer::new(1);
        buffer.putc(1000, 0, 'x', ElementStyle::NoStyle);
        assert!(buffer.exceeded);
        assert!(buffer.lines.is_empty());
    }

    #[test]
    fn replacing_a_character_accounts_for_its_utf8_size() {
        let mut buffer = StyledBuffer::new(4);
        buffer.puts(0, 0, "aa", ElementStyle::NoStyle);
        buffer.putc(0, 0, '😀', ElementStyle::NoStyle);
        assert!(buffer.exceeded);
        assert_eq!(buffer.bytes, 2);
        assert_eq!(buffer.lines[0][0].ch, 'a');
    }
}
