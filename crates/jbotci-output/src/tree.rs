//! Renderer for the source-backed syntax tree output format.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_morphology::{TreeNode as MorphologyTreeNode, Word, WordKind, WordLike};
use jbotci_source::SourceSpan;
use jbotci_syntax::WithIndicators;
use jbotci_syntax::ast::TextSyntax;
use jbotci_syntax::tree::{SyntaxTree, SyntaxTreeValue};
use jbotci_tree::{FieldRef, TreeVisitor};

use crate::{OutputError, TreeRenderOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum RenderEntry {
    Primary(TreeValue),
    Labelled(&'static str, TreeValue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct TreeEntry {
    label: Option<&'static str>,
    value: TreeValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct TreeNode {
    constructor: &'static str,
    entries: Vec<TreeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum TreeValue {
    Node(TreeNode),
    Collection(Vec<TreeValue>),
    Text(String),
    Span { char_start: usize, char_end: usize },
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
    let value = collapse_value(syntax_tree_value(value, source));
    let mut renderer = TreeRenderer {
        color: options.color,
        indent_step: options.indent,
        output: String::new(),
    };
    renderer.render_value(&value, 0);
    Ok(renderer.output)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn pretty_morphology_tree_with_options(
    words: &[WordLike],
    _source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    let value = collapse_value(TreeValue::Collection(
        words.iter().map(morphology_tree_value).collect(),
    ));
    let mut renderer = TreeRenderer {
        color: options.color,
        indent_step: options.indent,
        output: String::new(),
    };
    renderer.render_value(&value, 0);
    Ok(renderer.output)
}

#[requires(true)]
#[ensures(true)]
fn syntax_tree_value(value: SyntaxTreeValue, source: &str) -> TreeValue {
    match value {
        SyntaxTreeValue::Node(node) => TreeValue::Node(TreeNode {
            constructor: node.constructor,
            entries: node
                .entries
                .into_iter()
                .map(|entry| TreeEntry {
                    label: entry.label,
                    value: syntax_tree_value(entry.value, source),
                })
                .collect(),
        }),
        SyntaxTreeValue::Collection(items) => TreeValue::Collection(
            items
                .into_iter()
                .map(|item| syntax_tree_value(item, source))
                .collect(),
        ),
        SyntaxTreeValue::Word(word) => with_indicators_tree_value(&word),
        SyntaxTreeValue::Text(text) => TreeValue::Text(text),
    }
}

#[requires(true)]
#[ensures(true)]
fn with_indicators_tree_value(word: &WithIndicators<WordLike>) -> TreeValue {
    match word {
        WithIndicators::Bare(word_like) => morphology_tree_value(word_like),
        WithIndicators::Emphasized { bahe, word_like } => TreeValue::Node(TreeNode {
            constructor: "Emphasized",
            entries: vec![
                TreeEntry {
                    label: Some("bahe"),
                    value: word_tree_value(bahe),
                },
                TreeEntry {
                    label: None,
                    value: morphology_tree_value(word_like),
                },
            ],
        }),
        WithIndicators::WithIndicator {
            base,
            indicator,
            nai,
        } => {
            let mut entries = vec![
                TreeEntry {
                    label: None,
                    value: with_indicators_tree_value(base),
                },
                TreeEntry {
                    label: Some("indicator"),
                    value: word_tree_value(indicator),
                },
            ];
            if let Some(nai) = nai {
                entries.push(TreeEntry {
                    label: Some("nai"),
                    value: word_tree_value(nai),
                });
            }
            TreeValue::Node(TreeNode {
                constructor: "WithIndicator",
                entries,
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn word_tree_value(word: &Word) -> TreeValue {
    morphology_tree_value(&WordLike::bare(word.clone()))
}

#[requires(true)]
#[ensures(true)]
fn morphology_tree_value(word_like: &WordLike) -> TreeValue {
    let mut visitor = MorphologyTreeBuilder::default();
    word_like.visit_in_order(&mut visitor);
    visitor.finish()
}

#[requires(true)]
#[ensures(true)]
fn collapse_value(value: TreeValue) -> TreeValue {
    match value {
        TreeValue::Node(node) => collapse_node(node),
        TreeValue::Collection(items) => {
            TreeValue::Collection(items.into_iter().map(collapse_value).collect())
        }
        TreeValue::Text(..) | TreeValue::Span { .. } => value,
    }
}

#[requires(true)]
#[ensures(true)]
fn collapse_node(node: TreeNode) -> TreeValue {
    let entries = node
        .entries
        .into_iter()
        .map(|entry| TreeEntry {
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
    TreeValue::Node(TreeNode {
        constructor: node.constructor,
        entries,
    })
}

#[derive(Debug)]
#[invariant(true)]
struct TreeRenderer {
    color: bool,
    indent_step: usize,
    output: String,
}

impl TreeRenderer {
    #[requires(true)]
    #[ensures(true)]
    fn render_value(&mut self, value: &TreeValue, indent: usize) {
        match value {
            TreeValue::Node(node) => self.render_node(node, indent),
            TreeValue::Collection(items) => self.render_collection(items, indent),
            TreeValue::Text(text) => self.output.push_str(&self.string_literal(text)),
            TreeValue::Span {
                char_start,
                char_end,
            } => self
                .output
                .push_str(&self.span_literal(*char_start, *char_end)),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_node(&mut self, node: &TreeNode, indent: usize) {
        self.output
            .push_str(&self.constructor_token(node.constructor));
        if self.indent_step != 0 {
            self.output.push(' ');
        }
        self.output.push_str(&self.punctuation_token("{"));
        if node.entries.is_empty() {
            self.output.push_str(&self.punctuation_token("}"));
            return;
        }
        let entries = node.entries.iter().map(render_entry).collect::<Vec<_>>();
        if self.indent_step == 0 {
            self.render_inline_entries(&entries);
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
        for (index, entry) in entries.iter().enumerate() {
            if index > 0 {
                self.output.push_str(&self.punctuation_token(","));
            }
            match entry {
                RenderEntry::Primary(value) => self.render_value(value, 0),
                RenderEntry::Labelled(label, value) => {
                    self.output.push_str(&self.field_token(label));
                    self.output.push_str(&self.punctuation_token(":"));
                    self.render_value(value, 0);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_collection(&mut self, items: &[TreeValue], indent: usize) {
        self.output.push_str(&self.punctuation_token("["));
        if items.is_empty() {
            self.output.push_str(&self.punctuation_token("]"));
            return;
        }
        if self.indent_step == 0 {
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    self.output.push_str(&self.punctuation_token(","));
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
    #[ensures(!self.color -> ret.starts_with('"'))]
    fn string_literal(&self, text: &str) -> String {
        let literal = serde_json::to_string(text).expect("serializing string literal cannot fail");
        self.color_token(&literal, ColorRole::String)
    }

    #[requires(char_start <= char_end)]
    #[ensures(!ret.is_empty())]
    fn span_literal(&self, char_start: usize, char_end: usize) -> String {
        self.punctuation_token(&format!("[{char_start},{char_end}]"))
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
fn render_entry(entry: &TreeEntry) -> RenderEntry {
    match entry.label {
        Some(label) => RenderEntry::Labelled(label, entry.value.clone()),
        None => RenderEntry::Primary(entry.value.clone()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
enum MorphologyFrame {
    Node {
        constructor: &'static str,
        entries: Vec<TreeEntry>,
    },
    Field {
        name: Option<&'static str>,
        primary: bool,
        values: Vec<TreeValue>,
    },
}

#[derive(Debug, Default)]
#[invariant(true)]
struct MorphologyTreeBuilder {
    stack: Vec<MorphologyFrame>,
    root: Option<TreeValue>,
}

impl MorphologyTreeBuilder {
    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> TreeValue {
        self.root.expect("morphology tree walk produced a root")
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_value(&mut self, value: TreeValue) {
        match self.stack.last_mut() {
            Some(MorphologyFrame::Field { values, .. }) => values.push(value),
            Some(MorphologyFrame::Node { entries, .. }) => {
                entries.push(TreeEntry { label: None, value })
            }
            None => self.root = Some(value),
        }
    }
}

impl<'tree> TreeVisitor<'tree> for MorphologyTreeBuilder {
    type Node = jbotci_morphology::NodeRef<'tree>;
    type Atom = jbotci_morphology::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.stack.push(MorphologyFrame::Node {
            constructor: node.constructor_name(),
            entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        let Some(MorphologyFrame::Node {
            constructor,
            entries,
        }) = self.stack.pop()
        else {
            panic!("morphology tree walker exited a node without entering it");
        };
        self.push_value(TreeValue::Node(TreeNode {
            constructor,
            entries,
        }));
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: FieldRef) {
        self.stack.push(MorphologyFrame::Field {
            name: field.name,
            primary: field.primary,
            values: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: FieldRef) {
        let Some(MorphologyFrame::Field {
            name,
            primary,
            values,
        }) = self.stack.pop()
        else {
            panic!("morphology tree walker exited a field without entering it");
        };
        if values.is_empty() {
            return;
        }
        let Some(MorphologyFrame::Node { entries, .. }) = self.stack.last_mut() else {
            panic!("morphology tree field has no containing node");
        };
        if primary {
            for value in values {
                match value {
                    TreeValue::Collection(items) => {
                        entries.extend(
                            items
                                .into_iter()
                                .map(|value| TreeEntry { label: None, value }),
                        );
                    }
                    value => entries.push(TreeEntry { label: None, value }),
                }
            }
        } else {
            let value = if values.len() == 1 {
                values.into_iter().next().expect("length checked")
            } else {
                TreeValue::Collection(values)
            };
            entries.push(TreeEntry { label: name, value });
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        self.push_value(match atom {
            jbotci_morphology::AtomRef::WordKind(kind) => word_kind_value(kind),
            jbotci_morphology::AtomRef::String(text) => TreeValue::Text(text.clone()),
            jbotci_morphology::AtomRef::SourceSpan(span) => source_span_value(span),
        });
    }
}

#[requires(true)]
#[ensures(true)]
fn word_kind_value(kind: &WordKind) -> TreeValue {
    TreeValue::Text(kind.to_string())
}

#[requires(span.char_start <= span.char_end)]
#[ensures(true)]
fn source_span_value(span: &SourceSpan) -> TreeValue {
    TreeValue::Span {
        char_start: span.char_start,
        char_end: span.char_end,
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
            "Predicate {\n  leading_terms: [\n    Word {\n      kind: \"cmavo\",\n      phonemes: \"mi\",\n      span: [0,2],\n    },\n  ],\n  Word {\n    kind: \"gismu\",\n    phonemes: \"klama\",\n    span: [3,8],\n  },\n}"
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
    fn renders_compound_word_like_values_as_structured_nodes() {
        let zo = render("zo broda cu melbi", false);
        assert!(zo.contains("ZoQuote {"));
        assert!(zo.contains("phonemes: \"broda\""));

        let zoi = render("zoi gy hello gy cu melbi", false);
        assert!(zoi.contains("ZoiQuote {"));
        assert!(zoi.contains("quoted_text: [6,12]"));

        let lohu = render("lo'u mi klama le'u cu melbi", false);
        assert!(lohu.contains("LohuQuote {"));
        assert!(lohu.contains("phonemes: \"klama\""));

        let bu = render("abu cu lerfu", false);
        assert!(bu.contains("Letter {"));
        assert!(bu.contains("bu: Word {"));

        let zei = render("mi broda zei brode", false);
        assert!(zei.contains("ZeiLujvo {"));
        assert!(zei.contains("phonemes: \"brode\""));
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
        assert_eq!(
            output,
            r#"Predicate{leading_terms:[Word{kind:"cmavo",phonemes:"mi",span:[0,2]}],Word{kind:"gismu",phonemes:"klama",span:[3,8]}}"#
        );
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
