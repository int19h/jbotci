//! Shared gentufa block layout and SVG/PNG export support.

mod render;

use std::cmp::Reverse;
use std::collections::HashMap;
use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_morphology::{
    Cmavo, PhonemeRenderOptions, Phonemes, Word, WordKind, WordLike, WordLikeData,
};
pub use jbotci_output::{GlideMark, StressMark};
use jbotci_output::{
    ReferenceDisplayModel, ReferenceName as OutputReferenceName,
    ReferenceSlotName as OutputReferenceSlotName,
};
use jbotci_semantics::references::{RawSyntaxNodeId, ReferenceAnalysis, SyntaxNodeMetadata};
use jbotci_source::SourceSpan;
use jbotci_syntax::ast::{AtomRef as SyntaxAtomRef, NodeRef as SyntaxNodeRef, TextSyntax};
use jbotci_syntax::tree::TreeNode;
use jbotci_syntax::{WithIndicators, elidable_terminator_for_absent_field};
use jbotci_tree::TreeVisitor;
use serde::{Deserialize, Serialize};

pub use render::{
    DEFAULT_GENTUFA_PNG_SCALE, EmbeddedGentufaFonts, GentufaExportError, GentufaFontData,
    GentufaPngOptions, GentufaSvgOptions, render_gentufa_blocks_png, render_gentufa_blocks_svg,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum GentufaScript {
    #[default]
    Latin,
    Cyrillic,
    Zbalermorna,
}

impl fmt::Display for GentufaScript {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Latin => "latin",
            Self::Cyrillic => "cyrillic",
            Self::Zbalermorna => "zbalermorna",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct WebSourceRange {
    pub byte_start: usize,
    pub byte_end: usize,
    pub char_start: usize,
    pub char_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct ReferenceLabel {
    pub stem: String,
    pub occurrence: Option<usize>,
    pub slot: Option<ReferenceSlotLabel>,
}

impl ReferenceLabel {
    #[requires(!stem.is_empty())]
    #[ensures(ret.stem == stem)]
    pub fn new(stem: &str, occurrence: Option<usize>, slot: Option<ReferenceSlotLabel>) -> Self {
        Self {
            stem: stem.to_owned(),
            occurrence,
            slot,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn base_key(&self) -> String {
        let mut key = self.stem.clone();
        if let Some(occurrence) = self.occurrence {
            key.push_str(&occurrence.to_string());
        }
        key
    }

    #[requires(true)]
    #[ensures(ret.starts_with(&self.base_key()))]
    pub fn full_key(&self) -> String {
        let mut key = self.base_key();
        if let Some(slot) = &self.slot {
            key.push('<');
            key.push_str(&slot.text());
            key.push('>');
        }
        key
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(::Numbered(_) => true)]
#[invariant(::Modal(_) => true)]
#[invariant(::Fai => true)]
pub enum ReferenceSlotLabel {
    Numbered(u8),
    Modal(Vec<String>),
    Fai,
}

impl ReferenceSlotLabel {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn text(&self) -> String {
        match self {
            Self::Numbered(place) => place.to_string(),
            Self::Modal(words) if words.is_empty() => "modal".to_owned(),
            Self::Modal(words) => words
                .iter()
                .map(|word| reference_label_plain_text(word))
                .collect::<Vec<_>>()
                .join(" "),
            Self::Fai => "fai".to_owned(),
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn reference_slot_display_text(slot: &ReferenceSlotLabel) -> String {
    math_sans_alphanumeric_text(&slot.text())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct GentufaBlocksLayout<Tooltip = ()> {
    pub blocks: Vec<GentufaBlock<Tooltip>>,
    pub max_col: usize,
    pub max_row: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct GentufaBlock<Tooltip = ()> {
    pub block_id: String,
    pub label: String,
    pub is_leaf: bool,
    pub is_elided: bool,
    pub token_kind: Option<String>,
    pub ref_markers: Vec<ReferenceMarker>,
    pub span: Option<WebSourceRange>,
    pub node_types: Vec<String>,
    pub ancestors: Vec<String>,
    pub col: usize,
    pub col_span: usize,
    pub row: usize,
    pub row_span: usize,
    pub color: String,
    pub parent_color: Option<String>,
    pub raw_text: String,
    pub display_text: String,
    pub transform: Option<TransformInfo>,
    pub glosses: Vec<String>,
    pub definition: Option<String>,
    pub computed_gloss: Option<String>,
    pub tooltip: Option<Tooltip>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct TransformInfo {
    pub source_text: String,
    pub target_text: String,
    pub group_key: Option<String>,
    pub output_index: usize,
    pub output_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum ReferenceMarkerRole {
    Reference,
    Referent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct ReferenceMarker {
    pub role: ReferenceMarkerRole,
    pub kind: String,
    pub label: ReferenceLabel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct GentufaBlockAnnotation<Tooltip = ()> {
    pub range: WebSourceRange,
    pub text: Option<String>,
    pub glosses: Vec<String>,
    pub definition: Option<String>,
    pub tooltip: Option<Tooltip>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct RenderedLeaf {
    pub range: WebSourceRange,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct ElidedTerminator {
    pub parent_id: RawSyntaxNodeId,
    pub range: WebSourceRange,
    pub dictionary_text: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct GentufaBlockOptions {
    pub script: GentufaScript,
    pub show_elided: bool,
    pub phonemes: PhonemeRenderOptions,
}

impl Default for GentufaBlockOptions {
    #[requires(true)]
    #[ensures(ret.script == GentufaScript::Latin)]
    fn default() -> Self {
        Self {
            script: GentufaScript::Latin,
            show_elided: false,
            phonemes: PhonemeRenderOptions::default(),
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub fn rendered_leaves(
    syntax: &TextSyntax,
    source: &str,
    options: &GentufaBlockOptions,
) -> Vec<RenderedLeaf> {
    let mut collector = LeafCollector::new(source, options);
    syntax.visit_in_order(&mut collector);
    collector.finish()
}

#[requires(true)]
#[ensures(true)]
pub fn elided_terminators(
    analysis: &ReferenceAnalysis<'_>,
    syntax: &TextSyntax,
    options: &GentufaBlockOptions,
) -> Vec<ElidedTerminator> {
    if !options.show_elided {
        return Vec::new();
    }
    let mut collector = ElidedTerminatorCollector::new(analysis, options);
    syntax.visit_in_order(&mut collector);
    collector.finish()
}

#[requires(true)]
#[ensures(ret.max_col >= ret.blocks.iter().map(|block| block.col + block.col_span).max().unwrap_or(0))]
pub fn blocks_layout<Tooltip: Clone>(
    analysis: &ReferenceAnalysis<'_>,
    reference_model: &ReferenceDisplayModel,
    source: &str,
    leaves: &[RenderedLeaf],
    elided_terminators: &[ElidedTerminator],
    annotations: &[GentufaBlockAnnotation<Tooltip>],
    options: &GentufaBlockOptions,
) -> GentufaBlocksLayout<Tooltip> {
    let child_map = syntax_child_map(analysis);
    let root_id = analysis.syntax_index.root().0;
    let Some(root) = build_block_tree_node(
        analysis,
        reference_model,
        &child_map,
        root_id,
        source,
        leaves,
        elided_terminators,
        annotations,
        options,
    ) else {
        return GentufaBlocksLayout {
            blocks: Vec::new(),
            max_col: 0,
            max_row: 0,
        };
    };
    let root = collapse_safe_multi_child_parents(collapse_single_child_chains(root));
    let mut root = root;
    assign_tree_depths_and_ancestors(&mut root);
    let max_depth = block_tree_max_depth(&root);
    let mut temp_blocks = Vec::new();
    let max_col = push_positioned_blocks(&root, 0, max_depth, None, &mut temp_blocks);
    let blocks = annotate_blocks(assign_block_colors(temp_blocks, max_depth), annotations);
    GentufaBlocksLayout {
        blocks,
        max_col,
        max_row: max_depth + 1,
    }
}

#[requires(true)]
#[ensures(true)]
pub fn display_text_for_spans(
    spans: &[SourceSpan],
    leaves: &[RenderedLeaf],
    source: &str,
    options: &GentufaBlockOptions,
) -> String {
    spans
        .iter()
        .map(|span| {
            let range = range_from_span(span);
            leaves
                .iter()
                .find(|leaf| leaf.range == range)
                .map(|leaf| leaf.text.clone())
                .unwrap_or_else(|| {
                    render_loose_latin_surface(source_text_for_range(source, Some(range)), options)
                })
        })
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[requires(true)]
#[ensures(ret.is_none_or(|range| range.byte_start <= range.byte_end && range.char_start <= range.char_end))]
pub fn range_from_spans<'a, I>(spans: I) -> Option<WebSourceRange>
where
    I: IntoIterator<Item = &'a SourceSpan>,
{
    let mut iter = spans.into_iter();
    let first = iter.next()?;
    let mut range = range_from_span(first);
    for span in iter {
        range.byte_start = range.byte_start.min(span.byte_start);
        range.byte_end = range.byte_end.max(span.byte_end);
        range.char_start = range.char_start.min(span.char_start);
        range.char_end = range.char_end.max(span.char_end);
    }
    Some(range)
}

#[requires(span.byte_start <= span.byte_end)]
#[requires(span.char_start <= span.char_end)]
#[ensures(ret.byte_start == span.byte_start)]
pub fn range_from_span(span: &SourceSpan) -> WebSourceRange {
    WebSourceRange {
        byte_start: span.byte_start,
        byte_end: span.byte_end,
        char_start: span.char_start,
        char_end: span.char_end,
    }
}

#[requires(true)]
#[ensures(ret.chars().count() >= stem.chars().count())]
pub fn math_alphanumeric_stem(stem: &str) -> String {
    let mut output = String::new();
    for ch in stem.chars() {
        push_math_alphanumeric_char(&mut output, ch);
    }
    output
}

#[requires(true)]
#[ensures(ret.chars().count() >= text.chars().count())]
pub fn math_sans_alphanumeric_text(text: &str) -> String {
    text.chars()
        .map(|ch| math_sans_alphanumeric_ascii_char(ch).unwrap_or(ch))
        .collect()
}

#[requires(true)]
#[ensures(!ret.chars().any(is_reference_stem_combining_mark))]
pub fn reference_label_plain_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for ch in text.chars() {
        if is_reference_stem_combining_mark(ch) {
            continue;
        }
        output.push(normalized_reference_stem_char(ch).unwrap_or(ch));
    }
    output
}

#[derive(Debug)]
#[invariant(true)]
struct LeafCollector<'source, 'options> {
    source: &'source str,
    options: &'options GentufaBlockOptions,
    leaves: Vec<RenderedLeaf>,
}

impl<'source, 'options> LeafCollector<'source, 'options> {
    #[requires(true)]
    #[ensures(ret.source == source)]
    fn new(source: &'source str, options: &'options GentufaBlockOptions) -> Self {
        Self {
            source,
            options,
            leaves: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> Vec<RenderedLeaf> {
        self.leaves
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_word_like(&mut self, word_like: &WordLike) {
        if let Some(range) = range_from_spans(word_like.source_spans()) {
            self.leaves.push(RenderedLeaf {
                range,
                text: render_word_like(word_like, self.source, self.options),
            });
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_with_indicators(&mut self, value: &WithIndicators<WordLike>) {
        match value {
            WithIndicators::Bare(word_like) => self.push_word_like(word_like),
            WithIndicators::Emphasized { bahe, word_like } => {
                self.push_word(bahe);
                self.push_word_like(word_like);
            }
            WithIndicators::WithIndicator {
                base,
                indicator,
                nai,
            } => {
                self.push_with_indicators(base);
                self.push_word(indicator);
                if let Some(nai) = nai {
                    self.push_word(nai);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_word(&mut self, word: &Word) {
        self.leaves.push(RenderedLeaf {
            range: range_from_span(word.span()),
            text: render_word(word, self.options),
        });
    }
}

impl<'source, 'options, 'tree> TreeVisitor<'tree> for LeafCollector<'source, 'options> {
    type Node = SyntaxNodeRef<'tree>;
    type Atom = SyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        match atom {
            SyntaxAtomRef::Token(token) => self.push_with_indicators(token.as_indicators()),
            SyntaxAtomRef::Word(word) => self.push_word(word),
        }
    }
}

#[derive(Debug)]
#[invariant(true)]
struct ElidedTerminatorCollector<'analysis, 'options, 'tree> {
    analysis: &'analysis ReferenceAnalysis<'tree>,
    options: &'options GentufaBlockOptions,
    node_stack: Vec<RawSyntaxNodeId>,
    last_position: Option<RenderedPosition>,
    terminators: Vec<ElidedTerminator>,
}

impl<'analysis, 'options, 'tree> ElidedTerminatorCollector<'analysis, 'options, 'tree> {
    #[requires(true)]
    #[ensures(ret.terminators.is_empty())]
    fn new(
        analysis: &'analysis ReferenceAnalysis<'tree>,
        options: &'options GentufaBlockOptions,
    ) -> Self {
        Self {
            analysis,
            options,
            node_stack: Vec::new(),
            last_position: None,
            terminators: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> Vec<ElidedTerminator> {
        self.terminators
    }
}

impl<'analysis, 'options, 'tree> TreeVisitor<'tree>
    for ElidedTerminatorCollector<'analysis, 'options, 'tree>
{
    type Node = SyntaxNodeRef<'tree>;
    type Atom = SyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        if let Some(id) = self.analysis.syntax_index.id_of(node) {
            self.node_stack.push(id);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, node: Self::Node) {
        if self.analysis.syntax_index.id_of(node).is_some() {
            self.node_stack.pop();
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        self.last_position = syntax_atom_end_position(atom);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_absent_optional_field(&mut self, field: jbotci_tree::FieldRef) {
        let Some(parent_id) = self.node_stack.last().copied() else {
            return;
        };
        let Some(parent_node) = self.analysis.syntax_index.node(parent_id) else {
            return;
        };
        let Some(cmavo) = elidable_terminator_for_absent_field(parent_node, field) else {
            return;
        };
        let Some(position) = self.last_position.clone() else {
            return;
        };
        self.terminators.push(ElidedTerminator {
            parent_id,
            range: WebSourceRange {
                byte_start: position.byte_end,
                byte_end: position.byte_end,
                char_start: position.char_end,
                char_end: position.char_end,
            },
            dictionary_text: cmavo.canonical_text().to_owned(),
            text: render_elided_cmavo(cmavo, self.options),
        });
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
#[ensures(!ret.is_empty())]
fn render_elided_cmavo(cmavo: Cmavo, options: &GentufaBlockOptions) -> String {
    let text = Phonemes::from_canonical(cmavo.canonical_text().to_owned())
        .expect("cmavo canonical text is valid phoneme text")
        .render(options.phonemes);
    render_latin_surface(options.script, WordKind::Cmavo, &text)
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct BlockTreeNode {
    id: RawSyntaxNodeId,
    label: String,
    is_elided: bool,
    token_kind: Option<String>,
    ref_markers: Vec<ReferenceMarker>,
    span: Option<WebSourceRange>,
    source_spans: Vec<SourceSpan>,
    leaf_parts: Vec<BlockLeafPart>,
    node_types: Vec<String>,
    ancestors: Vec<String>,
    depth: usize,
    raw_text: String,
    leaf_word: Option<String>,
    computed_gloss: Option<String>,
    children: Vec<BlockTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct BlockLeafPart {
    id: RawSyntaxNodeId,
    range: WebSourceRange,
    is_elided: bool,
    raw_text: String,
    display_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Node(_) => true)]
#[invariant(::Leaf(_) => true)]
enum BlockLayoutChild<'a> {
    Node(&'a BlockTreeNode),
    Leaf(&'a BlockLeafPart),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct BlockTemp<Tooltip> {
    id: RawSyntaxNodeId,
    parent_id: Option<RawSyntaxNodeId>,
    child_ids: Vec<RawSyntaxNodeId>,
    block: GentufaBlock<Tooltip>,
}

#[requires(true)]
#[ensures(ret.len() == analysis.syntax_index.node_count())]
fn syntax_child_map(analysis: &ReferenceAnalysis<'_>) -> Vec<Vec<RawSyntaxNodeId>> {
    let mut child_map = vec![Vec::new(); analysis.syntax_index.node_count()];
    for raw_id in 0..analysis.syntax_index.node_count() {
        let id = RawSyntaxNodeId(raw_id);
        let Some(metadata) = analysis.syntax_index.metadata(id) else {
            continue;
        };
        if let Some(parent) = metadata.parent
            && let Some(children) = child_map.get_mut(parent.0)
        {
            children.push(id);
        }
    }
    child_map
}

#[requires(true)]
#[ensures(true)]
fn build_block_tree_node<Tooltip: Clone>(
    analysis: &ReferenceAnalysis<'_>,
    reference_model: &ReferenceDisplayModel,
    child_map: &[Vec<RawSyntaxNodeId>],
    id: RawSyntaxNodeId,
    source: &str,
    leaves: &[RenderedLeaf],
    elided_terminators: &[ElidedTerminator],
    annotations: &[GentufaBlockAnnotation<Tooltip>],
    options: &GentufaBlockOptions,
) -> Option<BlockTreeNode> {
    let metadata = analysis.syntax_index.metadata(id)?;
    let children = child_map
        .get(id.0)
        .into_iter()
        .flatten()
        .filter_map(|child| {
            build_block_tree_node(
                analysis,
                reference_model,
                child_map,
                *child,
                source,
                leaves,
                elided_terminators,
                annotations,
                options,
            )
        })
        .collect::<Vec<_>>();
    let span = range_from_spans(metadata.source_spans.iter());
    let label = analysis
        .syntax_index
        .node(id)
        .map(|node| syntax_constructor_name(node.constructor_name()).to_owned())
        .unwrap_or_else(|| "Node".to_owned());
    let leaf_parts = block_leaf_parts(
        analysis.syntax_index.node_count(),
        id,
        metadata,
        source,
        leaves,
        elided_terminators,
        options,
    );
    if span.is_none() && children.is_empty() && leaf_parts.is_empty() {
        return None;
    }
    let display_text = leaf_parts
        .iter()
        .map(|part| part.display_text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let leaf_word = if children.is_empty() && leaf_parts.len() == 1 && !display_text.is_empty() {
        Some(display_text.clone())
    } else {
        None
    };
    Some(BlockTreeNode {
        id,
        label: label.clone(),
        is_elided: false,
        token_kind: leaf_word.as_deref().and_then(token_kind_for_text),
        ref_markers: reference_markers_for_node(reference_model, id),
        span,
        source_spans: metadata.source_spans.clone(),
        leaf_parts,
        node_types: vec![label],
        ancestors: Vec::new(),
        depth: 0,
        raw_text: source_text_for_range(source, span),
        leaf_word,
        computed_gloss: annotation_for_range_and_text(annotations, span, None)
            .and_then(|annotation| annotation.glosses.first().cloned()),
        children,
    })
}

#[requires(true)]
#[ensures(true)]
fn block_leaf_parts(
    node_count: usize,
    id: RawSyntaxNodeId,
    metadata: &SyntaxNodeMetadata,
    source: &str,
    leaves: &[RenderedLeaf],
    elided_terminators: &[ElidedTerminator],
    options: &GentufaBlockOptions,
) -> Vec<BlockLeafPart> {
    let mut parts = metadata
        .source_spans
        .iter()
        .enumerate()
        .filter_map(|(index, span)| {
            let range = range_from_span(span);
            let display_text = leaves
                .iter()
                .find(|leaf| leaf.range == range)
                .map(|leaf| leaf.text.clone())
                .unwrap_or_else(|| {
                    render_loose_latin_surface(source_text_for_range(source, Some(range)), options)
                });
            if display_text.is_empty() {
                return None;
            }
            Some(BlockLeafPart {
                id: synthetic_leaf_id(node_count, id, index),
                range,
                is_elided: false,
                raw_text: source_text_for_range(source, Some(range)),
                display_text,
            })
        })
        .collect::<Vec<_>>();
    let elided_offset = parts.len();
    parts.extend(
        elided_terminators
            .iter()
            .filter(|terminator| terminator.parent_id == id)
            .enumerate()
            .map(|(index, terminator)| BlockLeafPart {
                id: synthetic_leaf_id(node_count, id, elided_offset + index),
                range: terminator.range,
                is_elided: true,
                raw_text: String::new(),
                display_text: terminator.text.clone(),
            }),
    );
    parts.sort_by_key(|part| (part.range.byte_start, usize::from(part.is_elided)));
    parts
}

#[requires(true)]
#[ensures(ret.0 >= node_count)]
fn synthetic_leaf_id(node_count: usize, parent: RawSyntaxNodeId, index: usize) -> RawSyntaxNodeId {
    RawSyntaxNodeId(
        node_count
            .saturating_add(parent.0.saturating_add(1).saturating_mul(1_000_000))
            .saturating_add(index),
    )
}

#[requires(true)]
#[ensures(true)]
fn collapse_single_child_chains(mut node: BlockTreeNode) -> BlockTreeNode {
    node.children = node
        .children
        .into_iter()
        .map(collapse_single_child_chains)
        .collect();
    if node.children.len() == 1 {
        let child = node.children.pop().expect("one child was checked above");
        if can_collapse_single_child(&node, &child) {
            return merge_parent_into_child(node, child);
        }
        node.children.push(child);
    }
    node
}

#[requires(true)]
#[ensures(true)]
fn can_collapse_single_child(parent: &BlockTreeNode, child: &BlockTreeNode) -> bool {
    parent.leaf_word.is_none()
        && parent.token_kind.is_none()
        && !parent.leaf_parts.iter().any(|part| part.is_elided)
        && spans_compatible(parent.span, child.span)
}

#[requires(true)]
#[ensures(true)]
fn spans_compatible(parent: Option<WebSourceRange>, child: Option<WebSourceRange>) -> bool {
    match (parent, child) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(parent), Some(child)) => parent == child,
    }
}

#[requires(true)]
#[ensures(true)]
fn merge_parent_into_child(parent: BlockTreeNode, mut child: BlockTreeNode) -> BlockTreeNode {
    let mut node_types = parent.node_types;
    extend_unique_strings(&mut node_types, child.node_types);
    let mut ref_markers = parent.ref_markers;
    extend_unique_ref_markers(&mut ref_markers, child.ref_markers);
    child.node_types = node_types;
    child.ref_markers = ref_markers;
    child.span = child.span.or(parent.span);
    child.source_spans = if child.source_spans.is_empty() {
        parent.source_spans
    } else {
        child.source_spans
    };
    child.leaf_parts = if child.leaf_parts.is_empty() {
        parent.leaf_parts
    } else {
        child.leaf_parts
    };
    if child.raw_text.is_empty() {
        child.raw_text = parent.raw_text;
    }
    child.leaf_word = child.leaf_word.or(parent.leaf_word);
    child.token_kind = child.token_kind.or(parent.token_kind);
    child.computed_gloss = child.computed_gloss.or(parent.computed_gloss);
    child.is_elided = child.is_elided || parent.is_elided;
    child
}

#[requires(true)]
#[ensures(true)]
fn extend_unique_strings(target: &mut Vec<String>, source: Vec<String>) {
    for item in source {
        if !target.iter().any(|existing| existing == &item) {
            target.push(item);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn extend_unique_ref_markers(target: &mut Vec<ReferenceMarker>, source: Vec<ReferenceMarker>) {
    for item in source {
        if !target.iter().any(|existing| existing == &item) {
            target.push(item);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collapse_safe_multi_child_parents(mut node: BlockTreeNode) -> BlockTreeNode {
    let mut children = Vec::new();
    for child in node.children {
        let child = collapse_safe_multi_child_parents(child);
        if should_collapse_safe_multi_child_parent(&child) {
            children.extend(child.children);
        } else {
            children.push(child);
        }
    }
    node.children = children;
    node
}

#[requires(true)]
#[ensures(true)]
fn should_collapse_safe_multi_child_parent(node: &BlockTreeNode) -> bool {
    node.children.len() > 1
        && node.leaf_parts.is_empty()
        && node.node_types.first().is_some_and(|node_type| {
            matches!(
                node_type.as_str(),
                "PredicateTail" | "PredicateTail1" | "PredicateTail2" | "Relation"
            )
        })
        && node.ref_markers.is_empty()
        && node.computed_gloss.is_none()
}

#[requires(true)]
#[ensures(true)]
fn assign_tree_depths_and_ancestors(root: &mut BlockTreeNode) {
    assign_tree_depths_and_ancestors_inner(root, 0, &mut Vec::new());
}

#[requires(true)]
#[ensures(node.depth == depth)]
fn assign_tree_depths_and_ancestors_inner(
    node: &mut BlockTreeNode,
    depth: usize,
    ancestors: &mut Vec<String>,
) {
    node.depth = depth;
    node.ancestors = ancestors.clone();
    ancestors.push(node.label.clone());
    for child in &mut node.children {
        assign_tree_depths_and_ancestors_inner(child, depth + 1, ancestors);
    }
    ancestors.pop();
}

#[requires(true)]
#[ensures(ret >= node.depth)]
fn block_tree_max_depth(node: &BlockTreeNode) -> usize {
    if node.children.is_empty() {
        return if node.leaf_parts.len() > 1 {
            node.depth + 1
        } else {
            node.depth
        };
    }
    let child_max = node
        .children
        .iter()
        .map(block_tree_max_depth)
        .max()
        .unwrap_or(node.depth);
    if has_uncovered_leaf_parts(node) {
        child_max.max(node.depth + 1)
    } else {
        child_max.max(node.depth)
    }
}

#[requires(true)]
#[ensures(ret >= col)]
fn push_positioned_blocks<Tooltip>(
    node: &BlockTreeNode,
    col: usize,
    max_depth: usize,
    parent_id: Option<RawSyntaxNodeId>,
    blocks: &mut Vec<BlockTemp<Tooltip>>,
) -> usize {
    if node.children.is_empty() {
        if node.leaf_parts.len() > 1 {
            return push_split_leaf_blocks(node, col, max_depth, parent_id, blocks);
        }
        push_leaf_or_structural_block(node, col, max_depth, parent_id, blocks);
        return col + 1;
    }
    let start_col = col;
    let mut next_col = col;
    let children = layout_children(node);
    let child_ids = children
        .iter()
        .map(|child| match child {
            BlockLayoutChild::Node(child) => child.id,
            BlockLayoutChild::Leaf(part) => part.id,
        })
        .collect::<Vec<_>>();
    for child in children {
        match child {
            BlockLayoutChild::Node(child) => {
                next_col =
                    push_positioned_blocks(child, next_col, max_depth, Some(node.id), blocks);
            }
            BlockLayoutChild::Leaf(part) => {
                let leaf_depth = node.depth + 1;
                blocks.push(BlockTemp {
                    id: part.id,
                    parent_id: Some(node.id),
                    child_ids: Vec::new(),
                    block: synthetic_leaf_block(
                        node,
                        part,
                        next_col,
                        leaf_depth,
                        max_depth.saturating_sub(leaf_depth) + 1,
                    ),
                });
                next_col += 1;
            }
        }
    }
    let col_span = next_col.saturating_sub(start_col).max(1);
    blocks.push(BlockTemp {
        id: node.id,
        parent_id,
        child_ids,
        block: block_from_tree_node(
            node,
            false,
            start_col,
            col_span,
            node.depth,
            1,
            node_display_text(node),
        ),
    });
    next_col
}

#[requires(true)]
#[ensures(true)]
fn layout_children(node: &BlockTreeNode) -> Vec<BlockLayoutChild<'_>> {
    let mut children = node
        .children
        .iter()
        .map(BlockLayoutChild::Node)
        .collect::<Vec<_>>();
    children.extend(
        node.leaf_parts
            .iter()
            .filter(|part| {
                part.is_elided
                    || !node
                        .children
                        .iter()
                        .any(|child| child_covers_part(child, part))
            })
            .map(BlockLayoutChild::Leaf),
    );
    children.sort_by_key(layout_child_sort_key);
    children
}

#[requires(true)]
#[ensures(true)]
fn has_uncovered_leaf_parts(node: &BlockTreeNode) -> bool {
    node.leaf_parts.iter().any(|part| {
        part.is_elided
            || !node
                .children
                .iter()
                .any(|child| child_covers_part(child, part))
    })
}

#[requires(true)]
#[ensures(true)]
fn child_covers_part(child: &BlockTreeNode, part: &BlockLeafPart) -> bool {
    child
        .span
        .is_some_and(|child_span| range_contains(child_span, part.range))
}

#[requires(container.byte_start <= container.byte_end)]
#[requires(part.byte_start <= part.byte_end)]
#[ensures(true)]
fn range_contains(container: WebSourceRange, part: WebSourceRange) -> bool {
    container.byte_start <= part.byte_start && part.byte_end <= container.byte_end
}

#[requires(true)]
#[ensures(true)]
fn layout_child_sort_key(child: &BlockLayoutChild<'_>) -> (usize, usize) {
    match child {
        BlockLayoutChild::Node(node) => node
            .span
            .map(|span| (span.byte_start, 1))
            .unwrap_or((usize::MAX, 1)),
        BlockLayoutChild::Leaf(part) => (part.range.byte_start, 0),
    }
}

#[requires(true)]
#[ensures(ret > col)]
fn push_split_leaf_blocks<Tooltip>(
    node: &BlockTreeNode,
    col: usize,
    max_depth: usize,
    parent_id: Option<RawSyntaxNodeId>,
    blocks: &mut Vec<BlockTemp<Tooltip>>,
) -> usize {
    let leaf_depth = node.depth + 1;
    let row_span = max_depth.saturating_sub(leaf_depth) + 1;
    for (offset, part) in node.leaf_parts.iter().enumerate() {
        blocks.push(BlockTemp {
            id: part.id,
            parent_id: Some(node.id),
            child_ids: Vec::new(),
            block: synthetic_leaf_block(node, part, col + offset, leaf_depth, row_span),
        });
    }
    let col_span = node.leaf_parts.len().max(1);
    blocks.push(BlockTemp {
        id: node.id,
        parent_id,
        child_ids: node.leaf_parts.iter().map(|part| part.id).collect(),
        block: block_from_tree_node(node, false, col, col_span, node.depth, 1, String::new()),
    });
    col + col_span
}

#[requires(row_span > 0)]
#[ensures(ret.is_leaf)]
fn synthetic_leaf_block<Tooltip>(
    node: &BlockTreeNode,
    part: &BlockLeafPart,
    col: usize,
    row: usize,
    row_span: usize,
) -> GentufaBlock<Tooltip> {
    GentufaBlock {
        block_id: format!("n{}", part.id.0),
        label: part.display_text.clone(),
        is_leaf: true,
        is_elided: part.is_elided,
        token_kind: token_kind_for_text(&part.display_text),
        ref_markers: Vec::new(),
        span: Some(part.range),
        node_types: node.node_types.clone(),
        ancestors: synthetic_leaf_ancestors(node),
        col,
        col_span: 1,
        row,
        row_span,
        color: String::new(),
        parent_color: None,
        raw_text: part.raw_text.clone(),
        display_text: part.display_text.clone(),
        transform: None,
        glosses: Vec::new(),
        definition: None,
        computed_gloss: None,
        tooltip: None,
    }
}

#[requires(true)]
#[ensures(true)]
fn synthetic_leaf_ancestors(node: &BlockTreeNode) -> Vec<String> {
    let mut ancestors = node.ancestors.clone();
    ancestors.push(node.label.clone());
    ancestors
}

#[requires(true)]
#[ensures(true)]
fn push_leaf_or_structural_block<Tooltip>(
    node: &BlockTreeNode,
    col: usize,
    max_depth: usize,
    parent_id: Option<RawSyntaxNodeId>,
    blocks: &mut Vec<BlockTemp<Tooltip>>,
) {
    if let [part] = node.leaf_parts.as_slice()
        && part.is_elided
    {
        blocks.push(BlockTemp {
            id: part.id,
            parent_id,
            child_ids: Vec::new(),
            block: synthetic_leaf_block(
                node,
                part,
                col,
                node.depth,
                max_depth.saturating_sub(node.depth) + 1,
            ),
        });
        return;
    }
    let is_leaf = node.leaf_word.is_some() && node.token_kind.is_some();
    let row_span = if is_leaf {
        max_depth.saturating_sub(node.depth) + 1
    } else {
        1
    };
    blocks.push(BlockTemp {
        id: node.id,
        parent_id,
        child_ids: Vec::new(),
        block: block_from_tree_node(
            node,
            is_leaf,
            col,
            1,
            node.depth,
            row_span,
            node_display_text(node),
        ),
    });
}

#[requires(true)]
#[ensures(true)]
fn node_display_text(node: &BlockTreeNode) -> String {
    node.leaf_word.clone().unwrap_or_default()
}

#[requires(col_span > 0)]
#[requires(row_span > 0)]
#[ensures(ret.col == col)]
fn block_from_tree_node<Tooltip>(
    node: &BlockTreeNode,
    is_leaf: bool,
    col: usize,
    col_span: usize,
    row: usize,
    row_span: usize,
    display_text: String,
) -> GentufaBlock<Tooltip> {
    GentufaBlock {
        block_id: format!("n{}", node.id.0),
        label: if is_leaf && !display_text.is_empty() {
            display_text.clone()
        } else {
            node.label.clone()
        },
        is_leaf,
        is_elided: node.is_elided,
        token_kind: node.token_kind.clone(),
        ref_markers: node.ref_markers.clone(),
        span: node.span,
        node_types: node.node_types.clone(),
        ancestors: node.ancestors.clone(),
        col,
        col_span,
        row,
        row_span,
        color: String::new(),
        parent_color: None,
        raw_text: node.raw_text.clone(),
        display_text,
        transform: None,
        glosses: Vec::new(),
        definition: None,
        computed_gloss: node.computed_gloss.clone(),
        tooltip: None,
    }
}

#[requires(true)]
#[ensures(true)]
fn assign_block_colors<Tooltip>(
    blocks: Vec<BlockTemp<Tooltip>>,
    max_depth: usize,
) -> Vec<GentufaBlock<Tooltip>> {
    let mut leaf_blocks = Vec::new();
    let mut nonleaf_blocks = Vec::new();
    for block in blocks {
        if block.block.is_leaf {
            leaf_blocks.push(block);
        } else {
            nonleaf_blocks.push(block);
        }
    }
    let parent_hues = leaf_parent_hues(&leaf_blocks);
    let mut hue_map = HashMap::new();
    let mut colored = Vec::with_capacity(leaf_blocks.len() + nonleaf_blocks.len());
    for mut block in leaf_blocks {
        let hue = parent_hues
            .iter()
            .find(|(parent, _)| *parent == block.parent_id)
            .map(|(_, hue)| *hue)
            .unwrap_or(0.0);
        block.block.color = hsl_to_hex(hue, 0.99, 0.85);
        hue_map.insert(block.id, (hue, block.block.col_span));
        colored.push(block.block);
    }
    nonleaf_blocks.sort_by_key(|block| Reverse(block.block.row));
    let mut nonleaf_colored = Vec::with_capacity(nonleaf_blocks.len());
    for mut block in nonleaf_blocks {
        let child_hues = block
            .child_ids
            .iter()
            .filter_map(|child_id| hue_map.get(child_id).copied())
            .collect::<Vec<_>>();
        let hue = weighted_circular_mean_hue(&child_hues).unwrap_or(0.0);
        let depth_ratio = if max_depth == 0 {
            0.0
        } else {
            block.block.row as f64 / max_depth as f64
        };
        let saturation = depth_ratio * 0.99;
        let lightness = 0.92 - saturation * 0.2;
        block.block.color = hsl_to_hex(hue, saturation, lightness);
        hue_map.insert(block.id, (hue, block.block.col_span));
        nonleaf_colored.push(block.block);
    }
    nonleaf_colored.reverse();
    colored.extend(nonleaf_colored);
    colored
}

#[requires(true)]
#[ensures(true)]
fn annotate_blocks<Tooltip: Clone>(
    blocks: Vec<GentufaBlock<Tooltip>>,
    annotations: &[GentufaBlockAnnotation<Tooltip>],
) -> Vec<GentufaBlock<Tooltip>> {
    blocks
        .into_iter()
        .map(|mut block| {
            let annotation = if block.is_elided {
                annotation_for_range_and_text(annotations, block.span, Some(&block.display_text))
            } else {
                annotation_for_range_and_text(annotations, block.span, None)
            };
            if let Some(annotation) = annotation {
                block.glosses = annotation.glosses.clone();
                block.definition = annotation.definition.clone();
                block.tooltip = annotation.tooltip.clone();
            }
            block
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn annotation_for_range_and_text<'a, Tooltip>(
    annotations: &'a [GentufaBlockAnnotation<Tooltip>],
    range: Option<WebSourceRange>,
    text: Option<&str>,
) -> Option<&'a GentufaBlockAnnotation<Tooltip>> {
    let range = range?;
    if let Some(text) = text {
        let exact = annotations.iter().find(|annotation| {
            annotation.range == range && annotation.text.as_deref() == Some(text)
        });
        if exact.is_some() || range.byte_start == range.byte_end {
            return exact;
        }
    }
    annotations
        .iter()
        .find(|annotation| annotation.range == range)
}

#[requires(true)]
#[ensures(true)]
fn leaf_parent_hues<Tooltip>(blocks: &[BlockTemp<Tooltip>]) -> Vec<(Option<RawSyntaxNodeId>, f64)> {
    let mut parents = Vec::new();
    for block in blocks {
        if !parents.iter().any(|parent| parent == &block.parent_id) {
            parents.push(block.parent_id);
        }
    }
    let count = parents.len();
    parents
        .into_iter()
        .enumerate()
        .map(|(index, parent)| {
            let hue = if count == 0 {
                0.0
            } else {
                360.0 * index as f64 / count as f64
            };
            (parent, hue)
        })
        .collect()
}

#[requires(true)]
#[ensures(ret.is_none_or(|hue| (0.0..360.0).contains(&hue)))]
fn weighted_circular_mean_hue(values: &[(f64, usize)]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut x = 0.0;
    let mut y = 0.0;
    for (hue, weight) in values {
        let radians = hue.to_radians();
        let weight = *weight as f64;
        x += radians.cos() * weight;
        y += radians.sin() * weight;
    }
    let mut degrees = y.atan2(x).to_degrees();
    if degrees < 0.0 {
        degrees += 360.0;
    }
    Some(degrees)
}

#[requires((0.0..=360.0).contains(&hue))]
#[requires((0.0..=1.0).contains(&saturation))]
#[requires((0.0..=1.0).contains(&lightness))]
#[ensures(ret.starts_with('#'))]
fn hsl_to_hex(hue: f64, saturation: f64, lightness: f64) -> String {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let hue_prime = hue / 60.0;
    let x = chroma * (1.0 - (hue_prime % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hue_prime {
        value if (0.0..1.0).contains(&value) => (chroma, x, 0.0),
        value if (1.0..2.0).contains(&value) => (x, chroma, 0.0),
        value if (2.0..3.0).contains(&value) => (0.0, chroma, x),
        value if (3.0..4.0).contains(&value) => (0.0, x, chroma),
        value if (4.0..5.0).contains(&value) => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let match_value = lightness - chroma / 2.0;
    format!(
        "#{:02x}{:02x}{:02x}",
        color_component_to_u8(r1 + match_value),
        color_component_to_u8(g1 + match_value),
        color_component_to_u8(b1 + match_value)
    )
}

#[requires((0.0..=1.0).contains(&value))]
#[ensures(true)]
fn color_component_to_u8(value: f64) -> u8 {
    (value * 255.0).round().clamp(0.0, 255.0) as u8
}

#[requires(true)]
#[ensures(true)]
pub fn reference_markers_for_node(
    reference_model: &ReferenceDisplayModel,
    id: RawSyntaxNodeId,
) -> Vec<ReferenceMarker> {
    let mut markers = Vec::new();
    let annotations = reference_model.annotations_for_syntax_ids(&[id]);
    for label in annotations.incoming {
        let label = reference_label_from_output(&label);
        markers.push(ReferenceMarker {
            role: ReferenceMarkerRole::Referent,
            kind: reference_kind_for_label(&label),
            label,
        });
    }
    for label in annotations.outgoing {
        let label = reference_label_from_output(&label);
        markers.push(ReferenceMarker {
            role: ReferenceMarkerRole::Reference,
            kind: reference_kind_for_label(&label),
            label,
        });
    }
    markers
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn reference_kind_for_label(label: &ReferenceLabel) -> String {
    if label.slot.is_some() {
        "argument".to_owned()
    } else {
        "reference".to_owned()
    }
}

#[requires(!label.stem.is_empty())]
#[ensures(ret.stem == label.stem)]
pub fn reference_label_from_output(label: &OutputReferenceName) -> ReferenceLabel {
    ReferenceLabel {
        stem: label.stem.clone(),
        occurrence: label.occurrence,
        slot: label.slot.as_ref().map(reference_slot_label_from_output),
    }
}

#[requires(true)]
#[ensures(true)]
pub fn reference_slot_label_from_output(slot: &OutputReferenceSlotName) -> ReferenceSlotLabel {
    match slot {
        OutputReferenceSlotName::Numbered(place) => ReferenceSlotLabel::Numbered(*place),
        OutputReferenceSlotName::Modal(words) => ReferenceSlotLabel::Modal(words.clone()),
        OutputReferenceSlotName::Fai => ReferenceSlotLabel::Fai,
    }
}

#[requires(true)]
#[ensures(true)]
fn token_kind_for_text(text: &str) -> Option<String> {
    if text.is_empty() {
        None
    } else {
        Some("word".to_owned())
    }
}

#[requires(true)]
#[ensures(!ret.ends_with("Syntax"))]
pub fn syntax_constructor_name(constructor: &'static str) -> &'static str {
    constructor.strip_suffix("Syntax").unwrap_or(constructor)
}

#[requires(true)]
#[ensures(true)]
fn source_text_for_range(source: &str, range: Option<WebSourceRange>) -> String {
    range
        .and_then(|range| source.get(range.byte_start..range.byte_end))
        .unwrap_or("")
        .to_owned()
}

#[requires(true)]
#[ensures(true)]
fn render_word_like(word_like: &WordLike, source: &str, options: &GentufaBlockOptions) -> String {
    match word_like.as_data() {
        WordLikeData::Bare(word) => render_word(word, options),
        WordLikeData::ZoQuote { zo, word } => {
            format!(
                "{} {}",
                render_word(zo, options),
                render_word(word, options)
            )
        }
        WordLikeData::ZoiQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        } => format!(
            "{} {} {} {}",
            render_word(zoi, options),
            render_word(opening_delimiter, options),
            source
                .get(quoted_text.span.byte_start..quoted_text.span.byte_end)
                .unwrap_or(&quoted_text.text),
            render_word(closing_delimiter, options)
        ),
        WordLikeData::LohuQuote {
            lohu,
            quoted_words,
            lehu,
        } => {
            let mut parts = Vec::with_capacity(quoted_words.len() + 2);
            parts.push(render_word(lohu, options));
            parts.extend(quoted_words.iter().map(|word| render_word(word, options)));
            parts.push(render_word(lehu, options));
            parts.join(" ")
        }
        WordLikeData::SingleWordQuote {
            marker,
            quoted_text,
        } => format!(
            "{} {}",
            render_word(marker, options),
            source
                .get(quoted_text.span.byte_start..quoted_text.span.byte_end)
                .unwrap_or(&quoted_text.text)
        ),
        WordLikeData::Letter { base, bu } => {
            format!(
                "{} {}",
                render_word_like(base, source, options),
                render_word(bu, options)
            )
        }
        WordLikeData::ZeiLujvo { left, zei, right } => format!(
            "{} {} {}",
            render_word_like(left, source, options),
            render_word(zei, options),
            render_word(right, options)
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn render_word(word: &Word, options: &GentufaBlockOptions) -> String {
    let latin = visible_latin_word_surface(word, options.phonemes);
    render_latin_surface(options.script, word.kind(), &latin)
}

#[requires(true)]
#[ensures(true)]
fn visible_latin_word_surface(word: &Word, options: PhonemeRenderOptions) -> String {
    let mut rendered = word.phonemes().render(options);
    if needs_leading_pause(word) {
        rendered.insert(0, '.');
    }
    if word.kind() == WordKind::Cmevla {
        rendered.push('.');
    }
    rendered
}

#[requires(true)]
#[ensures(true)]
fn needs_leading_pause(word: &Word) -> bool {
    word.kind() == WordKind::Cmevla
        || word
            .phonemes()
            .as_str()
            .chars()
            .next()
            .map(normalized_latin_char)
            .is_some_and(|ch| matches!(ch.base, 'a' | 'e' | 'i' | 'o' | 'u'))
}

#[requires(true)]
#[ensures(true)]
fn render_loose_latin_surface(text: String, options: &GentufaBlockOptions) -> String {
    match options.script {
        GentufaScript::Latin => text,
        GentufaScript::Cyrillic => latin_surface_to_cyrillic(&text),
        GentufaScript::Zbalermorna => latin_surface_to_zbalermorna(WordKind::Gismu, &text),
    }
}

#[requires(true)]
#[ensures(true)]
fn render_latin_surface(script: GentufaScript, kind: WordKind, latin: &str) -> String {
    match script {
        GentufaScript::Latin => latin.to_owned(),
        GentufaScript::Cyrillic => latin_surface_to_cyrillic(latin),
        GentufaScript::Zbalermorna => latin_surface_to_zbalermorna(kind, latin),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct NormalizedLatinChar {
    base: char,
    stressed: bool,
}

#[requires(true)]
#[ensures(true)]
fn normalized_latin_char(ch: char) -> NormalizedLatinChar {
    match ch {
        'á' | 'Á' => NormalizedLatinChar {
            base: 'a',
            stressed: true,
        },
        'é' | 'É' => NormalizedLatinChar {
            base: 'e',
            stressed: true,
        },
        'í' | 'Í' => NormalizedLatinChar {
            base: 'i',
            stressed: true,
        },
        'ó' | 'Ó' => NormalizedLatinChar {
            base: 'o',
            stressed: true,
        },
        'ú' | 'Ú' => NormalizedLatinChar {
            base: 'u',
            stressed: true,
        },
        'ý' | 'Ý' => NormalizedLatinChar {
            base: 'y',
            stressed: true,
        },
        'A' | 'E' | 'I' | 'O' | 'U' | 'Y' => NormalizedLatinChar {
            base: ch.to_ascii_lowercase(),
            stressed: true,
        },
        'ĭ' | 'Ĭ' => NormalizedLatinChar {
            base: 'ĭ',
            stressed: false,
        },
        'ŭ' | 'Ŭ' => NormalizedLatinChar {
            base: 'ŭ',
            stressed: false,
        },
        other => NormalizedLatinChar {
            base: other.to_ascii_lowercase(),
            stressed: false,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn latin_surface_to_cyrillic(text: &str) -> String {
    let mut output = String::new();
    for ch in text.chars() {
        let normalized = normalized_latin_char(ch);
        match normalized.base {
            '.' => output.push('.'),
            ',' => output.push(','),
            '\'' => {}
            'a' => push_cyrillic_vowel(&mut output, 'а', normalized.stressed),
            'e' => push_cyrillic_vowel(&mut output, 'е', normalized.stressed),
            'i' => push_cyrillic_vowel(&mut output, 'и', normalized.stressed),
            'o' => push_cyrillic_vowel(&mut output, 'о', normalized.stressed),
            'u' => push_cyrillic_vowel(&mut output, 'у', normalized.stressed),
            'y' => push_cyrillic_vowel(&mut output, 'ъ', normalized.stressed),
            'ĭ' => output.push('й'),
            'ŭ' => output.push('ў'),
            'b' => output.push('б'),
            'c' => output.push('ш'),
            'd' => output.push('д'),
            'f' => output.push('ф'),
            'g' => output.push('г'),
            'j' => output.push('ж'),
            'k' => output.push('к'),
            'l' => output.push('л'),
            'm' => output.push('м'),
            'n' => output.push('н'),
            'p' => output.push('п'),
            'r' => output.push('р'),
            's' => output.push('с'),
            't' => output.push('т'),
            'v' => output.push('в'),
            'x' => output.push('х'),
            'z' => output.push('з'),
            other => output.push(other),
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn push_cyrillic_vowel(output: &mut String, vowel: char, stressed: bool) {
    output.push(vowel);
    if stressed {
        output.push('\u{0301}');
    }
}

#[requires(true)]
#[ensures(true)]
fn latin_surface_to_zbalermorna(kind: WordKind, text: &str) -> String {
    let full_vowels = matches!(kind, WordKind::Fuhivla | WordKind::Cmevla);
    let mut output = String::new();
    for ch in text.chars() {
        let normalized = normalized_latin_char(ch);
        match normalized.base {
            '.' => output.push('\u{ed89}'),
            '\'' => output.push('\u{ed8a}'),
            ',' if full_vowels => output.push('\u{ed9a}'),
            ',' => {}
            'a' => push_zbalermorna_vowel(&mut output, 'a', full_vowels, normalized.stressed),
            'e' => push_zbalermorna_vowel(&mut output, 'e', full_vowels, normalized.stressed),
            'i' => push_zbalermorna_vowel(&mut output, 'i', full_vowels, normalized.stressed),
            'o' => push_zbalermorna_vowel(&mut output, 'o', full_vowels, normalized.stressed),
            'u' => push_zbalermorna_vowel(&mut output, 'u', full_vowels, normalized.stressed),
            'y' => push_zbalermorna_vowel(&mut output, 'y', full_vowels, normalized.stressed),
            'ĭ' => output.push('\u{edaa}'),
            'ŭ' => output.push('\u{edab}'),
            'b' => output.push('\u{ed90}'),
            'c' => output.push('\u{ed86}'),
            'd' => output.push('\u{ed91}'),
            'f' => output.push('\u{ed83}'),
            'g' => output.push('\u{ed92}'),
            'j' => output.push('\u{ed96}'),
            'k' => output.push('\u{ed82}'),
            'l' => output.push('\u{ed84}'),
            'm' => output.push('\u{ed87}'),
            'n' => output.push('\u{ed97}'),
            'p' => output.push('\u{ed80}'),
            'r' => output.push('\u{ed94}'),
            's' => output.push('\u{ed85}'),
            't' => output.push('\u{ed81}'),
            'v' => output.push('\u{ed93}'),
            'x' => output.push('\u{ed88}'),
            'z' => output.push('\u{ed95}'),
            other => output.push(other),
        }
    }
    output
}

#[requires(matches!(vowel, 'a' | 'e' | 'i' | 'o' | 'u' | 'y'))]
#[ensures(true)]
fn push_zbalermorna_vowel(output: &mut String, vowel: char, full: bool, stressed: bool) {
    let codepoint = match (full, vowel) {
        (false, 'a') => '\u{eda0}',
        (false, 'e') => '\u{eda1}',
        (false, 'i') => '\u{eda2}',
        (false, 'o') => '\u{eda3}',
        (false, 'u') => '\u{eda4}',
        (false, 'y') => '\u{eda5}',
        (true, 'a') => '\u{edb0}',
        (true, 'e') => '\u{edb1}',
        (true, 'i') => '\u{edb2}',
        (true, 'o') => '\u{edb3}',
        (true, 'u') => '\u{edb4}',
        (true, 'y') => '\u{edb5}',
        _ => unreachable!("requires Lojban vowel"),
    };
    output.push(codepoint);
    if stressed {
        output.push('\u{ed98}');
    }
}

#[requires(true)]
#[ensures(true)]
fn push_math_alphanumeric_char(output: &mut String, ch: char) {
    if is_reference_stem_combining_mark(ch) {
        return;
    }
    if let Some(base) = normalized_reference_stem_char(ch) {
        output.push(math_alphanumeric_ascii_char(base).unwrap_or(base));
    } else {
        output.push(math_alphanumeric_ascii_char(ch).unwrap_or(ch));
    }
}

#[requires(true)]
#[ensures(true)]
fn normalized_reference_stem_char(ch: char) -> Option<char> {
    match ch {
        'á' => Some('a'),
        'é' => Some('e'),
        'í' => Some('i'),
        'ó' => Some('o'),
        'ú' => Some('u'),
        'ý' => Some('y'),
        'Á' => Some('A'),
        'É' => Some('E'),
        'Í' => Some('I'),
        'Ó' => Some('O'),
        'Ú' => Some('U'),
        'Ý' => Some('Y'),
        'ĭ' => Some('i'),
        'ŭ' => Some('u'),
        'Ĭ' => Some('I'),
        'Ŭ' => Some('U'),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_reference_stem_combining_mark(ch: char) -> bool {
    matches!(ch, '\u{0301}' | '\u{0306}')
}

#[requires(true)]
#[ensures(true)]
fn math_alphanumeric_ascii_char(ch: char) -> Option<char> {
    const LOWER: [char; 26] = [
        '𝑎', '𝑏', '𝑐', '𝑑', '𝑒', '𝑓', '𝑔', 'ℎ', '𝑖', '𝑗', '𝑘', '𝑙', '𝑚', '𝑛', '𝑜', '𝑝', '𝑞', '𝑟',
        '𝑠', '𝑡', '𝑢', '𝑣', '𝑤', '𝑥', '𝑦', '𝑧',
    ];
    const UPPER: [char; 26] = [
        '𝐴', '𝐵', '𝐶', '𝐷', '𝐸', '𝐹', '𝐺', '𝐻', '𝐼', '𝐽', '𝐾', '𝐿', '𝑀', '𝑁', '𝑂', '𝑃', '𝑄', '𝑅',
        '𝑆', '𝑇', '𝑈', '𝑉', '𝑊', '𝑋', '𝑌', '𝑍',
    ];
    if ch.is_ascii_lowercase() {
        Some(LOWER[(ch as u8 - b'a') as usize])
    } else if ch.is_ascii_uppercase() {
        Some(UPPER[(ch as u8 - b'A') as usize])
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn math_sans_alphanumeric_ascii_char(ch: char) -> Option<char> {
    const LOWER: [char; 26] = [
        '𝖺', '𝖻', '𝖼', '𝖽', '𝖾', '𝖿', '𝗀', '𝗁', '𝗂', '𝗃', '𝗄', '𝗅', '𝗆', '𝗇', '𝗈', '𝗉', '𝗊', '𝗋',
        '𝗌', '𝗍', '𝗎', '𝗏', '𝗐', '𝗑', '𝗒', '𝗓',
    ];
    const UPPER: [char; 26] = [
        '𝖠', '𝖡', '𝖢', '𝖣', '𝖤', '𝖥', '𝖦', '𝖧', '𝖨', '𝖩', '𝖪', '𝖫', '𝖬', '𝖭', '𝖮', '𝖯', '𝖰', '𝖱',
        '𝖲', '𝖳', '𝖴', '𝖵', '𝖶', '𝖷', '𝖸', '𝖹',
    ];
    const DIGITS: [char; 10] = ['𝟢', '𝟣', '𝟤', '𝟥', '𝟦', '𝟧', '𝟨', '𝟩', '𝟪', '𝟫'];
    if ch.is_ascii_lowercase() {
        Some(LOWER[(ch as u8 - b'a') as usize])
    } else if ch.is_ascii_uppercase() {
        Some(UPPER[(ch as u8 - b'A') as usize])
    } else if ch.is_ascii_digit() {
        Some(DIGITS[(ch as u8 - b'0') as usize])
    } else {
        None
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
    fn reference_slot_text_removes_lojban_diacritics() {
        let slot = ReferenceSlotLabel::Modal(vec![
            "mléca".to_owned(),
            "be\u{301}rvi".to_owned(),
            "ta'i".to_owned(),
        ]);

        assert_eq!(slot.text(), "mleca bervi ta'i");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_slot_display_text_styles_all_slot_text() {
        assert_eq!(
            reference_slot_display_text(&ReferenceSlotLabel::Numbered(12)),
            "𝟣𝟤"
        );
        assert_eq!(reference_slot_display_text(&ReferenceSlotLabel::Fai), "𝖿𝖺𝗂");
        assert_eq!(
            reference_slot_display_text(&ReferenceSlotLabel::Modal(vec![
                "mléca".to_owned(),
                "be\u{301}rvi".to_owned(),
            ])),
            "𝗆𝗅𝖾𝖼𝖺 𝖻𝖾𝗋𝗏𝗂"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn childless_single_leaf_part_does_not_add_synthetic_depth() {
        let node = test_block_tree_node(6, 1);

        assert_eq!(block_tree_max_depth(&node), 6);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn childless_split_leaf_parts_add_synthetic_child_depth() {
        let node = test_block_tree_node(6, 2);

        assert_eq!(block_tree_max_depth(&node), 7);
    }

    #[requires(true)]
    #[ensures(ret.depth == depth)]
    fn test_block_tree_node(depth: usize, leaf_part_count: usize) -> BlockTreeNode {
        BlockTreeNode {
            id: RawSyntaxNodeId(depth),
            label: format!("node-{depth}"),
            is_elided: false,
            token_kind: None,
            ref_markers: Vec::new(),
            span: None,
            source_spans: Vec::new(),
            leaf_parts: test_leaf_parts(leaf_part_count),
            node_types: vec![format!("Node{depth}")],
            ancestors: Vec::new(),
            depth,
            raw_text: String::new(),
            leaf_word: None,
            computed_gloss: None,
            children: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(ret.len() == count)]
    fn test_leaf_parts(count: usize) -> Vec<BlockLeafPart> {
        (0..count)
            .map(|index| BlockLeafPart {
                id: RawSyntaxNodeId(index),
                range: WebSourceRange {
                    byte_start: index,
                    byte_end: index + 1,
                    char_start: index,
                    char_end: index + 1,
                },
                is_elided: false,
                raw_text: format!("w{index}"),
                display_text: format!("w{index}"),
            })
            .collect()
    }
}
