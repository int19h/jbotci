use anyhow::{Result, bail};
use bityzba::{ensures, invariant, new, requires};
use proc_macro2::{LineColumn, Span};
use sha2::{Digest, Sha256};

use super::model::SourceIdentity;

#[invariant(!path.is_empty() && !line_starts.is_empty() && line_starts[0] == 0)]
#[derive(Debug)]
pub(crate) struct SourceMap<'source> {
    path: &'source str,
    text: &'source str,
    line_starts: Vec<usize>,
}

impl<'source> SourceMap<'source> {
    #[requires(!path.is_empty())]
    #[ensures(ret.path == path)]
    pub(crate) fn new(path: &'source str, text: &'source str) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(
            text.bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
        );
        new!(SourceMap {
            path,
            text,
            line_starts,
        })
    }

    #[requires(true)]
    #[ensures(ret == self.path)]
    pub(crate) fn path(&self) -> &'source str {
        self.path
    }

    #[requires(true)]
    #[ensures(ret.byte_start == 0 && ret.byte_end == self.text.len())]
    pub(crate) fn whole_file(&self) -> SourceIdentity {
        let (line_end, column_end) = self.line_column(self.text.len());
        new!(SourceIdentity {
            path: self.path.to_owned(),
            byte_start: 0,
            byte_end: self.text.len(),
            line_start: 1,
            column_start: 0,
            line_end,
            column_end,
        })
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|identity| identity.path == self.path))]
    pub(crate) fn span(&self, span: Span) -> Result<SourceIdentity> {
        let start = self.byte_offset(span.start())?;
        let end = self.byte_offset(span.end())?;
        if start > end {
            bail!(
                "span in `{}` has inverted byte offsets {start}..{end}",
                self.path
            );
        }
        Ok(new!(SourceIdentity {
            path: self.path.to_owned(),
            byte_start: start,
            byte_end: end,
            line_start: span.start().line,
            column_start: span.start().column,
            line_end: span.end().line,
            column_end: span.end().column,
        }))
    }

    #[requires(start <= end && end <= self.text.len())]
    #[requires(self.text.is_char_boundary(start) && self.text.is_char_boundary(end))]
    #[ensures(ret.byte_start == start && ret.byte_end == end)]
    pub(crate) fn byte_range(&self, start: usize, end: usize) -> SourceIdentity {
        let (line_start, column_start) = self.line_column(start);
        let (line_end, column_end) = self.line_column(end);
        new!(SourceIdentity {
            path: self.path.to_owned(),
            byte_start: start,
            byte_end: end,
            line_start,
            column_start,
            line_end,
            column_end,
        })
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|text| text.len() <= self.text.len()))]
    pub(crate) fn slice(&self, span: Span) -> Result<&'source str> {
        let identity = self.span(span)?;
        self.text
            .get(identity.byte_start..identity.byte_end)
            .ok_or_else(|| anyhow::anyhow!("span in `{}` is not on UTF-8 boundaries", self.path))
    }

    #[requires(offset <= self.text.len() && self.text.is_char_boundary(offset))]
    #[ensures(ret.0 > 0)]
    fn line_column(&self, offset: usize) -> (usize, usize) {
        let line_index = self
            .line_starts
            .partition_point(|line_start| *line_start <= offset)
            .saturating_sub(1);
        let line_start = self.line_starts[line_index];
        (
            line_index + 1,
            self.text[line_start..offset].chars().count(),
        )
    }

    #[requires(location.line > 0)]
    #[ensures(ret.as_ref().is_ok_and(|offset| *offset <= self.text.len()))]
    fn byte_offset(&self, location: LineColumn) -> Result<usize> {
        let Some(line_start) = self.line_starts.get(location.line - 1) else {
            bail!(
                "span line {} is outside `{}` ({} lines)",
                location.line,
                self.path,
                self.line_starts.len()
            );
        };
        let line_end = self
            .line_starts
            .get(location.line)
            .copied()
            .unwrap_or(self.text.len());
        let line = &self.text[*line_start..line_end];
        let offset = line
            .char_indices()
            .nth(location.column)
            .map(|(offset, _)| *line_start + offset)
            .or_else(|| {
                (location.column == line.chars().count()).then_some(line_end)
            });
        let Some(offset) = offset else {
            bail!(
                "span column {} on line {} is outside `{}`",
                location.column,
                location.line,
                self.path
            );
        };
        Ok(offset)
    }
}

#[requires(true)]
#[ensures(ret.len() == 64 && ret.bytes().all(|byte| byte.is_ascii_hexdigit()))]
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[requires(!prefix.is_empty())]
#[ensures(!ret.is_empty())]
pub(crate) fn record_id(prefix: &str, source: &SourceIdentity) -> String {
    format!("{prefix}:{}", source.stable_id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn byte_ranges_preserve_utf8_byte_positions_and_line_columns() {
        let source = SourceMap::new("sample.rs", "aé\nxyz\n");
        let identity = source.byte_range(4, 7);
        assert_eq!(identity.stable_id(), "sample.rs:4-7");
        assert_eq!(identity.line_start, 2);
        assert_eq!(identity.column_start, 0);
        assert_eq!(identity.line_end, 2);
        assert_eq!(identity.column_end, 3);
        assert_eq!(source.line_column(3), (1, 2));
        assert_eq!(
            source
                .byte_offset(LineColumn { line: 1, column: 2 })
                .expect("character column maps to a byte offset"),
            3
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sha256_is_canonical_lowercase_hex() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
