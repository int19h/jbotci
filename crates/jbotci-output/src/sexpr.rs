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
    match depth % 5 {
        0 => "\x1b[35m",
        1 => "\x1b[34m",
        2 => "\x1b[32m",
        3 => "\x1b[31m",
        _ => "\x1b[33m",
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
