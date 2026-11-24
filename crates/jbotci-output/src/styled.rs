//! Colored text as lines of styled segments.
//!
//! A renderer builds this value first. One serializer turns it into ANSI.
//! Line breaks separate lines. They never sit inside a segment.

use bityzba::{invariant, new, requires};

/// Color and text attributes for one visible run.
///
/// `color` is the opening ANSI color sequence, such as `"\x1b[35m"`.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TextStyle {
    pub(crate) color: Option<&'static str>,
    pub(crate) italic: bool,
    pub(crate) strikethrough: bool,
}

impl TextStyle {
    #[requires(true)]
    #[ensures(ret.color.is_some() && !ret.italic && !ret.strikethrough)]
    pub(crate) fn for_depth(depth: usize) -> Self {
        Self {
            color: Some(ansi_color_for_depth(depth)),
            italic: false,
            strikethrough: false,
        }
    }

    #[requires(true)]
    #[ensures(ret.color == Some("\x1b[91m") && !ret.italic && !ret.strikethrough)]
    pub(crate) fn error_marker() -> Self {
        Self {
            color: Some("\x1b[91m"),
            italic: false,
            strikethrough: false,
        }
    }

    #[requires(true)]
    #[ensures(ret == (self.color.is_some() || self.italic || self.strikethrough))]
    pub(crate) fn is_styled(self) -> bool {
        self.color.is_some() || self.italic || self.strikethrough
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(crate) fn ansi_color_for_depth(depth: usize) -> &'static str {
    match depth % 6 {
        0 => "\x1b[35m",
        1 => "\x1b[94m",
        2 => "\x1b[32m",
        3 => "\x1b[31m",
        4 => "\x1b[33m",
        _ => "\x1b[96m",
    }
}

/// The break that ends one line and starts the next.
#[invariant(true)]
#[invariant(::Lf => true)]
#[invariant(::CrLf => true)]
#[invariant(::Cr => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineBreak {
    Lf,
    CrLf,
    Cr,
}

impl LineBreak {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
        }
    }

    #[requires(true)]
    #[ensures(ret == 1 || ret == 2)]
    fn char_count(self) -> usize {
        self.as_str().chars().count()
    }
}

/// One visible run on a line.
///
/// The text is not empty and holds no line break. A run of only whitespace
/// has the default style.
#[invariant(!text.is_empty(), "a segment contains text")]
#[invariant(
    !text.contains('\n') && !text.contains('\r'),
    "a line break separates lines"
)]
#[invariant(
    text.chars().any(|character| !character.is_whitespace())
        || (style.color.is_none() && !style.italic && !style.strikethrough),
    "whitespace-only text has no style"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StyledSegment {
    pub(crate) text: String,
    pub(crate) style: TextStyle,
}

/// One line. Adjacent segments do not share a style.
#[invariant(
    segments.windows(2).all(|pair| pair[0].style != pair[1].style),
    "adjacent segments on one line have different styles"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StyledLine {
    pub(crate) segments: Vec<StyledSegment>,
    pub(crate) ending: Option<LineBreak>,
}

/// A complete colored document.
///
/// Every line except the last ends with a line break. The last line does not.
#[invariant(
    lines.last().is_none_or(|line| line.ending.is_none()),
    "the last line has no break after it"
)]
#[invariant(
    lines.iter().rev().skip(1).all(|line| line.ending.is_some()),
    "every earlier line ends with a break"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StyledText {
    pub(crate) lines: Vec<StyledLine>,
}

impl StyledText {
    #[requires(true)]
    #[ensures(true)]
    #[expensive_ensures(color || (!ret.contains('\u{1b}') && ret == visible_text(self)))]
    pub(crate) fn serialize(&self, color: bool) -> String {
        let mut output = String::new();
        for line in &self.lines {
            if color {
                let mut current = TextStyle::default();
                for segment in &line.segments {
                    push_style_transition(&mut output, current, segment.style);
                    output.push_str(&segment.text);
                    current = segment.style;
                }
                if current.is_styled() {
                    output.push_str("\x1b[0m");
                }
            } else {
                for segment in &line.segments {
                    output.push_str(&segment.text);
                }
            }
            if let Some(ending) = line.ending {
                output.push_str(ending.as_str());
            }
        }
        output
    }

    /// Cut the document so its serialization fits in `max_chars` characters.
    ///
    /// The cut walks the typed text. It appends `…` as its own segment. It
    /// does not slice an escape sequence.
    #[requires(max_chars > 0)]
    #[ensures(ret.chars().count() <= max_chars)]
    #[ensures(self.serialize(color).chars().count() > max_chars || ret == self.serialize(color))]
    pub(crate) fn within_char_budget(&self, max_chars: usize, color: bool) -> String {
        let full = self.serialize(color);
        if full.chars().count() <= max_chars {
            return full;
        }
        let visible = visible_char_count(self);
        let mut low = 0usize;
        let mut high = visible;
        let mut best = 0usize;
        while low <= high {
            let mid = low + (high - low) / 2;
            let candidate = prefix_with_ellipsis(self, mid);
            if candidate.serialize(color).chars().count() <= max_chars {
                best = mid;
                if mid == visible {
                    break;
                }
                low = mid + 1;
            } else if mid == 0 {
                break;
            } else {
                high = mid - 1;
            }
        }
        prefix_with_ellipsis(self, best).serialize(color)
    }
}

#[requires(true)]
#[ensures(ret.lines.len() == parts.len() || parts.is_empty() || ret.lines.len() > parts.len())]
pub(crate) fn join_with_lf(parts: &[StyledText]) -> StyledText {
    let mut builder = StyledBuilder::new();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            builder.end_line(LineBreak::Lf);
        }
        builder.append_document(part);
    }
    builder.finish()
}

#[requires(!text.is_empty())]
#[ensures(!ret.lines.is_empty())]
pub(crate) fn plain_line(text: &str) -> StyledText {
    let mut builder = StyledBuilder::new();
    builder.push_text(text, TextStyle::default());
    builder.finish()
}

#[invariant(true)]
pub(crate) struct StyledBuilder {
    finished: Vec<StyledLine>,
    current: Vec<StyledSegment>,
}

impl StyledBuilder {
    #[requires(true)]
    #[ensures(ret.finished.is_empty() && ret.current.is_empty())]
    pub(crate) fn new() -> Self {
        Self {
            finished: Vec::new(),
            current: Vec::new(),
        }
    }

    /// Append text. Line breaks become line endings. Whitespace-only pieces
    /// lose their style.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn push_text(&mut self, text: &str, style: TextStyle) {
        let mut rest = text;
        while let Some((content, kind, after)) = split_first_line_break(rest) {
            self.push_segment(content, style);
            self.end_line(kind);
            rest = after;
        }
        self.push_segment(rest, style);
    }

    /// Put one inter-child space on the current line.
    ///
    /// A space that would start a line, or sit immediately before a line
    /// break, is not a separator. The caller passes that fact.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn push_separator(&mut self, next_starts_with_break: bool) {
        if next_starts_with_break || self.current.is_empty() {
            return;
        }
        self.push_text(" ", TextStyle::default());
    }

    #[requires(true)]
    #[ensures(self.current.is_empty())]
    pub(crate) fn end_line(&mut self, ending: LineBreak) {
        let segments = std::mem::take(&mut self.current);
        self.finished.push(new!(StyledLine {
            segments,
            ending: Some(ending),
        }));
    }

    #[requires(true)]
    #[ensures(true)]
    fn append_document(&mut self, document: &StyledText) {
        for line in &document.lines {
            for segment in &line.segments {
                self.push_segment(&segment.text, segment.style);
            }
            if let Some(ending) = line.ending {
                self.end_line(ending);
            }
        }
    }

    #[requires(true)]
    #[ensures(!ret.lines.is_empty())]
    #[ensures(ret.lines.last().is_some_and(|line| line.ending.is_none()))]
    pub(crate) fn finish(mut self) -> StyledText {
        let segments = std::mem::take(&mut self.current);
        self.finished.push(new!(StyledLine {
            segments,
            ending: None,
        }));
        new!(StyledText {
            lines: self.finished,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_segment(&mut self, text: &str, style: TextStyle) {
        if text.is_empty() {
            return;
        }
        let style = if text.chars().all(char::is_whitespace) {
            TextStyle::default()
        } else {
            style
        };
        if self
            .current
            .last()
            .is_some_and(|segment| segment.style == style)
        {
            let mut combined = self.current.pop().expect("segment").text.clone();
            combined.push_str(text);
            self.current.push(new!(StyledSegment {
                text: combined,
                style,
            }));
            return;
        }
        self.current.push(new!(StyledSegment {
            text: text.to_owned(),
            style,
        }));
    }
}

#[requires(true)]
#[ensures(true)]
fn split_first_line_break(text: &str) -> Option<(&str, LineBreak, &str)> {
    for (index, character) in text.char_indices() {
        match character {
            '\n' => return Some((&text[..index], LineBreak::Lf, &text[index + 1..])),
            '\r' => {
                if text.as_bytes().get(index + 1) == Some(&b'\n') {
                    return Some((&text[..index], LineBreak::CrLf, &text[index + 2..]));
                }
                return Some((&text[..index], LineBreak::Cr, &text[index + 1..]));
            }
            _ => {}
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn push_style_transition(output: &mut String, from: TextStyle, to: TextStyle) {
    if from == to {
        return;
    }
    if !to.is_styled() {
        output.push_str("\x1b[0m");
        return;
    }
    // Close attributes before opening the next color. A following bracket can
    // then keep the attribute close next to the text, as in `\x1b[3mtoi\x1b[23m`.
    if from.is_styled() && from.italic && !to.italic {
        output.push_str("\x1b[23m");
    }
    if from.is_styled() && from.strikethrough && !to.strikethrough {
        output.push_str("\x1b[29m");
    }
    if !from.is_styled() || to.color != from.color {
        if let Some(color) = to.color {
            output.push_str(color);
        } else if from.color.is_some() {
            output.push_str("\x1b[39m");
        }
    }
    if to.italic && !from.italic {
        output.push_str("\x1b[3m");
    }
    if to.strikethrough && !from.strikethrough {
        output.push_str("\x1b[9m");
    }
}

#[requires(true)]
#[ensures(true)]
fn visible_text(text: &StyledText) -> String {
    let mut output = String::new();
    for line in &text.lines {
        for segment in &line.segments {
            output.push_str(&segment.text);
        }
        if let Some(ending) = line.ending {
            output.push_str(ending.as_str());
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn visible_char_count(text: &StyledText) -> usize {
    text.lines
        .iter()
        .map(|line| {
            line.segments
                .iter()
                .map(|segment| segment.text.chars().count())
                .sum::<usize>()
                + line.ending.map(LineBreak::char_count).unwrap_or(0)
        })
        .sum()
}

/// Keep the first `keep` visible characters, then append an unstyled `…`.
///
/// A line break counts as its own characters. A break is kept whole or not
/// at all, so a cut never leaves a bare `\r` from a `\r\n` pair inside a
/// segment.
#[requires(true)]
#[ensures(ret.serialize(false).ends_with('…'))]
fn prefix_with_ellipsis(text: &StyledText, keep: usize) -> StyledText {
    let mut builder = StyledBuilder::new();
    let mut remaining = keep;
    for line in &text.lines {
        for segment in &line.segments {
            if remaining == 0 {
                builder.push_text("…", TextStyle::default());
                return builder.finish();
            }
            let count = segment.text.chars().count();
            if count <= remaining {
                builder.push_text(&segment.text, segment.style);
                remaining -= count;
            } else {
                let end = segment
                    .text
                    .char_indices()
                    .nth(remaining)
                    .map(|(index, _)| index)
                    .unwrap_or(segment.text.len());
                builder.push_text(&segment.text[..end], segment.style);
                builder.push_text("…", TextStyle::default());
                return builder.finish();
            }
        }
        if let Some(ending) = line.ending {
            let width = ending.char_count();
            if remaining < width {
                builder.push_text("…", TextStyle::default());
                return builder.finish();
            }
            remaining -= width;
            builder.end_line(ending);
            if remaining == 0 {
                builder.push_text("…", TextStyle::default());
                return builder.finish();
            }
        }
    }
    builder.push_text("…", TextStyle::default());
    builder.finish()
}
