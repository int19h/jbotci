use std::sync::OnceLock;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use math_core::{LatexToMathML, MathCoreConfig, MathDisplay};
use roxmltree::{Document, Node};

/// Strict level-1 Unicode rendering built on math_core's LaTeX parser.
///
/// math_core deliberately exposes MathML rather than its internal syntax tree. Translating its
/// generated MathML keeps TeX parsing in one place while making the plain-text faithfulness
/// boundary explicit: any node this renderer cannot represent without losing structure rejects
/// the entire island.
static CONVERTER: OnceLock<LatexToMathML> = OnceLock::new();

/// Render one delimiter-free LaTeX island, rejecting any unfaithful representation.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|rendered| !rendered.is_empty()))]
pub(crate) fn render(source: &str) -> Option<String> {
    if source.is_empty() {
        return None;
    }
    let converter = CONVERTER.get_or_init(|| {
        LatexToMathML::new(MathCoreConfig::default())
            .expect("math_core's macro-free default configuration must be valid")
    });
    let mathml = converter
        .convert_with_local_counter(source, MathDisplay::Inline)
        .ok()?;
    render_supported_mathml(&mathml)
}

#[requires(!mathml.is_empty())]
#[ensures(ret.as_ref().is_none_or(|rendered| !rendered.is_empty()))]
fn render_supported_mathml(mathml: &str) -> Option<String> {
    let document = Document::parse(mathml).ok()?;
    let root = document.root_element();
    if root.tag_name().name() != "math" {
        return None;
    }
    let mut output = String::new();
    if !render_mathml_element(root, &mut output) || output.is_empty() {
        return None;
    }
    Some(output)
}

#[requires(node.is_element())]
#[ensures(output.len() >= old(output.len()))]
fn render_mathml_element(node: Node<'_, '_>, output: &mut String) -> bool {
    match node.tag_name().name() {
        "math" | "mrow" => render_mathml_children(node, output),
        "mn" => render_number(node, output),
        "mi" => render_identifier(node, output),
        "mo" => render_operator(node, output),
        "msub" => render_scripted(node, ScriptPosition::Subscript, output),
        "msup" => render_scripted(node, ScriptPosition::Superscript, output),
        "msubsup" => render_subscript_and_superscript(node, output),
        "mfrac" => render_simple_fraction(node, output),
        _ => false,
    }
}

#[requires(parent.is_element())]
#[ensures(output.len() >= old(output.len()))]
fn render_mathml_children(parent: Node<'_, '_>, output: &mut String) -> bool {
    for child in parent.children() {
        if child.is_element() {
            if !render_mathml_element(child, output) {
                return false;
            }
        } else if child.is_text() {
            if !child.text().unwrap_or_default().trim().is_empty() {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

#[requires(node.is_element() && node.tag_name().name() == "mn")]
#[ensures(output.len() >= old(output.len()))]
fn render_number(node: Node<'_, '_>, output: &mut String) -> bool {
    let Some(text) = leaf_text(node) else {
        return false;
    };
    if text.is_empty()
        || !text
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
    {
        return false;
    }
    output.push_str(text);
    true
}

#[requires(node.is_element() && node.tag_name().name() == "mi")]
#[ensures(output.len() >= old(output.len()))]
fn render_identifier(node: Node<'_, '_>, output: &mut String) -> bool {
    if node.attribute("mathvariant").is_some() {
        return false;
    }
    let Some(text) = leaf_text(node) else {
        return false;
    };
    let mut characters = text.chars();
    let Some(character) = characters.next() else {
        return false;
    };
    if characters.next().is_some() {
        // Multi-letter <mi> is upright by MathML convention, so styling every letter as a
        // variable would change its meaning.
        return false;
    }
    if character == '\u{fffd}' {
        return false;
    }
    output.push(mathematical_italic_character(character));
    true
}

#[requires(node.is_element() && node.tag_name().name() == "mo")]
#[ensures(output.len() >= old(output.len()))]
fn render_operator(node: Node<'_, '_>, output: &mut String) -> bool {
    let Some(text) = leaf_text(node) else {
        return false;
    };
    let rendered = match text {
        "*" | "\u{2217}" | "×" => "×",
        "\u{22c5}" | "·" => "·",
        "-" | "\u{2212}" => "\u{2212}",
        "+" => "+",
        "=" => "=",
        "(" => "(",
        ")" => ")",
        "," => ",",
        "." => ".",
        "/" => "/",
        ":" => ":",
        _ => return false,
    };
    output.push_str(rendered);
    true
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptPosition {
    Subscript,
    Superscript,
}

#[requires(node.is_element())]
#[ensures(output.len() >= old(output.len()))]
fn render_scripted(node: Node<'_, '_>, position: ScriptPosition, output: &mut String) -> bool {
    let mut children = node.children().filter(Node::is_element);
    let Some(base) = children.next() else {
        return false;
    };
    let Some(script) = children.next() else {
        return false;
    };
    if children.next().is_some() || !has_only_elements_and_whitespace(node) {
        return false;
    }
    if !render_mathml_element(base, output) {
        return false;
    }
    render_script(script, position, output)
}

#[requires(node.is_element() && node.tag_name().name() == "msubsup")]
#[ensures(output.len() >= old(output.len()))]
fn render_subscript_and_superscript(node: Node<'_, '_>, output: &mut String) -> bool {
    let mut children = node.children().filter(Node::is_element);
    let Some(base) = children.next() else {
        return false;
    };
    let Some(subscript) = children.next() else {
        return false;
    };
    let Some(superscript) = children.next() else {
        return false;
    };
    if children.next().is_some() || !has_only_elements_and_whitespace(node) {
        return false;
    }
    render_mathml_element(base, output)
        && render_script(subscript, ScriptPosition::Subscript, output)
        && render_script(superscript, ScriptPosition::Superscript, output)
}

#[requires(node.is_element())]
#[ensures(output.len() >= old(output.len()))]
fn render_script(node: Node<'_, '_>, position: ScriptPosition, output: &mut String) -> bool {
    match node.tag_name().name() {
        "mrow" => {
            for child in node.children() {
                if child.is_element() {
                    if !render_script(child, position, output) {
                        return false;
                    }
                } else if child.is_text() {
                    if !child.text().unwrap_or_default().trim().is_empty() {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        }
        "mn" | "mo" => {
            let Some(text) = leaf_text(node) else {
                return false;
            };
            if text.is_empty() {
                return false;
            }
            for character in text.chars() {
                let Some(rendered) = script_character(character, position) else {
                    return false;
                };
                output.push(rendered);
            }
            true
        }
        _ => false,
    }
}

#[requires(node.is_element() && node.tag_name().name() == "mfrac")]
#[ensures(output.len() >= old(output.len()))]
fn render_simple_fraction(node: Node<'_, '_>, output: &mut String) -> bool {
    if node.attributes().len() != 0 {
        return false;
    }
    let mut children = node.children().filter(Node::is_element);
    let Some(numerator) = children.next() else {
        return false;
    };
    let Some(denominator) = children.next() else {
        return false;
    };
    if children.next().is_some() || !has_only_elements_and_whitespace(node) {
        return false;
    }
    if !render_simple_fraction_component(numerator, output) {
        return false;
    }
    output.push('\u{2044}');
    render_simple_fraction_component(denominator, output)
}

#[requires(node.is_element())]
#[ensures(output.len() >= old(output.len()))]
fn render_simple_fraction_component(node: Node<'_, '_>, output: &mut String) -> bool {
    match node.tag_name().name() {
        "mn" => {
            let Some(text) = leaf_text(node) else {
                return false;
            };
            if text.is_empty() || !text.chars().all(|character| character.is_ascii_digit()) {
                return false;
            }
            output.push_str(text);
            true
        }
        "mi" => render_identifier(node, output),
        _ => false,
    }
}

#[requires(node.is_element())]
#[ensures(ret.is_none_or(|text| !text.is_empty()))]
fn leaf_text<'a>(node: Node<'a, '_>) -> Option<&'a str> {
    let mut children = node.children();
    let child = children.next()?;
    if !child.is_text() || children.next().is_some() {
        return None;
    }
    child.text().filter(|text| !text.is_empty())
}

#[requires(node.is_element())]
#[ensures(true)]
fn has_only_elements_and_whitespace(node: Node<'_, '_>) -> bool {
    node.children().all(|child| {
        child.is_element()
            || (child.is_text() && child.text().unwrap_or_default().trim().is_empty())
    })
}

#[requires(character != '\u{fffd}')]
#[ensures(ret != '\u{fffd}')]
fn mathematical_italic_character(character: char) -> char {
    let codepoint = match character {
        'A'..='Z' => 0x1d434 + u32::from(character) - u32::from('A'),
        'a'..='g' => 0x1d44e + u32::from(character) - u32::from('a'),
        // U+1D455 is the sole gap in the Latin mathematical-italic ranges. Unicode assigns
        // the legacy letterlike symbol U+210E as the mathematical italic small h.
        'h' => return '\u{210e}',
        'i'..='z' => 0x1d44e + u32::from(character) - u32::from('a'),
        _ => return character,
    };
    char::from_u32(codepoint).expect("mathematical italic Latin ranges contain valid scalars")
}

#[requires(true)]
#[ensures(true)]
fn script_character(character: char, position: ScriptPosition) -> Option<char> {
    match position {
        ScriptPosition::Superscript => match character {
            '0' => Some('\u{2070}'),
            '1' => Some('\u{00b9}'),
            '2' => Some('\u{00b2}'),
            '3' => Some('\u{00b3}'),
            '4' => Some('\u{2074}'),
            '5' => Some('\u{2075}'),
            '6' => Some('\u{2076}'),
            '7' => Some('\u{2077}'),
            '8' => Some('\u{2078}'),
            '9' => Some('\u{2079}'),
            '+' => Some('\u{207a}'),
            '-' | '\u{2212}' => Some('\u{207b}'),
            '=' => Some('\u{207c}'),
            '(' => Some('\u{207d}'),
            ')' => Some('\u{207e}'),
            _ => None,
        },
        ScriptPosition::Subscript => match character {
            '0' => Some('\u{2080}'),
            '1' => Some('\u{2081}'),
            '2' => Some('\u{2082}'),
            '3' => Some('\u{2083}'),
            '4' => Some('\u{2084}'),
            '5' => Some('\u{2085}'),
            '6' => Some('\u{2086}'),
            '7' => Some('\u{2087}'),
            '8' => Some('\u{2088}'),
            '9' => Some('\u{2089}'),
            '+' => Some('\u{208a}'),
            '-' | '\u{2212}' => Some('\u{208b}'),
            '=' => Some('\u{208c}'),
            '(' => Some('\u{208d}'),
            ')' => Some('\u{208e}'),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn render(source: &str) -> Option<String> {
        super::render(source)
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_level_one_unicode_math() {
        assert_eq!(
            render(r"6.02214129(27) * 10^{23}"),
            Some("6.02214129(27)×10²³".to_owned())
        );
        assert_eq!(render(r"a \times b"), Some("𝑎×𝑏".to_owned()));
        assert_eq!(render(r"a \cdot b"), Some("𝑎·𝑏".to_owned()));
        assert_eq!(render("a-b"), Some("𝑎−𝑏".to_owned()));
        assert_eq!(render(r"x_{-12}^{+3}"), Some("𝑥₋₁₂⁺³".to_owned()));
        assert_eq!(render(r"^{-1}"), Some("⁻¹".to_owned()));
        assert_eq!(render(r"\frac{a}{b}"), Some("𝑎⁄𝑏".to_owned()));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn audits_the_complete_latin_mathematical_italic_range() {
        let ascii = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        let rendered = ascii
            .chars()
            .map(mathematical_italic_character)
            .collect::<String>();
        assert_eq!(
            rendered,
            concat!("𝐴𝐵𝐶𝐷𝐸𝐹𝐺𝐻𝐼𝐽𝐾𝐿𝑀𝑁𝑂𝑃𝑄𝑅𝑆𝑇𝑈𝑉𝑊𝑋𝑌𝑍", "𝑎𝑏𝑐𝑑𝑒𝑓𝑔ℎ𝑖𝑗𝑘𝑙𝑚𝑛𝑜𝑝𝑞𝑟𝑠𝑡𝑢𝑣𝑤𝑥𝑦𝑧",)
        );
        assert!(!rendered.contains('\u{fffd}'));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_structures_without_faithful_plain_unicode() {
        assert_eq!(render(r"\sqrt{x}"), None);
        assert_eq!(render(r"N_{A}"), None);
        assert_eq!(render(r"\frac{a+b}{c}"), None);
        assert_eq!(render(r"\unknown{x}"), None);
    }
}
