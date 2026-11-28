//! Colored text as lines of styled segments.
//!
//! A renderer builds this value first. One serializer turns it into ANSI.
//! Line breaks separate lines. They never sit inside a segment.

use std::io::{self, Write};

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, expensive_invariant, invariant, new, requires};

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
///
/// `Lf` is a newline, `CrLf` is a carriage return followed by a newline,
/// and `Cr` is a carriage return. The variants have no fields.
#[invariant(true)]
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
}

/// One visible run on a line.
///
/// The text is not empty and holds no line break. A run of only whitespace
/// has the default style. The serializer escapes other C0 controls, DEL, and
/// C1 controls when it writes the run.
///
/// The builder checks the appended piece. Whole-text scans live in the
/// expensive invariants.
#[invariant(!text.is_empty(), "a segment contains text")]
#[expensive_invariant(
    !text.contains('\n') && !text.contains('\r'),
    "a line break separates lines"
)]
#[expensive_invariant(
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
///
/// The builder checks the new pair when it appends a segment. The full
/// window scan is expensive.
#[invariant(true)]
#[expensive_invariant(
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
///
/// Closing a line checks that line and the line before it. The scan of every
/// earlier line is expensive.
#[invariant(
    lines.last().is_none_or(|line| line.ending.is_none()),
    "the last line has no break after it"
)]
#[expensive_invariant(
    lines.iter().rev().skip(1).all(|line| line.ending.is_some()),
    "every earlier line ends with a break"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StyledText {
    pub(crate) lines: Vec<StyledLine>,
}

const RESET_BYTES: &[u8] = b"\x1b[0m";

impl StyledText {
    /// Write the document.
    ///
    /// CR and LF stay line breaks. Every other C0 control, DEL, and every C1
    /// control is a visible `\u{...}` escape, in color mode and in plain mode.
    /// Color mode may also write its own ANSI sequences. Those sequences are
    /// the only raw ESC bytes.
    #[requires(true)]
    #[ensures(true)]
    #[expensive_ensures(
        serialization_obeys_control_rule(&ret, color)
            && (color || (ret == escaped_visible_text(self)))
    )]
    pub(crate) fn serialize(&self, color: bool) -> String {
        let mut output = Vec::new();
        self.write_to(&mut output, color, None)
            .expect("vector write");
        String::from_utf8(output).expect("styled output is UTF-8")
    }

    /// Write the document into a byte budget and stop when the caller cancels.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn write_limited(
        &self,
        color: bool,
        limit: crate::budget::ByteLimit,
        cancellation: &jbotci_interrupt::CancellationToken,
    ) -> io::Result<String> {
        let mut writer = crate::budget::OutputBudget::new(limit).writer();
        self.write_to(&mut writer, color, Some(cancellation))?;
        writer
            .into_string()
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    #[requires(true)]
    #[ensures(true)]
    fn write_to(
        &self,
        writer: &mut impl Write,
        color: bool,
        cancellation: Option<&jbotci_interrupt::CancellationToken>,
    ) -> io::Result<()> {
        for line in &self.lines {
            poll(cancellation)?;
            if color {
                let mut current = TextStyle::default();
                for segment in &line.segments {
                    poll(cancellation)?;
                    write_style_transition(writer, current, segment.style)?;
                    write_escaped(writer, &segment.text, cancellation)?;
                    current = segment.style;
                }
                if current.is_styled() {
                    writer.write_all(RESET_BYTES)?;
                }
            } else {
                for segment in &line.segments {
                    poll(cancellation)?;
                    write_escaped(writer, &segment.text, cancellation)?;
                }
            }
            if let Some(ending) = line.ending {
                writer.write_all(ending.as_str().as_bytes())?;
            }
        }
        Ok(())
    }
}

/// Bytes already promised to the serializer for this document.
#[invariant(budget.used() <= budget.limit().get())]
struct ByteAccount {
    budget: crate::budget::OutputBudget,
    color: bool,
    open: TextStyle,
}

#[invariant(true)]
pub(crate) struct StyledBuilder {
    finished: Vec<StyledLine>,
    current: Vec<StyledSegment>,
    account: Option<ByteAccount>,
    error: Option<io::Error>,
}

impl StyledBuilder {
    #[requires(true)]
    #[ensures(ret.finished.is_empty() && ret.current.is_empty())]
    pub(crate) fn new() -> Self {
        Self {
            finished: Vec::new(),
            current: Vec::new(),
            account: None,
            error: None,
        }
    }

    /// Reject text once the serialized form would pass `limit`.
    ///
    /// `color` selects the same ANSI bytes the serializer writes.
    #[requires(limit > 0)]
    #[ensures(true)]
    pub(crate) fn set_byte_budget(&mut self, limit: usize, color: bool) {
        let limit = crate::budget::ByteLimit::new(limit).expect("positive byte budget");
        self.account = Some(new!(ByteAccount {
            budget: crate::budget::OutputBudget::new(limit),
            color,
            open: TextStyle::default(),
        }));
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn check_budget(&self) -> io::Result<()> {
        if self.error.is_some() {
            Err(output_too_large())
        } else {
            Ok(())
        }
    }

    /// Append text. Line breaks become line endings. Whitespace-only pieces
    /// lose their style. An adjacent CR and LF with no text between them
    /// become one CR LF break.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn push_text(&mut self, text: &str, style: TextStyle) {
        if self.error.is_some() {
            return;
        }
        let mut rest = text;
        while let Some((content, kind, after)) = split_first_line_break(rest) {
            self.push_segment(content, style);
            self.end_line(kind);
            if self.error.is_some() {
                return;
            }
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
    #[ensures(self.finished.last().is_none_or(|line| line.ending.is_some()))]
    #[ensures(self.finished.len() < 2 || self.finished[self.finished.len() - 2].ending.is_some())]
    pub(crate) fn end_line(&mut self, ending: LineBreak) {
        if self.error.is_some() {
            self.current.clear();
            return;
        }
        if self.current.is_empty()
            && ending == LineBreak::Lf
            && self
                .finished
                .last()
                .is_some_and(|line| line.ending == Some(LineBreak::Cr))
        {
            if !self.try_charge(1) {
                self.error = Some(output_too_large());
                return;
            }
            let line = self.finished.pop().expect("previous line");
            self.finished
                .push(line.with_data(data! { ending: Some(LineBreak::CrLf) }));
            return;
        }
        let reset = self
            .account
            .as_ref()
            .map(|account| {
                if account.color && account.open.is_styled() {
                    RESET_BYTES.len()
                } else {
                    0
                }
            })
            .unwrap_or(0);
        if !self.try_charge(reset + ending.as_str().len()) {
            self.current.clear();
            self.error = Some(output_too_large());
            return;
        }
        if let Some(account) = self.account.take() {
            self.account = Some(account.with_data(data! { open: TextStyle::default() }));
        }
        let segments = std::mem::take(&mut self.current);
        self.finished.push(new!(StyledLine {
            segments,
            ending: Some(ending),
        }));
    }

    #[cfg(test)]
    #[requires(true)]
    #[ensures(!ret.lines.is_empty())]
    #[ensures(ret.lines.last().is_some_and(|line| line.ending.is_none()))]
    pub(crate) fn finish(self) -> StyledText {
        self.try_finish()
            .expect("an unlimited document has no byte budget")
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|text| !text.lines.is_empty()) || ret.is_err())]
    #[ensures(ret.as_ref().is_ok_and(|text| text.lines.len() < 2 || text.lines[text.lines.len() - 2].ending.is_some()) || ret.is_err())]
    pub(crate) fn try_finish(mut self) -> io::Result<StyledText> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        let reset = self
            .account
            .as_ref()
            .map(|account| {
                if account.color && account.open.is_styled() {
                    RESET_BYTES.len()
                } else {
                    0
                }
            })
            .unwrap_or(0);
        if !self.try_charge(reset) {
            return Err(output_too_large());
        }
        let segments = std::mem::take(&mut self.current);
        self.finished.push(new!(StyledLine {
            segments,
            ending: None,
        }));
        Ok(new!(StyledText {
            lines: self.finished,
        }))
    }

    #[requires(!text.contains('\n') && !text.contains('\r'))]
    #[ensures(self.current.len() < 2 || self.current[self.current.len() - 2].style != self.current[self.current.len() - 1].style)]
    fn push_segment(&mut self, text: &str, style: TextStyle) {
        if text.is_empty() || self.error.is_some() {
            return;
        }
        let style = if text.chars().all(char::is_whitespace) {
            TextStyle::default()
        } else {
            style
        };
        let merging = self
            .current
            .last()
            .is_some_and(|segment| segment.style == style);
        if let Some(account) = self.account.as_ref() {
            let transition = if account.color && !merging {
                style_transition_len(account.open, style)
            } else {
                0
            };
            let room = account.budget.remaining().saturating_sub(transition);
            let Some(text_bytes) = escaped_byte_len_within(text, room) else {
                self.error = Some(output_too_large());
                return;
            };
            let open = if merging { account.open } else { style };
            if !self.try_charge(transition + text_bytes) {
                self.error = Some(output_too_large());
                return;
            }
            if let Some(account) = self.account.take() {
                self.account = Some(account.with_data(data! { open: open }));
            }
        }
        if merging {
            let segment = self.current.pop().expect("segment");
            let mut data = segment.into_data();
            data.text.push_str(text);
            self.current.push(StyledSegment::from_data(data));
            return;
        }
        self.current.push(new!(StyledSegment {
            text: text.to_owned(),
            style,
        }));
    }

    #[requires(true)]
    #[ensures(true)]
    fn try_charge(&mut self, bytes: usize) -> bool {
        let Some(account) = self.account.take() else {
            return true;
        };
        let data = account.into_data();
        let charged = data.budget.charge(bytes).is_ok();
        self.account = Some(ByteAccount::from_data(data));
        charged
    }
}

/// C0 controls, DEL, and C1 controls, other than CR, LF, and horizontal tab.
///
/// A horizontal tab is whitespace. It does not change terminal state.
/// Quoted text and diagnostic excerpts keep the tab byte.
/// A JSON string is the place a raw tab is unsafe. The JSON writer emits `\t` there.
#[requires(true)]
#[ensures(ret == (character != '\n' && character != '\r' && character != '\t' && ((character as u32) <= 0x1F || (character as u32) == 0x7F || ((character as u32) >= 0x80 && (character as u32) <= 0x9F))))]
pub(crate) fn is_raw_terminal_control(character: char) -> bool {
    let code = character as u32;
    character != '\n'
        && character != '\r'
        && character != '\t'
        && (code <= 0x1F || code == 0x7F || (0x80..=0x9F).contains(&code))
}

/// Replacement text for one source character before a diagnostic line is trimmed.
///
/// A tab returns none, so the line layout uses the renderer's own tab width.
/// Every other terminal control expands to the `\u{hex}` text this crate prints.
#[requires(true)]
#[ensures(true)]
pub(crate) fn expanded_terminal_control(
    character: char,
) -> Option<annotate_snippets::renderer::ExpandedControl> {
    if !is_raw_terminal_control(character) {
        return None;
    }
    let mut text = [0u8; 6];
    let len = control_escape_bytes(character, &mut text);
    Some(annotate_snippets::renderer::ExpandedControl {
        text,
        len: len as u8,
    })
}

/// Replace C0 controls, DEL, and C1 controls other than CR and LF with `\u{hex}`.
#[requires(true)]
#[ensures(true)]
pub(crate) fn escape_terminal_controls(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.chars().any(is_raw_terminal_control) {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut output = String::new();
    push_escaped_controls(&mut output, text);
    std::borrow::Cow::Owned(output)
}

/// Escape one terminal field. CR and LF become `\u{hex}` too.
///
/// Styled text keeps those bytes as line breaks. A single-line field does not.
#[requires(true)]
#[ensures(!ret.contains('\n') && !ret.contains('\r'))]
pub(crate) fn escape_terminal_field(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.chars().any(is_single_line_control) {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut output = String::new();
    for character in text.chars() {
        if is_single_line_control(character) {
            push_control_escape(&mut output, character);
        } else {
            output.push(character);
        }
    }
    std::borrow::Cow::Owned(output)
}

/// Turn CR LF and a lone CR into LF. Escape every other terminal control.
///
/// Use this for a plain note. A styled note already has color bytes.
#[requires(true)]
#[ensures(!ret.contains('\r'))]
pub(crate) fn escape_terminal_note(text: &str) -> std::borrow::Cow<'_, str> {
    let normalized = normalize_note_breaks(text);
    match escape_terminal_controls(normalized.as_ref()) {
        std::borrow::Cow::Borrowed(_) => normalized,
        std::borrow::Cow::Owned(escaped) => std::borrow::Cow::Owned(escaped),
    }
}

/// Turn CR LF and a lone CR into LF. Keep every other byte.
///
/// A styled note already escaped its controls. This pass keeps the color bytes.
#[requires(true)]
#[ensures(!ret.contains('\r'))]
pub(crate) fn normalize_terminal_breaks(text: &str) -> std::borrow::Cow<'_, str> {
    normalize_note_breaks(text)
}

#[requires(true)]
#[ensures(ret == (is_raw_terminal_control(character) || character == '\n' || character == '\r'))]
fn is_single_line_control(character: char) -> bool {
    character == '\n' || character == '\r' || is_raw_terminal_control(character)
}

#[requires(true)]
#[ensures(!ret.contains('\r'))]
fn normalize_note_breaks(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.chars().any(|character| character == '\r') {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut output = String::with_capacity(text.len());
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\r' {
            if characters.peek() == Some(&'\n') {
                characters.next();
            }
            output.push('\n');
        } else {
            output.push(character);
        }
    }
    std::borrow::Cow::Owned(output)
}

/// Escaped excerpt text and a map from each original byte to an escaped byte.
///
/// An empty map means the text has no escaped control, so every original
/// offset is already an offset in `text`. A map starts at byte 0.
/// A source with no terminal control is borrowed.
#[invariant(map.is_empty() || map.first().copied() == Some(0))]
pub(crate) struct TerminalExcerpt<'text> {
    pub(crate) text: std::borrow::Cow<'text, str>,
    pub(crate) map: Vec<usize>,
}

#[requires(true)]
#[ensures(ret.map.is_empty() || ret.map.len() == text.len() + 1)]
pub(crate) fn escape_terminal_source(text: &str) -> TerminalExcerpt<'_> {
    if !text.chars().any(is_raw_terminal_control) {
        return new!(TerminalExcerpt {
            text: std::borrow::Cow::Borrowed(text),
            map: Vec::new(),
        });
    }
    // The offset table covers only this text. Callers clip before this copy.
    let mut output = String::new();
    let mut map = vec![0; text.len() + 1];
    let mut escaped_at = 0usize;
    let mut index = 0usize;
    for character in text.chars() {
        let len = character.len_utf8();
        let add = if is_raw_terminal_control(character) {
            let before = output.len();
            push_control_escape(&mut output, character);
            output.len() - before
        } else {
            output.push(character);
            len
        };
        for offset in index..index + len {
            map[offset] = escaped_at;
        }
        escaped_at += add;
        index += len;
    }
    map[text.len()] = escaped_at;
    new!(TerminalExcerpt {
        text: std::borrow::Cow::Owned(output),
        map,
    })
}

#[requires(true)]
#[ensures(true)]
fn push_escaped_controls(output: &mut String, text: &str) {
    let mut start = 0usize;
    for (index, character) in text.char_indices() {
        if !is_raw_terminal_control(character) {
            continue;
        }
        output.push_str(&text[start..index]);
        push_control_escape(output, character);
        start = index + character.len_utf8();
    }
    output.push_str(&text[start..]);
}

#[requires(true)]
#[ensures(true)]
fn push_control_escape(output: &mut String, character: char) {
    let mut bytes = [0u8; 6];
    let len = control_escape_bytes(character, &mut bytes);
    output.push_str(std::str::from_utf8(&bytes[..len]).expect("escape is ASCII"));
}

#[requires(true)]
#[ensures(ret == 5 || ret == 6)]
fn control_escape_bytes(character: char, bytes: &mut [u8; 6]) -> usize {
    let code = character as u32;
    let hex = b"0123456789abcdef";
    bytes[0] = b'\\';
    bytes[1] = b'u';
    bytes[2] = b'{';
    if code < 16 {
        bytes[3] = hex[code as usize];
        bytes[4] = b'}';
        5
    } else {
        bytes[3] = hex[((code >> 4) & 0xF) as usize];
        bytes[4] = hex[(code & 0xF) as usize];
        bytes[5] = b'}';
        6
    }
}

#[requires(true)]
#[ensures(true)]
fn write_control_escape(output: &mut impl Write, character: char) -> io::Result<()> {
    let mut bytes = [0u8; 6];
    let len = control_escape_bytes(character, &mut bytes);
    output.write_all(&bytes[..len])
}

#[requires(true)]
#[ensures(ret == 5 || ret == 6)]
fn control_escape_len(character: char) -> usize {
    if (character as u32) < 16 { 5 } else { 6 }
}

#[requires(true)]
#[ensures(true)]
fn escaped_byte_len(character: char) -> usize {
    if is_raw_terminal_control(character) {
        control_escape_len(character)
    } else {
        character.len_utf8()
    }
}

/// Count escaped bytes until `room` is passed. `None` means the text is larger.
#[requires(true)]
#[ensures(ret.is_some_and(|len| len <= room) || ret.is_none())]
pub(crate) fn escaped_byte_len_within(text: &str, room: usize) -> Option<usize> {
    let mut used = 0usize;
    for character in text.chars() {
        used = used.saturating_add(escaped_byte_len(character));
        if used > room {
            return None;
        }
    }
    Some(used)
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
fn write_style_transition(
    output: &mut impl Write,
    from: TextStyle,
    to: TextStyle,
) -> io::Result<()> {
    if from == to {
        return Ok(());
    }
    if !to.is_styled() {
        output.write_all(RESET_BYTES)?;
        return Ok(());
    }
    // Close attributes before opening the next color. A following bracket can
    // then keep the attribute close next to the text, as in `\x1b[3mtoi\x1b[23m`.
    if from.is_styled() && from.italic && !to.italic {
        output.write_all(b"\x1b[23m")?;
    }
    if from.is_styled() && from.strikethrough && !to.strikethrough {
        output.write_all(b"\x1b[29m")?;
    }
    if !from.is_styled() || to.color != from.color {
        if let Some(color) = to.color {
            output.write_all(color.as_bytes())?;
        } else if from.color.is_some() {
            output.write_all(b"\x1b[39m")?;
        }
    }
    if to.italic && !from.italic {
        output.write_all(b"\x1b[3m")?;
    }
    if to.strikethrough && !from.strikethrough {
        output.write_all(b"\x1b[9m")?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn style_transition_len(from: TextStyle, to: TextStyle) -> usize {
    if from == to {
        return 0;
    }
    if !to.is_styled() {
        return RESET_BYTES.len();
    }
    let mut len = 0usize;
    if from.is_styled() && from.italic && !to.italic {
        len += b"\x1b[23m".len();
    }
    if from.is_styled() && from.strikethrough && !to.strikethrough {
        len += b"\x1b[29m".len();
    }
    if !from.is_styled() || to.color != from.color {
        if let Some(color) = to.color {
            len += color.len();
        } else if from.color.is_some() {
            len += b"\x1b[39m".len();
        }
    }
    if to.italic && !from.italic {
        len += b"\x1b[3m".len();
    }
    if to.strikethrough && !from.strikethrough {
        len += b"\x1b[9m".len();
    }
    len
}

#[requires(true)]
#[ensures(true)]
fn write_escaped(
    writer: &mut impl Write,
    text: &str,
    cancellation: Option<&jbotci_interrupt::CancellationToken>,
) -> io::Result<()> {
    let mut start = 0usize;
    let mut since_poll = 0usize;
    for (index, character) in text.char_indices() {
        if since_poll == 1024 {
            poll(cancellation)?;
            since_poll = 0;
        }
        since_poll += 1;
        if !is_raw_terminal_control(character) {
            continue;
        }
        if index > start {
            writer.write_all(text[start..index].as_bytes())?;
        }
        write_control_escape(writer, character)?;
        start = index + character.len_utf8();
    }
    if start < text.len() {
        writer.write_all(text[start..].as_bytes())?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn poll(cancellation: Option<&jbotci_interrupt::CancellationToken>) -> io::Result<()> {
    if let Some(token) = cancellation {
        token
            .check()
            .map_err(|error| io::Error::new(io::ErrorKind::Interrupted, error))?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn output_too_large() -> io::Error {
    io::Error::new(io::ErrorKind::FileTooLarge, "output byte limit")
}

#[requires(true)]
#[ensures(true)]
fn escaped_visible_text(text: &StyledText) -> String {
    let mut output = String::new();
    for line in &text.lines {
        for segment in &line.segments {
            output.push_str(&escape_terminal_controls(&segment.text));
        }
        if let Some(ending) = line.ending {
            output.push_str(ending.as_str());
        }
    }
    output
}

/// The output has no raw C0 or C1 control except CR, LF, and color sequences.
#[requires(true)]
#[ensures(true)]
fn serialization_obeys_control_rule(text: &str, color: bool) -> bool {
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' {
            if !color || !skip_style_sequence(&mut characters) {
                return false;
            }
            continue;
        }
        if is_raw_terminal_control(character) {
            return false;
        }
    }
    true
}

#[requires(true)]
#[ensures(true)]
fn skip_style_sequence(characters: &mut std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    if characters.next() != Some('[') {
        return false;
    }
    let mut first = 0u8;
    let mut second = 0u8;
    let mut count = 0u8;
    loop {
        match characters.next() {
            Some(character) if character.is_ascii_digit() && count < 2 => {
                if count == 0 {
                    first = character as u8;
                } else {
                    second = character as u8;
                }
                count += 1;
            }
            Some('m') if count > 0 => {
                return emitted_style_parameter(first, second, count);
            }
            _ => return false,
        }
    }
}

/// `write_style_transition` emits only these SGR parameters.
#[requires(true)]
#[ensures(true)]
fn emitted_style_parameter(first: u8, second: u8, count: u8) -> bool {
    matches!(
        (count, first, second),
        (1, b'0', _)
            | (1, b'3', _)
            | (1, b'9', _)
            | (2, b'2', b'3')
            | (2, b'2', b'9')
            | (2, b'3', b'1')
            | (2, b'3', b'2')
            | (2, b'3', b'3')
            | (2, b'3', b'5')
            | (2, b'3', b'9')
            | (2, b'9', b'1')
            | (2, b'9', b'4')
            | (2, b'9', b'6')
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    #[requires(true)]
    #[ensures(true)]
    fn document_from(text: &str) -> StyledText {
        let mut builder = StyledBuilder::new();
        builder.push_text(text, TextStyle::default());
        builder.finish()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn one_chunk_keeps_crlf_whole_at_the_cut() {
        let document = document_from("a\r\nbbbb");
        assert_eq!(document.serialize(false), "a\r\nbbbb");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn two_pushes_keep_crlf_whole_at_the_cut() {
        let mut builder = StyledBuilder::new();
        builder.push_text("a\r", TextStyle::default());
        builder.push_text("\nbbbb", TextStyle::default());
        let document = builder.finish();
        assert_eq!(document.serialize(false), "a\r\nbbbb");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_lone_cr_stays_when_the_next_character_differs() {
        let document = document_from("a\rb");
        assert_eq!(document.serialize(false), "a\rb");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn controls_use_the_same_visible_escape_in_both_modes() {
        let mut builder = StyledBuilder::new();
        builder.push_text("a\u{1b}b\u{9f}\t\u{7f}", TextStyle::for_depth(0));
        let document = builder.finish();
        let plain = document.serialize(false);
        let colored = document.serialize(true);
        assert_eq!(plain, "a\\u{1b}b\\u{9f}\t\\u{7f}");
        assert!(colored.contains("a\\u{1b}b\\u{9f}\t\\u{7f}"));
        assert!(serialization_obeys_control_rule(&plain, false));
        assert!(serialization_obeys_control_rule(&colored, true));
        assert!(!plain.chars().any(is_raw_terminal_control));
        assert!(plain.contains('\t'));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_small_budget_does_not_copy_a_long_run() {
        let mut builder = StyledBuilder::new();
        builder.set_byte_budget(32, false);
        builder.push_text(&"a".repeat(1_000_000), TextStyle::default());
        let error = builder
            .try_finish()
            .expect_err("32 bytes cannot hold the run");
        assert_eq!(error.kind(), io::ErrorKind::FileTooLarge);
        let mut colored = StyledBuilder::new();
        colored.set_byte_budget(32, true);
        colored.push_text(&"a".repeat(1_000_000), TextStyle::for_depth(0));
        let colored_error = colored
            .try_finish()
            .expect_err("32 bytes cannot hold the colored run");
        assert_eq!(colored_error.kind(), io::ErrorKind::FileTooLarge);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_color_postcondition_accepts_only_emitted_styles() {
        assert!(serialization_obeys_control_rule("\u{1b}[0m", true));
        assert!(serialization_obeys_control_rule("\u{1b}[35m", true));
        assert!(serialization_obeys_control_rule("\u{1b}[94m", true));
        assert!(serialization_obeys_control_rule("\u{1b}[23m", true));
        assert!(serialization_obeys_control_rule("\u{1b}[39m", true));
        assert!(!serialization_obeys_control_rule("\u{1b}[2J", true));
        assert!(!serialization_obeys_control_rule("\u{1b}[35m", false));
        let excerpt = escape_terminal_source("ab\u{85}c\u{7f}");
        assert_eq!(excerpt.text.as_ref(), "ab\\u{85}c\\u{7f}");
        assert_eq!(excerpt.map[2], 2);
        assert_eq!(excerpt.map[4], 8);
        assert_eq!(excerpt.map[5], 9);
        assert_eq!(excerpt.map[6], excerpt.text.len());
        let clean = " ".repeat(1_000_000);
        let borrowed = escape_terminal_source(&clean);
        assert!(matches!(borrowed.text, std::borrow::Cow::Borrowed(_)));
        assert!(borrowed.map.is_empty());
        assert_eq!(borrowed.text.len(), clean.len());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_cancelled_serializer_returns_interrupted() {
        let source = jbotci_interrupt::CancellationSource::new();
        source.cancel();
        let document = document_from("hello");
        let error = document
            .write_limited(
                false,
                crate::budget::ByteLimit::new(100).expect("positive limit"),
                &source.token(),
            )
            .expect_err("a cancelled write stops");
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    }
}
