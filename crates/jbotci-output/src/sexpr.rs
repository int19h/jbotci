use crate::budget::ByteLimit;
use crate::styled::{self, StyledBuilder, StyledText, TextStyle};
use bityzba::{invariant, new, requires};

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

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
#[expensive_ensures(ret.as_ref().is_ok_and(|text| {
    text == &bracket_document(expr, options).serialize(options.color)
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
    let document =
        build_bracket_document(expr, options, cancellation, Some(limit)).map_err(|error| {
            if error.kind() == std::io::ErrorKind::Interrupted {
                crate::OutputError::Interrupted
            } else {
                crate::OutputError::from_render_io(error, limit)
            }
        })?;
    write_document(&document, options.color, limit, cancellation)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
#[expensive_ensures(ret.as_ref().is_ok_and(|text| {
    text == &plain_document(expr, options).serialize(options.color)
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
    let document =
        build_plain_document(expr, options, cancellation, Some(limit)).map_err(|error| {
            if error.kind() == std::io::ErrorKind::Interrupted {
                crate::OutputError::Interrupted
            } else {
                crate::OutputError::from_render_io(error, limit)
            }
        })?;
    write_document(&document, options.color, limit, cancellation)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
fn write_document(
    document: &styled::StyledText,
    color: bool,
    limit: ByteLimit,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> Result<String, crate::OutputError> {
    document
        .write_limited(color, limit, cancellation)
        .map_err(|error| crate::OutputError::from_render_io(error, limit))
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
        SExpr::Leaf { text, .. } => text.starts_with('\n') || text.starts_with('\r'),
        SExpr::Node { children, .. } => children
            .iter()
            .find(|child| has_text(child))
            .is_some_and(starts_with_line_break),
    }
}

#[requires(true)]
#[ensures(ret.italic == (role == LeafRole::Elided))]
#[ensures(ret.strikethrough == (role == LeafRole::Error))]
fn leaf_style(depth: usize, role: LeafRole) -> TextStyle {
    TextStyle {
        color: Some(if role == LeafRole::Error {
            "\x1b[91m"
        } else {
            styled::ansi_color_for_depth(depth)
        }),
        italic: role == LeafRole::Elided,
        strikethrough: role == LeafRole::Error,
    }
}

#[requires(true)]
#[ensures(true)]
fn check_cancellation(cancellation: &jbotci_interrupt::CancellationToken) -> std::io::Result<()> {
    if cancellation.is_cancelled() {
        Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            jbotci_interrupt::Interrupted,
        ))
    } else {
        Ok(())
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn bracket_document(expr: &SExpr, options: BracketRenderOptions) -> StyledText {
    build_bracket_document(
        expr,
        options,
        &jbotci_interrupt::CancellationToken::never(),
        None,
    )
    .expect("bracket document build does not cancel")
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn plain_document(expr: &SExpr, options: BracketRenderOptions) -> StyledText {
    build_plain_document(
        expr,
        options,
        &jbotci_interrupt::CancellationToken::never(),
        None,
    )
    .expect("plain document build does not cancel")
}

#[requires(true)]
#[ensures(true)]
fn build_bracket_document(
    expr: &SExpr,
    options: BracketRenderOptions,
    cancellation: &jbotci_interrupt::CancellationToken,
    budget: Option<ByteLimit>,
) -> std::io::Result<StyledText> {
    let mut builder = StyledBuilder::new();
    if let Some(limit) = budget {
        builder.set_byte_budget(limit.get(), options.color);
    }
    append_bracketed(&mut builder, 0, expr, options, cancellation)?;
    builder.try_finish()
}

#[requires(true)]
#[ensures(true)]
fn build_plain_document(
    expr: &SExpr,
    options: BracketRenderOptions,
    cancellation: &jbotci_interrupt::CancellationToken,
    budget: Option<ByteLimit>,
) -> std::io::Result<StyledText> {
    let mut builder = StyledBuilder::new();
    if let Some(limit) = budget {
        builder.set_byte_budget(limit.get(), options.color);
    }
    append_plain(&mut builder, 0, expr, options, cancellation)?;
    builder.try_finish()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn bracket_document_at_depth(
    depth: usize,
    expr: &SExpr,
    options: BracketRenderOptions,
) -> StyledText {
    let mut builder = StyledBuilder::new();
    append_bracketed(
        &mut builder,
        depth,
        expr,
        options,
        &jbotci_interrupt::CancellationToken::never(),
    )
    .expect("bracket document build does not cancel");
    builder.finish()
}

#[requires(true)]
#[ensures(true)]
fn append_bracketed(
    builder: &mut StyledBuilder,
    depth: usize,
    expr: &SExpr,
    options: BracketRenderOptions,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> std::io::Result<()> {
    check_cancellation(cancellation)?;
    builder.check_budget()?;
    match expr {
        SExpr::Leaf { text, role, .. } => {
            if text.is_empty() && *role != LeafRole::Error {
                return Ok(());
            }
            append_leaf(builder, depth, text, *role)
        }
        SExpr::Node { children, .. } => {
            let children = children
                .iter()
                .filter(|child| has_text(child))
                .collect::<Vec<_>>();
            let Some(first) = children.first() else {
                return Ok(());
            };
            if children.len() == 1 {
                return append_bracketed(builder, depth + 1, first, options, cancellation);
            }
            let node_style = TextStyle::for_depth(depth);
            let (open, close) = bracket_pair(depth);
            let mut open_token = String::from(open);
            if options.insert_hair_space {
                open_token.push('\u{200a}');
            }
            builder.push_text(&open_token, node_style);
            builder.check_budget()?;
            for (index, child) in children.iter().enumerate() {
                if index > 0 {
                    builder.push_separator(starts_with_line_break(child));
                    builder.check_budget()?;
                }
                append_bracketed(builder, depth + 1, child, options, cancellation)?;
            }
            let mut close_token = String::new();
            if options.insert_hair_space {
                close_token.push('\u{200a}');
            }
            close_token.push_str(close);
            builder.push_text(&close_token, node_style);
            builder.check_budget()
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn append_plain(
    builder: &mut StyledBuilder,
    depth: usize,
    expr: &SExpr,
    options: BracketRenderOptions,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> std::io::Result<()> {
    check_cancellation(cancellation)?;
    builder.check_budget()?;
    match expr {
        SExpr::Leaf { text, role, .. } => {
            if text.is_empty() && *role != LeafRole::Error {
                return Ok(());
            }
            append_leaf(builder, depth, text, *role)
        }
        SExpr::Node { children, .. } => {
            let children = children
                .iter()
                .filter(|child| has_text(child))
                .collect::<Vec<_>>();
            for (index, child) in children.iter().enumerate() {
                if index > 0 {
                    builder.push_separator(starts_with_line_break(child));
                    builder.check_budget()?;
                }
                append_plain(builder, depth + 1, child, options, cancellation)?;
            }
            Ok(())
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn append_leaf(
    builder: &mut StyledBuilder,
    depth: usize,
    text: &str,
    role: LeafRole,
) -> std::io::Result<()> {
    let style = leaf_style(depth, role);
    if role == LeafRole::Error {
        let marker = TextStyle::error_marker();
        builder.push_text("‼", marker);
        builder.push_text(text, style);
        builder.push_text("‼", marker);
    } else {
        builder.push_text(text, style);
    }
    builder.check_budget()
}

#[cfg(test)]
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

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
pub(crate) fn render_bracketed_at_depth(
    depth: usize,
    expr: &SExpr,
    options: BracketRenderOptions,
) -> String {
    bracket_document_at_depth(depth, expr, options).serialize(options.color)
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
#[ensures(options.color && role != LeafRole::Error && old(!text.is_empty()) && old(text.chars().any(|character| !character.is_whitespace())) && old(!text.starts_with('\n') && !text.starts_with('\r')) -> ret.starts_with(styled::ansi_color_for_depth(depth)))]
#[ensures(options.color && role == LeafRole::Error && old(!text.starts_with('\n') && !text.starts_with('\r')) -> ret.starts_with("\x1b[91m‼"))]
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
            assert_eq!(styled::ansi_color_for_depth(depth), *color);
            assert_eq!(styled::ansi_color_for_depth(depth + expected.len()), *color);
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
            "\x1b[35m(\x1b[94ma\x1b[0m\n\x1b[94mb\x1b[35m)\x1b[0m"
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
            "\x1b[32mfoo\x1b[0m"
        );
        assert_eq!(
            style_at_depth(6, String::from("foo"), options, LeafRole::Normal),
            "\x1b[35mfoo\x1b[0m"
        );
        assert_eq!(
            style_at_depth(1, String::from("foo"), options, LeafRole::Elided),
            "\x1b[94m\x1b[3mfoo\x1b[0m"
        );
        assert_eq!(
            style_at_depth(2, String::from("ku"), options, LeafRole::Error),
            "\x1b[91m‼\x1b[9mku\x1b[29m‼\x1b[0m"
        );
        assert_eq!(
            style_at_depth(2, String::new(), options, LeafRole::Error),
            "\x1b[91m‼‼\x1b[0m"
        );

        let grouped = node(vec![
            SExpr::Leaf {
                text: String::from("foo"),
                range: None,
                role: LeafRole::Elided,
            },
            leaf(String::from("bar")),
        ]);
        assert_eq!(
            render_bracketed_at_depth(0, &grouped, options),
            "\x1b[35m(\x1b[94m\x1b[3mfoo\x1b[0m \x1b[94mbar\x1b[35m)\x1b[0m"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colored_whitespace_lines_breaks_and_separators_stay_unstyled() {
        let options = BracketRenderOptions {
            color: true,
            ..BracketRenderOptions::default()
        };
        let cases = [
            (
                "hello\n   \nworld",
                "\x1b[35mhello\x1b[0m\n   \n\x1b[35mworld\x1b[0m",
            ),
            (
                "hello\n\t\nworld",
                "\x1b[35mhello\x1b[0m\n\\u{9}\n\x1b[35mworld\x1b[0m",
            ),
            ("hello\rworld", "\x1b[35mhello\x1b[0m\r\x1b[35mworld\x1b[0m"),
            (
                "hello\n\r\r\nworld",
                "\x1b[35mhello\x1b[0m\n\r\r\n\x1b[35mworld\x1b[0m",
            ),
        ];
        for (text, expected) in cases {
            let expr = SExpr::Leaf {
                text: text.to_owned(),
                range: None,
                role: LeafRole::Normal,
            };
            assert_eq!(
                super::render_colored(&expr, options, false),
                expected,
                "{text:?}"
            );
            assert_eq!(
                super::render_colored(&expr, options, true),
                expected,
                "{text:?}"
            );
            super::assert_blank_lines_have_no_escapes(&expr, options);
        }

        let broken = node(vec![leaf("a\n".to_owned()), leaf("\nb".to_owned())]);
        assert_eq!(
            super::render_colored(&broken, options, false),
            "\x1b[35m(\x1b[94ma\x1b[0m\n\n\x1b[94mb\x1b[35m)\x1b[0m"
        );
        assert_eq!(
            super::render_colored(&broken, options, true),
            "\x1b[94ma\x1b[0m\n\n\x1b[94mb\x1b[0m"
        );
        super::assert_blank_lines_have_no_escapes(&broken, options);

        let error = SExpr::Leaf {
            text: "ku\n".to_owned(),
            range: None,
            role: LeafRole::Error,
        };
        let error_expected = "\x1b[91m‼\x1b[9mku\x1b[0m\n\x1b[91m‼\x1b[0m";
        assert_eq!(
            super::render_colored(&error, options, false),
            error_expected
        );
        assert_eq!(super::render_colored(&error, options, true), error_expected);
        assert!(!error_expected.contains("\x1b[9m\x1b[29m"));

        let spaced_error = SExpr::Leaf {
            text: "ku\n   \nku".to_owned(),
            range: None,
            role: LeafRole::Error,
        };
        let spaced_expected = "\x1b[91m‼\x1b[9mku\x1b[0m\n   \n\x1b[91m\x1b[9mku\x1b[29m‼\x1b[0m";
        assert_eq!(
            super::render_colored(&spaced_error, options, false),
            spaced_expected
        );
        assert_eq!(
            super::render_colored(&spaced_error, options, true),
            spaced_expected
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colored_leaf_text_matches_the_styled_document_for_every_style() {
        let atoms = ["a", " ", "\t", "\n", "\r\n", "\r", "  ", "\t "];
        let mut texts = Vec::new();
        for first in atoms {
            texts.push(first.to_owned());
            for second in atoms {
                texts.push(format!("{first}{second}"));
                for third in atoms {
                    texts.push(format!("{first}{second}{third}"));
                }
            }
        }
        let options = BracketRenderOptions {
            color: true,
            ..BracketRenderOptions::default()
        };
        let roles = [LeafRole::Normal, LeafRole::Elided, LeafRole::Error];
        for text in &texts {
            for role in roles {
                for depth in 0..6 {
                    let expr = SExpr::Leaf {
                        text: text.clone(),
                        range: None,
                        role,
                    };
                    super::assert_document_round_trip(&expr, depth, options);
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_byte_limit_matches_the_serialized_length() {
        let expr = SExpr::Leaf {
            text: "abc".to_owned(),
            range: None,
            role: LeafRole::Normal,
        };
        let cancel = jbotci_interrupt::CancellationToken::never();
        for color in [false, true] {
            for plain in [false, true] {
                let options = BracketRenderOptions {
                    color,
                    ..BracketRenderOptions::default()
                };
                let full = if plain {
                    super::plain_document(&expr, options).serialize(color)
                } else {
                    super::bracket_document(&expr, options).serialize(color)
                };
                assert!(full.len() > 1, "{full:?}");
                let fit = ByteLimit::new(full.len()).expect("positive limit");
                let short = ByteLimit::new(full.len() - 1).expect("positive limit");
                let rendered = if plain {
                    super::render_plain_with_limit(&expr, options, fit, &cancel)
                } else {
                    super::render_bracketed_with_limit(&expr, options, fit, &cancel)
                };
                assert_eq!(rendered.expect("exact limit"), full);
                let rejected = if plain {
                    super::render_plain_with_limit(&expr, options, short, &cancel)
                } else {
                    super::render_bracketed_with_limit(&expr, options, short, &cancel)
                };
                assert!(
                    matches!(rejected, Err(crate::OutputError::Limit { .. })),
                    "{rejected:?}"
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_megabyte_leaf_stops_at_32_bytes() {
        let expr = SExpr::Leaf {
            text: "a".repeat(1_000_000),
            range: None,
            role: LeafRole::Normal,
        };
        let cancel = jbotci_interrupt::CancellationToken::never();
        let limit = ByteLimit::new(32).expect("positive limit");
        for color in [false, true] {
            let options = BracketRenderOptions {
                color,
                ..BracketRenderOptions::default()
            };
            assert!(matches!(
                super::render_bracketed_with_limit(&expr, options, limit, &cancel),
                Err(crate::OutputError::Limit { .. })
            ));
            assert!(matches!(
                super::render_plain_with_limit(&expr, options, limit, &cancel),
                Err(crate::OutputError::Limit { .. })
            ));
        }
    }
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
fn render_colored(expr: &SExpr, options: BracketRenderOptions, plain: bool) -> String {
    let limit = ByteLimit::new(usize::MAX).expect("positive limit");
    let cancellation = jbotci_interrupt::CancellationToken::never();
    if plain {
        render_plain_with_limit(expr, options, limit, &cancellation).expect("colored plain")
    } else {
        render_bracketed_with_limit(expr, options, limit, &cancellation).expect("colored brackets")
    }
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
fn assert_blank_lines_have_no_escapes(expr: &SExpr, options: BracketRenderOptions) {
    for plain in [false, true] {
        let rendered = render_colored(expr, options, plain);
        let document = if plain {
            plain_document(expr, options)
        } else {
            bracket_document(expr, options)
        };
        assert_eq!(rendered, document.serialize(true));
        let mut rest = rendered.as_str();
        for line in &document.lines {
            let piece = if let Some(ending) = line.ending {
                let marker = match ending {
                    styled::LineBreak::Lf => "\n",
                    styled::LineBreak::CrLf => "\r\n",
                    styled::LineBreak::Cr => "\r",
                };
                let index = rest.find(marker).expect("line break");
                let piece = &rest[..index];
                rest = &rest[index + marker.len()..];
                piece
            } else {
                let piece = rest;
                rest = "";
                piece
            };
            let unstyled = line
                .segments
                .iter()
                .all(|segment| !segment.style.is_styled());
            if unstyled {
                assert!(!piece.contains('\u{1b}'), "{piece:?} in {rendered:?}");
            }
            if line
                .segments
                .last()
                .is_some_and(|segment| segment.style.is_styled())
            {
                assert!(piece.ends_with("\x1b[0m"), "{piece:?}");
            }
        }
        assert!(rest.is_empty(), "{rest:?}");
    }
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
fn assert_document_round_trip(expr: &SExpr, depth: usize, options: BracketRenderOptions) {
    let bracketed = render_bracketed_at_depth(depth, expr, options);
    let document = bracket_document_at_depth(depth, expr, options);
    assert_eq!(bracketed, document.serialize(true));
    let plain_options = options;
    let plain = render_colored(expr, plain_options, true);
    let plain_document = plain_document(expr, plain_options);
    assert_eq!(plain, plain_document.serialize(true));
    assert_eq!(
        document.serialize(false).contains('\u{1b}'),
        false,
        "{expr:?}"
    );
    for line in document.lines.iter().chain(plain_document.lines.iter()) {
        let blank = line.segments.is_empty()
            || line
                .segments
                .iter()
                .all(|segment| segment.text.chars().all(char::is_whitespace));
        if blank {
            assert!(
                line.segments
                    .iter()
                    .all(|segment| !segment.style.is_styled()),
                "{line:?}"
            );
        }
    }
}
