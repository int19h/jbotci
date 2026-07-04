use super::*;

#[requires(true)]
#[ensures(true)]
pub(super) fn render_settings(
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
                { render_parsing_settings(settings, current_settings, page_find) }
                { render_output_settings(settings, current_settings, page_find) }
                { render_dialect_settings_section(dialect_settings, current_dialect_settings, selected_dialect, qr_uri, page_find) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_parsing_settings(
    mut settings: Signal<UserSettings>,
    current: UserSettings,
    page_find: &PageFindContext,
) -> Element {
    let value = current.error_context_depth.to_string();
    rsx! {
        section { class: "settings-section settings-parsing",
            div { class: "settings-section-head",
                h2 { { render_page_find_text(page_find, "Parsing") } }
            }
            label { class: "settings-field settings-number-field",
                span { class: "settings-field-label",
                    { render_page_find_text(page_find, "Error context depth") }
                }
                input {
                    class: "settings-text-input settings-number-input",
                    r#type: "number",
                    min: "0",
                    step: "1",
                    value: "{value}",
                    aria_label: "Error context depth",
                    oninput: move |event| {
                        if let Some(depth) = parse_error_context_depth(&event.value()) {
                            set_error_context_depth(&mut settings, depth);
                        }
                    },
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_output_settings(
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
pub(super) fn render_stress_mark_button(
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
pub(super) fn render_glide_mark_button(
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
pub(super) fn settings_output_toggle_class(active: bool) -> &'static str {
    if active {
        "settings-output-toggle active"
    } else {
        "settings-output-toggle"
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_dialect_settings_section(
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
pub(super) fn render_builtin_dialect_editor(
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
pub(super) fn render_custom_dialect_editor(
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
pub(super) fn render_dialect_qr_button(
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
pub(super) fn settings_dialect_gentufa_toggle_class(checked: bool, disabled: bool) -> String {
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
pub(super) fn settings_dialect_gentufa_toggle_title(dialect_name: &str) -> &'static str {
    if dialect_name_shows_in_gentufa_picker(dialect_name) {
        "Show this dialect as a checkbox in the Gentufa dialect picker."
    } else {
        "Slash-named dialects can be typed in formulas, but they do not appear as Gentufa checkbox options."
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_delete_icon() -> Element {
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
pub(super) fn render_dialect_qr_icon() -> Element {
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
pub(super) fn render_dialect_qr_popout(mut qr_uri: Signal<Option<String>>) -> Element {
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
pub(super) fn initial_dialect_settings_selection(settings: &DialectSettings) -> String {
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
pub(super) fn selected_dialect_name(settings: &DialectSettings, requested: &str) -> String {
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
pub(super) fn selected_dialect_definition_text(
    settings: &DialectSettings,
    name: &str,
) -> Option<String> {
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
pub(super) fn johau_uri_for_selected_dialect(
    settings: &DialectSettings,
    name: &str,
) -> Option<String> {
    let definition = selected_dialect_definition_text(settings, name)?;
    custom_dialect_definition_to_johau_uri_with_custom_dialects(
        &settings.custom_dialects,
        &definition,
    )
    .ok()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn builtin_dialect_shows_in_gentufa(settings: &DialectSettings, name: &str) -> bool {
    dialect_name_shows_in_gentufa_picker(name)
        && !settings.hidden_builtin_gentufa_dialects.contains(name)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_dialect_settings(
    mut dialect_settings: Signal<DialectSettings>,
    next: DialectSettings,
) {
    save_dialect_settings(&next);
    dialect_settings.set(next);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn add_custom_dialect(
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
pub(super) fn next_custom_dialect_name(customs: &[CustomDialect]) -> String {
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
pub(super) fn delete_custom_dialect(
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
pub(super) fn rename_custom_dialect(
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
pub(super) fn update_custom_dialect_definition(
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
pub(super) fn toggle_custom_dialect_gentufa_visibility(
    dialect_settings: Signal<DialectSettings>,
    name: &str,
) {
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
pub(super) fn toggle_builtin_dialect_gentufa_visibility(
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
pub(super) fn render_embedding_settings(
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
pub(super) fn render_embedding_progress(
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
pub(super) fn embedding_progress_display_label(state: &EmbeddingSettingsState) -> String {
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
