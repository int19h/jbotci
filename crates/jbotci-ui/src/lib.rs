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
mod cukta;
use cukta::*;
mod diagnostics;
use diagnostics::*;
mod routing;
use routing::*;
mod shell;
use shell::*;
mod layout;
use layout::*;

include!("page_find.rs");

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

#[requires(true)]
#[ensures(ret.gentufa)]
fn _feature_availability_for_linking() -> WebFeatureAvailability {
    WebFeatureAvailability::default()
}

#[cfg(test)]
mod tests;
