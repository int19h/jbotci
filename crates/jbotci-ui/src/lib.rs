use dioxus::core::Task;
use dioxus::prelude::*;
use jbotci_cll::{
    CllBlock, CllEbnfEntry, CllEbnfToken, CllInline, CllInterlinearRow, CllLanguageSpanKind,
    CllLinkKind, CllLojbanizationLine, CllLujvoPart, CllSimpleListOrientation, CllTableCell,
    cll_link_href, embedded_cll_site, wrap_ebnf_choice_lines,
};
use jbotci_diagnostics::{
    Diagnostic, DiagnosticLabel, DiagnosticSeverity, DiagnosticStyledNote, DiagnosticTextLink,
    DiagnosticTextRole, DiagnosticTextSegment, diagnostic_text_segments_text,
};
use jbotci_dialect::{
    CustomDialect, DialectSettings, add_dialect_formula_reference, builtin_dialect_names,
    custom_dialect_definition_to_johau_uri_with_custom_dialects, custom_dialect_is_valid,
    dialect_definition_to_text, dialect_formula_top_level_references,
    dialect_name_shows_in_gentufa_picker, find_builtin_dialect, import_johau_dialect_settings,
    parse_dialect_selection_formula, remove_dialect_formula_reference,
    replace_dialect_formula_reference,
};
#[cfg(test)]
use jbotci_gentufa::ReferenceMarkerKind;
use jbotci_output::{
    GlideMark, PhonemeRenderOptions, StressMark,
    qr_code::{encode_qr_alphanumeric_h, qr_code_svg},
    render_lojban_text_for_script,
};
use jbotci_web_core::CollisionScope;
#[cfg(test)]
use jbotci_web_core::ReferenceSlotLabel;
use jbotci_web_core::{
    APPLE_TOUCH_ICON_ASSET_PATH, CUKTA_WEB_DEFAULT_COUNT, CUKTA_WEB_MAX_COUNT, CuktaModeOption,
    CuktaPageData, CuktaPageKind, CuktaSearchResultCard, CuktaSearchTarget, CuktaSemanticSearchHit,
    CuktaTargetOption, CuktaTocNode, CuktaWebMode, CuktaWebSearchState, CuktaWebState,
    CuktaWebView, DictionaryTooltipCard, FAVICON_ASSET_PATH, GIMFIHI_MAX_COUNT, GIMFIHI_MAX_WEIGHT,
    GIMFIHI_MIN_WEIGHT, GentufaBlock, GentufaBlocksLayout, GentufaBracketFragment, GentufaCell,
    GentufaError, GentufaScript, GentufaSuccess, GentufaTreeGuide, GentufaTreeRow,
    GentufaWebOptions, GentufaWebRequest, GentufaWebResult, GentufaWebState, GentufaWebViewMode,
    GimfihiCandidate, GimfihiOutput, GimfihiPreset, GimfihiPresetOption, GimfihiWebResult,
    GimfihiWebSource, GimfihiWebState, GismuShape, MANIFEST_ASSET_PATH, PageMeta,
    RafsiAvailability, RafsiCandidate, ReferenceLabel, ReferenceMarker, ReferenceMarkerRole,
    ReferenceTooltip, ReferenceTooltipInline, ReferenceTooltipInlineData, ReferenceTooltipRow,
    VLACKU_WEB_DEFAULT_COUNT, VLACKU_WEB_MAX_COUNT, VlackuCompositionPiece,
    VlackuCompositionPieceKind, VlackuDictionaryCountNode, VlackuDictionaryInfo, VlackuInline,
    VlackuInlineData, VlackuJvozbaItem, VlackuJvozbaItemKind, VlackuJvozbaMode, VlackuJvozbaOutput,
    VlackuJvozbaSegmentTone, VlackuMath, VlackuSemanticSearchHit, VlackuVoteDisplay,
    VlackuWebAuthor, VlackuWebCard, VlackuWebMode, VlackuWebResult, VlackuWebState,
    VlackuWordTypeOption, VlackuWordTypeSection, WebComputeRequest, WebComputeResponse,
    WebFeatureAvailability, WebRoute, all_presets, build_gimfihi_page_meta_from_output,
    build_page_meta, build_vlacku_jvozba_output, dictionary_tooltip_for_rafsi,
    dictionary_tooltip_for_word, gentufa_web_url, gimfihi_web_url, normalize_gimfihi_state,
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

use std::cell::{Cell, RefCell};
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
mod storage;
use storage::*;
mod settings;
use settings::*;
mod gimfihi;
use gimfihi::*;
mod vlacku;
use vlacku::*;
mod gentufa;
use gentufa::*;

#[cfg(any(target_arch = "wasm32", test))]
mod f2llm_runtime_core;
#[cfg(target_arch = "wasm32")]
mod f2llm_webgpu_runtime;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const COMPUTE_WORKER_JS: Asset = asset!("/assets/compute-worker.js");
const EMBEDDING_WORKER_JS: Asset = asset!("/assets/embedding-worker.js");
// Worker-only ES modules imported from worker scripts; keep explicit asset pins for Dioxus.
#[allow(dead_code)]
const APP_MODULE_READY_JS: Asset = asset!("/assets/app-module-ready.js");
#[allow(dead_code)]
const MODEL_CATALOG_JS: Asset = asset!("/assets/model-catalog.js");
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
#[allow(dead_code)]
const ICON_MASKABLE_192: Asset = asset!("/assets/icons/jbotci-icon-maskable-192.png");
#[allow(dead_code)]
const ICON_MASKABLE_512: Asset = asset!("/assets/icons/jbotci-icon-maskable-512.png");
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
const COMPUTE_CHANNEL_GIMFIHI: &str = "gimfihi-page";
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
const F2LLM_160M_MODEL_KEY: &str = "f2llm-v2-160m-q4-640";
const F2LLM_330M_MODEL_KEY: &str = "f2llm-v2-330m-q4-896";
const F2LLM_0_6B_MODEL_KEY: &str = "f2llm-v2-0.6b-q4-1024";
const F2LLM_WEBGPU_RUNTIME: &str = "jbotci-webgpu-f2llm";
const F2LLM_WEBGPU_RUNTIME_VERSION: &str = "0.2.0";
const F2LLM_WASM_RUNTIME: &str = "jbotci-onnxruntime-web-f2llm";
const F2LLM_WASM_RUNTIME_VERSION: &str = "0.2.0";
const F2LLM_BROWSER_QUERY_PREFIX: &str =
    "Instruct: Given a question, retrieve passages that can help answer the question.\nQuery: ";
const F2LLM_BROWSER_POOLING: &str = "mean_normalized_windows";
const F2LLM_BROWSER_VECTOR_SPACE_KEY: &str = "jbotci-browser-f2llm-q4-f16-windowed-512-v1";
const F2LLM_BROWSER_MAX_SEQUENCE_LENGTH: usize = 512;
const F2LLM_BROWSER_LOCAL_EMBED_BATCH_SIZE: usize = 64;
const MI_B: usize = 1024 * 1024;
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

#[requires(true)]
#[ensures(!ret.is_empty())]
fn browser_embedding_model_catalog_json() -> String {
    serde_json::to_string(&serde_json::json!({
        "schemaVersion": 1,
        "defaultMobileModelKey": F2LLM_80M_MODEL_KEY,
        "defaultDesktopModelKey": F2LLM_330M_MODEL_KEY,
        "wasmFallbackModelKey": F2LLM_80M_MODEL_KEY,
        "models": {
            F2LLM_80M_MODEL_KEY: browser_embedding_model_spec_json(
                F2LLM_80M_MODEL_KEY,
                "F2LLM v2 80M",
                "codefuse-ai/F2LLM-v2-80M",
                "https://assets.jbotci.app/models/f2llm-v2-80m-webgpu/v1",
                Some("https://assets.jbotci.app/models/f2llm-v2-80m-onnx-q4/v1/model_q4.onnx"),
                320usize,
                68usize * MI_B,
                180usize * MI_B,
            ),
            F2LLM_160M_MODEL_KEY: browser_embedding_model_spec_json(
                F2LLM_160M_MODEL_KEY,
                "F2LLM v2 160M",
                "codefuse-ai/F2LLM-v2-160M",
                "https://assets.jbotci.app/models/f2llm-v2-160m-webgpu/v1",
                None,
                640usize,
                110usize * MI_B,
                260usize * MI_B,
            ),
            F2LLM_330M_MODEL_KEY: browser_embedding_model_spec_json(
                F2LLM_330M_MODEL_KEY,
                "F2LLM v2 330M",
                "codefuse-ai/F2LLM-v2-330M",
                "https://assets.jbotci.app/models/f2llm-v2-330m-webgpu/v1",
                None,
                896usize,
                231usize * MI_B,
                420usize * MI_B,
            ),
            F2LLM_0_6B_MODEL_KEY: browser_embedding_model_spec_json(
                F2LLM_0_6B_MODEL_KEY,
                "F2LLM v2 0.6B",
                "codefuse-ai/F2LLM-v2-0.6B",
                "https://assets.jbotci.app/models/f2llm-v2-0.6b-webgpu/v1",
                None,
                1024usize,
                416usize * MI_B,
                700usize * MI_B,
            ),
        },
    }))
    .expect("browser embedding model catalog is JSON-serializable")
}

#[requires(!model_key.is_empty())]
#[requires(!label.is_empty())]
#[requires(!model_id.is_empty())]
#[requires(!webgpu_artifact_base_url.is_empty())]
#[requires(dimensions > 0)]
#[requires(q4_model_bytes > 0)]
#[requires(q4_min_free_bytes > 0)]
#[ensures(ret.is_object())]
fn browser_embedding_model_spec_json(
    model_key: &str,
    label: &str,
    model_id: &str,
    webgpu_artifact_base_url: &str,
    wasm_onnx_url: Option<&str>,
    dimensions: usize,
    q4_model_bytes: usize,
    q4_min_free_bytes: usize,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "modelKey": model_key,
        "label": label,
        "modelId": model_id,
        "customRuntime": {
            "runtime": F2LLM_WEBGPU_RUNTIME,
            "version": F2LLM_WEBGPU_RUNTIME_VERSION,
            "artifactBaseUrl": webgpu_artifact_base_url,
            "dtype": "q4",
            "device": "webgpu",
        },
        "preferredRuntime": { "dtype": "q4", "device": "webgpu" },
        "dimensions": dimensions,
        "maxSequenceLength": F2LLM_BROWSER_MAX_SEQUENCE_LENGTH,
        "queryPrefix": F2LLM_BROWSER_QUERY_PREFIX,
        "remoteVectorPacks": true,
        "browserLocalIndexing": true,
        "localVectorSpaceKey": F2LLM_BROWSER_VECTOR_SPACE_KEY,
        "vectorElementType": "f16le",
        "embedBatchSize": F2LLM_BROWSER_LOCAL_EMBED_BATCH_SIZE,
        "modelSizeEstimates": { "q4": q4_model_bytes },
        "minFreeBytesByDtype": { "q4": q4_min_free_bytes },
        "outputPooling": F2LLM_BROWSER_POOLING,
    });
    if let Some(wasm_onnx_url) = wasm_onnx_url {
        value["wasmRuntime"] = serde_json::json!({
            "runtime": F2LLM_WASM_RUNTIME,
            "version": F2LLM_WASM_RUNTIME_VERSION,
            "onnxUrl": wasm_onnx_url,
            "dtype": "q4",
            "device": "wasm",
        });
    }
    value
}

thread_local! {
    static VLACKU_URL_TIMER: Cell<Option<platform::TimeoutHandle>> = const { Cell::new(None) };
    static VLACKU_SEARCH_TIMER: Cell<Option<platform::TimeoutHandle>> = const { Cell::new(None) };
    static CUKTA_SEARCH_TIMER: Cell<Option<platform::TimeoutHandle>> = const { Cell::new(None) };
}

#[cfg(target_arch = "wasm32")]
thread_local! {
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

#[invariant(width.is_finite() && *width > 0.0)]
#[invariant(height.is_finite() && *height > 0.0)]
#[invariant(!paths.is_empty())]
#[derive(Debug, Clone, PartialEq)]
struct ArrowOverlay {
    width: f64,
    height: f64,
    paths: Vec<String>,
}

#[invariant(left.is_finite() && top.is_finite() && right.is_finite() && bottom.is_finite())]
#[invariant(left <= right)]
#[invariant(top <= bottom)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct ReferenceRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
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
    link: Option<DiagnosticTextLink>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum AppRoute {
    Gentufa,
    Settings,
    Cukta,
    Vlacku,
    Gimfihi,
}

const TOPBAR_NAV_ROUTES: [AppRoute; 4] = [
    AppRoute::Cukta,
    AppRoute::Vlacku,
    AppRoute::Gentufa,
    AppRoute::Gimfihi,
];

include!("page_find.rs");

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
    #[ensures(matches!(ret.web_route, WebRoute::Vlacku(_)))]
    fn default_vlacku() -> Self {
        new!(JbotciRoute {
            web_route: WebRoute::Vlacku(VlackuWebState::default()),
            gentufa_text_explicit: false,
            settings_query: String::new(),
            hash: None,
        })
    }

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
    #[ensures(matches!(ret.web_route, WebRoute::Vlacku(_)))]
    fn default() -> Self {
        Self::default_vlacku()
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
    error_context_depth: usize,
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
#[invariant(::Gimfihi => true)]
#[invariant(::Settings => true)]
#[invariant(::Export => true)]
enum AsyncTaskKind {
    Gentufa,
    Cukta,
    Vlacku,
    Gimfihi,
    Settings,
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct AsyncActivityTask {
    id: AsyncTaskId,
    kind: AsyncTaskKind,
}

#[invariant(*next_task_id > 0)]
#[invariant(active_tasks.iter().enumerate().all(|(index, task)| {
    task.id > 0 && active_tasks.iter().skip(index + 1).all(|other| other.id != task.id)
}))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct AsyncActivityState {
    next_task_id: AsyncTaskId,
    active_tasks: Vec<AsyncActivityTask>,
}

impl Default for AsyncActivityState {
    #[requires(true)]
    #[ensures(ret.next_task_id == 1)]
    #[ensures(ret.active_tasks.is_empty())]
    fn default() -> Self {
        new!(AsyncActivityState {
            next_task_id: 1,
            active_tasks: Vec::new(),
        })
    }
}

impl AsyncActivityState {
    #[requires(self.next_task_id > 0)]
    #[ensures(ret > 0)]
    fn begin(&mut self, kind: AsyncTaskKind) -> AsyncTaskId {
        let mut data = self.clone().into_data();
        let id = data.next_task_id;
        data.next_task_id = data.next_task_id.saturating_add(1).max(1);
        data.active_tasks.push(AsyncActivityTask { id, kind });
        *self = Self::from_data(data);
        id
    }

    #[requires(task_id > 0)]
    #[ensures(true)]
    fn finish(&mut self, task_id: AsyncTaskId) -> bool {
        let mut data = self.clone().into_data();
        let Some(index) = data.active_tasks.iter().position(|task| task.id == task_id) else {
            return false;
        };
        data.active_tasks.remove(index);
        *self = Self::from_data(data);
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

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
struct GimfihiAsyncResultState {
    state: Option<GimfihiWebState>,
    result: GimfihiWebResult,
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
            error_context_depth: 1,
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

impl Default for GimfihiAsyncResultState {
    #[requires(true)]
    #[ensures(ret.state.is_none())]
    fn default() -> Self {
        let state = GimfihiWebState::default();
        Self {
            state: None,
            result: gimfihi_empty_result(&state),
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
    let initial_gimfihi = initial_gimfihi_state(&current_route_location);
    let initial_gimfihi_source_words = initial_gimfihi.clone();
    let gimfihi_source_word_memory =
        use_signal(move || gimfihi_source_word_memory_from_state(&initial_gimfihi_source_words));
    let gimfihi_draft_state = use_signal(|| initial_gimfihi.clone());
    let gimfihi_committed_state = use_signal(|| initial_gimfihi);
    let gimfihi_result = use_signal(GimfihiAsyncResultState::default);
    let gimfihi_result_cache = use_signal(BTreeMap::<String, GimfihiAsyncResultState>::new);
    let gimfihi_result_task = use_signal(|| None::<LatestAsyncTask>);
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
    let gimfihi_committed_state_value = gimfihi_committed_state.read().clone();
    let gimfihi_result_value = gimfihi_result.read().clone();
    let page_find_state_value = page_find_state.read().clone();
    let current_page_find_route_state = page_find_state_value.route_state(route_value).clone();
    let page_find_entries = if current_page_find_route_state.query.is_empty() {
        Vec::new()
    } else {
        page_find_entries_for_route(
            route_value,
            &cukta_page_value,
            &vlacku_committed_state_value,
            &vlacku_result_value,
            &gimfihi_committed_state_value,
            &gimfihi_result_value,
            &result,
            gentufa_request.as_ref(),
            view_mode_value,
            gentufa_display_value,
            settings_value,
            &dialect_settings_value,
            &settings_dialect_selection_value,
            &embedding_settings_value,
            settings_value.script,
        )
    };
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
    let topbar_gimfihi_route = JbotciRoute::from_web_route(
        WebRoute::Gimfihi(gimfihi_committed_state_value.clone()),
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
            gimfihi_draft_state,
            gimfihi_committed_state,
            gimfihi_source_word_memory,
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
        configure_embedding_model_catalog();
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
    let gimfihi_page_base_path = base_path.clone();
    use_effect(move || {
        if *route.read() != AppRoute::Gimfihi {
            cancel_compute_channel(COMPUTE_CHANNEL_GIMFIHI);
            cancel_latest_task(gimfihi_result_task);
            return;
        }
        let state = gimfihi_committed_state.read().clone();
        if !gimfihi_state_has_any_source_word(&state) {
            cancel_compute_channel(COMPUTE_CHANNEL_GIMFIHI);
            cancel_latest_task(gimfihi_result_task);
            let mut idle_result_signal = gimfihi_result;
            idle_result_signal.set(gimfihi_idle_result_state(&state));
            return;
        }
        let cache_key = gimfihi_generation_cache_key(&state);
        if let Some(cached) = gimfihi_result_cache.read().get(&cache_key).cloned()
            && let Some(cached_result) =
                gimfihi_cached_result_for_state(&gimfihi_page_base_path, &state, cached)
        {
            cancel_compute_channel(COMPUTE_CHANNEL_GIMFIHI);
            cancel_latest_task(gimfihi_result_task);
            if let Some(meta) = cached_result.meta.clone() {
                apply_document_meta(document_meta, meta);
            }
            let mut cached_result_signal = gimfihi_result;
            cached_result_signal.set(cached_result);
            return;
        }
        let mut page_signal = gimfihi_result;
        page_signal.with_mut(|page| {
            page.state = Some(state.clone());
            page.loading = true;
            page.error = None;
        });
        let mut result_signal = gimfihi_result;
        let mut cache_signal = gimfihi_result_cache;
        let request = WebComputeRequest::GimfihiPage {
            base_path: gimfihi_page_base_path.clone(),
            state: state.clone(),
        };
        cancel_compute_channel(COMPUTE_CHANNEL_GIMFIHI);
        spawn_latest_tracked(
            gimfihi_result_task,
            activity,
            AsyncTaskKind::Gimfihi,
            async move {
                let response = compute_request(COMPUTE_CHANNEL_GIMFIHI, request).await;
                match response {
                    Ok(WebComputeResponse::GimfihiPage { result, meta }) => {
                        let next = GimfihiAsyncResultState {
                            state: Some(state),
                            result,
                            meta: Some(meta.clone()),
                            loading: false,
                            error: None,
                        };
                        cache_signal.with_mut(|cache| {
                            cache.insert(cache_key, next.clone());
                            while cache.len() > 16 {
                                if let Some(first_key) = cache.keys().next().cloned() {
                                    cache.remove(&first_key);
                                } else {
                                    break;
                                }
                            }
                        });
                        result_signal.set(next);
                        apply_document_meta(document_meta, meta);
                    }
                    Ok(_) => {
                        result_signal.set(gimfihi_async_error_state(
                            &state,
                            "compute worker returned the wrong gimfihi response",
                        ));
                    }
                    Err(error) => {
                        result_signal.set(gimfihi_async_error_state(&state, &error));
                    }
                }
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
    let gimfihi_url_route_location = current_route_location.clone();
    let gimfihi_url_history = app_history.clone();
    use_effect(move || {
        if *route.read() == AppRoute::Gimfihi {
            let state = gimfihi_committed_state.read().clone();
            push_gimfihi_url(
                gimfihi_url_history.clone(),
                pending_local_route_writes,
                &gimfihi_url_route_location,
                &state,
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
    });
    use_effect(move || {
        let _ = (*route.read(), *topbar_nav_layout.read());
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
                topbar_gimfihi_route,
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
                            AppRoute::Gimfihi => {
                                render_gimfihi_page(
                                    gimfihi_draft_state,
                                    gimfihi_committed_state,
                                    gimfihi_result,
                                    gimfihi_source_word_memory,
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
    gimfihi_route: JbotciRoute,
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
    let gimfihi_loading = activity_visible && activity.has_kind(AsyncTaskKind::Gimfihi);
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
                            { render_topbar_nav(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gimfihi_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
                        }
                        TopbarNavLayout::Carousel => {
                            { render_topbar_nav_carousel(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gimfihi_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
                        }
                    }
                }
                { render_topbar_fit_probes(
                    settings,
                    current,
                    route,
                    cukta_loading,
                    vlacku_loading,
                    gimfihi_loading,
                    gentufa_loading,
                    cukta_route,
                    vlacku_route,
                    gimfihi_route,
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
    gimfihi_loading: bool,
    gentufa_loading: bool,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gimfihi_route: JbotciRoute,
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
            Link {
                class: topbar_link_class(route == AppRoute::Gimfihi, gimfihi_loading),
                to: gimfihi_route,
                aria_current: if route == AppRoute::Gimfihi { "page" } else { "false" },
                span { class: "app-topbar-link-label", "gimfi'i" }
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
    gimfihi_loading: bool,
    gentufa_loading: bool,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gimfihi_route: JbotciRoute,
    gentufa_route: JbotciRoute,
    base_path: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
) -> Element {
    let [first_route, second_route, third_route, fourth_route] = topbar_carousel_routes(route);
    rsx! {
        nav { class: "spa-nav app-topbar-nav-carousel", aria_label: "Primary navigation",
            div { class: "app-topbar-nav-carousel-track",
                { render_topbar_nav_carousel_link(
                    first_route,
                    route,
                    topbar_carousel_route_slot_class(first_route, route),
                    cukta_loading,
                    vlacku_loading,
                    gimfihi_loading,
                    gentufa_loading,
                    cukta_route.clone(),
                    vlacku_route.clone(),
                    gimfihi_route.clone(),
                    gentufa_route.clone(),
                    base_path,
                    pending_cukta_scroll,
                ) }
                { render_topbar_nav_carousel_link(
                    second_route,
                    route,
                    topbar_carousel_route_slot_class(second_route, route),
                    cukta_loading,
                    vlacku_loading,
                    gimfihi_loading,
                    gentufa_loading,
                    cukta_route.clone(),
                    vlacku_route.clone(),
                    gimfihi_route.clone(),
                    gentufa_route.clone(),
                    base_path,
                    pending_cukta_scroll,
                ) }
                { render_topbar_nav_carousel_link(
                    third_route,
                    route,
                    topbar_carousel_route_slot_class(third_route, route),
                    cukta_loading,
                    vlacku_loading,
                    gimfihi_loading,
                    gentufa_loading,
                    cukta_route.clone(),
                    vlacku_route.clone(),
                    gimfihi_route.clone(),
                    gentufa_route.clone(),
                    base_path,
                    pending_cukta_scroll,
                ) }
                { render_topbar_nav_carousel_link(
                    fourth_route,
                    route,
                    topbar_carousel_route_slot_class(fourth_route, route),
                    cukta_loading,
                    vlacku_loading,
                    gimfihi_loading,
                    gentufa_loading,
                    cukta_route,
                    vlacku_route,
                    gimfihi_route,
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
    gimfihi_loading: bool,
    gentufa_loading: bool,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gimfihi_route: JbotciRoute,
    gentufa_route: JbotciRoute,
    base_path: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
) -> Element {
    let active = target == active_route;
    let loading = topbar_carousel_route_loading(
        target,
        cukta_loading,
        vlacku_loading,
        gimfihi_loading,
        gentufa_loading,
    );
    let class = topbar_carousel_link_class(active, loading, slot_class);
    let aria_current = if active { "page" } else { "false" };
    let data_active = if active { "true" } else { "false" };
    let label = topbar_carousel_route_label(target);
    let target_route = match target {
        AppRoute::Cukta => cukta_route,
        AppRoute::Vlacku => vlacku_route,
        AppRoute::Gimfihi => gimfihi_route,
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
    gimfihi_loading: bool,
    gentufa_loading: bool,
) -> Element {
    let [first_route, second_route, third_route, fourth_route] = topbar_carousel_routes(route);
    let first_label = topbar_carousel_route_label(first_route);
    let second_label = topbar_carousel_route_label(second_route);
    let third_label = topbar_carousel_route_label(third_route);
    let fourth_label = topbar_carousel_route_label(fourth_route);
    rsx! {
        nav { class: "spa-nav app-topbar-nav-carousel", aria_label: "Primary navigation",
            div { class: "app-topbar-nav-carousel-track",
                span {
                    class: topbar_carousel_link_class(
                        first_route == route,
                        topbar_carousel_route_loading(first_route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading),
                        topbar_carousel_route_slot_class(first_route, route),
                    ),
                    "data-topbar-nav-active": if first_route == route { "true" } else { "false" },
                    span { class: "app-topbar-link-label", "{first_label}" }
                }
                span {
                    class: topbar_carousel_link_class(
                        second_route == route,
                        topbar_carousel_route_loading(second_route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading),
                        topbar_carousel_route_slot_class(second_route, route),
                    ),
                    "data-topbar-nav-active": if second_route == route { "true" } else { "false" },
                    span { class: "app-topbar-link-label", "{second_label}" }
                }
                span {
                    class: topbar_carousel_link_class(
                        third_route == route,
                        topbar_carousel_route_loading(third_route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading),
                        topbar_carousel_route_slot_class(third_route, route),
                    ),
                    "data-topbar-nav-active": if third_route == route { "true" } else { "false" },
                    span { class: "app-topbar-link-label", "{third_label}" }
                }
                span {
                    class: topbar_carousel_link_class(
                        fourth_route == route,
                        topbar_carousel_route_loading(fourth_route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading),
                        topbar_carousel_route_slot_class(fourth_route, route),
                    ),
                    "data-topbar-nav-active": if fourth_route == route { "true" } else { "false" },
                    span { class: "app-topbar-link-label", "{fourth_label}" }
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
#[ensures(route == AppRoute::Settings || ret.contains(&route))]
#[ensures(ret[0] == AppRoute::Cukta)]
#[ensures(ret[3] == AppRoute::Gimfihi)]
fn topbar_carousel_routes(route: AppRoute) -> [AppRoute; 4] {
    let _ = route;
    TOPBAR_NAV_ROUTES
}

#[requires(target != AppRoute::Settings)]
#[ensures(!ret.is_empty())]
fn topbar_carousel_route_slot_class(target: AppRoute, active_route: AppRoute) -> &'static str {
    if target == active_route {
        "is-current-slot"
    } else {
        "is-adjacent"
    }
}

#[requires(route != AppRoute::Settings)]
#[ensures(true)]
fn topbar_carousel_route_loading(
    route: AppRoute,
    cukta_loading: bool,
    vlacku_loading: bool,
    gimfihi_loading: bool,
    gentufa_loading: bool,
) -> bool {
    match route {
        AppRoute::Cukta => cukta_loading,
        AppRoute::Vlacku => vlacku_loading,
        AppRoute::Gimfihi => gimfihi_loading,
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
        AppRoute::Gimfihi => "gimfi'i",
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
        AppRoute::Gimfihi => "Find in candidates",
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
    gimfihi_loading: bool,
    gentufa_loading: bool,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gimfihi_route: JbotciRoute,
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
                { render_topbar_nav(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gimfihi_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-theme-full",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                span { class: "app-topbar-theme app-topbar-theme-mode",
                    { render_theme_switch(settings, current.theme) }
                }
                { render_topbar_nav(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gimfihi_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-none-full",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                { render_topbar_nav(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gimfihi_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
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
                { render_topbar_nav_carousel_probe(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-theme-carousel",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                span { class: "app-topbar-theme app-topbar-theme-mode",
                    { render_theme_switch(settings, current.theme) }
                }
                { render_topbar_nav_carousel_probe(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-none-carousel",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                { render_topbar_nav_carousel_probe(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading) }
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

    #[wasm_bindgen(js_name = jbotciEmbeddingConfigureCatalog)]
    fn js_embedding_configure_catalog(catalog_json: &str);

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

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = jbotciWorkerReady)]
#[requires(true)]
#[ensures(true)]
pub fn jbotci_worker_ready() -> js_sys::Promise {
    js_sys::Promise::resolve(&JsValue::UNDEFINED)
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
#[requires(true)]
#[ensures(true)]
fn configure_embedding_model_catalog() {
    js_embedding_configure_catalog(&browser_embedding_model_catalog_json());
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn configure_embedding_model_catalog() {}

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
        match target {
            CuktaSearchTarget::Section => push_unique_filter(&mut filters, "section"),
            CuktaSearchTarget::Paragraph => push_unique_filter(&mut filters, "paragraph"),
            CuktaSearchTarget::Example => push_unique_filter(&mut filters, "example"),
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

#[requires(true)]
#[ensures(ret.errors.is_empty())]
fn gimfihi_empty_result(state: &GimfihiWebState) -> GimfihiWebResult {
    let state = normalize_gimfihi_state(state);
    GimfihiWebResult {
        preset_options: gimfihi_preset_options_for_state(&state),
        language_suggestions: gimfihi_language_suggestions(),
        state,
        output: None,
        errors: Vec::new(),
    }
}

#[requires(true)]
#[ensures(ret.state.as_ref().is_some_and(|current| current == state))]
#[ensures(!ret.loading)]
fn gimfihi_idle_result_state(state: &GimfihiWebState) -> GimfihiAsyncResultState {
    GimfihiAsyncResultState {
        state: Some(state.clone()),
        result: gimfihi_empty_result(state),
        meta: None,
        loading: false,
        error: None,
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

#[requires(!message.is_empty())]
#[ensures(ret.error.as_ref().is_some_and(|error| error == message))]
fn gimfihi_async_error_state(state: &GimfihiWebState, message: &str) -> GimfihiAsyncResultState {
    GimfihiAsyncResultState {
        state: Some(state.clone()),
        result: gimfihi_empty_result(state),
        meta: None,
        loading: false,
        error: Some(message.to_owned()),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn gimfihi_generation_cache_key(state: &GimfihiWebState) -> String {
    let mut key_state = normalize_gimfihi_state(state);
    key_state.highlight = None;
    gimfihi_web_url("", &key_state)
}

#[requires(true)]
#[ensures(true)]
fn gimfihi_cached_result_for_state(
    base_path: &str,
    state: &GimfihiWebState,
    cached: GimfihiAsyncResultState,
) -> Option<GimfihiAsyncResultState> {
    let normalized = normalize_gimfihi_state(state);
    let output = cached.result.output.as_ref()?;
    if let Some(highlight) = normalized.highlight.as_deref()
        && output.winner.as_deref() != Some(highlight)
        && !output
            .candidates
            .iter()
            .any(|candidate| candidate.word == highlight)
    {
        return None;
    }
    let highlighted_output = gimfihi_output_with_highlight(output, normalized.highlight.as_deref());
    let result = GimfihiWebResult {
        state: normalized.clone(),
        output: Some(highlighted_output.clone()),
        preset_options: gimfihi_preset_options_for_state(&normalized),
        language_suggestions: gimfihi_language_suggestions(),
        errors: cached.result.errors.clone(),
    };
    Some(GimfihiAsyncResultState {
        state: Some(normalized.clone()),
        result,
        meta: Some(build_gimfihi_page_meta_from_output(
            base_path,
            &normalized,
            &highlighted_output,
        )),
        loading: false,
        error: None,
    })
}

#[requires(true)]
#[ensures(true)]
fn gimfihi_result_state_with_highlight(
    base_path: &str,
    state: &GimfihiWebState,
    current: &GimfihiAsyncResultState,
) -> Option<GimfihiAsyncResultState> {
    let normalized = normalize_gimfihi_state(state);
    let output = current.result.output.as_ref()?;
    let highlight = normalized.highlight.as_deref()?;
    if output.winner.as_deref() != Some(highlight)
        && !output
            .candidates
            .iter()
            .any(|candidate| candidate.word == highlight)
    {
        return None;
    }
    let highlighted_output = gimfihi_output_with_highlight(output, Some(highlight));
    let result = GimfihiWebResult {
        state: normalized.clone(),
        output: Some(highlighted_output.clone()),
        preset_options: gimfihi_preset_options_for_state(&normalized),
        language_suggestions: gimfihi_language_suggestions(),
        errors: current.result.errors.clone(),
    };
    Some(GimfihiAsyncResultState {
        state: Some(normalized.clone()),
        result,
        meta: Some(build_gimfihi_page_meta_from_output(
            base_path,
            &normalized,
            &highlighted_output,
        )),
        loading: false,
        error: None,
    })
}

#[requires(true)]
#[ensures(ret.candidates.len() == output.candidates.len())]
fn gimfihi_output_with_highlight(output: &GimfihiOutput, highlight: Option<&str>) -> GimfihiOutput {
    let requested = highlight
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);
    let selected = requested
        .filter(|value| {
            output
                .candidates
                .iter()
                .any(|candidate| &candidate.word == value)
        })
        .or_else(|| output.winner.clone());
    let mut next = output.clone();
    next.highlighted_word = selected.clone();
    for candidate in &mut next.candidates {
        candidate.highlighted = selected
            .as_ref()
            .is_some_and(|highlighted| highlighted == &candidate.word);
    }
    next
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
    let site = embedded_cll_site().ok();
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
                        { render_cll_block(site, block, pending_cukta_scroll, base_path, script, page_find) }
                    }
                }
            }
            for block in blocks.iter() {
                { render_cll_block(site, block, pending_cukta_scroll, base_path, script, page_find) }
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
    site: Option<&jbotci_cll::CllSite>,
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
                                    { render_cll_block(site, child, pending_cukta_scroll, base_path, script, page_find) }
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
                                    { render_cll_block(site, child, pending_cukta_scroll, base_path, script, page_find) }
                                }
                            }
                        }
                    }
                }
            }
        }
        CllBlock::Example { example_id } => {
            if let Some(example) =
                site.and_then(|site| jbotci_cll::cll_lookup_example(site, example_id))
            {
                rsx! {
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
                                        let kind = line.kind.as_str();
                                        let text = cll_display_text_for_kind(script, kind, &line.text);
                                        rsx! { p { class: "cll-ig-line cll-ig-{kind}", { render_page_find_text(page_find, &text) } } }
                                    }
                                }
                            }
                        } else {
                            for child in example.blocks.iter() {
                                { render_cll_block(site, child, pending_cukta_scroll, base_path, script, page_find) }
                            }
                        }
                    }
                }
            } else {
                rsx! {}
            }
        }
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
                                            { render_cll_block(site, child, pending_cukta_scroll, base_path, script, page_find) }
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
                                        { render_cll_block(site, child, pending_cukta_scroll, base_path, script, page_find) }
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
                            { render_cll_block(site, child, pending_cukta_scroll, base_path, script, page_find) }
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
                        { render_cll_block(site, child, pending_cukta_scroll, base_path, script, page_find) }
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
                    { render_cll_block(site, child, pending_cukta_scroll, base_path, script, page_find) }
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
                                    let kind = row.kind.as_str();
                                    let row_context = row.kind.is_lojban();
                                    rsx! {
                                        tr { class: "cll-ig-row cll-ig-{kind} cll-interlinear-row cll-interlinear-row-{kind}",
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
                                let kind = row.kind.as_str();
                                let row_context = row.kind.is_lojban();
                                rsx! {
                                    div { class: "cll-ig-line-wrap",
                                        p { class: "cll-ig-line cll-ig-inline cll-ig-{kind}",
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
                        let kind = line.kind.as_str();
                        let line_context = line.kind.is_lojban();
                        rsx! {
                            tr { class: "cll-lojbanization-row cll-lojbanization-line cll-lojbanization-line-{kind} cll-lojbanization-{kind}",
                                th { { render_page_find_text(page_find, kind) } }
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
                    let kind = part.kind.as_str();
                    let part_context = part.kind.is_lojban();
                        rsx! {
                            li { class: "cll-lujvo-part cll-lujvo-part-{kind}",
                            span { class: "cll-lujvo-part-kind",
                                { render_page_find_text(page_find, kind) }
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
fn cukta_draft_target_options(selected_targets: &[CuktaSearchTarget]) -> Vec<CuktaTargetOption> {
    [
        (CuktaSearchTarget::Section, "Sections"),
        (CuktaSearchTarget::Paragraph, "Paragraphs"),
        (CuktaSearchTarget::Example, "Examples"),
    ]
    .iter()
    .map(|(target, label)| CuktaTargetOption {
        value: target.as_str().to_owned(),
        label: (*label).to_owned(),
        selected: selected_targets.iter().any(|selected| selected == target),
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
        error_context_depth: settings.error_context_depth,
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
fn set_error_context_depth(settings: &mut Signal<UserSettings>, depth: usize) {
    let mut next = *settings.read();
    next.error_context_depth = depth;
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
fn initial_gimfihi_state(route: &JbotciRoute) -> GimfihiWebState {
    if let WebRoute::Gimfihi(state) = &route.web_route {
        state.clone()
    } else {
        GimfihiWebState::default()
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
        WebRoute::Gimfihi(_) => AppRoute::Gimfihi,
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
    let position = platform::place_tooltip(
        platform::Rect {
            left: host_rect.left(),
            top: host_rect.top(),
            width: host_rect.width().max(0.0),
            height: host_rect.height().max(0.0),
        },
        platform::Size {
            width: tooltip_rect.width(),
            height: tooltip_rect.height(),
        },
        platform::Viewport {
            top: viewport_top,
            width: viewport_width,
            height: viewport_height,
        },
        DICTIONARY_TOOLTIP_VIEWPORT_MARGIN_PX,
        DICTIONARY_TOOLTIP_HOST_GAP_PX,
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
    tooltip_size: platform::Size,
    viewport: platform::Viewport,
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
            let position = platform::place_tooltip(
                platform_rect_from_reference_rect(measure.host_rect),
                measure.tooltip_size,
                measure.viewport,
                DICTIONARY_TOOLTIP_VIEWPORT_MARGIN_PX,
                DICTIONARY_TOOLTIP_HOST_GAP_PX,
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
        || is_gimfihi_route_path_for_client(path)
        || path == "/settings"
        || path.starts_with("/settings/")
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn is_gimfihi_route_path_for_client(path: &str) -> bool {
    matches!(path, "/gimfihi" | "/gimfi'i" | "/gimfi%27i")
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
    mut gimfihi_draft_state: Signal<GimfihiWebState>,
    mut gimfihi_committed_state: Signal<GimfihiWebState>,
    mut gimfihi_source_word_memory: Signal<BTreeMap<String, String>>,
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
        WebRoute::Gimfihi(state) => {
            gimfihi_source_word_memory.with_mut(|memory| {
                update_gimfihi_source_word_memory(memory, state);
            });
            gimfihi_draft_state.set(state.clone());
            gimfihi_committed_state.set(state.clone());
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

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn schedule_scroll_container_to_y(_y: i32) {}

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

#[requires(true)]
#[ensures(true)]
fn schedule_vlacku_search_commit(
    mut committed_state: Signal<VlackuWebState>,
    state: VlackuWebState,
) {
    clear_vlacku_url_timer();
    clear_vlacku_search_timer();
    if let Some(handle) = platform::schedule_timeout_once(VLACKU_SEARCH_DEBOUNCE_MS, move || {
        committed_state.set(state);
    }) {
        VLACKU_SEARCH_TIMER.with(|timer| timer.set(Some(handle)));
    }
}

#[requires(true)]
#[ensures(true)]
fn schedule_cukta_search_commit(mut committed_state: Signal<CuktaWebState>, state: CuktaWebState) {
    clear_cukta_search_timer();
    if let Some(handle) = platform::schedule_timeout_once(CUKTA_SEARCH_DEBOUNCE_MS, move || {
        committed_state.set(state);
    }) {
        CUKTA_SEARCH_TIMER.with(|timer| timer.set(Some(handle)));
    }
}

#[requires(true)]
#[ensures(true)]
fn clear_vlacku_search_timer() {
    VLACKU_SEARCH_TIMER.with(|timer| {
        if let Some(handle) = timer.replace(None) {
            platform::clear_timeout(handle);
        }
    });
}

#[requires(true)]
#[ensures(true)]
fn clear_cukta_search_timer() {
    CUKTA_SEARCH_TIMER.with(|timer| {
        if let Some(handle) = timer.replace(None) {
            platform::clear_timeout(handle);
        }
    });
}

#[requires(true)]
#[ensures(true)]
fn clear_vlacku_url_timer() {
    VLACKU_URL_TIMER.with(|timer| {
        if let Some(handle) = timer.replace(None) {
            platform::clear_timeout(handle);
        }
    });
}

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

#[requires(true)]
#[ensures(true)]
fn schedule_route_push(
    history: Rc<dyn History>,
    pending_writes: Signal<PendingLocalRouteWrites>,
    target: JbotciRoute,
    delay_ms: i32,
    restore_scroll_y: Option<i32>,
) {
    clear_vlacku_url_timer();
    if let Some(handle) = platform::schedule_timeout_once(delay_ms, move || {
        let mut pending_writes = pending_writes;
        pending_writes.with_mut(|pending| pending.record(&target));
        history.push(route_path_for_route(&target));
        if let Some(y) = restore_scroll_y {
            schedule_scroll_container_to_y(y);
        }
    }) {
        VLACKU_URL_TIMER.with(|timer| timer.set(Some(handle)));
    }
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

#[requires(true)]
#[ensures(true)]
fn push_gimfihi_url(
    history: Rc<dyn History>,
    mut pending_writes: Signal<PendingLocalRouteWrites>,
    current: &JbotciRoute,
    state: &GimfihiWebState,
) {
    let target = JbotciRoute::from_web_route(WebRoute::Gimfihi(state.clone()), false);
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
        .unwrap_or_else(|| "/vlacku".to_owned())
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.starts_with('/'))]
fn current_path() -> String {
    "/vlacku".to_owned()
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
    let layout = platform::compute_jvozba_pane_layout(
        anchor_top,
        app_scroll_top,
        fallback_top,
        topbar_bottom,
        viewport_height,
        app_scrollbar_gutter_width,
        VLACKU_JVOZBA_HEIGHT_SCALE,
    );
    let style = pane.style();
    let _ = style.set_property("--jvozba-pane-top", &format!("{}px", layout.top));
    let _ = style.set_property("--jvozba-pane-bottom", &format!("{}px", layout.bottom));
    let _ = style.set_property("--jvozba-pane-height", &format!("{}px", layout.height));
    let _ = style.set_property(
        "--app-scrollbar-gutter-width",
        &format!("{}px", layout.scrollbar_gutter_width),
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
#[requires(true)]
#[ensures(ret.top >= metrics.topbar_bottom)]
fn jvozba_pane_layout_from_metrics(metrics: JvozbaPaneMetrics) -> platform::JvozbaPaneLayout {
    let fallback_top = metrics
        .form_bottom
        .unwrap_or(metrics.topbar_bottom)
        .max(metrics.topbar_bottom)
        + 12.0;
    platform::compute_jvozba_pane_layout(
        metrics.anchor_top,
        metrics.app_scroll_top,
        fallback_top,
        metrics.topbar_bottom,
        metrics.viewport_height,
        metrics.app_scrollbar_gutter_width,
        VLACKU_JVOZBA_HEIGHT_SCALE,
    )
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
async fn apply_vlacku_jvozba_pane_layout_desktop(layout: platform::JvozbaPaneLayout) {
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
            pane.style.setProperty("--app-scrollbar-gutter-width", `${{Number(layout.scrollbar_gutter_width)}}px`);
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

#[requires(true)]
#[ensures(ret.gentufa)]
fn _feature_availability_for_linking() -> WebFeatureAvailability {
    WebFeatureAvailability::default()
}

#[cfg(test)]
mod tests;
