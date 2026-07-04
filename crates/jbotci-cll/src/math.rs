use bityzba::{invariant, new, requires};
use roxmltree::Node;

use super::{escape_html, normalized_plain_text, raw_text};

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CllMathDisplay {
    Inline,
    Block,
}

#[invariant(markup.starts_with("<math") && markup.ends_with("</math>"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CllMathRender {
    pub(crate) text: String,
    pub(crate) latex: String,
    pub(crate) markup: String,
}

#[requires(node.is_element())]
#[ensures(!ret.markup.is_empty())]
pub(crate) fn render_math_node(node: Node<'_, '_>, display: CllMathDisplay) -> CllMathRender {
    let text = normalized_plain_text(&raw_text(node));
    let latex = render_math_latex_node(node);
    let tag_name = node.tag_name().name();
    let markup = if tag_name == "math" {
        render_math_element(node)
    } else {
        let display_attr = match display {
            CllMathDisplay::Inline => "",
            CllMathDisplay::Block => " display=\"block\"",
        };
        format!("<math{display_attr}>{}</math>", render_math_body(node))
    };
    new!(CllMathRender {
        text,
        latex,
        markup,
    })
}

#[requires(node.is_element())]
#[ensures(!ret.is_empty())]
fn render_math_body(node: Node<'_, '_>) -> String {
    let rendered = render_math_nodes(node);
    if rendered.is_empty() {
        format!(
            "<mtext>{}</mtext>",
            escape_html(&normalized_plain_text(&raw_text(node)))
        )
    } else {
        rendered
    }
}

#[requires(parent.is_element())]
#[ensures(true)]
fn render_math_nodes(parent: Node<'_, '_>) -> String {
    let mut parts = Vec::new();
    for child in parent.children() {
        if child.is_text() {
            let text = child.text().unwrap_or_default().trim();
            if !text.is_empty() {
                parts.push(format!("<mtext>{}</mtext>", escape_html(text)));
            }
        } else if child.is_element() {
            let child_tag = child.tag_name().name();
            match child_tag {
                "superscript" => attach_math_script(&mut parts, "msup", render_math_script(child)),
                "subscript" => attach_math_script(&mut parts, "msub", render_math_script(child)),
                "indexterm" => {}
                _ if is_math_ml_tag_name(child_tag) => parts.push(render_math_element(child)),
                _ => {
                    let rendered = render_math_nodes(child);
                    if !rendered.is_empty() {
                        parts.push(rendered);
                    }
                }
            }
        }
    }
    parts.concat()
}

#[requires(node.is_element())]
#[ensures(!ret.is_empty())]
fn render_math_script(node: Node<'_, '_>) -> String {
    let rendered = render_math_nodes(node);
    if rendered.is_empty() {
        "<mtext></mtext>".to_owned()
    } else {
        rendered
    }
}

#[requires(!tag_name.is_empty())]
#[ensures(true)]
fn attach_math_script(parts: &mut Vec<String>, tag_name: &str, script: String) {
    let base = parts.pop().unwrap_or_else(|| "<mtext></mtext>".to_owned());
    parts.push(format!(
        "<{tag_name}><mrow>{base}</mrow><mrow>{script}</mrow></{tag_name}>"
    ));
}

#[requires(node.is_element())]
#[ensures(!ret.is_empty())]
fn render_math_element(node: Node<'_, '_>) -> String {
    let tag_name = node.tag_name().name();
    let attrs = node
        .attributes()
        .map(|attribute| {
            format!(
                " {}=\"{}\"",
                escape_html(attribute.name()),
                escape_html(attribute.value())
            )
        })
        .collect::<String>();
    format!(
        "<{tag_name}{attrs}>{}</{tag_name}>",
        render_math_nodes(node)
    )
}

#[requires(!tag_name.is_empty())]
#[ensures(true)]
fn is_math_ml_tag_name(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "math"
            | "mrow"
            | "mfrac"
            | "msqrt"
            | "mroot"
            | "msub"
            | "msup"
            | "msubsup"
            | "munder"
            | "mover"
            | "munderover"
            | "mi"
            | "mn"
            | "mo"
            | "mtext"
            | "mtable"
            | "mtr"
            | "mtd"
            | "mlabeledtr"
            | "mstyle"
            | "mspace"
            | "mfenced"
            | "menclose"
            | "semantics"
            | "annotation"
            | "annotation-xml"
    )
}

#[requires(node.is_element())]
#[ensures(true)]
fn render_math_latex_node(node: Node<'_, '_>) -> String {
    match node.tag_name().name() {
        "superscript" => format!("^{{{}}}", render_math_latex_children(node)),
        "subscript" => format!("_{{{}}}", render_math_latex_children(node)),
        "math" | "mrow" | "mstyle" | "semantics" | "annotation" | "annotation-xml" => {
            render_math_latex_children(node)
        }
        "mfrac" => {
            let children = math_latex_child_elements(node);
            let numerator = children
                .first()
                .map(|child| render_math_latex_node(*child))
                .unwrap_or_default();
            let denominator = children
                .get(1)
                .map(|child| render_math_latex_node(*child))
                .unwrap_or_default();
            format!("\\frac{{{numerator}}}{{{denominator}}}")
        }
        "msqrt" => format!("\\sqrt{{{}}}", render_math_latex_children(node)),
        "mroot" => {
            let children = math_latex_child_elements(node);
            let body = children
                .first()
                .map(|child| render_math_latex_node(*child))
                .unwrap_or_default();
            let root = children
                .get(1)
                .map(|child| render_math_latex_node(*child))
                .unwrap_or_default();
            format!("\\sqrt[{root}]{{{body}}}")
        }
        "msub" | "msup" | "msubsup" | "munder" | "mover" | "munderover" => {
            render_math_latex_scripted(node)
        }
        "mfenced" => format!("({})", render_math_latex_children(node)),
        "mtable" => format!(
            "\\begin{{matrix}}{}\\end{{matrix}}",
            render_math_latex_table(node)
        ),
        "mtr" | "mlabeledtr" => render_math_latex_row(node),
        "mtd" => render_math_latex_children(node),
        "mi" | "mn" | "mo" | "mtext" => math_latex_text(&raw_text(node)),
        "mspace" => " ".to_owned(),
        "indexterm" => String::new(),
        _ => render_math_latex_children(node),
    }
}

#[requires(node.is_element())]
#[ensures(true)]
fn render_math_latex_children(node: Node<'_, '_>) -> String {
    let mut output = String::new();
    for child in node.children() {
        if child.is_text() {
            output.push_str(&math_latex_text(child.text().unwrap_or_default()));
        } else if child.is_element() {
            output.push_str(&render_math_latex_node(child));
        }
    }
    normalized_plain_text(&output)
}

#[requires(node.is_element())]
#[ensures(true)]
fn render_math_latex_scripted(node: Node<'_, '_>) -> String {
    let children = math_latex_child_elements(node);
    let base = children
        .first()
        .map(|child| render_math_latex_node(*child))
        .unwrap_or_default();
    let first_script = children
        .get(1)
        .map(|child| render_math_latex_node(*child))
        .unwrap_or_default();
    let second_script = children
        .get(2)
        .map(|child| render_math_latex_node(*child))
        .unwrap_or_default();
    match node.tag_name().name() {
        "msub" | "munder" => format!("{base}_{{{first_script}}}"),
        "msup" | "mover" => format!("{base}^{{{first_script}}}"),
        "msubsup" | "munderover" => format!("{base}_{{{first_script}}}^{{{second_script}}}"),
        _ => base,
    }
}

#[requires(node.is_element())]
#[ensures(true)]
fn render_math_latex_table(node: Node<'_, '_>) -> String {
    node.children()
        .filter(|child| child.is_element() && !child.has_tag_name("indexterm"))
        .map(render_math_latex_node)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" \\\\ ")
}

#[requires(node.is_element())]
#[ensures(true)]
fn render_math_latex_row(node: Node<'_, '_>) -> String {
    node.children()
        .filter(|child| child.is_element() && !child.has_tag_name("indexterm"))
        .map(render_math_latex_node)
        .collect::<Vec<_>>()
        .join(" & ")
}

#[requires(node.is_element())]
#[ensures(true)]
fn math_latex_child_elements<'a, 'input>(node: Node<'a, 'input>) -> Vec<Node<'a, 'input>> {
    node.children()
        .filter(|child| child.is_element() && !child.has_tag_name("indexterm"))
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn math_latex_text(text: &str) -> String {
    let normalized = normalized_plain_text(text);
    let mut output = String::new();
    for character in normalized.chars() {
        match character {
            '\u{2062}' => {}
            '×' => output.push_str("\\times"),
            '∞' => output.push_str("\\infty"),
            '≠' => output.push_str("\\ne"),
            '≤' => output.push_str("\\le"),
            '≥' => output.push_str("\\ge"),
            '%' => output.push_str("\\%"),
            '{' => output.push_str("\\{"),
            '}' => output.push_str("\\}"),
            _ => output.push(character),
        }
    }
    output
}
