use bityzba::{invariant, requires};

use crate::BracketRenderOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Leaf(..) => true)]
#[invariant(::Node(..) => true)]
pub(crate) enum SExpr {
    Leaf(String),
    Node(Vec<SExpr>),
}

#[requires(true)]
#[ensures(matches!(&ret, SExpr::Node(children) if children.is_empty()))]
pub(crate) fn empty_node() -> SExpr {
    SExpr::Node(Vec::new())
}

#[requires(true)]
#[ensures(matches!(&ret, SExpr::Node(children) if children.iter().all(|child| !is_empty(child))))]
pub(crate) fn node(children: Vec<SExpr>) -> SExpr {
    let mut node_children = Vec::new();
    for child in children {
        match child {
            other if !is_empty(&other) => node_children.push(other),
            _ => {}
        }
    }
    SExpr::Node(node_children)
}

#[requires(true)]
#[ensures(matches!(&ret, SExpr::Leaf(_)) || is_empty(&ret))]
pub(crate) fn leaf(text: String) -> SExpr {
    if text.is_empty() {
        empty_node()
    } else {
        SExpr::Leaf(text)
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn is_empty(expr: &SExpr) -> bool {
    match expr {
        SExpr::Leaf(text) => text.is_empty(),
        SExpr::Node(children) => children.is_empty(),
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn flatten(expr: SExpr) -> SExpr {
    match expr {
        SExpr::Leaf(text) => SExpr::Leaf(text),
        SExpr::Node(children) => {
            let mut flattened = children
                .into_iter()
                .map(flatten)
                .filter(|child| !is_empty(child))
                .collect::<Vec<_>>();
            if flattened.len() == 1 {
                flattened.remove(0)
            } else {
                SExpr::Node(flattened)
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
fn render_bracketed_at_depth(depth: usize, expr: &SExpr, options: BracketRenderOptions) -> String {
    match expr {
        SExpr::Leaf(text) => colorize_at_depth(depth, text.clone(), options),
        SExpr::Node(children) => {
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
                    colorize_at_depth(
                        depth,
                        format!("{open}{}{close}", rendered.join(" ")),
                        options,
                    )
                }
            }
        }
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
        let expr = SExpr::Node(vec![SExpr::Node(vec![SExpr::Node(vec![SExpr::Leaf(
            String::from("foo"),
        )])])]);

        let flattened = flatten(expr);

        assert_eq!(flattened, SExpr::Leaf(String::from("foo")));
        assert_eq!(render_bracketed(&flattened), "foo");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn render_collapses_single_non_empty_child_after_filtering_empty_children() {
        let expr = SExpr::Node(vec![
            SExpr::Node(Vec::new()),
            SExpr::Leaf(String::from("foo")),
            SExpr::Node(Vec::new()),
        ]);

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
