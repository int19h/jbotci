//! Renderer for the source-backed syntax tree output format.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_syntax::ast::TextSyntax;
use jbotci_syntax::tree::{SyntaxTree, SyntaxTreeEntry, SyntaxTreeNode, SyntaxTreeValue};

use crate::{OutputError, TreeRenderOptions, surface};

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum RenderEntry {
    Primary(SyntaxTreeValue),
    Labelled(&'static str, SyntaxTreeValue),
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub(crate) fn pretty_tree_with_options(
    tree: &TextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    let value = tree.syntax_tree_value().ok_or_else(|| {
        OutputError::InvalidSyntaxTree("syntax tree did not produce a root value".to_owned())
    })?;
    let value = collapse_value(value);
    let mut renderer = TreeRenderer {
        source,
        color: options.color,
        indent_step: options.indent,
        output: String::new(),
    };
    renderer.render_value(&value, 0);
    Ok(renderer.output)
}

#[requires(true)]
#[ensures(true)]
fn collapse_value(value: SyntaxTreeValue) -> SyntaxTreeValue {
    match value {
        SyntaxTreeValue::Node(node) => collapse_node(node),
        SyntaxTreeValue::Collection(items) => {
            SyntaxTreeValue::Collection(items.into_iter().map(collapse_value).collect())
        }
        SyntaxTreeValue::Word(..) | SyntaxTreeValue::Text(..) => value,
    }
}

#[requires(true)]
#[ensures(true)]
fn collapse_node(node: SyntaxTreeNode) -> SyntaxTreeValue {
    let entries = node
        .entries
        .into_iter()
        .map(|entry| SyntaxTreeEntry {
            label: entry.label,
            value: collapse_value(entry.value),
        })
        .collect::<Vec<_>>();
    if entries.len() == 1 && entries[0].label.is_none() {
        let mut entries = entries;
        return entries
            .pop()
            .expect("length check guarantees one entry")
            .value;
    }
    SyntaxTreeValue::Node(SyntaxTreeNode {
        constructor: node.constructor,
        entries,
    })
}

#[derive(Debug)]
#[invariant(true)]
struct TreeRenderer<'a> {
    source: &'a str,
    color: bool,
    indent_step: usize,
    output: String,
}

impl TreeRenderer<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn render_value(&mut self, value: &SyntaxTreeValue, indent: usize) {
        match value {
            SyntaxTreeValue::Node(node) => self.render_node(node, indent),
            SyntaxTreeValue::Collection(items) => self.render_collection(items, indent),
            SyntaxTreeValue::Word(word) => self.output.push_str(&self.word_literal(word)),
            SyntaxTreeValue::Text(text) => self.output.push_str(&self.string_literal(text)),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_node(&mut self, node: &SyntaxTreeNode, indent: usize) {
        self.output
            .push_str(&self.constructor_token(node.constructor));
        self.output.push(' ');
        self.output.push_str(&self.punctuation_token("{"));
        if node.entries.is_empty() {
            self.output.push_str(&self.punctuation_token("}"));
            return;
        }
        let entries = node.entries.iter().map(render_entry).collect::<Vec<_>>();
        if self.indent_step == 0 {
            self.render_inline_entries(&entries);
            self.output.push(' ');
        } else {
            self.render_entries(&entries, indent);
            self.output.push('\n');
            self.push_indent(indent);
        }
        self.output.push_str(&self.punctuation_token("}"));
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_entries(&mut self, entries: &[RenderEntry], indent: usize) {
        let child_indent = indent + self.indent_step;
        for entry in entries {
            self.output.push('\n');
            self.push_indent(child_indent);
            match entry {
                RenderEntry::Primary(value) => self.render_value(value, child_indent),
                RenderEntry::Labelled(label, value) => {
                    self.output.push_str(&self.field_token(label));
                    self.output.push_str(&self.punctuation_token(":"));
                    self.output.push(' ');
                    self.render_value(value, child_indent);
                }
            }
            self.output.push_str(&self.punctuation_token(","));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_inline_entries(&mut self, entries: &[RenderEntry]) {
        self.output.push(' ');
        for (index, entry) in entries.iter().enumerate() {
            if index > 0 {
                self.output.push_str(&self.punctuation_token(","));
                self.output.push(' ');
            }
            match entry {
                RenderEntry::Primary(value) => self.render_value(value, 0),
                RenderEntry::Labelled(label, value) => {
                    self.output.push_str(&self.field_token(label));
                    self.output.push_str(&self.punctuation_token(":"));
                    self.output.push(' ');
                    self.render_value(value, 0);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_collection(&mut self, items: &[SyntaxTreeValue], indent: usize) {
        self.output.push_str(&self.punctuation_token("["));
        if items.is_empty() {
            self.output.push_str(&self.punctuation_token("]"));
            return;
        }
        if self.indent_step == 0 {
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    self.output.push_str(&self.punctuation_token(","));
                    self.output.push(' ');
                }
                self.render_value(item, 0);
            }
            self.output.push_str(&self.punctuation_token("]"));
            return;
        }
        let child_indent = indent + self.indent_step;
        for item in items {
            self.output.push('\n');
            self.push_indent(child_indent);
            self.render_value(item, child_indent);
            self.output.push_str(&self.punctuation_token(","));
        }
        self.output.push('\n');
        self.push_indent(indent);
        self.output.push_str(&self.punctuation_token("]"));
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_indent(&mut self, indent: usize) {
        self.output.extend(std::iter::repeat_n(' ', indent));
    }

    #[requires(true)]
    #[ensures(true)]
    fn word_literal(
        &self,
        word: &jbotci_syntax::WithIndicators<jbotci_morphology::WordLike>,
    ) -> String {
        self.string_literal(&surface::format_with_indicators(word, self.source))
    }

    #[requires(true)]
    #[ensures(!self.color -> ret.starts_with('"'))]
    fn string_literal(&self, text: &str) -> String {
        let literal = serde_json::to_string(text).expect("serializing string literal cannot fail");
        self.color_token(&literal, ColorRole::String)
    }

    #[requires(!text.is_empty())]
    #[ensures(!ret.is_empty())]
    fn constructor_token(&self, text: &str) -> String {
        self.color_token(text, ColorRole::Constructor)
    }

    #[requires(!text.is_empty())]
    #[ensures(!ret.is_empty())]
    fn field_token(&self, text: &str) -> String {
        self.color_token(text, ColorRole::Field)
    }

    #[requires(!text.is_empty())]
    #[ensures(!ret.is_empty())]
    fn punctuation_token(&self, text: &str) -> String {
        self.color_token(text, ColorRole::Punctuation)
    }

    #[requires(true)]
    #[ensures(!self.color -> ret == text)]
    fn color_token(&self, text: &str, role: ColorRole) -> String {
        if !self.color {
            return text.to_owned();
        }
        format!("{}{}{}", role.open(), text, "\x1b[39m")
    }
}

#[requires(true)]
#[ensures(true)]
fn render_entry(entry: &SyntaxTreeEntry) -> RenderEntry {
    match entry.label {
        Some(label) => RenderEntry::Labelled(label, entry.value.clone()),
        None => RenderEntry::Primary(entry.value.clone()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum ColorRole {
    Constructor,
    Field,
    Punctuation,
    String,
}

impl ColorRole {
    #[requires(true)]
    #[ensures(ret.starts_with("\u{1b}["))]
    fn open(self) -> &'static str {
        match self {
            Self::Constructor => "\x1b[94m",
            Self::Field => "\x1b[32m",
            Self::Punctuation => "\x1b[90m",
            Self::String => "\x1b[33m",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};
    use jbotci_morphology::segment_words_with_modifiers;
    use jbotci_syntax::parse_syntax_tree;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_basic_tree_with_primary_collapse() {
        let output = render("mi klama", false);
        assert_eq!(
            output,
            "Predicate {\n  leading_terms: [\n    \"mi\",\n  ],\n  \"kláma\",\n}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colorizes_tree_tokens() {
        let output = render("mi klama", true);
        assert!(output.contains("\x1b[94mPredicate\x1b[39m"));
        assert!(output.contains("\x1b[32mleading_terms\x1b[39m"));
        assert!(output.contains("\x1b[33m\"mi\"\x1b[39m"));
        assert!(output.contains("\x1b[90m{\x1b[39m"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn keeps_free_modifiers_label_when_present() {
        let output = render("mi klama to coi toi", false);
        assert!(output.contains("WithFreeModifiers"));
        assert!(output.contains("free_modifiers: ["));
        assert!(output.contains("To {"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_compound_quote_word_like_values_as_single_string_leaves() {
        let zo = render("zo broda cu melbi", false);
        assert!(zo.contains("\"zo-«bróda»\""));

        let zoi = render("zoi gy hello gy cu melbi", false);
        assert!(zoi.contains("\"zoĭ-gy-«hello»-gy\""));

        let lohu = render("lo'u mi klama le'u cu melbi", false);
        assert!(lohu.contains("\"lo'u-«mi kláma»-le'u\""));

        let bu = render("abu cu lerfu", false);
        assert!(bu.contains("\".a-bu\""));

        let zei = render("mi broda zei brode", false);
        assert!(zei.contains("\"bróda-zeĭ-bróde\""));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_single_line_when_indent_is_zero() {
        let words = segment_words_with_modifiers("mi klama").expect("morphology");
        let parsed = parse_syntax_tree(&words).expect("syntax");
        let output = pretty_tree_with_options(
            &parsed.parse_tree,
            "mi klama",
            TreeRenderOptions {
                color: false,
                indent: 0,
            },
        )
        .expect("tree render");
        assert_eq!(output, r#"Predicate { leading_terms: ["mi"], "kláma" }"#);
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn render(text: &str, color: bool) -> String {
        let words = segment_words_with_modifiers(text).expect("morphology");
        let parsed = parse_syntax_tree(&words).expect("syntax");
        pretty_tree_with_options(
            &parsed.parse_tree,
            text,
            TreeRenderOptions { color, indent: 2 },
        )
        .expect("tree render")
    }
}
