//! Shared web/API view models and gentufa parser facade.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_diagnostics::{Diagnostic, DiagnosticPhase};
use jbotci_dialect::{DialectDefinition, parse_dialect_definition};
use jbotci_morphology::{
    MorphologyOptions, PhonemeRenderOptions, Word, WordKind, WordLike, WordLikeData,
    segment_words_with_modifiers_with_options_and_source_id_attempt,
};
use jbotci_output::{
    BracketRenderOptions, GlyphStyle, ReferenceDisplayModel, ReferenceName as OutputReferenceName,
    ReferenceSlotName as OutputReferenceSlotName, TreeRenderOptions, ipa_morphology_text,
    pretty_brackets_with_options, reference_display_model_for_syntax_tree,
};
use jbotci_semantics::references::{RawSyntaxNodeId, ReferenceAnalysis, SyntaxNodeMetadata};
use jbotci_source::{SourceId, SourceSpan};
use jbotci_syntax::ast::{
    AtomRef as SyntaxAtomRef, NodeRef as SyntaxNodeRef, TextSyntax, TreeNode as SyntaxTreeNode,
};
use jbotci_syntax::{
    ParseOptions, WithIndicators, parse_syntax_tree_with_source_and_options_attempt,
};
use jbotci_tree::TreeVisitor;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum GentufaWebViewMode {
    #[default]
    Blocks,
    Tree,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum GentufaScript {
    #[default]
    Latin,
    Cyrillic,
    Zbalermorna,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct GentufaWebOptions {
    pub dialect: Option<String>,
    pub view_mode: GentufaWebViewMode,
    pub script: GentufaScript,
    pub show_elided: bool,
    pub show_glosses: bool,
    pub show_definitions: bool,
    pub phonemes: PhonemeRenderOptions,
}

impl Default for GentufaWebOptions {
    #[requires(true)]
    #[ensures(ret.view_mode == GentufaWebViewMode::Blocks)]
    #[ensures(ret.script == GentufaScript::Latin)]
    fn default() -> Self {
        Self {
            dialect: None,
            view_mode: GentufaWebViewMode::Blocks,
            script: GentufaScript::Latin,
            show_elided: false,
            show_glosses: true,
            show_definitions: false,
            phonemes: PhonemeRenderOptions::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct GentufaWebRequest {
    pub text: String,
    pub options: GentufaWebOptions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
#[invariant(true)]
#[invariant(::Blank => true)]
#[invariant(::Success(_) => true)]
#[invariant(::Error(_) => true)]
pub enum GentufaWebResult {
    Blank,
    Success(GentufaSuccess),
    Error(GentufaError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct GentufaSuccess {
    pub ipa_text: String,
    pub surface_text: String,
    pub brackets_text: String,
    pub blocks_layout: GentufaBlocksLayout,
    pub tree_rows: Vec<GentufaTreeRow>,
    pub diagnostics: Vec<Diagnostic>,
    pub features: WebFeatureAvailability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct GentufaError {
    pub phase: Option<DiagnosticPhase>,
    pub message: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct WebFeatureAvailability {
    pub gentufa: bool,
    pub cukta: bool,
    pub vlacku: bool,
    pub glosses: bool,
    pub definitions: bool,
    pub rafsi_breakdown: bool,
    pub lean: bool,
}

impl Default for WebFeatureAvailability {
    #[requires(true)]
    #[ensures(ret.gentufa)]
    #[ensures(!ret.cukta)]
    #[ensures(!ret.vlacku)]
    fn default() -> Self {
        Self {
            gentufa: true,
            cukta: false,
            vlacku: false,
            glosses: false,
            definitions: false,
            rafsi_breakdown: false,
            lean: false,
        }
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
#[invariant(::Modal(..) => true)]
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
            Self::Modal(words) => words.join(" "),
            Self::Fai => "fai".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct GentufaBlocksLayout {
    pub blocks: Vec<GentufaBlock>,
    pub max_col: usize,
    pub max_row: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct GentufaBlock {
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct GentufaTreeRow {
    pub depth: usize,
    pub label: String,
    pub color: String,
    pub cells: Vec<GentufaCell>,
    pub computed_gloss: Option<String>,
    pub ref_markers: Vec<ReferenceMarker>,
    pub glosses: Vec<String>,
    pub definition: Option<String>,
    pub rafsi_breakdown: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct GentufaCell {
    pub text: String,
    pub is_word: bool,
    pub quoted: bool,
    pub tooltip: Option<String>,
    pub is_elided: bool,
    pub transform: Option<TransformInfo>,
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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Dialect(_) => true)]
pub enum GentufaWebError {
    #[error("invalid dialect definition: {0}")]
    Dialect(String),
}

#[requires(true)]
#[ensures(matches!(ret, GentufaWebResult::Blank) == request.text.trim().is_empty())]
pub fn parse_gentufa_for_web(request: &GentufaWebRequest) -> GentufaWebResult {
    let source = request.text.as_str();
    if source.trim().is_empty() {
        return GentufaWebResult::Blank;
    }

    let dialect = match dialect_definition(request.options.dialect.as_deref()) {
        Ok(dialect) => dialect,
        Err(error) => {
            return GentufaWebResult::Error(GentufaError {
                phase: None,
                message: error.to_string(),
                diagnostics: Vec::new(),
            });
        }
    };

    let source_id = Some(SourceId("<web-input>".to_owned()));
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let morphology_attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
        source,
        &morphology_options,
        source_id.clone(),
    )
    .into_data();
    let mut diagnostics = morphology_attempt
        .warnings
        .iter()
        .map(|warning| warning.to_diagnostic(source_id.clone(), source))
        .collect::<Vec<_>>();
    let words = match morphology_attempt.result {
        Ok(words) => words,
        Err(error) => {
            diagnostics.push(error.to_diagnostic(source_id, source));
            return GentufaWebResult::Error(GentufaError {
                phase: Some(DiagnosticPhase::Morphology),
                message: error.to_string(),
                diagnostics,
            });
        }
    };

    let parse_options = ParseOptions::default().with_dialect_definition(&dialect);
    let syntax_attempt =
        parse_syntax_tree_with_source_and_options_attempt(&words, source, &parse_options);
    let parsed = match syntax_attempt.result {
        Ok(parsed) => parsed,
        Err(error) => {
            diagnostics.push(error.to_diagnostic(source_id, source));
            return GentufaWebResult::Error(GentufaError {
                phase: Some(DiagnosticPhase::Syntax),
                message: error.to_string(),
                diagnostics,
            });
        }
    };
    diagnostics.extend(
        parsed
            .warnings
            .iter()
            .map(|warning| warning.to_diagnostic(Some(SourceId("<web-input>".to_owned())), source)),
    );

    let analysis = match ReferenceAnalysis::analyze(&parsed.parse_tree) {
        Ok(analysis) => analysis,
        Err(error) => {
            return GentufaWebResult::Error(GentufaError {
                phase: Some(DiagnosticPhase::Syntax),
                message: error.to_string(),
                diagnostics,
            });
        }
    };
    let leaves = rendered_leaves(&parsed.parse_tree, source, &request.options);
    let reference_model = reference_display_model_for_syntax_tree(
        &analysis,
        &parsed.parse_tree,
        source,
        tree_render_options(request.options.phonemes),
    );
    let blocks_layout = blocks_layout(
        &analysis,
        &reference_model,
        source,
        &leaves,
        &request.options,
    );
    let tree_rows = tree_rows(
        &analysis,
        &reference_model,
        source,
        &leaves,
        &request.options,
    );
    let ipa_text = ipa_morphology_text(&words, source).unwrap_or_else(|error| error.to_string());
    let brackets_text = pretty_brackets_with_options(
        &parsed.parse_tree,
        source,
        BracketRenderOptions {
            color: false,
            phonemes: request.options.phonemes,
            glyphs: GlyphStyle::Unicode,
            decompose_lujvo: false,
            insert_hair_space: true,
        },
    )
    .unwrap_or_else(|error| error.to_string());

    GentufaWebResult::Success(GentufaSuccess {
        ipa_text,
        surface_text: leaves
            .iter()
            .map(|leaf| leaf.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        brackets_text,
        blocks_layout,
        tree_rows,
        diagnostics,
        features: WebFeatureAvailability::default(),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn dialect_definition(source: Option<&str>) -> Result<DialectDefinition, GentufaWebError> {
    match source.map(str::trim).filter(|source| !source.is_empty()) {
        Some(source) => parse_dialect_definition(source)
            .map_err(|error| GentufaWebError::Dialect(error.to_string())),
        None => Ok(DialectDefinition::default()),
    }
}

#[requires(true)]
#[ensures(ret.show_refs)]
fn tree_render_options(phonemes: PhonemeRenderOptions) -> TreeRenderOptions {
    TreeRenderOptions {
        color: false,
        indent: 2,
        phonemes,
        glyphs: GlyphStyle::Unicode,
        show_spans: false,
        show_refs: true,
        decompose_lujvo: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct RenderedLeaf {
    range: WebSourceRange,
    text: String,
}

#[derive(Debug)]
#[invariant(true)]
struct LeafCollector<'source, 'options> {
    source: &'source str,
    options: &'options GentufaWebOptions,
    leaves: Vec<RenderedLeaf>,
}

impl<'source, 'options> LeafCollector<'source, 'options> {
    #[requires(true)]
    #[ensures(ret.source == source)]
    fn new(source: &'source str, options: &'options GentufaWebOptions) -> Self {
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

#[requires(true)]
#[ensures(true)]
fn rendered_leaves(
    syntax: &TextSyntax,
    source: &str,
    options: &GentufaWebOptions,
) -> Vec<RenderedLeaf> {
    let mut collector = LeafCollector::new(source, options);
    syntax.visit_in_order(&mut collector);
    collector.finish()
}

#[requires(true)]
#[ensures(ret.max_col >= ret.blocks.iter().map(|block| block.col + block.col_span).max().unwrap_or(0))]
fn blocks_layout(
    analysis: &ReferenceAnalysis<'_>,
    reference_model: &ReferenceDisplayModel,
    source: &str,
    leaves: &[RenderedLeaf],
    options: &GentufaWebOptions,
) -> GentufaBlocksLayout {
    let child_map = syntax_child_map(analysis);
    let root_id = analysis.syntax_index.root().0;
    let Some(root) = build_block_tree_node(
        analysis,
        reference_model,
        &child_map,
        root_id,
        source,
        leaves,
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
    let blocks = assign_block_colors(temp_blocks, max_depth);
    GentufaBlocksLayout {
        blocks,
        max_col,
        max_row: max_depth + 1,
    }
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
struct BlockTemp {
    id: RawSyntaxNodeId,
    parent_id: Option<RawSyntaxNodeId>,
    child_ids: Vec<RawSyntaxNodeId>,
    block: GentufaBlock,
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
fn build_block_tree_node(
    analysis: &ReferenceAnalysis<'_>,
    reference_model: &ReferenceDisplayModel,
    child_map: &[Vec<RawSyntaxNodeId>],
    id: RawSyntaxNodeId,
    source: &str,
    leaves: &[RenderedLeaf],
    options: &GentufaWebOptions,
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
                options,
            )
        })
        .collect::<Vec<_>>();
    let span = range_from_spans(metadata.source_spans.iter());
    if span.is_none() && children.is_empty() && !options.show_elided {
        return None;
    }
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
        options,
    );
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
        is_elided: span.is_none(),
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
        computed_gloss: None,
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
    options: &GentufaWebOptions,
) -> Vec<BlockLeafPart> {
    metadata
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
                raw_text: source_text_for_range(source, Some(range)),
                display_text,
            })
        })
        .collect()
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
    if node.children.is_empty() && node.leaf_parts.len() > 1 {
        return node.depth + 1;
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
fn push_positioned_blocks(
    node: &BlockTreeNode,
    col: usize,
    max_depth: usize,
    parent_id: Option<RawSyntaxNodeId>,
    blocks: &mut Vec<BlockTemp>,
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
                !node
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
        !node
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
fn push_split_leaf_blocks(
    node: &BlockTreeNode,
    col: usize,
    max_depth: usize,
    parent_id: Option<RawSyntaxNodeId>,
    blocks: &mut Vec<BlockTemp>,
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
fn synthetic_leaf_block(
    node: &BlockTreeNode,
    part: &BlockLeafPart,
    col: usize,
    row: usize,
    row_span: usize,
) -> GentufaBlock {
    GentufaBlock {
        block_id: format!("n{}", part.id.0),
        label: part.display_text.clone(),
        is_leaf: true,
        is_elided: false,
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
fn push_leaf_or_structural_block(
    node: &BlockTreeNode,
    col: usize,
    max_depth: usize,
    parent_id: Option<RawSyntaxNodeId>,
    blocks: &mut Vec<BlockTemp>,
) {
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
fn block_from_tree_node(
    node: &BlockTreeNode,
    is_leaf: bool,
    col: usize,
    col_span: usize,
    row: usize,
    row_span: usize,
    display_text: String,
) -> GentufaBlock {
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
    }
}

#[requires(true)]
#[ensures(true)]
fn assign_block_colors(blocks: Vec<BlockTemp>, max_depth: usize) -> Vec<GentufaBlock> {
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
fn leaf_parent_hues(blocks: &[BlockTemp]) -> Vec<(Option<RawSyntaxNodeId>, f64)> {
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
fn tree_rows(
    analysis: &ReferenceAnalysis<'_>,
    reference_model: &ReferenceDisplayModel,
    source: &str,
    leaves: &[RenderedLeaf],
    options: &GentufaWebOptions,
) -> Vec<GentufaTreeRow> {
    let mut rows = Vec::new();
    for raw_id in 0..analysis.syntax_index.node_count() {
        let id = RawSyntaxNodeId(raw_id);
        let Some(metadata) = analysis.syntax_index.metadata(id) else {
            continue;
        };
        if metadata.source_spans.is_empty() && !options.show_elided {
            continue;
        }
        let label = analysis
            .syntax_index
            .node(id)
            .map(|node| syntax_constructor_name(node.constructor_name()).to_owned())
            .unwrap_or_else(|| "Node".to_owned());
        if !tree_row_should_render(&label) {
            continue;
        }
        let text = display_text_for_spans(&metadata.source_spans, leaves, source, options);
        rows.push(GentufaTreeRow {
            depth: metadata.depth,
            label,
            color: color_for_node(metadata.depth, metadata.preorder),
            cells: vec![GentufaCell {
                text,
                is_word: !metadata.source_spans.is_empty(),
                quoted: false,
                tooltip: None,
                is_elided: metadata.source_spans.is_empty(),
                transform: None,
            }],
            computed_gloss: None,
            ref_markers: reference_markers_for_node(reference_model, id),
            glosses: Vec::new(),
            definition: None,
            rafsi_breakdown: Vec::new(),
        });
    }
    rows
}

#[requires(true)]
#[ensures(true)]
fn tree_row_should_render(label: &str) -> bool {
    !matches!(label, "PredicateTail" | "PredicateTail1" | "PredicateTail2")
}

#[requires(true)]
#[ensures(true)]
fn reference_markers_for_node(
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
fn reference_label_from_output(label: &OutputReferenceName) -> ReferenceLabel {
    ReferenceLabel {
        stem: label.stem.clone(),
        occurrence: label.occurrence,
        slot: label.slot.as_ref().map(reference_slot_label_from_output),
    }
}

#[requires(true)]
#[ensures(true)]
fn reference_slot_label_from_output(slot: &OutputReferenceSlotName) -> ReferenceSlotLabel {
    match slot {
        OutputReferenceSlotName::Numbered(place) => ReferenceSlotLabel::Numbered(*place),
        OutputReferenceSlotName::Modal(words) => ReferenceSlotLabel::Modal(words.clone()),
        OutputReferenceSlotName::Fai => ReferenceSlotLabel::Fai,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn color_for_node(depth: usize, preorder: usize) -> String {
    const PALETTE: [&str; 8] = [
        "#7fb3d5", "#82c596", "#f2c36b", "#d9927a", "#b48bd4", "#75c5bd", "#d8a35d", "#9eb36a",
    ];
    PALETTE[(depth + preorder) % PALETTE.len()].to_owned()
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
fn syntax_constructor_name(constructor: &'static str) -> &'static str {
    constructor.strip_suffix("Syntax").unwrap_or(constructor)
}

#[requires(true)]
#[ensures(true)]
fn display_text_for_spans(
    spans: &[SourceSpan],
    leaves: &[RenderedLeaf],
    source: &str,
    options: &GentufaWebOptions,
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
#[ensures(true)]
fn source_text_for_range(source: &str, range: Option<WebSourceRange>) -> String {
    range
        .and_then(|range| source.get(range.byte_start..range.byte_end))
        .unwrap_or("")
        .to_owned()
}

#[requires(true)]
#[ensures(ret.is_none_or(|range| range.byte_start <= range.byte_end && range.char_start <= range.char_end))]
fn range_from_spans<'a, I>(spans: I) -> Option<WebSourceRange>
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
fn range_from_span(span: &SourceSpan) -> WebSourceRange {
    WebSourceRange {
        byte_start: span.byte_start,
        byte_end: span.byte_end,
        char_start: span.char_start,
        char_end: span.char_end,
    }
}

#[requires(true)]
#[ensures(true)]
fn render_word_like(word_like: &WordLike, source: &str, options: &GentufaWebOptions) -> String {
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
fn render_word(word: &Word, options: &GentufaWebOptions) -> String {
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
fn render_loose_latin_surface(text: String, options: &GentufaWebOptions) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};
    use jbotci_morphology::{GlideMark, StressMark};
    use std::collections::BTreeSet;

    #[requires(!text.trim().is_empty())]
    #[ensures(true)]
    fn parse_success(text: &str) -> GentufaSuccess {
        let request = GentufaWebRequest {
            text: text.to_owned(),
            options: GentufaWebOptions::default(),
        };
        let result = parse_gentufa_for_web(&request);
        let GentufaWebResult::Success(success) = result else {
            panic!("expected successful parse");
        };
        success
    }

    #[requires(true)]
    #[ensures(true)]
    fn tree_reference_keys(
        success: &GentufaSuccess,
        role: ReferenceMarkerRole,
    ) -> BTreeSet<String> {
        success
            .tree_rows
            .iter()
            .flat_map(|row| row.ref_markers.iter())
            .filter(|marker| marker.role == role)
            .map(|marker| marker.label.full_key())
            .collect()
    }

    #[requires(true)]
    #[ensures(true)]
    fn all_reference_stems(success: &GentufaSuccess) -> BTreeSet<String> {
        success
            .tree_rows
            .iter()
            .flat_map(|row| row.ref_markers.iter())
            .map(|marker| marker.label.stem.clone())
            .collect()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn blank_input_returns_blank() {
        let request = GentufaWebRequest {
            text: "  \n ".to_owned(),
            options: GentufaWebOptions::default(),
        };
        assert_eq!(parse_gentufa_for_web(&request), GentufaWebResult::Blank);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn simple_parse_builds_blocks_and_tree_rows() {
        let request = GentufaWebRequest {
            text: "mi klama".to_owned(),
            options: GentufaWebOptions::default(),
        };
        let result = parse_gentufa_for_web(&request);
        let GentufaWebResult::Success(success) = result else {
            panic!("expected successful parse");
        };
        assert!(!success.blocks_layout.blocks.is_empty());
        assert!(!success.tree_rows.is_empty());
        assert!(success.ipa_text.contains("ˈkla.ma"));
        assert!(success.surface_text.contains("mi"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn simple_parse_builds_v0_style_block_spans() {
        let request = GentufaWebRequest {
            text: "mi klama le zarci".to_owned(),
            options: GentufaWebOptions::default(),
        };
        let result = parse_gentufa_for_web(&request);
        let GentufaWebResult::Success(success) = result else {
            panic!("expected successful parse");
        };
        let leaf_blocks = success
            .blocks_layout
            .blocks
            .iter()
            .filter(|block| block.is_leaf)
            .collect::<Vec<_>>();
        let nonleaf_blocks = success
            .blocks_layout
            .blocks
            .iter()
            .filter(|block| !block.is_leaf)
            .collect::<Vec<_>>();
        assert_eq!(success.ipa_text, "mi ˈkla.ma le ˈzar.ʃi");
        assert_eq!(success.blocks_layout.max_col, 4);
        assert_eq!(leaf_blocks.len(), 4);
        assert!(
            leaf_blocks
                .iter()
                .all(|block| block.row + block.row_span == success.blocks_layout.max_row)
        );
        assert!(nonleaf_blocks.iter().all(|block| block.row_span == 1));
        assert!(
            success
                .blocks_layout
                .blocks
                .iter()
                .take_while(|block| block.is_leaf)
                .count()
                >= leaf_blocks.len()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tree_rows_keep_depth_order_color_and_math_label_data() {
        let success = parse_success("mi klama le zarci");
        assert_eq!(success.tree_rows.first().map(|row| row.depth), Some(0));
        assert!(
            success
                .tree_rows
                .iter()
                .all(|row| row.color.starts_with('#'))
        );
        assert!(
            success
                .tree_rows
                .iter()
                .all(|row| !row.label.starts_with("PredicateTail"))
        );
        assert!(success.blocks_layout.blocks.iter().any(|block| {
            block.ref_markers.iter().any(|marker| {
                marker.role == ReferenceMarkerRole::Referent
                    && marker.label.stem == "k"
                    && marker.label.slot == Some(ReferenceSlotLabel::Numbered(1))
            })
        }));
        assert!(success.blocks_layout.blocks.iter().any(|block| {
            block.ref_markers.iter().any(|marker| {
                marker.role == ReferenceMarkerRole::Reference
                    && marker.label.stem == "k"
                    && marker.label.slot.is_none()
            })
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bracket_output_inserts_hair_spaces() {
        let success = parse_success("mi klama le zarci");
        assert!(success.brackets_text.contains('\u{200a}'));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_labels_match_cli_tree_model_for_anaphora() {
        let success = parse_success("mi klama le zarci i do klama ri");
        let referents = tree_reference_keys(&success, ReferenceMarkerRole::Referent);
        let references = tree_reference_keys(&success, ReferenceMarkerRole::Reference);
        assert!(referents.contains("k1<1>"));
        assert!(referents.contains("k1<2>"));
        assert!(referents.contains("k2<1>"));
        assert!(referents.contains("k2<2>"));
        assert!(references.contains("k1"));
        assert!(references.contains("k2"));
        assert!(references.contains("ri1"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_labels_match_cli_tree_model_for_modal_places() {
        let success = parse_success("mi ta'i do klama");
        let referents = tree_reference_keys(&success, ReferenceMarkerRole::Referent);
        let references = tree_reference_keys(&success, ReferenceMarkerRole::Reference);
        assert!(referents.contains("k<1>"));
        assert!(referents.contains("k<ta'i>"));
        assert!(references.contains("k"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_labels_match_cli_tree_model_for_se_conversion() {
        let success = parse_success("mi se klama do");
        let referents = tree_reference_keys(&success, ReferenceMarkerRole::Referent);
        let references = tree_reference_keys(&success, ReferenceMarkerRole::Reference);
        assert!(referents.contains("k<1>"));
        assert!(referents.contains("k<2>"));
        assert!(references.contains("k"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_labels_match_cli_tree_model_for_goi_and_goi_reference() {
        let success = parse_success("mi goi ko'a klama ko'a");
        let referents = tree_reference_keys(&success, ReferenceMarkerRole::Referent);
        let references = tree_reference_keys(&success, ReferenceMarkerRole::Reference);
        assert!(referents.contains("k<1>"));
        assert!(referents.contains("k<2>"));
        assert!(references.contains("k"));
        assert!(references.contains("ko'a1"));
        assert!(references.contains("ko'a2"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_labels_match_cli_tree_model_for_go_i() {
        let success = parse_success("mi klama i do go'i");
        let referents = tree_reference_keys(&success, ReferenceMarkerRole::Referent);
        let references = tree_reference_keys(&success, ReferenceMarkerRole::Reference);
        assert!(referents.contains("k<1>"));
        assert!(referents.contains("go'i1<1>"));
        assert!(references.contains("k"));
        assert!(references.contains("go'i1"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_labels_match_cli_tree_model_for_cei() {
        let success = parse_success("ti klama cei broda");
        let referents = tree_reference_keys(&success, ReferenceMarkerRole::Referent);
        let references = tree_reference_keys(&success, ReferenceMarkerRole::Reference);
        assert!(referents.contains("k<1>"));
        assert!(references.contains("k"));
        assert!(references.contains("b"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_labels_do_not_use_web_only_invented_names() {
        let success = parse_success("mi klama le zarci i do klama ri");
        let stems = all_reference_stems(&success);
        assert!(!stems.contains("q"));
        assert!(!stems.contains("r"));
        assert!(!stems.contains("x"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_dialect_returns_error() {
        let request = GentufaWebRequest {
            text: "mi klama".to_owned(),
            options: GentufaWebOptions {
                dialect: Some("not-a-list".to_owned()),
                ..GentufaWebOptions::default()
            },
        };
        let result = parse_gentufa_for_web(&request);
        assert!(matches!(result, GentufaWebResult::Error(_)));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cyrillic_script_renders_words() {
        let request = GentufaWebRequest {
            text: "mi klama".to_owned(),
            options: GentufaWebOptions {
                script: GentufaScript::Cyrillic,
                ..GentufaWebOptions::default()
            },
        };
        let result = parse_gentufa_for_web(&request);
        let GentufaWebResult::Success(success) = result else {
            panic!("expected successful parse");
        };
        assert!(success.surface_text.contains("ми"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zbalermorna_script_renders_private_use_letters() {
        let request = GentufaWebRequest {
            text: "mi klama".to_owned(),
            options: GentufaWebOptions {
                script: GentufaScript::Zbalermorna,
                ..GentufaWebOptions::default()
            },
        };
        let result = parse_gentufa_for_web(&request);
        let GentufaWebResult::Success(success) = result else {
            panic!("expected successful parse");
        };
        assert!(
            success
                .surface_text
                .chars()
                .any(|ch| ('\u{ed80}'..='\u{edff}').contains(&ch))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn phoneme_options_remove_stress_and_glide_marks() {
        let request = GentufaWebRequest {
            text: "brodau".to_owned(),
            options: GentufaWebOptions {
                phonemes: PhonemeRenderOptions {
                    mark_stress: StressMark::None,
                    mark_glides: GlideMark::None,
                },
                ..GentufaWebOptions::default()
            },
        };
        let result = parse_gentufa_for_web(&request);
        let GentufaWebResult::Success(success) = result else {
            panic!("expected successful parse");
        };
        assert!(!success.surface_text.contains('ĭ'));
    }
}
