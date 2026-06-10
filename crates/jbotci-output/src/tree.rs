//! Renderer for the source-backed syntax tree output format.

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_morphology::{
    Cmavo, Phonemes, TreeNode as MorphologyTreeNode, Word, WordKind, WordLike,
};
use jbotci_semantics::references::{RawSyntaxNodeId, ReferenceAnalysis, SyntaxIndex};
use jbotci_source::SourceSpan;
use jbotci_syntax::ast::{
    AtomRef as SyntaxAtomRef, NodeRef as SyntaxNodeRef, TextSyntax, TreeNode as SyntaxAstTreeNode,
};
use jbotci_syntax::{WithIndicators, elidable_terminator_for_absent_field};
use jbotci_tree::{FieldRef, TreeVisitor};

use crate::references::ReferenceDisplayModel;
use crate::{GlyphStyle, OutputError, TreeRenderOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Primary(..) => true)]
#[invariant(::Labelled(..) => true)]
pub(crate) enum RenderEntry {
    Primary(TreeValue),
    Labelled(&'static str, TreeValue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(crate) struct TreeEntry {
    pub(crate) label: Option<&'static str>,
    pub(crate) value: TreeValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(crate) struct TreeNode {
    pub(crate) constructor: &'static str,
    pub(crate) entries: Vec<TreeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Node(..) => true)]
#[invariant(::Collection(..) => true)]
#[invariant(::Syntax { .. } => true)]
#[invariant(::Word => true)]
#[invariant(::Verbatim => true)]
#[invariant(::Text(..) => true)]
#[invariant(::Span => true)]
pub(crate) enum TreeValue {
    Node(TreeNode),
    Collection(Vec<TreeValue>),
    Syntax {
        syntax_ids: Vec<RawSyntaxNodeId>,
        value: Box<TreeValue>,
    },
    Word {
        constructor: &'static str,
        phonemes: String,
        span: Option<(usize, usize)>,
        elided: bool,
    },
    Verbatim {
        text: String,
        span: Option<(usize, usize)>,
    },
    Text(String),
    Span {
        byte_start: usize,
        byte_end: usize,
        char_start: usize,
        char_end: usize,
    },
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
pub(crate) fn pretty_tree_with_options(
    tree: &TextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    let reference_analysis = if options.show_refs {
        Some(
            ReferenceAnalysis::analyze(tree)
                .map_err(|error| OutputError::References(error.to_string()))?,
        )
    } else {
        None
    };
    let syntax_index = reference_analysis
        .as_ref()
        .map(|analysis| &analysis.syntax_index);
    let value = collapse_value(syntax_tree_value(tree, source, options, syntax_index));
    let references = reference_analysis
        .as_ref()
        .map(|analysis| ReferenceDisplayModel::new(analysis, &value, source, options));
    let mut renderer = TreeRenderer {
        color: options.color,
        glyphs: options.glyphs,
        indent_step: options.indent,
        show_spans: options.show_spans,
        references: references.as_ref(),
        output: String::new(),
    };
    renderer.render_value(&value, 0);
    Ok(renderer.output)
}

#[requires(true)]
#[ensures(true)]
pub fn reference_display_model_for_syntax_tree(
    analysis: &ReferenceAnalysis<'_>,
    tree: &TextSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> ReferenceDisplayModel {
    let value = collapse_value(syntax_tree_value(
        tree,
        source,
        options,
        Some(&analysis.syntax_index),
    ));
    ReferenceDisplayModel::new(analysis, &value, source, options)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn pretty_morphology_tree_with_options(
    words: &[WordLike],
    source: &str,
    options: TreeRenderOptions,
) -> Result<String, OutputError> {
    let value = collapse_value(TreeValue::Collection(
        words
            .iter()
            .map(|word_like| morphology_tree_value(word_like, source, options))
            .collect(),
    ));
    let mut renderer = TreeRenderer {
        color: options.color,
        glyphs: options.glyphs,
        indent_step: options.indent,
        show_spans: options.show_spans,
        references: None,
        output: String::new(),
    };
    renderer.render_value(&value, 0);
    Ok(renderer.output)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(crate) fn render_plain_tree_value_with_options(
    value: TreeValue,
    options: TreeRenderOptions,
) -> String {
    let value = collapse_value(value);
    let mut renderer = TreeRenderer {
        color: options.color,
        glyphs: options.glyphs,
        indent_step: options.indent,
        show_spans: options.show_spans,
        references: None,
        output: String::new(),
    };
    renderer.render_value(&value, 0);
    renderer.output
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn with_indicators_tree_value(
    word: &WithIndicators<WordLike>,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    match word {
        WithIndicators::Plain(word_like) => morphology_tree_value(word_like, source, options),
        WithIndicators::Emphasized { bahe, word_like } => TreeValue::Node(TreeNode {
            constructor: "Emphasized",
            entries: vec![
                TreeEntry {
                    label: Some("bahe"),
                    value: word_tree_value(bahe, source, options),
                },
                TreeEntry {
                    label: None,
                    value: morphology_tree_value(word_like, source, options),
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
                    value: with_indicators_tree_value(base, source, options),
                },
                TreeEntry {
                    label: Some("indicator"),
                    value: word_tree_value(indicator, source, options),
                },
            ];
            if let Some(nai) = nai {
                entries.push(TreeEntry {
                    label: Some("nai"),
                    value: word_tree_value(nai, source, options),
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
pub(crate) fn word_tree_value(word: &Word, source: &str, options: TreeRenderOptions) -> TreeValue {
    morphology_tree_value(&WordLike::bare(word.clone()), source, options)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn morphology_tree_value(
    word_like: &WordLike,
    source: &str,
    options: TreeRenderOptions,
) -> TreeValue {
    let mut visitor = MorphologyTreeBuilder::new(source, options);
    word_like.visit_in_order(&mut visitor);
    visitor.finish()
}

#[requires(true)]
#[ensures(true)]
fn syntax_tree_value(
    tree: &TextSyntax,
    source: &str,
    options: TreeRenderOptions,
    syntax_index: Option<&SyntaxIndex<'_>>,
) -> TreeValue {
    let mut visitor = SyntaxTreeBuilder::new(source, options, syntax_index);
    tree.visit_in_order(&mut visitor);
    visitor.finish()
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Node => true)]
#[invariant(::Field => true)]
#[invariant(::Collection => true)]
enum SyntaxFrame<'tree> {
    Node {
        node_ref: SyntaxNodeRef<'tree>,
        constructor: &'static str,
        syntax_id: Option<RawSyntaxNodeId>,
        entries: Vec<TreeEntry>,
    },
    Field {
        name: Option<&'static str>,
        primary: bool,
        values: Vec<TreeValue>,
        nested_entries: Vec<TreeEntry>,
    },
    Collection {
        items: Vec<TreeValue>,
    },
}

#[derive(Debug, Default)]
#[invariant(true)]
struct SyntaxTreeBuilder<'source, 'index, 'tree> {
    source: &'source str,
    options: TreeRenderOptions,
    syntax_index: Option<&'index SyntaxIndex<'tree>>,
    stack: Vec<SyntaxFrame<'tree>>,
    last_position: Option<RenderedPosition>,
    root: Option<TreeValue>,
}

impl<'source, 'index, 'tree> SyntaxTreeBuilder<'source, 'index, 'tree> {
    #[requires(true)]
    #[ensures(ret.source == source)]
    fn new(
        source: &'source str,
        options: TreeRenderOptions,
        syntax_index: Option<&'index SyntaxIndex<'tree>>,
    ) -> Self {
        Self {
            source,
            options,
            syntax_index,
            stack: Vec::new(),
            last_position: None,
            root: None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> TreeValue {
        self.root.expect("syntax tree walk produced a root")
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_value(&mut self, value: TreeValue) {
        match self.stack.last_mut() {
            Some(SyntaxFrame::Field { values, .. }) => values.push(value),
            Some(SyntaxFrame::Collection { items }) => items.push(value),
            Some(SyntaxFrame::Node { entries, .. }) => {
                entries.push(TreeEntry { label: None, value })
            }
            None => self.root = Some(value),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_labelled_entry_to_nearest_node(&mut self, label: &'static str, value: TreeValue) {
        for frame in self.stack.iter_mut().rev() {
            match frame {
                SyntaxFrame::Field { nested_entries, .. } => {
                    nested_entries.push(TreeEntry {
                        label: Some(label),
                        value,
                    });
                    return;
                }
                SyntaxFrame::Node { entries, .. } => {
                    entries.push(TreeEntry {
                        label: Some(label),
                        value,
                    });
                    return;
                }
                SyntaxFrame::Collection { .. } => {}
            }
        }
        panic!("syntax tree labelled field has no containing node");
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_values_in_order(&mut self, values: Vec<TreeValue>) {
        for value in values {
            match value {
                TreeValue::Collection(items) => {
                    for value in items {
                        self.push_value(value);
                    }
                }
                TreeValue::Syntax { syntax_ids, value } => match *value {
                    TreeValue::Collection(items) => {
                        for value in items {
                            self.push_value(syntax_value(syntax_ids.clone(), value));
                        }
                    }
                    value => self.push_value(syntax_value(syntax_ids, value)),
                },
                value => self.push_value(value),
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_entries_in_order(&mut self, entries: Vec<TreeEntry>) {
        for entry in entries {
            match entry.label {
                Some(label) => self.push_labelled_entry_to_nearest_node(label, entry.value),
                None => self.push_value(entry.value),
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_labelled_field_value(&mut self, label: &'static str, values: Vec<TreeValue>) {
        if values.is_empty() {
            return;
        }
        let value = if values.len() == 1 {
            values.into_iter().next().expect("length checked")
        } else {
            TreeValue::Collection(values)
        };
        self.push_labelled_entry_to_nearest_node(label, value);
    }
}

impl<'source, 'index, 'tree> TreeVisitor<'tree> for SyntaxTreeBuilder<'source, 'index, 'tree> {
    type Node = SyntaxNodeRef<'tree>;
    type Atom = SyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.stack.push(SyntaxFrame::Node {
            node_ref: node,
            constructor: syntax_constructor_name(node.constructor_name()),
            syntax_id: self.syntax_index.and_then(|index| index.id_of(node)),
            entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        let Some(SyntaxFrame::Node {
            node_ref: _,
            constructor,
            syntax_id,
            entries,
        }) = self.stack.pop()
        else {
            panic!("syntax tree walker exited a node without entering it");
        };
        let value = TreeValue::Node(TreeNode {
            constructor,
            entries,
        });
        self.push_value(match syntax_id {
            Some(id) => syntax_value(vec![id], value),
            None => value,
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: FieldRef) {
        self.stack.push(SyntaxFrame::Field {
            name: field.name,
            primary: field.primary,
            values: Vec::new(),
            nested_entries: Vec::new(),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: FieldRef) {
        let Some(SyntaxFrame::Field {
            name,
            primary,
            values,
            nested_entries,
        }) = self.stack.pop()
        else {
            panic!("syntax tree walker exited a field without entering it");
        };
        if values.is_empty() && nested_entries.is_empty() {
            return;
        }
        if primary || name.is_none() {
            self.push_values_in_order(values);
        } else {
            self.push_labelled_field_value(name.expect("checked above"), values);
        }
        self.push_entries_in_order(nested_entries);
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.stack
            .push(SyntaxFrame::Collection { items: Vec::new() });
    }

    #[requires(matches!(self.stack.last(), Some(SyntaxFrame::Collection { .. })))]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        let Some(SyntaxFrame::Collection { items }) = self.stack.pop() else {
            panic!("syntax tree walker exited a collection without entering it");
        };
        if !items.is_empty() {
            self.push_value(TreeValue::Collection(items));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        self.last_position = syntax_atom_end_position(atom);
        self.push_value(match atom {
            SyntaxAtomRef::Token(word) => {
                with_indicators_tree_value(word.as_indicators(), self.source, self.options)
            }
            SyntaxAtomRef::Word(word) => word_tree_value(word, self.source, self.options),
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_absent_optional_field(&mut self, field: FieldRef) {
        if !self.options.show_elided {
            return;
        }
        let Some(node) = current_syntax_node(&self.stack) else {
            return;
        };
        let Some(cmavo) = elidable_terminator_for_absent_field(node, field) else {
            return;
        };
        let Some(position) = self.last_position.clone() else {
            return;
        };
        self.push_value(elided_cmavo_tree_value(cmavo, position, self.options));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct RenderedPosition {
    byte_end: usize,
    char_end: usize,
}

#[requires(true)]
#[ensures(true)]
fn current_syntax_node<'tree>(stack: &[SyntaxFrame<'tree>]) -> Option<SyntaxNodeRef<'tree>> {
    stack.iter().rev().find_map(|frame| match frame {
        SyntaxFrame::Node { node_ref, .. } => Some(*node_ref),
        SyntaxFrame::Field { .. } | SyntaxFrame::Collection { .. } => None,
    })
}

#[requires(true)]
#[ensures(true)]
fn syntax_atom_end_position(atom: SyntaxAtomRef<'_>) -> Option<RenderedPosition> {
    match atom {
        SyntaxAtomRef::Token(token) => token
            .source_spans()
            .into_iter()
            .last()
            .map(span_end_position),
        SyntaxAtomRef::Word(word) => Some(span_end_position(word.span())),
    }
}

#[requires(span.byte_start <= span.byte_end)]
#[requires(span.char_start <= span.char_end)]
#[ensures(ret.byte_end == span.byte_end)]
fn span_end_position(span: &SourceSpan) -> RenderedPosition {
    RenderedPosition {
        byte_end: span.byte_end,
        char_end: span.char_end,
    }
}

#[requires(true)]
#[ensures(true)]
fn elided_cmavo_tree_value(
    cmavo: Cmavo,
    position: RenderedPosition,
    options: TreeRenderOptions,
) -> TreeValue {
    TreeValue::Word {
        constructor: "Cmavo",
        phonemes: elided_cmavo_text(cmavo, options.phonemes),
        span: Some((position.char_end, position.char_end)),
        elided: true,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn elided_cmavo_text(cmavo: Cmavo, options: jbotci_morphology::PhonemeRenderOptions) -> String {
    Phonemes::from_canonical(cmavo.canonical_text().to_owned())
        .expect("cmavo canonical text is valid phoneme text")
        .render(options)
}

#[requires(true)]
#[ensures(!ret.ends_with("Syntax"))]
fn syntax_constructor_name(constructor: &'static str) -> &'static str {
    constructor.strip_suffix("Syntax").unwrap_or(constructor)
}

#[requires(true)]
#[ensures(true)]
fn syntax_value(syntax_ids: Vec<RawSyntaxNodeId>, value: TreeValue) -> TreeValue {
    if syntax_ids.is_empty() {
        return value;
    }
    match value {
        TreeValue::Syntax {
            syntax_ids: mut inner_ids,
            value,
        } => {
            inner_ids.extend(syntax_ids);
            TreeValue::Syntax {
                syntax_ids: inner_ids,
                value,
            }
        }
        value => TreeValue::Syntax {
            syntax_ids,
            value: Box::new(value),
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn collapse_value(value: TreeValue) -> TreeValue {
    match value {
        TreeValue::Node(node) => collapse_node(node),
        TreeValue::Collection(items) => {
            TreeValue::Collection(items.into_iter().map(collapse_value).collect())
        }
        TreeValue::Syntax { syntax_ids, value } => syntax_value(syntax_ids, collapse_value(*value)),
        TreeValue::Word { .. }
        | TreeValue::Verbatim { .. }
        | TreeValue::Text(..)
        | TreeValue::Span { .. } => value,
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
struct TreeRenderer<'references> {
    color: bool,
    glyphs: GlyphStyle,
    indent_step: usize,
    show_spans: bool,
    references: Option<&'references ReferenceDisplayModel>,
    output: String,
}

impl TreeRenderer<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn render_value(&mut self, value: &TreeValue, indent: usize) {
        match value {
            TreeValue::Node(node) => self.render_node(node, indent),
            TreeValue::Collection(items) => self.render_collection(items, indent),
            TreeValue::Syntax { syntax_ids, value } => {
                self.render_syntax_value(syntax_ids, value, indent)
            }
            TreeValue::Word {
                constructor,
                phonemes,
                span,
                elided,
            } => self.render_word(constructor, phonemes, *span, *elided),
            TreeValue::Verbatim { text, span } => self.render_verbatim(text, *span),
            TreeValue::Text(text) => self.output.push_str(&self.string_literal(text)),
            TreeValue::Span {
                byte_start: _,
                byte_end: _,
                char_start,
                char_end,
            } => self
                .output
                .push_str(&self.span_literal(*char_start, *char_end)),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_syntax_value(
        &mut self,
        syntax_ids: &[RawSyntaxNodeId],
        value: &TreeValue,
        indent: usize,
    ) {
        let annotations = self
            .references
            .map(|references| references.annotations_for_syntax_ids(syntax_ids));
        if let Some(annotations) = annotations.as_ref() {
            for name in &annotations.incoming {
                self.output
                    .push_str(&self.reference_name(name, ReferenceRenderRole::Referent));
                self.output
                    .push_str(&self.punctuation_token(self.glyphs.arrow()));
                self.output.push(' ');
            }
        }
        self.render_value(value, indent);
        if let Some(annotations) = annotations.as_ref() {
            for name in &annotations.outgoing {
                self.output.push(' ');
                self.output
                    .push_str(&self.punctuation_token(self.glyphs.arrow()));
                self.output
                    .push_str(&self.reference_name(name, ReferenceRenderRole::Reference));
            }
        }
    }

    #[requires(!constructor.is_empty())]
    #[ensures(true)]
    fn render_word(
        &mut self,
        constructor: &str,
        phonemes: &str,
        span: Option<(usize, usize)>,
        elided: bool,
    ) {
        self.output.push_str(&self.constructor_token(constructor));
        self.render_optional_node_span(span);
        self.output.push(' ');
        if elided {
            self.output.push_str(&self.elided_string_literal(phonemes));
        } else {
            self.output.push_str(&self.string_literal(phonemes));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_verbatim(&mut self, text: &str, span: Option<(usize, usize)>) {
        self.output.push_str(&self.constructor_token("Verbatim"));
        self.render_optional_node_span(span);
        self.output.push(' ');
        self.output.push_str(&self.string_literal(text));
    }

    #[requires(true)]
    #[ensures(true)]
    fn render_node(&mut self, node: &TreeNode, indent: usize) {
        self.output
            .push_str(&self.constructor_token(node.constructor));
        self.render_optional_node_span(tree_node_span(node));
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

    #[requires(true)]
    #[ensures(!self.color -> ret.starts_with('"'))]
    fn elided_string_literal(&self, text: &str) -> String {
        let literal = serde_json::to_string(text).expect("serializing string literal cannot fail");
        self.elided_color_token(&literal, ColorRole::String)
    }

    #[requires(char_start <= char_end)]
    #[ensures(!ret.is_empty())]
    fn span_literal(&self, char_start: usize, char_end: usize) -> String {
        let mut output = String::new();
        output.push_str(&self.punctuation_token("["));
        output.push_str(&self.number_token(char_start));
        output.push_str(&self.punctuation_token(","));
        output.push_str(&self.number_token(char_end));
        output.push_str(&self.punctuation_token("]"));
        output
    }

    #[requires(span.is_none_or(|(start, end)| start <= end))]
    #[ensures(true)]
    fn render_optional_node_span(&mut self, span: Option<(usize, usize)>) {
        if !self.show_spans {
            return;
        }
        if let Some((char_start, char_end)) = span {
            self.output.push(' ');
            self.output
                .push_str(&self.span_marker(char_start, char_end));
        }
    }

    #[requires(char_start <= char_end)]
    #[ensures(!ret.is_empty())]
    fn span_marker(&self, char_start: usize, char_end: usize) -> String {
        let mut output = String::new();
        output.push_str(&self.punctuation_token("@"));
        output.push_str(&self.punctuation_token("["));
        output.push_str(&self.span_number_token(char_start));
        output.push_str(&self.punctuation_token(self.glyphs.span_leader()));
        output.push_str(&self.span_number_token(char_end));
        output.push_str(&self.punctuation_token(")"));
        output
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn reference_name(
        &self,
        name: &crate::references::ReferenceName,
        role: ReferenceRenderRole,
    ) -> String {
        let mut output = String::new();
        output.push_str(&self.color_token(&name.stem, role.stem_color()));
        if let Some(index) = name.occurrence {
            output.push_str(
                &self.color_token(&self.glyphs.numeric_suffix(index), role.suffix_color()),
            );
        }
        if let Some(slot) = &name.slot {
            output.push_str(&self.punctuation_token(self.glyphs.slot_open()));
            output.push_str(&self.color_token(&slot.text(), ColorRole::ReferenceSlot));
            output.push_str(&self.punctuation_token(self.glyphs.slot_close()));
        }
        output
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
    #[ensures(!ret.is_empty())]
    fn number_token(&self, value: usize) -> String {
        self.color_token(&value.to_string(), ColorRole::Number)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn span_number_token(&self, value: usize) -> String {
        self.color_token(&value.to_string(), ColorRole::SpanNumber)
    }

    #[requires(true)]
    #[ensures(!self.color -> ret == text)]
    fn color_token(&self, text: &str, role: ColorRole) -> String {
        if !self.color {
            return text.to_owned();
        }
        format!("{}{}{}", role.open(), text, "\x1b[39m")
    }

    #[requires(true)]
    #[ensures(!self.color -> ret == text)]
    fn elided_color_token(&self, text: &str, role: ColorRole) -> String {
        if !self.color {
            return text.to_owned();
        }
        format!("{}\x1b[9m{}\x1b[29m{}", role.open(), text, "\x1b[39m")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum ReferenceRenderRole {
    Reference,
    Referent,
}

impl ReferenceRenderRole {
    #[requires(true)]
    #[ensures(matches!(ret, ColorRole::ReferenceStem | ColorRole::ReferentStem))]
    fn stem_color(self) -> ColorRole {
        match self {
            Self::Reference => ColorRole::ReferenceStem,
            Self::Referent => ColorRole::ReferentStem,
        }
    }

    #[requires(true)]
    #[ensures(matches!(ret, ColorRole::ReferenceSuffix | ColorRole::ReferentSuffix))]
    fn suffix_color(self) -> ColorRole {
        match self {
            Self::Reference => ColorRole::ReferenceSuffix,
            Self::Referent => ColorRole::ReferentSuffix,
        }
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

#[requires(true)]
#[ensures(ret.is_none_or(|(start, end)| start <= end))]
fn tree_node_span(node: &TreeNode) -> Option<(usize, usize)> {
    span_from_values(
        node.entries
            .iter()
            .filter_map(|entry| value_span(&entry.value)),
    )
}

#[requires(true)]
#[ensures(ret.is_none_or(|(start, end)| start <= end))]
fn value_span(value: &TreeValue) -> Option<(usize, usize)> {
    match value {
        TreeValue::Node(node) => tree_node_span(node),
        TreeValue::Collection(items) => span_from_values(items.iter().filter_map(value_span)),
        TreeValue::Syntax { value, .. } => value_span(value),
        TreeValue::Word { span, .. } | TreeValue::Verbatim { span, .. } => *span,
        TreeValue::Text(_) => None,
        TreeValue::Span {
            char_start,
            char_end,
            ..
        } => Some((*char_start, *char_end)),
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|(start, end)| start <= end))]
fn span_from_values<I>(spans: I) -> Option<(usize, usize)>
where
    I: IntoIterator<Item = (usize, usize)>,
{
    let mut iter = spans.into_iter();
    let (mut start, mut end) = iter.next()?;
    for (item_start, item_end) in iter {
        start = start.min(item_start);
        end = end.max(item_end);
    }
    Some((start, end))
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Node => true)]
#[invariant(::Field => true)]
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

#[derive(Debug)]
#[invariant(true)]
struct MorphologyTreeBuilder<'source> {
    source: &'source str,
    options: TreeRenderOptions,
    stack: Vec<MorphologyFrame>,
    root: Option<TreeValue>,
}

impl<'source> MorphologyTreeBuilder<'source> {
    #[requires(true)]
    #[ensures(ret.source == source)]
    fn new(source: &'source str, options: TreeRenderOptions) -> Self {
        Self {
            source,
            options,
            stack: Vec::new(),
            root: None,
        }
    }

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

impl<'tree> TreeVisitor<'tree> for MorphologyTreeBuilder<'_> {
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
        let value = match morphology_node_value(constructor, &entries, self.options) {
            Some(value) => value,
            None => TreeValue::Node(TreeNode {
                constructor,
                entries,
            }),
        };
        self.push_value(value);
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
            jbotci_morphology::AtomRef::Phonemes(phonemes) => {
                TreeValue::Text(phonemes.render(self.options.phonemes))
            }
            jbotci_morphology::AtomRef::String(text) => TreeValue::Text(text.clone()),
            jbotci_morphology::AtomRef::SourceSpan(span) => source_span_value(span),
        });
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn morphology_node_value(
    constructor: &'static str,
    entries: &[TreeEntry],
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    word_node_value(constructor, entries, options)
        .or_else(|| jvopau_node_value(constructor, entries, options))
        .or_else(|| verbatim_node_value(constructor, entries))
}

#[requires(true)]
#[ensures(true)]
fn word_node_value(
    constructor: &'static str,
    entries: &[TreeEntry],
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    let kind = word_kind_from_constructor(constructor)?;
    let phonemes = if kind == WordKind::Lujvo && options.decompose_lujvo {
        lujvo_phoneme_text_from_entries(entries, true, options.glyphs)?
    } else if kind == WordKind::Lujvo {
        lujvo_phoneme_text_from_entries(entries, false, options.glyphs)?
    } else {
        phonemes_from_labelled_entries(entries)?.render(options.phonemes)
    };
    Some(TreeValue::Word {
        constructor,
        phonemes,
        span: span_from_labelled_entries(entries),
        elided: false,
    })
}

#[requires(true)]
#[ensures(true)]
fn jvopau_node_value(
    constructor: &'static str,
    entries: &[TreeEntry],
    options: TreeRenderOptions,
) -> Option<TreeValue> {
    if !matches!(constructor, "Rafsi" | "Hyphen") {
        return None;
    }
    Some(TreeValue::Text(
        phonemes_from_labelled_entries(entries)?.render(options.phonemes),
    ))
}

#[requires(true)]
#[ensures(true)]
fn verbatim_node_value(constructor: &'static str, entries: &[TreeEntry]) -> Option<TreeValue> {
    if constructor != "Verbatim" {
        return None;
    }
    for entry in entries {
        if let (Some("text"), TreeValue::Text(text)) = (entry.label, &entry.value) {
            return Some(TreeValue::Verbatim {
                text: text.trim().to_owned(),
                span: span_from_labelled_entries(entries),
            });
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn phonemes_from_labelled_entries(entries: &[TreeEntry]) -> Option<Phonemes> {
    for entry in entries {
        if let (Some("phonemes") | None, TreeValue::Text(text)) = (entry.label, &entry.value) {
            return Phonemes::from_canonical(text.clone()).ok();
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn lujvo_phoneme_text_from_entries(
    entries: &[TreeEntry],
    decompose: bool,
    glyphs: GlyphStyle,
) -> Option<String> {
    let mut parts = Vec::new();
    for entry in entries {
        match &entry.value {
            TreeValue::Text(part) => parts.push(part.clone()),
            TreeValue::Collection(values) => {
                for part in values {
                    if let TreeValue::Text(part) = part {
                        parts.push(part.clone());
                    }
                }
            }
            _ => {}
        }
    }
    (!parts.is_empty()).then(|| {
        if decompose {
            parts.join(glyphs.lujvo_separator())
        } else {
            parts.join("")
        }
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(|(start, end)| start <= end))]
fn span_from_labelled_entries(entries: &[TreeEntry]) -> Option<(usize, usize)> {
    entries
        .iter()
        .find_map(|entry| match (&entry.label, &entry.value) {
            (
                Some("span"),
                TreeValue::Span {
                    char_start,
                    char_end,
                    ..
                },
            ) => Some((*char_start, *char_end)),
            _ => None,
        })
}

#[requires(true)]
#[ensures(true)]
fn word_kind_from_constructor(constructor: &str) -> Option<WordKind> {
    Some(match constructor {
        "Cmavo" => WordKind::Cmavo,
        "Gismu" => WordKind::Gismu,
        "Lujvo" => WordKind::Lujvo,
        "Fuhivla" => WordKind::Fuhivla,
        "Cmevla" => WordKind::Cmevla,
        _ => return None,
    })
}

#[requires(span.char_start <= span.char_end)]
#[ensures(true)]
pub(crate) fn source_span_value(span: &SourceSpan) -> TreeValue {
    TreeValue::Span {
        byte_start: span.byte_start,
        byte_end: span.byte_end,
        char_start: span.char_start,
        char_end: span.char_end,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorRole {
    Constructor,
    Field,
    Punctuation,
    Number,
    SpanNumber,
    ReferenceStem,
    ReferenceSuffix,
    ReferentStem,
    ReferentSuffix,
    ReferenceSlot,
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
            Self::Number => "\x1b[35m",
            Self::SpanNumber => "\x1b[37m",
            Self::ReferenceStem => "\x1b[36m",
            Self::ReferenceSuffix => "\x1b[96m",
            Self::ReferentStem => "\x1b[35m",
            Self::ReferentSuffix => "\x1b[95m",
            Self::ReferenceSlot => "\x1b[97m",
            Self::String => "\x1b[33m",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};
    use jbotci_morphology::{
        GlideMark, PhonemeRenderOptions, StressMark, segment_words_with_modifiers,
    };
    use jbotci_syntax::parse_syntax_tree;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_basic_tree_with_primary_collapse() {
        let output = render("mi klama", false);
        assert_eq!(
            output,
            "Bridi {\n  leading_terms: [\n    Cmavo \"mi\",\n  ],\n  Gismu \"kláma\",\n}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colorizes_tree_tokens() {
        let output = render("mi klama", true);
        assert!(output.contains("\x1b[94mBridi\x1b[39m"));
        assert!(output.contains("\x1b[32mleading_terms\x1b[39m"));
        assert!(output.contains("\x1b[33m\"mi\"\x1b[39m"));
        assert!(output.contains("\x1b[94mCmavo\x1b[39m"));
        assert!(output.contains("\x1b[90m{\x1b[39m"));
        assert!(output.contains("\x1b[90m[\x1b[39m"));
        assert!(output.contains("\x1b[90m]\x1b[39m"));
        assert!(!output.contains("\x1b[36m"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colorizes_visible_span_markers_with_white_offsets() {
        let output = run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("mi klama").expect("morphology");
            let parsed = parse_syntax_tree(&words).expect("syntax");
            pretty_tree_with_options(
                &parsed.parse_tree,
                "mi klama",
                TreeRenderOptions {
                    color: true,
                    show_spans: true,
                    ..TreeRenderOptions::default()
                },
            )
            .expect("tree render")
        });

        assert!(output.contains(
            "\x1b[90m@\x1b[39m\x1b[90m[\x1b[39m\x1b[37m0\x1b[39m\x1b[90m‥\x1b[39m\x1b[37m8\x1b[39m\x1b[90m)\x1b[39m"
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn colorizes_reference_names_by_direction() {
        let output = render_refs_with_options(
            "mi klama do i do klama mi",
            TreeRenderOptions {
                color: true,
                show_refs: true,
                ..TreeRenderOptions::default()
            },
        );

        assert!(output.contains("\x1b[35mk\x1b[39m\x1b[95m₁\x1b[39m"));
        assert!(output.contains("\x1b[36mk\x1b[39m\x1b[96m₁\x1b[39m"));
        assert!(output.contains("\x1b[90m⟨\x1b[39m\x1b[97m1\x1b[39m\x1b[90m⟩\x1b[39m"));
        assert!(output.contains("\x1b[90m→\x1b[39m"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn keeps_free_modifiers_label_when_present() {
        let output = render("mi klama to coi toi", false);
        assert!(output.contains("free_modifiers: ["));
        assert!(output.contains("ParentheticalText {"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_compound_word_like_values_as_structured_nodes() {
        let zo = render("zo broda cu melbi", false);
        assert!(zo.contains("QuotedWord {"));
        assert!(zo.contains("Cmavo \"zo\""));
        assert!(zo.contains("Gismu \"bróda\""));

        let zoi = render("zoi gy hello gy cu melbi", false);
        assert!(zoi.contains("DelimitedNonLojbanQuote {"));
        assert!(zoi.contains("quoted_text: Verbatim \"hello\""));

        let lohu = render("lo'u mi klama le'u cu melbi", false);
        assert!(lohu.contains("QuotedWords {"));
        assert!(lohu.contains("Gismu \"kláma\""));

        let bu = render("abu cu lerfu", false);
        assert!(bu.contains("LerfuWord {"));
        assert!(bu.contains("bu: Cmavo \"bu\""));

        let zei = render("mi broda zei brode", false);
        assert!(zei.contains("ZeiCompound {"));
        assert!(zei.contains("Gismu \"bróde\""));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_single_line_when_indent_is_zero() {
        run_on_normal_stack(|| {
            let words = segment_words_with_modifiers("mi klama").expect("morphology");
            let parsed = parse_syntax_tree(&words).expect("syntax");
            let output = pretty_tree_with_options(
                &parsed.parse_tree,
                "mi klama",
                TreeRenderOptions {
                    color: false,
                    indent: 0,
                    ..TreeRenderOptions::default()
                },
            )
            .expect("tree render");
            assert_eq!(output, r#"Bridi{leading_terms:[Cmavo "mi"],Gismu "kláma"}"#);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_resolved_references_in_tree_output() {
        let output = render_refs("mi klama le zarci i do klama ri", true);
        assert_eq!(
            output,
            "Paragraph @[0‥31) {\n  Bridi @[0‥17) {\n    leading_terms: [\n      k₁⟨1⟩→ Cmavo @[0‥2) \"mi\",\n    ],\n    SelbriBridiTail @[3‥17) {\n      Gismu @[3‥8) \"kláma\" →k₁,\n      terms: [\n        k₁⟨2⟩→ ri₁→ Description @[9‥17) {\n          description: Cmavo @[9‥11) \"le\",\n          selbri: Gismu @[12‥17) \"zárci\",\n        },\n      ],\n    },\n  },\n  ParagraphStatement @[18‥31) {\n    i: Cmavo @[18‥19) \"i\",\n    Bridi @[20‥31) {\n      leading_terms: [\n        k₂⟨1⟩→ Cmavo @[20‥22) \"do\",\n      ],\n      SelbriBridiTail @[23‥31) {\n        Gismu @[23‥28) \"kláma\" →k₂,\n        terms: [\n          k₂⟨2⟩→ Cmavo @[29‥31) \"ri\" →ri₁,\n        ],\n      },\n    },\n  },\n}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_ascii_references_spans_and_phonemes() {
        let output = render_refs_with_options(
            "mi klama le zarci i do klama ri",
            TreeRenderOptions {
                glyphs: GlyphStyle::Ascii,
                show_spans: true,
                show_refs: true,
                phonemes: PhonemeRenderOptions {
                    mark_stress: StressMark::None,
                    mark_glides: GlideMark::None,
                },
                ..TreeRenderOptions::default()
            },
        );

        assert!(output.contains("k1<1>-> Cmavo @[0..2) \"mi\""));
        assert!(output.contains("Gismu @[3..8) \"klama\" ->k1"));
        assert!(output.contains("k1<2>-> ri1-> Description"));
        assert!(output.contains("Cmavo @[29..31) \"ri\" ->ri1"));
        assert!(!output.contains('→'));
        assert!(!output.contains('⟨'));
        assert!(!output.contains('⟩'));
        assert!(!output.contains('‥'));
        assert!(!output.contains('á'));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_only_base_frame_for_converted_selbri() {
        let output = render_refs("mi se klama do", false);
        assert!(output.contains("k⟨2⟩→ Cmavo \"mi\""));
        assert!(output.contains("k⟨1⟩→ Cmavo \"do\""));
        assert!(output.contains("Gismu \"kláma\" →k"));
        assert!(!output.contains("s⟨"));
        assert!(!output.contains("Cmavo \"se\" →"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_duplicate_place_fillers_with_same_label() {
        let output = render_refs("fa mi fa do klama", false);
        assert_eq!(output.matches("k⟨1⟩→ Cmavo").count(), 2);
        assert!(output.contains("Gismu \"kláma\" →k"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_modal_place_labels() {
        let output = render_refs("mi ta'i do klama", false);
        assert!(output.contains("k⟨ta'i⟩→ Cmavo \"do\""));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_duplicate_prefixes_across_repeated_words() {
        let output = render_refs("mi klama le karce be do i do klama le karce be mi", false);
        assert!(output.contains("Gismu \"kláma\" →kl₁"));
        assert!(output.contains("Gismu \"kláma\" →kl₂"));
        assert!(output.contains("Gismu \"kárce\" →ká₁"));
        assert!(output.contains("Gismu \"kárce\" →ká₂"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn renders_resolved_discourse_reference_kinds() {
        let gohi = render_refs("mi klama .i go'i", false);
        assert!(gohi.contains("go'i₁→ Bridi"));
        assert!(gohi.contains("Cmavo \"go'i\" →go'i₁"));

        let goi = render_refs("le nanmu goi ko'a cu klama .i ko'a cadzu", false);
        assert!(goi.contains("Cmavo \"ko'a\" →ko'a₁"));
        assert!(goi.contains("Cmavo \"ko'a\" →ko'a₂"));
        assert!(goi.contains("ko'a₁→ ko'a₂→ Description"));

        let cei = render_refs("mi broda cei klama do", false);
        assert!(cei.contains("k→ Bridi"));
        assert!(cei.contains("Gismu \"bróda\" →b"));
        assert!(!cei.contains("k→ Gismu \"bróda\""));
        assert!(cei.contains("Gismu \"kláma\" →k"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn omits_unresolved_and_vague_discourse_references() {
        let output = render_refs("ri klama .i ra klama .i ru klama", false);
        assert!(!output.contains("→ri"));
        assert!(!output.contains("→ra"));
        assert!(!output.contains("→ru"));
        assert!(!output.contains("ri₁→"));
        assert!(!output.contains("ra₁→"));
        assert!(!output.contains("ru₁→"));
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn render(text: &str, color: bool) -> String {
        let text = text.to_owned();
        run_on_normal_stack(move || {
            let words = segment_words_with_modifiers(&text).expect("morphology");
            let parsed = parse_syntax_tree(&words).expect("syntax");
            pretty_tree_with_options(
                &parsed.parse_tree,
                &text,
                TreeRenderOptions {
                    color,
                    indent: 2,
                    ..TreeRenderOptions::default()
                },
            )
            .expect("tree render")
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn render_refs(text: &str, show_spans: bool) -> String {
        render_refs_with_options(
            text,
            TreeRenderOptions {
                color: false,
                indent: 2,
                show_spans,
                show_refs: true,
                ..TreeRenderOptions::default()
            },
        )
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn render_refs_with_options(text: &str, options: TreeRenderOptions) -> String {
        let text = text.to_owned();
        run_on_normal_stack(move || {
            let words = segment_words_with_modifiers(&text).expect("morphology");
            let parsed = parse_syntax_tree(&words).expect("syntax");
            pretty_tree_with_options(&parsed.parse_tree, &text, options).expect("tree render")
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_on_normal_stack<R>(f: impl FnOnce() -> R) -> R {
        f()
    }
}
