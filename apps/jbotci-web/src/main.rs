use dioxus::core::Task;
use dioxus::prelude::*;
use jbotci_cll::{
    CllBlock, CllEbnfEntry, CllEbnfToken, CllInline, CllInterlinearRow, CllLanguageSpanKind,
    CllLinkKind, CllLojbanizationLine, CllLujvoPart, CllSimpleListOrientation, cll_link_href,
    embedded_cll_site, wrap_ebnf_choice_lines,
};
use jbotci_output::{GlideMark, PhonemeRenderOptions, StressMark};
use jbotci_web_core::{
    CUKTA_WEB_DEFAULT_COUNT, CUKTA_WEB_MAX_COUNT, CuktaModeOption, CuktaPageData, CuktaPageKind,
    CuktaSearchResultCard, CuktaSemanticSearchHit, CuktaTargetOption, CuktaTocNode, CuktaWebMode,
    CuktaWebSearchState, CuktaWebState, CuktaWebView, DictionaryTooltipCard, GentufaBlock,
    GentufaBlocksLayout, GentufaBracketFragment, GentufaCell, GentufaError, GentufaScript,
    GentufaSuccess, GentufaTreeGuide, GentufaTreeRow, GentufaWebOptions, GentufaWebRequest,
    GentufaWebResult, GentufaWebState, GentufaWebViewMode, PageMeta, ReferenceLabel,
    ReferenceMarker, ReferenceMarkerRole, VLACKU_WEB_DEFAULT_COUNT, VLACKU_WEB_MAX_COUNT,
    VlackuCompositionPiece, VlackuCompositionPieceKind, VlackuDictionaryCountNode,
    VlackuDictionaryInfo, VlackuInline, VlackuInlineData, VlackuJvozbaItem, VlackuJvozbaItemKind,
    VlackuJvozbaMode, VlackuJvozbaOutput, VlackuJvozbaSegmentTone, VlackuMath, VlackuMathPart,
    VlackuMathPartData, VlackuSemanticSearchHit, VlackuVoteDisplay, VlackuWebCard, VlackuWebMode,
    VlackuWebResult, VlackuWebState, VlackuWordTypeOption, VlackuWordTypeSection,
    WebComputeRequest, WebComputeResponse, WebFeatureAvailability, WebRoute, build_page_meta,
    build_vlacku_jvozba_output, cukta_web_url, dictionary_tooltip_for_rafsi,
    dictionary_tooltip_for_word, gentufa_web_url, normalize_vlacku_state, parse_cukta_web_route,
    parse_gentufa_web_route, parse_vlacku_web_route, parse_web_route, reference_slot_display_text,
    toggle_cukta_target_selection, toggle_vlacku_word_type_selection,
    vlacku_brivla_filter_indeterminate, vlacku_web_url, vlacku_word_type_options, web_route_url,
};

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use std::cell::Cell;
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;
use std::future::Future;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const COMPUTE_JS: Asset = asset!("/assets/compute.js");
const COMPUTE_WORKER_JS: Asset = asset!("/assets/compute-worker.js");
const EMBEDDINGS_JS: Asset = asset!("/assets/embeddings.js");
const EMBEDDING_WORKER_JS: Asset = asset!("/assets/embedding-worker.js");
const MANIFEST: Asset = asset!("/assets/manifest.webmanifest");
const LOGO: Asset = asset!("/assets/icons/jbotci-dark.svg");
const FAVICON: Asset = asset!("/assets/icons/jbotci-icon-192.png");
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
const VLACKU_URL_DEBOUNCE_MS: i32 = 450;
const GENTUFA_URL_DEBOUNCE_MS: i32 = 650;
const COMPUTE_CHANNEL_GENTUFA: &str = "gentufa-page";
const COMPUTE_CHANNEL_CUKTA: &str = "cukta-page";
const COMPUTE_CHANNEL_VLACKU: &str = "vlacku-page";
const COMPUTE_CHANNEL_EMBEDDINGS: &str = "embedding-corpus";
const COMPUTE_CHANNEL_EXPORT: &str = "gentufa-export";
const ASYNC_ACTIVITY_INDICATOR_DELAY_MS: i32 = 100;
const SEMANTIC_LOADING_MESSAGE_DELAY_MS: i32 = 100;
#[cfg(target_arch = "wasm32")]
const VLACKU_JVOZBA_MIN_WIDTH_PX: f64 = 981.0;
#[cfg(target_arch = "wasm32")]
const VLACKU_JVOZBA_HEIGHT_SCALE: f64 = 0.5;
#[cfg(target_arch = "wasm32")]
const VLACKU_JVOZBA_LAYOUT_FRAME_PASSES: u8 = 2;
#[cfg(target_arch = "wasm32")]
const GENTUFA_BLOCK_REFERENCE_LAYOUT_DELAY_MS: i32 = 30;
#[cfg(target_arch = "wasm32")]
const GENTUFA_BLOCK_REFERENCE_LAYOUT_FRAME_PASSES: u8 = 2;
#[cfg(target_arch = "wasm32")]
const GENTUFA_TREE_LAYOUT_DELAY_MS: i32 = 30;
#[cfg(target_arch = "wasm32")]
const GENTUFA_TREE_LAYOUT_FRAME_PASSES: u8 = 2;
const BLOCK_REFERENCE_LABEL_GAP_PX: f64 = 8.0;
const BLOCK_REFERENCE_CONTAINMENT_GAP_PX: f64 = 1.0;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static VLACKU_URL_TIMER: Cell<Option<i32>> = const { Cell::new(None) };
    static VLACKU_SEARCH_TIMER: Cell<Option<i32>> = const { Cell::new(None) };
    static GENTUFA_URL_TIMER: Cell<Option<i32>> = const { Cell::new(None) };
    static BROWSER_STATE_HANDLERS_INSTALLED: Cell<bool> = const { Cell::new(false) };
}

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

#[derive(Debug, Clone, Default, PartialEq)]
#[invariant(true)]
struct ReferenceHoverState {
    hovered: Option<HoveredReference>,
    overlay: Option<ArrowOverlay>,
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

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy, PartialEq)]
#[invariant(true)]
struct ReferenceRect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum AppRoute {
    Gentufa,
    Settings,
    Cukta,
    Vlacku,
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
struct EmbeddingSettingsState {
    status: String,
    detail: String,
    model_size: String,
    index_size: String,
    progress_label: Option<String>,
    progress_percent: Option<u8>,
    busy: bool,
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
        Self {
            status: "unknown".to_owned(),
            detail: "Checking browser embedding storage.".to_owned(),
            model_size: "unknown".to_owned(),
            index_size: "unknown".to_owned(),
            progress_label: None,
            progress_percent: None,
            busy: false,
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

#[requires(true)]
#[ensures(true)]
fn main() {
    dioxus::launch(App);
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

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
fn App() -> Element {
    let route = use_signal(route_from_current_path);
    let base_path = base_path_from_current_path();
    let settings = use_signal(load_settings);
    let embedding_settings = use_signal(EmbeddingSettingsState::default);
    let activity = use_signal(AsyncActivityState::default);
    let activity_indicator_visible = use_signal(|| false);
    let activity_indicator_delay_task = use_signal(|| None::<Task>);
    let topbar_settings_layout = use_signal(|| TopbarSettingsLayout::BothInline);
    let topbar_settings_open = use_signal(|| false);
    let initial_gentufa = initial_gentufa_state();
    let initial_gentufa_has_text = initial_gentufa_text_explicit();
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
    let mut gentufa_text_explicit = use_signal(move || initial_gentufa_has_text);
    let initial_cukta = initial_cukta_state();
    let cukta_state = use_signal(|| initial_cukta);
    let cukta_toc_filter = use_signal(String::new);
    let cukta_toc_pinned = use_signal(load_cukta_toc_pinned);
    let cukta_toc_expansion = use_signal(load_cukta_toc_expansion);
    let cukta_toc_width = use_signal(load_cukta_toc_width);
    let cukta_toc_resize = use_signal(|| None::<CuktaTocResizeState>);
    let initial_vlacku = initial_vlacku_state();
    let vlacku_draft_state = use_signal(|| initial_vlacku.clone());
    let vlacku_committed_state = use_signal(|| initial_vlacku);
    let vlacku_semantic_result = use_signal(VlackuSemanticResultState::default);
    let vlacku_result = use_signal(VlackuAsyncResultState::default);
    let vlacku_result_task = use_signal(|| None::<LatestAsyncTask>);
    let vlacku_semantic_task = use_signal(|| None::<LatestAsyncTask>);
    let cukta_semantic_result = use_signal(CuktaSemanticResultState::default);
    let cukta_page = use_signal(CuktaAsyncPageState::default);
    let cukta_page_task = use_signal(|| None::<LatestAsyncTask>);
    let cukta_semantic_task = use_signal(|| None::<LatestAsyncTask>);
    let pending_cukta_scroll = use_signal(|| None::<String>);
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
    let gentufa_page = use_signal(GentufaAsyncPageState::default);
    let gentufa_page_task = use_signal(|| None::<LatestAsyncTask>);
    let export_task = use_signal(|| None::<LatestAsyncTask>);

    let settings_value = *settings.read();
    let activity_value = activity.read().clone();
    let activity_indicator_visible_value = *activity_indicator_visible.read();
    let route_value = *route.read();
    let view_mode_value = *view_mode.read();
    let gentufa_display_value = *gentufa_display.read();
    let result = gentufa_page.read().result.clone();
    let nav_gentufa_state = gentufa_state_from_parts(
        &input_text.read(),
        &dialect.read(),
        view_mode_value,
        gentufa_display_value,
        *gentufa_text_explicit.read(),
    );
    let topbar_cukta_href = cukta_web_url(&base_path, &cukta_state.read());
    let topbar_vlacku_href = vlacku_web_url(&base_path, &vlacku_committed_state.read());
    let topbar_gentufa_href = gentufa_web_url(&base_path, &nav_gentufa_state);
    install_browser_state_handlers(
        route,
        cukta_state,
        vlacku_draft_state,
        vlacku_committed_state,
        input_text,
        parsed_text,
        dialect,
        parsed_dialect,
        view_mode,
        gentufa_display,
        gentufa_text_explicit,
        pending_cukta_scroll,
        jvozba_available,
        topbar_settings_layout,
        topbar_settings_open,
        &base_path,
    );
    use_effect(move || {
        configure_embedding_worker_url(&format!("{EMBEDDING_WORKER_JS}"));
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
    let settings_meta_base_path = base_path.clone();
    use_effect(move || {
        if *route.read() == AppRoute::Settings {
            let meta = build_page_meta(&settings_meta_base_path, &WebRoute::Settings);
            sync_document_head(&meta);
            spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                refresh_embedding_settings(embedding_settings).await;
            });
        }
    });
    let gentufa_base_path = base_path.clone();
    use_effect(move || {
        if *route.read() != AppRoute::Gentufa {
            cancel_compute_channel(COMPUTE_CHANNEL_GENTUFA);
            cancel_latest_task(gentufa_page_task);
            return;
        }
        let settings_value = *settings.read();
        let display_value = *gentufa_display.read();
        let view_mode_value = *view_mode.read();
        let text = parsed_text.read().clone();
        let dialect_text = parsed_dialect.read().clone();
        let text_explicit = *gentufa_text_explicit.read();
        let state = gentufa_state_from_parts(
            &text,
            &dialect_text,
            view_mode_value,
            display_value,
            text_explicit,
        );
        let request = GentufaWebRequest {
            text,
            options: web_options(settings_value, display_value, view_mode_value, dialect_text),
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
                        sync_document_head(&meta);
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
    });
    use_effect(move || {
        let state = vlacku_committed_state.read().clone();
        let mut result_signal = vlacku_semantic_result;
        if *route.read() != AppRoute::Vlacku
            || state.mode != VlackuWebMode::Meaning
            || state.query.trim().is_empty()
        {
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
        let request = vlacku_compute_request(&vlacku_page_base_path, &state, &semantic);
        let mut page_signal = vlacku_result;
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
                        sync_document_head(&meta);
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
            },
        );
    });
    use_effect(move || {
        let mut result_signal = cukta_semantic_result;
        let state = cukta_state.read().clone();
        let search_state = match state.view {
            CuktaWebView::Search(search_state)
                if search_state.mode == CuktaWebMode::Meaning
                    && !search_state.query.trim().is_empty() =>
            {
                search_state
            }
            _ => {
                cancel_latest_task(cukta_semantic_task);
                result_signal.set(CuktaSemanticResultState::default());
                return;
            }
        };
        if *route.read() != AppRoute::Cukta {
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
        let state = cukta_state.read().clone();
        let semantic = cukta_semantic_result.read().clone();
        let request = cukta_compute_request(&cukta_page_base_path, &state, &semantic);
        let mut page_signal = cukta_page;
        page_signal.with_mut(|page| {
            page.state = Some(state.clone());
            page.loading = true;
            page.error = None;
        });
        let mut result_signal = cukta_page;
        let mut pending_scroll = pending_cukta_scroll;
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
                        sync_document_head(&meta);
                        if let Some(target) = pending_scroll.write().take() {
                            scroll_to_cukta_href(&target);
                        }
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
    let vlacku_url_base_path = base_path.clone();
    use_effect(move || {
        if *route.read() == AppRoute::Vlacku {
            let state = vlacku_committed_state.read().clone();
            schedule_vlacku_url_push(&vlacku_url_base_path, &state);
        }
    });
    let cukta_url_base_path = base_path.clone();
    use_effect(move || {
        if *route.read() == AppRoute::Cukta {
            let state = cukta_state.read().clone();
            push_cukta_url(&cukta_url_base_path, &state);
        }
    });
    let gentufa_url_base_path = base_path.clone();
    use_effect(move || {
        if *route.read() == AppRoute::Gentufa {
            let state = gentufa_state_from_parts(
                &input_text.read(),
                &dialect.read(),
                *view_mode.read(),
                *gentufa_display.read(),
                *gentufa_text_explicit.read(),
            );
            schedule_gentufa_url_replace(&gentufa_url_base_path, &state);
        }
    });
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
        );
        schedule_topbar_settings_layout_measure(topbar_settings_layout, topbar_settings_open);
        if *route.read() == AppRoute::Vlacku {
            schedule_vlacku_jvozba_pane_metrics_sync();
        }
    });
    use_effect(move || {
        if *route.read() == AppRoute::Gentufa {
            let _ = (
                parsed_text.read().len(),
                parsed_dialect.read().len(),
                *view_mode.read(),
                *gentufa_display.read(),
            );
            schedule_gentufa_block_reference_layout();
            schedule_gentufa_tree_layout();
        }
    });
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

    rsx! {
        style { "{font_face_css()}" }
        document::Stylesheet { href: MAIN_CSS }
        document::Link { rel: "modulepreload", href: COMPUTE_JS }
        document::Link { rel: "modulepreload", href: COMPUTE_WORKER_JS }
        document::Link { rel: "modulepreload", href: EMBEDDINGS_JS }
        document::Link { rel: "modulepreload", href: EMBEDDING_WORKER_JS }
        document::Link { rel: "manifest", href: MANIFEST }
        document::Link { rel: "icon", r#type: "image/png", href: FAVICON }
        document::Link { rel: "shortcut icon", r#type: "image/png", href: FAVICON }
        document::Link { rel: "apple-touch-icon", href: FAVICON }
        div { class: "{app_class}",
            { render_topbar(
                route_value,
                settings,
                settings_value,
                &topbar_cukta_href,
                &topbar_vlacku_href,
                &topbar_gentufa_href,
                &nav_href(&base_path, AppRoute::Settings),
                *topbar_settings_layout.read(),
                topbar_settings_open,
                &activity_value,
                activity_indicator_visible_value,
            ) }
            main { class: "spa-main",
                div { class: "spa-stack",
                    {
                        match route_value {
                            AppRoute::Gentufa => rsx! {
                                section {
                                    class: "spa-page parse-page spa-gentufa-page",
                                    onmousemove: move |_| refresh_reference_hover(reference_hover),
                                    onwheel: move |_| refresh_reference_hover(reference_hover),
                                    h1 { class: "sr-only", "jbotci gentufa" }
                                    div { class: "page-container",
                                        div { class: "input-form",
                                            div { class: "form-group",
                                                textarea {
                                                    id: "gentufa-text",
                                                    aria_label: "Lojban text",
                                                    placeholder: "{DEFAULT_GENTUFA_TEXT}",
                                                    value: "{input_text.read()}",
                                                    spellcheck: "false",
                                                    oninput: move |event| {
                                                        input_text.set(event.value());
                                                        gentufa_text_explicit.set(true);
                                                        schedule_gentufa_textarea_resize();
                                                    },
                                                }
                                                div { class: "form-actions",
                                                    { render_dialect_control(dialect) }
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
                                                            gentufa_text_explicit.set(true);
                                                            parsed_text.set(next_text);
                                                            parsed_dialect.set(next_dialect);
                                                        },
                                                        "Parse"
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "gentufa-result-stack",
                                            { render_result(&result, view_mode, view_mode_value, gentufa_display, gentufa_display_value, settings_value, reference_hover, activity, export_task) }
                                        }
                                    }
                                }
                            },
                            AppRoute::Settings => render_settings(settings, settings_value, embedding_settings, activity),
                            AppRoute::Cukta => {
                                render_cukta_page(
                                    cukta_state,
                                    cukta_page,
                                    cukta_toc_filter,
                                    cukta_toc_pinned,
                                    cukta_toc_expansion,
                                    cukta_toc_width,
                                    cukta_toc_resize,
                                    &base_path,
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
                                    &base_path,
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
    cukta_href: &str,
    vlacku_href: &str,
    gentufa_href: &str,
    settings_href: &str,
    settings_layout: TopbarSettingsLayout,
    settings_open: Signal<bool>,
    activity: &AsyncActivityState,
    activity_visible: bool,
) -> Element {
    let cukta_loading = activity_visible && activity.has_kind(AsyncTaskKind::Cukta);
    let vlacku_loading = activity_visible && activity.has_kind(AsyncTaskKind::Vlacku);
    let gentufa_loading = activity_visible && activity.has_kind(AsyncTaskKind::Gentufa);
    let activity_class = topbar_activity_class(activity_visible);
    let header_class = topbar_header_class(settings_layout, *settings_open.read());
    let show_theme_inline = settings_layout.shows_theme_inline();
    let show_script_inline = settings_layout.shows_script_inline();
    rsx! {
        header { class: "{header_class}",
            div { class: "app-topbar-inner spa-topbar-inner",
                div { class: "app-topbar-left",
                    a {
                        class: "app-topbar-brand",
                        href: "{settings_href}",
                        aria_label: "Settings",
                        title: "Settings",
                        img { class: "app-topbar-brand-logo", src: LOGO, alt: "jbotci" }
                    }
                    { render_topbar_settings_button(settings, current, settings_href, settings_layout, settings_open) }
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
                    nav { class: "spa-nav", aria_label: "Primary navigation",
                        a {
                            class: topbar_link_class(route == AppRoute::Cukta, cukta_loading),
                            href: "{cukta_href}",
                            aria_current: if route == AppRoute::Cukta { "page" } else { "false" },
                            span { class: "app-topbar-link-label", "cukta" }
                        }
                        a {
                            class: topbar_link_class(route == AppRoute::Vlacku, vlacku_loading),
                            href: "{vlacku_href}",
                            aria_current: if route == AppRoute::Vlacku { "page" } else { "false" },
                            span { class: "app-topbar-link-label", "vlacku" }
                        }
                        a {
                            class: topbar_link_class(route == AppRoute::Gentufa, gentufa_loading),
                            href: "{gentufa_href}",
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
                { render_topbar_fit_probes(settings, current, route, cukta_loading, vlacku_loading, gentufa_loading, cukta_href, vlacku_href, gentufa_href) }
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
                    { render_topbar_commit_link() }
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
    cukta_href: &str,
    vlacku_href: &str,
    gentufa_href: &str,
) -> Element {
    rsx! {
        nav { class: "spa-nav", aria_label: "Primary navigation",
            a {
                class: topbar_link_class(route == AppRoute::Cukta, cukta_loading),
                href: "{cukta_href}",
                aria_current: if route == AppRoute::Cukta { "page" } else { "false" },
                span { class: "app-topbar-link-label", "cukta" }
            }
            a {
                class: topbar_link_class(route == AppRoute::Vlacku, vlacku_loading),
                href: "{vlacku_href}",
                aria_current: if route == AppRoute::Vlacku { "page" } else { "false" },
                span { class: "app-topbar-link-label", "vlacku" }
            }
            a {
                class: topbar_link_class(route == AppRoute::Gentufa, gentufa_loading),
                href: "{gentufa_href}",
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

#[requires(true)]
#[ensures(true)]
fn render_topbar_settings_button(
    settings: Signal<UserSettings>,
    current: UserSettings,
    settings_href: &str,
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
                    { render_topbar_settings_menu(settings, current, settings_href, settings_layout) }
                }
            } else {
                a {
                    class: "{button_class}",
                    href: "{settings_href}",
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
    settings_href: &str,
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
                a {
                    class: "app-topbar-settings-all",
                    href: "{settings_href}",
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
    cukta_href: &str,
    vlacku_href: &str,
    gentufa_href: &str,
) -> Element {
    rsx! {
        div {
            class: "app-topbar-fit-probes",
            aria_hidden: "true",
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-both",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                span { class: "app-topbar-theme app-topbar-theme-mode",
                    { render_theme_switch(settings, current.theme) }
                }
                span { class: "app-topbar-theme app-topbar-orthography",
                    { render_script_switch(settings, current.script) }
                }
                { render_topbar_nav(route, cukta_loading, vlacku_loading, gentufa_loading, cukta_href, vlacku_href, gentufa_href) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-theme",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                span { class: "app-topbar-theme app-topbar-theme-mode",
                    { render_theme_switch(settings, current.theme) }
                }
                { render_topbar_nav(route, cukta_loading, vlacku_loading, gentufa_loading, cukta_href, vlacku_href, gentufa_href) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-none",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                { render_topbar_nav(route, cukta_loading, vlacku_loading, gentufa_loading, cukta_href, vlacku_href, gentufa_href) }
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

#[requires(true)]
#[ensures(true)]
fn render_topbar_commit_link() -> Element {
    let Some(full_commit) = BUILD_GIT_COMMIT else {
        return rsx! {};
    };
    let Some(short_commit) = BUILD_GIT_COMMIT_SHORT else {
        return rsx! {};
    };
    let href = format!("https://codeberg.org/int_19h/jbotci/commit/{full_commit}");
    let display_commit = math_monospace_git_commit(short_commit);
    rsx! {
        a {
            class: "app-topbar-commit",
            href: "{href}",
            title: "Git commit from which this version of jbotci was built.",
            aria_label: "Build commit {short_commit}",
            "{display_commit}"
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
fn topbar_header_class(settings_layout: TopbarSettingsLayout, settings_open: bool) -> String {
    format!(
        "app-topbar spa-topbar {}{}",
        match settings_layout {
            TopbarSettingsLayout::BothInline => "topbar-settings-both-inline",
            TopbarSettingsLayout::ThemeInline => "topbar-settings-theme-inline",
            TopbarSettingsLayout::NoneInline => "topbar-settings-none-inline",
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

#[requires(true)]
#[ensures(true)]
fn update_topbar_settings_layout(
    mut settings_layout: Signal<TopbarSettingsLayout>,
    mut settings_open: Signal<bool>,
    next_layout: TopbarSettingsLayout,
) {
    if *settings_layout.read() != next_layout {
        settings_layout.set(next_layout);
    }
    if next_layout == TopbarSettingsLayout::BothInline && *settings_open.read() {
        settings_open.set(false);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_topbar_settings_layout_measure(
    settings_layout: Signal<TopbarSettingsLayout>,
    settings_open: Signal<bool>,
) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move || {
        update_topbar_settings_layout(
            settings_layout,
            settings_open,
            measure_topbar_settings_layout(),
        );
    });
    let _ = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), 0);
    closure.forget();
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn schedule_topbar_settings_layout_measure(
    settings_layout: Signal<TopbarSettingsLayout>,
    settings_open: Signal<bool>,
) {
    update_topbar_settings_layout(
        settings_layout,
        settings_open,
        TopbarSettingsLayout::BothInline,
    );
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_topbar_settings_layout_after_fonts_ready(
    document: &web_sys::Document,
    settings_layout: Signal<TopbarSettingsLayout>,
    settings_open: Signal<bool>,
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
        schedule_topbar_settings_layout_measure(settings_layout, settings_open);
    });
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn measure_topbar_settings_layout() -> TopbarSettingsLayout {
    if topbar_probe_fits(".app-topbar-fit-probe-both") {
        TopbarSettingsLayout::BothInline
    } else if topbar_probe_fits(".app-topbar-fit-probe-theme") {
        TopbarSettingsLayout::ThemeInline
    } else {
        TopbarSettingsLayout::NoneInline
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(!selector.is_empty())]
#[ensures(true)]
fn topbar_probe_fits(selector: &str) -> bool {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return true;
    };
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
fn render_dialect_control(mut dialect: Signal<String>) -> Element {
    rsx! {
        div { class: "gentufa-dialect-control",
            button {
                class: "gentufa-dialect-label",
                r#type: "button",
                aria_expanded: "false",
                "Dialect:"
            }
            div { class: "gentufa-dialect-input-shell",
                div { class: "gentufa-dialect-formula-wrap",
                    pre {
                        class: "settings-dialect-definition-highlight gentufa-dialect-formula-highlight",
                        aria_hidden: "true",
                        "{dialect.read()}"
                    }
                    input {
                        class: "settings-text-input settings-dialect-definition gentufa-dialect-formula-input",
                        value: "{dialect.read()}",
                        placeholder: "baseline (CLL + xorlo + LTR-magic)",
                        spellcheck: "false",
                        aria_label: "Dialect formula",
                        oninput: move |event| dialect.set(event.value()),
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(module = "/assets/embeddings.js")]
extern "C" {
    #[wasm_bindgen(js_name = jbotciEmbeddingConfigureWorker)]
    fn js_embedding_configure_worker(worker_url: &str);

    #[wasm_bindgen(js_name = jbotciEmbeddingStatus)]
    fn js_embedding_status() -> js_sys::Promise;

    #[wasm_bindgen(js_name = jbotciEmbeddingSetup)]
    fn js_embedding_setup(corpus_json: &str) -> js_sys::Promise;

    #[wasm_bindgen(js_name = jbotciEmbeddingRemove)]
    fn js_embedding_remove() -> js_sys::Promise;

    #[wasm_bindgen(js_name = jbotciEmbeddingSearch)]
    fn js_embedding_search(
        corpus_id: &str,
        query: &str,
        limit: usize,
        kind_filters_json: &str,
    ) -> js_sys::Promise;
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
    match embedding_status_json().await {
        Ok(json) => settings.set(embedding_settings_from_json(
            &json,
            "Browser embeddings are ready.",
        )),
        Err(error) => settings.set(EmbeddingSettingsState {
            status: "unavailable".to_owned(),
            detail: error,
            model_size: "unknown".to_owned(),
            index_size: "unknown".to_owned(),
            progress_label: None,
            progress_percent: None,
            busy: false,
        }),
    }
}

#[requires(true)]
#[ensures(true)]
async fn setup_browser_embeddings(mut settings: Signal<EmbeddingSettingsState>) {
    let corpus_json = match embedding_corpus_json_from_compute_worker().await {
        Ok(json) => json,
        Err(error) => {
            settings.set(EmbeddingSettingsState {
                status: "error".to_owned(),
                detail: error,
                model_size: "unknown".to_owned(),
                index_size: "unknown".to_owned(),
                progress_label: None,
                progress_percent: None,
                busy: false,
            });
            return;
        }
    };
    match embedding_setup_json(&corpus_json).await {
        Ok(json) => settings.set(embedding_settings_from_json(
            &json,
            "Browser embeddings are ready.",
        )),
        Err(error) => settings.set(EmbeddingSettingsState {
            status: "error".to_owned(),
            detail: error,
            model_size: "unknown".to_owned(),
            index_size: "unknown".to_owned(),
            progress_label: None,
            progress_percent: None,
            busy: false,
        }),
    }
}

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
            let mut next =
                embedding_settings_from_json(&json, "Browser embeddings are being prepared.");
            next.busy = true;
            settings.set(next);
        }
    }
}

#[requires(true)]
#[ensures(true)]
async fn remove_browser_embeddings(mut settings: Signal<EmbeddingSettingsState>) {
    match embedding_remove_json().await {
        Ok(json) => settings.set(embedding_settings_from_json(
            &json,
            "Browser embeddings were removed.",
        )),
        Err(error) => settings.set(EmbeddingSettingsState {
            status: "error".to_owned(),
            detail: error,
            model_size: "unknown".to_owned(),
            index_size: "unknown".to_owned(),
            progress_label: None,
            progress_percent: None,
            busy: false,
        }),
    }
}

#[requires(true)]
#[ensures(true)]
async fn load_vlacku_semantic_result(state: VlackuWebState) -> VlackuSemanticResultState {
    let limit = vlacku_semantic_worker_limit(&state);
    let normalized_state = normalize_vlacku_state(&state);
    match embedding_search_json(
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
    match embedding_search_json("cukta-cll", &state.query, limit, &kind_filters).await {
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
    let loading = !state.query.trim().is_empty()
        && (semantic.state.as_ref() != Some(state) || semantic.loading);
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

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
async fn embedding_status_json() -> Result<String, String> {
    Err("Browser embeddings are available only in the wasm web app.".to_owned())
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
    promise_to_string(js_embedding_setup(corpus_json)).await
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
async fn embedding_setup_json(_corpus_json: &str) -> Result<String, String> {
    Err("Browser embeddings are available only in the wasm web app.".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn embedding_remove_json() -> Result<String, String> {
    promise_to_string(js_embedding_remove()).await
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
async fn embedding_remove_json() -> Result<String, String> {
    Err("Browser embeddings are available only in the wasm web app.".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
async fn embedding_search_json(
    corpus_id: &str,
    query: &str,
    limit: usize,
    kind_filters: &[String],
) -> Result<String, String> {
    let kind_filters_json = serde_json::to_string(kind_filters).unwrap_or_else(|_| "[]".to_owned());
    promise_to_string(js_embedding_search(
        corpus_id,
        query,
        limit,
        &kind_filters_json,
    ))
    .await
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
async fn embedding_search_json(
    _corpus_id: &str,
    _query: &str,
    _limit: usize,
    _kind_filters: &[String],
) -> Result<String, String> {
    Err(
        "Open Settings and download embeddings in the browser before using meaning search."
            .to_owned(),
    )
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
    let _ = milliseconds;
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
    let index_size = value
        .get("indexBytes")
        .and_then(serde_json::Value::as_u64)
        .map(human_bytes)
        .unwrap_or_else(|| "unknown".to_owned());
    let progress = value.get("progress");
    let progress_label = progress
        .and_then(|progress| json_string(progress, "label"))
        .filter(|label| !label.is_empty());
    let progress_percent = progress
        .and_then(|progress| progress.get("percent"))
        .and_then(serde_json::Value::as_u64)
        .map(|percent| percent.min(100) as u8);
    EmbeddingSettingsState {
        status,
        detail,
        model_size,
        index_size,
        progress_label,
        progress_percent,
        busy: false,
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
    cukta_state: Signal<CuktaWebState>,
    cukta_page: Signal<CuktaAsyncPageState>,
    mut toc_filter: Signal<String>,
    mut toc_pinned: Signal<bool>,
    toc_expansion: Signal<CuktaTocExpansionState>,
    toc_width: Signal<f64>,
    mut toc_resize: Signal<Option<CuktaTocResizeState>>,
    base_path: &str,
) -> Element {
    let page = cukta_page.read().page.clone();
    let toc_is_pinned = *toc_pinned.read();
    let is_resizing = toc_resize.read().is_some();
    let shell_class = class_names(
        "cll-shell",
        &[
            ("cll-toc-autohide", !toc_is_pinned),
            ("cll-is-resizing", is_resizing),
        ],
    );
    let current_toc_width = clamp_cukta_toc_width(*toc_width.read());
    let shell_style = format!("--cll-sidebar-width:{current_toc_width:.0}px;");
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
                aside { class: "cll-sidebar",
                    button {
                        class: "cll-sidebar-toggle",
                        r#type: "button",
                        title: "Table of contents",
                        aria_pressed: pressed_attr(toc_is_pinned),
                        onclick: move |_| set_cukta_toc_pinned(&mut toc_pinned, !toc_is_pinned),
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
                    }
                    div { class: "cll-toc-popup",
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
                                a {
                                    class: "cll-toc-header-link cll-toc-index-link",
                                    href: cukta_web_url(base_path, &CuktaWebState { view: CuktaWebView::Index }),
                                    onclick: move |event| {
                                        event.prevent_default();
                                        set_cukta_state(&mut cukta_state.clone(), CuktaWebState { view: CuktaWebView::Index });
                                    },
                                    "index"
                                }
                                a {
                                    class: "cll-toc-header-link cll-toc-advanced-link",
                                    href: cukta_web_url(base_path, &CuktaWebState { view: CuktaWebView::Search(CuktaWebSearchState::default()) }),
                                    onclick: move |event| {
                                        event.prevent_default();
                                        set_cukta_state(&mut cukta_state.clone(), CuktaWebState { view: CuktaWebView::Search(CuktaWebSearchState::default()) });
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
                                    { render_cukta_toc_node(cukta_state, toc_expansion, node, &toc_filter.read()) }
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
                        if toc_is_pinned {
                            let x = event.data().client_coordinates().x;
                            toc_resize.set(Some(new!(CuktaTocResizeState {
                                start_x: x,
                                start_width: *toc_width.read(),
                            })));
                        }
                    },
                    span { class: "cll-splitter-grip", aria_hidden: "true" }
                }
                main { class: "cll-main",
                    {
                        match &page.page_kind {
                            CuktaPageKind::Section {
                                section_heading,
                                chapter_title,
                                previous_section,
                                next_section,
                                chapter_prelude_blocks,
                                blocks,
                            } => render_cukta_section(
                                cukta_state,
                                section_heading,
                                chapter_title.as_deref(),
                                previous_section.as_ref(),
                                next_section.as_ref(),
                                chapter_prelude_blocks,
                                blocks,
                                base_path,
                            ),
                            CuktaPageKind::Index { entries } => render_cukta_index(cukta_state, entries),
                            CuktaPageKind::Search {
                                state,
                                mode_options,
                                target_options,
                                results,
                                message,
                                has_more,
                                load_more_href: _,
                            } => render_cukta_search(
                                cukta_state,
                                state,
                                mode_options,
                                target_options,
                                results,
                                message.as_deref(),
                                *has_more,
                            ),
                            CuktaPageKind::Error { message } => rsx! {
                                div { class: "spa-error", "{message}" }
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
#[ensures(true)]
fn render_cukta_toc_node(
    cukta_state: Signal<CuktaWebState>,
    toc_expansion: Signal<CuktaTocExpansionState>,
    node: &CuktaTocNode,
    filter: &str,
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
    let target_reference = node
        .section_id
        .clone()
        .or_else(|| cukta_section_reference_from_href(&node.href));
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
                a {
                    class: "cll-toc-link",
                    href: "{node.href}",
                    onclick: {
                        let target_reference = target_reference.clone();
                        move |event| {
                            if let Some(reference) = target_reference.clone() {
                                event.prevent_default();
                                set_cukta_state(&mut cukta_state.clone(), CuktaWebState {
                                    view: CuktaWebView::Section { reference },
                                });
                            }
                        }
                    },
                    if let Some(number) = &node.number_label {
                        { render_cukta_toc_number(number, number_has_trailing_dot) }
                    }
                    { render_cukta_toc_title(&node.label) }
                }
            }
            if !node.children.is_empty() && expanded {
                ol { class: "cll-toc-children",
                    for child in node.children.iter() {
                        { render_cukta_toc_node(cukta_state, toc_expansion, child, &filter) }
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
    cukta_state: Signal<CuktaWebState>,
    heading: &str,
    chapter_title: Option<&str>,
    previous: Option<&jbotci_web_core::CuktaSectionLink>,
    next: Option<&jbotci_web_core::CuktaSectionLink>,
    prelude_blocks: &[CllBlock],
    blocks: &[CllBlock],
    base_path: &str,
) -> Element {
    let _ = chapter_title;
    rsx! {
        article { class: "cll-section-content",
            h1 { "{heading}" }
            if !prelude_blocks.is_empty() {
                div { class: "cll-chapter-prelude",
                    for block in prelude_blocks.iter() {
                        { render_cll_block(cukta_state, block, base_path) }
                    }
                }
            }
            for block in blocks.iter() {
                { render_cll_block(cukta_state, block, base_path) }
            }
            if previous.is_some() || next.is_some() {
                nav { class: "cll-section-pager",
                    if let Some(previous) = previous {
                        { render_cukta_section_pager_link(cukta_state, previous, "prev") }
                    }
                    if let Some(next) = next {
                        { render_cukta_section_pager_link(cukta_state, next, "next") }
                    }
                }
            }
        }
    }
}

#[requires(direction == "prev" || direction == "next")]
#[ensures(true)]
fn render_cukta_section_pager_link(
    cukta_state: Signal<CuktaWebState>,
    section: &jbotci_web_core::CuktaSectionLink,
    direction: &str,
) -> Element {
    let class_name = format!("cll-section-pager-link cll-section-pager-link-{direction}");
    rsx! {
        a {
            class: "{class_name}",
            href: "{section.href}",
            onclick: {
                let href = section.href.clone();
                move |event| {
                    event.prevent_default();
                    if let Some(reference) = cukta_section_reference_from_href(&href) {
                        set_cukta_state(&mut cukta_state.clone(), CuktaWebState {
                            view: CuktaWebView::Section { reference },
                        });
                    }
                }
            },
            span { class: "cll-section-pager-link-label", "{section.label}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_index(
    cukta_state: Signal<CuktaWebState>,
    entries: &[jbotci_web_core::CuktaIndexEntry],
) -> Element {
    rsx! {
        section { class: "cll-index-view",
            h1 { "Index" }
            div { class: "cll-index-list",
                for entry in entries.iter() {
                    div { class: "cll-index-entry",
                        span { class: "cll-index-key", "{entry.key}" }
                        span { class: "cll-index-refs",
                            for reference in entry.references.iter() {
                                a {
                                    href: "{reference.href}",
                                    onclick: {
                                        let href = reference.href.clone();
                                        move |event| {
                                            event.prevent_default();
                                            if let Some(section_id) = href.rsplit('/').next() {
                                                set_cukta_state(&mut cukta_state.clone(), CuktaWebState {
                                                    view: CuktaWebView::Section { reference: section_id.to_owned() },
                                                });
                                            }
                                        }
                                    },
                                    "{reference.label}"
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
    cukta_state: Signal<CuktaWebState>,
    state: &CuktaWebSearchState,
    mode_options: &[CuktaModeOption],
    target_options: &[CuktaTargetOption],
    results: &[CuktaSearchResultCard],
    message: Option<&str>,
    has_more: bool,
) -> Element {
    let state_for_load_more = state.clone();
    rsx! {
        section { class: "cll-search-view dictionary-page",
            { render_cukta_search_controls(cukta_state, state, mode_options, target_options) }
            if let Some(message) = message {
                p { class: "dictionary-empty cll-search-message", "{message}" }
            }
            div { class: "cll-search-results",
                for card in results.iter() {
                    { render_cukta_search_card(cukta_state, card) }
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
                            set_cukta_state(&mut cukta_state.clone(), CuktaWebState {
                                view: CuktaWebView::Search(next),
                            });
                        },
                        "Load more"
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_search_controls(
    cukta_state: Signal<CuktaWebState>,
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
                                    { render_cukta_mode_button(cukta_state, state, option) }
                                }
                            }
                        }
                    }
                }
                div { class: "cll-target-control",
                    div { class: "cll-target-grid", aria_label: "CLL search targets",
                        for option in target_options.iter() {
                            { render_cukta_target_check(cukta_state, state, option) }
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
                        let mut next = state_for_input.clone();
                        next.query = event.value();
                        next.count = CUKTA_WEB_DEFAULT_COUNT;
                        set_cukta_state(&mut cukta_state.clone(), CuktaWebState {
                            view: CuktaWebView::Search(next),
                        });
                    },
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_mode_button(
    cukta_state: Signal<CuktaWebState>,
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
                    set_cukta_state(&mut cukta_state.clone(), CuktaWebState {
                        view: CuktaWebView::Search(next),
                    });
                }
            },
            "{option_label}"
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_target_check(
    cukta_state: Signal<CuktaWebState>,
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
                    set_cukta_state(&mut cukta_state.clone(), CuktaWebState {
                        view: CuktaWebView::Search(next),
                    });
                },
            }
            span { class: "vlacku-filter-label", "{option.label}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_search_card(
    cukta_state: Signal<CuktaWebState>,
    card: &CuktaSearchResultCard,
) -> Element {
    rsx! {
        article { class: "cll-search-result-card result-card",
            header { class: "cll-search-result-head result-header",
                div {
                    p { class: "cll-search-result-meta", "{card.kind} · {card.section_label}" }
                    h2 { class: "cll-search-result-title",
                        a {
                            href: "{card.href}",
                            onclick: {
                                let href = card.href.clone();
                                move |event| {
                                    if let Some(reference) = cukta_section_reference_from_href(&href) {
                                        event.prevent_default();
                                        set_cukta_state(&mut cukta_state.clone(), CuktaWebState {
                                            view: CuktaWebView::Section { reference },
                                        });
                                    }
                                }
                            },
                            "{card.rank}. {card.label}"
                        }
                    }
                }
                if let Some(similarity) = &card.similarity_label {
                    span { class: "dictionary-meta-segment dictionary-meta-tooltip", "{similarity}" }
                }
            }
            p { class: "cll-search-preview", "{card.preview}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_block(
    cukta_state: Signal<CuktaWebState>,
    block: &CllBlock,
    base_path: &str,
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
                        "{text}"
                    } else {
                        for inline in inlines.iter() {
                            { render_cll_inline(cukta_state, inline, base_path) }
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
                                    { render_cll_block(cukta_state, child, base_path) }
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
                                    { render_cll_block(cukta_state, child, base_path) }
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
                    span { class: "cll-example-title", "{example.label}" }
                    if let Some(parse_href) = &example.parse_href {
                        a {
                            class: "cll-parse-example spa-cll-link spa-cll-link-parse",
                            href: cll_parse_href(base_path, parse_href),
                            "Parse"
                        }
                    }
                }
                if example.blocks.is_empty() {
                    div { class: "cll-interlinear",
                        for line in example.lines.iter() {
                            p { class: "cll-ig-line cll-ig-{line.kind}", "{line.text}" }
                        }
                    }
                } else {
                    for child in example.blocks.iter() {
                        { render_cll_block(cukta_state, child, base_path) }
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
                            { render_cll_inline(cukta_state, inline, base_path) }
                        }
                    }
                }
                if !header_rows.is_empty() {
                    thead {
                        for row in header_rows.iter() {
                            tr {
                                for cell in row.iter() {
                                    th {
                                        colspan: "{cell.col_span.unwrap_or(1)}",
                                        rowspan: "{cell.row_span.unwrap_or(1)}",
                                        for child in cell.blocks.iter() {
                                            { render_cll_block(cukta_state, child, base_path) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                tbody {
                    for row in body_rows.iter() {
                        tr {
                            for cell in row.iter() {
                                td {
                                    colspan: "{cell.col_span.unwrap_or(1)}",
                                    rowspan: "{cell.row_span.unwrap_or(1)}",
                                    for child in cell.blocks.iter() {
                                        { render_cll_block(cukta_state, child, base_path) }
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
                                                { render_cll_inline(cukta_state, inline, base_path) }
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
                            { render_cll_inline(cukta_state, inline, base_path) }
                        }
                    }
                    dd {
                        for child in entry.blocks.iter() {
                            { render_cll_block(cukta_state, child, base_path) }
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
                                { render_cll_inline(cukta_state, inline, base_path) }
                            }
                        }
                    }
                }
            }
        }
        CllBlock::Rule { id, term, body } => rsx! {
            div { id: id.clone().unwrap_or_default(), class: "cll-rule",
                dt { "{term}" }
                dd {
                    for child in body.iter() {
                        { render_cll_block(cukta_state, child, base_path) }
                    }
                }
            }
        },
        CllBlock::Code { text, .. } => rsx! {
            pre { class: "cll-code", code { "{text}" } }
        },
        CllBlock::DisplayMath { id, markup, .. } => rsx! {
            div {
                id: id.clone().unwrap_or_default(),
                class: "cll-math-block",
                dangerous_inner_html: "{markup}"
            }
        },
        CllBlock::Heading { level, title } => {
            let class_name = format!("cll-heading cll-heading-{level}");
            rsx! { h2 { class: "{class_name}", "{title}" } }
        }
        CllBlock::BlockQuote { id, blocks } => rsx! {
            blockquote { id: id.clone().unwrap_or_default(), class: "cll-blockquote",
                for child in blocks.iter() {
                    { render_cll_block(cukta_state, child, base_path) }
                }
            }
        },
        CllBlock::Definition { id, body } => rsx! {
            p { id: id.clone().unwrap_or_default(), class: "cll-definition",
                for inline in body.iter() {
                    { render_cll_inline(cukta_state, inline, base_path) }
                }
            }
        },
        CllBlock::InterlinearGloss {
            id,
            aligned,
            itemized,
            rows,
            natlang,
            comments,
        } => render_cll_interlinear(
            cukta_state,
            id.as_deref(),
            *aligned,
            *itemized,
            rows,
            natlang,
            comments,
            base_path,
        ),
        CllBlock::CmavoList {
            id,
            titles,
            headers,
            rows,
        } => render_cll_cmavo_list(cukta_state, id.as_deref(), titles, headers, rows, base_path),
        CllBlock::Lojbanization { id, lines } => {
            render_cll_lojbanization(cukta_state, id.as_deref(), lines, base_path)
        }
        CllBlock::LujvoMaking { id, parts } => {
            render_cll_lujvo_making(cukta_state, id.as_deref(), parts, base_path)
        }
        CllBlock::GrammarTemplate { id, body } => rsx! {
            p { id: id.clone().unwrap_or_default(), class: "cll-grammar-template",
                for inline in body.iter() {
                    { render_cll_inline(cukta_state, inline, base_path) }
                }
            }
        },
        CllBlock::Ebnf { id, entries } => render_cll_ebnf(id.as_deref(), entries, base_path),
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_inline(
    cukta_state: Signal<CuktaWebState>,
    inline: &CllInline,
    base_path: &str,
) -> Element {
    match inline {
        CllInline::Text(text) => rsx! { "{text}" },
        CllInline::Emphasis { language, inlines } => rsx! {
            em { lang: language.clone().unwrap_or_default(),
                for child in inlines.iter() {
                    { render_cll_inline(cukta_state, child, base_path) }
                }
            }
        },
        CllInline::Quote { language, inlines } => rsx! {
            q { lang: language.clone().unwrap_or_default(),
                for child in inlines.iter() {
                    { render_cll_inline(cukta_state, child, base_path) }
                }
            }
        },
        CllInline::LanguageSpan {
            kind,
            language,
            inlines,
        } => {
            let class_name = cll_language_span_class(*kind);
            rsx! {
                span { class: "{class_name}", lang: language.clone().unwrap_or_default(),
                    for child in inlines.iter() {
                        { render_cll_inline(cukta_state, child, base_path) }
                    }
                }
            }
        }
        CllInline::CiteTitle { inlines } => rsx! {
            cite {
                for child in inlines.iter() {
                    { render_cll_inline(cukta_state, child, base_path) }
                }
            }
        },
        CllInline::Subscript { inlines } => rsx! {
            sub {
                for child in inlines.iter() {
                    { render_cll_inline(cukta_state, child, base_path) }
                }
            }
        },
        CllInline::Superscript { inlines } => rsx! {
            sup {
                for child in inlines.iter() {
                    { render_cll_inline(cukta_state, child, base_path) }
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
            let is_cukta_link = matches!(kind, CllLinkKind::Section | CllLinkKind::Example);
            let tooltip = cll_dictionary_tooltip_for_link(base_path, *kind, target);
            if let Some(card) = &tooltip {
                let href_for_click = href.clone();
                rsx! {
                    span { class: "dictionary-tooltip-host",
                        a {
                            class: "{class_name}",
                            href: "{href}",
                            onclick: move |event| {
                                if is_cukta_link {
                                    if let Some(reference) = cukta_section_reference_from_href(&href_for_click) {
                                        event.prevent_default();
                                        set_cukta_state(&mut cukta_state.clone(), CuktaWebState {
                                            view: CuktaWebView::Section { reference },
                                        });
                                        scroll_to_cukta_href(&href_for_click);
                                    }
                                }
                            },
                            for child in inlines.iter() {
                                { render_cll_inline(cukta_state, child, base_path) }
                            }
                        }
                        { render_dictionary_tooltip(card, false, base_path) }
                    }
                }
            } else {
                let href_for_click = href.clone();
                rsx! {
                    a {
                        class: "{class_name}",
                        href: "{href}",
                        onclick: move |event| {
                            if is_cukta_link {
                                if let Some(reference) = cukta_section_reference_from_href(&href_for_click) {
                                    event.prevent_default();
                                    set_cukta_state(&mut cukta_state.clone(), CuktaWebState {
                                        view: CuktaWebView::Section { reference },
                                    });
                                    scroll_to_cukta_href(&href_for_click);
                                }
                            }
                        },
                        for child in inlines.iter() {
                            { render_cll_inline(cukta_state, child, base_path) }
                        }
                    }
                }
            }
        }
        CllInline::Code(text) => rsx! { code { "{text}" } },
        CllInline::Elidable {
            shown,
            forced,
            inlines,
        } => {
            let class_name = class_names("cll-elidable", &[("cll-elidable-forced", *forced)]);
            rsx! {
                span { class: "{class_name}",
                    if inlines.is_empty() {
                        "{shown}"
                    } else {
                        for child in inlines.iter() {
                            { render_cll_inline(cukta_state, child, base_path) }
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
fn render_cll_interlinear(
    cukta_state: Signal<CuktaWebState>,
    id: Option<&str>,
    aligned: bool,
    itemized: bool,
    rows: &[CllInterlinearRow],
    natlang: &[Vec<CllInline>],
    comments: &[Vec<CllInline>],
    base_path: &str,
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
            if !rows.is_empty() {
                if aligned {
                    table { class: "{table_class}",
                        tbody {
                            for row in rows.iter() {
                                tr { class: "cll-ig-row cll-ig-{row.kind} cll-interlinear-row cll-interlinear-row-{row.kind}",
                                    for cell in row.cells.iter() {
                                        td { class: "cll-ig-cell",
                                            for inline in cell.iter() {
                                                { render_cll_inline(cukta_state, inline, base_path) }
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
                            div { class: "cll-ig-line-wrap",
                                p { class: "cll-ig-line cll-ig-inline cll-ig-{row.kind}",
                                    for cell in row.cells.iter() {
                                        for inline in cell.iter() {
                                            { render_cll_inline(cukta_state, inline, base_path) }
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
                        { render_cll_inline(cukta_state, inline, base_path) }
                    }
                }
            }
            for line in natlang.iter() {
                p { class: "cll-ig-natlang-text cll-natlang",
                    for inline in line.iter() {
                        { render_cll_inline(cukta_state, inline, base_path) }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_cmavo_list(
    cukta_state: Signal<CuktaWebState>,
    id: Option<&str>,
    titles: &[Vec<CllInline>],
    headers: &[Vec<CllInline>],
    rows: &[Vec<Vec<CllInline>>],
    base_path: &str,
) -> Element {
    rsx! {
        div { id: id.unwrap_or_default(), class: "cll-cmavo-list",
            for title in titles.iter() {
                p { class: "cll-cmavo-list-title",
                    for inline in title.iter() {
                        { render_cll_inline(cukta_state, inline, base_path) }
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
                                        { render_cll_inline(cukta_state, inline, base_path) }
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
                                        { render_cll_inline(cukta_state, inline, base_path) }
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
    cukta_state: Signal<CuktaWebState>,
    id: Option<&str>,
    lines: &[CllLojbanizationLine],
    base_path: &str,
) -> Element {
    rsx! {
        table { id: id.unwrap_or_default(), class: "cll-lojbanization cll-lojbanization-table",
            tbody {
                for line in lines.iter() {
                    tr { class: "cll-lojbanization-row cll-lojbanization-line cll-lojbanization-line-{line.kind} cll-lojbanization-{line.kind}",
                        th { "{line.kind}" }
                        td {
                            for inline in line.body.iter() {
                                { render_cll_inline(cukta_state, inline, base_path) }
                            }
                        }
                        td {
                            if let Some(comment) = &line.comment {
                                for inline in comment.iter() {
                                    { render_cll_inline(cukta_state, inline, base_path) }
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
    cukta_state: Signal<CuktaWebState>,
    id: Option<&str>,
    parts: &[CllLujvoPart],
    base_path: &str,
) -> Element {
    rsx! {
        ul { id: id.unwrap_or_default(), class: "cll-lujvo-making",
            for part in parts.iter() {
                li { class: "cll-lujvo-part cll-lujvo-part-{part.kind}",
                    span { class: "cll-lujvo-part-kind", "{part.kind}" }
                    for inline in part.body.iter() {
                        { render_cll_inline(cukta_state, inline, base_path) }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_ebnf(id: Option<&str>, entries: &[CllEbnfEntry], base_path: &str) -> Element {
    rsx! {
        div { id: id.unwrap_or_default(), class: "cll-ebnf",
            for entry in entries.iter() {
                section { id: "{entry.anchor_id}", class: "cll-ebnf-entry",
                    div { class: "cll-ebnf-head",
                        { render_cll_ebnf_link("cll-ebnf-rule", &entry.rule_name, entry.rule_href.as_deref(), base_path) }
                        " "
                        span { class: "cll-ebnf-assign", "⩴" }
                    }
                    pre { class: "cll-ebnf-rhs",
                        { render_cll_ebnf_rhs(&entry.rhs, base_path) }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_ebnf_rhs(tokens: &[CllEbnfToken], base_path: &str) -> Element {
    let lines = wrap_ebnf_choice_lines(tokens);
    if lines.len() == 1 {
        let line = lines.into_iter().next().unwrap_or_default();
        return rsx! {
            for token in line.iter() {
                { render_cll_ebnf_token(token, base_path) }
            }
        };
    }
    rsx! {
        for line in lines.iter() {
            span { class: "cll-ebnf-choice-line",
                for token in line.iter() {
                    { render_cll_ebnf_token(token, base_path) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_ebnf_token(token: &CllEbnfToken, base_path: &str) -> Element {
    match token {
        CllEbnfToken::Text { body } => rsx! { "{body}" },
        CllEbnfToken::Operator { body } => {
            rsx! { span { class: "cll-ebnf-op", "{body}" } }
        }
        CllEbnfToken::Hash { body } => rsx! { span { class: "cll-ebnf-hash", "{body}" } },
        CllEbnfToken::Terminal { body, href } => {
            render_cll_ebnf_link("cll-ebnf-terminal", body, href.as_deref(), base_path)
        }
        CllEbnfToken::ElidableTerminator { body, href } => {
            render_cll_ebnf_elidable(body, href.as_deref(), base_path)
        }
        CllEbnfToken::Nonterminal { body, href } => {
            render_cll_ebnf_link("cll-ebnf-nonterminal", body, href.as_deref(), base_path)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_cll_ebnf_elidable(body: &str, href: Option<&str>, base_path: &str) -> Element {
    let pieces = cll_ebnf_elidable_hash_pieces(body);
    if let Some(href) = href {
        let tooltip = cll_dictionary_tooltip_for_href(base_path, href);
        let href = cll_ebnf_href(base_path, href);
        if let Some(card) = &tooltip {
            rsx! {
                span { class: "dictionary-tooltip-host",
                    a { class: "cll-ebnf-elidable", href: "{href}",
                        if let Some((prefix, suffix)) = pieces {
                            "{prefix}"
                            span { class: "cll-ebnf-hash", "#" }
                            "{suffix}"
                        } else {
                            "{body}"
                        }
                    }
                    { render_dictionary_tooltip(card, false, base_path) }
                }
            }
        } else {
            rsx! {
                a { class: "cll-ebnf-elidable", href: "{href}",
                    if let Some((prefix, suffix)) = pieces {
                        "{prefix}"
                        span { class: "cll-ebnf-hash", "#" }
                        "{suffix}"
                    } else {
                        "{body}"
                    }
                }
            }
        }
    } else {
        rsx! {
            span { class: "cll-ebnf-elidable",
                if let Some((prefix, suffix)) = pieces {
                    "{prefix}"
                    span { class: "cll-ebnf-hash", "#" }
                    "{suffix}"
                } else {
                    "{body}"
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
    base_path: &str,
) -> Element {
    if let Some(href) = href {
        let tooltip = cll_dictionary_tooltip_for_href(base_path, href);
        let href = cll_ebnf_href(base_path, href);
        if let Some(card) = &tooltip {
            rsx! {
                span { class: "dictionary-tooltip-host",
                    a { class: "{class_name}", href: "{href}", "{body}" }
                    { render_dictionary_tooltip(card, false, base_path) }
                }
            }
        } else {
            rsx! {
                a { class: "{class_name}", href: "{href}", "{body}" }
            }
        }
    } else {
        rsx! { span { class: "{class_name}", "{body}" } }
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
    if let Some(target) = href.strip_prefix("../vlacku/") {
        format!("{}/vlacku/{target}", base_path.trim_end_matches('/'))
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
                dialect: Some("allow-cgv".to_owned()),
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
                element.scroll_into_view();
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
fn set_cukta_state(cukta_state: &mut Signal<CuktaWebState>, state: CuktaWebState) {
    cukta_state.set(state);
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
    base_path: &str,
) -> Element {
    let committed_state = vlacku_committed_state.read().clone();
    let result_state = vlacku_result.read().clone();
    let result = if result_state.state.as_ref() == Some(&committed_state) {
        result_state.result
    } else {
        vlacku_loading_result(&committed_state, "Loading dictionary results.")
    };
    let draft_state = vlacku_draft_state.read().clone();
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
                    p { class: "dictionary-empty", "{message}" }
                }
                for error in result.errors.iter() {
                    div { class: "spa-error dictionary-error", "{error}" }
                }
                div { class: "dictionary-layout",
                    div { class: "dictionary-main-column",
                        { render_vlacku_body(&result, vlacku_draft_state, vlacku_committed_state, jvozba_pane, jvozba_available_value, base_path) }
                    }
                    if jvozba_available_value {
                        { render_vlacku_jvozba_pane(jvozba_pane, jvozba_drag) }
                    }
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
    base_path: &str,
) -> Element {
    rsx! {
        div { class: "dictionary-results",
            if !result.cards.is_empty() {
                div { class: "dictionary-results-grid",
                    for card in result.cards.iter() {
                        { render_vlacku_card(card, jvozba_pane, jvozba_available, base_path) }
                    }
                }
            }
            if result.has_more {
                div { class: "load-more-wrap",
                    button {
                        class: "btn-parse load-more-link",
                        r#type: "button",
                        onclick: move |_| {
                            let mut next = vlacku_draft_state.read().clone();
                            next.count = next.count.saturating_mul(2).clamp(1, VLACKU_WEB_MAX_COUNT);
                            set_vlacku_state_immediate(
                                &mut vlacku_draft_state,
                                &mut vlacku_committed_state,
                                next,
                            );
                        },
                        "Load more"
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
    base_path: &str,
) -> Element {
    rsx! {
        section { class: "result-card",
            header { class: "result-header",
                h2 { class: "word",
                    span { class: "dictionary-word-line",
                        { render_vlacku_headword_line(card, jvozba_pane, jvozba_available, base_path) }
                    }
                }
                div { class: "tag-row",
                    { render_vlacku_metadata_pill(card, base_path) }
                }
            }
            if !card.definition.is_empty() {
                p { class: "dictionary-definition-copy",
                    { render_inline_spans(&card.definition, jvozba_pane, jvozba_available, base_path) }
                }
            }
            if !card.glosses.is_empty() {
                div { class: "chip-row dictionary-gloss-row",
                    for gloss in card.glosses.iter() {
                        span { class: "chip dictionary-gloss-pill", title: "Gloss word", "{gloss}" }
                    }
                }
            }
            if !card.notes.is_empty() {
                p { class: "dictionary-note-copy", title: "Dictionary note",
                    { render_inline_spans(&card.notes, jvozba_pane, jvozba_available, base_path) }
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
) -> Element {
    rsx! {
        span { class: "rich-dictionary-tooltip", role: "tooltip",
            span { class: "tooltip-word-line",
                span { class: "tooltip-headword",
                    if show_link {
                        a { class: "tooltip-word", href: "{card.href}", "{card.display_word}" }
                    } else {
                        span { class: "tooltip-word", "{card.display_word}" }
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
                            if let Some(source) = &piece.source {
                                if show_link {
                                    a {
                                        class: "tooltip-rafsi-piece",
                                        href: "{piece.source_href.clone().unwrap_or_else(|| card.href.clone())}",
                                        span { class: "tooltip-rafsi-surface", "{piece.display_surface}" }
                                        span { class: "tooltip-rafsi-source", "{piece.display_source.as_deref().unwrap_or(source)}" }
                                    }
                                } else {
                                    span { class: "tooltip-rafsi-piece",
                                        span { class: "tooltip-rafsi-surface", "{piece.display_surface}" }
                                        span { class: "tooltip-rafsi-source", "{piece.display_source.as_deref().unwrap_or(source)}" }
                                    }
                                }
                            } else {
                                span { class: "tooltip-rafsi-piece",
                                    span { class: "tooltip-rafsi-surface", "{piece.display_surface}" }
                                }
                            }
                        }
                    }
                }
            }
            if !card.definition.is_empty() {
                span { class: "tooltip-copy",
                    { render_tooltip_inline_spans(&card.definition, base_path, show_link) }
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
                    { render_tooltip_inline_spans(&card.notes, base_path, show_link) }
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
) -> Element {
    rsx! {
        for span in spans.iter() {
            {
                match span.as_data() {
                    data!(VlackuInline::Text(text)) => rsx! { "{text}" },
                    data!(VlackuInline::Math(math)) => render_vlacku_math(math),
                    data!(VlackuInline::WordRef { label, href, .. }) => {
                        let resolved_href = resolved_href_with_base_path(base_path, href);
                        if interactive_links {
                            rsx! {
                                a { class: "tooltip-inline-link", href: "{resolved_href}", "{label}" }
                            }
                        } else {
                            rsx! {
                                span { class: "tooltip-inline-link", "{label}" }
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
    rsx! {
        { render_vlacku_headword_action(
            jvozba_pane,
            jvozba_available,
            card.can_add_to_jvozba,
            &card.word,
            &card.display_word,
            &word_href,
        ) }
        if let Some(ipa) = &card.ipa {
            span { class: "dictionary-headword-ipa", "/{ipa}/" }
        }
        if !card.decomposition.is_empty() {
            { render_vlacku_inline_separator("=") }
            { render_vlacku_decomposition_inline(card, jvozba_pane, jvozba_available, base_path) }
        } else if !card.rafsi.is_empty() {
            { render_vlacku_inline_separator("≘") }
            span { class: "dictionary-inline-pill-row",
                for rafsi in card.rafsi.iter() {
                    { render_rafsi_pill(jvozba_pane, jvozba_available, &card.word, rafsi) }
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
                "{display_word}"
            }
        }
    } else if pane_open {
        rsx! {
            span { class: "dictionary-headword-link", "{display_word}" }
        }
    } else {
        rsx! {
            a {
                class: "dictionary-headword-link",
                href: "{href}",
                "{display_word}"
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_decomposition_inline(
    card: &VlackuWebCard,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    base_path: &str,
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
            { render_composition_piece(piece, jvozba_pane, jvozba_available, base_path) }
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
fn render_vlacku_metadata_pill(card: &VlackuWebCard, base_path: &str) -> Element {
    rsx! {
        div { class: "dictionary-meta-pill",
            span { class: word_type_tag_class(&card.word_type_key), "{card.word_type}" }
            if let Some(selmaho) = &card.selmaho {
                { render_vlacku_selmaho_segment(card, selmaho, base_path) }
            }
            if let Some(similarity) = card.similarity {
                span { class: "dictionary-meta-segment dictionary-meta-tooltip", title: "Phonetic similarity to the current query.",
                    "{format_similarity(similarity)}"
                }
            }
            { render_vote_display(&card.votes) }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_selmaho_segment(card: &VlackuWebCard, selmaho: &str, base_path: &str) -> Element {
    if card.word_type_key == "gismu" {
        let href = format!("{}/cukta", base_path.trim_end_matches('/'));
        rsx! {
            a { class: "dictionary-meta-segment dictionary-selmaho-tag", href: "{href}", title: "CLL gismu section",
                em { "{selmaho}" }
            }
        }
    } else {
        rsx! {
            span { class: "dictionary-meta-segment dictionary-selmaho-tag", title: "selma'o classification",
                em { "{selmaho}" }
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
                        "{display_word}"
                    }
                    { render_dictionary_tooltip(card, false, base_path) }
                }
            }
        } else {
            rsx! {
                button {
                    class: "{class_name}",
                    r#type: "button",
                    title: "Add to jvozba",
                    onclick: move |_| add_vlacku_jvozba_word(&mut jvozba_pane, word_value.clone()),
                    "{display_word}"
                }
            }
        }
    } else if pane_open {
        rsx! {
            span { class: "{static_class_name}", "{display_word}" }
        }
    } else {
        if let Some(card) = &tooltip {
            rsx! {
                span { class: "dictionary-tooltip-host",
                    a { class: "{static_class_name}", href: "{href}", "{display_word}" }
                    { render_dictionary_tooltip(card, false, base_path) }
                }
            }
        } else {
            rsx! {
                a { class: "{static_class_name}", href: "{href}", "{display_word}" }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vote_display(votes: &VlackuVoteDisplay) -> Element {
    match votes {
        VlackuVoteDisplay::Known(value) => rsx! {
            span { class: vote_class(value), title: vote_title(value), "{value}" }
        },
        VlackuVoteDisplay::Unknown => rsx! {
            span { class: "dictionary-meta-segment dictionary-meta-tooltip dictionary-vote-tag is-unknown", title: "This parses as a valid Lojban word, but it is not present in the embedded dictionary, so no Lensisku vote tally is available.", "?" }
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
) -> Element {
    match piece.kind {
        VlackuCompositionPieceKind::Hyphen => rsx! {
            span { class: "dictionary-word-inline-separator", "{piece.display_surface}" }
        },
        VlackuCompositionPieceKind::Rafsi => {
            if let Some(source) = &piece.source {
                let href = vlacku_web_url(
                    base_path,
                    &VlackuWebState {
                        mode: VlackuWebMode::Word,
                        query: source.clone(),
                        count: VLACKU_WEB_DEFAULT_COUNT,
                        word_types: Vec::new(),
                    },
                );
                rsx! {
                    span { class: "rafsi-split-pill",
                        { render_vlacku_rafsi_add_piece(jvozba_pane, jvozba_available, &piece.surface, source, &piece.display_surface) }
                        span { class: "rafsi-split-right",
                            { render_vlacku_word_action(
                                jvozba_pane,
                                jvozba_available,
                                true,
                                source,
                                piece.display_source.as_deref().unwrap_or(source),
                                &href,
                                "dictionary-word-link rafsi-source-link dictionary-jvozba-add-link-hint",
                                base_path,
                            ) }
                        }
                    }
                }
            } else {
                rsx! {
                    span { class: "chip rafsi-chip", "{piece.display_surface}" }
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
                "{display_rafsi}"
            }
        }
    } else {
        rsx! { span { class: "rafsi-split-left", "{display_rafsi}" } }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_rafsi_pill(
    mut jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    source_word: &str,
    rafsi: &str,
) -> Element {
    let pane_open = jvozba_available && jvozba_pane.read().open;
    let rafsi_value = rafsi.to_owned();
    let source_value = source_word.to_owned();
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
                "{rafsi}"
            }
        }
    } else {
        rsx! { span { class: "chip rafsi-chip", "{rafsi}" } }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_inline_spans(
    spans: &[VlackuInline],
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    base_path: &str,
) -> Element {
    rsx! {
        for span in spans.iter() {
            {
                match span.as_data() {
                    data!(VlackuInline::Text(text)) => rsx! { "{text}" },
                    data!(VlackuInline::Math(math)) => render_vlacku_math(math),
                    data!(VlackuInline::WordRef { label, href, can_add_to_jvozba }) => {
                        render_vlacku_inline_word_ref(jvozba_pane, jvozba_available, *can_add_to_jvozba, label, href, base_path)
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
) -> Element {
    let pane_open = jvozba_available && jvozba_pane.read().open;
    let word_value = label.to_owned();
    let resolved_href = resolved_href_with_base_path(base_path, href);
    let tooltip = dictionary_tooltip_for_word(base_path, label);
    if pane_open && can_add_to_jvozba {
        if let Some(card) = &tooltip {
            rsx! {
                span { class: "dictionary-tooltip-host",
                    button {
                        class: "dictionary-word-link dictionary-jvozba-add-link-hint",
                        r#type: "button",
                        title: "Add to jvozba",
                        onclick: move |_| add_vlacku_jvozba_word(&mut jvozba_pane, word_value.clone()),
                        "{label}"
                    }
                    { render_dictionary_tooltip(card, false, base_path) }
                }
            }
        } else {
            rsx! {
                button {
                    class: "dictionary-word-link dictionary-jvozba-add-link-hint",
                    r#type: "button",
                    title: "Add to jvozba",
                    onclick: move |_| add_vlacku_jvozba_word(&mut jvozba_pane, word_value.clone()),
                    "{label}"
                }
            }
        }
    } else if pane_open {
        rsx! {
            span { class: "dictionary-word-link", "{label}" }
        }
    } else {
        if let Some(card) = &tooltip {
            rsx! {
                span { class: "dictionary-tooltip-host",
                    a { class: "dictionary-word-link", href: "{resolved_href}", "{label}" }
                    { render_dictionary_tooltip(card, false, base_path) }
                }
            }
        } else {
            rsx! {
                a { class: "dictionary-word-link", href: "{resolved_href}", "{label}" }
            }
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
        span { class: "spa-cll-math",
            math { class: "math-var", display: "inline",
                mrow {
                    for part in math.parts.iter() {
                        { render_vlacku_math_part(part) }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_math_part(part: &VlackuMathPart) -> Element {
    match part.as_data() {
        data!(VlackuMathPart::Text(text)) => rsx! { mtext { "{text}" } },
        data!(VlackuMathPart::Operator(text)) => rsx! { mo { "{text}" } },
        data!(VlackuMathPart::Variable { stem, subscript }) => {
            let math_stem = math_alphanumeric_stem(stem);
            if let Some(subscript) = subscript {
                rsx! {
                    msub {
                        mi { "{math_stem}" }
                        mn { "{subscript}" }
                    }
                }
            } else {
                rsx! { mi { "{math_stem}" } }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_jvozba_pane(
    mut jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_drag: Signal<Option<VlackuJvozbaDragState>>,
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
                        { render_jvozba_output(&output) }
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
                            { render_jvozba_item(jvozba_pane, jvozba_drag, index, item) }
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
                { render_jvozba_item_chip(item) }
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
fn render_jvozba_item_chip(item: &VlackuJvozbaItem) -> Element {
    match item.kind {
        VlackuJvozbaItemKind::FixedRafsi => {
            let source_label = item.source.as_deref().unwrap_or("rafsi");
            rsx! {
                span { class: "rafsi-split-pill dictionary-jvozba-pane-rafsi-pill",
                    span { class: "rafsi-split-left", "{item.value}" }
                    span { class: "rafsi-split-right", "{source_label}" }
                }
            }
        }
        VlackuJvozbaItemKind::Word => rsx! {
            span { class: "chip dictionary-jvozba-pane-word-chip", "{item.value}" }
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn render_jvozba_output(output: &VlackuJvozbaOutput) -> Element {
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
                    span { class: jvozba_segment_class(segment.tone), "{segment.text}" }
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
        VlackuWebMode::Word => "valsi",
        VlackuWebMode::Rafsi => "rafsi",
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
        "Official baseline lexicon word. The infinity marker replaces the raw Lensisku community tally once the official-word threshold is exceeded."
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
fn render_result(
    result: &GentufaWebResult,
    view_mode: Signal<GentufaWebViewMode>,
    view_mode_value: GentufaWebViewMode,
    display: Signal<GentufaDisplayState>,
    display_value: GentufaDisplayState,
    settings_value: UserSettings,
    reference_hover: Signal<ReferenceHoverState>,
    activity: Signal<AsyncActivityState>,
    export_task: Signal<Option<LatestAsyncTask>>,
) -> Element {
    match result {
        GentufaWebResult::Blank => rsx! {},
        GentufaWebResult::Error(error) => render_error(error),
        GentufaWebResult::Success(success) => render_success(
            success,
            view_mode,
            view_mode_value,
            display,
            display_value,
            settings_value,
            reference_hover,
            activity,
            export_task,
        ),
    }
}

#[requires(true)]
#[ensures(true)]
fn render_error(error: &GentufaError) -> Element {
    rsx! {
        section { class: "result-section error-section",
            div { class: "error-box failure-errors",
                pre { class: "error-message", "{error.message}" }
                if !error.diagnostics.is_empty() {
                    ul { class: "error-list",
                        for diagnostic in error.diagnostics.iter() {
                            li { class: diagnostic_class(diagnostic),
                                strong { "{diagnostic.code}" }
                                span { " {diagnostic.message}" }
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
fn render_success(
    success: &GentufaSuccess,
    view_mode: Signal<GentufaWebViewMode>,
    view_mode_value: GentufaWebViewMode,
    display: Signal<GentufaDisplayState>,
    display_value: GentufaDisplayState,
    settings_value: UserSettings,
    reference_hover: Signal<ReferenceHoverState>,
    activity: Signal<AsyncActivityState>,
    export_task: Signal<Option<LatestAsyncTask>>,
) -> Element {
    let reference_hover_value = reference_hover.read().clone();
    rsx! {
        section { class: "result-section",
            { render_reference_overlay(&reference_hover_value) }
            { render_surface_output(success) }
            { render_diagnostics(success) }
            div { class: "view-toolbar",
                { render_view_tabs(view_mode, view_mode_value) }
                { render_output_controls(display, display_value) }
            }
            match view_mode_value {
                GentufaWebViewMode::Blocks => rsx! {
                    { render_blocks(success, display_value.show_glosses, settings_value.script, reference_hover, activity, export_task) }
                },
                GentufaWebViewMode::Tree => rsx! {
                    { render_tree(success, reference_hover) }
                },
                GentufaWebViewMode::Ipa => rsx! {
                    { render_ipa_output(success) }
                },
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_surface_output(success: &GentufaSuccess) -> Element {
    rsx! {
        div { class: "brackets-section",
            div { class: "brackets-output-stack",
                pre { class: "brackets-output compact-output",
                    span { class: "brackets-output-markup",
                        for fragment in success.bracket_fragments.iter() {
                            { render_bracket_fragment(fragment) }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_bracket_fragment(fragment: &GentufaBracketFragment) -> Element {
    match fragment {
        GentufaBracketFragment::Text { text, elided } => {
            if *elided {
                rsx! { s { "{text}" } }
            } else {
                rsx! { "{text}" }
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
                if let Some(card) = tooltip {
                    rsx! {
                        span {
                            class: "bracket-fragment bracket-word dictionary-tooltip-host",
                            style: "{style}",
                            a { class: "bracket-word-link", href: "{href}",
                                for child in children.iter() {
                                    { render_bracket_fragment(child) }
                                }
                            }
                            { render_dictionary_tooltip(card, false, "") }
                        }
                    }
                } else {
                    rsx! {
                        a {
                            class: "bracket-fragment bracket-word",
                            style: "{style}",
                            href: "{href}",
                            for child in children.iter() {
                                { render_bracket_fragment(child) }
                            }
                        }
                    }
                }
            } else {
                rsx! {
                    span { class: "bracket-fragment", style: "{style}",
                        for child in children.iter() {
                            { render_bracket_fragment(child) }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_diagnostics(success: &GentufaSuccess) -> Element {
    if success.diagnostics.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "lean-warning-bar syntax-warning-list", role: "alert", aria_live: "polite",
            pre { class: "lean-warning-text",
                for diagnostic in success.diagnostics.iter() {
                    span { class: diagnostic_class(diagnostic),
                        "{diagnostic.code}: {diagnostic.message}\n"
                    }
                }
            }
        }
    }
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
                onclick: move |_| view_mode.set(GentufaWebViewMode::Blocks),
                "Blocks"
            }
            button {
                class: view_tab_class(current == GentufaWebViewMode::Tree),
                r#type: "button",
                aria_current: if current == GentufaWebViewMode::Tree { "page" } else { "false" },
                onclick: move |_| view_mode.set(GentufaWebViewMode::Tree),
                "Tree"
            }
            button {
                class: view_tab_class(current == GentufaWebViewMode::Ipa),
                r#type: "button",
                aria_current: if current == GentufaWebViewMode::Ipa { "page" } else { "false" },
                onclick: move |_| view_mode.set(GentufaWebViewMode::Ipa),
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
fn render_ipa_output(success: &GentufaSuccess) -> Element {
    rsx! {
        section { class: "ipa-view",
            pre { class: "ipa-tab-output", "{success.ipa_text}" }
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
    activity: Signal<AsyncActivityState>,
    export_task: Signal<Option<LatestAsyncTask>>,
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
                                { render_block(block, reference_hover, export_anchor_id, &success.blocks_layout, show_glosses, script, activity, export_task) }
                            }
                            if show_glosses {
                                for block in success.blocks_layout.blocks.iter().filter(|block| block.is_leaf) {
                                    { render_gloss_block(block, gloss_row) }
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
    export_anchor_id: Option<&str>,
    export_layout: &GentufaBlocksLayout,
    export_show_glosses: bool,
    export_script: GentufaScript,
    activity: Signal<AsyncActivityState>,
    export_task: Signal<Option<LatestAsyncTask>>,
) -> Element {
    let row = block.row + 1;
    let col = block.col + 1;
    let classes = block_class(block);
    let hover_state = reference_hover.read().clone();
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
                            { render_ref_marker(marker, reference_hover, &hover_state) }
                        }
                    }
                }
            }
            if let Some(card) = &block.tooltip {
                span {
                    class: "block-label dictionary-tooltip-host",
                    title: "{block.label}",
                    a { class: "block-label-link", href: "{card.href}",
                        span { class: "block-label-text",
                            { render_elidable_text(&block.label, block.is_elided) }
                        }
                    }
                    { render_dictionary_tooltip(card, false, "") }
                }
            } else {
                span { class: "block-label", title: "{block.label}",
                    span { class: "block-label-text",
                        { render_elidable_text(&block.label, block.is_elided) }
                    }
                }
            }
            if block.ref_markers.iter().any(|marker| marker.role == ReferenceMarkerRole::Reference) {
                span { class: "block-ref-source",
                    span { class: "ref-math",
                        for marker in block.ref_markers.iter().filter(|marker| marker.role == ReferenceMarkerRole::Reference) {
                            span { class: "ref-arrow", "→" }
                            { render_ref_marker(marker, reference_hover, &hover_state) }
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
#[ensures(true)]
fn render_ref_marker(
    marker: &ReferenceMarker,
    reference_hover: Signal<ReferenceHoverState>,
    hover_state: &ReferenceHoverState,
) -> Element {
    let class = reference_marker_class(marker, hover_state);
    let role = reference_role_attr(marker.role);
    let base = marker.label.base_key();
    let label = marker.label.full_key();
    let enter_hover = reference_hover;
    let leave_hover = reference_hover;
    let enter_role = marker.role;
    let enter_label = marker.label.clone();
    rsx! {
        span {
            class: "{class}",
            title: "{marker.kind}",
            "data-ref-role": "{role}",
            "data-ref-kind": "{marker.kind}",
            "data-ref-label": "{label}",
            "data-ref-base": "{base}",
            onmouseenter: move |_| set_reference_hover(enter_hover, enter_role, enter_label.clone()),
            onmouseleave: move |_| clear_reference_hover(leave_hover),
            { render_reference_label(&marker.label) }
        }
    }
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

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
async fn download_gentufa_blocks_svg_result(
    _layout: GentufaBlocksLayout,
    _show_glosses: bool,
    _script: GentufaScript,
) -> Result<(), String> {
    Err("gentufa SVG export is only available in the browser".to_owned())
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

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
async fn download_gentufa_blocks_png_result(
    _layout: GentufaBlocksLayout,
    _show_glosses: bool,
    _script: GentufaScript,
) -> Result<(), String> {
    Err("gentufa PNG export is only available in the browser".to_owned())
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

#[requires(true)]
#[ensures(true)]
fn render_gloss_block(block: &GentufaBlock, gloss_row: usize) -> Element {
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
            div { class: "gloss-list", "{text}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_tree(success: &GentufaSuccess, reference_hover: Signal<ReferenceHoverState>) -> Element {
    rsx! {
        div { class: "table-view",
            div { class: "table-wrap",
                svg { class: "tree-lines", "aria-hidden": "true" }
                table { class: "parse-table spa-gentufa-table",
                    thead {
                        tr {
                            th { class: "col-edge col-edge-in", div { class: "cell-pad", "" } }
                            th { class: "col-node", div { class: "cell-pad", "Node" } }
                            th { class: "col-edge col-edge-out", div { class: "cell-pad", "" } }
                            th { class: "col-text", div { class: "cell-pad", "Text" } }
                        }
                    }
                    tbody {
                        for row in success.tree_rows.iter() {
                            { render_tree_row(row, reference_hover) }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_tree_row(row: &GentufaTreeRow, reference_hover: Signal<ReferenceHoverState>) -> Element {
    let row_class = class_names("tree-row", &[("elided-row", tree_row_is_elided(row))]);
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
            "data-color": "{row.color}",
            { render_tree_edge_cell("in", incoming_markers, false, reference_hover, &hover_state) }
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
                            "{row.label}"
                        }
                    }
                }
            }
            { render_tree_edge_cell("out", outgoing_markers, true, reference_hover, &hover_state) }
            td { class: "col-text",
                div { class: "cell-pad",
                    for cell in row.cells.iter() {
                        { render_tree_cell(cell) }
                    }
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

#[requires(!side.is_empty())]
#[ensures(true)]
fn render_tree_edge_cell(
    side: &str,
    markers: Vec<&ReferenceMarker>,
    arrow_before: bool,
    reference_hover: Signal<ReferenceHoverState>,
    hover_state: &ReferenceHoverState,
) -> Element {
    let class = format!("col-edge col-edge-{side}");
    let has_markers = !markers.is_empty();
    rsx! {
        td { class: "{class}",
            div { class: "cell-pad edge-cell",
                if has_markers && arrow_before {
                    span { class: "ref-arrow edge-arrow", "→" }
                }
                for marker in markers {
                    { render_ref_marker(marker, reference_hover, hover_state) }
                }
                if has_markers && !arrow_before {
                    span { class: "ref-arrow edge-arrow", "→" }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_tree_cell(cell: &GentufaCell) -> Element {
    let class = if cell.is_elided {
        "token is-elided"
    } else {
        "token"
    };
    rsx! {
        span { class: "{class}",
            span { class: "token-raw lojban-text",
                { render_elidable_text(&cell.text, cell.is_elided) }
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
    let overlay = measure_reference_overlay(&hovered);
    reference_hover.set(ReferenceHoverState {
        hovered: Some(hovered),
        overlay,
    });
}

#[requires(true)]
#[ensures(true)]
fn clear_reference_hover(mut reference_hover: Signal<ReferenceHoverState>) {
    reference_hover.set(ReferenceHoverState::default());
}

#[requires(true)]
#[ensures(true)]
fn refresh_reference_hover(mut reference_hover: Signal<ReferenceHoverState>) {
    let Some(hovered) = reference_hover.read().hovered.clone() else {
        return;
    };
    let overlay = measure_reference_overlay(&hovered);
    reference_hover.set(ReferenceHoverState {
        hovered: Some(hovered),
        overlay,
    });
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

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
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
    current: UserSettings,
    embedding_settings: Signal<EmbeddingSettingsState>,
    activity: Signal<AsyncActivityState>,
) -> Element {
    let embedding_state = embedding_settings.read().clone();
    rsx! {
        section { class: "spa-page settings-page",
            div { class: "page-container settings-container",
                h1 { "Settings" }
                { render_embedding_settings(embedding_settings, &embedding_state, activity) }
                section { class: "settings-section",
                    h2 { "Theme" }
                    { render_theme_switch(settings, current.theme) }
                }
                section { class: "settings-section",
                    h2 { "Script" }
                    { render_script_switch(settings, current.script) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_embedding_settings(
    mut embedding_settings: Signal<EmbeddingSettingsState>,
    state: &EmbeddingSettingsState,
    activity: Signal<AsyncActivityState>,
) -> Element {
    let busy = state.busy;
    rsx! {
        section { class: "settings-section embeddings-settings",
            h2 { "Embeddings" }
            div { class: "settings-kv-grid",
                span { class: "settings-kv-label", "Status" }
                span { class: "settings-kv-value", "{state.status}" }
                span { class: "settings-kv-label", "Model" }
                span { class: "settings-kv-value", "{state.model_size}" }
                span { class: "settings-kv-label", "Index" }
                span { class: "settings-kv-value", "{state.index_size}" }
            }
            p { class: "settings-help-text", "{state.detail}" }
            { render_embedding_progress(state) }
            div { class: "settings-actions",
                button {
                    class: "settings-action-button",
                    r#type: "button",
                    disabled: busy,
                    onclick: move |_| {
                        let mut next = embedding_settings.read().clone();
                        next.busy = true;
                        next.detail = "Downloading model and preparing the browser index.".to_owned();
                        next.progress_label = Some("Embedding setup".to_owned());
                        next.progress_percent = None;
                        embedding_settings.set(next);
                        spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                            poll_embedding_settings_while_busy(embedding_settings).await;
                        });
                        spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                            setup_browser_embeddings(embedding_settings).await;
                        });
                    },
                    "Download"
                }
                button {
                    class: "settings-action-button",
                    r#type: "button",
                    disabled: busy,
                    onclick: move |_| {
                        let mut next = embedding_settings.read().clone();
                        next.busy = true;
                        next.detail = "Checking for a compatible vector pack.".to_owned();
                        next.progress_label = Some("Embedding setup".to_owned());
                        next.progress_percent = None;
                        embedding_settings.set(next);
                        spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                            poll_embedding_settings_while_busy(embedding_settings).await;
                        });
                        spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                            setup_browser_embeddings(embedding_settings).await;
                        });
                    },
                    "Update"
                }
                button {
                    class: "settings-action-button danger",
                    r#type: "button",
                    disabled: busy,
                    onclick: move |_| {
                        let mut next = embedding_settings.read().clone();
                        next.busy = true;
                        next.detail = "Removing browser embedding storage.".to_owned();
                        next.progress_label = None;
                        next.progress_percent = None;
                        embedding_settings.set(next);
                        spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                            remove_browser_embeddings(embedding_settings).await;
                        });
                    },
                    "Remove"
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_embedding_progress(state: &EmbeddingSettingsState) -> Element {
    if !state.busy && state.progress_percent.is_none() {
        return rsx! {};
    }
    let label = state.progress_label.as_deref().unwrap_or("Embedding setup");
    if let Some(percent) = state.progress_percent {
        rsx! {
            div { class: "settings-progress-row",
                progress {
                    class: "settings-progress",
                    max: "100",
                    value: "{percent}",
                    aria_label: "{label}",
                }
                span { class: "settings-progress-label", "{label} {percent}%" }
            }
        }
    } else {
        rsx! {
            div { class: "settings-progress-row",
                progress {
                    class: "settings-progress",
                    aria_label: "{label}",
                }
                span { class: "settings-progress-label", "{label}" }
            }
        }
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
) -> GentufaWebOptions {
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
#[ensures(true)]
fn diagnostic_class(diagnostic: &jbotci_diagnostics::Diagnostic) -> &'static str {
    match diagnostic.severity {
        jbotci_diagnostics::DiagnosticSeverity::Error => "diagnostic error",
        jbotci_diagnostics::DiagnosticSeverity::Warning
        | jbotci_diagnostics::DiagnosticSeverity::Advice => "diagnostic",
    }
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

#[requires(base_path.starts_with('/'))]
#[ensures(ret.starts_with('/'))]
fn nav_href(base_path: &str, route: AppRoute) -> String {
    let path = match route {
        AppRoute::Gentufa => "/gentufa",
        AppRoute::Settings => "/settings",
        AppRoute::Cukta => "/cukta",
        AppRoute::Vlacku => "/vlacku",
    };
    if base_path == "/" {
        path.to_owned()
    } else {
        format!("{base_path}{path}")
    }
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
fn initial_vlacku_state() -> VlackuWebState {
    parse_vlacku_web_route(&logical_current_path(), &current_query())
}

#[requires(true)]
#[ensures(true)]
fn initial_cukta_state() -> CuktaWebState {
    parse_cukta_web_route(&logical_current_path(), &current_query())
}

#[requires(true)]
#[ensures(true)]
fn initial_gentufa_state() -> GentufaWebState {
    parse_gentufa_web_route(&logical_current_path(), &current_query())
}

#[requires(true)]
#[ensures(true)]
fn initial_gentufa_text_explicit() -> bool {
    current_query_has_key("text")
}

#[requires(true)]
#[ensures(true)]
fn route_from_current_path() -> AppRoute {
    route_from_path(&current_path())
}

#[requires(true)]
#[ensures(true)]
fn base_path_from_current_path() -> String {
    let path = current_path();
    if path == "/jbotci" || path.starts_with("/jbotci/") {
        "/jbotci".to_owned()
    } else {
        "/".to_owned()
    }
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn route_from_path(path: &str) -> AppRoute {
    let logical = path.strip_prefix("/jbotci").unwrap_or(path);
    let logical = if logical == "/" {
        logical
    } else {
        logical.trim_end_matches('/')
    };
    match logical {
        "" | "/" | "/gentufa" => AppRoute::Gentufa,
        "/settings" => AppRoute::Settings,
        "/cukta" => AppRoute::Cukta,
        "/cukta/index" | "/cukta/search" => AppRoute::Cukta,
        "/vlacku" => AppRoute::Vlacku,
        _ if logical.starts_with("/cukta/section/") => AppRoute::Cukta,
        _ if logical.starts_with("/vlacku/") => AppRoute::Vlacku,
        _ => AppRoute::Gentufa,
    }
}

#[requires(true)]
#[ensures(ret.starts_with('/'))]
fn logical_current_path() -> String {
    let path = current_path();
    path.strip_prefix("/jbotci").unwrap_or(&path).to_owned()
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

#[allow(clippy::too_many_arguments)]
#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn install_browser_state_handlers(
    route: Signal<AppRoute>,
    cukta_state: Signal<CuktaWebState>,
    vlacku_draft_state: Signal<VlackuWebState>,
    vlacku_committed_state: Signal<VlackuWebState>,
    input_text: Signal<String>,
    parsed_text: Signal<String>,
    dialect: Signal<String>,
    parsed_dialect: Signal<String>,
    view_mode: Signal<GentufaWebViewMode>,
    gentufa_display: Signal<GentufaDisplayState>,
    gentufa_text_explicit: Signal<bool>,
    pending_cukta_scroll: Signal<Option<String>>,
    jvozba_available: Signal<bool>,
    topbar_settings_layout: Signal<TopbarSettingsLayout>,
    topbar_settings_open: Signal<bool>,
    base_path: &str,
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
    let base_path_for_click = base_path.to_owned();
    let click_route = route;
    let click_cukta = cukta_state;
    let click_vlacku_draft = vlacku_draft_state;
    let click_vlacku_committed = vlacku_committed_state;
    let click_input = input_text;
    let click_parsed = parsed_text;
    let click_dialect = dialect;
    let click_parsed_dialect = parsed_dialect;
    let click_view = view_mode;
    let click_display = gentufa_display;
    let click_text_explicit = gentufa_text_explicit;
    let click_pending_cukta_scroll = pending_cukta_scroll;
    let mut click_topbar_open = topbar_settings_open;
    let click_closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        let Some(href) = internal_href_from_click_event(&event, &base_path_for_click) else {
            return;
        };
        event.prevent_default();
        event.stop_propagation();
        click_topbar_open.set(false);
        navigate_to_internal_href(
            &href,
            &base_path_for_click,
            true,
            click_route,
            click_cukta,
            click_vlacku_draft,
            click_vlacku_committed,
            click_input,
            click_parsed,
            click_dialect,
            click_parsed_dialect,
            click_view,
            click_display,
            click_text_explicit,
            click_pending_cukta_scroll,
        );
    }) as Box<dyn FnMut(_)>);
    let _ = document.add_event_listener_with_callback_and_bool(
        "click",
        click_closure.as_ref().unchecked_ref(),
        true,
    );
    click_closure.forget();

    let base_path_for_pop = base_path.to_owned();
    let pop_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        let logical_path = logical_current_path();
        let query = current_query();
        let web_route = parse_web_route(&logical_path, &query);
        apply_web_route_to_client_state(
            &web_route,
            current_query_has_key("text"),
            route,
            cukta_state,
            vlacku_draft_state,
            vlacku_committed_state,
            input_text,
            parsed_text,
            dialect,
            parsed_dialect,
            view_mode,
            gentufa_display,
            gentufa_text_explicit,
        );
        restore_scroll_for_current_url();
        if let Some(hash) = current_hash() {
            let target = format!(
                "{}{}#{}",
                current_path(),
                current_query(),
                hash.trim_start_matches('#')
            );
            if app_route_for_web_route(&web_route) == AppRoute::Cukta {
                let mut pending_scroll = pending_cukta_scroll;
                pending_scroll.set(Some(target));
            }
        }
        let _ = base_path_for_pop;
    }) as Box<dyn FnMut(_)>);
    let _ =
        window.add_event_listener_with_callback("popstate", pop_closure.as_ref().unchecked_ref());
    pop_closure.forget();

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

    let resize_layout = topbar_settings_layout;
    let resize_open = topbar_settings_open;
    let resize_jvozba_available = jvozba_available;
    let resize_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        schedule_gentufa_block_reference_layout();
        schedule_gentufa_tree_layout();
        schedule_topbar_settings_layout_measure(resize_layout, resize_open);
        update_vlacku_jvozba_availability(resize_jvozba_available);
        schedule_vlacku_jvozba_pane_metrics_sync();
    }) as Box<dyn FnMut(_)>);
    let _ =
        window.add_event_listener_with_callback("resize", resize_closure.as_ref().unchecked_ref());
    resize_closure.forget();

    let load_layout = topbar_settings_layout;
    let load_open = topbar_settings_open;
    let load_jvozba_available = jvozba_available;
    let window_load_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        schedule_gentufa_block_reference_layout();
        schedule_gentufa_tree_layout();
        schedule_topbar_settings_layout_measure(load_layout, load_open);
        update_vlacku_jvozba_availability(load_jvozba_available);
        schedule_vlacku_jvozba_pane_metrics_sync();
    }) as Box<dyn FnMut(_)>);
    let _ = window
        .add_event_listener_with_callback("load", window_load_closure.as_ref().unchecked_ref());
    window_load_closure.forget();

    let stylesheet_layout = topbar_settings_layout;
    let stylesheet_open = topbar_settings_open;
    let stylesheet_load_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if event_target_is_stylesheet_link(&event) {
            schedule_gentufa_block_reference_layout();
            schedule_gentufa_tree_layout();
            schedule_topbar_settings_layout_measure(stylesheet_layout, stylesheet_open);
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
    );
    schedule_vlacku_jvozba_pane_metrics_after_fonts_ready(&document);

    let scroll_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        save_current_scroll_position();
    }) as Box<dyn FnMut(_)>);
    let _ =
        window.add_event_listener_with_callback("scroll", scroll_closure.as_ref().unchecked_ref());
    scroll_closure.forget();
    restore_scroll_for_current_url();
}

#[allow(clippy::too_many_arguments)]
#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn install_browser_state_handlers(
    route: Signal<AppRoute>,
    cukta_state: Signal<CuktaWebState>,
    vlacku_draft_state: Signal<VlackuWebState>,
    vlacku_committed_state: Signal<VlackuWebState>,
    input_text: Signal<String>,
    parsed_text: Signal<String>,
    dialect: Signal<String>,
    parsed_dialect: Signal<String>,
    view_mode: Signal<GentufaWebViewMode>,
    gentufa_display: Signal<GentufaDisplayState>,
    gentufa_text_explicit: Signal<bool>,
    pending_cukta_scroll: Signal<Option<String>>,
    jvozba_available: Signal<bool>,
    topbar_settings_layout: Signal<TopbarSettingsLayout>,
    topbar_settings_open: Signal<bool>,
    base_path: &str,
) {
    let _ = (
        route,
        cukta_state,
        vlacku_draft_state,
        vlacku_committed_state,
        input_text,
        parsed_text,
        dialect,
        parsed_dialect,
        view_mode,
        gentufa_display,
        gentufa_text_explicit,
        pending_cukta_scroll,
        jvozba_available,
        topbar_settings_layout,
        topbar_settings_open,
        base_path,
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
    if textarea.value().is_empty() {
        let _ = style.remove_property("height");
        return;
    }
    let _ = style.set_property("height", "auto");
    let height = textarea_html.scroll_height().saturating_add(2);
    let _ = style.set_property("height", &format!("{height}px"));
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

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy)]
#[invariant(true)]
struct GentufaTreeLineAnchor {
    parent_id: Option<usize>,
    label_left: f64,
    label_right: f64,
    label_center_y: f64,
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
    let width = f64::from(table_html.scroll_width()).max(table.get_bounding_client_rect().width());
    let height =
        f64::from(table_html.scroll_height()).max(table.get_bounding_client_rect().height());
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let _ = svg.set_attribute("width", &format!("{width:.3}"));
    let _ = svg.set_attribute("height", &format!("{height:.3}"));
    let _ = svg.set_attribute("viewBox", &format!("0 0 {width:.3} {height:.3}"));
    let Ok(row_nodes) = table.query_selector_all("tbody tr.tree-row") else {
        return;
    };
    let mut anchors = HashMap::new();
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
        anchors.insert(node_id, anchor);
    }
    for child in anchors.values() {
        let Some(parent_id) = child.parent_id else {
            continue;
        };
        let Some(parent) = anchors.get(&parent_id) else {
            continue;
        };
        append_gentufa_tree_line_path(&document, &svg, parent, child);
    }
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
    let wrap_rect = wrap.get_bounding_client_rect();
    let scroll_left = f64::from(wrap_html.scroll_left());
    let scroll_top = f64::from(wrap_html.scroll_top());
    Some(GentufaTreeLineAnchor {
        parent_id: element_usize_attr(row, "data-parent-id"),
        label_left: label_rect.left() - wrap_rect.left() + scroll_left,
        label_right: label_rect.right() - wrap_rect.left() + scroll_left,
        label_center_y: label_rect.top() - wrap_rect.top() + scroll_top + label_rect.height() / 2.0,
    })
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn append_gentufa_tree_line_path(
    document: &web_sys::Document,
    svg: &web_sys::Element,
    parent: &GentufaTreeLineAnchor,
    child: &GentufaTreeLineAnchor,
) {
    let Ok(path) = document.create_element_ns(Some("http://www.w3.org/2000/svg"), "path") else {
        return;
    };
    let d = format!(
        "M {:.3} {:.3} V {:.3} H {:.3}",
        parent.label_right, parent.label_center_y, child.label_center_y, child.label_left
    );
    let _ = path.set_attribute("class", "tree-line");
    let _ = path.set_attribute("d", &d);
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

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy)]
#[invariant(true)]
struct ReferenceBottoms {
    stack_bottom: f64,
    overlapping_label_bottom: Option<f64>,
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
    let Some(target) = event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
    else {
        return;
    };
    let Ok(Some(host)) = target.closest(".dictionary-tooltip-host") else {
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
    let Ok(hosts) = document.query_selector_all(".dictionary-tooltip-host") else {
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
    let _ = style.remove_property("visibility");
    let _ = style.remove_property("pointer-events");
    let _ = style.remove_property("transform");
    let _ = style.remove_property("transition");
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn dictionary_tooltip_element_for_host(host: &web_sys::Element) -> Option<web_sys::HtmlElement> {
    host.query_selector(".rich-dictionary-tooltip")
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn position_dictionary_tooltip(host: &web_sys::Element) {
    let Ok(Some(tooltip)) = host.query_selector(".rich-dictionary-tooltip") else {
        return;
    };
    let Some(tooltip_html) = tooltip.dyn_ref::<web_sys::HtmlElement>() else {
        return;
    };
    let Some(window) = web_sys::window() else {
        return;
    };
    let host_rect = host.get_bounding_client_rect();
    let tooltip_rect = tooltip.get_bounding_client_rect();
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
    let margin = 8.0;
    let gap = 8.0;
    let tooltip_width = tooltip_rect.width().max(1.0);
    let tooltip_height = tooltip_rect.height().max(1.0);
    let max_left = (viewport_width - tooltip_width - margin).max(margin);
    let centered_left = host_rect.left() + host_rect.width() / 2.0 - tooltip_width / 2.0;
    let left = centered_left.clamp(margin, max_left);
    let preferred_top = host_rect.top() - tooltip_height - gap;
    let max_top = (viewport_height - tooltip_height - margin).max(margin);
    let top = if preferred_top >= margin {
        preferred_top.min(max_top)
    } else {
        (host_rect.bottom() + gap).clamp(margin, max_top)
    };
    let style = tooltip_html.style();
    let _ = style.set_property("--dictionary-tooltip-left", &format!("{left:.2}px"));
    let _ = style.set_property("--dictionary-tooltip-top", &format!("{top:.2}px"));
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn internal_href_from_click_event(event: &web_sys::MouseEvent, base_path: &str) -> Option<String> {
    if event.default_prevented()
        || event.button() != 0
        || event.alt_key()
        || event.ctrl_key()
        || event.meta_key()
        || event.shift_key()
    {
        return None;
    }
    let target = event.target()?.dyn_into::<web_sys::Element>().ok()?;
    let anchor = target.closest("a[href]").ok().flatten()?;
    if anchor
        .get_attribute("target")
        .is_some_and(|target| !target.is_empty() && target != "_self")
        || anchor.has_attribute("download")
    {
        return None;
    }
    normalize_internal_href(&anchor.get_attribute("href")?, base_path)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn normalize_internal_href(href: &str, base_path: &str) -> Option<String> {
    let trimmed = href.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("mailto:")
        || trimmed.starts_with("javascript:")
        || trimmed.starts_with("//")
    {
        return None;
    }
    let path_query_hash = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        let origin = web_sys::window()?.location().origin().ok()?;
        trimmed.strip_prefix(&origin)?.to_owned()
    } else if trimmed.starts_with('/') {
        trimmed.to_owned()
    } else {
        return None;
    };
    let (path, _, _) = split_href(&path_query_hash);
    if has_app_asset_extension(path) {
        return None;
    }
    let logical = strip_base_path_for_client(path, base_path)?;
    if is_app_route_path_for_client(&logical) {
        Some(path_query_hash)
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn has_app_asset_extension(path: &str) -> bool {
    path.rsplit_once('/')
        .map(|(_, name)| name.contains('.'))
        .unwrap_or(false)
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

#[allow(clippy::too_many_arguments)]
#[requires(true)]
#[ensures(true)]
fn navigate_to_internal_href(
    href: &str,
    base_path: &str,
    push_history: bool,
    route: Signal<AppRoute>,
    cukta_state: Signal<CuktaWebState>,
    vlacku_draft_state: Signal<VlackuWebState>,
    vlacku_committed_state: Signal<VlackuWebState>,
    input_text: Signal<String>,
    parsed_text: Signal<String>,
    dialect: Signal<String>,
    parsed_dialect: Signal<String>,
    view_mode: Signal<GentufaWebViewMode>,
    gentufa_display: Signal<GentufaDisplayState>,
    gentufa_text_explicit: Signal<bool>,
    pending_cukta_scroll: Signal<Option<String>>,
) {
    let (path, query, hash) = split_href(href);
    let Some(logical_path) = strip_base_path_for_client(path, base_path) else {
        return;
    };
    let web_route = parse_web_route(&logical_path, query);
    let mut target = web_route_url(base_path, &web_route);
    if let Some(hash) = hash.filter(|hash| !hash.is_empty()) {
        target.push('#');
        target.push_str(hash);
    }
    save_current_scroll_position();
    set_browser_url(&target, push_history);
    apply_web_route_to_client_state(
        &web_route,
        query_has_key(query, "text"),
        route,
        cukta_state,
        vlacku_draft_state,
        vlacku_committed_state,
        input_text,
        parsed_text,
        dialect,
        parsed_dialect,
        view_mode,
        gentufa_display,
        gentufa_text_explicit,
    );
    if app_route_for_web_route(&web_route) == AppRoute::Cukta && hash.is_some() {
        let mut pending_scroll = pending_cukta_scroll;
        pending_scroll.set(Some(target));
    } else {
        restore_scroll_for_url(&target);
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(true)]
#[ensures(true)]
fn apply_web_route_to_client_state(
    web_route: &WebRoute,
    gentufa_text_is_explicit: bool,
    mut route: Signal<AppRoute>,
    mut cukta_state: Signal<CuktaWebState>,
    mut vlacku_draft_state: Signal<VlackuWebState>,
    mut vlacku_committed_state: Signal<VlackuWebState>,
    mut input_text: Signal<String>,
    mut parsed_text: Signal<String>,
    mut dialect: Signal<String>,
    mut parsed_dialect: Signal<String>,
    mut view_mode: Signal<GentufaWebViewMode>,
    mut gentufa_display: Signal<GentufaDisplayState>,
    mut gentufa_text_explicit: Signal<bool>,
) {
    route.set(app_route_for_web_route(web_route));
    match web_route {
        WebRoute::Gentufa(state) => {
            let input = state.text.clone();
            let parsed = if state.text.is_empty() && !gentufa_text_is_explicit {
                DEFAULT_GENTUFA_TEXT.to_owned()
            } else {
                state.text.clone()
            };
            let dialect_text = state.dialect.clone().unwrap_or_default();
            input_text.set(input);
            parsed_text.set(parsed);
            dialect.set(dialect_text.clone());
            parsed_dialect.set(dialect_text);
            view_mode.set(state.view_mode);
            gentufa_display.set(GentufaDisplayState {
                show_elided: state.show_elided,
                show_glosses: state.show_glosses,
            });
            gentufa_text_explicit.set(gentufa_text_is_explicit);
        }
        WebRoute::Cukta(state) => {
            cukta_state.set(state.clone());
        }
        WebRoute::Vlacku(state) => {
            clear_vlacku_search_timer();
            vlacku_draft_state.set(state.clone());
            vlacku_committed_state.set(state.clone());
        }
        WebRoute::Settings => {}
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
fn set_browser_url(url: &str, push_history: bool) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let location = window.location();
    let current = format!(
        "{}{}{}",
        location.pathname().unwrap_or_default(),
        location.search().unwrap_or_default(),
        location.hash().unwrap_or_default()
    );
    if current == url {
        return;
    }
    if let Ok(history) = window.history() {
        let method_result = if push_history {
            history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(url))
        } else {
            history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(url))
        };
        let _ = method_result;
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn set_browser_url(url: &str, push_history: bool) {
    let _ = (url, push_history);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn current_hash() -> Option<String> {
    web_sys::window()
        .and_then(|window| window.location().hash().ok())
        .filter(|hash| !hash.is_empty())
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
    let y = window.scroll_y().unwrap_or(0.0);
    session_storage_set(&key, &format!("{y:.0}"));
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
#[ensures(true)]
fn restore_scroll_for_url(url: &str) {
    let key = scroll_storage_key(url);
    let Some(raw) = session_storage_get(&key) else {
        web_sys::window().map(|window| window.scroll_to_with_x_and_y(0.0, 0.0));
        return;
    };
    let Ok(y) = raw.parse::<f64>() else {
        return;
    };
    let Some(window) = web_sys::window() else {
        return;
    };
    let closure = Closure::once(move || {
        if let Some(window) = web_sys::window() {
            window.scroll_to_with_x_and_y(0.0, y);
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
fn restore_scroll_for_url(url: &str) {
    let _ = url;
}

#[requires(true)]
#[ensures(true)]
fn set_vlacku_state_immediate(
    draft_state: &mut Signal<VlackuWebState>,
    committed_state: &mut Signal<VlackuWebState>,
    state: VlackuWebState,
) {
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
fn schedule_vlacku_url_push(base_path: &str, state: &VlackuWebState) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let target = vlacku_web_url(base_path, state);
    VLACKU_URL_TIMER.with(|timer| {
        if let Some(handle) = timer.replace(None) {
            window.clear_timeout_with_handle(handle);
        }
    });
    let closure = Closure::once(move || {
        if let Some(window) = web_sys::window() {
            let location = window.location();
            let current_url = format!(
                "{}{}",
                location.pathname().unwrap_or_default(),
                location.search().unwrap_or_default()
            );
            if current_url == target {
                return;
            }
            if let Ok(history) = window.history() {
                let _ =
                    history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&target));
            }
        }
    });
    if let Ok(handle) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        VLACKU_URL_DEBOUNCE_MS,
    ) {
        VLACKU_URL_TIMER.with(|timer| timer.set(Some(handle)));
        closure.forget();
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn schedule_vlacku_url_push(base_path: &str, state: &VlackuWebState) {
    let _ = VLACKU_URL_DEBOUNCE_MS;
    let _ = (base_path, state);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_url_replace(base_path: &str, state: &GentufaWebState) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let target = gentufa_web_url(base_path, state);
    GENTUFA_URL_TIMER.with(|timer| {
        if let Some(handle) = timer.replace(None) {
            window.clear_timeout_with_handle(handle);
        }
    });
    let closure = Closure::once(move || {
        if route_from_current_path() != AppRoute::Gentufa {
            return;
        }
        set_browser_url(&target, false);
    });
    if let Ok(handle) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        GENTUFA_URL_DEBOUNCE_MS,
    ) {
        GENTUFA_URL_TIMER.with(|timer| timer.set(Some(handle)));
        closure.forget();
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn schedule_gentufa_url_replace(base_path: &str, state: &GentufaWebState) {
    let _ = GENTUFA_URL_DEBOUNCE_MS;
    let _ = (base_path, state);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn push_cukta_url(base_path: &str, state: &CuktaWebState) {
    let target = cukta_web_url(base_path, state);
    if let Some(window) = web_sys::window() {
        let location = window.location();
        let current_url = format!(
            "{}{}",
            location.pathname().unwrap_or_default(),
            location.search().unwrap_or_default()
        );
        if current_url == target {
            return;
        }
        if let Ok(history) = window.history() {
            let _ = history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&target));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn push_cukta_url(base_path: &str, state: &CuktaWebState) {
    let _ = (base_path, state);
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
    document.set_title(&meta.title);
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
    let canonical_url = absolute_href_for_client(&meta.canonical_url);
    let manifest_href = absolute_href_for_client(&format!("{MANIFEST}"));
    let icon_href = absolute_href_for_client(&format!("{FAVICON}"));
    append_meta_name(&document, &head, "application-name", "jbotci");
    append_meta_name(&document, &head, "apple-mobile-web-app-capable", "yes");
    append_meta_name(&document, &head, "apple-mobile-web-app-title", "jbotci");
    append_meta_name(&document, &head, "mobile-web-app-capable", "yes");
    append_meta_name_with_extra(
        &document,
        &head,
        "theme-color",
        "#f6f1e8",
        &[("media", "(prefers-color-scheme: light)")],
    );
    append_meta_name_with_extra(
        &document,
        &head,
        "theme-color",
        "#090705",
        &[("media", "(prefers-color-scheme: dark)")],
    );
    append_link(&document, &head, "manifest", &manifest_href);
    append_link(&document, &head, "icon", &icon_href);
    append_link(&document, &head, "shortcut icon", &icon_href);
    append_link(&document, &head, "apple-touch-icon", &icon_href);
    append_meta_name(&document, &head, "description", &meta.description);
    append_link(&document, &head, "canonical", &canonical_url);
    append_meta_property(&document, &head, "og:title", &meta.title);
    append_meta_property(&document, &head, "og:description", &meta.description);
    append_meta_property(&document, &head, "og:type", "website");
    append_meta_property(&document, &head, "og:url", &canonical_url);
    append_meta_name(&document, &head, "twitter:title", &meta.title);
    append_meta_name(&document, &head, "twitter:description", &meta.description);
    if let Some(image) = &meta.image {
        let image_url = absolute_href_for_client(&image.href);
        append_meta_name(&document, &head, "twitter:card", "summary_large_image");
        append_meta_property(&document, &head, "og:image", &image_url);
        append_meta_name(&document, &head, "twitter:image", &image_url);
        append_meta_property(&document, &head, "og:image:width", &image.width.to_string());
        append_meta_property(
            &document,
            &head,
            "og:image:height",
            &image.height.to_string(),
        );
    } else {
        append_meta_name(&document, &head, "twitter:card", "summary");
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

#[cfg(not(target_arch = "wasm32"))]
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
    let viewport_height = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(720.0);
    let top = form_bottom.unwrap_or(topbar_bottom).max(topbar_bottom) + 12.0;
    let bottom = 12.0;
    let height = (viewport_height - top - bottom).max(280.0) * VLACKU_JVOZBA_HEIGHT_SCALE;
    let style = pane.style();
    let _ = style.set_property("--jvozba-pane-top", &format!("{top}px"));
    let _ = style.set_property("--jvozba-pane-bottom", &format!("{bottom}px"));
    let _ = style.set_property("--jvozba-pane-height", &format!("{height}px"));
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn sync_vlacku_jvozba_pane_metrics() {}

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
#[ensures(true)]
fn current_query_value(key: &str) -> Option<String> {
    current_query()
        .trim_start_matches('?')
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(candidate_key, value)| {
            if candidate_key == key {
                Some(value.to_owned())
            } else {
                None
            }
        })
}

#[requires(!key.is_empty())]
#[ensures(true)]
fn current_query_has_key(key: &str) -> bool {
    current_query()
        .trim_start_matches('?')
        .split('&')
        .filter(|pair| !pair.is_empty())
        .any(|pair| {
            pair.split_once('=')
                .map_or(pair == key, |(candidate, _)| candidate == key)
        })
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
fn load_settings() -> UserSettings {
    let mut settings = UserSettings::default();
    if let Some(theme) = storage_get("jbotci.theme").and_then(|value| parse_theme(&value)) {
        settings.theme = theme;
    }
    if let Some(script) = storage_get("jbotci.script").and_then(|value| parse_script(&value)) {
        settings.script = script;
    }
    settings
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
fn save_settings(settings: &UserSettings) {
    storage_set("jbotci.theme", theme_class(settings.theme));
    storage_set("jbotci.script", script_class(settings.script));
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
    let _ = key;
    None
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
    let _ = (key, value);
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
    let _ = key;
    None
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
    let _ = (key, value);
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
    fn vlacku_search_debounce_is_longer_than_url_debounce() {
        assert_eq!(VLACKU_SEARCH_DEBOUNCE_MS, 900);
        assert!(VLACKU_SEARCH_DEBOUNCE_MS > VLACKU_URL_DEBOUNCE_MS);
        assert!(GENTUFA_URL_DEBOUNCE_MS > VLACKU_URL_DEBOUNCE_MS);
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
    fn top_level_routes_accept_trailing_slashes() {
        assert_eq!(route_from_path("/jbotci/cukta/"), AppRoute::Cukta);
        assert_eq!(route_from_path("/jbotci/vlacku/"), AppRoute::Vlacku);
        assert_eq!(route_from_path("/jbotci/settings/"), AppRoute::Settings);
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
        }
    }
}
