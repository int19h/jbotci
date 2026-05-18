use bityzba::{invariant, requires};

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
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
    SExpr::Node(
        children
            .into_iter()
            .filter(|child| !is_empty(child))
            .collect(),
    )
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
    render_bracketed_at_depth(0, expr)
}

#[requires(true)]
#[ensures(true)]
fn render_bracketed_at_depth(depth: usize, expr: &SExpr) -> String {
    match expr {
        SExpr::Leaf(text) => text.clone(),
        SExpr::Node(children) => {
            let rendered = children
                .iter()
                .map(|child| render_bracketed_at_depth(depth + 1, child))
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>();
            match rendered.as_slice() {
                [] => String::new(),
                [single] => single.clone(),
                _ => {
                    let (open, close) = bracket_pair(depth);
                    format!("{open}{}{close}", rendered.join(" "))
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
