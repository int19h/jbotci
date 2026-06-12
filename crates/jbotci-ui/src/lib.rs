use dioxus::core::Task;
use dioxus::prelude::*;
use jbotci_cll::{
    CllBlock, CllEbnfEntry, CllEbnfToken, CllInline, CllInterlinearRow, CllLanguageSpanKind,
    CllLinkKind, CllLojbanizationLine, CllLujvoPart, CllSimpleListOrientation, CllTableCell,
    cll_link_href, embedded_cll_site, wrap_ebnf_choice_lines,
};
use jbotci_diagnostics::{
    Diagnostic, DiagnosticLabel, DiagnosticSeverity, DiagnosticStyledNote, DiagnosticTextRole,
    DiagnosticTextSegment,
};
use jbotci_dialect::{
    CustomDialect, DialectSettings, add_dialect_formula_reference, builtin_dialect_names,
    custom_dialect_definition_to_johau_uri_with_custom_dialects, custom_dialect_is_valid,
    dialect_definition_to_text, dialect_formula_top_level_references,
    dialect_name_shows_in_gentufa_picker, find_builtin_dialect, import_johau_dialect_settings,
    parse_dialect_selection_formula, remove_dialect_formula_reference,
    replace_dialect_formula_reference,
};
use jbotci_output::{
    GlideMark, PhonemeRenderOptions, StressMark,
    qr_code::{encode_qr_alphanumeric_h, qr_code_svg},
    render_lojban_text_for_script,
};
#[cfg(test)]
use jbotci_web_core::ReferenceSlotLabel;
use jbotci_web_core::{
    APPLE_TOUCH_ICON_ASSET_PATH, CUKTA_WEB_DEFAULT_COUNT, CUKTA_WEB_MAX_COUNT, CuktaModeOption,
    CuktaPageData, CuktaPageKind, CuktaSearchResultCard, CuktaSemanticSearchHit, CuktaTargetOption,
    CuktaTocNode, CuktaWebMode, CuktaWebSearchState, CuktaWebState, CuktaWebView,
    DictionaryTooltipCard, FAVICON_ASSET_PATH, GentufaBlock, GentufaBlocksLayout,
    GentufaBracketFragment, GentufaCell, GentufaError, GentufaScript, GentufaSuccess,
    GentufaTreeGuide, GentufaTreeRow, GentufaWebOptions, GentufaWebRequest, GentufaWebResult,
    GentufaWebState, GentufaWebViewMode, MANIFEST_ASSET_PATH, PageMeta, ReferenceLabel,
    ReferenceMarker, ReferenceMarkerRole, ReferenceTooltip, ReferenceTooltipInline,
    ReferenceTooltipInlineData, ReferenceTooltipRow, VLACKU_WEB_DEFAULT_COUNT,
    VLACKU_WEB_MAX_COUNT, VlackuCompositionPiece, VlackuCompositionPieceKind,
    VlackuDictionaryCountNode, VlackuDictionaryInfo, VlackuInline, VlackuInlineData,
    VlackuJvozbaItem, VlackuJvozbaItemKind, VlackuJvozbaMode, VlackuJvozbaOutput,
    VlackuJvozbaSegmentTone, VlackuMath, VlackuSemanticSearchHit, VlackuVoteDisplay,
    VlackuWebAuthor, VlackuWebCard, VlackuWebMode, VlackuWebResult, VlackuWebState,
    VlackuWordTypeOption, VlackuWordTypeSection, WebComputeRequest, WebComputeResponse,
    WebFeatureAvailability, WebRoute, build_page_meta, build_vlacku_jvozba_output,
    dictionary_tooltip_for_rafsi, dictionary_tooltip_for_word, gentufa_web_url,
    normalize_vlacku_state, parse_web_route, reference_slot_display_text,
    toggle_cukta_target_selection, toggle_vlacku_word_type_selection,
    vlacku_brivla_filter_indeterminate, vlacku_web_url, vlacku_word_type_options, web_route_url,
};

#[cfg(target_arch = "wasm32")]
use jbotci_web_core::build_page_head;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::hash::{Hash, Hasher};
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::path::Path;
use std::rc::Rc;
use std::str::FromStr;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Mutex, OnceLock};

pub mod platform;

#[cfg(any(target_arch = "wasm32", test))]
mod f2llm_runtime_core;
#[cfg(target_arch = "wasm32")]
mod f2llm_webgpu_runtime;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const COMPUTE_WORKER_JS: Asset = asset!("/assets/compute-worker.js");
const EMBEDDING_WORKER_JS: Asset = asset!("/assets/embedding-worker.js");
// The embedding worker imports these dynamically, so keep explicit asset pins for Dioxus.
#[allow(dead_code)]
const ORT_WASM_MIN_MJS: Asset = asset!("/assets/ort/ort.wasm.min.mjs");
#[allow(dead_code)]
const ORT_WASM_SIMD_THREADED_MJS: Asset = asset!("/assets/ort/ort-wasm-simd-threaded.mjs");
#[allow(dead_code)]
const ORT_WASM_SIMD_THREADED_WASM: Asset = asset!("/assets/ort/ort-wasm-simd-threaded.wasm");
// These are referenced from generated head metadata or the web manifest rather than directly
// rendered as RSX assets, so keep explicit pins for raw `dx build` without xtask public prep.
#[allow(dead_code)]
const MANIFEST_WEBMANIFEST: Asset = asset!("/assets/manifest.webmanifest");
#[allow(dead_code)]
const FAVICON_192: Asset = asset!("/assets/icons/jbotci-icon-192.png");
#[allow(dead_code)]
const APPLE_TOUCH_ICON: Asset = asset!("/assets/icons/apple-touch-icon.png");
#[allow(dead_code)]
const ICON_512: Asset = asset!("/assets/icons/jbotci-icon-512.png");
#[allow(dead_code)]
const ICON_SVG: Asset = asset!("/assets/icons/jbotci-icon.svg");
const LOGO: Asset = asset!("/assets/icons/jbotci-dark.svg");
pub const APP_DISPLAY_NAME: &str = "jbotci";
const DEFAULT_WEB_EMBEDDINGS_BASE_URL: &str = "https://assets.jbotci.app/embeddings/web/v1";
const BUILD_WEB_EMBEDDINGS_BASE_URL: Option<&str> = option_env!("JBOTCI_WEB_EMBEDDINGS_BASE_URL");
const BUILD_GIT_COMMIT: Option<&str> = option_env!("JBOTCI_GIT_COMMIT");
const BUILD_GIT_COMMIT_SHORT: Option<&str> = option_env!("JBOTCI_GIT_COMMIT_SHORT");
const NOTO_SANS: Asset = asset!("/assets/fonts/noto-sans-variable.ttf");
const NOTO_SANS_ITALIC: Asset = asset!("/assets/fonts/noto-sans-italic-variable.ttf");
const STIX_TWO_MATH: Asset = asset!("/assets/fonts/stix-two-math-regular.ttf");
const STIX_TWO_TEXT: Asset = asset!("/assets/fonts/stix-two-text-regular.ttf");
const STIX_TWO_TEXT_BOLD: Asset = asset!("/assets/fonts/stix-two-text-bold.ttf");
const CRISA: Asset = asset!("/assets/fonts/crisa-regular.otf");
const CLL_MEDIA_CHAPTER_2_DIAGRAM: Asset = asset!("/assets/cll/media/chapter-2-diagram.svg.png");
const CLL_MEDIA_CHAPTER_ABOUT: Asset = asset!("/assets/cll/media/chapter-about.svg.png");
const CLL_MEDIA_CHAPTER_ABSTRACTIONS: Asset =
    asset!("/assets/cll/media/chapter-abstractions.svg.png");
const CLL_MEDIA_CHAPTER_ANAPHORIC_CMAVO: Asset =
    asset!("/assets/cll/media/chapter-anaphoric-cmavo.svg.png");
const CLL_MEDIA_CHAPTER_ATTITUDINALS: Asset = asset!("/assets/cll/media/chapter-attitudinals.gif");
const CLL_MEDIA_CHAPTER_CATALOGUE: Asset = asset!("/assets/cll/media/chapter-catalogue.svg.png");
const CLL_MEDIA_CHAPTER_CONNECTIVES: Asset =
    asset!("/assets/cll/media/chapter-connectives.svg.png");
const CLL_MEDIA_CHAPTER_GRAMMARS: Asset = asset!("/assets/cll/media/chapter-grammars.svg.png");
const CLL_MEDIA_CHAPTER_LETTERALS: Asset = asset!("/assets/cll/media/chapter-letterals.svg.png");
const CLL_MEDIA_CHAPTER_LUJVO: Asset = asset!("/assets/cll/media/chapter-lujvo.svg.png");
const CLL_MEDIA_CHAPTER_MEKSO: Asset = asset!("/assets/cll/media/chapter-mekso.gif");
const CLL_MEDIA_CHAPTER_MORPHOLOGY: Asset = asset!("/assets/cll/media/chapter-morphology.gif");
const CLL_MEDIA_CHAPTER_NEGATION: Asset = asset!("/assets/cll/media/chapter-negation.gif");
const CLL_MEDIA_CHAPTER_PHONOLOGY: Asset = asset!("/assets/cll/media/chapter-phonology.gif");
const CLL_MEDIA_CHAPTER_QUANTIFIERS: Asset = asset!("/assets/cll/media/chapter-quantifiers.gif");
const CLL_MEDIA_CHAPTER_RELATIVE_CLAUSES: Asset =
    asset!("/assets/cll/media/chapter-relative-clauses.svg.png");
const CLL_MEDIA_CHAPTER_SELBRI: Asset = asset!("/assets/cll/media/chapter-selbri.svg.png");
const CLL_MEDIA_CHAPTER_STRUCTURE: Asset = asset!("/assets/cll/media/chapter-structure.svg.png");
const CLL_MEDIA_CHAPTER_SUMTI: Asset = asset!("/assets/cll/media/chapter-sumti.gif");
const CLL_MEDIA_CHAPTER_SUMTI_TCITA: Asset = asset!("/assets/cll/media/chapter-sumti-tcita.gif");
const CLL_MEDIA_CHAPTER_TENSES: Asset = asset!("/assets/cll/media/chapter-tenses.gif");
const CLL_MEDIA_CHAPTER_TOUR: Asset = asset!("/assets/cll/media/chapter-tour.svg.png");
const CLL_MEDIA_LOGO: Asset = asset!("/assets/cll/media/logo.png");
const DEFAULT_GENTUFA_TEXT: &str = "cadga fa lonu ro lo prenu goi ko'a cu troci lonu ko'a tarti loka ce'u xendo je cnikansa ro lo jmive kei ta'i lo racli";
const VLACKU_SEARCH_DEBOUNCE_MS: i32 = 900;
const CUKTA_SEARCH_DEBOUNCE_MS: i32 = VLACKU_SEARCH_DEBOUNCE_MS;
const VLACKU_URL_DEBOUNCE_MS: i32 = 450;
const COMPUTE_CHANNEL_GENTUFA: &str = "gentufa-page";
const COMPUTE_CHANNEL_CUKTA: &str = "cukta-page";
const COMPUTE_CHANNEL_VLACKU: &str = "vlacku-page";
#[cfg(target_arch = "wasm32")]
const COMPUTE_CHANNEL_EMBEDDINGS: &str = "embedding-corpus";
const COMPUTE_CHANNEL_EXPORT: &str = "gentufa-export";
const EMBEDDING_CHANNEL_VLACKU_SEMANTIC: &str = "embedding-vlacku-semantic";
const EMBEDDING_CHANNEL_CUKTA_SEMANTIC: &str = "embedding-cukta-semantic";
const ASYNC_ACTIVITY_INDICATOR_DELAY_MS: i32 = 100;
const SEMANTIC_LOADING_MESSAGE_DELAY_MS: i32 = 100;
const SEMANTIC_SEARCH_SETUP_MESSAGE: &str = "Download model and embeddings to use semantic search";
const SEMANTIC_SEARCH_SETUP_LINK_LABEL: &str = "Download";
const SEMANTIC_SEARCH_SETUP_LINK_SUFFIX: &str = " model and embeddings to use semantic search";
const PAGE_FIND_INPUT_ID: &str = "app-page-find-input";
#[cfg(target_arch = "wasm32")]
const VLACKU_JVOZBA_MIN_WIDTH_PX: f64 = 981.0;
#[cfg(target_arch = "wasm32")]
const CUKTA_TOC_FORCED_AUTOHIDE_WIDTH_PX: f64 = 1100.0;
#[cfg(any(target_arch = "wasm32", feature = "desktop"))]
const VLACKU_JVOZBA_HEIGHT_SCALE: f64 = 0.5;
#[cfg(any(target_arch = "wasm32", feature = "desktop"))]
const VLACKU_JVOZBA_LAYOUT_FRAME_PASSES: u8 = 2;
#[cfg(any(target_arch = "wasm32", feature = "desktop"))]
const GENTUFA_BLOCK_REFERENCE_LAYOUT_DELAY_MS: i32 = 30;
#[cfg(any(target_arch = "wasm32", feature = "desktop"))]
const GENTUFA_BLOCK_REFERENCE_LAYOUT_FRAME_PASSES: u8 = 2;
#[cfg(any(target_arch = "wasm32", feature = "desktop"))]
const GENTUFA_TREE_LAYOUT_DELAY_MS: i32 = 30;
#[cfg(any(target_arch = "wasm32", feature = "desktop"))]
const GENTUFA_TREE_LAYOUT_FRAME_PASSES: u8 = 2;
#[allow(dead_code)]
const BLOCK_REFERENCE_LABEL_GAP_PX: f64 = 8.0;
#[allow(dead_code)]
const BLOCK_REFERENCE_CONTAINMENT_GAP_PX: f64 = 1.0;
#[allow(dead_code)]
const DICTIONARY_TOOLTIP_VIEWPORT_MARGIN_PX: f64 = 8.0;
#[allow(dead_code)]
const DICTIONARY_TOOLTIP_HOST_GAP_PX: f64 = 8.0;
const DIALECT_SETTINGS_STORAGE_KEY: &str = "jbotci.dialect-settings.v1";
const EMBEDDING_MODEL_STORAGE_KEY: &str = "jbotci.embedding-model.v1";
#[cfg(not(target_arch = "wasm32"))]
const F2LLM_NATIVE_80M_MODEL_KEY: &str = "f2llm-v2-80m-q4-k-m-320";
#[cfg(not(target_arch = "wasm32"))]
const F2LLM_NATIVE_160M_MODEL_KEY: &str = "f2llm-v2-160m-q4-k-m-640";
#[cfg(not(target_arch = "wasm32"))]
const F2LLM_NATIVE_330M_MODEL_KEY: &str = "f2llm-v2-330m-q4-k-m-896";
#[cfg(not(target_arch = "wasm32"))]
const F2LLM_NATIVE_0_6B_MODEL_KEY: &str = "f2llm-v2-0.6b-q4-k-m-1024";
const F2LLM_80M_MODEL_KEY: &str = "f2llm-v2-80m-q4-320";
#[cfg(target_arch = "wasm32")]
const F2LLM_160M_MODEL_KEY: &str = "f2llm-v2-160m-q4-640";
#[cfg(target_arch = "wasm32")]
const F2LLM_330M_MODEL_KEY: &str = "f2llm-v2-330m-q4-896";
#[cfg(target_arch = "wasm32")]
const F2LLM_0_6B_MODEL_KEY: &str = "f2llm-v2-0.6b-q4-1024";
#[cfg(target_arch = "wasm32")]
const WEB_EMBEDDING_MODEL_OPTIONS: &[EmbeddingModelOption] = &[
    EmbeddingModelOption {
        key: F2LLM_80M_MODEL_KEY,
        label: "F2LLM v2 80M",
    },
    EmbeddingModelOption {
        key: F2LLM_160M_MODEL_KEY,
        label: "F2LLM v2 160M",
    },
    EmbeddingModelOption {
        key: F2LLM_330M_MODEL_KEY,
        label: "F2LLM v2 330M",
    },
    EmbeddingModelOption {
        key: F2LLM_0_6B_MODEL_KEY,
        label: "F2LLM v2 0.6B",
    },
];
#[cfg(not(target_arch = "wasm32"))]
const NATIVE_EMBEDDING_MODEL_OPTIONS: &[EmbeddingModelOption] = &[
    EmbeddingModelOption {
        key: F2LLM_NATIVE_80M_MODEL_KEY,
        label: "F2LLM v2 80M",
    },
    EmbeddingModelOption {
        key: F2LLM_NATIVE_160M_MODEL_KEY,
        label: "F2LLM v2 160M",
    },
    EmbeddingModelOption {
        key: F2LLM_NATIVE_330M_MODEL_KEY,
        label: "F2LLM v2 330M",
    },
    EmbeddingModelOption {
        key: F2LLM_NATIVE_0_6B_MODEL_KEY,
        label: "F2LLM v2 0.6B",
    },
];

#[cfg(target_arch = "wasm32")]
thread_local! {
    static VLACKU_URL_TIMER: Cell<Option<i32>> = const { Cell::new(None) };
    static VLACKU_SEARCH_TIMER: Cell<Option<i32>> = const { Cell::new(None) };
    static CUKTA_SEARCH_TIMER: Cell<Option<i32>> = const { Cell::new(None) };
    static BROWSER_STATE_HANDLERS_INSTALLED: Cell<bool> = const { Cell::new(false) };
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
static DESKTOP_DOM_HANDLERS_INSTALLED: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum ThemeMode {
    Auto,
    Day,
    Night,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum TopbarSettingsLayout {
    BothInline,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    ThemeInline,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    NoneInline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum TopbarNavLayout {
    Full,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    Carousel,
}

#[invariant(!self.settings.shows_script_inline() || self.settings.shows_theme_inline())]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TopbarLayout {
    settings: TopbarSettingsLayout,
    nav: TopbarNavLayout,
}

#[derive(Debug, Clone, Default, PartialEq)]
#[invariant(true)]
struct ReferenceHoverState {
    hovered: Option<HoveredReference>,
    overlay: Option<ArrowOverlay>,
    measurement_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum ReferenceHoverRefreshReason {
    PointerMove,
    ViewportShift,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct HoveredReference {
    role: ReferenceMarkerRole,
    label: ReferenceLabel,
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
struct ArrowOverlay {
    width: f64,
    height: f64,
    paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[invariant(true)]
struct ReferenceRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[invariant(true)]
struct ElementSize {
    width: f64,
    height: f64,
}

#[invariant(*top >= 0.0)]
#[invariant(*width >= 0.0)]
#[invariant(*height >= 0.0)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct TooltipViewport {
    top: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[invariant(true)]
struct PositionedPoint {
    left: f64,
    top: f64,
}

#[invariant(self.line > 0)]
#[invariant(self.column > 0)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct DiagnosticSourceLocation {
    line: usize,
    column: usize,
}

#[invariant(self.errors <= usize::MAX - self.warnings)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DiagnosticCounts {
    errors: usize,
    warnings: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum DiagnosticOverlayRole {
    Primary,
    ActivePrimary,
    ActiveContextPrefix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct DiagnosticOverlayMark {
    diagnostic_index: usize,
    role: DiagnosticOverlayRole,
}

#[invariant(self.class_name.split_whitespace().next().is_some())]
#[invariant(self.diagnostic_index.is_none() || css_class_contains(&self.class_name, "has-diagnostic"))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct DiagnosticOverlayFragment {
    text: String,
    class_name: String,
    selection_start: u32,
    diagnostic_index: Option<usize>,
}

#[invariant(self.x.is_finite())]
#[invariant(self.y.is_finite())]
#[derive(Debug, Clone, Copy, PartialEq)]
struct DiagnosticInputTooltip {
    diagnostic_index: usize,
    x: f64,
    y: f64,
}

#[invariant(!self.text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct DiagnosticTextRenderPart {
    role: DiagnosticTextRole,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum AppRoute {
    Gentufa,
    Settings,
    Cukta,
    Vlacku,
}

#[invariant(true)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PageFindState {
    cukta: PageFindRouteState,
    vlacku: PageFindRouteState,
    gentufa: PageFindRouteState,
    settings: PageFindRouteState,
}

impl PageFindState {
    #[requires(true)]
    #[ensures(true)]
    fn route_state(&self, route: AppRoute) -> &PageFindRouteState {
        match route {
            AppRoute::Cukta => &self.cukta,
            AppRoute::Vlacku => &self.vlacku,
            AppRoute::Gentufa => &self.gentufa,
            AppRoute::Settings => &self.settings,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn route_state_mut(&mut self, route: AppRoute) -> &mut PageFindRouteState {
        match route {
            AppRoute::Cukta => &mut self.cukta,
            AppRoute::Vlacku => &mut self.vlacku,
            AppRoute::Gentufa => &mut self.gentufa,
            AppRoute::Settings => &mut self.settings,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn set_page_find_query(
    state: &mut PageFindState,
    route: AppRoute,
    query: String,
    update: PageFindRouteQueryUpdate,
) {
    let route_state = state.route_state_mut(route);
    match update {
        PageFindRouteQueryUpdate::Replace => {
            if route_state.query != query {
                *route_state = route_state.clone().with_data(data! {
                    query: query,
                    active_index: None,
                    result_signature: 0,
                });
            }
        }
        PageFindRouteQueryUpdate::Clear => {
            if !route_state.query.is_empty() || route_state.active_index.is_some() {
                *route_state = route_state.clone().with_data(data! {
                    query: String::new(),
                    active_index: None,
                    result_signature: 0,
                });
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn update_page_find_active(
    state: &mut PageFindState,
    route: AppRoute,
    direction: PageFindDirection,
    match_count: usize,
) {
    if match_count == 0 {
        let route_state = state.route_state_mut(route);
        reset_page_find_active(route_state);
        return;
    }
    let route_state = state.route_state_mut(route);
    let next = match (route_state.active_index, direction) {
        (Some(index), PageFindDirection::Next) => (index + 1) % match_count,
        (Some(0), PageFindDirection::Previous) => match_count - 1,
        (Some(index), PageFindDirection::Previous) => index - 1,
        (None, PageFindDirection::Next) => 0,
        (None, PageFindDirection::Previous) => match_count - 1,
    };
    *route_state = route_state.clone().with_data(data! {
        active_index: Some(next),
        scroll_request: route_state.scroll_request.wrapping_add(1),
    });
}

#[requires(true)]
#[ensures(true)]
fn sync_page_find_result_signature(
    state: &mut PageFindState,
    route: AppRoute,
    signature: u64,
    match_count: usize,
) {
    let route_state = state.route_state_mut(route);
    if route_state.result_signature != signature {
        *route_state = route_state.clone().with_data(data! {
            result_signature: signature,
            active_index: None,
        });
        return;
    }
    if route_state
        .active_index
        .is_some_and(|active_index| active_index >= match_count)
    {
        reset_page_find_active(route_state);
    }
}

#[requires(true)]
#[ensures(route_state.active_index.is_none())]
fn reset_page_find_active(route_state: &mut PageFindRouteState) {
    if route_state.active_index.is_some() {
        *route_state = route_state.clone().with_data(data! { active_index: None });
    }
}

#[invariant(self.active_index.is_none() || !self.query.is_empty())]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PageFindRouteState {
    query: String,
    active_index: Option<usize>,
    result_signature: u64,
    scroll_request: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[invariant(true)]
struct PageFindTextKey {
    ordinal: usize,
}

#[invariant(byte_start <= byte_end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PageFindTextRange {
    byte_start: usize,
    byte_end: usize,
}

#[invariant(self.range.byte_start < self.range.byte_end)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PageFindMatch {
    key: PageFindTextKey,
    range: PageFindTextRange,
    index: usize,
}

#[invariant(!self.text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PageFindTextEntry {
    text: String,
}

#[invariant(!self.query.is_empty() || self.matches.is_empty())]
#[invariant(self.matches.iter().enumerate().all(|(expected, page_match)| page_match.index == expected))]
#[invariant(self.matches_by_key.values().map(Vec::len).sum::<usize>() == self.matches.len())]
#[invariant(self.matches_by_key.values().flatten().all(|page_match| self.matches.get(page_match.index).is_some_and(|indexed| indexed == page_match)))]
#[invariant(self.matches.iter().all(|page_match| self.matches_by_key.get(&page_match.key).is_some_and(|key_matches| key_matches.iter().any(|mapped| mapped == page_match))))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PageFindIndex {
    query: String,
    matches: Vec<PageFindMatch>,
    matches_by_key: BTreeMap<PageFindTextKey, Vec<PageFindMatch>>,
    signature: u64,
}

#[invariant(self.active_index.is_none_or(|index| index < self.match_count))]
#[derive(Debug, Clone)]
struct PageFindContext {
    query: String,
    active_index: Option<usize>,
    match_count: usize,
    matches_by_key: Rc<BTreeMap<PageFindTextKey, Vec<PageFindMatch>>>,
    next_text_ordinal: Rc<Cell<usize>>,
}

#[invariant(!self.text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PageFindRenderPiece {
    text: String,
    match_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum PageFindDirection {
    Previous,
    Next,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum PageFindRouteQueryUpdate {
    Replace,
    Clear,
}

impl PageFindContext {
    #[requires(true)]
    #[ensures(ret.match_count == index.matches.len())]
    fn new(index: &PageFindIndex, route_state: &PageFindRouteState) -> Self {
        let active_index = if route_state.result_signature == index.signature {
            route_state
                .active_index
                .filter(|active_index| *active_index < index.matches.len())
        } else {
            None
        };
        new!(PageFindContext {
            query: index.query.clone(),
            active_index,
            match_count: index.matches.len(),
            matches_by_key: Rc::new(index.matches_by_key.clone()),
            next_text_ordinal: Rc::new(Cell::new(0)),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_text_key(&self) -> PageFindTextKey {
        let ordinal = self.next_text_ordinal.get();
        self.next_text_ordinal.set(ordinal.saturating_add(1));
        PageFindTextKey { ordinal }
    }

    #[requires(true)]
    #[ensures(true)]
    fn matches_for_key(&self, key: PageFindTextKey) -> &[PageFindMatch] {
        self.matches_by_key
            .get(&key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

#[requires(true)]
#[ensures(ret.query == query)]
fn build_page_find_index(query: &str, entries: &[PageFindTextEntry]) -> PageFindIndex {
    let signature = page_find_result_signature(query, entries);
    let mut matches = Vec::<PageFindMatch>::new();
    let mut matches_by_key = BTreeMap::<PageFindTextKey, Vec<PageFindMatch>>::new();
    if query.is_empty() {
        return new!(PageFindIndex {
            query: query.to_owned(),
            matches,
            matches_by_key,
            signature,
        });
    }

    for (ordinal, entry) in entries.iter().enumerate() {
        let key = PageFindTextKey { ordinal };
        for range in page_find_match_ranges(&entry.text, query) {
            let index = matches.len();
            let page_match = new!(PageFindMatch { key, range, index });
            matches_by_key
                .entry(key)
                .or_default()
                .push(page_match.clone());
            matches.push(page_match);
        }
    }

    new!(PageFindIndex {
        query: query.to_owned(),
        matches,
        matches_by_key,
        signature,
    })
}

#[requires(true)]
#[ensures(true)]
fn page_find_result_signature(query: &str, entries: &[PageFindTextEntry]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    query.hash(&mut hasher);
    entries.len().hash(&mut hasher);
    for entry in entries {
        entry.text.hash(&mut hasher);
    }
    hasher.finish()
}

#[requires(true)]
#[ensures(ret.iter().all(|range| range.byte_start < range.byte_end))]
fn page_find_match_ranges(text: &str, query: &str) -> Vec<PageFindTextRange> {
    if text.is_empty() || query.is_empty() {
        return Vec::new();
    }
    let normalized_text = normalized_page_find_text(text);
    let normalized_query = lowercase_page_find_text(query);
    if normalized_text.text.is_empty() || normalized_query.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut search_start = 0;
    while search_start <= normalized_text.text.len() {
        let Some(relative_start) = normalized_text.text[search_start..].find(&normalized_query)
        else {
            break;
        };
        let normalized_start = search_start + relative_start;
        let normalized_end = normalized_start + normalized_query.len();
        if let Some(range) = original_range_for_normalized_match(
            &normalized_text.spans,
            normalized_start,
            normalized_end,
        ) {
            ranges.push(range);
        }
        search_start = normalized_end;
    }
    ranges
}

#[invariant(self.spans.iter().all(|span| span.normalized_end <= self.text.len()))]
#[invariant(self.spans.windows(2).all(|pair| pair[0].normalized_end <= pair[1].normalized_start))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedPageFindText {
    text: String,
    spans: Vec<NormalizedPageFindCharSpan>,
}

#[invariant(normalized_start <= normalized_end)]
#[invariant(original_start <= original_end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NormalizedPageFindCharSpan {
    normalized_start: usize,
    normalized_end: usize,
    original_start: usize,
    original_end: usize,
}

#[requires(true)]
#[ensures(true)]
fn normalized_page_find_text(text: &str) -> NormalizedPageFindText {
    let mut normalized = String::new();
    let mut spans = Vec::new();
    for (original_start, character) in text.char_indices() {
        let original_end = original_start + character.len_utf8();
        for lower in character.to_lowercase() {
            let normalized_start = normalized.len();
            normalized.push(lower);
            let normalized_end = normalized.len();
            spans.push(new!(NormalizedPageFindCharSpan {
                normalized_start,
                normalized_end,
                original_start,
                original_end,
            }));
        }
    }
    new!(NormalizedPageFindText {
        text: normalized,
        spans,
    })
}

#[requires(true)]
#[ensures(true)]
fn lowercase_page_find_text(text: &str) -> String {
    let mut normalized = String::new();
    for character in text.chars() {
        normalized.extend(character.to_lowercase());
    }
    normalized
}

#[requires(normalized_start < normalized_end)]
#[ensures(ret.is_none_or(|range| range.byte_start < range.byte_end))]
fn original_range_for_normalized_match(
    spans: &[NormalizedPageFindCharSpan],
    normalized_start: usize,
    normalized_end: usize,
) -> Option<PageFindTextRange> {
    let first = spans
        .iter()
        .find(|span| span.normalized_end > normalized_start)?;
    let last = spans
        .iter()
        .rev()
        .find(|span| span.normalized_start < normalized_end)?;
    Some(new!(PageFindTextRange {
        byte_start: first.original_start,
        byte_end: last.original_end,
    }))
}

#[requires(true)]
#[ensures(true)]
fn push_page_find_entry(entries: &mut Vec<PageFindTextEntry>, text: impl Into<String>) {
    let text = text.into();
    if !text.is_empty() {
        entries.push(new!(PageFindTextEntry { text }));
    }
}

#[requires(true)]
#[ensures(true)]
fn page_find_render_pieces(text: &str, matches: &[PageFindMatch]) -> Vec<PageFindRenderPiece> {
    if text.is_empty() {
        return Vec::new();
    }
    if matches.is_empty() {
        return vec![new!(PageFindRenderPiece {
            text: text.to_owned(),
            match_index: None,
        })];
    }
    let mut pieces = Vec::new();
    let mut cursor = 0;
    for page_match in matches {
        if page_match.range.byte_start > cursor {
            pieces.push(new!(PageFindRenderPiece {
                text: text[cursor..page_match.range.byte_start].to_owned(),
                match_index: None,
            }));
        }
        pieces.push(new!(PageFindRenderPiece {
            text: text[page_match.range.byte_start..page_match.range.byte_end].to_owned(),
            match_index: Some(page_match.index),
        }));
        cursor = page_match.range.byte_end;
    }
    if cursor < text.len() {
        pieces.push(new!(PageFindRenderPiece {
            text: text[cursor..].to_owned(),
            match_index: None,
        }));
    }
    pieces.retain(|piece| !piece.text.is_empty());
    pieces
}

#[requires(true)]
#[ensures(true)]
fn render_page_find_text(page_find: &PageFindContext, text: &str) -> Element {
    if text.is_empty() {
        return rsx! {};
    }
    let key = page_find.next_text_key();
    let matches = page_find.matches_for_key(key);
    if matches.is_empty() {
        return rsx! { "{text}" };
    }
    let pieces = page_find_render_pieces(text, matches);
    rsx! {
        for piece in pieces.iter() {
            if let Some(match_index) = piece.match_index {
                {
                    let class_name = page_find_mark_class(page_find.active_index == Some(match_index));
                    rsx! {
                        mark {
                            class: "{class_name}",
                            "data-page-find-match-index": "{match_index}",
                            aria_current: if page_find.active_index == Some(match_index) { "true" } else { "false" },
                            "{piece.text}"
                        }
                    }
                }
            } else {
                "{piece.text}"
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_optional_page_find_text(page_find: Option<&PageFindContext>, text: &str) -> Element {
    if let Some(page_find) = page_find {
        render_page_find_text(page_find, text)
    } else {
        rsx! { "{text}" }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn page_find_mark_class(active: bool) -> &'static str {
    if active {
        "page-find-hit is-active"
    } else {
        "page-find-hit"
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(true)]
#[ensures(true)]
fn page_find_entries_for_route(
    route: AppRoute,
    cukta_page: &CuktaAsyncPageState,
    vlacku_committed_state: &VlackuWebState,
    vlacku_result_state: &VlackuAsyncResultState,
    gentufa_result: &GentufaWebResult,
    gentufa_request: Option<&GentufaWebRequest>,
    gentufa_view_mode: GentufaWebViewMode,
    gentufa_display: GentufaDisplayState,
    current_settings: UserSettings,
    dialect_settings: &DialectSettings,
    selected_dialect: &str,
    embedding_settings: &EmbeddingSettingsState,
    script: GentufaScript,
) -> Vec<PageFindTextEntry> {
    let mut entries = Vec::new();
    match route {
        AppRoute::Cukta => collect_cukta_page_find_entries(&mut entries, &cukta_page.page, script),
        AppRoute::Vlacku => {
            let result =
                visible_vlacku_result_for_find(vlacku_committed_state, vlacku_result_state);
            collect_vlacku_page_find_entries(&mut entries, &result, script);
        }
        AppRoute::Gentufa => collect_gentufa_page_find_entries(
            &mut entries,
            gentufa_result,
            gentufa_request,
            gentufa_view_mode,
            gentufa_display,
            script,
        ),
        AppRoute::Settings => collect_settings_page_find_entries(
            &mut entries,
            current_settings,
            dialect_settings,
            selected_dialect,
            embedding_settings,
        ),
    }
    entries
}

#[requires(true)]
#[ensures(true)]
fn visible_vlacku_result_for_find(
    committed_state: &VlackuWebState,
    result_state: &VlackuAsyncResultState,
) -> VlackuWebResult {
    if result_state.state.as_ref() == Some(committed_state) {
        result_state.result.clone()
    } else {
        vlacku_loading_result(committed_state, "Loading dictionary results.")
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_cukta_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    page: &CuktaPageData,
    script: GentufaScript,
) {
    match &page.page_kind {
        CuktaPageKind::Section {
            section_heading,
            chapter_prelude_blocks,
            blocks,
            previous_section,
            next_section,
            ..
        } => {
            push_page_find_entry(entries, section_heading.clone());
            for block in chapter_prelude_blocks {
                collect_cll_block_page_find_entries(entries, block, script);
            }
            for block in blocks {
                collect_cll_block_page_find_entries(entries, block, script);
            }
            if let Some(previous) = previous_section {
                push_page_find_entry(entries, previous.label.clone());
            }
            if let Some(next) = next_section {
                push_page_find_entry(entries, next.label.clone());
            }
        }
        CuktaPageKind::Index {
            entries: index_entries,
        } => {
            push_page_find_entry(entries, "Index");
            for entry in index_entries {
                push_page_find_entry(entries, entry.key.clone());
                for reference in &entry.references {
                    push_page_find_entry(entries, reference.label.clone());
                }
            }
        }
        CuktaPageKind::Search {
            results,
            message,
            has_more,
            ..
        } => {
            if let Some(message) = message {
                push_page_find_entry(entries, semantic_search_message_visible_text(message));
            }
            for card in results {
                collect_cukta_search_card_page_find_entries(entries, card);
            }
            if *has_more {
                push_page_find_entry(entries, "Load more");
            }
        }
        CuktaPageKind::Error { message } => push_page_find_entry(entries, message.clone()),
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_cukta_search_card_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    card: &CuktaSearchResultCard,
) {
    push_page_find_entry(entries, format!("{} · {}", card.kind, card.section_label));
    push_page_find_entry(entries, format!("{}. {}", card.rank, card.label));
    if let Some(similarity) = &card.similarity_label {
        push_page_find_entry(entries, similarity.clone());
    }
    push_page_find_entry(entries, card.preview.clone());
}

#[requires(true)]
#[ensures(true)]
fn collect_cll_block_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    block: &CllBlock,
    script: GentufaScript,
) {
    match block {
        CllBlock::Paragraph { inlines, text, .. } => {
            if inlines.is_empty() {
                push_page_find_entry(entries, text.clone());
            } else {
                collect_cll_inlines_page_find_entries(entries, inlines, script, false);
            }
        }
        CllBlock::List { items, .. } => {
            for item in items {
                for child in item {
                    collect_cll_block_page_find_entries(entries, child, script);
                }
            }
        }
        CllBlock::Example(example) => {
            push_page_find_entry(entries, example.label.clone());
            if example.blocks.is_empty() {
                for line in &example.lines {
                    push_page_find_entry(
                        entries,
                        cll_display_text_for_kind(script, &line.kind, &line.text),
                    );
                }
            } else {
                for child in &example.blocks {
                    collect_cll_block_page_find_entries(entries, child, script);
                }
            }
        }
        CllBlock::Table {
            caption,
            header_rows,
            body_rows,
            ..
        } => {
            if let Some(caption) = caption {
                collect_cll_inlines_page_find_entries(entries, caption, script, false);
            }
            for row in header_rows.iter().chain(body_rows.iter()) {
                for cell in row {
                    collect_cll_table_cell_page_find_entries(entries, cell, script);
                }
            }
        }
        CllBlock::SimpleListTable { rows, .. } => {
            for row in rows {
                for cell in row {
                    if let Some(inlines) = cell {
                        collect_cll_inlines_page_find_entries(entries, inlines, script, false);
                    }
                }
            }
        }
        CllBlock::VariableList { entries: items, .. } => {
            for entry in items {
                collect_cll_inlines_page_find_entries(entries, &entry.term, script, false);
                for child in &entry.blocks {
                    collect_cll_block_page_find_entries(entries, child, script);
                }
            }
        }
        CllBlock::Media { title, .. } => {
            if let Some(title) = title {
                collect_cll_inlines_page_find_entries(entries, title, script, false);
            }
        }
        CllBlock::Rule { term, body, .. } => {
            push_page_find_entry(entries, term.clone());
            for child in body {
                collect_cll_block_page_find_entries(entries, child, script);
            }
        }
        CllBlock::Code { text, .. } => push_page_find_entry(entries, text.clone()),
        CllBlock::DisplayMath { .. } => {}
        CllBlock::Heading { inlines, .. } => {
            collect_cll_inlines_page_find_entries(entries, inlines, script, false);
        }
        CllBlock::BlockQuote { blocks, .. } => {
            for child in blocks {
                collect_cll_block_page_find_entries(entries, child, script);
            }
        }
        CllBlock::Definition { body, .. } | CllBlock::GrammarTemplate { body, .. } => {
            collect_cll_inlines_page_find_entries(entries, body, script, false);
        }
        CllBlock::InterlinearGloss {
            rows,
            natlang,
            comments,
            ..
        } => {
            for row in rows {
                let row_context = cll_kind_is_lojban(&row.kind);
                for cell in &row.cells {
                    collect_cll_inlines_page_find_entries(entries, cell, script, row_context);
                }
            }
            for comment in comments {
                collect_cll_inlines_page_find_entries(entries, comment, script, false);
            }
            for line in natlang {
                collect_cll_inlines_page_find_entries(entries, line, script, false);
            }
        }
        CllBlock::CmavoList {
            titles,
            headers,
            rows,
            ..
        } => {
            for title in titles {
                collect_cll_inlines_page_find_entries(entries, title, script, false);
            }
            for header in headers {
                collect_cll_inlines_page_find_entries(entries, header, script, false);
            }
            for row in rows {
                for cell in row {
                    collect_cll_inlines_page_find_entries(entries, cell, script, false);
                }
            }
        }
        CllBlock::Lojbanization { lines, .. } => {
            for line in lines {
                push_page_find_entry(entries, line.kind.clone());
                let line_context = cll_kind_is_lojban(&line.kind);
                collect_cll_inlines_page_find_entries(entries, &line.body, script, line_context);
                if let Some(comment) = &line.comment {
                    collect_cll_inlines_page_find_entries(entries, comment, script, false);
                }
            }
        }
        CllBlock::LujvoMaking { parts, .. } => {
            for part in parts {
                push_page_find_entry(entries, part.kind.clone());
                let part_context = cll_kind_is_lojban(&part.kind);
                collect_cll_inlines_page_find_entries(entries, &part.body, script, part_context);
            }
        }
        CllBlock::Ebnf { entries: rules, .. } => {
            for rule in rules {
                push_page_find_entry(entries, rule.rule_name.clone());
                collect_cll_ebnf_tokens_page_find_entries(entries, &rule.rhs);
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_cll_table_cell_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    cell: &CllTableCell,
    script: GentufaScript,
) {
    for child in &cell.blocks {
        collect_cll_block_page_find_entries(entries, child, script);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_cll_inlines_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    inlines: &[CllInline],
    script: GentufaScript,
    lojban_context: bool,
) {
    for inline in inlines {
        collect_cll_inline_page_find_entries(entries, inline, script, lojban_context);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_cll_inline_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    inline: &CllInline,
    script: GentufaScript,
    lojban_context: bool,
) {
    match inline {
        CllInline::Text(text) => push_page_find_entry(
            entries,
            display_lojban_text_if(script, text, lojban_context),
        ),
        CllInline::Emphasis { language, inlines } | CllInline::Quote { language, inlines } => {
            let child_context = lojban_context || cll_language_is_lojban(language.as_deref());
            collect_cll_inlines_page_find_entries(entries, inlines, script, child_context);
        }
        CllInline::LanguageSpan {
            kind,
            language,
            inlines,
        } => {
            let child_context = lojban_context
                || *kind == CllLanguageSpanKind::JboPhrase
                || cll_language_is_lojban(language.as_deref());
            collect_cll_inlines_page_find_entries(entries, inlines, script, child_context);
        }
        CllInline::CiteTitle { inlines }
        | CllInline::Subscript { inlines }
        | CllInline::Superscript { inlines }
        | CllInline::Link { inlines, .. } => {
            collect_cll_inlines_page_find_entries(entries, inlines, script, lojban_context);
        }
        CllInline::Code(text) => push_page_find_entry(entries, text.clone()),
        CllInline::Elidable { shown, inlines, .. } => {
            if inlines.is_empty() {
                push_page_find_entry(
                    entries,
                    display_lojban_text_if(script, shown, lojban_context),
                );
            } else {
                collect_cll_inlines_page_find_entries(entries, inlines, script, lojban_context);
            }
        }
        CllInline::InlineMath { .. } | CllInline::Anchor { .. } => {}
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_cll_ebnf_tokens_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    tokens: &[CllEbnfToken],
) {
    let lines = wrap_ebnf_choice_lines(tokens);
    for line in lines {
        for token in line {
            match token {
                CllEbnfToken::Text { body }
                | CllEbnfToken::Operator { body }
                | CllEbnfToken::Hash { body }
                | CllEbnfToken::Terminal { body, .. }
                | CllEbnfToken::ElidableTerminator { body, .. }
                | CllEbnfToken::Nonterminal { body, .. } => {
                    if let Some((prefix, suffix)) = cll_ebnf_elidable_hash_pieces(&body) {
                        push_page_find_entry(entries, prefix.to_owned());
                        push_page_find_entry(entries, "#");
                        push_page_find_entry(entries, suffix.to_owned());
                    } else {
                        push_page_find_entry(entries, body.clone());
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_vlacku_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    result: &VlackuWebResult,
    script: GentufaScript,
) {
    if let Some(message) = &result.message {
        push_page_find_entry(entries, semantic_search_message_visible_text(message));
    }
    for error in &result.errors {
        push_page_find_entry(entries, error.clone());
    }
    for card in &result.cards {
        collect_vlacku_card_page_find_entries(entries, card, script);
    }
    if result.has_more {
        push_page_find_entry(entries, "Load more");
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_vlacku_card_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    card: &VlackuWebCard,
    script: GentufaScript,
) {
    push_page_find_entry(entries, display_lojban_text(script, &card.display_word));
    if let Some(ipa) = &card.ipa {
        push_page_find_entry(entries, format!("/{ipa}/"));
    }
    for piece in card
        .decomposition
        .iter()
        .filter(|piece| piece.kind != VlackuCompositionPieceKind::Hyphen)
    {
        collect_vlacku_composition_piece_page_find_entries(entries, piece, script);
    }
    if card.decomposition.is_empty() {
        for rafsi in &card.rafsi {
            push_page_find_entry(entries, display_lojban_text(script, rafsi));
        }
    }
    if let Some(author) = &card.author {
        push_page_find_entry(entries, vlacku_author_credit_text(author));
    }
    push_page_find_entry(entries, card.word_type.clone());
    if let Some(selmaho) = &card.selmaho {
        push_page_find_entry(entries, selmaho.clone());
    }
    if let Some(similarity) = card.similarity {
        push_page_find_entry(entries, format_similarity(similarity));
    }
    collect_vote_display_page_find_entries(entries, &card.votes);
    collect_vlacku_inlines_page_find_entries(entries, &card.definition, script);
    for gloss in &card.glosses {
        push_page_find_entry(entries, gloss.clone());
    }
    collect_vlacku_inlines_page_find_entries(entries, &card.notes, script);
    if !card.etymology.is_empty() {
        push_page_find_entry(entries, "etymology: ");
        collect_vlacku_inlines_page_find_entries(entries, &card.etymology, script);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_vlacku_composition_piece_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    piece: &VlackuCompositionPiece,
    script: GentufaScript,
) {
    if piece.kind != VlackuCompositionPieceKind::Rafsi {
        return;
    }
    push_page_find_entry(entries, display_lojban_text(script, &piece.display_surface));
    if let Some(source) = &piece.source
        && !piece.source_is_surface
    {
        let source_label = piece.display_source.as_deref().unwrap_or(source);
        push_page_find_entry(entries, display_lojban_text(script, source_label));
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_vote_display_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    votes: &VlackuVoteDisplay,
) {
    match votes {
        VlackuVoteDisplay::Known(value) => push_page_find_entry(entries, value.to_string()),
        VlackuVoteDisplay::Unknown => push_page_find_entry(entries, "?"),
        VlackuVoteDisplay::Hidden => {}
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_vlacku_inlines_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    spans: &[VlackuInline],
    script: GentufaScript,
) {
    for span in spans {
        match span.as_data() {
            data!(VlackuInline::Text(text)) => push_page_find_entry(entries, text.clone()),
            data!(VlackuInline::Math(_math)) => {}
            data!(VlackuInline::WordRef { label, .. }) => {
                push_page_find_entry(entries, display_lojban_text(script, label));
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_gentufa_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    result: &GentufaWebResult,
    request: Option<&GentufaWebRequest>,
    view_mode: GentufaWebViewMode,
    display: GentufaDisplayState,
    script: GentufaScript,
) {
    match result {
        GentufaWebResult::Blank => {}
        GentufaWebResult::Error(error) => {
            collect_diagnostics_pane_page_find_entries(
                entries,
                &error.diagnostics,
                gentufa_request_source(request),
                Some(error.message.as_str()),
                true,
                script,
            );
        }
        GentufaWebResult::Success(success) => {
            collect_bracket_fragments_page_find_entries(entries, &success.bracket_fragments);
            collect_diagnostics_pane_page_find_entries(
                entries,
                &success.diagnostics,
                gentufa_request_source(request),
                None,
                true,
                script,
            );
            match view_mode {
                GentufaWebViewMode::Blocks => {
                    for block in &success.blocks_layout.blocks {
                        push_page_find_entry(entries, block.label.clone());
                    }
                    if display.show_glosses {
                        for block in success
                            .blocks_layout
                            .blocks
                            .iter()
                            .filter(|block| block.is_leaf)
                        {
                            let text = block
                                .computed_gloss
                                .as_deref()
                                .or_else(|| block.glosses.first().map(String::as_str))
                                .unwrap_or("");
                            push_page_find_entry(entries, text.to_owned());
                        }
                    }
                }
                GentufaWebViewMode::Tree => {
                    for row in &success.tree_rows {
                        push_page_find_entry(entries, row.label.clone());
                        for cell in &row.cells {
                            push_page_find_entry(entries, cell.text.clone());
                        }
                    }
                }
                GentufaWebViewMode::Ipa => push_page_find_entry(entries, success.ipa_text.clone()),
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_bracket_fragments_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    fragments: &[GentufaBracketFragment],
) {
    for fragment in fragments {
        match fragment {
            GentufaBracketFragment::Text { text, .. } => {
                push_page_find_entry(entries, text.clone())
            }
            GentufaBracketFragment::Span { children, .. } => {
                collect_bracket_fragments_page_find_entries(entries, children);
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_diagnostics_pane_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    diagnostics: &[Diagnostic],
    source: &str,
    fallback_error: Option<&str>,
    diagnostics_open: bool,
    script: GentufaScript,
) {
    let fallback_error = fallback_error.filter(|message| !message.is_empty());
    if diagnostics.is_empty() && fallback_error.is_none() {
        return;
    }
    push_page_find_entry(
        entries,
        diagnostic_pane_title(diagnostic_counts(diagnostics, fallback_error)),
    );
    push_page_find_entry(entries, diagnostics_toggle_label(diagnostics_open));
    if !diagnostics_open {
        return;
    }
    if diagnostics.is_empty() {
        if let Some(message) = fallback_error {
            push_page_find_entry(entries, "error");
            push_page_find_entry(entries, message.to_owned());
        }
        return;
    }
    for diagnostic in diagnostics {
        collect_diagnostic_card_page_find_entries(entries, diagnostic, source, script);
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_diagnostic_card_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    diagnostic: &Diagnostic,
    source: &str,
    script: GentufaScript,
) {
    push_page_find_entry(entries, diagnostic_severity_text(diagnostic.severity));
    push_page_find_entry(entries, diagnostic.code.clone());
    let location = diagnostic_label_location(source, diagnostic.primary_label());
    push_page_find_entry(
        entries,
        format!(
            "{}:{}: {}",
            location.line, location.column, diagnostic.message
        ),
    );
    for label in diagnostic_context_labels(diagnostic) {
        if let Some(descriptor) = diagnostic_context_descriptor(&label.message) {
            push_page_find_entry(entries, "while parsing ");
            push_page_find_entry(entries, descriptor);
        } else {
            push_page_find_entry(entries, label.message.clone());
        }
    }
    for segment in diagnostic_primary_detail_parts(diagnostic) {
        push_page_find_entry(
            entries,
            diagnostic_display_text_part_for_script(&segment, script),
        );
    }
    for note in diagnostic_plain_notes_for_web(diagnostic) {
        for segment in diagnostic_plain_text_render_parts(note) {
            push_page_find_entry(
                entries,
                diagnostic_display_text_part_for_script(&segment, script),
            );
        }
    }
    for note in diagnostic_styled_notes_for_web(diagnostic) {
        for segment in &note.segments {
            for part in diagnostic_text_segment_render_parts(segment) {
                push_page_find_entry(
                    entries,
                    diagnostic_display_text_part_for_script(&part, script),
                );
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_settings_page_find_entries(
    entries: &mut Vec<PageFindTextEntry>,
    _current_settings: UserSettings,
    dialect_settings: &DialectSettings,
    selected_dialect: &str,
    embedding_settings: &EmbeddingSettingsState,
) {
    push_page_find_entry(entries, "Settings");
    if let Some(commit) = build_commit_info() {
        push_page_find_entry(entries, format!("commit {}", commit.short));
    }
    push_page_find_entry(entries, "Semantic search");
    push_page_find_entry(entries, "Embedding model");
    push_page_find_entry(entries, "Status");
    push_page_find_entry(entries, embedding_settings.status.clone());
    push_page_find_entry(entries, "Model");
    push_page_find_entry(entries, embedding_settings.model_size.clone());
    push_page_find_entry(entries, "Index");
    push_page_find_entry(entries, embedding_settings.index_size.clone());
    push_page_find_entry(entries, embedding_settings.detail.clone());
    if embedding_settings.busy || embedding_settings.progress_percent.is_some() {
        push_page_find_entry(
            entries,
            embedding_progress_display_label(embedding_settings),
        );
    }
    push_page_find_entry(entries, "Download");
    push_page_find_entry(entries, "Update");
    push_page_find_entry(entries, "Remove");
    if embedding_settings.remove_confirmation_open {
        push_page_find_entry(
            entries,
            format!("Remove {}", embedding_settings.selected_model_label),
        );
        push_page_find_entry(
            entries,
            "This will remove the selected model files and vector index from this device.",
        );
        push_page_find_entry(entries, "Cancel");
        push_page_find_entry(entries, "Remove");
    }
    push_page_find_entry(entries, "Output");
    push_page_find_entry(entries, "Stress");
    push_page_find_entry(entries, "none");
    push_page_find_entry(entries, "acute");
    push_page_find_entry(entries, "caps");
    push_page_find_entry(entries, "Glides");
    push_page_find_entry(entries, "none");
    push_page_find_entry(entries, "breve");
    push_page_find_entry(entries, "Lojban dialects");
    push_page_find_entry(entries, "Builtins");
    for name in builtin_dialect_names() {
        push_page_find_entry(entries, name);
    }
    push_page_find_entry(entries, "Custom");
    for custom in &dialect_settings.custom_dialects {
        let item_name = custom.name.trim();
        push_page_find_entry(
            entries,
            if item_name.is_empty() {
                "(unnamed)"
            } else {
                item_name
            },
        );
    }
    if selected_dialect.trim().is_empty() {
        push_page_find_entry(entries, "Select a dialect to edit it.");
    } else {
        push_page_find_entry(entries, "Name");
        push_page_find_entry(entries, "Show in gentufa");
        push_page_find_entry(entries, "Definition");
        if let Some(custom) = dialect_settings
            .custom_dialects
            .iter()
            .find(|custom| custom.name.trim() == selected_dialect.trim())
            && let Err(error) = custom_dialect_is_valid(&dialect_settings.custom_dialects, custom)
        {
            push_page_find_entry(entries, error.message().to_owned());
        } else {
            push_page_find_entry(entries, "Definition is valid.");
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn semantic_search_message_visible_text(message: &str) -> String {
    if message == SEMANTIC_SEARCH_SETUP_MESSAGE {
        format!("{SEMANTIC_SEARCH_SETUP_LINK_LABEL}{SEMANTIC_SEARCH_SETUP_LINK_SUFFIX}")
    } else {
        message.to_owned()
    }
}

#[invariant(!self.gentufa_text_explicit || matches!(&self.web_route, WebRoute::Gentufa(_)))]
#[invariant(self.settings_query.is_empty() || matches!(&self.web_route, WebRoute::Settings))]
#[invariant(self.hash.as_ref().is_none_or(|hash| !hash.is_empty() && !hash.starts_with('#')))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct JbotciRoute {
    web_route: WebRoute,
    gentufa_text_explicit: bool,
    settings_query: String,
    hash: Option<String>,
}

impl JbotciRoute {
    #[requires(true)]
    #[ensures(matches!(ret.web_route, WebRoute::Gentufa(_)))]
    fn default_gentufa() -> Self {
        new!(JbotciRoute {
            web_route: WebRoute::Gentufa(GentufaWebState::default()),
            gentufa_text_explicit: false,
            settings_query: String::new(),
            hash: None,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn from_web_route(web_route: WebRoute, gentufa_text_explicit: bool) -> Self {
        new!(JbotciRoute {
            web_route,
            gentufa_text_explicit,
            settings_query: String::new(),
            hash: None,
        })
    }

    #[requires(true)]
    #[ensures(ret == app_route_for_web_route(&self.web_route))]
    fn app_route(&self) -> AppRoute {
        app_route_for_web_route(&self.web_route)
    }

    #[requires(true)]
    #[ensures(ret.web_route == self.web_route)]
    fn without_hash(&self) -> Self {
        new!(JbotciRoute {
            web_route: self.web_route.clone(),
            gentufa_text_explicit: self.gentufa_text_explicit,
            settings_query: self.settings_query.clone(),
            hash: None,
        })
    }
}

impl Default for JbotciRoute {
    #[requires(true)]
    #[ensures(matches!(ret.web_route, WebRoute::Gentufa(_)))]
    fn default() -> Self {
        Self::default_gentufa()
    }
}

impl fmt::Display for JbotciRoute {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut route = match &self.web_route {
            WebRoute::Settings if !self.settings_query.is_empty() => {
                format!("/settings?{}", self.settings_query)
            }
            _ => web_route_url("", &self.web_route),
        };
        if let Some(hash) = self.hash.as_ref().filter(|hash| !hash.is_empty()) {
            route.push('#');
            route.push_str(hash.trim_start_matches('#'));
        }
        f.write_str(&route)
    }
}

impl FromStr for JbotciRoute {
    type Err = JbotciRouteParseError;

    #[requires(true)]
    #[ensures(true)]
    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        jbotci_route_from_dioxus_route(raw).ok_or_else(JbotciRouteParseError::new)
    }
}

impl Routable for JbotciRoute {
    const SITE_MAP: &'static [dioxus::router::SiteMapSegment] = &[];

    #[requires(true)]
    #[ensures(true)]
    fn render(&self, level: usize) -> Element {
        if level == 0 {
            rsx! { AppShell {} }
        } else {
            rsx! {}
        }
    }
}

#[invariant(std::mem::size_of_val(self) == 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct JbotciRouteParseError {
    marker: (),
}

impl JbotciRouteParseError {
    #[requires(true)]
    #[ensures(true)]
    fn new() -> Self {
        new!(JbotciRouteParseError { marker: () })
    }
}

impl fmt::Display for JbotciRouteParseError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid jbotci route")
    }
}

impl Error for JbotciRouteParseError {}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[invariant(true)]
struct PendingLocalRouteWrites {
    routes: Vec<JbotciRoute>,
}

impl PendingLocalRouteWrites {
    #[requires(true)]
    #[ensures(self.routes.iter().any(|pending| pending == &canonical_local_route(route)))]
    fn record(&mut self, route: &JbotciRoute) {
        self.routes.push(canonical_local_route(route));
    }

    #[requires(true)]
    #[ensures(ret -> !self.routes.iter().any(|pending| pending == &canonical_local_route(route)))]
    fn consume(&mut self, route: &JbotciRoute) -> bool {
        let route = canonical_local_route(route);
        let initial_len = self.routes.len();
        self.routes.retain(|pending| pending != &route);
        self.routes.len() != initial_len
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct RouteLocationSyncAction {
    app_route: AppRoute,
    hydrate_route_bound_state: bool,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GentufaUrlWriteIntent {
    ReplaceCurrent,
    PushParse,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GentufaUrlHistoryAction {
    NoWrite,
    ReplaceCurrent,
    PushParse,
}

#[invariant(*text_explicit || state.text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct GentufaUrlInputs {
    active_route: AppRoute,
    current_route: JbotciRoute,
    state: GentufaWebState,
    text_explicit: bool,
    intent: GentufaUrlWriteIntent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct UserSettings {
    theme: ThemeMode,
    script: GentufaScript,
    stress: StressMark,
    glides: GlideMark,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct GentufaComputeInputs {
    route: AppRoute,
    settings: UserSettings,
    dialect_settings: DialectSettings,
    display: GentufaDisplayState,
    view_mode: GentufaWebViewMode,
    text: String,
    dialect_text: String,
    text_explicit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct GentufaLayoutInputs {
    route: AppRoute,
    parsed_text_len: usize,
    parsed_dialect_len: usize,
    display: GentufaDisplayState,
    view_mode: GentufaWebViewMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct EmbeddingModelOption {
    key: &'static str,
    label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct EmbeddingSettingsState {
    selected_model_key: String,
    selected_model_label: String,
    effective_model_key: String,
    webgpu_available: Option<bool>,
    status: String,
    detail: String,
    model_size: String,
    index_size: String,
    progress_kind: Option<String>,
    progress_label: Option<String>,
    progress_loaded: Option<u64>,
    progress_total: Option<u64>,
    progress_percent: Option<u8>,
    busy: bool,
    remove_confirmation_open: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct DialectHighlightToken {
    class_name: String,
    text: String,
}

type AsyncTaskId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(::Gentufa => true)]
#[invariant(::Cukta => true)]
#[invariant(::Vlacku => true)]
#[invariant(::Settings => true)]
#[invariant(::Export => true)]
enum AsyncTaskKind {
    Gentufa,
    Cukta,
    Vlacku,
    Settings,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct AsyncActivityTask {
    id: AsyncTaskId,
    kind: AsyncTaskKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct AsyncActivityState {
    next_task_id: AsyncTaskId,
    active_tasks: Vec<AsyncActivityTask>,
}

impl Default for AsyncActivityState {
    #[requires(true)]
    #[ensures(ret.next_task_id == 1)]
    #[ensures(ret.active_tasks.is_empty())]
    fn default() -> Self {
        Self {
            next_task_id: 1,
            active_tasks: Vec::new(),
        }
    }
}

impl AsyncActivityState {
    #[requires(self.next_task_id > 0)]
    #[ensures(ret > 0)]
    fn begin(&mut self, kind: AsyncTaskKind) -> AsyncTaskId {
        let id = self.next_task_id;
        self.next_task_id = self.next_task_id.saturating_add(1).max(1);
        self.active_tasks.push(AsyncActivityTask { id, kind });
        id
    }

    #[requires(task_id > 0)]
    #[ensures(true)]
    fn finish(&mut self, task_id: AsyncTaskId) -> bool {
        let Some(index) = self.active_tasks.iter().position(|task| task.id == task_id) else {
            return false;
        };
        self.active_tasks.remove(index);
        true
    }

    #[requires(true)]
    #[ensures(ret == !self.active_tasks.is_empty())]
    fn is_active(&self) -> bool {
        !self.active_tasks.is_empty()
    }

    #[requires(true)]
    #[ensures(true)]
    fn has_kind(&self, kind: AsyncTaskKind) -> bool {
        self.active_tasks.iter().any(|task| task.kind == kind)
    }
}

#[derive(Debug)]
#[invariant(true)]
struct AsyncActivityGuard {
    activity: Signal<AsyncActivityState>,
    task_id: AsyncTaskId,
    finished: bool,
}

impl AsyncActivityGuard {
    #[requires(true)]
    #[ensures(ret.task_id > 0)]
    fn new(mut activity: Signal<AsyncActivityState>, kind: AsyncTaskKind) -> Self {
        let task_id = activity.with_mut(|state| state.begin(kind));
        Self {
            activity,
            task_id,
            finished: false,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(&mut self) {
        if self.finished {
            return;
        }
        let task_id = self.task_id;
        self.activity.with_mut(|state| {
            state.finish(task_id);
        });
        self.finished = true;
    }
}

impl Drop for AsyncActivityGuard {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        self.finish();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct LatestAsyncTask {
    task: Task,
    task_id: AsyncTaskId,
}

#[requires(true)]
#[ensures(true)]
fn spawn_tracked(
    activity: Signal<AsyncActivityState>,
    kind: AsyncTaskKind,
    future: impl Future<Output = ()> + 'static,
) -> Task {
    let guard = AsyncActivityGuard::new(activity, kind);
    spawn(async move {
        let _guard = guard;
        future.await;
    })
}

#[requires(true)]
#[ensures(true)]
fn cancel_latest_task(mut slot: Signal<Option<LatestAsyncTask>>) {
    if let Some(latest) = slot.write().take() {
        latest.task.cancel();
    }
}

#[requires(task_id > 0)]
#[ensures(true)]
fn clear_latest_task_if_current(mut slot: Signal<Option<LatestAsyncTask>>, task_id: AsyncTaskId) {
    slot.with_mut(|current| {
        if current
            .as_ref()
            .is_some_and(|latest| latest.task_id == task_id)
        {
            *current = None;
        }
    });
}

#[requires(true)]
#[ensures(true)]
fn spawn_latest_tracked(
    mut slot: Signal<Option<LatestAsyncTask>>,
    activity: Signal<AsyncActivityState>,
    kind: AsyncTaskKind,
    future: impl Future<Output = ()> + 'static,
) -> Task {
    cancel_latest_task(slot);
    let guard = AsyncActivityGuard::new(activity, kind);
    let task_id = guard.task_id;
    let slot_for_task = slot;
    let task = spawn(async move {
        let _guard = guard;
        future.await;
        clear_latest_task_if_current(slot_for_task, task_id);
    });
    slot.set(Some(LatestAsyncTask { task, task_id }));
    task
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
struct VlackuSemanticResultState {
    state: Option<VlackuWebState>,
    hits: Vec<VlackuSemanticSearchHit>,
    message: Option<String>,
    loading: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
struct CuktaSemanticResultState {
    state: Option<CuktaWebSearchState>,
    hits: Vec<CuktaSemanticSearchHit>,
    message: Option<String>,
    loading: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct GentufaAsyncPageState {
    state: Option<GentufaWebState>,
    request: Option<GentufaWebRequest>,
    result: GentufaWebResult,
    meta: Option<PageMeta>,
    loading: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct CuktaAsyncPageState {
    state: Option<CuktaWebState>,
    page: CuktaPageData,
    meta: Option<PageMeta>,
    loading: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct CuktaPendingScroll {
    mode: CuktaPendingScrollMode,
    target: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum CuktaPendingScrollMode {
    Anchor,
    Stored,
    Top,
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
struct VlackuAsyncResultState {
    state: Option<VlackuWebState>,
    result: VlackuWebResult,
    meta: Option<PageMeta>,
    loading: bool,
    error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct GentufaDisplayState {
    show_elided: bool,
    show_glosses: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct VlackuJvozbaPaneState {
    open: bool,
    mode: VlackuJvozbaMode,
    items: Vec<VlackuJvozbaItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct VlackuJvozbaDragState {
    start_index: usize,
    target_index: usize,
    item_height: usize,
    preview_visible: bool,
}

#[invariant(self.start_x.is_finite())]
#[invariant(self.start_width >= cukta_toc_width_min() && self.start_width <= cukta_toc_width_max())]
#[derive(Debug, Clone, PartialEq)]
struct CuktaTocResizeState {
    start_x: f64,
    start_width: f64,
}

#[invariant(self.expanded.iter().all(|node_id| !node_id.is_empty()))]
#[invariant(self.collapsed.iter().all(|node_id| !node_id.is_empty()))]
#[invariant(
    self.expanded
        .iter()
        .all(|expanded| !self.collapsed.iter().any(|collapsed| collapsed == expanded))
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CuktaTocExpansionState {
    expanded: Vec<String>,
    collapsed: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct CuktaTocInteractionState {
    pinned: bool,
    overlay_visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum CuktaTocButtonState {
    Hidden,
    ForcedAutoHideVisible,
    PinnedVisible,
    UnpinnedVisible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum CuktaTocButtonAction {
    ShowOverlay,
    HideOverlay,
    Pin,
    Unpin,
}

impl Default for UserSettings {
    #[requires(true)]
    #[ensures(ret.theme == ThemeMode::Auto)]
    fn default() -> Self {
        Self {
            theme: ThemeMode::Auto,
            script: GentufaScript::Latin,
            stress: StressMark::Acute,
            glides: GlideMark::Breve,
        }
    }
}

impl Default for GentufaAsyncPageState {
    #[requires(true)]
    #[ensures(matches!(ret.result, GentufaWebResult::Blank))]
    fn default() -> Self {
        Self {
            state: None,
            request: None,
            result: GentufaWebResult::Blank,
            meta: None,
            loading: false,
            error: None,
        }
    }
}

impl Default for CuktaAsyncPageState {
    #[requires(true)]
    #[ensures(ret.state.is_none())]
    fn default() -> Self {
        Self {
            state: None,
            page: cukta_loading_page_data("Loading CLL page."),
            meta: None,
            loading: false,
            error: None,
        }
    }
}

impl Default for VlackuAsyncResultState {
    #[requires(true)]
    #[ensures(ret.state.is_none())]
    fn default() -> Self {
        let state = VlackuWebState::default();
        Self {
            state: None,
            result: vlacku_loading_result(&state, "Loading dictionary results."),
            meta: None,
            loading: false,
            error: None,
        }
    }
}

impl Default for EmbeddingSettingsState {
    #[requires(true)]
    #[ensures(!ret.busy)]
    fn default() -> Self {
        let selected_model_key = load_embedding_model_key();
        let selected_model_label = embedding_model_label(&selected_model_key).to_owned();
        Self {
            effective_model_key: selected_model_key.clone(),
            selected_model_key,
            selected_model_label,
            webgpu_available: None,
            status: "unknown".to_owned(),
            detail: "Checking embedding storage.".to_owned(),
            model_size: "unknown".to_owned(),
            index_size: "unknown".to_owned(),
            progress_kind: None,
            progress_label: None,
            progress_loaded: None,
            progress_total: None,
            progress_percent: None,
            busy: false,
            remove_confirmation_open: false,
        }
    }
}

impl Default for VlackuSemanticResultState {
    #[requires(true)]
    #[ensures(!ret.loading)]
    fn default() -> Self {
        Self {
            state: None,
            hits: Vec::new(),
            message: None,
            loading: false,
        }
    }
}

impl Default for CuktaSemanticResultState {
    #[requires(true)]
    #[ensures(!ret.loading)]
    fn default() -> Self {
        Self {
            state: None,
            hits: Vec::new(),
            message: None,
            loading: false,
        }
    }
}

impl Default for GentufaDisplayState {
    #[requires(true)]
    #[ensures(!ret.show_elided)]
    #[ensures(!ret.show_glosses)]
    fn default() -> Self {
        Self {
            show_elided: false,
            show_glosses: false,
        }
    }
}

impl Default for VlackuJvozbaPaneState {
    #[requires(true)]
    #[ensures(!ret.open)]
    fn default() -> Self {
        Self {
            open: false,
            mode: VlackuJvozbaMode::Lujvo,
            items: Vec::new(),
        }
    }
}

impl Default for CuktaTocExpansionState {
    #[requires(true)]
    #[ensures(ret.expanded.is_empty())]
    #[ensures(ret.collapsed.is_empty())]
    fn default() -> Self {
        new!(CuktaTocExpansionState {
            expanded: Vec::new(),
            collapsed: Vec::new(),
        })
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub fn launch_app() {
    if is_window_document_context() {
        dioxus::launch(App);
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
pub fn launch_app() {
    dioxus::launch(App);
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub fn launch_app() {
    dioxus::LaunchBuilder::new()
        .with_cfg(
            dioxus::desktop::Config::new()
                .with_window(dioxus::desktop::WindowBuilder::new().with_title(APP_DISPLAY_NAME)),
        )
        .launch(App);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn is_window_document_context() -> bool {
    let global = js_sys::global();
    let Ok(window) = js_sys::Reflect::get(&global, &JsValue::from_str("window")) else {
        return false;
    };
    if window.is_null() || window.is_undefined() {
        return false;
    }
    let Ok(document) = js_sys::Reflect::get(&window, &JsValue::from_str("document")) else {
        return false;
    };
    !document.is_null() && !document.is_undefined()
}

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
fn App() -> Element {
    rsx! {
        Router::<JbotciRoute> {}
    }
}

#[requires(true)]
#[ensures(!ret.title.is_empty())]
fn route_document_meta(base_path: &str, route: &JbotciRoute) -> PageMeta {
    build_page_meta(base_path, &route.web_route)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn document_title_from_meta(meta: &PageMeta) -> String {
    meta.title.clone()
}

#[requires(true)]
#[ensures(true)]
fn apply_document_meta(mut document_meta: Signal<PageMeta>, meta: PageMeta) {
    sync_document_head(&meta);
    document_meta.set(meta);
}

#[requires(true)]
#[ensures(ret.contains("STIX Two Math"))]
#[ensures(ret.contains("STIX Two Text"))]
fn font_face_css() -> String {
    format!(
        r#"
@font-face {{
  font-family: "Noto Sans";
  src: url("{noto_sans}") format("truetype");
  font-weight: 100 900;
  font-stretch: 62.5% 100%;
  font-style: normal;
  font-display: swap;
}}

@font-face {{
  font-family: "Noto Sans";
  src: url("{noto_sans_italic}") format("truetype");
  font-weight: 100 900;
  font-stretch: 62.5% 100%;
  font-style: italic;
  font-display: swap;
}}

@font-face {{
  font-family: "STIX Two Math";
  src: url("{stix_two_math}") format("truetype");
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}}

@font-face {{
  font-family: "STIX Two Text";
  src: url("{stix_two_text}") format("truetype");
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}}

@font-face {{
  font-family: "STIX Two Text";
  src: url("{stix_two_text_bold}") format("truetype");
  font-weight: 700;
  font-style: normal;
  font-display: swap;
}}

@font-face {{
  font-family: "Crisa";
  src: url("{crisa}") format("opentype");
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}}
"#,
        noto_sans = NOTO_SANS,
        noto_sans_italic = NOTO_SANS_ITALIC,
        stix_two_math = STIX_TWO_MATH,
        stix_two_text = STIX_TWO_TEXT,
        stix_two_text_bold = STIX_TWO_TEXT_BOLD,
        crisa = CRISA,
    )
}

#[requires(true)]
#[ensures(ret.contains(".app-topbar-brand-logo"))]
#[ensures(ret.contains(".rich-dictionary-tooltip"))]
fn critical_startup_css() -> &'static str {
    r#"
.app-topbar-brand-logo {
  display: block;
  height: 1.9rem;
  width: auto;
}

.rich-dictionary-tooltip,
.rich-reference-tooltip-stack {
  position: fixed;
  left: 0;
  top: 0;
  visibility: hidden;
  pointer-events: none;
}
"#
}

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
fn AppShell() -> Element {
    let current_route_location = use_route::<JbotciRoute>();
    let route = use_signal(|| current_route_location.app_route());
    let base_path = router_base_path();
    let initial_document_meta = route_document_meta(&base_path, &current_route_location);
    let document_meta = use_signal(move || initial_document_meta.clone());
    let app_history = history();
    let settings = use_signal(load_settings);
    let initial_dialect_settings = load_dialect_settings();
    let initial_settings_dialect_selection =
        initial_dialect_settings_selection(&initial_dialect_settings);
    let mut dialect_settings = use_signal(move || initial_dialect_settings.clone());
    let mut settings_dialect_selection =
        use_signal(move || initial_settings_dialect_selection.clone());
    let settings_dialect_qr_uri = use_signal(|| None::<String>);
    let gentufa_dialect_picker_open = use_signal(|| false);
    let mut settings_johau_import_seen = use_signal(|| None::<String>);
    let embedding_settings = use_signal(EmbeddingSettingsState::default);
    let activity = use_signal(AsyncActivityState::default);
    let activity_indicator_visible = use_signal(|| false);
    let activity_indicator_delay_task = use_signal(|| None::<Task>);
    let topbar_settings_layout = use_signal(|| TopbarSettingsLayout::BothInline);
    let topbar_settings_open = use_signal(|| false);
    let topbar_nav_layout = use_signal(|| TopbarNavLayout::Full);
    let mut page_find_state = use_signal(PageFindState::default);
    let initial_gentufa = initial_gentufa_state(&current_route_location);
    let initial_gentufa_has_text = initial_gentufa_text_explicit(&current_route_location);
    let initial_gentufa_input_text = if initial_gentufa_has_text {
        initial_gentufa.text.clone()
    } else {
        String::new()
    };
    let initial_gentufa_parsed_text =
        if initial_gentufa.text.is_empty() && !initial_gentufa_has_text {
            DEFAULT_GENTUFA_TEXT.to_owned()
        } else {
            initial_gentufa.text.clone()
        };
    let initial_gentufa_dialect = initial_gentufa.dialect.clone().unwrap_or_default();
    let initial_gentufa_view_mode = initial_gentufa.view_mode;
    let initial_gentufa_display = GentufaDisplayState {
        show_elided: initial_gentufa.show_elided,
        show_glosses: initial_gentufa.show_glosses,
    };
    let view_mode = use_signal(move || initial_gentufa_view_mode);
    let gentufa_display = use_signal(move || initial_gentufa_display);
    let mut parsed_text_explicit = use_signal(move || initial_gentufa_has_text);
    let mut gentufa_url_write_intent = use_signal(|| GentufaUrlWriteIntent::ReplaceCurrent);
    let initial_cukta = initial_cukta_state(&current_route_location);
    let cukta_draft_state = use_signal(|| initial_cukta.clone());
    let cukta_committed_state = use_signal(|| initial_cukta);
    let cukta_toc_filter = use_signal(String::new);
    let cukta_toc_pinned = use_signal(load_cukta_toc_pinned);
    let cukta_toc_expansion = use_signal(load_cukta_toc_expansion);
    let cukta_toc_width = use_signal(load_cukta_toc_width);
    let cukta_toc_resize = use_signal(|| None::<CuktaTocResizeState>);
    let cukta_toc_overlay_visible = use_signal(|| false);
    let cukta_toc_forced_autohide = use_signal(cukta_toc_forced_autohide_active);
    let initial_vlacku = initial_vlacku_state(&current_route_location);
    let vlacku_draft_state = use_signal(|| initial_vlacku.clone());
    let vlacku_committed_state = use_signal(|| initial_vlacku);
    let pending_vlacku_scroll_restore = use_signal(|| None::<i32>);
    let vlacku_semantic_result = use_signal(VlackuSemanticResultState::default);
    let vlacku_result = use_signal(VlackuAsyncResultState::default);
    let vlacku_result_task = use_signal(|| None::<LatestAsyncTask>);
    let vlacku_semantic_task = use_signal(|| None::<LatestAsyncTask>);
    let cukta_semantic_result = use_signal(CuktaSemanticResultState::default);
    let cukta_page = use_signal(CuktaAsyncPageState::default);
    let cukta_page_task = use_signal(|| None::<LatestAsyncTask>);
    let cukta_semantic_task = use_signal(|| None::<LatestAsyncTask>);
    let initial_pending_cukta_scroll = current_cukta_pending_scroll(&current_route_location);
    let pending_cukta_scroll = use_signal(move || initial_pending_cukta_scroll.clone());
    let initial_last_route_for_scroll = current_route_location.clone();
    let mut last_route_for_scroll = use_signal(move || initial_last_route_for_scroll.clone());
    let initial_last_page_find_route = current_route_location.app_route();
    let mut last_page_find_route = use_signal(move || initial_last_page_find_route);
    let jvozba_pane = use_signal(load_vlacku_jvozba_pane_state);
    let jvozba_available = use_signal(vlacku_jvozba_available);
    let jvozba_drag = use_signal(|| None::<VlackuJvozbaDragState>);
    let initial_input_text = initial_gentufa_input_text;
    let initial_parsed_text = initial_gentufa_parsed_text;
    let initial_dialect = initial_gentufa_dialect.clone();
    let initial_parsed_dialect = initial_gentufa_dialect;
    let mut input_text = use_signal(move || initial_input_text.clone());
    let mut parsed_text = use_signal(move || initial_parsed_text.clone());
    let dialect = use_signal(move || initial_dialect.clone());
    let mut parsed_dialect = use_signal(move || initial_parsed_dialect.clone());
    let reference_hover = use_signal(ReferenceHoverState::default);
    let reference_tooltip_open = use_signal(|| None::<HoveredReference>);
    let gentufa_page = use_signal(GentufaAsyncPageState::default);
    let gentufa_page_task = use_signal(|| None::<LatestAsyncTask>);
    let gentufa_diagnostics_open = use_signal(|| true);
    let gentufa_active_diagnostic = use_signal(|| None::<usize>);
    let gentufa_input_diagnostic_tooltip = use_signal(|| None::<DiagnosticInputTooltip>);
    let export_task = use_signal(|| None::<LatestAsyncTask>);
    let mut pending_local_route_writes = use_signal(PendingLocalRouteWrites::default);

    let settings_value = *settings.read();
    let dialect_settings_value = dialect_settings.read().clone();
    let settings_dialect_selection_value = settings_dialect_selection.read().clone();
    let embedding_settings_value = embedding_settings.read().clone();
    let activity_value = activity.read().clone();
    let activity_indicator_visible_value = *activity_indicator_visible.read();
    let route_value = *route.read();
    let view_mode_value = *view_mode.read();
    let gentufa_display_value = *gentufa_display.read();
    let parsed_text_value = parsed_text.read().clone();
    let parsed_dialect_value = parsed_dialect.read().clone();
    let parsed_text_explicit_value = *parsed_text_explicit.read();
    let gentufa_url_write_intent_value = *gentufa_url_write_intent.read();
    let gentufa_page_value = gentufa_page.read().clone();
    let document_meta_value = document_meta.read().clone();
    let document_title = document_title_from_meta(&document_meta_value);
    let result = gentufa_page_value.result.clone();
    let gentufa_request = gentufa_page_value.request.clone();
    let cukta_committed_state_value = cukta_committed_state.read().clone();
    let cukta_page_value = cukta_page.read().clone();
    let vlacku_committed_state_value = vlacku_committed_state.read().clone();
    let vlacku_result_value = vlacku_result.read().clone();
    let page_find_state_value = page_find_state.read().clone();
    let current_page_find_route_state = page_find_state_value.route_state(route_value).clone();
    let page_find_entries = page_find_entries_for_route(
        route_value,
        &cukta_page_value,
        &vlacku_committed_state_value,
        &vlacku_result_value,
        &result,
        gentufa_request.as_ref(),
        view_mode_value,
        gentufa_display_value,
        settings_value,
        &dialect_settings_value,
        &settings_dialect_selection_value,
        &embedding_settings_value,
        settings_value.script,
    );
    let page_find_index =
        build_page_find_index(&current_page_find_route_state.query, &page_find_entries);
    let page_find_context = PageFindContext::new(&page_find_index, &current_page_find_route_state);
    let committed_gentufa_state = gentufa_state_from_parts(
        &parsed_text_value,
        &parsed_dialect_value,
        view_mode_value,
        gentufa_display_value,
        parsed_text_explicit_value,
    );
    let gentufa_url_inputs = new!(GentufaUrlInputs {
        active_route: route_value,
        current_route: current_route_location.clone(),
        state: committed_gentufa_state.clone(),
        text_explicit: parsed_text_explicit_value,
        intent: gentufa_url_write_intent_value,
    });
    let gentufa_compute_inputs = GentufaComputeInputs {
        route: route_value,
        settings: settings_value,
        dialect_settings: dialect_settings_value.clone(),
        display: gentufa_display_value,
        view_mode: view_mode_value,
        text: parsed_text_value.clone(),
        dialect_text: parsed_dialect_value.clone(),
        text_explicit: parsed_text_explicit_value,
    };
    let gentufa_layout_inputs = GentufaLayoutInputs {
        route: route_value,
        parsed_text_len: parsed_text_value.len(),
        parsed_dialect_len: parsed_dialect_value.len(),
        display: gentufa_display_value,
        view_mode: view_mode_value,
    };
    let topbar_cukta_route =
        JbotciRoute::from_web_route(WebRoute::Cukta(cukta_committed_state_value.clone()), false);
    let topbar_vlacku_route = JbotciRoute::from_web_route(
        WebRoute::Vlacku(vlacku_committed_state_value.clone()),
        false,
    );
    let topbar_gentufa_route =
        gentufa_route_for_committed_state(&committed_gentufa_state, parsed_text_explicit_value);
    let topbar_settings_route = JbotciRoute::from_web_route(WebRoute::Settings, false);
    install_browser_dom_handlers(
        jvozba_available,
        topbar_settings_layout,
        topbar_settings_open,
        topbar_nav_layout,
        cukta_toc_forced_autohide,
    );
    let scroll_base_path = base_path.clone();
    let scroll_route_location = current_route_location.clone();
    use_effect(use_reactive(
        (&scroll_route_location,),
        move |(location,)| {
            let previous = last_route_for_scroll.read().clone();
            if previous == location {
                return;
            }
            let scroll_already_pending = pending_cukta_scroll.read().is_some();
            if !scroll_already_pending {
                if let Some(scroll) =
                    cukta_pending_scroll_for_route_change(&scroll_base_path, &location)
                {
                    let mut pending = pending_cukta_scroll;
                    pending.set(Some(scroll));
                }
            }
            last_route_for_scroll.set(location);
        },
    ));
    let document_meta_route_location = current_route_location.clone();
    let document_meta_base_path = base_path.clone();
    use_effect(use_reactive(
        (&document_meta_route_location,),
        move |(location,)| {
            let meta = route_document_meta(&document_meta_base_path, &location);
            apply_document_meta(document_meta, meta);
        },
    ));
    let sync_route_location = current_route_location.clone();
    use_effect(use_reactive((&sync_route_location,), move |(location,)| {
        let is_local_route_write =
            pending_local_route_writes.with_mut(|pending| pending.consume(&location));
        apply_web_route_to_client_state(
            &location,
            is_local_route_write,
            route,
            cukta_draft_state,
            cukta_committed_state,
            vlacku_draft_state,
            vlacku_committed_state,
            input_text,
            parsed_text,
            parsed_text_explicit,
            dialect,
            parsed_dialect,
            view_mode,
            gentufa_display,
        );
    }));
    use_effect(move || {
        let current = *route.read();
        let previous = *last_page_find_route.read();
        if previous == current {
            return;
        }
        page_find_state.with_mut(|state| {
            reset_page_find_active(state.route_state_mut(previous));
            reset_page_find_active(state.route_state_mut(current));
        });
        last_page_find_route.set(current);
    });
    let page_find_signature = page_find_index.signature;
    let page_find_match_count = page_find_index.matches.len();
    use_effect(use_reactive(
        &(route_value, page_find_signature, page_find_match_count),
        move |(route, signature, match_count)| {
            page_find_state.with_mut(|state| {
                sync_page_find_result_signature(state, route, signature, match_count);
            });
        },
    ));
    let page_find_scroll_request = current_page_find_route_state.scroll_request;
    let page_find_active_index = page_find_context.active_index;
    use_effect(use_reactive(
        &(
            route_value,
            page_find_scroll_request,
            page_find_active_index,
        ),
        move |(_route, scroll_request, active_index)| {
            if scroll_request > 0
                && let Some(active_index) = active_index
            {
                schedule_page_find_match_scroll(active_index);
            }
        },
    ));
    use_effect(move || {
        pin_worker_client_asset();
        configure_embedding_worker_url(&format!("{EMBEDDING_WORKER_JS}"));
        configure_embedding_ort_assets(
            &format!("{ORT_WASM_MIN_MJS}"),
            &format!("{ORT_WASM_SIMD_THREADED_MJS}"),
            &format!("{ORT_WASM_SIMD_THREADED_WASM}"),
        );
        configure_embedding_remote_base_url(web_embeddings_base_url());
        configure_embedding_model_key(&embedding_settings.read().selected_model_key);
        configure_compute_worker_url(&format!("{COMPUTE_WORKER_JS}"));
    });
    use_effect(move || {
        let active = activity.read().is_active();
        let mut visible = activity_indicator_visible;
        let mut delay_task = activity_indicator_delay_task;
        if !active {
            if let Some(task) = delay_task.write().take() {
                task.cancel();
            }
            visible.set(false);
            return;
        }
        if *visible.read() || delay_task.read().is_some() {
            return;
        }
        let activity_for_delay = activity;
        let mut visible_for_delay = visible;
        let mut delay_task_for_delay = delay_task;
        let task = spawn(async move {
            sleep_ms(ASYNC_ACTIVITY_INDICATOR_DELAY_MS).await;
            if activity_for_delay.read().is_active() {
                visible_for_delay.set(true);
            }
            delay_task_for_delay.set(None);
        });
        delay_task.set(Some(task));
    });
    use_effect(move || {
        if *route.read() == AppRoute::Settings {
            spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                refresh_embedding_settings(embedding_settings).await;
            });
        }
    });
    let settings_route_location = current_route_location.clone();
    use_effect(use_reactive(
        (&settings_route_location,),
        move |(location,)| {
            if location.app_route() != AppRoute::Settings {
                return;
            }
            let Some(raw_johau) = query_param(&location.settings_query, "johau") else {
                return;
            };
            if settings_johau_import_seen.read().as_deref() == Some(raw_johau.as_str()) {
                return;
            }
            settings_johau_import_seen.set(Some(raw_johau.clone()));
            let current_settings = dialect_settings.read().clone();
            if let Ok((selected_name, next_settings)) =
                import_johau_dialect_settings(&raw_johau, &current_settings)
            {
                save_dialect_settings(&next_settings);
                dialect_settings.set(next_settings);
                settings_dialect_selection.set(selected_name);
            }
        },
    ));
    let gentufa_base_path = base_path.clone();
    use_effect(use_reactive((&gentufa_compute_inputs,), move |(inputs,)| {
        if inputs.route != AppRoute::Gentufa {
            cancel_compute_channel(COMPUTE_CHANNEL_GENTUFA);
            cancel_latest_task(gentufa_page_task);
            return;
        }
        let state = gentufa_state_from_parts(
            &inputs.text,
            &inputs.dialect_text,
            inputs.view_mode,
            inputs.display,
            inputs.text_explicit,
        );
        let request = GentufaWebRequest {
            text: inputs.text.clone(),
            options: web_options(
                inputs.settings,
                inputs.display,
                inputs.view_mode,
                inputs.dialect_text.clone(),
                &inputs.dialect_settings,
            ),
        };
        let mut page_signal = gentufa_page;
        page_signal.with_mut(|page| {
            page.state = Some(state.clone());
            page.request = Some(request.clone());
            page.loading = true;
            page.error = None;
        });
        let base_path = gentufa_base_path.clone();
        let mut result_signal = gentufa_page;
        cancel_compute_channel(COMPUTE_CHANNEL_GENTUFA);
        spawn_latest_tracked(
            gentufa_page_task,
            activity,
            AsyncTaskKind::Gentufa,
            async move {
                let response = compute_request(
                    COMPUTE_CHANNEL_GENTUFA,
                    WebComputeRequest::GentufaPage {
                        base_path,
                        state: state.clone(),
                        request: request.clone(),
                    },
                )
                .await;
                match response {
                    Ok(WebComputeResponse::GentufaPage { result, meta }) => {
                        result_signal.set(GentufaAsyncPageState {
                            state: Some(state),
                            request: Some(request),
                            result,
                            meta: Some(meta.clone()),
                            loading: false,
                            error: None,
                        });
                        apply_document_meta(document_meta, meta);
                        schedule_gentufa_block_reference_layout();
                        schedule_gentufa_tree_layout();
                    }
                    Ok(_) => {
                        result_signal.set(gentufa_async_error_state(
                            state,
                            request,
                            "compute worker returned the wrong gentufa response",
                        ));
                    }
                    Err(error) => {
                        result_signal.set(gentufa_async_error_state(state, request, &error));
                    }
                }
            },
        );
    }));
    use_effect(move || {
        let state = vlacku_committed_state.read().clone();
        let mut result_signal = vlacku_semantic_result;
        if *route.read() != AppRoute::Vlacku
            || state.mode != VlackuWebMode::Meaning
            || state.query.trim().is_empty()
        {
            cancel_embedding_channel(EMBEDDING_CHANNEL_VLACKU_SEMANTIC);
            cancel_latest_task(vlacku_semantic_task);
            result_signal.set(VlackuSemanticResultState::default());
            return;
        }
        result_signal.set(VlackuSemanticResultState {
            state: Some(state.clone()),
            hits: Vec::new(),
            message: None,
            loading: true,
        });
        cancel_embedding_channel(EMBEDDING_CHANNEL_VLACKU_SEMANTIC);
        spawn_latest_tracked(
            vlacku_semantic_task,
            activity,
            AsyncTaskKind::Vlacku,
            async move {
                spawn_vlacku_semantic_loading_message(result_signal, state.clone());
                let result = load_vlacku_semantic_result(state).await;
                result_signal.set(result);
            },
        );
    });
    let vlacku_page_base_path = base_path.clone();
    use_effect(move || {
        if *route.read() != AppRoute::Vlacku {
            cancel_compute_channel(COMPUTE_CHANNEL_VLACKU);
            cancel_latest_task(vlacku_result_task);
            return;
        }
        let state = vlacku_committed_state.read().clone();
        let semantic = vlacku_semantic_result.read().clone();
        let mut page_signal = vlacku_result;
        if vlacku_semantic_result_is_pending(&state, &semantic) {
            cancel_compute_channel(COMPUTE_CHANNEL_VLACKU);
            cancel_latest_task(vlacku_result_task);
            let meta = page_signal.with_mut(|page| {
                apply_vlacku_semantic_pending_page(page, &vlacku_page_base_path, &state, &semantic)
            });
            apply_document_meta(document_meta, meta);
            return;
        }
        let request = vlacku_compute_request(&vlacku_page_base_path, &state, &semantic);
        page_signal.with_mut(|page| {
            page.state = Some(state.clone());
            page.loading = true;
            page.error = None;
        });
        let mut result_signal = vlacku_result;
        cancel_compute_channel(COMPUTE_CHANNEL_VLACKU);
        spawn_latest_tracked(
            vlacku_result_task,
            activity,
            AsyncTaskKind::Vlacku,
            async move {
                let response = compute_request(COMPUTE_CHANNEL_VLACKU, request).await;
                match response {
                    Ok(WebComputeResponse::VlackuPage { result, meta }) => {
                        result_signal.set(VlackuAsyncResultState {
                            state: Some(state),
                            result,
                            meta: Some(meta.clone()),
                            loading: false,
                            error: None,
                        });
                        apply_document_meta(document_meta, meta);
                    }
                    Ok(_) => {
                        result_signal.set(vlacku_async_error_state(
                            &state,
                            "compute worker returned the wrong vlacku response",
                        ));
                    }
                    Err(error) => {
                        result_signal.set(vlacku_async_error_state(&state, &error));
                    }
                }
                schedule_vlacku_jvozba_pane_metrics_sync();
            },
        );
    });
    use_effect(move || {
        let mut result_signal = cukta_semantic_result;
        let state = cukta_committed_state.read().clone();
        let search_state = match state.view {
            CuktaWebView::Search(search_state)
                if search_state.mode == CuktaWebMode::Meaning
                    && !search_state.query.trim().is_empty() =>
            {
                search_state
            }
            _ => {
                cancel_embedding_channel(EMBEDDING_CHANNEL_CUKTA_SEMANTIC);
                cancel_latest_task(cukta_semantic_task);
                result_signal.set(CuktaSemanticResultState::default());
                return;
            }
        };
        if *route.read() != AppRoute::Cukta {
            cancel_embedding_channel(EMBEDDING_CHANNEL_CUKTA_SEMANTIC);
            cancel_latest_task(cukta_semantic_task);
            result_signal.set(CuktaSemanticResultState::default());
            return;
        }
        result_signal.set(CuktaSemanticResultState {
            state: Some(search_state.clone()),
            hits: Vec::new(),
            message: None,
            loading: true,
        });
        cancel_embedding_channel(EMBEDDING_CHANNEL_CUKTA_SEMANTIC);
        spawn_latest_tracked(
            cukta_semantic_task,
            activity,
            AsyncTaskKind::Cukta,
            async move {
                spawn_cukta_semantic_loading_message(result_signal, search_state.clone());
                let result = load_cukta_semantic_result(search_state).await;
                result_signal.set(result);
            },
        );
    });
    let cukta_page_base_path = base_path.clone();
    use_effect(move || {
        if *route.read() != AppRoute::Cukta {
            cancel_compute_channel(COMPUTE_CHANNEL_CUKTA);
            cancel_latest_task(cukta_page_task);
            return;
        }
        let state = cukta_committed_state.read().clone();
        let semantic = cukta_semantic_result.read().clone();
        let request = cukta_compute_request(&cukta_page_base_path, &state, &semantic);
        let mut page_signal = cukta_page;
        page_signal.with_mut(|page| {
            page.state = Some(state.clone());
            page.loading = true;
            page.error = None;
        });
        let mut result_signal = cukta_page;
        cancel_compute_channel(COMPUTE_CHANNEL_CUKTA);
        spawn_latest_tracked(
            cukta_page_task,
            activity,
            AsyncTaskKind::Cukta,
            async move {
                let response = compute_request(COMPUTE_CHANNEL_CUKTA, request).await;
                match response {
                    Ok(WebComputeResponse::CuktaPage { page, meta }) => {
                        result_signal.set(CuktaAsyncPageState {
                            state: Some(state),
                            page,
                            meta: Some(meta.clone()),
                            loading: false,
                            error: None,
                        });
                        apply_document_meta(document_meta, meta);
                    }
                    Ok(_) => {
                        result_signal.set(cukta_async_error_state(
                            state,
                            "compute worker returned the wrong cukta response",
                        ));
                    }
                    Err(error) => {
                        result_signal.set(cukta_async_error_state(state, &error));
                    }
                }
            },
        );
    });
    let cukta_scroll_route = route;
    let cukta_scroll_state = cukta_committed_state;
    let cukta_scroll_page = cukta_page;
    let mut cukta_scroll_pending = pending_cukta_scroll;
    use_effect(move || {
        if cukta_scroll_pending.read().is_none() {
            return;
        }
        if *cukta_scroll_route.read() != AppRoute::Cukta {
            return;
        }
        let page_ready = {
            let state = cukta_scroll_state.read();
            let page = cukta_scroll_page.read();
            cukta_page_ready_for_scroll(&page, &state)
        };
        if !page_ready {
            return;
        }
        if let Some(scroll) = cukta_scroll_pending.write().take() {
            apply_cukta_pending_scroll(scroll);
        }
    });
    let vlacku_url_history = app_history.clone();
    let vlacku_url_route_location = current_route_location.clone();
    let mut vlacku_url_scroll_restore = pending_vlacku_scroll_restore;
    use_effect(move || {
        if *route.read() == AppRoute::Vlacku {
            let state = vlacku_committed_state.read().clone();
            let restore_scroll_y = vlacku_url_scroll_restore.write().take();
            schedule_vlacku_url_push(
                vlacku_url_history.clone(),
                pending_local_route_writes,
                &vlacku_url_route_location,
                &state,
                restore_scroll_y,
            );
        }
    });
    let cukta_url_route_location = current_route_location.clone();
    let cukta_url_history = app_history.clone();
    use_effect(move || {
        if *route.read() == AppRoute::Cukta {
            let state = cukta_committed_state.read().clone();
            push_cukta_url(
                cukta_url_history.clone(),
                pending_local_route_writes,
                &cukta_url_route_location,
                &state,
            );
        }
    });
    let gentufa_url_history = app_history.clone();
    let mut gentufa_url_intent_for_effect = gentufa_url_write_intent;
    use_effect(use_reactive((&gentufa_url_inputs,), move |(inputs,)| {
        if !gentufa_url_sync_allowed(inputs.active_route, &inputs.current_route) {
            set_gentufa_url_write_intent_if_changed(
                &mut gentufa_url_intent_for_effect,
                inputs.intent,
                GentufaUrlWriteIntent::ReplaceCurrent,
            );
            return;
        }
        sync_gentufa_committed_url(
            gentufa_url_history.clone(),
            pending_local_route_writes,
            &inputs.current_route,
            &inputs.state,
            inputs.text_explicit,
            inputs.intent,
            gentufa_url_intent_for_effect,
        );
    }));
    use_effect(move || {
        if *route.read() == AppRoute::Vlacku {
            let state = vlacku_draft_state.read().clone();
            let pane_open = jvozba_pane.read().open;
            let pane_available = *jvozba_available.read();
            set_brivla_toggle_indeterminate(vlacku_brivla_filter_indeterminate(&state.word_types));
            let _ = (pane_open, pane_available);
            schedule_vlacku_jvozba_pane_metrics_sync();
        }
    });
    use_effect(move || {
        if *route.read() == AppRoute::Cukta {
            restore_cukta_toc_scroll();
        }
    });
    use_effect(move || {
        let _ = (
            *route.read(),
            settings.read().theme,
            settings.read().script,
            activity.read().is_active(),
            *topbar_settings_layout.read(),
            *topbar_nav_layout.read(),
        );
        schedule_topbar_settings_layout_measure(
            topbar_settings_layout,
            topbar_settings_open,
            topbar_nav_layout,
        );
        schedule_topbar_active_nav_sync();
        if *route.read() == AppRoute::Vlacku {
            schedule_vlacku_jvozba_pane_metrics_sync();
        }
    });
    use_effect(use_reactive((&gentufa_layout_inputs,), move |(inputs,)| {
        if inputs.route == AppRoute::Gentufa {
            schedule_gentufa_block_reference_layout();
            schedule_gentufa_tree_layout();
        }
    }));
    use_effect(move || {
        if *route.read() == AppRoute::Gentufa {
            let _ = input_text.read().len();
            schedule_gentufa_textarea_resize();
        }
    });
    let app_class = format!(
        "spa-shell app-page theme-{} orthography-{}",
        theme_class(settings_value.theme),
        script_class(settings_value.script)
    );
    let manifest_href = static_asset_href_with_base_path(&base_path, MANIFEST_ASSET_PATH);
    let favicon_href = static_asset_href_with_base_path(&base_path, FAVICON_ASSET_PATH);
    let apple_touch_icon_href =
        static_asset_href_with_base_path(&base_path, APPLE_TOUCH_ICON_ASSET_PATH);

    rsx! {
        document::Title { "{document_title}" }
        style { "{font_face_css()}\n{critical_startup_css()}" }
        document::Stylesheet { href: MAIN_CSS }
        if cfg!(target_arch = "wasm32") {
            document::Link { rel: "modulepreload", href: COMPUTE_WORKER_JS }
            document::Link { rel: "modulepreload", href: EMBEDDING_WORKER_JS }
            document::Link { rel: "manifest", href: "{manifest_href}" }
        }
        document::Link { rel: "icon", r#type: "image/png", href: "{favicon_href}" }
        document::Link { rel: "shortcut icon", r#type: "image/png", href: "{favicon_href}" }
        document::Link { rel: "apple-touch-icon", href: "{apple_touch_icon_href}" }
        div { class: "{app_class}",
            { render_topbar(
                route_value,
                settings,
                settings_value,
                topbar_cukta_route,
                topbar_vlacku_route,
                topbar_gentufa_route,
                topbar_settings_route,
                &base_path,
                pending_cukta_scroll,
                *topbar_settings_layout.read(),
                topbar_settings_open,
                *topbar_nav_layout.read(),
                page_find_state,
                &page_find_context,
                &activity_value,
                activity_indicator_visible_value,
            ) }
            main { class: "spa-main", "data-app-scroll": "main",
                div { class: "spa-stack",
                    {
                        match route_value {
                            AppRoute::Gentufa => rsx! {
                                section {
                                    class: "spa-page parse-page spa-gentufa-page",
                                    onmousemove: move |_| refresh_reference_hover(reference_hover, ReferenceHoverRefreshReason::PointerMove),
                                    onwheel: move |_| refresh_reference_hover(reference_hover, ReferenceHoverRefreshReason::ViewportShift),
                                    h1 { class: "sr-only", "jbotci gentufa" }
                                    div { class: "page-container",
                                        div { class: "input-form",
                                            div { class: "form-group",
                                                { render_gentufa_input(
                                                    input_text,
                                                    &result,
                                                    gentufa_request.as_ref(),
                                                    *gentufa_active_diagnostic.read(),
                                                    gentufa_active_diagnostic,
                                                    gentufa_input_diagnostic_tooltip,
                                                    *gentufa_input_diagnostic_tooltip.read(),
                                                    pending_cukta_scroll,
                                                    &base_path,
                                                    settings_value.script,
                                                ) }
                                                div { class: "form-actions",
                                                    { render_dialect_control(dialect, dialect_settings_value.clone(), gentufa_dialect_picker_open) }
                                                    button {
                                                        class: "btn-parse",
                                                        r#type: "button",
                                                        onclick: move |_| {
                                                            let mut next_text = input_text.read().clone();
                                                            let next_dialect = dialect.read().clone();
                                                            if next_text.trim().is_empty() {
                                                                next_text = DEFAULT_GENTUFA_TEXT.to_owned();
                                                                input_text.set(next_text.clone());
                                                                schedule_gentufa_textarea_resize();
                                                            }
                                                            parsed_text_explicit.set(true);
                                                            parsed_text.set(next_text);
                                                            parsed_dialect.set(next_dialect);
                                                            gentufa_url_write_intent.set(GentufaUrlWriteIntent::PushParse);
                                                        },
                                                        "Parse"
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "gentufa-result-stack",
                                            { render_result(
                                                &result,
                                                gentufa_request.as_ref(),
                                                gentufa_diagnostics_open,
                                                *gentufa_diagnostics_open.read(),
                                                gentufa_active_diagnostic,
                                                pending_cukta_scroll,
                                                &base_path,
                                                view_mode,
                                                view_mode_value,
                                                gentufa_display,
                                                gentufa_display_value,
                                                settings_value,
                                                reference_hover,
                                                reference_tooltip_open,
                                                activity,
                                                export_task,
                                                &page_find_context,
                                            ) }
                                        }
                                    }
                                }
                            },
                            AppRoute::Settings => render_settings(
                                settings,
                                settings_value,
                                dialect_settings,
                                dialect_settings_value.clone(),
                                settings_dialect_selection,
                                settings_dialect_qr_uri,
                                embedding_settings,
                                activity,
                                &page_find_context,
                            ),
                            AppRoute::Cukta => {
                                render_cukta_page(
                                    cukta_draft_state,
                                    cukta_committed_state,
                                    cukta_page,
                                    cukta_toc_filter,
                                    cukta_toc_pinned,
                                    cukta_toc_expansion,
                                    cukta_toc_width,
                                    cukta_toc_resize,
                                    cukta_toc_overlay_visible,
                                    cukta_toc_forced_autohide,
                                    pending_cukta_scroll,
                                    &base_path,
                                    settings_value.script,
                                    &page_find_context,
                                )
                            }
                            AppRoute::Vlacku => {
                                render_vlacku_page(
                                    vlacku_draft_state,
                                    vlacku_committed_state,
                                    vlacku_result,
                                    jvozba_pane,
                                    jvozba_available,
                                    jvozba_drag,
                                    pending_cukta_scroll,
                                    pending_vlacku_scroll_restore,
                                    &base_path,
                                    settings_value.script,
                                    &page_find_context,
                                )
                            },
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_topbar(
    route: AppRoute,
    settings: Signal<UserSettings>,
    current: UserSettings,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gentufa_route: JbotciRoute,
    settings_route: JbotciRoute,
    base_path: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    settings_layout: TopbarSettingsLayout,
    settings_open: Signal<bool>,
    nav_layout: TopbarNavLayout,
    page_find_state: Signal<PageFindState>,
    page_find: &PageFindContext,
    activity: &AsyncActivityState,
    activity_visible: bool,
) -> Element {
    let cukta_loading = activity_visible && activity.has_kind(AsyncTaskKind::Cukta);
    let vlacku_loading = activity_visible && activity.has_kind(AsyncTaskKind::Vlacku);
    let gentufa_loading = activity_visible && activity.has_kind(AsyncTaskKind::Gentufa);
    let activity_class = topbar_activity_class(activity_visible);
    let header_class = topbar_header_class(settings_layout, *settings_open.read(), nav_layout);
    let show_theme_inline = settings_layout.shows_theme_inline();
    let show_script_inline = settings_layout.shows_script_inline();
    let topbar_home_href = deployment_root_href(base_path);
    let logo_title = logo_title_text();
    rsx! {
        header { class: "{header_class}",
            div { class: "app-topbar-inner spa-topbar-inner",
                div { class: "app-topbar-left",
                    a {
                        class: "app-topbar-brand",
                        href: "{topbar_home_href}",
                        aria_label: "jbotci home",
                        title: "{logo_title}",
                        img { class: "app-topbar-brand-logo", src: LOGO, alt: "jbotci" }
                    }
                    { render_topbar_settings_button(settings, current, settings_route.clone(), settings_layout, settings_open) }
                    if show_theme_inline {
                        span { class: "app-topbar-theme app-topbar-theme-mode",
                            { render_theme_switch(settings, current.theme) }
                        }
                    }
                    if show_script_inline {
                        span { class: "app-topbar-theme app-topbar-orthography",
                            { render_script_switch(settings, current.script) }
                        }
                    }
                    match nav_layout {
                        TopbarNavLayout::Full => {
                            { render_topbar_nav(route, cukta_loading, vlacku_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
                        }
                        TopbarNavLayout::Carousel => {
                            { render_topbar_nav_carousel(route, cukta_loading, vlacku_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
                        }
                    }
                }
                { render_topbar_fit_probes(
                    settings,
                    current,
                    route,
                    cukta_loading,
                    vlacku_loading,
                    gentufa_loading,
                    cukta_route,
                    vlacku_route,
                    gentufa_route,
                    base_path,
                    pending_cukta_scroll,
                ) }
                div { class: "{activity_class}", role: "status", aria_live: "polite",
                    span { class: "sr-only",
                        if activity_visible {
                            "Working"
                        }
                    }
                    span { class: "app-topbar-activity-dots", aria_hidden: "true",
                        span { class: "app-topbar-activity-dot" }
                        span { class: "app-topbar-activity-dot" }
                        span { class: "app-topbar-activity-dot" }
                    }
                }
                div { class: "app-topbar-right",
                    { render_page_find_control(route, page_find_state, page_find) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_topbar_nav(
    route: AppRoute,
    cukta_loading: bool,
    vlacku_loading: bool,
    gentufa_loading: bool,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gentufa_route: JbotciRoute,
    base_path: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
) -> Element {
    let topbar_cukta_scroll_target = route_href_with_base_path(base_path, &cukta_route);
    let topbar_cukta_click_route = cukta_route.clone();
    rsx! {
        nav { class: "spa-nav", aria_label: "Primary navigation",
            Link {
                class: topbar_link_class(route == AppRoute::Cukta, cukta_loading),
                to: cukta_route,
                aria_current: if route == AppRoute::Cukta { "page" } else { "false" },
                onclick_only: true,
                onclick: move |_| {
                    push_route_with_cukta_scroll_intent(
                        pending_cukta_scroll,
                        Some(cukta_stored_pending_scroll(topbar_cukta_scroll_target.clone())),
                        topbar_cukta_click_route.clone(),
                    );
                },
                span { class: "app-topbar-link-label", "cukta" }
            }
            Link {
                class: topbar_link_class(route == AppRoute::Vlacku, vlacku_loading),
                to: vlacku_route,
                aria_current: if route == AppRoute::Vlacku { "page" } else { "false" },
                span { class: "app-topbar-link-label", "vlacku" }
            }
            Link {
                class: topbar_link_class(route == AppRoute::Gentufa, gentufa_loading),
                to: gentufa_route,
                aria_current: if route == AppRoute::Gentufa { "page" } else { "false" },
                span { class: "app-topbar-link-label", "gentufa" }
                span { class: "app-topbar-link-dots", aria_hidden: "true",
                    span { class: "app-topbar-link-dot" }
                    span { class: "app-topbar-link-dot" }
                    span { class: "app-topbar-link-dot" }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(true)]
#[ensures(true)]
fn render_topbar_nav_carousel(
    route: AppRoute,
    cukta_loading: bool,
    vlacku_loading: bool,
    gentufa_loading: bool,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gentufa_route: JbotciRoute,
    base_path: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
) -> Element {
    let [previous_route, current_route, next_route] = topbar_carousel_routes(route);
    rsx! {
        nav { class: "spa-nav app-topbar-nav-carousel", aria_label: "Primary navigation",
            div { class: "app-topbar-nav-carousel-track",
                { render_topbar_nav_carousel_link(
                    previous_route,
                    route,
                    "is-adjacent is-previous",
                    cukta_loading,
                    vlacku_loading,
                    gentufa_loading,
                    cukta_route.clone(),
                    vlacku_route.clone(),
                    gentufa_route.clone(),
                    base_path,
                    pending_cukta_scroll,
                ) }
                { render_topbar_nav_carousel_link(
                    current_route,
                    route,
                    "is-current-slot",
                    cukta_loading,
                    vlacku_loading,
                    gentufa_loading,
                    cukta_route.clone(),
                    vlacku_route.clone(),
                    gentufa_route.clone(),
                    base_path,
                    pending_cukta_scroll,
                ) }
                { render_topbar_nav_carousel_link(
                    next_route,
                    route,
                    "is-adjacent is-next",
                    cukta_loading,
                    vlacku_loading,
                    gentufa_loading,
                    cukta_route,
                    vlacku_route,
                    gentufa_route,
                    base_path,
                    pending_cukta_scroll,
                ) }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(target != AppRoute::Settings)]
#[requires(!slot_class.is_empty())]
#[ensures(true)]
fn render_topbar_nav_carousel_link(
    target: AppRoute,
    active_route: AppRoute,
    slot_class: &'static str,
    cukta_loading: bool,
    vlacku_loading: bool,
    gentufa_loading: bool,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gentufa_route: JbotciRoute,
    base_path: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
) -> Element {
    let active = target == active_route;
    let loading =
        topbar_carousel_route_loading(target, cukta_loading, vlacku_loading, gentufa_loading);
    let class = topbar_carousel_link_class(active, loading, slot_class);
    let aria_current = if active { "page" } else { "false" };
    let data_active = if active { "true" } else { "false" };
    let label = topbar_carousel_route_label(target);
    let target_route = match target {
        AppRoute::Cukta => cukta_route,
        AppRoute::Vlacku => vlacku_route,
        AppRoute::Gentufa => gentufa_route,
        AppRoute::Settings => return rsx! {},
    };
    let href = route_href_with_base_path(base_path, &target_route);
    let pending_scroll = if target == AppRoute::Cukta {
        Some(cukta_stored_pending_scroll(href.clone()))
    } else {
        None
    };
    let click_route = target_route.clone();
    rsx! {
        a {
            key: "{label}",
            class: "{class}",
            href: "{href}",
            aria_current,
            "data-topbar-nav-active": data_active,
            onclick: move |event| {
                if !event.modifiers().is_empty() {
                    return;
                }
                event.prevent_default();
                push_route_with_cukta_scroll_intent(
                    pending_cukta_scroll,
                    pending_scroll.clone(),
                    click_route.clone(),
                );
            },
            span { class: "app-topbar-link-label", "{label}" }
            if target == AppRoute::Gentufa {
                span { class: "app-topbar-link-dots", aria_hidden: "true",
                    span { class: "app-topbar-link-dot" }
                    span { class: "app-topbar-link-dot" }
                    span { class: "app-topbar-link-dot" }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(true)]
#[ensures(true)]
fn render_topbar_nav_carousel_probe(
    route: AppRoute,
    cukta_loading: bool,
    vlacku_loading: bool,
    gentufa_loading: bool,
) -> Element {
    let [previous_route, current_route, next_route] = topbar_carousel_routes(route);
    let previous_label = topbar_carousel_route_label(previous_route);
    let current_label = topbar_carousel_route_label(current_route);
    let next_label = topbar_carousel_route_label(next_route);
    rsx! {
        nav { class: "spa-nav app-topbar-nav-carousel", aria_label: "Primary navigation",
            div { class: "app-topbar-nav-carousel-track",
                span {
                    class: topbar_carousel_link_class(
                        previous_route == route,
                        topbar_carousel_route_loading(previous_route, cukta_loading, vlacku_loading, gentufa_loading),
                        "is-adjacent is-previous",
                    ),
                    "data-topbar-nav-active": if previous_route == route { "true" } else { "false" },
                    span { class: "app-topbar-link-label", "{previous_label}" }
                }
                span {
                    class: topbar_carousel_link_class(
                        current_route == route,
                        topbar_carousel_route_loading(current_route, cukta_loading, vlacku_loading, gentufa_loading),
                        "is-current-slot",
                    ),
                    "data-topbar-nav-active": if current_route == route { "true" } else { "false" },
                    span { class: "app-topbar-link-label", "{current_label}" }
                }
                span {
                    class: topbar_carousel_link_class(
                        next_route == route,
                        topbar_carousel_route_loading(next_route, cukta_loading, vlacku_loading, gentufa_loading),
                        "is-adjacent is-next",
                    ),
                    "data-topbar-nav-active": if next_route == route { "true" } else { "false" },
                    span { class: "app-topbar-link-label", "{next_label}" }
                }
            }
        }
    }
}

#[requires(true)]
#[requires(!slot_class.is_empty())]
#[ensures(!ret.is_empty())]
fn topbar_carousel_link_class(active: bool, loading: bool, slot_class: &'static str) -> String {
    let base = format!("app-topbar-link app-topbar-carousel-link {slot_class}");
    class_names(&base, &[("active", active), ("is-loading", loading)])
}

#[requires(true)]
#[ensures(!ret.contains(&AppRoute::Settings))]
#[ensures(route == AppRoute::Settings || ret[1] == route)]
fn topbar_carousel_routes(route: AppRoute) -> [AppRoute; 3] {
    match route {
        AppRoute::Cukta => [AppRoute::Gentufa, AppRoute::Cukta, AppRoute::Vlacku],
        AppRoute::Vlacku => [AppRoute::Cukta, AppRoute::Vlacku, AppRoute::Gentufa],
        AppRoute::Gentufa => [AppRoute::Vlacku, AppRoute::Gentufa, AppRoute::Cukta],
        AppRoute::Settings => [AppRoute::Cukta, AppRoute::Vlacku, AppRoute::Gentufa],
    }
}

#[requires(route != AppRoute::Settings)]
#[ensures(true)]
fn topbar_carousel_route_loading(
    route: AppRoute,
    cukta_loading: bool,
    vlacku_loading: bool,
    gentufa_loading: bool,
) -> bool {
    match route {
        AppRoute::Cukta => cukta_loading,
        AppRoute::Vlacku => vlacku_loading,
        AppRoute::Gentufa => gentufa_loading,
        AppRoute::Settings => false,
    }
}

#[requires(route != AppRoute::Settings)]
#[ensures(!ret.is_empty())]
fn topbar_carousel_route_label(route: AppRoute) -> &'static str {
    match route {
        AppRoute::Cukta => "cukta",
        AppRoute::Vlacku => "vlacku",
        AppRoute::Gentufa => "gentufa",
        AppRoute::Settings => "",
    }
}

#[requires(true)]
#[ensures(true)]
fn render_page_find_control(
    route: AppRoute,
    mut page_find_state: Signal<PageFindState>,
    page_find: &PageFindContext,
) -> Element {
    let query = page_find.query.clone();
    let placeholder = page_find_placeholder(route);
    let match_count = page_find.match_count;
    let counter = page_find_counter_text(page_find.active_index, match_count, !query.is_empty());
    let controls_disabled = match_count == 0;
    let query_for_keydown = query.clone();
    rsx! {
        div { class: "page-find-control", role: "search",
            span { class: "page-find-icon", aria_hidden: "true",
                svg { view_box: "0 0 20 20",
                    circle { cx: "8.5", cy: "8.5", r: "5.5" }
                    path { d: "M12.5 12.5L17 17" }
                }
            }
            input {
                id: PAGE_FIND_INPUT_ID,
                class: "page-find-input",
                r#type: "search",
                aria_label: "Find on this page",
                placeholder,
                spellcheck: "false",
                value: "{query}",
                oninput: move |event| {
                    let next_query = event.value();
                    page_find_state.with_mut(|state| {
                        set_page_find_query(
                            state,
                            route,
                            next_query,
                            PageFindRouteQueryUpdate::Replace,
                        );
                    });
                },
                onkeydown: move |event| {
                    let key = event.data().key();
                    if key == Key::Enter {
                        event.prevent_default();
                        let direction = if event.data().modifiers().contains(Modifiers::SHIFT) {
                            PageFindDirection::Previous
                        } else {
                            PageFindDirection::Next
                        };
                        page_find_state.with_mut(|state| {
                            update_page_find_active(state, route, direction, match_count);
                        });
                    } else if key == Key::Escape && !query_for_keydown.is_empty() {
                        event.prevent_default();
                        page_find_state.with_mut(|state| {
                            set_page_find_query(
                                state,
                                route,
                                String::new(),
                                PageFindRouteQueryUpdate::Clear,
                            );
                        });
                    }
                },
            }
            span { class: "page-find-actions",
                if !query.is_empty() {
                    button {
                        class: "page-find-button page-find-clear",
                        r#type: "button",
                        aria_label: "Clear page find",
                        title: "Clear",
                        onclick: move |_| {
                            page_find_state.with_mut(|state| {
                                set_page_find_query(
                                    state,
                                    route,
                                    String::new(),
                                    PageFindRouteQueryUpdate::Clear,
                                );
                            });
                        },
                        svg {
                            class: "page-find-button-icon",
                            view_box: "0 0 20 20",
                            "aria-hidden": "true",
                            path {
                                d: "M5 5L15 15M15 5L5 15",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "2.2",
                                stroke_linecap: "round",
                            }
                        }
                    }
                }
                button {
                    class: "page-find-button page-find-prev",
                    r#type: "button",
                    aria_label: "Previous page find match",
                    title: "Previous",
                    disabled: controls_disabled,
                    onclick: move |_| {
                        page_find_state.with_mut(|state| {
                            update_page_find_active(
                                state,
                                route,
                                PageFindDirection::Previous,
                                match_count,
                            );
                        });
                    },
                    svg {
                        class: "page-find-button-icon",
                        view_box: "0 0 20 20",
                        "aria-hidden": "true",
                        path {
                            d: "M12.5 5L7.5 10L12.5 15",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2.2",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                        }
                    }
                }
                if !counter.is_empty() {
                    span { class: "page-find-count", aria_live: "polite", "{counter}" }
                }
                button {
                    class: "page-find-button page-find-next",
                    r#type: "button",
                    aria_label: "Next page find match",
                    title: "Next",
                    disabled: controls_disabled,
                    onclick: move |_| {
                        page_find_state.with_mut(|state| {
                            update_page_find_active(
                                state,
                                route,
                                PageFindDirection::Next,
                                match_count,
                            );
                        });
                    },
                    svg {
                        class: "page-find-button-icon",
                        view_box: "0 0 20 20",
                        "aria-hidden": "true",
                        path {
                            d: "M7.5 5L12.5 10L7.5 15",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2.2",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn page_find_placeholder(route: AppRoute) -> &'static str {
    match route {
        AppRoute::Cukta => "Find in section",
        AppRoute::Vlacku => "Find in cards",
        AppRoute::Gentufa => "Find in output",
        AppRoute::Settings => "Find in settings",
    }
}

#[requires(true)]
#[ensures(true)]
fn page_find_counter_text(
    active_index: Option<usize>,
    match_count: usize,
    query_present: bool,
) -> String {
    if !query_present {
        String::new()
    } else if match_count == 0 {
        "0/0".to_owned()
    } else {
        let current = active_index.map_or(1, |index| index + 1);
        format!("{current}/{match_count}")
    }
}

#[requires(true)]
#[ensures(true)]
fn render_topbar_settings_button(
    settings: Signal<UserSettings>,
    current: UserSettings,
    settings_route: JbotciRoute,
    settings_layout: TopbarSettingsLayout,
    mut settings_open: Signal<bool>,
) -> Element {
    let menu_open = *settings_open.read() && settings_layout.uses_popout();
    let button_class = topbar_settings_button_class(menu_open);
    rsx! {
        div { class: "app-topbar-settings",
            if settings_layout.uses_popout() {
                button {
                    class: "{button_class}",
                    r#type: "button",
                    aria_label: "Settings",
                    aria_expanded: if menu_open { "true" } else { "false" },
                    aria_controls: "app-topbar-settings-menu",
                    title: "Settings",
                    onclick: move |_| settings_open.set(!menu_open),
                    span { class: "app-topbar-settings-icon", aria_hidden: "true", "⚙" }
                }
                if menu_open {
                    { render_topbar_settings_menu(settings, current, settings_route, settings_layout) }
                }
            } else {
                Link {
                    class: "{button_class}",
                    to: settings_route,
                    aria_label: "Settings",
                    title: "Settings",
                    span { class: "app-topbar-settings-icon", aria_hidden: "true", "⚙" }
                }
            }
        }
    }
}

#[requires(settings_layout.uses_popout())]
#[ensures(true)]
fn render_topbar_settings_menu(
    settings: Signal<UserSettings>,
    current: UserSettings,
    settings_route: JbotciRoute,
    settings_layout: TopbarSettingsLayout,
) -> Element {
    rsx! {
        div {
            id: "app-topbar-settings-menu",
            class: "app-topbar-settings-menu",
            role: "dialog",
            aria_label: "Settings",
            if !settings_layout.shows_theme_inline() {
                div { class: "app-topbar-settings-menu-row",
                    { render_theme_switch(settings, current.theme) }
                }
            }
            if !settings_layout.shows_script_inline() {
                div { class: "app-topbar-settings-menu-row",
                    { render_script_switch(settings, current.script) }
                }
            }
            div { class: "app-topbar-settings-menu-row",
                Link {
                    class: "app-topbar-settings-all",
                    to: settings_route,
                    "All settings"
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(true)]
#[ensures(true)]
fn render_topbar_fit_probes(
    settings: Signal<UserSettings>,
    current: UserSettings,
    route: AppRoute,
    cukta_loading: bool,
    vlacku_loading: bool,
    gentufa_loading: bool,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gentufa_route: JbotciRoute,
    base_path: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
) -> Element {
    rsx! {
        div {
            class: "app-topbar-fit-probes",
            aria_hidden: "true",
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-both-full",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                span { class: "app-topbar-theme app-topbar-theme-mode",
                    { render_theme_switch(settings, current.theme) }
                }
                span { class: "app-topbar-theme app-topbar-orthography",
                    { render_script_switch(settings, current.script) }
                }
                { render_topbar_nav(route, cukta_loading, vlacku_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-theme-full",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                span { class: "app-topbar-theme app-topbar-theme-mode",
                    { render_theme_switch(settings, current.theme) }
                }
                { render_topbar_nav(route, cukta_loading, vlacku_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-none-full",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                { render_topbar_nav(route, cukta_loading, vlacku_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-both-carousel",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                span { class: "app-topbar-theme app-topbar-theme-mode",
                    { render_theme_switch(settings, current.theme) }
                }
                span { class: "app-topbar-theme app-topbar-orthography",
                    { render_script_switch(settings, current.script) }
                }
                { render_topbar_nav_carousel_probe(route, cukta_loading, vlacku_loading, gentufa_loading) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-theme-carousel",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                span { class: "app-topbar-theme app-topbar-theme-mode",
                    { render_theme_switch(settings, current.theme) }
                }
                { render_topbar_nav_carousel_probe(route, cukta_loading, vlacku_loading, gentufa_loading) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-none-carousel",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                { render_topbar_nav_carousel_probe(route, cukta_loading, vlacku_loading, gentufa_loading) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_topbar_probe_brand() -> Element {
    rsx! {
        span { class: "app-topbar-brand app-topbar-brand-probe",
            img { class: "app-topbar-brand-logo", src: LOGO, alt: "" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_topbar_probe_settings_button() -> Element {
    rsx! {
        span { class: "app-topbar-settings",
            span { class: "app-topbar-settings-toggle", aria_hidden: "true",
                span { class: "app-topbar-settings-icon", "⚙" }
            }
        }
    }
}

#[invariant(!short.is_empty())]
#[invariant(!href.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct BuildCommitInfo {
    short: String,
    href: String,
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|commit| !commit.href.is_empty()))]
fn build_commit_info() -> Option<BuildCommitInfo> {
    let Some(full_commit) = BUILD_GIT_COMMIT else {
        return None;
    };
    let Some(short_commit) = BUILD_GIT_COMMIT_SHORT else {
        return None;
    };
    Some(new!(BuildCommitInfo {
        short: short_commit.to_owned(),
        href: format!("https://codeberg.org/int_19h/jbotci/commit/{full_commit}"),
    }))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn logo_title_text() -> String {
    build_commit_info()
        .map(|commit| format!("jbotci #{}", commit.short))
        .unwrap_or_else(|| "jbotci".to_owned())
}

#[requires(true)]
#[ensures(true)]
fn render_settings_commit_link(page_find: &PageFindContext) -> Element {
    let Some(commit) = build_commit_info() else {
        return rsx! {};
    };
    let label = format!("commit {}", commit.short);
    rsx! {
        a {
            class: "settings-commit-link",
            href: "{commit.href}",
            title: "Git commit from which this version of jbotci was built.",
            aria_label: "Build commit {commit.short}",
            { render_page_find_text(page_find, &label) }
        }
    }
}

#[requires(commit.chars().all(|character| character.is_ascii_hexdigit()))]
#[ensures(ret.chars().count() == commit.chars().count())]
fn math_monospace_git_commit(commit: &str) -> String {
    commit.chars().map(math_monospace_hex_char).collect()
}

#[requires(character.is_ascii_hexdigit())]
#[ensures(true)]
fn math_monospace_hex_char(character: char) -> char {
    const DIGITS: [char; 10] = ['𝟶', '𝟷', '𝟸', '𝟹', '𝟺', '𝟻', '𝟼', '𝟽', '𝟾', '𝟿'];
    const HEX_LETTERS: [char; 6] = ['𝚊', '𝚋', '𝚌', '𝚍', '𝚎', '𝚏'];
    if character.is_ascii_digit() {
        DIGITS[(character as u8 - b'0') as usize]
    } else {
        HEX_LETTERS[(character.to_ascii_lowercase() as u8 - b'a') as usize]
    }
}

impl TopbarSettingsLayout {
    #[requires(true)]
    #[ensures(true)]
    fn shows_theme_inline(self) -> bool {
        matches!(self, Self::BothInline | Self::ThemeInline)
    }

    #[requires(true)]
    #[ensures(true)]
    fn shows_script_inline(self) -> bool {
        matches!(self, Self::BothInline)
    }

    #[requires(true)]
    #[ensures(ret == !self.shows_script_inline())]
    fn uses_popout(self) -> bool {
        !self.shows_script_inline()
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn topbar_header_class(
    settings_layout: TopbarSettingsLayout,
    settings_open: bool,
    nav_layout: TopbarNavLayout,
) -> String {
    format!(
        "app-topbar spa-topbar {} {}{}",
        match settings_layout {
            TopbarSettingsLayout::BothInline => "topbar-settings-both-inline",
            TopbarSettingsLayout::ThemeInline => "topbar-settings-theme-inline",
            TopbarSettingsLayout::NoneInline => "topbar-settings-none-inline",
        },
        match nav_layout {
            TopbarNavLayout::Full => "topbar-nav-full",
            TopbarNavLayout::Carousel => "topbar-nav-carousel",
        },
        if settings_open && settings_layout.uses_popout() {
            " topbar-settings-open"
        } else {
            ""
        }
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn topbar_settings_button_class(open: bool) -> &'static str {
    if open {
        "app-topbar-settings-toggle is-open"
    } else {
        "app-topbar-settings-toggle"
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn vlacku_jvozba_available() -> bool {
    web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|width| width.as_f64())
        .map_or(true, |width| width >= VLACKU_JVOZBA_MIN_WIDTH_PX)
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret)]
fn vlacku_jvozba_available() -> bool {
    true
}

#[requires(true)]
#[ensures(true)]
fn update_vlacku_jvozba_availability(mut available: Signal<bool>) {
    let next = vlacku_jvozba_available();
    if *available.read() != next {
        available.set(next);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn cukta_toc_forced_autohide_active() -> bool {
    web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|width| width.as_f64())
        .map_or(false, |width| width <= CUKTA_TOC_FORCED_AUTOHIDE_WIDTH_PX)
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(!ret)]
fn cukta_toc_forced_autohide_active() -> bool {
    false
}

#[requires(true)]
#[ensures(true)]
fn update_cukta_toc_forced_autohide(mut forced_autohide: Signal<bool>) {
    let next = cukta_toc_forced_autohide_active();
    if *forced_autohide.read() != next {
        forced_autohide.set(next);
    }
}

#[requires(true)]
#[ensures(true)]
fn update_topbar_layout(
    mut settings_layout: Signal<TopbarSettingsLayout>,
    mut settings_open: Signal<bool>,
    mut nav_layout: Signal<TopbarNavLayout>,
    next_layout: TopbarLayout,
) {
    if *settings_layout.read() != next_layout.settings {
        settings_layout.set(next_layout.settings);
    }
    if *nav_layout.read() != next_layout.nav {
        nav_layout.set(next_layout.nav);
    }
    if next_layout.settings == TopbarSettingsLayout::BothInline && *settings_open.read() {
        settings_open.set(false);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_topbar_settings_layout_measure(
    settings_layout: Signal<TopbarSettingsLayout>,
    settings_open: Signal<bool>,
    nav_layout: Signal<TopbarNavLayout>,
) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move || {
        update_topbar_layout(
            settings_layout,
            settings_open,
            nav_layout,
            measure_topbar_settings_layout(),
        );
    });
    let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
    closure.forget();
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_topbar_active_nav_sync() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move || {
        scroll_active_topbar_nav_into_view();
    });
    let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
    closure.forget();
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn schedule_topbar_active_nav_sync() {
    spawn(async move {
        sleep_ms(0).await;
        let _ = document::eval(
            r#"
            const active = document.querySelector('.app-topbar-nav-carousel-track [data-topbar-nav-active="true"]');
            if (active) {
                active.scrollIntoView({ block: "nearest", inline: "center" });
            }
            return null;
            "#,
        )
        .await;
    });
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
fn schedule_topbar_active_nav_sync() {}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn schedule_topbar_settings_layout_measure(
    settings_layout: Signal<TopbarSettingsLayout>,
    settings_open: Signal<bool>,
    nav_layout: Signal<TopbarNavLayout>,
) {
    spawn(async move {
        let mut layout = None;
        for delay_ms in [0, 16, 64] {
            sleep_ms(delay_ms).await;
            layout = measure_topbar_settings_layout_desktop().await;
            if layout.is_some() {
                break;
            }
        }
        update_topbar_layout(
            settings_layout,
            settings_open,
            nav_layout,
            layout.unwrap_or(new!(TopbarLayout {
                settings: TopbarSettingsLayout::BothInline,
                nav: TopbarNavLayout::Full,
            })),
        );
    });
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
fn schedule_topbar_settings_layout_measure(
    settings_layout: Signal<TopbarSettingsLayout>,
    settings_open: Signal<bool>,
    nav_layout: Signal<TopbarNavLayout>,
) {
    update_topbar_layout(
        settings_layout,
        settings_open,
        nav_layout,
        new!(TopbarLayout {
            settings: TopbarSettingsLayout::BothInline,
            nav: TopbarNavLayout::Full,
        }),
    );
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_topbar_settings_layout_after_fonts_ready(
    document: &web_sys::Document,
    settings_layout: Signal<TopbarSettingsLayout>,
    settings_open: Signal<bool>,
    nav_layout: Signal<TopbarNavLayout>,
) {
    let Ok(fonts) = js_sys::Reflect::get(document.as_ref(), &JsValue::from_str("fonts")) else {
        return;
    };
    let Ok(ready) = js_sys::Reflect::get(&fonts, &JsValue::from_str("ready")) else {
        return;
    };
    let Ok(promise) = ready.dyn_into::<js_sys::Promise>() else {
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        schedule_topbar_settings_layout_measure(settings_layout, settings_open, nav_layout);
    });
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn measure_topbar_settings_layout() -> TopbarLayout {
    topbar_layout_from_probe_fits(|selector| topbar_probe_fits(selector))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[invariant(true)]
struct TopbarLayoutMetrics {
    available_width: f64,
    both_full_required_width: f64,
    theme_full_required_width: f64,
    none_full_required_width: f64,
    both_carousel_required_width: f64,
    theme_carousel_required_width: f64,
    none_carousel_required_width: f64,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn measure_topbar_settings_layout_desktop() -> Option<TopbarLayout> {
    let metrics: TopbarLayoutMetrics = document::eval(
        r#"
        const inner = document.querySelector(".app-topbar-inner");
        const stylesReady = () => {
            const shell = document.querySelector(".spa-shell.app-page");
            if (!shell) {
                return false;
            }
            const shellStyle = window.getComputedStyle(shell);
            return String(shellStyle.getPropertyValue("--topbar-bg") || "").trim().length > 0;
        };
        if (!stylesReady()) {
            return null;
        }
        const widthFor = (parent, selector) => {
            const element = parent && parent.querySelector(selector);
            if (!element) {
                return 0;
            }
            const style = window.getComputedStyle(element);
            if (style.display === "none" || style.visibility === "hidden") {
                return 0;
            }
            const rect = element.getBoundingClientRect();
            return Math.max(Number(element.scrollWidth || 0), rect.width);
        };
        const centerWidthFor = (parent) => {
            const center = parent && parent.querySelector(".app-topbar-center");
            if (!center) {
                return 0;
            }
            const style = window.getComputedStyle(center);
            if (style.display === "none" || style.visibility === "hidden") {
                return 0;
            }
            const dots = center.querySelector(".app-topbar-activity-dots");
            if (!dots) {
                return 0;
            }
            const rect = dots.getBoundingClientRect();
            return Math.max(Number(dots.scrollWidth || 0), rect.width);
        };
        const columnGapFor = (element) => {
            if (!element) {
                return 0;
            }
            const value = Number.parseFloat(window.getComputedStyle(element).columnGap || "0");
            return Number.isFinite(value) && value >= 0 ? value : 0;
        };
        const requiredFor = (selector) => {
            if (!inner) {
                return 0;
            }
            const probe = document.querySelector(selector);
            if (!probe) {
                return 0;
            }
            const probeRect = probe.getBoundingClientRect();
            const probeWidth = Math.max(Number(probe.scrollWidth || 0), probeRect.width);
            const centerWidth = centerWidthFor(inner);
            const rightWidth = widthFor(inner, ".app-topbar-right");
            const visibleColumns = 1 + (centerWidth > 0 ? 1 : 0) + (rightWidth > 0 ? 1 : 0);
            return probeWidth + centerWidth + rightWidth + (visibleColumns - 1) * columnGapFor(inner);
        };
        const availableWidth = inner ? inner.getBoundingClientRect().width : 0;
        const bothFullRequiredWidth = requiredFor(".app-topbar-fit-probe-both-full");
        const rightWidth = widthFor(inner, ".app-topbar-right");
        if (!inner || availableWidth <= 0 || bothFullRequiredWidth <= 0 || rightWidth <= 0) {
            return null;
        }
        return {
            available_width: availableWidth,
            both_full_required_width: bothFullRequiredWidth,
            theme_full_required_width: requiredFor(".app-topbar-fit-probe-theme-full"),
            none_full_required_width: requiredFor(".app-topbar-fit-probe-none-full"),
            both_carousel_required_width: requiredFor(".app-topbar-fit-probe-both-carousel"),
            theme_carousel_required_width: requiredFor(".app-topbar-fit-probe-theme-carousel"),
            none_carousel_required_width: requiredFor(".app-topbar-fit-probe-none-carousel"),
        };
        "#,
    )
    .join()
    .await
    .ok()?;
    Some(topbar_layout_from_metrics(metrics))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn topbar_layout_from_metrics(metrics: TopbarLayoutMetrics) -> TopbarLayout {
    topbar_layout_from_probe_fits(|selector| {
        let required_width = match selector {
            ".app-topbar-fit-probe-both-full" => metrics.both_full_required_width,
            ".app-topbar-fit-probe-theme-full" => metrics.theme_full_required_width,
            ".app-topbar-fit-probe-none-full" => metrics.none_full_required_width,
            ".app-topbar-fit-probe-both-carousel" => metrics.both_carousel_required_width,
            ".app-topbar-fit-probe-theme-carousel" => metrics.theme_carousel_required_width,
            ".app-topbar-fit-probe-none-carousel" => metrics.none_carousel_required_width,
            _ => metrics.none_carousel_required_width,
        };
        required_width <= metrics.available_width + 1.0
    })
}

#[requires(true)]
#[ensures(true)]
fn topbar_layout_from_probe_fits(fits: impl Fn(&str) -> bool) -> TopbarLayout {
    let candidates = [
        new!(TopbarLayout {
            settings: TopbarSettingsLayout::BothInline,
            nav: TopbarNavLayout::Full,
        }),
        new!(TopbarLayout {
            settings: TopbarSettingsLayout::ThemeInline,
            nav: TopbarNavLayout::Full,
        }),
        new!(TopbarLayout {
            settings: TopbarSettingsLayout::NoneInline,
            nav: TopbarNavLayout::Full,
        }),
        new!(TopbarLayout {
            settings: TopbarSettingsLayout::BothInline,
            nav: TopbarNavLayout::Carousel,
        }),
        new!(TopbarLayout {
            settings: TopbarSettingsLayout::ThemeInline,
            nav: TopbarNavLayout::Carousel,
        }),
        new!(TopbarLayout {
            settings: TopbarSettingsLayout::NoneInline,
            nav: TopbarNavLayout::Carousel,
        }),
    ];
    for candidate in candidates {
        if fits(topbar_layout_probe_selector(candidate)) {
            return candidate;
        }
    }
    new!(TopbarLayout {
        settings: TopbarSettingsLayout::NoneInline,
        nav: TopbarNavLayout::Carousel,
    })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn topbar_layout_probe_selector(layout: TopbarLayout) -> &'static str {
    match (layout.settings, layout.nav) {
        (TopbarSettingsLayout::BothInline, TopbarNavLayout::Full) => {
            ".app-topbar-fit-probe-both-full"
        }
        (TopbarSettingsLayout::ThemeInline, TopbarNavLayout::Full) => {
            ".app-topbar-fit-probe-theme-full"
        }
        (TopbarSettingsLayout::NoneInline, TopbarNavLayout::Full) => {
            ".app-topbar-fit-probe-none-full"
        }
        (TopbarSettingsLayout::BothInline, TopbarNavLayout::Carousel) => {
            ".app-topbar-fit-probe-both-carousel"
        }
        (TopbarSettingsLayout::ThemeInline, TopbarNavLayout::Carousel) => {
            ".app-topbar-fit-probe-theme-carousel"
        }
        (TopbarSettingsLayout::NoneInline, TopbarNavLayout::Carousel) => {
            ".app-topbar-fit-probe-none-carousel"
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(!selector.is_empty())]
#[ensures(true)]
fn topbar_probe_fits(selector: &str) -> bool {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return true;
    };
    if !topbar_styles_ready(&document) {
        return true;
    }
    let Some(inner) = document.query_selector(".app-topbar-inner").ok().flatten() else {
        return true;
    };
    let Some(probe) = document.query_selector(selector).ok().flatten() else {
        return true;
    };
    let available_width = inner.get_bounding_client_rect().width();
    let center_width = topbar_center_content_width(&inner);
    let right_width = topbar_visible_width(&inner, ".app-topbar-right");
    let visible_columns = 1.0
        + if center_width > 0.0 { 1.0 } else { 0.0 }
        + if right_width > 0.0 { 1.0 } else { 0.0 };
    let required_width = element_layout_width(&probe)
        + center_width
        + right_width
        + (visible_columns - 1.0) * topbar_column_gap(&inner);
    required_width <= available_width + 1.0
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn topbar_styles_ready(document: &web_sys::Document) -> bool {
    let Some(shell) = document
        .query_selector(".spa-shell.app-page")
        .ok()
        .flatten()
    else {
        return false;
    };
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Some(style) = window.get_computed_style(&shell).ok().flatten() else {
        return false;
    };
    style
        .get_property_value("--topbar-bg")
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0.0)]
fn topbar_center_content_width(parent: &web_sys::Element) -> f64 {
    let Some(center) = parent.query_selector(".app-topbar-center").ok().flatten() else {
        return 0.0;
    };
    let Some(window) = web_sys::window() else {
        return 0.0;
    };
    let Some(style) = window.get_computed_style(&center).ok().flatten() else {
        return 0.0;
    };
    if style.get_property_value("display").ok().as_deref() == Some("none")
        || style.get_property_value("visibility").ok().as_deref() == Some("hidden")
    {
        return 0.0;
    }
    center
        .query_selector(".app-topbar-activity-dots")
        .ok()
        .flatten()
        .map_or(0.0, |dots| element_layout_width(&dots))
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0.0)]
fn topbar_visible_width(parent: &web_sys::Element, selector: &str) -> f64 {
    let Some(element) = parent.query_selector(selector).ok().flatten() else {
        return 0.0;
    };
    let Some(window) = web_sys::window() else {
        return 0.0;
    };
    let Some(style) = window.get_computed_style(&element).ok().flatten() else {
        return 0.0;
    };
    if style.get_property_value("display").ok().as_deref() == Some("none")
        || style.get_property_value("visibility").ok().as_deref() == Some("hidden")
    {
        return 0.0;
    }
    element_layout_width(&element)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0.0)]
fn element_layout_width(element: &web_sys::Element) -> f64 {
    f64::from(element.scroll_width()).max(element.get_bounding_client_rect().width())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0.0)]
fn topbar_column_gap(element: &web_sys::Element) -> f64 {
    web_sys::window()
        .and_then(|window| window.get_computed_style(element).ok().flatten())
        .and_then(|style| style.get_property_value("column-gap").ok())
        .and_then(|value| value.trim_end_matches("px").parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(0.0)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_page_find_match_scroll(match_index: usize) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move || scroll_page_find_match(match_index));
    let _ = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), 0);
    closure.forget();
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn schedule_page_find_match_scroll(match_index: usize) {
    spawn(async move {
        sleep_ms(0).await;
        scroll_page_find_match_desktop(match_index).await;
    });
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
fn schedule_page_find_match_scroll(_match_index: usize) {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn scroll_page_find_match(match_index: usize) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let selector = format!(r#"[data-page-find-match-index="{match_index}"]"#);
    if let Ok(Some(element)) = document.query_selector(&selector) {
        element.scroll_into_view();
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn event_is_page_find_shortcut(event: &web_sys::Event) -> bool {
    let key = js_event_string_property(event, "key")
        .unwrap_or_default()
        .to_lowercase();
    let ctrl_key = js_event_bool_property(event, "ctrlKey");
    let meta_key = js_event_bool_property(event, "metaKey");
    let alt_key = js_event_bool_property(event, "altKey");
    (ctrl_key || meta_key) && !alt_key && key == "f"
}

#[cfg(target_arch = "wasm32")]
#[requires(!name.is_empty())]
#[ensures(true)]
fn js_event_string_property(event: &web_sys::Event, name: &str) -> Option<String> {
    js_sys::Reflect::get(event.as_ref(), &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_string())
}

#[cfg(target_arch = "wasm32")]
#[requires(!name.is_empty())]
#[ensures(true)]
fn js_event_bool_property(event: &web_sys::Event, name: &str) -> bool {
    js_sys::Reflect::get(event.as_ref(), &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn focus_page_find_input() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(element) = document.get_element_by_id(PAGE_FIND_INPUT_ID) else {
        return;
    };
    if let Ok(input) = element.dyn_into::<web_sys::HtmlInputElement>() {
        let _ = input.focus();
        input.select();
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn scroll_active_topbar_nav_into_view() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(Some(element)) = document
        .query_selector(r#".app-topbar-nav-carousel-track [data-topbar-nav-active="true"]"#)
    else {
        return;
    };
    let options = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        options.as_ref(),
        &JsValue::from_str("block"),
        &JsValue::from_str("nearest"),
    );
    let _ = js_sys::Reflect::set(
        options.as_ref(),
        &JsValue::from_str("inline"),
        &JsValue::from_str("center"),
    );
    if let Ok(function) =
        js_sys::Reflect::get(element.as_ref(), &JsValue::from_str("scrollIntoView"))
            .and_then(|value| value.dyn_into::<js_sys::Function>())
    {
        let _ = function.call1(element.as_ref(), options.as_ref());
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn scroll_page_find_match_desktop(match_index: usize) {
    let script = format!(
        r#"
        const element = document.querySelector('[data-page-find-match-index="{match_index}"]');
        if (element) {{
            element.scrollIntoView({{ block: "center", inline: "nearest" }});
        }}
        return null;
        "#
    );
    let _ = document::eval(&script).await;
}

#[requires(true)]
#[ensures(true)]
fn render_theme_switch(mut settings: Signal<UserSettings>, current: ThemeMode) -> Element {
    rsx! {
        div { class: "theme-switch", aria_label: "Theme mode", role: "group",
            button {
                class: theme_button_class(current == ThemeMode::Auto),
                r#type: "button",
                aria_label: "Use system theme",
                aria_pressed: pressed_attr(current == ThemeMode::Auto),
                onclick: move |_| set_theme(&mut settings, ThemeMode::Auto),
                "◐"
            }
            button {
                class: theme_button_class(current == ThemeMode::Day),
                r#type: "button",
                aria_label: "Use light theme",
                aria_pressed: pressed_attr(current == ThemeMode::Day),
                onclick: move |_| set_theme(&mut settings, ThemeMode::Day),
                "☀"
            }
            button {
                class: theme_button_class(current == ThemeMode::Night),
                r#type: "button",
                aria_label: "Use dark theme",
                aria_pressed: pressed_attr(current == ThemeMode::Night),
                onclick: move |_| set_theme(&mut settings, ThemeMode::Night),
                "☾"
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_script_switch(mut settings: Signal<UserSettings>, current: GentufaScript) -> Element {
    rsx! {
        div {
            class: "theme-switch orthography-switch",
            aria_label: "Orthography",
            role: "group",
            title: "Orthography icons: j = latin, ж = cyrillic,  = zbalermorna",
            button {
                class: orthography_button_class(current == GentufaScript::Latin, false),
                r#type: "button",
                aria_label: "Latin orthography",
                aria_pressed: pressed_attr(current == GentufaScript::Latin),
                onclick: move |_| set_script(&mut settings, GentufaScript::Latin),
                span { class: "orthography-btn-icon", "j" }
            }
            button {
                class: orthography_button_class(current == GentufaScript::Cyrillic, false),
                r#type: "button",
                aria_label: "Cyrillic orthography",
                aria_pressed: pressed_attr(current == GentufaScript::Cyrillic),
                onclick: move |_| set_script(&mut settings, GentufaScript::Cyrillic),
                span { class: "orthography-btn-icon", "ж" }
            }
            button {
                class: orthography_button_class(current == GentufaScript::Zbalermorna, true),
                r#type: "button",
                aria_label: "Zbalermorna orthography",
                aria_pressed: pressed_attr(current == GentufaScript::Zbalermorna),
                onclick: move |_| set_script(&mut settings, GentufaScript::Zbalermorna),
                span { class: "orthography-btn-icon", "" }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_dialect_control(
    mut dialect: Signal<String>,
    dialect_settings: DialectSettings,
    mut picker_open: Signal<bool>,
) -> Element {
    let formula_text = dialect.read().clone();
    let picker_is_open = *picker_open.read();
    let picker_names = gentufa_picker_dialect_names(&dialect_settings);
    let selected_references = dialect_formula_top_level_references(&formula_text)
        .into_iter()
        .collect::<BTreeSet<_>>();
    rsx! {
        div { class: "gentufa-dialect-control",
            button {
                class: "gentufa-dialect-label",
                r#type: "button",
                aria_expanded: if picker_is_open { "true" } else { "false" },
                onclick: move |_| {
                    let next = !*picker_open.read();
                    picker_open.set(next);
                },
                "Dialect:"
            }
            div { class: "gentufa-dialect-input-shell",
                div { class: "gentufa-dialect-formula-wrap",
                    pre {
                        class: "settings-dialect-definition-highlight gentufa-dialect-formula-highlight",
                        aria_hidden: "true",
                        { render_dialect_highlight(&formula_text) }
                    }
                    textarea {
                        class: "settings-text-input settings-dialect-definition gentufa-dialect-formula-input",
                        rows: "1",
                        value: "{formula_text}",
                        placeholder: "baseline (CLL + xorlo + LTR-magic)",
                        spellcheck: "false",
                        aria_label: "Dialect formula",
                        oninput: move |event| {
                            dialect.set(event.value());
                        },
                    }
                }
                if picker_is_open {
                    div { class: "gentufa-dialect-picker",
                        for name in picker_names.iter() {
                            {
                                let item_name = name.clone();
                                let checked = selected_references.contains(name);
                                rsx! {
                                    label { class: "gentufa-dialect-picker-row",
                                        input {
                                            r#type: "checkbox",
                                            checked,
                                            onchange: move |_| {
                                                let current = dialect.read().clone();
                                                let next = if checked {
                                                    remove_dialect_formula_reference(&item_name, &current)
                                                } else {
                                                    add_dialect_formula_reference(&item_name, &current)
                                                };
                                                dialect.set(next);
                                            },
                                        }
                                        span { "{name}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn gentufa_picker_dialect_names(settings: &DialectSettings) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut names = Vec::new();
    for name in builtin_dialect_names() {
        if builtin_dialect_shows_in_gentufa(settings, name) && seen.insert(name.to_owned()) {
            names.push(name.to_owned());
        }
    }
    for custom in &settings.custom_dialects {
        let name = custom.name.trim();
        if custom.show_in_gentufa
            && dialect_name_shows_in_gentufa_picker(name)
            && seen.insert(name.to_owned())
        {
            names.push(name.to_owned());
        }
    }
    names
}

#[requires(true)]
#[ensures(true)]
fn render_dialect_highlight(text: &str) -> Element {
    let tokens = dialect_highlight_tokens(text);
    rsx! {
        for token in tokens.iter() {
            span { class: "{token.class_name}", "{token.text}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn dialect_highlight_tokens(text: &str) -> Vec<DialectHighlightToken> {
    let mut tokens = Vec::new();
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        let character = chars[index];
        if character.is_whitespace() {
            let start = index;
            while chars.get(index).is_some_and(|value| value.is_whitespace()) {
                index += 1;
            }
            tokens.push(dialect_highlight_token(
                "dialect-token-space",
                chars[start..index].iter().collect(),
            ));
        } else if matches!(character, '(' | ')') {
            tokens.push(dialect_highlight_token(
                "dialect-token-paren",
                character.to_string(),
            ));
            index += 1;
        } else {
            let start = index;
            while chars
                .get(index)
                .is_some_and(|value| !value.is_whitespace() && !matches!(*value, '(' | ')'))
            {
                index += 1;
            }
            let token_text = chars[start..index].iter().collect::<String>();
            let class_name = dialect_highlight_class(&token_text);
            tokens.push(dialect_highlight_token(class_name, token_text));
        }
    }
    if tokens.is_empty() {
        tokens.push(dialect_highlight_token(
            "dialect-token-empty",
            String::new(),
        ));
    }
    tokens
}

#[requires(!class_name.is_empty())]
#[ensures(ret.class_name == class_name)]
fn dialect_highlight_token(class_name: &str, text: String) -> DialectHighlightToken {
    DialectHighlightToken {
        class_name: class_name.to_owned(),
        text,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn dialect_highlight_class(token: &str) -> &'static str {
    if token.starts_with('+') || token.starts_with('-') {
        "dialect-token-feature"
    } else if token == "↦" || token == "->" || token == "↔" || token == "<->" || token == "🣐"
    {
        "dialect-token-operator"
    } else if find_builtin_dialect(token).is_some() {
        "dialect-token-reference"
    } else {
        "dialect-token-word"
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = r#"
export function jbotciCopyTextToClipboard(text) {
  const value = String(text ?? "");
  const fallback = () => {
    const textarea = document.createElement("textarea");
    textarea.value = value;
    textarea.setAttribute("readonly", "");
    textarea.style.position = "fixed";
    textarea.style.left = "-10000px";
    textarea.style.top = "0";
    document.body.appendChild(textarea);
    textarea.select();
    try {
      document.execCommand("copy");
    } finally {
      textarea.remove();
    }
  };

  if (navigator.clipboard?.writeText) {
    navigator.clipboard.writeText(value).catch(fallback);
  } else {
    fallback();
  }
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = jbotciCopyTextToClipboard)]
    fn js_copy_text_to_clipboard(text: &str);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(module = "/assets/embeddings.js")]
extern "C" {
    #[wasm_bindgen(js_name = jbotciEmbeddingConfigureWorker)]
    fn js_embedding_configure_worker(worker_url: &str);

    #[wasm_bindgen(js_name = jbotciEmbeddingConfigureOrtAssets)]
    fn js_embedding_configure_ort_assets(module_url: &str, wasm_mjs_url: &str, wasm_url: &str);

    #[wasm_bindgen(js_name = jbotciEmbeddingConfigureRemoteBase)]
    fn js_embedding_configure_remote_base(remote_base_url: &str);

    #[wasm_bindgen(js_name = jbotciEmbeddingConfigureModel)]
    fn js_embedding_configure_model(model_key: &str);

    #[wasm_bindgen(js_name = jbotciEmbeddingPreferredModelKey)]
    fn js_embedding_preferred_model_key() -> String;

    #[wasm_bindgen(js_name = jbotciEmbeddingStatus)]
    fn js_embedding_status() -> js_sys::Promise;

    #[wasm_bindgen(js_name = jbotciEmbeddingSetup)]
    fn js_embedding_setup(corpus_json: &str, remote_base_url: &str) -> js_sys::Promise;

    #[wasm_bindgen(js_name = jbotciEmbeddingRemove)]
    fn js_embedding_remove() -> js_sys::Promise;

    #[wasm_bindgen(js_name = jbotciEmbeddingCancel)]
    fn js_embedding_cancel(channel: &str);

    #[wasm_bindgen(js_name = jbotciEmbeddingSearch)]
    fn js_embedding_search(
        channel: &str,
        corpus_id: &str,
        query: &str,
        limit: usize,
        kind_filters_json: &str,
    ) -> js_sys::Promise;
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(module = "/assets/worker-client.js")]
extern "C" {
    #[wasm_bindgen(js_name = jbotciWorkerClientAssetPin)]
    fn js_worker_client_asset_pin();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(module = "/assets/compute.js")]
extern "C" {
    #[wasm_bindgen(js_name = jbotciComputeConfigureWorker)]
    fn js_compute_configure_worker(worker_url: &str);

    #[wasm_bindgen(js_name = jbotciComputeCancel)]
    fn js_compute_cancel(channel: &str);

    #[wasm_bindgen(js_name = jbotciComputeRequest)]
    fn js_compute_request(channel: &str, request_json: &str) -> js_sys::Promise;
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = jbotciComputeHandle)]
#[requires(true)]
#[ensures(true)]
pub fn jbotci_compute_handle(request_json: &str) -> Result<String, JsValue> {
    web_compute_handle(request_json).map_err(|error| JsValue::from_str(&error))
}

#[requires(!request_json.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|json| !json.is_empty()) || ret.is_err())]
fn web_compute_handle(request_json: &str) -> Result<String, String> {
    jbotci_web_core::run_web_compute_request_json(request_json).map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
#[requires(!worker_url.is_empty())]
#[ensures(true)]
fn configure_embedding_worker_url(worker_url: &str) {
    js_embedding_configure_worker(worker_url);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!worker_url.is_empty())]
#[ensures(true)]
fn configure_embedding_worker_url(worker_url: &str) {
    let _ = worker_url;
}

#[cfg(target_arch = "wasm32")]
#[requires(!module_url.is_empty())]
#[requires(!wasm_mjs_url.is_empty())]
#[requires(!wasm_url.is_empty())]
#[ensures(true)]
fn configure_embedding_ort_assets(module_url: &str, wasm_mjs_url: &str, wasm_url: &str) {
    js_embedding_configure_ort_assets(module_url, wasm_mjs_url, wasm_url);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!module_url.is_empty())]
#[requires(!wasm_mjs_url.is_empty())]
#[requires(!wasm_url.is_empty())]
#[ensures(true)]
fn configure_embedding_ort_assets(module_url: &str, wasm_mjs_url: &str, wasm_url: &str) {
    let _ = (module_url, wasm_mjs_url, wasm_url);
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn web_embeddings_base_url() -> &'static str {
    match BUILD_WEB_EMBEDDINGS_BASE_URL {
        Some(base_url) if !base_url.trim().is_empty() => base_url.trim(),
        _ => DEFAULT_WEB_EMBEDDINGS_BASE_URL,
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(!remote_base_url.is_empty())]
#[ensures(true)]
fn configure_embedding_remote_base_url(remote_base_url: &str) {
    js_embedding_configure_remote_base(remote_base_url);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!remote_base_url.is_empty())]
#[ensures(true)]
fn configure_embedding_remote_base_url(remote_base_url: &str) {
    let _ = remote_base_url;
}

#[cfg(target_arch = "wasm32")]
#[requires(is_supported_embedding_model_key(model_key))]
#[ensures(true)]
fn configure_embedding_model_key(model_key: &str) {
    js_embedding_configure_model(model_key);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(is_supported_embedding_model_key(model_key))]
#[ensures(true)]
fn configure_embedding_model_key(model_key: &str) {
    let _ = model_key;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn pin_worker_client_asset() {
    js_worker_client_asset_pin();
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn pin_worker_client_asset() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(is_supported_embedding_model_key(&ret))]
fn preferred_embedding_model_key() -> String {
    let key = js_embedding_preferred_model_key();
    if is_supported_embedding_model_key(&key) {
        key
    } else {
        F2LLM_330M_MODEL_KEY.to_owned()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(is_supported_embedding_model_key(&ret))]
fn preferred_embedding_model_key() -> String {
    F2LLM_NATIVE_330M_MODEL_KEY.to_owned()
}

#[cfg(target_arch = "wasm32")]
#[requires(!worker_url.is_empty())]
#[ensures(true)]
fn configure_compute_worker_url(worker_url: &str) {
    js_compute_configure_worker(worker_url);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!worker_url.is_empty())]
#[ensures(true)]
fn configure_compute_worker_url(worker_url: &str) {
    let _ = worker_url;
}

#[requires(true)]
#[ensures(true)]
async fn refresh_embedding_settings(mut settings: Signal<EmbeddingSettingsState>) {
    configure_embedding_model_key(&settings.read().selected_model_key);
    match embedding_status_json().await {
        Ok(json) => settings.set(embedding_settings_from_json(&json, "Embeddings are ready.")),
        Err(error) => {
            let previous = settings.read().clone();
            settings.set(embedding_settings_error_state(
                &previous,
                "unavailable",
                error,
            ));
        }
    }
}

#[requires(true)]
#[ensures(true)]
async fn setup_embeddings(mut settings: Signal<EmbeddingSettingsState>) {
    configure_embedding_model_key(&settings.read().selected_model_key);
    let corpus_json = match embedding_setup_corpus_json().await {
        Ok(json) => json,
        Err(error) => {
            let previous = settings.read().clone();
            settings.set(embedding_settings_error_state(&previous, "error", error));
            return;
        }
    };
    match embedding_setup_json(&corpus_json).await {
        Ok(json) => settings.set(embedding_settings_from_json(&json, "Embeddings are ready.")),
        Err(error) => {
            let previous = settings.read().clone();
            settings.set(embedding_settings_error_state(&previous, "error", error));
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn embedding_setup_corpus_json() -> Result<String, String> {
    embedding_corpus_json_from_compute_worker().await
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|json| json.is_empty()) || ret.is_err())]
async fn embedding_setup_corpus_json() -> Result<String, String> {
    Ok(String::new())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn embedding_corpus_json_from_compute_worker() -> Result<String, String> {
    let response = compute_request(
        COMPUTE_CHANNEL_EMBEDDINGS,
        WebComputeRequest::EmbeddingCorpusJson,
    )
    .await?;
    let WebComputeResponse::EmbeddingCorpusJson { json } = response else {
        return Err("compute worker returned the wrong embedding corpus response".to_owned());
    };
    Ok(json)
}

#[requires(true)]
#[ensures(true)]
async fn poll_embedding_settings_while_busy(mut settings: Signal<EmbeddingSettingsState>) {
    loop {
        sleep_ms(350).await;
        if !settings.read().busy {
            break;
        }
        if let Ok(json) = embedding_status_json().await {
            let mut next = embedding_settings_from_json(&json, "Embeddings are being prepared.");
            next.busy = true;
            settings.set(next);
        }
    }
}

#[requires(true)]
#[ensures(true)]
async fn remove_embeddings(mut settings: Signal<EmbeddingSettingsState>) {
    configure_embedding_model_key(&settings.read().selected_model_key);
    match embedding_remove_json().await {
        Ok(json) => settings.set(embedding_settings_from_json(
            &json,
            "Embeddings were removed.",
        )),
        Err(error) => {
            let previous = settings.read().clone();
            settings.set(embedding_settings_error_state(&previous, "error", error));
        }
    }
}

#[requires(true)]
#[ensures(true)]
async fn load_vlacku_semantic_result(state: VlackuWebState) -> VlackuSemanticResultState {
    let limit = vlacku_semantic_worker_limit(&state);
    let normalized_state = normalize_vlacku_state(&state);
    match embedding_search_json(
        EMBEDDING_CHANNEL_VLACKU_SEMANTIC,
        "vlacku-en",
        &state.query,
        limit,
        &normalized_state.word_types,
    )
    .await
    {
        Ok(json) => {
            let (hits, message) = parse_vlacku_semantic_search_json(&json);
            VlackuSemanticResultState {
                state: Some(state),
                hits,
                message,
                loading: false,
            }
        }
        Err(error) => VlackuSemanticResultState {
            state: Some(state),
            hits: Vec::new(),
            message: Some(error),
            loading: false,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn spawn_vlacku_semantic_loading_message(
    mut result_signal: Signal<VlackuSemanticResultState>,
    state: VlackuWebState,
) {
    spawn(async move {
        sleep_ms(SEMANTIC_LOADING_MESSAGE_DELAY_MS).await;
        if embedding_status_is_loading_model().await {
            result_signal.with_mut(|current| {
                if current.loading && current.state.as_ref() == Some(&state) {
                    current.message = Some("Loading semantic search model.".to_owned());
                }
            });
        }
    });
}

#[requires(true)]
#[ensures(ret >= 1 && ret <= VLACKU_WEB_MAX_COUNT)]
fn vlacku_semantic_worker_limit(state: &VlackuWebState) -> usize {
    let normalized_state = normalize_vlacku_state(state);
    normalized_state
        .count
        .saturating_add(1)
        .min(VLACKU_WEB_MAX_COUNT)
}

#[requires(true)]
#[ensures(true)]
async fn load_cukta_semantic_result(state: CuktaWebSearchState) -> CuktaSemanticResultState {
    let limit = cukta_semantic_worker_limit(&state);
    let kind_filters = cukta_semantic_worker_kind_filters(&state);
    match embedding_search_json(
        EMBEDDING_CHANNEL_CUKTA_SEMANTIC,
        "cukta-cll",
        &state.query,
        limit,
        &kind_filters,
    )
    .await
    {
        Ok(json) => {
            let (hits, message) = parse_cukta_semantic_search_json(&json);
            CuktaSemanticResultState {
                state: Some(state),
                hits,
                message,
                loading: false,
            }
        }
        Err(error) => CuktaSemanticResultState {
            state: Some(state),
            hits: Vec::new(),
            message: Some(error),
            loading: false,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn spawn_cukta_semantic_loading_message(
    mut result_signal: Signal<CuktaSemanticResultState>,
    state: CuktaWebSearchState,
) {
    spawn(async move {
        sleep_ms(SEMANTIC_LOADING_MESSAGE_DELAY_MS).await;
        if embedding_status_is_loading_model().await {
            result_signal.with_mut(|current| {
                if current.loading && current.state.as_ref() == Some(&state) {
                    current.message = Some("Loading semantic search model.".to_owned());
                }
            });
        }
    });
}

#[requires(true)]
#[ensures(ret >= 1 && ret <= CUKTA_WEB_MAX_COUNT)]
fn cukta_semantic_worker_limit(state: &CuktaWebSearchState) -> usize {
    state
        .count
        .clamp(1, CUKTA_WEB_MAX_COUNT)
        .saturating_add(1)
        .min(CUKTA_WEB_MAX_COUNT)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn cukta_semantic_worker_kind_filters(state: &CuktaWebSearchState) -> Vec<String> {
    let mut filters = Vec::new();
    for target in &state.targets {
        match target.trim().to_ascii_lowercase().as_str() {
            "section" | "sections" => push_unique_filter(&mut filters, "section"),
            "paragraph" | "paragraphs" => push_unique_filter(&mut filters, "paragraph"),
            "example" | "examples" => push_unique_filter(&mut filters, "example"),
            _ => {}
        }
    }
    if filters.is_empty() {
        filters.extend(
            ["section", "paragraph", "example"]
                .into_iter()
                .map(str::to_owned),
        );
    }
    filters
}

#[requires(!filter.is_empty())]
#[ensures(filters.iter().any(|candidate| candidate == filter))]
fn push_unique_filter(filters: &mut Vec<String>, filter: &str) {
    if !filters.iter().any(|candidate| candidate == filter) {
        filters.push(filter.to_owned());
    }
}

#[requires(!message.is_empty())]
#[ensures(matches!(ret.page_kind, CuktaPageKind::Error { .. }))]
fn cukta_loading_page_data(message: &str) -> CuktaPageData {
    CuktaPageData {
        toc: Vec::new(),
        current_section_id: None,
        page_kind: CuktaPageKind::Error {
            message: message.to_owned(),
        },
    }
}

#[requires(!message.is_empty())]
#[ensures(ret.message.as_ref().is_some_and(|value| value == message))]
fn vlacku_loading_result(state: &VlackuWebState, message: &str) -> VlackuWebResult {
    VlackuWebResult {
        state: state.clone(),
        cards: Vec::new(),
        word_type_options: vlacku_word_type_options(&state.word_types),
        dictionary_info: None,
        has_more: false,
        message: Some(message.to_owned()),
        errors: Vec::new(),
    }
}

#[requires(!message.is_empty())]
#[ensures(ret.error.as_ref().is_some_and(|error| error == message))]
fn gentufa_async_error_state(
    state: GentufaWebState,
    request: GentufaWebRequest,
    message: &str,
) -> GentufaAsyncPageState {
    GentufaAsyncPageState {
        state: Some(state),
        request: Some(request),
        result: GentufaWebResult::Error(GentufaError {
            phase: None,
            message: message.to_owned(),
            diagnostics: Vec::new(),
        }),
        meta: None,
        loading: false,
        error: Some(message.to_owned()),
    }
}

#[requires(!message.is_empty())]
#[ensures(ret.error.as_ref().is_some_and(|error| error == message))]
fn cukta_async_error_state(state: CuktaWebState, message: &str) -> CuktaAsyncPageState {
    CuktaAsyncPageState {
        state: Some(state),
        page: cukta_loading_page_data(message),
        meta: None,
        loading: false,
        error: Some(message.to_owned()),
    }
}

#[requires(!message.is_empty())]
#[ensures(ret.error.as_ref().is_some_and(|error| error == message))]
fn vlacku_async_error_state(state: &VlackuWebState, message: &str) -> VlackuAsyncResultState {
    VlackuAsyncResultState {
        state: Some(state.clone()),
        result: vlacku_loading_result(state, message),
        meta: None,
        loading: false,
        error: Some(message.to_owned()),
    }
}

#[requires(true)]
#[ensures(!ret || state.mode == VlackuWebMode::Meaning)]
fn vlacku_semantic_result_is_pending(
    state: &VlackuWebState,
    semantic: &VlackuSemanticResultState,
) -> bool {
    state.mode == VlackuWebMode::Meaning
        && !state.query.trim().is_empty()
        && (semantic.state.as_ref() != Some(state) || semantic.loading)
}

#[requires(vlacku_semantic_result_is_pending(state, semantic))]
#[ensures(page.state.as_ref() == Some(state))]
#[ensures(page.loading)]
#[ensures(page.error.is_none())]
fn apply_vlacku_semantic_pending_page(
    page: &mut VlackuAsyncResultState,
    base_path: &str,
    state: &VlackuWebState,
    semantic: &VlackuSemanticResultState,
) -> PageMeta {
    let meta = build_page_meta(base_path, &WebRoute::Vlacku(state.clone()));
    page.state = Some(state.clone());
    page.meta = Some(meta.clone());
    page.loading = true;
    page.error = None;
    if semantic.state.as_ref() == Some(state)
        && let Some(message) = &semantic.message
    {
        page.result = vlacku_loading_result(state, message);
    }
    meta
}

#[requires(true)]
#[ensures(true)]
fn vlacku_compute_request(
    base_path: &str,
    state: &VlackuWebState,
    semantic: &VlackuSemanticResultState,
) -> WebComputeRequest {
    if state.mode != VlackuWebMode::Meaning {
        return WebComputeRequest::VlackuPage {
            base_path: base_path.to_owned(),
            state: state.clone(),
        };
    }
    let loading = vlacku_semantic_result_is_pending(state, semantic);
    let message = if semantic.state.as_ref() == Some(state) {
        semantic.message.clone()
    } else {
        None
    };
    let hits = if !loading && semantic.state.as_ref() == Some(state) {
        semantic.hits.clone()
    } else {
        Vec::new()
    };
    WebComputeRequest::VlackuSemanticPage {
        base_path: base_path.to_owned(),
        state: state.clone(),
        hits,
        message,
        loading,
    }
}

#[requires(true)]
#[ensures(true)]
fn cukta_compute_request(
    base_path: &str,
    state: &CuktaWebState,
    semantic: &CuktaSemanticResultState,
) -> WebComputeRequest {
    let CuktaWebView::Search(search_state) = &state.view else {
        return WebComputeRequest::CuktaPage {
            base_path: base_path.to_owned(),
            state: state.clone(),
        };
    };
    if search_state.mode != CuktaWebMode::Meaning {
        return WebComputeRequest::CuktaPage {
            base_path: base_path.to_owned(),
            state: state.clone(),
        };
    }
    let loading = !search_state.query.trim().is_empty()
        && (semantic.state.as_ref() != Some(search_state) || semantic.loading);
    let message = if semantic.state.as_ref() == Some(search_state) {
        semantic.message.clone()
    } else {
        None
    };
    let hits = if !loading && semantic.state.as_ref() == Some(search_state) {
        semantic.hits.clone()
    } else {
        Vec::new()
    };
    WebComputeRequest::CuktaSemanticPage {
        base_path: base_path.to_owned(),
        state: state.clone(),
        hits,
        message,
        loading,
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn embedding_status_json() -> Result<String, String> {
    promise_to_string(js_embedding_status()).await
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn embedding_status_json() -> Result<String, String> {
    run_native_task(native_embedding_status_json_result).await
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
async fn embedding_status_json() -> Result<String, String> {
    Err("Native embeddings are not available for this platform yet.".to_owned())
}

#[requires(true)]
#[ensures(true)]
async fn embedding_status_is_loading_model() -> bool {
    let Ok(json) = embedding_status_json().await else {
        return false;
    };
    let value = serde_json::from_str::<serde_json::Value>(&json).unwrap_or(serde_json::Value::Null);
    json_string(&value, "status").as_deref() == Some("loading-model")
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn embedding_setup_json(corpus_json: &str) -> Result<String, String> {
    promise_to_string(js_embedding_setup(corpus_json, web_embeddings_base_url())).await
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn embedding_setup_json(corpus_json: &str) -> Result<String, String> {
    let _ = corpus_json;
    let model_key = load_embedding_model_key();
    run_native_task(move || native_embedding_setup_json_result(model_key)).await
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
async fn embedding_setup_json(_corpus_json: &str) -> Result<String, String> {
    Err("Native embeddings are not available for this platform yet.".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn embedding_remove_json() -> Result<String, String> {
    promise_to_string(js_embedding_remove()).await
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn embedding_remove_json() -> Result<String, String> {
    let model_key = load_embedding_model_key();
    run_native_task(move || native_embedding_remove_json_result(model_key)).await
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
async fn embedding_remove_json() -> Result<String, String> {
    Err("Native embeddings are not available for this platform yet.".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn embedding_search_json(
    channel: &str,
    corpus_id: &str,
    query: &str,
    limit: usize,
    kind_filters: &[String],
) -> Result<String, String> {
    configure_embedding_model_key(&load_embedding_model_key());
    let kind_filters_json = serde_json::to_string(kind_filters).unwrap_or_else(|_| "[]".to_owned());
    promise_to_string(js_embedding_search(
        channel,
        corpus_id,
        query,
        limit,
        &kind_filters_json,
    ))
    .await
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn embedding_search_json(
    channel: &str,
    corpus_id: &str,
    query: &str,
    limit: usize,
    kind_filters: &[String],
) -> Result<String, String> {
    let _ = channel;
    let model_key = load_embedding_model_key();
    let corpus_id = corpus_id.to_owned();
    let query = query.to_owned();
    let kind_filters = kind_filters.to_owned();
    run_native_task(move || {
        native_embedding_search_json_result(&model_key, &corpus_id, &query, limit, &kind_filters)
    })
    .await
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
async fn embedding_search_json(
    _channel: &str,
    _corpus_id: &str,
    _query: &str,
    _limit: usize,
    _kind_filters: &[String],
) -> Result<String, String> {
    Err(SEMANTIC_SEARCH_SETUP_MESSAGE.to_owned())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
static NATIVE_EMBEDDING_SEARCH_WORKER: OnceLock<Mutex<Option<NativeEmbeddingSearchWorkerHandle>>> =
    OnceLock::new();

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
static NATIVE_EMBEDDING_SETUP_PROGRESS: OnceLock<Mutex<Option<jbotci_embeddings::SetupProgress>>> =
    OnceLock::new();

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|progress| !progress.kind.is_empty()))]
fn native_embedding_setup_progress() -> Option<jbotci_embeddings::SetupProgress> {
    NATIVE_EMBEDDING_SETUP_PROGRESS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|progress| progress.clone())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!progress.kind.is_empty())]
#[ensures(true)]
fn set_native_embedding_setup_progress(progress: jbotci_embeddings::SetupProgress) {
    if let Ok(mut stored) = NATIVE_EMBEDDING_SETUP_PROGRESS
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        *stored = Some(progress);
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn clear_native_embedding_setup_progress() {
    if let Ok(mut stored) = NATIVE_EMBEDDING_SETUP_PROGRESS
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        *stored = None;
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone)]
#[invariant(true)]
struct NativeEmbeddingSearchWorkerHandle {
    sender: std::sync::mpsc::Sender<NativeEmbeddingSearchCommand>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug)]
#[invariant(::Search { .. } => true)]
#[invariant(::Clear { .. } => true)]
enum NativeEmbeddingSearchCommand {
    Search {
        model_key: String,
        corpus_id: String,
        query: String,
        count: usize,
        kind_filters: Vec<String>,
        response: std::sync::mpsc::Sender<Result<String, String>>,
    },
    Clear {
        response: std::sync::mpsc::Sender<Result<(), String>>,
    },
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl NativeEmbeddingSearchWorkerHandle {
    #[requires(!model_key.is_empty())]
    #[requires(!corpus_id.is_empty())]
    #[requires(count > 0)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
    fn search(
        &self,
        model_key: &str,
        corpus_id: &str,
        query: &str,
        count: usize,
        kind_filters: &[String],
    ) -> Result<String, String> {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.sender
            .send(NativeEmbeddingSearchCommand::Search {
                model_key: model_key.to_owned(),
                corpus_id: corpus_id.to_owned(),
                query: query.to_owned(),
                count,
                kind_filters: kind_filters.to_owned(),
                response: sender,
            })
            .map_err(|_| "native embedding search worker is unavailable".to_owned())?;
        receiver
            .recv()
            .map_err(|_| "native embedding search worker stopped before replying".to_owned())?
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
    fn clear(&self) -> Result<(), String> {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.sender
            .send(NativeEmbeddingSearchCommand::Clear { response: sender })
            .map_err(|_| "native embedding search worker is unavailable".to_owned())?;
        receiver
            .recv()
            .map_err(|_| "native embedding search worker stopped before replying".to_owned())?
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn native_embedding_status_json_result() -> Result<String, String> {
    let model_key = load_embedding_model_key();
    let spec = jbotci_embeddings::model_spec(&model_key)
        .ok_or_else(|| format!("unsupported native embedding model `{model_key}`"))?;
    let model_root = jbotci_embeddings::default_model_root().map_err(|error| error.to_string())?;
    let index_root = jbotci_embeddings::default_index_root().map_err(|error| error.to_string())?;
    let model_path = jbotci_embeddings::model_file_path(&model_root, &spec);
    let model_bytes = std::fs::metadata(&model_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let model_present = model_path.is_file() && model_bytes == spec.native_size_bytes;
    let pack_result = jbotci_embeddings::load_latest_pack(&index_root, &model_key);
    let index_bytes = pack_result
        .as_ref()
        .ok()
        .and_then(|(pack_dir, _)| directory_size(pack_dir).ok())
        .unwrap_or(0);
    let setup_progress = native_embedding_setup_progress();
    let (status, detail) = if let Some(progress) = &setup_progress {
        ("preparing", progress.detail.clone())
    } else if !model_path.is_file() {
        (
            "missing-model",
            format!(
                "No native embedding model is installed at `{}`.",
                model_path.display()
            ),
        )
    } else if !model_present {
        (
            "invalid-model",
            format!(
                "The installed native embedding model has {} bytes; expected {}.",
                model_bytes, spec.native_size_bytes
            ),
        )
    } else if let Err(error) = &pack_result {
        ("missing-index", error.to_string())
    } else {
        (
            "ready",
            "Native embeddings are ready for semantic search.".to_owned(),
        )
    };
    let mut json = serde_json::json!({
        "selectedModelKey": model_key,
        "effectiveModelKey": spec.model_key,
        "modelKey": spec.model_key,
        "modelLabel": embedding_model_label(&model_key),
        "modelBytes": model_bytes,
        "modelDtype": "Q4_K_M",
        "modelDevice": "llama.cpp",
        "indexBytes": index_bytes,
        "status": status,
        "detail": detail,
    });
    if let Some(progress) = setup_progress
        && let Ok(progress_value) = serde_json::to_value(progress)
    {
        json["progress"] = progress_value;
    }
    Ok(json.to_string())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!model_key.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn native_embedding_setup_json_result(model_key: String) -> Result<String, String> {
    let options = jbotci_embeddings::SetupOptions {
        model_key,
        force: false,
        index_dir: None,
        model_dir: None,
        ..jbotci_embeddings::SetupOptions::default()
    };
    clear_native_embedding_setup_progress();
    let mut progress = |progress| {
        set_native_embedding_setup_progress(progress);
    };
    let setup_result =
        jbotci_embeddings::native::setup_embeddings_with_progress(&options, &mut progress);
    clear_native_embedding_setup_progress();
    setup_result.map_err(|error| error.to_string())?;
    native_clear_embedding_search_service()?;
    native_embedding_status_json_result()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!model_key.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn native_embedding_remove_json_result(model_key: String) -> Result<String, String> {
    native_clear_embedding_search_service()?;
    let Some(spec) = jbotci_embeddings::model_spec(&model_key) else {
        return Err(format!("unsupported native embedding model `{model_key}`"));
    };
    let model_root = jbotci_embeddings::default_model_root().map_err(|error| error.to_string())?;
    let model_path = jbotci_embeddings::model_file_path(&model_root, &spec);
    if let Some(model_dir) = model_path.parent() {
        remove_dir_if_exists(model_dir)?;
    }
    let index_root = jbotci_embeddings::default_index_root().map_err(|error| error.to_string())?;
    let model_index_dir = index_root
        .join(jbotci_embeddings::INDEX_BASE_VERSION)
        .join("models")
        .join(&model_key);
    remove_dir_if_exists(&model_index_dir)?;
    remove_model_from_native_catalog(&index_root, &model_key)?;
    native_embedding_status_json_result()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!model_key.is_empty())]
#[requires(!corpus_id.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn native_embedding_search_json_result(
    model_key: &str,
    corpus_id: &str,
    query: &str,
    limit: usize,
    kind_filters: &[String],
) -> Result<String, String> {
    if query.trim().is_empty() {
        return Ok(serde_json::json!({ "hits": [] }).to_string());
    }
    let count = limit.max(1);
    native_embedding_search_worker_handle()?.search(
        model_key,
        corpus_id,
        query,
        count,
        kind_filters,
    )
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(service.model_key() == model_key)]
#[requires(!model_key.is_empty())]
#[requires(!corpus_id.is_empty())]
#[requires(!query.trim().is_empty())]
#[requires(count > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn native_embedding_worker_search_json(
    service: &mut jbotci_embeddings::native::NativeEmbeddingSearchService,
    model_key: &str,
    corpus_id: &str,
    query: &str,
    count: usize,
    kind_filters: &[String],
) -> Result<String, String> {
    match corpus_id {
        jbotci_embeddings::VLACKU_CORPUS_ID => {
            native_embedding_vlacku_search_json(service, query, count)
        }
        jbotci_embeddings::CUKTA_CORPUS_ID => {
            native_embedding_cukta_search_json(service, query, count, kind_filters)
        }
        _ => Err(format!("unsupported semantic corpus `{corpus_id}`")),
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!query.trim().is_empty())]
#[requires(count > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn native_embedding_vlacku_search_json(
    service: &mut jbotci_embeddings::native::NativeEmbeddingSearchService,
    query: &str,
    count: usize,
) -> Result<String, String> {
    let hits = service
        .semantic_vlacku_hits(query, count)
        .map_err(native_embedding_search_setup_error)?
        .into_iter()
        .map(|hit| {
            serde_json::json!({
                "id": hit.entry_index,
                "score": hit.score,
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({ "hits": hits }).to_string())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!query.trim().is_empty())]
#[requires(count > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn native_embedding_cukta_search_json(
    service: &mut jbotci_embeddings::native::NativeEmbeddingSearchService,
    query: &str,
    count: usize,
    kind_filters: &[String],
) -> Result<String, String> {
    let site = embedded_cll_site().map_err(|error| error.to_string())?;
    let chunks = jbotci_cll::cll_search_all_chunks(site);
    let targets = native_cukta_target_filter(kind_filters);
    let output = service
        .semantic_cukta_output(chunks, query, count, targets)
        .map_err(native_embedding_search_setup_error)?;
    let hits = output
        .matches
        .into_iter()
        .map(|hit| {
            let chunk_index = chunks
                .iter()
                .position(|chunk| chunk == &hit.chunk)
                .ok_or_else(|| "native CLL semantic search returned an unknown chunk".to_owned())?;
            let score = hit.similarity.ok_or_else(|| {
                "native CLL semantic search returned a hit without similarity".to_owned()
            })?;
            Ok(serde_json::json!({
                "id": chunk_index,
                "score": score,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(serde_json::json!({
        "hits": hits,
        "message": output.message,
    })
    .to_string())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn native_embedding_search_worker_cell() -> &'static Mutex<Option<NativeEmbeddingSearchWorkerHandle>>
{
    NATIVE_EMBEDDING_SEARCH_WORKER.get_or_init(|| Mutex::new(None))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn native_embedding_search_worker_handle() -> Result<NativeEmbeddingSearchWorkerHandle, String> {
    let mut guard = native_embedding_search_worker_cell()
        .lock()
        .map_err(|_| "native embedding search worker lock was poisoned".to_owned())?;
    if let Some(handle) = guard.as_ref() {
        return Ok(handle.clone());
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("jbotci-native-embedding-search".to_owned())
        .spawn(move || native_embedding_search_worker_loop(receiver))
        .map_err(|error| format!("failed to spawn native embedding search worker: {error}"))?;
    let handle = NativeEmbeddingSearchWorkerHandle { sender };
    *guard = Some(handle.clone());
    Ok(handle)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn native_clear_embedding_search_service() -> Result<(), String> {
    native_embedding_search_worker_handle()?.clear()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn native_embedding_search_worker_loop(
    receiver: std::sync::mpsc::Receiver<NativeEmbeddingSearchCommand>,
) {
    let mut service: Option<jbotci_embeddings::native::NativeEmbeddingSearchService> = None;
    while let Ok(command) = receiver.recv() {
        match command {
            NativeEmbeddingSearchCommand::Search {
                model_key,
                corpus_id,
                query,
                count,
                kind_filters,
                response,
            } => {
                let result = native_embedding_search_worker_command(
                    &mut service,
                    &model_key,
                    &corpus_id,
                    &query,
                    count,
                    &kind_filters,
                );
                let _ = response.send(result);
            }
            NativeEmbeddingSearchCommand::Clear { response } => {
                service = None;
                let _ = response.send(Ok(()));
            }
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!model_key.is_empty())]
#[requires(!corpus_id.is_empty())]
#[requires(count > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn native_embedding_search_worker_command(
    service: &mut Option<jbotci_embeddings::native::NativeEmbeddingSearchService>,
    model_key: &str,
    corpus_id: &str,
    query: &str,
    count: usize,
    kind_filters: &[String],
) -> Result<String, String> {
    if service
        .as_ref()
        .is_none_or(|service| service.model_key() != model_key)
    {
        *service = Some(
            jbotci_embeddings::native::NativeEmbeddingSearchService::load(model_key, None, None)
                .map_err(native_embedding_search_setup_error)?,
        );
    }
    let service = service
        .as_mut()
        .ok_or_else(|| "native embedding search service was not initialized".to_owned())?;
    native_embedding_worker_search_json(service, model_key, corpus_id, query, count, kind_filters)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn native_cukta_target_filter(kind_filters: &[String]) -> jbotci_cll::CuktaTargetFilter {
    if kind_filters.is_empty() {
        return jbotci_cll::CuktaTargetFilter::default();
    }
    let sections = kind_filters
        .iter()
        .any(|filter| matches!(filter.trim(), "section" | "sections"));
    let paragraphs = kind_filters
        .iter()
        .any(|filter| matches!(filter.trim(), "paragraph" | "paragraphs"));
    let examples = kind_filters
        .iter()
        .any(|filter| matches!(filter.trim(), "example" | "examples"));
    if !sections && !paragraphs && !examples {
        return jbotci_cll::CuktaTargetFilter::default();
    }
    jbotci_cll::CuktaTargetFilter {
        sections,
        paragraphs,
        examples,
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(!ret.is_empty())]
fn native_embedding_search_setup_error(error: jbotci_embeddings::EmbeddingError) -> String {
    match error {
        jbotci_embeddings::EmbeddingError::MissingCompatiblePack { .. }
        | jbotci_embeddings::EmbeddingError::InvalidModel { .. } => {
            SEMANTIC_SEARCH_SETUP_MESSAGE.to_owned()
        }
        other => other.to_string(),
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn remove_model_from_native_catalog(index_root: &Path, model_key: &str) -> Result<(), String> {
    let catalog_path =
        jbotci_embeddings::catalog_path(index_root).map_err(|error| error.to_string())?;
    if !catalog_path.is_file() {
        return Ok(());
    }
    let bytes = std::fs::read(&catalog_path)
        .map_err(|error| format!("failed to read `{}`: {error}", catalog_path.display()))?;
    let mut catalog: jbotci_embeddings::EmbeddingCatalog = serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse `{}`: {error}", catalog_path.display()))?;
    catalog.models.retain(|model| model.model_key != model_key);
    let bytes = serde_json::to_vec_pretty(&catalog)
        .map_err(|error| format!("failed to serialize `{}`: {error}", catalog_path.display()))?;
    std::fs::write(&catalog_path, bytes)
        .map_err(|error| format!("failed to write `{}`: {error}", catalog_path.display()))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn remove_dir_if_exists(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(path)
        .map_err(|error| format!("failed to remove `{}`: {error}", path.display()))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn directory_size(path: &Path) -> Result<u64, String> {
    if !path.exists() {
        return Ok(0);
    }
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("failed to inspect `{}`: {error}", path.display()))?;
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    let mut total = 0u64;
    for entry in std::fs::read_dir(path)
        .map_err(|error| format!("failed to list `{}`: {error}", path.display()))?
    {
        let entry =
            entry.map_err(|error| format!("failed to read `{}` entry: {error}", path.display()))?;
        total = total.saturating_add(directory_size(&entry.path())?);
    }
    Ok(total)
}

#[cfg(target_arch = "wasm32")]
#[requires(!channel.is_empty())]
#[requires(!request_json.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn compute_request_json(channel: &str, request_json: &str) -> Result<String, String> {
    promise_to_string(js_compute_request(channel, request_json)).await
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!channel.is_empty())]
#[requires(!request_json.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn compute_request_json(channel: &str, request_json: &str) -> Result<String, String> {
    let _ = channel;
    jbotci_web_core::run_web_compute_request_json(request_json).map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
#[requires(!channel.is_empty())]
#[ensures(true)]
fn cancel_compute_channel(channel: &str) {
    js_compute_cancel(channel);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!channel.is_empty())]
#[ensures(true)]
fn cancel_compute_channel(channel: &str) {
    let _ = channel;
}

#[cfg(target_arch = "wasm32")]
#[requires(!channel.is_empty())]
#[ensures(true)]
fn cancel_embedding_channel(channel: &str) {
    js_embedding_cancel(channel);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!channel.is_empty())]
#[ensures(true)]
fn cancel_embedding_channel(channel: &str) {
    let _ = channel;
}

#[requires(!channel.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn compute_request(
    channel: &str,
    request: WebComputeRequest,
) -> Result<WebComputeResponse, String> {
    let request_json = serde_json::to_string(&request).map_err(|error| error.to_string())?;
    let response_json = compute_request_json(channel, &request_json).await?;
    serde_json::from_str(&response_json).map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn promise_to_string(promise: js_sys::Promise) -> Result<String, String> {
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(js_value_to_string)?;
    value
        .as_string()
        .ok_or_else(|| "embedding worker returned a non-string response".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[requires(milliseconds >= 0)]
#[ensures(true)]
async fn sleep_ms(milliseconds: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let Some(window) = web_sys::window() else {
            let _ = resolve.call0(&JsValue::NULL);
            return;
        };
        let resolve_now = resolve.clone();
        let closure = Closure::once(move || {
            let _ = resolve_now.call0(&JsValue::NULL);
        });
        if window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                milliseconds,
            )
            .is_ok()
        {
            closure.forget();
        } else {
            let _ = resolve.call0(&JsValue::NULL);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(milliseconds >= 0)]
#[ensures(true)]
async fn sleep_ms(milliseconds: i32) {
    let delay = u64::try_from(milliseconds).unwrap_or(0);
    let _ = run_native_task(move || {
        std::thread::sleep(std::time::Duration::from_millis(delay));
        Ok(())
    })
    .await;
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn run_native_task<T>(
    task: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String>
where
    T: Send + 'static,
{
    let (sender, receiver) = futures_channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = sender.send(task());
    });
    receiver
        .await
        .map_err(|_| "native task was cancelled before it completed".to_owned())?
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(!ret.is_empty())]
fn js_value_to_string(value: JsValue) -> String {
    value.as_string().unwrap_or_else(|| {
        js_sys::JSON::stringify(&value)
            .ok()
            .and_then(|text| text.as_string())
            .unwrap_or_else(|| "embedding worker request failed".to_owned())
    })
}

#[requires(true)]
#[ensures(!ret.status.is_empty())]
fn embedding_settings_from_json(json: &str, fallback_detail: &str) -> EmbeddingSettingsState {
    let value = serde_json::from_str::<serde_json::Value>(json).unwrap_or(serde_json::Value::Null);
    let mut selected_model_key = json_string(&value, "selectedModelKey")
        .filter(|key| is_supported_embedding_model_key(key))
        .unwrap_or_else(load_embedding_model_key);
    let effective_model_key = json_string(&value, "effectiveModelKey")
        .or_else(|| json_string(&value, "modelKey"))
        .filter(|key| is_supported_embedding_model_key(key))
        .unwrap_or_else(|| selected_model_key.clone());
    let webgpu_available = value
        .get("webGpuAvailable")
        .and_then(serde_json::Value::as_bool);
    if webgpu_available == Some(false) && selected_model_key != F2LLM_80M_MODEL_KEY {
        selected_model_key = F2LLM_80M_MODEL_KEY.to_owned();
        save_embedding_model_key(&selected_model_key);
        configure_embedding_model_key(&selected_model_key);
    }
    let selected_model_label = embedding_model_label(&selected_model_key).to_owned();
    let status = json_string(&value, "status").unwrap_or_else(|| "unknown".to_owned());
    let detail = json_string(&value, "detail")
        .or_else(|| json_string(&value, "message"))
        .unwrap_or_else(|| fallback_detail.to_owned());
    let model_size = value
        .get("modelBytes")
        .and_then(serde_json::Value::as_u64)
        .map(human_bytes)
        .unwrap_or_else(|| "unknown".to_owned());
    let model_runtime = match (
        json_string(&value, "modelDtype"),
        json_string(&value, "modelDevice"),
    ) {
        (Some(dtype), Some(device)) => Some(format!("{dtype}/{device}")),
        (Some(dtype), None) => Some(dtype),
        _ => None,
    };
    let model_size = match model_runtime {
        Some(runtime) if model_size != "unknown" => format!("{model_size} ({runtime})"),
        Some(runtime) => runtime,
        None => model_size,
    };
    let model_size = json_string(&value, "modelLabel")
        .filter(|label| !label.is_empty())
        .map(|label| format!("{label}, {model_size}"))
        .unwrap_or(model_size);
    let index_size = value
        .get("indexBytes")
        .and_then(serde_json::Value::as_u64)
        .map(human_bytes)
        .unwrap_or_else(|| "unknown".to_owned());
    let progress = value.get("progress");
    let progress_kind = progress
        .and_then(|progress| json_string(progress, "kind"))
        .filter(|kind| !kind.is_empty());
    let progress_label = progress
        .and_then(|progress| json_string(progress, "label"))
        .filter(|label| !label.is_empty());
    let progress_loaded = progress
        .and_then(|progress| progress.get("loaded"))
        .and_then(serde_json::Value::as_u64);
    let progress_total = progress
        .and_then(|progress| progress.get("total"))
        .and_then(serde_json::Value::as_u64);
    let progress_percent = progress
        .and_then(|progress| progress.get("percent"))
        .and_then(serde_json::Value::as_u64)
        .map(|percent| percent.min(100) as u8);
    EmbeddingSettingsState {
        selected_model_key,
        selected_model_label,
        effective_model_key,
        webgpu_available,
        status,
        detail,
        model_size,
        index_size,
        progress_kind,
        progress_label,
        progress_loaded,
        progress_total,
        progress_percent,
        busy: false,
        remove_confirmation_open: false,
    }
}

#[requires(true)]
#[ensures(is_supported_embedding_model_key(&ret))]
fn load_embedding_model_key() -> String {
    storage_get(EMBEDDING_MODEL_STORAGE_KEY)
        .filter(|key| is_supported_embedding_model_key(key))
        .unwrap_or_else(preferred_embedding_model_key)
}

#[requires(is_supported_embedding_model_key(model_key))]
#[ensures(true)]
fn save_embedding_model_key(model_key: &str) {
    storage_set(EMBEDDING_MODEL_STORAGE_KEY, model_key);
}

#[requires(true)]
#[ensures(true)]
fn is_supported_embedding_model_key(model_key: &str) -> bool {
    embedding_model_options()
        .iter()
        .any(|option| option.key == model_key)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn embedding_model_label(model_key: &str) -> &'static str {
    embedding_model_options()
        .iter()
        .find(|option| option.key == model_key)
        .map(|option| option.label)
        .unwrap_or("F2LLM v2 330M")
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(!ret.is_empty())]
fn embedding_model_options() -> &'static [EmbeddingModelOption] {
    WEB_EMBEDDING_MODEL_OPTIONS
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(!ret.is_empty())]
fn embedding_model_options() -> &'static [EmbeddingModelOption] {
    NATIVE_EMBEDDING_MODEL_OPTIONS
}

#[requires(!status.is_empty())]
#[requires(true)]
#[ensures(!ret.status.is_empty())]
fn embedding_settings_error_state(
    previous: &EmbeddingSettingsState,
    status: &str,
    detail: String,
) -> EmbeddingSettingsState {
    let detail = if detail.is_empty() {
        "Embedding request failed.".to_owned()
    } else {
        detail
    };
    EmbeddingSettingsState {
        selected_model_key: previous.selected_model_key.clone(),
        selected_model_label: previous.selected_model_label.clone(),
        effective_model_key: previous.effective_model_key.clone(),
        webgpu_available: previous.webgpu_available,
        status: status.to_owned(),
        detail,
        model_size: "unknown".to_owned(),
        index_size: "unknown".to_owned(),
        progress_kind: None,
        progress_label: None,
        progress_loaded: None,
        progress_total: None,
        progress_percent: None,
        busy: false,
        remove_confirmation_open: false,
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_vlacku_semantic_search_json(json: &str) -> (Vec<VlackuSemanticSearchHit>, Option<String>) {
    let value = serde_json::from_str::<serde_json::Value>(json).unwrap_or(serde_json::Value::Null);
    let message = json_string(&value, "message");
    let hits = value
        .get("hits")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|hit| {
            Some(VlackuSemanticSearchHit {
                entry_index: hit.get("id")?.as_u64()? as usize,
                score: hit.get("score")?.as_f64()? as f32,
            })
        })
        .collect();
    (hits, message)
}

#[requires(true)]
#[ensures(true)]
fn parse_cukta_semantic_search_json(json: &str) -> (Vec<CuktaSemanticSearchHit>, Option<String>) {
    let value = serde_json::from_str::<serde_json::Value>(json).unwrap_or(serde_json::Value::Null);
    let message = json_string(&value, "message");
    let hits = value
        .get("hits")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|hit| {
            Some(CuktaSemanticSearchHit {
                chunk_index: hit.get("id")?.as_u64()? as usize,
                score: hit.get("score")?.as_f64()? as f32,
            })
        })
        .collect();
    (hits, message)
}

#[requires(true)]
#[ensures(true)]
fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn human_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes < 1024 * 1024 {
        format!("{bytes} B")
    } else {
        format!("{:.1} MiB", bytes as f64 / MIB)
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_page(
    cukta_draft_state: Signal<CuktaWebState>,
    cukta_committed_state: Signal<CuktaWebState>,
    cukta_page: Signal<CuktaAsyncPageState>,
    mut toc_filter: Signal<String>,
    mut toc_pinned: Signal<bool>,
    toc_expansion: Signal<CuktaTocExpansionState>,
    toc_width: Signal<f64>,
    mut toc_resize: Signal<Option<CuktaTocResizeState>>,
    mut toc_overlay_visible: Signal<bool>,
    toc_forced_autohide: Signal<bool>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let page = cukta_page.read().page.clone();
    let toc_is_pinned = *toc_pinned.read();
    let toc_is_forced_autohide = *toc_forced_autohide.read();
    let toc_overlay_is_visible = *toc_overlay_visible.read();
    let toc_is_visible = cukta_toc_panel_visible(
        toc_is_pinned,
        toc_is_forced_autohide,
        toc_overlay_is_visible,
    );
    let toc_uses_autohide = toc_is_forced_autohide || !toc_is_pinned;
    let toc_button_state = cukta_toc_button_state(
        toc_is_pinned,
        toc_is_forced_autohide,
        toc_overlay_is_visible,
    );
    let toc_button_action = cukta_toc_button_action(toc_button_state);
    let toc_button_title = cukta_toc_button_title(toc_button_state);
    let toc_hides_on_leave =
        cukta_toc_hides_overlay_on_pointer_leave(toc_is_pinned, toc_is_forced_autohide);
    let is_resizing = toc_resize.read().is_some();
    let shell_class = class_names(
        "cll-shell",
        &[
            ("cll-toc-autohide", toc_uses_autohide),
            ("cll-toc-visible", toc_is_visible),
            ("cll-is-resizing", is_resizing),
        ],
    );
    let current_toc_width = clamp_cukta_toc_width(*toc_width.read());
    let shell_style = format!("--cll-sidebar-width:{current_toc_width:.0}px;");
    let cukta_index_route = JbotciRoute::from_web_route(
        WebRoute::Cukta(CuktaWebState {
            view: CuktaWebView::Index,
        }),
        false,
    );
    let cukta_search_route = JbotciRoute::from_web_route(
        WebRoute::Cukta(CuktaWebState {
            view: CuktaWebView::Search(CuktaWebSearchState::default()),
        }),
        false,
    );
    rsx! {
        section { class: "spa-page cll-page spa-cukta-page",
            h1 { class: "sr-only", "jbotci cukta" }
            div {
                class: "{shell_class}",
                style: "{shell_style}",
                onmousemove: move |event| {
                    if let Some(resize) = toc_resize.read().clone() {
                        let x = event.data().client_coordinates().x;
                        set_cukta_toc_width(&mut toc_width.clone(), resize.start_width + x - resize.start_x);
                    }
                },
                onmouseup: move |_| toc_resize.set(None),
                onmouseleave: move |_| toc_resize.set(None),
                aside {
                    class: "cll-sidebar",
                    onmouseleave: move |_| {
                        if toc_hides_on_leave {
                            toc_overlay_visible.set(false);
                        }
                    },
                    button {
                        class: "cll-sidebar-toggle",
                        r#type: "button",
                        title: "{toc_button_title}",
                        aria_label: "{toc_button_title}",
                        aria_pressed: pressed_attr(toc_button_state == CuktaTocButtonState::PinnedVisible),
                        onmouseenter: move |_| {
                            if toc_button_state == CuktaTocButtonState::Hidden {
                                toc_overlay_visible.set(true);
                            }
                        },
                        onclick: move |_| {
                            apply_cukta_toc_button_action(
                                &mut toc_pinned,
                                &mut toc_overlay_visible,
                                toc_button_action,
                            );
                        },
                        { render_cukta_toc_button_icon(toc_button_state) }
                    }
                    div {
                        class: "cll-toc-popup",
                        onmouseenter: move |_| {
                            if toc_button_state == CuktaTocButtonState::Hidden {
                                toc_overlay_visible.set(true);
                            }
                        },
                        div { class: "cll-toc-head",
                            label { class: "cll-toc-search",
                                input {
                                    class: "cll-toc-search-input",
                                    r#type: "search",
                                    placeholder: "Search sections",
                                    value: "{toc_filter.read()}",
                                    oninput: move |event| toc_filter.set(event.value()),
                                }
                            }
                            div { class: "cll-toc-search-meta",
                                Link {
                                    class: "cll-toc-header-link cll-toc-index-link",
                                    to: cukta_index_route.clone(),
                                    onclick_only: true,
                                    onclick: move |_| {
                                        push_route_with_cukta_scroll_intent(
                                            pending_cukta_scroll,
                                            Some(cukta_top_pending_scroll()),
                                            cukta_index_route.clone(),
                                        );
                                    },
                                    "index"
                                }
                                Link {
                                    class: "cll-toc-header-link cll-toc-advanced-link",
                                    to: cukta_search_route.clone(),
                                    onclick_only: true,
                                    onclick: move |_| {
                                        push_route_with_cukta_scroll_intent(
                                            pending_cukta_scroll,
                                            Some(cukta_top_pending_scroll()),
                                            cukta_search_route.clone(),
                                        );
                                    },
                                    "advanced search"
                                }
                            }
                        }
                        nav {
                            class: "cll-toc-scroll",
                            aria_label: "CLL table of contents",
                            "data-cukta-toc-scroll": "1",
                            onscroll: move |_| save_cukta_toc_scroll(),
                            ol { class: "cll-toc-tree",
                                for node in page.toc.iter() {
                                    { render_cukta_toc_node(toc_expansion, node, &toc_filter.read(), pending_cukta_scroll, base_path) }
                                }
                            }
                        }
                    }
                }
                div {
                    class: "cll-splitter",
                    role: "separator",
                    aria_orientation: "vertical",
                    aria_label: "Resize table of contents",
                    onmousedown: move |event| {
                        event.prevent_default();
                        if !toc_uses_autohide {
                            let x = event.data().client_coordinates().x;
                            toc_resize.set(Some(new!(CuktaTocResizeState {
                                start_x: x,
                                start_width: *toc_width.read(),
                            })));
                        }
                    },
                    span { class: "cll-splitter-grip", aria_hidden: "true" }
                }
                main {
                    class: "cll-main",
                    "data-cukta-scroll": "main",
                    onclick: move |_| {
                        if toc_hides_on_leave {
                            toc_overlay_visible.set(false);
                        }
                    },
                    {
                        match &page.page_kind {
                            CuktaPageKind::Section {
                                section_heading,
                                section_parse_href,
                                chapter_title,
                                previous_section,
                                next_section,
                                chapter_prelude_blocks,
                                blocks,
                            } => render_cukta_section(
                                pending_cukta_scroll,
                                section_heading,
                                section_parse_href.as_deref(),
                                chapter_title.as_deref(),
                                previous_section.as_ref(),
                                next_section.as_ref(),
                                chapter_prelude_blocks,
                                blocks,
                                base_path,
                                script,
                                page_find,
                            ),
                            CuktaPageKind::Index { entries } => {
                                render_cukta_index(entries, pending_cukta_scroll, base_path, page_find)
                            }
                            CuktaPageKind::Search {
                                state,
                                mode_options: _,
                                target_options: _,
                                results,
                                message,
                                has_more,
                                load_more_href: _,
                            } => {
                                // Keep CLL search results out of the draft-query dependency path;
                                // the focused input already reflects keystrokes until debounce commits.
                                let draft_search =
                                    cukta_search_draft_for_page(&cukta_draft_state.peek(), state);
                                render_cukta_search(
                                    cukta_draft_state,
                                    cukta_committed_state,
                                    pending_cukta_scroll,
                                    &draft_search,
                                    results,
                                    message.as_deref(),
                                    *has_more,
                                    base_path,
                                    script,
                                    page_find,
                                )
                            }
                            CuktaPageKind::Error { message } => rsx! {
                                div { class: "spa-error", { render_page_find_text(page_find, message) } }
                            },
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(ret >= cukta_toc_width_min())]
#[ensures(ret <= cukta_toc_width_max())]
fn clamp_cukta_toc_width(width: f64) -> f64 {
    width.max(cukta_toc_width_min()).min(cukta_toc_width_max())
}

#[requires(true)]
#[ensures(ret > 0.0)]
fn cukta_toc_width_min() -> f64 {
    300.0
}

#[requires(true)]
#[ensures(ret > cukta_toc_width_min())]
fn cukta_toc_width_max() -> f64 {
    560.0
}

#[requires(true)]
#[ensures(ret >= cukta_toc_width_min())]
#[ensures(ret <= cukta_toc_width_max())]
fn default_cukta_toc_width() -> f64 {
    390.0
}

#[requires(true)]
#[ensures(ret >= cukta_toc_width_min())]
#[ensures(ret <= cukta_toc_width_max())]
fn load_cukta_toc_width() -> f64 {
    storage_get("jbotci.cukta.toc.width.v1")
        .and_then(|value| value.parse::<f64>().ok())
        .map(clamp_cukta_toc_width)
        .unwrap_or_else(default_cukta_toc_width)
}

#[requires(true)]
#[ensures(true)]
fn load_cukta_toc_pinned() -> bool {
    storage_get("jbotci.cukta.toc.pinned.v1").as_deref() != Some("0")
}

#[requires(true)]
#[ensures(true)]
fn load_cukta_toc_expansion() -> CuktaTocExpansionState {
    session_storage_get("jbotci.cukta.toc.expansion.v1")
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|value| {
            let object = value.as_object()?;
            let expanded = json_string_array(object.get("expanded"));
            let mut collapsed = json_string_array(object.get("collapsed"));
            collapsed.retain(|node_id| !expanded.iter().any(|expanded| expanded == node_id));
            Some(new!(CuktaTocExpansionState {
                expanded,
                collapsed,
            }))
        })
        .unwrap_or_default()
}

#[requires(true)]
#[ensures(true)]
fn json_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

#[requires(true)]
#[ensures(true)]
fn save_cukta_toc_expansion(state: &CuktaTocExpansionState) {
    let value = serde_json::json!({
        "expanded": &state.expanded,
        "collapsed": &state.collapsed,
    });
    session_storage_set("jbotci.cukta.toc.expansion.v1", &value.to_string());
}

#[requires(true)]
#[ensures(true)]
fn set_cukta_toc_width(width: &mut Signal<f64>, next_width: f64) {
    let next_width = clamp_cukta_toc_width(next_width);
    storage_set("jbotci.cukta.toc.width.v1", &format!("{next_width:.0}"));
    width.set(next_width);
}

#[requires(true)]
#[ensures(true)]
fn set_cukta_toc_pinned(pinned: &mut Signal<bool>, value: bool) {
    storage_set("jbotci.cukta.toc.pinned.v1", if value { "1" } else { "0" });
    pinned.set(value);
}

#[requires(true)]
#[ensures(ret == ((!forced_autohide && pinned) || overlay_visible))]
fn cukta_toc_panel_visible(pinned: bool, forced_autohide: bool, overlay_visible: bool) -> bool {
    (!forced_autohide && pinned) || overlay_visible
}

#[requires(true)]
#[ensures(cukta_toc_panel_visible(pinned, forced_autohide, overlay_visible) || ret == CuktaTocButtonState::Hidden)]
fn cukta_toc_button_state(
    pinned: bool,
    forced_autohide: bool,
    overlay_visible: bool,
) -> CuktaTocButtonState {
    if !cukta_toc_panel_visible(pinned, forced_autohide, overlay_visible) {
        CuktaTocButtonState::Hidden
    } else if forced_autohide {
        CuktaTocButtonState::ForcedAutoHideVisible
    } else if pinned {
        CuktaTocButtonState::PinnedVisible
    } else {
        CuktaTocButtonState::UnpinnedVisible
    }
}

#[requires(true)]
#[ensures(state == CuktaTocButtonState::Hidden -> ret == CuktaTocButtonAction::ShowOverlay)]
#[ensures(state == CuktaTocButtonState::ForcedAutoHideVisible -> ret == CuktaTocButtonAction::HideOverlay)]
#[ensures(state == CuktaTocButtonState::PinnedVisible -> ret == CuktaTocButtonAction::Unpin)]
#[ensures(state == CuktaTocButtonState::UnpinnedVisible -> ret == CuktaTocButtonAction::Pin)]
fn cukta_toc_button_action(state: CuktaTocButtonState) -> CuktaTocButtonAction {
    match state {
        CuktaTocButtonState::Hidden => CuktaTocButtonAction::ShowOverlay,
        CuktaTocButtonState::ForcedAutoHideVisible => CuktaTocButtonAction::HideOverlay,
        CuktaTocButtonState::PinnedVisible => CuktaTocButtonAction::Unpin,
        CuktaTocButtonState::UnpinnedVisible => CuktaTocButtonAction::Pin,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn cukta_toc_button_title(state: CuktaTocButtonState) -> &'static str {
    match state {
        CuktaTocButtonState::Hidden => "Show table of contents",
        CuktaTocButtonState::ForcedAutoHideVisible => "Hide table of contents",
        CuktaTocButtonState::PinnedVisible => "Unpin table of contents",
        CuktaTocButtonState::UnpinnedVisible => "Pin table of contents",
    }
}

#[requires(true)]
#[ensures(ret == (forced_autohide || !pinned))]
fn cukta_toc_hides_overlay_on_pointer_leave(pinned: bool, forced_autohide: bool) -> bool {
    forced_autohide || !pinned
}

#[requires(true)]
#[ensures(true)]
fn cukta_toc_interaction_after_button_action(
    state: CuktaTocInteractionState,
    action: CuktaTocButtonAction,
) -> CuktaTocInteractionState {
    match action {
        CuktaTocButtonAction::ShowOverlay => CuktaTocInteractionState {
            pinned: state.pinned,
            overlay_visible: true,
        },
        CuktaTocButtonAction::HideOverlay => CuktaTocInteractionState {
            pinned: state.pinned,
            overlay_visible: false,
        },
        CuktaTocButtonAction::Pin => CuktaTocInteractionState {
            pinned: true,
            overlay_visible: false,
        },
        CuktaTocButtonAction::Unpin => CuktaTocInteractionState {
            pinned: false,
            overlay_visible: true,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn apply_cukta_toc_button_action(
    pinned: &mut Signal<bool>,
    overlay_visible: &mut Signal<bool>,
    action: CuktaTocButtonAction,
) {
    let current = CuktaTocInteractionState {
        pinned: *pinned.read(),
        overlay_visible: *overlay_visible.read(),
    };
    let next = cukta_toc_interaction_after_button_action(current, action);
    if current.pinned != next.pinned {
        set_cukta_toc_pinned(pinned, next.pinned);
    }
    if current.overlay_visible != next.overlay_visible {
        overlay_visible.set(next.overlay_visible);
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_toc_button_icon(state: CuktaTocButtonState) -> Element {
    match state {
        CuktaTocButtonState::Hidden => rsx! {
            svg {
                class: "cll-sidebar-toggle-icon",
                view_box: "0 0 24 24",
                path {
                    d: "M4.5 5.5H19.5 M4.5 11.5H7.5 M9.75 11.5H19.5 M7.5 17.5H10.5 M12.75 17.5H19.5",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2",
                    stroke_linecap: "round",
                }
            }
        },
        CuktaTocButtonState::ForcedAutoHideVisible => rsx! {
            svg {
                class: "cll-sidebar-toggle-icon",
                view_box: "0 0 24 24",
                path {
                    d: "M7 7L17 17M17 7L7 17",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2.2",
                    stroke_linecap: "round",
                }
            }
        },
        CuktaTocButtonState::PinnedVisible => rsx! {
            svg {
                class: "cll-sidebar-toggle-icon",
                view_box: "0 0 24 24",
                path {
                    d: "M8 4.5H16L14.75 10L18 13.25V15H12.7L12 20H10.8L11.3 15H6V13.25L9.25 10L8 4.5Z",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.7",
                    stroke_linejoin: "round",
                }
                path {
                    d: "M5 5L19 19",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2",
                    stroke_linecap: "round",
                }
            }
        },
        CuktaTocButtonState::UnpinnedVisible => rsx! {
            svg {
                class: "cll-sidebar-toggle-icon",
                view_box: "0 0 24 24",
                path {
                    d: "M8 4.5H16L14.75 10L18 13.25V15H12.7L12 20H10.8L11.3 15H6V13.25L9.25 10L8 4.5Z",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.7",
                    stroke_linejoin: "round",
                }
                path {
                    d: "M9.25 10H14.75",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.5",
                    stroke_linecap: "round",
                }
            }
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_toc_node(
    toc_expansion: Signal<CuktaTocExpansionState>,
    node: &CuktaTocNode,
    filter: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
) -> Element {
    let filter = filter.trim().to_ascii_lowercase();
    let visible = filter.is_empty()
        || node.label.to_ascii_lowercase().contains(&filter)
        || node
            .number_label
            .as_ref()
            .is_some_and(|number| number.contains(&filter))
        || node.children.iter().any(|child| {
            child.label.to_ascii_lowercase().contains(&filter)
                || child
                    .number_label
                    .as_ref()
                    .is_some_and(|number| number.contains(&filter))
        });
    if !visible {
        return rsx! {};
    }
    let expanded = toc_node_expanded(node, &filter, &toc_expansion.read());
    let number_has_trailing_dot = node.section_id.is_none();
    let class = class_names(
        "cll-toc-node",
        &[
            ("active", node.active),
            ("is-active", node.active),
            ("current", node.current),
            ("is-current", node.current),
            ("cll-chapter-node", node.section_id.is_none()),
            ("is-chapter", node.section_id.is_none()),
            ("has-children", !node.children.is_empty()),
            ("is-expanded", expanded),
        ],
    );
    let route = jbotci_route_from_href(base_path, &node.href).map(|route| {
        let pending_scroll = cukta_pending_scroll_for_route_link(base_path, &route);
        let click_route = route.clone();
        (route, click_route, pending_scroll)
    });
    rsx! {
        li { key: "{node.node_id}", class: "{class}",
            div { class: "cll-toc-row",
                if !node.children.is_empty() {
                    button {
                        class: "cll-toc-toggle",
                        r#type: "button",
                        aria_expanded: if expanded { "true" } else { "false" },
                        title: if expanded { "Collapse" } else { "Expand" },
                        onclick: {
                            let node_id = node.node_id.clone();
                            let default_expanded = node.active;
                            move |_| {
                                toggle_cukta_toc_node(
                                    &mut toc_expansion.clone(),
                                    &node_id,
                                    default_expanded,
                                    expanded,
                                )
                            }
                        },
                        span { aria_hidden: "true",
                            if expanded { "▾" } else { "▸" }
                        }
                    }
                } else {
                    span { class: "cll-toc-spacer", aria_hidden: "true" }
                }
                if let Some((route, click_route, pending_scroll)) = route {
                    Link {
                        class: "cll-toc-link",
                        to: route,
                        onclick_only: true,
                        onclick: move |_| {
                            push_route_with_cukta_scroll_intent(
                                pending_cukta_scroll,
                                Some(pending_scroll.clone()),
                                click_route.clone(),
                            );
                        },
                        if let Some(number) = &node.number_label {
                            { render_cukta_toc_number(number, number_has_trailing_dot) }
                        }
                        { render_cukta_toc_title(&node.label) }
                    }
                } else {
                    a {
                        class: "cll-toc-link",
                        href: "{node.href}",
                        if let Some(number) = &node.number_label {
                            { render_cukta_toc_number(number, number_has_trailing_dot) }
                        }
                        { render_cukta_toc_title(&node.label) }
                    }
                }
            }
            if !node.children.is_empty() && expanded {
                ol { class: "cll-toc-children",
                    for child in node.children.iter() {
                        { render_cukta_toc_node(toc_expansion, child, &filter, pending_cukta_scroll, base_path) }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_toc_number(number: &str, trailing_dot: bool) -> Element {
    if let Some((before_dot, after_dot)) = number.split_once('.') {
        return rsx! {
            span { class: "cll-toc-number",
                span { class: "cll-toc-number-before-dot", "{before_dot}" }
                span { class: "cll-toc-number-dot", "." }
                span { class: "cll-toc-number-after-dot", "{after_dot}" }
            }
        };
    }

    rsx! {
        span { class: "cll-toc-number",
            span { class: "cll-toc-number-before-dot", "{number}" }
            if trailing_dot {
                span { class: "cll-toc-number-dot", "." }
            } else {
                span { class: "cll-toc-number-dot" }
            }
            span { class: "cll-toc-number-after-dot" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_toc_title(label: &str) -> Element {
    if let Some((prefix, suffix)) = label.split_once(':') {
        let prefix = format!("{prefix}:");
        let suffix = suffix.trim_start();
        return rsx! {
            span { class: "cll-toc-title cll-toc-title-has-colon",
                span { class: "cll-toc-title-before-colon", "{prefix}" }
                span { class: "cll-toc-title-after-colon", "{suffix}" }
            }
        };
    }
    rsx! {
        span { class: "cll-toc-title", "{label}" }
    }
}

#[requires(true)]
#[ensures(true)]
fn toc_node_expanded(
    node: &CuktaTocNode,
    filter: &str,
    expansion: &CuktaTocExpansionState,
) -> bool {
    if !filter.trim().is_empty() {
        return true;
    }
    cukta_toc_node_expanded_with_default(&node.node_id, node.active, expansion)
}

#[requires(!node_id.is_empty())]
#[ensures(true)]
fn cukta_toc_node_expanded_with_default(
    node_id: &str,
    default_expanded: bool,
    expansion: &CuktaTocExpansionState,
) -> bool {
    if expansion.expanded.iter().any(|id| id == node_id) {
        true
    } else if expansion.collapsed.iter().any(|id| id == node_id) {
        false
    } else {
        default_expanded
    }
}

#[requires(!node_id.is_empty())]
#[ensures(true)]
fn toggle_cukta_toc_node(
    toc_expansion: &mut Signal<CuktaTocExpansionState>,
    node_id: &str,
    default_expanded: bool,
    currently_expanded: bool,
) {
    let current = toc_expansion.read().clone();
    let next = cukta_toc_expansion_with_node_state(
        &current,
        node_id,
        default_expanded,
        !currently_expanded,
    );
    save_cukta_toc_expansion(&next);
    toc_expansion.set(next);
}

#[requires(!node_id.is_empty())]
#[ensures(cukta_toc_node_expanded_with_default(node_id, default_expanded, &ret) == desired_expanded)]
fn cukta_toc_expansion_with_node_state(
    expansion: &CuktaTocExpansionState,
    node_id: &str,
    default_expanded: bool,
    desired_expanded: bool,
) -> CuktaTocExpansionState {
    let data = expansion.clone().into_data();
    let mut expanded = data.expanded;
    let mut collapsed = data.collapsed;
    expanded.retain(|id| id != node_id);
    collapsed.retain(|id| id != node_id);
    if desired_expanded != default_expanded {
        if desired_expanded {
            expanded.push(node_id.to_owned());
        } else {
            collapsed.push(node_id.to_owned());
        }
    }
    new!(CuktaTocExpansionState {
        expanded,
        collapsed,
    })
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_section(
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    heading: &str,
    parse_href: Option<&str>,
    chapter_title: Option<&str>,
    previous: Option<&jbotci_web_core::CuktaSectionLink>,
    next: Option<&jbotci_web_core::CuktaSectionLink>,
    prelude_blocks: &[CllBlock],
    blocks: &[CllBlock],
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let _ = chapter_title;
    rsx! {
        article { class: "cll-section-content",
            div { class: "cll-section-heading",
                h1 { { render_page_find_text(page_find, heading) } }
                if let Some(parse_href) = parse_href {
                    { render_cll_parse_link(
                        "cll-parse-example cll-parse-section spa-cll-link spa-cll-link-parse",
                        parse_href,
                        base_path,
                    ) }
                }
            }
            if !prelude_blocks.is_empty() {
                div { class: "cll-chapter-prelude",
                    for block in prelude_blocks.iter() {
                        { render_cll_block(block, pending_cukta_scroll, base_path, script, page_find) }
                    }
                }
            }
            for block in blocks.iter() {
                { render_cll_block(block, pending_cukta_scroll, base_path, script, page_find) }
            }
            if previous.is_some() || next.is_some() {
                nav { class: "cll-section-pager",
                    if let Some(previous) = previous {
                        { render_cukta_section_pager_link(previous, "prev", pending_cukta_scroll, base_path, page_find) }
                    }
                    if let Some(next) = next {
                        { render_cukta_section_pager_link(next, "next", pending_cukta_scroll, base_path, page_find) }
                    }
                }
            }
        }
    }
}

#[requires(direction == "prev" || direction == "next")]
#[ensures(true)]
fn render_cukta_section_pager_link(
    section: &jbotci_web_core::CuktaSectionLink,
    direction: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    page_find: &PageFindContext,
) -> Element {
    let class_name = format!("cll-section-pager-link cll-section-pager-link-{direction}");
    if let Some(route) = jbotci_route_from_href(base_path, &section.href) {
        let pending_scroll = cukta_pending_scroll_for_route_link(base_path, &route);
        let click_route = route.clone();
        rsx! {
            Link {
                class: "{class_name}",
                to: route,
                onclick_only: true,
                onclick: move |_| {
                    push_route_with_cukta_scroll_intent(
                        pending_cukta_scroll,
                        Some(pending_scroll.clone()),
                        click_route.clone(),
                    );
                },
                span { class: "cll-section-pager-link-label",
                    { render_page_find_text(page_find, &section.label) }
                }
            }
        }
    } else {
        rsx! {
            a {
                class: "{class_name}",
                href: "{section.href}",
                span { class: "cll-section-pager-link-label",
                    { render_page_find_text(page_find, &section.label) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_index(
    entries: &[jbotci_web_core::CuktaIndexEntry],
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        section { class: "cll-index-view",
            h1 { { render_page_find_text(page_find, "Index") } }
            div { class: "cll-index-list",
                for entry in entries.iter() {
                    div { class: "cll-index-entry",
                        span { class: "cll-index-key",
                            { render_page_find_text(page_find, &entry.key) }
                        }
                        span { class: "cll-index-refs",
                            for reference in entry.references.iter() {
                                {
                                    if let Some(route) = jbotci_route_from_href(base_path, &reference.href) {
                                        let pending_scroll = cukta_pending_scroll_for_route_link(base_path, &route);
                                        let click_route = route.clone();
                                        rsx! {
                                            Link {
                                                to: route,
                                                onclick_only: true,
                                                onclick: move |_| {
                                                    push_route_with_cukta_scroll_intent(
                                                        pending_cukta_scroll,
                                                        Some(pending_scroll.clone()),
                                                        click_route.clone(),
                                                    );
                                                },
                                                { render_page_find_text(page_find, &reference.label) }
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            a {
                                                href: "{reference.href}",
                                                { render_page_find_text(page_find, &reference.label) }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_search(
    cukta_draft_state: Signal<CuktaWebState>,
    cukta_committed_state: Signal<CuktaWebState>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    draft_state: &CuktaWebSearchState,
    results: &[CuktaSearchResultCard],
    message: Option<&str>,
    has_more: bool,
    base_path: &str,
    _script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let state_for_load_more = draft_state.clone();
    let mode_options = cukta_draft_mode_options(draft_state.mode);
    let target_options = cukta_draft_target_options(&draft_state.targets);
    rsx! {
        section { class: "cll-search-view dictionary-page",
            { render_cukta_search_controls(
                cukta_draft_state,
                cukta_committed_state,
                draft_state,
                &mode_options,
                &target_options,
            ) }
            if let Some(message) = message {
                { render_semantic_search_message("dictionary-empty cll-search-message", message, Some(page_find)) }
            }
            div { class: "cll-search-results",
                for card in results.iter() {
                    { render_cukta_search_card(card, pending_cukta_scroll, base_path, page_find) }
                }
            }
            if has_more {
                div { class: "load-more-wrap",
                    button {
                        class: "btn-parse load-more-link",
                        r#type: "button",
                        onclick: move |_| {
                            let mut next = state_for_load_more.clone();
                            next.count = next.count.saturating_mul(2).clamp(1, CUKTA_WEB_MAX_COUNT);
                            set_cukta_state_immediate(
                                cukta_draft_state,
                                cukta_committed_state,
                                CuktaWebState {
                                    view: CuktaWebView::Search(next),
                                },
                            );
                        },
                        { render_page_find_text(page_find, "Load more") }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_search_controls(
    mut cukta_draft_state: Signal<CuktaWebState>,
    cukta_committed_state: Signal<CuktaWebState>,
    state: &CuktaWebSearchState,
    mode_options: &[CuktaModeOption],
    target_options: &[CuktaTargetOption],
) -> Element {
    let state_for_input = state.clone();
    rsx! {
        div { class: "dictionary-form cll-search-form",
            div { class: "dictionary-controls cll-search-controls",
                div { class: "dictionary-mode-control",
                    div { class: "mode-toggle-row",
                        div { class: "mode-selector-wrap",
                            div { class: "mode-bracket-row", aria_hidden: "true",
                                span { class: "mode-bracket-label", "similar" }
                                span { class: "mode-bracket-label", "contains" }
                            }
                            div { class: "mode-toggle-group", role: "group", aria_label: "CLL search mode",
                                for option in mode_options.iter() {
                                    { render_cukta_mode_button(cukta_draft_state, cukta_committed_state, state, option) }
                                }
                            }
                        }
                    }
                }
                div { class: "cll-target-control",
                    div { class: "cll-target-grid", aria_label: "CLL search targets",
                        for option in target_options.iter() {
                            { render_cukta_target_check(cukta_draft_state, cukta_committed_state, state, option) }
                        }
                    }
                }
            }
            div { class: "dictionary-query-row",
                input {
                    class: "query-input",
                    r#type: "search",
                    aria_label: "CLL search query",
                    placeholder: if state.mode == CuktaWebMode::Word { "valsi" } else { "semantic search" },
                    spellcheck: "false",
                    value: "{state.query}",
                    oninput: move |event| {
                        let query = event.value();
                        let next = cukta_search_state_with_query(&state_for_input, &query);
                        let next_state = CuktaWebState {
                            view: CuktaWebView::Search(next),
                        };
                        cukta_draft_state.set(next_state.clone());
                        schedule_cukta_search_commit(cukta_committed_state, next_state);
                    },
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_mode_button(
    cukta_draft_state: Signal<CuktaWebState>,
    cukta_committed_state: Signal<CuktaWebState>,
    state: &CuktaWebSearchState,
    option: &CuktaModeOption,
) -> Element {
    let state_for_click = state.clone();
    let option_disabled = option.disabled;
    let option_selected = option.selected;
    let option_label = option.label.clone();
    let mode = if option.value == "valsi" {
        CuktaWebMode::Word
    } else {
        CuktaWebMode::Meaning
    };
    rsx! {
        button {
            class: vlacku_mode_class(option_selected),
            r#type: "button",
            disabled: option_disabled,
            title: if mode == CuktaWebMode::Meaning { "Find CLL passages with similar meaning" } else { "Find CLL passages containing this word" },
            aria_pressed: pressed_attr(option_selected),
            onclick: move |_| {
                if !option_disabled {
                    let mut next = state_for_click.clone();
                    next.mode = mode;
                    next.count = CUKTA_WEB_DEFAULT_COUNT;
                    set_cukta_state_immediate(
                        cukta_draft_state,
                        cukta_committed_state,
                        CuktaWebState {
                            view: CuktaWebView::Search(next),
                        },
                    );
                }
            },
            "{option_label}"
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_target_check(
    cukta_draft_state: Signal<CuktaWebState>,
    cukta_committed_state: Signal<CuktaWebState>,
    state: &CuktaWebSearchState,
    option: &CuktaTargetOption,
) -> Element {
    let state_for_change = state.clone();
    let class_name = if option.selected {
        "compact-check is-selected"
    } else {
        "compact-check"
    };
    let value = option.value.clone();
    rsx! {
        label { class: "{class_name}",
            input {
                r#type: "checkbox",
                checked: option.selected,
                onchange: move |_| {
                    let mut next = state_for_change.clone();
                    next.targets = toggle_cukta_target_selection(&next.targets, &value);
                    next.count = CUKTA_WEB_DEFAULT_COUNT;
                    set_cukta_state_immediate(
                        cukta_draft_state,
                        cukta_committed_state,
                        CuktaWebState {
                            view: CuktaWebView::Search(next),
                        },
                    );
                },
            }
            span { class: "vlacku-filter-label", "{option.label}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_search_card(
    card: &CuktaSearchResultCard,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    page_find: &PageFindContext,
) -> Element {
    let route = jbotci_route_from_href(base_path, &card.href).map(|route| {
        let pending_scroll = cukta_pending_scroll_for_route_link(base_path, &route);
        let click_route = route.clone();
        (route, click_route, pending_scroll)
    });
    rsx! {
        article { class: "cll-search-result-card result-card",
            header { class: "cll-search-result-head result-header",
                div {
                    p { class: "cll-search-result-meta",
                        { render_page_find_text(page_find, &format!("{} · {}", card.kind, card.section_label)) }
                    }
                    h2 { class: "cll-search-result-title",
                        if let Some((route, click_route, pending_scroll)) = route {
                            {
                                let label = format!("{}. {}", card.rank, card.label);
                                rsx! {
                            Link {
                                to: route,
                                onclick_only: true,
                                onclick: move |_| {
                                    push_route_with_cukta_scroll_intent(
                                        pending_cukta_scroll,
                                        Some(pending_scroll.clone()),
                                        click_route.clone(),
                                    );
                                },
                                { render_page_find_text(page_find, &label) }
                            }
                                }
                            }
                        } else {
                            {
                                let label = format!("{}. {}", card.rank, card.label);
                                rsx! {
                            a {
                                href: "{card.href}",
                                { render_page_find_text(page_find, &label) }
                            }
                                }
                            }
                        }
                    }
                }
                if let Some(similarity) = &card.similarity_label {
                    span { class: "dictionary-meta-segment dictionary-meta-tooltip",
                        { render_page_find_text(page_find, similarity) }
                    }
                }
            }
            p { class: "cll-search-preview",
                { render_page_find_text(page_find, &card.preview) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_block(
    block: &CllBlock,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    match block {
        CllBlock::Paragraph {
            anchor_id,
            role,
            inlines,
            text,
        } => {
            let class_name = role
                .as_ref()
                .map(|role| format!("cll-para cll-para-{role}"))
                .unwrap_or_else(|| "cll-para".to_owned());
            rsx! {
                p { id: anchor_id.clone().unwrap_or_default(), class: "{class_name}",
                    if inlines.is_empty() {
                        { render_page_find_text(page_find, text) }
                    } else {
                        for inline in inlines.iter() {
                            { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                        }
                    }
                }
            }
        }
        CllBlock::List { ordered, items } => {
            if *ordered {
                rsx! {
                    ol { class: "cll-list",
                        for item in items.iter() {
                            li {
                                for child in item.iter() {
                                    { render_cll_block(child, pending_cukta_scroll, base_path, script, page_find) }
                                }
                            }
                        }
                    }
                }
            } else {
                rsx! {
                    ul { class: "cll-list",
                        for item in items.iter() {
                            li {
                                for child in item.iter() {
                                    { render_cll_block(child, pending_cukta_scroll, base_path, script, page_find) }
                                }
                            }
                        }
                    }
                }
            }
        }
        CllBlock::Example(example) => rsx! {
            figure { id: "{example.anchor_id}", class: "cll-example",
                figcaption { class: "cll-example-head",
                    span { class: "cll-example-title",
                        { render_page_find_text(page_find, &example.label) }
                    }
                    if let Some(parse_href) = &example.parse_href {
                        { render_cll_parse_link(
                            "cll-parse-example spa-cll-link spa-cll-link-parse",
                            parse_href,
                            base_path,
                        ) }
                    }
                }
                if example.blocks.is_empty() {
                    div { class: "cll-interlinear",
                        for line in example.lines.iter() {
                            {
                                let text = cll_display_text_for_kind(script, &line.kind, &line.text);
                                rsx! { p { class: "cll-ig-line cll-ig-{line.kind}", { render_page_find_text(page_find, &text) } } }
                            }
                        }
                    }
                } else {
                    for child in example.blocks.iter() {
                        { render_cll_block(child, pending_cukta_scroll, base_path, script, page_find) }
                    }
                }
            }
        },
        CllBlock::Table {
            id,
            caption,
            header_rows,
            body_rows,
            classes,
        } => {
            let table_class = cll_table_class(classes);
            rsx! {
            table { id: id.clone().unwrap_or_default(), class: "{table_class}",
                if let Some(caption) = caption {
                    caption {
                        for inline in caption.iter() {
                            { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                        }
                    }
                }
                if !header_rows.is_empty() {
                    thead {
                        for row in header_rows.iter() {
                            {
                                let row_class = cll_table_row_parse_class(row);
                                let row_group_id = cll_table_row_parse_group_id(row).unwrap_or_default();
                                rsx! {
                            tr { class: "{row_class}", "data-cll-parse-group": "{row_group_id}",
                                for cell in row.iter() {
                                    th {
                                        colspan: "{cell.col_span.unwrap_or(1)}",
                                        rowspan: "{cell.row_span.unwrap_or(1)}",
                                        if let Some(parse_href) = &cell.parse_href {
                                            {
                                                let parse_class = cll_table_cell_parse_link_class(cell);
                                                rsx! {
                                            { render_cll_parse_link(
                                                &parse_class,
                                                parse_href,
                                                base_path,
                                            ) }
                                                }
                                            }
                                        }
                                        for child in cell.blocks.iter() {
                                            { render_cll_block(child, pending_cukta_scroll, base_path, script, page_find) }
                                        }
                                    }
                                }
                            }
                                }
                            }
                        }
                    }
                }
                tbody {
                    for row in body_rows.iter() {
                        {
                            let row_class = cll_table_row_parse_class(row);
                            let row_group_id = cll_table_row_parse_group_id(row).unwrap_or_default();
                            rsx! {
                        tr { class: "{row_class}", "data-cll-parse-group": "{row_group_id}",
                            for cell in row.iter() {
                                td {
                                    colspan: "{cell.col_span.unwrap_or(1)}",
                                    rowspan: "{cell.row_span.unwrap_or(1)}",
                                    if let Some(parse_href) = &cell.parse_href {
                                        {
                                            let parse_class = cll_table_cell_parse_link_class(cell);
                                            rsx! {
                                        { render_cll_parse_link(
                                            &parse_class,
                                            parse_href,
                                            base_path,
                                        ) }
                                            }
                                        }
                                    }
                                    for child in cell.blocks.iter() {
                                        { render_cll_block(child, pending_cukta_scroll, base_path, script, page_find) }
                                    }
                                }
                            }
                        }
                            }
                        }
                    }
                }
            }
            }
        }
        CllBlock::SimpleListTable {
            id,
            orientation,
            rows,
        } => {
            let orientation_class = match orientation {
                CllSimpleListOrientation::Horizontal => "horizontal",
                CllSimpleListOrientation::Vertical => "vertical",
            };
            rsx! {
                table {
                    id: id.clone().unwrap_or_default(),
                    class: "cll-simplelist cll-simplelist-{orientation_class}",
                    tbody {
                        for row in rows.iter() {
                            tr {
                                for cell in row.iter() {
                                    td {
                                        if let Some(inlines) = cell {
                                            for inline in inlines.iter() {
                                                { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        CllBlock::VariableList { id, entries } => rsx! {
            dl { id: id.clone().unwrap_or_default(), class: "cll-variable-list",
                for entry in entries.iter() {
                    dt {
                        for inline in entry.term.iter() {
                            { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                        }
                    }
                    dd {
                        for child in entry.blocks.iter() {
                            { render_cll_block(child, pending_cukta_scroll, base_path, script, page_find) }
                        }
                    }
                }
            }
        },
        CllBlock::Media {
            id,
            title,
            src,
            alt,
        } => {
            let asset_src = cll_asset_href(base_path, src);
            rsx! {
                figure { id: id.clone().unwrap_or_default(), class: "cll-media",
                    img { src: "{asset_src}", alt: "{alt}" }
                    if let Some(title) = title {
                        figcaption {
                            for inline in title.iter() {
                                { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                            }
                        }
                    }
                }
            }
        }
        CllBlock::Rule { id, term, body } => rsx! {
            div { id: id.clone().unwrap_or_default(), class: "cll-rule",
                dt { { render_page_find_text(page_find, term) } }
                dd {
                    for child in body.iter() {
                        { render_cll_block(child, pending_cukta_scroll, base_path, script, page_find) }
                    }
                }
            }
        },
        CllBlock::Code { text, .. } => rsx! {
            pre { class: "cll-code", code { { render_page_find_text(page_find, text) } } }
        },
        CllBlock::DisplayMath { id, markup, .. } => rsx! {
            div {
                id: id.clone().unwrap_or_default(),
                class: "cll-math-block",
                dangerous_inner_html: "{markup}"
            }
        },
        CllBlock::Heading {
            id, level, inlines, ..
        } => {
            let class_name = format!("cll-heading cll-heading-{level}");
            rsx! {
                h2 { id: id.clone().unwrap_or_default(), class: "{class_name}",
                    for inline in inlines.iter() {
                        { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                    }
                }
            }
        }
        CllBlock::BlockQuote { id, blocks } => rsx! {
            blockquote { id: id.clone().unwrap_or_default(), class: "cll-blockquote",
                for child in blocks.iter() {
                    { render_cll_block(child, pending_cukta_scroll, base_path, script, page_find) }
                }
            }
        },
        CllBlock::Definition { id, body } => rsx! {
            p { id: id.clone().unwrap_or_default(), class: "cll-definition",
                for inline in body.iter() {
                    { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                }
            }
        },
        CllBlock::InterlinearGloss {
            id,
            aligned,
            itemized,
            parse_href,
            rows,
            natlang,
            comments,
        } => render_cll_interlinear(
            id.as_deref(),
            *aligned,
            *itemized,
            parse_href.as_deref(),
            rows,
            natlang,
            comments,
            pending_cukta_scroll,
            base_path,
            script,
            page_find,
        ),
        CllBlock::CmavoList {
            id,
            titles,
            headers,
            rows,
        } => render_cll_cmavo_list(
            id.as_deref(),
            titles,
            headers,
            rows,
            pending_cukta_scroll,
            base_path,
            script,
            page_find,
        ),
        CllBlock::Lojbanization { id, lines } => render_cll_lojbanization(
            id.as_deref(),
            lines,
            pending_cukta_scroll,
            base_path,
            script,
            page_find,
        ),
        CllBlock::LujvoMaking { id, parts } => render_cll_lujvo_making(
            id.as_deref(),
            parts,
            pending_cukta_scroll,
            base_path,
            script,
            page_find,
        ),
        CllBlock::GrammarTemplate { id, body } => rsx! {
            p { id: id.clone().unwrap_or_default(), class: "cll-grammar-template",
                for inline in body.iter() {
                    { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                }
            }
        },
        CllBlock::Ebnf { id, entries } => render_cll_ebnf(
            id.as_deref(),
            entries,
            pending_cukta_scroll,
            base_path,
            script,
            page_find,
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_inline(
    inline: &CllInline,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    lojban_context: bool,
    page_find: &PageFindContext,
) -> Element {
    match inline {
        CllInline::Text(text) => {
            let text = if lojban_context {
                display_lojban_text(script, text)
            } else {
                text.clone()
            };
            rsx! { { render_page_find_text(page_find, &text) } }
        }
        CllInline::Emphasis { language, inlines } => {
            let child_context = lojban_context || cll_language_is_lojban(language.as_deref());
            rsx! {
                em { lang: language.clone().unwrap_or_default(),
                    for child in inlines.iter() {
                        { render_cll_inline(child, pending_cukta_scroll, base_path, script, child_context, page_find) }
                    }
                }
            }
        }
        CllInline::Quote { language, inlines } => {
            let child_context = lojban_context || cll_language_is_lojban(language.as_deref());
            rsx! {
                q { lang: language.clone().unwrap_or_default(),
                    for child in inlines.iter() {
                        { render_cll_inline(child, pending_cukta_scroll, base_path, script, child_context, page_find) }
                    }
                }
            }
        }
        CllInline::LanguageSpan {
            kind,
            language,
            inlines,
        } => {
            let class_name = cll_language_span_class(*kind);
            let child_context = lojban_context
                || *kind == CllLanguageSpanKind::JboPhrase
                || cll_language_is_lojban(language.as_deref());
            rsx! {
                span { class: "{class_name}", lang: language.clone().unwrap_or_default(),
                    for child in inlines.iter() {
                        { render_cll_inline(child, pending_cukta_scroll, base_path, script, child_context, page_find) }
                    }
                }
            }
        }
        CllInline::CiteTitle { inlines } => rsx! {
            cite {
                for child in inlines.iter() {
                    { render_cll_inline(child, pending_cukta_scroll, base_path, script, lojban_context, page_find) }
                }
            }
        },
        CllInline::Subscript { inlines } => rsx! {
            sub {
                for child in inlines.iter() {
                    { render_cll_inline(child, pending_cukta_scroll, base_path, script, lojban_context, page_find) }
                }
            }
        },
        CllInline::Superscript { inlines } => rsx! {
            sup {
                for child in inlines.iter() {
                    { render_cll_inline(child, pending_cukta_scroll, base_path, script, lojban_context, page_find) }
                }
            }
        },
        CllInline::Link {
            target,
            inlines,
            kind,
        } => {
            let href = cll_inline_href(base_path, *kind, target);
            let class_name = format!("spa-cll-link {}", cll_link_kind_class(*kind));
            let tooltip = cll_dictionary_tooltip_for_link(base_path, *kind, target);
            let child_context = lojban_context || cll_link_text_is_lojban(*kind);
            let route = jbotci_route_from_href(base_path, &href).map(|route| {
                let pending_scroll =
                    cukta_pending_scroll_for_explicit_route_link(base_path, &route);
                let click_route = route.clone();
                (route, click_route, pending_scroll)
            });
            if let Some(card) = &tooltip {
                rsx! {
                    span { class: "dictionary-tooltip-host",
                        if let Some((route, click_route, pending_scroll)) = route {
                            Link {
                                class: "{class_name}",
                                to: route,
                                onclick_only: true,
                                onclick: move |_| {
                                    push_route_with_cukta_scroll_intent(
                                        pending_cukta_scroll,
                                        pending_scroll.clone(),
                                        click_route.clone(),
                                    );
                                },
                                for child in inlines.iter() {
                                    { render_cll_inline(child, pending_cukta_scroll, base_path, script, child_context, page_find) }
                                }
                            }
                        } else {
                            a {
                                class: "{class_name}",
                                href: "{href}",
                                for child in inlines.iter() {
                                    { render_cll_inline(child, pending_cukta_scroll, base_path, script, child_context, page_find) }
                                }
                            }
                        }
                        { render_dictionary_tooltip(card, false, base_path, script) }
                    }
                }
            } else {
                if let Some((route, click_route, pending_scroll)) = route {
                    rsx! {
                        Link {
                            class: "{class_name}",
                            to: route,
                            onclick_only: true,
                            onclick: move |_| {
                                push_route_with_cukta_scroll_intent(
                                    pending_cukta_scroll,
                                    pending_scroll.clone(),
                                    click_route.clone(),
                                );
                            },
                                for child in inlines.iter() {
                                    { render_cll_inline(child, pending_cukta_scroll, base_path, script, child_context, page_find) }
                                }
                            }
                    }
                } else {
                    rsx! {
                        a {
                            class: "{class_name}",
                            href: "{href}",
                                for child in inlines.iter() {
                                    { render_cll_inline(child, pending_cukta_scroll, base_path, script, child_context, page_find) }
                                }
                            }
                    }
                }
            }
        }
        CllInline::Code(text) => rsx! { code { { render_page_find_text(page_find, text) } } },
        CllInline::Elidable {
            shown,
            forced,
            inlines,
        } => {
            let class_name = class_names("cll-elidable", &[("cll-elidable-forced", *forced)]);
            rsx! {
                span { class: "{class_name}",
                    if inlines.is_empty() {
                        { render_page_find_text(page_find, &display_lojban_text_if(script, shown, lojban_context)) }
                    } else {
                        for child in inlines.iter() {
                            { render_cll_inline(child, pending_cukta_scroll, base_path, script, lojban_context, page_find) }
                        }
                    }
                }
            }
        }
        CllInline::InlineMath { markup, .. } => rsx! {
            span { class: "cll-inline-math", dangerous_inner_html: "{markup}" }
        },
        CllInline::Anchor { id } => rsx! { span { id: "{id}" } },
    }
}

#[requires(true)]
#[ensures(true)]
fn display_lojban_text(script: GentufaScript, text: &str) -> String {
    render_lojban_text_for_script(text, script, display_lojban_phoneme_options())
        .unwrap_or_else(|_| text.to_owned())
}

#[requires(true)]
#[ensures(true)]
fn display_lojban_text_if(script: GentufaScript, text: &str, lojban_context: bool) -> String {
    if lojban_context {
        display_lojban_text(script, text)
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(!matches!(ret.mark_stress, StressMark::Acute | StressMark::Caps))]
#[ensures(ret.mark_glides == GlideMark::Breve)]
fn display_lojban_phoneme_options() -> PhonemeRenderOptions {
    PhonemeRenderOptions {
        mark_stress: StressMark::None,
        mark_glides: GlideMark::Breve,
    }
}

#[requires(true)]
#[ensures(ret == language.is_some_and(|language| language.eq_ignore_ascii_case("jbo") || language.eq_ignore_ascii_case("lojban")))]
fn cll_language_is_lojban(language: Option<&str>) -> bool {
    language.is_some_and(|language| {
        language.eq_ignore_ascii_case("jbo") || language.eq_ignore_ascii_case("lojban")
    })
}

#[requires(true)]
#[ensures(true)]
fn cll_link_text_is_lojban(kind: CllLinkKind) -> bool {
    matches!(
        kind,
        CllLinkKind::Dictionary | CllLinkKind::Rafsi | CllLinkKind::Parse
    )
}

#[requires(true)]
#[ensures(true)]
fn cll_kind_is_lojban(kind: &str) -> bool {
    matches!(kind, "jbo" | "jbophrase" | "veljvo" | "rafsi")
}

#[requires(true)]
#[ensures(true)]
fn cll_display_text_for_kind(script: GentufaScript, kind: &str, text: &str) -> String {
    display_lojban_text_if(script, text, cll_kind_is_lojban(kind))
}

#[requires(true)]
#[ensures(true)]
fn render_cll_interlinear(
    id: Option<&str>,
    aligned: bool,
    itemized: bool,
    parse_href: Option<&str>,
    rows: &[CllInterlinearRow],
    natlang: &[Vec<CllInline>],
    comments: &[Vec<CllInline>],
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let class_name = class_names(
        "cll-interlinear",
        &[("cll-interlinear-aligned", aligned || itemized)],
    );
    let table_class = class_names(
        "cll-interlinear-table",
        &[("cll-interlinear-table-plain", aligned && !itemized)],
    );
    rsx! {
        div { id: id.unwrap_or_default(), class: "{class_name}",
            if let Some(parse_href) = parse_href {
                { render_cll_parse_link(
                    "cll-parse-example spa-cll-link spa-cll-link-parse",
                    parse_href,
                    base_path,
                ) }
            }
            if !rows.is_empty() {
                if aligned {
                    table { class: "{table_class}",
                        tbody {
                            for row in rows.iter() {
                                {
                                    let row_context = cll_kind_is_lojban(&row.kind);
                                    rsx! {
                                        tr { class: "cll-ig-row cll-ig-{row.kind} cll-interlinear-row cll-interlinear-row-{row.kind}",
                                            for cell in row.cells.iter() {
                                                td { class: "cll-ig-cell",
                                                    for inline in cell.iter() {
                                                        { render_cll_inline(inline, pending_cukta_scroll, base_path, script, row_context, page_find) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "cll-interlinear-itemized",
                        for row in rows.iter() {
                            {
                                let row_context = cll_kind_is_lojban(&row.kind);
                                rsx! {
                                    div { class: "cll-ig-line-wrap",
                                        p { class: "cll-ig-line cll-ig-inline cll-ig-{row.kind}",
                                            for cell in row.cells.iter() {
                                                for inline in cell.iter() {
                                                    { render_cll_inline(inline, pending_cukta_scroll, base_path, script, row_context, page_find) }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            for comment in comments.iter() {
                p { class: "cll-ig-comment cll-interlinear-comment",
                    for inline in comment.iter() {
                        { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                    }
                }
            }
            for line in natlang.iter() {
                p { class: "cll-ig-natlang-text cll-natlang",
                    for inline in line.iter() {
                        { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_cmavo_list(
    id: Option<&str>,
    titles: &[Vec<CllInline>],
    headers: &[Vec<CllInline>],
    rows: &[Vec<Vec<CllInline>>],
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        div { id: id.unwrap_or_default(), class: "cll-cmavo-list",
            for title in titles.iter() {
                p { class: "cll-cmavo-list-title",
                    for inline in title.iter() {
                        { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                    }
                }
            }
            table {
                tbody {
                    if !headers.is_empty() {
                        tr {
                            for header in headers.iter() {
                                th {
                                    for inline in header.iter() {
                                        { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                                    }
                                }
                            }
                        }
                    }
                    for row in rows.iter() {
                        tr {
                            for cell in row.iter() {
                                td {
                                    for inline in cell.iter() {
                                        { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_lojbanization(
    id: Option<&str>,
    lines: &[CllLojbanizationLine],
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        table { id: id.unwrap_or_default(), class: "cll-lojbanization cll-lojbanization-table",
            tbody {
                for line in lines.iter() {
                    {
                        let line_context = cll_kind_is_lojban(&line.kind);
                        rsx! {
                            tr { class: "cll-lojbanization-row cll-lojbanization-line cll-lojbanization-line-{line.kind} cll-lojbanization-{line.kind}",
                                th { { render_page_find_text(page_find, &line.kind) } }
                                td {
                                    for inline in line.body.iter() {
                                        { render_cll_inline(inline, pending_cukta_scroll, base_path, script, line_context, page_find) }
                                    }
                                }
                                td {
                                    if let Some(comment) = &line.comment {
                                        for inline in comment.iter() {
                                            { render_cll_inline(inline, pending_cukta_scroll, base_path, script, false, page_find) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_lujvo_making(
    id: Option<&str>,
    parts: &[CllLujvoPart],
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        ul { id: id.unwrap_or_default(), class: "cll-lujvo-making",
            for part in parts.iter() {
                {
                    let part_context = cll_kind_is_lojban(&part.kind);
                        rsx! {
                            li { class: "cll-lujvo-part cll-lujvo-part-{part.kind}",
                            span { class: "cll-lujvo-part-kind",
                                { render_page_find_text(page_find, &part.kind) }
                            }
                            for inline in part.body.iter() {
                                { render_cll_inline(inline, pending_cukta_scroll, base_path, script, part_context, page_find) }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_ebnf(
    id: Option<&str>,
    entries: &[CllEbnfEntry],
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        div { id: id.unwrap_or_default(), class: "cll-ebnf",
            for entry in entries.iter() {
                section { id: "{entry.anchor_id}", class: "cll-ebnf-entry",
                    div { class: "cll-ebnf-head",
                        { render_cll_ebnf_link("cll-ebnf-rule", &entry.rule_name, entry.rule_href.as_deref(), pending_cukta_scroll, base_path, script, page_find) }
                        " "
                        span { class: "cll-ebnf-assign", "⩴" }
                    }
                    pre { class: "cll-ebnf-rhs",
                        { render_cll_ebnf_rhs(&entry.rhs, pending_cukta_scroll, base_path, script, page_find) }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_ebnf_rhs(
    tokens: &[CllEbnfToken],
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let lines = wrap_ebnf_choice_lines(tokens);
    if lines.len() == 1 {
        let line = lines.into_iter().next().unwrap_or_default();
        return rsx! {
            for token in line.iter() {
                { render_cll_ebnf_token(token, pending_cukta_scroll, base_path, script, page_find) }
            }
        };
    }
    rsx! {
        for line in lines.iter() {
            span { class: "cll-ebnf-choice-line",
                for token in line.iter() {
                    { render_cll_ebnf_token(token, pending_cukta_scroll, base_path, script, page_find) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_ebnf_token(
    token: &CllEbnfToken,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    match token {
        CllEbnfToken::Text { body } => rsx! { { render_page_find_text(page_find, body) } },
        CllEbnfToken::Operator { body } => {
            rsx! { span { class: "cll-ebnf-op", { render_page_find_text(page_find, body) } } }
        }
        CllEbnfToken::Hash { body } => {
            rsx! { span { class: "cll-ebnf-hash", { render_page_find_text(page_find, body) } } }
        }
        CllEbnfToken::Terminal { body, href } => render_cll_ebnf_link(
            "cll-ebnf-terminal",
            body,
            href.as_deref(),
            pending_cukta_scroll,
            base_path,
            script,
            page_find,
        ),
        CllEbnfToken::ElidableTerminator { body, href } => render_cll_ebnf_elidable(
            body,
            href.as_deref(),
            pending_cukta_scroll,
            base_path,
            script,
            page_find,
        ),
        CllEbnfToken::Nonterminal { body, href } => render_cll_ebnf_link(
            "cll-ebnf-nonterminal",
            body,
            href.as_deref(),
            pending_cukta_scroll,
            base_path,
            script,
            page_find,
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_ebnf_elidable(
    body: &str,
    href: Option<&str>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let pieces = cll_ebnf_elidable_hash_pieces(body);
    if let Some(href) = href {
        let tooltip = cll_dictionary_tooltip_for_href(base_path, href);
        let href = cll_ebnf_href(base_path, href);
        let route = jbotci_route_from_href(base_path, &href).map(|route| {
            let pending_scroll = cukta_pending_scroll_for_explicit_route_link(base_path, &route);
            let click_route = route.clone();
            (route, click_route, pending_scroll)
        });
        if let Some(card) = &tooltip {
            rsx! {
                span { class: "dictionary-tooltip-host",
                    if let Some((route, click_route, pending_scroll)) = route {
                        Link {
                            class: "cll-ebnf-elidable",
                            to: route,
                            onclick_only: true,
                            onclick: move |_| {
                                push_route_with_cukta_scroll_intent(
                                    pending_cukta_scroll,
                                    pending_scroll.clone(),
                                    click_route.clone(),
                                );
                            },
                            if let Some((prefix, suffix)) = pieces {
                                { render_page_find_text(page_find, &prefix) }
                                span { class: "cll-ebnf-hash", { render_page_find_text(page_find, "#") } }
                                { render_page_find_text(page_find, &suffix) }
                            } else {
                                { render_page_find_text(page_find, body) }
                            }
                        }
                    } else {
                        a { class: "cll-ebnf-elidable", href: "{href}",
                            if let Some((prefix, suffix)) = pieces {
                                { render_page_find_text(page_find, &prefix) }
                                span { class: "cll-ebnf-hash", { render_page_find_text(page_find, "#") } }
                                { render_page_find_text(page_find, &suffix) }
                            } else {
                                { render_page_find_text(page_find, body) }
                            }
                        }
                    }
                    { render_dictionary_tooltip(card, false, base_path, script) }
                }
            }
        } else {
            if let Some((route, click_route, pending_scroll)) = route {
                rsx! {
                    Link {
                        class: "cll-ebnf-elidable",
                        to: route,
                        onclick_only: true,
                        onclick: move |_| {
                            push_route_with_cukta_scroll_intent(
                                pending_cukta_scroll,
                                pending_scroll.clone(),
                                click_route.clone(),
                            );
                        },
                        if let Some((prefix, suffix)) = pieces {
                            { render_page_find_text(page_find, &prefix) }
                            span { class: "cll-ebnf-hash", { render_page_find_text(page_find, "#") } }
                            { render_page_find_text(page_find, &suffix) }
                        } else {
                            { render_page_find_text(page_find, body) }
                        }
                    }
                }
            } else {
                rsx! {
                    a { class: "cll-ebnf-elidable", href: "{href}",
                        if let Some((prefix, suffix)) = pieces {
                            { render_page_find_text(page_find, &prefix) }
                            span { class: "cll-ebnf-hash", { render_page_find_text(page_find, "#") } }
                            { render_page_find_text(page_find, &suffix) }
                        } else {
                            { render_page_find_text(page_find, body) }
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            span { class: "cll-ebnf-elidable",
                if let Some((prefix, suffix)) = pieces {
                    { render_page_find_text(page_find, &prefix) }
                    span { class: "cll-ebnf-hash", { render_page_find_text(page_find, "#") } }
                    { render_page_find_text(page_find, &suffix) }
                } else {
                    { render_page_find_text(page_find, body) }
                }
            }
        }
    }
}

#[requires(!class_name.is_empty())]
#[ensures(true)]
fn render_cll_ebnf_link(
    class_name: &str,
    body: &str,
    href: Option<&str>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    if let Some(href) = href {
        let tooltip = cll_dictionary_tooltip_for_href(base_path, href);
        let href = cll_ebnf_href(base_path, href);
        let route = jbotci_route_from_href(base_path, &href).map(|route| {
            let pending_scroll = cukta_pending_scroll_for_explicit_route_link(base_path, &route);
            let click_route = route.clone();
            (route, click_route, pending_scroll)
        });
        if let Some(card) = &tooltip {
            rsx! {
                span { class: "dictionary-tooltip-host",
                    if let Some((route, click_route, pending_scroll)) = route {
                        Link {
                            class: "{class_name}",
                            to: route,
                            onclick_only: true,
                            onclick: move |_| {
                                push_route_with_cukta_scroll_intent(
                                    pending_cukta_scroll,
                                    pending_scroll.clone(),
                                    click_route.clone(),
                                );
                            },
                            { render_page_find_text(page_find, body) }
                        }
                    } else {
                        a { class: "{class_name}", href: "{href}", { render_page_find_text(page_find, body) } }
                    }
                    { render_dictionary_tooltip(card, false, base_path, script) }
                }
            }
        } else {
            if let Some((route, click_route, pending_scroll)) = route {
                rsx! {
                    Link {
                        class: "{class_name}",
                        to: route,
                        onclick_only: true,
                        onclick: move |_| {
                            push_route_with_cukta_scroll_intent(
                                pending_cukta_scroll,
                                pending_scroll.clone(),
                                click_route.clone(),
                            );
                        },
                        { render_page_find_text(page_find, body) }
                    }
                }
            } else {
                rsx! {
                    a { class: "{class_name}", href: "{href}", { render_page_find_text(page_find, body) } }
                }
            }
        }
    } else {
        rsx! { span { class: "{class_name}", { render_page_find_text(page_find, body) } } }
    }
}

#[requires(!class_name.is_empty())]
#[ensures(true)]
fn render_cll_parse_link(class_name: &str, href: &str, base_path: &str) -> Element {
    let href = cll_parse_href(base_path, href);
    if let Some(route) = jbotci_route_from_href(base_path, &href) {
        rsx! {
            Link {
                class: "{class_name}",
                to: route,
                "Parse"
            }
        }
    } else {
        rsx! {
            a {
                class: "{class_name}",
                href: "{href}",
                "Parse"
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn cll_dictionary_tooltip_for_link(
    base_path: &str,
    kind: CllLinkKind,
    target: &str,
) -> Option<DictionaryTooltipCard> {
    match kind {
        CllLinkKind::Dictionary => dictionary_tooltip_for_word(base_path, target),
        CllLinkKind::Rafsi => dictionary_tooltip_for_rafsi(base_path, target),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn cll_dictionary_tooltip_for_href(base_path: &str, href: &str) -> Option<DictionaryTooltipCard> {
    if let Some(target) = href.strip_prefix("../vlacku/") {
        return dictionary_tooltip_for_word(base_path, target);
    }
    let Some(query) = href.strip_prefix("../vlacku?") else {
        return None;
    };
    let mut mode_is_rafsi = false;
    let mut rafsi = None;
    for part in query.split('&') {
        if part == "mode=rafsi" {
            mode_is_rafsi = true;
        } else if let Some(value) = part.strip_prefix("q=") {
            rafsi = Some(value);
        }
    }
    if mode_is_rafsi {
        rafsi.and_then(|value| dictionary_tooltip_for_rafsi(base_path, value))
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn cll_ebnf_elidable_hash_pieces(body: &str) -> Option<(String, String)> {
    let inner = body.strip_prefix('/')?.strip_suffix('/')?;
    let inner_without_hash = inner.strip_suffix('#')?;
    Some((format!("/{inner_without_hash}"), "/".to_owned()))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn cll_table_class(classes: &[String]) -> String {
    let mut class_name = String::from("cll-table");
    for class in classes {
        class_name.push(' ');
        class_name.push_str("cll-table-");
        class_name.push_str(class);
    }
    class_name
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn cll_language_span_class(kind: CllLanguageSpanKind) -> &'static str {
    match kind {
        CllLanguageSpanKind::ForeignPhrase => "spa-cll-foreignphrase",
        CllLanguageSpanKind::JboPhrase => "spa-cll-jbophrase",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn cll_link_kind_class(kind: CllLinkKind) -> &'static str {
    match kind {
        CllLinkKind::Section => "spa-cll-link-section",
        CllLinkKind::Example => "spa-cll-link-example",
        CllLinkKind::Dictionary => "spa-cll-link-dictionary",
        CllLinkKind::Rafsi => "spa-cll-link-rafsi",
        CllLinkKind::Parse => "spa-cll-link-parse",
        CllLinkKind::Asset => "spa-cll-link-asset",
        CllLinkKind::External => "spa-cll-link-external",
    }
}

#[requires(true)]
#[ensures(true)]
fn cll_table_row_parse_class(row: &[CllTableCell]) -> String {
    let Some(group) = cll_table_row_parse_group(row) else {
        return String::new();
    };
    let mut classes = vec!["cll-parse-group-row"];
    if group.row_count > 1 {
        classes.push("cll-parse-group-multi");
    }
    if group.row_index == 0 {
        classes.push("cll-parse-group-start");
    }
    if group.row_index + 1 == group.row_count {
        classes.push("cll-parse-group-end");
    }
    if group.row_index > 0 {
        classes.push("cll-parse-group-continuation");
    }
    classes.join(" ")
}

#[requires(true)]
#[ensures(true)]
fn cll_table_row_parse_group_id(row: &[CllTableCell]) -> Option<String> {
    cll_table_row_parse_group(row).map(|group| group.group_id.clone())
}

#[requires(true)]
#[ensures(true)]
fn cll_table_row_parse_group(row: &[CllTableCell]) -> Option<&jbotci_cll::CllTableParseGroup> {
    row.first().and_then(|cell| cell.parse_group.as_ref())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn cll_table_cell_parse_link_class(cell: &CllTableCell) -> String {
    let mut class_name =
        "cll-parse-example cll-parse-snippet spa-cll-link spa-cll-link-parse".to_owned();
    if cell
        .parse_group
        .as_ref()
        .is_some_and(|group| group.row_count > 1)
    {
        class_name.push_str(" cll-parse-group-link");
    }
    class_name
}

#[requires(true)]
#[ensures(true)]
fn cll_parse_href(base_path: &str, href: &str) -> String {
    if let Some(query) = href.strip_prefix("../gentufa") {
        format!("{}/gentufa{query}", base_path.trim_end_matches('/'))
    } else {
        href.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn cll_ebnf_href(base_path: &str, href: &str) -> String {
    let prefix = base_path.trim_end_matches('/');
    if let Some(target) = href.strip_prefix("../vlacku/") {
        format!("{prefix}/vlacku/{target}")
    } else if let Some(section) = href.strip_prefix("section/") {
        format!("{prefix}/cukta/section/{section}")
    } else {
        href.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn cll_inline_href(base_path: &str, kind: CllLinkKind, target: &str) -> String {
    let prefix = base_path.trim_end_matches('/');
    match kind {
        CllLinkKind::Dictionary => format!("{prefix}/vlacku/{target}"),
        CllLinkKind::Rafsi => vlacku_web_url(
            base_path,
            &VlackuWebState {
                mode: VlackuWebMode::Rafsi,
                query: target.to_owned(),
                count: VLACKU_WEB_DEFAULT_COUNT,
                word_types: Vec::new(),
            },
        ),
        CllLinkKind::Parse => gentufa_web_url(
            base_path,
            &GentufaWebState {
                text: target.to_owned(),
                dialect: None,
                view_mode: GentufaWebViewMode::Blocks,
                show_elided: false,
                show_glosses: false,
            },
        ),
        CllLinkKind::Asset => cll_asset_href(base_path, target),
        CllLinkKind::Section | CllLinkKind::Example => embedded_cll_site()
            .map(|site| {
                let relative = cll_link_href(site, kind, target);
                if let Some(section) = relative.strip_prefix("section/") {
                    format!("{prefix}/cukta/section/{section}")
                } else {
                    relative
                }
            })
            .unwrap_or_else(|_| format!("{prefix}/cukta/section/{target}")),
        CllLinkKind::External => target.to_owned(),
    }
}

#[requires(true)]
#[ensures(true)]
fn cukta_section_reference_from_href(href: &str) -> Option<String> {
    let without_hash = href.split('#').next().unwrap_or(href);
    if let Some(reference) = without_hash
        .rsplit_once("/cukta/section/")
        .map(|(_, value)| value)
    {
        return (!reference.is_empty()).then(|| reference.to_owned());
    }
    if let Some(reference) = without_hash.strip_prefix("section/") {
        return (!reference.is_empty()).then(|| reference.to_owned());
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn cukta_anchor_from_href(href: &str) -> Option<String> {
    href.split_once('#')
        .map(|(_, anchor)| anchor)
        .filter(|anchor| !anchor.is_empty())
        .map(str::to_owned)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn scroll_to_cukta_href(href: &str) {
    let Some(anchor) = cukta_anchor_from_href(href) else {
        return;
    };
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move || {
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            if let Some(element) = document.get_element_by_id(&anchor) {
                scroll_to_cukta_anchor_element(&element);
            }
        }
    });
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        30,
    );
    closure.forget();
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn scroll_to_cukta_href(href: &str) {
    let _ = href;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn save_cukta_toc_scroll() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(Some(element)) = document.query_selector("[data-cukta-toc-scroll='1']") else {
        return;
    };
    if let Some(element) = element.dyn_ref::<web_sys::HtmlElement>() {
        session_storage_set(
            "jbotci.cukta.toc.scroll.v1",
            &element.scroll_top().to_string(),
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn save_cukta_toc_scroll() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn restore_cukta_toc_scroll() {
    let Some(raw) = session_storage_get("jbotci.cukta.toc.scroll.v1") else {
        return;
    };
    let Ok(scroll_top) = raw.parse::<i32>() else {
        return;
    };
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move || {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let Ok(Some(element)) = document.query_selector("[data-cukta-toc-scroll='1']") else {
            return;
        };
        if let Some(element) = element.dyn_ref::<web_sys::HtmlElement>() {
            element.set_scroll_top(scroll_top);
        }
    });
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        30,
    );
    closure.forget();
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn restore_cukta_toc_scroll() {}

#[requires(true)]
#[ensures(true)]
fn cll_asset_href(base_path: &str, src: &str) -> String {
    let media_name = src
        .trim_start_matches("assets/media/")
        .trim_start_matches("media/")
        .trim_start_matches("assets/cll/media/")
        .trim_start_matches("cll/media/");
    if let Some(href) = cll_known_media_href(media_name) {
        return href;
    }
    format!(
        "{}/assets/cll/{}",
        base_path.trim_end_matches('/'),
        src.trim_start_matches("assets/")
    )
}

#[requires(true)]
#[ensures(true)]
fn cll_known_media_href(file_name: &str) -> Option<String> {
    match file_name {
        "chapter-2-diagram.svg.png" => Some(format!("{CLL_MEDIA_CHAPTER_2_DIAGRAM}")),
        "chapter-about.svg.png" => Some(format!("{CLL_MEDIA_CHAPTER_ABOUT}")),
        "chapter-abstractions.svg.png" => Some(format!("{CLL_MEDIA_CHAPTER_ABSTRACTIONS}")),
        "chapter-anaphoric-cmavo.svg.png" => Some(format!("{CLL_MEDIA_CHAPTER_ANAPHORIC_CMAVO}")),
        "chapter-attitudinals.gif" => Some(format!("{CLL_MEDIA_CHAPTER_ATTITUDINALS}")),
        "chapter-catalogue.svg.png" => Some(format!("{CLL_MEDIA_CHAPTER_CATALOGUE}")),
        "chapter-connectives.svg.png" => Some(format!("{CLL_MEDIA_CHAPTER_CONNECTIVES}")),
        "chapter-grammars.svg.png" => Some(format!("{CLL_MEDIA_CHAPTER_GRAMMARS}")),
        "chapter-letterals.svg.png" => Some(format!("{CLL_MEDIA_CHAPTER_LETTERALS}")),
        "chapter-lujvo.svg.png" => Some(format!("{CLL_MEDIA_CHAPTER_LUJVO}")),
        "chapter-mekso.gif" => Some(format!("{CLL_MEDIA_CHAPTER_MEKSO}")),
        "chapter-morphology.gif" => Some(format!("{CLL_MEDIA_CHAPTER_MORPHOLOGY}")),
        "chapter-negation.gif" => Some(format!("{CLL_MEDIA_CHAPTER_NEGATION}")),
        "chapter-phonology.gif" => Some(format!("{CLL_MEDIA_CHAPTER_PHONOLOGY}")),
        "chapter-quantifiers.gif" => Some(format!("{CLL_MEDIA_CHAPTER_QUANTIFIERS}")),
        "chapter-relative-clauses.svg.png" => Some(format!("{CLL_MEDIA_CHAPTER_RELATIVE_CLAUSES}")),
        "chapter-selbri.svg.png" => Some(format!("{CLL_MEDIA_CHAPTER_SELBRI}")),
        "chapter-structure.svg.png" => Some(format!("{CLL_MEDIA_CHAPTER_STRUCTURE}")),
        "chapter-sumti.gif" => Some(format!("{CLL_MEDIA_CHAPTER_SUMTI}")),
        "chapter-sumti-tcita.gif" => Some(format!("{CLL_MEDIA_CHAPTER_SUMTI_TCITA}")),
        "chapter-tenses.gif" => Some(format!("{CLL_MEDIA_CHAPTER_TENSES}")),
        "chapter-tour.svg.png" => Some(format!("{CLL_MEDIA_CHAPTER_TOUR}")),
        "logo.png" => Some(format!("{CLL_MEDIA_LOGO}")),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn cukta_search_draft_for_page(
    draft_state: &CuktaWebState,
    committed_search: &CuktaWebSearchState,
) -> CuktaWebSearchState {
    if let CuktaWebView::Search(search) = &draft_state.view {
        search.clone()
    } else {
        committed_search.clone()
    }
}

#[requires(true)]
#[ensures(ret.len() == 2)]
fn cukta_draft_mode_options(selected: CuktaWebMode) -> Vec<CuktaModeOption> {
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
#[ensures(ret.len() == 3)]
fn cukta_draft_target_options(selected_targets: &[String]) -> Vec<CuktaTargetOption> {
    [
        ("section", "Sections"),
        ("paragraph", "Paragraphs"),
        ("example", "Examples"),
    ]
    .iter()
    .map(|(value, label)| CuktaTargetOption {
        value: (*value).to_owned(),
        label: (*label).to_owned(),
        selected: selected_targets.iter().any(|target| target == value),
    })
    .collect()
}

#[requires(true)]
#[ensures(ret.query == query)]
#[ensures(ret.count == CUKTA_WEB_DEFAULT_COUNT)]
fn cukta_search_state_with_query(state: &CuktaWebSearchState, query: &str) -> CuktaWebSearchState {
    CuktaWebSearchState {
        mode: state.mode,
        query: query.to_owned(),
        count: CUKTA_WEB_DEFAULT_COUNT,
        targets: state.targets.clone(),
    }
}

#[requires(true)]
#[ensures(true)]
fn set_cukta_state_immediate(
    mut draft_state: Signal<CuktaWebState>,
    mut committed_state: Signal<CuktaWebState>,
    state: CuktaWebState,
) {
    clear_cukta_search_timer();
    draft_state.set(state.clone());
    committed_state.set(state);
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_page(
    vlacku_draft_state: Signal<VlackuWebState>,
    vlacku_committed_state: Signal<VlackuWebState>,
    vlacku_result: Signal<VlackuAsyncResultState>,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: Signal<bool>,
    jvozba_drag: Signal<Option<VlackuJvozbaDragState>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    pending_vlacku_scroll_restore: Signal<Option<i32>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let committed_state = vlacku_committed_state.read().clone();
    let result_state = vlacku_result.read().clone();
    let result = if result_state.state.as_ref() == Some(&committed_state) {
        result_state.result
    } else {
        vlacku_loading_result(&committed_state, "Loading dictionary results.")
    };
    // Keep result cards out of the draft-query dependency path; the focused input
    // already reflects keystrokes until the debounced committed state catches up.
    let draft_state = vlacku_draft_state.peek().clone();
    let word_type_options = vlacku_word_type_options(&draft_state.word_types);
    let jvozba_available_value = *jvozba_available.read();
    let jvozba_open = jvozba_available_value && jvozba_pane.read().open;
    let shell_class = class_names(
        "dictionary-shell",
        &[
            ("dictionary-jvozba-available", jvozba_available_value),
            ("dictionary-jvozba-hints-active", jvozba_open),
        ],
    );
    rsx! {
        section { class: "spa-page dictionary-page vlacku-page",
            h1 { class: "sr-only", "jbotci vlacku" }
            div { class: "{shell_class}",
                { render_vlacku_controls(vlacku_draft_state, vlacku_committed_state, &draft_state, &word_type_options) }
                if let Some(info) = &result.dictionary_info {
                    { render_dictionary_info(info) }
                }
                if let Some(message) = &result.message {
                    { render_semantic_search_message("dictionary-empty", message, Some(page_find)) }
                }
                for error in result.errors.iter() {
                    div { class: "spa-error dictionary-error",
                        { render_page_find_text(page_find, error) }
                    }
                }
                div { class: "dictionary-layout",
                    div { class: "dictionary-main-column",
                        { render_vlacku_body(&result, vlacku_draft_state, vlacku_committed_state, jvozba_pane, jvozba_available_value, pending_cukta_scroll, pending_vlacku_scroll_restore, base_path, script, page_find) }
                    }
                    if jvozba_available_value {
                        { render_vlacku_jvozba_pane(jvozba_pane, jvozba_drag, script) }
                    }
                }
            }
        }
    }
}

#[requires(!class_name.is_empty())]
#[ensures(true)]
fn render_semantic_search_message(
    class_name: &str,
    message: &str,
    page_find: Option<&PageFindContext>,
) -> Element {
    if message == SEMANTIC_SEARCH_SETUP_MESSAGE {
        let settings_route = JbotciRoute::from_web_route(WebRoute::Settings, false);
        rsx! {
            p { class: "{class_name}",
                Link {
                    to: settings_route,
                    if let Some(page_find) = page_find {
                        { render_page_find_text(page_find, SEMANTIC_SEARCH_SETUP_LINK_LABEL) }
                    } else {
                        "{SEMANTIC_SEARCH_SETUP_LINK_LABEL}"
                    }
                }
                if let Some(page_find) = page_find {
                    { render_page_find_text(page_find, SEMANTIC_SEARCH_SETUP_LINK_SUFFIX) }
                } else {
                    "{SEMANTIC_SEARCH_SETUP_LINK_SUFFIX}"
                }
            }
        }
    } else {
        rsx! {
            p { class: "{class_name}",
                if let Some(page_find) = page_find {
                    { render_page_find_text(page_find, message) }
                } else {
                    "{message}"
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_controls(
    mut vlacku_draft_state: Signal<VlackuWebState>,
    vlacku_committed_state: Signal<VlackuWebState>,
    state: &VlackuWebState,
    word_type_options: &[VlackuWordTypeOption],
) -> Element {
    rsx! {
        div { class: "dictionary-form",
            div { class: "dictionary-controls",
                div { class: "dictionary-mode-control",
                    div { class: "mode-toggle-row",
                        div { class: "mode-selector-wrap",
                            div { class: "mode-bracket-row", aria_hidden: "true",
                                span { class: "mode-bracket-label", "similar" }
                                span { class: "mode-bracket-label", "exact" }
                            }
                            div { class: "mode-toggle-group", role: "group", aria_label: "Dictionary search mode",
                                { render_vlacku_mode_button(vlacku_draft_state, vlacku_committed_state, state.mode, VlackuWebMode::Meaning, "meaning", false) }
                                { render_vlacku_mode_button(vlacku_draft_state, vlacku_committed_state, state.mode, VlackuWebMode::Sound, "sound", false) }
                                { render_vlacku_mode_button(vlacku_draft_state, vlacku_committed_state, state.mode, VlackuWebMode::Word, "word", false) }
                                { render_vlacku_mode_button(vlacku_draft_state, vlacku_committed_state, state.mode, VlackuWebMode::Rafsi, "rafsi", false) }
                            }
                        }
                    }
                }
                div { class: "dictionary-word-type-control",
                    { render_vlacku_word_type_controls(vlacku_draft_state, vlacku_committed_state, word_type_options) }
                }
            }
            div { class: "dictionary-query-row",
                input {
                    class: "query-input",
                    r#type: "search",
                    aria_label: "Dictionary query",
                    placeholder: vlacku_query_placeholder(state.mode),
                    spellcheck: "false",
                    value: "{state.query}",
                    oninput: move |event| {
                        let mut next = vlacku_draft_state.read().clone();
                        next.query = event.value();
                        next.count = VLACKU_WEB_DEFAULT_COUNT;
                        vlacku_draft_state.set(next.clone());
                        schedule_vlacku_search_commit(vlacku_committed_state, next);
                    },
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_mode_button(
    mut vlacku_draft_state: Signal<VlackuWebState>,
    mut vlacku_committed_state: Signal<VlackuWebState>,
    current: VlackuWebMode,
    mode: VlackuWebMode,
    label: &'static str,
    disabled: bool,
) -> Element {
    rsx! {
        button {
            class: vlacku_mode_class(current == mode),
            r#type: "button",
            disabled,
            title: vlacku_mode_title(mode, disabled),
            aria_pressed: pressed_attr(current == mode),
            onclick: move |_| {
                if !disabled {
                    let mut next = vlacku_draft_state.read().clone();
                    next.mode = mode;
                    next.count = VLACKU_WEB_DEFAULT_COUNT;
                    set_vlacku_state_immediate(
                        &mut vlacku_draft_state,
                        &mut vlacku_committed_state,
                        next,
                    );
                }
            },
            "{label}"
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_word_type_controls(
    vlacku_draft_state: Signal<VlackuWebState>,
    vlacku_committed_state: Signal<VlackuWebState>,
    options: &[VlackuWordTypeOption],
) -> Element {
    rsx! {
        div { class: "word-type-grid", aria_label: "Word type filters",
            div { class: "word-type-divider", aria_hidden: "true" }
            div { class: "word-type-cell word-type-cell-brivla",
                { render_word_type_filter_value(vlacku_draft_state, vlacku_committed_state, options, "brivla") }
            }
            div { class: "word-type-cell word-type-cell-gismu",
                { render_word_type_filter_value(vlacku_draft_state, vlacku_committed_state, options, "gismu") }
            }
            div { class: "word-type-cell word-type-cell-cmavo",
                { render_word_type_filter_value(vlacku_draft_state, vlacku_committed_state, options, "cmavo") }
            }
            div { class: "word-type-cell word-type-cell-letteral",
                { render_word_type_filter_value(vlacku_draft_state, vlacku_committed_state, options, "letteral") }
            }
            div { class: "word-type-cell word-type-cell-fuhivla",
                { render_word_type_filter_value(vlacku_draft_state, vlacku_committed_state, options, "fu'ivla") }
            }
            div { class: "word-type-cell word-type-cell-lujvo",
                { render_word_type_filter_value(vlacku_draft_state, vlacku_committed_state, options, "lujvo") }
            }
            div { class: "word-type-cell word-type-cell-cmevla",
                { render_word_type_filter_value(vlacku_draft_state, vlacku_committed_state, options, "cmevla") }
            }
            div { class: "word-type-cell word-type-cell-phrase",
                { render_word_type_filter_value(vlacku_draft_state, vlacku_committed_state, options, "phrase") }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_word_type_filter_value(
    vlacku_draft_state: Signal<VlackuWebState>,
    vlacku_committed_state: Signal<VlackuWebState>,
    options: &[VlackuWordTypeOption],
    value: &'static str,
) -> Element {
    if let Some(option) = options.iter().find(|option| option.value == value) {
        render_word_type_filter(vlacku_draft_state, vlacku_committed_state, option)
    } else {
        rsx! {}
    }
}

#[requires(true)]
#[ensures(true)]
fn render_word_type_filter(
    mut vlacku_draft_state: Signal<VlackuWebState>,
    mut vlacku_committed_state: Signal<VlackuWebState>,
    option: &VlackuWordTypeOption,
) -> Element {
    let value = option.value.clone();
    let is_parent = value == "brivla";
    let filter_class = class_names(
        "compact-check",
        &[
            ("is-selected", option.selected),
            ("is-indeterminate", option.indeterminate),
        ],
    );
    rsx! {
        label {
            class: "{filter_class}",
            input {
                r#type: "checkbox",
                checked: option.selected,
                "data-brivla-toggle": if is_parent { "1" } else { "0" },
                "data-brivla-member": if option.section == VlackuWordTypeSection::Brivla && !is_parent { "1" } else { "0" },
                onchange: move |_| toggle_vlacku_word_type(
                    &mut vlacku_draft_state,
                    &mut vlacku_committed_state,
                    &value,
                ),
            }
            span { class: "vlacku-filter-label", "{option.label}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_body(
    result: &jbotci_web_core::VlackuWebResult,
    mut vlacku_draft_state: Signal<VlackuWebState>,
    mut vlacku_committed_state: Signal<VlackuWebState>,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    mut pending_vlacku_scroll_restore: Signal<Option<i32>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        div { class: "dictionary-results",
            if !result.cards.is_empty() {
                div { class: "dictionary-results-grid", "data-jvozba-pane-anchor": "1",
                    for card in result.cards.iter() {
                        { render_vlacku_card(card, jvozba_pane, jvozba_available, pending_cukta_scroll, base_path, script, page_find) }
                    }
                }
            }
            if result.has_more {
                div { class: "load-more-wrap",
                    button {
                        class: "btn-parse load-more-link",
                        r#type: "button",
                        onclick: move |_| {
                            pending_vlacku_scroll_restore.set(Some(current_scroll_y()));
                            let next = vlacku_load_more_state(&vlacku_draft_state.read());
                            set_vlacku_state_immediate(
                                &mut vlacku_draft_state,
                                &mut vlacku_committed_state,
                                next,
                            );
                        },
                        { render_page_find_text(page_find, "Load more") }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_dictionary_info(info: &VlackuDictionaryInfo) -> Element {
    rsx! {
        section { class: "dictionary-info-report",
            p { class: "dictionary-info-lede",
                "Serving dictionary entries from "
                a {
                    href: "https://lensisku.lojban.org",
                    title: "Open Lensisku",
                    "Lensisku"
                }
                " as of "
                time {
                    datetime: "{info.lensisku_created_at}",
                    "{info.lensisku_created_date}"
                }
                "."
            }
            ul { class: "dictionary-info-list",
                for node in info.count_tree.iter() {
                    { render_dictionary_count_node(node) }
                }
            }
            div { class: "dictionary-info-total",
                span { class: "dictionary-info-count-label", "total" }
                span { class: "dictionary-info-count-leader", aria_hidden: "true" }
                span { class: "dictionary-info-count-value", "{info.total_count}" }
            }
        }
    }
}

#[requires(!class_name.is_empty())]
#[ensures(true)]
fn render_text_route_link(class_name: &str, href: &str, base_path: &str, label: &str) -> Element {
    if let Some(route) = jbotci_route_from_href(base_path, href) {
        rsx! {
            Link {
                class: "{class_name}",
                to: route,
                "{label}"
            }
        }
    } else {
        rsx! {
            a {
                class: "{class_name}",
                href: "{href}",
                "{label}"
            }
        }
    }
}

#[requires(!class_name.is_empty())]
#[ensures(true)]
fn render_text_route_link_with_page_find(
    class_name: &str,
    href: &str,
    base_path: &str,
    label: &str,
    page_find: &PageFindContext,
) -> Element {
    if let Some(route) = jbotci_route_from_href(base_path, href) {
        rsx! {
            Link {
                class: "{class_name}",
                to: route,
                { render_page_find_text(page_find, label) }
            }
        }
    } else {
        rsx! {
            a {
                class: "{class_name}",
                href: "{href}",
                { render_page_find_text(page_find, label) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_dictionary_count_node(node: &VlackuDictionaryCountNode) -> Element {
    rsx! {
        li { class: "dictionary-info-count-item",
            div { class: "dictionary-info-count-row",
                span { class: "dictionary-info-count-label", "{node.label}" }
                span { class: "dictionary-info-count-leader", aria_hidden: "true" }
                span { class: "dictionary-info-count-value", "{node.count}" }
            }
            if !node.children.is_empty() {
                ul { class: "dictionary-info-list dictionary-info-sublist",
                    for child in node.children.iter() {
                        { render_dictionary_count_node(child) }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_card(
    card: &VlackuWebCard,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        section { class: "result-card",
            header { class: "result-header",
                { render_vlacku_headword_line(card, jvozba_pane, jvozba_available, base_path, script, page_find) }
                div { class: "tag-row",
                    if let Some(author) = &card.author {
                        { render_vlacku_author_credit(author, page_find) }
                    }
                    { render_vlacku_metadata_pill(card, pending_cukta_scroll, base_path, page_find) }
                }
            }
            if !card.definition.is_empty() {
                p { class: "dictionary-definition-copy",
                    { render_inline_spans(&card.definition, jvozba_pane, jvozba_available, base_path, script, page_find) }
                    {
                        let definition_source = card.definition_source.clone();
                        rsx! {
                            button {
                                class: "dictionary-definition-copy-button",
                                r#type: "button",
                                aria_label: "Copy definition",
                                title: "Copy definition",
                                onclick: move |_| copy_text_to_clipboard(&definition_source),
                                { render_copy_icon() }
                            }
                        }
                    }
                }
            }
            if !card.glosses.is_empty() {
                div { class: "chip-row dictionary-gloss-row",
                    for gloss in card.glosses.iter() {
                        span { class: "chip dictionary-gloss-pill", title: "Gloss word",
                            { render_page_find_text(page_find, gloss) }
                        }
                    }
                }
            }
            if !card.notes.is_empty() {
                p { class: "dictionary-note-copy", "data-note-tooltip": "Dictionary notes",
                    { render_inline_spans(&card.notes, jvozba_pane, jvozba_available, base_path, script, page_find) }
                }
            }
            if !card.etymology.is_empty() {
                p { class: "dictionary-etymology-copy", title: "Etymology",
                    span { class: "dictionary-detail-label",
                        { render_page_find_text(page_find, "etymology: ") }
                    }
                    { render_inline_spans(&card.etymology, jvozba_pane, jvozba_available, base_path, script, page_find) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_dictionary_tooltip(
    card: &DictionaryTooltipCard,
    show_link: bool,
    base_path: &str,
    script: GentufaScript,
) -> Element {
    let display_word = display_lojban_text(script, &card.display_word);
    rsx! {
        span { class: "rich-dictionary-tooltip", role: "tooltip",
            span { class: "tooltip-word-line",
                span { class: "tooltip-headword",
                    if show_link {
                        { render_text_route_link("tooltip-word", &card.href, base_path, &display_word) }
                    } else {
                        span { class: "tooltip-word", "{display_word}" }
                    }
                    if let Some(ipa) = &card.ipa {
                        span { class: "tooltip-ipa", "/{ipa}/" }
                    }
                }
                span { class: "tooltip-head-tags",
                    span { class: word_type_tag_class(&card.word_type_key), "{card.word_type}" }
                    if let Some(selmaho) = &card.selmaho {
                        span { class: "dictionary-meta-segment dictionary-selmaho-tag",
                            em { "{selmaho}" }
                        }
                    }
                }
            }
            if !card.decomposition.is_empty() {
                span { class: "tooltip-row tooltip-decomposition",
                    span { class: "tooltip-label", "decomposition" }
                    span { class: "tooltip-decomposition-pieces",
                        for piece in card.decomposition.iter().filter(|piece| piece.kind != VlackuCompositionPieceKind::Hyphen) {
                            {
                                let display_surface = display_lojban_text(script, &piece.display_surface);
                                if let Some(source) = &piece.source {
                                    let display_source = display_lojban_text(script, piece.display_source.as_deref().unwrap_or(source));
                                    if show_link {
                                        let href = piece.source_href.as_deref().unwrap_or(&card.href);
                                        if let Some(route) = jbotci_route_from_href(base_path, href) {
                                            rsx! {
                                                Link {
                                                    class: "tooltip-rafsi-piece",
                                                    to: route,
                                                    span { class: "tooltip-rafsi-surface", "{display_surface}" }
                                                    span { class: "tooltip-rafsi-source", "{display_source}" }
                                                }
                                            }
                                        } else {
                                            rsx! {
                                                a {
                                                    class: "tooltip-rafsi-piece",
                                                    href: "{href}",
                                                    span { class: "tooltip-rafsi-surface", "{display_surface}" }
                                                    span { class: "tooltip-rafsi-source", "{display_source}" }
                                                }
                                            }
                                        }
                                    } else {
                                        rsx! {
                                            span { class: "tooltip-rafsi-piece",
                                                span { class: "tooltip-rafsi-surface", "{display_surface}" }
                                                span { class: "tooltip-rafsi-source", "{display_source}" }
                                            }
                                        }
                                    }
                                } else {
                                    rsx! {
                                        span { class: "tooltip-rafsi-piece",
                                            span { class: "tooltip-rafsi-surface", "{display_surface}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !card.definition.is_empty() {
                span { class: "tooltip-copy",
                    { render_tooltip_inline_spans(&card.definition, base_path, show_link, script) }
                }
            }
            if !card.glosses.is_empty() {
                span { class: "tooltip-chip-row tooltip-glosses",
                    for gloss in card.glosses.iter() {
                        span { class: "tooltip-chip", "{gloss}" }
                    }
                }
            }
            if !card.notes.is_empty() {
                span { class: "tooltip-notes",
                    { render_tooltip_inline_spans(&card.notes, base_path, show_link, script) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_reference_tooltip(
    tooltip: &ReferenceTooltip,
    base_path: &str,
    script: GentufaScript,
) -> Element {
    rsx! {
        span { class: "rich-reference-tooltip-stack", role: "tooltip",
            if let Some(card) = &tooltip.card {
                { render_reference_dictionary_card(card, tooltip, base_path, script) }
            } else if let Some(word) = &tooltip.missing_word {
                {
                    let display_word = display_lojban_text(script, word);
                    rsx! {
                        span { class: "rich-dictionary-tooltip reference-missing-card",
                            span { class: "tooltip-word-line",
                                span { class: "tooltip-headword",
                                    span { class: "tooltip-word", "{display_word}" }
                                }
                            }
                            span { class: "tooltip-copy",
                                "No dictionary card available."
                            }
                        }
                    }
                }
            }
            for row in tooltip.rows.iter() {
                { render_reference_tooltip_row(row) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_reference_dictionary_card(
    card: &DictionaryTooltipCard,
    tooltip: &ReferenceTooltip,
    base_path: &str,
    script: GentufaScript,
) -> Element {
    let display_word = display_lojban_text(script, &card.display_word);
    rsx! {
        span { class: "rich-dictionary-tooltip reference-definition-card",
            span { class: "tooltip-word-line",
                span { class: "tooltip-headword",
                    span { class: "tooltip-word", "{display_word}" }
                    if let Some(ipa) = &card.ipa {
                        span { class: "tooltip-ipa", "/{ipa}/" }
                    }
                }
                span { class: "tooltip-head-tags",
                    span { class: word_type_tag_class(&card.word_type_key), "{card.word_type}" }
                    if let Some(selmaho) = &card.selmaho {
                        span { class: "dictionary-meta-segment dictionary-selmaho-tag",
                            em { "{selmaho}" }
                        }
                    }
                }
            }
            if !card.decomposition.is_empty() {
                span { class: "tooltip-row tooltip-decomposition",
                    span { class: "tooltip-label", "decomposition" }
                    span { class: "tooltip-decomposition-pieces",
                        for piece in card.decomposition.iter().filter(|piece| piece.kind != VlackuCompositionPieceKind::Hyphen) {
                            {
                                let display_surface = display_lojban_text(script, &piece.display_surface);
                                if let Some(source) = &piece.source {
                                    let display_source = display_lojban_text(script, piece.display_source.as_deref().unwrap_or(source));
                                    rsx! {
                                        span { class: "tooltip-rafsi-piece",
                                            span { class: "tooltip-rafsi-surface", "{display_surface}" }
                                            span { class: "tooltip-rafsi-source", "{display_source}" }
                                        }
                                    }
                                } else {
                                    rsx! {
                                        span { class: "tooltip-rafsi-piece",
                                            span { class: "tooltip-rafsi-surface", "{display_surface}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !tooltip.definition.is_empty() {
                span { class: "tooltip-copy",
                    { render_reference_tooltip_inline_spans(&tooltip.definition, base_path, script) }
                }
            }
            if !card.glosses.is_empty() {
                span { class: "tooltip-chip-row tooltip-glosses",
                    for gloss in card.glosses.iter() {
                        span { class: "tooltip-chip", "{gloss}" }
                    }
                }
            }
            if !tooltip.notes.is_empty() {
                span { class: "tooltip-notes",
                    { render_reference_tooltip_inline_spans(&tooltip.notes, base_path, script) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_reference_tooltip_inline_spans(
    spans: &[ReferenceTooltipInline],
    base_path: &str,
    script: GentufaScript,
) -> Element {
    rsx! {
        for span in spans.iter() {
            {
                match span.as_data() {
                    data!(ReferenceTooltipInline::Text(text)) => rsx! { "{text}" },
                    data!(ReferenceTooltipInline::Math(math)) => render_vlacku_math(math),
                    data!(ReferenceTooltipInline::WordRef { label, href, .. }) => {
                        let resolved_href = resolved_href_with_base_path(base_path, href);
                        let display_label = display_lojban_text(script, label);
                        rsx! {
                            span { class: "tooltip-inline-link", "data-href": "{resolved_href}", "{display_label}" }
                        }
                    }
                    data!(ReferenceTooltipInline::IndexedPlace { text, highlighted, .. }) => {
                        let class = if *highlighted {
                            "tooltip-indexed-place is-highlighted"
                        } else {
                            "tooltip-indexed-place"
                        };
                        rsx! {
                            span { class: "{class}", "{text}" }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_reference_tooltip_row(row: &ReferenceTooltipRow) -> Element {
    let view = reference_tooltip_row_view_model(row);
    rsx! {
        span { class: "reference-resolution-tooltip",
            span { class: "reference-row-symbol reference-row-base",
                { render_reference_base_label(&row.label) }
            }
            if let Some(slot) = view.slot_text.as_deref() {
                span { class: "reference-row-symbol", "⟨" }
                span { class: "reference-row-slot", "{slot}" }
                span { class: "reference-row-symbol", "⟩" }
            }
            span { class: "reference-row-symbol reference-row-arrow", "→" }
            span { class: "reference-row-target", "{view.target_text}" }
        }
    }
}

#[invariant(self.slot_text.as_ref().map_or(true, |slot| !slot.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferenceTooltipRowViewModel {
    slot_text: Option<String>,
    target_text: String,
}

#[requires(true)]
#[ensures(ret.target_text == row.target_text)]
fn reference_tooltip_row_view_model(row: &ReferenceTooltipRow) -> ReferenceTooltipRowViewModel {
    new!(ReferenceTooltipRowViewModel {
        slot_text: row.label.slot.as_ref().map(reference_slot_display_text),
        target_text: row.target_text.clone(),
    })
}

#[requires(true)]
#[ensures(true)]
fn render_reference_base_label(label: &ReferenceLabel) -> Element {
    let stem = math_alphanumeric_stem(&label.stem);
    rsx! {
        span { class: "spa-cll-math reference-row-base-math",
            math { class: "math-var", display: "inline",
                if let Some(occurrence) = label.occurrence {
                    msub {
                        mi { "{stem}" }
                        mtext { "{occurrence}" }
                    }
                } else {
                    mi { "{stem}" }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_tooltip_inline_spans(
    spans: &[VlackuInline],
    base_path: &str,
    interactive_links: bool,
    script: GentufaScript,
) -> Element {
    rsx! {
        for span in spans.iter() {
            {
                match span.as_data() {
                    data!(VlackuInline::Text(text)) => rsx! { "{text}" },
                    data!(VlackuInline::Math(math)) => render_vlacku_math(math),
                    data!(VlackuInline::WordRef { label, href, .. }) => {
                        let resolved_href = resolved_href_with_base_path(base_path, href);
                        let display_label = display_lojban_text(script, label);
                        if interactive_links {
                            rsx! {
                                { render_text_route_link("tooltip-inline-link", &resolved_href, base_path, &display_label) }
                            }
                        } else {
                            rsx! {
                                span { class: "tooltip-inline-link", "{display_label}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_headword_line(
    card: &VlackuWebCard,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let word_href = vlacku_web_url(
        base_path,
        &VlackuWebState {
            mode: VlackuWebMode::Word,
            query: card.word.clone(),
            count: VLACKU_WEB_DEFAULT_COUNT,
            word_types: Vec::new(),
        },
    );
    let display_word = display_lojban_text(script, &card.display_word);
    rsx! {
        div { class: "dictionary-word-cluster",
            h2 { class: "word dictionary-headword-title",
                { render_vlacku_headword_action(
                    jvozba_pane,
                    jvozba_available,
                    card.can_add_to_jvozba,
                    &card.word,
                    &display_word,
                    &word_href,
                    base_path,
                    page_find,
                ) }
            }
            if let Some(ipa) = &card.ipa {
                span { class: "dictionary-headword-ipa",
                    { render_page_find_text(page_find, &format!("/{ipa}/")) }
                }
            }
            if !card.decomposition.is_empty() {
                span { class: "dictionary-word-composition-group dictionary-word-decomposition-group",
                    { render_vlacku_inline_separator("=") }
                    { render_vlacku_decomposition_inline(card, jvozba_pane, jvozba_available, base_path, script, page_find) }
                }
            } else if !card.rafsi.is_empty() {
                span { class: "dictionary-word-composition-group dictionary-word-rafsi-group",
                    { render_vlacku_inline_separator("≘") }
                    span { class: "dictionary-inline-pill-row",
                        for rafsi in card.rafsi.iter() {
                            { render_rafsi_pill(jvozba_pane, jvozba_available, &card.word, rafsi, script, page_find) }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_headword_action(
    mut jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    can_add_to_jvozba: bool,
    word: &str,
    display_word: &str,
    href: &str,
    base_path: &str,
    page_find: &PageFindContext,
) -> Element {
    let pane_open = jvozba_available && jvozba_pane.read().open;
    let word_value = word.to_owned();
    if pane_open && can_add_to_jvozba {
        rsx! {
            button {
                class: "dictionary-headword-link dictionary-jvozba-highlighted-word",
                r#type: "button",
                title: "Add to jvozba",
                onclick: move |_| add_vlacku_jvozba_word(&mut jvozba_pane, word_value.clone()),
                { render_page_find_text(page_find, display_word) }
            }
        }
    } else if pane_open {
        rsx! {
            span { class: "dictionary-headword-link",
                { render_page_find_text(page_find, display_word) }
            }
        }
    } else {
        render_text_route_link_with_page_find(
            "dictionary-headword-link",
            href,
            base_path,
            display_word,
            page_find,
        )
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_decomposition_inline(
    card: &VlackuWebCard,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let visible_pieces = card
        .decomposition
        .iter()
        .filter(|piece| piece.kind != VlackuCompositionPieceKind::Hyphen)
        .collect::<Vec<_>>();
    rsx! {
        for (index, piece) in visible_pieces.iter().enumerate() {
            if index > 0 {
                { render_vlacku_inline_separator("+") }
            }
            { render_composition_piece(piece, jvozba_pane, jvozba_available, base_path, script, page_find) }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_inline_separator(text: &str) -> Element {
    rsx! { span { class: "dictionary-word-inline-separator", "{text}" } }
}

#[requires(true)]
#[ensures(true)]
fn render_copy_icon() -> Element {
    rsx! {
        svg {
            class: "dictionary-copy-icon",
            "viewBox": "0 0 24 24",
            "aria-hidden": "true",
            path {
                d: "M8 7h9a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2zM5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_linecap: "round",
                stroke_linejoin: "round",
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn copy_text_to_clipboard(text: &str) {
    js_copy_text_to_clipboard(text);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn copy_text_to_clipboard(text: &str) {
    let _ = copy_text_to_clipboard_result(text);
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn copy_text_to_clipboard_result(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    clipboard
        .set_text(text.to_owned())
        .map_err(|error| error.to_string())
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
fn copy_text_to_clipboard_result(_text: &str) -> Result<(), String> {
    Err("Native clipboard is not available for this platform yet.".to_owned())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlacku_author_credit_text(author: &VlackuWebAuthor) -> String {
    match author.realname.as_deref() {
        Some(realname) if !realname.trim().is_empty() => {
            format!("by {} ({realname})", author.username)
        }
        _ => format!("by {}", author.username),
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_author_credit(author: &VlackuWebAuthor, page_find: &PageFindContext) -> Element {
    let credit = vlacku_author_credit_text(author);
    rsx! {
        span { class: "dictionary-author-credit",
            { render_page_find_text(page_find, &credit) }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_metadata_pill(
    card: &VlackuWebCard,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        div { class: "dictionary-meta-pill",
            span { class: word_type_tag_class(&card.word_type_key),
                { render_page_find_text(page_find, &card.word_type) }
            }
            if let Some(selmaho) = &card.selmaho {
                { render_vlacku_selmaho_segment(card, selmaho, pending_cukta_scroll, base_path, page_find) }
            }
            if let Some(similarity) = card.similarity {
                span { class: "dictionary-meta-segment dictionary-meta-tooltip", title: "Phonetic similarity to the current query.",
                    { render_page_find_text(page_find, &format_similarity(similarity)) }
                }
            }
            { render_vote_display(&card.votes, page_find) }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_selmaho_segment(
    card: &VlackuWebCard,
    selmaho: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    page_find: &PageFindContext,
) -> Element {
    if card.word_type_key == "gismu" {
        let href = format!("{}/cukta", base_path.trim_end_matches('/'));
        if let Some(route) = jbotci_route_from_href(base_path, &href) {
            let pending_scroll = cukta_pending_scroll_for_explicit_route_link(base_path, &route);
            let click_route = route.clone();
            rsx! {
                Link {
                    class: "dictionary-meta-segment dictionary-selmaho-tag",
                    to: route,
                    title: "CLL gismu section",
                    onclick_only: true,
                    onclick: move |_| {
                        push_route_with_cukta_scroll_intent(
                            pending_cukta_scroll,
                            pending_scroll.clone(),
                            click_route.clone(),
                        );
                    },
                    em { { render_page_find_text(page_find, selmaho) } }
                }
            }
        } else {
            rsx! {
                a { class: "dictionary-meta-segment dictionary-selmaho-tag", href: "{href}", title: "CLL gismu section",
                    em { { render_page_find_text(page_find, selmaho) } }
                }
            }
        }
    } else {
        rsx! {
            span { class: "dictionary-meta-segment dictionary-selmaho-tag", title: "selma'o classification",
                em { { render_page_find_text(page_find, selmaho) } }
            }
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn word_type_tag_class(word_type_key: &str) -> String {
    format!(
        "dictionary-meta-segment dictionary-word-type-tag {}",
        vlacku_word_type_tag_class(word_type_key)
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlacku_word_type_tag_class(word_type_key: &str) -> &'static str {
    match word_type_key {
        "gismu" | "experimental-gismu" => "is-gismu",
        "lujvo" | "zei-lujvo" | "obsolete-zei-lujvo" => "is-lujvo",
        "cmevla" | "obsolete-cmevla" => "is-cmevla",
        "fu'ivla" | "obsolete-fu'ivla" => "is-fuhivla",
        "cmavo" | "cmavo-compound" | "experimental-cmavo" | "obsolete-cmavo" => "is-cmavo",
        "letteral" | "bu-letteral" => "is-letteral",
        _ => "is-other",
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_word_action(
    mut jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    can_add_to_jvozba: bool,
    word: &str,
    display_word: &str,
    href: &str,
    class_name: &str,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let pane_open = jvozba_available && jvozba_pane.read().open;
    let word_value = word.to_owned();
    let tooltip = dictionary_tooltip_for_word(base_path, word);
    let static_class_name = class_name
        .split_whitespace()
        .filter(|class| {
            *class != "dictionary-jvozba-add-link-hint"
                && *class != "dictionary-jvozba-highlighted-word"
        })
        .collect::<Vec<_>>()
        .join(" ");
    if pane_open && can_add_to_jvozba {
        if let Some(card) = &tooltip {
            rsx! {
                span { class: "dictionary-tooltip-host",
                    button {
                        class: "{class_name}",
                        r#type: "button",
                        title: "Add to jvozba",
                        onclick: move |_| add_vlacku_jvozba_word(&mut jvozba_pane, word_value.clone()),
                        { render_page_find_text(page_find, display_word) }
                    }
                    { render_dictionary_tooltip(card, false, base_path, script) }
                }
            }
        } else {
            rsx! {
                button {
                    class: "{class_name}",
                    r#type: "button",
                    title: "Add to jvozba",
                    onclick: move |_| add_vlacku_jvozba_word(&mut jvozba_pane, word_value.clone()),
                    { render_page_find_text(page_find, display_word) }
                }
            }
        }
    } else if pane_open {
        rsx! {
            span { class: "{static_class_name}",
                { render_page_find_text(page_find, display_word) }
            }
        }
    } else {
        if let Some(card) = &tooltip {
            rsx! {
                span { class: "dictionary-tooltip-host",
                    { render_text_route_link_with_page_find(&static_class_name, href, base_path, display_word, page_find) }
                    { render_dictionary_tooltip(card, false, base_path, script) }
                }
            }
        } else {
            render_text_route_link_with_page_find(
                &static_class_name,
                href,
                base_path,
                display_word,
                page_find,
            )
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vote_display(votes: &VlackuVoteDisplay, page_find: &PageFindContext) -> Element {
    match votes {
        VlackuVoteDisplay::Known(value) => rsx! {
            span { class: vote_class(value), title: vote_title(value),
                { render_page_find_text(page_find, &value.to_string()) }
            }
        },
        VlackuVoteDisplay::Unknown => rsx! {
            span { class: "dictionary-meta-segment dictionary-meta-tooltip dictionary-vote-tag is-unknown", title: "This parses as a valid Lojban word, but it is not present in the embedded dictionary, so no Lensisku vote tally is available.",
                { render_page_find_text(page_find, "?") }
            }
        },
        VlackuVoteDisplay::Hidden => rsx! {},
    }
}

#[requires(true)]
#[ensures(true)]
fn render_composition_piece(
    piece: &VlackuCompositionPiece,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    match piece.kind {
        VlackuCompositionPieceKind::Hyphen => {
            let display_surface = display_lojban_text(script, &piece.display_surface);
            rsx! {
                span { class: "dictionary-word-inline-separator",
                    { render_page_find_text(page_find, &display_surface) }
                }
            }
        }
        VlackuCompositionPieceKind::Rafsi => {
            let display_surface = display_lojban_text(script, &piece.display_surface);
            if let Some(source) = &piece.source {
                let display_source =
                    display_lojban_text(script, piece.display_source.as_deref().unwrap_or(source));
                let href = vlacku_web_url(
                    base_path,
                    &VlackuWebState {
                        mode: VlackuWebMode::Word,
                        query: source.clone(),
                        count: VLACKU_WEB_DEFAULT_COUNT,
                        word_types: Vec::new(),
                    },
                );
                if piece.source_is_surface {
                    rsx! {
                        { render_vlacku_word_action(
                            jvozba_pane,
                            jvozba_available,
                            true,
                            source,
                            &display_surface,
                            &href,
                            "chip rafsi-chip dictionary-word-link rafsi-source-link dictionary-jvozba-add-link-hint",
                            base_path,
                            script,
                            page_find,
                        ) }
                    }
                } else {
                    rsx! {
                        span { class: "rafsi-split-pill",
                            { render_vlacku_rafsi_add_piece(jvozba_pane, jvozba_available, &piece.surface, source, &display_surface, page_find) }
                            span { class: "rafsi-split-right",
                                { render_vlacku_word_action(
                                    jvozba_pane,
                                    jvozba_available,
                                    true,
                                    source,
                                    &display_source,
                                    &href,
                                    "dictionary-word-link rafsi-source-link dictionary-jvozba-add-link-hint",
                                    base_path,
                                    script,
                                    page_find,
                                ) }
                            }
                        }
                    }
                }
            } else {
                rsx! {
                    span { class: "chip rafsi-chip",
                        { render_page_find_text(page_find, &display_surface) }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_rafsi_add_piece(
    mut jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    rafsi: &str,
    source_word: &str,
    display_rafsi: &str,
    page_find: &PageFindContext,
) -> Element {
    let pane_open = jvozba_available && jvozba_pane.read().open;
    let rafsi_value = rafsi.to_owned();
    let source_value = source_word.to_owned();
    if pane_open {
        rsx! {
            button {
                class: "rafsi-split-left dictionary-jvozba-add-pill dictionary-jvozba-add-pill-hint",
                r#type: "button",
                aria_label: "Add rafsi {rafsi} from {source_word}",
                onclick: move |_| add_vlacku_jvozba_rafsi(
                    &mut jvozba_pane,
                    rafsi_value.clone(),
                    Some(source_value.clone()),
                ),
                { render_page_find_text(page_find, display_rafsi) }
            }
        }
    } else {
        rsx! {
            span { class: "rafsi-split-left",
                { render_page_find_text(page_find, display_rafsi) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_rafsi_pill(
    mut jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    source_word: &str,
    rafsi: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let pane_open = jvozba_available && jvozba_pane.read().open;
    let rafsi_value = rafsi.to_owned();
    let source_value = source_word.to_owned();
    let display_rafsi = display_lojban_text(script, rafsi);
    if pane_open {
        rsx! {
            button {
                class: "chip rafsi-chip dictionary-jvozba-add-pill dictionary-jvozba-add-pill-hint",
                r#type: "button",
                aria_label: "Add rafsi {rafsi} from {source_word}",
                onclick: move |_| add_vlacku_jvozba_rafsi(
                    &mut jvozba_pane,
                    rafsi_value.clone(),
                    Some(source_value.clone()),
                ),
                { render_page_find_text(page_find, &display_rafsi) }
            }
        }
    } else {
        rsx! {
            span { class: "chip rafsi-chip",
                { render_page_find_text(page_find, &display_rafsi) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_inline_spans(
    spans: &[VlackuInline],
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        for span in spans.iter() {
            {
                match span.as_data() {
                    data!(VlackuInline::Text(text)) => rsx! {
                        { render_page_find_text(page_find, text) }
                    },
                    data!(VlackuInline::Math(math)) => render_vlacku_math(math),
                    data!(VlackuInline::WordRef { label, href, can_add_to_jvozba }) => {
                        render_vlacku_inline_word_ref(
                            jvozba_pane,
                            jvozba_available,
                            *can_add_to_jvozba,
                            label,
                            href,
                            base_path,
                            script,
                            page_find,
                        )
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_inline_word_ref(
    mut jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    can_add_to_jvozba: bool,
    label: &str,
    href: &str,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let pane_open = jvozba_available && jvozba_pane.read().open;
    let word_value = label.to_owned();
    let resolved_href = resolved_href_with_base_path(base_path, href);
    let tooltip = dictionary_tooltip_for_word(base_path, label);
    let display_label = display_lojban_text(script, label);
    if pane_open && can_add_to_jvozba {
        if let Some(card) = &tooltip {
            rsx! {
                span { class: "dictionary-tooltip-host",
                    button {
                        class: "dictionary-word-link dictionary-jvozba-add-link-hint",
                        r#type: "button",
                        title: "Add to jvozba",
                        onclick: move |_| add_vlacku_jvozba_word(&mut jvozba_pane, word_value.clone()),
                        { render_page_find_text(page_find, &display_label) }
                    }
                    { render_dictionary_tooltip(card, false, base_path, script) }
                }
            }
        } else {
            rsx! {
                button {
                    class: "dictionary-word-link dictionary-jvozba-add-link-hint",
                    r#type: "button",
                    title: "Add to jvozba",
                    onclick: move |_| add_vlacku_jvozba_word(&mut jvozba_pane, word_value.clone()),
                    { render_page_find_text(page_find, &display_label) }
                }
            }
        }
    } else if pane_open {
        rsx! {
            span { class: "dictionary-word-link",
                { render_page_find_text(page_find, &display_label) }
            }
        }
    } else {
        if let Some(card) = &tooltip {
            rsx! {
                span { class: "dictionary-tooltip-host",
                    { render_text_route_link_with_page_find("dictionary-word-link", &resolved_href, base_path, &display_label, page_find) }
                    { render_dictionary_tooltip(card, false, base_path, script) }
                }
            }
        } else {
            render_text_route_link_with_page_find(
                "dictionary-word-link",
                &resolved_href,
                base_path,
                &display_label,
                page_find,
            )
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty() || href.is_empty())]
fn resolved_href_with_base_path(base_path: &str, href: &str) -> String {
    if href.starts_with('/') {
        format!("{}{}", base_path.trim_end_matches('/'), href)
    } else {
        href.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_math(math: &VlackuMath) -> Element {
    rsx! {
        span { class: "spa-cll-math", dangerous_inner_html: "{math.markup}" }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_jvozba_pane(
    mut jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_drag: Signal<Option<VlackuJvozbaDragState>>,
    script: GentufaScript,
) -> Element {
    let pane = jvozba_pane.read().clone();
    let output = build_vlacku_jvozba_output(pane.mode, &pane.items);
    rsx! {
        aside {
            class: "dictionary-jvozba-pane",
            "data-jvozba-open": if pane.open { "1" } else { "0" },
            "data-jvozba-pane": "1",
            button {
                class: "dictionary-jvozba-tab",
                r#type: "button",
                aria_expanded: if pane.open { "true" } else { "false" },
                aria_controls: "dictionary-jvozba-body",
                "data-jvozba-toggle": "1",
                onclick: move |_| {
                    let mut next = jvozba_pane.read().clone();
                    next.open = !next.open;
                    set_vlacku_jvozba_pane(&mut jvozba_pane, next);
                },
                "jvozba"
            }
            section {
                class: "dictionary-jvozba-body",
                id: "dictionary-jvozba-body",
                "data-jvozba-body": "1",
                div { class: "dictionary-jvozba-output",
                    div { class: "dictionary-jvozba-output-row",
                        div { class: "dictionary-jvozba-output-controls",
                            div { class: "dictionary-jvozba-mode-toggle-group", role: "group", aria_label: "jvozba output mode",
                                button {
                                    class: vlacku_jvozba_mode_class(pane.mode == VlackuJvozbaMode::Lujvo),
                                    r#type: "button",
                                    aria_pressed: pressed_attr(pane.mode == VlackuJvozbaMode::Lujvo),
                                    onclick: move |_| set_vlacku_jvozba_mode(&mut jvozba_pane, VlackuJvozbaMode::Lujvo),
                                    "lujvo"
                                }
                                button {
                                    class: vlacku_jvozba_mode_class(pane.mode == VlackuJvozbaMode::Cmevla),
                                    r#type: "button",
                                    aria_pressed: pressed_attr(pane.mode == VlackuJvozbaMode::Cmevla),
                                    onclick: move |_| set_vlacku_jvozba_mode(&mut jvozba_pane, VlackuJvozbaMode::Cmevla),
                                    "cmevla"
                                }
                            }
                            button {
                                class: "dictionary-jvozba-clear",
                                r#type: "button",
                                disabled: pane.items.is_empty(),
                                "data-jvozba-clear": "1",
                                onclick: move |_| clear_vlacku_jvozba_items(&mut jvozba_pane),
                                "Clear"
                            }
                        }
                        { render_jvozba_output(&output, script) }
                    }
                }
                if pane.items.is_empty() {
                    div { class: "dictionary-jvozba-empty", "data-jvozba-empty": "1",
                        p {
                            "Click on "
                            span { class: "dictionary-jvozba-highlighted-word", "highlighted items" }
                            " to add them here."
                        }
                        p { "Added words are represented by their best scoring rafsi." }
                        p { em { "Added rafsi are used as-is regardless of their score." } }
                    }
                } else {
                    ol { class: "dictionary-jvozba-list", "data-jvozba-list": "1",
                        for (index, item) in pane.items.iter().enumerate() {
                            { render_jvozba_item(jvozba_pane, jvozba_drag, index, item, script) }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_jvozba_item(
    mut jvozba_pane: Signal<VlackuJvozbaPaneState>,
    mut jvozba_drag: Signal<Option<VlackuJvozbaDragState>>,
    index: usize,
    item: &VlackuJvozbaItem,
    script: GentufaScript,
) -> Element {
    let drag = *jvozba_drag.read();
    let is_dragging = drag.is_some_and(|state| state.preview_visible && state.start_index == index);
    let is_drop_before = drag.is_some_and(|state| {
        state.preview_visible
            && state.target_index < state.start_index
            && state.target_index == index
    });
    let is_drop_after = drag.is_some_and(|state| {
        state.preview_visible
            && state.target_index > state.start_index
            && state.target_index == index
    });
    let item_class = class_names(
        "dictionary-jvozba-pane-item",
        &[
            ("is-dragging", is_dragging),
            ("is-drop-before", is_drop_before),
            ("is-drop-after", is_drop_after),
        ],
    );
    let item_height = drag.map(|state| state.item_height).unwrap_or(32);
    let item_style = if is_drop_before {
        format!("--jvozba-drop-gap-before:{item_height}px;")
    } else if is_drop_after {
        format!("--jvozba-drop-gap-after:{item_height}px;")
    } else {
        String::new()
    };
    rsx! {
        li {
            class: "{item_class}",
            style: "{item_style}",
            draggable: "true",
            "data-jvozba-item-index": "{index}",
            ondragstart: move |_| start_vlacku_jvozba_drag(&mut jvozba_drag, index),
            ondragenter: move |event| {
                event.prevent_default();
                set_vlacku_jvozba_drag_target(&mut jvozba_drag, index);
            },
            ondragover: move |event| {
                event.prevent_default();
                set_vlacku_jvozba_drag_target(&mut jvozba_drag, index);
            },
            ondrop: move |event| {
                event.prevent_default();
                commit_vlacku_jvozba_drag(&mut jvozba_pane, &mut jvozba_drag);
            },
            ondragend: move |_| finish_vlacku_jvozba_drag(&mut jvozba_pane, &mut jvozba_drag),
            div { class: "dictionary-jvozba-item-reorder",
                div {
                    class: "dictionary-jvozba-drag-handle",
                    role: "button",
                    aria_label: "Drag to reorder",
                    "data-jvozba-drag-handle": "1",
                    "::"
                }
                button {
                    class: "sr-only",
                    r#type: "button",
                    aria_label: "Move item later",
                    onclick: move |_| move_vlacku_jvozba_item(&mut jvozba_pane, index, 1),
                    "Move later"
                }
                button {
                    class: "sr-only",
                    r#type: "button",
                    aria_label: "Move item earlier",
                    onclick: move |_| move_vlacku_jvozba_item(&mut jvozba_pane, index, -1),
                    "Move earlier"
                }
            }
            div {
                class: "dictionary-jvozba-pane-item-content",
                style: "--rafsi-indent-level:{item.indent_level};",
                if item.indent_level > 0 {
                    span { class: "dictionary-jvozba-indent-markers", aria_hidden: "true",
                        for _ in 0..item.indent_level.min(4) {
                            span { class: "dictionary-jvozba-indent-marker-step", "⇥" }
                        }
                    }
                }
                { render_jvozba_item_chip(item, script) }
            }
            button {
                class: "dictionary-jvozba-item-remove",
                r#type: "button",
                aria_label: "Remove",
                onclick: move |_| remove_vlacku_jvozba_item(&mut jvozba_pane, index),
                "×"
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_jvozba_item_chip(item: &VlackuJvozbaItem, script: GentufaScript) -> Element {
    match item.kind {
        VlackuJvozbaItemKind::FixedRafsi => {
            let source_label = item.source.as_deref().unwrap_or("rafsi");
            let display_value = display_lojban_text(script, &item.value);
            let display_source_label = display_lojban_text(script, source_label);
            rsx! {
                span { class: "rafsi-split-pill dictionary-jvozba-pane-rafsi-pill",
                    span { class: "rafsi-split-left", "{display_value}" }
                    span { class: "rafsi-split-right", "{display_source_label}" }
                }
            }
        }
        VlackuJvozbaItemKind::Word => {
            let display_value = display_lojban_text(script, &item.value);
            rsx! {
                span { class: "chip dictionary-jvozba-pane-word-chip", "{display_value}" }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_jvozba_output(output: &VlackuJvozbaOutput, script: GentufaScript) -> Element {
    match output {
        VlackuJvozbaOutput::Empty => rsx! {},
        VlackuJvozbaOutput::NeedsMore => rsx! {
            p { class: "dictionary-jvozba-output-line is-pending", "Add at least two words or rafsi." }
        },
        VlackuJvozbaOutput::Error { message } => rsx! {
            p { class: "dictionary-jvozba-output-line is-error", "{message}" }
        },
        VlackuJvozbaOutput::Success { word: _, segments } => rsx! {
            p { class: "dictionary-jvozba-output-line",
                for segment in segments.iter() {
                    span { class: jvozba_segment_class(segment.tone), "{display_lojban_text(script, &segment.text)}" }
                }
            }
        },
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlacku_mode_class(active: bool) -> &'static str {
    if active {
        "dictionary-mode-toggle active"
    } else {
        "dictionary-mode-toggle"
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlacku_mode_title(mode: VlackuWebMode, disabled: bool) -> &'static str {
    if disabled {
        "Semantic search is unavailable in this browser"
    } else {
        match mode {
            VlackuWebMode::Word => "Find the word with exact spelling",
            VlackuWebMode::Rafsi => "Find the word by rafsi",
            VlackuWebMode::Sound => {
                "Find words with similar pronunciation; use [IPA] for IPA input"
            }
            VlackuWebMode::Meaning => "Find words with similar meaning",
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlacku_query_placeholder(mode: VlackuWebMode) -> &'static str {
    match mode {
        VlackuWebMode::Word | VlackuWebMode::Rafsi => {
            "/regex/ or glob (@ = any vowel, $ = any consonant, ? = any character)"
        }
        VlackuWebMode::Sound => "Lojban or [aj piː ej]",
        VlackuWebMode::Meaning => "semantic search",
    }
}

#[requires(true)]
#[ensures(true)]
fn toggle_vlacku_word_type(
    vlacku_draft_state: &mut Signal<VlackuWebState>,
    vlacku_committed_state: &mut Signal<VlackuWebState>,
    value: &str,
) {
    let mut next = vlacku_draft_state.read().clone();
    next.word_types = toggle_vlacku_word_type_selection(&next.word_types, value);
    next.count = VLACKU_WEB_DEFAULT_COUNT;
    set_vlacku_state_immediate(vlacku_draft_state, vlacku_committed_state, next);
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_similarity(value: f32) -> String {
    format!("{:.0}%", value * 100.0)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vote_class(value: &str) -> &'static str {
    if value == "∞" {
        "dictionary-meta-segment dictionary-meta-tooltip dictionary-vote-tag is-official"
    } else if parsed_vote_value(value).is_some_and(|count| count >= 5) {
        "dictionary-meta-segment dictionary-meta-tooltip dictionary-vote-tag is-high"
    } else if parsed_vote_value(value).is_some_and(|count| count >= 2) {
        "dictionary-meta-segment dictionary-meta-tooltip dictionary-vote-tag is-medium"
    } else {
        "dictionary-meta-segment dictionary-meta-tooltip dictionary-vote-tag is-low"
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vote_title(value: &str) -> &'static str {
    if value == "∞" {
        "Official baseline lexicon word. The infinity marker replaces the raw Lensisku community tally for officialdata entries."
    } else {
        "Community upvote/downvote tally from Lensisku contributors."
    }
}

#[requires(true)]
#[ensures(true)]
fn parsed_vote_value(value: &str) -> Option<i32> {
    value.trim_start_matches('+').parse().ok()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlacku_jvozba_mode_class(active: bool) -> &'static str {
    if active {
        "dictionary-jvozba-mode-toggle active"
    } else {
        "dictionary-jvozba-mode-toggle"
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn jvozba_segment_class(tone: VlackuJvozbaSegmentTone) -> &'static str {
    match tone {
        VlackuJvozbaSegmentTone::RafsiA => "dictionary-jvozba-output-segment is-rafsi-a",
        VlackuJvozbaSegmentTone::RafsiB => "dictionary-jvozba-output-segment is-rafsi-b",
        VlackuJvozbaSegmentTone::Hyphen => "dictionary-jvozba-output-segment is-hyphen",
    }
}

#[requires(true)]
#[ensures(true)]
fn add_vlacku_jvozba_word(jvozba_pane: &mut Signal<VlackuJvozbaPaneState>, value: String) {
    if value.trim().is_empty() {
        return;
    }
    let mut next = jvozba_pane.read().clone();
    next.open = true;
    next.items.push(VlackuJvozbaItem {
        kind: VlackuJvozbaItemKind::Word,
        value: value.trim().to_owned(),
        source: None,
        indent_level: 0,
    });
    set_vlacku_jvozba_pane(jvozba_pane, next);
}

#[requires(true)]
#[ensures(true)]
fn add_vlacku_jvozba_rafsi(
    jvozba_pane: &mut Signal<VlackuJvozbaPaneState>,
    value: String,
    source: Option<String>,
) {
    if value.trim().is_empty() {
        return;
    }
    let mut next = jvozba_pane.read().clone();
    next.open = true;
    next.items.push(VlackuJvozbaItem {
        kind: VlackuJvozbaItemKind::FixedRafsi,
        value: value.trim().to_owned(),
        source: source.map(|value| value.trim().to_owned()),
        indent_level: 0,
    });
    set_vlacku_jvozba_pane(jvozba_pane, next);
}

#[requires(true)]
#[ensures(true)]
fn set_vlacku_jvozba_mode(jvozba_pane: &mut Signal<VlackuJvozbaPaneState>, mode: VlackuJvozbaMode) {
    let mut next = jvozba_pane.read().clone();
    next.mode = mode;
    set_vlacku_jvozba_pane(jvozba_pane, next);
}

#[requires(true)]
#[ensures(true)]
fn move_vlacku_jvozba_item(
    jvozba_pane: &mut Signal<VlackuJvozbaPaneState>,
    index: usize,
    delta: isize,
) {
    let mut next = jvozba_pane.read().clone();
    let Some(target) = index.checked_add_signed(delta) else {
        return;
    };
    if index < next.items.len() && target < next.items.len() {
        next.items.swap(index, target);
        set_vlacku_jvozba_pane(jvozba_pane, next);
    }
}

#[requires(true)]
#[ensures(true)]
fn remove_vlacku_jvozba_item(jvozba_pane: &mut Signal<VlackuJvozbaPaneState>, index: usize) {
    let mut next = jvozba_pane.read().clone();
    if index < next.items.len() {
        next.items.remove(index);
        set_vlacku_jvozba_pane(jvozba_pane, next);
    }
}

#[requires(true)]
#[ensures(true)]
fn clear_vlacku_jvozba_items(jvozba_pane: &mut Signal<VlackuJvozbaPaneState>) {
    let mut next = jvozba_pane.read().clone();
    next.items.clear();
    set_vlacku_jvozba_pane(jvozba_pane, next);
}

#[requires(true)]
#[ensures(true)]
fn start_vlacku_jvozba_drag(jvozba_drag: &mut Signal<Option<VlackuJvozbaDragState>>, index: usize) {
    let state = VlackuJvozbaDragState {
        start_index: index,
        target_index: index,
        item_height: measure_vlacku_jvozba_item_height(index)
            .filter(|height| *height > 0)
            .unwrap_or(32),
        preview_visible: true,
    };
    jvozba_drag.set(Some(state));
}

#[requires(true)]
#[ensures(true)]
fn set_vlacku_jvozba_drag_target(
    jvozba_drag: &mut Signal<Option<VlackuJvozbaDragState>>,
    index: usize,
) {
    let current = *jvozba_drag.read();
    if let Some(mut state) = current {
        state.target_index = index;
        jvozba_drag.set(Some(state));
    }
}

#[requires(true)]
#[ensures(true)]
fn commit_vlacku_jvozba_drag(
    jvozba_pane: &mut Signal<VlackuJvozbaPaneState>,
    jvozba_drag: &mut Signal<Option<VlackuJvozbaDragState>>,
) {
    let Some(state) = *jvozba_drag.read() else {
        return;
    };
    let mut next = jvozba_pane.read().clone();
    if state.start_index < next.items.len() && state.target_index < next.items.len() {
        let item = next.items.remove(state.start_index);
        next.items.insert(state.target_index, item);
        set_vlacku_jvozba_pane(jvozba_pane, next);
    }
    jvozba_drag.set(None);
}

#[requires(true)]
#[ensures(true)]
fn finish_vlacku_jvozba_drag(
    jvozba_pane: &mut Signal<VlackuJvozbaPaneState>,
    jvozba_drag: &mut Signal<Option<VlackuJvozbaDragState>>,
) {
    let Some(state) = *jvozba_drag.read() else {
        return;
    };
    if state.start_index != state.target_index {
        commit_vlacku_jvozba_drag(jvozba_pane, jvozba_drag);
    } else {
        jvozba_drag.set(None);
    }
}

#[requires(!base.is_empty())]
#[ensures(!ret.is_empty())]
fn class_names(base: &str, conditional: &[(&str, bool)]) -> String {
    let mut classes = vec![base.to_owned()];
    classes.extend(
        conditional
            .iter()
            .filter_map(|(class, enabled)| enabled.then_some((*class).to_owned())),
    );
    classes.join(" ")
}

#[requires(true)]
#[ensures(true)]
fn set_vlacku_jvozba_pane(
    jvozba_pane: &mut Signal<VlackuJvozbaPaneState>,
    state: VlackuJvozbaPaneState,
) {
    save_vlacku_jvozba_pane_state(&state);
    jvozba_pane.set(state);
}

#[requires(true)]
#[ensures(true)]
fn render_gentufa_input(
    mut input_text: Signal<String>,
    result: &GentufaWebResult,
    request: Option<&GentufaWebRequest>,
    active_diagnostic: Option<usize>,
    mut active_diagnostic_signal: Signal<Option<usize>>,
    mut diagnostic_tooltip: Signal<Option<DiagnosticInputTooltip>>,
    diagnostic_tooltip_value: Option<DiagnosticInputTooltip>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
) -> Element {
    let text = input_text.read().clone();
    let content_sizer_text = gentufa_textarea_content_sizer_text(&text);
    let diagnostics = current_gentufa_input_diagnostics(&text, result, request);
    rsx! {
        div { class: "gentufa-input-editor",
            div { class: "gentufa-text-sizer", aria_hidden: "true", "{content_sizer_text}" }
            div { class: "gentufa-text-sizer", aria_hidden: "true", "{DEFAULT_GENTUFA_TEXT}" }
            { render_gentufa_diagnostic_overlay(
                &text,
                diagnostics,
                active_diagnostic,
                diagnostic_tooltip,
            ) }
            textarea {
                id: "gentufa-text",
                aria_label: "Lojban text",
                placeholder: "{DEFAULT_GENTUFA_TEXT}",
                value: "{text}",
                spellcheck: "false",
                oninput: move |event| {
                    input_text.set(event.value());
                    active_diagnostic_signal.set(None);
                    diagnostic_tooltip.set(None);
                    schedule_gentufa_textarea_resize();
                },
            }
            { render_gentufa_diagnostic_input_tooltip(
                diagnostic_tooltip_value,
                diagnostics,
                &text,
                active_diagnostic_signal,
                pending_cukta_scroll,
                base_path,
                script,
            ) }
        }
    }
}

#[requires(true)]
#[ensures(!source.ends_with('\n') || ret.ends_with(' '))]
#[ensures(source.ends_with('\n') || ret == source)]
fn gentufa_textarea_content_sizer_text(source: &str) -> String {
    if source.ends_with('\n') {
        format!("{source} ")
    } else {
        source.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn render_gentufa_diagnostic_overlay(
    text: &str,
    diagnostics: &[Diagnostic],
    active_diagnostic: Option<usize>,
    diagnostic_tooltip: Signal<Option<DiagnosticInputTooltip>>,
) -> Element {
    if diagnostics.is_empty() {
        return rsx! {};
    }
    let fragments = diagnostic_overlay_fragments(text, diagnostics, active_diagnostic);
    rsx! {
        div { class: "gentufa-text-overlay", aria_hidden: "true",
            for fragment in fragments.iter() {
                { render_gentufa_diagnostic_overlay_fragment(
                    fragment,
                    diagnostic_tooltip,
                ) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_gentufa_diagnostic_overlay_fragment(
    fragment: &DiagnosticOverlayFragment,
    diagnostic_tooltip: Signal<Option<DiagnosticInputTooltip>>,
) -> Element {
    let diagnostic_index = fragment.diagnostic_index;
    let mut over_tooltip = diagnostic_tooltip;
    let mut enter_tooltip = diagnostic_tooltip;
    let mut move_tooltip = diagnostic_tooltip;
    let mut out_tooltip = diagnostic_tooltip;
    let mut leave_tooltip = diagnostic_tooltip;
    rsx! {
        span {
            class: "{fragment.class_name}",
            "data-selection-start": "{fragment.selection_start}",
            onmouseover: move |event| {
                if let Some(diagnostic_index) = diagnostic_index {
                    let coordinates = event.data().client_coordinates();
                    over_tooltip.set(Some(new!(DiagnosticInputTooltip {
                        diagnostic_index,
                        x: coordinates.x,
                        y: coordinates.y,
                    })));
                }
            },
            onmouseenter: move |event| {
                if let Some(diagnostic_index) = diagnostic_index {
                    let coordinates = event.data().client_coordinates();
                    enter_tooltip.set(Some(new!(DiagnosticInputTooltip {
                        diagnostic_index,
                        x: coordinates.x,
                        y: coordinates.y,
                    })));
                }
            },
            onmousemove: move |event| {
                if let Some(diagnostic_index) = diagnostic_index {
                    let coordinates = event.data().client_coordinates();
                    move_tooltip.set(Some(new!(DiagnosticInputTooltip {
                        diagnostic_index,
                        x: coordinates.x,
                        y: coordinates.y,
                    })));
                }
            },
            onmouseout: move |_| {
                if diagnostic_index.is_some() {
                    out_tooltip.set(None);
                }
            },
            onmouseleave: move |_| {
                if diagnostic_index.is_some() {
                    leave_tooltip.set(None);
                }
            },
            onmousedown: move |event| {
                event.prevent_default();
                let coordinates = event.data().client_coordinates();
                place_gentufa_textarea_caret_from_overlay_click(
                    coordinates.x,
                    coordinates.y,
                );
            },
            "{fragment.text}"
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_gentufa_diagnostic_input_tooltip(
    tooltip: Option<DiagnosticInputTooltip>,
    diagnostics: &[Diagnostic],
    source: &str,
    active_diagnostic: Signal<Option<usize>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
) -> Element {
    let Some(tooltip) = tooltip else {
        return rsx! {};
    };
    let Some(diagnostic) = diagnostics.get(tooltip.diagnostic_index) else {
        return rsx! {};
    };
    let style = format!(
        "--diagnostic-tooltip-x: {:.2}px; --diagnostic-tooltip-y: {:.2}px;",
        tooltip.x, tooltip.y
    );
    rsx! {
        div { class: "gentufa-diagnostic-input-tooltip", style: "{style}",
            { render_diagnostic_card(
                tooltip.diagnostic_index,
                diagnostic,
                source,
                active_diagnostic,
                pending_cukta_scroll,
                base_path,
                script,
                None,
            ) }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn place_gentufa_textarea_caret_from_overlay_click(x: f64, y: f64) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(selection_start) = diagnostic_overlay_selection_offset_from_point(&document, x, y)
    else {
        return;
    };
    let Some(textarea) = document
        .get_element_by_id("gentufa-text")
        .and_then(|element| element.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
    else {
        return;
    };
    let _ = textarea.focus();
    let _ = textarea.set_selection_start(Some(selection_start));
    let _ = textarea.set_selection_end(Some(selection_start));
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn place_gentufa_textarea_caret_from_overlay_click(_x: f64, _y: f64) {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn diagnostic_overlay_selection_offset_from_point(
    document: &web_sys::Document,
    x: f64,
    y: f64,
) -> Option<u32> {
    diagnostic_overlay_caret_position_offset_from_point(document, x, y)
        .or_else(|| diagnostic_overlay_caret_range_offset_from_point(document, x, y))
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn diagnostic_overlay_caret_position_offset_from_point(
    document: &web_sys::Document,
    x: f64,
    y: f64,
) -> Option<u32> {
    let document_value = document.as_ref();
    let function = js_sys::Reflect::get(
        document_value,
        &wasm_bindgen::JsValue::from_str("caretPositionFromPoint"),
    )
    .ok()?
    .dyn_into::<js_sys::Function>()
    .ok()?;
    let position = function
        .call2(
            document_value,
            &wasm_bindgen::JsValue::from_f64(x),
            &wasm_bindgen::JsValue::from_f64(y),
        )
        .ok()?;
    if position.is_null() || position.is_undefined() {
        return None;
    }
    let node = js_sys::Reflect::get(&position, &wasm_bindgen::JsValue::from_str("offsetNode"))
        .ok()?
        .dyn_into::<web_sys::Node>()
        .ok()?;
    let offset = js_sys::Reflect::get(&position, &wasm_bindgen::JsValue::from_str("offset"))
        .ok()?
        .as_f64()? as u32;
    diagnostic_overlay_selection_offset_from_node_offset(node, offset)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn diagnostic_overlay_caret_range_offset_from_point(
    document: &web_sys::Document,
    x: f64,
    y: f64,
) -> Option<u32> {
    let document_value = document.as_ref();
    let function = js_sys::Reflect::get(
        document_value,
        &wasm_bindgen::JsValue::from_str("caretRangeFromPoint"),
    )
    .ok()?
    .dyn_into::<js_sys::Function>()
    .ok()?;
    let range = function
        .call2(
            document_value,
            &wasm_bindgen::JsValue::from_f64(x),
            &wasm_bindgen::JsValue::from_f64(y),
        )
        .ok()?;
    if range.is_null() || range.is_undefined() {
        return None;
    }
    let node = js_sys::Reflect::get(&range, &wasm_bindgen::JsValue::from_str("startContainer"))
        .ok()?
        .dyn_into::<web_sys::Node>()
        .ok()?;
    let offset = js_sys::Reflect::get(&range, &wasm_bindgen::JsValue::from_str("startOffset"))
        .ok()?
        .as_f64()? as u32;
    diagnostic_overlay_selection_offset_from_node_offset(node, offset)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn diagnostic_overlay_selection_offset_from_node_offset(
    node: web_sys::Node,
    offset: u32,
) -> Option<u32> {
    let mut element = node
        .dyn_ref::<web_sys::Element>()
        .cloned()
        .or_else(|| node.parent_element());
    while let Some(current) = element {
        if let Some(start) = current
            .get_attribute("data-selection-start")
            .and_then(|value| value.parse::<u32>().ok())
        {
            return Some(start.saturating_add(offset));
        }
        element = current.parent_element();
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn render_result(
    result: &GentufaWebResult,
    request: Option<&GentufaWebRequest>,
    diagnostics_open: Signal<bool>,
    diagnostics_open_value: bool,
    active_diagnostic: Signal<Option<usize>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    view_mode: Signal<GentufaWebViewMode>,
    view_mode_value: GentufaWebViewMode,
    display: Signal<GentufaDisplayState>,
    display_value: GentufaDisplayState,
    settings_value: UserSettings,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    activity: Signal<AsyncActivityState>,
    export_task: Signal<Option<LatestAsyncTask>>,
    page_find: &PageFindContext,
) -> Element {
    match result {
        GentufaWebResult::Blank => rsx! {},
        GentufaWebResult::Error(error) => render_error(
            error,
            request,
            diagnostics_open,
            diagnostics_open_value,
            active_diagnostic,
            pending_cukta_scroll,
            base_path,
            settings_value.script,
            page_find,
        ),
        GentufaWebResult::Success(success) => render_success(
            success,
            request,
            diagnostics_open,
            diagnostics_open_value,
            active_diagnostic,
            pending_cukta_scroll,
            base_path,
            view_mode,
            view_mode_value,
            display,
            display_value,
            settings_value,
            reference_hover,
            reference_tooltip_open,
            activity,
            export_task,
            page_find,
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn render_error(
    error: &GentufaError,
    request: Option<&GentufaWebRequest>,
    diagnostics_open: Signal<bool>,
    diagnostics_open_value: bool,
    active_diagnostic: Signal<Option<usize>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let source = gentufa_request_source(request);
    rsx! {
        section { class: "result-section error-section",
            { render_diagnostics_pane(
                &error.diagnostics,
                source,
                Some(error.message.as_str()),
                diagnostics_open,
                diagnostics_open_value,
                active_diagnostic,
                pending_cukta_scroll,
                base_path,
                script,
                Some(page_find),
            ) }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_success(
    success: &GentufaSuccess,
    request: Option<&GentufaWebRequest>,
    diagnostics_open: Signal<bool>,
    diagnostics_open_value: bool,
    active_diagnostic: Signal<Option<usize>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    view_mode: Signal<GentufaWebViewMode>,
    view_mode_value: GentufaWebViewMode,
    display: Signal<GentufaDisplayState>,
    display_value: GentufaDisplayState,
    settings_value: UserSettings,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    activity: Signal<AsyncActivityState>,
    export_task: Signal<Option<LatestAsyncTask>>,
    page_find: &PageFindContext,
) -> Element {
    let reference_hover_value = reference_hover.read().clone();
    let source = gentufa_request_source(request);
    rsx! {
        section { class: "result-section",
            { render_reference_overlay(&reference_hover_value) }
            { render_surface_output(success, settings_value.script, page_find) }
            { render_diagnostics_pane(
                &success.diagnostics,
                source,
                None,
                diagnostics_open,
                diagnostics_open_value,
                active_diagnostic,
                pending_cukta_scroll,
                base_path,
                settings_value.script,
                Some(page_find),
            ) }
            div { class: "view-toolbar",
                { render_view_tabs(view_mode, view_mode_value) }
                { render_output_controls(display, display_value) }
            }
            match view_mode_value {
                GentufaWebViewMode::Blocks => rsx! {
                    { render_blocks(success, display_value.show_glosses, settings_value.script, reference_hover, reference_tooltip_open, activity, export_task, page_find) }
                },
                GentufaWebViewMode::Tree => rsx! {
                    { render_tree(success, reference_hover, reference_tooltip_open, settings_value.script, page_find) }
                },
                GentufaWebViewMode::Ipa => rsx! {
                    { render_ipa_output(success, page_find) }
                },
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_surface_output(
    success: &GentufaSuccess,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        div { class: "brackets-section",
            div { class: "brackets-output-stack",
                pre { class: "brackets-output compact-output",
                    span { class: "brackets-output-markup",
                        for fragment in success.bracket_fragments.iter() {
                            { render_bracket_fragment(fragment, script, page_find) }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_bracket_fragment(
    fragment: &GentufaBracketFragment,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    match fragment {
        GentufaBracketFragment::Text { text, elided } => {
            if *elided {
                rsx! { s { { render_page_find_text(page_find, text) } } }
            } else {
                render_page_find_text(page_find, text)
            }
        }
        GentufaBracketFragment::Span {
            color,
            href,
            tooltip,
            children,
        } => {
            let style = color
                .as_ref()
                .map(|color| format!("color: {color};"))
                .unwrap_or_default();
            if let Some(href) = href {
                let base_path = router_base_path();
                let route = jbotci_route_from_href(&base_path, href);
                if let Some(card) = tooltip {
                    rsx! {
                        span {
                            class: "bracket-fragment bracket-word dictionary-tooltip-host",
                            style: "{style}",
                            if let Some(route) = route {
                                Link { class: "bracket-word-link", to: route,
                                    for child in children.iter() {
                                        { render_bracket_fragment(child, script, page_find) }
                                    }
                                }
                            } else {
                                a { class: "bracket-word-link", href: "{href}",
                                    for child in children.iter() {
                                        { render_bracket_fragment(child, script, page_find) }
                                    }
                                }
                            }
                            { render_dictionary_tooltip(card, false, &base_path, script) }
                        }
                    }
                } else {
                    if let Some(route) = route {
                        rsx! {
                            Link {
                                class: "bracket-fragment bracket-word",
                                style: "{style}",
                                to: route,
                                for child in children.iter() {
                                    { render_bracket_fragment(child, script, page_find) }
                                }
                            }
                        }
                    } else {
                        rsx! {
                            a {
                                class: "bracket-fragment bracket-word",
                                style: "{style}",
                                href: "{href}",
                                for child in children.iter() {
                                    { render_bracket_fragment(child, script, page_find) }
                                }
                            }
                        }
                    }
                }
            } else {
                rsx! {
                    span { class: "bracket-fragment", style: "{style}",
                        for child in children.iter() {
                            { render_bracket_fragment(child, script, page_find) }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_diagnostics_pane(
    diagnostics: &[Diagnostic],
    source: &str,
    fallback_error: Option<&str>,
    mut diagnostics_open: Signal<bool>,
    diagnostics_open_value: bool,
    active_diagnostic: Signal<Option<usize>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: Option<&PageFindContext>,
) -> Element {
    let fallback_error = fallback_error.filter(|message| !message.is_empty());
    if diagnostics.is_empty() && fallback_error.is_none() {
        return rsx! {};
    }
    let counts = diagnostic_counts(diagnostics, fallback_error);
    let title = diagnostic_pane_title(counts);
    let toggle_label = diagnostics_toggle_label(diagnostics_open_value);
    rsx! {
        section { class: "gentufa-diagnostics-pane", role: "alert", aria_live: "polite",
            div { class: "gentufa-diagnostics-header",
                h2 { class: "gentufa-diagnostics-title",
                    { render_optional_page_find_text(page_find, &title) }
                }
                button {
                    class: "gentufa-diagnostics-toggle",
                    r#type: "button",
                    aria_expanded: if diagnostics_open_value { "true" } else { "false" },
                    onclick: move |_| diagnostics_open.set(!diagnostics_open_value),
                    { render_optional_page_find_text(page_find, toggle_label) }
                }
            }
            if diagnostics_open_value {
                div { class: "gentufa-diagnostics-list",
                    if diagnostics.is_empty() {
                        if let Some(message) = fallback_error {
                            article { class: "gentufa-diagnostic-card is-error",
                                div { class: "gentufa-diagnostic-main",
                                    span { class: "gentufa-diagnostic-severity",
                                        { render_optional_page_find_text(page_find, "error") }
                                    }
                                    span { class: "gentufa-diagnostic-message",
                                        { render_optional_page_find_text(page_find, message) }
                                    }
                                }
                            }
                        }
                    } else {
                        for (index, diagnostic) in diagnostics.iter().enumerate() {
                            { render_diagnostic_card(
                                index,
                                diagnostic,
                                source,
                                active_diagnostic,
                                pending_cukta_scroll,
                                base_path,
                                script,
                                page_find,
                            ) }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_diagnostic_card(
    index: usize,
    diagnostic: &Diagnostic,
    source: &str,
    active_diagnostic: Signal<Option<usize>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: Option<&PageFindContext>,
) -> Element {
    let mut enter_active = active_diagnostic;
    let mut leave_active = active_diagnostic;
    let card_class = diagnostic_card_class(diagnostic);
    let primary = diagnostic.primary_label();
    let location = diagnostic_label_location(source, primary);
    let location_text = format!(
        "{}:{}: {}",
        location.line, location.column, diagnostic.message
    );
    let context_labels = diagnostic_context_labels(diagnostic);
    let styled_notes = diagnostic_styled_notes_for_web(diagnostic);
    let plain_notes = diagnostic_plain_notes_for_web(diagnostic);
    let primary_detail_segments = diagnostic_primary_detail_parts(diagnostic);
    rsx! {
        article {
            class: "{card_class}",
            onmouseenter: move |_| enter_active.set(Some(index)),
            onmouseleave: move |_| leave_active.set(None),
            div { class: "gentufa-diagnostic-main",
                span { class: "gentufa-diagnostic-severity",
                    { render_optional_page_find_text(page_find, diagnostic_severity_text(diagnostic.severity)) }
                }
                code { class: "gentufa-diagnostic-code",
                    { render_optional_page_find_text(page_find, &diagnostic.code) }
                }
                span { class: "gentufa-diagnostic-message",
                    { render_optional_page_find_text(page_find, &location_text) }
                }
            }
            for label in context_labels {
                { render_diagnostic_context_label(label, page_find) }
            }
            if !primary_detail_segments.is_empty() {
                div { class: "gentufa-diagnostic-primary-detail",
                    for segment in primary_detail_segments.iter() {
                        { render_diagnostic_text_part(segment, pending_cukta_scroll, base_path, script, page_find) }
                    }
                }
            }
            if !plain_notes.is_empty() || !styled_notes.is_empty() {
                div { class: "gentufa-diagnostic-notes",
                    for note in plain_notes.iter() {
                        div { class: "gentufa-diagnostic-note",
                            for segment in diagnostic_plain_text_render_parts(note).iter() {
                                { render_diagnostic_text_part(segment, pending_cukta_scroll, base_path, script, page_find) }
                            }
                        }
                    }
                    for note in styled_notes {
                        { render_styled_diagnostic_note(note, pending_cukta_scroll, base_path, script, page_find) }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_styled_diagnostic_note(
    note: &DiagnosticStyledNote,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: Option<&PageFindContext>,
) -> Element {
    let class_name = diagnostic_styled_note_class(note);
    rsx! {
        div { class: "{class_name}",
            for segment in note.segments.iter() {
                { render_diagnostic_note_segment(segment, pending_cukta_scroll, base_path, script, page_find) }
            }
        }
    }
}

#[requires(!segment.text.is_empty())]
#[ensures(true)]
fn render_diagnostic_note_segment(
    segment: &DiagnosticTextSegment,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: Option<&PageFindContext>,
) -> Element {
    let parts = diagnostic_text_segment_render_parts(segment);
    rsx! {
        for part in parts.iter() {
            { render_diagnostic_text_part(part, pending_cukta_scroll, base_path, script, page_find) }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn current_gentufa_input_diagnostics<'a>(
    input_text: &str,
    result: &'a GentufaWebResult,
    request: Option<&GentufaWebRequest>,
) -> &'a [Diagnostic] {
    if diagnostics_decorate_current_input(input_text, request) {
        gentufa_result_diagnostics(result)
    } else {
        &[]
    }
}

#[requires(true)]
#[ensures(ret -> request.is_some())]
fn diagnostics_decorate_current_input(
    input_text: &str,
    request: Option<&GentufaWebRequest>,
) -> bool {
    request.is_some_and(|request| request.text == input_text)
}

#[requires(true)]
#[ensures(true)]
fn gentufa_result_diagnostics(result: &GentufaWebResult) -> &[Diagnostic] {
    match result {
        GentufaWebResult::Blank => &[],
        GentufaWebResult::Success(success) => &success.diagnostics,
        GentufaWebResult::Error(error) => &error.diagnostics,
    }
}

#[requires(true)]
#[ensures(true)]
fn gentufa_request_source(request: Option<&GentufaWebRequest>) -> &str {
    request.map_or("", |request| request.text.as_str())
}

#[requires(true)]
#[ensures(ret.errors + ret.warnings >= diagnostics.len() || fallback_error.is_some())]
fn diagnostic_counts(diagnostics: &[Diagnostic], fallback_error: Option<&str>) -> DiagnosticCounts {
    if diagnostics.is_empty() && fallback_error.is_some() {
        return new!(DiagnosticCounts {
            errors: 1,
            warnings: 0,
        });
    }
    let mut errors = 0;
    let mut warnings = 0;
    for diagnostic in diagnostics {
        match diagnostic.severity {
            DiagnosticSeverity::Error => errors += 1,
            DiagnosticSeverity::Warning | DiagnosticSeverity::Advice => warnings += 1,
        }
    }
    new!(DiagnosticCounts { errors, warnings })
}

#[requires(true)]
#[ensures(ret.contains("Diagnostics"))]
fn diagnostic_pane_title(counts: DiagnosticCounts) -> String {
    format!(
        "Diagnostics: {}, {}",
        plural_count(counts.errors, "error", "errors"),
        plural_count(counts.warnings, "warning", "warnings")
    )
}

#[requires(!singular.is_empty())]
#[requires(!plural.is_empty())]
#[ensures(!ret.is_empty())]
fn plural_count(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn diagnostics_toggle_label(open: bool) -> &'static str {
    if open { "Hide" } else { "Show" }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn diagnostic_card_class(diagnostic: &Diagnostic) -> String {
    class_names(
        "gentufa-diagnostic-card",
        &[
            ("is-error", diagnostic.severity == DiagnosticSeverity::Error),
            (
                "is-warning",
                diagnostic.severity != DiagnosticSeverity::Error,
            ),
        ],
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn diagnostic_severity_text(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Advice => "advice",
    }
}

#[requires(true)]
#[ensures(ret.line > 0)]
#[ensures(ret.column > 0)]
fn diagnostic_label_location(source: &str, label: &DiagnosticLabel) -> DiagnosticSourceLocation {
    source_location_for_char_offset(source, label.span.char_start)
}

#[requires(true)]
#[ensures(ret.line > 0)]
#[ensures(ret.column > 0)]
fn source_location_for_char_offset(source: &str, char_offset: usize) -> DiagnosticSourceLocation {
    let mut line = 1;
    let mut column = 1;
    for (index, character) in source.chars().enumerate() {
        if index == char_offset {
            return new!(DiagnosticSourceLocation { line, column });
        }
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    new!(DiagnosticSourceLocation { line, column })
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_primary_detail(diagnostic: &Diagnostic) -> Option<&str> {
    let label = diagnostic.primary_label();
    (label.message != diagnostic.message).then_some(label.message.as_str())
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_primary_detail_parts(diagnostic: &Diagnostic) -> Vec<DiagnosticTextRenderPart> {
    if let Some(detail) = diagnostic_primary_detail(diagnostic)
        && detail.starts_with("expected:")
    {
        return diagnostic_plain_text_render_parts(detail);
    }
    diagnostic_expected_detail_parts_from_detailed_note(diagnostic).unwrap_or_else(|| {
        diagnostic_primary_detail(diagnostic)
            .map(diagnostic_plain_text_render_parts)
            .unwrap_or_default()
    })
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_primary_detail_display_text(diagnostic: &Diagnostic) -> Option<String> {
    let parts = diagnostic_primary_detail_parts(diagnostic);
    (!parts.is_empty()).then(|| diagnostic_text_parts_text(&parts))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|parts| !parts.is_empty()))]
fn diagnostic_expected_detail_parts_from_detailed_note(
    diagnostic: &Diagnostic,
) -> Option<Vec<DiagnosticTextRenderPart>> {
    let note = diagnostic.styled_notes.iter().find(|note| {
        matches!(note.mode, jbotci_diagnostics::DiagnosticNoteMode::Detailed)
            && diagnostic_styled_note_text(note)
                .trim_start()
                .starts_with("needs one of:")
    })?;
    let mut output = vec![
        diagnostic_text_render_part(DiagnosticTextRole::Keyword, "expected".to_owned()),
        diagnostic_text_render_part(DiagnosticTextRole::Plain, " ".to_owned()),
    ];
    let mut heading_seen = false;
    let mut skipping_heading_tail = false;
    let mut at_line_start = true;
    let mut pending_separator = false;
    let mut content_started = false;

    for segment in &note.segments {
        for part in diagnostic_text_segment_render_parts(segment) {
            let mut index = 0usize;
            if !heading_seen {
                if part.role == DiagnosticTextRole::Keyword && part.text == "needs one of" {
                    heading_seen = true;
                    skipping_heading_tail = true;
                }
                continue;
            }
            while index < part.text.len() {
                let Some(character) = part.text[index..].chars().next() else {
                    break;
                };
                if skipping_heading_tail {
                    index += character.len_utf8();
                    if character == '\n' {
                        skipping_heading_tail = false;
                        at_line_start = true;
                    }
                    continue;
                }
                if character == '\n' {
                    index += character.len_utf8();
                    if content_started {
                        pending_separator = true;
                    }
                    at_line_start = true;
                    continue;
                }
                if at_line_start {
                    if character.is_whitespace() {
                        index += character.len_utf8();
                        continue;
                    }
                    if character == '-' {
                        index += character.len_utf8();
                        if index < part.text.len()
                            && part.text[index..]
                                .chars()
                                .next()
                                .is_some_and(char::is_whitespace)
                        {
                            let next = part.text[index..]
                                .chars()
                                .next()
                                .expect("checked above that a character is present");
                            index += next.len_utf8();
                        }
                        continue;
                    }
                    if pending_separator {
                        output.push(diagnostic_text_render_part(
                            DiagnosticTextRole::Punctuation,
                            ", ".to_owned(),
                        ));
                        pending_separator = false;
                    }
                    at_line_start = false;
                }
                let start = index;
                index += character.len_utf8();
                while index < part.text.len() {
                    let next = part.text[index..]
                        .chars()
                        .next()
                        .expect("index is inside the current text part");
                    if next == '\n' {
                        break;
                    }
                    index += next.len_utf8();
                }
                output.push(diagnostic_text_render_part(
                    part.role,
                    part.text[start..index].to_owned(),
                ));
                content_started = true;
            }
        }
    }

    if heading_seen && content_started {
        Some(merge_diagnostic_text_parts(output))
    } else {
        None
    }
}

#[requires(true)]
#[ensures(ret.iter().all(|label| !label.primary))]
fn diagnostic_context_labels(diagnostic: &Diagnostic) -> Vec<&DiagnosticLabel> {
    diagnostic
        .labels
        .iter()
        .filter(|label| !label.primary)
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn render_diagnostic_context_label(
    label: &DiagnosticLabel,
    page_find: Option<&PageFindContext>,
) -> Element {
    let descriptor = diagnostic_context_descriptor(&label.message);
    rsx! {
        div { class: "gentufa-diagnostic-context",
            em {
                if let Some(descriptor) = descriptor {
                    { render_optional_page_find_text(page_find, "while parsing ") }
                    span { class: "gentufa-diagnostic-context-descriptor",
                        { render_optional_page_find_text(page_find, descriptor) }
                    }
                } else {
                    { render_optional_page_find_text(page_find, &label.message) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_context_descriptor(message: &str) -> Option<&str> {
    message.strip_prefix("while parsing ")
}

#[requires(true)]
#[ensures(ret.iter().all(|note| !diagnostic_plain_note_is_hidden(note)))]
fn diagnostic_plain_notes_for_web(diagnostic: &Diagnostic) -> Vec<&str> {
    diagnostic
        .notes
        .iter()
        .map(String::as_str)
        .filter(|note| !note.is_empty() && !diagnostic_plain_note_is_hidden(note))
        .collect()
}

#[requires(true)]
#[ensures(text.starts_with("expected one of:") -> ret)]
fn diagnostic_plain_note_is_hidden(text: &str) -> bool {
    text.trim_start().starts_with("expected one of:")
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_styled_notes_for_web(diagnostic: &Diagnostic) -> Vec<&DiagnosticStyledNote> {
    diagnostic
        .styled_notes
        .iter()
        .filter(|note| !diagnostic_styled_note_is_hidden(note))
        .collect()
}

#[requires(true)]
#[ensures(matches!(note.mode, jbotci_diagnostics::DiagnosticNoteMode::Summary) && diagnostic_styled_note_text(note).trim_start().starts_with("expected one of:") -> ret)]
fn diagnostic_styled_note_is_hidden(note: &DiagnosticStyledNote) -> bool {
    matches!(note.mode, jbotci_diagnostics::DiagnosticNoteMode::Summary)
        && diagnostic_styled_note_text(note)
            .trim_start()
            .starts_with("expected one of:")
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_styled_note_text(note: &DiagnosticStyledNote) -> String {
    diagnostic_text_segments_text(&note.segments)
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_text_segments_text(segments: &[DiagnosticTextSegment]) -> String {
    segments.iter().fold(String::new(), |mut text, segment| {
        text.push_str(&segment.text);
        text
    })
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_text_parts_text(parts: &[DiagnosticTextRenderPart]) -> String {
    parts.iter().fold(String::new(), |mut text, part| {
        text.push_str(&part.text);
        text
    })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn diagnostic_styled_note_class(note: &DiagnosticStyledNote) -> String {
    class_names(
        "gentufa-diagnostic-note gentufa-diagnostic-styled-note",
        &[
            (
                "is-always",
                matches!(note.mode, jbotci_diagnostics::DiagnosticNoteMode::Always),
            ),
            (
                "is-summary",
                matches!(note.mode, jbotci_diagnostics::DiagnosticNoteMode::Summary),
            ),
            (
                "is-detailed",
                matches!(note.mode, jbotci_diagnostics::DiagnosticNoteMode::Detailed),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn diagnostic_text_role_class(role: DiagnosticTextRole) -> &'static str {
    match role {
        DiagnosticTextRole::Construct => "diagnostic-text diagnostic-text-construct",
        DiagnosticTextRole::SpecificWord => "diagnostic-text diagnostic-text-specific-word",
        DiagnosticTextRole::Selmaho => "diagnostic-text diagnostic-text-selmaho",
        DiagnosticTextRole::WordCategory => "diagnostic-text diagnostic-text-word-category",
        DiagnosticTextRole::Keyword => "diagnostic-text diagnostic-text-keyword",
        DiagnosticTextRole::Punctuation => "diagnostic-text diagnostic-text-punctuation",
        DiagnosticTextRole::Plain => "diagnostic-text diagnostic-text-plain",
    }
}

#[requires(!segment.text.is_empty())]
#[ensures(!ret.is_empty())]
fn diagnostic_text_segment_render_parts(
    segment: &DiagnosticTextSegment,
) -> Vec<DiagnosticTextRenderPart> {
    if segment.role == DiagnosticTextRole::Plain {
        diagnostic_plain_text_render_parts(&segment.text)
    } else {
        vec![diagnostic_text_render_part(
            segment.role,
            diagnostic_text_segment_display_text(segment),
        )]
    }
}

#[requires(!text.is_empty())]
#[ensures(!ret.is_empty())]
fn diagnostic_plain_text_render_parts(text: &str) -> Vec<DiagnosticTextRenderPart> {
    let mut parts = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        if let Some((mut matched, next_index)) = diagnostic_plain_prefix_parts(text, index) {
            parts.append(&mut matched);
            index = next_index;
            continue;
        }
        let Some(character) = text[index..].chars().next() else {
            break;
        };
        if diagnostic_identifier_char(character) {
            let start = index;
            index += character.len_utf8();
            while index < text.len()
                && text[index..]
                    .chars()
                    .next()
                    .is_some_and(diagnostic_identifier_char)
            {
                let next = text[index..]
                    .chars()
                    .next()
                    .expect("checked above that a character is present");
                index += next.len_utf8();
            }
            let token = &text[start..index];
            let role = diagnostic_identifier_role(token).unwrap_or(DiagnosticTextRole::Plain);
            parts.push(diagnostic_text_render_part(
                role,
                diagnostic_display_text_for_role(role, token),
            ));
        } else {
            parts.push(diagnostic_text_render_part(
                if character.is_ascii_punctuation() {
                    DiagnosticTextRole::Punctuation
                } else {
                    DiagnosticTextRole::Plain
                },
                character.to_string(),
            ));
            index += character.len_utf8();
        }
    }
    merge_diagnostic_text_parts(parts)
}

#[requires(index < text.len())]
#[ensures(ret.as_ref().is_none_or(|(_, next)| *next > index && *next <= text.len()))]
fn diagnostic_plain_prefix_parts(
    text: &str,
    index: usize,
) -> Option<(Vec<DiagnosticTextRenderPart>, usize)> {
    for (label, has_colon) in DIAGNOSTIC_KEYWORD_LABELS {
        if let Some(next_index) = diagnostic_match_phrase(text, index, label) {
            let mut parts = vec![diagnostic_text_render_part(
                DiagnosticTextRole::Keyword,
                (*label).to_owned(),
            )];
            if *has_colon && text[next_index..].starts_with(':') {
                parts.push(diagnostic_text_render_part(
                    DiagnosticTextRole::Punctuation,
                    ":".to_owned(),
                ));
                return Some((parts, next_index + 1));
            }
            return Some((parts, next_index));
        }
    }
    for (phrase, display, role) in DIAGNOSTIC_PHRASE_ROLES {
        if let Some(next_index) = diagnostic_match_phrase(text, index, phrase) {
            return Some((
                vec![diagnostic_text_render_part(
                    *role,
                    diagnostic_display_text_for_role(*role, display),
                )],
                next_index,
            ));
        }
    }
    None
}

#[requires(!phrase.is_empty())]
#[requires(index < text.len())]
#[ensures(ret.is_none_or(|next| next > index && next <= text.len()))]
fn diagnostic_match_phrase(text: &str, index: usize, phrase: &str) -> Option<usize> {
    let after = index.checked_add(phrase.len())?;
    if after > text.len() || !text[index..].starts_with(phrase) {
        return None;
    }
    let before_ok = index == 0
        || text[..index]
            .chars()
            .next_back()
            .is_none_or(|character| !diagnostic_identifier_char(character));
    let after_ok = after == text.len()
        || text[after..]
            .chars()
            .next()
            .is_none_or(|character| !diagnostic_identifier_char(character));
    (before_ok && after_ok).then_some(after)
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_identifier_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '\'' | '-' | 'h')
}

#[requires(!token.is_empty())]
#[ensures(true)]
fn diagnostic_identifier_role(token: &str) -> Option<DiagnosticTextRole> {
    if diagnostic_word_category_display(token).is_some() {
        Some(DiagnosticTextRole::WordCategory)
    } else if DIAGNOSTIC_SELMAHO_NAMES.contains(&token) {
        Some(DiagnosticTextRole::Selmaho)
    } else if DIAGNOSTIC_SPECIFIC_WORDS.contains(&token) {
        Some(DiagnosticTextRole::SpecificWord)
    } else if DIAGNOSTIC_KEYWORDS.contains(&token) {
        Some(DiagnosticTextRole::Keyword)
    } else {
        None
    }
}

#[requires(!text.is_empty())]
#[ensures(!ret.is_empty())]
fn diagnostic_display_text_for_role(role: DiagnosticTextRole, text: &str) -> String {
    match role {
        DiagnosticTextRole::WordCategory => diagnostic_word_category_display(text)
            .unwrap_or(text)
            .to_owned(),
        _ => text.to_owned(),
    }
}

#[requires(!segment.text.is_empty())]
#[ensures(!ret.is_empty())]
fn diagnostic_text_segment_display_text(segment: &DiagnosticTextSegment) -> String {
    diagnostic_display_text_for_role(segment.role, &segment.text)
}

#[requires(!text.is_empty())]
#[ensures(!ret.text.is_empty())]
fn diagnostic_text_render_part(role: DiagnosticTextRole, text: String) -> DiagnosticTextRenderPart {
    new!(DiagnosticTextRenderPart { role, text })
}

#[requires(true)]
#[ensures(ret.iter().all(|part| !part.text.is_empty()))]
fn merge_diagnostic_text_parts(
    parts: Vec<DiagnosticTextRenderPart>,
) -> Vec<DiagnosticTextRenderPart> {
    let mut merged = Vec::<DiagnosticTextRenderPart>::new();
    for part in parts {
        if let Some(previous) = merged.last()
            && previous.role == part.role
            && diagnostic_text_part_href(previous, "") == diagnostic_text_part_href(&part, "")
        {
            let mut previous_data = merged
                .pop()
                .expect("last text part was checked above")
                .into_data();
            previous_data.text.push_str(&part.text);
            merged.push(DiagnosticTextRenderPart::from_data(previous_data));
            continue;
        }
        merged.push(part);
    }
    merged
}

#[requires(!part.text.is_empty())]
#[ensures(true)]
fn render_diagnostic_text_part(
    part: &DiagnosticTextRenderPart,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: Option<&PageFindContext>,
) -> Element {
    let class_name = diagnostic_text_role_class(part.role);
    let href = diagnostic_text_part_href(part, base_path);
    let label = diagnostic_display_text_part_for_script(part, script);
    if let Some(href) = href {
        render_diagnostic_text_link(
            class_name,
            &href,
            base_path,
            &label,
            pending_cukta_scroll,
            page_find,
        )
    } else {
        rsx! {
            span { class: "{class_name}",
                { render_optional_page_find_text(page_find, &label) }
            }
        }
    }
}

#[requires(!part.text.is_empty())]
#[ensures(!ret.is_empty())]
fn diagnostic_display_text_part_for_script(
    part: &DiagnosticTextRenderPart,
    script: GentufaScript,
) -> String {
    if part.role == DiagnosticTextRole::SpecificWord {
        display_lojban_text(script, &part.text)
    } else {
        part.text.clone()
    }
}

#[requires(!class_name.is_empty())]
#[requires(!href.is_empty())]
#[requires(!label.is_empty())]
#[ensures(true)]
fn render_diagnostic_text_link(
    class_name: &str,
    href: &str,
    base_path: &str,
    label: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    page_find: Option<&PageFindContext>,
) -> Element {
    let class_name = format!("{class_name} diagnostic-text-link");
    if let Some(route) = jbotci_route_from_href(base_path, href) {
        let pending_scroll = cukta_pending_scroll_for_explicit_route_link(base_path, &route);
        let click_route = route.clone();
        rsx! {
            Link {
                class: "{class_name}",
                to: route,
                onclick_only: true,
                onclick: move |_| {
                    if let Some(pending_scroll) = pending_scroll.clone() {
                        push_route_with_cukta_scroll_intent(
                            pending_cukta_scroll,
                            Some(pending_scroll),
                            click_route.clone(),
                        );
                    }
                },
                { render_optional_page_find_text(page_find, label) }
            }
        }
    } else {
        rsx! {
            a { class: "{class_name}", href: "{href}",
                { render_optional_page_find_text(page_find, label) }
            }
        }
    }
}

#[requires(!part.text.is_empty())]
#[ensures(ret.as_ref().is_none_or(|href| !href.is_empty()))]
fn diagnostic_text_part_href(part: &DiagnosticTextRenderPart, base_path: &str) -> Option<String> {
    match part.role {
        DiagnosticTextRole::SpecificWord => {
            Some(diagnostic_vlacku_href(base_path, part.text.as_str()))
        }
        DiagnosticTextRole::Selmaho => Some(diagnostic_cukta_section_href(
            base_path,
            "section-index",
            Some(part.text.as_str()),
        )),
        DiagnosticTextRole::WordCategory => diagnostic_word_category_href(base_path, &part.text),
        DiagnosticTextRole::Construct => diagnostic_construct_href(base_path, &part.text),
        DiagnosticTextRole::Keyword
        | DiagnosticTextRole::Punctuation
        | DiagnosticTextRole::Plain => None,
    }
}

#[requires(!word.is_empty())]
#[ensures(!ret.is_empty())]
fn diagnostic_vlacku_href(base_path: &str, word: &str) -> String {
    format!("{}/vlacku/{word}", base_path.trim_end_matches('/'))
}

#[requires(!section_id.is_empty())]
#[ensures(!ret.is_empty())]
fn diagnostic_cukta_section_href(
    base_path: &str,
    section_id: &str,
    anchor: Option<&str>,
) -> String {
    let mut href = format!(
        "{}/cukta/section/{section_id}",
        base_path.trim_end_matches('/')
    );
    if let Some(anchor) = anchor.filter(|anchor| !anchor.is_empty()) {
        href.push('#');
        href.push_str(anchor.trim_start_matches('#'));
    }
    href
}

#[requires(!text.is_empty())]
#[ensures(ret.as_ref().is_none_or(|href| !href.is_empty()))]
fn diagnostic_word_category_href(base_path: &str, text: &str) -> Option<String> {
    match text {
        "BRIVLA" => Some(diagnostic_cukta_section_href(
            base_path,
            "section-morphology-brivla",
            None,
        )),
        "CMEVLA" => Some(diagnostic_cukta_section_href(
            base_path,
            "section-cmevla",
            None,
        )),
        "LERFU" => Some(diagnostic_cukta_section_href(
            base_path,
            "section-lerfu-liste",
            None,
        )),
        "SELBRI WORD" => Some(diagnostic_ebnf_rule_href(base_path, "selbri")),
        "PRO-SUMTI" => Some(diagnostic_cukta_section_href(
            base_path,
            "section-anaphoric-cmavo-introduction",
            None,
        )),
        "QUOTE" => Some(diagnostic_cukta_section_href(
            base_path,
            "section-quotation",
            None,
        )),
        _ => None,
    }
}

#[requires(!text.is_empty())]
#[ensures(ret.as_ref().is_none_or(|href| !href.is_empty()))]
fn diagnostic_construct_href(base_path: &str, text: &str) -> Option<String> {
    let rule = match text {
        "FIhO modal" => "tense-modal",
        "NA KU term" => "term",
        "VUhU operator" => "mex-operator",
        "abstraction" => "tanru-unit",
        "bridi" => "bridi-tail",
        "bridi description" => "sumti-6",
        "connected tag" => "tag",
        "converted operator" => "mex-operator",
        "converted sumti" => "sumti-6",
        "converted tanru unit" => "tanru-unit-2",
        "descriptor" => "sumti-6",
        "description" => "sumti-tail",
        "description tail" => "sumti-tail",
        "forethought bridi connection" => "gek-sentence",
        "forethought mex" => "mex",
        "forethought selbri connection" => "selbri-6",
        "forethought sumti connection" => "sumti-4",
        "free modifier" => "free",
        "fragment" => "fragment",
        "grouped tanru" => "tanru-unit-2",
        "lerfu string" => "lerfu-string",
        "linked arguments" => "linkargs",
        "mekso array" => "operand-3",
        "mex" => "mex",
        "modal conversion" => "tanru-unit-2",
        "modal tag" => "simple-tense-modal",
        "name" => "sumti-6",
        "negated selbri" => "selbri-1",
        "non-Lojban quote" => "sumti-6",
        "number sumti" => "sumti-6",
        "number" => "number",
        "operand" => "operand",
        "operand-to-operator" => "mex-operator",
        "operator" => "operator",
        "operator-to-selbri" => "tanru-unit-2",
        "ordinal selbri" => "tanru-unit-2",
        "parenthesized mex" => "mex",
        "place tag" => "term",
        "pro-sumti" => "sumti-6",
        "quantifier" => "quantifier",
        "qualified operand" => "operand-3",
        "quote" => "sumti-6",
        "relative clause" => "relative-clause",
        "relative clauses" => "relative-clauses",
        "reverse Polish mex" => "rp-expression",
        "selbri operand" => "operand-3",
        "selbri relative phrase" => "tanru-unit",
        "selbri-to-operator" => "mex-operator",
        "simple tense/modal" => "simple-tense-modal",
        "space tense" => "space",
        "subbridi" => "subsentence",
        "sumti" => "sumti",
        "sumti operand" => "operand-3",
        "sumti-to-selbri" => "tanru-unit-2",
        "tag" => "tag",
        "tail terms" => "tail-terms",
        "tanru" => "selbri",
        "tanru unit" => "tanru-unit",
        "term" => "term",
        "termset" => "termset",
        "terms" => "terms",
        "selbri" => "selbri",
        "statement" => "statement",
        "text group" => "statement-3",
        "text quote" => "sumti-6",
        "time tense" => "time",
        "word quote" => "sumti-6",
        "word-sequence quote" => "sumti-6",
        "metalinguistic comment"
        | "parenthetical text"
        | "reciprocal"
        | "replacement phrase"
        | "subscript"
        | "utterance ordinal"
        | "vocative phrase" => "free",
        "text" | "parse_text" => "text",
        _ => return None,
    };
    Some(diagnostic_ebnf_rule_href(base_path, rule))
}

#[requires(!rule_name.is_empty())]
#[ensures(!ret.is_empty())]
fn diagnostic_ebnf_rule_href(base_path: &str, rule_name: &str) -> String {
    diagnostic_cukta_section_href(
        base_path,
        "section-EBNF",
        Some(jbotci_cll::ebnf_rule_anchor_id(rule_name).as_str()),
    )
}

#[requires(!text.is_empty())]
#[ensures(ret.is_none_or(|display| !display.is_empty()))]
fn diagnostic_word_category_display(text: &str) -> Option<&'static str> {
    match text {
        "BRIVLA" => Some("BRIVLA"),
        "CMEVLA" => Some("CMEVLA"),
        "LERFU" => Some("LERFU"),
        "SELBRI WORD" => Some("SELBRI WORD"),
        "PRO-SUMTI" => Some("PRO-SUMTI"),
        "REPLACEMENT WORD" => Some("REPLACEMENT WORD"),
        "QUOTE" => Some("QUOTE"),
        _ => None,
    }
}

const DIAGNOSTIC_KEYWORD_LABELS: &[(&str, bool)] = &[
    ("expected one of", true),
    ("needs one of", true),
    ("morphology detail", true),
    ("reason", true),
    ("expected", false),
];

const DIAGNOSTIC_PHRASE_ROLES: &[(&str, &str, DiagnosticTextRole)] = &[
    (
        "forethought bridi connection",
        "forethought bridi connection",
        DiagnosticTextRole::Construct,
    ),
    (
        "forethought selbri connection",
        "forethought selbri connection",
        DiagnosticTextRole::Construct,
    ),
    (
        "forethought sumti connection",
        "forethought sumti connection",
        DiagnosticTextRole::Construct,
    ),
    (
        "forethought mex",
        "forethought mex",
        DiagnosticTextRole::Construct,
    ),
    (
        "metalinguistic comment",
        "metalinguistic comment",
        DiagnosticTextRole::Construct,
    ),
    (
        "word-sequence quote",
        "word-sequence quote",
        DiagnosticTextRole::Construct,
    ),
    (
        "parenthetical text",
        "parenthetical text",
        DiagnosticTextRole::Construct,
    ),
    (
        "operand-to-operator",
        "operand-to-operator",
        DiagnosticTextRole::Construct,
    ),
    (
        "operator-to-selbri",
        "operator-to-selbri",
        DiagnosticTextRole::Construct,
    ),
    (
        "selbri-to-operator",
        "selbri-to-operator",
        DiagnosticTextRole::Construct,
    ),
    (
        "sumti-to-selbri",
        "sumti-to-selbri",
        DiagnosticTextRole::Construct,
    ),
    (
        "converted tanru unit",
        "converted tanru unit",
        DiagnosticTextRole::Construct,
    ),
    (
        "selbri relative phrase",
        "selbri relative phrase",
        DiagnosticTextRole::Construct,
    ),
    (
        "simple tense/modal",
        "simple tense/modal",
        DiagnosticTextRole::Construct,
    ),
    (
        "reverse Polish mex",
        "reverse Polish mex",
        DiagnosticTextRole::Construct,
    ),
    (
        "replacement phrase",
        "replacement phrase",
        DiagnosticTextRole::Construct,
    ),
    (
        "bridi description",
        "bridi description",
        DiagnosticTextRole::Construct,
    ),
    (
        "description tail",
        "description tail",
        DiagnosticTextRole::Construct,
    ),
    (
        "free modifier",
        "free modifier",
        DiagnosticTextRole::Construct,
    ),
    (
        "linked arguments",
        "linked arguments",
        DiagnosticTextRole::Construct,
    ),
    (
        "parenthesized mex",
        "parenthesized mex",
        DiagnosticTextRole::Construct,
    ),
    (
        "converted operator",
        "converted operator",
        DiagnosticTextRole::Construct,
    ),
    (
        "converted sumti",
        "converted sumti",
        DiagnosticTextRole::Construct,
    ),
    (
        "non-Lojban quote",
        "non-Lojban quote",
        DiagnosticTextRole::Construct,
    ),
    (
        "utterance ordinal",
        "utterance ordinal",
        DiagnosticTextRole::Construct,
    ),
    (
        "vocative phrase",
        "vocative phrase",
        DiagnosticTextRole::Construct,
    ),
    (
        "grouped tanru",
        "grouped tanru",
        DiagnosticTextRole::Construct,
    ),
    (
        "modal conversion",
        "modal conversion",
        DiagnosticTextRole::Construct,
    ),
    (
        "ordinal selbri",
        "ordinal selbri",
        DiagnosticTextRole::Construct,
    ),
    (
        "qualified operand",
        "qualified operand",
        DiagnosticTextRole::Construct,
    ),
    (
        "selbri operand",
        "selbri operand",
        DiagnosticTextRole::Construct,
    ),
    (
        "sumti operand",
        "sumti operand",
        DiagnosticTextRole::Construct,
    ),
    (
        "connected tag",
        "connected tag",
        DiagnosticTextRole::Construct,
    ),
    (
        "lerfu string",
        "lerfu string",
        DiagnosticTextRole::Construct,
    ),
    ("mekso array", "mekso array", DiagnosticTextRole::Construct),
    ("modal tag", "modal tag", DiagnosticTextRole::Construct),
    ("NA KU term", "NA KU term", DiagnosticTextRole::Construct),
    (
        "negated selbri",
        "negated selbri",
        DiagnosticTextRole::Construct,
    ),
    (
        "number sumti",
        "number sumti",
        DiagnosticTextRole::Construct,
    ),
    ("place tag", "place tag", DiagnosticTextRole::Construct),
    ("space tense", "space tense", DiagnosticTextRole::Construct),
    ("text group", "text group", DiagnosticTextRole::Construct),
    ("text quote", "text quote", DiagnosticTextRole::Construct),
    ("time tense", "time tense", DiagnosticTextRole::Construct),
    (
        "VUhU operator",
        "VUhU operator",
        DiagnosticTextRole::Construct,
    ),
    ("word quote", "word quote", DiagnosticTextRole::Construct),
    ("FIhO modal", "FIhO modal", DiagnosticTextRole::Construct),
    ("abstraction", "abstraction", DiagnosticTextRole::Construct),
    ("descriptor", "descriptor", DiagnosticTextRole::Construct),
    ("description", "description", DiagnosticTextRole::Construct),
    ("fragment", "fragment", DiagnosticTextRole::Construct),
    ("subbridi", "subbridi", DiagnosticTextRole::Construct),
    ("prenex", "prenex", DiagnosticTextRole::Construct),
    ("bridi", "bridi", DiagnosticTextRole::Construct),
    ("mex", "mex", DiagnosticTextRole::Construct),
    ("name", "name", DiagnosticTextRole::Construct),
    ("number", "number", DiagnosticTextRole::Construct),
    ("operand", "operand", DiagnosticTextRole::Construct),
    ("operator", "operator", DiagnosticTextRole::Construct),
    ("pro-sumti", "pro-sumti", DiagnosticTextRole::Construct),
    ("quantifier", "quantifier", DiagnosticTextRole::Construct),
    ("quote", "quote", DiagnosticTextRole::Construct),
    (
        "relative clauses",
        "relative clauses",
        DiagnosticTextRole::Construct,
    ),
    (
        "relative clause",
        "relative clause",
        DiagnosticTextRole::Construct,
    ),
    ("sumti", "sumti", DiagnosticTextRole::Construct),
    ("selbri", "selbri", DiagnosticTextRole::Construct),
    ("statement", "statement", DiagnosticTextRole::Construct),
    (
        "syntax construct",
        "syntax construct",
        DiagnosticTextRole::Construct,
    ),
    ("subscript", "subscript", DiagnosticTextRole::Construct),
    ("tag", "tag", DiagnosticTextRole::Construct),
    ("tail terms", "tail terms", DiagnosticTextRole::Construct),
    ("tanru unit", "tanru unit", DiagnosticTextRole::Construct),
    ("tanru", "tanru", DiagnosticTextRole::Construct),
    ("termset", "termset", DiagnosticTextRole::Construct),
    ("terms", "terms", DiagnosticTextRole::Construct),
    ("term", "term", DiagnosticTextRole::Construct),
    ("text", "text", DiagnosticTextRole::Construct),
    (
        "end of input",
        "end of input",
        DiagnosticTextRole::Construct,
    ),
    (
        "SELBRI WORD",
        "SELBRI WORD",
        DiagnosticTextRole::WordCategory,
    ),
    (
        "REPLACEMENT WORD",
        "REPLACEMENT WORD",
        DiagnosticTextRole::WordCategory,
    ),
];

const DIAGNOSTIC_KEYWORDS: &[&str] = &["continues", "ends"];

const DIAGNOSTIC_SPECIFIC_WORDS: &[&str] = &[
    "doi", "fe'e", "fi'o", "fu'a", "jo'i", "ke", "ki", "le", "le'ai", "lo", "lo'ai", "ma'o",
    "mo'e", "na'u", "ni'e", "pe'o", "sa'ai", "soi", "tei", "vei",
];

const DIAGNOSTIC_SELMAHO_NAMES: &[&str] = &[
    "A", "BAI", "BE", "BEI", "BEhO", "BIhE", "BIhI", "BO", "BOI", "BRIVLA", "BU", "BY", "CAI",
    "CAhA", "CEI", "CEhE", "CMEVLA", "CO", "COI", "CU", "CUhE", "DAhO", "DOI", "DOhU", "FA",
    "FAhA", "FEhE", "FEhU", "FOI", "FUhA", "FUhE", "FUhO", "GA", "GAhO", "GEhU", "GI", "GIhA",
    "GOI", "GOhA", "GUhA", "I", "JA", "JAI", "JOhI", "JOI", "KE", "KEI", "KEhE", "KI", "KOhA",
    "KU", "KUhE", "KUhO", "LA", "LAU", "LAhE", "LE", "LEhU", "LI", "LIhU", "LOhO", "LOhU", "LU",
    "LUhU", "MAI", "MAhO", "ME", "MEhU", "MOI", "MOhE", "MOhI", "NA", "NAI", "NAhE", "NAhU",
    "NIhO", "NOI", "NU", "NUhA", "NUhI", "NUhU", "PA", "PEhE", "PEhO", "PU", "RAhO", "ROI", "SA",
    "SE", "SEI", "SEhU", "SI", "SOI", "SU", "TAhE", "TEI", "TEhU", "TO", "TOI", "TUhE", "TUhU",
    "UI", "VA", "VAU", "VEI", "VEhA", "VEhO", "VIhA", "VUhO", "VUhU", "XI", "Y", "ZA", "ZAhO",
    "ZEI", "ZEhA", "ZI", "ZIhE", "ZO", "ZOhU", "ZOI",
];

#[requires(true)]
#[ensures(true)]
fn diagnostic_overlay_fragments(
    text: &str,
    diagnostics: &[Diagnostic],
    active_diagnostic: Option<usize>,
) -> Vec<DiagnosticOverlayFragment> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut fragments = Vec::new();
    let mut run_text = String::new();
    let mut run_class = String::new();
    let mut run_selection_start = 0u32;
    let mut run_diagnostic_index = None;
    let mut selection_offset = 0u32;

    for index in 0..=chars.len() {
        if has_diagnostic_caret_at(index, chars.len(), diagnostics, active_diagnostic) {
            flush_diagnostic_overlay_run(
                &mut fragments,
                &mut run_text,
                &mut run_class,
                &mut run_selection_start,
                &mut run_diagnostic_index,
            );
            push_diagnostic_overlay_carets(
                &mut fragments,
                index,
                chars.len(),
                selection_offset,
                diagnostics,
                active_diagnostic,
            );
        }
        let Some(character) = chars.get(index) else {
            break;
        };
        let mark =
            diagnostic_overlay_mark_for_char(index, chars.len(), diagnostics, active_diagnostic);
        let class_name = diagnostic_overlay_class(mark, diagnostics);
        let diagnostic_index = mark.map(|mark| mark.diagnostic_index);
        if !run_text.is_empty()
            && (run_class != class_name || run_diagnostic_index != diagnostic_index)
        {
            flush_diagnostic_overlay_run(
                &mut fragments,
                &mut run_text,
                &mut run_class,
                &mut run_selection_start,
                &mut run_diagnostic_index,
            );
        }
        if run_text.is_empty() {
            run_class = class_name;
            run_selection_start = selection_offset;
            run_diagnostic_index = diagnostic_index;
        }
        run_text.push(*character);
        selection_offset += character.len_utf16() as u32;
    }
    flush_diagnostic_overlay_run(
        &mut fragments,
        &mut run_text,
        &mut run_class,
        &mut run_selection_start,
        &mut run_diagnostic_index,
    );
    mark_active_context_overlay_groups(&mut fragments);
    fragments
}

#[requires(true)]
#[ensures(run_text.is_empty())]
fn flush_diagnostic_overlay_run(
    fragments: &mut Vec<DiagnosticOverlayFragment>,
    run_text: &mut String,
    run_class: &mut String,
    run_selection_start: &mut u32,
    run_diagnostic_index: &mut Option<usize>,
) {
    if run_text.is_empty() {
        return;
    }
    fragments.push(new!(DiagnosticOverlayFragment {
        text: std::mem::take(run_text),
        class_name: std::mem::take(run_class),
        selection_start: *run_selection_start,
        diagnostic_index: *run_diagnostic_index,
    }));
    *run_selection_start = 0;
    *run_diagnostic_index = None;
}

#[requires(true)]
#[ensures(true)]
fn mark_active_context_overlay_groups(fragments: &mut [DiagnosticOverlayFragment]) {
    let mut group_start = None;
    for index in 0..=fragments.len() {
        let in_group = fragments
            .get(index)
            .is_some_and(|fragment| diagnostic_overlay_fragment_is_active_context(fragment));
        match (group_start, in_group) {
            (None, true) => group_start = Some(index),
            (Some(start), false) => {
                mark_active_context_overlay_group(fragments, start, index);
                group_start = None;
            }
            _ => {}
        }
    }
}

#[requires(start < end)]
#[requires(end <= fragments.len())]
#[ensures(true)]
fn mark_active_context_overlay_group(
    fragments: &mut [DiagnosticOverlayFragment],
    start: usize,
    end: usize,
) {
    if let Some(first) = fragments.get_mut(start) {
        append_diagnostic_overlay_fragment_css_class(first, "is-active-context-start");
    }
    if let Some(last) = fragments.get_mut(end - 1) {
        append_diagnostic_overlay_fragment_css_class(last, "is-active-context-end");
    }
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_overlay_fragment_is_active_context(fragment: &DiagnosticOverlayFragment) -> bool {
    css_class_contains(&fragment.class_name, "is-active-context")
        || css_class_contains(&fragment.class_name, "is-active-context-token")
}

#[requires(!class_to_add.is_empty())]
#[ensures(css_class_contains(&fragment.class_name, class_to_add))]
fn append_diagnostic_overlay_fragment_css_class(
    fragment: &mut DiagnosticOverlayFragment,
    class_to_add: &str,
) {
    if css_class_contains(&fragment.class_name, class_to_add) {
        return;
    }
    let mut data = fragment.clone().into_data();
    append_css_class(&mut data.class_name, class_to_add);
    *fragment = DiagnosticOverlayFragment::from_data(data);
}

#[requires(!class_name.is_empty())]
#[requires(!class_to_add.is_empty())]
#[ensures(css_class_contains(class_name, class_to_add))]
fn append_css_class(class_name: &mut String, class_to_add: &str) {
    if css_class_contains(class_name, class_to_add) {
        return;
    }
    class_name.push(' ');
    class_name.push_str(class_to_add);
}

#[requires(true)]
#[ensures(true)]
fn css_class_contains(class_name: &str, expected: &str) -> bool {
    class_name
        .split_whitespace()
        .any(|class_name| class_name == expected)
}

#[requires(index <= char_len)]
#[ensures(true)]
fn has_diagnostic_caret_at(
    index: usize,
    char_len: usize,
    diagnostics: &[Diagnostic],
    active_diagnostic: Option<usize>,
) -> bool {
    diagnostics
        .iter()
        .enumerate()
        .any(|(diagnostic_index, diagnostic)| {
            diagnostic.labels.iter().any(|label| {
                diagnostic_label_is_visible_in_overlay(diagnostic_index, label, active_diagnostic)
                    && label_span_char_range(label, char_len) == (index, index)
            })
        })
}

#[requires(index <= char_len)]
#[ensures(true)]
fn push_diagnostic_overlay_carets(
    fragments: &mut Vec<DiagnosticOverlayFragment>,
    index: usize,
    char_len: usize,
    selection_offset: u32,
    diagnostics: &[Diagnostic],
    active_diagnostic: Option<usize>,
) {
    for (diagnostic_index, diagnostic) in diagnostics.iter().enumerate() {
        for label in &diagnostic.labels {
            if !diagnostic_label_is_visible_in_overlay(diagnostic_index, label, active_diagnostic) {
                continue;
            }
            if label_span_char_range(label, char_len) != (index, index) {
                continue;
            }
            let role = if label.primary {
                if active_diagnostic == Some(diagnostic_index) {
                    DiagnosticOverlayRole::ActivePrimary
                } else {
                    DiagnosticOverlayRole::Primary
                }
            } else {
                DiagnosticOverlayRole::ActiveContextPrefix
            };
            let mark = Some(DiagnosticOverlayMark {
                diagnostic_index,
                role,
            });
            fragments.push(new!(DiagnosticOverlayFragment {
                text: String::new(),
                class_name: diagnostic_overlay_caret_class(mark, diagnostics),
                selection_start: selection_offset,
                diagnostic_index: mark.map(|mark| mark.diagnostic_index),
            }));
        }
    }
}

#[requires(index < char_len)]
#[ensures(true)]
fn diagnostic_overlay_mark_for_char(
    index: usize,
    char_len: usize,
    diagnostics: &[Diagnostic],
    active_diagnostic: Option<usize>,
) -> Option<DiagnosticOverlayMark> {
    if let Some(active_index) = active_diagnostic
        && let Some(active) = diagnostics.get(active_index)
    {
        if label_contains_char(active.primary_label(), index, char_len) {
            return Some(DiagnosticOverlayMark {
                diagnostic_index: active_index,
                role: DiagnosticOverlayRole::ActivePrimary,
            });
        }
        if active_context_range_contains_char(active, index, char_len) {
            return Some(DiagnosticOverlayMark {
                diagnostic_index: active_index,
                role: DiagnosticOverlayRole::ActiveContextPrefix,
            });
        }
    }
    primary_overlay_mark_for_char(index, char_len, diagnostics)
}

#[requires(index < char_len)]
#[ensures(true)]
fn active_context_range_contains_char(
    diagnostic: &Diagnostic,
    index: usize,
    char_len: usize,
) -> bool {
    let (primary_start, primary_end) = label_span_char_range(diagnostic.primary_label(), char_len);
    diagnostic.labels.iter().any(|label| {
        if label.primary {
            return false;
        }
        let (context_start, _) = label_span_char_range(label, char_len);
        let start = context_start.min(primary_start);
        let end = primary_end.max(primary_start);
        start <= index && index < end
    })
}

#[requires(index < char_len)]
#[ensures(true)]
fn primary_overlay_mark_for_char(
    index: usize,
    char_len: usize,
    diagnostics: &[Diagnostic],
) -> Option<DiagnosticOverlayMark> {
    diagnostics
        .iter()
        .enumerate()
        .filter(|(_, diagnostic)| diagnostic.severity == DiagnosticSeverity::Error)
        .find(|(_, diagnostic)| label_contains_char(diagnostic.primary_label(), index, char_len))
        .map(|(diagnostic_index, _)| DiagnosticOverlayMark {
            diagnostic_index,
            role: DiagnosticOverlayRole::Primary,
        })
        .or_else(|| {
            diagnostics
                .iter()
                .enumerate()
                .find(|(_, diagnostic)| {
                    label_contains_char(diagnostic.primary_label(), index, char_len)
                })
                .map(|(diagnostic_index, _)| DiagnosticOverlayMark {
                    diagnostic_index,
                    role: DiagnosticOverlayRole::Primary,
                })
        })
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_label_is_visible_in_overlay(
    diagnostic_index: usize,
    label: &DiagnosticLabel,
    active_diagnostic: Option<usize>,
) -> bool {
    label.primary || active_diagnostic == Some(diagnostic_index)
}

#[requires(true)]
#[ensures(true)]
fn label_contains_char(label: &DiagnosticLabel, index: usize, char_len: usize) -> bool {
    let (start, end) = label_span_char_range(label, char_len);
    start <= index && index < end
}

#[requires(true)]
#[ensures(ret.0 <= ret.1)]
#[ensures(ret.1 <= char_len)]
fn label_span_char_range(label: &DiagnosticLabel, char_len: usize) -> (usize, usize) {
    let start = label.span.char_start.min(char_len);
    let end = label.span.char_end.min(char_len).max(start);
    (start, end)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn diagnostic_overlay_class(
    mark: Option<DiagnosticOverlayMark>,
    diagnostics: &[Diagnostic],
) -> String {
    let Some(mark) = mark else {
        return "gentufa-diagnostic-overlay-fragment".to_owned();
    };
    diagnostic_overlay_mark_class("gentufa-diagnostic-overlay-fragment", mark, diagnostics)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn diagnostic_overlay_caret_class(
    mark: Option<DiagnosticOverlayMark>,
    diagnostics: &[Diagnostic],
) -> String {
    let Some(mark) = mark else {
        return "gentufa-diagnostic-overlay-caret".to_owned();
    };
    diagnostic_overlay_mark_class("gentufa-diagnostic-overlay-caret", mark, diagnostics)
}

#[requires(!base.is_empty())]
#[ensures(!ret.is_empty())]
fn diagnostic_overlay_mark_class(
    base: &str,
    mark: DiagnosticOverlayMark,
    diagnostics: &[Diagnostic],
) -> String {
    let severity = diagnostics
        .get(mark.diagnostic_index)
        .map(|diagnostic| diagnostic.severity)
        .unwrap_or(DiagnosticSeverity::Warning);
    class_names(
        base,
        &[
            ("has-diagnostic", true),
            ("is-error", severity == DiagnosticSeverity::Error),
            ("is-warning", severity != DiagnosticSeverity::Error),
            (
                "is-active-primary",
                mark.role == DiagnosticOverlayRole::ActivePrimary,
            ),
            (
                "is-active-context",
                mark.role == DiagnosticOverlayRole::ActiveContextPrefix,
            ),
            (
                "is-active-context-token",
                mark.role == DiagnosticOverlayRole::ActivePrimary
                    && diagnostics
                        .get(mark.diagnostic_index)
                        .is_some_and(|diagnostic| {
                            diagnostic.labels.iter().any(|label| !label.primary)
                        }),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn diagnostic_tooltip_text(diagnostic: &Diagnostic) -> String {
    let message = diagnostic_primary_detail_display_text(diagnostic)
        .unwrap_or_else(|| diagnostic.message.clone());
    format!("{}: {message}", diagnostic.code)
}

#[requires(true)]
#[ensures(true)]
fn render_view_tabs(
    mut view_mode: Signal<GentufaWebViewMode>,
    current: GentufaWebViewMode,
) -> Element {
    rsx! {
        div { class: "view-tabs",
            button {
                class: view_tab_class(current == GentufaWebViewMode::Blocks),
                r#type: "button",
                aria_current: if current == GentufaWebViewMode::Blocks { "page" } else { "false" },
                onclick: move |_| {
                    view_mode.set(GentufaWebViewMode::Blocks);
                },
                "Blocks"
            }
            button {
                class: view_tab_class(current == GentufaWebViewMode::Tree),
                r#type: "button",
                aria_current: if current == GentufaWebViewMode::Tree { "page" } else { "false" },
                onclick: move |_| {
                    view_mode.set(GentufaWebViewMode::Tree);
                },
                "Tree"
            }
            button {
                class: view_tab_class(current == GentufaWebViewMode::Ipa),
                r#type: "button",
                aria_current: if current == GentufaWebViewMode::Ipa { "page" } else { "false" },
                onclick: move |_| {
                    view_mode.set(GentufaWebViewMode::Ipa);
                },
                "IPA"
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_output_controls(
    display: Signal<GentufaDisplayState>,
    current: GentufaDisplayState,
) -> Element {
    rsx! {
        div { class: "controls output-controls",
            { render_gloss_checkbox(display, current.show_glosses) }
            { render_elided_checkbox(display, current.show_elided) }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_gloss_checkbox(mut display: Signal<GentufaDisplayState>, checked: bool) -> Element {
    rsx! {
        label {
            input {
                r#type: "checkbox",
                checked,
                onchange: move |_| toggle_glosses(&mut display),
            }
            " gloss"
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_elided_checkbox(mut display: Signal<GentufaDisplayState>, checked: bool) -> Element {
    rsx! {
        label {
            input {
                r#type: "checkbox",
                checked,
                onchange: move |_| toggle_elided(&mut display),
            }
            " elided"
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_ipa_output(success: &GentufaSuccess, page_find: &PageFindContext) -> Element {
    rsx! {
        section { class: "ipa-view",
            pre { class: "ipa-tab-output",
                { render_page_find_text(page_find, &success.ipa_text) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_blocks(
    success: &GentufaSuccess,
    show_glosses: bool,
    script: GentufaScript,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    activity: Signal<AsyncActivityState>,
    export_task: Signal<Option<LatestAsyncTask>>,
    page_find: &PageFindContext,
) -> Element {
    let column_count = success.blocks_layout.max_col.max(1);
    let column_template = repeated_parse_tree_template(column_count);
    let row_template = blocks_grid_row_template(success.blocks_layout.max_row, show_glosses);
    let container_class = if show_glosses {
        "blocks-container"
    } else {
        "blocks-container gloss-hidden"
    };
    let gloss_row = success.blocks_layout.max_row + 1;
    let export_anchor_id = success
        .blocks_layout
        .blocks
        .iter()
        .min_by_key(|block| (block.row, std::cmp::Reverse(block.col + block.col_span)))
        .map(|block| block.block_id.as_str());
    rsx! {
        section { class: "blocks-view",
            div { class: "blocks-scroll-shell",
                div {
                    class: "blocks-scroll-viewport",
                    "data-jbotci-blocks-scroll-viewport": "1",
                    div {
                        class: "{container_class}",
                        "data-elided": "0",
                        "data-col-count": "{column_count}",
                        div {
                            class: "blocks-grid",
                            style: "grid-template-columns: {column_template}; grid-template-rows: {row_template};",
                            for row in 0..success.blocks_layout.max_row {
                                { render_block_row_height_probe(row, column_count) }
                            }
                            for block in success.blocks_layout.blocks.iter() {
                                { render_block_reference_height_sizer(block) }
                                { render_block(block, reference_hover, reference_tooltip_open, export_anchor_id, &success.blocks_layout, show_glosses, script, activity, export_task, page_find) }
                            }
                            if show_glosses {
                                for block in success.blocks_layout.blocks.iter().filter(|block| block.is_leaf) {
                                    { render_gloss_block(block, gloss_row, page_find) }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(ret >= block.row)]
fn block_bottom_row(block: &GentufaBlock) -> usize {
    block.row + block.row_span.saturating_sub(1)
}

#[requires(true)]
#[ensures(true)]
fn block_has_incoming_reference(block: &GentufaBlock) -> bool {
    block
        .ref_markers
        .iter()
        .any(|marker| matches!(marker.role, ReferenceMarkerRole::Referent))
}

#[requires(true)]
#[ensures(ret == block_has_incoming_reference(block))]
fn block_needs_reference_height_sizer(block: &GentufaBlock) -> bool {
    block_has_incoming_reference(block)
}

#[requires(true)]
#[ensures(ret >= 0.0)]
fn reference_clearance_deficit(reference_bottom: f64, label_top: f64, existing_growth: f64) -> f64 {
    let clearance = label_top + existing_growth - reference_bottom;
    (BLOCK_REFERENCE_LABEL_GAP_PX - clearance).max(0.0)
}

#[requires(true)]
#[ensures(ret >= 0.0)]
fn reference_containment_deficit(
    reference_bottom: f64,
    block_height: f64,
    existing_growth: f64,
) -> f64 {
    (reference_bottom + BLOCK_REFERENCE_CONTAINMENT_GAP_PX - block_height - existing_growth)
        .max(0.0)
}

#[requires(left_start <= left_end)]
#[requires(right_start <= right_end)]
#[ensures(true)]
fn horizontal_ranges_overlap(
    left_start: f64,
    left_end: f64,
    right_start: f64,
    right_end: f64,
) -> bool {
    left_start < right_end && right_start < left_end
}

#[requires(true)]
#[ensures(ret >= 0.0)]
fn reference_rect_width(rect: ReferenceRect) -> f64 {
    (rect.right - rect.left).max(0.0)
}

#[requires(true)]
#[requires(tooltip_size.width >= 0.0)]
#[requires(tooltip_size.height >= 0.0)]
#[requires(viewport.top >= 0.0)]
#[requires(viewport.width >= 0.0)]
#[requires(viewport.height >= 0.0)]
#[ensures(ret.left.is_finite())]
#[ensures(ret.top.is_finite())]
#[ensures(ret.top >= viewport.top + DICTIONARY_TOOLTIP_VIEWPORT_MARGIN_PX)]
fn dictionary_tooltip_position(
    host_rect: ReferenceRect,
    tooltip_size: ElementSize,
    viewport: TooltipViewport,
) -> PositionedPoint {
    let margin = DICTIONARY_TOOLTIP_VIEWPORT_MARGIN_PX;
    let gap = DICTIONARY_TOOLTIP_HOST_GAP_PX;
    let tooltip_width = tooltip_size.width.max(1.0);
    let tooltip_height = tooltip_size.height.max(1.0);
    let host_width = reference_rect_width(host_rect);
    let max_left = (viewport.width - tooltip_width - margin).max(margin);
    let centered_left = host_rect.left + host_width / 2.0 - tooltip_width / 2.0;
    let left = centered_left.clamp(margin, max_left);
    let min_top = viewport.top + margin;
    let preferred_top = host_rect.top - tooltip_height - gap;
    let max_top = (viewport.height - tooltip_height - margin).max(min_top);
    let top = if preferred_top >= min_top {
        preferred_top.min(max_top)
    } else {
        (host_rect.bottom + gap).clamp(min_top, max_top)
    };
    PositionedPoint { left, top }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn blocks_grid_row_template(row_count: usize, show_glosses: bool) -> String {
    let mut tracks = Vec::with_capacity(row_count + usize::from(show_glosses));
    for _ in 0..row_count {
        tracks.push("minmax(var(--blocks-compact-min-height), auto)");
    }
    if show_glosses {
        tracks.push("auto");
    }
    if tracks.is_empty() {
        "auto".to_owned()
    } else {
        tracks.join(" ")
    }
}

#[requires(true)]
#[ensures(true)]
fn render_block_row_height_probe(row: usize, column_count: usize) -> Element {
    let grid_row = row + 1;
    let style = format!("grid-row: {grid_row} / span 1; grid-column: 1 / span {column_count};");
    rsx! {
        span {
            key: "row-probe-{row}",
            class: "block-row-height-probe",
            style: "{style}",
            "data-block-row": "{row}",
            aria_hidden: "true",
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_block_reference_height_sizer(block: &GentufaBlock) -> Element {
    if !block_needs_reference_height_sizer(block) {
        return rsx! {};
    }

    let bottom_row = block_bottom_row(block);
    let row = bottom_row + 1;
    let col = block.col + 1;
    let style = format!(
        "grid-row: {row} / span 1; grid-column: {col} / span {};",
        block.col_span
    );
    rsx! {
        span {
            key: "edge-height-{block.block_id}",
            class: "block-row-height-sizer",
            style: "{style}",
            "data-block-row": "{bottom_row}",
            aria_hidden: "true",
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_block(
    block: &GentufaBlock,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    export_anchor_id: Option<&str>,
    export_layout: &GentufaBlocksLayout,
    export_show_glosses: bool,
    export_script: GentufaScript,
    activity: Signal<AsyncActivityState>,
    export_task: Signal<Option<LatestAsyncTask>>,
    page_find: &PageFindContext,
) -> Element {
    let row = block.row + 1;
    let col = block.col + 1;
    let classes = block_class(block);
    let hover_state = reference_hover.read().clone();
    let tooltip_open_state = reference_tooltip_open.read().clone();
    let incoming_count = block
        .ref_markers
        .iter()
        .filter(|marker| marker.role == ReferenceMarkerRole::Referent)
        .count();
    let incoming_class = if incoming_count > 1 {
        "block-ref-target has-multiple"
    } else {
        "block-ref-target"
    };
    let style = format!(
        "grid-row: {row} / span {}; grid-column: {col} / span {}; --block-color: {}; background-color: {};",
        block.row_span, block.col_span, block.color, block.color
    );
    let native_title = block_native_title(block, export_script);
    let is_export_anchor = export_anchor_id == Some(block.block_id.as_str());
    let export_controls =
        is_export_anchor.then(|| (export_layout.clone(), export_show_glosses, export_script));
    rsx! {
        div {
            key: "{block.block_id}",
            class: "{classes}",
            style: "{style}",
            "data-block-id": "{block.block_id}",
            "data-row": "{block.row}",
            "data-rowspan": "{block.row_span}",
            "data-col": "{block.col}",
            "data-colspan": "{block.col_span}",
            "data-color": "{block.color}",
            "data-token-kind": "{block.token_kind.clone().unwrap_or_default()}",
            "data-raw-text": "{block.raw_text}",
            "data-label": "{block.label}",
            "data-node-type": "{block.node_types.join(\" \")}",
            if block.ref_markers.iter().any(|marker| marker.role == ReferenceMarkerRole::Referent) {
                span { class: "{incoming_class}",
                    for marker in block.ref_markers.iter().filter(|marker| marker.role == ReferenceMarkerRole::Referent) {
                        span { class: "ref-math ref-line",
                            { render_ref_marker(marker, reference_hover, reference_tooltip_open, &hover_state, &tooltip_open_state, export_script) }
                        }
                    }
                }
            }
            if let Some(card) = &block.tooltip {
                {
                    let base_path = router_base_path();
                    rsx! {
                        span { class: "block-label",
                            span { class: "block-label-tooltip dictionary-tooltip-host",
                                title: "{native_title}",
                                if let Some(route) = jbotci_route_from_href(&base_path, &card.href) {
                                    Link { class: "block-label-link", to: route,
                                        span { class: "block-label-text",
                                            { render_elidable_page_find_text(page_find, &block.label, block.is_elided) }
                                        }
                                    }
                                } else {
                                    a { class: "block-label-link", href: "{card.href}",
                                        span { class: "block-label-text",
                                            { render_elidable_page_find_text(page_find, &block.label, block.is_elided) }
                                        }
                                    }
                                }
                                { render_dictionary_tooltip(card, false, &base_path, export_script) }
                            }
                        }
                    }
                }
            } else {
                span { class: "block-label", title: "{native_title}",
                    span { class: "block-label-text",
                        { render_elidable_page_find_text(page_find, &block.label, block.is_elided) }
                    }
                }
            }
            if block.ref_markers.iter().any(|marker| marker.role == ReferenceMarkerRole::Reference) {
                span { class: "block-ref-source",
                    span { class: "ref-math",
                        for marker in block.ref_markers.iter().filter(|marker| marker.role == ReferenceMarkerRole::Reference) {
                            span { class: "ref-arrow", "→" }
                            { render_ref_marker(marker, reference_hover, reference_tooltip_open, &hover_state, &tooltip_open_state, export_script) }
                        }
                    }
                }
            }
            if let Some((export_layout, export_show_glosses, export_script)) = export_controls {
                {
                    let svg_layout = export_layout.clone();
                    let png_layout = export_layout.clone();
                    let svg_activity = activity;
                    let png_activity = activity;
                    let svg_export_task = export_task;
                    let png_export_task = export_task;
                    rsx! {
                span { class: "blocks-svg-link",
                    button {
                        class: "export-link",
                        r#type: "button",
                        onclick: move |_| {
                            let layout = svg_layout.clone();
                            cancel_compute_channel(COMPUTE_CHANNEL_EXPORT);
                            spawn_latest_tracked(svg_export_task, svg_activity, AsyncTaskKind::Export, async move {
                                download_gentufa_blocks_svg(layout, export_show_glosses, export_script).await;
                            });
                        },
                        "SVG"
                    }
                    button {
                        class: "export-link",
                        r#type: "button",
                        onclick: move |_| {
                            let layout = png_layout.clone();
                            cancel_compute_channel(COMPUTE_CHANNEL_EXPORT);
                            spawn_latest_tracked(png_export_task, png_activity, AsyncTaskKind::Export, async move {
                                download_gentufa_blocks_png(layout, export_show_glosses, export_script).await;
                            });
                        },
                        "PNG"
                    }
                }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(matches!(script, GentufaScript::Zbalermorna) -> ret.is_empty())]
fn block_native_title(block: &GentufaBlock, script: GentufaScript) -> &str {
    if matches!(script, GentufaScript::Zbalermorna) {
        ""
    } else {
        &block.label
    }
}

#[requires(true)]
#[ensures(true)]
fn render_ref_marker(
    marker: &ReferenceMarker,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    hover_state: &ReferenceHoverState,
    tooltip_open_state: &Option<HoveredReference>,
    script: GentufaScript,
) -> Element {
    let view = reference_marker_view_model(marker, hover_state).into_data();
    let class = view.class;
    let role = view.role_attr;
    let base = view.base_key;
    let label = view.full_key;
    let kind = view.kind;
    if let Some(tooltip) = &marker.tooltip {
        let host_class = reference_tooltip_host_class(marker, tooltip_open_state);
        let base_path = router_base_path();
        let enter_hover = reference_hover;
        let leave_hover = reference_hover;
        let leave_tooltip_open = reference_tooltip_open;
        let click_tooltip_open = reference_tooltip_open;
        let enter_role = marker.role;
        let enter_label = marker.label.clone();
        let click_role = marker.role;
        let click_label = marker.label.clone();
        rsx! {
            span {
                class: "{host_class}",
                onmouseenter: move |_| set_reference_hover(enter_hover, enter_role, enter_label.clone()),
                onmouseleave: move |_| {
                    clear_reference_hover(leave_hover);
                    clear_reference_tooltip_open(leave_tooltip_open);
                },
                onclick: move |_| set_reference_tooltip_open(click_tooltip_open, click_role, click_label.clone()),
                span {
                    class: "{class}",
                    "data-ref-role": "{role}",
                    "data-ref-kind": "{kind}",
                    "data-ref-label": "{label}",
                    "data-ref-base": "{base}",
                    { render_reference_label(&marker.label) }
                }
                { render_reference_tooltip(tooltip, &base_path, script) }
            }
        }
    } else {
        let enter_hover = reference_hover;
        let leave_hover = reference_hover;
        let leave_tooltip_open = reference_tooltip_open;
        let enter_role = marker.role;
        let enter_label = marker.label.clone();
        rsx! {
            span {
                class: "{class}",
                "data-ref-role": "{role}",
                "data-ref-kind": "{kind}",
                "data-ref-label": "{label}",
                "data-ref-base": "{base}",
                onmouseenter: move |_| set_reference_hover(enter_hover, enter_role, enter_label.clone()),
                onmouseleave: move |_| {
                    clear_reference_hover(leave_hover);
                    clear_reference_tooltip_open(leave_tooltip_open);
                },
                { render_reference_label(&marker.label) }
            }
        }
    }
}

#[invariant(!self.class.is_empty())]
#[invariant(!self.role_attr.is_empty())]
#[invariant(!self.kind.is_empty())]
#[invariant(!self.base_key.is_empty())]
#[invariant(!self.full_key.is_empty())]
#[invariant(self.native_title.is_none())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferenceMarkerViewModel {
    class: String,
    role_attr: &'static str,
    kind: String,
    base_key: String,
    full_key: String,
    has_tooltip: bool,
    native_title: Option<String>,
}

#[requires(true)]
#[ensures(ret.native_title.is_none())]
fn reference_marker_view_model(
    marker: &ReferenceMarker,
    hover_state: &ReferenceHoverState,
) -> ReferenceMarkerViewModel {
    new!(ReferenceMarkerViewModel {
        class: reference_marker_class(marker, hover_state),
        role_attr: reference_role_attr(marker.role),
        kind: marker.kind.clone(),
        base_key: marker.label.base_key(),
        full_key: marker.label.full_key(),
        has_tooltip: marker.tooltip.is_some(),
        native_title: None,
    })
}

#[requires(true)]
#[ensures(ret.contains("reference-tooltip-host"))]
fn reference_tooltip_host_class(
    marker: &ReferenceMarker,
    tooltip_open_state: &Option<HoveredReference>,
) -> String {
    class_names(
        "reference-tooltip-host",
        &[(
            "is-open",
            reference_tooltip_matches_marker(marker, tooltip_open_state),
        )],
    )
}

#[requires(true)]
#[ensures(true)]
async fn download_gentufa_blocks_svg(
    layout: GentufaBlocksLayout,
    show_glosses: bool,
    script: GentufaScript,
) {
    let _ = download_gentufa_blocks_svg_result(layout, show_glosses, script).await;
}

#[requires(true)]
#[ensures(true)]
async fn download_gentufa_blocks_png(
    layout: GentufaBlocksLayout,
    show_glosses: bool,
    script: GentufaScript,
) {
    let _ = download_gentufa_blocks_png_result(layout, show_glosses, script).await;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn download_gentufa_blocks_svg_result(
    layout: GentufaBlocksLayout,
    show_glosses: bool,
    script: GentufaScript,
) -> Result<(), String> {
    let response = compute_request(
        COMPUTE_CHANNEL_EXPORT,
        WebComputeRequest::GentufaBlocksSvg {
            layout,
            show_glosses,
            script,
        },
    )
    .await?;
    let WebComputeResponse::GentufaBlocksSvg { svg } = response else {
        return Err("compute worker returned the wrong SVG export response".to_owned());
    };
    download_browser_bytes(
        "jbotci-blocks.svg",
        "image/svg+xml;charset=utf-8",
        svg.as_bytes(),
    )
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn download_gentufa_blocks_svg_result(
    layout: GentufaBlocksLayout,
    show_glosses: bool,
    script: GentufaScript,
) -> Result<(), String> {
    let response = compute_request(
        COMPUTE_CHANNEL_EXPORT,
        WebComputeRequest::GentufaBlocksSvg {
            layout,
            show_glosses,
            script,
        },
    )
    .await?;
    let WebComputeResponse::GentufaBlocksSvg { svg } = response else {
        return Err("compute worker returned the wrong SVG export response".to_owned());
    };
    save_native_bytes("jbotci-blocks.svg", svg.as_bytes())
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
async fn download_gentufa_blocks_svg_result(
    _layout: GentufaBlocksLayout,
    _show_glosses: bool,
    _script: GentufaScript,
) -> Result<(), String> {
    Err("gentufa SVG export is not available for this platform yet".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn download_gentufa_blocks_png_result(
    layout: GentufaBlocksLayout,
    show_glosses: bool,
    script: GentufaScript,
) -> Result<(), String> {
    let response = compute_request(
        COMPUTE_CHANNEL_EXPORT,
        WebComputeRequest::GentufaBlocksPng {
            layout,
            show_glosses,
            script,
        },
    )
    .await?;
    let WebComputeResponse::GentufaBlocksPng { bytes } = response else {
        return Err("compute worker returned the wrong PNG export response".to_owned());
    };
    download_browser_bytes("jbotci-blocks.png", "image/png", &bytes)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn download_gentufa_blocks_png_result(
    layout: GentufaBlocksLayout,
    show_glosses: bool,
    script: GentufaScript,
) -> Result<(), String> {
    let response = compute_request(
        COMPUTE_CHANNEL_EXPORT,
        WebComputeRequest::GentufaBlocksPng {
            layout,
            show_glosses,
            script,
        },
    )
    .await?;
    let WebComputeResponse::GentufaBlocksPng { bytes } = response else {
        return Err("compute worker returned the wrong PNG export response".to_owned());
    };
    save_native_bytes("jbotci-blocks.png", &bytes)
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
async fn download_gentufa_blocks_png_result(
    _layout: GentufaBlocksLayout,
    _show_glosses: bool,
    _script: GentufaScript,
) -> Result<(), String> {
    Err("gentufa PNG export is not available for this platform yet".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[requires(!file_name.is_empty())]
#[requires(!mime_type.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn download_browser_bytes(file_name: &str, mime_type: &str, bytes: &[u8]) -> Result<(), String> {
    let Some(window) = web_sys::window() else {
        return Err("browser window is unavailable".to_owned());
    };
    let Some(document) = window.document() else {
        return Err("browser document is unavailable".to_owned());
    };
    let Some(body) = document.body() else {
        return Err("document body is unavailable".to_owned());
    };
    let parts = js_sys::Array::new();
    let bytes = js_sys::Uint8Array::from(bytes);
    parts.push(&bytes);
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime_type);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(parts.as_ref(), &options)
        .map_err(js_value_to_string)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js_value_to_string)?;
    let anchor = document
        .create_element("a")
        .map_err(js_value_to_string)?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "created anchor element had an unexpected DOM type".to_owned())?;
    anchor.set_href(&url);
    anchor.set_download(file_name);
    let anchor_html: &web_sys::HtmlElement = anchor.unchecked_ref();
    let _ = anchor_html.style().set_property("display", "none");
    body.append_child(anchor.unchecked_ref())
        .map_err(js_value_to_string)?;
    anchor_html.click();
    let _ = body.remove_child(anchor.unchecked_ref());
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!file_name.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn save_native_bytes(file_name: &str, bytes: &[u8]) -> Result<(), String> {
    let Some(path) = rfd::FileDialog::new().set_file_name(file_name).save_file() else {
        return Ok(());
    };
    std::fs::write(&path, bytes)
        .map_err(|error| format!("failed to write `{}`: {error}", path.display()))
}

#[requires(true)]
#[ensures(true)]
fn render_gloss_block(
    block: &GentufaBlock,
    gloss_row: usize,
    page_find: &PageFindContext,
) -> Element {
    let col = block.col + 1;
    let text = block
        .computed_gloss
        .as_deref()
        .or_else(|| block.glosses.first().map(String::as_str))
        .unwrap_or("");
    let style = format!(
        "grid-row: {gloss_row}; grid-column: {col} / span {};",
        block.col_span
    );
    rsx! {
        div {
            key: "gloss-{block.block_id}",
            class: "block block-gloss",
            style: "{style}",
            "data-block-id": "{block.block_id}",
            "data-col": "{block.col}",
            "data-colspan": "{block.col_span}",
            "data-color": "{block.color}",
            div { class: "gloss-list",
                { render_page_find_text(page_find, text) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_tree(
    success: &GentufaSuccess,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        div { class: "table-view",
            div { class: "table-wrap",
                svg { class: "tree-lines", "aria-hidden": "true" }
                table { class: "parse-table spa-gentufa-table",
                        tbody {
                            for row in success.tree_rows.iter() {
                            { render_tree_row(row, reference_hover, reference_tooltip_open, script, page_find) }
                            }
                        }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_tree_row(
    row: &GentufaTreeRow,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let row_class = class_names(
        "tree-row",
        &[
            ("elided-row", tree_row_is_elided(row)),
            ("tree-leaf", !row.has_children),
        ],
    );
    let parent_id = row
        .parent_id
        .map(|parent_id| parent_id.to_string())
        .unwrap_or_default();
    let indent_count = row.guides.len() + 1;
    let style = format!(
        "--row-color: {}; --block-color: {}; --indent-count: {};",
        row.color, row.color, indent_count
    );
    let hover_state = reference_hover.read().clone();
    let tooltip_open_state = reference_tooltip_open.read().clone();
    let incoming_markers = row
        .ref_markers
        .iter()
        .filter(|marker| marker.role == ReferenceMarkerRole::Referent)
        .collect::<Vec<_>>();
    let outgoing_markers = row
        .ref_markers
        .iter()
        .filter(|marker| marker.role == ReferenceMarkerRole::Reference)
        .collect::<Vec<_>>();
    let current_guide_class = class_names(
        "indent-block tree-current-guide",
        &[
            ("has-parent", !row.guides.is_empty()),
            ("line-bottom", row.has_children),
        ],
    );
    rsx! {
        tr {
            class: "{row_class}",
            style: "{style}",
            "data-node-id": "{row.node_id}",
            "data-parent-id": "{parent_id}",
            "data-depth": "{row.depth}",
            "data-color": "{row.color}",
            td { class: "col-node",
                span { class: "indent-stack",
                    for guide in row.guides.iter() {
                        { render_tree_guide(guide) }
                    }
                    span { class: "{current_guide_class}", style: "--block-color: {row.color};" }
                }
                div { class: "node-cell",
                    span { class: "node-content",
                        span { class: "node-label", style: "--block-color: {row.color};",
                            { render_page_find_text(page_find, &row.label) }
                        }
                    }
                }
            }
            { render_tree_edge_cell(incoming_markers, reference_hover, reference_tooltip_open, &hover_state, &tooltip_open_state, script) }
            td { class: "col-text",
                div { class: "cell-pad tree-text-cell",
                    for cell in row.cells.iter() {
                        { render_tree_cell(cell, page_find) }
                    }
                    { render_tree_outgoing_edges(outgoing_markers, reference_hover, reference_tooltip_open, &hover_state, &tooltip_open_state, script) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_tree_guide(guide: &GentufaTreeGuide) -> Element {
    let class = class_names(
        "indent-block tree-guide",
        &[
            ("line-top", guide.line_top),
            ("line-bottom", guide.line_bottom),
        ],
    );
    rsx! {
        span { class: "{class}", style: "--block-color: {guide.color};" }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_tree_edge_cell(
    markers: Vec<&ReferenceMarker>,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    hover_state: &ReferenceHoverState,
    tooltip_open_state: &Option<HoveredReference>,
    script: GentufaScript,
) -> Element {
    let has_markers = !markers.is_empty();
    rsx! {
        td { class: "col-edge col-edge-in",
            div { class: "cell-pad edge-cell",
                for marker in markers {
                    { render_ref_marker(marker, reference_hover, reference_tooltip_open, hover_state, tooltip_open_state, script) }
                }
                if has_markers {
                    span { class: "ref-arrow edge-arrow", "→" }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_tree_outgoing_edges(
    markers: Vec<&ReferenceMarker>,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    hover_state: &ReferenceHoverState,
    tooltip_open_state: &Option<HoveredReference>,
    script: GentufaScript,
) -> Element {
    let has_markers = !markers.is_empty();
    rsx! {
        if has_markers {
            span { class: "tree-outgoing-edge edge-cell",
                span { class: "ref-arrow edge-arrow", "→" }
                for marker in markers {
                    { render_ref_marker(marker, reference_hover, reference_tooltip_open, hover_state, tooltip_open_state, script) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_tree_cell(cell: &GentufaCell, page_find: &PageFindContext) -> Element {
    let class = if cell.is_elided {
        "token is-elided"
    } else {
        "token"
    };
    rsx! {
        span { class: "{class}",
            span { class: "token-raw lojban-text",
                { render_elidable_page_find_text(page_find, &cell.text, cell.is_elided) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_elidable_text(text: &str, elided: bool) -> Element {
    if elided {
        rsx! { s { "{text}" } }
    } else {
        rsx! { "{text}" }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_elidable_page_find_text(
    page_find: &PageFindContext,
    text: &str,
    elided: bool,
) -> Element {
    if elided {
        rsx! { s { { render_page_find_text(page_find, text) } } }
    } else {
        render_page_find_text(page_find, text)
    }
}

#[requires(true)]
#[ensures(true)]
fn render_reference_label(label: &ReferenceLabel) -> Element {
    let slot_text = label.slot.as_ref().map(reference_slot_display_text);
    let stem = math_alphanumeric_stem(&label.stem);
    rsx! {
        span { class: "spa-cll-math",
            math { class: "math-var", display: "inline",
                mrow {
                    if let Some(occurrence) = label.occurrence {
                        msub {
                            mi { "{stem}" }
                            mtext { "{occurrence}" }
                        }
                    } else {
                        mi { "{stem}" }
                    }
                    if let Some(text) = slot_text.as_deref() {
                        mtext { "⟨{text}⟩" }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(ret.chars().count() >= stem.chars().count())]
fn math_alphanumeric_stem(stem: &str) -> String {
    let mut output = String::new();
    for ch in stem.chars() {
        push_math_alphanumeric_char(&mut output, ch);
    }
    output
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
fn render_reference_overlay(state: &ReferenceHoverState) -> Element {
    let Some(overlay) = state.overlay.as_ref() else {
        return rsx! {};
    };
    let view_box = format!(
        "0 0 {:.2} {:.2}",
        overlay.width.max(1.0),
        overlay.height.max(1.0)
    );
    rsx! {
        svg {
            class: "arrow-overlay",
            "viewBox": "{view_box}",
            "aria-hidden": "true",
            defs {
                marker {
                    id: "jbotci-ref-arrowhead",
                    "markerWidth": "7",
                    "markerHeight": "7",
                    "refX": "6",
                    "refY": "3.5",
                    orient: "auto",
                    "markerUnits": "strokeWidth",
                    path { class: "arrow-head", d: "M 0 0 L 7 3.5 L 0 7 z" }
                }
            }
            for path_data in overlay.paths.iter() {
                path {
                    class: "arrow-path",
                    d: "{path_data}",
                    "marker-end": "url(#jbotci-ref-arrowhead)"
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn set_reference_hover(
    mut reference_hover: Signal<ReferenceHoverState>,
    role: ReferenceMarkerRole,
    label: ReferenceLabel,
) {
    let hovered = HoveredReference { role, label };
    let current = reference_hover.read().clone();
    let measured_overlay = measure_reference_overlay(&hovered);
    let overlay = reference_overlay_for_measurement_request(
        &current,
        &hovered,
        &measured_overlay,
        reference_overlay_measurement_is_async(),
    );
    let measurement_id = next_reference_hover_measurement_id(&current);
    reference_hover.set(ReferenceHoverState {
        hovered: Some(hovered.clone()),
        overlay,
        measurement_id,
    });
    schedule_reference_overlay_measure(reference_hover, hovered, measurement_id);
}

#[requires(true)]
#[ensures(true)]
fn clear_reference_hover(mut reference_hover: Signal<ReferenceHoverState>) {
    let measurement_id = next_reference_hover_measurement_id(&reference_hover.read());
    reference_hover.set(ReferenceHoverState {
        hovered: None,
        overlay: None,
        measurement_id,
    });
}

#[requires(true)]
#[ensures(true)]
fn set_reference_tooltip_open(
    mut reference_tooltip_open: Signal<Option<HoveredReference>>,
    role: ReferenceMarkerRole,
    label: ReferenceLabel,
) {
    reference_tooltip_open.set(Some(HoveredReference { role, label }));
}

#[requires(true)]
#[ensures(true)]
fn clear_reference_tooltip_open(mut reference_tooltip_open: Signal<Option<HoveredReference>>) {
    reference_tooltip_open.set(None);
}

#[requires(true)]
#[ensures(true)]
fn refresh_reference_hover(
    mut reference_hover: Signal<ReferenceHoverState>,
    reason: ReferenceHoverRefreshReason,
) {
    let async_measurement = reference_overlay_measurement_is_async();
    if !reference_hover_refresh_requires_measurement(reason, async_measurement) {
        return;
    }
    let current = reference_hover.read().clone();
    let Some(hovered) = current.hovered.clone() else {
        return;
    };
    let measured_overlay = measure_reference_overlay(&hovered);
    let overlay = reference_overlay_for_measurement_request(
        &current,
        &hovered,
        &measured_overlay,
        async_measurement,
    );
    let measurement_id = next_reference_hover_measurement_id(&current);
    reference_hover.set(ReferenceHoverState {
        hovered: Some(hovered.clone()),
        overlay,
        measurement_id,
    });
    schedule_reference_overlay_measure(reference_hover, hovered, measurement_id);
}

#[requires(true)]
#[ensures(ret >= state.measurement_id)]
fn next_reference_hover_measurement_id(state: &ReferenceHoverState) -> u64 {
    state.measurement_id.saturating_add(1)
}

#[requires(true)]
#[ensures(!async_measurement || !matches!(reason, ReferenceHoverRefreshReason::PointerMove) || !ret)]
fn reference_hover_refresh_requires_measurement(
    reason: ReferenceHoverRefreshReason,
    async_measurement: bool,
) -> bool {
    !(async_measurement && matches!(reason, ReferenceHoverRefreshReason::PointerMove))
}

#[requires(true)]
#[ensures(measured_overlay.is_some() -> ret.as_ref() == measured_overlay.as_ref())]
#[ensures(!async_measurement && measured_overlay.is_none() -> ret.is_none())]
fn reference_overlay_for_measurement_request(
    current: &ReferenceHoverState,
    hovered: &HoveredReference,
    measured_overlay: &Option<ArrowOverlay>,
    async_measurement: bool,
) -> Option<ArrowOverlay> {
    if let Some(overlay) = measured_overlay {
        return Some(overlay.clone());
    }
    if async_measurement && current.hovered.as_ref() == Some(hovered) {
        current.overlay.clone()
    } else {
        None
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret)]
fn reference_overlay_measurement_is_async() -> bool {
    true
}

#[cfg(any(
    target_arch = "wasm32",
    all(not(target_arch = "wasm32"), not(feature = "desktop"))
))]
#[requires(true)]
#[ensures(!ret)]
fn reference_overlay_measurement_is_async() -> bool {
    false
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn schedule_reference_overlay_measure(
    mut reference_hover: Signal<ReferenceHoverState>,
    hovered: HoveredReference,
    measurement_id: u64,
) {
    spawn(async move {
        let overlay = measure_reference_overlay_desktop(&hovered).await;
        reference_hover.with_mut(|state| {
            if state.measurement_id == measurement_id && state.hovered.as_ref() == Some(&hovered) {
                state.overlay = overlay;
            }
        });
    });
}

#[cfg(any(
    target_arch = "wasm32",
    all(not(target_arch = "wasm32"), not(feature = "desktop"))
))]
#[requires(true)]
#[ensures(true)]
fn schedule_reference_overlay_measure(
    _reference_hover: Signal<ReferenceHoverState>,
    _hovered: HoveredReference,
    _measurement_id: u64,
) {
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn reference_marker_class(marker: &ReferenceMarker, state: &ReferenceHoverState) -> String {
    let mut class = match marker.role {
        ReferenceMarkerRole::Reference => "ref-var ref-source".to_owned(),
        ReferenceMarkerRole::Referent => "ref-var ref-target".to_owned(),
    };
    if marker.label.slot.is_some() {
        class.push_str(" place-var");
    }
    if reference_matches_hover(marker, state) {
        class.push_str(" ref-highlight");
        if marker.label.slot.is_some() {
            class.push_str(" place-highlight");
        }
    }
    class
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn reference_role_attr(role: ReferenceMarkerRole) -> &'static str {
    match role {
        ReferenceMarkerRole::Reference => "reference",
        ReferenceMarkerRole::Referent => "referent",
    }
}

#[requires(true)]
#[ensures(true)]
fn reference_matches_hover(marker: &ReferenceMarker, state: &ReferenceHoverState) -> bool {
    let Some(hovered) = state.hovered.as_ref() else {
        return false;
    };
    if marker.label.base_key() != hovered.label.base_key() {
        return false;
    }
    match hovered.role {
        ReferenceMarkerRole::Reference => true,
        ReferenceMarkerRole::Referent => match marker.role {
            ReferenceMarkerRole::Reference => true,
            ReferenceMarkerRole::Referent => marker.label.full_key() == hovered.label.full_key(),
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn reference_tooltip_matches_marker(
    marker: &ReferenceMarker,
    opened: &Option<HoveredReference>,
) -> bool {
    opened
        .as_ref()
        .is_some_and(|opened| marker.role == opened.role && marker.label == opened.label)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn measure_reference_overlay(hovered: &HoveredReference) -> Option<ArrowOverlay> {
    let base_key = hovered.label.base_key();
    let full_key = hovered.label.full_key();
    let window = web_sys::window()?;
    let document = window.document()?;
    let nodes = document
        .query_selector_all(".parse-page .ref-var[data-ref-role]")
        .ok()?;
    let mut sources = Vec::new();
    let mut targets = Vec::new();
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if element.get_attribute("data-ref-base").as_deref() != Some(base_key.as_str()) {
            continue;
        }
        let role = element.get_attribute("data-ref-role");
        let label = element.get_attribute("data-ref-label");
        if role.as_deref() == Some("reference") {
            sources.push(reference_rect_from_element(&element));
        } else if role.as_deref() == Some("referent")
            && (hovered.role == ReferenceMarkerRole::Reference
                || label.as_deref() == Some(full_key.as_str()))
        {
            targets.push(reference_rect_from_element(&element));
        }
    }
    let mut paths = reference_arrow_paths(&sources, &targets);
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return None;
    }
    Some(ArrowOverlay {
        width: window
            .inner_width()
            .ok()
            .and_then(|width| width.as_f64())
            .unwrap_or(1.0),
        height: window
            .inner_height()
            .ok()
            .and_then(|height| height.as_f64())
            .unwrap_or(1.0),
        paths,
    })
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.is_none())]
fn measure_reference_overlay(_hovered: &HoveredReference) -> Option<ArrowOverlay> {
    None
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
struct DesktopReferenceOverlayMetrics {
    width: f64,
    height: f64,
    markers: Vec<DesktopReferenceMarkerMetrics>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
struct DesktopReferenceMarkerMetrics {
    role: String,
    base: String,
    label: String,
    rect: ReferenceRect,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn measure_reference_overlay_desktop(hovered: &HoveredReference) -> Option<ArrowOverlay> {
    let metrics: DesktopReferenceOverlayMetrics = document::eval(
        r#"
        const rectFor = (element) => {
            const rect = element.getBoundingClientRect();
            return {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            };
        };
        return {
            width: Number(window.innerWidth || 1),
            height: Number(window.innerHeight || 1),
            markers: Array.from(document.querySelectorAll(".parse-page .ref-var[data-ref-role]")).map((element) => ({
                role: element.getAttribute("data-ref-role") || "",
                base: element.getAttribute("data-ref-base") || "",
                label: element.getAttribute("data-ref-label") || "",
                rect: rectFor(element),
            })),
        };
        "#,
    )
    .join()
    .await
    .ok()?;
    reference_overlay_from_marker_metrics(hovered, metrics)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn reference_overlay_from_marker_metrics(
    hovered: &HoveredReference,
    metrics: DesktopReferenceOverlayMetrics,
) -> Option<ArrowOverlay> {
    let base_key = hovered.label.base_key();
    let full_key = hovered.label.full_key();
    let mut sources = Vec::new();
    let mut targets = Vec::new();
    for marker in metrics.markers {
        if marker.base != base_key {
            continue;
        }
        if marker.role == "reference" {
            sources.push(marker.rect);
        } else if marker.role == "referent"
            && (hovered.role == ReferenceMarkerRole::Reference || marker.label == full_key)
        {
            targets.push(marker.rect);
        }
    }
    let mut paths = reference_arrow_paths(&sources, &targets);
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        None
    } else {
        Some(ArrowOverlay {
            width: metrics.width.max(1.0),
            height: metrics.height.max(1.0),
            paths,
        })
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn reference_rect_from_element(element: &web_sys::Element) -> ReferenceRect {
    let rect = element.get_bounding_client_rect();
    ReferenceRect {
        left: rect.left(),
        top: rect.top(),
        right: rect.right(),
        bottom: rect.bottom(),
    }
}

#[requires(true)]
#[ensures(true)]
fn reference_arrow_paths(sources: &[ReferenceRect], targets: &[ReferenceRect]) -> Vec<String> {
    let mut paths = Vec::new();
    for source in sources {
        for target in targets {
            paths.push(reference_arrow_path(*source, *target));
        }
    }
    paths
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn reference_arrow_path(source: ReferenceRect, target: ReferenceRect) -> String {
    let (sx, sy) = rect_anchor_toward(source, target);
    let (tx, ty) = rect_anchor_toward(target, source);
    let dx = tx - sx;
    let dy = ty - sy;
    let distance = (dx * dx + dy * dy).sqrt();
    if distance <= f64::EPSILON {
        return format!("M {sx:.2} {sy:.2} L {tx:.2} {ty:.2}");
    }
    let curvature = (distance * 0.3).min(80.0);
    let normal_x = -dy / distance;
    let normal_y = dx / distance;
    let cx = (sx + tx) / 2.0 + normal_x * curvature;
    let cy = (sy + ty) / 2.0 + normal_y * curvature;
    format!("M {sx:.2} {sy:.2} Q {cx:.2} {cy:.2} {tx:.2} {ty:.2}")
}

#[requires(true)]
#[ensures(true)]
fn rect_anchor_toward(from: ReferenceRect, to: ReferenceRect) -> (f64, f64) {
    let from_center_x = (from.left + from.right) / 2.0;
    let from_center_y = (from.top + from.bottom) / 2.0;
    let to_center_x = (to.left + to.right) / 2.0;
    let to_center_y = (to.top + to.bottom) / 2.0;
    let dx = to_center_x - from_center_x;
    let dy = to_center_y - from_center_y;
    if dx.abs() >= dy.abs() {
        let x = if dx >= 0.0 { from.right } else { from.left };
        (x, from_center_y)
    } else {
        let y = if dy >= 0.0 { from.bottom } else { from.top };
        (from_center_x, y)
    }
}

#[requires(true)]
#[ensures(true)]
fn render_settings(
    settings: Signal<UserSettings>,
    current_settings: UserSettings,
    dialect_settings: Signal<DialectSettings>,
    current_dialect_settings: DialectSettings,
    selected_dialect: Signal<String>,
    qr_uri: Signal<Option<String>>,
    embedding_settings: Signal<EmbeddingSettingsState>,
    activity: Signal<AsyncActivityState>,
    page_find: &PageFindContext,
) -> Element {
    let embedding_state = embedding_settings.read().clone();
    rsx! {
        section { class: "spa-page settings-page",
            div { class: "page-container settings-container",
                div { class: "settings-page-header",
                    h1 { { render_page_find_text(page_find, "Settings") } }
                    { render_settings_commit_link(page_find) }
                }
                { render_embedding_settings(embedding_settings, &embedding_state, activity, page_find) }
                { render_output_settings(settings, current_settings, page_find) }
                { render_dialect_settings_section(dialect_settings, current_dialect_settings, selected_dialect, qr_uri, page_find) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_output_settings(
    settings: Signal<UserSettings>,
    current: UserSettings,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        section { class: "settings-section settings-output",
            div { class: "settings-section-head",
                h2 { { render_page_find_text(page_find, "Output") } }
            }
            div { class: "settings-output-grid",
                div { class: "settings-output-selector",
                    p { class: "settings-output-label",
                        { render_page_find_text(page_find, "Stress") }
                    }
                    div {
                        class: "settings-output-toggle-group",
                        role: "group",
                        aria_label: "Stress mark rendering",
                        { render_stress_mark_button(settings, current.stress, StressMark::None, "none", page_find) }
                        { render_stress_mark_button(settings, current.stress, StressMark::Acute, "acute", page_find) }
                        { render_stress_mark_button(settings, current.stress, StressMark::Caps, "caps", page_find) }
                    }
                }
                div { class: "settings-output-selector",
                    p { class: "settings-output-label",
                        { render_page_find_text(page_find, "Glides") }
                    }
                    div {
                        class: "settings-output-toggle-group",
                        role: "group",
                        aria_label: "Glide mark rendering",
                        { render_glide_mark_button(settings, current.glides, GlideMark::None, "none", page_find) }
                        { render_glide_mark_button(settings, current.glides, GlideMark::Breve, "breve", page_find) }
                    }
                }
            }
        }
    }
}

#[requires(!label.is_empty())]
#[ensures(true)]
fn render_stress_mark_button(
    mut settings: Signal<UserSettings>,
    current: StressMark,
    mark: StressMark,
    label: &'static str,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        button {
            class: settings_output_toggle_class(current == mark),
            r#type: "button",
            aria_pressed: pressed_attr(current == mark),
            onclick: move |_| set_stress_mark(&mut settings, mark),
            { render_page_find_text(page_find, label) }
        }
    }
}

#[requires(!label.is_empty())]
#[ensures(true)]
fn render_glide_mark_button(
    mut settings: Signal<UserSettings>,
    current: GlideMark,
    mark: GlideMark,
    label: &'static str,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        button {
            class: settings_output_toggle_class(current == mark),
            r#type: "button",
            aria_pressed: pressed_attr(current == mark),
            onclick: move |_| set_glide_mark(&mut settings, mark),
            { render_page_find_text(page_find, label) }
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn settings_output_toggle_class(active: bool) -> &'static str {
    if active {
        "settings-output-toggle active"
    } else {
        "settings-output-toggle"
    }
}

#[requires(true)]
#[ensures(true)]
fn render_dialect_settings_section(
    dialect_settings: Signal<DialectSettings>,
    current: DialectSettings,
    mut selected_dialect: Signal<String>,
    qr_uri: Signal<Option<String>>,
    page_find: &PageFindContext,
) -> Element {
    let selected_name = selected_dialect_name(&current, &selected_dialect.read());
    let builtin_names = builtin_dialect_names()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let custom_dialects = current.custom_dialects.clone();
    let selected_custom = custom_dialects
        .iter()
        .find(|custom| custom.name.trim() == selected_name)
        .cloned();
    let selected_is_builtin = find_builtin_dialect(&selected_name).is_some();
    let selected_definition = selected_dialect_definition_text(&current, &selected_name);
    let selected_johau_uri = johau_uri_for_selected_dialect(&current, &selected_name);
    let selected_validation = selected_custom
        .as_ref()
        .and_then(|custom| custom_dialect_is_valid(&current.custom_dialects, custom).err())
        .map(|error| error.message().to_owned());
    rsx! {
        section { class: "settings-section settings-dialects",
            div { class: "settings-section-head",
                h2 { { render_page_find_text(page_find, "Lojban dialects") } }
            }
            div { class: "settings-dialect-grid",
                nav { class: "settings-dialect-list", aria_label: "Dialects",
                    div { class: "settings-dialect-list-group",
                        p { class: "settings-dialect-list-heading",
                            { render_page_find_text(page_find, "Builtins") }
                        }
                        for name in builtin_names.iter() {
                            {
                                let item_name = name.clone();
                                let selected = item_name == selected_name;
                                let class_name = class_names("settings-dialect-list-item", &[("is-selected", selected)]);
                                rsx! {
                                    button {
                                        class: "{class_name}",
                                        r#type: "button",
                                        aria_pressed: pressed_attr(selected),
                                        onclick: move |_| selected_dialect.set(item_name.clone()),
                                        { render_page_find_text(page_find, name) }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "settings-dialect-list-group",
                        p { class: "settings-dialect-list-heading",
                            { render_page_find_text(page_find, "Custom") }
                        }
                        for custom in custom_dialects.iter() {
                            {
                                let item_name = custom.name.trim().to_owned();
                                let label = if item_name.is_empty() { "(unnamed)".to_owned() } else { item_name.clone() };
                                let selected = item_name == selected_name;
                                let class_name = class_names("settings-dialect-list-item", &[("is-selected", selected), ("is-invalid", custom_dialect_is_valid(&current.custom_dialects, custom).is_err())]);
                                rsx! {
                                    button {
                                        class: "{class_name}",
                                        r#type: "button",
                                        aria_pressed: pressed_attr(selected),
                                        onclick: move |_| selected_dialect.set(item_name.clone()),
                                        { render_page_find_text(page_find, &label) }
                                    }
                                }
                            }
                        }
                        button {
                            class: "settings-dialect-add",
                            r#type: "button",
                            aria_label: "Add custom dialect",
                            title: "Add custom dialect",
                            onclick: move |_| add_custom_dialect(dialect_settings, selected_dialect),
                            span { class: "settings-dialect-add-icon", "⨁" }
                        }
                    }
                }
                div { class: "settings-dialect-editor",
                    if selected_is_builtin {
                        { render_builtin_dialect_editor(dialect_settings, &current, &selected_name, selected_definition.as_deref(), selected_johau_uri.as_deref(), qr_uri, page_find) }
                    } else if let Some(custom) = selected_custom {
                        { render_custom_dialect_editor(dialect_settings, selected_dialect, &custom, selected_validation.as_deref(), selected_johau_uri.as_deref(), qr_uri, page_find) }
                    } else {
                        p { class: "settings-help-text",
                            { render_page_find_text(page_find, "Select a dialect to edit it.") }
                        }
                    }
                }
            }
            { render_dialect_qr_popout(qr_uri) }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_builtin_dialect_editor(
    dialect_settings: Signal<DialectSettings>,
    current: &DialectSettings,
    name: &str,
    definition: Option<&str>,
    johau_uri: Option<&str>,
    qr_uri: Signal<Option<String>>,
    page_find: &PageFindContext,
) -> Element {
    let show_in_gentufa = builtin_dialect_shows_in_gentufa(current, name);
    let definition = definition.unwrap_or_default();
    let johau_uri = johau_uri.map(str::to_owned);
    let name_for_toggle = name.to_owned();
    let gentufa_toggle_disabled = !dialect_name_shows_in_gentufa_picker(name);
    let gentufa_toggle_class =
        settings_dialect_gentufa_toggle_class(show_in_gentufa, gentufa_toggle_disabled);
    rsx! {
        div { class: "settings-dialect-form settings-dialect-readonly",
            div { class: "settings-dialect-name-row",
                div { class: "settings-dialect-name-stack",
                    label { class: "settings-field settings-dialect-name-field",
                        span { class: "settings-field-label",
                            { render_page_find_text(page_find, "Name") }
                        }
                        input {
                            class: "settings-text-input settings-dialect-name",
                            value: "{name}",
                            readonly: true,
                            spellcheck: "false",
                            title: "Builtin dialect names cannot be edited.",
                            aria_label: "Dialect name",
                        }
                    }
                    label {
                        class: "{gentufa_toggle_class}",
                        title: settings_dialect_gentufa_toggle_title(name),
                        input {
                            r#type: "checkbox",
                            checked: show_in_gentufa && !gentufa_toggle_disabled,
                        disabled: gentufa_toggle_disabled,
                        onchange: move |_| toggle_builtin_dialect_gentufa_visibility(dialect_settings, &name_for_toggle, show_in_gentufa),
                    }
                        span { { render_page_find_text(page_find, "Show in gentufa") } }
                    }
                }
                div { class: "settings-dialect-name-actions",
                    { render_dialect_qr_button(johau_uri, qr_uri) }
                }
            }
            label { class: "settings-field settings-dialect-definition-field",
                span { class: "settings-field-label",
                    { render_page_find_text(page_find, "Definition") }
                }
                div { class: "settings-dialect-definition-wrap is-readonly",
                    pre { class: "settings-dialect-definition-highlight", aria_hidden: "true",
                        { render_dialect_highlight(definition) }
                    }
                    textarea {
                        class: "settings-text-input settings-dialect-definition",
                        value: "{definition}",
                        readonly: true,
                        spellcheck: "false",
                        title: "Builtin dialect definitions cannot be edited.",
                        aria_label: "Builtin dialect definition",
                    }
                }
            }
            p { class: "settings-dialect-validation is-ok",
                { render_page_find_text(page_find, "Definition is valid.") }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_custom_dialect_editor(
    dialect_settings: Signal<DialectSettings>,
    selected_dialect: Signal<String>,
    custom: &CustomDialect,
    validation: Option<&str>,
    johau_uri: Option<&str>,
    qr_uri: Signal<Option<String>>,
    page_find: &PageFindContext,
) -> Element {
    let previous_name = custom.name.trim().to_owned();
    let name_for_rename = previous_name.clone();
    let name_for_delete = previous_name.clone();
    let name_for_show = previous_name.clone();
    let name_for_definition = previous_name.clone();
    let custom_name = custom.name.clone();
    let custom_definition = custom.definition.clone();
    let show_in_gentufa = custom.show_in_gentufa;
    let johau_uri = johau_uri.map(str::to_owned);
    let gentufa_toggle_disabled = !dialect_name_shows_in_gentufa_picker(&custom_name);
    let gentufa_toggle_class =
        settings_dialect_gentufa_toggle_class(show_in_gentufa, gentufa_toggle_disabled);
    rsx! {
        div { class: "settings-dialect-form",
            div { class: "settings-dialect-name-row",
                div { class: "settings-dialect-name-stack",
                    label { class: "settings-field settings-dialect-name-field",
                        span { class: "settings-field-label",
                            { render_page_find_text(page_find, "Name") }
                        }
                        input {
                            class: "settings-text-input settings-dialect-name",
                            value: "{custom_name}",
                            spellcheck: "false",
                            aria_label: "Dialect name",
                            oninput: move |event| rename_custom_dialect(dialect_settings, selected_dialect, &name_for_rename, &event.value()),
                        }
                    }
                    label {
                        class: "{gentufa_toggle_class}",
                        title: settings_dialect_gentufa_toggle_title(&custom_name),
                        input {
                            r#type: "checkbox",
                            checked: show_in_gentufa && !gentufa_toggle_disabled,
                        disabled: gentufa_toggle_disabled,
                        onchange: move |_| toggle_custom_dialect_gentufa_visibility(dialect_settings, &name_for_show),
                    }
                        span { { render_page_find_text(page_find, "Show in gentufa") } }
                    }
                }
                div { class: "settings-dialect-name-actions",
                    button {
                        class: "settings-dialect-icon-button settings-dialect-delete",
                        r#type: "button",
                        aria_label: "Delete custom dialect",
                        title: "Delete custom dialect",
                        onclick: move |_| delete_custom_dialect(dialect_settings, selected_dialect, &name_for_delete),
                        { render_delete_icon() }
                    }
                    { render_dialect_qr_button(johau_uri, qr_uri) }
                }
            }
            label { class: "settings-field settings-dialect-definition-field",
                span { class: "settings-field-label",
                    { render_page_find_text(page_find, "Definition") }
                }
                div { class: "settings-dialect-definition-wrap",
                    pre { class: "settings-dialect-definition-highlight", aria_hidden: "true",
                        { render_dialect_highlight(&custom_definition) }
                    }
                    textarea {
                        class: "settings-text-input settings-dialect-definition",
                        value: "{custom_definition}",
                        spellcheck: "false",
                        aria_label: "Dialect definition",
                        oninput: move |event| update_custom_dialect_definition(dialect_settings, &name_for_definition, &event.value()),
                    }
                }
            }
            if let Some(message) = validation {
                p { class: "settings-dialect-validation is-error",
                    { render_page_find_text(page_find, message) }
                }
            } else {
                p { class: "settings-dialect-validation is-ok",
                    { render_page_find_text(page_find, "Definition is valid.") }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_dialect_qr_button(
    johau_uri: Option<String>,
    mut qr_uri: Signal<Option<String>>,
) -> Element {
    if let Some(uri) = johau_uri {
        rsx! {
            button {
                class: "settings-dialect-icon-button settings-dialect-qr-button",
                r#type: "button",
                aria_label: "Show dialect QR code",
                title: "Show dialect QR code",
                onclick: move |_| qr_uri.set(Some(uri.clone())),
                { render_dialect_qr_icon() }
            }
        }
    } else {
        rsx! {
            button {
                class: "settings-dialect-icon-button settings-dialect-qr-button",
                r#type: "button",
                aria_label: "Dialect QR code unavailable",
                title: "QR export is available for valid non-baseline dialect definitions.",
                disabled: true,
                { render_dialect_qr_icon() }
            }
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn settings_dialect_gentufa_toggle_class(checked: bool, disabled: bool) -> String {
    class_names(
        "settings-toggle settings-dialect-gentufa-toggle",
        &[
            ("is-selected", checked && !disabled),
            ("is-disabled", disabled),
        ],
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn settings_dialect_gentufa_toggle_title(dialect_name: &str) -> &'static str {
    if dialect_name_shows_in_gentufa_picker(dialect_name) {
        "Show this dialect as a checkbox in the Gentufa dialect picker."
    } else {
        "Slash-named dialects can be typed in formulas, but they do not appear as Gentufa checkbox options."
    }
}

#[requires(true)]
#[ensures(true)]
fn render_delete_icon() -> Element {
    rsx! {
        svg {
            class: "settings-dialect-button-icon",
            "viewBox": "0 0 24 24",
            "aria-hidden": "true",
            path {
                d: "M9 3h6l1 2h4v2H4V5h4zM6 9h12l-1 12H7zM10 11v8h2v-8zM14 11v8h2v-8z",
                fill: "currentColor",
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_dialect_qr_icon() -> Element {
    rsx! {
        svg {
            class: "settings-dialect-button-icon settings-dialect-qr-icon",
            "viewBox": "0 0 24 24",
            "aria-hidden": "true",
            path {
                d: "M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h2v2h-2zM18 14h2v2h-2zM14 18h2v2h-2zM18 18h2v2h-2z",
                fill: "currentColor",
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_dialect_qr_popout(mut qr_uri: Signal<Option<String>>) -> Element {
    let current_uri = qr_uri.read().clone();
    let Some(uri) = current_uri else {
        return rsx! {};
    };
    let qr_svg = encode_qr_alphanumeric_h(&uri)
        .map(|qr| qr_code_svg(&qr))
        .unwrap_or_default();
    rsx! {
        div { class: "settings-dialect-qr-popout", role: "dialog", aria_label: "Dialect QR code",
            div { class: "settings-dialect-qr-card",
                div { class: "settings-dialect-qr-head",
                    button {
                        class: "settings-icon-button",
                        r#type: "button",
                        aria_label: "Close",
                        onclick: move |_| qr_uri.set(None),
                        "×"
                    }
                }
                a {
                    class: "settings-dialect-qr-link",
                    href: "{uri}",
                    title: "{uri}",
                    div { class: "settings-dialect-qr-svg", dangerous_inner_html: "{qr_svg}" }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn initial_dialect_settings_selection(settings: &DialectSettings) -> String {
    settings
        .custom_dialects
        .first()
        .map(|custom| custom.name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .or_else(|| {
            builtin_dialect_names()
                .first()
                .map(|name| (*name).to_owned())
        })
        .unwrap_or_default()
}

#[requires(true)]
#[ensures(true)]
fn selected_dialect_name(settings: &DialectSettings, requested: &str) -> String {
    let requested = requested.trim();
    if !requested.is_empty()
        && (find_builtin_dialect(requested).is_some()
            || settings
                .custom_dialects
                .iter()
                .any(|custom| custom.name.trim() == requested))
    {
        return requested.to_owned();
    }
    initial_dialect_settings_selection(settings)
}

#[requires(true)]
#[ensures(true)]
fn selected_dialect_definition_text(settings: &DialectSettings, name: &str) -> Option<String> {
    if let Some(builtin) = find_builtin_dialect(name) {
        return Some(builtin.definition.to_owned());
    }
    settings
        .custom_dialects
        .iter()
        .find(|custom| custom.name.trim() == name)
        .map(|custom| custom.definition.clone())
}

#[requires(true)]
#[ensures(true)]
fn johau_uri_for_selected_dialect(settings: &DialectSettings, name: &str) -> Option<String> {
    let definition = selected_dialect_definition_text(settings, name)?;
    custom_dialect_definition_to_johau_uri_with_custom_dialects(
        &settings.custom_dialects,
        &definition,
    )
    .ok()
}

#[requires(true)]
#[ensures(true)]
fn builtin_dialect_shows_in_gentufa(settings: &DialectSettings, name: &str) -> bool {
    dialect_name_shows_in_gentufa_picker(name)
        && !settings.hidden_builtin_gentufa_dialects.contains(name)
}

#[requires(true)]
#[ensures(true)]
fn set_dialect_settings(mut dialect_settings: Signal<DialectSettings>, next: DialectSettings) {
    save_dialect_settings(&next);
    dialect_settings.set(next);
}

#[requires(true)]
#[ensures(true)]
fn add_custom_dialect(
    dialect_settings: Signal<DialectSettings>,
    mut selected_dialect: Signal<String>,
) {
    let mut next = dialect_settings.read().clone();
    let name = next_custom_dialect_name(&next.custom_dialects);
    next.custom_dialects.push(CustomDialect {
        name: name.clone(),
        definition: String::from("()"),
        show_in_gentufa: dialect_name_shows_in_gentufa_picker(&name),
    });
    set_dialect_settings(dialect_settings, next);
    selected_dialect.set(name);
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn next_custom_dialect_name(customs: &[CustomDialect]) -> String {
    let existing = customs
        .iter()
        .map(|custom| custom.name.trim().to_owned())
        .collect::<BTreeSet<_>>();
    for index in 1.. {
        let candidate = format!("custom-{index}");
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded custom dialect names must contain a free candidate")
}

#[requires(true)]
#[ensures(true)]
fn delete_custom_dialect(
    dialect_settings: Signal<DialectSettings>,
    mut selected_dialect: Signal<String>,
    name: &str,
) {
    let mut next = dialect_settings.read().clone();
    next.custom_dialects
        .retain(|custom| custom.name.trim() != name.trim());
    let selected = initial_dialect_settings_selection(&next);
    set_dialect_settings(dialect_settings, next);
    selected_dialect.set(selected);
}

#[requires(true)]
#[ensures(true)]
fn rename_custom_dialect(
    dialect_settings: Signal<DialectSettings>,
    mut selected_dialect: Signal<String>,
    previous_name: &str,
    next_name: &str,
) {
    let clean_previous = previous_name.trim().to_owned();
    let clean_next = next_name.trim().to_owned();
    let mut next = dialect_settings.read().clone();
    for custom in &mut next.custom_dialects {
        if custom.name.trim() == clean_previous {
            custom.name = next_name.to_owned();
        } else {
            custom.definition =
                replace_dialect_formula_reference(&clean_previous, &clean_next, &custom.definition);
        }
    }
    set_dialect_settings(dialect_settings, next);
    selected_dialect.set(clean_next);
}

#[requires(true)]
#[ensures(true)]
fn update_custom_dialect_definition(
    dialect_settings: Signal<DialectSettings>,
    name: &str,
    definition: &str,
) {
    let mut next = dialect_settings.read().clone();
    for custom in &mut next.custom_dialects {
        if custom.name.trim() == name.trim() {
            custom.definition = definition.to_owned();
        }
    }
    set_dialect_settings(dialect_settings, next);
}

#[requires(true)]
#[ensures(true)]
fn toggle_custom_dialect_gentufa_visibility(dialect_settings: Signal<DialectSettings>, name: &str) {
    let mut next = dialect_settings.read().clone();
    for custom in &mut next.custom_dialects {
        if custom.name.trim() == name.trim() {
            custom.show_in_gentufa = !custom.show_in_gentufa;
        }
    }
    set_dialect_settings(dialect_settings, next);
}

#[requires(true)]
#[ensures(true)]
fn toggle_builtin_dialect_gentufa_visibility(
    dialect_settings: Signal<DialectSettings>,
    name: &str,
    currently_visible: bool,
) {
    let mut next = dialect_settings.read().clone();
    if currently_visible {
        next.hidden_builtin_gentufa_dialects.insert(name.to_owned());
    } else {
        next.hidden_builtin_gentufa_dialects.remove(name);
    }
    set_dialect_settings(dialect_settings, next);
}

#[requires(true)]
#[ensures(true)]
fn render_embedding_settings(
    mut embedding_settings: Signal<EmbeddingSettingsState>,
    state: &EmbeddingSettingsState,
    activity: Signal<AsyncActivityState>,
    page_find: &PageFindContext,
) -> Element {
    let busy = state.busy;
    let webgpu_unavailable = state.webgpu_available == Some(false);
    let selected_model_key = state.selected_model_key.clone();
    rsx! {
        section { class: "settings-section embeddings-settings",
            h2 { { render_page_find_text(page_find, "Semantic search") } }
            label { class: "settings-model-select-row",
                span { class: "settings-model-select-label",
                    { render_page_find_text(page_find, "Embedding model") }
                }
                select {
                    class: "settings-select",
                    value: "{state.selected_model_key}",
                    disabled: busy,
                    onchange: move |event| {
                        let next_key = event.value();
                        if !is_supported_embedding_model_key(&next_key) {
                            return;
                        }
                        save_embedding_model_key(&next_key);
                        configure_embedding_model_key(&next_key);
                        let mut next = embedding_settings.read().clone();
                        next.selected_model_key = next_key.clone();
                        next.selected_model_label = embedding_model_label(&next_key).to_owned();
                        next.effective_model_key = next_key;
                        next.status = "unknown".to_owned();
                        next.detail = "Checking embedding storage.".to_owned();
                        next.model_size = "unknown".to_owned();
                        next.index_size = "unknown".to_owned();
                        next.progress_kind = None;
                        next.progress_label = None;
                        next.progress_loaded = None;
                        next.progress_total = None;
                        next.progress_percent = None;
                        next.remove_confirmation_open = false;
                        embedding_settings.set(next);
                        spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                            refresh_embedding_settings(embedding_settings).await;
                        });
                    },
                    for option in embedding_model_options().iter() {
                        {
                            let disabled = webgpu_unavailable && option.key != F2LLM_80M_MODEL_KEY;
                            rsx! {
                                option {
                                    value: "{option.key}",
                                    disabled,
                                    "{option.label}"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "settings-kv-grid",
                span { class: "settings-kv-label",
                    { render_page_find_text(page_find, "Status") }
                }
                span { class: "settings-kv-value",
                    { render_page_find_text(page_find, &state.status) }
                }
                span { class: "settings-kv-label",
                    { render_page_find_text(page_find, "Model") }
                }
                span { class: "settings-kv-value",
                    { render_page_find_text(page_find, &state.model_size) }
                }
                span { class: "settings-kv-label",
                    { render_page_find_text(page_find, "Index") }
                }
                span { class: "settings-kv-value",
                    { render_page_find_text(page_find, &state.index_size) }
                }
            }
            p { class: "settings-help-text",
                { render_page_find_text(page_find, &state.detail) }
            }
            { render_embedding_progress(state, page_find) }
            div { class: "settings-actions",
                button {
                    class: "settings-action-button",
                    r#type: "button",
                    disabled: busy,
                    onclick: move |_| {
                        let mut next = embedding_settings.read().clone();
                        next.busy = true;
                        next.detail = "Downloading model and preparing the embedding index.".to_owned();
                        next.progress_kind = Some("setup".to_owned());
                        next.progress_label = Some("Embedding setup".to_owned());
                        next.progress_loaded = None;
                        next.progress_total = None;
                        next.progress_percent = None;
                        embedding_settings.set(next);
                        spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                            poll_embedding_settings_while_busy(embedding_settings).await;
                        });
                        spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                            setup_embeddings(embedding_settings).await;
                        });
                    },
                    { render_page_find_text(page_find, "Download") }
                }
                button {
                    class: "settings-action-button",
                    r#type: "button",
                    disabled: busy,
                    onclick: move |_| {
                        let mut next = embedding_settings.read().clone();
                        next.busy = true;
                        next.detail = "Checking for a compatible vector pack.".to_owned();
                        next.progress_kind = Some("setup".to_owned());
                        next.progress_label = Some("Embedding setup".to_owned());
                        next.progress_loaded = None;
                        next.progress_total = None;
                        next.progress_percent = None;
                        embedding_settings.set(next);
                        spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                            poll_embedding_settings_while_busy(embedding_settings).await;
                        });
                        spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                            setup_embeddings(embedding_settings).await;
                        });
                    },
                    { render_page_find_text(page_find, "Update") }
                }
                button {
                    class: "settings-action-button danger",
                    r#type: "button",
                    disabled: busy,
                    onclick: move |_| {
                        let mut next = embedding_settings.read().clone();
                        next.remove_confirmation_open = true;
                        embedding_settings.set(next);
                    },
                    { render_page_find_text(page_find, "Remove") }
                }
            }
            if state.remove_confirmation_open {
                div {
                    class: "settings-confirmation-popout",
                    role: "dialog",
                    aria_modal: "true",
                    aria_label: "Remove embedding model",
                    div { class: "settings-confirmation-card",
                        h3 {
                            { render_page_find_text(page_find, &format!("Remove {}", state.selected_model_label)) }
                        }
                        p {
                            { render_page_find_text(page_find, "This will remove the selected model files and vector index from this device.") }
                        }
                        div { class: "settings-actions",
                            button {
                                class: "settings-action-button",
                                r#type: "button",
                                onclick: move |_| {
                                    let mut next = embedding_settings.read().clone();
                                    next.remove_confirmation_open = false;
                                    embedding_settings.set(next);
                                },
                                { render_page_find_text(page_find, "Cancel") }
                            }
                            button {
                                class: "settings-action-button danger",
                                r#type: "button",
                                onclick: move |_| {
                                    configure_embedding_model_key(&selected_model_key);
                                    let mut next = embedding_settings.read().clone();
                                    next.busy = true;
                                    next.remove_confirmation_open = false;
                                    next.detail = "Removing selected embedding model and index.".to_owned();
                                    next.progress_kind = None;
                                    next.progress_label = None;
                                    next.progress_loaded = None;
                                    next.progress_total = None;
                                    next.progress_percent = None;
                                    embedding_settings.set(next);
                                    spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                                        remove_embeddings(embedding_settings).await;
                                    });
                                },
                                { render_page_find_text(page_find, "Remove") }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_embedding_progress(
    state: &EmbeddingSettingsState,
    page_find: &PageFindContext,
) -> Element {
    if !state.busy && state.progress_percent.is_none() {
        return rsx! {};
    }
    let label = embedding_progress_display_label(state);
    if let Some(percent) = state.progress_percent {
        rsx! {
            div { class: "settings-progress-row",
                progress {
                    class: "settings-progress",
                    max: "100",
                    value: "{percent}",
                    aria_label: "{label}",
                }
                span { class: "settings-progress-label",
                    { render_page_find_text(page_find, &label) }
                }
            }
        }
    } else {
        rsx! {
            div { class: "settings-progress-row",
                progress {
                    class: "settings-progress",
                    aria_label: "{label}",
                }
                span { class: "settings-progress-label",
                    { render_page_find_text(page_find, &label) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn embedding_progress_display_label(state: &EmbeddingSettingsState) -> String {
    let label = state.progress_label.as_deref().unwrap_or("Embedding setup");
    let Some(loaded) = state.progress_loaded else {
        return state
            .progress_percent
            .map(|percent| format!("{label} {percent}%"))
            .unwrap_or_else(|| label.to_owned());
    };
    let Some(total) = state.progress_total else {
        return label.to_owned();
    };
    let progress_suffix = state
        .progress_percent
        .map(|percent| format!(" ({percent}%)"))
        .unwrap_or_default();
    match state.progress_kind.as_deref() {
        Some("download") | Some("validate") => {
            format!(
                "{label} {} / {}{progress_suffix}",
                human_bytes(loaded),
                human_bytes(total)
            )
        }
        _ => format!("{label} {loaded}/{total} rows{progress_suffix}"),
    }
}

#[requires(!name.is_empty())]
#[ensures(true)]
fn render_disabled(name: &str) -> Element {
    rsx! {
        section { class: "spa-page disabled-page",
            div { class: "page-container",
                h1 { "{name}" }
                p { "This tool is not available in jbotci v1 yet." }
            }
        }
    }
}

#[requires(count > 0)]
#[ensures(!ret.is_empty())]
fn repeated_parse_tree_template(count: usize) -> String {
    format!("repeat({count}, max-content)")
}

#[requires(true)]
#[ensures(true)]
fn tree_row_is_elided(row: &GentufaTreeRow) -> bool {
    !row.cells.is_empty() && row.cells.iter().all(|cell| cell.is_elided)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn block_class(block: &GentufaBlock) -> String {
    let mut class = if block.is_leaf {
        "block block-leaf".to_owned()
    } else {
        "block block-nonleaf".to_owned()
    };
    if block.is_elided {
        class.push_str(" block-elided");
    }
    class
}

#[requires(true)]
#[ensures(true)]
fn web_options(
    settings: UserSettings,
    display: GentufaDisplayState,
    view_mode: GentufaWebViewMode,
    dialect: String,
    dialect_settings: &DialectSettings,
) -> GentufaWebOptions {
    let dialect = resolved_dialect_formula_for_request(dialect_settings, &dialect);
    GentufaWebOptions {
        dialect: if dialect.trim().is_empty() {
            None
        } else {
            Some(dialect)
        },
        view_mode,
        script: settings.script,
        show_elided: display.show_elided,
        show_glosses: display.show_glosses,
        show_definitions: false,
        phonemes: PhonemeRenderOptions {
            mark_stress: settings.stress,
            mark_glides: settings.glides,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn resolved_dialect_formula_for_request(settings: &DialectSettings, dialect: &str) -> String {
    if dialect.trim().is_empty() {
        return String::new();
    }
    parse_dialect_selection_formula(settings, dialect)
        .map(|definition| dialect_definition_to_text(&definition))
        .unwrap_or_else(|_| dialect.to_owned())
}

#[requires(true)]
#[ensures(true)]
fn set_theme(settings: &mut Signal<UserSettings>, theme: ThemeMode) {
    let mut next = *settings.read();
    next.theme = theme;
    settings.set(next);
    save_settings(&next);
}

#[requires(true)]
#[ensures(true)]
fn set_script(settings: &mut Signal<UserSettings>, script: GentufaScript) {
    let mut next = *settings.read();
    next.script = script;
    settings.set(next);
    save_settings(&next);
}

#[requires(true)]
#[ensures(true)]
fn set_stress_mark(settings: &mut Signal<UserSettings>, stress: StressMark) {
    let mut next = *settings.read();
    next.stress = stress;
    settings.set(next);
    save_settings(&next);
}

#[requires(true)]
#[ensures(true)]
fn set_glide_mark(settings: &mut Signal<UserSettings>, glides: GlideMark) {
    let mut next = *settings.read();
    next.glides = glides;
    settings.set(next);
    save_settings(&next);
}

#[requires(true)]
#[ensures(true)]
fn toggle_elided(display: &mut Signal<GentufaDisplayState>) {
    let mut next = *display.read();
    next.show_elided = !next.show_elided;
    display.set(next);
}

#[requires(true)]
#[ensures(true)]
fn toggle_glosses(display: &mut Signal<GentufaDisplayState>) {
    let mut next = *display.read();
    next.show_glosses = !next.show_glosses;
    display.set(next);
}

#[requires(true)]
#[ensures(active -> ret.contains("active"))]
#[ensures(loading -> ret.contains("is-loading"))]
fn topbar_link_class(active: bool, loading: bool) -> String {
    class_names(
        "app-topbar-link",
        &[("active", active), ("is-loading", loading)],
    )
}

#[requires(true)]
#[ensures(active -> ret.contains("is-active"))]
fn topbar_activity_class(active: bool) -> String {
    class_names(
        "app-topbar-center app-topbar-activity",
        &[("is-active", active)],
    )
}

#[requires(true)]
#[ensures(active -> ret.contains("active"))]
fn view_tab_class(active: bool) -> &'static str {
    if active {
        "view-tab active"
    } else {
        "view-tab"
    }
}

#[requires(true)]
#[ensures(active -> ret.contains("is-active"))]
fn theme_button_class(active: bool) -> &'static str {
    if active {
        "theme-btn is-active"
    } else {
        "theme-btn"
    }
}

#[requires(true)]
#[ensures(active -> ret.contains("is-active"))]
fn orthography_button_class(active: bool, zbalermorna: bool) -> &'static str {
    match (active, zbalermorna) {
        (true, true) => "theme-btn orthography-btn is-zbalermorna is-active",
        (true, false) => "theme-btn orthography-btn is-active",
        (false, true) => "theme-btn orthography-btn is-zbalermorna",
        (false, false) => "theme-btn orthography-btn",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn pressed_attr(active: bool) -> &'static str {
    if active { "true" } else { "false" }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn theme_class(theme: ThemeMode) -> &'static str {
    match theme {
        ThemeMode::Auto => "auto",
        ThemeMode::Day => "day",
        ThemeMode::Night => "night",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn script_class(script: GentufaScript) -> &'static str {
    match script {
        GentufaScript::Latin => "latin",
        GentufaScript::Cyrillic => "cyrillic",
        GentufaScript::Zbalermorna => "zbalermorna",
    }
}

#[requires(true)]
#[ensures(true)]
fn initial_vlacku_state(route: &JbotciRoute) -> VlackuWebState {
    if let WebRoute::Vlacku(state) = &route.web_route {
        state.clone()
    } else {
        VlackuWebState::default()
    }
}

#[requires(true)]
#[ensures(true)]
fn initial_cukta_state(route: &JbotciRoute) -> CuktaWebState {
    if let WebRoute::Cukta(state) = &route.web_route {
        state.clone()
    } else {
        CuktaWebState::default()
    }
}

#[requires(true)]
#[ensures(true)]
fn initial_gentufa_state(route: &JbotciRoute) -> GentufaWebState {
    if let WebRoute::Gentufa(state) = &route.web_route {
        state.clone()
    } else {
        GentufaWebState::default()
    }
}

#[requires(true)]
#[ensures(true)]
fn initial_gentufa_text_explicit(route: &JbotciRoute) -> bool {
    route.gentufa_text_explicit
}

#[requires(true)]
#[ensures(ret.is_empty() || ret.starts_with('/'))]
fn router_base_path() -> String {
    dioxus::router::router().prefix().unwrap_or_default()
}

#[requires(true)]
#[ensures(ret.starts_with('/'))]
fn route_href_with_base_path(base_path: &str, route: &JbotciRoute) -> String {
    let route_href = route.to_string();
    let prefix = base_path.trim_end_matches('/');
    if prefix.is_empty() || prefix == "/" {
        route_href
    } else {
        format!("{prefix}{route_href}")
    }
}

#[requires(base_path.is_empty() || base_path.starts_with('/'))]
#[ensures(ret.starts_with('/'))]
fn deployment_root_href(base_path: &str) -> String {
    let prefix = base_path.trim_end_matches('/');
    if prefix.is_empty() || prefix == "/" {
        "/".to_owned()
    } else {
        format!("{prefix}/")
    }
}

#[requires(base_path.is_empty() || base_path.starts_with('/'))]
#[requires(path.starts_with('/'))]
#[ensures(ret.starts_with('/'))]
fn static_asset_href_with_base_path(base_path: &str, path: &str) -> String {
    let prefix = base_path.trim_end_matches('/');
    if prefix.is_empty() || prefix == "/" {
        path.to_owned()
    } else {
        format!("{prefix}{path}")
    }
}

#[requires(true)]
#[ensures(true)]
fn gentufa_state_from_parts(
    text: &str,
    dialect: &str,
    view_mode: GentufaWebViewMode,
    display: GentufaDisplayState,
    text_explicit: bool,
) -> GentufaWebState {
    GentufaWebState {
        text: if text_explicit {
            text.to_owned()
        } else {
            String::new()
        },
        dialect: if dialect.trim().is_empty() {
            None
        } else {
            Some(dialect.to_owned())
        },
        view_mode,
        show_elided: display.show_elided,
        show_glosses: display.show_glosses,
    }
}

#[requires(true)]
#[ensures(true)]
fn app_route_for_web_route(route: &WebRoute) -> AppRoute {
    match route {
        WebRoute::Gentufa(_) => AppRoute::Gentufa,
        WebRoute::Cukta(_) => AppRoute::Cukta,
        WebRoute::Vlacku(_) => AppRoute::Vlacku,
        WebRoute::Settings => AppRoute::Settings,
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn install_browser_dom_handlers(
    jvozba_available: Signal<bool>,
    topbar_settings_layout: Signal<TopbarSettingsLayout>,
    topbar_settings_open: Signal<bool>,
    topbar_nav_layout: Signal<TopbarNavLayout>,
    cukta_toc_forced_autohide: Signal<bool>,
) {
    let should_install = BROWSER_STATE_HANDLERS_INSTALLED.with(|installed| {
        if installed.get() {
            false
        } else {
            installed.set(true);
            true
        }
    });
    if !should_install {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let tooltip_pointer_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        position_dictionary_tooltip_from_event(&event);
    }) as Box<dyn FnMut(_)>);
    let _ = document.add_event_listener_with_callback(
        "mouseover",
        tooltip_pointer_closure.as_ref().unchecked_ref(),
    );
    tooltip_pointer_closure.forget();

    let tooltip_focus_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        position_dictionary_tooltip_from_event(&event);
    }) as Box<dyn FnMut(_)>);
    let _ = document.add_event_listener_with_callback(
        "focusin",
        tooltip_focus_closure.as_ref().unchecked_ref(),
    );
    tooltip_focus_closure.forget();

    let page_find_keydown_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if event_is_page_find_shortcut(&event) {
            event.prevent_default();
            focus_page_find_input();
        }
    }) as Box<dyn FnMut(_)>);
    let _ = document.add_event_listener_with_callback_and_bool(
        "keydown",
        page_find_keydown_closure.as_ref().unchecked_ref(),
        true,
    );
    page_find_keydown_closure.forget();

    let resize_layout = topbar_settings_layout;
    let resize_open = topbar_settings_open;
    let resize_nav_layout = topbar_nav_layout;
    let resize_jvozba_available = jvozba_available;
    let resize_cukta_toc_forced_autohide = cukta_toc_forced_autohide;
    let resize_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        schedule_gentufa_block_reference_layout();
        schedule_gentufa_tree_layout();
        schedule_topbar_settings_layout_measure(resize_layout, resize_open, resize_nav_layout);
        update_vlacku_jvozba_availability(resize_jvozba_available);
        update_cukta_toc_forced_autohide(resize_cukta_toc_forced_autohide);
        schedule_vlacku_jvozba_pane_metrics_sync();
    }) as Box<dyn FnMut(_)>);
    let _ =
        window.add_event_listener_with_callback("resize", resize_closure.as_ref().unchecked_ref());
    resize_closure.forget();

    let load_layout = topbar_settings_layout;
    let load_open = topbar_settings_open;
    let load_nav_layout = topbar_nav_layout;
    let load_jvozba_available = jvozba_available;
    let load_cukta_toc_forced_autohide = cukta_toc_forced_autohide;
    let window_load_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        schedule_gentufa_block_reference_layout();
        schedule_gentufa_tree_layout();
        schedule_topbar_settings_layout_measure(load_layout, load_open, load_nav_layout);
        update_vlacku_jvozba_availability(load_jvozba_available);
        update_cukta_toc_forced_autohide(load_cukta_toc_forced_autohide);
        schedule_vlacku_jvozba_pane_metrics_sync();
    }) as Box<dyn FnMut(_)>);
    let _ = window
        .add_event_listener_with_callback("load", window_load_closure.as_ref().unchecked_ref());
    window_load_closure.forget();

    let stylesheet_layout = topbar_settings_layout;
    let stylesheet_open = topbar_settings_open;
    let stylesheet_nav_layout = topbar_nav_layout;
    let stylesheet_load_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if event_target_is_stylesheet_link(&event) {
            schedule_gentufa_block_reference_layout();
            schedule_gentufa_tree_layout();
            schedule_topbar_settings_layout_measure(
                stylesheet_layout,
                stylesheet_open,
                stylesheet_nav_layout,
            );
            schedule_vlacku_jvozba_pane_metrics_sync();
        }
    }) as Box<dyn FnMut(_)>);
    let _ = document.add_event_listener_with_callback_and_bool(
        "load",
        stylesheet_load_closure.as_ref().unchecked_ref(),
        true,
    );
    stylesheet_load_closure.forget();
    schedule_gentufa_block_reference_layout_after_fonts_ready(&document);
    schedule_gentufa_tree_layout_after_fonts_ready(&document);
    schedule_topbar_settings_layout_after_fonts_ready(
        &document,
        topbar_settings_layout,
        topbar_settings_open,
        topbar_nav_layout,
    );
    schedule_vlacku_jvozba_pane_metrics_after_fonts_ready(&document);

    let document_scroll_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        save_current_scroll_position();
    }) as Box<dyn FnMut(_)>);
    let _ = document.add_event_listener_with_callback_and_bool(
        "scroll",
        document_scroll_closure.as_ref().unchecked_ref(),
        true,
    );
    document_scroll_closure.forget();

    let window_scroll_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        save_current_scroll_position();
    }) as Box<dyn FnMut(_)>);
    let _ = window
        .add_event_listener_with_callback("scroll", window_scroll_closure.as_ref().unchecked_ref());
    window_scroll_closure.forget();
    restore_scroll_for_current_url();
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn install_browser_dom_handlers(
    jvozba_available: Signal<bool>,
    topbar_settings_layout: Signal<TopbarSettingsLayout>,
    topbar_settings_open: Signal<bool>,
    topbar_nav_layout: Signal<TopbarNavLayout>,
    cukta_toc_forced_autohide: Signal<bool>,
) {
    if DESKTOP_DOM_HANDLERS_INSTALLED.set(()).is_err() {
        return;
    }
    install_desktop_tooltip_bridge();
    spawn(async move {
        let mut eval = document::eval(
            r#"
            window.addEventListener("keydown", (event) => {
                if ((event.ctrlKey || event.metaKey) && !event.altKey && String(event.key || "").toLowerCase() === "f") {
                    event.preventDefault();
                    const input = document.getElementById("app-page-find-input");
                    if (input) {
                        input.focus();
                        if (typeof input.select === "function") {
                            input.select();
                        }
                    }
                }
            }, true);
            const sendLayout = () => {
                try {
                    dioxus.send("layout");
                } catch (_error) {
                }
            };
            const scheduleLayout = () => requestAnimationFrame(sendLayout);
            window.addEventListener("resize", scheduleLayout);
            window.addEventListener("load", sendLayout);
            for (const link of Array.from(document.querySelectorAll('link[rel~="stylesheet"]'))) {
                link.addEventListener("load", scheduleLayout, { once: true });
            }
            if (document.fonts && document.fonts.ready) {
                document.fonts.ready.then(sendLayout).catch(() => {});
            }
            scheduleLayout();
            await new Promise(() => {});
            "#,
        );
        while eval.recv::<String>().await.is_ok() {
            schedule_gentufa_block_reference_layout();
            schedule_gentufa_tree_layout();
            schedule_topbar_settings_layout_measure(
                topbar_settings_layout,
                topbar_settings_open,
                topbar_nav_layout,
            );
            update_vlacku_jvozba_availability(jvozba_available);
            update_cukta_toc_forced_autohide(cukta_toc_forced_autohide);
            schedule_vlacku_jvozba_pane_metrics_sync();
        }
    });
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
fn install_browser_dom_handlers(
    jvozba_available: Signal<bool>,
    topbar_settings_layout: Signal<TopbarSettingsLayout>,
    topbar_settings_open: Signal<bool>,
    topbar_nav_layout: Signal<TopbarNavLayout>,
    cukta_toc_forced_autohide: Signal<bool>,
) {
    let _ = (
        jvozba_available,
        topbar_settings_layout,
        topbar_settings_open,
        topbar_nav_layout,
        cukta_toc_forced_autohide,
    );
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_textarea_resize() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move || resize_gentufa_textarea());
    let _ = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), 0);
    closure.forget();
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_textarea_resize() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn resize_gentufa_textarea() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(element) = document.get_element_by_id("gentufa-text") else {
        return;
    };
    let Some(textarea) = element.dyn_ref::<web_sys::HtmlTextAreaElement>() else {
        return;
    };
    let textarea_html: &web_sys::HtmlElement = textarea.unchecked_ref();
    let style = textarea_html.style();
    let _ = style.remove_property("height");
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_block_reference_layout() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move || {
        adjust_gentufa_block_reference_layout();
        schedule_gentufa_block_reference_layout_animation_frames(
            GENTUFA_BLOCK_REFERENCE_LAYOUT_FRAME_PASSES,
        );
    });
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        GENTUFA_BLOCK_REFERENCE_LAYOUT_DELAY_MS,
    );
    closure.forget();
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_block_reference_layout() {
    spawn(async move {
        sleep_ms(GENTUFA_BLOCK_REFERENCE_LAYOUT_DELAY_MS).await;
        adjust_gentufa_block_reference_layout_desktop().await;
        for _ in 0..GENTUFA_BLOCK_REFERENCE_LAYOUT_FRAME_PASSES {
            sleep_ms(16).await;
            adjust_gentufa_block_reference_layout_desktop().await;
        }
    });
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_block_reference_layout() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_tree_layout() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move || {
        layout_gentufa_tree_lines();
        schedule_gentufa_tree_layout_animation_frames(GENTUFA_TREE_LAYOUT_FRAME_PASSES);
    });
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        GENTUFA_TREE_LAYOUT_DELAY_MS,
    );
    closure.forget();
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_tree_layout() {
    spawn(async move {
        sleep_ms(GENTUFA_TREE_LAYOUT_DELAY_MS).await;
        layout_gentufa_tree_lines_desktop().await;
        for _ in 0..GENTUFA_TREE_LAYOUT_FRAME_PASSES {
            sleep_ms(16).await;
            layout_gentufa_tree_lines_desktop().await;
        }
    });
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_tree_layout() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_block_reference_layout_animation_frames(remaining: u8) {
    if remaining == 0 {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move |_timestamp: f64| {
        adjust_gentufa_block_reference_layout();
        schedule_gentufa_block_reference_layout_animation_frames(remaining - 1);
    });
    let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
    closure.forget();
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_tree_layout_animation_frames(remaining: u8) {
    if remaining == 0 {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move |_timestamp: f64| {
        layout_gentufa_tree_lines();
        schedule_gentufa_tree_layout_animation_frames(remaining - 1);
    });
    let _ = window.request_animation_frame(closure.as_ref().unchecked_ref());
    closure.forget();
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_block_reference_layout_after_fonts_ready(document: &web_sys::Document) {
    let Ok(fonts) = js_sys::Reflect::get(document.as_ref(), &JsValue::from_str("fonts")) else {
        return;
    };
    let Ok(ready) = js_sys::Reflect::get(&fonts, &JsValue::from_str("ready")) else {
        return;
    };
    let Ok(promise) = ready.dyn_into::<js_sys::Promise>() else {
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        adjust_gentufa_block_reference_layout();
    });
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_tree_layout_after_fonts_ready(document: &web_sys::Document) {
    let Ok(fonts) = js_sys::Reflect::get(document.as_ref(), &JsValue::from_str("fonts")) else {
        return;
    };
    let Ok(ready) = js_sys::Reflect::get(&fonts, &JsValue::from_str("ready")) else {
        return;
    };
    let Ok(promise) = ready.dyn_into::<js_sys::Promise>() else {
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        layout_gentufa_tree_lines();
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_tree_layout_after_fonts_ready(document: &()) {
    let _ = document;
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[invariant(true)]
struct GentufaTreeLineAnchor {
    parent_id: Option<usize>,
    depth: usize,
    label_left: f64,
    label_center_y: f64,
    row_top: f64,
    row_bottom: f64,
}

#[requires(true)]
#[ensures(true)]
fn gentufa_tree_line_paths(
    ordered_anchors: &[(usize, GentufaTreeLineAnchor)],
    table_bottom: f64,
) -> Vec<String> {
    let mut parents_with_children = BTreeSet::new();
    for (_, anchor) in ordered_anchors {
        if let Some(parent_id) = anchor.parent_id {
            parents_with_children.insert(parent_id);
        }
    }
    let mut paths = Vec::new();
    for (index, (node_id, anchor)) in ordered_anchors.iter().enumerate() {
        if !parents_with_children.contains(node_id) {
            continue;
        }
        let end_y = ordered_anchors
            .iter()
            .skip(index + 1)
            .find_map(|(_, candidate)| {
                (candidate.depth <= anchor.depth).then_some(candidate.row_top)
            })
            .unwrap_or(table_bottom.max(anchor.row_bottom));
        if end_y <= anchor.label_center_y {
            continue;
        }
        paths.push(gentufa_tree_line_path_data(
            anchor.label_left,
            anchor.label_center_y,
            end_y,
        ));
    }
    paths
}

#[requires(end_y >= start_y)]
#[ensures(!ret.is_empty())]
fn gentufa_tree_line_path_data(x: f64, start_y: f64, end_y: f64) -> String {
    format!("M {x:.3} {start_y:.3} V {end_y:.3}")
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn layout_gentufa_tree_lines() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(Some(wrap)) = document.query_selector(".parse-page .table-wrap") else {
        return;
    };
    let Ok(Some(svg)) = wrap.query_selector(".tree-lines") else {
        return;
    };
    let Ok(Some(table)) = wrap.query_selector(".parse-table") else {
        clear_svg_children(&svg);
        return;
    };
    let Some(wrap_html) = wrap.dyn_ref::<web_sys::HtmlElement>() else {
        return;
    };
    let Some(table_html) = table.dyn_ref::<web_sys::HtmlElement>() else {
        return;
    };
    clear_svg_children(&svg);
    let wrap_rect = wrap.get_bounding_client_rect();
    let table_rect = table.get_bounding_client_rect();
    let scroll_left = f64::from(wrap_html.scroll_left());
    let scroll_top = f64::from(wrap_html.scroll_top());
    let width = f64::from(wrap_html.scroll_width())
        .max(f64::from(table_html.scroll_width()))
        .max(table_rect.right() - wrap_rect.left() + scroll_left);
    let height = f64::from(wrap_html.scroll_height())
        .max(f64::from(table_html.scroll_height()))
        .max(table_rect.bottom() - wrap_rect.top() + scroll_top);
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let _ = svg.set_attribute("width", &format!("{width:.3}"));
    let _ = svg.set_attribute("height", &format!("{height:.3}"));
    let _ = svg.set_attribute("viewBox", &format!("0 0 {width:.3} {height:.3}"));
    let Ok(row_nodes) = table.query_selector_all("tbody tr.tree-row") else {
        return;
    };
    let mut ordered_anchors = Vec::new();
    for index in 0..row_nodes.length() {
        let Some(node) = row_nodes.item(index) else {
            continue;
        };
        let Ok(row) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(node_id) = element_usize_attr(&row, "data-node-id") else {
            continue;
        };
        let Some(anchor) = tree_line_anchor_for_row(&row, &wrap, wrap_html) else {
            continue;
        };
        ordered_anchors.push((node_id, anchor));
    }
    let table_bottom = table_rect.bottom() - wrap_rect.top() + scroll_top;
    for path_data in gentufa_tree_line_paths(&ordered_anchors, table_bottom) {
        append_gentufa_tree_line_path(&document, &svg, &path_data);
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
struct DesktopGentufaTreeMetrics {
    width: f64,
    height: f64,
    table_bottom: f64,
    anchors: Vec<DesktopGentufaTreeAnchorMetrics>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[invariant(true)]
struct DesktopGentufaTreeAnchorMetrics {
    node_id: usize,
    parent_id: Option<usize>,
    depth: usize,
    label_left: f64,
    label_center_y: f64,
    row_top: f64,
    row_bottom: f64,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[invariant(true)]
struct DesktopGentufaTreeLayout {
    width: f64,
    height: f64,
    paths: Vec<String>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn layout_gentufa_tree_lines_desktop() {
    let Some(metrics) = measure_gentufa_tree_layout_desktop().await else {
        return;
    };
    let layout = gentufa_tree_layout_from_metrics(metrics);
    apply_gentufa_tree_layout_desktop(layout).await;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn gentufa_tree_layout_from_metrics(
    metrics: DesktopGentufaTreeMetrics,
) -> DesktopGentufaTreeLayout {
    let ordered_anchors = metrics
        .anchors
        .into_iter()
        .map(|anchor| {
            (
                anchor.node_id,
                GentufaTreeLineAnchor {
                    parent_id: anchor.parent_id,
                    depth: anchor.depth,
                    label_left: anchor.label_left,
                    label_center_y: anchor.label_center_y,
                    row_top: anchor.row_top,
                    row_bottom: anchor.row_bottom,
                },
            )
        })
        .collect::<Vec<_>>();
    DesktopGentufaTreeLayout {
        width: metrics.width,
        height: metrics.height,
        paths: gentufa_tree_line_paths(&ordered_anchors, metrics.table_bottom),
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn measure_gentufa_tree_layout_desktop() -> Option<DesktopGentufaTreeMetrics> {
    document::eval(
        r#"
        const wrap = document.querySelector(".parse-page .table-wrap");
        const svg = wrap && wrap.querySelector(".tree-lines");
        if (!wrap || !svg) {
            return null;
        }
        const table = wrap.querySelector(".parse-table");
        if (!table) {
            return {
                width: 0,
                height: 0,
                table_bottom: 0,
                anchors: [],
            };
        }
        const wrapRect = wrap.getBoundingClientRect();
        const tableRect = table.getBoundingClientRect();
        const scrollLeft = Number(wrap.scrollLeft || 0);
        const scrollTop = Number(wrap.scrollTop || 0);
        const width = Math.max(
            Number(wrap.scrollWidth || 0),
            Number(table.scrollWidth || 0),
            tableRect.right - wrapRect.left + scrollLeft,
        );
        const height = Math.max(
            Number(wrap.scrollHeight || 0),
            Number(table.scrollHeight || 0),
            tableRect.bottom - wrapRect.top + scrollTop,
        );
        const parseOptionalInt = (value) => {
            if (value === null || value === undefined || value === "") {
                return null;
            }
            const parsed = Number.parseInt(value, 10);
            return Number.isFinite(parsed) ? parsed : null;
        };
        const anchors = [];
        for (const row of Array.from(table.querySelectorAll("tbody tr.tree-row"))) {
            const nodeId = parseOptionalInt(row.getAttribute("data-node-id"));
            const depth = parseOptionalInt(row.getAttribute("data-depth"));
            const label = row.querySelector(".node-label");
            if (nodeId === null || depth === null || !label) {
                continue;
            }
            const labelRect = label.getBoundingClientRect();
            const rowRect = row.getBoundingClientRect();
            anchors.push({
                node_id: nodeId,
                parent_id: parseOptionalInt(row.getAttribute("data-parent-id")),
                depth,
                label_left: labelRect.left - wrapRect.left + scrollLeft,
                label_center_y: labelRect.top - wrapRect.top + scrollTop + labelRect.height / 2,
                row_top: rowRect.top - wrapRect.top + scrollTop,
                row_bottom: rowRect.bottom - wrapRect.top + scrollTop,
            });
        }
        return {
            width,
            height,
            table_bottom: tableRect.bottom - wrapRect.top + scrollTop,
            anchors,
        };
        "#,
    )
    .join()
    .await
    .ok()
    .flatten()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn apply_gentufa_tree_layout_desktop(layout: DesktopGentufaTreeLayout) {
    let Ok(layout_json) = serde_json::to_string(&layout) else {
        return;
    };
    let script = format!(
        r#"
        const layout = {layout_json};
        const svg = document.querySelector(".parse-page .table-wrap .tree-lines");
        if (svg) {{
            while (svg.firstChild) {{
                svg.removeChild(svg.firstChild);
            }}
            if (Number(layout.width) > 0 && Number(layout.height) > 0) {{
                svg.setAttribute("width", Number(layout.width).toFixed(3));
                svg.setAttribute("height", Number(layout.height).toFixed(3));
                svg.setAttribute("viewBox", `0 0 ${{Number(layout.width).toFixed(3)}} ${{Number(layout.height).toFixed(3)}}`);
                for (const d of layout.paths) {{
                    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
                    path.setAttribute("class", "tree-line");
                    path.setAttribute("d", d);
                    svg.appendChild(path);
                }}
            }}
        }}
        return null;
        "#
    );
    let _ = document::eval(&script).await;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn clear_svg_children(svg: &web_sys::Element) {
    while let Some(child) = svg.first_child() {
        let _ = svg.remove_child(&child);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn tree_line_anchor_for_row(
    row: &web_sys::Element,
    wrap: &web_sys::Element,
    wrap_html: &web_sys::HtmlElement,
) -> Option<GentufaTreeLineAnchor> {
    let label = row.query_selector(".node-label").ok().flatten()?;
    let label_rect = label.get_bounding_client_rect();
    let row_rect = row.get_bounding_client_rect();
    let wrap_rect = wrap.get_bounding_client_rect();
    let scroll_left = f64::from(wrap_html.scroll_left());
    let scroll_top = f64::from(wrap_html.scroll_top());
    Some(GentufaTreeLineAnchor {
        parent_id: element_usize_attr(row, "data-parent-id"),
        depth: element_usize_attr(row, "data-depth")?,
        label_left: label_rect.left() - wrap_rect.left() + scroll_left,
        label_center_y: label_rect.top() - wrap_rect.top() + scroll_top + label_rect.height() / 2.0,
        row_top: row_rect.top() - wrap_rect.top() + scroll_top,
        row_bottom: row_rect.bottom() - wrap_rect.top() + scroll_top,
    })
}

#[cfg(target_arch = "wasm32")]
#[requires(!d.is_empty())]
#[ensures(true)]
fn append_gentufa_tree_line_path(document: &web_sys::Document, svg: &web_sys::Element, d: &str) {
    let Ok(path) = document.create_element_ns(Some("http://www.w3.org/2000/svg"), "path") else {
        return;
    };
    let _ = path.set_attribute("class", "tree-line");
    let _ = path.set_attribute("d", d);
    let _ = svg.append_child(&path);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn event_target_is_stylesheet_link(event: &web_sys::Event) -> bool {
    let Some(element) = event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
    else {
        return false;
    };
    if !element.tag_name().eq_ignore_ascii_case("link") {
        return false;
    }
    element.get_attribute("rel").is_some_and(|rel| {
        rel.split_ascii_whitespace()
            .any(|part| part.eq_ignore_ascii_case("stylesheet"))
    })
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn adjust_gentufa_block_reference_layout() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(nodes) = document.query_selector_all(".parse-page .block") else {
        return;
    };
    let mut blocks = Vec::new();
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(block) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        reset_block_reference_fit_width(&block);
        blocks.push(block);
    }
    reset_block_reference_height_sizers(&document);
    for block in &blocks {
        adjust_block_reference_fit_width(block);
    }
    let row_heights = measured_block_row_heights(&document);
    if row_heights.is_empty() {
        return;
    }
    let mut row_growths = vec![0.0; row_heights.len()];
    let mut indexed_blocks = blocks
        .into_iter()
        .filter_map(|block| {
            let (row, row_span, bottom_row) = block_row_range_for_element(&block)?;
            Some((bottom_row, row, row_span, block))
        })
        .collect::<Vec<_>>();
    indexed_blocks.sort_by_key(|(bottom_row, row, _, _)| (*bottom_row, *row));
    for (_, row, row_span, block) in indexed_blocks {
        if let Some((bottom_row, deficit)) =
            block_reference_height_growth(&block, row, row_span, &row_growths)
            && bottom_row < row_growths.len()
        {
            row_growths[bottom_row] += deficit;
        }
    }
    apply_block_reference_height_sizers(&document, &row_heights, &row_growths);
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn adjust_gentufa_block_reference_layout_desktop() {
    let Some(fit_metrics) = measure_block_reference_fit_metrics_desktop().await else {
        return;
    };
    let fit_updates = block_reference_fit_updates(fit_metrics);
    apply_block_reference_fit_updates_desktop(&fit_updates).await;
    let Some(height_metrics) = measure_block_reference_height_metrics_desktop().await else {
        return;
    };
    if height_metrics.row_heights.is_empty() {
        return;
    }
    let row_growths = block_reference_row_growths(&height_metrics);
    apply_block_reference_height_updates_desktop(BlockReferenceHeightUpdates {
        row_heights: height_metrics.row_heights,
        row_growths,
    })
    .await;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn measure_block_reference_fit_metrics_desktop() -> Option<Vec<BlockReferenceFitMetrics>> {
    document::eval(
        r#"
        const parseMetrics = [];
        const rectFor = (element) => {
            const rect = element.getBoundingClientRect();
            return {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            };
        };
        for (const block of Array.from(document.querySelectorAll(".parse-page .block"))) {
            block.style.removeProperty("--block-reference-fit-width");
        }
        for (const sizer of Array.from(document.querySelectorAll(".parse-page .block-row-height-sizer"))) {
            sizer.style.removeProperty("height");
            sizer.style.removeProperty("min-height");
        }
        for (const block of Array.from(document.querySelectorAll(".parse-page .block"))) {
            const blockId = block.getAttribute("data-block-id") || "";
            const label = block.querySelector(".block-label-text");
            const referenceTarget = block.querySelector(".block-ref-target");
            if (!blockId || !label || !referenceTarget) {
                continue;
            }
            const blockRect = block.getBoundingClientRect();
            const labelRect = label.getBoundingClientRect();
            let referenceRight = null;
            let referenceBottom = null;
            for (const element of Array.from(referenceTarget.querySelectorAll(".ref-var, .ref-var *"))) {
                const rect = element.getBoundingClientRect();
                referenceRight = referenceRight === null ? rect.right : Math.max(referenceRight, rect.right);
                referenceBottom = referenceBottom === null ? rect.bottom : Math.max(referenceBottom, rect.bottom);
            }
            parseMetrics.push({
                block_id: blockId,
                current_width: blockRect.width,
                block_left: blockRect.left,
                label_left: labelRect.left,
                label_top: labelRect.top,
                label_width: labelRect.width,
                reference_right: referenceRight,
                reference_bottom: referenceBottom,
            });
        }
        return parseMetrics;
        "#,
    )
    .join()
    .await
    .ok()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn apply_block_reference_fit_updates_desktop(updates: &[BlockReferenceFitUpdate]) {
    if updates.is_empty() {
        return;
    }
    let Ok(updates_json) = serde_json::to_string(updates) else {
        return;
    };
    let script = format!(
        r#"
        const updates = {updates_json};
        const blocks = new Map(Array.from(document.querySelectorAll(".parse-page .block")).map(
            (block) => [block.getAttribute("data-block-id") || "", block],
        ));
        for (const update of updates) {{
            const block = blocks.get(String(update.block_id));
            if (!block) {{
                continue;
            }}
            block.style.setProperty("--block-reference-fit-width", `${{Number(update.fit_width).toFixed(2)}}px`);
        }}
        return null;
        "#
    );
    let _ = document::eval(&script).await;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn measure_block_reference_height_metrics_desktop()
-> Option<BlockReferenceHeightLayoutMetrics> {
    document::eval(
        r#"
        const rectFor = (element) => {
            const rect = element.getBoundingClientRect();
            return {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            };
        };
        const parseRequiredInt = (value) => {
            const parsed = Number.parseInt(value || "", 10);
            return Number.isFinite(parsed) && parsed >= 0 ? parsed : null;
        };
        const rowHeights = [];
        for (const probe of Array.from(document.querySelectorAll(".parse-page .block-row-height-probe"))) {
            const row = parseRequiredInt(probe.getAttribute("data-block-row"));
            if (row === null) {
                continue;
            }
            while (rowHeights.length <= row) {
                rowHeights.push(0);
            }
            rowHeights[row] = probe.getBoundingClientRect().height;
        }
        const blocks = [];
        for (const block of Array.from(document.querySelectorAll(".parse-page .block"))) {
            const blockId = block.getAttribute("data-block-id") || "";
            const row = parseRequiredInt(block.getAttribute("data-row"));
            const rowSpanRaw = parseRequiredInt(block.getAttribute("data-rowspan"));
            const label = block.querySelector(".block-label-text");
            const referenceTarget = block.querySelector(".block-ref-target");
            if (!blockId || row === null || !label || !referenceTarget) {
                continue;
            }
            const rowSpan = Math.max(1, rowSpanRaw === null ? 1 : rowSpanRaw);
            const blockRect = block.getBoundingClientRect();
            const labelRect = label.getBoundingClientRect();
            blocks.push({
                block_id: blockId,
                row,
                row_span: rowSpan,
                block_top: blockRect.top,
                block_height: blockRect.height,
                label_top: labelRect.top,
                label_left: labelRect.left,
                label_right: labelRect.right,
                reference_target_rect: rectFor(referenceTarget),
                reference_line_rects: Array.from(referenceTarget.querySelectorAll(".ref-line")).map(rectFor),
            });
        }
        return {
            row_heights: rowHeights,
            blocks,
        };
        "#,
    )
    .join()
    .await
    .ok()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn apply_block_reference_height_updates_desktop(updates: BlockReferenceHeightUpdates) {
    let Ok(updates_json) = serde_json::to_string(&updates) else {
        return;
    };
    let script = format!(
        r#"
        const updates = {updates_json};
        for (const sizer of Array.from(document.querySelectorAll(".parse-page .block-row-height-sizer"))) {{
            const row = Number.parseInt(sizer.getAttribute("data-block-row") || "", 10);
            if (!Number.isFinite(row)) {{
                continue;
            }}
            const growth = Number(updates.row_growths[row] || 0);
            const baseHeight = Number(updates.row_heights[row] || 0);
            if (!(growth > 0) || !(baseHeight >= 0)) {{
                continue;
            }}
            const targetHeight = baseHeight + growth;
            const value = `${{targetHeight.toFixed(2)}}px`;
            sizer.style.setProperty("height", value);
            sizer.style.setProperty("min-height", value);
        }}
        return null;
        "#
    );
    let _ = document::eval(&script).await;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn reset_block_reference_fit_width(block: &web_sys::Element) {
    let Some(block) = block.dyn_ref::<web_sys::HtmlElement>() else {
        return;
    };
    let _ = block.style().remove_property("--block-reference-fit-width");
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn reset_block_reference_height_sizers(document: &web_sys::Document) {
    let Ok(nodes) = document.query_selector_all(".parse-page .block-row-height-sizer") else {
        return;
    };
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(html) = element.dyn_ref::<web_sys::HtmlElement>() else {
            continue;
        };
        let style = html.style();
        let _ = style.remove_property("height");
        let _ = style.remove_property("min-height");
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn measured_block_row_heights(document: &web_sys::Document) -> Vec<f64> {
    let Ok(nodes) = document.query_selector_all(".parse-page .block-row-height-probe") else {
        return Vec::new();
    };
    let mut row_heights = Vec::new();
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(row) = element_usize_attr(&element, "data-block-row") else {
            continue;
        };
        if row >= row_heights.len() {
            row_heights.resize(row + 1, 0.0);
        }
        row_heights[row] = element.get_bounding_client_rect().height();
    }
    row_heights
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn block_row_range_for_element(block: &web_sys::Element) -> Option<(usize, usize, usize)> {
    let row = element_usize_attr(block, "data-row")?;
    let row_span = element_usize_attr(block, "data-rowspan")?.max(1);
    Some((row, row_span, row + row_span.saturating_sub(1)))
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn element_usize_attr(element: &web_sys::Element, name: &str) -> Option<usize> {
    element.get_attribute(name)?.parse::<usize>().ok()
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn block_reference_height_growth(
    block: &web_sys::Element,
    row: usize,
    row_span: usize,
    row_growths: &[f64],
) -> Option<(usize, f64)> {
    let bottom_row = row + row_span.saturating_sub(1);
    if bottom_row >= row_growths.len() {
        return None;
    }
    let label_text = block_label_text_for_block(block)?;
    let block_rect = block.get_bounding_client_rect();
    let label_rect = label_text.get_bounding_client_rect();
    let reference_bottoms = reference_bottoms_for_block(block, &label_rect, block_rect.top())?;
    let existing_growth = row_growths[row..=bottom_row].iter().sum::<f64>();
    let containment_deficit = reference_containment_deficit(
        reference_bottoms.stack_bottom,
        block_rect.height(),
        existing_growth,
    );
    let label_deficit = reference_bottoms
        .overlapping_label_bottom
        .map(|reference_bottom| {
            reference_clearance_deficit(
                reference_bottom,
                label_rect.top() - block_rect.top(),
                existing_growth,
            )
        })
        .unwrap_or(0.0);
    let deficit = containment_deficit.max(label_deficit);
    if deficit > 0.0 {
        Some((bottom_row, deficit))
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[invariant(true)]
struct ReferenceBottoms {
    stack_bottom: f64,
    overlapping_label_bottom: Option<f64>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
struct BlockReferenceFitMetrics {
    block_id: String,
    current_width: f64,
    block_left: f64,
    label_left: f64,
    label_top: f64,
    label_width: f64,
    reference_right: Option<f64>,
    reference_bottom: Option<f64>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[invariant(true)]
struct BlockReferenceFitUpdate {
    block_id: String,
    fit_width: f64,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
struct BlockReferenceHeightLayoutMetrics {
    row_heights: Vec<f64>,
    blocks: Vec<BlockReferenceHeightMetrics>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
struct BlockReferenceHeightMetrics {
    block_id: String,
    row: usize,
    row_span: usize,
    block_top: f64,
    block_height: f64,
    label_top: f64,
    label_left: f64,
    label_right: f64,
    reference_target_rect: Option<ReferenceRect>,
    reference_line_rects: Vec<ReferenceRect>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[invariant(true)]
struct BlockReferenceHeightUpdates {
    row_heights: Vec<f64>,
    row_growths: Vec<f64>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn block_reference_fit_updates(
    metrics: Vec<BlockReferenceFitMetrics>,
) -> Vec<BlockReferenceFitUpdate> {
    metrics
        .into_iter()
        .filter_map(|metric| {
            block_reference_fit_width_from_metrics(&metric).map(|fit_width| {
                BlockReferenceFitUpdate {
                    block_id: metric.block_id,
                    fit_width,
                }
            })
        })
        .collect()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.is_none_or(|width| width.is_finite() && width > metric.current_width))]
fn block_reference_fit_width_from_metrics(metric: &BlockReferenceFitMetrics) -> Option<f64> {
    let reference_right = metric.reference_right?;
    let reference_bottom = metric.reference_bottom?;
    let reference_right_in_block = reference_right - metric.block_left;
    if reference_right_in_block <= 0.0 {
        return None;
    }
    let reference_fit_width = reference_right_in_block + BLOCK_REFERENCE_LABEL_GAP_PX;
    let overlap_fit_width = if reference_bottom > metric.label_top {
        let desired_text_left = reference_right + BLOCK_REFERENCE_LABEL_GAP_PX;
        if desired_text_left > metric.label_left {
            (reference_right_in_block + BLOCK_REFERENCE_LABEL_GAP_PX) * 2.0 + metric.label_width
        } else {
            0.0
        }
    } else {
        0.0
    };
    let fit_width = metric
        .current_width
        .max(reference_fit_width)
        .max(overlap_fit_width);
    (fit_width.is_finite() && fit_width > metric.current_width).then_some(fit_width)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.len() == metrics.row_heights.len())]
#[ensures(ret.iter().all(|growth| growth.is_finite() && *growth >= 0.0))]
fn block_reference_row_growths(metrics: &BlockReferenceHeightLayoutMetrics) -> Vec<f64> {
    let mut row_growths = vec![0.0; metrics.row_heights.len()];
    let mut indexed_blocks = metrics
        .blocks
        .iter()
        .filter_map(|block| {
            let bottom_row = block.row + block.row_span.saturating_sub(1);
            Some((bottom_row, block.row, block.row_span, block))
        })
        .collect::<Vec<_>>();
    indexed_blocks.sort_by_key(|(bottom_row, row, _, _)| (*bottom_row, *row));
    for (_, _, _, block) in indexed_blocks {
        if let Some((bottom_row, deficit)) =
            block_reference_height_growth_from_metrics(block, &row_growths)
            && bottom_row < row_growths.len()
        {
            row_growths[bottom_row] += deficit;
        }
    }
    row_growths
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn block_reference_height_growth_from_metrics(
    block: &BlockReferenceHeightMetrics,
    row_growths: &[f64],
) -> Option<(usize, f64)> {
    let bottom_row = block.row + block.row_span.saturating_sub(1);
    if bottom_row >= row_growths.len() {
        return None;
    }
    let reference_bottoms = reference_bottoms_for_block_metrics(block)?;
    let existing_growth = row_growths[block.row..=bottom_row].iter().sum::<f64>();
    let containment_deficit = reference_containment_deficit(
        reference_bottoms.stack_bottom,
        block.block_height,
        existing_growth,
    );
    let label_deficit = reference_bottoms
        .overlapping_label_bottom
        .map(|reference_bottom| {
            reference_clearance_deficit(
                reference_bottom,
                block.label_top - block.block_top,
                existing_growth,
            )
        })
        .unwrap_or(0.0);
    let deficit = containment_deficit.max(label_deficit);
    (deficit > 0.0).then_some((bottom_row, deficit))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn reference_bottoms_for_block_metrics(
    block: &BlockReferenceHeightMetrics,
) -> Option<ReferenceBottoms> {
    if block.reference_line_rects.is_empty() {
        return block
            .reference_target_rect
            .map(|rect| reference_bottoms_for_rect(rect, block));
    }
    let mut stack_bottom = None;
    let mut overlapping_label_bottom = None;
    for rect in &block.reference_line_rects {
        let line_bottom = rect.bottom - block.block_top;
        stack_bottom = Some(stack_bottom.unwrap_or(f64::NEG_INFINITY).max(line_bottom));
        if horizontal_ranges_overlap(rect.left, rect.right, block.label_left, block.label_right) {
            overlapping_label_bottom = Some(
                overlapping_label_bottom
                    .unwrap_or(f64::NEG_INFINITY)
                    .max(line_bottom),
            );
        }
    }
    stack_bottom.map(|stack_bottom| ReferenceBottoms {
        stack_bottom,
        overlapping_label_bottom,
    })
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn reference_bottoms_for_rect(
    rect: ReferenceRect,
    block: &BlockReferenceHeightMetrics,
) -> ReferenceBottoms {
    let stack_bottom = rect.bottom - block.block_top;
    let overlapping_label_bottom =
        horizontal_ranges_overlap(rect.left, rect.right, block.label_left, block.label_right)
            .then_some(stack_bottom);
    ReferenceBottoms {
        stack_bottom,
        overlapping_label_bottom,
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn reference_bottoms_for_block(
    block: &web_sys::Element,
    label_rect: &web_sys::DomRect,
    block_top: f64,
) -> Option<ReferenceBottoms> {
    let reference_target = block_reference_target_for_block(block)?;
    let Ok(line_nodes) = reference_target.query_selector_all(".ref-line") else {
        return reference_bottoms_for_element(&reference_target, label_rect, block_top);
    };
    if line_nodes.length() == 0 {
        return reference_bottoms_for_element(&reference_target, label_rect, block_top);
    }
    let mut stack_bottom = None;
    let mut overlapping_label_bottom = None;
    for index in 0..line_nodes.length() {
        let Some(node) = line_nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let rect = element.get_bounding_client_rect();
        let line_bottom = rect.bottom() - block_top;
        stack_bottom = Some(stack_bottom.unwrap_or(f64::NEG_INFINITY).max(line_bottom));
        if horizontal_ranges_overlap(
            rect.left(),
            rect.right(),
            label_rect.left(),
            label_rect.right(),
        ) {
            overlapping_label_bottom = Some(
                overlapping_label_bottom
                    .unwrap_or(f64::NEG_INFINITY)
                    .max(line_bottom),
            );
        }
    }
    stack_bottom.map(|stack_bottom| ReferenceBottoms {
        stack_bottom,
        overlapping_label_bottom,
    })
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn reference_bottoms_for_element(
    element: &web_sys::Element,
    label_rect: &web_sys::DomRect,
    block_top: f64,
) -> Option<ReferenceBottoms> {
    let rect = element.get_bounding_client_rect();
    let stack_bottom = rect.bottom() - block_top;
    let overlapping_label_bottom = horizontal_ranges_overlap(
        rect.left(),
        rect.right(),
        label_rect.left(),
        label_rect.right(),
    )
    .then_some(stack_bottom);
    Some(ReferenceBottoms {
        stack_bottom,
        overlapping_label_bottom,
    })
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn apply_block_reference_height_sizers(
    document: &web_sys::Document,
    row_heights: &[f64],
    row_growths: &[f64],
) {
    let Ok(nodes) = document.query_selector_all(".parse-page .block-row-height-sizer") else {
        return;
    };
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(row) = element_usize_attr(&element, "data-block-row") else {
            continue;
        };
        let Some(growth) = row_growths.get(row).copied() else {
            continue;
        };
        if growth <= 0.0 {
            continue;
        }
        let Some(base_height) = row_heights.get(row).copied() else {
            continue;
        };
        let Some(html) = element.dyn_ref::<web_sys::HtmlElement>() else {
            continue;
        };
        let target_height = base_height + growth;
        let value = format!("{target_height:.2}px");
        let style = html.style();
        let _ = style.set_property("height", &value);
        let _ = style.set_property("min-height", &value);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn adjust_block_reference_fit_width(block: &web_sys::Element) {
    let Some(block_html) = block.dyn_ref::<web_sys::HtmlElement>() else {
        return;
    };
    let Some(label_text) = block_label_text_for_block(block) else {
        return;
    };
    let Some(reference_target) = block_reference_target_for_block(block) else {
        return;
    };
    let Ok(reference_nodes) = reference_target.query_selector_all(".ref-var, .ref-var *") else {
        return;
    };
    let text_rect = label_text.get_bounding_client_rect();
    let block_rect = block.get_bounding_client_rect();
    let mut reference_right = f64::NEG_INFINITY;
    let mut reference_bottom = f64::NEG_INFINITY;
    for index in 0..reference_nodes.length() {
        let Some(node) = reference_nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let rect = element.get_bounding_client_rect();
        reference_right = reference_right.max(rect.right());
        reference_bottom = reference_bottom.max(rect.bottom());
    }
    if !reference_right.is_finite() || !reference_bottom.is_finite() {
        return;
    }
    let reference_right_in_block = reference_right - block_rect.left();
    if reference_right_in_block <= 0.0 {
        return;
    }
    let reference_fit_width = reference_right_in_block + BLOCK_REFERENCE_LABEL_GAP_PX;
    let overlap_fit_width = if reference_bottom > text_rect.top() {
        let desired_text_left = reference_right + BLOCK_REFERENCE_LABEL_GAP_PX;
        if desired_text_left > text_rect.left() {
            (reference_right_in_block + BLOCK_REFERENCE_LABEL_GAP_PX) * 2.0 + text_rect.width()
        } else {
            0.0
        }
    } else {
        0.0
    };
    let current_width = block_rect.width();
    let fit_width = current_width
        .max(reference_fit_width)
        .max(overlap_fit_width);
    if !fit_width.is_finite() || fit_width <= current_width {
        return;
    }
    let _ = block_html
        .style()
        .set_property("--block-reference-fit-width", &format!("{fit_width:.2}px"));
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn block_label_text_for_block(block: &web_sys::Element) -> Option<web_sys::Element> {
    block.query_selector(".block-label-text").ok().flatten()
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn block_reference_target_for_block(block: &web_sys::Element) -> Option<web_sys::Element> {
    block.query_selector(".block-ref-target").ok().flatten()
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn position_dictionary_tooltip_from_event(event: &web_sys::Event) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    if !topbar_styles_ready(&document) {
        return;
    }
    let Some(target) = event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
    else {
        return;
    };
    let Ok(Some(host)) = target.closest(".dictionary-tooltip-host, .reference-tooltip-host") else {
        return;
    };
    activate_dictionary_tooltip_host(&host);
    position_dictionary_tooltip(&host);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn activate_dictionary_tooltip_host(active_host: &web_sys::Element) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(hosts) =
        document.query_selector_all(".dictionary-tooltip-host, .reference-tooltip-host")
    else {
        return;
    };
    for index in 0..hosts.length() {
        let Some(node) = hosts.item(index) else {
            continue;
        };
        let Ok(host) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if js_sys::Object::is(host.as_ref(), active_host.as_ref()) {
            clear_dictionary_tooltip_immediate_hide(&host);
        } else {
            hide_dictionary_tooltip_immediately(&host);
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn hide_dictionary_tooltip_immediately(host: &web_sys::Element) {
    let Some(tooltip) = dictionary_tooltip_element_for_host(host) else {
        return;
    };
    let style = tooltip.style();
    let _ = tooltip.remove_attribute("data-jbotci-position-ready");
    let _ = style.set_property("visibility", "hidden");
    let _ = style.set_property("pointer-events", "none");
    let _ = style.set_property("transition", "none");
    let _ = style.remove_property("transform");
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn clear_dictionary_tooltip_immediate_hide(host: &web_sys::Element) {
    let Some(tooltip) = dictionary_tooltip_element_for_host(host) else {
        return;
    };
    let style = tooltip.style();
    let _ = tooltip.remove_attribute("data-jbotci-position-ready");
    let _ = style.remove_property("visibility");
    let _ = style.remove_property("pointer-events");
    let _ = style.remove_property("transform");
    let _ = style.remove_property("transition");
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn dictionary_tooltip_element_for_host(host: &web_sys::Element) -> Option<web_sys::HtmlElement> {
    host.query_selector(".rich-reference-tooltip-stack")
        .ok()
        .flatten()
        .or_else(|| {
            host.query_selector(".rich-dictionary-tooltip")
                .ok()
                .flatten()
        })
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn position_dictionary_tooltip(host: &web_sys::Element) {
    let Some(tooltip_html) = dictionary_tooltip_element_for_host(host) else {
        return;
    };
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let _ = tooltip_html.remove_attribute("data-jbotci-position-ready");
    let host_rect = host.get_bounding_client_rect();
    let tooltip_rect = tooltip_html.get_bounding_client_rect();
    let viewport_width = window
        .inner_width()
        .ok()
        .and_then(|width| width.as_f64())
        .unwrap_or(1.0);
    let viewport_height = window
        .inner_height()
        .ok()
        .and_then(|height| height.as_f64())
        .unwrap_or(1.0);
    let viewport_top = dictionary_tooltip_visible_top(&document);
    let position = dictionary_tooltip_position(
        ReferenceRect {
            left: host_rect.left(),
            top: host_rect.top(),
            right: host_rect.right(),
            bottom: host_rect.bottom(),
        },
        ElementSize {
            width: tooltip_rect.width(),
            height: tooltip_rect.height(),
        },
        new!(TooltipViewport {
            top: viewport_top,
            width: viewport_width,
            height: viewport_height,
        }),
    );
    let style = tooltip_html.style();
    let _ = style.set_property(
        "--dictionary-tooltip-left",
        &format!("{:.2}px", position.left),
    );
    let _ = style.set_property(
        "--dictionary-tooltip-top",
        &format!("{:.2}px", position.top),
    );
    let _ = style.set_property("left", &format!("{:.2}px", position.left));
    let _ = style.set_property("top", &format!("{:.2}px", position.top));
    let _ = tooltip_html.set_attribute("data-jbotci-position-ready", "true");
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0.0)]
fn dictionary_tooltip_visible_top(document: &web_sys::Document) -> f64 {
    let topbar_bottom = document
        .query_selector(".app-topbar")
        .ok()
        .flatten()
        .map(|element| element.get_bounding_client_rect().bottom())
        .unwrap_or(0.0);
    let app_scroll_top = document
        .query_selector("[data-app-scroll='main']")
        .ok()
        .flatten()
        .map(|element| element.get_bounding_client_rect().top())
        .unwrap_or(0.0);
    topbar_bottom.max(app_scroll_top).max(0.0)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
struct DesktopTooltipMeasure {
    id: String,
    host_rect: ReferenceRect,
    tooltip_size: ElementSize,
    viewport: TooltipViewport,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[invariant(true)]
struct DesktopTooltipPlacement {
    id: String,
    left: f64,
    top: f64,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn install_desktop_tooltip_bridge() {
    spawn(async move {
        let mut eval = document::eval(
            r#"
            let nextTooltipId = 1;
            const hostSelector = ".dictionary-tooltip-host, .reference-tooltip-host";
            const stylesReady = () => {
                const shell = document.querySelector(".spa-shell.app-page");
                if (!shell) {
                    return false;
                }
                const shellStyle = window.getComputedStyle(shell);
                return String(shellStyle.getPropertyValue("--topbar-bg") || "").trim().length > 0;
            };
            const tooltipForHost = (host) => {
                for (const child of Array.from(host.children)) {
                    if (
                        child.classList &&
                        (child.classList.contains("rich-reference-tooltip-stack") ||
                            child.classList.contains("rich-dictionary-tooltip"))
                    ) {
                        return child;
                    }
                }
                return host.querySelector(".rich-reference-tooltip-stack, .rich-dictionary-tooltip");
            };
            const rectFor = (element) => {
                const rect = element.getBoundingClientRect();
                return {
                    left: rect.left,
                    top: rect.top,
                    right: rect.right,
                    bottom: rect.bottom,
                };
            };
            const rectTop = (selector) => {
                const element = document.querySelector(selector);
                return element ? element.getBoundingClientRect().top : 0;
            };
            const rectBottom = (selector) => {
                const element = document.querySelector(selector);
                return element ? element.getBoundingClientRect().bottom : 0;
            };
            const visibleViewportTop = () => Math.max(
                0,
                rectBottom(".app-topbar"),
                rectTop("[data-app-scroll='main']"),
            );
            const hideInactiveTooltip = (host) => {
                const tooltip = tooltipForHost(host);
                if (!tooltip) {
                    return;
                }
                tooltip.removeAttribute("data-jbotci-position-ready");
                tooltip.style.setProperty("visibility", "hidden");
                tooltip.style.setProperty("pointer-events", "none");
                tooltip.style.setProperty("transition", "none");
                tooltip.style.removeProperty("transform");
            };
            const activateHost = (activeHost) => {
                for (const host of Array.from(document.querySelectorAll(hostSelector))) {
                    const tooltip = tooltipForHost(host);
                    if (!tooltip) {
                        continue;
                    }
                    if (host === activeHost) {
                        tooltip.removeAttribute("data-jbotci-position-ready");
                        tooltip.style.removeProperty("visibility");
                        tooltip.style.removeProperty("pointer-events");
                        tooltip.style.removeProperty("transform");
                        tooltip.style.removeProperty("transition");
                    } else {
                        hideInactiveTooltip(host);
                    }
                }
            };
            const hostForId = (id) => Array.from(document.querySelectorAll(hostSelector)).find(
                (host) => host.dataset.jbotciTooltipId === String(id),
            );
            const measureHost = (target) => {
                if (!stylesReady()) {
                    return;
                }
                const element = target instanceof Element ? target : target && target.parentElement;
                const host = element && element.closest ? element.closest(hostSelector) : null;
                if (!host) {
                    return;
                }
                if (!host.dataset.jbotciTooltipId) {
                    host.dataset.jbotciTooltipId = String(nextTooltipId++);
                }
                const tooltip = tooltipForHost(host);
                if (!tooltip) {
                    return;
                }
                activateHost(host);
                const tooltipRect = tooltip.getBoundingClientRect();
                dioxus.send({
                    id: host.dataset.jbotciTooltipId,
                    host_rect: rectFor(host),
                    tooltip_size: {
                        width: tooltipRect.width,
                        height: tooltipRect.height,
                    },
                    viewport: {
                        top: visibleViewportTop(),
                        width: Number(window.innerWidth || 1),
                        height: Number(window.innerHeight || 1),
                    },
                });
            };
            const scheduleMeasure = (event) => {
                const target = event.target;
                requestAnimationFrame(() => requestAnimationFrame(() => measureHost(target)));
            };
            document.addEventListener("mouseover", scheduleMeasure, true);
            document.addEventListener("focusin", scheduleMeasure, true);
            document.addEventListener("click", scheduleMeasure, true);
            (async () => {
                while (true) {
                    const placement = await dioxus.recv();
                    const host = hostForId(placement.id);
                    if (!host) {
                        continue;
                    }
                    const tooltip = tooltipForHost(host);
                    if (!tooltip) {
                        continue;
                    }
                    const left = `${Number(placement.left).toFixed(2)}px`;
                    const top = `${Number(placement.top).toFixed(2)}px`;
                    tooltip.style.setProperty("--dictionary-tooltip-left", left);
                    tooltip.style.setProperty("--dictionary-tooltip-top", top);
                    tooltip.style.setProperty("left", left);
                    tooltip.style.setProperty("top", top);
                    tooltip.setAttribute("data-jbotci-position-ready", "true");
                }
            })();
            await new Promise(() => {});
            "#,
        );
        while let Ok(measure) = eval.recv::<DesktopTooltipMeasure>().await {
            let position = dictionary_tooltip_position(
                measure.host_rect,
                measure.tooltip_size,
                measure.viewport,
            );
            let _ = eval.send(DesktopTooltipPlacement {
                id: measure.id,
                left: position.left,
                top: position.top,
            });
        }
    });
}

#[requires(true)]
#[ensures(true)]
fn strip_base_path_for_client(path: &str, base_path: &str) -> Option<String> {
    let normalized = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    };
    let base = base_path.trim_end_matches('/');
    if base.is_empty() || base == "/" {
        Some(normalized)
    } else if normalized == base {
        Some("/".to_owned())
    } else {
        normalized
            .strip_prefix(&format!("{base}/"))
            .map(|rest| format!("/{rest}"))
    }
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn is_app_route_path_for_client(path: &str) -> bool {
    let path = path.trim_end_matches('/');
    path.is_empty()
        || path == "/"
        || path == "/gentufa"
        || path.starts_with("/gentufa/")
        || path == "/cukta"
        || path.starts_with("/cukta/")
        || path == "/vlacku"
        || path.starts_with("/vlacku/")
        || path == "/settings"
        || path.starts_with("/settings/")
}

#[requires(true)]
#[ensures(true)]
fn split_href(href: &str) -> (&str, &str, Option<&str>) {
    let (without_hash, hash) = href
        .split_once('#')
        .map(|(before, after)| (before, Some(after)))
        .unwrap_or((href, None));
    let (path, query) = without_hash
        .split_once('?')
        .map(|(path, query)| (path, query))
        .unwrap_or((without_hash, ""));
    (path, query, hash)
}

#[requires(true)]
#[ensures(true)]
fn logical_app_path_for_client(path: &str, base_path: &str) -> Option<String> {
    if let Some(logical_path) = strip_base_path_for_client(path, base_path)
        && is_app_route_path_for_client(&logical_path)
    {
        return Some(logical_path);
    }
    let normalized = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    };
    if is_app_route_path_for_client(&normalized) {
        Some(normalized)
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn jbotci_route_from_href(base_path: &str, href: &str) -> Option<JbotciRoute> {
    let trimmed = href.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("mailto:")
        || trimmed.starts_with("javascript:")
        || trimmed.starts_with("//")
        || trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
    {
        return None;
    }
    if !trimmed.starts_with('/') {
        return None;
    }
    let (path, query, hash) = split_href(trimmed);
    let logical_path = logical_app_path_for_client(path, base_path)?;
    let web_route = parse_web_route(&logical_path, query);
    let app_route = app_route_for_web_route(&web_route);
    Some(new!(JbotciRoute {
        gentufa_text_explicit: app_route == AppRoute::Gentufa && query_has_key(query, "text"),
        settings_query: if app_route == AppRoute::Settings {
            query.trim_start_matches('?').to_owned()
        } else {
            String::new()
        },
        hash: hash
            .map(|hash| hash.trim_start_matches('#').to_owned())
            .filter(|hash| !hash.is_empty()),
        web_route,
    }))
}

#[requires(true)]
#[ensures(true)]
fn jbotci_route_from_dioxus_route(raw: &str) -> Option<JbotciRoute> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return jbotci_route_from_href("", "/");
    }
    if trimmed.starts_with('/') {
        jbotci_route_from_href("", trimmed)
    } else {
        let href = format!("/{trimmed}");
        jbotci_route_from_href("", &href)
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(true)]
#[ensures(true)]
fn apply_web_route_to_client_state(
    location: &JbotciRoute,
    is_local_route_write: bool,
    mut route: Signal<AppRoute>,
    mut cukta_draft_state: Signal<CuktaWebState>,
    mut cukta_committed_state: Signal<CuktaWebState>,
    mut vlacku_draft_state: Signal<VlackuWebState>,
    mut vlacku_committed_state: Signal<VlackuWebState>,
    mut input_text: Signal<String>,
    mut parsed_text: Signal<String>,
    mut parsed_text_explicit: Signal<bool>,
    mut dialect: Signal<String>,
    mut parsed_dialect: Signal<String>,
    mut view_mode: Signal<GentufaWebViewMode>,
    mut gentufa_display: Signal<GentufaDisplayState>,
) {
    let web_route = &location.web_route;
    let action = route_location_sync_action(location, is_local_route_write);
    set_app_route_if_changed(&mut route, action.app_route);
    if !action.hydrate_route_bound_state {
        return;
    }
    clear_route_bound_input_timers();
    match web_route {
        WebRoute::Gentufa(state) => {
            let input = state.text.clone();
            let parsed = if state.text.is_empty() && !location.gentufa_text_explicit {
                DEFAULT_GENTUFA_TEXT.to_owned()
            } else {
                state.text.clone()
            };
            let dialect_text = state.dialect.clone().unwrap_or_default();
            input_text.set(input);
            parsed_text.set(parsed);
            parsed_text_explicit.set(location.gentufa_text_explicit);
            dialect.set(dialect_text.clone());
            parsed_dialect.set(dialect_text);
            view_mode.set(state.view_mode);
            gentufa_display.set(GentufaDisplayState {
                show_elided: state.show_elided,
                show_glosses: state.show_glosses,
            });
        }
        WebRoute::Cukta(state) => {
            clear_cukta_search_timer();
            cukta_draft_state.set(state.clone());
            cukta_committed_state.set(state.clone());
        }
        WebRoute::Vlacku(state) => {
            clear_vlacku_url_timer();
            clear_vlacku_search_timer();
            vlacku_draft_state.set(state.clone());
            vlacku_committed_state.set(state.clone());
        }
        WebRoute::Settings => {}
    }
}

#[requires(true)]
#[ensures(ret.app_route == location.app_route())]
#[ensures(ret.hydrate_route_bound_state == !is_local_route_write)]
fn route_location_sync_action(
    location: &JbotciRoute,
    is_local_route_write: bool,
) -> RouteLocationSyncAction {
    RouteLocationSyncAction {
        app_route: location.app_route(),
        hydrate_route_bound_state: !is_local_route_write,
    }
}

#[requires(true)]
#[ensures(ret == (current != next))]
fn app_route_update_needed(current: AppRoute, next: AppRoute) -> bool {
    current != next
}

#[requires(true)]
#[ensures(true)]
fn set_app_route_if_changed(route: &mut Signal<AppRoute>, next: AppRoute) {
    let current = *route.read();
    if app_route_update_needed(current, next) {
        route.set(next);
    }
}

#[requires(!key.is_empty())]
#[ensures(true)]
fn query_has_key(query: &str, key: &str) -> bool {
    query
        .trim_start_matches('?')
        .split('&')
        .filter(|part| !part.is_empty())
        .any(|part| {
            part.split_once('=')
                .map_or(part == key, |(candidate, _)| candidate == key)
        })
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn current_hash() -> Option<String> {
    web_sys::window()
        .and_then(|window| window.location().hash().ok())
        .filter(|hash| !hash.is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.is_none())]
fn current_hash() -> Option<String> {
    None
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|target| target.contains('#')))]
fn cukta_hash_scroll_target(
    path: &str,
    query: &str,
    hash: Option<&str>,
    route: AppRoute,
) -> Option<String> {
    let hash = hash?.trim_start_matches('#');
    if route != AppRoute::Cukta || hash.is_empty() {
        return None;
    }
    Some(format!("{path}{query}#{hash}"))
}

#[requires(true)]
#[ensures(true)]
fn current_cukta_pending_scroll(route: &JbotciRoute) -> Option<CuktaPendingScroll> {
    cukta_hash_scroll_target(
        &current_path(),
        &current_query(),
        current_hash().as_deref(),
        route.app_route(),
    )
    .map(cukta_anchor_pending_scroll)
}

#[requires(true)]
#[ensures(true)]
fn cukta_anchor_pending_scroll(target: String) -> CuktaPendingScroll {
    CuktaPendingScroll {
        mode: CuktaPendingScrollMode::Anchor,
        target,
    }
}

#[requires(true)]
#[ensures(true)]
fn cukta_stored_pending_scroll(target: String) -> CuktaPendingScroll {
    CuktaPendingScroll {
        mode: CuktaPendingScrollMode::Stored,
        target,
    }
}

#[requires(true)]
#[ensures(true)]
fn cukta_top_pending_scroll() -> CuktaPendingScroll {
    CuktaPendingScroll {
        mode: CuktaPendingScrollMode::Top,
        target: String::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn cukta_pending_scroll_for_navigation(
    route: AppRoute,
    target: &str,
    has_hash: bool,
    restore_stored: bool,
) -> Option<CuktaPendingScroll> {
    if route != AppRoute::Cukta {
        return None;
    }
    if has_hash {
        Some(cukta_anchor_pending_scroll(target.to_owned()))
    } else if restore_stored {
        Some(cukta_stored_pending_scroll(target.to_owned()))
    } else {
        Some(cukta_top_pending_scroll())
    }
}

#[requires(true)]
#[ensures(true)]
fn cukta_pending_scroll_for_route_change(
    base_path: &str,
    route: &JbotciRoute,
) -> Option<CuktaPendingScroll> {
    if route.app_route() != AppRoute::Cukta {
        return None;
    }
    let target = route_href_with_base_path(base_path, route);
    Some(cukta_stored_pending_scroll(target))
}

#[requires(route.app_route() == AppRoute::Cukta)]
#[ensures(matches!(ret.mode, CuktaPendingScrollMode::Anchor) == route.hash.is_some())]
fn cukta_pending_scroll_for_route_link(base_path: &str, route: &JbotciRoute) -> CuktaPendingScroll {
    if route.hash.is_some() {
        cukta_anchor_pending_scroll(route_href_with_base_path(base_path, route))
    } else {
        cukta_top_pending_scroll()
    }
}

#[requires(true)]
#[ensures(route.app_route() == AppRoute::Cukta -> ret.is_some())]
#[ensures(route.app_route() != AppRoute::Cukta -> ret.is_none())]
fn cukta_pending_scroll_for_explicit_route_link(
    base_path: &str,
    route: &JbotciRoute,
) -> Option<CuktaPendingScroll> {
    if route.app_route() == AppRoute::Cukta {
        Some(cukta_pending_scroll_for_route_link(base_path, route))
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn push_route_with_cukta_scroll_intent(
    mut pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    pending_scroll: Option<CuktaPendingScroll>,
    route: JbotciRoute,
) {
    if let Some(scroll) = pending_scroll {
        pending_cukta_scroll.set(Some(scroll));
    }
    let _ = navigator().push(route);
}

#[requires(true)]
#[ensures(!ret || page.state.as_ref().is_some_and(|page_state| page_state == state))]
#[ensures(!ret || !page.loading)]
#[ensures(!ret || page.error.is_none())]
fn cukta_page_ready_for_scroll(page: &CuktaAsyncPageState, state: &CuktaWebState) -> bool {
    page.state
        .as_ref()
        .is_some_and(|page_state| page_state == state)
        && !page.loading
        && page.error.is_none()
}

#[requires(true)]
#[ensures(true)]
fn apply_cukta_pending_scroll(scroll: CuktaPendingScroll) {
    match scroll.mode {
        CuktaPendingScrollMode::Anchor => scroll_to_cukta_href(&scroll.target),
        CuktaPendingScrollMode::Stored => restore_scroll_for_url(&scroll.target),
        CuktaPendingScrollMode::Top => scroll_to_top(),
    }
}

#[requires(true)]
#[ensures(ret.starts_with("jbotci.scroll."))]
fn scroll_storage_key(path_query_or_url: &str) -> String {
    let (path, query, _) = split_href(path_query_or_url);
    if query.is_empty() {
        format!("jbotci.scroll.{path}")
    } else {
        format!("jbotci.scroll.{path}?{query}")
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(!selector.is_empty())]
#[ensures(true)]
fn scroll_container_by_selector(selector: &str) -> Option<web_sys::HtmlElement> {
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.query_selector(selector).ok().flatten())
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn scroll_container_is_scrollable(element: &web_sys::HtmlElement) -> bool {
    element.scroll_height() > element.client_height()
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn cukta_scroll_container() -> Option<web_sys::HtmlElement> {
    scroll_container_by_selector("[data-cukta-scroll='main']")
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn active_scroll_container() -> Option<web_sys::HtmlElement> {
    cukta_scroll_container()
        .filter(scroll_container_is_scrollable)
        .or_else(|| {
            scroll_container_by_selector("[data-app-scroll='main']")
                .filter(scroll_container_is_scrollable)
        })
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0.0)]
fn element_scroll_margin_top(element: &web_sys::Element) -> f64 {
    web_sys::window()
        .and_then(|window| window.get_computed_style(element).ok().flatten())
        .and_then(|style| style.get_property_value("scroll-margin-top").ok())
        .and_then(|value| value.trim().strip_suffix("px")?.parse::<f64>().ok())
        .unwrap_or(0.0)
        .max(0.0)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn scroll_container_to_y(y: i32) {
    if let Some(element) = active_scroll_container() {
        element.set_scroll_top(y.max(0));
    } else if let Some(window) = web_sys::window() {
        window.scroll_to_with_x_and_y(0.0, f64::from(y.max(0)));
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_scroll_container_to_y(y: i32) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move || scroll_container_to_y(y));
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        30,
    );
    closure.forget();
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn scroll_to_cukta_anchor_element(element: &web_sys::Element) {
    let Some(container) = cukta_scroll_container().or_else(active_scroll_container) else {
        element.scroll_into_view();
        return;
    };
    let container_rect = container.get_bounding_client_rect();
    let element_rect = element.get_bounding_client_rect();
    let next_scroll_top = f64::from(container.scroll_top()) + element_rect.top()
        - container_rect.top()
        - element_scroll_margin_top(element);
    container.set_scroll_top(next_scroll_top.round().max(0.0) as i32);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn save_current_scroll_position() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let location = window.location();
    let key = scroll_storage_key(&format!(
        "{}{}",
        location.pathname().unwrap_or_default(),
        location.search().unwrap_or_default()
    ));
    let y = current_scroll_y();
    session_storage_set(&key, &y.to_string());
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn save_current_scroll_position() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn restore_scroll_for_current_url() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let location = window.location();
    restore_scroll_for_url(&format!(
        "{}{}",
        location.pathname().unwrap_or_default(),
        location.search().unwrap_or_default()
    ));
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn restore_scroll_for_current_url() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0)]
fn current_scroll_y() -> i32 {
    active_scroll_container()
        .map(|element| element.scroll_top().max(0))
        .unwrap_or_else(|| {
            web_sys::window()
                .and_then(|window| window.scroll_y().ok())
                .unwrap_or(0.0)
                .round()
                .max(0.0) as i32
        })
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret == 0)]
fn current_scroll_y() -> i32 {
    0
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn scroll_to_top() {
    schedule_scroll_container_to_y(0);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn scroll_to_top() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn restore_scroll_for_url(url: &str) {
    let key = scroll_storage_key(url);
    let Some(raw) = session_storage_get(&key) else {
        scroll_container_to_y(0);
        return;
    };
    let Ok(y) = raw.parse::<i32>() else {
        return;
    };
    schedule_scroll_container_to_y(y);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn restore_scroll_for_url(url: &str) {
    let _ = url;
}

#[requires(true)]
#[ensures(ret.mode == state.mode)]
#[ensures(ret.query == state.query)]
#[ensures(ret.word_types == state.word_types)]
#[ensures(ret.count >= 1 && ret.count <= VLACKU_WEB_MAX_COUNT)]
fn vlacku_load_more_state(state: &VlackuWebState) -> VlackuWebState {
    let mut next = state.clone();
    next.count = next.count.saturating_mul(2).clamp(1, VLACKU_WEB_MAX_COUNT);
    next
}

#[requires(match anchor_viewport_top { Some(top) => top.is_finite(), None => true })]
#[requires(scroll_top >= 0)]
#[requires(fallback_top.is_finite())]
#[requires(topbar_bottom.is_finite())]
#[ensures(ret.is_finite())]
#[ensures(ret >= topbar_bottom)]
fn stable_jvozba_pane_top(
    anchor_viewport_top: Option<f64>,
    scroll_top: i32,
    fallback_top: f64,
    topbar_bottom: f64,
) -> f64 {
    anchor_viewport_top
        .map(|top| top + f64::from(scroll_top))
        .unwrap_or(fallback_top)
        .max(topbar_bottom)
}

#[requires(true)]
#[ensures(true)]
fn set_vlacku_state_immediate(
    draft_state: &mut Signal<VlackuWebState>,
    committed_state: &mut Signal<VlackuWebState>,
    state: VlackuWebState,
) {
    clear_vlacku_url_timer();
    clear_vlacku_search_timer();
    draft_state.set(state.clone());
    committed_state.set(state);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_vlacku_search_commit(
    mut committed_state: Signal<VlackuWebState>,
    state: VlackuWebState,
) {
    let Some(window) = web_sys::window() else {
        committed_state.set(state);
        return;
    };
    clear_vlacku_url_timer();
    clear_vlacku_search_timer();
    let closure = Closure::once(move || {
        committed_state.set(state);
    });
    if let Ok(handle) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        VLACKU_SEARCH_DEBOUNCE_MS,
    ) {
        VLACKU_SEARCH_TIMER.with(|timer| timer.set(Some(handle)));
        closure.forget();
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn schedule_vlacku_search_commit(
    mut committed_state: Signal<VlackuWebState>,
    state: VlackuWebState,
) {
    let _ = VLACKU_SEARCH_DEBOUNCE_MS;
    clear_vlacku_url_timer();
    committed_state.set(state);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_cukta_search_commit(mut committed_state: Signal<CuktaWebState>, state: CuktaWebState) {
    let Some(window) = web_sys::window() else {
        committed_state.set(state);
        return;
    };
    clear_cukta_search_timer();
    let closure = Closure::once(move || {
        committed_state.set(state);
    });
    if let Ok(handle) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        CUKTA_SEARCH_DEBOUNCE_MS,
    ) {
        CUKTA_SEARCH_TIMER.with(|timer| timer.set(Some(handle)));
        closure.forget();
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn schedule_cukta_search_commit(mut committed_state: Signal<CuktaWebState>, state: CuktaWebState) {
    let _ = CUKTA_SEARCH_DEBOUNCE_MS;
    committed_state.set(state);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn clear_vlacku_search_timer() {
    let Some(window) = web_sys::window() else {
        return;
    };
    VLACKU_SEARCH_TIMER.with(|timer| {
        if let Some(handle) = timer.replace(None) {
            window.clear_timeout_with_handle(handle);
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn clear_vlacku_search_timer() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn clear_cukta_search_timer() {
    let Some(window) = web_sys::window() else {
        return;
    };
    CUKTA_SEARCH_TIMER.with(|timer| {
        if let Some(handle) = timer.replace(None) {
            window.clear_timeout_with_handle(handle);
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn clear_cukta_search_timer() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn clear_vlacku_url_timer() {
    let Some(window) = web_sys::window() else {
        return;
    };
    VLACKU_URL_TIMER.with(|timer| {
        if let Some(handle) = timer.replace(None) {
            window.clear_timeout_with_handle(handle);
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn clear_vlacku_url_timer() {}

#[requires(true)]
#[ensures(true)]
fn clear_route_bound_input_timers() {
    clear_vlacku_url_timer();
    clear_vlacku_search_timer();
    clear_cukta_search_timer();
}

#[requires(true)]
#[ensures(true)]
fn schedule_vlacku_url_push(
    history: Rc<dyn History>,
    pending_writes: Signal<PendingLocalRouteWrites>,
    current: &JbotciRoute,
    state: &VlackuWebState,
    restore_scroll_y: Option<i32>,
) {
    let target = JbotciRoute::from_web_route(WebRoute::Vlacku(state.clone()), false);
    if current.without_hash() == target {
        return;
    }
    schedule_route_push(
        history,
        pending_writes,
        target,
        VLACKU_URL_DEBOUNCE_MS,
        restore_scroll_y,
    );
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_route_push(
    history: Rc<dyn History>,
    pending_writes: Signal<PendingLocalRouteWrites>,
    target: JbotciRoute,
    delay_ms: i32,
    restore_scroll_y: Option<i32>,
) {
    let Some(window) = web_sys::window() else {
        return;
    };
    clear_vlacku_url_timer();
    let closure = Closure::once(move || {
        let mut pending_writes = pending_writes;
        pending_writes.with_mut(|pending| pending.record(&target));
        history.push(route_path_for_route(&target));
        if let Some(y) = restore_scroll_y {
            schedule_scroll_container_to_y(y);
        }
    });
    if let Ok(handle) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        delay_ms,
    ) {
        VLACKU_URL_TIMER.with(|timer| timer.set(Some(handle)));
        closure.forget();
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn schedule_route_push(
    history: Rc<dyn History>,
    mut pending_writes: Signal<PendingLocalRouteWrites>,
    target: JbotciRoute,
    delay_ms: i32,
    restore_scroll_y: Option<i32>,
) {
    let _ = (delay_ms, restore_scroll_y);
    pending_writes.with_mut(|pending| pending.record(&target));
    history.push(route_path_for_route(&target));
}

#[requires(true)]
#[ensures(ret.app_route() == AppRoute::Gentufa)]
#[ensures(ret.gentufa_text_explicit == text_explicit)]
fn gentufa_route_for_committed_state(state: &GentufaWebState, text_explicit: bool) -> JbotciRoute {
    JbotciRoute::from_web_route(WebRoute::Gentufa(state.clone()), text_explicit)
}

#[requires(true)]
#[ensures(ret == (active_route == AppRoute::Gentufa && current_route.app_route() == AppRoute::Gentufa))]
fn gentufa_url_sync_allowed(active_route: AppRoute, current_route: &JbotciRoute) -> bool {
    active_route == AppRoute::Gentufa && current_route.app_route() == AppRoute::Gentufa
}

#[requires(true)]
#[ensures((current.without_hash() == *target) == (ret == GentufaUrlHistoryAction::NoWrite))]
fn gentufa_url_history_action(
    current: &JbotciRoute,
    target: &JbotciRoute,
    intent: GentufaUrlWriteIntent,
) -> GentufaUrlHistoryAction {
    if current.without_hash() == *target {
        GentufaUrlHistoryAction::NoWrite
    } else {
        match intent {
            GentufaUrlWriteIntent::ReplaceCurrent => GentufaUrlHistoryAction::ReplaceCurrent,
            GentufaUrlWriteIntent::PushParse => GentufaUrlHistoryAction::PushParse,
        }
    }
}

#[requires(true)]
#[ensures(action == GentufaUrlHistoryAction::NoWrite -> ret == GentufaUrlWriteIntent::ReplaceCurrent)]
#[ensures(action != GentufaUrlHistoryAction::NoWrite -> ret == intent)]
fn gentufa_url_intent_after_sync_action(
    intent: GentufaUrlWriteIntent,
    action: GentufaUrlHistoryAction,
) -> GentufaUrlWriteIntent {
    match action {
        GentufaUrlHistoryAction::NoWrite => GentufaUrlWriteIntent::ReplaceCurrent,
        GentufaUrlHistoryAction::ReplaceCurrent | GentufaUrlHistoryAction::PushParse => intent,
    }
}

#[requires(true)]
#[ensures(true)]
fn set_gentufa_url_write_intent_if_changed(
    intent: &mut Signal<GentufaUrlWriteIntent>,
    current: GentufaUrlWriteIntent,
    next: GentufaUrlWriteIntent,
) {
    if current != next {
        intent.set(next);
    }
}

#[requires(true)]
#[ensures(true)]
fn sync_gentufa_committed_url(
    history: Rc<dyn History>,
    mut pending_writes: Signal<PendingLocalRouteWrites>,
    current: &JbotciRoute,
    state: &GentufaWebState,
    text_explicit: bool,
    write_intent: GentufaUrlWriteIntent,
    mut intent_signal: Signal<GentufaUrlWriteIntent>,
) {
    let target = gentufa_route_for_committed_state(state, text_explicit);
    let action = gentufa_url_history_action(current, &target, write_intent);
    match action {
        GentufaUrlHistoryAction::NoWrite => {}
        GentufaUrlHistoryAction::ReplaceCurrent => {
            pending_writes.with_mut(|pending| pending.record(&target));
            history.replace(route_path_for_route(&target));
        }
        GentufaUrlHistoryAction::PushParse => {
            pending_writes.with_mut(|pending| pending.record(&target));
            history.push(route_path_for_route(&target));
        }
    }
    let next_intent = gentufa_url_intent_after_sync_action(write_intent, action);
    set_gentufa_url_write_intent_if_changed(&mut intent_signal, write_intent, next_intent);
}

#[requires(true)]
#[ensures(ret.starts_with('/'))]
fn route_path_for_route(route: &JbotciRoute) -> String {
    route.to_string()
}

#[requires(true)]
#[ensures(route_path_for_route(&ret).starts_with('/'))]
fn canonical_local_route(route: &JbotciRoute) -> JbotciRoute {
    jbotci_route_from_dioxus_route(&route_path_for_route(route)).unwrap_or_else(|| route.clone())
}

#[requires(true)]
#[ensures(true)]
fn push_cukta_url(
    history: Rc<dyn History>,
    mut pending_writes: Signal<PendingLocalRouteWrites>,
    current: &JbotciRoute,
    state: &CuktaWebState,
) {
    let target = JbotciRoute::from_web_route(WebRoute::Cukta(state.clone()), false);
    if current.without_hash() == target {
        return;
    }
    pending_writes.with_mut(|pending| pending.record(&target));
    history.push(route_path_for_route(&target));
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn sync_document_head(meta: &PageMeta) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let head_model = build_page_head(meta);
    if let Ok(nodes) = document.query_selector_all("[data-jbotci-meta='1']") {
        for index in 0..nodes.length() {
            if let Some(node) = nodes.item(index)
                && let Some(parent) = node.parent_node()
            {
                let _ = parent.remove_child(&node);
            }
        }
    }
    let Ok(Some(head)) = document.query_selector("head") else {
        return;
    };
    let canonical_url = absolute_href_for_client(&head_model.canonical_url);
    let manifest_href = absolute_href_for_client(&head_model.manifest_href);
    let icon_href = absolute_href_for_client(&head_model.icon_href);
    let apple_touch_icon_href = absolute_href_for_client(&head_model.apple_touch_icon_href);
    append_meta_name(&document, &head, "application-name", "jbotci");
    append_meta_name(&document, &head, "apple-mobile-web-app-capable", "yes");
    append_meta_name(&document, &head, "apple-mobile-web-app-title", "jbotci");
    append_meta_name(&document, &head, "mobile-web-app-capable", "yes");
    append_meta_name_with_extra(
        &document,
        &head,
        "theme-color",
        &head_model.light_theme_color,
        &[("media", "(prefers-color-scheme: light)")],
    );
    append_meta_name_with_extra(
        &document,
        &head,
        "theme-color",
        &head_model.dark_theme_color,
        &[("media", "(prefers-color-scheme: dark)")],
    );
    append_link(&document, &head, "manifest", &manifest_href);
    append_link(&document, &head, "icon", &icon_href);
    append_link(&document, &head, "shortcut icon", &icon_href);
    append_link(&document, &head, "apple-touch-icon", &apple_touch_icon_href);
    append_meta_name(&document, &head, "description", &head_model.description);
    append_link(&document, &head, "canonical", &canonical_url);
    append_meta_property(&document, &head, "og:title", &head_model.title);
    append_meta_property(&document, &head, "og:description", &head_model.description);
    append_meta_property(&document, &head, "og:type", "website");
    append_meta_property(&document, &head, "og:url", &canonical_url);
    append_meta_name(&document, &head, "twitter:title", &head_model.title);
    append_meta_name(
        &document,
        &head,
        "twitter:description",
        &head_model.description,
    );
    append_meta_name(&document, &head, "twitter:card", &head_model.twitter_card);
    if let Some(image) = &head_model.image {
        let image_url = absolute_href_for_client(&image.href);
        append_meta_property(&document, &head, "og:image", &image_url);
        append_meta_name(&document, &head, "twitter:image", &image_url);
        append_meta_property(&document, &head, "og:image:width", &image.width.to_string());
        append_meta_property(
            &document,
            &head,
            "og:image:height",
            &image.height.to_string(),
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn sync_document_head(meta: &PageMeta) {
    let _ = meta;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn append_meta_name(
    document: &web_sys::Document,
    head: &web_sys::Element,
    name: &str,
    content: &str,
) {
    append_meta_name_with_extra(document, head, name, content, &[]);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn append_meta_name_with_extra(
    document: &web_sys::Document,
    head: &web_sys::Element,
    name: &str,
    content: &str,
    extra: &[(&str, &str)],
) {
    if let Ok(element) = document.create_element("meta") {
        let _ = element.set_attribute("data-jbotci-meta", "1");
        let _ = element.set_attribute("name", name);
        let _ = element.set_attribute("content", content);
        for (key, value) in extra {
            let _ = element.set_attribute(key, value);
        }
        let _ = head.append_child(&element);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn append_meta_property(
    document: &web_sys::Document,
    head: &web_sys::Element,
    property: &str,
    content: &str,
) {
    if let Ok(element) = document.create_element("meta") {
        let _ = element.set_attribute("data-jbotci-meta", "1");
        let _ = element.set_attribute("property", property);
        let _ = element.set_attribute("content", content);
        let _ = head.append_child(&element);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn append_link(document: &web_sys::Document, head: &web_sys::Element, rel: &str, href: &str) {
    if let Ok(element) = document.create_element("link") {
        let _ = element.set_attribute("data-jbotci-meta", "1");
        let _ = element.set_attribute("rel", rel);
        let _ = element.set_attribute("href", href);
        let _ = head.append_child(&element);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn absolute_href_for_client(href: &str) -> String {
    if href.starts_with('/') {
        if let Some(window) = web_sys::window()
            && let Ok(origin) = window.location().origin()
        {
            return format!("{}{}", origin.trim_end_matches('/'), href);
        }
    }
    href.to_owned()
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.starts_with('/'))]
fn current_path() -> String {
    web_sys::window()
        .and_then(|window| window.location().pathname().ok())
        .filter(|path| path.starts_with('/'))
        .unwrap_or_else(|| "/gentufa".to_owned())
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.starts_with('/'))]
fn current_path() -> String {
    "/gentufa".to_owned()
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn set_brivla_toggle_indeterminate(indeterminate: bool) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(Some(element)) = document.query_selector("input[data-brivla-toggle='1']") else {
        return;
    };
    if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
        input.set_indeterminate(indeterminate);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn set_brivla_toggle_indeterminate(_indeterminate: bool) {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_vlacku_jvozba_pane_metrics_sync() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move || {
        sync_vlacku_jvozba_pane_metrics();
        schedule_vlacku_jvozba_pane_metrics_animation_frames(VLACKU_JVOZBA_LAYOUT_FRAME_PASSES);
    });
    if window
        .set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), 0)
        .is_ok()
    {
        closure.forget();
    } else {
        sync_vlacku_jvozba_pane_metrics();
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn schedule_vlacku_jvozba_pane_metrics_sync() {
    spawn(async move {
        sleep_ms(0).await;
        sync_vlacku_jvozba_pane_metrics_desktop().await;
        for _ in 0..VLACKU_JVOZBA_LAYOUT_FRAME_PASSES {
            sleep_ms(16).await;
            sync_vlacku_jvozba_pane_metrics_desktop().await;
        }
    });
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
fn schedule_vlacku_jvozba_pane_metrics_sync() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_vlacku_jvozba_pane_metrics_animation_frames(remaining: u8) {
    if remaining == 0 {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move |_timestamp: f64| {
        sync_vlacku_jvozba_pane_metrics();
        schedule_vlacku_jvozba_pane_metrics_animation_frames(remaining - 1);
    });
    if window
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .is_ok()
    {
        closure.forget();
    } else {
        sync_vlacku_jvozba_pane_metrics();
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_vlacku_jvozba_pane_metrics_after_fonts_ready(document: &web_sys::Document) {
    let Ok(fonts) = js_sys::Reflect::get(document.as_ref(), &JsValue::from_str("fonts")) else {
        return;
    };
    let Ok(ready) = js_sys::Reflect::get(&fonts, &JsValue::from_str("ready")) else {
        return;
    };
    let Ok(promise) = ready.dyn_into::<js_sys::Promise>() else {
        return;
    };
    wasm_bindgen_futures::spawn_local(async move {
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        schedule_vlacku_jvozba_pane_metrics_sync();
    });
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn sync_vlacku_jvozba_pane_metrics() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Ok(Some(pane)) = document.query_selector("[data-jvozba-pane='1']") else {
        return;
    };
    let Some(pane) = pane.dyn_ref::<web_sys::HtmlElement>() else {
        return;
    };
    let topbar_bottom = document
        .query_selector(".app-topbar")
        .ok()
        .flatten()
        .map(|element| element.get_bounding_client_rect().bottom())
        .unwrap_or(0.0);
    let form_bottom = document
        .query_selector(".vlacku-page .dictionary-form .dictionary-query-row")
        .ok()
        .flatten()
        .map(|element| element.get_bounding_client_rect().bottom());
    let anchor_top = document
        .query_selector("[data-jvozba-pane-anchor='1']")
        .ok()
        .flatten()
        .map(|element| element.get_bounding_client_rect().top());
    let viewport_height = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(720.0);
    let app_scroll_container = document
        .query_selector("[data-app-scroll='main']")
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok());
    let app_scroll_top = app_scroll_container
        .as_ref()
        .map(|main| main.scroll_top().max(0))
        .unwrap_or(0);
    let app_scrollbar_gutter_width = app_scroll_container
        .as_ref()
        .map(|main| (main.offset_width() - main.client_width()).max(0))
        .unwrap_or(0);
    let fallback_top = form_bottom.unwrap_or(topbar_bottom).max(topbar_bottom) + 12.0;
    let top = stable_jvozba_pane_top(anchor_top, app_scroll_top, fallback_top, topbar_bottom);
    let bottom = 12.0;
    let height = (viewport_height - top - bottom).max(280.0) * VLACKU_JVOZBA_HEIGHT_SCALE;
    let style = pane.style();
    let _ = style.set_property("--jvozba-pane-top", &format!("{top}px"));
    let _ = style.set_property("--jvozba-pane-bottom", &format!("{bottom}px"));
    let _ = style.set_property("--jvozba-pane-height", &format!("{height}px"));
    let _ = style.set_property(
        "--app-scrollbar-gutter-width",
        &format!("{app_scrollbar_gutter_width}px"),
    );
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn sync_vlacku_jvozba_pane_metrics() {
    spawn(async move {
        sync_vlacku_jvozba_pane_metrics_desktop().await;
    });
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
fn sync_vlacku_jvozba_pane_metrics() {}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[invariant(true)]
struct JvozbaPaneMetrics {
    topbar_bottom: f64,
    form_bottom: Option<f64>,
    anchor_top: Option<f64>,
    viewport_height: f64,
    app_scroll_top: i32,
    app_scrollbar_gutter_width: i32,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[invariant(true)]
struct JvozbaPaneLayout {
    top: f64,
    bottom: f64,
    height: f64,
    app_scrollbar_gutter_width: i32,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.top >= metrics.topbar_bottom)]
fn jvozba_pane_layout_from_metrics(metrics: JvozbaPaneMetrics) -> JvozbaPaneLayout {
    let fallback_top = metrics
        .form_bottom
        .unwrap_or(metrics.topbar_bottom)
        .max(metrics.topbar_bottom)
        + 12.0;
    let top = stable_jvozba_pane_top(
        metrics.anchor_top,
        metrics.app_scroll_top,
        fallback_top,
        metrics.topbar_bottom,
    );
    let bottom = 12.0;
    let height = (metrics.viewport_height - top - bottom).max(280.0) * VLACKU_JVOZBA_HEIGHT_SCALE;
    JvozbaPaneLayout {
        top,
        bottom,
        height,
        app_scrollbar_gutter_width: metrics.app_scrollbar_gutter_width,
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn sync_vlacku_jvozba_pane_metrics_desktop() {
    let Some(metrics) = measure_vlacku_jvozba_pane_metrics_desktop().await else {
        return;
    };
    let layout = jvozba_pane_layout_from_metrics(metrics);
    apply_vlacku_jvozba_pane_layout_desktop(layout).await;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn measure_vlacku_jvozba_pane_metrics_desktop() -> Option<JvozbaPaneMetrics> {
    document::eval(
        r#"
        if (!document.querySelector("[data-jvozba-pane='1']")) {
            return null;
        }
        const rectBottom = (selector) => {
            const element = document.querySelector(selector);
            return element ? element.getBoundingClientRect().bottom : null;
        };
        const rectTop = (selector) => {
            const element = document.querySelector(selector);
            return element ? element.getBoundingClientRect().top : null;
        };
        const appScroll = document.querySelector("[data-app-scroll='main']");
        return {
            topbar_bottom: rectBottom(".app-topbar") ?? 0,
            form_bottom: rectBottom(".vlacku-page .dictionary-form .dictionary-query-row"),
            anchor_top: rectTop("[data-jvozba-pane-anchor='1']"),
            viewport_height: Number(window.innerHeight || 720),
            app_scroll_top: appScroll ? Math.max(0, Number(appScroll.scrollTop || 0)) : 0,
            app_scrollbar_gutter_width: appScroll ? Math.max(0, Number(appScroll.offsetWidth || 0) - Number(appScroll.clientWidth || 0)) : 0,
        };
        "#,
    )
    .join()
    .await
    .ok()
    .flatten()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
async fn apply_vlacku_jvozba_pane_layout_desktop(layout: JvozbaPaneLayout) {
    let Ok(layout_json) = serde_json::to_string(&layout) else {
        return;
    };
    let script = format!(
        r#"
        const layout = {layout_json};
        const pane = document.querySelector("[data-jvozba-pane='1']");
        if (pane) {{
            pane.style.setProperty("--jvozba-pane-top", `${{Number(layout.top).toFixed(2)}}px`);
            pane.style.setProperty("--jvozba-pane-bottom", `${{Number(layout.bottom).toFixed(2)}}px`);
            pane.style.setProperty("--jvozba-pane-height", `${{Number(layout.height).toFixed(2)}}px`);
            pane.style.setProperty("--app-scrollbar-gutter-width", `${{Number(layout.app_scrollbar_gutter_width)}}px`);
        }}
        return null;
        "#
    );
    let _ = document::eval(&script).await;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn measure_vlacku_jvozba_item_height(index: usize) -> Option<usize> {
    let document = web_sys::window()?.document()?;
    let selector = format!("[data-jvozba-item-index='{index}']");
    let element = document.query_selector(&selector).ok().flatten()?;
    Some(element.get_bounding_client_rect().height().round().max(1.0) as usize)
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn measure_vlacku_jvozba_item_height(_index: usize) -> Option<usize> {
    None
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn current_query() -> String {
    web_sys::window()
        .and_then(|window| window.location().search().ok())
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.is_empty())]
fn current_query() -> String {
    String::new()
}

#[requires(true)]
#[ensures(true)]
fn query_param(query: &str, name: &str) -> Option<String> {
    let trimmed = query.strip_prefix('?').unwrap_or(query);
    trimmed
        .split('&')
        .filter(|part| !part.is_empty())
        .find_map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            (percent_decode_query_component(key) == name)
                .then(|| percent_decode_query_component(value))
        })
}

#[requires(true)]
#[ensures(true)]
fn percent_decode_query_component(input: &str) -> String {
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
fn load_settings() -> UserSettings {
    let mut settings = UserSettings::default();
    if let Some(theme) = storage_get("jbotci.theme").and_then(|value| parse_theme(&value)) {
        settings.theme = theme;
    }
    if let Some(script) = storage_get("jbotci.script").and_then(|value| parse_script(&value)) {
        settings.script = script;
    }
    if let Some(stress) =
        storage_get("jbotci.output.stress").and_then(|value| parse_stress_mark(&value))
    {
        settings.stress = stress;
    }
    if let Some(glides) =
        storage_get("jbotci.output.glides").and_then(|value| parse_glide_mark(&value))
    {
        settings.glides = glides;
    }
    settings
}

#[requires(true)]
#[ensures(true)]
fn load_dialect_settings() -> DialectSettings {
    storage_get(DIALECT_SETTINGS_STORAGE_KEY)
        .and_then(|raw| serde_json::from_str::<DialectSettings>(&raw).ok())
        .map(normalize_loaded_dialect_settings)
        .unwrap_or_default()
}

#[requires(true)]
#[ensures(true)]
fn normalize_loaded_dialect_settings(mut settings: DialectSettings) -> DialectSettings {
    settings
        .custom_dialects
        .retain(|custom| !custom.name.trim().is_empty());
    settings
}

#[requires(true)]
#[ensures(true)]
fn save_dialect_settings(settings: &DialectSettings) {
    if let Ok(raw) = serde_json::to_string(settings) {
        storage_set(DIALECT_SETTINGS_STORAGE_KEY, &raw);
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_theme(value: &str) -> Option<ThemeMode> {
    match value {
        "auto" | "system" => Some(ThemeMode::Auto),
        "day" | "light" => Some(ThemeMode::Day),
        "night" | "dark" => Some(ThemeMode::Night),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_stress_mark(value: &str) -> Option<StressMark> {
    match value {
        "none" => Some(StressMark::None),
        "acute" => Some(StressMark::Acute),
        "caps" => Some(StressMark::Caps),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_glide_mark(value: &str) -> Option<GlideMark> {
    match value {
        "none" => Some(GlideMark::None),
        "breve" => Some(GlideMark::Breve),
        _ => None,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn stress_mark_storage_value(mark: StressMark) -> &'static str {
    match mark {
        StressMark::None => "none",
        StressMark::Acute => "acute",
        StressMark::Caps => "caps",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn glide_mark_storage_value(mark: GlideMark) -> &'static str {
    match mark {
        GlideMark::None => "none",
        GlideMark::Breve => "breve",
    }
}

#[requires(true)]
#[ensures(true)]
fn save_settings(settings: &UserSettings) {
    storage_set("jbotci.theme", theme_class(settings.theme));
    storage_set("jbotci.script", script_class(settings.script));
    storage_set(
        "jbotci.output.stress",
        stress_mark_storage_value(settings.stress),
    );
    storage_set(
        "jbotci.output.glides",
        glide_mark_storage_value(settings.glides),
    );
}

#[requires(true)]
#[ensures(true)]
fn load_vlacku_jvozba_pane_state() -> VlackuJvozbaPaneState {
    let open = matches!(
        storage_get("jbotci.vlacku.jvozba.open.v1").as_deref(),
        Some("1" | "true")
    );
    let mode = storage_get("jbotci.vlacku.jvozba.mode.v1")
        .as_deref()
        .and_then(parse_vlacku_jvozba_mode)
        .unwrap_or(VlackuJvozbaMode::Lujvo);
    let items = storage_get("jbotci.vlacku.jvozba.items.v1")
        .map(|raw| parse_vlacku_jvozba_items(&raw))
        .unwrap_or_default();
    VlackuJvozbaPaneState { open, mode, items }
}

#[requires(true)]
#[ensures(true)]
fn save_vlacku_jvozba_pane_state(state: &VlackuJvozbaPaneState) {
    storage_set(
        "jbotci.vlacku.jvozba.open.v1",
        if state.open { "1" } else { "0" },
    );
    storage_set(
        "jbotci.vlacku.jvozba.mode.v1",
        match state.mode {
            VlackuJvozbaMode::Lujvo => "lujvo",
            VlackuJvozbaMode::Cmevla => "cmevla",
        },
    );
    storage_set(
        "jbotci.vlacku.jvozba.items.v1",
        &format_vlacku_jvozba_items(&state.items),
    );
}

#[requires(true)]
#[ensures(true)]
fn parse_vlacku_jvozba_mode(value: &str) -> Option<VlackuJvozbaMode> {
    match value {
        "lujvo" => Some(VlackuJvozbaMode::Lujvo),
        "cmevla" => Some(VlackuJvozbaMode::Cmevla),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn parse_vlacku_jvozba_items(raw: &str) -> Vec<VlackuJvozbaItem> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(items) = value.as_array() {
            return items
                .iter()
                .filter_map(parse_vlacku_jvozba_json_item)
                .collect();
        }
    }
    parse_vlacku_jvozba_legacy_items(raw)
}

#[requires(true)]
#[ensures(true)]
fn parse_vlacku_jvozba_json_item(value: &serde_json::Value) -> Option<VlackuJvozbaItem> {
    let object = value.as_object()?;
    let kind_text = object.get("kind")?.as_str()?;
    let item_kind = match kind_text {
        "word" => VlackuJvozbaItemKind::Word,
        "rafsi" | "fixed-rafsi" => VlackuJvozbaItemKind::FixedRafsi,
        _ => return None,
    };
    let item_value = object.get("value")?.as_str()?.trim();
    if item_value.is_empty() {
        return None;
    }
    let source = object
        .get("source")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let indent_level = object
        .get("indentLevel")
        .or_else(|| object.get("indent_level"))
        .and_then(serde_json::Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(0);
    Some(VlackuJvozbaItem {
        kind: item_kind,
        value: item_value.to_owned(),
        source,
        indent_level,
    })
}

#[requires(true)]
#[ensures(true)]
fn parse_vlacku_jvozba_legacy_items(raw: &str) -> Vec<VlackuJvozbaItem> {
    raw.lines()
        .filter_map(|line| {
            let (kind, value) = line.split_once('\t')?;
            let item_kind = match kind {
                "word" => VlackuJvozbaItemKind::Word,
                "rafsi" => VlackuJvozbaItemKind::FixedRafsi,
                _ => return None,
            };
            (!value.is_empty()).then(|| VlackuJvozbaItem {
                kind: item_kind,
                value: value.to_owned(),
                source: None,
                indent_level: 0,
            })
        })
        .collect()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_vlacku_jvozba_items(items: &[VlackuJvozbaItem]) -> String {
    let values = items
        .iter()
        .map(|item| {
            serde_json::json!({
                "kind": match item.kind {
                    VlackuJvozbaItemKind::Word => "word",
                    VlackuJvozbaItemKind::FixedRafsi => "rafsi",
                },
                "value": item.value.as_str(),
                "source": item.source.as_deref(),
                "indentLevel": item.indent_level,
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&values).unwrap_or_else(|_| "[]".to_owned())
}

#[requires(true)]
#[ensures(true)]
fn parse_script(value: &str) -> Option<GentufaScript> {
    match value {
        "latin" => Some(GentufaScript::Latin),
        "cyrillic" => Some(GentufaScript::Cyrillic),
        "zbalermorna" => Some(GentufaScript::Zbalermorna),
        _ => None,
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(!key.is_empty())]
#[ensures(true)]
fn storage_get(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!key.is_empty())]
#[ensures(true)]
fn storage_get(key: &str) -> Option<String> {
    native_storage_get(key)
}

#[cfg(target_arch = "wasm32")]
#[requires(!key.is_empty())]
#[ensures(true)]
fn storage_set(key: &str, value: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(key, value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!key.is_empty())]
#[ensures(true)]
fn storage_set(key: &str, value: &str) {
    let _ = native_storage_set(key, value);
}

#[cfg(target_arch = "wasm32")]
#[requires(!key.is_empty())]
#[ensures(true)]
fn session_storage_get(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|window| window.session_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!key.is_empty())]
#[ensures(true)]
fn session_storage_get(key: &str) -> Option<String> {
    native_session_storage()
        .lock()
        .ok()
        .and_then(|values| values.get(key).cloned())
}

#[cfg(target_arch = "wasm32")]
#[requires(!key.is_empty())]
#[ensures(true)]
fn session_storage_set(key: &str, value: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.session_storage().ok().flatten())
    {
        let _ = storage.set_item(key, value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!key.is_empty())]
#[ensures(true)]
fn session_storage_set(key: &str, value: &str) {
    if let Ok(mut values) = native_session_storage().lock() {
        values.insert(key.to_owned(), value.to_owned());
    }
}

#[cfg(not(target_arch = "wasm32"))]
static NATIVE_SESSION_STORAGE: OnceLock<Mutex<std::collections::HashMap<String, String>>> =
    OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn native_session_storage() -> &'static Mutex<std::collections::HashMap<String, String>> {
    NATIVE_SESSION_STORAGE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!key.is_empty())]
#[ensures(true)]
fn native_storage_get(key: &str) -> Option<String> {
    native_storage_values().ok()?.get(key).cloned()
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!key.is_empty())]
#[ensures(true)]
fn native_storage_set(key: &str, value: &str) -> Result<(), String> {
    let mut values = native_storage_values()?;
    values.insert(key.to_owned(), value.to_owned());
    write_native_storage_values(&values)
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|values| values.keys().all(|key| !key.is_empty())) || ret.is_err())]
fn native_storage_values() -> Result<std::collections::BTreeMap<String, String>, String> {
    let path = native_storage_path()?;
    if !path.is_file() {
        return Ok(std::collections::BTreeMap::new());
    }
    let raw = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "failed to read native settings `{}`: {error}",
            path.display()
        )
    })?;
    serde_json::from_str(&raw).map_err(|error| {
        format!(
            "failed to parse native settings `{}`: {error}",
            path.display()
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn write_native_storage_values(
    values: &std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    let path = native_storage_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create native settings directory `{}`: {error}",
                parent.display()
            )
        })?;
    }
    let raw = serde_json::to_string_pretty(values)
        .map_err(|error| format!("failed to serialize native settings: {error}"))?;
    std::fs::write(&path, raw).map_err(|error| {
        format!(
            "failed to write native settings `{}`: {error}",
            path.display()
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|path| path.ends_with("ui-settings.json")) || ret.is_err())]
fn native_storage_path() -> Result<std::path::PathBuf, String> {
    let dirs = directories::ProjectDirs::from("org", "int19h", "jbotci")
        .ok_or_else(|| "could not resolve native settings directory".to_owned())?;
    Ok(dirs.config_dir().join("ui-settings.json"))
}

#[requires(true)]
#[ensures(ret.gentufa)]
fn _feature_availability_for_linking() -> WebFeatureAvailability {
    WebFeatureAvailability::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn git_commit_display_uses_math_monospace_hex() {
        assert_eq!(math_monospace_git_commit("f4a90c1"), "𝚏𝟺𝚊𝟿𝟶𝚌𝟷");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn topbar_carousel_routes_center_active_page_and_wrap_neighbors() {
        assert_eq!(
            topbar_carousel_routes(AppRoute::Cukta),
            [AppRoute::Gentufa, AppRoute::Cukta, AppRoute::Vlacku]
        );
        assert_eq!(
            topbar_carousel_routes(AppRoute::Vlacku),
            [AppRoute::Cukta, AppRoute::Vlacku, AppRoute::Gentufa]
        );
        assert_eq!(
            topbar_carousel_routes(AppRoute::Gentufa),
            [AppRoute::Vlacku, AppRoute::Gentufa, AppRoute::Cukta]
        );
        assert_eq!(
            topbar_carousel_routes(AppRoute::Settings),
            [AppRoute::Cukta, AppRoute::Vlacku, AppRoute::Gentufa]
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn page_find_entry_texts(entries: &[PageFindTextEntry]) -> Vec<String> {
        entries.iter().map(|entry| entry.text.clone()).collect()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn page_find_matching_handles_empty_overlap_and_unicode_ranges() {
        assert!(page_find_match_ranges("banana", "").is_empty());
        assert!(page_find_match_ranges("", "ana").is_empty());

        let overlapping = page_find_match_ranges("banana", "ana");
        assert_eq!(overlapping.len(), 1);
        assert_eq!(overlapping[0].byte_start, 1);
        assert_eq!(overlapping[0].byte_end, 4);

        let unicode_text = "İS";
        let unicode = page_find_match_ranges(unicode_text, "i\u{307}s");
        assert_eq!(unicode.len(), 1);
        assert!(unicode_text.is_char_boundary(unicode[0].byte_start));
        assert!(unicode_text.is_char_boundary(unicode[0].byte_end));
        assert_eq!(
            &unicode_text[unicode[0].byte_start..unicode[0].byte_end],
            unicode_text
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn page_find_route_state_remembers_queries_and_resets_active_selection() {
        let mut state = PageFindState::default();

        set_page_find_query(
            &mut state,
            AppRoute::Cukta,
            "broda".to_owned(),
            PageFindRouteQueryUpdate::Replace,
        );
        update_page_find_active(&mut state, AppRoute::Cukta, PageFindDirection::Next, 3);

        set_page_find_query(
            &mut state,
            AppRoute::Vlacku,
            "valsi".to_owned(),
            PageFindRouteQueryUpdate::Replace,
        );

        assert_eq!(state.cukta.query, "broda");
        assert_eq!(state.cukta.active_index, Some(0));
        assert_eq!(state.vlacku.query, "valsi");
        assert_eq!(state.vlacku.active_index, None);

        set_page_find_query(
            &mut state,
            AppRoute::Cukta,
            "brode".to_owned(),
            PageFindRouteQueryUpdate::Replace,
        );
        assert_eq!(state.cukta.query, "brode");
        assert_eq!(state.cukta.active_index, None);

        state.cukta = state.cukta.clone().with_data(data! {
            active_index: Some(2),
            result_signature: 10,
        });
        sync_page_find_result_signature(&mut state, AppRoute::Cukta, 11, 3);
        assert_eq!(state.cukta.active_index, None);

        state.cukta = state.cukta.clone().with_data(data! {
            active_index: Some(5),
            result_signature: 11,
        });
        sync_page_find_result_signature(&mut state, AppRoute::Cukta, 11, 3);
        assert_eq!(state.cukta.active_index, None);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn topbar_layout_resolver_prefers_full_nav_then_compact_settings_then_carousel() {
        let layout =
            topbar_layout_from_probe_fits(|selector| selector == ".app-topbar-fit-probe-both-full");
        assert_eq!(layout.settings, TopbarSettingsLayout::BothInline);
        assert_eq!(layout.nav, TopbarNavLayout::Full);

        let layout = topbar_layout_from_probe_fits(|selector| {
            selector == ".app-topbar-fit-probe-theme-full"
        });
        assert_eq!(layout.settings, TopbarSettingsLayout::ThemeInline);
        assert_eq!(layout.nav, TopbarNavLayout::Full);

        let layout = topbar_layout_from_probe_fits(|selector| {
            selector == ".app-topbar-fit-probe-both-carousel"
        });
        assert_eq!(layout.settings, TopbarSettingsLayout::BothInline);
        assert_eq!(layout.nav, TopbarNavLayout::Carousel);

        let layout = topbar_layout_from_probe_fits(|_selector| false);
        assert_eq!(layout.settings, TopbarSettingsLayout::NoneInline);
        assert_eq!(layout.nav, TopbarNavLayout::Carousel);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn page_find_collects_cukta_content_but_not_toc() {
        let page = CuktaPageData {
            toc: vec![CuktaTocNode {
                node_id: "toc-hidden".to_owned(),
                number_label: None,
                label: "TOC hidden label".to_owned(),
                href: "/cukta#toc-hidden".to_owned(),
                active: false,
                section_id: None,
                current: false,
                children: Vec::new(),
            }],
            current_section_id: None,
            page_kind: CuktaPageKind::Section {
                section_heading: "Section heading".to_owned(),
                section_parse_href: None,
                chapter_title: None,
                previous_section: None,
                next_section: None,
                chapter_prelude_blocks: Vec::new(),
                blocks: vec![CllBlock::Paragraph {
                    anchor_id: None,
                    role: None,
                    inlines: vec![CllInline::Text("Visible CLL body".to_owned())],
                    text: "Visible CLL body".to_owned(),
                }],
            },
        };
        let mut entries = Vec::new();

        collect_cukta_page_find_entries(&mut entries, &page, GentufaScript::Latin);
        let texts = page_find_entry_texts(&entries);

        assert!(texts.contains(&"Section heading".to_owned()));
        assert!(texts.contains(&"Visible CLL body".to_owned()));
        assert!(!texts.contains(&"TOC hidden label".to_owned()));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn page_find_collects_vlacku_card_text_without_controls() {
        let result = VlackuWebResult {
            state: VlackuWebState::default(),
            cards: vec![VlackuWebCard {
                rank: 1,
                word: "broda".to_owned(),
                display_word: "broda".to_owned(),
                word_type: "gismu".to_owned(),
                word_type_key: "gismu".to_owned(),
                selmaho: Some("BRIVLA".to_owned()),
                author: Some(new!(VlackuWebAuthor {
                    username: "alice".to_owned(),
                    realname: Some("Alice A.".to_owned()),
                })),
                ipa: Some("bɾoda".to_owned()),
                similarity: Some(0.42),
                votes: VlackuVoteDisplay::Known("+7".to_owned()),
                rafsi: vec!["bro".to_owned()],
                glosses: vec!["predicate".to_owned()],
                definition_source: "definition source".to_owned(),
                definition: vec![
                    new!(VlackuInline::Text("definition body ".to_owned())),
                    new!(VlackuInline::WordRef {
                        label: "klama".to_owned(),
                        href: "/vlacku?mode=word&q=klama".to_owned(),
                        can_add_to_jvozba: true,
                    }),
                ],
                notes: vec![new!(VlackuInline::Text("note body".to_owned()))],
                etymology: vec![new!(VlackuInline::Text("etymology body".to_owned()))],
                decomposition: vec![VlackuCompositionPiece {
                    kind: VlackuCompositionPieceKind::Rafsi,
                    surface: "bro".to_owned(),
                    display_surface: "bro".to_owned(),
                    source: Some("broda".to_owned()),
                    display_source: Some("broda".to_owned()),
                    source_href: None,
                    source_is_surface: false,
                }],
                can_add_to_jvozba: true,
            }],
            word_type_options: Vec::new(),
            dictionary_info: None,
            has_more: true,
            message: Some("semantic message".to_owned()),
            errors: vec!["visible error".to_owned()],
        };
        let mut entries = Vec::new();

        collect_vlacku_page_find_entries(&mut entries, &result, GentufaScript::Latin);
        let texts = page_find_entry_texts(&entries);

        for expected in [
            "semantic message",
            "visible error",
            "broda",
            "/bɾoda/",
            "bro",
            "by alice (Alice A.)",
            "gismu",
            "BRIVLA",
            "42%",
            "+7",
            "definition body ",
            "klama",
            "predicate",
            "note body",
            "etymology: ",
            "etymology body",
            "Load more",
        ] {
            assert!(texts.contains(&expected.to_owned()), "missing {expected}");
        }
        assert!(!texts.contains(&"Dictionary query".to_owned()));
        assert!(!texts.contains(&"jvozba".to_owned()));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn page_find_collects_gentufa_outputs_and_excludes_edge_labels() {
        let edge_marker = ReferenceMarker {
            role: ReferenceMarkerRole::Reference,
            kind: "edge-only-kind".to_owned(),
            label: ReferenceLabel::new("edgeonly", None, None),
            source: None,
            tooltip: None,
        };
        let success = GentufaSuccess {
            ipa_text: "ipa-visible".to_owned(),
            surface_text: String::new(),
            brackets_text: "bracket visible".to_owned(),
            bracket_fragments: vec![GentufaBracketFragment::Text {
                text: "bracket visible".to_owned(),
                elided: false,
            }],
            blocks_layout: GentufaBlocksLayout {
                blocks: vec![GentufaBlock {
                    block_id: "block-1".to_owned(),
                    node_ids: vec![1],
                    label: "block label".to_owned(),
                    is_leaf: true,
                    is_elided: false,
                    token_kind: None,
                    ref_markers: Vec::new(),
                    span: None,
                    node_types: Vec::new(),
                    ancestors: Vec::new(),
                    col: 0,
                    col_span: 1,
                    row: 0,
                    row_span: 1,
                    color: "#cccccc".to_owned(),
                    parent_color: None,
                    raw_text: "block label".to_owned(),
                    display_text: "block label".to_owned(),
                    transform: None,
                    glosses: vec!["block gloss".to_owned()],
                    definition: None,
                    computed_gloss: None,
                    tooltip: None,
                }],
                max_col: 1,
                max_row: 1,
            },
            tree_rows: vec![GentufaTreeRow {
                node_id: 1,
                parent_id: None,
                depth: 0,
                label: "tree category".to_owned(),
                color: "#cccccc".to_owned(),
                guides: Vec::new(),
                has_children: false,
                cells: vec![GentufaCell {
                    text: "tree token".to_owned(),
                    is_word: true,
                    quoted: false,
                    tooltip: None,
                    is_elided: false,
                    transform: None,
                }],
                computed_gloss: None,
                ref_markers: vec![edge_marker],
                glosses: Vec::new(),
                definition: None,
                rafsi_breakdown: Vec::new(),
            }],
            diagnostics: Vec::new(),
            features: WebFeatureAvailability::default(),
        };
        let mut tree_entries = Vec::new();
        collect_gentufa_page_find_entries(
            &mut tree_entries,
            &GentufaWebResult::Success(success.clone()),
            None,
            GentufaWebViewMode::Tree,
            GentufaDisplayState {
                show_elided: false,
                show_glosses: true,
            },
            GentufaScript::Latin,
        );
        let tree_texts = page_find_entry_texts(&tree_entries);
        assert!(tree_texts.contains(&"bracket visible".to_owned()));
        assert!(tree_texts.contains(&"tree category".to_owned()));
        assert!(tree_texts.contains(&"tree token".to_owned()));
        assert!(!tree_texts.contains(&"edgeonly".to_owned()));
        assert!(!tree_texts.contains(&"edge-only-kind".to_owned()));
        assert!(!tree_texts.contains(&"ipa-visible".to_owned()));

        let mut block_entries = Vec::new();
        collect_gentufa_page_find_entries(
            &mut block_entries,
            &GentufaWebResult::Success(success.clone()),
            None,
            GentufaWebViewMode::Blocks,
            GentufaDisplayState {
                show_elided: false,
                show_glosses: true,
            },
            GentufaScript::Latin,
        );
        let block_texts = page_find_entry_texts(&block_entries);
        assert!(block_texts.contains(&"block label".to_owned()));
        assert!(block_texts.contains(&"block gloss".to_owned()));

        let mut ipa_entries = Vec::new();
        collect_gentufa_page_find_entries(
            &mut ipa_entries,
            &GentufaWebResult::Success(success),
            None,
            GentufaWebViewMode::Ipa,
            GentufaDisplayState {
                show_elided: false,
                show_glosses: true,
            },
            GentufaScript::Latin,
        );
        let ipa_texts = page_find_entry_texts(&ipa_entries);
        assert!(ipa_texts.contains(&"ipa-visible".to_owned()));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn page_find_collects_settings_static_text_without_editable_values() {
        let dialect_settings = DialectSettings {
            custom_dialects: vec![CustomDialect {
                name: "custom-visible".to_owned(),
                definition: "()".to_owned(),
                show_in_gentufa: true,
            }],
            hidden_builtin_gentufa_dialects: BTreeSet::new(),
        };
        let mut entries = Vec::new();

        collect_settings_page_find_entries(
            &mut entries,
            UserSettings {
                theme: ThemeMode::Day,
                script: GentufaScript::Latin,
                stress: StressMark::None,
                glides: GlideMark::None,
            },
            &dialect_settings,
            "custom-visible",
            &EmbeddingSettingsState::default(),
        );
        let texts = page_find_entry_texts(&entries);

        for expected in [
            "Settings",
            "Semantic search",
            "Embedding model",
            "Status",
            "Download",
            "Output",
            "Stress",
            "none",
            "Glides",
            "Lojban dialects",
            "Builtins",
            "Custom",
            "custom-visible",
            "Name",
            "Show in gentufa",
            "Definition",
            "Definition is valid.",
        ] {
            assert!(texts.contains(&expected.to_owned()), "missing {expected}");
        }
        assert!(!texts.contains(&"()".to_owned()));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn async_activity_tracks_overlapping_tasks_by_id() {
        let mut activity = AsyncActivityState::default();

        let gentufa_id = activity.begin(AsyncTaskKind::Gentufa);
        let cukta_id = activity.begin(AsyncTaskKind::Cukta);

        assert_ne!(gentufa_id, cukta_id);
        assert!(activity.is_active());
        assert!(activity.has_kind(AsyncTaskKind::Gentufa));
        assert!(activity.has_kind(AsyncTaskKind::Cukta));

        assert!(activity.finish(gentufa_id));
        assert!(activity.is_active());
        assert!(!activity.has_kind(AsyncTaskKind::Gentufa));
        assert!(activity.has_kind(AsyncTaskKind::Cukta));

        assert!(activity.finish(cukta_id));
        assert!(!activity.is_active());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn async_activity_finish_is_idempotent_for_cleanup_paths() {
        let mut activity = AsyncActivityState::default();
        let task_id = activity.begin(AsyncTaskKind::Export);

        assert!(activity.finish(task_id));
        assert!(!activity.finish(task_id));
        assert!(!activity.finish(task_id + 1));
        assert!(!activity.is_active());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedding_settings_parse_native_progress_payload() {
        let json = serde_json::json!({
            "selectedModelKey": F2LLM_NATIVE_330M_MODEL_KEY,
            "effectiveModelKey": F2LLM_NATIVE_330M_MODEL_KEY,
            "status": "preparing",
            "detail": "Indexing dictionary.",
            "progress": {
                "kind": "index",
                "label": "Indexing dictionary",
                "loaded": 3,
                "total": 10,
                "percent": 30
            }
        });

        let state = embedding_settings_from_json(&json.to_string(), "fallback");

        assert_eq!(state.progress_kind.as_deref(), Some("index"));
        assert_eq!(state.progress_loaded, Some(3));
        assert_eq!(state.progress_total, Some(10));
        assert_eq!(state.progress_percent, Some(30));
        assert_eq!(
            embedding_progress_display_label(&state),
            "Indexing dictionary 3/10 rows (30%)"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn native_embedding_model_options_cover_f2llm_size_family() {
        let keys = NATIVE_EMBEDDING_MODEL_OPTIONS
            .iter()
            .map(|option| option.key)
            .collect::<Vec<_>>();
        assert_eq!(
            keys,
            vec![
                F2LLM_NATIVE_80M_MODEL_KEY,
                F2LLM_NATIVE_160M_MODEL_KEY,
                F2LLM_NATIVE_330M_MODEL_KEY,
                F2LLM_NATIVE_0_6B_MODEL_KEY,
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedding_progress_display_formats_byte_progress() {
        let state = EmbeddingSettingsState {
            progress_kind: Some("download".to_owned()),
            progress_label: Some("Downloading model".to_owned()),
            progress_loaded: Some(1024),
            progress_total: Some(2048),
            progress_percent: Some(50),
            ..EmbeddingSettingsState::default()
        };

        assert_eq!(
            embedding_progress_display_label(&state),
            "Downloading model 1024 B / 2048 B (50%)"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_hover_measurement_id_is_monotonic_and_saturating() {
        let mut state = ReferenceHoverState::default();
        assert_eq!(next_reference_hover_measurement_id(&state), 1);
        state.measurement_id = 41;
        assert_eq!(next_reference_hover_measurement_id(&state), 42);
        state.measurement_id = u64::MAX;
        assert_eq!(next_reference_hover_measurement_id(&state), u64::MAX);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_hover_pointer_moves_do_not_request_async_measurement() {
        assert!(!reference_hover_refresh_requires_measurement(
            ReferenceHoverRefreshReason::PointerMove,
            true
        ));
        assert!(reference_hover_refresh_requires_measurement(
            ReferenceHoverRefreshReason::ViewportShift,
            true
        ));
        assert!(reference_hover_refresh_requires_measurement(
            ReferenceHoverRefreshReason::PointerMove,
            false
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_hover_keeps_overlay_during_same_target_async_measurement() {
        let hovered = HoveredReference {
            role: ReferenceMarkerRole::Reference,
            label: ReferenceLabel::new("b", Some(1), None),
        };
        let overlay = ArrowOverlay {
            width: 100.0,
            height: 80.0,
            paths: vec!["M 1.00 2.00 L 3.00 4.00".to_owned()],
        };
        let state = ReferenceHoverState {
            hovered: Some(hovered.clone()),
            overlay: Some(overlay.clone()),
            measurement_id: 7,
        };
        assert_eq!(
            reference_overlay_for_measurement_request(&state, &hovered, &None, true),
            Some(overlay.clone())
        );

        let other_hovered = HoveredReference {
            role: ReferenceMarkerRole::Referent,
            label: hovered.label.clone(),
        };
        assert_eq!(
            reference_overlay_for_measurement_request(&state, &other_hovered, &None, true),
            None
        );

        let measured_overlay = Some(ArrowOverlay {
            width: 120.0,
            height: 90.0,
            paths: vec!["M 5.00 6.00 L 7.00 8.00".to_owned()],
        });
        assert_eq!(
            reference_overlay_for_measurement_request(&state, &hovered, &measured_overlay, true),
            measured_overlay
        );
        assert_eq!(
            reference_overlay_for_measurement_request(&state, &hovered, &None, false),
            None
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostic_location_uses_one_indexed_line_column() {
        let source = "coi\nmi broda";
        let diagnostic = test_diagnostic(
            source,
            DiagnosticSeverity::Error,
            "syntax.unexpected-cmavo",
            "unexpected cmavo",
            4,
            6,
            "expected selbri",
        );

        let location = diagnostic_label_location(source, diagnostic.primary_label());

        assert_eq!(location.line, 2);
        assert_eq!(location.column, 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostic_location_uses_character_offsets_for_unicode() {
        let source = "coi\nzo'é mi";
        let diagnostic = test_diagnostic(
            source,
            DiagnosticSeverity::Error,
            "morphology.invalid",
            "invalid morphology",
            7,
            8,
            "invalid character",
        );

        let location = diagnostic_label_location(source, diagnostic.primary_label());

        assert_eq!(location.line, 2);
        assert_eq!(location.column, 4);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostic_pane_title_counts_errors_and_warning_like_diagnostics() {
        let source = "coi";
        let diagnostics = vec![
            test_diagnostic(
                source,
                DiagnosticSeverity::Error,
                "syntax.unexpected-cmavo",
                "unexpected cmavo",
                0,
                1,
                "expected text",
            ),
            test_diagnostic(
                source,
                DiagnosticSeverity::Warning,
                "syntax.warning.experimental",
                "experimental syntax",
                1,
                2,
                "experimental",
            ),
            test_diagnostic(
                source,
                DiagnosticSeverity::Advice,
                "syntax.advice",
                "syntax advice",
                2,
                3,
                "advice",
            ),
        ];

        let title = diagnostic_pane_title(diagnostic_counts(&diagnostics, None));

        assert_eq!(title, "Diagnostics: 1 error, 2 warnings");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stale_gentufa_input_disables_decorations() {
        let source = "coi";
        let diagnostic = test_diagnostic(
            source,
            DiagnosticSeverity::Error,
            "syntax.unexpected-cmavo",
            "unexpected cmavo",
            0,
            1,
            "expected text",
        );
        let request = GentufaWebRequest {
            text: source.to_owned(),
            options: GentufaWebOptions::default(),
        };
        let result = GentufaWebResult::Error(GentufaError {
            phase: None,
            message: "unexpected cmavo".to_owned(),
            diagnostics: vec![diagnostic],
        });

        assert_eq!(
            current_gentufa_input_diagnostics(source, &result, Some(&request)).len(),
            1
        );
        assert!(current_gentufa_input_diagnostics("coi mi", &result, Some(&request)).is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn active_overlay_context_prefix_extends_to_primary_span() {
        let source = "mi broda";
        let diagnostic = test_diagnostic(
            source,
            DiagnosticSeverity::Error,
            "syntax.unexpected-cmavo",
            "unexpected cmavo",
            3,
            8,
            "expected selbri",
        );
        let context_span = jbotci_diagnostics::source_span_from_char_offsets(None, source, 0, 2)
            .expect("test context span is valid");
        let mut labels = diagnostic.labels.clone();
        labels.push(DiagnosticLabel::new(
            context_span,
            "while parsing sumti".to_owned(),
            false,
        ));
        let diagnostic = diagnostic.with_data(data! { labels: labels });
        let diagnostics = vec![diagnostic];

        let fragments = diagnostic_overlay_fragments(source, &diagnostics, Some(0));
        let context_prefix = fragments
            .iter()
            .find(|fragment| fragment.text == "mi ")
            .expect("context prefix should include text up to the primary span");
        let primary = fragments
            .iter()
            .find(|fragment| fragment.text == "broda")
            .expect("primary span should be a separate fragment");

        assert!(has_css_class(
            &context_prefix.class_name,
            "is-active-context"
        ));
        assert!(has_css_class(
            &context_prefix.class_name,
            "is-active-context-start"
        ));
        assert!(!has_css_class(
            &context_prefix.class_name,
            "is-active-context-end"
        ));
        assert!(!has_css_class(
            &context_prefix.class_name,
            "is-active-primary"
        ));
        assert!(has_css_class(&primary.class_name, "is-active-primary"));
        assert!(has_css_class(
            &primary.class_name,
            "is-active-context-token"
        ));
        assert!(!has_css_class(
            &primary.class_name,
            "is-active-context-start"
        ));
        assert!(has_css_class(&primary.class_name, "is-active-context-end"));
        assert!(!has_css_class(&primary.class_name, "is-active-context"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostic_overlay_selection_offsets_are_utf16_offsets() {
        let source = "a 😀 broda";
        let diagnostic = test_diagnostic(
            source,
            DiagnosticSeverity::Error,
            "syntax.unexpected-cmavo",
            "unexpected cmavo",
            4,
            9,
            "expected selbri",
        );
        let context_span = jbotci_diagnostics::source_span_from_char_offsets(None, source, 0, 1)
            .expect("test context span is valid");
        let mut labels = diagnostic.labels.clone();
        labels.push(DiagnosticLabel::new(
            context_span,
            "while parsing sumti".to_owned(),
            false,
        ));
        let diagnostic = diagnostic.with_data(data! { labels: labels });
        let diagnostics = vec![diagnostic];

        let fragments = diagnostic_overlay_fragments(source, &diagnostics, Some(0));
        let primary = fragments
            .iter()
            .find(|fragment| fragment.text == "broda")
            .expect("primary span should be present");

        assert_eq!(
            primary.selection_start,
            "a 😀 ".encode_utf16().count() as u32
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn styled_diagnostic_notes_include_detailed_needs_one_of() {
        let source = "coi";
        let diagnostic = test_diagnostic(
            source,
            DiagnosticSeverity::Error,
            "syntax.unexpected-cmavo",
            "unexpected cmavo",
            0,
            1,
            "expected text",
        )
        .with_styled_notes(vec![DiagnosticStyledNote::new(
            jbotci_diagnostics::DiagnosticNoteMode::Detailed,
            vec![
                DiagnosticTextSegment::new(DiagnosticTextRole::Plain, "needs one of:\n".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, "- ".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Construct, "selbri".to_owned()),
            ],
        )]);

        let notes = diagnostic_styled_notes_for_web(&diagnostic);
        let note_text = notes[0]
            .segments
            .iter()
            .fold(String::new(), |mut text, segment| {
                text.push_str(&segment.text);
                text
            });

        assert_eq!(notes.len(), 1);
        assert!(note_text.contains("needs one of:"));
        assert!(note_text.contains("selbri"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostic_tooltip_uses_primary_detail_when_available() {
        let source = "coi";
        let diagnostic = test_diagnostic(
            source,
            DiagnosticSeverity::Error,
            "syntax.unexpected-cmavo",
            "unexpected cmavo",
            0,
            1,
            "expected free modifier, SE",
        );

        assert_eq!(
            diagnostic_tooltip_text(&diagnostic),
            "syntax.unexpected-cmavo: expected free modifier, SE"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostic_tooltip_prefers_structured_expected_headline() {
        let source = "li nu";
        let diagnostic = test_diagnostic(
            source,
            DiagnosticSeverity::Error,
            "syntax.unexpected-cmavo",
            "unexpected cmavo",
            3,
            5,
            "expected: free modifier or mex",
        )
        .with_styled_notes(vec![DiagnosticStyledNote::new(
            jbotci_diagnostics::DiagnosticNoteMode::Detailed,
            vec![
                DiagnosticTextSegment::new(DiagnosticTextRole::Plain, "needs one of:\n".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, "- ".to_owned()),
                DiagnosticTextSegment::new(
                    DiagnosticTextRole::Construct,
                    "free modifier".to_owned(),
                ),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " (".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::WordCategory, "LERFU".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ")\n".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, "- ".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Construct, "mex".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " (".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Selmaho, "PA".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ")".to_owned()),
            ],
        )]);

        assert_eq!(
            diagnostic_tooltip_text(&diagnostic),
            "syntax.unexpected-cmavo: expected: free modifier or mex"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostic_tooltip_uses_detailed_expectation_order_and_lerfu_name() {
        let source = "coi";
        let diagnostic = test_diagnostic(
            source,
            DiagnosticSeverity::Error,
            "syntax.unexpected-cmavo",
            "unexpected cmavo",
            0,
            1,
            "expected SE, free modifier, LERFU",
        )
        .with_styled_notes(vec![DiagnosticStyledNote::new(
            jbotci_diagnostics::DiagnosticNoteMode::Detailed,
            vec![
                DiagnosticTextSegment::new(DiagnosticTextRole::Plain, "needs one of:\n".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, "- ".to_owned()),
                DiagnosticTextSegment::new(
                    DiagnosticTextRole::Construct,
                    "free modifier".to_owned(),
                ),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " (".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::WordCategory, "LERFU".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " or ".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Selmaho, "COI".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ")\n".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, "- ".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::WordCategory, "BRIVLA".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " or ".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Selmaho, "SE".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " [".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Keyword, "continues".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " ".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Construct, "sumti".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, "]".to_owned()),
            ],
        )]);

        assert_eq!(
            diagnostic_tooltip_text(&diagnostic),
            "syntax.unexpected-cmavo: expected free modifier (LERFU or COI), BRIVLA or SE [continues sumti]"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn web_diagnostics_hide_redundant_expected_summary_notes() {
        let source = "coi";
        let diagnostic = test_diagnostic(
            source,
            DiagnosticSeverity::Error,
            "syntax.unexpected-cmavo",
            "unexpected cmavo",
            0,
            1,
            "expected text",
        )
        .with_data(data! {
            notes: vec![
                "expected one of: BRIVLA, SE".to_owned(),
                "another note".to_owned(),
            ],
        })
        .with_styled_notes(vec![
            DiagnosticStyledNote::new(
                jbotci_diagnostics::DiagnosticNoteMode::Summary,
                vec![
                    DiagnosticTextSegment::new(
                        DiagnosticTextRole::Plain,
                        "expected one of: ".to_owned(),
                    ),
                    DiagnosticTextSegment::new(DiagnosticTextRole::Selmaho, "SE".to_owned()),
                ],
            ),
            DiagnosticStyledNote::new(
                jbotci_diagnostics::DiagnosticNoteMode::Detailed,
                vec![
                    DiagnosticTextSegment::new(
                        DiagnosticTextRole::Plain,
                        "needs one of:\n".to_owned(),
                    ),
                    DiagnosticTextSegment::new(DiagnosticTextRole::Construct, "sumti".to_owned()),
                ],
            ),
        ]);

        let plain_notes = diagnostic_plain_notes_for_web(&diagnostic);
        let styled_notes = diagnostic_styled_notes_for_web(&diagnostic);

        assert_eq!(plain_notes, vec!["another note"]);
        assert_eq!(styled_notes.len(), 1);
        assert_eq!(
            diagnostic_styled_note_text(styled_notes[0]),
            "needs one of:\nsumti"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostic_plain_text_segments_style_expectation_terms() {
        let parts = diagnostic_plain_text_render_parts(
            "expected forethought selbri connection, linked arguments, FIhO modal, VUhU operator, statement, SE, LERFU, fe'e",
        );

        assert!(
            parts.iter().any(|part| {
                part.role == DiagnosticTextRole::Keyword && part.text == "expected"
            })
        );
        assert!(parts.iter().any(|part| {
            part.role == DiagnosticTextRole::Construct
                && part.text == "forethought selbri connection"
        }));
        assert!(parts.iter().any(|part| {
            part.role == DiagnosticTextRole::Construct && part.text == "linked arguments"
        }));
        assert!(parts.iter().any(|part| {
            part.role == DiagnosticTextRole::Construct && part.text == "FIhO modal"
        }));
        assert!(parts.iter().any(|part| {
            part.role == DiagnosticTextRole::Construct && part.text == "VUhU operator"
        }));
        assert!(parts.iter().any(|part| {
            part.role == DiagnosticTextRole::Construct && part.text == "statement"
        }));
        assert!(
            parts
                .iter()
                .any(|part| { part.role == DiagnosticTextRole::Selmaho && part.text == "SE" })
        );
        assert!(
            parts.iter().any(|part| {
                part.role == DiagnosticTextRole::WordCategory && part.text == "LERFU"
            })
        );
        assert!(
            parts.iter().any(|part| {
                part.role == DiagnosticTextRole::SpecificWord && part.text == "fe'e"
            })
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn selected_script_renders_visible_lojban_text_only() {
        assert_eq!(
            display_lojban_text(GentufaScript::Cyrillic, "mi klama le zarci"),
            "ми клама ле зарши"
        );
        assert_eq!(display_lojban_text(GentufaScript::Cyrillic, "coi"), "шой");
        assert_eq!(
            display_lojban_text(GentufaScript::Zbalermorna, "coi"),
            "\u{ed86}\u{eda8}"
        );
        assert_eq!(
            display_lojban_text(GentufaScript::Cyrillic, "hello!"),
            "hello!"
        );
        assert_eq!(
            display_lojban_text_if(GentufaScript::Cyrillic, "mi klama", false),
            "mi klama"
        );
        assert_eq!(
            cll_display_text_for_kind(GentufaScript::Cyrillic, "jbo", "mi klama"),
            "ми клама"
        );
        assert_eq!(
            cll_display_text_for_kind(GentufaScript::Cyrillic, "natlang", "I go"),
            "I go"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_tooltip_position_keeps_normal_above_host_placement() {
        let position = dictionary_tooltip_position(
            ReferenceRect {
                left: 240.0,
                top: 300.0,
                right: 260.0,
                bottom: 320.0,
            },
            ElementSize {
                width: 160.0,
                height: 120.0,
            },
            new!(TooltipViewport {
                top: 40.0,
                width: 640.0,
                height: 480.0,
            }),
        );

        assert_eq!(position.top, 172.0);
        assert_eq!(position.left, 170.0);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_tooltip_position_clamps_oversized_stack_below_visible_top() {
        let position = dictionary_tooltip_position(
            ReferenceRect {
                left: 240.0,
                top: 300.0,
                right: 260.0,
                bottom: 320.0,
            },
            ElementSize {
                width: 160.0,
                height: 460.0,
            },
            new!(TooltipViewport {
                top: 56.0,
                width: 640.0,
                height: 480.0,
            }),
        );

        assert_eq!(position.top, 64.0);
        assert_eq!(position.left, 170.0);
    }

    #[requires(!selector.is_empty())]
    #[ensures(!ret.is_empty())]
    fn css_rule<'a>(css: &'a str, selector: &str) -> &'a str {
        let selector_start = css.find(selector).expect("CSS selector");
        let rule_tail = &css[selector_start..];
        let rule_end = rule_tail.find('}').expect("CSS rule end");
        &rule_tail[..rule_end]
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn css_font_stacks_cover_ui_controls_and_lojban_fallbacks() {
        let css = include_str!("../assets/main.css");
        let root_rule = css_rule(css, ":root");
        assert!(root_rule.contains(
            "--ui-font: \"Noto Sans\", \"STIX Two Math\", \"Crisa\", Verdana, sans-serif;"
        ));
        assert!(root_rule.contains(
            "--lojban-font: \"Crisa\", \"Noto Sans\", \"STIX Two Math\", Verdana, sans-serif;"
        ));
        assert!(root_rule.contains(
            "--math-font: \"STIX Two Text\", \"STIX Two Math\", \"Noto Sans\", \"Crisa\", math, serif;"
        ));
        assert!(root_rule.contains(
            "--math-symbol-font: \"STIX Two Math\", \"Noto Sans\", \"Crisa\", Verdana, sans-serif;"
        ));
        assert!(
            root_rule
                .contains("--code-font: \"Noto Sans\", \"STIX Two Math\", \"Crisa\", monospace;")
        );
        assert!(root_rule.contains("font-family: var(--ui-font);"));
        let form_controls_rule = css_rule(css, "button,");
        assert!(form_controls_rule.contains("input,"));
        assert!(form_controls_rule.contains("textarea"));
        assert!(form_controls_rule.contains("font: inherit;"));

        let blocks_rule = css_rule(css, ".spa-shell.app-page");
        assert!(blocks_rule.contains("--blocks-font: var(--ui-font);"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zbalermorna_linked_lojban_css_uses_lojban_font() {
        let css = include_str!("../assets/main.css");
        let selectors = [
            ".app-page.orthography-zbalermorna .parse-page .brackets-output",
            ".app-page.orthography-zbalermorna .parse-page .diagnostic-text-specific-word",
            ".app-page.orthography-zbalermorna .dictionary-page .dictionary-word-link",
            ".app-page.orthography-zbalermorna .reference-resolution-tooltip .reference-row-target",
            ".app-page.orthography-zbalermorna .rich-dictionary-tooltip .tooltip-inline-link",
            ".app-page.orthography-zbalermorna .cll-page .spa-cll-link-dictionary",
            ".app-page.orthography-zbalermorna .cll-page .spa-cll-link-parse",
            ".app-page.orthography-zbalermorna .cll-page .spa-cll-jbophrase",
        ];

        for selector in selectors {
            let rule = css_rule(css, selector);
            assert!(
                rule.contains("font-family: var(--lojban-font);"),
                "{selector}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zbalermorna_block_native_titles_are_suppressed() {
        let block = test_gentufa_block(0, 1, &[]);

        assert_eq!(block_native_title(&block, GentufaScript::Latin), "test");
        assert_eq!(block_native_title(&block, GentufaScript::Cyrillic), "test");
        assert_eq!(block_native_title(&block, GentufaScript::Zbalermorna), "");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostic_specific_words_render_with_selected_script() {
        let word = diagnostic_text_render_part(DiagnosticTextRole::SpecificWord, "fe'e".to_owned());
        let selmaho = diagnostic_text_render_part(DiagnosticTextRole::Selmaho, "FAhA".to_owned());

        let rendered_word = diagnostic_display_text_part_for_script(&word, GentufaScript::Cyrillic);

        assert_ne!(rendered_word, "fe'e");
        assert!(rendered_word.contains('ф'));
        assert_eq!(
            diagnostic_display_text_part_for_script(&selmaho, GentufaScript::Cyrillic),
            "FAhA"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cll_link_kinds_identify_lojban_link_text() {
        assert!(cll_link_text_is_lojban(CllLinkKind::Dictionary));
        assert!(cll_link_text_is_lojban(CllLinkKind::Rafsi));
        assert!(cll_link_text_is_lojban(CllLinkKind::Parse));
        assert!(!cll_link_text_is_lojban(CllLinkKind::Section));
        assert!(!cll_link_text_is_lojban(CllLinkKind::External));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostic_input_tooltip_card_uses_opaque_background_variable() {
        let css = include_str!("../assets/main.css");
        let selector = ".parse-page .gentufa-diagnostic-input-tooltip .gentufa-diagnostic-card";
        let selector_start = css.find(selector).expect("tooltip card selector");
        let rule_tail = &css[selector_start..];
        let rule_end = rule_tail.find('}').expect("tooltip card rule end");
        let rule = &rule_tail[..rule_end];

        assert!(css.contains("--diagnostic-tooltip-card-bg: var(--app-surface-0);"));
        assert!(css.contains("--diagnostic-tooltip-card-bg: var(--app-surface-2);"));
        assert!(rule.contains("background: var(--diagnostic-tooltip-card-bg);"));
        let background_line = rule
            .lines()
            .find(|line| line.trim_start().starts_with("background:"))
            .expect("tooltip card background declaration");
        assert!(!background_line.contains("transparent"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostic_token_links_follow_cukta_and_vlacku_conventions() {
        let word = new!(DiagnosticTextRenderPart {
            role: DiagnosticTextRole::SpecificWord,
            text: "fe'e".to_owned(),
        });
        let selmaho = new!(DiagnosticTextRenderPart {
            role: DiagnosticTextRole::Selmaho,
            text: "BAI".to_owned(),
        });
        let category = new!(DiagnosticTextRenderPart {
            role: DiagnosticTextRole::WordCategory,
            text: "BRIVLA".to_owned(),
        });
        let construct = new!(DiagnosticTextRenderPart {
            role: DiagnosticTextRole::Construct,
            text: "sumti".to_owned(),
        });
        let statement = new!(DiagnosticTextRenderPart {
            role: DiagnosticTextRole::Construct,
            text: "statement".to_owned(),
        });

        assert_eq!(
            diagnostic_text_part_href(&word, "/jbotci").as_deref(),
            Some("/jbotci/vlacku/fe'e")
        );
        assert_eq!(
            diagnostic_text_part_href(&selmaho, "/jbotci").as_deref(),
            Some("/jbotci/cukta/section/section-index#BAI")
        );
        assert_eq!(
            diagnostic_text_part_href(&category, "/jbotci").as_deref(),
            Some("/jbotci/cukta/section/section-morphology-brivla")
        );
        assert_eq!(
            diagnostic_text_part_href(&construct, "/jbotci").as_deref(),
            Some("/jbotci/cukta/section/section-EBNF#ebnf-rule-sumti")
        );
        assert_eq!(
            diagnostic_text_part_href(&statement, "/jbotci").as_deref(),
            Some("/jbotci/cukta/section/section-EBNF#ebnf-rule-statement")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostic_construct_links_cover_new_syntax_constructs() {
        for (construct, rule) in [
            ("forethought bridi connection", "gek-sentence"),
            ("forethought sumti connection", "sumti-4"),
            ("forethought selbri connection", "selbri-6"),
            ("forethought mex", "mex"),
            ("termset", "termset"),
            ("place tag", "term"),
            ("quantifier", "quantifier"),
            ("linked arguments", "linkargs"),
            ("operator", "operator"),
            ("word-sequence quote", "sumti-6"),
            ("FIhO modal", "tense-modal"),
            ("VUhU operator", "mex-operator"),
        ] {
            let part = new!(DiagnosticTextRenderPart {
                role: DiagnosticTextRole::Construct,
                text: construct.to_owned(),
            });
            let expected_href = diagnostic_ebnf_rule_href("/jbotci", rule);

            assert_eq!(
                diagnostic_text_part_href(&part, "/jbotci").as_deref(),
                Some(expected_href.as_str()),
                "unexpected link for diagnostic construct {construct:?}",
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_search_debounce_is_longer_than_url_debounce() {
        assert_eq!(VLACKU_SEARCH_DEBOUNCE_MS, 900);
        assert_eq!(CUKTA_SEARCH_DEBOUNCE_MS, VLACKU_SEARCH_DEBOUNCE_MS);
        assert!(VLACKU_SEARCH_DEBOUNCE_MS > VLACKU_URL_DEBOUNCE_MS);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pending_local_route_writes_consume_exact_route_once() {
        let route = parse_test_route("", "/vlacku/klama");
        let mut pending = PendingLocalRouteWrites::default();

        pending.record(&route);

        assert!(pending.consume(&route));
        assert!(!pending.consume(&route));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pending_local_route_writes_do_not_consume_nonmatching_routes() {
        let route = parse_test_route("", "/vlacku/klama");
        let other = parse_test_route("", "/vlacku/ciska");
        let mut pending = PendingLocalRouteWrites::default();

        pending.record(&route);

        assert!(!pending.consume(&other));
        assert!(pending.consume(&route));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pending_local_route_writes_consume_duplicate_targets_together() {
        let route = parse_test_route("", "/gentufa?text=coi");
        let mut pending = PendingLocalRouteWrites::default();

        pending.record(&route);
        pending.record(&route);

        assert!(pending.consume(&route));
        assert!(!pending.consume(&route));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pending_local_gentufa_writes_match_router_normalized_routes() {
        let target = JbotciRoute::from_web_route(
            WebRoute::Gentufa(GentufaWebState {
                text: " coi ".to_owned(),
                dialect: Some(" (cbm) ".to_owned()),
                view_mode: GentufaWebViewMode::Blocks,
                show_elided: false,
                show_glosses: false,
            }),
            true,
        );
        let reported = parse_test_route("", "/gentufa?text=coi&dialect=%28cbm%29");
        let mut pending = PendingLocalRouteWrites::default();

        pending.record(&target);

        assert!(pending.consume(&reported));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn document_title_uses_route_default_meta() {
        let route = parse_test_route("", "/settings");
        let meta = route_document_meta("", &route);

        assert_eq!(document_title_from_meta(&meta), "Settings");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn document_title_uses_result_meta_when_available() {
        let meta = new!(PageMeta {
            title: "coi - jbotci gentufa".to_owned(),
            description: "Gentufa parse result.".to_owned(),
            canonical_url: "/gentufa?text=coi".to_owned(),
            image: None,
        });

        assert_eq!(document_title_from_meta(&meta), "coi - jbotci gentufa");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_url_target_uses_committed_parse_state() {
        let draft_text = "mi klama";
        let committed_state = gentufa_state_from_parts(
            "coi",
            "",
            GentufaWebViewMode::Blocks,
            GentufaDisplayState {
                show_elided: false,
                show_glosses: false,
            },
            true,
        );

        let target = gentufa_route_for_committed_state(&committed_state, true);

        assert_eq!(target.to_string(), "/gentufa?text=coi");
        assert!(!target.to_string().contains(draft_text));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_url_sync_requires_gentufa_browser_location() {
        let gentufa_route = parse_test_route("", "/gentufa?text=coi");
        let vlacku_route = parse_test_route("", "/vlacku/klama");
        let cukta_route = parse_test_route("", "/cukta/search?q=klama");

        assert!(gentufa_url_sync_allowed(AppRoute::Gentufa, &gentufa_route));
        assert!(!gentufa_url_sync_allowed(AppRoute::Gentufa, &vlacku_route));
        assert!(!gentufa_url_sync_allowed(AppRoute::Gentufa, &cukta_route));
        assert!(!gentufa_url_sync_allowed(AppRoute::Vlacku, &gentufa_route));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_parse_intent_pushes_changed_route() {
        let current = parse_test_route("", "/gentufa?text=coi");
        let target_state = gentufa_state_from_parts(
            "mi klama",
            "",
            GentufaWebViewMode::Blocks,
            GentufaDisplayState {
                show_elided: false,
                show_glosses: false,
            },
            true,
        );
        let target = gentufa_route_for_committed_state(&target_state, true);

        assert_eq!(
            gentufa_url_history_action(&current, &target, GentufaUrlWriteIntent::PushParse),
            GentufaUrlHistoryAction::PushParse
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_display_changes_replace_current_route() {
        let current = parse_test_route("", "/gentufa?text=coi");
        let target_state = gentufa_state_from_parts(
            "coi",
            "",
            GentufaWebViewMode::Tree,
            GentufaDisplayState {
                show_elided: false,
                show_glosses: false,
            },
            true,
        );
        let target = gentufa_route_for_committed_state(&target_state, true);

        assert_eq!(
            gentufa_url_history_action(&current, &target, GentufaUrlWriteIntent::ReplaceCurrent),
            GentufaUrlHistoryAction::ReplaceCurrent
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_matching_route_has_no_url_write() {
        let current = parse_test_route("", "/gentufa?text=coi&view=tree");
        let target_state = gentufa_state_from_parts(
            "coi",
            "",
            GentufaWebViewMode::Tree,
            GentufaDisplayState {
                show_elided: false,
                show_glosses: false,
            },
            true,
        );
        let target = gentufa_route_for_committed_state(&target_state, true);

        assert_eq!(
            gentufa_url_history_action(&current, &target, GentufaUrlWriteIntent::PushParse),
            GentufaUrlHistoryAction::NoWrite
        );
        assert_eq!(
            gentufa_url_history_action(&current, &target, GentufaUrlWriteIntent::ReplaceCurrent),
            GentufaUrlHistoryAction::NoWrite
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_noop_sync_clears_pending_parse_intent() {
        assert_eq!(
            gentufa_url_intent_after_sync_action(
                GentufaUrlWriteIntent::PushParse,
                GentufaUrlHistoryAction::NoWrite,
            ),
            GentufaUrlWriteIntent::ReplaceCurrent
        );
        assert_eq!(
            gentufa_url_intent_after_sync_action(
                GentufaUrlWriteIntent::ReplaceCurrent,
                GentufaUrlHistoryAction::NoWrite,
            ),
            GentufaUrlWriteIntent::ReplaceCurrent
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_changed_parse_intent_survives_until_route_matches() {
        assert_eq!(
            gentufa_url_intent_after_sync_action(
                GentufaUrlWriteIntent::PushParse,
                GentufaUrlHistoryAction::PushParse,
            ),
            GentufaUrlWriteIntent::PushParse
        );
        assert_eq!(
            gentufa_url_intent_after_sync_action(
                GentufaUrlWriteIntent::ReplaceCurrent,
                GentufaUrlHistoryAction::ReplaceCurrent,
            ),
            GentufaUrlWriteIntent::ReplaceCurrent
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn local_route_writes_still_update_active_page_selection() {
        let route = parse_test_route("", "/gentufa?text=coi");

        let action = route_location_sync_action(&route, true);

        assert_eq!(action.app_route, AppRoute::Gentufa);
        assert!(!action.hydrate_route_bound_state);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn same_app_route_does_not_need_signal_update() {
        assert!(!app_route_update_needed(
            AppRoute::Gentufa,
            AppRoute::Gentufa
        ));
        assert!(app_route_update_needed(AppRoute::Gentufa, AppRoute::Vlacku));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn browser_route_changes_update_page_selection_and_hydrate_state() {
        let route = parse_test_route("", "/vlacku?mode=smuni&q=nonsense");

        let action = route_location_sync_action(&route, false);

        assert_eq!(action.app_route, AppRoute::Vlacku);
        assert!(action.hydrate_route_bound_state);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_search_query_draft_resets_count_and_preserves_controls() {
        let state = CuktaWebSearchState {
            mode: CuktaWebMode::Word,
            query: "klama".to_owned(),
            count: 80,
            targets: vec!["example".to_owned()],
        };

        let next = cukta_search_state_with_query(&state, "ciska");

        assert_eq!(next.mode, CuktaWebMode::Word);
        assert_eq!(next.query, "ciska");
        assert_eq!(next.count, CUKTA_WEB_DEFAULT_COUNT);
        assert_eq!(next.targets, vec!["example".to_owned()]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn output_settings_parse_cli_mark_names() {
        assert_eq!(parse_stress_mark("none"), Some(StressMark::None));
        assert_eq!(parse_stress_mark("acute"), Some(StressMark::Acute));
        assert_eq!(parse_stress_mark("caps"), Some(StressMark::Caps));
        assert_eq!(parse_stress_mark("uppercase"), None);
        assert_eq!(stress_mark_storage_value(StressMark::Caps), "caps");

        assert_eq!(parse_glide_mark("none"), Some(GlideMark::None));
        assert_eq!(parse_glide_mark("breve"), Some(GlideMark::Breve));
        assert_eq!(parse_glide_mark("acute"), None);
        assert_eq!(glide_mark_storage_value(GlideMark::Breve), "breve");
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_jvozba_is_available_without_browser_width() {
        assert!(vlacku_jvozba_available());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_dictionary_tooltip_helpers_cover_inline_and_ebnf_links() {
        let inline_card = cll_dictionary_tooltip_for_link("", CllLinkKind::Dictionary, "klama")
            .expect("dictionary CLL links should have tooltips");
        assert_eq!(inline_card.display_word, "klama");

        let ebnf_card = cll_dictionary_tooltip_for_href("", "../vlacku/klama")
            .expect("EBNF vlacku links should have tooltips");
        assert_eq!(ebnf_card.display_word, "klama");

        assert!(cll_dictionary_tooltip_for_link("", CllLinkKind::Rafsi, "kla").is_some());
        assert!(cll_dictionary_tooltip_for_href("", "../vlacku?mode=rafsi&q=kla").is_some());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_ebnf_section_links_use_v1_routes() {
        let href = cll_ebnf_href("/jbotci", "section/section-index#BAI");

        assert_eq!(href, "/jbotci/cukta/section/section-index#BAI");
        assert_eq!(
            cukta_section_reference_from_href(&href),
            Some("section-index".to_owned())
        );
        assert_eq!(cukta_anchor_from_href(&href), Some("BAI".to_owned()));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_hash_scroll_target_requires_cukta_route_and_anchor() {
        assert_eq!(
            cukta_hash_scroll_target(
                "/cukta/section/section-index",
                "",
                Some("#KEhE"),
                AppRoute::Cukta,
            ),
            Some("/cukta/section/section-index#KEhE".to_owned())
        );
        assert_eq!(
            cukta_hash_scroll_target(
                "/jbotci/cukta/section/section-index",
                "?q=unused",
                Some("KEhE"),
                AppRoute::Cukta,
            ),
            Some("/jbotci/cukta/section/section-index?q=unused#KEhE".to_owned())
        );
        assert_eq!(
            cukta_hash_scroll_target("/gentufa", "", Some("#KEhE"), AppRoute::Gentufa),
            None
        );
        assert_eq!(
            cukta_hash_scroll_target(
                "/cukta/section/section-index",
                "",
                Some("#"),
                AppRoute::Cukta,
            ),
            None
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_navigation_scroll_distinguishes_history_topbar_and_fresh_links() {
        assert_eq!(
            cukta_pending_scroll_for_navigation(
                AppRoute::Cukta,
                "/cukta/section/section-index#NAI",
                true,
                false,
            ),
            Some(cukta_anchor_pending_scroll(
                "/cukta/section/section-index#NAI".to_owned()
            ))
        );
        assert_eq!(
            cukta_pending_scroll_for_navigation(
                AppRoute::Cukta,
                "/cukta/section/section-index",
                false,
                true,
            ),
            Some(cukta_stored_pending_scroll(
                "/cukta/section/section-index".to_owned()
            ))
        );
        assert_eq!(
            cukta_pending_scroll_for_navigation(
                AppRoute::Cukta,
                "/cukta/section/section-index",
                false,
                false,
            ),
            Some(cukta_top_pending_scroll())
        );
        assert_eq!(
            cukta_pending_scroll_for_navigation(AppRoute::Gentufa, "/gentufa", false, true),
            None
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_route_links_preserve_anchor_scroll_intent() {
        let anchor_route = parse_test_route("", "/cukta/section/section-index#KE");
        assert_eq!(
            cukta_pending_scroll_for_route_link("", &anchor_route),
            cukta_anchor_pending_scroll("/cukta/section/section-index#KE".to_owned())
        );

        let section_route = parse_test_route("", "/cukta/section/section-index");
        assert_eq!(
            cukta_pending_scroll_for_route_link("", &section_route),
            cukta_top_pending_scroll()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn explicit_route_links_only_set_cukta_scroll_intent_for_cukta_routes() {
        let anchor_route = parse_test_route("", "/cukta/section/section-index#KE");
        assert_eq!(
            cukta_pending_scroll_for_explicit_route_link("", &anchor_route),
            Some(cukta_anchor_pending_scroll(
                "/cukta/section/section-index#KE".to_owned()
            ))
        );

        let section_route = parse_test_route("", "/cukta/section/section-index");
        assert_eq!(
            cukta_pending_scroll_for_explicit_route_link("", &section_route),
            Some(cukta_top_pending_scroll())
        );

        let prefixed_section_route =
            parse_test_route("/jbotci", "/jbotci/cukta/section/section-index#KE");
        assert_eq!(
            cukta_pending_scroll_for_explicit_route_link("/jbotci", &prefixed_section_route),
            Some(cukta_anchor_pending_scroll(
                "/jbotci/cukta/section/section-index#KE".to_owned()
            ))
        );

        let gentufa_route = parse_test_route("", "/gentufa");
        assert_eq!(
            cukta_pending_scroll_for_explicit_route_link("", &gentufa_route),
            None
        );

        let vlacku_route = parse_test_route("", "/vlacku/klama");
        assert_eq!(
            cukta_pending_scroll_for_explicit_route_link("", &vlacku_route),
            None
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_history_route_changes_restore_stored_scroll_even_with_hash() {
        let anchor_route = parse_test_route("", "/cukta/section/section-index#KE");
        assert_eq!(
            cukta_pending_scroll_for_route_change("", &anchor_route),
            Some(cukta_stored_pending_scroll(
                "/cukta/section/section-index#KE".to_owned()
            ))
        );

        let prefixed_anchor_route =
            parse_test_route("/jbotci", "/jbotci/cukta/section/section-index#KE");
        assert_eq!(
            cukta_pending_scroll_for_route_change("/jbotci", &prefixed_anchor_route),
            Some(cukta_stored_pending_scroll(
                "/jbotci/cukta/section/section-index#KE".to_owned()
            ))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_scroll_waits_for_matching_rendered_page() {
        let state = CuktaWebState {
            view: CuktaWebView::Index,
        };
        let ready_page = CuktaAsyncPageState {
            state: Some(state.clone()),
            page: cukta_loading_page_data("Loaded CLL page."),
            meta: None,
            loading: false,
            error: None,
        };
        assert!(cukta_page_ready_for_scroll(&ready_page, &state));

        let mut loading_page = ready_page.clone();
        loading_page.loading = true;
        assert!(!cukta_page_ready_for_scroll(&loading_page, &state));

        let mut error_page = ready_page.clone();
        error_page.error = Some("failed".to_owned());
        assert!(!cukta_page_ready_for_scroll(&error_page, &state));

        let other_state = CuktaWebState {
            view: CuktaWebView::Search(CuktaWebSearchState::default()),
        };
        assert!(!cukta_page_ready_for_scroll(&ready_page, &other_state));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn block_bottom_row_uses_leaf_span_bottom() {
        let tall_leaf = test_gentufa_block(0, 3, &[ReferenceMarkerRole::Referent]);

        assert_eq!(block_bottom_row(&tall_leaf), 2);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_height_sizer_requires_incoming_reference() {
        let outgoing = test_gentufa_block(0, 1, &[ReferenceMarkerRole::Reference]);
        let incoming = test_gentufa_block(0, 1, &[ReferenceMarkerRole::Referent]);
        let plain = test_gentufa_block(0, 1, &[]);

        assert!(!block_needs_reference_height_sizer(&outgoing));
        assert!(block_needs_reference_height_sizer(&incoming));
        assert!(!block_needs_reference_height_sizer(&plain));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_marker_view_model_omits_native_title() {
        let hover_state = ReferenceHoverState::default();
        let plain = test_reference_marker(ReferenceMarkerRole::Reference, 0);
        let plain_view = reference_marker_view_model(&plain, &hover_state);
        assert_eq!(plain_view.native_title, None);
        assert!(!plain_view.has_tooltip);

        let mut rich = test_reference_marker(ReferenceMarkerRole::Reference, 0);
        rich.tooltip = Some(test_reference_tooltip());
        let rich_view = reference_marker_view_model(&rich, &hover_state);
        assert_eq!(rich_view.native_title, None);
        assert!(rich_view.has_tooltip);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_tooltip_host_class_opens_only_for_clicked_marker() {
        let marker = test_reference_marker(ReferenceMarkerRole::Reference, 0);
        assert_eq!(
            reference_tooltip_host_class(&marker, &None),
            "reference-tooltip-host"
        );

        let opened = Some(HoveredReference {
            role: marker.role,
            label: marker.label.clone(),
        });
        assert_eq!(
            reference_tooltip_host_class(&marker, &opened),
            "reference-tooltip-host is-open"
        );

        let other_role = Some(HoveredReference {
            role: ReferenceMarkerRole::Referent,
            label: marker.label.clone(),
        });
        assert_eq!(
            reference_tooltip_host_class(&marker, &other_role),
            "reference-tooltip-host"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_tooltip_row_view_model_separates_slot_and_target_text() {
        let row = new!(ReferenceTooltipRow {
            label: ReferenceLabel::new("k", None, Some(ReferenceSlotLabel::Numbered(1))),
            target_text: "lo mlatu be mi".to_owned(),
        });
        let view = reference_tooltip_row_view_model(&row);
        assert_eq!(view.slot_text.as_deref(), Some("𝟣"));
        assert_eq!(view.target_text, "lo mlatu be mi");

        let discourse_row = new!(ReferenceTooltipRow {
            label: ReferenceLabel::new("ko'a", Some(1), None),
            target_text: "mi".to_owned(),
        });
        let discourse_view = reference_tooltip_row_view_model(&discourse_row);
        assert_eq!(discourse_view.slot_text, None);
        assert_eq!(discourse_view.target_text, "mi");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn blocks_grid_row_template_uses_compact_rows() {
        assert_eq!(
            blocks_grid_row_template(3, true),
            "minmax(var(--blocks-compact-min-height), auto) minmax(var(--blocks-compact-min-height), auto) minmax(var(--blocks-compact-min-height), auto) auto"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_clearance_deficit_only_reports_needed_growth() {
        assert_eq!(reference_clearance_deficit(20.0, 40.0, 0.0), 0.0);
        assert_eq!(
            reference_clearance_deficit(20.0, 24.0, 0.0),
            BLOCK_REFERENCE_LABEL_GAP_PX - 4.0
        );
        assert_eq!(reference_clearance_deficit(20.0, 24.0, 4.0), 0.0);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_containment_deficit_only_reports_block_overflow() {
        assert_eq!(reference_containment_deficit(20.0, 32.0, 0.0), 0.0);
        assert_eq!(
            reference_containment_deficit(36.0, 32.0, 0.0),
            4.0 + BLOCK_REFERENCE_CONTAINMENT_GAP_PX
        );
        assert_eq!(reference_containment_deficit(36.0, 32.0, 5.0), 0.0);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn horizontal_ranges_overlap_requires_shared_interior() {
        assert!(horizontal_ranges_overlap(0.0, 10.0, 5.0, 15.0));
        assert!(!horizontal_ranges_overlap(0.0, 10.0, 10.0, 15.0));
        assert!(!horizontal_ranges_overlap(0.0, 10.0, 11.0, 15.0));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_semantic_worker_limit_bounds_unfiltered_results() {
        let state = VlackuWebState {
            mode: VlackuWebMode::Meaning,
            query: "klama".to_owned(),
            count: 20,
            word_types: Vec::new(),
        };
        assert_eq!(vlacku_semantic_worker_limit(&state), 21);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_semantic_worker_limit_bounds_filtered_results() {
        let state = VlackuWebState {
            mode: VlackuWebMode::Meaning,
            query: "klama".to_owned(),
            count: 20,
            word_types: vec!["gismu".to_owned()],
        };
        assert_eq!(vlacku_semantic_worker_limit(&state), 21);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_semantic_worker_limit_clamps_unfiltered_results() {
        let state = VlackuWebState {
            mode: VlackuWebMode::Meaning,
            query: "klama".to_owned(),
            count: usize::MAX,
            word_types: Vec::new(),
        };
        assert_eq!(vlacku_semantic_worker_limit(&state), VLACKU_WEB_MAX_COUNT);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_load_more_state_only_expands_count() {
        let state = VlackuWebState {
            mode: VlackuWebMode::Rafsi,
            query: "kla".to_owned(),
            count: 20,
            word_types: vec!["gismu".to_owned()],
        };

        let next = vlacku_load_more_state(&state);

        assert_eq!(next.mode, state.mode);
        assert_eq!(next.query, state.query);
        assert_eq!(next.word_types, state.word_types);
        assert_eq!(next.count, 40);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_load_more_state_clamps_count() {
        let state = VlackuWebState {
            mode: VlackuWebMode::Word,
            query: "klama".to_owned(),
            count: VLACKU_WEB_MAX_COUNT,
            word_types: Vec::new(),
        };

        let next = vlacku_load_more_state(&state);

        assert_eq!(next.count, VLACKU_WEB_MAX_COUNT);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stable_jvozba_pane_top_uses_anchor_at_unscrolled_position() {
        let top_at_page_top = stable_jvozba_pane_top(Some(242.0), 0, 46.0, 34.0);
        let top_after_scroll = stable_jvozba_pane_top(Some(-658.0), 900, 46.0, 34.0);

        assert_eq!(top_at_page_top, 242.0);
        assert_eq!(top_after_scroll, top_at_page_top);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stable_jvozba_pane_top_uses_fallback_until_results_render() {
        let top = stable_jvozba_pane_top(None, 900, 46.0, 34.0);

        assert_eq!(top, 46.0);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_semantic_result_is_pending_for_stale_or_loading_results() {
        let state = VlackuWebState {
            mode: VlackuWebMode::Meaning,
            query: "klama".to_owned(),
            count: 20,
            word_types: Vec::new(),
        };
        let semantic = VlackuSemanticResultState::default();

        assert!(vlacku_semantic_result_is_pending(&state, &semantic));

        let loading = VlackuSemanticResultState {
            state: Some(state.clone()),
            hits: Vec::new(),
            message: Some("Loading semantic search model.".to_owned()),
            loading: true,
        };
        assert!(vlacku_semantic_result_is_pending(&state, &loading));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_semantic_pending_page_preserves_existing_result() {
        let previous_state = VlackuWebState {
            mode: VlackuWebMode::Meaning,
            query: "klama".to_owned(),
            count: 20,
            word_types: Vec::new(),
        };
        let state = VlackuWebState {
            mode: VlackuWebMode::Meaning,
            query: "klama!".to_owned(),
            count: 20,
            word_types: Vec::new(),
        };
        let mut page = VlackuAsyncResultState {
            state: Some(previous_state.clone()),
            result: vlacku_loading_result(&previous_state, "Previous result remains visible."),
            meta: None,
            loading: false,
            error: None,
        };
        let semantic = VlackuSemanticResultState::default();

        let meta = apply_vlacku_semantic_pending_page(&mut page, "/jbotci", &state, &semantic);

        assert_eq!(page.state.as_ref(), Some(&state));
        assert!(page.loading);
        assert!(page.error.is_none());
        assert_eq!(
            page.result.message.as_deref(),
            Some("Previous result remains visible.")
        );
        assert_eq!(meta.title, "klama! - jbotci vlacku");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_semantic_pending_page_shows_explicit_loading_message() {
        let state = VlackuWebState {
            mode: VlackuWebMode::Meaning,
            query: "klama".to_owned(),
            count: 20,
            word_types: Vec::new(),
        };
        let mut page = VlackuAsyncResultState {
            state: Some(state.clone()),
            result: vlacku_loading_result(&state, "Previous result remains visible."),
            meta: None,
            loading: false,
            error: None,
        };
        let semantic = VlackuSemanticResultState {
            state: Some(state.clone()),
            hits: Vec::new(),
            message: Some("Loading semantic search model.".to_owned()),
            loading: true,
        };

        apply_vlacku_semantic_pending_page(&mut page, "/jbotci", &state, &semantic);

        assert_eq!(
            page.result.message.as_deref(),
            Some("Loading semantic search model.")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_semantic_result_ready_uses_compute_worker_path() {
        let state = VlackuWebState {
            mode: VlackuWebMode::Meaning,
            query: "klama".to_owned(),
            count: 20,
            word_types: Vec::new(),
        };
        let semantic = VlackuSemanticResultState {
            state: Some(state.clone()),
            hits: Vec::new(),
            message: None,
            loading: false,
        };

        assert!(!vlacku_semantic_result_is_pending(&state, &semantic));

        let request = vlacku_compute_request("/jbotci", &state, &semantic);
        assert!(matches!(
            request,
            WebComputeRequest::VlackuSemanticPage { loading: false, .. }
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn top_level_routes_accept_trailing_slashes() {
        assert_eq!(
            parse_test_route("/jbotci", "/jbotci/cukta/").app_route(),
            AppRoute::Cukta
        );
        assert_eq!(
            parse_test_route("/jbotci", "/jbotci/vlacku/").app_route(),
            AppRoute::Vlacku
        );
        assert_eq!(
            parse_test_route("/jbotci", "/jbotci/settings/").app_route(),
            AppRoute::Settings
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn typed_routes_preserve_canonical_url_contract() {
        assert_eq!(parse_test_route("", "/").to_string(), "/gentufa");

        let gentufa =
            parse_test_route("/jbotci", "/jbotci/gentufa?text=coi&view=tree&glosses=true");
        assert_eq!(
            gentufa.to_string(),
            "/gentufa?text=coi&view=tree&glosses=true"
        );
        assert!(gentufa.gentufa_text_explicit);

        let settings = parse_test_route("", "/settings?johau=lojban");
        assert_eq!(settings.to_string(), "/settings?johau=lojban");
        assert_eq!(settings.settings_query, "johau=lojban");

        let cukta_search = parse_test_route("", "/cukta/search?q=klama&target=example&count=40");
        assert_eq!(
            cukta_search.to_string(),
            "/cukta/search?q=klama&count=40&target=example"
        );

        let cukta_section =
            parse_test_route("", "/cukta/section/chapter-abstractions#section-example");
        assert_eq!(
            cukta_section.to_string(),
            "/cukta/section/chapter-abstractions#section-example"
        );

        assert_eq!(parse_test_route("", "/vlacku").to_string(), "/vlacku");
        assert_eq!(
            parse_test_route("", "/vlacku/klama").to_string(),
            "/vlacku/klama"
        );
        assert_eq!(
            parse_test_route("/jbotci", "/vlacku/klama").to_string(),
            "/vlacku/klama"
        );
        assert_eq!(
            parse_test_route("/jbotci", "/jbotci/vlacku/klama").to_string(),
            "/vlacku/klama"
        );
        assert_eq!(
            parse_test_route("", "/vlacku/%2Fma.*%2F").to_string(),
            "/vlacku/%2Fma.%2A%2F"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn typed_routes_accept_dioxus_route_strings() {
        assert_eq!(JbotciRoute::from_str("").unwrap().to_string(), "/gentufa");
        assert_eq!(
            JbotciRoute::from_str("gentufa?text=coi")
                .unwrap()
                .to_string(),
            "/gentufa?text=coi"
        );
        assert_eq!(
            JbotciRoute::from_str("settings?johau=lojban")
                .unwrap()
                .to_string(),
            "/settings?johau=lojban"
        );
        assert_eq!(
            JbotciRoute::from_str("cukta/section/chapter-abstractions#section-example")
                .unwrap()
                .to_string(),
            "/cukta/section/chapter-abstractions#section-example"
        );
        assert_eq!(
            JbotciRoute::from_str("vlacku/klama").unwrap().to_string(),
            "/vlacku/klama"
        );
        assert_eq!(
            JbotciRoute::from_str("vlacku/%2Fma.*%2F")
                .unwrap()
                .to_string(),
            "/vlacku/%2Fma.%2A%2F"
        );
        assert!(JbotciRoute::from_str("assets/compute-worker.js").is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn deployment_root_href_targets_router_prefix_root() {
        assert_eq!(deployment_root_href(""), "/");
        assert_eq!(deployment_root_href("/"), "/");
        assert_eq!(deployment_root_href("/jbotci"), "/jbotci/");
        assert_eq!(deployment_root_href("/jbotci/"), "/jbotci/");
    }

    #[requires(true)]
    #[ensures(true)]
    fn parse_test_route(base_path: &str, href: &str) -> JbotciRoute {
        jbotci_route_from_href(base_path, href).expect("test route should parse")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_toc_hidden_button_opens_overlay_without_pinning() {
        let state = CuktaTocInteractionState {
            pinned: false,
            overlay_visible: false,
        };
        let button_state = cukta_toc_button_state(state.pinned, false, state.overlay_visible);

        assert_eq!(button_state, CuktaTocButtonState::Hidden);
        assert_eq!(
            cukta_toc_button_action(button_state),
            CuktaTocButtonAction::ShowOverlay
        );
        assert_eq!(
            cukta_toc_interaction_after_button_action(state, cukta_toc_button_action(button_state)),
            CuktaTocInteractionState {
                pinned: false,
                overlay_visible: true,
            }
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_toc_forced_visible_button_hides_overlay_without_changing_pin() {
        let state = CuktaTocInteractionState {
            pinned: true,
            overlay_visible: true,
        };
        let button_state = cukta_toc_button_state(state.pinned, true, state.overlay_visible);

        assert_eq!(button_state, CuktaTocButtonState::ForcedAutoHideVisible);
        assert_eq!(
            cukta_toc_button_action(button_state),
            CuktaTocButtonAction::HideOverlay
        );
        assert_eq!(
            cukta_toc_interaction_after_button_action(state, cukta_toc_button_action(button_state)),
            CuktaTocInteractionState {
                pinned: true,
                overlay_visible: false,
            }
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_toc_pinned_visible_button_unpins_and_keeps_overlay_visible() {
        let state = CuktaTocInteractionState {
            pinned: true,
            overlay_visible: false,
        };
        let button_state = cukta_toc_button_state(state.pinned, false, state.overlay_visible);

        assert_eq!(button_state, CuktaTocButtonState::PinnedVisible);
        assert_eq!(
            cukta_toc_button_action(button_state),
            CuktaTocButtonAction::Unpin
        );
        assert_eq!(
            cukta_toc_interaction_after_button_action(state, cukta_toc_button_action(button_state)),
            CuktaTocInteractionState {
                pinned: false,
                overlay_visible: true,
            }
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_toc_unpinned_visible_button_pins_and_returns_to_pinned_layout() {
        let state = CuktaTocInteractionState {
            pinned: false,
            overlay_visible: true,
        };
        let button_state = cukta_toc_button_state(state.pinned, false, state.overlay_visible);

        assert_eq!(button_state, CuktaTocButtonState::UnpinnedVisible);
        assert_eq!(
            cukta_toc_button_action(button_state),
            CuktaTocButtonAction::Pin
        );
        assert_eq!(
            cukta_toc_interaction_after_button_action(state, cukta_toc_button_action(button_state)),
            CuktaTocInteractionState {
                pinned: true,
                overlay_visible: false,
            }
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_toc_manual_expansion_survives_active_default_changes() {
        let state = CuktaTocExpansionState::default();
        assert!(!cukta_toc_node_expanded_with_default(
            "chapter-tour",
            false,
            &state
        ));

        let state = cukta_toc_expansion_with_node_state(&state, "chapter-tour", false, true);

        assert!(cukta_toc_node_expanded_with_default(
            "chapter-tour",
            false,
            &state
        ));
        assert!(cukta_toc_node_expanded_with_default(
            "chapter-tour",
            true,
            &state
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_toc_default_matching_toggle_prunes_override() {
        let state = CuktaTocExpansionState::default();
        let state = cukta_toc_expansion_with_node_state(&state, "chapter-tour", false, true);
        let state = cukta_toc_expansion_with_node_state(&state, "chapter-tour", false, false);

        assert!(state.expanded.is_empty());
        assert!(state.collapsed.is_empty());
        assert!(!cukta_toc_node_expanded_with_default(
            "chapter-tour",
            false,
            &state
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_jvozba_storage_reads_v0_json_items() {
        let raw = r#"
            [
              {"kind":"word","value":"cmene","indentLevel":2},
              {"kind":"rafsi","value":"vla","source":"valsi"}
            ]
        "#;

        let items = parse_vlacku_jvozba_items(raw);
        assert_eq!(
            items,
            vec![
                VlackuJvozbaItem {
                    kind: VlackuJvozbaItemKind::Word,
                    value: "cmene".to_owned(),
                    source: None,
                    indent_level: 2,
                },
                VlackuJvozbaItem {
                    kind: VlackuJvozbaItemKind::FixedRafsi,
                    value: "vla".to_owned(),
                    source: Some("valsi".to_owned()),
                    indent_level: 0,
                },
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_jvozba_storage_migrates_legacy_newline_items() {
        let items = parse_vlacku_jvozba_items("word\tcmene\nrafsi\tvla\nbad\tno\nword\t");

        assert_eq!(
            items,
            vec![
                VlackuJvozbaItem {
                    kind: VlackuJvozbaItemKind::Word,
                    value: "cmene".to_owned(),
                    source: None,
                    indent_level: 0,
                },
                VlackuJvozbaItem {
                    kind: VlackuJvozbaItemKind::FixedRafsi,
                    value: "vla".to_owned(),
                    source: None,
                    indent_level: 0,
                },
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_jvozba_storage_writes_v0_json_shape() {
        let raw = format_vlacku_jvozba_items(&[VlackuJvozbaItem {
            kind: VlackuJvozbaItemKind::FixedRafsi,
            value: "vla".to_owned(),
            source: Some("valsi".to_owned()),
            indent_level: 1,
        }]);

        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&raw).expect("valid json"),
            serde_json::json!([
                {"kind":"rafsi","value":"vla","source":"valsi","indentLevel":1}
            ])
        );
    }

    #[requires(row_span > 0)]
    #[ensures(ret.row == row)]
    fn test_gentufa_block(
        row: usize,
        row_span: usize,
        marker_roles: &[ReferenceMarkerRole],
    ) -> GentufaBlock {
        GentufaBlock {
            block_id: format!("test-{row}"),
            node_ids: Vec::new(),
            label: "test".to_owned(),
            is_leaf: true,
            is_elided: false,
            token_kind: None,
            ref_markers: marker_roles
                .iter()
                .enumerate()
                .map(|(index, role)| test_reference_marker(*role, index))
                .collect(),
            span: None,
            node_types: Vec::new(),
            ancestors: Vec::new(),
            col: 0,
            col_span: 1,
            row,
            row_span,
            color: "#ffffff".to_owned(),
            parent_color: None,
            raw_text: "test".to_owned(),
            display_text: "test".to_owned(),
            transform: None,
            glosses: Vec::new(),
            definition: None,
            computed_gloss: None,
            tooltip: None,
        }
    }

    #[requires(true)]
    #[ensures(ret.role == role)]
    fn test_reference_marker(role: ReferenceMarkerRole, index: usize) -> ReferenceMarker {
        ReferenceMarker {
            role,
            kind: "test".to_owned(),
            label: ReferenceLabel::new("b", Some(index + 1), None),
            source: None,
            tooltip: None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn test_reference_tooltip() -> ReferenceTooltip {
        new!(ReferenceTooltip {
            card: None,
            missing_word: Some("b".to_owned()),
            highlighted_places: Vec::new(),
            definition: Vec::new(),
            notes: Vec::new(),
            rows: Vec::new(),
        })
    }

    #[requires(char_start <= char_end)]
    #[requires(!code.is_empty())]
    #[requires(!message.is_empty())]
    #[requires(!label.is_empty())]
    #[ensures(!ret.labels.is_empty())]
    fn test_diagnostic(
        source: &str,
        severity: DiagnosticSeverity,
        code: &str,
        message: &str,
        char_start: usize,
        char_end: usize,
        label: &str,
    ) -> Diagnostic {
        let span =
            jbotci_diagnostics::source_span_from_char_offsets(None, source, char_start, char_end)
                .expect("test span is valid");
        Diagnostic::new(
            severity,
            jbotci_diagnostics::DiagnosticPhase::Syntax,
            code.to_owned(),
            message.to_owned(),
            vec![DiagnosticLabel::new(span, label.to_owned(), true)],
            Vec::new(),
            None,
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn has_css_class(class_name: &str, expected: &str) -> bool {
        class_name
            .split_whitespace()
            .any(|class_name| class_name == expected)
    }
}
