//! Shared web/API view models and gentufa parser facade.

use std::collections::BTreeMap;
use std::sync::OnceLock;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_cll::{
    CllBlock, CllSearchChunkKind, CuktaSearchMode, CuktaTargetFilter, DEFAULT_CUKTA_SECTION_ID,
    DEFAULT_CUKTA_WEB_RESULT_COUNT, MAX_CUKTA_RESULT_COUNT, cll_first_section_id,
    cll_index_entries, cll_lookup_section, cll_next_section_id, cll_previous_section_id,
    cll_resolve_section_reference, cll_search_all_chunks, cll_search_chunk_href,
    cll_section_chapter_title, cukta_search, embedded_cll_site, format_section_display_title,
    truncate_preview,
};
use jbotci_diagnostics::{Diagnostic, DiagnosticPhase};
use jbotci_dialect::{DialectDefinition, parse_dialect_definition};
use jbotci_dictionary::{Dictionary, DictionaryEntry};
use jbotci_embedding_inputs::embedding_input_corpus_json;
use jbotci_gentufa::{
    ElidedTerminator, RenderedLeaf, blocks_layout as build_blocks_layout,
    display_text_for_spans as gentufa_display_text_for_spans,
    elided_terminators as build_elided_terminators, range_from_spans as gentufa_range_from_spans,
    reference_markers_for_node as gentufa_reference_markers_for_node,
    rendered_leaves as build_rendered_leaves,
    syntax_constructor_name as gentufa_syntax_constructor_name,
};
pub use jbotci_gentufa::{
    GentufaBlockAnnotation, GentufaBlockOptions, GentufaScript, ReferenceLabel, ReferenceMarker,
    ReferenceMarkerRole, ReferenceSlotLabel, TransformInfo, WebSourceRange,
    reference_slot_display_text,
};
use jbotci_jvozba::{
    JvozbaInput as JvozbaSourceInput, JvozbaMode, JvozbaSegment, JvozbaSegmentKind,
    build_best_jvozba_detailed,
};
use jbotci_morphology::{
    MorphologyOptions, PhonemeRenderOptions, WordLike, ends_with_consonant,
    segment_words_with_modifiers_with_options_and_source_id_attempt,
};
use jbotci_output::{
    BracketRenderOptions, BracketSourceFragment, BracketSourceRange, GlyphStyle,
    ReferenceDisplayModel, TreeRenderOptions, ipa_morphology_text,
    pretty_bracket_source_fragments_with_options, pretty_brackets_with_options,
    reference_display_model_for_syntax_tree,
};
use jbotci_search::vlacku::{
    DEFAULT_VLACKU_RESULT_COUNT, OFFICIAL_WORD_VOTE_THRESHOLD, ParsedWordDictionaryMatch,
    VlackuCard, VlackuCompositionKind, VlackuRequest, VlackuSearchOptions, dictionary_entry_card,
    dictionary_matches_for_word_likes, filter_vlacku_cards, format_votes,
    grouped_word_type_filter_key, is_brivla_like, normalize_word_type_filter, run_vlacku_requests,
};
use jbotci_semantics::references::{RawSyntaxNodeId, ReferenceAnalysis};
use jbotci_source::SourceId;
use jbotci_syntax::{ParseOptions, parse_syntax_tree_with_source_and_options_attempt};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type GentufaBlock = jbotci_gentufa::GentufaBlock<DictionaryTooltipCard>;
pub type GentufaBlocksLayout = jbotci_gentufa::GentufaBlocksLayout<DictionaryTooltipCard>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum GentufaWebViewMode {
    #[default]
    Blocks,
    Tree,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct GentufaWebState {
    pub text: String,
    pub dialect: Option<String>,
    pub view_mode: GentufaWebViewMode,
    pub show_elided: bool,
    pub show_glosses: bool,
}

impl Default for GentufaWebState {
    #[requires(true)]
    #[ensures(ret.text.is_empty())]
    #[ensures(ret.view_mode == GentufaWebViewMode::Blocks)]
    fn default() -> Self {
        Self {
            text: String::new(),
            dialect: None,
            view_mode: GentufaWebViewMode::Blocks,
            show_elided: false,
            show_glosses: true,
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
    pub bracket_fragments: Vec<GentufaBracketFragment>,
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
    #[ensures(ret.cukta)]
    #[ensures(ret.vlacku)]
    fn default() -> Self {
        Self {
            gentufa: true,
            cukta: true,
            vlacku: true,
            glosses: false,
            definitions: false,
            rafsi_breakdown: false,
            lean: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
#[invariant(true)]
#[invariant(::Text { .. } => true)]
#[invariant(::Span { .. } => true)]
pub enum GentufaBracketFragment {
    Text {
        text: String,
        elided: bool,
    },
    Span {
        color: Option<String>,
        href: Option<String>,
        tooltip: Option<DictionaryTooltipCard>,
        children: Vec<GentufaBracketFragment>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct DictionaryTooltipCard {
    pub word: String,
    pub display_word: String,
    pub href: String,
    pub word_type: String,
    pub word_type_key: String,
    pub selmaho: Option<String>,
    pub ipa: Option<String>,
    pub similarity: Option<String>,
    pub votes: VlackuVoteDisplay,
    pub rafsi: Vec<String>,
    pub glosses: Vec<String>,
    pub definition: Vec<VlackuInline>,
    pub notes: Vec<VlackuInline>,
    pub decomposition: Vec<VlackuCompositionPiece>,
    pub can_add_to_jvozba: bool,
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
    let block_options = gentufa_block_options(&request.options);
    let leaves = build_rendered_leaves(&parsed.parse_tree, source, &block_options);
    let elided_terminators =
        build_elided_terminators(&analysis, &parsed.parse_tree, &block_options);
    let mut dictionary_annotations =
        dictionary_annotations_for_words(jbotci_dictionary_data::english(), &words, "");
    dictionary_annotations.extend(dictionary_annotations_for_elided_terminators(
        &elided_terminators,
        "",
    ));
    let reference_model = reference_display_model_for_syntax_tree(
        &analysis,
        &parsed.parse_tree,
        source,
        tree_render_options(request.options.phonemes, request.options.show_elided),
    );
    let blocks_layout = build_blocks_layout(
        &analysis,
        &reference_model,
        source,
        &leaves,
        &elided_terminators,
        &dictionary_annotations,
        &block_options,
    );
    let tree_rows = tree_rows(
        &analysis,
        &reference_model,
        source,
        &leaves,
        &elided_terminators,
        &dictionary_annotations,
        &block_options,
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
            show_elided: request.options.show_elided,
        },
    )
    .unwrap_or_else(|error| error.to_string());
    let bracket_fragments = pretty_bracket_source_fragments_with_options(
        &parsed.parse_tree,
        source,
        BracketRenderOptions {
            color: false,
            phonemes: request.options.phonemes,
            glyphs: GlyphStyle::Unicode,
            decompose_lujvo: false,
            insert_hair_space: true,
            show_elided: request.options.show_elided,
        },
    )
    .map(|fragments| {
        gentufa_bracket_fragments_from_source(&fragments, &blocks_layout, &dictionary_annotations)
    })
    .unwrap_or_else(|_| {
        vec![GentufaBracketFragment::Text {
            text: brackets_text.clone(),
            elided: false,
        }]
    });

    GentufaWebResult::Success(GentufaSuccess {
        ipa_text,
        surface_text: leaves
            .iter()
            .map(|leaf| leaf.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        brackets_text,
        bracket_fragments,
        blocks_layout,
        tree_rows,
        diagnostics,
        features: WebFeatureAvailability {
            glosses: true,
            definitions: true,
            ..WebFeatureAvailability::default()
        },
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
fn tree_render_options(phonemes: PhonemeRenderOptions, show_elided: bool) -> TreeRenderOptions {
    TreeRenderOptions {
        color: false,
        indent: 2,
        phonemes,
        glyphs: GlyphStyle::Unicode,
        show_spans: false,
        show_refs: true,
        decompose_lujvo: false,
        show_elided,
    }
}

#[requires(true)]
#[ensures(ret.script == options.script)]
fn gentufa_block_options(options: &GentufaWebOptions) -> GentufaBlockOptions {
    GentufaBlockOptions {
        script: options.script,
        show_elided: options.show_elided,
        phonemes: options.phonemes,
    }
}

#[requires(true)]
#[ensures(true)]
fn tooltip_definition_text(card: &DictionaryTooltipCard) -> Option<String> {
    let text = inline_plain_text(&card.definition);
    (!text.trim().is_empty()).then_some(text)
}

#[requires(true)]
#[ensures(true)]
fn inline_plain_text(inlines: &[VlackuInline]) -> String {
    let mut output = String::new();
    for inline in inlines {
        append_inline_plain_text(inline, &mut output);
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn append_inline_plain_text(inline: &VlackuInline, output: &mut String) {
    match inline.as_data() {
        data!(VlackuInline::Text(text)) => output.push_str(text),
        data!(VlackuInline::WordRef { label, .. }) => output.push_str(label),
        data!(VlackuInline::Math(math)) => {
            for part in &math.parts {
                match part.as_data() {
                    data!(VlackuMathPart::Text(text)) | data!(VlackuMathPart::Operator(text)) => {
                        output.push_str(text)
                    }
                    data!(VlackuMathPart::Variable { stem, subscript }) => {
                        output.push_str(stem);
                        if let Some(subscript) = subscript {
                            output.push('_');
                            output.push_str(subscript);
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn tree_rows(
    analysis: &ReferenceAnalysis<'_>,
    reference_model: &ReferenceDisplayModel,
    source: &str,
    leaves: &[RenderedLeaf],
    elided_terminators: &[ElidedTerminator],
    dictionary_annotations: &[GentufaBlockAnnotation<DictionaryTooltipCard>],
    options: &GentufaBlockOptions,
) -> Vec<GentufaTreeRow> {
    let mut rows = Vec::new();
    for raw_id in 0..analysis.syntax_index.node_count() {
        let id = RawSyntaxNodeId(raw_id);
        let Some(metadata) = analysis.syntax_index.metadata(id) else {
            continue;
        };
        if metadata.source_spans.is_empty() {
            continue;
        }
        let label = analysis
            .syntax_index
            .node(id)
            .map(|node| gentufa_syntax_constructor_name(node.constructor_name()).to_owned())
            .unwrap_or_else(|| "Node".to_owned());
        if !tree_row_should_render(&label) {
            continue;
        }
        let text = gentufa_display_text_for_spans(&metadata.source_spans, leaves, source, options);
        let annotation = gentufa_range_from_spans(metadata.source_spans.iter()).and_then(|range| {
            annotation_for_range_and_text(dictionary_annotations, Some(range), None)
        });
        rows.push(GentufaTreeRow {
            depth: metadata.depth,
            label,
            color: color_for_node(metadata.depth, metadata.preorder),
            cells: vec![GentufaCell {
                text,
                is_word: !metadata.source_spans.is_empty(),
                quoted: false,
                tooltip: None,
                is_elided: false,
                transform: None,
            }],
            computed_gloss: None,
            ref_markers: gentufa_reference_markers_for_node(reference_model, id),
            glosses: annotation
                .map(|annotation| annotation.glosses.clone())
                .unwrap_or_default(),
            definition: annotation.and_then(|annotation| annotation.definition.clone()),
            rafsi_breakdown: Vec::new(),
        });
        for terminator in elided_terminators
            .iter()
            .filter(|terminator| terminator.parent_id == id)
        {
            let annotation = annotation_for_range_and_text(
                dictionary_annotations,
                Some(terminator.range),
                Some(&terminator.text),
            );
            rows.push(GentufaTreeRow {
                depth: metadata.depth + 1,
                label: "Cmavo".to_owned(),
                color: color_for_node(metadata.depth + 1, metadata.preorder),
                cells: vec![GentufaCell {
                    text: terminator.text.clone(),
                    is_word: true,
                    quoted: false,
                    tooltip: None,
                    is_elided: true,
                    transform: None,
                }],
                computed_gloss: None,
                ref_markers: Vec::new(),
                glosses: annotation
                    .map(|annotation| annotation.glosses.clone())
                    .unwrap_or_default(),
                definition: annotation.and_then(|annotation| annotation.definition.clone()),
                rafsi_breakdown: Vec::new(),
            });
        }
    }
    rows
}

#[requires(true)]
#[ensures(true)]
fn tree_row_should_render(label: &str) -> bool {
    !matches!(label, "PredicateTail" | "PredicateTail1" | "PredicateTail2")
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn color_for_node(depth: usize, preorder: usize) -> String {
    const PALETTE: [&str; 8] = [
        "#7fb3d5", "#82c596", "#f2c36b", "#d9927a", "#b48bd4", "#75c5bd", "#d8a35d", "#9eb36a",
    ];
    PALETTE[(depth + preorder) % PALETTE.len()].to_owned()
}

pub const VLACKU_WEB_DEFAULT_COUNT: usize = DEFAULT_VLACKU_RESULT_COUNT;
pub const VLACKU_WEB_MAX_COUNT: usize = 2048;

pub const CUKTA_WEB_DEFAULT_COUNT: usize = DEFAULT_CUKTA_WEB_RESULT_COUNT;
pub const CUKTA_WEB_MAX_COUNT: usize = MAX_CUKTA_RESULT_COUNT;
pub const WEB_EMBEDDING_MODEL_KEY: &str = jbotci_embedding_inputs::DEFAULT_MODEL_KEY;

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn embedding_worker_corpus_json() -> String {
    embedding_input_corpus_json()
}

#[requires(!request_json.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|json| !json.is_empty()) || ret.is_err())]
pub fn run_web_compute_request_json(request_json: &str) -> Result<String, WebComputeError> {
    let request = serde_json::from_str::<WebComputeRequest>(request_json)
        .map_err(|error| WebComputeError::Json(error.to_string()))?;
    let response = run_web_compute_request(request)?;
    serde_json::to_string(&response).map_err(|error| WebComputeError::Json(error.to_string()))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub fn run_web_compute_request(
    request: WebComputeRequest,
) -> Result<WebComputeResponse, WebComputeError> {
    match request {
        WebComputeRequest::GentufaPage {
            base_path,
            state,
            request,
        } => {
            let result = parse_gentufa_for_web(&request);
            let meta = build_gentufa_page_meta_from_result(&base_path, &state, &result);
            Ok(WebComputeResponse::GentufaPage { result, meta })
        }
        WebComputeRequest::CuktaPage { base_path, state } => {
            let page = build_cukta_web_page(&base_path, &state);
            let meta = build_page_meta(&base_path, &WebRoute::Cukta(state));
            Ok(WebComputeResponse::CuktaPage { page, meta })
        }
        WebComputeRequest::CuktaSemanticPage {
            base_path,
            state,
            hits,
            message,
            loading,
        } => {
            let page = build_cukta_semantic_web_page_with_loading(
                &base_path, &state, &hits, message, loading,
            );
            let meta = build_page_meta(&base_path, &WebRoute::Cukta(state));
            Ok(WebComputeResponse::CuktaPage { page, meta })
        }
        WebComputeRequest::VlackuPage { base_path, state } => {
            let result = build_vlacku_web_result(&state);
            let meta = build_page_meta(&base_path, &WebRoute::Vlacku(state));
            Ok(WebComputeResponse::VlackuPage { result, meta })
        }
        WebComputeRequest::VlackuSemanticPage {
            base_path,
            state,
            hits,
            message,
            loading,
        } => {
            let result =
                build_vlacku_semantic_web_result_with_loading(&state, &hits, message, loading);
            let meta = build_page_meta(&base_path, &WebRoute::Vlacku(state));
            Ok(WebComputeResponse::VlackuPage { result, meta })
        }
        WebComputeRequest::EmbeddingCorpusJson => Ok(WebComputeResponse::EmbeddingCorpusJson {
            json: embedding_worker_corpus_json(),
        }),
        WebComputeRequest::GentufaBlocksSvg {
            layout,
            show_glosses,
            script,
        } => {
            let svg = jbotci_gentufa::render_gentufa_blocks_svg(
                &layout,
                &gentufa_svg_export_options(show_glosses, script),
                jbotci_gentufa::EmbeddedGentufaFonts::get(),
            )
            .map_err(|error| WebComputeError::Export(error.to_string()))?;
            Ok(WebComputeResponse::GentufaBlocksSvg { svg })
        }
        WebComputeRequest::GentufaBlocksPng {
            layout,
            show_glosses,
            script,
        } => {
            let bytes = jbotci_gentufa::render_gentufa_blocks_png(
                &layout,
                &jbotci_gentufa::GentufaPngOptions {
                    svg: gentufa_svg_export_options(show_glosses, script),
                    ..jbotci_gentufa::GentufaPngOptions::default()
                },
                jbotci_gentufa::EmbeddedGentufaFonts::get(),
            )
            .map_err(|error| WebComputeError::Export(error.to_string()))?;
            Ok(WebComputeResponse::GentufaBlocksPng { bytes })
        }
    }
}

#[invariant(true)]
#[invariant(::Meaning => true)]
#[invariant(::Word => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CuktaWebMode {
    Meaning,
    Word,
}

impl Default for CuktaWebMode {
    #[requires(true)]
    #[ensures(ret == CuktaWebMode::Meaning)]
    fn default() -> Self {
        Self::Meaning
    }
}

impl From<CuktaWebMode> for CuktaSearchMode {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: CuktaWebMode) -> Self {
        match value {
            CuktaWebMode::Meaning => Self::Meaning,
            CuktaWebMode::Word => Self::Word,
        }
    }
}

#[invariant(true)]
#[invariant(::Section { .. } => true)]
#[invariant(::Index => true)]
#[invariant(::Search(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CuktaWebView {
    Section { reference: String },
    Index,
    Search(CuktaWebSearchState),
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CuktaWebState {
    pub view: CuktaWebView,
}

impl Default for CuktaWebState {
    #[requires(true)]
    #[ensures(matches!(ret.view, CuktaWebView::Section { .. }))]
    fn default() -> Self {
        Self {
            view: CuktaWebView::Section {
                reference: DEFAULT_CUKTA_SECTION_ID.to_owned(),
            },
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CuktaWebSearchState {
    pub mode: CuktaWebMode,
    pub query: String,
    pub count: usize,
    pub targets: Vec<String>,
}

impl Default for CuktaWebSearchState {
    #[requires(true)]
    #[ensures(ret.count == CUKTA_WEB_DEFAULT_COUNT)]
    fn default() -> Self {
        Self {
            mode: CuktaWebMode::Meaning,
            query: String::new(),
            count: CUKTA_WEB_DEFAULT_COUNT,
            targets: default_cukta_target_values(),
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CuktaTocNode {
    pub node_id: String,
    pub number_label: Option<String>,
    pub label: String,
    pub href: String,
    pub active: bool,
    pub section_id: Option<String>,
    pub current: bool,
    pub children: Vec<CuktaTocNode>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CuktaSectionLink {
    pub label: String,
    pub href: String,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CuktaIndexEntry {
    pub key: String,
    pub references: Vec<CuktaSectionLink>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CuktaModeOption {
    pub value: String,
    pub label: String,
    pub selected: bool,
    pub disabled: bool,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CuktaTargetOption {
    pub value: String,
    pub label: String,
    pub selected: bool,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CuktaSearchResultCard {
    pub rank: usize,
    pub similarity_label: Option<String>,
    pub kind: String,
    pub label: String,
    pub href: String,
    pub section_label: String,
    pub preview: String,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CuktaSemanticSearchHit {
    pub chunk_index: usize,
    pub score: f32,
}

#[invariant(true)]
#[invariant(::Section { .. } => true)]
#[invariant(::Index { .. } => true)]
#[invariant(::Search { .. } => true)]
#[invariant(::Error { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CuktaPageKind {
    Section {
        section_heading: String,
        chapter_title: Option<String>,
        previous_section: Option<CuktaSectionLink>,
        next_section: Option<CuktaSectionLink>,
        chapter_prelude_blocks: Vec<CllBlock>,
        blocks: Vec<CllBlock>,
    },
    Index {
        entries: Vec<CuktaIndexEntry>,
    },
    Search {
        state: CuktaWebSearchState,
        mode_options: Vec<CuktaModeOption>,
        target_options: Vec<CuktaTargetOption>,
        results: Vec<CuktaSearchResultCard>,
        message: Option<String>,
        has_more: bool,
        load_more_href: Option<String>,
    },
    Error {
        message: String,
    },
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CuktaPageData {
    pub toc: Vec<CuktaTocNode>,
    pub current_section_id: Option<String>,
    pub page_kind: CuktaPageKind,
}

#[invariant(true)]
#[invariant(::Word => true)]
#[invariant(::Rafsi => true)]
#[invariant(::Sound => true)]
#[invariant(::Meaning => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VlackuWebMode {
    Word,
    Rafsi,
    Sound,
    Meaning,
}

impl Default for VlackuWebMode {
    #[requires(true)]
    #[ensures(ret == VlackuWebMode::Word)]
    fn default() -> Self {
        Self::Word
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VlackuWebState {
    pub mode: VlackuWebMode,
    pub query: String,
    pub count: usize,
    pub word_types: Vec<String>,
}

impl Default for VlackuWebState {
    #[requires(true)]
    #[ensures(ret.mode == VlackuWebMode::Word)]
    #[ensures(ret.count == VLACKU_WEB_DEFAULT_COUNT)]
    fn default() -> Self {
        Self {
            mode: VlackuWebMode::Word,
            query: String::new(),
            count: VLACKU_WEB_DEFAULT_COUNT,
            word_types: Vec::new(),
        }
    }
}

#[invariant(true)]
#[invariant(::Gentufa(_) => true)]
#[invariant(::Cukta(_) => true)]
#[invariant(::Vlacku(_) => true)]
#[invariant(::Settings => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WebRoute {
    Gentufa(GentufaWebState),
    Cukta(CuktaWebState),
    Vlacku(VlackuWebState),
    Settings,
}

impl Default for WebRoute {
    #[requires(true)]
    #[ensures(matches!(ret, WebRoute::Gentufa(_)))]
    fn default() -> Self {
        WebRoute::Gentufa(GentufaWebState::default())
    }
}

#[invariant(!self.href.is_empty())]
#[invariant(self.width > 0)]
#[invariant(self.height > 0)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SocialImage {
    pub href: String,
    pub width: usize,
    pub height: usize,
}

#[invariant(!self.title.is_empty())]
#[invariant(!self.description.is_empty())]
#[invariant(self.canonical_url.starts_with('/'))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PageMeta {
    pub title: String,
    pub description: String,
    pub canonical_url: String,
    pub image: Option<SocialImage>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VlackuWebResult {
    pub state: VlackuWebState,
    pub cards: Vec<VlackuWebCard>,
    pub word_type_options: Vec<VlackuWordTypeOption>,
    pub dictionary_info: Option<VlackuDictionaryInfo>,
    pub has_more: bool,
    pub message: Option<String>,
    pub errors: Vec<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VlackuSemanticSearchHit {
    pub entry_index: usize,
    pub score: f32,
}

#[invariant(true)]
#[invariant(::GentufaPage { .. } => true)]
#[invariant(::CuktaPage { .. } => true)]
#[invariant(::CuktaSemanticPage { .. } => true)]
#[invariant(::VlackuPage { .. } => true)]
#[invariant(::VlackuSemanticPage { .. } => true)]
#[invariant(::EmbeddingCorpusJson => true)]
#[invariant(::GentufaBlocksSvg { .. } => true)]
#[invariant(::GentufaBlocksPng { .. } => true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum WebComputeRequest {
    GentufaPage {
        base_path: String,
        state: GentufaWebState,
        request: GentufaWebRequest,
    },
    CuktaPage {
        base_path: String,
        state: CuktaWebState,
    },
    CuktaSemanticPage {
        base_path: String,
        state: CuktaWebState,
        hits: Vec<CuktaSemanticSearchHit>,
        message: Option<String>,
        loading: bool,
    },
    VlackuPage {
        base_path: String,
        state: VlackuWebState,
    },
    VlackuSemanticPage {
        base_path: String,
        state: VlackuWebState,
        hits: Vec<VlackuSemanticSearchHit>,
        message: Option<String>,
        loading: bool,
    },
    EmbeddingCorpusJson,
    GentufaBlocksSvg {
        layout: GentufaBlocksLayout,
        show_glosses: bool,
        script: GentufaScript,
    },
    GentufaBlocksPng {
        layout: GentufaBlocksLayout,
        show_glosses: bool,
        script: GentufaScript,
    },
}

#[invariant(true)]
#[invariant(::GentufaPage { .. } => true)]
#[invariant(::CuktaPage { .. } => true)]
#[invariant(::VlackuPage { .. } => true)]
#[invariant(::EmbeddingCorpusJson { .. } => true)]
#[invariant(::GentufaBlocksSvg { .. } => true)]
#[invariant(::GentufaBlocksPng { .. } => true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum WebComputeResponse {
    GentufaPage {
        result: GentufaWebResult,
        meta: PageMeta,
    },
    CuktaPage {
        page: CuktaPageData,
        meta: PageMeta,
    },
    VlackuPage {
        result: VlackuWebResult,
        meta: PageMeta,
    },
    EmbeddingCorpusJson {
        json: String,
    },
    GentufaBlocksSvg {
        svg: String,
    },
    GentufaBlocksPng {
        bytes: Vec<u8>,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Json(_) => true)]
#[invariant(::Export(_) => true)]
pub enum WebComputeError {
    #[error("web compute JSON error: {0}")]
    Json(String),
    #[error("gentufa export failed: {0}")]
    Export(String),
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VlackuWebCard {
    pub rank: usize,
    pub word: String,
    pub display_word: String,
    pub word_type: String,
    pub word_type_key: String,
    pub selmaho: Option<String>,
    pub ipa: Option<String>,
    pub similarity: Option<f32>,
    pub votes: VlackuVoteDisplay,
    pub rafsi: Vec<String>,
    pub glosses: Vec<String>,
    pub definition: Vec<VlackuInline>,
    pub notes: Vec<VlackuInline>,
    pub decomposition: Vec<VlackuCompositionPiece>,
    pub can_add_to_jvozba: bool,
}

#[invariant(true)]
#[invariant(::Hidden => true)]
#[invariant(::Known(_) => true)]
#[invariant(::Unknown => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VlackuVoteDisplay {
    Hidden,
    Known(String),
    Unknown,
}

#[invariant(true)]
#[invariant(::Text(text) => !text.is_empty())]
#[invariant(::WordRef { label, href, .. } => !label.is_empty() && !href.is_empty())]
#[invariant(::Math(math) => !math.parts.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VlackuInline {
    Text(String),
    WordRef {
        label: String,
        href: String,
        can_add_to_jvozba: bool,
    },
    Math(VlackuMath),
}

#[invariant(!self.parts.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VlackuMath {
    pub parts: Vec<VlackuMathPart>,
}

#[invariant(true)]
#[invariant(::Text(text) => !text.is_empty())]
#[invariant(::Operator(operator) => matches!(operator.as_str(), "=" | "," | ";" | ":" | "/" | "+" | "-"))]
#[invariant(::Variable { stem, subscript } => !stem.is_empty() && subscript.as_ref().map_or(true, |value| !value.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VlackuMathPart {
    Text(String),
    Operator(String),
    Variable {
        stem: String,
        subscript: Option<String>,
    },
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VlackuCompositionPiece {
    pub kind: VlackuCompositionPieceKind,
    pub surface: String,
    pub display_surface: String,
    pub source: Option<String>,
    pub display_source: Option<String>,
    pub source_href: Option<String>,
}

#[invariant(true)]
#[invariant(::Rafsi => true)]
#[invariant(::Hyphen => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VlackuCompositionPieceKind {
    Rafsi,
    Hyphen,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VlackuWordTypeOption {
    pub value: String,
    pub label: String,
    pub section: VlackuWordTypeSection,
    pub count: usize,
    pub selected: bool,
    pub indeterminate: bool,
}

#[invariant(!self.value.is_empty() && !self.label.is_empty() && self.count > 0)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct VlackuWordTypeOptionTemplate {
    value: String,
    label: String,
    section: VlackuWordTypeSection,
    count: usize,
}

#[invariant(true)]
#[invariant(::Cmavo => true)]
#[invariant(::Cmevla => true)]
#[invariant(::Brivla => true)]
#[invariant(::Other => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VlackuWordTypeSection {
    Cmavo,
    Cmevla,
    Brivla,
    Other,
}

static VLACKU_WORD_TYPE_OPTION_TEMPLATES: OnceLock<Vec<VlackuWordTypeOptionTemplate>> =
    OnceLock::new();

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VlackuDictionaryInfo {
    pub entry_count: usize,
    pub rafsi_count: usize,
    pub word_type_counts: Vec<VlackuDictionaryWordTypeCount>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VlackuDictionaryWordTypeCount {
    pub label: String,
    pub count: usize,
}

#[invariant(true)]
#[invariant(::Lujvo => true)]
#[invariant(::Cmevla => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VlackuJvozbaMode {
    Lujvo,
    Cmevla,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VlackuJvozbaItem {
    pub kind: VlackuJvozbaItemKind,
    pub value: String,
    pub source: Option<String>,
    pub indent_level: usize,
}

#[invariant(true)]
#[invariant(::Word => true)]
#[invariant(::FixedRafsi => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VlackuJvozbaItemKind {
    Word,
    FixedRafsi,
}

#[invariant(true)]
#[invariant(::Empty => true)]
#[invariant(::NeedsMore => true)]
#[invariant(::Success { .. } => true)]
#[invariant(::Error { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "status")]
pub enum VlackuJvozbaOutput {
    Empty,
    NeedsMore,
    Success {
        word: String,
        segments: Vec<VlackuJvozbaSegment>,
    },
    Error {
        message: String,
    },
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VlackuJvozbaSegment {
    pub kind: VlackuJvozbaSegmentKind,
    pub text: String,
    pub tone: VlackuJvozbaSegmentTone,
}

#[invariant(true)]
#[invariant(::Rafsi => true)]
#[invariant(::Hyphen => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VlackuJvozbaSegmentKind {
    Rafsi,
    Hyphen,
}

#[invariant(true)]
#[invariant(::RafsiA => true)]
#[invariant(::RafsiB => true)]
#[invariant(::Hyphen => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VlackuJvozbaSegmentTone {
    RafsiA,
    RafsiB,
    Hyphen,
}

#[requires(true)]
#[ensures(true)]
pub fn build_vlacku_web_result(state: &VlackuWebState) -> VlackuWebResult {
    let normalized_state = normalize_vlacku_state(state);
    let word_type_options = dictionary_word_type_options(&normalized_state.word_types);
    if normalized_state.query.trim().is_empty() {
        return VlackuWebResult {
            state: normalized_state,
            cards: Vec::new(),
            word_type_options,
            dictionary_info: Some(build_vlacku_dictionary_info()),
            has_more: false,
            message: None,
            errors: Vec::new(),
        };
    }
    if normalized_state.mode == VlackuWebMode::Meaning {
        return VlackuWebResult {
            state: normalized_state,
            cards: Vec::new(),
            word_type_options,
            dictionary_info: None,
            has_more: false,
            message: Some("Meaning search is not available yet.".to_owned()),
            errors: Vec::new(),
        };
    }

    let request = match normalized_state.mode {
        VlackuWebMode::Word => VlackuRequest::Valsi(normalized_state.query.clone()),
        VlackuWebMode::Rafsi => VlackuRequest::Rafsi(normalized_state.query.clone()),
        VlackuWebMode::Sound => VlackuRequest::Sound(normalized_state.query.clone()),
        VlackuWebMode::Meaning => unreachable!("meaning mode returned above"),
    };
    let fetch_count = normalized_state
        .count
        .saturating_add(1)
        .min(VLACKU_WEB_MAX_COUNT);
    let output = run_vlacku_requests(
        jbotci_dictionary_data::english(),
        &[request],
        &VlackuSearchOptions {
            count: fetch_count,
            word_types: normalized_state.word_types.clone(),
            min_votes: None,
            min_similarity: None,
            decompose_lujvo: true,
        },
    );
    let has_more = output.cards.len() > normalized_state.count;
    let cards = output
        .cards
        .into_iter()
        .take(normalized_state.count)
        .enumerate()
        .map(|(index, card)| web_card_from_search_card(index + 1, card))
        .collect::<Vec<_>>();
    let message = if cards.is_empty() && output.diagnostics.is_empty() {
        Some("No matches found.".to_owned())
    } else {
        None
    };

    VlackuWebResult {
        state: normalized_state,
        cards,
        word_type_options,
        dictionary_info: None,
        has_more,
        message,
        errors: output.diagnostics,
    }
}

#[requires(true)]
#[ensures(true)]
pub fn build_vlacku_semantic_web_result(
    state: &VlackuWebState,
    hits: &[VlackuSemanticSearchHit],
    message: Option<String>,
) -> VlackuWebResult {
    build_vlacku_semantic_web_result_with_loading(state, hits, message, false)
}

#[requires(true)]
#[ensures(true)]
pub fn build_vlacku_semantic_web_result_with_loading(
    state: &VlackuWebState,
    hits: &[VlackuSemanticSearchHit],
    message: Option<String>,
    loading: bool,
) -> VlackuWebResult {
    let normalized_state = normalize_vlacku_state(state);
    let word_type_options = dictionary_word_type_options(&normalized_state.word_types);
    if normalized_state.query.trim().is_empty() {
        return VlackuWebResult {
            state: normalized_state,
            cards: Vec::new(),
            word_type_options,
            dictionary_info: Some(build_vlacku_dictionary_info()),
            has_more: false,
            message: None,
            errors: Vec::new(),
        };
    }
    if let Some(message) = message {
        return VlackuWebResult {
            state: normalized_state,
            cards: Vec::new(),
            word_type_options,
            dictionary_info: None,
            has_more: false,
            message: Some(message),
            errors: Vec::new(),
        };
    }
    if loading {
        return VlackuWebResult {
            state: normalized_state,
            cards: Vec::new(),
            word_type_options,
            dictionary_info: None,
            has_more: false,
            message,
            errors: Vec::new(),
        };
    }

    let dictionary = jbotci_dictionary_data::english();
    let cards = hits
        .iter()
        .filter_map(|hit| {
            dictionary
                .entries()
                .get(hit.entry_index)
                .map(|entry| dictionary_entry_card(dictionary, entry, Some(hit.score), true))
        })
        .collect::<Vec<_>>();
    let fetch_count = normalized_state
        .count
        .saturating_add(1)
        .min(VLACKU_WEB_MAX_COUNT);
    let filtered = filter_vlacku_cards(
        cards,
        &VlackuSearchOptions {
            count: fetch_count,
            word_types: normalized_state.word_types.clone(),
            min_votes: None,
            min_similarity: None,
            decompose_lujvo: true,
        },
        true,
    );
    let has_more = filtered.len() > normalized_state.count;
    let cards = filtered
        .into_iter()
        .take(normalized_state.count)
        .enumerate()
        .map(|(index, card)| web_card_from_search_card(index + 1, card))
        .collect::<Vec<_>>();
    let message = if cards.is_empty() {
        Some("No matches found.".to_owned())
    } else {
        None
    };

    VlackuWebResult {
        state: normalized_state,
        cards,
        word_type_options,
        dictionary_info: None,
        has_more,
        message,
        errors: Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
pub fn build_cukta_web_page(base_path: &str, state: &CuktaWebState) -> CuktaPageData {
    let normalized_state = normalize_cukta_state(state);
    let site = match embedded_cll_site() {
        Ok(site) => site,
        Err(error) => {
            return CuktaPageData {
                toc: Vec::new(),
                current_section_id: None,
                page_kind: CuktaPageKind::Error {
                    message: error.to_string(),
                },
            };
        }
    };
    match normalized_state.view {
        CuktaWebView::Section { reference } => {
            let section_id = cll_resolve_section_reference(site, &reference)
                .or_else(|| cll_first_section_id(site).map(str::to_owned))
                .unwrap_or_else(|| DEFAULT_CUKTA_SECTION_ID.to_owned());
            let Some(section) = cll_lookup_section(site, &section_id) else {
                return CuktaPageData {
                    toc: build_cukta_toc(site, base_path, None),
                    current_section_id: None,
                    page_kind: CuktaPageKind::Error {
                        message: "CLL section not found.".to_owned(),
                    },
                };
            };
            let chapter_prelude_blocks = site
                .chapters
                .iter()
                .find(|chapter| chapter.chapter_id == section.chapter_id)
                .filter(|chapter| {
                    chapter
                        .root_section_ids
                        .first()
                        .is_some_and(|first_section_id| first_section_id == &section.section_id)
                })
                .map(|chapter| chapter.prelude_blocks.clone())
                .unwrap_or_default();
            CuktaPageData {
                toc: build_cukta_toc(site, base_path, Some(&section.section_id)),
                current_section_id: Some(section.section_id.clone()),
                page_kind: CuktaPageKind::Section {
                    section_heading: format_section_display_title(section),
                    chapter_title: cll_section_chapter_title(site, &section.section_id),
                    previous_section: cll_previous_section_id(site, &section.section_id).and_then(
                        |section_id| build_cukta_section_link(site, base_path, section_id),
                    ),
                    next_section: cll_next_section_id(site, &section.section_id).and_then(
                        |section_id| build_cukta_section_link(site, base_path, section_id),
                    ),
                    chapter_prelude_blocks,
                    blocks: section.blocks.clone(),
                },
            }
        }
        CuktaWebView::Index => CuktaPageData {
            toc: build_cukta_toc(site, base_path, None),
            current_section_id: None,
            page_kind: CuktaPageKind::Index {
                entries: cll_index_entries(site)
                    .iter()
                    .map(|entry| CuktaIndexEntry {
                        key: entry.key.clone(),
                        references: entry
                            .section_ids
                            .iter()
                            .filter_map(|section_id| {
                                build_cukta_section_link(site, base_path, section_id)
                            })
                            .collect(),
                    })
                    .collect(),
            },
        },
        CuktaWebView::Search(search_state) => {
            let output = cukta_search(
                site,
                search_state.mode.into(),
                &search_state.query,
                search_state.count,
                cukta_target_filter(&search_state.targets),
            );
            let results = output
                .matches
                .into_iter()
                .map(|item| CuktaSearchResultCard {
                    rank: item.rank,
                    similarity_label: item
                        .similarity
                        .map(|similarity| format!("{:.0}%", similarity * 100.0)),
                    kind: cukta_chunk_kind_label(item.chunk.kind).to_owned(),
                    label: item.chunk.label.clone(),
                    href: cukta_chunk_href(base_path, &item.chunk),
                    section_label: format!(
                        "{}. {}",
                        item.chunk.section_number, item.chunk.section_title
                    ),
                    preview: truncate_preview(&item.chunk.text, 420),
                })
                .collect::<Vec<_>>();
            let has_more = output.has_more;
            CuktaPageData {
                toc: build_cukta_toc(site, base_path, None),
                current_section_id: None,
                page_kind: CuktaPageKind::Search {
                    state: search_state.clone(),
                    mode_options: cukta_mode_options(search_state.mode),
                    target_options: cukta_target_options(&search_state.targets),
                    results,
                    message: output.message,
                    has_more,
                    load_more_href: if has_more {
                        let mut next = search_state;
                        next.count = next.count.saturating_mul(2).clamp(1, CUKTA_WEB_MAX_COUNT);
                        Some(cukta_web_url(
                            base_path,
                            &CuktaWebState {
                                view: CuktaWebView::Search(next),
                            },
                        ))
                    } else {
                        None
                    },
                },
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub fn build_cukta_semantic_web_page(
    base_path: &str,
    state: &CuktaWebState,
    hits: &[CuktaSemanticSearchHit],
    message: Option<String>,
) -> CuktaPageData {
    build_cukta_semantic_web_page_with_loading(base_path, state, hits, message, false)
}

#[requires(true)]
#[ensures(true)]
pub fn build_cukta_semantic_web_page_with_loading(
    base_path: &str,
    state: &CuktaWebState,
    hits: &[CuktaSemanticSearchHit],
    message: Option<String>,
    loading: bool,
) -> CuktaPageData {
    let normalized_state = normalize_cukta_state(state);
    let search_state = match normalized_state.view.clone() {
        CuktaWebView::Search(search_state) => search_state,
        _ => return build_cukta_web_page(base_path, &normalized_state),
    };
    let site = match embedded_cll_site() {
        Ok(site) => site,
        Err(error) => {
            return CuktaPageData {
                toc: Vec::new(),
                current_section_id: None,
                page_kind: CuktaPageKind::Error {
                    message: error.to_string(),
                },
            };
        }
    };
    let targets = cukta_target_filter(&search_state.targets);
    let chunks = cll_search_all_chunks(site);
    let mut results = Vec::new();
    if message.is_none() && !search_state.query.trim().is_empty() {
        for hit in hits {
            let Some(chunk) = chunks.get(hit.chunk_index) else {
                continue;
            };
            if !cukta_chunk_allowed(chunk.kind, targets) {
                continue;
            }
            results.push(CuktaSearchResultCard {
                rank: results.len() + 1,
                similarity_label: Some(format!("{:.0}%", hit.score * 100.0)),
                kind: cukta_chunk_kind_label(chunk.kind).to_owned(),
                label: chunk.label.clone(),
                href: cukta_chunk_href(base_path, chunk),
                section_label: format!("{}. {}", chunk.section_number, chunk.section_title),
                preview: truncate_preview(&chunk.text, 420),
            });
            if results.len() > search_state.count {
                break;
            }
        }
    }
    let has_more = results.len() > search_state.count;
    results.truncate(search_state.count);
    let message = message.or_else(|| {
        (!loading && results.is_empty() && !search_state.query.trim().is_empty())
            .then(|| "No matches found.".to_owned())
    });
    CuktaPageData {
        toc: build_cukta_toc(site, base_path, None),
        current_section_id: None,
        page_kind: CuktaPageKind::Search {
            state: search_state.clone(),
            mode_options: cukta_mode_options(search_state.mode),
            target_options: cukta_target_options(&search_state.targets),
            results,
            message,
            has_more,
            load_more_href: if has_more {
                let mut next = search_state;
                next.count = next.count.saturating_mul(2).clamp(1, CUKTA_WEB_MAX_COUNT);
                Some(cukta_web_url(
                    base_path,
                    &CuktaWebState {
                        view: CuktaWebView::Search(next),
                    },
                ))
            } else {
                None
            },
        },
    }
}

#[requires(true)]
#[ensures(true)]
pub fn parse_web_route(path: &str, query: &str) -> WebRoute {
    let logical = path.trim_start_matches('/').trim_end_matches('/');
    if logical.is_empty() {
        WebRoute::Gentufa(GentufaWebState::default())
    } else if logical == "settings" {
        WebRoute::Settings
    } else if logical == "cukta" || logical.starts_with("cukta/") {
        WebRoute::Cukta(parse_cukta_web_route(path, query))
    } else if logical == "vlacku" || logical.starts_with("vlacku/") {
        WebRoute::Vlacku(parse_vlacku_web_route(path, query))
    } else if logical == "gentufa" || logical.starts_with("gentufa/") {
        WebRoute::Gentufa(parse_gentufa_web_route(path, query))
    } else {
        WebRoute::Gentufa(GentufaWebState::default())
    }
}

#[requires(true)]
#[ensures(ret.starts_with(base_path) || base_path.is_empty())]
pub fn web_route_url(base_path: &str, route: &WebRoute) -> String {
    match route {
        WebRoute::Gentufa(state) => gentufa_web_url(base_path, state),
        WebRoute::Cukta(state) => cukta_web_url(base_path, state),
        WebRoute::Vlacku(state) => vlacku_web_url(base_path, state),
        WebRoute::Settings => prefixed_web_path(base_path, "/settings"),
    }
}

#[requires(true)]
#[ensures(true)]
pub fn build_page_meta(base_path: &str, route: &WebRoute) -> PageMeta {
    match route {
        WebRoute::Gentufa(state) => build_gentufa_page_meta(base_path, state),
        WebRoute::Cukta(state) => build_cukta_page_meta(base_path, state),
        WebRoute::Vlacku(state) => build_vlacku_page_meta(base_path, state),
        WebRoute::Settings => page_meta(
            "Settings".to_owned(),
            "Browser-facing jbotci display and parser preferences.".to_owned(),
            web_route_url(base_path, route),
            None,
        ),
    }
}

#[requires(!title.is_empty())]
#[requires(!description.is_empty())]
#[requires(canonical_url.starts_with('/'))]
#[ensures(!ret.title.is_empty())]
fn page_meta(
    title: String,
    description: String,
    canonical_url: String,
    image: Option<SocialImage>,
) -> PageMeta {
    new!(PageMeta {
        title,
        description,
        canonical_url,
        image,
    })
}

#[requires(!href.is_empty())]
#[requires(width > 0)]
#[requires(height > 0)]
#[ensures(!ret.href.is_empty())]
fn social_image(href: String, width: usize, height: usize) -> SocialImage {
    new!(SocialImage {
        href,
        width,
        height,
    })
}

#[requires(true)]
#[ensures(ret.title == "jbotci gentufa blocks")]
fn gentufa_svg_export_options(
    show_glosses: bool,
    script: GentufaScript,
) -> jbotci_gentufa::GentufaSvgOptions {
    jbotci_gentufa::GentufaSvgOptions {
        show_glosses,
        script,
        title: "jbotci gentufa blocks".to_owned(),
    }
}

#[requires(true)]
#[ensures(true)]
fn build_gentufa_page_meta(base_path: &str, state: &GentufaWebState) -> PageMeta {
    let state = normalize_gentufa_state(state);
    let request = GentufaWebRequest {
        text: state.text.clone(),
        options: GentufaWebOptions {
            dialect: state.dialect.clone(),
            view_mode: state.view_mode,
            script: GentufaScript::Latin,
            show_elided: state.show_elided,
            show_glosses: state.show_glosses,
            show_definitions: false,
            phonemes: PhonemeRenderOptions::default(),
        },
    };
    let result = parse_gentufa_for_web(&request);
    build_gentufa_page_meta_from_result(base_path, &state, &result)
}

#[requires(true)]
#[ensures(true)]
fn build_gentufa_page_meta_from_result(
    base_path: &str,
    state: &GentufaWebState,
    result: &GentufaWebResult,
) -> PageMeta {
    let state = normalize_gentufa_state(state);
    let title = if state.text.trim().is_empty() {
        "jbotci gentufa".to_owned()
    } else {
        format!("{} - jbotci gentufa", state.text.trim())
    };
    let description = match result {
        GentufaWebResult::Blank => {
            "Parse Lojban text into blocks, table rows, or Lean semantics.".to_owned()
        }
        GentufaWebResult::Success(success) => {
            format!(
                "Parse succeeded: {}",
                truncate_preview(&success.brackets_text, 160)
            )
        }
        GentufaWebResult::Error(error) => {
            format!("Parse failed: {}", truncate_preview(&error.message, 160))
        }
    };
    page_meta(title, description, gentufa_web_url(base_path, &state), None)
}

#[requires(true)]
#[ensures(true)]
fn build_cukta_page_meta(base_path: &str, state: &CuktaWebState) -> PageMeta {
    let state = normalize_cukta_state(state);
    let canonical_url = cukta_web_url(base_path, &state);
    let site = match embedded_cll_site() {
        Ok(site) => site,
        Err(error) => {
            return page_meta(
                "jbotci CLL - missing section".to_owned(),
                format!("The requested CLL section was not found: {error}."),
                canonical_url,
                None,
            );
        }
    };
    match &state.view {
        CuktaWebView::Index => page_meta(
            "jbotci CLL - CLL index".to_owned(),
            "Browse indexed CLL terms and jump directly into the embedded book.".to_owned(),
            canonical_url,
            None,
        ),
        CuktaWebView::Search(search) => {
            let query = search.query.trim();
            page_meta(
                if query.is_empty() {
                    "jbotci CLL - CLL search".to_owned()
                } else {
                    format!("{query} - jbotci CLL")
                },
                if query.is_empty() {
                    "Search sections, paragraphs, and examples across the embedded CLL.".to_owned()
                } else {
                    format!("Searching cukta for “{query}”.")
                },
                canonical_url,
                None,
            )
        }
        CuktaWebView::Section { reference } => {
            let Some(section_id) = cll_resolve_section_reference(site, reference) else {
                return page_meta(
                    "jbotci CLL - missing section".to_owned(),
                    "The requested CLL section was not found.".to_owned(),
                    canonical_url,
                    None,
                );
            };
            let Some(section) = cll_lookup_section(site, &section_id) else {
                return page_meta(
                    "jbotci CLL - missing section".to_owned(),
                    "The requested CLL section was not found.".to_owned(),
                    canonical_url,
                    None,
                );
            };
            let title = format!(
                "The Complete Lojban Language - Chapter {}",
                cll_section_chapter_title(site, &section.section_id).unwrap_or_else(|| {
                    section
                        .number
                        .split_once('.')
                        .map(|(chapter_number, _)| chapter_number.to_owned())
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| "Unknown chapter".to_owned())
                })
            );
            page_meta(
                title,
                format!("Section {}", format_section_display_title(section)),
                canonical_url,
                cukta_section_social_image(base_path, site, section),
            )
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn build_vlacku_page_meta(base_path: &str, state: &VlackuWebState) -> PageMeta {
    let state = normalize_vlacku_state(state);
    let query = state.query.trim();
    page_meta(
        if state.mode == VlackuWebMode::Word && !query.is_empty() {
            format!("{query} - jbotci vlacku")
        } else if query.is_empty() {
            "jbotci dictionary".to_owned()
        } else {
            format!("{query} - jbotci vlacku")
        },
        if state.mode == VlackuWebMode::Word && !query.is_empty() {
            format!("Dictionary lookup for “{query}”.")
        } else if query.is_empty() {
            "Browse the embedded dictionary and Lensisku import metadata.".to_owned()
        } else {
            match state.mode {
                VlackuWebMode::Meaning => format!("Meaning search for “{query}”."),
                VlackuWebMode::Word => format!("Exact lookup for “{query}”."),
                VlackuWebMode::Rafsi => format!("Rafsi search for “{query}”."),
                VlackuWebMode::Sound => format!("Sound search for “{query}”."),
            }
        },
        vlacku_web_url(base_path, &state),
        None,
    )
}

#[requires(true)]
#[ensures(true)]
fn cukta_section_social_image(
    base_path: &str,
    site: &jbotci_cll::CllSite,
    section: &jbotci_cll::CllSection,
) -> Option<SocialImage> {
    let chapter = site
        .chapters
        .iter()
        .find(|chapter| chapter.chapter_id == section.chapter_id)?;
    first_social_image_from_blocks(base_path, &chapter.prelude_blocks)
}

#[requires(true)]
#[ensures(true)]
fn first_social_image_from_blocks(base_path: &str, blocks: &[CllBlock]) -> Option<SocialImage> {
    for block in blocks {
        match block {
            CllBlock::Media { src, .. } => return social_image_for_cll_media(base_path, src),
            CllBlock::List { items, .. } => {
                for item in items {
                    if let Some(image) = first_social_image_from_blocks(base_path, item) {
                        return Some(image);
                    }
                }
            }
            CllBlock::Example(example) => {
                if let Some(image) = first_social_image_from_blocks(base_path, &example.blocks) {
                    return Some(image);
                }
            }
            CllBlock::Table {
                header_rows,
                body_rows,
                ..
            } => {
                for cell in header_rows.iter().chain(body_rows.iter()).flatten() {
                    if let Some(image) = first_social_image_from_blocks(base_path, &cell.blocks) {
                        return Some(image);
                    }
                }
            }
            CllBlock::VariableList { entries, .. } => {
                for entry in entries {
                    if let Some(image) = first_social_image_from_blocks(base_path, &entry.blocks) {
                        return Some(image);
                    }
                }
            }
            CllBlock::BlockQuote { blocks, .. } => {
                if let Some(image) = first_social_image_from_blocks(base_path, blocks) {
                    return Some(image);
                }
            }
            _ => {}
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn social_image_for_cll_media(base_path: &str, src: &str) -> Option<SocialImage> {
    let file_name = src
        .trim_start_matches("assets/media/")
        .trim_start_matches("media/")
        .trim_start_matches("assets/cll/media/")
        .trim_start_matches("cll/media/");
    let (width, height) = cll_media_dimensions(file_name)?;
    Some(social_image(
        prefixed_web_path(base_path, &format!("/assets/cll/media/{file_name}")),
        width,
        height,
    ))
}

#[requires(true)]
#[ensures(true)]
fn cll_media_dimensions(file_name: &str) -> Option<(usize, usize)> {
    match file_name {
        "chapter-2-diagram.svg.png" => Some((400, 267)),
        "chapter-about.svg.png" => Some((400, 320)),
        "chapter-abstractions.svg.png" => Some((400, 381)),
        "chapter-anaphoric-cmavo.svg.png" => Some((400, 290)),
        "chapter-attitudinals.gif" => Some((398, 404)),
        "chapter-catalogue.svg.png" => Some((400, 348)),
        "chapter-connectives.svg.png" => Some((400, 287)),
        "chapter-grammars.svg.png" => Some((400, 720)),
        "chapter-letterals.svg.png" => Some((400, 406)),
        "chapter-lujvo.svg.png" => Some((400, 357)),
        "chapter-mekso.gif" => Some((398, 404)),
        "chapter-morphology.gif" => Some((398, 404)),
        "chapter-negation.gif" => Some((398, 404)),
        "chapter-phonology.gif" => Some((398, 404)),
        "chapter-quantifiers.gif" => Some((398, 404)),
        "chapter-relative-clauses.svg.png" => Some((400, 277)),
        "chapter-selbri.svg.png" => Some((400, 394)),
        "chapter-structure.svg.png" => Some((400, 406)),
        "chapter-sumti.gif" => Some((398, 404)),
        "chapter-sumti-tcita.gif" => Some((398, 404)),
        "chapter-tenses.gif" => Some((398, 404)),
        "chapter-tour.svg.png" => Some((400, 409)),
        "logo.png" => Some((200, 133)),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
pub fn normalize_gentufa_state(state: &GentufaWebState) -> GentufaWebState {
    GentufaWebState {
        text: state.text.trim().to_owned(),
        dialect: state
            .dialect
            .as_deref()
            .map(str::trim)
            .filter(|dialect| !dialect.is_empty())
            .map(str::to_owned),
        view_mode: state.view_mode,
        show_elided: state.show_elided,
        show_glosses: state.show_glosses,
    }
}

#[requires(true)]
#[ensures(true)]
pub fn parse_gentufa_web_route(path: &str, query: &str) -> GentufaWebState {
    let logical = path.trim_start_matches('/').trim_end_matches('/');
    let mut state = if logical == "gentufa" || logical.is_empty() {
        GentufaWebState::default()
    } else {
        GentufaWebState::default()
    };
    for (key, value) in parse_query_pairs(query) {
        match key.as_str() {
            "text" => state.text = value,
            "dialect" => state.dialect = Some(value),
            "view" => {
                if let Some(view_mode) = parse_gentufa_view_mode(&value) {
                    state.view_mode = view_mode;
                }
            }
            "glosses" => state.show_glosses = parse_query_bool(&value, true),
            "elided" => state.show_elided = parse_query_bool(&value, false),
            _ => {}
        }
    }
    normalize_gentufa_state(&state)
}

#[requires(true)]
#[ensures(ret.starts_with(base_path) || base_path.is_empty())]
pub fn gentufa_web_url(base_path: &str, state: &GentufaWebState) -> String {
    let state = normalize_gentufa_state(state);
    let mut pairs = Vec::new();
    if !state.text.is_empty() {
        pairs.push(("text".to_owned(), state.text.clone()));
    }
    if let Some(dialect) = state.dialect.as_ref() {
        pairs.push(("dialect".to_owned(), dialect.clone()));
    }
    if state.view_mode != GentufaWebViewMode::Blocks {
        pairs.push((
            "view".to_owned(),
            gentufa_view_mode_query_value(state.view_mode).to_owned(),
        ));
    }
    if !state.show_glosses {
        pairs.push(("glosses".to_owned(), "false".to_owned()));
    }
    if state.show_elided {
        pairs.push(("elided".to_owned(), "true".to_owned()));
    }
    let path = prefixed_web_path(base_path, "/gentufa");
    if pairs.is_empty() {
        path
    } else {
        format!(
            "{path}?{}",
            pairs
                .iter()
                .map(|(key, value)| format!("{key}={}", percent_encode(value)))
                .collect::<Vec<_>>()
                .join("&")
        )
    }
}

#[requires(true)]
#[ensures(true)]
pub fn normalize_cukta_state(state: &CuktaWebState) -> CuktaWebState {
    match &state.view {
        CuktaWebView::Section { reference } => CuktaWebState {
            view: CuktaWebView::Section {
                reference: if reference.trim().is_empty() {
                    DEFAULT_CUKTA_SECTION_ID.to_owned()
                } else {
                    reference.trim().to_owned()
                },
            },
        },
        CuktaWebView::Index => CuktaWebState {
            view: CuktaWebView::Index,
        },
        CuktaWebView::Search(search) => CuktaWebState {
            view: CuktaWebView::Search(CuktaWebSearchState {
                mode: search.mode,
                query: search.query.trim().to_owned(),
                count: search.count.clamp(1, CUKTA_WEB_MAX_COUNT),
                targets: normalize_cukta_targets(&search.targets),
            }),
        },
    }
}

#[requires(true)]
#[ensures(true)]
pub fn parse_cukta_web_route(path: &str, query: &str) -> CuktaWebState {
    let logical = path.trim_start_matches('/').trim_end_matches('/');
    let mut state = if logical == "cukta" || logical.is_empty() {
        CuktaWebState::default()
    } else if logical == "cukta/index" {
        CuktaWebState {
            view: CuktaWebView::Index,
        }
    } else if logical == "cukta/search" {
        CuktaWebState {
            view: CuktaWebView::Search(CuktaWebSearchState::default()),
        }
    } else if let Some(reference) = logical.strip_prefix("cukta/section/") {
        CuktaWebState {
            view: CuktaWebView::Section {
                reference: percent_decode(reference),
            },
        }
    } else {
        CuktaWebState::default()
    };
    if let CuktaWebView::Search(search) = &mut state.view {
        let mut target_seen = false;
        for (key, value) in parse_query_pairs(query) {
            match key.as_str() {
                "mode" => {
                    if let Some(mode) = parse_cukta_mode(&value) {
                        search.mode = mode;
                    }
                }
                "q" | "query" => search.query = value,
                "count" => {
                    if let Ok(count) = value.parse::<usize>() {
                        search.count = count;
                    }
                }
                "target" | "searchFor" | "search-for" => {
                    if !target_seen {
                        search.targets.clear();
                        target_seen = true;
                    }
                    search.targets.push(value);
                }
                _ => {}
            }
        }
    }
    normalize_cukta_state(&state)
}

#[requires(true)]
#[ensures(ret.starts_with(base_path) || base_path.is_empty())]
pub fn cukta_web_url(base_path: &str, state: &CuktaWebState) -> String {
    let state = normalize_cukta_state(state);
    let prefix = base_path.trim_end_matches('/');
    match state.view {
        CuktaWebView::Section { reference } => {
            format!("{prefix}/cukta/section/{}", percent_encode(&reference))
        }
        CuktaWebView::Index => format!("{prefix}/cukta/index"),
        CuktaWebView::Search(search) => {
            let mut pairs = Vec::new();
            if search.mode != CuktaWebMode::Meaning {
                pairs.push((
                    "mode".to_owned(),
                    cukta_mode_query_value(search.mode).to_owned(),
                ));
            }
            if !search.query.is_empty() {
                pairs.push(("q".to_owned(), search.query.clone()));
            }
            if search.count != CUKTA_WEB_DEFAULT_COUNT {
                pairs.push(("count".to_owned(), search.count.to_string()));
            }
            for target in non_default_cukta_targets(&search.targets) {
                pairs.push(("target".to_owned(), target));
            }
            if pairs.is_empty() {
                format!("{prefix}/cukta/search")
            } else {
                format!(
                    "{prefix}/cukta/search?{}",
                    pairs
                        .iter()
                        .map(|(key, value)| format!("{key}={}", percent_encode(value)))
                        .collect::<Vec<_>>()
                        .join("&")
                )
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub fn toggle_cukta_target_selection(current: &[String], value: &str) -> Vec<String> {
    let mut targets = normalize_cukta_targets(current);
    let normalized = normalize_cukta_target(value);
    if normalized.is_empty() {
        return targets;
    }
    if targets.iter().any(|target| target == &normalized) {
        if targets.len() > 1 {
            targets.retain(|target| target != &normalized);
        }
    } else {
        targets.push(normalized);
        targets = normalize_cukta_targets(&targets);
    }
    targets
}

#[requires(true)]
#[ensures(true)]
fn build_cukta_toc(
    site: &jbotci_cll::CllSite,
    base_path: &str,
    current_section_id: Option<&str>,
) -> Vec<CuktaTocNode> {
    site.chapters
        .iter()
        .map(|chapter| {
            let children = chapter
                .root_section_ids
                .iter()
                .filter_map(|section_id| {
                    build_cukta_toc_section(site, base_path, current_section_id, section_id)
                })
                .collect::<Vec<_>>();
            let href = children
                .first()
                .map(|node| node.href.clone())
                .unwrap_or_else(|| format!("{}/cukta/index", base_path.trim_end_matches('/')));
            CuktaTocNode {
                node_id: chapter.chapter_id.clone(),
                number_label: Some(chapter.chapter_number.to_string()),
                label: chapter.chapter_title.clone(),
                href,
                active: children.iter().any(|child| child.active),
                section_id: None,
                current: false,
                children,
            }
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn build_cukta_toc_section(
    site: &jbotci_cll::CllSite,
    base_path: &str,
    current_section_id: Option<&str>,
    section_id: &str,
) -> Option<CuktaTocNode> {
    let section = cll_lookup_section(site, section_id)?;
    let children = section
        .child_section_ids
        .iter()
        .filter_map(|child_id| {
            build_cukta_toc_section(site, base_path, current_section_id, child_id)
        })
        .collect::<Vec<_>>();
    let current = current_section_id == Some(section.section_id.as_str());
    Some(CuktaTocNode {
        node_id: section.section_id.clone(),
        number_label: Some(section.number.clone()),
        label: section.title.clone(),
        href: cukta_section_href(base_path, &section.section_id),
        active: current || children.iter().any(|child| child.active),
        section_id: Some(section.section_id.clone()),
        current,
        children,
    })
}

#[requires(true)]
#[ensures(true)]
fn build_cukta_section_link(
    site: &jbotci_cll::CllSite,
    base_path: &str,
    section_id: &str,
) -> Option<CuktaSectionLink> {
    let section = cll_lookup_section(site, section_id)?;
    Some(CuktaSectionLink {
        label: format_section_display_title(section),
        href: cukta_section_href(base_path, &section.section_id),
    })
}

#[requires(!section_id.is_empty())]
#[ensures(ret.contains(section_id))]
fn cukta_section_href(base_path: &str, section_id: &str) -> String {
    format!(
        "{}/cukta/section/{}",
        base_path.trim_end_matches('/'),
        percent_encode(section_id)
    )
}

#[requires(true)]
#[ensures(true)]
fn cukta_chunk_href(base_path: &str, chunk: &jbotci_cll::CllSearchChunk) -> String {
    let relative = cll_search_chunk_href(chunk);
    let relative = relative
        .strip_prefix("section/")
        .map(|section| format!("cukta/section/{section}"))
        .unwrap_or(relative);
    format!("{}/{}", base_path.trim_end_matches('/'), relative)
}

#[requires(true)]
#[ensures(true)]
fn cukta_mode_options(selected: CuktaWebMode) -> Vec<CuktaModeOption> {
    vec![
        CuktaModeOption {
            value: "smuni".to_owned(),
            label: "meaning".to_owned(),
            selected: selected == CuktaWebMode::Meaning,
            disabled: false,
        },
        CuktaModeOption {
            value: "valsi".to_owned(),
            label: "word".to_owned(),
            selected: selected == CuktaWebMode::Word,
            disabled: false,
        },
    ]
}

#[requires(true)]
#[ensures(true)]
fn cukta_target_options(selected_targets: &[String]) -> Vec<CuktaTargetOption> {
    let selected = normalize_cukta_targets(selected_targets);
    [
        ("section", "Sections"),
        ("paragraph", "Paragraphs"),
        ("example", "Examples"),
    ]
    .iter()
    .map(|(value, label)| CuktaTargetOption {
        value: (*value).to_owned(),
        label: (*label).to_owned(),
        selected: selected.iter().any(|target| target == value),
    })
    .collect()
}

#[requires(true)]
#[ensures(true)]
fn normalize_cukta_targets(raw_targets: &[String]) -> Vec<String> {
    let mut targets = Vec::new();
    for raw in raw_targets {
        for part in raw.split(',') {
            let normalized = normalize_cukta_target(part);
            if !normalized.is_empty() && !targets.iter().any(|target| target == &normalized) {
                targets.push(normalized);
            }
        }
    }
    if targets.is_empty() {
        default_cukta_target_values()
    } else {
        targets
    }
}

#[requires(true)]
#[ensures(true)]
fn non_default_cukta_targets(targets: &[String]) -> Vec<String> {
    let normalized = normalize_cukta_targets(targets);
    if normalized == default_cukta_target_values() {
        Vec::new()
    } else {
        normalized
    }
}

#[requires(true)]
#[ensures(true)]
fn default_cukta_target_values() -> Vec<String> {
    vec![
        "section".to_owned(),
        "paragraph".to_owned(),
        "example".to_owned(),
    ]
}

#[requires(true)]
#[ensures(true)]
fn normalize_cukta_target(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "section" | "sections" => "section".to_owned(),
        "paragraph" | "paragraphs" => "paragraph".to_owned(),
        "example" | "examples" => "example".to_owned(),
        _ => String::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn cukta_target_filter(targets: &[String]) -> CuktaTargetFilter {
    let normalized = normalize_cukta_targets(targets);
    CuktaTargetFilter {
        sections: normalized.iter().any(|target| target == "section"),
        paragraphs: normalized.iter().any(|target| target == "paragraph"),
        examples: normalized.iter().any(|target| target == "example"),
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_cukta_mode(value: &str) -> Option<CuktaWebMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "smuni" | "meaning" => Some(CuktaWebMode::Meaning),
        "valsi" | "word" => Some(CuktaWebMode::Word),
        _ => None,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn cukta_mode_query_value(mode: CuktaWebMode) -> &'static str {
    match mode {
        CuktaWebMode::Meaning => "smuni",
        CuktaWebMode::Word => "valsi",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn cukta_chunk_kind_label(kind: CllSearchChunkKind) -> &'static str {
    match kind {
        CllSearchChunkKind::Section => "section",
        CllSearchChunkKind::Paragraph => "paragraph",
        CllSearchChunkKind::Example => "example",
    }
}

#[requires(true)]
#[ensures(true)]
fn cukta_chunk_allowed(kind: CllSearchChunkKind, targets: CuktaTargetFilter) -> bool {
    match kind {
        CllSearchChunkKind::Section => targets.sections,
        CllSearchChunkKind::Paragraph => targets.paragraphs,
        CllSearchChunkKind::Example => targets.examples,
    }
}

#[requires(true)]
#[ensures(ret.count >= 1)]
pub fn normalize_vlacku_state(state: &VlackuWebState) -> VlackuWebState {
    let mut word_types = Vec::new();
    for raw in &state.word_types {
        let normalized = grouped_word_type_filter_key(&normalize_word_type_filter(raw));
        if normalized == "brivla" {
            for child in vlacku_brivla_child_filter_values() {
                if !word_types.iter().any(|candidate| candidate == child) {
                    word_types.push((*child).to_owned());
                }
            }
        } else if !normalized.is_empty()
            && !word_types.iter().any(|candidate| candidate == &normalized)
        {
            word_types.push(normalized);
        }
    }
    VlackuWebState {
        mode: state.mode,
        query: state.query.trim().to_owned(),
        count: state.count.clamp(1, VLACKU_WEB_MAX_COUNT),
        word_types,
    }
}

#[requires(true)]
#[ensures(true)]
pub fn parse_vlacku_web_route(path: &str, query: &str) -> VlackuWebState {
    let mut state = VlackuWebState::default();
    let path_without_slash = path.trim_start_matches('/');
    if let Some(path_word) = path_without_slash.strip_prefix("vlacku/") {
        state.mode = VlackuWebMode::Word;
        state.query = percent_decode(path_word);
    }
    for (key, value) in parse_query_pairs(query) {
        match key.as_str() {
            "mode" => {
                if let Some(mode) = parse_vlacku_mode(&value) {
                    state.mode = mode;
                }
            }
            "q" | "query" => state.query = value,
            "count" => {
                if let Ok(count) = value.parse::<usize>() {
                    state.count = count;
                }
            }
            "wordType" | "word-type" | "word_type" => state.word_types.push(value),
            _ => {}
        }
    }
    normalize_vlacku_state(&state)
}

#[requires(true)]
#[ensures(ret.starts_with(base_path) || base_path.is_empty())]
pub fn vlacku_web_url(base_path: &str, state: &VlackuWebState) -> String {
    let state = normalize_vlacku_state(state);
    let prefix = base_path.trim_end_matches('/');
    if state.mode == VlackuWebMode::Word
        && !state.query.is_empty()
        && state.count == VLACKU_WEB_DEFAULT_COUNT
        && state.word_types.is_empty()
    {
        return format!("{prefix}/vlacku/{}", percent_encode(&state.query));
    }
    let mut pairs = Vec::new();
    if state.mode != VlackuWebMode::Word {
        pairs.push((
            "mode".to_owned(),
            vlacku_mode_query_value(state.mode).to_owned(),
        ));
    }
    if !state.query.is_empty() {
        pairs.push(("q".to_owned(), state.query.clone()));
    }
    if state.count != VLACKU_WEB_DEFAULT_COUNT {
        pairs.push(("count".to_owned(), state.count.to_string()));
    }
    for word_type in vlacku_url_word_type_values(&state.word_types) {
        pairs.push(("wordType".to_owned(), word_type));
    }
    if pairs.is_empty() {
        format!("{prefix}/vlacku")
    } else {
        format!(
            "{prefix}/vlacku?{}",
            pairs
                .iter()
                .map(|(key, value)| format!("{key}={}", percent_encode(value)))
                .collect::<Vec<_>>()
                .join("&")
        )
    }
}

#[requires(true)]
#[ensures(true)]
pub fn build_vlacku_jvozba_output(
    mode: VlackuJvozbaMode,
    items: &[VlackuJvozbaItem],
) -> VlackuJvozbaOutput {
    if items.is_empty() {
        return VlackuJvozbaOutput::Empty;
    }
    if items.len() < 2 {
        return VlackuJvozbaOutput::NeedsMore;
    }
    let parsed_inputs = items
        .iter()
        .map(|item| match item.kind {
            VlackuJvozbaItemKind::Word => JvozbaSourceInput::Word(item.value.clone()),
            VlackuJvozbaItemKind::FixedRafsi => JvozbaSourceInput::FixedRafsi(item.value.clone()),
        })
        .collect::<Vec<_>>();
    let result = build_best_jvozba_detailed(
        match mode {
            VlackuJvozbaMode::Lujvo => JvozbaMode::Lujvo,
            VlackuJvozbaMode::Cmevla => JvozbaMode::Cmevla,
        },
        jbotci_dictionary_data::english(),
        &parsed_inputs,
    );
    match result {
        Ok(result) => VlackuJvozbaOutput::Success {
            word: result.word.clone(),
            segments: render_jvozba_segments(mode, &result.segments),
        },
        Err(error) => VlackuJvozbaOutput::Error {
            message: error.to_string(),
        },
    }
}

#[requires(true)]
#[ensures(true)]
pub fn render_jvozba_segments(
    mode: VlackuJvozbaMode,
    segments: &[JvozbaSegment],
) -> Vec<VlackuJvozbaSegment> {
    let mut rendered = Vec::new();
    let mut rafsi_index = 0usize;
    let last_segment_index = segments.len().saturating_sub(1);
    for (segment_index, segment) in segments.iter().enumerate() {
        match segment.kind {
            JvozbaSegmentKind::Hyphen => rendered.push(VlackuJvozbaSegment {
                kind: VlackuJvozbaSegmentKind::Hyphen,
                text: segment.text.clone(),
                tone: VlackuJvozbaSegmentTone::Hyphen,
            }),
            JvozbaSegmentKind::Rafsi => {
                let tone = if rafsi_index % 2 == 0 {
                    VlackuJvozbaSegmentTone::RafsiA
                } else {
                    VlackuJvozbaSegmentTone::RafsiB
                };
                if mode == VlackuJvozbaMode::Cmevla
                    && segment_index == last_segment_index
                    && ends_with_consonant(&segment.text)
                {
                    let split_index = segment
                        .text
                        .char_indices()
                        .last()
                        .map(|(index, _)| index)
                        .unwrap_or(segment.text.len());
                    if split_index > 0 {
                        rendered.push(VlackuJvozbaSegment {
                            kind: VlackuJvozbaSegmentKind::Rafsi,
                            text: segment.text[..split_index].to_owned(),
                            tone,
                        });
                    }
                    rendered.push(VlackuJvozbaSegment {
                        kind: VlackuJvozbaSegmentKind::Hyphen,
                        text: segment.text[split_index..].to_owned(),
                        tone: VlackuJvozbaSegmentTone::Hyphen,
                    });
                } else {
                    rendered.push(VlackuJvozbaSegment {
                        kind: VlackuJvozbaSegmentKind::Rafsi,
                        text: segment.text.clone(),
                        tone,
                    });
                }
                rafsi_index += 1;
            }
        }
    }
    rendered
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn vlacku_mode_query_value(mode: VlackuWebMode) -> &'static str {
    match mode {
        VlackuWebMode::Word => "valsi",
        VlackuWebMode::Rafsi => "rafsi",
        VlackuWebMode::Sound => "sound",
        VlackuWebMode::Meaning => "smuni",
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_vlacku_mode(value: &str) -> Option<VlackuWebMode> {
    match value {
        "word" | "valsi" => Some(VlackuWebMode::Word),
        "rafsi" => Some(VlackuWebMode::Rafsi),
        "sound" => Some(VlackuWebMode::Sound),
        "meaning" | "smuni" => Some(VlackuWebMode::Meaning),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_gentufa_view_mode(value: &str) -> Option<GentufaWebViewMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "tree" | "table" => Some(GentufaWebViewMode::Tree),
        "blocks" => Some(GentufaWebViewMode::Blocks),
        _ => None,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn gentufa_view_mode_query_value(mode: GentufaWebViewMode) -> &'static str {
    match mode {
        GentufaWebViewMode::Blocks => "blocks",
        GentufaWebViewMode::Tree => "tree",
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_query_bool(value: &str, default: bool) -> bool {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

#[requires(suffix.starts_with('/'))]
#[ensures(ret.starts_with('/'))]
fn prefixed_web_path(base_path: &str, suffix: &str) -> String {
    let prefix = base_path.trim_end_matches('/');
    if prefix.is_empty() {
        suffix.to_owned()
    } else {
        format!("{prefix}{suffix}")
    }
}

#[requires(true)]
#[ensures(true)]
pub fn dictionary_tooltip_for_word(base_path: &str, word: &str) -> Option<DictionaryTooltipCard> {
    let output = run_vlacku_requests(
        jbotci_dictionary_data::english(),
        &[VlackuRequest::Valsi(word.to_owned())],
        &tooltip_vlacku_options(),
    );
    output
        .cards
        .into_iter()
        .next()
        .map(|card| dictionary_tooltip_card_from_search_card(base_path, card))
}

#[requires(true)]
#[ensures(true)]
pub fn dictionary_tooltip_for_rafsi(base_path: &str, rafsi: &str) -> Option<DictionaryTooltipCard> {
    let output = run_vlacku_requests(
        jbotci_dictionary_data::english(),
        &[VlackuRequest::Rafsi(rafsi.to_owned())],
        &tooltip_vlacku_options(),
    );
    output
        .cards
        .into_iter()
        .next()
        .map(|card| dictionary_tooltip_card_from_search_card(base_path, card))
}

#[requires(true)]
#[ensures(ret.count == 1)]
fn tooltip_vlacku_options() -> VlackuSearchOptions {
    VlackuSearchOptions {
        count: 1,
        word_types: Vec::new(),
        min_votes: None,
        min_similarity: None,
        decompose_lujvo: true,
    }
}

#[requires(true)]
#[ensures(true)]
fn dictionary_annotations_for_words(
    dictionary: &Dictionary<'_>,
    words: &[WordLike],
    base_path: &str,
) -> Vec<GentufaBlockAnnotation<DictionaryTooltipCard>> {
    dictionary_matches_for_word_likes(dictionary, words)
        .into_iter()
        .map(|parsed_match| dictionary_annotation_from_match(parsed_match, base_path))
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn dictionary_annotations_for_elided_terminators(
    terminators: &[ElidedTerminator],
    base_path: &str,
) -> Vec<GentufaBlockAnnotation<DictionaryTooltipCard>> {
    terminators
        .iter()
        .filter_map(|terminator| {
            let card = dictionary_tooltip_for_word(base_path, &terminator.dictionary_text)?;
            Some(GentufaBlockAnnotation {
                range: terminator.range,
                text: Some(terminator.text.clone()),
                glosses: card.glosses.clone(),
                definition: tooltip_definition_text(&card),
                tooltip: Some(card),
            })
        })
        .collect()
}

#[requires(parsed_match.byte_start <= parsed_match.byte_end)]
#[requires(parsed_match.char_start <= parsed_match.char_end)]
#[ensures(ret.range.byte_start == parsed_match.byte_start)]
fn dictionary_annotation_from_match(
    parsed_match: ParsedWordDictionaryMatch,
    base_path: &str,
) -> GentufaBlockAnnotation<DictionaryTooltipCard> {
    let first_card = parsed_match
        .cards
        .into_iter()
        .next()
        .map(|card| dictionary_tooltip_card_from_search_card(base_path, card));
    GentufaBlockAnnotation {
        range: WebSourceRange {
            byte_start: parsed_match.byte_start,
            byte_end: parsed_match.byte_end,
            char_start: parsed_match.char_start,
            char_end: parsed_match.char_end,
        },
        text: Some(parsed_match.lookup_text),
        glosses: first_card
            .as_ref()
            .map(|card| card.glosses.clone())
            .unwrap_or_default(),
        definition: first_card.as_ref().and_then(tooltip_definition_text),
        tooltip: first_card,
    }
}

#[requires(true)]
#[ensures(!ret.word.is_empty())]
fn dictionary_tooltip_card_from_search_card(
    base_path: &str,
    card: VlackuCard,
) -> DictionaryTooltipCard {
    let word_href = vlacku_web_url(
        base_path,
        &VlackuWebState {
            mode: VlackuWebMode::Word,
            query: card.word.clone(),
            count: VLACKU_WEB_DEFAULT_COUNT,
            word_types: Vec::new(),
        },
    );
    DictionaryTooltipCard {
        word: card.word.clone(),
        display_word: card.word.clone(),
        href: word_href,
        word_type: card.word_type.clone(),
        word_type_key: normalize_word_type_filter(&card.word_type),
        selmaho: card.selmaho,
        ipa: dictionary_word_ipa(&card.word),
        similarity: card
            .similarity
            .map(|similarity| format!("{:.0}%", similarity * 100.0)),
        votes: card
            .votes
            .map(|votes| VlackuVoteDisplay::Known(format_votes(votes)))
            .unwrap_or(VlackuVoteDisplay::Unknown),
        rafsi: card.rafsi,
        glosses: card.glosses,
        definition: parse_vlacku_inline_text(jbotci_dictionary_data::english(), &card.definition),
        notes: parse_vlacku_inline_text(jbotci_dictionary_data::english(), &card.notes),
        decomposition: card
            .decomposition
            .into_iter()
            .map(|piece| {
                let source_href = piece.source.as_ref().map(|source| {
                    vlacku_web_url(
                        base_path,
                        &VlackuWebState {
                            mode: VlackuWebMode::Word,
                            query: source.clone(),
                            count: VLACKU_WEB_DEFAULT_COUNT,
                            word_types: Vec::new(),
                        },
                    )
                });
                VlackuCompositionPiece {
                    kind: match piece.kind {
                        VlackuCompositionKind::Rafsi => VlackuCompositionPieceKind::Rafsi,
                        VlackuCompositionKind::Hyphen => VlackuCompositionPieceKind::Hyphen,
                    },
                    display_surface: piece.surface.clone(),
                    surface: piece.surface,
                    display_source: piece.source.clone(),
                    source: piece.source,
                    source_href,
                }
            })
            .collect(),
        can_add_to_jvozba: word_type_allows_jvozba(&card.word_type),
    }
}

#[requires(true)]
#[ensures(true)]
fn gentufa_bracket_fragments_from_source(
    fragments: &[BracketSourceFragment],
    blocks_layout: &GentufaBlocksLayout,
    dictionary_annotations: &[GentufaBlockAnnotation<DictionaryTooltipCard>],
) -> Vec<GentufaBracketFragment> {
    fragments
        .iter()
        .flat_map(|fragment| {
            gentufa_bracket_fragment_from_source(fragment, blocks_layout, dictionary_annotations)
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn gentufa_bracket_fragment_from_source(
    fragment: &BracketSourceFragment,
    blocks_layout: &GentufaBlocksLayout,
    dictionary_annotations: &[GentufaBlockAnnotation<DictionaryTooltipCard>],
) -> Vec<GentufaBracketFragment> {
    match fragment {
        BracketSourceFragment::Text {
            text,
            range,
            elided,
        } => {
            if text.is_empty() {
                return Vec::new();
            }
            decorated_bracket_fragment(
                vec![GentufaBracketFragment::Text {
                    text: text.clone(),
                    elided: *elided,
                }],
                bracket_source_range_to_web(*range),
                Some(text),
                blocks_layout,
                dictionary_annotations,
            )
        }
        BracketSourceFragment::Span { range, children } => {
            let children = gentufa_bracket_fragments_from_source(
                children,
                blocks_layout,
                dictionary_annotations,
            );
            decorated_bracket_fragment(
                children,
                bracket_source_range_to_web(*range),
                None,
                blocks_layout,
                dictionary_annotations,
            )
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn decorated_bracket_fragment(
    children: Vec<GentufaBracketFragment>,
    range: Option<WebSourceRange>,
    text: Option<&str>,
    blocks_layout: &GentufaBlocksLayout,
    dictionary_annotations: &[GentufaBlockAnnotation<DictionaryTooltipCard>],
) -> Vec<GentufaBracketFragment> {
    if children.is_empty() {
        return Vec::new();
    }
    let color = bracket_color_for_range_and_text(blocks_layout, range, text);
    let tooltip = annotation_for_range_and_text(dictionary_annotations, range, text)
        .and_then(|annotation| annotation.tooltip.clone());
    let href = tooltip.as_ref().map(|card| card.href.clone());
    if color.is_none() && tooltip.is_none() {
        return children;
    }
    vec![GentufaBracketFragment::Span {
        color,
        href,
        tooltip,
        children,
    }]
}

#[requires(true)]
#[ensures(true)]
fn bracket_source_range_to_web(range: Option<BracketSourceRange>) -> Option<WebSourceRange> {
    range.map(|range| WebSourceRange {
        byte_start: range.byte_start,
        byte_end: range.byte_end,
        char_start: 0,
        char_end: 0,
    })
}

#[requires(true)]
#[ensures(true)]
fn annotation_for_range_and_text<'a>(
    dictionary_annotations: &'a [GentufaBlockAnnotation<DictionaryTooltipCard>],
    range: Option<WebSourceRange>,
    text: Option<&str>,
) -> Option<&'a GentufaBlockAnnotation<DictionaryTooltipCard>> {
    let range = range?;
    if let Some(text) = text {
        let exact = dictionary_annotations.iter().find(|annotation| {
            same_byte_range(annotation.range, range) && annotation.text.as_deref() == Some(text)
        });
        if exact.is_some() || range.byte_start == range.byte_end {
            return exact;
        }
    }
    dictionary_annotations
        .iter()
        .find(|annotation| same_byte_range(annotation.range, range))
}

#[requires(true)]
#[ensures(true)]
fn bracket_color_for_range_and_text(
    blocks_layout: &GentufaBlocksLayout,
    range: Option<WebSourceRange>,
    text: Option<&str>,
) -> Option<String> {
    let range = range?;
    if let Some(text) = text {
        let exact = blocks_layout.blocks.iter().find(|block| {
            block.span.is_some_and(|span| same_byte_range(span, range))
                && block.display_text == text
        });
        if let Some(block) = exact {
            return Some(block.color.clone());
        }
        if range.byte_start == range.byte_end {
            return None;
        }
    }
    if let Some(block) = blocks_layout
        .blocks
        .iter()
        .find(|block| block.span.is_some_and(|span| same_byte_range(span, range)))
    {
        return Some(block.color.clone());
    }
    blocks_layout
        .blocks
        .iter()
        .filter(|block| {
            block
                .span
                .is_some_and(|span| byte_range_contains(span, range))
        })
        .min_by_key(|block| block.span.map(byte_range_len).unwrap_or(usize::MAX))
        .map(|block| block.color.clone())
}

#[requires(left.byte_start <= left.byte_end)]
#[requires(right.byte_start <= right.byte_end)]
#[ensures(true)]
fn same_byte_range(left: WebSourceRange, right: WebSourceRange) -> bool {
    left.byte_start == right.byte_start && left.byte_end == right.byte_end
}

#[requires(container.byte_start <= container.byte_end)]
#[requires(part.byte_start <= part.byte_end)]
#[ensures(true)]
fn byte_range_contains(container: WebSourceRange, part: WebSourceRange) -> bool {
    container.byte_start <= part.byte_start && part.byte_end <= container.byte_end
}

#[requires(range.byte_start <= range.byte_end)]
#[ensures(true)]
fn byte_range_len(range: WebSourceRange) -> usize {
    range.byte_end.saturating_sub(range.byte_start)
}

#[requires(true)]
#[ensures(true)]
fn web_card_from_search_card(
    rank: usize,
    card: jbotci_search::vlacku::VlackuCard,
) -> VlackuWebCard {
    VlackuWebCard {
        rank,
        ipa: dictionary_word_ipa(&card.word),
        word: card.word.clone(),
        display_word: card.word.clone(),
        word_type: card.word_type.clone(),
        word_type_key: normalize_word_type_filter(&card.word_type),
        selmaho: card.selmaho,
        similarity: card.similarity,
        votes: card
            .votes
            .map(|votes| VlackuVoteDisplay::Known(format_votes(votes)))
            .unwrap_or(VlackuVoteDisplay::Unknown),
        rafsi: card.rafsi,
        glosses: card.glosses,
        definition: parse_vlacku_inline_text(jbotci_dictionary_data::english(), &card.definition),
        notes: parse_vlacku_inline_text(jbotci_dictionary_data::english(), &card.notes),
        decomposition: card
            .decomposition
            .into_iter()
            .map(|piece| {
                let source_href = piece.source.as_ref().map(|source| {
                    vlacku_web_url(
                        "",
                        &VlackuWebState {
                            mode: VlackuWebMode::Word,
                            query: source.clone(),
                            count: VLACKU_WEB_DEFAULT_COUNT,
                            word_types: Vec::new(),
                        },
                    )
                });
                VlackuCompositionPiece {
                    kind: match piece.kind {
                        VlackuCompositionKind::Rafsi => VlackuCompositionPieceKind::Rafsi,
                        VlackuCompositionKind::Hyphen => VlackuCompositionPieceKind::Hyphen,
                    },
                    display_surface: piece.surface.clone(),
                    surface: piece.surface,
                    display_source: piece.source.clone(),
                    source: piece.source,
                    source_href,
                }
            })
            .collect(),
        can_add_to_jvozba: word_type_allows_jvozba(&card.word_type),
    }
}

#[requires(true)]
#[ensures(true)]
fn dictionary_word_ipa(word: &str) -> Option<String> {
    let words = jbotci_morphology::segment_words_with_modifiers(word).ok()?;
    ipa_morphology_text(&words, word).ok()
}

#[requires(true)]
#[ensures(true)]
fn word_type_allows_jvozba(word_type: &str) -> bool {
    let normalized = normalize_word_type_filter(word_type);
    !normalized.contains("fu'ivla") && !normalized.contains("fuivla")
}

#[requires(true)]
#[ensures(true)]
pub fn vlacku_word_type_options(selected_values: &[String]) -> Vec<VlackuWordTypeOption> {
    dictionary_word_type_options(selected_values)
}

#[requires(true)]
#[ensures(true)]
pub fn toggle_vlacku_word_type_selection(selected_values: &[String], value: &str) -> Vec<String> {
    let selected = normalize_vlacku_state(&VlackuWebState {
        mode: VlackuWebMode::Word,
        query: String::new(),
        count: VLACKU_WEB_DEFAULT_COUNT,
        word_types: selected_values.to_vec(),
    })
    .word_types;
    let normalized = grouped_word_type_filter_key(&normalize_word_type_filter(value));
    let mut output = selected;
    if normalized == "brivla" {
        let children = vlacku_brivla_child_filter_values();
        let all_selected = children
            .iter()
            .all(|child| output.iter().any(|candidate| candidate == child));
        if all_selected {
            output.retain(|candidate| !children.iter().any(|child| child == candidate));
        } else {
            for child in children {
                if !output.iter().any(|candidate| candidate == child) {
                    output.push((*child).to_owned());
                }
            }
        }
    } else if output.iter().any(|candidate| candidate == &normalized) {
        output.retain(|candidate| candidate != &normalized);
    } else if !normalized.is_empty() {
        output.push(normalized);
    }
    output
}

#[requires(true)]
#[ensures(true)]
pub fn vlacku_brivla_filter_indeterminate(selected_values: &[String]) -> bool {
    let selected = normalize_vlacku_state(&VlackuWebState {
        mode: VlackuWebMode::Word,
        query: String::new(),
        count: VLACKU_WEB_DEFAULT_COUNT,
        word_types: selected_values.to_vec(),
    })
    .word_types;
    let children = vlacku_brivla_child_filter_values();
    let selected_count = children
        .iter()
        .filter(|child| selected.iter().any(|candidate| candidate == *child))
        .count();
    selected_count > 0 && selected_count < children.len()
}

#[requires(true)]
#[ensures(true)]
fn dictionary_word_type_options(selected_values: &[String]) -> Vec<VlackuWordTypeOption> {
    let selected_values = normalize_vlacku_state(&VlackuWebState {
        mode: VlackuWebMode::Word,
        query: String::new(),
        count: VLACKU_WEB_DEFAULT_COUNT,
        word_types: selected_values.to_vec(),
    })
    .word_types;
    let brivla_child_values = vlacku_brivla_child_filter_values();
    let brivla_selected_count = brivla_child_values
        .iter()
        .filter(|value| selected_values.iter().any(|selected| selected == **value))
        .count();
    dictionary_word_type_option_templates()
        .iter()
        .map(|template| VlackuWordTypeOption {
            label: template.label.clone(),
            section: template.section,
            selected: if template.value == "brivla" {
                brivla_selected_count == brivla_child_values.len()
            } else {
                selected_values
                    .iter()
                    .any(|selected| selected == &template.value)
            },
            indeterminate: template.value == "brivla"
                && brivla_selected_count > 0
                && brivla_selected_count < brivla_child_values.len(),
            value: template.value.clone(),
            count: template.count,
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn dictionary_word_type_option_templates() -> &'static [VlackuWordTypeOptionTemplate] {
    VLACKU_WORD_TYPE_OPTION_TEMPLATES.get_or_init(|| {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for entry in jbotci_dictionary_data::english().entries() {
            let key = dictionary_option_key(entry);
            *counts.entry(key).or_default() += 1;
        }
        let brivla_count = vlacku_brivla_child_filter_values()
            .iter()
            .filter_map(|value| counts.get(*value))
            .copied()
            .sum::<usize>();
        if brivla_count > 0 {
            counts.insert("brivla".to_owned(), brivla_count);
        }
        let mut templates = counts
            .into_iter()
            .filter(|(value, _)| is_visible_word_type_filter(value))
            .map(|(value, count)| {
                new!(VlackuWordTypeOptionTemplate {
                    label: word_type_label(&value),
                    section: word_type_section(&value),
                    value,
                    count,
                })
            })
            .collect::<Vec<_>>();
        templates.sort_by(|left, right| {
            word_type_order_key(left.section, &left.value)
                .cmp(&word_type_order_key(right.section, &right.value))
        });
        templates
    })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn dictionary_option_key(entry: &DictionaryEntry<'_>) -> String {
    let normalized = normalize_word_type_filter(entry.word_type.as_str());
    grouped_word_type_filter_key(&normalized)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlacku_brivla_child_filter_values() -> &'static [&'static str] {
    &["gismu", "lujvo", "fu'ivla"]
}

#[requires(true)]
#[ensures(true)]
fn vlacku_url_word_type_values(selected_values: &[String]) -> Vec<String> {
    let children = vlacku_brivla_child_filter_values();
    if children
        .iter()
        .all(|child| selected_values.iter().any(|selected| selected == *child))
    {
        let mut values = vec!["brivla".to_owned()];
        values.extend(
            selected_values
                .iter()
                .filter(|value| !children.iter().any(|child| child == value))
                .cloned(),
        );
        values
    } else {
        selected_values.to_vec()
    }
}

#[requires(true)]
#[ensures(true)]
fn is_visible_word_type_filter(value: &str) -> bool {
    matches!(
        value,
        "brivla" | "gismu" | "lujvo" | "fu'ivla" | "cmavo" | "letteral" | "cmevla" | "phrase"
    )
}

#[requires(true)]
#[ensures(true)]
fn build_vlacku_dictionary_info() -> VlackuDictionaryInfo {
    let dictionary = jbotci_dictionary_data::english();
    let mut rafsi = BTreeMap::new();
    for entry in dictionary.entries() {
        for rafsi_value in entry.rafsi {
            rafsi.insert(rafsi_value.0, ());
        }
    }
    let word_type_counts = dictionary_word_type_options(&[])
        .into_iter()
        .filter(|option| option.value != "brivla")
        .map(|option| VlackuDictionaryWordTypeCount {
            label: option.label,
            count: option.count,
        })
        .collect();
    VlackuDictionaryInfo {
        entry_count: dictionary.entries().len(),
        rafsi_count: rafsi.len(),
        word_type_counts,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn word_type_label(value: &str) -> String {
    if value.is_empty() {
        "other".to_owned()
    } else {
        value.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn word_type_section(value: &str) -> VlackuWordTypeSection {
    if value == "cmavo" || value == "letteral" {
        VlackuWordTypeSection::Cmavo
    } else if value == "cmevla" || value == "obsolete-cmevla" {
        VlackuWordTypeSection::Cmevla
    } else if value == "brivla" || is_brivla_like(value) {
        VlackuWordTypeSection::Brivla
    } else {
        VlackuWordTypeSection::Other
    }
}

#[requires(true)]
#[ensures(true)]
fn word_type_order_key(section: VlackuWordTypeSection, value: &str) -> (u8, String) {
    let section_order = match section {
        VlackuWordTypeSection::Brivla if value == "brivla" => 0,
        VlackuWordTypeSection::Brivla if value == "gismu" => 1,
        VlackuWordTypeSection::Brivla if value == "lujvo" => 2,
        VlackuWordTypeSection::Brivla if value == "fu'ivla" => 3,
        VlackuWordTypeSection::Brivla => 3,
        VlackuWordTypeSection::Cmavo if value == "cmavo" => 4,
        VlackuWordTypeSection::Cmavo if value == "letteral" => 5,
        VlackuWordTypeSection::Cmavo => 5,
        VlackuWordTypeSection::Cmevla => 6,
        VlackuWordTypeSection::Other => 7,
    };
    (section_order, value.to_owned())
}

#[requires(true)]
#[ensures(true)]
fn parse_vlacku_inline_text(dictionary: &Dictionary<'_>, text: &str) -> Vec<VlackuInline> {
    let mut output = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        let Some(open_index) = remaining.find('$') else {
            append_vlacku_text_inlines(dictionary, remaining, &mut output);
            break;
        };
        append_vlacku_text_inlines(dictionary, &remaining[..open_index], &mut output);
        let after_open = &remaining[open_index + 1..];
        let Some(close_index) = after_open.find('$') else {
            append_vlacku_text_inlines(dictionary, &remaining[open_index..], &mut output);
            break;
        };
        let math_body = &after_open[..close_index];
        if let Some(math) = parse_vlacku_math(math_body) {
            output.push(new!(VlackuInline::Math(math)));
        } else {
            push_vlacku_text_inline(&format!("${math_body}$"), &mut output);
        }
        remaining = &after_open[close_index + 1..];
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn append_vlacku_text_inlines(
    dictionary: &Dictionary<'_>,
    text: &str,
    output: &mut Vec<VlackuInline>,
) {
    let mut remaining = text;
    while !remaining.is_empty() {
        let Some(open_index) = remaining.find('{') else {
            push_vlacku_text_inline(remaining, output);
            break;
        };
        push_vlacku_text_inline(&remaining[..open_index], output);
        let after_open = &remaining[open_index + 1..];
        let Some(close_index) = after_open.find('}') else {
            push_vlacku_text_inline(&remaining[open_index..], output);
            break;
        };
        let inside = &after_open[..close_index];
        let link_value = inside.trim();
        if is_vlacku_word_link(link_value) {
            output.push(new!(VlackuInline::WordRef {
                label: link_value.to_owned(),
                href: vlacku_web_url(
                    "",
                    &VlackuWebState {
                        mode: VlackuWebMode::Word,
                        query: link_value.to_owned(),
                        count: VLACKU_WEB_DEFAULT_COUNT,
                        word_types: Vec::new(),
                    },
                ),
                can_add_to_jvozba: dictionary_word_allows_jvozba(dictionary, link_value),
            }));
        } else {
            push_vlacku_text_inline(&format!("{{{inside}}}"), output);
        }
        remaining = &after_open[close_index + 1..];
    }
}

#[requires(true)]
#[ensures(true)]
fn push_vlacku_text_inline(text: &str, output: &mut Vec<VlackuInline>) {
    if !text.is_empty() {
        output.push(new!(VlackuInline::Text(text.to_owned())));
    }
}

#[requires(true)]
#[ensures(true)]
fn is_vlacku_word_link(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_whitespace)
}

#[requires(true)]
#[ensures(true)]
fn dictionary_word_allows_jvozba(dictionary: &Dictionary<'_>, word: &str) -> bool {
    dictionary
        .lookup_word(word)
        .map(|entry| word_type_allows_jvozba(entry.word_type.as_str()))
        .unwrap_or(true)
}

#[requires(true)]
#[ensures(true)]
fn parse_vlacku_math(source: &str) -> Option<VlackuMath> {
    let parts = parse_vlacku_math_parts(source)?;
    if parts.is_empty() {
        None
    } else {
        Some(new!(VlackuMath { parts }))
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_vlacku_math_parts(source: &str) -> Option<Vec<VlackuMathPart>> {
    let chars = source.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    let mut parts = Vec::new();
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            let start = index;
            while index < chars.len() && chars[index].is_whitespace() {
                index += 1;
            }
            parts.push(new!(VlackuMathPart::Text(
                chars[start..index].iter().collect()
            )));
        } else if ch == '='
            || ch == ','
            || ch == ';'
            || ch == ':'
            || ch == '/'
            || ch == '+'
            || ch == '-'
        {
            parts.push(new!(VlackuMathPart::Operator(ch.to_string())));
            index += 1;
        } else if ch.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < chars.len() && chars[index].is_ascii_alphabetic() {
                index += 1;
            }
            let stem = chars[start..index].iter().collect::<String>();
            let subscript = if index < chars.len() && chars[index] == '_' {
                index += 1;
                parse_vlacku_math_subscript(&chars, &mut index)?
            } else {
                None
            };
            parts.push(new!(VlackuMathPart::Variable { stem, subscript }));
        } else if ch.is_ascii_digit() {
            let start = index;
            while index < chars.len() && chars[index].is_ascii_digit() {
                index += 1;
            }
            parts.push(new!(VlackuMathPart::Text(
                chars[start..index].iter().collect()
            )));
        } else {
            return None;
        }
    }
    Some(parts)
}

#[requires(*index <= chars.len())]
#[ensures(*index <= chars.len())]
fn parse_vlacku_math_subscript(chars: &[char], index: &mut usize) -> Option<Option<String>> {
    if *index >= chars.len() {
        return None;
    }
    if chars[*index] == '{' {
        *index += 1;
        let start = *index;
        while *index < chars.len() && chars[*index] != '}' {
            *index += 1;
        }
        if *index >= chars.len() {
            return None;
        }
        let body = chars[start..*index].iter().collect::<String>();
        *index += 1;
        if body.is_empty() {
            None
        } else {
            Some(Some(body))
        }
    } else {
        let start = *index;
        while *index < chars.len() && chars[*index].is_ascii_alphanumeric() {
            *index += 1;
        }
        if start == *index {
            None
        } else {
            Some(Some(chars[start..*index].iter().collect()))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_query_pairs(query: &str) -> Vec<(String, String)> {
    let trimmed = query.strip_prefix('?').unwrap_or(query);
    trimmed
        .split('&')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            (percent_decode(key), percent_decode(value))
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn percent_decode(input: &str) -> String {
    let mut output = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'+' {
            output.push(b' ');
            index += 1;
        } else if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(value) = u8::from_str_radix(&input[index + 1..index + 3], 16) {
                output.push(value);
                index += 3;
            } else {
                output.push(bytes[index]);
                index += 1;
            }
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}

#[requires(true)]
#[ensures(true)]
fn percent_encode(input: &str) -> String {
    input
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            other => format!("%{other:02X}").chars().collect(),
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn _official_vote_marker_for_linking(entry: &DictionaryEntry<'_>) -> Option<String> {
    let votes = entry.score.get().round() as i32;
    (votes > OFFICIAL_WORD_VOTE_THRESHOLD).then(|| format_votes(votes))
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

    #[requires(true)]
    #[ensures(true)]
    fn bracket_fragment_text(fragments: &[GentufaBracketFragment]) -> String {
        let mut output = String::new();
        for fragment in fragments {
            append_bracket_fragment_text(fragment, &mut output);
        }
        output
    }

    #[requires(true)]
    #[ensures(true)]
    fn append_bracket_fragment_text(fragment: &GentufaBracketFragment, output: &mut String) {
        match fragment {
            GentufaBracketFragment::Text { text, .. } => output.push_str(text),
            GentufaBracketFragment::Span { children, .. } => {
                for child in children {
                    append_bracket_fragment_text(child, output);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bracket_fragments_contain_tooltip_for(
        fragments: &[GentufaBracketFragment],
        word: &str,
    ) -> bool {
        fragments
            .iter()
            .any(|fragment| bracket_fragment_contains_tooltip_for(fragment, word))
    }

    #[requires(true)]
    #[ensures(true)]
    fn bracket_fragment_contains_tooltip_for(
        fragment: &GentufaBracketFragment,
        word: &str,
    ) -> bool {
        match fragment {
            GentufaBracketFragment::Text { .. } => false,
            GentufaBracketFragment::Span {
                tooltip, children, ..
            } => {
                tooltip.as_ref().is_some_and(|card| card.word == word)
                    || children
                        .iter()
                        .any(|child| bracket_fragment_contains_tooltip_for(child, word))
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bracket_fragments_contain_block_color(
        fragments: &[GentufaBracketFragment],
        color: &str,
    ) -> bool {
        fragments
            .iter()
            .any(|fragment| bracket_fragment_contains_block_color(fragment, color))
    }

    #[requires(true)]
    #[ensures(true)]
    fn bracket_fragment_contains_block_color(
        fragment: &GentufaBracketFragment,
        color: &str,
    ) -> bool {
        match fragment {
            GentufaBracketFragment::Text { .. } => false,
            GentufaBracketFragment::Span {
                color: fragment_color,
                children,
                ..
            } => {
                fragment_color.as_deref() == Some(color)
                    || children
                        .iter()
                        .any(|child| bracket_fragment_contains_block_color(child, color))
            }
        }
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
    fn elided_terminators_only_render_when_requested() {
        let hidden = parse_success("mi klama");
        assert!(
            hidden
                .blocks_layout
                .blocks
                .iter()
                .all(|block| !block.is_elided)
        );
        assert!(
            hidden
                .tree_rows
                .iter()
                .flat_map(|row| row.cells.iter())
                .all(|cell| !cell.is_elided)
        );

        let request = GentufaWebRequest {
            text: "mi klama".to_owned(),
            options: GentufaWebOptions {
                show_elided: true,
                ..GentufaWebOptions::default()
            },
        };
        let GentufaWebResult::Success(shown) = parse_gentufa_for_web(&request) else {
            panic!("expected successful parse");
        };
        assert!(shown.tree_rows.iter().any(|row| {
            row.label == "Cmavo"
                && row
                    .cells
                    .iter()
                    .any(|cell| cell.is_word && cell.is_elided && cell.text == "vau")
                && !row.glosses.is_empty()
                && row.definition.is_some()
        }));
        assert!(
            bracket_fragments_contain_tooltip_for(&shown.bracket_fragments, "vau"),
            "{:?}",
            shown.bracket_fragments
        );
        let elided_block_labels = shown
            .blocks_layout
            .blocks
            .iter()
            .filter(|block| block.is_leaf && block.is_elided)
            .map(|block| block.label.clone())
            .collect::<Vec<_>>();
        assert!(
            elided_block_labels.iter().any(|label| label == "vau"),
            "{elided_block_labels:?}"
        );
        let vau_block = shown
            .blocks_layout
            .blocks
            .iter()
            .find(|block| block.is_leaf && block.is_elided && block.label == "vau")
            .expect("vau elided block");
        assert!(!vau_block.glosses.is_empty());
        assert!(vau_block.definition.is_some());
        assert_eq!(
            vau_block.tooltip.as_ref().map(|card| card.href.as_str()),
            Some("/vlacku/vau")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn single_synthetic_elided_leaf_keeps_elided_block_metadata() {
        let request = GentufaWebRequest {
            text: "klama cei brode i mi brode do ta'i ny fi'o mleca bervi fe'u i brode".to_owned(),
            options: GentufaWebOptions {
                show_elided: true,
                show_glosses: false,
                ..GentufaWebOptions::default()
            },
        };
        let GentufaWebResult::Success(success) = parse_gentufa_for_web(&request) else {
            panic!("expected successful parse");
        };
        let ku_blocks = success
            .blocks_layout
            .blocks
            .iter()
            .filter(|block| block.is_leaf && block.label == "ku")
            .collect::<Vec<_>>();
        assert!(!ku_blocks.is_empty(), "{:?}", success.blocks_layout.blocks);
        assert!(
            ku_blocks.iter().all(|block| block.is_elided),
            "{ku_blocks:?}"
        );
        assert!(
            ku_blocks.iter().all(|block| block
                .span
                .is_some_and(|range| range.byte_start == range.byte_end)),
            "{ku_blocks:?}"
        );
        assert!(
            ku_blocks.iter().any(|block| block
                .tooltip
                .as_ref()
                .is_some_and(|card| card.word == "ku")
                && !block.glosses.is_empty()
                && block.definition.is_some()),
            "{ku_blocks:?}"
        );
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
    fn reported_fiho_compound_leaves_do_not_span_phantom_bottom_row() {
        let success = parse_success("klama cei brode i mi brode do ta'i ny fi'o mleca bervi fe'u");
        let mleca = success
            .blocks_layout
            .blocks
            .iter()
            .find(|block| block.is_leaf && block.raw_text == "mleca")
            .expect("mleca leaf block");
        let bervi = success
            .blocks_layout
            .blocks
            .iter()
            .find(|block| block.is_leaf && block.raw_text == "bervi")
            .expect("bervi leaf block");

        assert_eq!(success.blocks_layout.max_row, 7);
        assert_eq!(mleca.row_span, 1);
        assert_eq!(bervi.row_span, 1);
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
    fn gentufa_dictionary_annotations_fill_glosses_and_tooltips() {
        let success = parse_success("mi klama");
        let klama_block = success
            .blocks_layout
            .blocks
            .iter()
            .find(|block| block.is_leaf && block.raw_text == "klama")
            .expect("klama leaf block");

        assert!(klama_block.glosses.iter().any(|gloss| gloss == "come"));
        assert!(
            klama_block
                .definition
                .as_deref()
                .is_some_and(|definition| definition.contains("comes/goes"))
        );
        assert_eq!(
            klama_block.tooltip.as_ref().map(|card| card.word.as_str()),
            Some("klama")
        );
        assert!(success.features.glosses);
        assert!(success.features.definitions);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_bracket_fragments_are_colored_and_linked() {
        let success = parse_success("mi klama");
        let fragment_text = bracket_fragment_text(&success.bracket_fragments);
        assert_eq!(fragment_text, success.brackets_text);
        assert!(
            bracket_fragments_contain_tooltip_for(&success.bracket_fragments, "klama"),
            "{:?}",
            success.bracket_fragments
        );
        assert!(
            bracket_fragments_contain_block_color(
                &success.bracket_fragments,
                &success
                    .blocks_layout
                    .blocks
                    .iter()
                    .find(|block| block.is_leaf && block.raw_text == "klama")
                    .expect("klama block")
                    .color,
            ),
            "{:?}",
            success.bracket_fragments
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_tooltip_for_word_contains_vlacku_card_content() {
        let card = dictionary_tooltip_for_word("", "klama").expect("klama tooltip");
        assert_eq!(card.word, "klama");
        assert!(card.glosses.iter().any(|gloss| gloss == "come"));
        assert!(!card.definition.is_empty());
        assert!(matches!(card.votes, VlackuVoteDisplay::Known(_)));
        assert_eq!(card.href, "/vlacku/klama");
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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_route_parses_v0_query_shape() {
        let state = parse_vlacku_web_route("/vlacku", "?mode=rafsi&q=kla&count=40&wordType=brivla");

        assert_eq!(state.mode, VlackuWebMode::Rafsi);
        assert_eq!(state.query, "kla");
        assert_eq!(state.count, 40);
        assert_eq!(
            state.word_types,
            vec!["gismu".to_owned(), "lujvo".to_owned(), "fu'ivla".to_owned()]
        );
        assert_eq!(
            vlacku_web_url("", &state),
            "/vlacku?mode=rafsi&q=kla&count=40&wordType=brivla"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_route_parses_section_index_and_search_shapes() {
        let root = parse_cukta_web_route("/cukta/", "");
        assert_eq!(
            cukta_web_url("", &root),
            "/cukta/section/section-what-is-lojban"
        );

        let section = parse_cukta_web_route("/cukta/section/section-what-is-lojban", "");
        assert_eq!(
            cukta_web_url("", &section),
            "/cukta/section/section-what-is-lojban"
        );

        let index = parse_cukta_web_route("/cukta/index", "");
        assert_eq!(cukta_web_url("", &index), "/cukta/index");

        let search = parse_cukta_web_route(
            "/cukta/search",
            "?mode=valsi&q=lojban&count=40&target=section,example",
        );
        let CuktaWebView::Search(search_state) = search.view else {
            panic!("expected search state");
        };
        assert_eq!(search_state.mode, CuktaWebMode::Word);
        assert_eq!(search_state.query, "lojban");
        assert_eq!(search_state.count, 40);
        assert_eq!(
            search_state.targets,
            vec!["section".to_owned(), "example".to_owned()]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_route_round_trips_primary_url_state() {
        let state = parse_gentufa_web_route(
            "/gentufa",
            "?text=mi+klama&dialect=allow-cgv&view=tree&glosses=false&elided=true",
        );

        assert_eq!(state.text, "mi klama");
        assert_eq!(state.dialect.as_deref(), Some("allow-cgv"));
        assert_eq!(state.view_mode, GentufaWebViewMode::Tree);
        assert!(!state.show_glosses);
        assert!(state.show_elided);
        assert_eq!(
            gentufa_web_url("/jbotci", &state),
            "/jbotci/gentufa?text=mi+klama&dialect=allow-cgv&view=tree&glosses=false&elided=true"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn web_route_and_metadata_follow_v0_page_details() {
        let route = parse_web_route("/vlacku/klama", "");
        assert_eq!(web_route_url("/jbotci", &route), "/jbotci/vlacku/klama");
        let meta = build_page_meta("/jbotci", &route);
        assert_eq!(meta.title, "klama - jbotci vlacku");
        assert_eq!(meta.description, "Dictionary lookup for “klama”.");
        assert_eq!(meta.canonical_url, "/jbotci/vlacku/klama");
        assert!(meta.image.is_none());

        let gentufa = build_page_meta(
            "",
            &WebRoute::Gentufa(GentufaWebState {
                text: "mi klama".to_owned(),
                dialect: None,
                view_mode: GentufaWebViewMode::Blocks,
                show_elided: false,
                show_glosses: true,
            }),
        );
        assert_eq!(gentufa.title, "mi klama - jbotci gentufa");
        assert!(gentufa.description.starts_with("Parse succeeded:"));
        assert!(gentufa.image.is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn web_compute_gentufa_matches_direct_builder() {
        let state = GentufaWebState {
            text: "mi klama".to_owned(),
            dialect: None,
            view_mode: GentufaWebViewMode::Blocks,
            show_elided: false,
            show_glosses: true,
        };
        let request = GentufaWebRequest {
            text: state.text.clone(),
            options: GentufaWebOptions::default(),
        };

        let response = run_web_compute_request(WebComputeRequest::GentufaPage {
            base_path: "/jbotci".to_owned(),
            state: state.clone(),
            request: request.clone(),
        })
        .expect("gentufa compute succeeds");

        let WebComputeResponse::GentufaPage { result, meta } = response else {
            panic!("expected gentufa page response");
        };
        let direct = parse_gentufa_for_web(&request);
        assert_eq!(result, direct);
        assert_eq!(
            meta,
            build_gentufa_page_meta_from_result("/jbotci", &state, &direct)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn web_compute_cukta_and_vlacku_match_direct_builders() {
        let cukta_state = CuktaWebState::default();
        let cukta_response = run_web_compute_request(WebComputeRequest::CuktaPage {
            base_path: String::new(),
            state: cukta_state.clone(),
        })
        .expect("cukta compute succeeds");
        let WebComputeResponse::CuktaPage { page, meta } = cukta_response else {
            panic!("expected cukta page response");
        };
        assert_eq!(page, build_cukta_web_page("", &cukta_state));
        assert_eq!(
            meta,
            build_page_meta("", &WebRoute::Cukta(cukta_state.clone()))
        );

        let vlacku_state = VlackuWebState {
            mode: VlackuWebMode::Word,
            query: "klama".to_owned(),
            count: 10,
            word_types: Vec::new(),
        };
        let vlacku_response = run_web_compute_request(WebComputeRequest::VlackuPage {
            base_path: String::new(),
            state: vlacku_state.clone(),
        })
        .expect("vlacku compute succeeds");
        let WebComputeResponse::VlackuPage { result, meta } = vlacku_response else {
            panic!("expected vlacku page response");
        };
        assert_eq!(result, build_vlacku_web_result(&vlacku_state));
        assert_eq!(meta, build_page_meta("", &WebRoute::Vlacku(vlacku_state)));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn web_compute_json_round_trips_embedding_corpus_response() {
        let request =
            serde_json::to_string(&WebComputeRequest::EmbeddingCorpusJson).expect("valid request");
        let response_json = run_web_compute_request_json(&request).expect("compute succeeds");
        let response =
            serde_json::from_str::<WebComputeResponse>(&response_json).expect("valid response");
        let WebComputeResponse::EmbeddingCorpusJson { json } = response else {
            panic!("expected embedding corpus response");
        };
        assert!(json.contains(WEB_EMBEDDING_MODEL_KEY));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_chapter_metadata_uses_available_chapter_image() {
        let site = embedded_cll_site().expect("embedded CLL site");
        let (section_id, expected_file_name) = site
            .chapters
            .iter()
            .find_map(|chapter| {
                let file_name = chapter
                    .prelude_blocks
                    .iter()
                    .find_map(|block| match block {
                        CllBlock::Media { src, .. } => src.rsplit('/').next().map(str::to_owned),
                        _ => None,
                    })?;
                let section_id = chapter.root_section_ids.first()?.clone();
                Some((section_id, file_name))
            })
            .expect("at least one CLL chapter image");

        let meta = build_page_meta(
            "/jbotci",
            &WebRoute::Cukta(CuktaWebState {
                view: CuktaWebView::Section {
                    reference: section_id,
                },
            }),
        );
        let image = meta.image.as_ref().expect("chapter image metadata");
        assert!(image.href.ends_with(&expected_file_name));
        assert!(image.href.starts_with("/jbotci/assets/cll/media/"));
        assert!(image.width > 0);
        assert!(image.height > 0);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_builds_section_word_search_and_semantic_pages() {
        let section_page = build_cukta_web_page("", &CuktaWebState::default());
        assert!(section_page.current_section_id.is_some());
        assert!(matches!(
            section_page.page_kind,
            CuktaPageKind::Section { .. }
        ));

        let word_page = build_cukta_web_page(
            "",
            &CuktaWebState {
                view: CuktaWebView::Search(CuktaWebSearchState {
                    mode: CuktaWebMode::Word,
                    query: "lojban".to_owned(),
                    count: 3,
                    targets: default_cukta_target_values(),
                }),
            },
        );
        let CuktaPageKind::Search {
            results,
            message,
            has_more,
            ..
        } = word_page.page_kind
        else {
            panic!("expected word search page");
        };
        assert!(message.is_none(), "{message:?}");
        assert!(has_more);
        assert_eq!(
            results.first().map(|card| card.label.as_str()),
            Some("4.3. brivla")
        );

        let meaning_state = CuktaWebState {
            view: CuktaWebView::Search(CuktaWebSearchState {
                mode: CuktaWebMode::Meaning,
                query: "lojban".to_owned(),
                count: 3,
                targets: default_cukta_target_values(),
            }),
        };
        let meaning_page = build_cukta_semantic_web_page(
            "",
            &meaning_state,
            &[CuktaSemanticSearchHit {
                chunk_index: 0,
                score: 0.75,
            }],
            None,
        );
        let CuktaPageKind::Search {
            results,
            message,
            mode_options,
            ..
        } = meaning_page.page_kind
        else {
            panic!("expected meaning search page");
        };
        assert!(message.is_none(), "{message:?}");
        assert_eq!(
            results
                .first()
                .map(|result| result.similarity_label.as_deref()),
            Some(Some("75%"))
        );
        assert!(mode_options.iter().all(|option| !option.disabled));

        let loading_meaning_page =
            build_cukta_semantic_web_page_with_loading("", &meaning_state, &[], None, true);
        let CuktaPageKind::Search {
            results, message, ..
        } = loading_meaning_page.page_kind
        else {
            panic!("expected loading meaning search page");
        };
        assert!(results.is_empty());
        assert!(message.is_none(), "{message:?}");

        let empty_meaning_page =
            build_cukta_semantic_web_page_with_loading("", &meaning_state, &[], None, false);
        let CuktaPageKind::Search {
            results, message, ..
        } = empty_meaning_page.page_kind
        else {
            panic!("expected empty meaning search page");
        };
        assert!(results.is_empty());
        assert_eq!(message.as_deref(), Some("No matches found."));

        let section_only_page = build_cukta_semantic_web_page(
            "",
            &CuktaWebState {
                view: CuktaWebView::Search(CuktaWebSearchState {
                    mode: CuktaWebMode::Meaning,
                    query: "lojban".to_owned(),
                    count: 1,
                    targets: vec!["section".to_owned()],
                }),
            },
            &[
                CuktaSemanticSearchHit {
                    chunk_index: 1,
                    score: 0.99,
                },
                CuktaSemanticSearchHit {
                    chunk_index: 0,
                    score: 0.75,
                },
                CuktaSemanticSearchHit {
                    chunk_index: 3,
                    score: 0.74,
                },
            ],
            None,
        );
        let CuktaPageKind::Search {
            results, has_more, ..
        } = section_only_page.page_kind
        else {
            panic!("expected section-only meaning search page");
        };
        assert!(has_more);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, "section");
        assert_eq!(results[0].rank, 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_grouped_word_types_follow_v0_filter_shape() {
        let options = vlacku_word_type_options(&[]);
        let values = options
            .iter()
            .map(|option| option.value.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            values,
            vec![
                "brivla", "gismu", "lujvo", "fu'ivla", "cmavo", "letteral", "cmevla", "phrase"
            ]
        );

        let brivla_children = toggle_vlacku_word_type_selection(&[], "brivla");
        assert_eq!(
            brivla_children,
            vec!["gismu".to_owned(), "lujvo".to_owned(), "fu'ivla".to_owned()]
        );
        assert!(!brivla_children.iter().any(|value| value == "phrase"));
        assert!(!brivla_children.iter().any(|value| value == "letteral"));
        assert!(!vlacku_brivla_filter_indeterminate(&brivla_children));

        let letteral_only = toggle_vlacku_word_type_selection(&[], "letteral");
        assert_eq!(letteral_only, vec!["letteral".to_owned()]);
        assert!(!vlacku_brivla_filter_indeterminate(&letteral_only));

        let gismu_only = toggle_vlacku_word_type_selection(&[], "gismu");
        assert_eq!(gismu_only, vec!["gismu".to_owned()]);
        assert!(vlacku_brivla_filter_indeterminate(&gismu_only));

        assert!(toggle_vlacku_word_type_selection(&brivla_children, "brivla").is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_word_search_returns_cards() {
        let result = build_vlacku_web_result(&VlackuWebState {
            mode: VlackuWebMode::Word,
            query: "klama".to_owned(),
            count: 20,
            word_types: Vec::new(),
        });

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(
            result.cards.first().map(|card| card.word.as_str()),
            Some("klama")
        );
        assert!(
            result
                .cards
                .first()
                .is_some_and(|card| matches!(card.votes, VlackuVoteDisplay::Known(_)))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_word_search_segments_multiword_query_like_v0() {
        let result = build_vlacku_web_result(&VlackuWebState {
            mode: VlackuWebMode::Word,
            query: "mi klama".to_owned(),
            count: 20,
            word_types: Vec::new(),
        });

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(
            result
                .cards
                .iter()
                .map(|card| card.word.as_str())
                .collect::<Vec<_>>(),
            vec!["mi", "klama"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_word_search_prefers_exact_phrase_before_segmenting() {
        let result = build_vlacku_web_result(&VlackuWebState {
            mode: VlackuWebMode::Word,
            query: "ca ma".to_owned(),
            count: 20,
            word_types: Vec::new(),
        });

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(
            result.cards.first().map(|card| card.word.as_str()),
            Some("ca ma")
        );
        assert!(
            !result
                .cards
                .iter()
                .any(|card| card.word == "ca" || card.word == "ma")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_word_search_filters_segmented_components_after_lookup() {
        let result = build_vlacku_web_result(&VlackuWebState {
            mode: VlackuWebMode::Word,
            query: "mi klama".to_owned(),
            count: 20,
            word_types: vec!["gismu".to_owned()],
        });

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(
            result
                .cards
                .iter()
                .map(|card| card.word.as_str())
                .collect::<Vec<_>>(),
            vec!["klama"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_word_search_does_not_return_partial_segmented_results() {
        let result = build_vlacku_web_result(&VlackuWebState {
            mode: VlackuWebMode::Word,
            query: "mi brodau".to_owned(),
            count: 20,
            word_types: Vec::new(),
        });

        assert!(result.cards.is_empty(), "{:?}", result.cards);
        assert!(
            result
                .errors
                .iter()
                .any(|error| error.contains("Invalid Lojban word"))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_segmented_word_lookup_does_not_apply_to_rafsi_mode() {
        let result = build_vlacku_web_result(&VlackuWebState {
            mode: VlackuWebMode::Rafsi,
            query: "kla bau".to_owned(),
            count: 20,
            word_types: Vec::new(),
        });

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert!(result.cards.is_empty(), "{:?}", result.cards);
        assert_eq!(result.message.as_deref(), Some("No matches found."));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_blank_query_returns_dictionary_info_and_semantic_results_render() {
        let blank = build_vlacku_web_result(&VlackuWebState::default());
        assert!(blank.dictionary_info.is_some());

        let dictionary = jbotci_dictionary_data::english();
        let klama_index = dictionary
            .entries()
            .iter()
            .position(|entry| entry.word == "klama")
            .expect("klama exists");
        let meaning = build_vlacku_semantic_web_result(
            &VlackuWebState {
                mode: VlackuWebMode::Meaning,
                query: "go somewhere".to_owned(),
                count: 20,
                word_types: Vec::new(),
            },
            &[VlackuSemanticSearchHit {
                entry_index: klama_index,
                score: 0.91,
            }],
            None,
        );
        assert_eq!(
            meaning.cards.first().map(|card| card.word.as_str()),
            Some("klama")
        );
        assert_eq!(
            meaning.cards.first().and_then(|card| card.similarity),
            Some(0.91)
        );

        let missing = build_vlacku_semantic_web_result(
            &VlackuWebState {
                mode: VlackuWebMode::Meaning,
                query: "klama".to_owned(),
                count: 20,
                word_types: Vec::new(),
            },
            &[],
            Some("Open Settings".to_owned()),
        );
        assert_eq!(missing.message.as_deref(), Some("Open Settings"));

        let loading = build_vlacku_semantic_web_result_with_loading(
            &VlackuWebState {
                mode: VlackuWebMode::Meaning,
                query: "nonsense".to_owned(),
                count: 20,
                word_types: Vec::new(),
            },
            &[],
            None,
            true,
        );
        assert!(loading.cards.is_empty());
        assert!(loading.errors.is_empty(), "{:?}", loading.errors);
        assert!(loading.message.is_none(), "{:?}", loading.message);

        let empty = build_vlacku_semantic_web_result_with_loading(
            &VlackuWebState {
                mode: VlackuWebMode::Meaning,
                query: "nonsense".to_owned(),
                count: 20,
                word_types: Vec::new(),
            },
            &[],
            None,
            false,
        );
        assert!(empty.cards.is_empty());
        assert_eq!(empty.message.as_deref(), Some("No matches found."));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_semantic_nonsense_result_set_renders() {
        let dictionary = jbotci_dictionary_data::english();
        let hit_words = [
            ("nonselsmu", 0.67),
            ("smucau", 0.58),
            ("nonselra'u", 0.54),
            ("nonselkosmu", 0.54),
            ("ko'o'o'o'o", 0.54),
            ("snafu", 0.53),
            ("postmo", 0.52),
            ("roflma'o", 0.52),
            ("narplixau", 0.51),
            ("nunbebna", 0.49),
            ("kosmycau", 0.49),
            ("selterselxeliumadbro", 0.48),
            ("terckasu", 0.48),
            ("bebna", 0.47),
            ("zo si si zei fa'o", 0.47),
            ("selbebna", 0.47),
            ("gleua", 0.46),
            ("nalzungi", 0.46),
            ("nalra'a", 0.46),
            ("tolmencre", 0.46),
        ];
        let hits = hit_words
            .iter()
            .map(|(word, score)| VlackuSemanticSearchHit {
                entry_index: dictionary
                    .entries()
                    .iter()
                    .position(|entry| entry.word == *word)
                    .unwrap_or_else(|| panic!("missing dictionary entry {word}")),
                score: *score,
            })
            .collect::<Vec<_>>();

        let result = build_vlacku_semantic_web_result(
            &VlackuWebState {
                mode: VlackuWebMode::Meaning,
                query: "nonsense".to_owned(),
                count: 20,
                word_types: Vec::new(),
            },
            &hits,
            None,
        );
        let result_json = serde_json::to_string(&result).expect("semantic result serializes");

        assert_eq!(
            result.cards.first().map(|card| card.word.as_str()),
            Some("nonselsmu")
        );
        assert!(result.cards.iter().any(|card| card.word == "ko'o'o'o'o"));
        assert!(
            result
                .cards
                .iter()
                .any(|card| card.word == "zo si si zei fa'o")
        );
        assert!(result.message.is_none(), "{:?}", result.message);
        assert!(result_json.contains("nonselsmu"));
        serde_json::from_str::<VlackuWebResult>(&result_json)
            .expect("semantic result deserializes");
        dictionary_tooltip_for_word("", "zo si si zei fa'o")
            .expect("multi-word dictionary entry tooltip renders");
        for word in ["si", "fa'o"] {
            let tooltip = dictionary_tooltip_for_word("", word).expect("cmavo tooltip renders");
            assert!(tooltip.ipa.is_none(), "{word} should not render blank IPA");
        }

        let response_json = run_web_compute_request_json(
            &serde_json::to_string(&WebComputeRequest::VlackuSemanticPage {
                base_path: "/jbotci".to_owned(),
                state: VlackuWebState {
                    mode: VlackuWebMode::Meaning,
                    query: "nonsense".to_owned(),
                    count: 20,
                    word_types: Vec::new(),
                },
                hits,
                message: None,
                loading: false,
            })
            .expect("compute request serializes"),
        )
        .expect("compute request succeeds");
        let response = serde_json::from_str::<WebComputeResponse>(&response_json)
            .expect("response deserializes");
        let WebComputeResponse::VlackuPage { result, .. } = response else {
            panic!("expected vlacku page response");
        };
        assert!(
            result
                .cards
                .iter()
                .any(|card| card.word == "zo si si zei fa'o")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_exact_missing_card_keeps_lujvo_decomposition() {
        let result = build_vlacku_web_result(&VlackuWebState {
            mode: VlackuWebMode::Word,
            query: "brodau".to_owned(),
            count: 20,
            word_types: Vec::new(),
        });

        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let card = result
            .cards
            .first()
            .expect("expected missing-word headword card");
        assert_eq!(card.word, "brodau");
        assert_eq!(card.word_type_key, "lujvo");
        assert!(card.definition.is_empty());
        assert!(card.glosses.is_empty());
        assert!(!card.decomposition.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_rich_text_parses_math_and_dictionary_links() {
        let spans = parse_vlacku_inline_text(
            jbotci_dictionary_data::english(),
            "$x_1$ refers to {valsi}; unmatched $ stays and {two words} stays.",
        );

        let first = spans.first().expect("expected leading math span");
        let data!(VlackuInline::Math(math)) = first.as_data() else {
            panic!("expected leading math span, got {first:?}");
        };
        let [part] = math.parts.as_slice() else {
            panic!("expected single math variable, got {math:?}");
        };
        let data!(VlackuMathPart::Variable { stem, subscript }) = part.as_data() else {
            panic!("expected math variable, got {part:?}");
        };
        assert_eq!(stem, "x");
        assert_eq!(subscript.as_deref(), Some("1"));
        assert!(spans.iter().any(|span| matches!(
            span.as_data(),
            data!(VlackuInline::WordRef { label, can_add_to_jvozba, .. })
                if label == "valsi" && *can_add_to_jvozba
        )));
        assert!(spans.iter().any(|span| matches!(
            span.as_data(),
            data!(VlackuInline::Text(text)) if text.contains("$ stays")
        )));
        assert!(spans.iter().any(|span| matches!(
            span.as_data(),
            data!(VlackuInline::Text(text)) if text.contains("{two words}")
        )));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_rich_text_marks_fuivla_links_non_addable() {
        let spans = parse_vlacku_inline_text(jbotci_dictionary_data::english(), "{a'anmo}");

        let [span] = spans.as_slice() else {
            panic!("expected one word reference span, got {spans:?}");
        };
        assert!(matches!(
            span.as_data(),
            data!(VlackuInline::WordRef { label, can_add_to_jvozba, .. })
                if label == "a'anmo" && !*can_add_to_jvozba
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_jvozba_builds_in_browser_model() {
        let output = build_vlacku_jvozba_output(
            VlackuJvozbaMode::Lujvo,
            &[
                VlackuJvozbaItem {
                    kind: VlackuJvozbaItemKind::Word,
                    value: "lojbo".to_owned(),
                    source: None,
                    indent_level: 0,
                },
                VlackuJvozbaItem {
                    kind: VlackuJvozbaItemKind::Word,
                    value: "bangu".to_owned(),
                    source: None,
                    indent_level: 0,
                },
            ],
        );

        assert!(matches!(
            output,
            VlackuJvozbaOutput::Success { ref word, .. } if word == "jbobau"
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedding_worker_corpus_json_uses_browser_worker_schema() {
        let json = embedding_worker_corpus_json();
        let value = serde_json::from_str::<serde_json::Value>(&json)
            .expect("embedding worker corpus should be valid JSON");

        assert_eq!(
            value.get("modelKey").and_then(serde_json::Value::as_str),
            Some(WEB_EMBEDDING_MODEL_KEY)
        );
        assert!(value.get("model-key").is_none());
        assert!(value.get("model_key").is_none());
        assert!(
            value
                .get("dictionary")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|items| !items.is_empty())
        );
        let dictionary = value
            .get("dictionary")
            .and_then(serde_json::Value::as_array)
            .expect("dictionary rows");
        let klama = dictionary
            .iter()
            .find(|item| {
                item.get("input")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|input| input.contains("title: klama | text:"))
            })
            .expect("klama dictionary row");
        assert_eq!(
            klama.get("kind").and_then(serde_json::Value::as_str),
            Some("gismu")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_jvozba_segments_match_cli_coloring_rules() {
        let segments = vec![
            new!(JvozbaSegment {
                kind: JvozbaSegmentKind::Rafsi,
                text: "cme".to_owned(),
            }),
            new!(JvozbaSegment {
                kind: JvozbaSegmentKind::Rafsi,
                text: "vlas".to_owned(),
            }),
        ];

        let rendered = render_jvozba_segments(VlackuJvozbaMode::Cmevla, &segments);
        assert_eq!(
            rendered,
            vec![
                VlackuJvozbaSegment {
                    kind: VlackuJvozbaSegmentKind::Rafsi,
                    text: "cme".to_owned(),
                    tone: VlackuJvozbaSegmentTone::RafsiA,
                },
                VlackuJvozbaSegment {
                    kind: VlackuJvozbaSegmentKind::Rafsi,
                    text: "vla".to_owned(),
                    tone: VlackuJvozbaSegmentTone::RafsiB,
                },
                VlackuJvozbaSegment {
                    kind: VlackuJvozbaSegmentKind::Hyphen,
                    text: "s".to_owned(),
                    tone: VlackuJvozbaSegmentTone::Hyphen,
                },
            ]
        );
    }
}
