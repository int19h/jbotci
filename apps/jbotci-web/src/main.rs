use dioxus::prelude::*;
use jbotci_cll::{
    CllBlock, CllEbnfEntry, CllEbnfToken, CllInline, CllInterlinearRow, CllLanguageSpanKind,
    CllLinkKind, CllLojbanizationLine, CllLujvoPart, CllSimpleListOrientation, cll_link_href,
    embedded_cll_site, wrap_ebnf_choice_lines,
};
use jbotci_output::{GlideMark, PhonemeRenderOptions, StressMark};
use jbotci_web_core::{
    CUKTA_WEB_DEFAULT_COUNT, CUKTA_WEB_MAX_COUNT, CuktaModeOption, CuktaPageData, CuktaPageKind,
    CuktaSearchResultCard, CuktaTargetOption, CuktaTocNode, CuktaWebMode, CuktaWebSearchState,
    CuktaWebState, CuktaWebView, GentufaBlock, GentufaCell, GentufaError, GentufaScript,
    GentufaSuccess, GentufaTreeRow, GentufaWebOptions, GentufaWebRequest, GentufaWebResult,
    GentufaWebViewMode, ReferenceLabel, ReferenceMarker, ReferenceMarkerRole, ReferenceSlotLabel,
    VLACKU_WEB_DEFAULT_COUNT, VLACKU_WEB_MAX_COUNT, VlackuCompositionPiece,
    VlackuCompositionPieceKind, VlackuDictionaryInfo, VlackuInline, VlackuInlineData,
    VlackuJvozbaItem, VlackuJvozbaItemKind, VlackuJvozbaMode, VlackuJvozbaOutput,
    VlackuJvozbaSegmentTone, VlackuMath, VlackuMathPart, VlackuMathPartData, VlackuVoteDisplay,
    VlackuWebCard, VlackuWebMode, VlackuWebResult, VlackuWebState, VlackuWordTypeOption,
    VlackuWordTypeSection, WebFeatureAvailability, build_cukta_web_page,
    build_vlacku_jvozba_output, build_vlacku_web_result, cukta_web_url, parse_cukta_web_route,
    parse_gentufa_for_web, parse_vlacku_web_route, toggle_cukta_target_selection,
    toggle_vlacku_word_type_selection, vlacku_brivla_filter_indeterminate, vlacku_web_url,
    vlacku_word_type_options,
};

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;

#[cfg(target_arch = "wasm32")]
use std::cell::Cell;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const LOGO: Asset = asset!("/assets/icons/jbotci-dark.svg");
const FAVICON: Asset = asset!("/assets/icons/jbotci-icon-192.png");
const NOTO_SANS: Asset = asset!("/assets/fonts/noto-sans-variable.ttf");
const NOTO_SANS_ITALIC: Asset = asset!("/assets/fonts/noto-sans-italic-variable.ttf");
const NOTO_SANS_MATH: Asset = asset!("/assets/fonts/noto-sans-math-regular.otf");
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

#[cfg(target_arch = "wasm32")]
thread_local! {
    static VLACKU_URL_TIMER: Cell<Option<i32>> = const { Cell::new(None) };
    static VLACKU_SEARCH_TIMER: Cell<Option<i32>> = const { Cell::new(None) };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum ThemeMode {
    Auto,
    Day,
    Night,
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
    show_elided: bool,
    show_glosses: bool,
    stress: StressMark,
    glides: GlideMark,
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

impl Default for UserSettings {
    #[requires(true)]
    #[ensures(ret.theme == ThemeMode::Auto)]
    fn default() -> Self {
        Self {
            theme: ThemeMode::Auto,
            script: GentufaScript::Latin,
            show_elided: false,
            show_glosses: true,
            stress: StressMark::Acute,
            glides: GlideMark::Breve,
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

#[requires(true)]
#[ensures(true)]
fn main() {
    dioxus::launch(App);
}

#[requires(true)]
#[ensures(ret.contains("Noto Sans Math"))]
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
  font-family: "Noto Sans Math";
  src: url("{noto_sans_math}") format("opentype");
  font-weight: 400;
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
        noto_sans_math = NOTO_SANS_MATH,
        crisa = CRISA,
    )
}

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
fn App() -> Element {
    let route = route_from_current_path();
    let base_path = base_path_from_current_path();
    let settings = use_signal(load_settings);
    let view_mode = use_signal(initial_view_mode);
    let initial_cukta = initial_cukta_state();
    let cukta_state = use_signal(|| initial_cukta);
    let cukta_toc_filter = use_signal(String::new);
    let cukta_toc_pinned = use_signal(|| true);
    let cukta_toc_toggles = use_signal(Vec::<String>::new);
    let cukta_toc_width = use_signal(default_cukta_toc_width);
    let cukta_toc_resize = use_signal(|| None::<CuktaTocResizeState>);
    let initial_vlacku = initial_vlacku_state();
    let vlacku_draft_state = use_signal(|| initial_vlacku.clone());
    let vlacku_committed_state = use_signal(|| initial_vlacku);
    let vlacku_result = use_memo(move || {
        let state = vlacku_committed_state.read().clone();
        build_vlacku_web_result(&state)
    });
    let cukta_page_base_path = base_path.clone();
    let cukta_page = use_memo(move || {
        let state = cukta_state.read().clone();
        build_cukta_web_page(&cukta_page_base_path, &state)
    });
    let jvozba_pane = use_signal(load_vlacku_jvozba_pane_state);
    let jvozba_drag = use_signal(|| None::<VlackuJvozbaDragState>);
    let mut input_text = use_signal(|| DEFAULT_GENTUFA_TEXT.to_owned());
    let mut parsed_text = use_signal(|| DEFAULT_GENTUFA_TEXT.to_owned());
    let dialect = use_signal(String::new);
    let mut parsed_dialect = use_signal(String::new);
    let reference_hover = use_signal(ReferenceHoverState::default);

    let settings_value = *settings.read();
    let view_mode_value = *view_mode.read();
    let request = GentufaWebRequest {
        text: parsed_text.read().clone(),
        options: web_options(
            settings_value,
            view_mode_value,
            parsed_dialect.read().clone(),
        ),
    };
    let result = parse_gentufa_for_web(&request);
    let vlacku_url_base_path = base_path.clone();
    use_effect(move || {
        if route == AppRoute::Vlacku {
            let state = vlacku_committed_state.read().clone();
            schedule_vlacku_url_push(&vlacku_url_base_path, &state);
        }
    });
    let cukta_url_base_path = base_path.clone();
    use_effect(move || {
        if route == AppRoute::Cukta {
            let state = cukta_state.read().clone();
            push_cukta_url(&cukta_url_base_path, &state);
        }
    });
    use_effect(move || {
        if route == AppRoute::Vlacku {
            let state = vlacku_draft_state.read().clone();
            set_brivla_toggle_indeterminate(vlacku_brivla_filter_indeterminate(&state.word_types));
            sync_vlacku_jvozba_pane_metrics();
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
        document::Link { rel: "icon", r#type: "image/png", href: FAVICON }
        document::Link { rel: "shortcut icon", r#type: "image/png", href: FAVICON }
        div { class: "{app_class}",
            { render_topbar(route, &base_path, settings, settings_value) }
            main { class: "spa-main",
                div { class: "spa-stack",
                    {
                        match route {
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
                                                    oninput: move |event| input_text.set(event.value()),
                                                }
                                                div { class: "form-actions",
                                                    { render_dialect_control(dialect) }
                                                    button {
                                                        class: "btn-parse",
                                                        r#type: "button",
                                                        onclick: move |_| {
                                                            let next_text = input_text.read().clone();
                                                            let next_dialect = dialect.read().clone();
                                                            parsed_text.set(next_text);
                                                            parsed_dialect.set(next_dialect);
                                                        },
                                                        "Parse"
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "gentufa-result-stack",
                                            { render_result(&result, view_mode, view_mode_value, settings, settings_value, reference_hover) }
                                        }
                                    }
                                }
                            },
                            AppRoute::Settings => render_settings(settings, settings_value),
                            AppRoute::Cukta => {
                                render_cukta_page(
                                    cukta_state,
                                    cukta_page,
                                    cukta_toc_filter,
                                    cukta_toc_pinned,
                                    cukta_toc_toggles,
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
    base_path: &str,
    settings: Signal<UserSettings>,
    current: UserSettings,
) -> Element {
    rsx! {
        header { class: "app-topbar spa-topbar",
            div { class: "app-topbar-inner spa-topbar-inner",
                div { class: "app-topbar-left",
                    a {
                        class: "app-topbar-brand",
                        href: nav_href(base_path, AppRoute::Settings),
                        aria_label: "Settings",
                        title: "Settings",
                        img { class: "app-topbar-brand-logo", src: LOGO, alt: "jbotci" }
                    }
                    span { class: "app-topbar-theme app-topbar-theme-mode",
                        { render_theme_switch(settings, current.theme) }
                    }
                    span { class: "app-topbar-theme app-topbar-orthography",
                        { render_script_switch(settings, current.script) }
                    }
                    nav { class: "spa-nav", aria_label: "Primary navigation",
                        a {
                            class: topbar_link_class(route == AppRoute::Cukta),
                            href: nav_href(base_path, AppRoute::Cukta),
                            aria_current: if route == AppRoute::Cukta { "page" } else { "false" },
                            span { class: "app-topbar-link-label", "cukta" }
                        }
                        a {
                            class: topbar_link_class(route == AppRoute::Vlacku),
                            href: nav_href(base_path, AppRoute::Vlacku),
                            aria_current: if route == AppRoute::Vlacku { "page" } else { "false" },
                            span { class: "app-topbar-link-label", "vlacku" }
                        }
                        a {
                            class: topbar_link_class(route == AppRoute::Gentufa),
                            href: nav_href(base_path, AppRoute::Gentufa),
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
                div { class: "app-topbar-center app-topbar-activity", role: "status", aria_live: "polite" }
                div { class: "app-topbar-right",
                    a {
                        class: topbar_link_class(route == AppRoute::Settings),
                        href: nav_href(base_path, AppRoute::Settings),
                        title: "Settings",
                        span { class: "app-topbar-link-label", "settings" }
                    }
                }
            }
        }
    }
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

#[requires(true)]
#[ensures(true)]
fn render_cukta_page(
    cukta_state: Signal<CuktaWebState>,
    cukta_page: Memo<CuktaPageData>,
    mut toc_filter: Signal<String>,
    mut toc_pinned: Signal<bool>,
    toc_toggles: Signal<Vec<String>>,
    toc_width: Signal<f64>,
    mut toc_resize: Signal<Option<CuktaTocResizeState>>,
    base_path: &str,
) -> Element {
    let page = cukta_page.read();
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
                        onclick: move |_| toc_pinned.set(!toc_is_pinned),
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
                        nav { class: "cll-toc-scroll", aria_label: "CLL table of contents",
                            ol { class: "cll-toc-tree",
                                for node in page.toc.iter() {
                                    { render_cukta_toc_node(cukta_state, toc_toggles, node, &toc_filter.read()) }
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
#[ensures(true)]
fn set_cukta_toc_width(width: &mut Signal<f64>, next_width: f64) {
    width.set(clamp_cukta_toc_width(next_width));
}

#[requires(true)]
#[ensures(true)]
fn render_cukta_toc_node(
    cukta_state: Signal<CuktaWebState>,
    toc_toggles: Signal<Vec<String>>,
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
    let expanded = toc_node_expanded(node, &filter, &toc_toggles.read());
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
        li { class: "{class}",
            div { class: "cll-toc-row",
                if !node.children.is_empty() {
                    button {
                        class: "cll-toc-toggle",
                        r#type: "button",
                        aria_expanded: if expanded { "true" } else { "false" },
                        title: if expanded { "Collapse" } else { "Expand" },
                        onclick: {
                            let node_id = node.node_id.clone();
                            move |_| toggle_cukta_toc_node(&mut toc_toggles.clone(), &node_id)
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
                        { render_cukta_toc_node(cukta_state, toc_toggles, child, &filter) }
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
fn toc_node_expanded(node: &CuktaTocNode, filter: &str, toggles: &[String]) -> bool {
    if !filter.trim().is_empty() {
        return true;
    }
    let default_expanded = node.active;
    if toggles.iter().any(|id| id == &node.node_id) {
        !default_expanded
    } else {
        default_expanded
    }
}

#[requires(!node_id.is_empty())]
#[ensures(true)]
fn toggle_cukta_toc_node(toc_toggles: &mut Signal<Vec<String>>, node_id: &str) {
    let mut toggles = toc_toggles.read().clone();
    if let Some(index) = toggles.iter().position(|id| id == node_id) {
        toggles.remove(index);
    } else {
        toggles.push(node_id.to_owned());
    }
    toc_toggles.set(toggles);
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
                    placeholder: if state.mode == CuktaWebMode::Word { "valsi" } else { "meaning search disabled" },
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
            title: if option_disabled { "Meaning search will be enabled when vector search is ported" } else { "Find CLL passages containing this word" },
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
            rsx! {
                a {
                    class: "{class_name}",
                    href: "{href}",
                    onclick: move |event| {
                        if is_cukta_link {
                            if let Some(reference) = cukta_section_reference_from_href(&href) {
                                event.prevent_default();
                                set_cukta_state(&mut cukta_state.clone(), CuktaWebState {
                                    view: CuktaWebView::Section { reference },
                                });
                                scroll_to_cukta_href(&href);
                            }
                        }
                    },
                    for child in inlines.iter() {
                        { render_cll_inline(cukta_state, child, base_path) }
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
        let href = cll_ebnf_href(base_path, href);
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
        let href = cll_ebnf_href(base_path, href);
        rsx! {
            a { class: "{class_name}", href: "{href}", "{body}" }
        }
    } else {
        rsx! { span { class: "{class_name}", "{body}" } }
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
        CllLinkKind::Rafsi => format!("{prefix}/vlacku?mode=rafsi&q={target}"),
        CllLinkKind::Parse => format!("{prefix}/gentufa?text={target}&dialect=allow-cgv"),
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
    vlacku_result: Memo<VlackuWebResult>,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_drag: Signal<Option<VlackuJvozbaDragState>>,
    base_path: &str,
) -> Element {
    let result = vlacku_result.read();
    let draft_state = vlacku_draft_state.read().clone();
    let word_type_options = vlacku_word_type_options(&draft_state.word_types);
    let shell_class = if jvozba_pane.read().open {
        "dictionary-shell dictionary-jvozba-hints-active"
    } else {
        "dictionary-shell"
    };
    rsx! {
        section { class: "spa-page dictionary-page vlacku-page",
            h1 { class: "sr-only", "jbotci vlacku" }
            div { class: "{shell_class}",
                div { class: "dictionary-layout",
                    div { class: "dictionary-main-column",
                        { render_vlacku_controls(vlacku_draft_state, vlacku_committed_state, &draft_state, &word_type_options) }
                        { render_vlacku_body(&result, vlacku_draft_state, vlacku_committed_state, jvozba_pane, base_path) }
                    }
                    { render_vlacku_jvozba_pane(jvozba_pane, jvozba_drag) }
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
                                { render_vlacku_mode_button(vlacku_draft_state, vlacku_committed_state, state.mode, VlackuWebMode::Meaning, "meaning", true) }
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
    base_path: &str,
) -> Element {
    rsx! {
        div { class: "dictionary-results",
            for error in result.errors.iter() {
                div { class: "spa-error dictionary-error", "{error}" }
            }
            if let Some(message) = &result.message {
                p { class: "dictionary-empty", "{message}" }
            }
            if let Some(info) = &result.dictionary_info {
                { render_dictionary_info(info) }
            }
            if !result.cards.is_empty() {
                div { class: "dictionary-results-grid",
                    for card in result.cards.iter() {
                        { render_vlacku_card(card, jvozba_pane, base_path) }
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
        div { class: "dictionary-info",
            div { class: "dictionary-info-grid",
                div { class: "dictionary-info-metric",
                    span { class: "dictionary-info-metric-value", "{info.entry_count}" }
                    span { class: "dictionary-info-metric-label", "entries" }
                }
                div { class: "dictionary-info-metric",
                    span { class: "dictionary-info-metric-value", "{info.rafsi_count}" }
                    span { class: "dictionary-info-metric-label", "rafsi" }
                }
                for word_type in info.word_type_counts.iter() {
                    div { class: "dictionary-info-metric",
                        span { class: "dictionary-info-metric-value", "{word_type.count}" }
                        span { class: "dictionary-info-metric-label", "{word_type.label}" }
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
    base_path: &str,
) -> Element {
    rsx! {
        section { class: "result-card",
            header { class: "result-header",
                h2 { class: "word",
                    span { class: "dictionary-word-line",
                        { render_vlacku_headword_line(card, jvozba_pane, base_path) }
                    }
                }
                div { class: "tag-row",
                    { render_vlacku_metadata_pill(card, base_path) }
                }
            }
            if !card.definition.is_empty() {
                p { class: "dictionary-definition-copy",
                    { render_inline_spans(&card.definition, jvozba_pane, base_path) }
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
                    { render_inline_spans(&card.notes, jvozba_pane, base_path) }
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
        { render_vlacku_word_action(
            jvozba_pane,
            card.can_add_to_jvozba,
            &card.word,
            &card.display_word,
            &word_href,
            "dictionary-headword-link dictionary-jvozba-highlighted-word",
        ) }
        if let Some(ipa) = &card.ipa {
            span { class: "dictionary-headword-ipa", "/{ipa}/" }
        }
        if !card.decomposition.is_empty() {
            { render_vlacku_inline_separator("=") }
            { render_vlacku_decomposition_inline(card, jvozba_pane, base_path) }
        } else if !card.rafsi.is_empty() {
            { render_vlacku_inline_separator("≘") }
            span { class: "dictionary-inline-pill-row",
                for rafsi in card.rafsi.iter() {
                    { render_rafsi_pill(jvozba_pane, &card.word, rafsi) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_decomposition_inline(
    card: &VlackuWebCard,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
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
            { render_composition_piece(piece, jvozba_pane, base_path) }
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
    can_add_to_jvozba: bool,
    word: &str,
    display_word: &str,
    href: &str,
    class_name: &str,
) -> Element {
    let pane_open = jvozba_pane.read().open;
    let word_value = word.to_owned();
    let static_class_name = class_name
        .split_whitespace()
        .filter(|class| {
            *class != "dictionary-jvozba-add-link-hint"
                && *class != "dictionary-jvozba-highlighted-word"
        })
        .collect::<Vec<_>>()
        .join(" ");
    if pane_open && can_add_to_jvozba {
        rsx! {
            button {
                class: "{class_name}",
                r#type: "button",
                title: "Add to jvozba",
                onclick: move |_| add_vlacku_jvozba_word(&mut jvozba_pane, word_value.clone()),
                "{display_word}"
            }
        }
    } else if pane_open {
        rsx! {
            span { class: "{static_class_name}", "{display_word}" }
        }
    } else {
        rsx! {
            a { class: "{class_name}", href: "{href}", "{display_word}" }
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
                        { render_vlacku_rafsi_add_piece(jvozba_pane, &piece.surface, source, &piece.display_surface) }
                        span { class: "rafsi-split-right",
                            { render_vlacku_word_action(
                                jvozba_pane,
                                true,
                                source,
                                piece.display_source.as_deref().unwrap_or(source),
                                &href,
                                "dictionary-word-link rafsi-source-link dictionary-jvozba-add-link-hint",
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
    rafsi: &str,
    source_word: &str,
    display_rafsi: &str,
) -> Element {
    let pane_open = jvozba_pane.read().open;
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
    source_word: &str,
    rafsi: &str,
) -> Element {
    let pane_open = jvozba_pane.read().open;
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
    base_path: &str,
) -> Element {
    rsx! {
        for span in spans.iter() {
            {
                match span.as_data() {
                    data!(VlackuInline::Text(text)) => rsx! { "{text}" },
                    data!(VlackuInline::Math(math)) => render_vlacku_math(math),
                    data!(VlackuInline::WordRef { label, href, can_add_to_jvozba }) => {
                        render_vlacku_inline_word_ref(jvozba_pane, *can_add_to_jvozba, label, href, base_path)
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
    can_add_to_jvozba: bool,
    label: &str,
    href: &str,
    base_path: &str,
) -> Element {
    let pane_open = jvozba_pane.read().open;
    let word_value = label.to_owned();
    let resolved_href = resolved_href_with_base_path(base_path, href);
    if pane_open && can_add_to_jvozba {
        rsx! {
            button {
                class: "dictionary-word-link dictionary-jvozba-add-link-hint",
                r#type: "button",
                title: "Add to jvozba",
                onclick: move |_| add_vlacku_jvozba_word(&mut jvozba_pane, word_value.clone()),
                "{label}"
            }
        }
    } else if pane_open {
        rsx! {
            span { class: "dictionary-word-link", "{label}" }
        }
    } else {
        rsx! {
            a { class: "dictionary-word-link dictionary-jvozba-add-link-hint", href: "{resolved_href}", "{label}" }
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
        "Meaning search will be enabled when vector search is ported"
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
        VlackuWebMode::Meaning => "meaning search disabled",
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
    settings: Signal<UserSettings>,
    settings_value: UserSettings,
    reference_hover: Signal<ReferenceHoverState>,
) -> Element {
    match result {
        GentufaWebResult::Blank => rsx! {},
        GentufaWebResult::Error(error) => render_error(error),
        GentufaWebResult::Success(success) => render_success(
            success,
            view_mode,
            view_mode_value,
            settings,
            settings_value,
            reference_hover,
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
    settings: Signal<UserSettings>,
    settings_value: UserSettings,
    reference_hover: Signal<ReferenceHoverState>,
) -> Element {
    let reference_hover_value = reference_hover.read().clone();
    rsx! {
        section { class: "result-section",
            { render_reference_overlay(&reference_hover_value) }
            { render_surface_output(success) }
            { render_diagnostics(success) }
            { render_view_tabs(view_mode, view_mode_value) }
            { render_output_controls(view_mode_value, settings, settings_value) }
            if view_mode_value == GentufaWebViewMode::Blocks {
                { render_blocks(success, settings_value.show_glosses, reference_hover) }
            } else {
                { render_tree(success, settings_value.show_glosses, false, reference_hover) }
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
                pre { class: "brackets-output ipa-output", "{success.ipa_text}" }
                pre { class: "brackets-output compact-output",
                    span { class: "brackets-output-markup", "{success.brackets_text}" }
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
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_output_controls(
    view_mode: GentufaWebViewMode,
    settings: Signal<UserSettings>,
    current: UserSettings,
) -> Element {
    match view_mode {
        GentufaWebViewMode::Blocks => rsx! {
            div { class: "controls blocks-controls",
                { render_gloss_checkbox(settings, current.show_glosses) }
                { render_elided_checkbox(settings, current.show_elided) }
            }
        },
        GentufaWebViewMode::Tree => rsx! {
            div { class: "controls table-controls",
                { render_gloss_checkbox(settings, current.show_glosses) }
                { render_static_checkbox("Show definitions", false, true) }
                { render_static_checkbox("Decompose known lujvo", false, true) }
                { render_elided_checkbox(settings, current.show_elided) }
            }
        },
    }
}

#[requires(!label.is_empty())]
#[ensures(true)]
fn render_static_checkbox(label: &'static str, checked: bool, disabled: bool) -> Element {
    rsx! {
        label {
            input {
                r#type: "checkbox",
                checked,
                disabled,
            }
            " {label}"
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_gloss_checkbox(mut settings: Signal<UserSettings>, checked: bool) -> Element {
    rsx! {
        label {
            input {
                r#type: "checkbox",
                checked,
                onchange: move |_| toggle_glosses(&mut settings),
            }
            " Show glosses"
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_elided_checkbox(mut settings: Signal<UserSettings>, checked: bool) -> Element {
    rsx! {
        label {
            input {
                r#type: "checkbox",
                checked,
                onchange: move |_| toggle_elided(&mut settings),
            }
            " Show elided terminators"
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_blocks(
    success: &GentufaSuccess,
    show_glosses: bool,
    reference_hover: Signal<ReferenceHoverState>,
) -> Element {
    let column_count = success.blocks_layout.max_col.max(1);
    let column_template = repeated_parse_tree_template(column_count);
    let row_count = success.blocks_layout.max_row + usize::from(show_glosses);
    let row_template = format!("repeat({}, auto)", row_count.max(1));
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
                            for block in success.blocks_layout.blocks.iter() {
                                { render_block(block, reference_hover, export_anchor_id) }
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
#[ensures(true)]
fn render_block(
    block: &GentufaBlock,
    reference_hover: Signal<ReferenceHoverState>,
    export_anchor_id: Option<&str>,
) -> Element {
    let row = block.row + 1;
    let col = block.col + 1;
    let classes = block_class(block);
    let style = format!(
        "grid-row: {row} / span {}; grid-column: {col} / span {}; --block-color: {}; background-color: {};",
        block.row_span, block.col_span, block.color, block.color
    );
    let hover_state = reference_hover.read().clone();
    let incoming_markers = block
        .ref_markers
        .iter()
        .filter(|marker| marker.role == ReferenceMarkerRole::Referent);
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
    let needs_incoming_overlap_sizer = incoming_count > 0 && block.row_span == 1;
    let outgoing_markers = block
        .ref_markers
        .iter()
        .filter(|marker| marker.role == ReferenceMarkerRole::Reference);
    let is_export_anchor = export_anchor_id == Some(block.block_id.as_str());
    rsx! {
        div {
            key: "{block.block_id}",
            class: "{classes}",
            style: "{style}",
            "data-block-id": "{block.block_id}",
            "data-col": "{block.col}",
            "data-colspan": "{block.col_span}",
            "data-color": "{block.color}",
            "data-token-kind": "{block.token_kind.clone().unwrap_or_default()}",
            "data-raw-text": "{block.raw_text}",
            "data-node-type": "{block.node_types.join(\" \")}",
            if block.ref_markers.iter().any(|marker| marker.role == ReferenceMarkerRole::Referent) {
                span { class: "{incoming_class}",
                    for marker in incoming_markers {
                        span { class: "ref-math ref-line",
                            { render_ref_marker(marker, reference_hover, &hover_state) }
                            span { class: "ref-arrow", "→" }
                        }
                    }
                }
            }
            if needs_incoming_overlap_sizer {
                span { class: "block-overlap-sizer", aria_hidden: "true",
                    for marker in block.ref_markers.iter().filter(|marker| marker.role == ReferenceMarkerRole::Referent) {
                        span { class: "block-overlap-line",
                            span { class: "block-overlap-ref ref-math",
                                { render_reference_label(&marker.label) }
                                span { class: "ref-arrow", "→" }
                            }
                            span { class: "block-overlap-primary", "{block.label}" }
                            span { class: "block-overlap-ref block-overlap-ref-mirror ref-math",
                                { render_reference_label(&marker.label) }
                                span { class: "ref-arrow", "→" }
                            }
                        }
                    }
                }
            }
            span { class: "block-label", title: "{block.label}",
                "{block.label}"
            }
            if block.ref_markers.iter().any(|marker| marker.role == ReferenceMarkerRole::Reference) {
                span { class: "block-ref-source",
                    span { class: "ref-math",
                        for marker in outgoing_markers {
                            span { class: "ref-arrow", "→" }
                            { render_ref_marker(marker, reference_hover, &hover_state) }
                        }
                    }
                }
            }
            if is_export_anchor {
                span { class: "blocks-svg-link",
                    span { class: "export-link is-disabled", "SVG" }
                    span { class: "export-link is-disabled", "PNG" }
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
fn render_tree(
    success: &GentufaSuccess,
    show_glosses: bool,
    show_definitions: bool,
    reference_hover: Signal<ReferenceHoverState>,
) -> Element {
    rsx! {
        div { class: "table-view",
            div { class: "table-wrap",
                table { class: "parse-table spa-gentufa-table",
                    thead {
                        tr {
                            th { class: "col-node", div { class: "cell-pad", "Node" } }
                            th { class: "col-valsis", div { class: "cell-pad", "Word" } }
                            th { class: "col-gloss", div { class: "cell-pad", "Glosses" } }
                            th { class: "col-definition", div { class: "cell-pad", "Definitions" } }
                        }
                    }
                    tbody {
                        for row in success.tree_rows.iter() {
                            { render_tree_row(row, show_glosses, show_definitions, reference_hover) }
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
    show_glosses: bool,
    show_definitions: bool,
    reference_hover: Signal<ReferenceHoverState>,
) -> Element {
    let row_class = if tree_row_is_elided(row) {
        "elided-row"
    } else {
        ""
    };
    let style = format!("--row-color: {}; --indent-count: {};", row.color, row.depth);
    let hover_state = reference_hover.read().clone();
    let incoming_markers = row
        .ref_markers
        .iter()
        .filter(|marker| marker.role == ReferenceMarkerRole::Referent);
    let outgoing_markers = row
        .ref_markers
        .iter()
        .filter(|marker| marker.role == ReferenceMarkerRole::Reference);
    rsx! {
        tr { class: "{row_class}", style: "{style}",
            td { class: "col-node",
                div { class: "node-cell",
                    span { class: "indent-stack",
                        for _ in 0..row.depth {
                            span { class: "indent-block line-top line-bottom" }
                        }
                    }
                    span { class: "node-content",
                        span { class: "node-toggle-spacer", aria_hidden: "true" }
                        span { class: "node-label",
                            for marker in incoming_markers {
                                { render_ref_marker(marker, reference_hover, &hover_state) }
                                span { class: "ref-arrow", "→" }
                            }
                            "{row.label}"
                            for marker in outgoing_markers {
                                span { class: "ref-arrow", "→" }
                                { render_ref_marker(marker, reference_hover, &hover_state) }
                            }
                        }
                    }
                }
            }
            td { class: "col-valsis",
                div { class: "cell-pad",
                    for cell in row.cells.iter() {
                        { render_tree_cell(cell) }
                    }
                }
            }
            td { class: "col-gloss",
                div { class: "cell-pad",
                    if show_glosses {
                        { render_tree_glosses(row) }
                    }
                }
            }
            td { class: "col-definition",
                div { class: "cell-pad",
                    if show_definitions {
                        if let Some(definition) = &row.definition {
                            span { class: "def-line", "{definition}" }
                        }
                    }
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
            span { class: "token-raw lojban-text", "{cell.text}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_tree_glosses(row: &GentufaTreeRow) -> Element {
    rsx! {
        if let Some(gloss) = &row.computed_gloss {
            span { class: "gloss-item", "{gloss}" }
        }
        for gloss in row.glosses.iter() {
            span { class: "gloss-item", "{gloss}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_reference_label(label: &ReferenceLabel) -> Element {
    let slot_text = label.slot.as_ref().map(ReferenceSlotLabel::text);
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
fn render_settings(settings: Signal<UserSettings>, current: UserSettings) -> Element {
    rsx! {
        section { class: "spa-page settings-page",
            div { class: "page-container settings-container",
                h1 { "Settings" }
                section { class: "settings-section",
                    h2 { "Theme" }
                    { render_theme_switch(settings, current.theme) }
                }
                section { class: "settings-section",
                    h2 { "Script" }
                    { render_script_switch(settings, current.script) }
                }
                section { class: "settings-section",
                    h2 { "Gentufa" }
                    { render_gloss_checkbox(settings, current.show_glosses) }
                    { render_elided_checkbox(settings, current.show_elided) }
                }
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
        show_elided: settings.show_elided,
        show_glosses: settings.show_glosses,
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
fn toggle_elided(settings: &mut Signal<UserSettings>) {
    let mut next = *settings.read();
    next.show_elided = !next.show_elided;
    settings.set(next);
    save_settings(&next);
}

#[requires(true)]
#[ensures(true)]
fn toggle_glosses(settings: &mut Signal<UserSettings>) {
    let mut next = *settings.read();
    next.show_glosses = !next.show_glosses;
    settings.set(next);
    save_settings(&next);
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
fn topbar_link_class(active: bool) -> &'static str {
    if active {
        "app-topbar-link active"
    } else {
        "app-topbar-link"
    }
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
fn initial_view_mode() -> GentufaWebViewMode {
    current_query_value("view")
        .as_deref()
        .and_then(parse_view_mode)
        .unwrap_or(GentufaWebViewMode::Blocks)
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
fn parse_view_mode(value: &str) -> Option<GentufaWebViewMode> {
    match value {
        "tree" | "table" => Some(GentufaWebViewMode::Tree),
        "blocks" => Some(GentufaWebViewMode::Blocks),
        _ => None,
    }
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
    let viewport_height = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(720.0);
    let top = topbar_bottom + 12.0;
    let bottom = 12.0;
    let height = (viewport_height - top - bottom).max(280.0);
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
    settings.show_elided = storage_get("jbotci.show_elided").as_deref() == Some("true");
    settings.show_glosses = storage_get("jbotci.show_glosses").as_deref() != Some("false");
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
    storage_set(
        "jbotci.show_elided",
        if settings.show_elided {
            "true"
        } else {
            "false"
        },
    );
    storage_set(
        "jbotci.show_glosses",
        if settings.show_glosses {
            "true"
        } else {
            "false"
        },
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
    fn vlacku_search_debounce_is_longer_than_url_debounce() {
        assert_eq!(VLACKU_SEARCH_DEBOUNCE_MS, 900);
        assert!(VLACKU_SEARCH_DEBOUNCE_MS > VLACKU_URL_DEBOUNCE_MS);
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
}
