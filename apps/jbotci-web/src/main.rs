use dioxus::prelude::*;
use jbotci_output::{GlideMark, PhonemeRenderOptions, StressMark};
use jbotci_web_core::{
    GentufaBlock, GentufaCell, GentufaError, GentufaScript, GentufaSuccess, GentufaTreeRow,
    GentufaWebOptions, GentufaWebRequest, GentufaWebResult, GentufaWebViewMode, ReferenceLabel,
    ReferenceMarker, ReferenceMarkerRole, ReferenceSlotLabel, WebFeatureAvailability,
    parse_gentufa_for_web,
};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

const MAIN_CSS: Asset = asset!("/assets/main.css");
const LOGO: Asset = asset!("/assets/icons/jbotci-dark.svg");
const NOTO_SANS: Asset = asset!("/assets/fonts/noto-sans-variable.ttf");
const NOTO_SANS_ITALIC: Asset = asset!("/assets/fonts/noto-sans-italic-variable.ttf");
const NOTO_SANS_MATH: Asset = asset!("/assets/fonts/noto-sans-math-regular.otf");
const CRISA: Asset = asset!("/assets/fonts/crisa-regular.otf");
const DEFAULT_GENTUFA_TEXT: &str = "cadga fa lonu ro lo prenu goi ko'a cu troci lonu ko'a tarti loka ce'u xendo je cnikansa ro lo jmive kei ta'i lo racli";

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
  font-style: normal;
  font-display: swap;
}}

@font-face {{
  font-family: "Noto Sans";
  src: url("{noto_sans_italic}") format("truetype");
  font-weight: 100 900;
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
    let app_class = format!(
        "spa-shell app-page theme-{} orthography-{}",
        theme_class(settings_value.theme),
        script_class(settings_value.script)
    );

    rsx! {
        style { "{font_face_css()}" }
        document::Stylesheet { href: MAIN_CSS }
        document::Link { rel: "icon", r#type: "image/svg+xml", href: LOGO }
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
                            AppRoute::Vlacku => render_disabled("vlacku"),
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
                        span {
                            class: "app-topbar-link is-disabled",
                            aria_disabled: "true",
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
                div { class: "definition-panel",
                    div { class: "def-connector" }
                    div { class: "def-content" }
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
                            mn { "{occurrence}" }
                        }
                    } else {
                        mi { "{stem}" }
                    }
                    if let Some(text) = slot_text.as_deref() {
                        mo { "⟨" }
                        mtext { "{text}" }
                        mo { "⟩" }
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
        _ => AppRoute::Gentufa,
    }
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
