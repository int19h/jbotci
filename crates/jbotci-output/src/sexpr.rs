use crate::budget::{ByteLimit, OutputBudget};
use bityzba::{invariant, new, requires};
use std::io::Write;

use crate::{
    BracketRenderOptions, BracketSourceConstruct, BracketSourceFragment, BracketSourceFragmentRole,
    BracketSourceRange,
};

#[invariant(true)]
#[invariant(::Normal => true)]
#[invariant(::Elided => true)]
#[invariant(::Error => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafRole {
    Normal,
    Elided,
    Error,
}

#[invariant(true)]
#[invariant(::Leaf { .. } => true)]
#[invariant(::Node { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SExpr {
    Leaf {
        text: String,
        range: Option<BracketSourceRange>,
        role: LeafRole,
    },
    Node {
        children: Vec<SExpr>,
        range: Option<BracketSourceRange>,
        constructs: Vec<BracketSourceConstruct>,
    },
}

#[requires(true)]
#[ensures(matches!(&ret, SExpr::Node { children, .. } if children.is_empty()))]
pub(crate) fn empty_node() -> SExpr {
    SExpr::Node {
        children: Vec::new(),
        range: None,
        constructs: Vec::new(),
    }
}

#[requires(true)]
#[ensures(matches!(&ret, SExpr::Node { children, .. } if children.iter().all(|child| !is_empty(child))))]
pub(crate) fn node(children: Vec<SExpr>) -> SExpr {
    node_with_constructs(children, Vec::new())
}

#[requires(true)]
#[ensures(matches!(&ret, SExpr::Node { children, .. } if children.iter().all(|child| !is_empty(child))))]
pub(crate) fn node_with_constructs(
    children: Vec<SExpr>,
    constructs: Vec<BracketSourceConstruct>,
) -> SExpr {
    let mut node_children = Vec::new();
    for child in children {
        match child {
            other if !is_empty(&other) => node_children.push(other),
            _ => {}
        }
    }
    let range = union_child_ranges(&node_children);
    SExpr::Node {
        children: node_children,
        range,
        constructs,
    }
}

#[requires(true)]
#[ensures(matches!(&ret, SExpr::Leaf { .. }) || is_empty(&ret))]
pub(crate) fn leaf(text: String) -> SExpr {
    leaf_with_range(text, None)
}

#[requires(range.is_none_or(|range| range.byte_start <= range.byte_end))]
#[ensures(matches!(&ret, SExpr::Leaf { .. }) || is_empty(&ret))]
pub(crate) fn leaf_with_range(text: String, range: Option<BracketSourceRange>) -> SExpr {
    leaf_with_range_and_role(text, range, LeafRole::Normal)
}

#[requires(range.is_none_or(|range| range.byte_start <= range.byte_end))]
#[ensures(matches!(&ret, SExpr::Leaf { .. }) || is_empty(&ret))]
pub(crate) fn elided_leaf_with_range(text: String, range: Option<BracketSourceRange>) -> SExpr {
    leaf_with_range_and_role(text, range, LeafRole::Elided)
}

#[requires(range.is_none_or(|range| range.byte_start <= range.byte_end))]
#[ensures(matches!(&ret, SExpr::Leaf { role: LeafRole::Error, .. }))]
pub(crate) fn error_leaf_with_range(text: String, range: Option<BracketSourceRange>) -> SExpr {
    SExpr::Leaf {
        text,
        range,
        role: LeafRole::Error,
    }
}

#[requires(range.is_none_or(|range| range.byte_start <= range.byte_end))]
#[ensures(matches!(&ret, SExpr::Leaf { .. }) || is_empty(&ret))]
fn leaf_with_range_and_role(
    text: String,
    range: Option<BracketSourceRange>,
    role: LeafRole,
) -> SExpr {
    if text.is_empty() {
        empty_node()
    } else {
        SExpr::Leaf { text, range, role }
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_empty(expr: &SExpr) -> bool {
    match expr {
        SExpr::Leaf { text, role, .. } => *role != LeafRole::Error && text.is_empty(),
        SExpr::Node { children, .. } => children.is_empty(),
    }
}

#[invariant(true)]
struct FlattenFrame {
    remaining: Vec<SExpr>,
    range: Option<BracketSourceRange>,
    constructs: Vec<BracketSourceConstruct>,
    flattened: Vec<SExpr>,
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn flatten(expr: SExpr) -> SExpr {
    let mut frames = Vec::new();
    let mut next = Some(expr);
    let mut completed = None;
    loop {
        if let Some(expr) = next.take() {
            match expr {
                SExpr::Leaf { text, range, role } => {
                    completed = Some(SExpr::Leaf { text, range, role });
                }
                SExpr::Node {
                    mut children,
                    range,
                    constructs,
                } => {
                    children.reverse();
                    frames.push(FlattenFrame {
                        remaining: children,
                        range,
                        constructs,
                        flattened: Vec::new(),
                    });
                    continue;
                }
            }
        }

        if let Some(value) = completed.take() {
            if let Some(mut parent) = frames.pop() {
                if !is_empty(&value) {
                    parent.flattened.push(value);
                }
                frames.push(parent);
            } else {
                return value;
            }
        }

        let Some(mut frame) = frames.pop() else {
            panic!("S-expression flatten traversal lost its root frame");
        };
        if let Some(child) = frame.remaining.pop() {
            frames.push(frame);
            next = Some(child);
            continue;
        }

        let FlattenFrame {
            remaining: _,
            range,
            constructs,
            flattened,
        } = frame;
        let mut flattened = flattened;
        completed = Some(if flattened.len() == 1 {
            attach_constructs(flattened.remove(0), constructs)
        } else {
            SExpr::Node {
                children: flattened,
                range,
                constructs,
            }
        });
    }
}

#[requires(true)]
#[ensures(true)]
fn attach_constructs(mut expr: SExpr, inherited: Vec<BracketSourceConstruct>) -> SExpr {
    let SExpr::Node { constructs, .. } = &mut expr else {
        return expr;
    };
    for construct in inherited {
        if !constructs.contains(&construct) {
            constructs.push(construct);
        }
    }
    expr
}

/// Whether any leaf in the expression marks a recovery error (‼…‼).
#[requires(true)]
#[ensures(true)]
pub(crate) fn contains_error_leaf(expr: &SExpr) -> bool {
    match expr {
        SExpr::Leaf { role, .. } => *role == LeafRole::Error,
        SExpr::Node { children, .. } => children.iter().any(contains_error_leaf),
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LineStyleBoundary {
    visible_text: bool,
    styled_before_word: bool,
    styled_before_reset: TextStyle,
}

#[requires(true)]
#[ensures(true)]
fn colored_lines_have_style_boundaries(text: &str) -> bool {
    let mut previous_style = TextStyle::default();
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        if !line.ends_with("\x1b[0m") {
            return false;
        }
        let Some(boundary) = line_style_boundary(line) else {
            return false;
        };
        if !boundary.visible_text {
            return false;
        }
        if previous_style.is_styled() && !boundary.styled_before_word {
            return false;
        }
        previous_style = boundary.styled_before_reset;
    }
    true
}

#[requires(true)]
#[ensures(true)]
fn line_style_boundary(line: &str) -> Option<LineStyleBoundary> {
    let bytes = line.as_bytes();
    let mut index = 0;
    let mut style = TextStyle::default();
    let mut styled_before_reset = TextStyle::default();
    let mut styled_before_word = false;
    let mut saw_word = false;
    let mut visible_text = false;
    while index < bytes.len() {
        if bytes[index] == 0x1b {
            if bytes.get(index + 1) != Some(&b'[') {
                return None;
            }
            let parameters_start = index + 2;
            let end = bytes[parameters_start..]
                .iter()
                .position(|byte| *byte == b'm')
                .map(|offset| parameters_start + offset)?;
            let parameters = std::str::from_utf8(&bytes[parameters_start..end]).ok()?;
            for parameter in parameters.split(';') {
                match parameter.parse::<u32>().ok()? {
                    0 => {
                        styled_before_reset = style;
                        style = TextStyle::default();
                    }
                    3 => style.italic = true,
                    23 => style.italic = false,
                    9 => style.strikethrough = true,
                    29 => style.strikethrough = false,
                    30..=37 | 90..=97 => style.color = Some(""),
                    39 => style.color = None,
                    _ => {}
                }
            }
            index = end + 1;
            continue;
        }
        let character = line[index..].chars().next().expect("character boundary");
        visible_text = true;
        if !saw_word && !character.is_whitespace() {
            styled_before_word = style.is_styled();
            saw_word = true;
        }
        index += character.len_utf8();
    }
    Some(LineStyleBoundary {
        visible_text,
        styled_before_word,
        styled_before_reset,
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
#[expensive_ensures(ret.as_ref().is_ok_and(|text| {
    !options.color || colored_lines_have_style_boundaries(text)
}) || ret.is_err())]
pub(crate) fn render_bracketed_with_limit(
    expr: &SExpr,
    options: BracketRenderOptions,
    limit: ByteLimit,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> Result<String, crate::OutputError> {
    cancellation
        .check()
        .map_err(|_| crate::OutputError::Interrupted)?;
    let mut output = OutputBudget::new(limit).writer();
    write_bracketed(
        &mut output,
        0,
        TextStyle::default(),
        expr,
        options,
        cancellation,
    )
    .map_err(|error| {
        if error.kind() == std::io::ErrorKind::Interrupted {
            crate::OutputError::Interrupted
        } else {
            crate::OutputError::from_render_io(error, limit)
        }
    })?;
    Ok(output.into_string().expect("bracket output is UTF-8"))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
#[expensive_ensures(ret.as_ref().is_ok_and(|text| {
    !options.color || colored_lines_have_style_boundaries(text)
}) || ret.is_err())]
pub(crate) fn render_plain_with_limit(
    expr: &SExpr,
    options: BracketRenderOptions,
    limit: ByteLimit,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> Result<String, crate::OutputError> {
    cancellation
        .check()
        .map_err(|_| crate::OutputError::Interrupted)?;
    let mut output = OutputBudget::new(limit).writer();
    write_plain(&mut output, 0, expr, options, cancellation).map_err(|error| {
        if error.kind() == std::io::ErrorKind::Interrupted {
            crate::OutputError::Interrupted
        } else {
            crate::OutputError::from_render_io(error, limit)
        }
    })?;
    Ok(output.into_string().expect("plain output is UTF-8"))
}

#[requires(true)]
#[ensures(true)]
fn has_text(expr: &SExpr) -> bool {
    match expr {
        SExpr::Leaf { text, role, .. } => !text.is_empty() || *role == LeafRole::Error,
        SExpr::Node { children, .. } => children.iter().any(has_text),
    }
}

#[requires(true)]
#[ensures(true)]
fn starts_with_line_break(expr: &SExpr) -> bool {
    match expr {
        SExpr::Leaf {
            role: LeafRole::Error,
            ..
        } => false,
        SExpr::Leaf { text, .. } => text.starts_with('\n') || text.starts_with("\r\n"),
        SExpr::Node { children, .. } => children
            .iter()
            .find(|child| has_text(child))
            .is_some_and(starts_with_line_break),
    }
}

#[requires(true)]
#[ensures(true)]
fn ends_with_line_break(expr: &SExpr) -> bool {
    match expr {
        SExpr::Leaf {
            role: LeafRole::Error,
            ..
        } => false,
        SExpr::Leaf { text, .. } => text.ends_with('\n'),
        SExpr::Node { children, .. } => children
            .iter()
            .rev()
            .find(|child| has_text(child))
            .is_some_and(ends_with_line_break),
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct TextStyle {
    color: Option<&'static str>,
    italic: bool,
    strikethrough: bool,
}

impl TextStyle {
    #[requires(true)]
    #[ensures(ret.color.is_some())]
    fn for_depth(depth: usize) -> Self {
        Self {
            color: Some(ansi_color_for_depth(depth)),
            italic: false,
            strikethrough: false,
        }
    }

    #[requires(true)]
    #[ensures(ret.italic == (role == LeafRole::Elided))]
    #[ensures(ret.strikethrough == (role == LeafRole::Error))]
    fn for_leaf(depth: usize, role: LeafRole) -> Self {
        Self {
            color: Some(if role == LeafRole::Error {
                "\x1b[91m"
            } else {
                ansi_color_for_depth(depth)
            }),
            italic: role == LeafRole::Elided,
            strikethrough: role == LeafRole::Error,
        }
    }

    #[requires(true)]
    #[ensures(ret.color == Some("\x1b[91m") && !ret.italic && !ret.strikethrough)]
    fn error_marker() -> Self {
        Self {
            color: Some("\x1b[91m"),
            italic: false,
            strikethrough: false,
        }
    }

    #[requires(true)]
    #[ensures(ret == (self.color.is_some() || self.italic || self.strikethrough))]
    fn is_styled(self) -> bool {
        self.color.is_some() || self.italic || self.strikethrough
    }

    #[requires(true)]
    #[ensures(true)]
    fn write_open_from(
        self,
        previous: Self,
        output: &mut crate::budget::BudgetedWriter,
    ) -> std::io::Result<()> {
        if self.color != previous.color
            && let Some(color) = self.color
        {
            output.write_all(color.as_bytes())?;
        }
        if self.italic && !previous.italic {
            output.write_all(b"\x1b[3m")?;
        }
        if self.strikethrough && !previous.strikethrough {
            output.write_all(b"\x1b[9m")?;
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn write_close_attributes(
        self,
        output: &mut crate::budget::BudgetedWriter,
    ) -> std::io::Result<()> {
        if self.italic {
            output.write_all(b"\x1b[23m")?;
        }
        if self.strikethrough {
            output.write_all(b"\x1b[29m")?;
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn write_open_or_reset(
        self,
        output: &mut crate::budget::BudgetedWriter,
    ) -> std::io::Result<()> {
        if self.is_styled() {
            self.write_open_from(Self::default(), output)
        } else {
            output.write_all(b"\x1b[0m")
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn write_bracketed(
    output: &mut crate::budget::BudgetedWriter,
    depth: usize,
    parent_style: TextStyle,
    expr: &SExpr,
    options: BracketRenderOptions,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> std::io::Result<()> {
    // Stop before writing another node. The bytes already written are discarded
    // by the caller when this returns Interrupted.
    if cancellation.is_cancelled() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            jbotci_interrupt::Interrupted,
        ));
    }
    match expr {
        SExpr::Leaf { text, role, .. } => {
            if text.is_empty() && *role != LeafRole::Error {
                return Ok(());
            }
            write_leaf_text(output, depth, parent_style, text, *role, options, false)
        }
        SExpr::Node { children, .. } => {
            let mut children = children.iter().filter(|child| has_text(child)).peekable();
            let Some(first) = children.next() else {
                return Ok(());
            };
            if children.peek().is_none() {
                return write_bracketed(
                    output,
                    depth + 1,
                    parent_style,
                    first,
                    options,
                    cancellation,
                );
            }
            let node_style = TextStyle::for_depth(depth);
            if options.color {
                node_style.write_open_from(parent_style, output)?;
            }
            let (open, close) = bracket_pair(depth);
            write!(output, "{open}")?;
            if options.insert_hair_space {
                output.write_all("\u{200a}".as_bytes())?;
            }
            write_bracketed(output, depth + 1, node_style, first, options, cancellation)?;
            for child in children {
                output.write_all(b" ")?;
                write_bracketed(output, depth + 1, node_style, child, options, cancellation)?;
            }
            if options.insert_hair_space {
                output.write_all("\u{200a}".as_bytes())?;
            }
            write!(output, "{close}")?;
            if options.color {
                parent_style.write_open_or_reset(output)?;
            }
            Ok(())
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn write_plain(
    output: &mut crate::budget::BudgetedWriter,
    depth: usize,
    expr: &SExpr,
    options: BracketRenderOptions,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> std::io::Result<()> {
    if cancellation.is_cancelled() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            jbotci_interrupt::Interrupted,
        ));
    }
    match expr {
        SExpr::Leaf { text, role, .. } => {
            if text.is_empty() && *role != LeafRole::Error {
                return Ok(());
            }
            write_leaf_text(
                output,
                depth,
                TextStyle::default(),
                text,
                *role,
                options,
                true,
            )
        }
        SExpr::Node { children, .. } => {
            let mut children = children.iter().filter(|child| has_text(child)).peekable();
            let Some(first) = children.next() else {
                return Ok(());
            };
            if children.peek().is_none() {
                return write_plain(output, depth + 1, first, options, cancellation);
            }
            write_plain(output, depth + 1, first, options, cancellation)?;
            let mut previous = first;
            for child in children {
                if !ends_with_line_break(previous) && !starts_with_line_break(child) {
                    output.write_all(b" ")?;
                }
                write_plain(output, depth + 1, child, options, cancellation)?;
                previous = child;
            }
            Ok(())
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn write_leaf_text(
    output: &mut crate::budget::BudgetedWriter,
    depth: usize,
    parent_style: TextStyle,
    text: &str,
    role: LeafRole,
    options: BracketRenderOptions,
    plain: bool,
) -> std::io::Result<()> {
    if !options.color {
        if role == LeafRole::Error {
            output.write_all("‼".as_bytes())?;
        }
        output.write_all(text.as_bytes())?;
        if role == LeafRole::Error {
            output.write_all("‼".as_bytes())?;
        }
        return Ok(());
    }

    let style = TextStyle::for_leaf(depth, role);
    let mut current_style = parent_style;
    let mut leaf_open = false;
    if role == LeafRole::Error {
        let marker_style = TextStyle::error_marker();
        marker_style.write_open_from(current_style, output)?;
        output.write_all("‼".as_bytes())?;
        style.write_open_from(marker_style, output)?;
        current_style = style;
        leaf_open = true;
    } else if parent_style.is_styled() {
        style.write_open_from(parent_style, output)?;
        current_style = style;
        leaf_open = true;
    }

    let mut start = 0;
    for (index, character) in text.char_indices() {
        if character != '\n' {
            continue;
        }
        let content_end = if index > start && text.as_bytes()[index - 1] == b'\r' {
            index - 1
        } else {
            index
        };
        if content_end > start {
            if !leaf_open {
                style.write_open_from(current_style, output)?;
                current_style = style;
                leaf_open = true;
            }
            output.write_all(text[start..content_end].as_bytes())?;
        }
        if current_style.is_styled() {
            output.write_all(b"\x1b[0m")?;
            current_style = TextStyle::default();
            leaf_open = false;
        }
        output.write_all(text[content_end..=index].as_bytes())?;
        start = index + 1;
    }
    if start < text.len() {
        if !leaf_open {
            style.write_open_from(current_style, output)?;
            current_style = style;
            leaf_open = true;
        }
        output.write_all(text[start..].as_bytes())?;
    }

    if role == LeafRole::Error {
        if !leaf_open {
            style.write_open_from(current_style, output)?;
        }
        style.write_close_attributes(output)?;
        output.write_all("‼".as_bytes())?;
        parent_style.write_open_or_reset(output)?;
    } else if leaf_open {
        style.write_close_attributes(output)?;
        parent_style.write_open_or_reset(output)?;
    } else if !plain && current_style != parent_style {
        parent_style.write_open_or_reset(output)?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_bracketed_with_options(expr: &SExpr, options: BracketRenderOptions) -> String {
    render_bracketed_with_limit(
        expr,
        options,
        ByteLimit::new(usize::MAX).expect("positive limit"),
        &jbotci_interrupt::CancellationToken::never(),
    )
    .expect("unrestricted bracket rendering")
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_bracketed_source_fragments_with_options(
    expr: &SExpr,
    options: BracketRenderOptions,
) -> Vec<BracketSourceFragment> {
    render_source_fragments_at_depth(0, expr, options)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_bracketed_at_depth(
    depth: usize,
    expr: &SExpr,
    options: BracketRenderOptions,
) -> String {
    let mut output =
        OutputBudget::new(ByteLimit::new(usize::MAX).expect("positive limit")).writer();
    let parent_style = if depth == 0 {
        TextStyle::default()
    } else {
        TextStyle::for_depth(depth - 1)
    };
    write_bracketed(
        &mut output,
        depth,
        parent_style,
        expr,
        options,
        &jbotci_interrupt::CancellationToken::never(),
    )
    .expect("unrestricted bracket rendering");
    output.into_string().expect("bracket output is UTF-8")
}

#[requires(true)]
#[ensures(true)]
fn render_source_fragments_at_depth(
    depth: usize,
    expr: &SExpr,
    options: BracketRenderOptions,
) -> Vec<BracketSourceFragment> {
    match expr {
        SExpr::Leaf { text, range, role } => vec![BracketSourceFragment::Text {
            text: if *role == LeafRole::Error {
                format!("‼{text}‼")
            } else {
                text.clone()
            },
            range: *range,
            role: match role {
                LeafRole::Normal => BracketSourceFragmentRole::Normal,
                LeafRole::Elided => BracketSourceFragmentRole::Elided,
                LeafRole::Error => BracketSourceFragmentRole::Error,
            },
        }],
        SExpr::Node {
            children,
            range,
            constructs,
        } => {
            let rendered = children
                .iter()
                .flat_map(|child| render_source_fragments_at_depth(depth + 1, child, options))
                .filter(|fragment| !source_fragment_is_empty(fragment))
                .collect::<Vec<_>>();
            match rendered.as_slice() {
                [] => Vec::new(),
                [single] => vec![single.clone()],
                _ => {
                    let (open, close) = bracket_pair(depth);
                    let hair_space = if options.insert_hair_space {
                        "\u{200a}"
                    } else {
                        ""
                    };
                    let mut children = Vec::new();
                    children.push(BracketSourceFragment::Text {
                        text: format!("{open}{hair_space}"),
                        range: *range,
                        role: BracketSourceFragmentRole::Normal,
                    });
                    for (index, fragment) in rendered.into_iter().enumerate() {
                        if index > 0 {
                            children.push(BracketSourceFragment::Text {
                                text: " ".to_owned(),
                                range: None,
                                role: BracketSourceFragmentRole::Normal,
                            });
                        }
                        children.push(fragment);
                    }
                    children.push(BracketSourceFragment::Text {
                        text: format!("{hair_space}{close}"),
                        range: *range,
                        role: BracketSourceFragmentRole::Normal,
                    });
                    vec![BracketSourceFragment::Span {
                        range: *range,
                        constructs: constructs.clone(),
                        children,
                    }]
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_plain_source_fragments_with_options(
    expr: &SExpr,
    options: BracketRenderOptions,
) -> Vec<BracketSourceFragment> {
    render_plain_source_fragments_at_depth(0, expr, options)
}

#[requires(true)]
#[ensures(true)]
fn render_plain_source_fragments_at_depth(
    depth: usize,
    expr: &SExpr,
    options: BracketRenderOptions,
) -> Vec<BracketSourceFragment> {
    match expr {
        SExpr::Leaf { text, range, role } => vec![BracketSourceFragment::Text {
            text: if *role == LeafRole::Error {
                format!("‼{text}‼")
            } else {
                text.clone()
            },
            range: *range,
            role: match role {
                LeafRole::Normal => BracketSourceFragmentRole::Normal,
                LeafRole::Elided => BracketSourceFragmentRole::Elided,
                LeafRole::Error => BracketSourceFragmentRole::Error,
            },
        }],
        SExpr::Node {
            children,
            range,
            constructs,
        } => {
            let rendered = children
                .iter()
                .flat_map(|child| render_plain_source_fragments_at_depth(depth + 1, child, options))
                .filter(|fragment| !source_fragment_is_empty(fragment))
                .collect::<Vec<_>>();
            match rendered.as_slice() {
                [] => Vec::new(),
                [single] => vec![single.clone()],
                _ => {
                    let mut children = Vec::new();
                    for (index, fragment) in rendered.into_iter().enumerate() {
                        if index > 0 {
                            children.push(BracketSourceFragment::Text {
                                text: " ".to_owned(),
                                range: None,
                                role: BracketSourceFragmentRole::Normal,
                            });
                        }
                        children.push(fragment);
                    }
                    vec![BracketSourceFragment::Span {
                        range: *range,
                        constructs: constructs.clone(),
                        children,
                    }]
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn source_fragment_is_empty(fragment: &BracketSourceFragment) -> bool {
    match fragment {
        BracketSourceFragment::Text { text, .. } => text.is_empty(),
        BracketSourceFragment::Span { children, .. } => children.is_empty(),
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|range| range.byte_start <= range.byte_end))]
fn union_child_ranges(children: &[SExpr]) -> Option<BracketSourceRange> {
    let mut ranges = children.iter().filter_map(expr_range);
    let range = ranges.next()?;
    let mut byte_start = range.byte_start;
    let mut byte_end = range.byte_end;
    for child_range in ranges {
        byte_start = byte_start.min(child_range.byte_start);
        byte_end = byte_end.max(child_range.byte_end);
    }
    Some(new!(BracketSourceRange {
        byte_start,
        byte_end,
    }))
}

#[requires(true)]
#[ensures(ret.is_none_or(|range| range.byte_start <= range.byte_end))]
pub(crate) fn expr_range(expr: &SExpr) -> Option<BracketSourceRange> {
    match expr {
        SExpr::Leaf { range, .. } | SExpr::Node { range, .. } => *range,
    }
}

#[requires(true)]
#[ensures(!ret.0.is_empty() && !ret.1.is_empty())]
fn bracket_pair(depth: usize) -> (&'static str, &'static str) {
    match depth % 3 {
        0 => ("(", ")"),
        1 => ("[", "]"),
        _ => ("{", "}"),
    }
}

#[cfg(test)]
#[requires(true)]
#[ensures(!options.color && role != LeafRole::Error -> ret == old(text.clone()))]
#[ensures(!options.color && role == LeafRole::Error -> ret.starts_with('‼') && ret.ends_with('‼'))]
#[ensures(options.color && role != LeafRole::Error && !old(text.is_empty()) -> ret.starts_with(ansi_color_for_depth(depth)))]
#[ensures(options.color && role == LeafRole::Error -> ret.starts_with("\x1b[91m‼"))]
fn style_at_depth(
    depth: usize,
    text: String,
    options: BracketRenderOptions,
    role: LeafRole,
) -> String {
    render_bracketed_at_depth(
        depth,
        &SExpr::Leaf {
            text,
            range: None,
            role,
        },
        options,
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn ansi_color_for_depth(depth: usize) -> &'static str {
    match depth % 6 {
        0 => "\x1b[35m",
        1 => "\x1b[94m",
        2 => "\x1b[32m",
        3 => "\x1b[31m",
        4 => "\x1b[33m",
        _ => "\x1b[96m",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn flatten_collapses_deeply_nested_single_child_groups() {
        let expr = node(vec![node(vec![node(vec![leaf(String::from("foo"))])])]);

        let flattened = flatten(expr);

        assert_eq!(flattened, leaf(String::from("foo")));
        assert_eq!(
            render_bracketed_with_options(&flattened, BracketRenderOptions::default()),
            "foo"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn render_collapses_single_non_empty_child_after_filtering_empty_children() {
        let expr = node(vec![empty_node(), leaf(String::from("foo")), empty_node()]);

        assert_eq!(
            render_bracketed_with_options(&expr, BracketRenderOptions::default()),
            "foo"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn color_cycle_uses_bright_blue_and_bright_cyan() {
        let expected = [
            "\x1b[35m", "\x1b[94m", "\x1b[32m", "\x1b[31m", "\x1b[33m", "\x1b[96m",
        ];

        for (depth, color) in expected.iter().enumerate() {
            assert_eq!(ansi_color_for_depth(depth), *color);
            assert_eq!(ansi_color_for_depth(depth + expected.len()), *color);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colored_multiline_leaves_reset_each_line_in_brackets_and_plain() {
        let options = BracketRenderOptions {
            color: true,
            ..BracketRenderOptions::default()
        };
        for (text, expected) in [
            ("hello\nworld", "\x1b[35mhello\x1b[0m\n\x1b[35mworld\x1b[0m"),
            (
                "hello\r\nworld",
                "\x1b[35mhello\x1b[0m\r\n\x1b[35mworld\x1b[0m",
            ),
            (
                "hello\n\nworld",
                "\x1b[35mhello\x1b[0m\n\n\x1b[35mworld\x1b[0m",
            ),
        ] {
            let expr = SExpr::Leaf {
                text: text.to_owned(),
                range: None,
                role: LeafRole::Normal,
            };
            let bracketed = render_bracketed_with_limit(
                &expr,
                options,
                ByteLimit::new(usize::MAX).expect("positive limit"),
                &jbotci_interrupt::CancellationToken::never(),
            )
            .expect("colored brackets");
            let plain = render_plain_with_limit(
                &expr,
                options,
                ByteLimit::new(usize::MAX).expect("positive limit"),
                &jbotci_interrupt::CancellationToken::never(),
            )
            .expect("colored plain");
            assert_eq!(bracketed, expected, "{text:?}");
            assert_eq!(plain, expected, "{text:?}");
            assert!(
                plain
                    .lines()
                    .all(|line| line.is_empty() || line.ends_with("\x1b[0m")),
                "{plain:?}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colored_trailing_line_breaks_leave_blank_lines_bare() {
        let options = BracketRenderOptions {
            color: true,
            ..BracketRenderOptions::default()
        };
        let expr = SExpr::Leaf {
            text: "hello\n\n".to_owned(),
            range: None,
            role: LeafRole::Normal,
        };
        let expected = "\x1b[35mhello\x1b[0m\n\n";
        let bracketed = render_bracketed_with_limit(
            &expr,
            options,
            ByteLimit::new(usize::MAX).expect("positive limit"),
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("colored trailing brackets");
        let plain = render_plain_with_limit(
            &expr,
            options,
            ByteLimit::new(usize::MAX).expect("positive limit"),
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("colored trailing plain");
        assert_eq!(bracketed, expected);
        assert_eq!(plain, expected);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colored_leading_line_break_resets_and_reopens_the_parent_and_leaf() {
        let options = BracketRenderOptions {
            color: true,
            ..BracketRenderOptions::default()
        };
        let expr = node(vec![leaf("a".to_owned()), leaf("\nb".to_owned())]);
        let bracketed = render_bracketed_with_limit(
            &expr,
            options,
            ByteLimit::new(usize::MAX).expect("positive limit"),
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("colored leading brackets");
        let plain = render_plain_with_limit(
            &expr,
            options,
            ByteLimit::new(usize::MAX).expect("positive limit"),
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("colored leading plain");
        assert_eq!(
            bracketed,
            "\x1b[35m(\x1b[94ma\x1b[35m \x1b[94m\x1b[0m\n\x1b[94mb\x1b[35m)\x1b[0m"
        );
        assert_eq!(plain, "\x1b[94ma\x1b[0m\n\x1b[94mb\x1b[0m");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colored_multiline_error_leaves_mark_only_the_outer_bounds() {
        let options = BracketRenderOptions {
            color: true,
            ..BracketRenderOptions::default()
        };
        let expr = SExpr::Leaf {
            text: "hello\nworld".to_owned(),
            range: None,
            role: LeafRole::Error,
        };
        let expected = "\x1b[91m‼\x1b[9mhello\x1b[0m\n\x1b[91m\x1b[9mworld\x1b[29m‼\x1b[0m";
        let bracketed = render_bracketed_with_limit(
            &expr,
            options,
            ByteLimit::new(usize::MAX).expect("positive limit"),
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("colored error brackets");
        let plain = render_plain_with_limit(
            &expr,
            options,
            ByteLimit::new(usize::MAX).expect("positive limit"),
            &jbotci_interrupt::CancellationToken::never(),
        )
        .expect("colored error plain");
        assert_eq!(bracketed, expected);
        assert_eq!(plain, expected);
        assert_eq!(plain.matches('‼').count(), 2, "{plain:?}");
        assert!(plain.starts_with("\x1b[91m‼\x1b[9m"), "{plain:?}");
        assert!(plain.ends_with("\x1b[29m‼\x1b[0m"), "{plain:?}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colorized_text_restores_updated_parent_depth_color() {
        let options = BracketRenderOptions {
            color: true,
            ..BracketRenderOptions::default()
        };

        assert_eq!(
            style_at_depth(0, String::from("foo"), options, LeafRole::Normal),
            "\x1b[35mfoo\x1b[0m"
        );
        assert_eq!(
            style_at_depth(2, String::from("foo"), options, LeafRole::Normal),
            "\x1b[32mfoo\x1b[94m"
        );
        assert_eq!(
            style_at_depth(6, String::from("foo"), options, LeafRole::Normal),
            "\x1b[35mfoo\x1b[96m"
        );
        assert_eq!(
            style_at_depth(1, String::from("foo"), options, LeafRole::Elided),
            "\x1b[94m\x1b[3mfoo\x1b[23m\x1b[35m"
        );
        assert_eq!(
            style_at_depth(2, String::from("ku"), options, LeafRole::Error),
            "\x1b[91m‼\x1b[9mku\x1b[29m‼\x1b[94m"
        );
        assert_eq!(
            style_at_depth(2, String::new(), options, LeafRole::Error),
            "\x1b[91m‼\x1b[9m\x1b[29m‼\x1b[94m"
        );
    }
}
