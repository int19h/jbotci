use bityzba::{invariant, requires};

use crate::{BracketRenderOptions, BracketSourceFragment, BracketSourceRange};

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Leaf { .. } => true)]
#[invariant(::Node { .. } => true)]
pub(crate) enum SExpr {
    Leaf {
        text: String,
        range: Option<BracketSourceRange>,
    },
    Node {
        children: Vec<SExpr>,
        range: Option<BracketSourceRange>,
    },
}

#[requires(true)]
#[ensures(matches!(&ret, SExpr::Node { children, .. } if children.is_empty()))]
pub(crate) fn empty_node() -> SExpr {
    SExpr::Node {
        children: Vec::new(),
        range: None,
    }
}

#[requires(true)]
#[ensures(matches!(&ret, SExpr::Node { children, .. } if children.iter().all(|child| !is_empty(child))))]
pub(crate) fn node(children: Vec<SExpr>) -> SExpr {
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
    if text.is_empty() {
        empty_node()
    } else {
        SExpr::Leaf { text, range }
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_empty(expr: &SExpr) -> bool {
    match expr {
        SExpr::Leaf { text, .. } => text.is_empty(),
        SExpr::Node { children, .. } => children.is_empty(),
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn flatten(expr: SExpr) -> SExpr {
    match expr {
        SExpr::Leaf { text, range } => SExpr::Leaf { text, range },
        SExpr::Node { children, range } => {
            let mut flattened = children
                .into_iter()
                .map(flatten)
                .filter(|child| !is_empty(child))
                .collect::<Vec<_>>();
            if flattened.len() == 1 {
                flattened.remove(0)
            } else {
                SExpr::Node {
                    children: flattened,
                    range,
                }
            }
        }
    }
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
        SExpr::Leaf { text, .. } => colorize_at_depth(depth, text.clone(), options),
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
                    colorize_at_depth(
                        depth,
                        format!(
                            "{open}{hair_space}{}{hair_space}{close}",
                            rendered.join(" ")
                        ),
                        options,
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
        SExpr::Leaf { text, range } => vec![BracketSourceFragment::Text {
            text: text.clone(),
            range: *range,
        }],
        SExpr::Node { children, range } => {
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
                    });
                    for (index, fragment) in rendered.into_iter().enumerate() {
                        if index > 0 {
                            children.push(BracketSourceFragment::Text {
                                text: " ".to_owned(),
                                range: None,
                            });
                        }
                        children.push(fragment);
                    }
                    children.push(BracketSourceFragment::Text {
                        text: format!("{hair_space}{close}"),
                        range: *range,
                    });
                    vec![BracketSourceFragment::Span {
                        range: *range,
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
    let mut range = ranges.next()?;
    for child_range in ranges {
        range.byte_start = range.byte_start.min(child_range.byte_start);
        range.byte_end = range.byte_end.max(child_range.byte_end);
    }
    Some(range)
}

#[requires(true)]
#[ensures(ret.is_none_or(|range| range.byte_start <= range.byte_end))]
fn expr_range(expr: &SExpr) -> Option<BracketSourceRange> {
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
#[ensures(!options.color -> ret == old(text.clone()))]
#[ensures(options.color && !old(text.is_empty()) -> ret.starts_with(ansi_color_for_depth(depth)))]
fn colorize_at_depth(depth: usize, text: String, options: BracketRenderOptions) -> String {
    if options.color && !text.is_empty() {
        format!(
            "{}{}{}",
            ansi_color_for_depth(depth),
            text,
            ansi_parent_color_for_depth(depth)
        )
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
            colorize_at_depth(0, String::from("foo"), options),
            "\x1b[35mfoo\x1b[0m"
        );
        assert_eq!(
            colorize_at_depth(2, String::from("foo"), options),
            "\x1b[32mfoo\x1b[94m"
        );
        assert_eq!(
            colorize_at_depth(6, String::from("foo"), options),
            "\x1b[35mfoo\x1b[96m"
        );
    }
}
