//! Shared gentufa block layout and SVG/PNG export support.

mod render;

use std::cmp::Reverse;
use std::collections::HashMap;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_morphology::{PhonemeRenderOptions, Word, WordKind, WordLike, WordLikeData};
pub use jbotci_orthography::{
    LojbanScript as GentufaScript, render_latin_word_surface_for_script,
    render_loose_latin_text_for_script,
};
pub use jbotci_output::{GlideMark, StressMark};
use jbotci_output::{
    ReferenceAnnotationSource, ReferenceAnnotationSourceData, ReferenceDisplayModel,
    ReferenceName as OutputReferenceName, ReferenceSlotName as OutputReferenceSlotName,
    RichReferenceAnnotation,
};
use jbotci_semantics::references::{GeneratedSyntaxIndex, RawSyntaxNodeId};
use jbotci_source::SourceSpan;
use jbotci_syntax::WithIndicators;
use jbotci_syntax::generated_model::{
    self, AtomRef as GeneratedSyntaxAtomRef, NodeRef as GeneratedSyntaxNodeRef,
    TextSyntax as GeneratedTextSyntax, TreeNode as GeneratedSyntaxTreeNode,
};
use jbotci_syntax::tree::Token;
use jbotci_tree::TreeVisitor;
use serde::{Deserialize, Serialize};

pub use render::{
    DEFAULT_GENTUFA_PNG_SCALE, EmbeddedGentufaFonts, GentufaExportError, GentufaFontData,
    GentufaPngOptions, GentufaSvgOptions, render_gentufa_blocks_png, render_gentufa_blocks_svg,
};

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
#[invariant(::PlaceQuestion => true)]
#[invariant(::Fai => true)]
pub enum ReferenceSlotLabel {
    Numbered(u8),
    Modal(Vec<String>),
    PlaceQuestion,
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
            Self::PlaceQuestion => "place-question".to_owned(),
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
pub struct GentufaBlocksLayout<Tooltip = (), ReferenceTooltip = ()> {
    pub blocks: Vec<GentufaBlock<Tooltip, ReferenceTooltip>>,
    pub max_col: usize,
    pub max_row: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct GentufaBlock<Tooltip = (), ReferenceTooltip = ()> {
    pub block_id: String,
    pub node_ids: Vec<usize>,
    pub label: String,
    pub is_leaf: bool,
    pub is_elided: bool,
    pub token_kind: Option<String>,
    pub ref_markers: Vec<ReferenceMarker<ReferenceTooltip>>,
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
pub struct ReferenceMarker<Tooltip = ()> {
    pub role: ReferenceMarkerRole,
    pub kind: String,
    pub label: ReferenceLabel,
    pub source: Option<ReferenceMarkerSource>,
    pub tooltip: Option<Tooltip>,
}

#[invariant(true)]
#[invariant(::PlaceFrame { display_word, lookup_word, .. } => !display_word.is_empty() && !lookup_word.is_empty())]
#[invariant(::PlaceAssignment { display_word, lookup_word, .. } => !display_word.is_empty() && !lookup_word.is_empty())]
#[invariant(::DiscourseEdge { display_word, lookup_word, .. } => !display_word.is_empty() && !lookup_word.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum ReferenceMarkerSource {
    PlaceFrame {
        frame: usize,
        source_node: usize,
        display_word: String,
        lookup_word: String,
    },
    PlaceAssignment {
        frame: usize,
        assignment: usize,
        source_node: usize,
        target_node: usize,
        display_word: String,
        lookup_word: String,
    },
    DiscourseEdge {
        edge: usize,
        source_node: usize,
        target_node: usize,
        display_word: String,
        lookup_word: String,
    },
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
#[ensures(ret.max_col >= ret.blocks.iter().map(|block| block.col + block.col_span).max().unwrap_or(0))]
pub fn generated_model_blocks_layout<Tooltip: Clone>(
    syntax: &GeneratedTextSyntax,
    source: &str,
    annotations: &[GentufaBlockAnnotation<Tooltip>],
    options: &GentufaBlockOptions,
) -> GentufaBlocksLayout<Tooltip> {
    generated_model_blocks_layout_with_references(syntax, source, None, None, annotations, options)
}

#[requires(true)]
#[ensures(ret.max_col >= ret.blocks.iter().map(|block| block.col + block.col_span).max().unwrap_or(0))]
pub fn generated_model_blocks_layout_with_references<Tooltip: Clone>(
    syntax: &GeneratedTextSyntax,
    source: &str,
    syntax_index: Option<&GeneratedSyntaxIndex<'_>>,
    reference_model: Option<&ReferenceDisplayModel>,
    annotations: &[GentufaBlockAnnotation<Tooltip>],
    options: &GentufaBlockOptions,
) -> GentufaBlocksLayout<Tooltip> {
    let mut collector =
        GeneratedBlockCollector::new(source, options, syntax_index, reference_model);
    syntax.visit_in_order(&mut collector);
    let Some(root) = collector.finish() else {
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

#[derive(Debug, Default)]
#[invariant(true)]
struct GeneratedBlockPayload {
    children: Vec<BlockTreeNode>,
    leaf_parts: Vec<BlockLeafPart>,
    source_spans: Vec<SourceSpan>,
}

impl GeneratedBlockPayload {
    #[requires(true)]
    #[ensures(true)]
    fn push_node(&mut self, node: BlockTreeNode) {
        self.source_spans.extend(node.source_spans.clone());
        self.children.push(node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_leaf_part(&mut self, part: BlockLeafPart, source_spans: Vec<SourceSpan>) {
        self.source_spans.extend(source_spans);
        self.leaf_parts.push(part);
    }

    #[requires(true)]
    #[ensures(true)]
    fn extend(&mut self, payload: GeneratedBlockPayload) {
        self.source_spans.extend(payload.source_spans);
        self.leaf_parts.extend(payload.leaf_parts);
        self.children.extend(payload.children);
    }
}

#[derive(Debug)]
#[invariant(true)]
struct GeneratedNodeFrame {
    id: RawSyntaxNodeId,
    label: String,
    ref_markers: Vec<ReferenceMarker>,
    payload: GeneratedBlockPayload,
}

#[derive(Debug)]
#[invariant(true)]
struct GeneratedFieldFrame {
    name: Option<&'static str>,
    payload: GeneratedBlockPayload,
}

#[derive(Debug)]
#[invariant(::Node(_) => true)]
#[invariant(::Field(_) => true)]
#[invariant(::Collection(_) => true)]
#[invariant(::Chain(_) => true)]
enum GeneratedBlockFrame {
    Node(GeneratedNodeFrame),
    Field(GeneratedFieldFrame),
    Collection(GeneratedBlockPayload),
    Chain(GeneratedBlockPayload),
}

impl GeneratedBlockFrame {
    #[requires(true)]
    #[ensures(true)]
    fn payload_mut(&mut self) -> &mut GeneratedBlockPayload {
        match self {
            Self::Node(frame) => &mut frame.payload,
            Self::Field(frame) => &mut frame.payload,
            Self::Collection(payload) | Self::Chain(payload) => payload,
        }
    }
}

#[derive(Debug)]
#[invariant(true)]
struct GeneratedBlockCollector<'source, 'options, 'index, 'tree> {
    source: &'source str,
    options: &'options GentufaBlockOptions,
    syntax_index: Option<&'index GeneratedSyntaxIndex<'tree>>,
    reference_model: Option<&'index ReferenceDisplayModel>,
    stack: Vec<GeneratedBlockFrame>,
    root: Option<BlockTreeNode>,
    next_id: usize,
}

impl<'source, 'options, 'index, 'tree> GeneratedBlockCollector<'source, 'options, 'index, 'tree> {
    #[requires(true)]
    #[ensures(ret.source == source)]
    fn new(
        source: &'source str,
        options: &'options GentufaBlockOptions,
        syntax_index: Option<&'index GeneratedSyntaxIndex<'tree>>,
        reference_model: Option<&'index ReferenceDisplayModel>,
    ) -> Self {
        Self {
            source,
            options,
            syntax_index,
            reference_model,
            stack: Vec::new(),
            root: None,
            next_id: syntax_index
                .map(GeneratedSyntaxIndex::node_count)
                .unwrap_or(0),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> Option<BlockTreeNode> {
        self.root
    }

    #[requires(true)]
    #[ensures(true)]
    fn allocate_id(&mut self) -> RawSyntaxNodeId {
        let id = RawSyntaxNodeId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn id_for_node(&mut self, node: GeneratedSyntaxNodeRef<'tree>) -> RawSyntaxNodeId {
        self.syntax_index
            .and_then(|index| index.id_of(node))
            .unwrap_or_else(|| self.allocate_id())
    }

    #[requires(true)]
    #[ensures(true)]
    fn reference_markers_for_id(&self, id: RawSyntaxNodeId) -> Vec<ReferenceMarker> {
        self.reference_model
            .map(|model| reference_markers_for_node(model, id))
            .unwrap_or_default()
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_payload(&mut self, payload: GeneratedBlockPayload) {
        if let Some(frame) = self.stack.last_mut() {
            frame.payload_mut().extend(payload);
            return;
        }
        if self.root.is_none() && payload.children.len() == 1 && payload.leaf_parts.is_empty() {
            self.root = payload.children.into_iter().next();
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_node(&mut self, node: BlockTreeNode) {
        if let Some(frame) = self.stack.last_mut() {
            frame.payload_mut().push_node(node);
        } else {
            self.root = Some(node);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_leaf_part(&mut self, part: BlockLeafPart, source_spans: Vec<SourceSpan>) {
        if let Some(frame) = self.stack.last_mut() {
            frame.payload_mut().push_leaf_part(part, source_spans);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_token(&mut self, token: &Token) {
        self.push_with_indicators(token.as_indicators());
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_with_indicators(&mut self, value: &WithIndicators<WordLike>) {
        match value {
            WithIndicators::Plain(word_like) => self.push_word_like(word_like),
            WithIndicators::Emphasized {
                bahe,
                extra_bahe,
                word_like,
            } => {
                self.push_word(bahe);
                for bahe in extra_bahe {
                    self.push_word(bahe);
                }
                self.push_word_like(word_like);
            }
            WithIndicators::WithIndicator {
                base,
                indicator_bahe,
                indicator,
                nai_bahe,
                nai,
            } => {
                self.push_with_indicators(base);
                for bahe in indicator_bahe {
                    self.push_word(bahe);
                }
                self.push_word(indicator);
                for bahe in nai_bahe {
                    self.push_word(bahe);
                }
                if let Some(nai) = nai {
                    self.push_word(nai);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_word_like(&mut self, word_like: &WordLike) {
        let span_refs = word_like.source_spans();
        let Some(range) = range_from_spans(span_refs.iter().copied()) else {
            return;
        };
        let source_spans = span_refs.into_iter().cloned().collect::<Vec<_>>();
        let id = self.allocate_id();
        self.push_leaf_part(
            BlockLeafPart {
                id,
                range,
                is_elided: false,
                raw_text: source_text_for_range(self.source, Some(range)),
                display_text: render_word_like(word_like, self.source, self.options),
            },
            source_spans,
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_word(&mut self, word: &Word) {
        let span = word.span().clone();
        let range = range_from_span(&span);
        let id = self.allocate_id();
        self.push_leaf_part(
            BlockLeafPart {
                id,
                range,
                is_elided: false,
                raw_text: source_text_for_range(self.source, Some(range)),
                display_text: render_word(word, self.options),
            },
            vec![span],
        );
    }
}

impl<'tree> TreeVisitor<'tree> for GeneratedBlockCollector<'_, '_, '_, 'tree> {
    type Node = GeneratedSyntaxNodeRef<'tree>;
    type Atom = GeneratedSyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        let id = self.id_for_node(node);
        let ref_markers = self.reference_markers_for_id(id);
        self.stack
            .push(GeneratedBlockFrame::Node(GeneratedNodeFrame {
                id,
                label: syntax_constructor_name(node.constructor_name()).to_owned(),
                ref_markers,
                payload: GeneratedBlockPayload::default(),
            }));
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        let Some(GeneratedBlockFrame::Node(frame)) = self.stack.pop() else {
            panic!("generated block collector exited a node without entering it");
        };
        let node = generated_block_tree_node_from_frame(frame, self.source);
        if let Some(node) = node {
            self.push_node(node);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: jbotci_tree::FieldRef) {
        self.stack
            .push(GeneratedBlockFrame::Field(GeneratedFieldFrame {
                name: field.name,
                payload: GeneratedBlockPayload::default(),
            }));
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: jbotci_tree::FieldRef) {
        let Some(GeneratedBlockFrame::Field(frame)) = self.stack.pop() else {
            panic!("generated block collector exited a field without entering it");
        };
        let mut payload = frame.payload;
        if let Some(name) = frame.name {
            for child in &mut payload.children {
                child.field_label = Some(name);
            }
        }
        self.push_payload(payload);
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.stack.push(GeneratedBlockFrame::Collection(
            GeneratedBlockPayload::default(),
        ));
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        let Some(GeneratedBlockFrame::Collection(payload)) = self.stack.pop() else {
            panic!("generated block collector exited a sequence without entering it");
        };
        self.push_payload(payload);
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_chain(&mut self) {
        self.stack
            .push(GeneratedBlockFrame::Chain(GeneratedBlockPayload::default()));
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_chain(&mut self) {
        let Some(GeneratedBlockFrame::Chain(mut payload)) = self.stack.pop() else {
            panic!("generated block collector exited a chain without entering it");
        };
        payload.children =
            flatten_generated_chain_block_nodes(payload.children, self.source, &mut self.next_id);
        payload.source_spans = generated_block_source_spans(&payload.children, &payload.leaf_parts);
        self.push_payload(payload);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        match atom {
            GeneratedSyntaxAtomRef::Token(token) => self.push_token(token),
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_block_tree_node_from_frame(
    frame: GeneratedNodeFrame,
    source: &str,
) -> Option<BlockTreeNode> {
    let GeneratedNodeFrame {
        id,
        label,
        ref_markers,
        payload,
    } = frame;
    generated_block_tree_node_from_parts(
        id,
        None,
        vec![id],
        label.clone(),
        false,
        None,
        ref_markers,
        vec![label],
        payload.children,
        payload.leaf_parts,
        source,
        None,
    )
}

#[requires(!label.is_empty())]
#[ensures(true)]
fn generated_block_tree_node_from_parts(
    id: RawSyntaxNodeId,
    field_label: Option<&'static str>,
    node_ids: Vec<RawSyntaxNodeId>,
    label: String,
    is_elided: bool,
    token_kind: Option<String>,
    ref_markers: Vec<ReferenceMarker>,
    node_types: Vec<String>,
    mut children: Vec<BlockTreeNode>,
    mut leaf_parts: Vec<BlockLeafPart>,
    source: &str,
    computed_gloss: Option<String>,
) -> Option<BlockTreeNode> {
    leaf_parts.sort_by_key(|part| (part.range.byte_start, usize::from(part.is_elided)));
    let source_spans = generated_block_source_spans(&children, &leaf_parts);
    let span = range_from_spans(source_spans.iter());
    if span.is_none() && children.is_empty() && leaf_parts.is_empty() {
        return None;
    }
    children.sort_by_key(|child| child.span.map(|span| span.byte_start).unwrap_or(usize::MAX));
    let display_text = leaf_parts
        .iter()
        .map(|part| part.display_text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let leaf_word = if children.is_empty() && leaf_parts.len() == 1 && !display_text.is_empty() {
        Some(display_text)
    } else {
        None
    };
    Some(BlockTreeNode {
        id,
        field_label,
        node_ids,
        label,
        is_elided,
        token_kind: leaf_word
            .as_deref()
            .and_then(token_kind_for_text)
            .or(token_kind),
        ref_markers,
        span,
        source_spans,
        leaf_parts,
        node_types,
        ancestors: Vec::new(),
        depth: 0,
        raw_text: source_text_for_range(source, span),
        leaf_word,
        computed_gloss,
        children,
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_block_source_spans(
    children: &[BlockTreeNode],
    leaf_parts: &[BlockLeafPart],
) -> Vec<SourceSpan> {
    let mut spans = Vec::new();
    for child in children {
        spans.extend(child.source_spans.clone());
    }
    spans.extend(
        leaf_parts
            .iter()
            .map(|part| source_span_from_range(part.range)),
    );
    spans
}

#[requires(range.byte_start <= range.byte_end)]
#[requires(range.char_start <= range.char_end)]
#[ensures(ret.byte_start == range.byte_start)]
fn source_span_from_range(range: WebSourceRange) -> SourceSpan {
    SourceSpan::new(
        None,
        range.byte_start,
        range.byte_end,
        range.char_start,
        range.char_end,
    )
    .expect("block ranges are ordered")
}

#[requires(true)]
#[ensures(true)]
fn flatten_generated_chain_block_nodes(
    nodes: Vec<BlockTreeNode>,
    source: &str,
    next_id: &mut usize,
) -> Vec<BlockTreeNode> {
    nodes
        .into_iter()
        .flat_map(|node| split_generated_chain_link_block_node(node, source, next_id))
        .collect()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn split_generated_chain_link_block_node(
    node: BlockTreeNode,
    source: &str,
    next_id: &mut usize,
) -> Vec<BlockTreeNode> {
    let original = node.clone();
    let Some(element_label) = generated_chain_link_element_field(&node.label) else {
        return vec![node];
    };
    let Some(element_index) = node
        .children
        .iter()
        .position(|child| child.field_label == Some(element_label))
    else {
        return vec![node];
    };
    let Some(element_span) = node.children[element_index].span else {
        return vec![node];
    };

    let mut prefix_children = Vec::new();
    let mut suffix_children = Vec::new();
    let mut element = None;
    for (index, child) in node.children.into_iter().enumerate() {
        if index == element_index {
            element = Some(child);
        } else if index < element_index {
            prefix_children.push(child);
        } else {
            suffix_children.push(child);
        }
    }

    let mut prefix_leaf_parts = Vec::new();
    let mut suffix_leaf_parts = Vec::new();
    for part in node.leaf_parts {
        if part.range.byte_end <= element_span.byte_start
            || part.range.byte_start < element_span.byte_start
        {
            prefix_leaf_parts.push(part);
        } else {
            suffix_leaf_parts.push(part);
        }
    }

    let element = element.expect("element index came from the children");
    let mut fragments = Vec::new();
    let mut original_identity_available = true;
    if let Some(prefix) = generated_chain_link_fragment_node(
        &original,
        next_id,
        &mut original_identity_available,
        prefix_children,
        prefix_leaf_parts,
        source,
    ) {
        fragments.push(prefix);
    }
    fragments.push(element);
    if let Some(suffix) = generated_chain_link_fragment_node(
        &original,
        next_id,
        &mut original_identity_available,
        suffix_children,
        suffix_leaf_parts,
        source,
    ) {
        fragments.push(suffix);
    }
    fragments
}

#[requires(true)]
#[ensures(true)]
fn generated_chain_link_fragment_node(
    original: &BlockTreeNode,
    next_id: &mut usize,
    original_identity_available: &mut bool,
    children: Vec<BlockTreeNode>,
    leaf_parts: Vec<BlockLeafPart>,
    source: &str,
) -> Option<BlockTreeNode> {
    if children.is_empty() && leaf_parts.is_empty() {
        return None;
    }
    let (id, node_ids, ref_markers, computed_gloss) = if *original_identity_available {
        *original_identity_available = false;
        (
            original.id,
            original.node_ids.clone(),
            original.ref_markers.clone(),
            original.computed_gloss.clone(),
        )
    } else {
        let id = allocate_generated_block_id(next_id);
        (id, vec![id], Vec::new(), None)
    };
    generated_block_tree_node_from_parts(
        id,
        original.field_label,
        node_ids,
        original.label.clone(),
        original.is_elided,
        original.token_kind.clone(),
        ref_markers,
        original.node_types.clone(),
        children,
        leaf_parts,
        source,
        computed_gloss,
    )
}

#[requires(true)]
#[ensures(true)]
fn allocate_generated_block_id(next_id: &mut usize) -> RawSyntaxNodeId {
    let id = RawSyntaxNodeId(*next_id);
    *next_id = (*next_id).saturating_add(1);
    id
}

#[requires(true)]
#[ensures(true)]
fn generated_chain_link_element_field(constructor: &str) -> Option<&'static str> {
    generated_model::GENERATED_MODEL_CHAIN_LINK_TREE_ELEMENT_FIELDS
        .iter()
        .find_map(|(link_constructor, element_label)| {
            (*link_constructor == constructor).then_some(*element_label)
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct BlockTreeNode {
    id: RawSyntaxNodeId,
    field_label: Option<&'static str>,
    node_ids: Vec<RawSyntaxNodeId>,
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
    let mut node_ids = parent.node_ids;
    extend_unique_node_ids(&mut node_ids, child.node_ids);
    let mut node_types = parent.node_types;
    extend_unique_strings(&mut node_types, child.node_types);
    let mut ref_markers = parent.ref_markers;
    extend_unique_ref_markers(&mut ref_markers, child.ref_markers);
    child.node_ids = node_ids;
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
fn extend_unique_node_ids(target: &mut Vec<RawSyntaxNodeId>, source: Vec<RawSyntaxNodeId>) {
    for item in source {
        if !target.iter().any(|existing| existing == &item) {
            target.push(item);
        }
    }
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
                "BridiTail" | "AfterthoughtBridiTail" | "BoGroupedBridiTail" | "Selbri"
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
        node_ids: node.node_ids.iter().map(|id| id.0).collect(),
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
        node_ids: node.node_ids.iter().map(|id| id.0).collect(),
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
    let rich_annotations = reference_model.rich_annotations_for_syntax_ids(&[id]);
    for label in annotations.incoming {
        let source = source_for_rich_annotation(&rich_annotations.incoming, &label);
        let label = reference_label_from_output(&label);
        markers.push(ReferenceMarker {
            role: ReferenceMarkerRole::Referent,
            kind: reference_kind_for_label(&label),
            label,
            source,
            tooltip: None,
        });
    }
    for label in annotations.outgoing {
        let source = source_for_rich_annotation(&rich_annotations.outgoing, &label);
        let label = reference_label_from_output(&label);
        markers.push(ReferenceMarker {
            role: ReferenceMarkerRole::Reference,
            kind: reference_kind_for_label(&label),
            label,
            source,
            tooltip: None,
        });
    }
    markers
}

#[requires(true)]
#[ensures(true)]
fn source_for_rich_annotation(
    annotations: &[RichReferenceAnnotation],
    label: &OutputReferenceName,
) -> Option<ReferenceMarkerSource> {
    annotations
        .iter()
        .find(|annotation| &annotation.name == label)
        .and_then(|annotation| reference_marker_source_from_output(&annotation.source))
}

#[requires(true)]
#[ensures(true)]
fn reference_marker_source_from_output(
    source: &ReferenceAnnotationSource,
) -> Option<ReferenceMarkerSource> {
    match source.as_data() {
        data!(ReferenceAnnotationSource::PlaceFrame {
            frame,
            source_node,
            display_word,
            lookup_word,
        }) => Some(new!(ReferenceMarkerSource::PlaceFrame {
            frame: frame.0,
            source_node: source_node.0,
            display_word: display_word.clone(),
            lookup_word: lookup_word.clone(),
        })),
        data!(ReferenceAnnotationSource::PlaceAssignment {
            frame,
            assignment,
            source_node,
            target_node,
            display_word,
            lookup_word,
        }) => Some(new!(ReferenceMarkerSource::PlaceAssignment {
            frame: frame.0,
            assignment: assignment.0,
            source_node: source_node.0,
            target_node: target_node.0,
            display_word: display_word.clone(),
            lookup_word: lookup_word.clone(),
        })),
        data!(ReferenceAnnotationSource::DiscourseEdge {
            edge,
            source_node,
            target_node,
            display_word,
            lookup_word,
            ..
        }) => Some(new!(ReferenceMarkerSource::DiscourseEdge {
            edge: edge.0,
            source_node: source_node.0,
            target_node: target_node.0,
            display_word: display_word.clone(),
            lookup_word: lookup_word.clone(),
        })),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn reference_kind_for_label(label: &ReferenceLabel) -> String {
    if label.slot.is_some() {
        "sumti".to_owned()
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
        OutputReferenceSlotName::PlaceQuestion => ReferenceSlotLabel::PlaceQuestion,
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
        WordLikeData::PlainWord(word) => render_word(word, options),
        WordLikeData::QuotedWord { zo, word } => {
            format!(
                "{} {}",
                render_word(zo, options),
                render_word(word, options)
            )
        }
        WordLikeData::DelimitedNonLojbanQuote {
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
        WordLikeData::QuotedWords {
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
        WordLikeData::DelimitedWordQuote {
            marker,
            quoted_text,
        } => format!(
            "{} {}",
            render_word(marker, options),
            source
                .get(quoted_text.span.byte_start..quoted_text.span.byte_end)
                .unwrap_or(&quoted_text.text)
        ),
        WordLikeData::LerfuWord { base, bu } => {
            format!(
                "{} {}",
                render_word_like(base, source, options),
                render_word(bu, options)
            )
        }
        WordLikeData::ZeiCompound { left, zei, right } => format!(
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
    render_latin_word_surface_for_script(options.script, word.kind(), &latin)
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
            .is_some_and(is_latin_vowel_surface_char)
}

#[requires(true)]
#[ensures(true)]
fn render_loose_latin_surface(text: String, options: &GentufaBlockOptions) -> String {
    render_loose_latin_text_for_script(options.script, &text)
}

#[requires(true)]
#[ensures(true)]
fn is_latin_vowel_surface_char(ch: char) -> bool {
    match ch {
        'a' | 'e' | 'i' | 'o' | 'u' | 'á' | 'é' | 'í' | 'ó' | 'ú' => true,
        other => matches!(
            other,
            'A' | 'E' | 'I' | 'O' | 'U' | 'Á' | 'É' | 'Í' | 'Ó' | 'Ú'
        ),
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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_chain_blocks_render_tanru_run_in_flat_source_order() {
        let layout = generated_test_blocks_layout("mi melbi je cmalu je blanu");

        assert_eq!(
            generated_leaf_display_texts(&layout),
            vec!["mi", "mélbi", "je", "cmálu", "je", "blánu"]
        );
        assert!(layout.blocks.iter().any(|block| block.label == "TanruUnit"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_chain_link_split_keeps_suffix_after_element_for_blocks() {
        let link = BlockTreeNode {
            id: RawSyntaxNodeId(1),
            field_label: None,
            node_ids: vec![RawSyntaxNodeId(1)],
            label: "BridiTailContinuation".to_owned(),
            is_elided: false,
            token_kind: None,
            ref_markers: Vec::new(),
            span: Some(test_range(0, 3)),
            source_spans: vec![source_span_from_range(test_range(0, 3))],
            leaf_parts: vec![
                test_leaf_part(2, "gi'e", test_range(0, 1)),
                test_leaf_part(3, "do", test_range(2, 3)),
            ],
            node_types: vec!["BridiTailContinuation".to_owned()],
            ancestors: Vec::new(),
            depth: 0,
            raw_text: String::new(),
            leaf_word: None,
            computed_gloss: None,
            children: vec![test_generated_block_node(
                4,
                "SelbriSimpleBridiTail",
                Some("bridi_tail"),
                test_range(1, 2),
                "cadzu",
            )],
        };
        let mut next_id = 10;

        let parts = split_generated_chain_link_block_node(link, "", &mut next_id);

        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].label, "BridiTailContinuation");
        assert_eq!(parts[0].leaf_parts[0].display_text, "gi'e");
        assert_eq!(parts[1].label, "SelbriSimpleBridiTail");
        assert_eq!(parts[2].label, "BridiTailContinuation");
        assert_eq!(parts[2].leaf_parts[0].display_text, "do");
    }

    #[requires(true)]
    #[ensures(ret.depth == depth)]
    fn test_block_tree_node(depth: usize, leaf_part_count: usize) -> BlockTreeNode {
        BlockTreeNode {
            id: RawSyntaxNodeId(depth),
            field_label: None,
            node_ids: vec![RawSyntaxNodeId(depth)],
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

    #[requires(!source.is_empty())]
    #[ensures(true)]
    fn generated_test_blocks_layout(source: &str) -> GentufaBlocksLayout {
        let words =
            jbotci_morphology::segment_words_with_modifiers(source).expect("valid morphology");
        let syntax = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            source,
            &jbotci_syntax::ParseOptions::default(),
        )
        .expect("valid generated syntax");
        generated_model_blocks_layout(
            &syntax,
            source,
            &Vec::<GentufaBlockAnnotation<()>>::new(),
            &GentufaBlockOptions::default(),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn generated_leaf_display_texts(layout: &GentufaBlocksLayout) -> Vec<String> {
        layout
            .blocks
            .iter()
            .filter(|block| block.is_leaf && !block.is_elided)
            .map(|block| block.display_text.clone())
            .collect()
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(ret.byte_start == byte_start)]
    fn test_range(byte_start: usize, byte_end: usize) -> WebSourceRange {
        WebSourceRange {
            byte_start,
            byte_end,
            char_start: byte_start,
            char_end: byte_end,
        }
    }

    #[requires(!display_text.is_empty())]
    #[requires(range.byte_start <= range.byte_end)]
    #[ensures(ret.display_text == display_text)]
    fn test_leaf_part(id: usize, display_text: &str, range: WebSourceRange) -> BlockLeafPart {
        BlockLeafPart {
            id: RawSyntaxNodeId(id),
            range,
            is_elided: false,
            raw_text: display_text.to_owned(),
            display_text: display_text.to_owned(),
        }
    }

    #[requires(!label.is_empty() && !display_text.is_empty())]
    #[requires(range.byte_start <= range.byte_end)]
    #[ensures(ret.label == label)]
    fn test_generated_block_node(
        id: usize,
        label: &str,
        field_label: Option<&'static str>,
        range: WebSourceRange,
        display_text: &str,
    ) -> BlockTreeNode {
        BlockTreeNode {
            id: RawSyntaxNodeId(id),
            field_label,
            node_ids: vec![RawSyntaxNodeId(id)],
            label: label.to_owned(),
            is_elided: false,
            token_kind: token_kind_for_text(display_text),
            ref_markers: Vec::new(),
            span: Some(range),
            source_spans: vec![source_span_from_range(range)],
            leaf_parts: vec![test_leaf_part(id + 100, display_text, range)],
            node_types: vec![label.to_owned()],
            ancestors: Vec::new(),
            depth: 0,
            raw_text: display_text.to_owned(),
            leaf_word: Some(display_text.to_owned()),
            computed_gloss: None,
            children: Vec::new(),
        }
    }
}
