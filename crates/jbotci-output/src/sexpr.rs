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

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_bracketed(expr: &SExpr) -> String {
    render_bracketed_with_options(expr, BracketRenderOptions::default())
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_bracketed_with_options(expr: &SExpr, options: BracketRenderOptions) -> String {
    render_bracketed_at_depth(0, expr, options)
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
fn render_bracketed_at_depth(depth: usize, expr: &SExpr, options: BracketRenderOptions) -> String {
    match expr {
        SExpr::Leaf { text, role, .. } => style_at_depth(depth, text.clone(), options, *role),
        SExpr::Node { children, .. } => {
            let rendered = children
                .iter()
                .map(|child| render_bracketed_at_depth(depth + 1, child, options))
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>();
            match rendered.as_slice() {
                [] => String::new(),
                [single] => single.clone(),
                _ => {
                    let (open, close) = bracket_pair(depth);
                    let hair_space = if options.insert_hair_space {
                        "\u{200a}"
                    } else {
                        ""
                    };
                    style_at_depth(
                        depth,
                        format!(
                            "{open}{hair_space}{}{hair_space}{close}",
                            rendered.join(" ")
                        ),
                        options,
                        LeafRole::Normal,
                    )
                }
            }
        }
    }
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
    if role == LeafRole::Error {
        if !options.color {
            return format!("‼{text}‼");
        }
        return format!(
            "\x1b[91m‼\x1b[9m{text}\x1b[29m‼{}",
            ansi_parent_color_for_depth(depth)
        );
    }
    if options.color && !text.is_empty() {
        if role == LeafRole::Elided {
            format!(
                "{}\x1b[3m{}\x1b[23m{}",
                ansi_color_for_depth(depth),
                text,
                ansi_parent_color_for_depth(depth)
            )
        } else {
            format!(
                "{}{}{}",
                ansi_color_for_depth(depth),
                text,
                ansi_parent_color_for_depth(depth)
            )
        }
    } else {
        text
    }
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

#[requires(true)]
#[ensures(!ret.is_empty())]
fn ansi_parent_color_for_depth(depth: usize) -> &'static str {
    if depth == 0 {
        "\x1b[0m"
    } else {
        ansi_color_for_depth(depth - 1)
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
        assert_eq!(render_bracketed(&flattened), "foo");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn render_collapses_single_non_empty_child_after_filtering_empty_children() {
        let expr = node(vec![empty_node(), leaf(String::from("foo")), empty_node()]);

        assert_eq!(render_bracketed(&expr), "foo");
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
