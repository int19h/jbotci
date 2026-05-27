use dioxus::prelude::*;
use jbotci_output::{GlideMark, PhonemeRenderOptions, StressMark};
use jbotci_web_core::{
    GentufaBlock, GentufaCell, GentufaError, GentufaScript, GentufaSuccess, GentufaTreeRow,
    GentufaWebOptions, GentufaWebRequest, GentufaWebResult, GentufaWebViewMode, MathVariable,
    ReferenceMarker, ReferenceMarkerRole, WebFeatureAvailability, parse_gentufa_for_web,
};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

const MAIN_CSS: Asset = asset!("/assets/main.css");
const LOGO: Asset = asset!("/assets/icons/jbotci.svg");
const DEFAULT_GENTUFA_TEXT: &str = "mi klama le zarci";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum ThemeMode {
    Day,
    Night,
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
    #[ensures(ret.theme == ThemeMode::Day)]
    fn default() -> Self {
        Self {
            theme: ThemeMode::Day,
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
        document::Stylesheet { href: MAIN_CSS }
        document::Link { rel: "icon", r#type: "image/svg+xml", href: LOGO }
        div { class: "{app_class}",
            { render_topbar(route, &base_path, settings, settings_value) }
            main { class: "spa-main",
                div { class: "spa-stack",
                    {
                        match route {
                            AppRoute::Gentufa => rsx! {
                                section { class: "spa-page parse-page spa-gentufa-page",
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
                                            { render_result(&result, view_mode, view_mode_value, settings, settings_value) }
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
                class: theme_button_class(current == ThemeMode::Day),
                r#type: "button",
                aria_label: "Use light theme",
                aria_pressed: pressed_attr(current == ThemeMode::Day),
                onclick: move |_| {
                    let mut next = *settings.read();
                    next.theme = ThemeMode::Day;
                    settings.set(next);
                    save_settings(&next);
                },
                "☀"
            }
            button {
                class: theme_button_class(current == ThemeMode::Night),
                r#type: "button",
                aria_label: "Use dark theme",
                aria_pressed: pressed_attr(current == ThemeMode::Night),
                onclick: move |_| {
                    let mut next = *settings.read();
                    next.theme = ThemeMode::Night;
                    settings.set(next);
                    save_settings(&next);
                },
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
            title: "Orthography icons: j = latin, ж = cyrillic, z = zbalermorna",
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
                span { class: "orthography-btn-icon", "z" }
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
) -> Element {
    rsx! {
        section { class: "result-section",
            { render_surface_output(success) }
            { render_diagnostics(success) }
            { render_view_tabs(view_mode, view_mode_value) }
            { render_output_controls(view_mode_value, settings, settings_value) }
            if view_mode_value == GentufaWebViewMode::Blocks {
                { render_blocks(success, settings_value.show_glosses) }
            } else {
                { render_tree(success, settings_value.show_glosses, false) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_surface_output(success: &GentufaSuccess) -> Element {
    rsx! {
        div { class: "brackets-section",
            div { class: "surface-output-stack",
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
                { render_static_checkbox("English labels", true, true) }
                { render_gloss_checkbox(settings, current.show_glosses) }
                { render_elided_checkbox(settings, current.show_elided) }
            }
        },
        GentufaWebViewMode::Tree => rsx! {
            div { class: "controls table-controls",
                { render_static_checkbox("English labels", true, true) }
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
fn render_blocks(success: &GentufaSuccess, show_glosses: bool) -> Element {
    let column_count = success.blocks_layout.max_col.max(1);
    let column_template = repeated_auto_template(column_count);
    let row_count = success.blocks_layout.max_row + usize::from(show_glosses);
    let row_template = format!("repeat({}, auto)", row_count.max(1));
    let container_class = if show_glosses {
        "blocks-container"
    } else {
        "blocks-container gloss-hidden"
    };
    let gloss_row = success.blocks_layout.max_row + 1;
    rsx! {
        section { class: "blocks-view",
            div { class: "blocks-scroll-shell",
                div { class: "blocks-svg-link",
                    span { class: "export-link is-disabled", "SVG" }
                    span { class: "export-link is-disabled", "PNG" }
                }
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
                                { render_block(block) }
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
fn render_block(block: &GentufaBlock) -> Element {
    let row = block.row + 1;
    let col = block.col + 1;
    let classes = block_class(block);
    let style = format!(
        "grid-row: {row} / span {}; grid-column: {col} / span {}; --block-color: {}; background-color: {};",
        block.row_span, block.col_span, block.color, block.color
    );
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
            if let Some(variable) = &block.place_label {
                { render_block_target_ref(variable) }
            }
            span { class: "block-label", title: "{block.label}",
                "{block.label}"
            }
            if block.relation_var.is_some() || !block.ref_markers.is_empty() {
                { render_block_source_refs(block) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_block_target_ref(variable: &MathVariable) -> Element {
    rsx! {
        span { class: "block-ref-target",
            span { class: "ref-math",
                span { class: "ref-var ref-target place-var",
                    { render_math_variable(variable) }
                }
                span { class: "ref-assign", "≔" }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_block_source_refs(block: &GentufaBlock) -> Element {
    rsx! {
        span { class: "block-ref-source",
            span { class: "ref-math",
                span { class: "ref-arrow", "→" }
                if let Some(variable) = &block.relation_var {
                    span { class: "ref-var ref-source",
                        { render_math_variable(variable) }
                    }
                }
                for marker in block.ref_markers.iter() {
                    { render_ref_marker(marker) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_ref_marker(marker: &ReferenceMarker) -> Element {
    let class = match marker.role {
        ReferenceMarkerRole::Reference => "ref-var ref-source",
        ReferenceMarkerRole::Referent | ReferenceMarkerRole::Place => "ref-var ref-target",
    };
    rsx! {
        span { class: "{class}", title: "{marker.kind}",
            { render_math_variable(&marker.label) }
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
fn render_tree(success: &GentufaSuccess, show_glosses: bool, show_definitions: bool) -> Element {
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
                            { render_tree_row(row, show_glosses, show_definitions) }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_tree_row(row: &GentufaTreeRow, show_glosses: bool, show_definitions: bool) -> Element {
    let row_class = if tree_row_is_elided(row) {
        "elided-row"
    } else {
        ""
    };
    let style = format!("--row-color: {}; --indent-count: {};", row.color, row.depth);
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
                            "{row.label}"
                            if let Some(variable) = &row.relation_var {
                                { render_math_variable(variable) }
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
fn render_math_variable(variable: &MathVariable) -> Element {
    match variable.subscript {
        Some(subscript) => rsx! {
            span { class: "spa-cll-math",
                math { class: "math-var", display: "inline",
                    msub {
                        mi { "{variable.base}" }
                        mtext { "{subscript}" }
                    }
                }
            }
        },
        None => rsx! {
            span { class: "spa-cll-math",
                math { class: "math-var", display: "inline",
                    mi { "{variable.base}" }
                }
            }
        },
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
fn repeated_auto_template(count: usize) -> String {
    std::iter::repeat_n("auto", count)
        .collect::<Vec<_>>()
        .join(" ")
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
    if storage_get("jbotci.theme").as_deref() == Some("night") {
        settings.theme = ThemeMode::Night;
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
