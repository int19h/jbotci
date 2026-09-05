//! Shared gentufa block layout and SVG/PNG export support.

#![recursion_limit = "1024"]

mod compounds;
mod render;
pub use compounds::{
    AppliedGentufaCompound, GentufaCompoundExpectation, GentufaCompoundExpectationData,
    GentufaCompoundKind, GentufaCompoundLayout, GentufaCompoundMember,
    GentufaCompoundNonApplication, GentufaCompoundNonApplicationReason, GentufaCompoundSpec,
};
use compounds::{
    BlockLeafOrigin, BlockLeafOriginData, CompositeKind, node_compound_kind, part_compound_kind,
    rewrite_compounds,
};

use std::cmp::Reverse;
use std::collections::HashMap;
use std::num::NonZeroUsize;

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_invariant, invariant, new, requires};
use jbotci_morphology::{
    Cmavo, LeadingPauseContext, LeadingPauseVowelMode, NodeRef as MorphologyNodeRef,
    PhonemeRenderOptions, Phonemes, TreeNode as MorphologyTreeNode, Verbatim, Word, WordKind,
    WordLike, segment_words_with_modifiers, word_needs_leading_pause_in_context,
};
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
use jbotci_syntax::generated_model::recovered::{
    AtomRef as RecoveredSyntaxAtomRef, NodeRef as RecoveredSyntaxNodeRef,
    TextSyntax as RecoveredTextSyntax, TreeNode as RecoveredSyntaxTreeNode,
};
use jbotci_syntax::generated_model::{
    self, AtomRef as GeneratedSyntaxAtomRef, NodeRef as GeneratedSyntaxNodeRef,
    TextSyntax as GeneratedTextSyntax, TreeNode as GeneratedSyntaxTreeNode,
};
use jbotci_syntax::tree::Token;
use jbotci_syntax::{WithIndicators, WithIndicatorsData, elidable_terminator_for_absent_field_ref};
use jbotci_tree::{RecoveryItemState, RecoveryProjection, TreeVisitor};
use serde::{Deserialize, Serialize};

pub use render::{
    DEFAULT_GENTUFA_PNG_SCALE, EmbeddedGentufaFonts, GentufaExportError, GentufaFontData,
    GentufaPngOptions, GentufaSvgOptions, render_gentufa_blocks_png, render_gentufa_blocks_svg,
};

#[invariant(byte_start <= byte_end, "byte range must be ordered")]
#[invariant(char_start <= char_end, "character range must be ordered")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WebSourceRange {
    pub byte_start: usize,
    pub byte_end: usize,
    pub char_start: usize,
    pub char_end: usize,
}

#[invariant(!stem.is_empty(), "reference labels must have a non-empty stem")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ReferenceLabel {
    pub stem: String,
    pub occurrence: Option<NonZeroUsize>,
    pub slot: Option<ReferenceSlotLabel>,
}

impl ReferenceLabel {
    #[requires(true)]
    #[ensures(ret.stem == stem)]
    pub fn new(
        stem: &str,
        occurrence: Option<NonZeroUsize>,
        slot: Option<ReferenceSlotLabel>,
    ) -> Self {
        new!(ReferenceLabel {
            stem: stem.to_owned(),
            occurrence,
            slot,
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn base_key(&self) -> String {
        // Reference stems are digit-free normalized word stems, so appending a
        // decimal one-based occurrence preserves generated-label injectivity.
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

    #[requires(true)]
    #[ensures(ret == (self.stem == other.stem && self.occurrence == other.occurrence))]
    pub fn same_base(&self, other: &Self) -> bool {
        self.stem == other.stem && self.occurrence == other.occurrence
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

#[expensive_invariant(
    blocks.iter().all(|block| {
        block.col < *max_col
            && block.row < *max_row
            && block.col + block.col_span <= *max_col
            && block.row + block.row_span <= *max_row
    }),
    "all blocks must fit inside the declared layout grid"
)]
#[expensive_invariant(
    blocks.iter().all(|block| block.role.is_error() == block.error_index.is_some()),
    "only recovered error blocks carry diagnostic indices"
)]
#[expensive_invariant(blocks.iter().filter(|block| block.is_leaf && block.role.is_normal()).all(|block| block.row + block.row_span == *max_row), "normal leaves reach the bottom row")]
#[expensive_invariant(blocks.iter().enumerate().all(|(index, block)| blocks[index + 1..].iter().all(|other|
    block.row + block.row_span <= other.row || other.row + other.row_span <= block.row
        || block.col + block.col_span <= other.col || other.col + other.col_span <= block.col)), "block rectangles never overlap")]
// Together with containment and non-overlap, equal area means every grid cell
// is covered exactly once, without scanning every cell against every block.
#[expensive_invariant(
    max_col.checked_mul(*max_row).is_some_and(|grid_area| {
        blocks.iter().try_fold(0usize, |area, block| {
            area.checked_add(block.col_span.checked_mul(block.row_span)?)
        }) == Some(grid_area)
    }),
    "block rectangles must cover every grid cell exactly once"
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GentufaBlocksLayout<Tooltip = (), ReferenceTooltip = ()> {
    pub blocks: Vec<GentufaBlock<Tooltip, ReferenceTooltip>>,
    pub max_col: usize,
    pub max_row: usize,
}

#[invariant(*col_span > 0, "block column span must be positive")]
#[invariant(*row_span > 0, "block row span must be positive")]
#[invariant(
    span.as_ref().is_none_or(|span| {
        span.byte_start <= span.byte_end && span.char_start <= span.char_end
    }),
    "block source ranges must be ordered"
)]
#[invariant(
    role.is_error() == error_index.is_some(),
    "error blocks must carry exactly one diagnostic index"
)]
#[invariant(
    !role.is_error() || *is_leaf,
    "error blocks must be layout leaves"
)]
#[invariant(
    !role.is_error()
        || !raw_text.is_empty()
        || span.as_ref().is_some_and(|span| {
            span.byte_start == span.byte_end && span.char_start == span.char_end
        }),
    "empty error markers must have zero-width source ranges"
)]
#[invariant(compound_kind.is_none() || (*is_leaf && role.is_normal() && *col_span >= 2))]
#[invariant(match compound_kind {Some(GentufaCompoundKind::CmavoSequence) => *token_kind == Some(WordKind::Cmavo), Some(GentufaCompoundKind::Zei) => token_kind.is_none(), None => true})]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GentufaBlock<Tooltip = (), ReferenceTooltip = ()> {
    pub block_id: String,
    pub node_ids: Vec<usize>,
    pub label: String,
    pub is_leaf: bool,
    #[serde(default, skip_serializing_if = "GentufaBlockRole::is_normal")]
    pub role: GentufaBlockRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_index: Option<usize>,
    pub token_kind: Option<WordKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compound_kind: Option<GentufaCompoundKind>,
    pub ref_markers: Vec<ReferenceMarker<ReferenceTooltip>>,
    pub span: Option<WebSourceRange>,
    pub node_types: Vec<String>,
    pub ancestors: Vec<String>,
    pub col: usize,
    pub col_span: usize,
    pub row: usize,
    pub row_span: usize,
    pub color: String,
    pub raw_text: String,
    pub display_text: String,
    pub glosses: Vec<String>,
    pub definition: Option<String>,
    pub computed_gloss: Option<String>,
    pub tooltip: Option<Tooltip>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GentufaBlockRole {
    #[default]
    Normal,
    Elided,
    Error,
}

impl GentufaBlockRole {
    #[requires(true)]
    #[ensures(ret == matches!(self, Self::Normal))]
    pub const fn is_normal(&self) -> bool {
        matches!(self, Self::Normal)
    }

    #[requires(true)]
    #[ensures(ret == matches!(self, Self::Elided))]
    pub const fn is_elided(self) -> bool {
        matches!(self, Self::Elided)
    }

    #[requires(true)]
    #[ensures(ret == matches!(self, Self::Error))]
    pub const fn is_error(self) -> bool {
        matches!(self, Self::Error)
    }

    #[requires(true)]
    #[ensures(ret <= 2)]
    const fn sort_key(self) -> usize {
        match self {
            Self::Normal => 0,
            Self::Error => 1,
            Self::Elided => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum ReferenceMarkerRole {
    Reference,
    Referent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum ReferenceMarkerKind {
    Reference,
    Sumti,
}

impl ReferenceMarkerKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reference => "reference",
            Self::Sumti => "sumti",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct ReferenceMarker<Tooltip = ()> {
    pub role: ReferenceMarkerRole,
    pub kind: ReferenceMarkerKind,
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
    pub target: GentufaAnnotationTarget,
    pub glosses: Vec<String>,
    pub definition: Option<String>,
    pub tooltip: Option<Tooltip>,
}

#[invariant(::SourceRange { .. } => true)]
#[invariant(::Block { block_id, .. } => !block_id.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GentufaAnnotationTarget {
    SourceRange {
        range: WebSourceRange,
        text: Option<String>,
    },
    Block {
        block_id: String,
        range: WebSourceRange,
    },
}

impl<Tooltip> GentufaBlockAnnotation<Tooltip> {
    #[requires(true)]
    #[ensures(true)]
    pub fn range(&self) -> WebSourceRange {
        match self.target.as_data() {
            data!(GentufaAnnotationTarget::SourceRange { range, .. })
            | data!(GentufaAnnotationTarget::Block { range, .. }) => *range,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn text(&self) -> Option<&str> {
        match self.target.as_data() {
            data!(GentufaAnnotationTarget::SourceRange { text, .. }) => text.as_deref(),
            _ => None,
        }
    }
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
        GeneratedBlockCollector::<false>::new(source, options, syntax_index, reference_model);
    syntax.visit_in_order(&mut collector);
    finish_blocks_layout(collector, annotations, &[])
        .into_data()
        .layout
}

#[requires(true)]
#[ensures(ret.max_col >= ret.blocks.iter().map(|block| block.col + block.col_span).max().unwrap_or(0))]
#[ensures(ret.blocks.iter().all(|block| {
    block.error_index.is_none_or(|error_index| error_index < error_count)
}))]
pub fn recovered_generated_model_blocks_layout<Tooltip: Clone>(
    syntax: &RecoveredTextSyntax,
    source: &str,
    error_count: usize,
    annotations: &[GentufaBlockAnnotation<Tooltip>],
    options: &GentufaBlockOptions,
) -> GentufaBlocksLayout<Tooltip> {
    let mut collector = GeneratedBlockCollector::<true>::new(source, options, None, None);
    syntax.visit_in_order(&mut collector);
    finish_blocks_layout(collector, annotations, &[])
        .into_data()
        .layout
}

#[requires(true)]
#[ensures(true)]
pub fn generated_model_blocks_layout_with_compounds<Tooltip: Clone>(
    syntax: &GeneratedTextSyntax,
    source: &str,
    syntax_index: Option<&GeneratedSyntaxIndex<'_>>,
    reference_model: Option<&ReferenceDisplayModel>,
    annotations: &[GentufaBlockAnnotation<Tooltip>],
    options: &GentufaBlockOptions,
    specs: &[GentufaCompoundSpec],
) -> GentufaCompoundLayout<Tooltip> {
    let mut collector =
        GeneratedBlockCollector::<false>::new(source, options, syntax_index, reference_model);
    syntax.visit_in_order(&mut collector);
    finish_blocks_layout(collector, annotations, specs)
}

#[requires(true)]
#[ensures(ret.layout.blocks.iter().all(|block| block.error_index.is_none_or(|index| index < error_count)))]
pub fn recovered_generated_model_blocks_layout_with_compounds<Tooltip: Clone>(
    syntax: &RecoveredTextSyntax,
    source: &str,
    error_count: usize,
    annotations: &[GentufaBlockAnnotation<Tooltip>],
    options: &GentufaBlockOptions,
    specs: &[GentufaCompoundSpec],
) -> GentufaCompoundLayout<Tooltip> {
    let mut collector = GeneratedBlockCollector::<true>::new(source, options, None, None);
    syntax.visit_in_order(&mut collector);
    finish_blocks_layout(collector, annotations, specs)
}

#[requires(true)]
#[ensures(true)]
fn finish_blocks_layout<Tooltip: Clone, const RECOVERED: bool>(
    collector: GeneratedBlockCollector<'_, '_, '_, '_, RECOVERED>,
    annotations: &[GentufaBlockAnnotation<Tooltip>],
    specs: &[GentufaCompoundSpec],
) -> GentufaCompoundLayout<Tooltip> {
    let source = collector.source;
    let Some(root) = collector.finish() else {
        return new!(GentufaCompoundLayout {
            layout: new!(GentufaBlocksLayout {
                blocks: Vec::new(),
                max_col: 0,
                max_row: 0
            }),
            applied: Vec::new(),
            unapplied: specs
                .iter()
                .enumerate()
                .map(|(spec_index, _)| GentufaCompoundNonApplication {
                    spec_index,
                    reason: GentufaCompoundNonApplicationReason::MissingOrAmbiguousMember
                })
                .collect(),
        });
    };
    let (root, applied, unapplied) = if specs.is_empty() {
        (root, Vec::new(), Vec::new())
    } else {
        rewrite_compounds(root, source, specs)
    };
    let root = collapse_safe_multi_child_parents(collapse_single_child_chains(root));
    let root = assign_tree_depths_and_ancestors(root);
    let max_depth = block_tree_max_depth(&root);
    let mut temp_blocks = Vec::new();
    let max_col = push_positioned_blocks(&root, 0, max_depth, None, &mut temp_blocks);
    let blocks = annotate_blocks(assign_block_colors(temp_blocks, max_depth), annotations);
    new!(GentufaCompoundLayout {
        layout: new!(GentufaBlocksLayout {
            blocks,
            max_col,
            max_row: max_depth + 1
        }),
        applied,
        unapplied
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(|range| range.byte_start <= range.byte_end && range.char_start <= range.char_end))]
pub fn range_from_spans<'a, I>(spans: I) -> Option<WebSourceRange>
where
    I: IntoIterator<Item = &'a SourceSpan>,
{
    let mut iter = spans.into_iter();
    let first = iter.next()?;
    let mut byte_start = first.byte_start;
    let mut byte_end = first.byte_end;
    let mut char_start = first.char_start;
    let mut char_end = first.char_end;
    for span in iter {
        byte_start = byte_start.min(span.byte_start);
        byte_end = byte_end.max(span.byte_end);
        char_start = char_start.min(span.char_start);
        char_end = char_end.max(span.char_end);
    }
    Some(new!(WebSourceRange {
        byte_start,
        byte_end,
        char_start,
        char_end,
    }))
}

#[requires(span.byte_start <= span.byte_end)]
#[requires(span.char_start <= span.char_end)]
#[ensures(ret.byte_start == span.byte_start)]
pub fn range_from_span(span: &SourceSpan) -> WebSourceRange {
    new!(WebSourceRange {
        byte_start: span.byte_start,
        byte_end: span.byte_end,
        char_start: span.char_start,
        char_end: span.char_end,
    })
}

#[requires(true)]
#[ensures(ret.is_none_or(|range| range.byte_start <= range.byte_end && range.char_start <= range.char_end))]
fn merge_source_ranges(
    first: Option<WebSourceRange>,
    second: Option<WebSourceRange>,
) -> Option<WebSourceRange> {
    match (first, second) {
        (None, None) => None,
        (Some(range), None) | (None, Some(range)) => Some(range),
        (Some(first), Some(second)) => Some(new!(WebSourceRange {
            byte_start: first.byte_start.min(second.byte_start),
            byte_end: first.byte_end.max(second.byte_end),
            char_start: first.char_start.min(second.char_start),
            char_end: first.char_end.max(second.char_end),
        })),
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
    source_range: Option<WebSourceRange>,
}

impl GeneratedBlockPayload {
    #[requires(true)]
    #[ensures(true)]
    fn push_node(&mut self, node: BlockTreeNode) {
        self.source_range = merge_source_ranges(self.source_range, node.span);
        self.children.push(node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_leaf_part(&mut self, part: BlockLeafPart) {
        self.source_range = merge_source_ranges(self.source_range, Some(part.range));
        self.leaf_parts.push(part);
    }

    #[requires(true)]
    #[ensures(true)]
    fn extend(&mut self, payload: GeneratedBlockPayload) {
        self.source_range = merge_source_ranges(self.source_range, payload.source_range);
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

#[invariant(true)]
#[invariant(::Word { word, .. } => word.span().char_len() > 0)]
#[invariant(::Verbatim(verbatim) => verbatim.span.char_len() == verbatim.text.chars().count())]
#[derive(Debug, Clone, Copy)]
enum MorphologyBlockLeaf<'tree> {
    Word {
        word: &'tree Word,
        context: LeadingPauseContext,
    },
    Verbatim(&'tree Verbatim),
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
enum MorphologyBlockNode {
    Other,
    LerfuWord,
    PlainWord,
    BuLetterBasePlainWord,
}

#[invariant(fields.len() <= nodes.len(), "open morphology fields must belong to open nodes")]
#[derive(Debug, Default)]
struct MorphologyBlockLeafCollector<'tree> {
    leaves: Vec<MorphologyBlockLeaf<'tree>>,
    nodes: Vec<MorphologyBlockNode>,
    fields: Vec<jbotci_tree::FieldRef>,
}

impl<'tree> MorphologyBlockLeafCollector<'tree> {
    #[requires(true)]
    #[ensures(ret.nodes.is_empty() && ret.fields.is_empty() && ret.leaves.is_empty())]
    fn new() -> Self {
        Self::default()
    }

    #[requires(self.nodes.is_empty() && self.fields.is_empty())]
    #[ensures(ret.len() == old(self.leaves.len()))]
    fn finish(self) -> Vec<MorphologyBlockLeaf<'tree>> {
        self.into_data().leaves
    }
}

impl<'tree> TreeVisitor<'tree> for MorphologyBlockLeafCollector<'tree> {
    type Node = MorphologyNodeRef<'tree>;
    type Atom = jbotci_morphology::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(self.nodes.len() == old(self.nodes.len()) + 1)]
    fn enter_node(&mut self, node: Self::Node) {
        let mut data = std::mem::take(self).into_data();
        let node = match node {
            MorphologyNodeRef::WordLikeLerfuWord(_) => MorphologyBlockNode::LerfuWord,
            MorphologyNodeRef::WordLikePlainWord(_)
                if matches!(data.nodes.last(), Some(MorphologyBlockNode::LerfuWord))
                    && data
                        .fields
                        .last()
                        .is_some_and(|field| field.name == Some("base")) =>
            {
                MorphologyBlockNode::BuLetterBasePlainWord
            }
            MorphologyNodeRef::WordLikePlainWord(_) => MorphologyBlockNode::PlainWord,
            MorphologyNodeRef::WordCmavo(word)
            | MorphologyNodeRef::WordGismu(word)
            | MorphologyNodeRef::WordLujvo(word)
            | MorphologyNodeRef::WordFuhivla(word)
            | MorphologyNodeRef::WordCmevla(word) => {
                let context = if matches!(
                    data.nodes.last(),
                    Some(MorphologyBlockNode::BuLetterBasePlainWord)
                ) {
                    LeadingPauseContext::BuLetterBase
                } else {
                    LeadingPauseContext::IndependentWord
                };
                data.leaves
                    .push(new!(MorphologyBlockLeaf::Word { word, context }));
                MorphologyBlockNode::Other
            }
            MorphologyNodeRef::Verbatim(verbatim) => {
                data.leaves
                    .push(new!(MorphologyBlockLeaf::Verbatim(verbatim)));
                MorphologyBlockNode::Other
            }
            _ => MorphologyBlockNode::Other,
        };
        data.nodes.push(node);
        *self = Self::from_data(data);
    }

    #[requires(self.fields.len() < self.nodes.len())]
    #[ensures(self.nodes.len() + 1 == old(self.nodes.len()))]
    fn exit_node(&mut self, _node: Self::Node) {
        let mut data = std::mem::take(self).into_data();
        data.nodes.pop();
        *self = Self::from_data(data);
    }

    #[requires(self.fields.len() < self.nodes.len())]
    #[ensures(self.fields.len() == old(self.fields.len()) + 1)]
    fn enter_field(&mut self, field: jbotci_tree::FieldRef) {
        let mut data = std::mem::take(self).into_data();
        data.fields.push(field);
        *self = Self::from_data(data);
    }

    #[requires(self.fields.last().is_some_and(|entered| *entered == field))]
    #[ensures(self.fields.len() + 1 == old(self.fields.len()))]
    fn exit_field(&mut self, field: jbotci_tree::FieldRef) {
        let mut data = std::mem::take(self).into_data();
        data.fields.pop();
        *self = Self::from_data(data);
    }
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
struct GeneratedBlockCollector<'source, 'options, 'index, 'tree, const RECOVERED: bool> {
    source: &'source str,
    options: &'options GentufaBlockOptions,
    syntax_index: Option<&'index GeneratedSyntaxIndex<'tree>>,
    reference_model: Option<&'index ReferenceDisplayModel>,
    stack: Vec<GeneratedBlockFrame>,
    root: Option<BlockTreeNode>,
    next_id: usize,
    last_token_end_range: Option<WebSourceRange>,
    recovery_projection: RecoveryProjection,
}

impl<'source, 'options, 'index, 'tree, const RECOVERED: bool>
    GeneratedBlockCollector<'source, 'options, 'index, 'tree, RECOVERED>
{
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
            last_token_end_range: None,
            recovery_projection: RecoveryProjection::default(),
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

    #[requires(!label.is_empty())]
    #[ensures(self.stack.len() == old(self.stack.len()) + 1)]
    fn enter_node_frame(
        &mut self,
        id: RawSyntaxNodeId,
        label: String,
        ref_markers: Vec<ReferenceMarker>,
    ) {
        self.stack
            .push(GeneratedBlockFrame::Node(GeneratedNodeFrame {
                id,
                label,
                ref_markers,
                payload: GeneratedBlockPayload::default(),
            }));
    }

    #[requires(matches!(self.stack.last(), Some(GeneratedBlockFrame::Node(_))))]
    #[ensures(self.stack.len() + 1 == old(self.stack.len()))]
    fn exit_node_frame(&mut self) {
        let Some(GeneratedBlockFrame::Node(frame)) = self.stack.pop() else {
            panic!("generated block collector exited a node without entering it");
        };
        let node = generated_block_tree_node_from_frame(frame, self.source);
        if let Some(node) = node {
            self.push_node(node);
        }
    }

    #[requires(true)]
    #[ensures(self.stack.len() == old(self.stack.len()) + 1)]
    fn enter_field_frame(&mut self, field: jbotci_tree::FieldRef) {
        self.stack
            .push(GeneratedBlockFrame::Field(GeneratedFieldFrame {
                name: field.name,
                payload: GeneratedBlockPayload::default(),
            }));
    }

    #[requires(matches!(self.stack.last(), Some(GeneratedBlockFrame::Field(_))))]
    #[ensures(self.stack.len() + 1 == old(self.stack.len()))]
    fn exit_field_frame(&mut self) {
        let Some(GeneratedBlockFrame::Field(frame)) = self.stack.pop() else {
            panic!("generated block collector exited a field without entering it");
        };
        let mut payload = frame.payload;
        if let Some(name) = frame.name {
            payload.children = payload
                .children
                .into_iter()
                .map(|child| child.with_data(data! { field_label: Some(name) }))
                .collect();
        }
        self.push_payload(payload);
    }

    #[requires(true)]
    #[ensures(self.stack.len() == old(self.stack.len()) + 1)]
    fn enter_sequence_frame(&mut self) {
        self.stack.push(GeneratedBlockFrame::Collection(
            GeneratedBlockPayload::default(),
        ));
    }

    #[requires(matches!(self.stack.last(), Some(GeneratedBlockFrame::Collection(_))))]
    #[ensures(self.stack.len() + 1 == old(self.stack.len()))]
    fn exit_sequence_frame(&mut self) {
        let Some(GeneratedBlockFrame::Collection(payload)) = self.stack.pop() else {
            panic!("generated block collector exited a sequence without entering it");
        };
        self.push_payload(payload);
    }

    #[requires(true)]
    #[ensures(self.stack.len() == old(self.stack.len()) + 1)]
    fn enter_chain_frame(&mut self) {
        self.stack
            .push(GeneratedBlockFrame::Chain(GeneratedBlockPayload::default()));
    }

    #[requires(matches!(self.stack.last(), Some(GeneratedBlockFrame::Chain(_))))]
    #[ensures(self.stack.len() + 1 == old(self.stack.len()))]
    fn exit_chain_frame(&mut self) {
        let Some(GeneratedBlockFrame::Chain(mut payload)) = self.stack.pop() else {
            panic!("generated block collector exited a chain without entering it");
        };
        payload.children =
            flatten_generated_chain_block_nodes(payload.children, self.source, &mut self.next_id);
        payload.source_range = generated_block_source_range(&payload.children, &payload.leaf_parts);
        self.push_payload(payload);
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_payload(&mut self, payload: GeneratedBlockPayload) {
        if let Some(frame) = self.stack.last_mut() {
            frame.payload_mut().extend(payload);
            return;
        }
        if self.root.is_some() {
            assert!(
                payload.children.is_empty() && payload.leaf_parts.is_empty(),
                "generated block collector received more than one top-level payload"
            );
            return;
        }
        if payload.children.len() == 1 && payload.leaf_parts.is_empty() {
            self.root = payload.children.into_iter().next();
            return;
        }
        self.root = self.synthetic_root_from_payload(payload);
    }

    #[requires(true)]
    #[ensures(true)]
    fn synthetic_root_from_payload(
        &mut self,
        payload: GeneratedBlockPayload,
    ) -> Option<BlockTreeNode> {
        if payload.children.is_empty() && payload.leaf_parts.is_empty() {
            return None;
        }
        let id = self.allocate_id();
        generated_block_tree_node_from_parts(
            id,
            None,
            vec![id],
            "GeneratedSyntaxRoot".to_owned(),
            GentufaBlockRole::Normal,
            None,
            None,
            Vec::new(),
            vec!["GeneratedSyntaxRoot".to_owned()],
            payload.children,
            payload.leaf_parts,
            self.source,
            None,
        )
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
    fn push_leaf_part(&mut self, part: BlockLeafPart) {
        if let Some(frame) = self.stack.last_mut() {
            frame.payload_mut().push_leaf_part(part);
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
        match value.as_data() {
            data!(WithIndicators::Plain(word_like)) => self.push_word_like(word_like),
            data!(WithIndicators::Emphasized {
                bahe,
                extra_bahe,
                word_like,
            }) => {
                self.push_word(bahe);
                for bahe in extra_bahe {
                    self.push_word(bahe);
                }
                self.push_word_like(word_like);
            }
            data!(WithIndicators::WithIndicator {
                base,
                indicator_bahe,
                indicator,
                nai_bahe,
                nai,
            }) => {
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
        if let Some(word) = word_like.bare_word() {
            self.push_word(word);
            return;
        }
        let kind = if matches!(
            word_like.as_data(),
            data!(jbotci_morphology::WordLike::ZeiCompound { .. })
        ) {
            CompositeKind::Zei
        } else if matches!(
            word_like.as_data(),
            data!(jbotci_morphology::WordLike::LerfuWord { .. })
        ) {
            CompositeKind::Bu
        } else {
            CompositeKind::Quote
        };
        let group = RawSyntaxNodeId(self.next_id);
        let mut collector = MorphologyBlockLeafCollector::new();
        word_like.visit_in_order(&mut collector);
        for leaf in collector.finish() {
            match leaf.into_data() {
                data!(MorphologyBlockLeaf::Word { word, context }) => {
                    self.push_word_in_context(
                        word,
                        context,
                        Some(new!(BlockLeafOrigin::CompositeMember { group, kind })),
                    );
                }
                data!(MorphologyBlockLeaf::Verbatim(verbatim)) => self.push_verbatim(verbatim),
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_verbatim(&mut self, verbatim: &Verbatim) {
        let range = range_from_span(&verbatim.span);
        let raw_text = self
            .source
            .get(range.byte_start..range.byte_end)
            .unwrap_or(&verbatim.text)
            .to_owned();
        if raw_text.is_empty() {
            return;
        }
        self.recovery_projection.separate();
        self.last_token_end_range = Some(end_range_from_span(&verbatim.span));
        let id = self.allocate_id();
        self.push_leaf_part(new!(BlockLeafPart {
            id,
            range,
            role: GentufaBlockRole::Normal,
            error_index: None,
            token_kind: None,
            raw_text: raw_text.clone(),
            display_text: raw_text,
            origin: new!(BlockLeafOrigin::Verbatim),
            columns: NonZeroUsize::MIN,
        }));
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_word_in_context(
        &mut self,
        word: &Word,
        context: LeadingPauseContext,
        origin: Option<BlockLeafOrigin>,
    ) {
        let span = word.span();
        let range = range_from_span(span);
        self.recovery_projection.separate();
        self.last_token_end_range = Some(end_range_from_span(span));
        let id = self.allocate_id();
        self.push_leaf_part(new!(BlockLeafPart {
            id,
            range,
            role: GentufaBlockRole::Normal,
            error_index: None,
            token_kind: Some(word.kind()),
            raw_text: source_text_for_range(self.source, Some(range)),
            display_text: render_word_in_context(word, self.options, context),
            origin: origin.unwrap_or_else(|| if word.kind() == WordKind::Cmavo {
                new!(BlockLeafOrigin::PlainCmavo {
                    canonical: word.canonical_phonemes()
                })
            } else {
                new!(BlockLeafOrigin::PlainOther)
            }),
            columns: NonZeroUsize::MIN,
        }));
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_word(&mut self, word: &Word) {
        self.push_word_in_context(word, LeadingPauseContext::IndependentWord, None);
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_elided_terminator(&mut self, field: jbotci_tree::FieldRef) {
        if !self.options.show_elided {
            return;
        }
        let Some(cmavo) = elidable_terminator_for_absent_field_ref(field) else {
            return;
        };
        let Some(range) = self.last_token_end_range else {
            return;
        };
        self.recovery_projection.separate();
        let id = self.allocate_id();
        self.push_leaf_part(new!(BlockLeafPart {
            id,
            range,
            role: GentufaBlockRole::Elided,
            error_index: None,
            token_kind: Some(WordKind::Cmavo),
            raw_text: String::new(),
            display_text: render_elided_cmavo(cmavo, self.options),
            origin: new!(BlockLeafOrigin::Elided),
            columns: NonZeroUsize::MIN,
        }));
    }

    #[requires(item.recovery_error_index().is_some())]
    #[ensures(true)]
    fn push_recovered_error<E: RecoveryItemState>(&mut self, item: &E) {
        if !self.recovery_projection.include(item) {
            return;
        }
        let mut spans = Vec::new();
        item.visit_source_spans(&mut |span| spans.push(span.clone()));
        let Some(range) = range_from_spans(&spans) else {
            panic!("syntax recovery items must carry a source position");
        };
        let error_index = item
            .recovery_error_index()
            .expect("the recovery-item contract requires a diagnostic index");
        let raw_text = source_text_for_range(self.source, Some(range));
        self.last_token_end_range = Some(new!(WebSourceRange {
            byte_start: range.byte_end,
            byte_end: range.byte_end,
            char_start: range.char_end,
            char_end: range.char_end,
        }));
        let id = self.allocate_id();
        self.push_leaf_part(new!(BlockLeafPart {
            id,
            range,
            role: GentufaBlockRole::Error,
            error_index: Some(error_index),
            token_kind: None,
            raw_text: raw_text.clone(),
            display_text: raw_text,
            origin: new!(BlockLeafOrigin::Error),
            columns: NonZeroUsize::MIN,
        }));
    }
}

impl<'tree> TreeVisitor<'tree> for GeneratedBlockCollector<'_, '_, '_, 'tree, false> {
    type Node = GeneratedSyntaxNodeRef<'tree>;
    type Atom = GeneratedSyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        let id = self.id_for_node(node);
        let ref_markers = self.reference_markers_for_id(id);
        self.enter_node_frame(
            id,
            syntax_constructor_name(node.constructor_name()).to_owned(),
            ref_markers,
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        self.exit_node_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: jbotci_tree::FieldRef) {
        self.enter_field_frame(field);
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: jbotci_tree::FieldRef) {
        self.exit_field_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.enter_sequence_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        self.exit_sequence_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_chain(&mut self) {
        self.enter_chain_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_chain(&mut self) {
        self.exit_chain_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        match atom {
            GeneratedSyntaxAtomRef::Token(token) => self.push_token(token),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_absent_optional_field(&mut self, field: jbotci_tree::FieldRef) {
        self.push_elided_terminator(field);
    }
}

impl<'tree> TreeVisitor<'tree> for GeneratedBlockCollector<'_, '_, '_, 'tree, true> {
    type Node = RecoveredSyntaxNodeRef<'tree>;
    type Atom = RecoveredSyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        let id = self.allocate_id();
        self.enter_node_frame(
            id,
            syntax_constructor_name(node.constructor_name()).to_owned(),
            Vec::new(),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        self.exit_node_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: jbotci_tree::FieldRef) {
        self.enter_field_frame(field);
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: jbotci_tree::FieldRef) {
        self.exit_field_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.enter_sequence_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        self.exit_sequence_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_chain(&mut self) {
        self.enter_chain_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_chain(&mut self) {
        self.exit_chain_frame();
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        match atom {
            RecoveredSyntaxAtomRef::Token(token) => self.push_token(token),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_absent_optional_field(&mut self, field: jbotci_tree::FieldRef) {
        self.push_elided_terminator(field);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: RecoveryItemState + Serialize>(&mut self, item: &'tree E) {
        self.push_recovered_error(item);
    }
}

#[requires(!frame.label.is_empty())]
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
        GentufaBlockRole::Normal,
        None,
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
    role: GentufaBlockRole,
    error_index: Option<usize>,
    token_kind: Option<WordKind>,
    ref_markers: Vec<ReferenceMarker>,
    node_types: Vec<String>,
    mut children: Vec<BlockTreeNode>,
    mut leaf_parts: Vec<BlockLeafPart>,
    source: &str,
    computed_gloss: Option<String>,
) -> Option<BlockTreeNode> {
    leaf_parts.sort_by_key(|part| (part.range.byte_start, part.role.sort_key()));
    let span = generated_block_source_range(&children, &leaf_parts);
    if span.is_none() && children.is_empty() && leaf_parts.is_empty() {
        return None;
    }
    children.sort_by_key(|child| child.span.map(|span| span.byte_start).unwrap_or(usize::MAX));
    let summary = generated_block_leaf_summary(&children, &leaf_parts);
    let leaf_word = summary.leaf_word.map(str::to_owned);
    let leaf_token_kind = summary.token_kind;
    Some(new!(BlockTreeNode {
        keep_structural_host: false,
        id,
        field_label,
        node_ids,
        label,
        role,
        error_index,
        token_kind: leaf_token_kind.or(token_kind),
        ref_markers,
        span,
        leaf_parts,
        node_types,
        ancestors: Vec::new(),
        depth: 0,
        raw_text: source_text_for_range(source, span),
        leaf_word,
        computed_gloss,
        children,
    }))
}

#[invariant(leaf_word.as_ref().is_none_or(|word| !word.is_empty()))]
struct BlockLeafSummary<'part> {
    leaf_word: Option<&'part str>,
    token_kind: Option<WordKind>,
}

/// Construction and compound removal must classify the same surviving parts as leaves.
#[requires(true)]
#[ensures(ret.leaf_word.is_some() == (children.is_empty() && leaf_parts.len() == 1 && !leaf_parts[0].display_text.is_empty()))]
#[ensures(ret.token_kind == if children.is_empty() && leaf_parts.len() == 1 { leaf_parts[0].token_kind } else { None })]
fn generated_block_leaf_summary<'part>(
    children: &[BlockTreeNode],
    leaf_parts: &'part [BlockLeafPart],
) -> BlockLeafSummary<'part> {
    let part = if children.is_empty() && leaf_parts.len() == 1 {
        Some(&leaf_parts[0])
    } else {
        None
    };
    new!(BlockLeafSummary {
        leaf_word: part
            .filter(|part| !part.display_text.is_empty())
            .map(|part| part.display_text.as_str()),
        token_kind: part.and_then(|part| part.token_kind),
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_block_source_range(
    children: &[BlockTreeNode],
    leaf_parts: &[BlockLeafPart],
) -> Option<WebSourceRange> {
    let mut range = None;
    for child in children {
        range = merge_source_ranges(range, child.span);
    }
    for part in leaf_parts {
        range = merge_source_ranges(range, Some(part.range));
    }
    range
}

#[requires(span.byte_start <= span.byte_end)]
#[requires(span.char_start <= span.char_end)]
#[ensures(ret.byte_start == ret.byte_end)]
#[ensures(ret.byte_start == span.byte_end)]
fn end_range_from_span(span: &SourceSpan) -> WebSourceRange {
    new!(WebSourceRange {
        byte_start: span.byte_end,
        byte_end: span.byte_end,
        char_start: span.char_end,
        char_end: span.char_end,
    })
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
    let node_data = node.into_data();
    let original = new!(GeneratedChainLinkFragmentSource {
        id: node_data.id,
        field_label: node_data.field_label,
        node_ids: node_data.node_ids,
        label: node_data.label,
        role: node_data.role,
        error_index: node_data.error_index,
        token_kind: node_data.token_kind,
        ref_markers: node_data.ref_markers,
        node_types: node_data.node_types,
        computed_gloss: node_data.computed_gloss,
    });

    let mut prefix_children = Vec::new();
    let mut suffix_children = Vec::new();
    let mut element = None;
    for (index, child) in node_data.children.into_iter().enumerate() {
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
    for part in node_data.leaf_parts {
        if leaf_part_precedes_chain_element(&part, element_span) {
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
fn leaf_part_precedes_chain_element(part: &BlockLeafPart, element_span: WebSourceRange) -> bool {
    // #197 restored zero-width elided leaves. Non-empty leaves are classified
    // by start offset; an elided zero-width leaf exactly at the element start
    // remains attached to the prefix fragment to preserve existing output.
    part.range.byte_start < element_span.byte_start
        || (part.range.byte_start == element_span.byte_start
            && part.range.byte_end == element_span.byte_start)
}

#[invariant(!label.is_empty(), "chain link source label must not be empty")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedChainLinkFragmentSource {
    id: RawSyntaxNodeId,
    field_label: Option<&'static str>,
    node_ids: Vec<RawSyntaxNodeId>,
    label: String,
    role: GentufaBlockRole,
    error_index: Option<usize>,
    token_kind: Option<WordKind>,
    ref_markers: Vec<ReferenceMarker>,
    node_types: Vec<String>,
    computed_gloss: Option<String>,
}

#[requires(true)]
#[ensures(true)]
fn generated_chain_link_fragment_node(
    original: &GeneratedChainLinkFragmentSource,
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
        original.role,
        original.error_index,
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

#[invariant(!node_ids.is_empty(), "block tree nodes must carry at least one syntax id")]
#[invariant(!*keep_structural_host || !children.is_empty(), "shared compound identity needs a surviving structural host")]
#[invariant(!label.is_empty(), "block tree nodes must have a display label")]
#[invariant(
    field_label.as_ref().is_none_or(|label| !label.is_empty()),
    "field labels must not be empty when present"
)]
#[invariant(
    role.is_error() == error_index.is_some(),
    "error block tree nodes must carry exactly one diagnostic index"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct BlockTreeNode {
    id: RawSyntaxNodeId,
    field_label: Option<&'static str>,
    node_ids: Vec<RawSyntaxNodeId>,
    label: String,
    role: GentufaBlockRole,
    error_index: Option<usize>,
    token_kind: Option<WordKind>,
    ref_markers: Vec<ReferenceMarker>,
    span: Option<WebSourceRange>,
    leaf_parts: Vec<BlockLeafPart>,
    node_types: Vec<String>,
    ancestors: Vec<String>,
    depth: usize,
    raw_text: String,
    leaf_word: Option<String>,
    computed_gloss: Option<String>,
    keep_structural_host: bool,
    children: Vec<BlockTreeNode>,
}

#[invariant(
    role.is_error() || !display_text.is_empty(),
    "non-error leaf parts must have display text"
)]
#[invariant(
    role.is_elided() || role.is_error() || !raw_text.is_empty(),
    "ordinary leaf parts must have source text"
)]
#[invariant(
    role.is_error() == error_index.is_some(),
    "error leaf parts must carry exactly one diagnostic index"
)]
#[invariant(
    !role.is_error()
        || !raw_text.is_empty()
        || (range.byte_start == range.byte_end && range.char_start == range.char_end),
    "empty error markers must have zero-width source ranges"
)]
#[invariant(role.is_error() == matches!(origin.as_data(), data!(BlockLeafOrigin::Error)))]
#[invariant(role.is_elided() == matches!(origin.as_data(), data!(BlockLeafOrigin::Elided)))]
#[invariant(match origin.as_data() {data!(BlockLeafOrigin::Compound {..}) => columns.get() >= 2, _ => columns.get() == 1})]
#[derive(Debug, Clone, PartialEq, Eq)]
struct BlockLeafPart {
    id: RawSyntaxNodeId,
    range: WebSourceRange,
    role: GentufaBlockRole,
    error_index: Option<usize>,
    token_kind: Option<WordKind>,
    raw_text: String,
    display_text: String,
    origin: BlockLeafOrigin,
    columns: NonZeroUsize,
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

#[invariant(true)]
struct BlockCollapseFrame {
    // Children live in the sibling accumulators while the frame is open, so
    // revalidate the complete node only after all of them have been restored.
    node: BlockTreeNodeData,
    remaining_children: Vec<BlockTreeNode>,
    collapsed_children: Vec<BlockTreeNode>,
}

#[requires(true)]
#[ensures(true)]
fn collapse_single_child_chains(node: BlockTreeNode) -> BlockTreeNode {
    let mut frames = Vec::new();
    let mut next = Some(node);
    let mut completed = None;
    loop {
        if let Some(node) = next.take() {
            let mut node_data = node.into_data();
            let mut remaining_children = std::mem::take(&mut node_data.children);
            remaining_children.reverse();
            frames.push(BlockCollapseFrame {
                node: node_data,
                remaining_children,
                collapsed_children: Vec::new(),
            });
            continue;
        }

        if let Some(node) = completed.take() {
            if let Some(mut parent) = frames.pop() {
                parent.collapsed_children.push(node);
                frames.push(parent);
            } else {
                return node;
            }
        }

        let Some(mut frame) = frames.pop() else {
            panic!("block collapse traversal lost its root frame");
        };
        if let Some(child) = frame.remaining_children.pop() {
            frames.push(frame);
            next = Some(child);
            continue;
        }

        let mut node_data = frame.node;
        node_data.children = frame.collapsed_children;
        completed = Some(collapse_single_child_node(BlockTreeNode::from_data(
            node_data,
        )));
    }
}

#[requires(true)]
#[ensures(true)]
fn collapse_single_child_node(mut node: BlockTreeNode) -> BlockTreeNode {
    if !node.keep_structural_host && node.children.len() == 1 {
        let mut node_data = node.into_data();
        let child = node_data
            .children
            .pop()
            .expect("one child was checked above");
        node = BlockTreeNode::from_data(node_data);
        if can_collapse_single_child(&node, &child) {
            return merge_parent_into_child(node, child);
        }
        let mut node_data = node.into_data();
        node_data.children.push(child);
        node = BlockTreeNode::from_data(node_data);
    }
    node
}

#[requires(true)]
#[ensures(true)]
fn can_collapse_single_child(parent: &BlockTreeNode, child: &BlockTreeNode) -> bool {
    !parent.keep_structural_host
        && parent.leaf_word.is_none()
        && parent.token_kind.is_none()
        && parent.leaf_parts.iter().all(|part| part.role.is_normal())
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
fn merge_parent_into_child(parent: BlockTreeNode, child: BlockTreeNode) -> BlockTreeNode {
    let parent = parent.into_data();
    let mut child = child.into_data();
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
    if child.role.is_normal() {
        child.role = parent.role;
        child.error_index = parent.error_index;
    }
    BlockTreeNode::from_data(child)
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
fn collapse_safe_multi_child_parents(node: BlockTreeNode) -> BlockTreeNode {
    let mut node_data = node.into_data();
    let mut children = Vec::new();
    for child in std::mem::take(&mut node_data.children) {
        let child = collapse_safe_multi_child_parents(child);
        if should_collapse_safe_multi_child_parent(&child) {
            children.extend(child.into_data().children);
        } else {
            children.push(child);
        }
    }
    node_data.children = children;
    BlockTreeNode::from_data(node_data)
}

#[requires(true)]
#[ensures(true)]
fn should_collapse_safe_multi_child_parent(node: &BlockTreeNode) -> bool {
    !node.keep_structural_host
        && node.children.len() > 1
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
#[ensures(ret.depth == 0)]
fn assign_tree_depths_and_ancestors(root: BlockTreeNode) -> BlockTreeNode {
    assign_tree_depths_and_ancestors_inner(root, 0, &mut Vec::new())
}

#[requires(true)]
#[ensures(ret.depth == depth)]
fn assign_tree_depths_and_ancestors_inner(
    node: BlockTreeNode,
    depth: usize,
    ancestors: &mut Vec<String>,
) -> BlockTreeNode {
    let mut node_data = node.into_data();
    node_data.depth = depth;
    node_data.ancestors = ancestors.clone();
    ancestors.push(node_data.label.clone());
    let children = std::mem::take(&mut node_data.children);
    node_data.children = children
        .into_iter()
        .map(|child| assign_tree_depths_and_ancestors_inner(child, depth + 1, ancestors))
        .collect();
    ancestors.pop();
    BlockTreeNode::from_data(node_data)
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
        return col + node.leaf_parts.first().map_or(1, |part| part.columns.get());
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
                next_col += part.columns.get();
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
            .filter(|part| leaf_part_is_uncovered_by_children(&node.children, part))
            .map(BlockLayoutChild::Leaf),
    );
    children.sort_by_key(layout_child_sort_key);
    children
}

#[requires(true)]
#[ensures(true)]
fn has_uncovered_leaf_parts(node: &BlockTreeNode) -> bool {
    node.leaf_parts
        .iter()
        .any(|part| leaf_part_is_uncovered_by_children(&node.children, part))
}

#[requires(true)]
#[ensures(true)]
fn leaf_part_is_uncovered_by_children(children: &[BlockTreeNode], part: &BlockLeafPart) -> bool {
    !part.role.is_normal() || !children.iter().any(|child| child_covers_part(child, part))
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
    let mut next_col = col;
    for part in &node.leaf_parts {
        blocks.push(BlockTemp {
            id: part.id,
            parent_id: Some(node.id),
            child_ids: Vec::new(),
            block: synthetic_leaf_block(node, part, next_col, leaf_depth, row_span),
        });
        next_col += part.columns.get();
    }
    let col_span = node
        .leaf_parts
        .iter()
        .map(|part| part.columns.get())
        .sum::<usize>()
        .max(1);
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
    new!(GentufaBlock {
        block_id: format!("n{}", part.id.0),
        node_ids: if node.keep_structural_host {
            // The shared donor identity belongs only to the structural host;
            // its surviving own parts retain their individual identities.
            vec![part.id.0]
        } else {
            node.node_ids.iter().map(|id| id.0).collect()
        },
        label: part.display_text.clone(),
        is_leaf: true,
        role: part.role,
        error_index: part.error_index,
        token_kind: part.token_kind,
        compound_kind: part_compound_kind(part),
        ref_markers: Vec::new(),
        span: Some(part.range),
        node_types: node.node_types.clone(),
        ancestors: synthetic_leaf_ancestors(node),
        col,
        col_span: part.columns.get(),
        row,
        row_span,
        color: String::new(),
        raw_text: part.raw_text.clone(),
        display_text: part.display_text.clone(),
        glosses: Vec::new(),
        definition: None,
        computed_gloss: None,
        tooltip: None,
    })
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
        && !part.role.is_normal()
    {
        // This terminal is the node's only emitted block. Unlike split leaves,
        // it has no separate structural host to retain the node's annotations.
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
            )
            .with_data(data! {
                ref_markers: node.ref_markers.clone(),
                computed_gloss: node.computed_gloss.clone(),
            }),
        });
        return;
    }
    let is_leaf = node.leaf_word.is_some()
        && (node.token_kind.is_some() || node_compound_kind(node).is_some());
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
            node.leaf_parts.first().map_or(1, |part| part.columns.get()),
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
    new!(GentufaBlock {
        block_id: format!("n{}", node.id.0),
        node_ids: node.node_ids.iter().map(|id| id.0).collect(),
        label: if is_leaf && !display_text.is_empty() {
            display_text.clone()
        } else {
            syntax_constructor_display_label(&node.label).to_owned()
        },
        is_leaf,
        role: node.role,
        error_index: node.error_index,
        token_kind: node.token_kind,
        compound_kind: node_compound_kind(node),
        ref_markers: node.ref_markers.clone(),
        span: node.span,
        node_types: node.node_types.clone(),
        ancestors: node.ancestors.clone(),
        col,
        col_span,
        row,
        row_span,
        color: String::new(),
        raw_text: node.raw_text.clone(),
        display_text,
        glosses: Vec::new(),
        definition: None,
        computed_gloss: node.computed_gloss.clone(),
        tooltip: None,
    })
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
    for block in leaf_blocks {
        let block_id = block.id;
        let hue = parent_hues
            .iter()
            .find(|(parent, _)| *parent == block.parent_id)
            .map(|(_, hue)| *hue)
            .unwrap_or(0.0);
        let block_value = block
            .block
            .with_data(data! { color: hsl_to_hex(hue, 0.99, 0.85) });
        hue_map.insert(block_id, (hue, block_value.col_span));
        colored.push(block_value);
    }
    nonleaf_blocks.sort_by_key(|block| Reverse(block.block.row));
    let mut nonleaf_colored = Vec::with_capacity(nonleaf_blocks.len());
    for block in nonleaf_blocks {
        let block_id = block.id;
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
        let block_value = block
            .block
            .with_data(data! { color: hsl_to_hex(hue, saturation, lightness) });
        hue_map.insert(block_id, (hue, block_value.col_span));
        nonleaf_colored.push(block_value);
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
    let recipients = block_annotation_recipients(&blocks, annotations);
    blocks
        .into_iter()
        .zip(recipients)
        .map(|(block, annotation)| {
            if let Some(annotation) = annotation {
                return block.with_data(data! {
                    glosses: annotation.glosses.clone(),
                    definition: annotation.definition.clone(),
                    tooltip: annotation.tooltip.clone(),
                });
            }
            block
        })
        .collect()
}

#[requires(true)]
#[ensures(ret.len() == blocks.len())]
pub fn block_annotation_recipients<'a, Tooltip, ReferenceTooltip>(
    blocks: &[GentufaBlock<Tooltip, ReferenceTooltip>],
    annotations: &'a [GentufaBlockAnnotation<Tooltip>],
) -> Vec<Option<&'a GentufaBlockAnnotation<Tooltip>>> {
    let mut recipients = vec![None; blocks.len()];
    for annotation in annotations {
        if let Some(index) = annotation_recipient(blocks, &annotation.target) {
            recipients[index].get_or_insert(annotation);
        }
    }
    recipients
}

/// Resolve one presentation host, shared by initial annotation and later enrichment.
#[requires(true)]
#[ensures(ret.is_none_or(|index| index < blocks.len()))]
fn annotation_recipient<Tooltip, ReferenceTooltip>(
    blocks: &[GentufaBlock<Tooltip, ReferenceTooltip>],
    target: &GentufaAnnotationTarget,
) -> Option<usize> {
    match target.as_data() {
        data!(GentufaAnnotationTarget::Block { block_id, range }) => {
            blocks.iter().position(|block| {
                block.block_id == *block_id
                    && block.span == Some(*range)
                    && block.role.is_normal()
                    && block.is_leaf
                    && block.compound_kind.is_some()
            })
        }
        data!(GentufaAnnotationTarget::SourceRange { range, text }) => {
            if range.byte_start == range.byte_end {
                let text = text.as_ref()?;
                return blocks.iter().position(|block| {
                    block.role.is_elided()
                        && block.span == Some(*range)
                        && block.display_text == *text
                });
            }
            let mut leaves = blocks.iter().enumerate().filter(|(_, block)| {
                block.role.is_normal() && block.is_leaf && block.span == Some(*range)
            });
            let first = leaves.next();
            if first.is_some() && leaves.next().is_none() {
                return first.map(|(index, _)| index);
            }
            blocks
                .iter()
                .enumerate()
                .filter(|(_, block)| {
                    block.role.is_normal() && !block.is_leaf && block.span == Some(*range)
                })
                .max_by_key(|(index, block)| (block.row, Reverse(*index)))
                .map(|(index, _)| index)
        }
    }
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
    let mut degrees = y.atan2(x).to_degrees().rem_euclid(360.0);
    // Floating-point remainder can round a tiny negative angle to the upper
    // boundary; hues are represented as the half-open range [0, 360).
    if degrees >= 360.0 {
        degrees = 0.0;
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
#[ensures(!ret.as_str().is_empty())]
fn reference_kind_for_label(label: &ReferenceLabel) -> ReferenceMarkerKind {
    if label.slot.is_some() {
        ReferenceMarkerKind::Sumti
    } else {
        ReferenceMarkerKind::Reference
    }
}

#[requires(!label.stem.is_empty())]
#[ensures(ret.stem == label.stem)]
pub fn reference_label_from_output(label: &OutputReferenceName) -> ReferenceLabel {
    new!(ReferenceLabel {
        stem: label.stem.clone(),
        occurrence: label.occurrence,
        slot: label.slot.as_ref().map(reference_slot_label_from_output),
    })
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
fn token_kind_for_text(text: &str) -> Option<WordKind> {
    let words = segment_words_with_modifiers(text).ok()?;
    let [word_like] = words.as_slice() else {
        return None;
    };
    word_like.bare_word().map(Word::kind)
}

#[requires(true)]
#[ensures((constructor.ends_with("Syntax") && ret.len() + "Syntax".len() == constructor.len()) || (!constructor.ends_with("Syntax") && ret == constructor))]
pub fn syntax_constructor_name(constructor: &str) -> &str {
    constructor.strip_suffix("Syntax").unwrap_or(constructor)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn syntax_constructor_display_label<'a>(constructor: &'a str) -> &'a str {
    generated_model::GENERATED_MODEL_CONSTRUCTOR_LABELS
        .iter()
        .find_map(|(candidate, label)| (*candidate == constructor).then_some(*label))
        .unwrap_or_else(|| {
            let label = syntax_constructor_name(constructor);
            if label.is_empty() { "unknown" } else { label }
        })
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
fn render_word_in_context(
    word: &Word,
    options: &GentufaBlockOptions,
    context: LeadingPauseContext,
) -> String {
    let latin = visible_latin_word_surface(word, options.phonemes, context);
    render_latin_word_surface_for_script(options.script, word.kind(), &latin)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn render_elided_cmavo(cmavo: Cmavo, options: &GentufaBlockOptions) -> String {
    let phonemes = Phonemes::from_canonical(cmavo.canonical_text().to_owned())
        .expect("cmavo canonical text is valid phoneme text");
    render_latin_word_surface_for_script(
        options.script,
        WordKind::Cmavo,
        &phonemes.render(options.phonemes),
    )
}

#[requires(true)]
#[ensures(true)]
fn visible_latin_word_surface(
    word: &Word,
    options: PhonemeRenderOptions,
    context: LeadingPauseContext,
) -> String {
    let mut rendered = word.phonemes().render(options);
    if word_needs_leading_pause_in_context(word, LeadingPauseVowelMode::LatinSurfaceVowels, context)
    {
        rendered.insert(0, '.');
    }
    if word.kind() == WordKind::Cmevla {
        rendered.push('.');
    }
    rendered
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

    #[invariant(byte_start < byte_end, "expected composite leaves must cover source text")]
    #[derive(Debug, Clone, Copy)]
    struct ExpectedCompositeLeaf {
        raw_text: &'static str,
        display_text: &'static str,
        byte_start: usize,
        byte_end: usize,
    }

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
    fn weighted_circular_mean_hue_stays_in_half_open_range() {
        let hue = weighted_circular_mean_hue(&[(360.0, 1)]).expect("hue");

        assert!((0.0..360.0).contains(&hue), "{hue}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_constructor_labels_degrade_to_non_empty_text() {
        assert_eq!(syntax_constructor_name("FooSyntaxSyntax"), "FooSyntax");
        assert_eq!(syntax_constructor_name("Syntax"), "");
        assert_eq!(syntax_constructor_display_label("Syntax"), "unknown");
        assert_eq!(syntax_constructor_display_label(""), "unknown");
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
    fn generated_selbri_connection_blocks_render_in_flat_source_order() {
        let layout = generated_test_blocks_layout("mi melbi je cmalu je blanu");

        assert_eq!(
            generated_leaf_display_texts(&layout),
            vec!["mi", "mélbi", "je", "cmálu", "je", "blánu"]
        );
        assert!(
            layout
                .blocks
                .iter()
                .any(|block| block.label == "selbri connection")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_zoi_quote_blocks_expose_each_morphology_leaf() {
        assert_composite_layout(
            "mi klama zoi gy house gy",
            "quote",
            &[
                expected_composite_leaf("mi", "mi", 0, 2),
                expected_composite_leaf("klama", "kláma", 3, 8),
                expected_composite_leaf("zoi", "zoĭ", 9, 12),
                expected_composite_leaf("gy", "gy", 13, 15),
                expected_composite_leaf("house", "house", 16, 21),
                expected_composite_leaf("gy", "gy", 22, 24),
            ],
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_zo_quote_blocks_expose_each_morphology_leaf() {
        assert_composite_layout(
            "mi klama zo coi",
            "quote",
            &[
                expected_composite_leaf("mi", "mi", 0, 2),
                expected_composite_leaf("klama", "kláma", 3, 8),
                expected_composite_leaf("zo", "zo", 9, 11),
                expected_composite_leaf("coi", "coĭ", 12, 15),
            ],
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_zei_compound_blocks_expose_each_morphology_leaf() {
        assert_composite_layout(
            "mi bakni zei kanla",
            "tanru unit",
            &[
                expected_composite_leaf("mi", "mi", 0, 2),
                expected_composite_leaf("bakni", "bákni", 3, 8),
                expected_composite_leaf("zei", "zeĭ", 9, 12),
                expected_composite_leaf("kanla", "kánla", 13, 18),
            ],
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_bu_lerfu_blocks_expose_each_morphology_leaf() {
        assert_composite_layout(
            ".abu",
            "lerfu word",
            &[
                expected_composite_leaf("a", ".a", 1, 2),
                expected_composite_leaf("bu", "bu", 2, 4),
            ],
        );
        assert_composite_layout(
            "ybu",
            "lerfu word",
            &[
                expected_composite_leaf("y", ".y", 0, 1),
                expected_composite_leaf("bu", "bu", 1, 3),
            ],
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_layout_keeps_composite_leaves_next_to_skipped_region() {
        let (layout, errors) = recovered_test_blocks_layout("mi bakni zei kanla ku i do");
        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert_eq!(
            generated_leaf_display_texts(&layout),
            vec!["mi", "bákni", "zeĭ", "kánla", "ku", ".i", "do"]
        );
        let error = only_error_block(&layout);
        assert_eq!(error.raw_text, "ku");
        assert!(
            layout
                .blocks
                .iter()
                .any(|block| !block.is_leaf && block.label == "tanru unit")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_layout_keeps_composite_token_inside_error_region() {
        let (layout, errors) = recovered_test_blocks_layout("mi ku zoi gy house gy i do");
        assert_eq!(errors.len(), 1, "{errors:#?}");
        let error = only_error_block(&layout);
        assert_eq!(error.raw_text, "ku zoi gy house gy");
        assert_eq!(error.display_text, "ku zoi gy house gy");
        assert_eq!(block_byte_range(error), (3, 21));
        assert!(
            generated_leaf_display_texts(&layout).ends_with(&[".i".to_owned(), "do".to_owned()])
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_chain_link_split_keeps_suffix_after_element_for_blocks() {
        let link = new!(BlockTreeNode {
            keep_structural_host: false,
            id: RawSyntaxNodeId(1),
            field_label: None,
            node_ids: vec![RawSyntaxNodeId(1)],
            label: "BridiTailContinuation".to_owned(),
            role: GentufaBlockRole::Normal,
            error_index: None,
            token_kind: None,
            ref_markers: Vec::new(),
            span: Some(test_range(0, 3)),
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
        });
        let mut next_id = 10;

        let parts = split_generated_chain_link_block_node(link, "", &mut next_id);

        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].label, "BridiTailContinuation");
        assert_eq!(parts[0].leaf_parts[0].display_text, "gi'e");
        assert_eq!(parts[1].label, "SelbriSimpleBridiTail");
        assert_eq!(parts[2].label, "BridiTailContinuation");
        assert_eq!(parts[2].leaf_parts[0].display_text, "do");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_layout_keeps_valid_statements_around_skipped_slot() {
        let (layout, errors) = recovered_test_blocks_layout("mi ku i do");
        assert_eq!(errors.len(), 1);

        let mi = normal_leaf_for_raw_text(&layout, "mi");
        let error = only_error_block(&layout);
        let separator = normal_leaf_for_raw_text(&layout, "i");
        let do_block = normal_leaf_for_raw_text(&layout, "do");

        assert_eq!(block_byte_range(mi), (0, 2));
        assert_eq!(block_byte_range(error), (3, 5));
        assert_eq!(error.raw_text, "ku");
        assert_eq!(error.display_text, "ku");
        assert_eq!(error.error_index, Some(0));
        assert_eq!(syntax_error_byte_range(&errors[0]), (3, 5));
        assert_eq!(block_byte_range(do_block), (8, 10));

        assert_eq!(mi.col + mi.col_span, error.col);
        assert_eq!(error.col + error.col_span, separator.col);
        assert_eq!(separator.col + separator.col_span, do_block.col);
        assert_eq!(mi.row, error.row);
        assert!(do_block.row > error.row);
        assert_eq!(mi.ancestors, error.ancestors);
        assert!(
            do_block
                .ancestors
                .iter()
                .any(|ancestor| ancestor == "FollowingParagraphStatement")
        );
        assert!(
            error
                .ancestors
                .iter()
                .all(|ancestor| ancestor != "FollowingParagraphStatement")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_model_slots_and_rendered_missing_marker_stay_distinct() {
        let source = "mi viska lo";
        let words = segment_words_with_modifiers(source).expect("test source has valid morphology");
        let recovered = jbotci_syntax::parse_syntax_tree_recovered_with_source_and_options(
            &words,
            source,
            &jbotci_syntax::ParseOptions::default(),
        );
        assert_eq!(recovered.errors.len(), 1);
        // One slot per abandoned construct, and at this offset there is now exactly one. The
        // count was three until epoch 9 deleted the unsourced description-head connective route
        // (#837 SUM-01): its `connective`, `trailing_description_head` and `tail` fields were the
        // other two slots, and a route that no longer exists abandons nothing. The rendering is
        // unchanged, which is the distinction this test exists to hold.
        assert_eq!(
            jbotci_tree::RecoveredFieldState::recovery_error_slots(recovered.parse_tree.as_ref()),
            1,
            "the recovered model keeps one slot per abandoned construct"
        );

        let brackets = jbotci_output::pretty_recovered_syntax_brackets_with_options(
            &recovered,
            source,
            jbotci_output::BracketRenderOptions::default(),
        )
        .expect("recovered brackets");
        assert_eq!(brackets, "(mi [víska {lo ‼‼}])");
        assert_eq!(brackets.matches("‼‼").count(), 1);

        let layout = recovered_generated_model_blocks_layout(
            recovered.parse_tree.as_ref(),
            source,
            recovered.errors.len(),
            &Vec::<GentufaBlockAnnotation<()>>::new(),
            &GentufaBlockOptions::default(),
        );

        let mi = normal_leaf_for_raw_text(&layout, "mi");
        let viska = normal_leaf_for_raw_text(&layout, "viska");
        let lo = normal_leaf_for_raw_text(&layout, "lo");
        let markers = error_blocks(&layout);

        assert_eq!(block_byte_range(mi), (0, 2));
        assert_eq!(block_byte_range(viska), (3, 8));
        assert_eq!(block_byte_range(lo), (9, 11));
        assert_eq!(markers.len(), 1, "the blocks projection collapses the run");
        assert_eq!(lo.col + lo.col_span, markers[0].col);
        assert!(viska.col < lo.col);
        for marker in markers {
            assert_eq!(block_byte_range(marker), (11, 11));
            assert!(marker.raw_text.is_empty());
            assert!(marker.display_text.is_empty());
            assert_eq!(marker.error_index, Some(0));
            assert_eq!(marker.row, lo.row);
            assert_eq!(marker.ancestors, lo.ancestors);
            assert_eq!(
                syntax_error_byte_range(&recovered.errors[0]),
                block_byte_range(marker)
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_layout_links_each_skipped_block_to_matching_diagnostic() {
        let source = "mi ku i do ku i mi klama";
        let (layout, errors) = recovered_test_blocks_layout(source);
        let error_blocks = error_blocks(&layout);

        assert_eq!(errors.len(), 2);
        assert_eq!(error_blocks.len(), 2);
        assert_eq!(block_byte_range(error_blocks[0]), (3, 5));
        assert_eq!(block_byte_range(error_blocks[1]), (11, 13));
        assert_eq!(error_blocks[0].raw_text, "ku");
        assert_eq!(error_blocks[1].raw_text, "ku");
        for block in error_blocks {
            let error_index = block.error_index.expect("error block index");
            assert_eq!(
                block_byte_range(block),
                syntax_error_byte_range(&errors[error_index])
            );
        }

        let do_block = normal_leaf_for_raw_text(&layout, "do");
        let klama = normal_leaf_for_raw_text(&layout, "klama");
        assert!(error_blocks_between(&layout, do_block, klama));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn top_level_payload_synthesizes_root_for_multiple_children() {
        let options = GentufaBlockOptions::default();
        let mut collector = GeneratedBlockCollector::<false>::new("mi do", &options, None, None);
        let mut payload = GeneratedBlockPayload::default();
        payload.push_node(test_generated_block_node(
            1,
            "First",
            None,
            test_range(0, 2),
            "mi",
        ));
        payload.push_node(test_generated_block_node(
            2,
            "Second",
            None,
            test_range(3, 5),
            "do",
        ));

        collector.push_payload(payload);
        let root = collector.finish().expect("synthetic root");

        assert_eq!(root.label, "GeneratedSyntaxRoot");
        assert_eq!(root.children.len(), 2);
        assert!(root.leaf_parts.is_empty());
    }

    #[requires(true)]
    #[ensures(ret.depth == depth)]
    fn test_block_tree_node(depth: usize, leaf_part_count: usize) -> BlockTreeNode {
        new!(BlockTreeNode {
            keep_structural_host: false,
            id: RawSyntaxNodeId(depth),
            field_label: None,
            node_ids: vec![RawSyntaxNodeId(depth)],
            label: format!("node-{depth}"),
            role: GentufaBlockRole::Normal,
            error_index: None,
            token_kind: None,
            ref_markers: Vec::new(),
            span: None,
            leaf_parts: test_leaf_parts(leaf_part_count),
            node_types: vec![format!("Node{depth}")],
            ancestors: Vec::new(),
            depth,
            raw_text: String::new(),
            leaf_word: None,
            computed_gloss: None,
            children: Vec::new(),
        })
    }

    #[requires(true)]
    #[ensures(ret.len() == count)]
    fn test_leaf_parts(count: usize) -> Vec<BlockLeafPart> {
        (0..count)
            .map(|index| {
                new!(BlockLeafPart {
                    origin: new!(BlockLeafOrigin::PlainOther),
                    columns: NonZeroUsize::MIN,
                    id: RawSyntaxNodeId(index),
                    range: new!(WebSourceRange {
                        byte_start: index,
                        byte_end: index + 1,
                        char_start: index,
                        char_end: index + 1,
                    }),
                    role: GentufaBlockRole::Normal,
                    error_index: None,
                    token_kind: token_kind_for_text(&format!("w{index}")),
                    raw_text: format!("w{index}"),
                    display_text: format!("w{index}"),
                })
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
    #[ensures(ret.0.blocks.iter().all(|block| {
        block.error_index.is_none_or(|error_index| error_index < ret.1.len())
    }))]
    fn recovered_test_blocks_layout(
        source: &str,
    ) -> (GentufaBlocksLayout, Vec<jbotci_syntax::SyntaxError>) {
        let words = segment_words_with_modifiers(source).expect("test source has valid morphology");
        let recovered = jbotci_syntax::parse_syntax_tree_recovered_with_source_and_options(
            &words,
            source,
            &jbotci_syntax::ParseOptions::default(),
        );
        let layout = recovered_generated_model_blocks_layout(
            recovered.parse_tree.as_ref(),
            source,
            recovered.errors.len(),
            &Vec::<GentufaBlockAnnotation<()>>::new(),
            &GentufaBlockOptions::default(),
        );
        (layout, recovered.errors.clone())
    }

    #[requires(!raw_text.is_empty())]
    #[ensures(ret.is_leaf)]
    #[ensures(ret.role == GentufaBlockRole::Normal)]
    fn normal_leaf_for_raw_text<'layout>(
        layout: &'layout GentufaBlocksLayout,
        raw_text: &str,
    ) -> &'layout GentufaBlock {
        layout
            .blocks
            .iter()
            .find(|block| {
                block.is_leaf
                    && block.role == GentufaBlockRole::Normal
                    && block.raw_text == raw_text
            })
            .unwrap_or_else(|| panic!("missing normal leaf for {raw_text:?}: {layout:#?}"))
    }

    #[requires(true)]
    #[ensures(ret.role == GentufaBlockRole::Error)]
    fn only_error_block(layout: &GentufaBlocksLayout) -> &GentufaBlock {
        let blocks = error_blocks(layout);
        assert_eq!(blocks.len(), 1, "{blocks:#?}");
        blocks[0]
    }

    #[requires(true)]
    #[ensures(ret.iter().all(|block| block.role == GentufaBlockRole::Error))]
    fn error_blocks(layout: &GentufaBlocksLayout) -> Vec<&GentufaBlock> {
        let mut blocks = layout
            .blocks
            .iter()
            .filter(|block| block.role == GentufaBlockRole::Error)
            .collect::<Vec<_>>();
        blocks.sort_by_key(|block| (block.col, block.row));
        blocks
    }

    #[requires(block.span.is_some())]
    #[ensures(ret.0 <= ret.1)]
    fn block_byte_range(block: &GentufaBlock) -> (usize, usize) {
        let span = block.span.expect("block source range");
        (span.byte_start, span.byte_end)
    }

    #[requires(true)]
    #[ensures(ret.0 <= ret.1)]
    fn syntax_error_byte_range(error: &jbotci_syntax::SyntaxError) -> (usize, usize) {
        match error {
            jbotci_syntax::SyntaxError::Parse {
                byte_start,
                byte_end,
                ..
            } => (*byte_start, *byte_end),
            jbotci_syntax::SyntaxError::NotImplemented => (0, 0),
        }
    }

    #[requires(left.col + left.col_span <= right.col)]
    #[ensures(true)]
    fn error_blocks_between(
        layout: &GentufaBlocksLayout,
        left: &GentufaBlock,
        right: &GentufaBlock,
    ) -> bool {
        error_blocks(layout).iter().any(|block| {
            left.col + left.col_span <= block.col && block.col + block.col_span <= right.col
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn generated_leaf_display_texts(layout: &GentufaBlocksLayout) -> Vec<String> {
        layout
            .blocks
            .iter()
            .filter(|block| block.is_leaf && !block.role.is_elided())
            .map(|block| block.display_text.clone())
            .collect()
    }

    #[requires(!raw_text.is_empty() && !display_text.is_empty())]
    #[requires(byte_start < byte_end)]
    #[ensures(ret.raw_text == raw_text && ret.display_text == display_text)]
    fn expected_composite_leaf(
        raw_text: &'static str,
        display_text: &'static str,
        byte_start: usize,
        byte_end: usize,
    ) -> ExpectedCompositeLeaf {
        new!(ExpectedCompositeLeaf {
            raw_text,
            display_text,
            byte_start,
            byte_end,
        })
    }

    #[requires(!source.is_empty() && !construct_label.is_empty() && !expected.is_empty())]
    #[ensures(true)]
    fn assert_composite_layout(
        source: &str,
        construct_label: &str,
        expected: &[ExpectedCompositeLeaf],
    ) {
        let layout = generated_test_blocks_layout(source);
        let mut leaves = layout
            .blocks
            .iter()
            .filter(|block| block.is_leaf && block.role.is_normal())
            .collect::<Vec<_>>();
        leaves.sort_by_key(|block| (block.col, block.row));
        assert_eq!(leaves.len(), expected.len(), "{layout:#?}");
        for (block, expected) in leaves.into_iter().zip(expected) {
            assert_eq!(block.raw_text, expected.raw_text);
            assert_eq!(block.display_text, expected.display_text);
            assert_eq!(
                block.span,
                Some(new!(WebSourceRange {
                    byte_start: expected.byte_start,
                    byte_end: expected.byte_end,
                    char_start: expected.byte_start,
                    char_end: expected.byte_end,
                }))
            );
        }
        assert!(
            layout
                .blocks
                .iter()
                .any(|block| !block.is_leaf && block.label == construct_label),
            "missing composite construct {construct_label:?}: {layout:#?}"
        );
    }

    #[requires(byte_start <= byte_end)]
    #[ensures(ret.byte_start == byte_start)]
    fn test_range(byte_start: usize, byte_end: usize) -> WebSourceRange {
        new!(WebSourceRange {
            byte_start,
            byte_end,
            char_start: byte_start,
            char_end: byte_end,
        })
    }

    #[requires(!display_text.is_empty())]
    #[requires(range.byte_start <= range.byte_end)]
    #[ensures(ret.display_text == display_text)]
    fn test_leaf_part(id: usize, display_text: &str, range: WebSourceRange) -> BlockLeafPart {
        new!(BlockLeafPart {
            origin: new!(BlockLeafOrigin::PlainOther),
            columns: NonZeroUsize::MIN,
            id: RawSyntaxNodeId(id),
            range,
            role: GentufaBlockRole::Normal,
            error_index: None,
            token_kind: token_kind_for_text(display_text),
            raw_text: display_text.to_owned(),
            display_text: display_text.to_owned(),
        })
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
        new!(BlockTreeNode {
            keep_structural_host: false,
            id: RawSyntaxNodeId(id),
            field_label,
            node_ids: vec![RawSyntaxNodeId(id)],
            label: label.to_owned(),
            role: GentufaBlockRole::Normal,
            error_index: None,
            token_kind: token_kind_for_text(display_text),
            ref_markers: Vec::new(),
            span: Some(range),
            leaf_parts: vec![test_leaf_part(id + 100, display_text, range)],
            node_types: vec![label.to_owned()],
            ancestors: Vec::new(),
            depth: 0,
            raw_text: display_text.to_owned(),
            leaf_word: Some(display_text.to_owned()),
            computed_gloss: None,
            children: Vec::new(),
        })
    }
}
