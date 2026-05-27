use dioxus::prelude::*;
use jbotci_output::{GlideMark, PhonemeRenderOptions, StressMark};
use jbotci_web_core::{
    GentufaBlock, GentufaCell, GentufaError, GentufaScript, GentufaSuccess, GentufaTreeRow,
    GentufaWebOptions, GentufaWebRequest, GentufaWebResult, GentufaWebViewMode, ReferenceLabel,
    ReferenceMarker, ReferenceMarkerRole, ReferenceSlotLabel, VLACKU_WEB_DEFAULT_COUNT,
    VLACKU_WEB_MAX_COUNT, VlackuCompositionPiece, VlackuCompositionPieceKind, VlackuDictionaryInfo,
    VlackuInline, VlackuJvozbaItem, VlackuJvozbaItemKind, VlackuJvozbaMode, VlackuJvozbaOutput,
    VlackuJvozbaSegmentKind, VlackuVoteDisplay, VlackuWebCard, VlackuWebMode, VlackuWebState,
    VlackuWordTypeOption, VlackuWordTypeSection, WebFeatureAvailability,
    build_vlacku_jvozba_output, build_vlacku_web_result, parse_gentufa_for_web,
    parse_vlacku_web_route, vlacku_web_url,
};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

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
const DEFAULT_GENTUFA_TEXT: &str = "cadga fa lonu ro lo prenu goi ko'a cu troci lonu ko'a tarti loka ce'u xendo je cnikansa ro lo jmive kei ta'i lo racli";

#[cfg(target_arch = "wasm32")]
thread_local! {
    static VLACKU_URL_TIMER: Cell<Option<i32>> = const { Cell::new(None) };
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
    let vlacku_state = use_signal(initial_vlacku_state);
    let jvozba_pane = use_signal(load_vlacku_jvozba_pane_state);
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
            let state = vlacku_state.read().clone();
            schedule_vlacku_url_push(&vlacku_url_base_path, &state);
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
                            AppRoute::Cukta => render_disabled("cukta"),
                            AppRoute::Vlacku => {
                                render_vlacku_page(vlacku_state, jvozba_pane, &base_path)
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
                        span {
                            class: "app-topbar-link is-disabled",
                            aria_disabled: "true",
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
fn render_vlacku_page(
    vlacku_state: Signal<VlackuWebState>,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    base_path: &str,
) -> Element {
    let result = build_vlacku_web_result(&vlacku_state.read());
    rsx! {
        section { class: "spa-page vlacku-page",
            h1 { class: "sr-only", "jbotci vlacku" }
            div { class: "page-container vlacku-layout",
                div { class: "vlacku-main",
                    { render_vlacku_controls(vlacku_state, &result) }
                    { render_vlacku_body(&result, vlacku_state, jvozba_pane, base_path) }
                }
                { render_vlacku_jvozba_pane(jvozba_pane) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_controls(
    mut vlacku_state: Signal<VlackuWebState>,
    result: &jbotci_web_core::VlackuWebResult,
) -> Element {
    let state = result.state.clone();
    rsx! {
        div { class: "vlacku-controls",
            div { class: "vlacku-mode-row", role: "group", aria_label: "Dictionary search mode",
                { render_vlacku_mode_button(vlacku_state, state.mode, VlackuWebMode::Word, "word", false) }
                { render_vlacku_mode_button(vlacku_state, state.mode, VlackuWebMode::Rafsi, "rafsi", false) }
                { render_vlacku_mode_button(vlacku_state, state.mode, VlackuWebMode::Sound, "sound", false) }
                { render_vlacku_mode_button(vlacku_state, state.mode, VlackuWebMode::Meaning, "meaning", true) }
            }
            input {
                class: "vlacku-query-input",
                r#type: "search",
                aria_label: "Dictionary query",
                placeholder: vlacku_query_placeholder(state.mode),
                spellcheck: "false",
                value: "{state.query}",
                oninput: move |event| {
                    let mut next = vlacku_state.read().clone();
                    next.query = event.value();
                    next.count = VLACKU_WEB_DEFAULT_COUNT;
                    vlacku_state.set(next);
                },
            }
            div { class: "vlacku-filter-grid", aria_label: "Word type filters",
                for option in result.word_type_options.iter() {
                    { render_word_type_filter(vlacku_state, option) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_mode_button(
    mut vlacku_state: Signal<VlackuWebState>,
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
                    let mut next = vlacku_state.read().clone();
                    next.mode = mode;
                    next.count = VLACKU_WEB_DEFAULT_COUNT;
                    vlacku_state.set(next);
                }
            },
            "{label}"
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_word_type_filter(
    mut vlacku_state: Signal<VlackuWebState>,
    option: &VlackuWordTypeOption,
) -> Element {
    let value = option.value.clone();
    let is_parent = value == "brivla";
    rsx! {
        label {
            class: word_type_filter_class(option.section, is_parent),
            title: "{option.count} entries",
            input {
                r#type: "checkbox",
                checked: option.selected,
                onchange: move |_| toggle_vlacku_word_type(&mut vlacku_state, &value),
            }
            span { class: "vlacku-filter-label", "{option.label}" }
            span { class: "vlacku-filter-count", "{option.count}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_body(
    result: &jbotci_web_core::VlackuWebResult,
    mut vlacku_state: Signal<VlackuWebState>,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    base_path: &str,
) -> Element {
    rsx! {
        div { class: "vlacku-results",
            for error in result.errors.iter() {
                div { class: "error-box failure-errors", "{error}" }
            }
            if let Some(message) = &result.message {
                div { class: "vlacku-empty-message", "{message}" }
            }
            if let Some(info) = &result.dictionary_info {
                { render_dictionary_info(info) }
            }
            if !result.cards.is_empty() {
                div { class: "vlacku-card-grid",
                    for card in result.cards.iter() {
                        { render_vlacku_card(card, jvozba_pane, base_path) }
                    }
                }
            }
            if result.has_more {
                button {
                    class: "btn-parse vlacku-load-more",
                    r#type: "button",
                    onclick: move |_| {
                        let mut next = vlacku_state.read().clone();
                        next.count = next.count.saturating_mul(2).clamp(1, VLACKU_WEB_MAX_COUNT);
                        vlacku_state.set(next);
                    },
                    "Load more"
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_dictionary_info(info: &VlackuDictionaryInfo) -> Element {
    rsx! {
        div { class: "vlacku-dictionary-info",
            div { class: "vlacku-info-metric",
                span { class: "vlacku-info-value", "{info.entry_count}" }
                span { class: "vlacku-info-label", "entries" }
            }
            div { class: "vlacku-info-metric",
                span { class: "vlacku-info-value", "{info.rafsi_count}" }
                span { class: "vlacku-info-label", "rafsi" }
            }
            for word_type in info.word_type_counts.iter() {
                div { class: "vlacku-info-metric",
                    span { class: "vlacku-info-value", "{word_type.count}" }
                    span { class: "vlacku-info-label", "{word_type.label}" }
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
        article { class: "vlacku-card",
            header { class: "vlacku-card-header",
                span { class: "vlacku-card-rank", "{card.rank}." }
                { render_vlacku_word_action(jvozba_pane, card.can_add_to_jvozba, &card.word, &word_href) }
                span { class: "vlacku-card-type", "{card.word_type}" }
                if let Some(selmaho) = &card.selmaho {
                    span { class: "vlacku-card-selmaho", "{selmaho}" }
                }
                if let Some(similarity) = card.similarity {
                    span { class: "vlacku-card-meta", "similarity: {format_similarity(similarity)}" }
                }
                { render_vote_display(&card.votes) }
            }
            if let Some(ipa) = &card.ipa {
                div { class: "vlacku-ipa", "{ipa}" }
            }
            if !card.decomposition.is_empty() {
                div { class: "vlacku-decomposition",
                    for piece in card.decomposition.iter() {
                        { render_composition_piece(piece, jvozba_pane, base_path) }
                    }
                }
            }
            if !card.rafsi.is_empty() {
                div { class: "vlacku-rafsi-row",
                    span { class: "vlacku-detail-label", "rafsi" }
                    for rafsi in card.rafsi.iter() {
                        { render_rafsi_pill(jvozba_pane, rafsi) }
                    }
                }
            }
            if !card.glosses.is_empty() {
                div { class: "vlacku-gloss-row",
                    span { class: "vlacku-detail-label", "glosses" }
                    for gloss in card.glosses.iter() {
                        span { class: "vlacku-gloss-chip", "{gloss}" }
                    }
                }
            }
            if !card.definition.is_empty() {
                div { class: "vlacku-text-row",
                    span { class: "vlacku-detail-label", "definition" }
                    p { class: "vlacku-definition-text",
                        { render_inline_spans(&card.definition, jvozba_pane) }
                    }
                }
            }
            if !card.notes.is_empty() {
                div { class: "vlacku-text-row",
                    span { class: "vlacku-detail-label", "notes" }
                    p { class: "vlacku-note-text",
                        { render_inline_spans(&card.notes, jvozba_pane) }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_word_action(
    mut jvozba_pane: Signal<VlackuJvozbaPaneState>,
    can_add_to_jvozba: bool,
    word: &str,
    href: &str,
) -> Element {
    let pane_open = jvozba_pane.read().open;
    let word_value = word.to_owned();
    if pane_open && can_add_to_jvozba {
        rsx! {
            button {
                class: "vlacku-headword vlacku-jvozba-add-link-hint",
                r#type: "button",
                title: "Add to jvozba",
                onclick: move |_| add_vlacku_jvozba_item(&mut jvozba_pane, VlackuJvozbaItemKind::Word, word_value.clone()),
                "{word}"
            }
        }
    } else {
        rsx! {
            a { class: "vlacku-headword", href: "{href}", "{word}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vote_display(votes: &VlackuVoteDisplay) -> Element {
    match votes {
        VlackuVoteDisplay::Known(value) => rsx! {
            span { class: vote_class(value), title: vote_title(value), "votes: {value}" }
        },
        VlackuVoteDisplay::Unknown => rsx! {
            span { class: "vlacku-card-meta vlacku-votes is-unknown", title: "No dictionary vote count", "votes: ?" }
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
            span { class: "vlacku-composition-hyphen", "{piece.surface}" }
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
                    span { class: "vlacku-composition-rafsi",
                        span { class: "vlacku-composition-surface", "{piece.surface}" }
                        { render_vlacku_word_action(jvozba_pane, true, source, &href) }
                    }
                }
            } else {
                rsx! {
                    span { class: "vlacku-composition-rafsi", "{piece.surface}" }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_rafsi_pill(mut jvozba_pane: Signal<VlackuJvozbaPaneState>, rafsi: &str) -> Element {
    let pane_open = jvozba_pane.read().open;
    let rafsi_value = rafsi.to_owned();
    if pane_open {
        rsx! {
            button {
                class: "vlacku-rafsi-pill vlacku-jvozba-add-pill-hint",
                r#type: "button",
                title: "Add fixed rafsi to jvozba",
                onclick: move |_| add_vlacku_jvozba_item(&mut jvozba_pane, VlackuJvozbaItemKind::FixedRafsi, rafsi_value.clone()),
                "{rafsi}"
            }
        }
    } else {
        rsx! { span { class: "vlacku-rafsi-pill", "{rafsi}" } }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_inline_spans(
    spans: &[VlackuInline],
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
) -> Element {
    rsx! {
        for span in spans.iter() {
            {
                match span {
                    VlackuInline::Text(text) => rsx! { "{text}" },
                    VlackuInline::Place { label } => rsx! { span { class: "vlacku-place-ref", "{label}" } },
                    VlackuInline::WordRef { label, href } => {
                        render_vlacku_inline_word_ref(jvozba_pane, label, href)
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
    label: &str,
    href: &str,
) -> Element {
    let pane_open = jvozba_pane.read().open;
    let word_value = label.to_owned();
    if pane_open {
        rsx! {
            button {
                class: "vlacku-inline-word-ref",
                r#type: "button",
                title: "Add to jvozba",
                onclick: move |_| add_vlacku_jvozba_item(&mut jvozba_pane, VlackuJvozbaItemKind::Word, word_value.clone()),
                "{label}"
            }
        }
    } else {
        rsx! {
            a { class: "vlacku-inline-word-ref", href: "{href}", "{label}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_jvozba_pane(mut jvozba_pane: Signal<VlackuJvozbaPaneState>) -> Element {
    let pane = jvozba_pane.read().clone();
    let output = build_vlacku_jvozba_output(pane.mode, &pane.items);
    rsx! {
        aside { class: jvozba_pane_class(pane.open),
            button {
                class: "vlacku-jvozba-tab",
                r#type: "button",
                aria_expanded: if pane.open { "true" } else { "false" },
                onclick: move |_| {
                    let mut next = jvozba_pane.read().clone();
                    next.open = !next.open;
                    set_vlacku_jvozba_pane(&mut jvozba_pane, next);
                },
                "jvozba"
            }
            if pane.open {
                div { class: "vlacku-jvozba-body",
                    div { class: "vlacku-jvozba-toolbar",
                        button {
                            class: vlacku_jvozba_mode_class(pane.mode == VlackuJvozbaMode::Lujvo),
                            r#type: "button",
                            onclick: move |_| set_vlacku_jvozba_mode(&mut jvozba_pane, VlackuJvozbaMode::Lujvo),
                            "lujvo"
                        }
                        button {
                            class: vlacku_jvozba_mode_class(pane.mode == VlackuJvozbaMode::Cmevla),
                            r#type: "button",
                            onclick: move |_| set_vlacku_jvozba_mode(&mut jvozba_pane, VlackuJvozbaMode::Cmevla),
                            "cmevla"
                        }
                        button {
                            class: "vlacku-jvozba-clear",
                            r#type: "button",
                            disabled: pane.items.is_empty(),
                            onclick: move |_| {
                                let mut next = jvozba_pane.read().clone();
                                next.items.clear();
                                set_vlacku_jvozba_pane(&mut jvozba_pane, next);
                            },
                            "clear"
                        }
                    }
                    if pane.items.is_empty() {
                        p { class: "vlacku-jvozba-empty", "Click highlighted words or rafsi to add them here." }
                    } else {
                        ol { class: "vlacku-jvozba-items",
                            for (index, item) in pane.items.iter().enumerate() {
                                { render_jvozba_item(jvozba_pane, index, item) }
                            }
                        }
                    }
                    { render_jvozba_output(&output) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_jvozba_item(
    mut jvozba_pane: Signal<VlackuJvozbaPaneState>,
    index: usize,
    item: &VlackuJvozbaItem,
) -> Element {
    rsx! {
        li { class: "vlacku-jvozba-item",
            span { class: "vlacku-jvozba-item-kind", "{jvozba_item_kind_label(item.kind)}" }
            span { class: "vlacku-jvozba-item-value", "{item.value}" }
            button {
                class: "vlacku-jvozba-move",
                r#type: "button",
                disabled: index == 0,
                aria_label: "Move up",
                onclick: move |_| move_vlacku_jvozba_item(&mut jvozba_pane, index, -1),
                "↑"
            }
            button {
                class: "vlacku-jvozba-move",
                r#type: "button",
                aria_label: "Move down",
                onclick: move |_| move_vlacku_jvozba_item(&mut jvozba_pane, index, 1),
                "↓"
            }
            button {
                class: "vlacku-jvozba-remove",
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
fn render_jvozba_output(output: &VlackuJvozbaOutput) -> Element {
    match output {
        VlackuJvozbaOutput::Empty => rsx! {},
        VlackuJvozbaOutput::NeedsMore => rsx! {
            div { class: "vlacku-jvozba-output is-muted", "Add at least two items." }
        },
        VlackuJvozbaOutput::Error { message } => rsx! {
            div { class: "vlacku-jvozba-output is-error", "{message}" }
        },
        VlackuJvozbaOutput::Success { word, segments } => rsx! {
            div { class: "vlacku-jvozba-output",
                div { class: "vlacku-jvozba-word", "{word}" }
                div { class: "vlacku-jvozba-segments",
                    for segment in segments.iter() {
                        span { class: jvozba_segment_class(segment.kind), "{segment.text}" }
                    }
                }
            }
        },
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlacku_mode_class(active: bool) -> &'static str {
    if active {
        "vlacku-mode-button active"
    } else {
        "vlacku-mode-button"
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
        VlackuWebMode::Word => "word",
        VlackuWebMode::Rafsi => "rafsi",
        VlackuWebMode::Sound => "sound or [IPA]",
        VlackuWebMode::Meaning => "meaning search disabled",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn word_type_filter_class(section: VlackuWordTypeSection, parent: bool) -> &'static str {
    match (section, parent) {
        (VlackuWordTypeSection::Brivla, true) => "vlacku-filter is-brivla-parent",
        (VlackuWordTypeSection::Brivla, false) => "vlacku-filter is-brivla-child",
        (VlackuWordTypeSection::Cmavo, _) => "vlacku-filter is-cmavo",
        (VlackuWordTypeSection::Cmevla, _) => "vlacku-filter is-cmevla",
        (VlackuWordTypeSection::Other, _) => "vlacku-filter is-other",
    }
}

#[requires(true)]
#[ensures(true)]
fn toggle_vlacku_word_type(vlacku_state: &mut Signal<VlackuWebState>, value: &str) {
    let mut next = vlacku_state.read().clone();
    if next.word_types.iter().any(|candidate| candidate == value) {
        next.word_types.retain(|candidate| candidate != value);
    } else {
        if value == "brivla" {
            next.word_types.retain(|candidate| {
                !candidate.contains("gismu")
                    && !candidate.contains("lujvo")
                    && !candidate.contains("fu'ivla")
            });
        } else if value.contains("gismu") || value.contains("lujvo") || value.contains("fu'ivla") {
            next.word_types.retain(|candidate| candidate != "brivla");
        }
        next.word_types.push(value.to_owned());
    }
    next.count = VLACKU_WEB_DEFAULT_COUNT;
    vlacku_state.set(next);
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
        "vlacku-card-meta vlacku-votes is-official"
    } else {
        "vlacku-card-meta vlacku-votes"
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vote_title(value: &str) -> &'static str {
    if value == "∞" {
        "Official word"
    } else {
        "Dictionary vote count"
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn jvozba_pane_class(open: bool) -> &'static str {
    if open {
        "vlacku-jvozba-pane is-open"
    } else {
        "vlacku-jvozba-pane"
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlacku_jvozba_mode_class(active: bool) -> &'static str {
    if active {
        "vlacku-jvozba-mode active"
    } else {
        "vlacku-jvozba-mode"
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn jvozba_item_kind_label(kind: VlackuJvozbaItemKind) -> &'static str {
    match kind {
        VlackuJvozbaItemKind::Word => "word",
        VlackuJvozbaItemKind::FixedRafsi => "rafsi",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn jvozba_segment_class(kind: VlackuJvozbaSegmentKind) -> &'static str {
    match kind {
        VlackuJvozbaSegmentKind::Rafsi => "vlacku-jvozba-segment is-rafsi",
        VlackuJvozbaSegmentKind::Hyphen => "vlacku-jvozba-segment is-hyphen",
    }
}

#[requires(true)]
#[ensures(true)]
fn add_vlacku_jvozba_item(
    jvozba_pane: &mut Signal<VlackuJvozbaPaneState>,
    kind: VlackuJvozbaItemKind,
    value: String,
) {
    let mut next = jvozba_pane.read().clone();
    next.open = true;
    next.items.push(VlackuJvozbaItem { kind, value });
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
    match logical {
        "" | "/" | "/gentufa" => AppRoute::Gentufa,
        "/settings" => AppRoute::Settings,
        "/cukta" => AppRoute::Cukta,
        "/vlacku" => AppRoute::Vlacku,
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
        450,
    ) {
        VLACKU_URL_TIMER.with(|timer| timer.set(Some(handle)));
        closure.forget();
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn schedule_vlacku_url_push(base_path: &str, state: &VlackuWebState) {
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
    let open = storage_get("jbotci.vlacku.jvozba.open.v1").as_deref() == Some("true");
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
        if state.open { "true" } else { "false" },
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
        &state
            .items
            .iter()
            .map(format_vlacku_jvozba_item)
            .collect::<Vec<_>>()
            .join("\n"),
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
            })
        })
        .collect()
}

#[requires(true)]
#[ensures(!ret.is_empty() || item.value.is_empty())]
fn format_vlacku_jvozba_item(item: &VlackuJvozbaItem) -> String {
    let kind = match item.kind {
        VlackuJvozbaItemKind::Word => "word",
        VlackuJvozbaItemKind::FixedRafsi => "rafsi",
    };
    format!("{kind}\t{}", item.value.replace('\n', " "))
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
