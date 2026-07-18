use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct GimfihiPageSnapshot {
    pub(super) source_word_count: usize,
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_page_snapshot(
    gimfihi_source_word_memory: Signal<BTreeMap<String, String>>,
) -> GimfihiPageSnapshot {
    GimfihiPageSnapshot {
        source_word_count: gimfihi_source_word_memory.read().len(),
    }
}

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
#[component]
pub(super) fn GimfihiPage(
    gimfihi_draft_state: Signal<GimfihiWebState>,
    gimfihi_committed_state: Signal<GimfihiWebState>,
    gimfihi_result: Signal<GimfihiAsyncResultState>,
    gimfihi_source_word_memory: Signal<BTreeMap<String, String>>,
    base_path: String,
    script: GentufaScript,
    page_find: PageFindContext,
) -> Element {
    let snapshot = use_memo(move || gimfihi_page_snapshot(gimfihi_source_word_memory));
    let snapshot = snapshot.read().clone();
    render_gimfihi_page(
        gimfihi_draft_state,
        gimfihi_committed_state,
        gimfihi_result,
        gimfihi_source_word_memory,
        &snapshot,
        &base_path,
        script,
        &page_find,
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gimfihi_page(
    gimfihi_draft_state: Signal<GimfihiWebState>,
    gimfihi_committed_state: Signal<GimfihiWebState>,
    gimfihi_result: Signal<GimfihiAsyncResultState>,
    gimfihi_source_word_memory: Signal<BTreeMap<String, String>>,
    snapshot: &GimfihiPageSnapshot,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let _ = snapshot.source_word_count;
    rsx! {
        section { class: "spa-page dictionary-page gimfihi-page",
            h1 { class: "sr-only", "jbotci gimfi'i" }
            div { class: "gimfihi-shell",
                GimfihiControlsPanel {
                    gimfihi_draft_state,
                    gimfihi_committed_state,
                    gimfihi_source_word_memory,
                }
                GimfihiResultPanel {
                    gimfihi_draft_state,
                    gimfihi_committed_state,
                    gimfihi_result,
                    base_path: base_path.to_owned(),
                    script,
                    page_find: page_find.clone(),
                }
            }
        }
    }
}

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
#[component]
pub(super) fn GimfihiControlsPanel(
    gimfihi_draft_state: Signal<GimfihiWebState>,
    gimfihi_committed_state: Signal<GimfihiWebState>,
    gimfihi_source_word_memory: Signal<BTreeMap<String, String>>,
) -> Element {
    let draft_state = gimfihi_draft_state.read().clone();
    render_gimfihi_controls(
        gimfihi_draft_state,
        gimfihi_committed_state,
        gimfihi_source_word_memory,
        &draft_state,
    )
}

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
#[component]
pub(super) fn GimfihiResultPanel(
    gimfihi_draft_state: Signal<GimfihiWebState>,
    gimfihi_committed_state: Signal<GimfihiWebState>,
    gimfihi_result: Signal<GimfihiAsyncResultState>,
    base_path: String,
    script: GentufaScript,
    page_find: PageFindContext,
) -> Element {
    let committed_state = gimfihi_committed_state.read().clone();
    let result_state = gimfihi_result.read().clone();
    let result_current = result_state.state.as_ref() == Some(&committed_state);
    let result = if result_current {
        result_state.result.clone()
    } else {
        gimfihi_empty_result(&committed_state)
    };
    let loading = gimfihi_result_panel_is_loading(&committed_state, &result_state);
    let show_result_errors = gimfihi_state_has_any_source_word(&committed_state);
    rsx! {
        if loading {
            p { class: "gimfihi-status",
                { render_page_find_text(&page_find, "Loading gismu candidates.") }
            }
        }
        if let Some(error) = &result_state.error {
            div { class: "spa-error dictionary-error",
                { render_page_find_text(&page_find, error) }
            }
        }
        if show_result_errors {
            for error in result.errors.iter() {
                div { class: "spa-error dictionary-error",
                    { render_page_find_text(&page_find, error) }
                }
            }
        }
        { render_gimfihi_results(
            &result,
            gimfihi_draft_state,
            gimfihi_committed_state,
            gimfihi_result,
            &base_path,
            script,
            &page_find,
        ) }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gimfihi_controls(
    mut gimfihi_draft_state: Signal<GimfihiWebState>,
    mut gimfihi_committed_state: Signal<GimfihiWebState>,
    mut gimfihi_source_word_memory: Signal<BTreeMap<String, String>>,
    state: &GimfihiWebState,
) -> Element {
    let current_preset = state
        .preset
        .map(|preset| preset.as_str().to_owned())
        .unwrap_or_default();
    let scorer = state.scorer.as_str();
    let collision_scope = state.check_collisions.as_str();
    let preset_options = gimfihi_preset_options_for_state(state);
    let language_suggestions = gimfihi_language_suggestions();
    rsx! {
        div { class: "gimfihi-form",
            div { class: "gimfihi-preset-row",
                label { class: "gimfihi-control gimfihi-preset-control",
                    span { class: "gimfihi-control-label", "Preset" }
                    select {
                        class: "gimfihi-select",
                        value: "{current_preset}",
                        onchange: move |event| {
                            let value = event.value();
                            let current = gimfihi_draft_state.read().clone();
                            let source_words = gimfihi_source_word_memory.with_mut(|memory| {
                                update_gimfihi_source_word_memory(memory, &current);
                                memory.clone()
                            });
                            let next = match value.parse::<GimfihiPreset>() {
                                Ok(preset) => gimfihi_state_for_selected_preset(&current, Some(preset), &source_words),
                                Err(_) => gimfihi_state_with_explicit_custom_weights(&current),
                            };
                            gimfihi_draft_state.set(next);
                        },
                        option { value: "", "custom" }
                        for option in preset_options.iter() {
                            option {
                                value: "{option.value}",
                                "{option.label}"
                            }
                        }
                    }
                }
                label { class: "gimfihi-control gimfihi-scorer-control",
                    span { class: "gimfihi-control-label", "Scorer" }
                    select {
                        class: "gimfihi-select",
                        value: "{scorer}",
                        onchange: move |event| {
                            let mut next = gimfihi_draft_state.read().clone();
                            next.scorer = if event.value() == GimfihiScorer::Phonetic.as_str() {
                                GimfihiScorer::Phonetic
                            } else {
                                GimfihiScorer::Classic
                            };
                            gimfihi_draft_state.set(next);
                        },
                        option { value: "classic", "Classic" }
                        option { value: "phonetic", "Phonetic" }
                    }
                }
            }
            div { class: "gimfihi-source-table-wrap",
                datalist { id: "gimfihi-language-options",
                    for language in language_suggestions.iter() {
                        option { value: "{language}" }
                    }
                }
                table { class: "gimfihi-source-table",
                    thead {
                        tr {
                            th { class: "gimfihi-language-column", "Language" }
                            th { "Weight" }
                            th { "Word" }
                            th { class: "gimfihi-actions-column", "Actions" }
                        }
                    }
                    tbody {
                        for (index, source) in state.sources.iter().enumerate() {
                            { render_gimfihi_source_row(gimfihi_draft_state, gimfihi_source_word_memory, state, index, source) }
                        }
                        { render_gimfihi_add_source_row(gimfihi_draft_state) }
                    }
                }
            }
            div { class: "gimfihi-option-row",
                div { class: "gimfihi-shape-group", role: "group", aria_label: "Gismu shapes",
                    { render_gimfihi_shape_toggle(gimfihi_draft_state, state, GismuShape::Ccvcv) }
                    { render_gimfihi_shape_toggle(gimfihi_draft_state, state, GismuShape::Cvccv) }
                }
                label { class: "compact-check gimfihi-checkbox",
                    input {
                        r#type: "checkbox",
                        checked: state.all_letters,
                        onchange: move |_| {
                            let mut next = gimfihi_draft_state.read().clone();
                            next.all_letters = !next.all_letters;
                            gimfihi_draft_state.set(next);
                        },
                    }
                    span { "use all letters" }
                }
                label { class: "gimfihi-inline-select gimfihi-collision-control",
                    span { "collisions" }
                    select {
                        class: "gimfihi-select",
                        value: "{collision_scope}",
                        onchange: move |event| {
                            let mut next = gimfihi_draft_state.read().clone();
                            if let Ok(scope) = event.value().parse::<CollisionScope>() {
                                next.check_collisions = scope;
                            }
                            gimfihi_draft_state.set(next);
                        },
                        option { value: "all", "all" }
                        option { value: "official", "official" }
                        option { value: "none", "none" }
                    }
                }
                label { class: "compact-check gimfihi-checkbox",
                    input {
                        r#type: "checkbox",
                        checked: state.show_collisions,
                        onchange: move |_| {
                            let mut next = gimfihi_draft_state.read().clone();
                            next.show_collisions = !next.show_collisions;
                            gimfihi_draft_state.set(next);
                        },
                    }
                    span { "show collisions" }
                }
                label { class: "compact-check gimfihi-checkbox",
                    input {
                        r#type: "checkbox",
                        checked: state.require_free_short_rafsi,
                        onchange: move |_| {
                            let mut next = gimfihi_draft_state.read().clone();
                            next.require_free_short_rafsi = !next.require_free_short_rafsi;
                            gimfihi_draft_state.set(next);
                        },
                    }
                    span { "require free rafsi" }
                }
                button {
                    class: "btn-parse gimfihi-generate-button",
                    r#type: "button",
                    onclick: move |_| {
                        let next = normalize_gimfihi_state(&gimfihi_draft_state.read());
                        gimfihi_draft_state.set(next.clone());
                        gimfihi_committed_state.set(next);
                    },
                    "Generate"
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gimfihi_source_row(
    mut gimfihi_draft_state: Signal<GimfihiWebState>,
    mut gimfihi_source_word_memory: Signal<BTreeMap<String, String>>,
    state: &GimfihiWebState,
    index: usize,
    source: &GimfihiWebSource,
) -> Element {
    let row_count = state.sources.len();
    let weight_value = gimfihi_source_weight_value(state, source);
    let min_weight = GIMFIHI_MIN_WEIGHT.to_string();
    let max_weight = GIMFIHI_MAX_WEIGHT.to_string();
    let language = source.language.clone();
    let word = source.word.clone();
    rsx! {
        tr { class: "gimfihi-source-row",
            td { class: "gimfihi-language-column",
                input {
                    class: "gimfihi-lang-input",
                    r#type: "text",
                    list: "gimfihi-language-options",
                    spellcheck: "false",
                    value: "{language}",
                    oninput: move |event| {
                        let next_language = event.value();
                        let current = gimfihi_draft_state.read().clone();
                        if let Some(source) = current.sources.get(index) {
                            gimfihi_source_word_memory.with_mut(|memory| {
                                if gimfihi_source_language_key(&source.language)
                                    != gimfihi_source_language_key(&next_language)
                                {
                                    gimfihi_remove_source_word_memory_entry(memory, &source.language);
                                }
                                gimfihi_set_source_word_memory_entry(
                                    memory,
                                    &next_language,
                                    &source.word,
                                );
                            });
                        }
                        let next = gimfihi_state_with_source_language(&current, index, &next_language);
                        gimfihi_draft_state.set(next);
                    },
                }
            }
            td {
                div { class: "gimfihi-weight-cell",
                    input {
                        class: "gimfihi-weight-slider",
                        r#type: "range",
                        min: "{min_weight}",
                        max: "{max_weight}",
                        step: "1",
                        value: "{weight_value}",
                        onchange: move |event| {
                            let next = gimfihi_state_with_source_weight(&gimfihi_draft_state.read(), index, &event.value());
                            gimfihi_draft_state.set(next);
                        },
                    }
                    input {
                        class: "gimfihi-weight-number",
                        r#type: "number",
                        min: "{min_weight}",
                        max: "{max_weight}",
                        step: "1",
                        value: "{weight_value}",
                        onchange: move |event| {
                            let next = gimfihi_state_with_source_weight(&gimfihi_draft_state.read(), index, &event.value());
                            gimfihi_draft_state.set(next);
                        },
                    }
                }
            }
            td {
                input {
                    class: "gimfihi-word-input",
                    r#type: "text",
                    spellcheck: "false",
                    placeholder: "Lojban or [aj piː ej]",
                    value: "{word}",
                    oninput: move |event| {
                        let next_word = event.value();
                        let current = gimfihi_draft_state.read().clone();
                        if let Some(source) = current.sources.get(index) {
                            gimfihi_source_word_memory.with_mut(|memory| {
                                gimfihi_set_source_word_memory_entry(
                                    memory,
                                    &source.language,
                                    &next_word,
                                );
                            });
                        }
                        let next = gimfihi_state_with_source_word(&current, index, &next_word);
                        gimfihi_draft_state.set(next);
                    },
                }
            }
            td { class: "gimfihi-actions-column",
                button {
                    class: "gimfihi-row-button gimfihi-delete-button",
                    r#type: "button",
                    disabled: row_count <= 1,
                    onclick: move |_| {
                        let current = gimfihi_draft_state.read().clone();
                        if let Some(source) = current.sources.get(index) {
                            gimfihi_source_word_memory.with_mut(|memory| {
                                gimfihi_remove_source_word_memory_entry(memory, &source.language);
                            });
                        }
                        let next = gimfihi_state_without_source(&current, index);
                        gimfihi_draft_state.set(next);
                    },
                    "Delete"
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gimfihi_add_source_row(
    mut gimfihi_draft_state: Signal<GimfihiWebState>,
) -> Element {
    rsx! {
        tr { class: "gimfihi-source-row gimfihi-add-row",
            td { class: "gimfihi-language-column" }
            td {}
            td {}
            td { class: "gimfihi-actions-column",
                button {
                    class: "gimfihi-row-button gimfihi-add-button",
                    r#type: "button",
                    onclick: move |_| {
                        let mut next = gimfihi_state_with_explicit_custom_weights(&gimfihi_draft_state.read());
                        next.sources.push(GimfihiWebSource {
                            language: String::new(),
                            weight: Some("1".to_owned()),
                            word: String::new(),
                        });
                        gimfihi_draft_state.set(next);
                    },
                    "Add"
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gimfihi_shape_toggle(
    mut gimfihi_draft_state: Signal<GimfihiWebState>,
    state: &GimfihiWebState,
    shape: GismuShape,
) -> Element {
    let selected = state.shapes.iter().any(|current| *current == shape);
    let label = shape.as_str().to_ascii_uppercase();
    rsx! {
        label { class: class_names("compact-check gimfihi-shape-toggle", &[("is-selected", selected)]),
            input {
                r#type: "checkbox",
                checked: selected,
                onchange: move |_| {
                    let next = gimfihi_state_with_shape_toggled(&gimfihi_draft_state.read(), shape);
                    gimfihi_draft_state.set(next);
                },
            }
            span { "{label}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gimfihi_results(
    result: &GimfihiWebResult,
    mut gimfihi_draft_state: Signal<GimfihiWebState>,
    mut gimfihi_committed_state: Signal<GimfihiWebState>,
    gimfihi_result: Signal<GimfihiAsyncResultState>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let Some(output) = &result.output else {
        return rsx! {};
    };
    if output.candidates.is_empty() {
        return rsx! {
            p { class: "dictionary-empty",
                { render_page_find_text(page_find, "No candidates found.") }
            }
        };
    }
    let summary = gimfihi_result_summary(output);
    let show_collisions = result.state.show_collisions;
    let has_more =
        result.state.count < output.filtered_count && result.state.count < GIMFIHI_MAX_COUNT;
    rsx! {
        div { class: "gimfihi-results",
            div { class: "gimfihi-result-summary",
                { render_page_find_text(page_find, &summary) }
            }
            div { class: "gimfihi-results-table-wrap",
                table { class: "gimfihi-results-table",
                    thead {
                        tr {
                            th { "Candidate" }
                            th { "Score" }
                            if show_collisions {
                                th { "Existing" }
                            }
                            th { "possible rafsi" }
                        }
                    }
                    tbody {
                        for candidate in output.candidates.iter() {
                            GimfihiCandidateRow {
                                key: "{candidate.word}",
                                candidate: candidate.clone(),
                                show_collisions,
                                gimfihi_draft_state,
                                gimfihi_committed_state,
                                gimfihi_result,
                                base_path: base_path.to_owned(),
                                script,
                                page_find: page_find.clone(),
                            }
                        }
                    }
                }
            }
            if has_more {
                div { class: "load-more-wrap",
                    button {
                        class: "btn-parse load-more-link",
                        r#type: "button",
                        onclick: move |_| {
                            let next = gimfihi_load_more_state(&gimfihi_committed_state.read());
                            gimfihi_draft_state.set(next.clone());
                            gimfihi_committed_state.set(next);
                        },
                        { render_page_find_text(page_find, "Load more") }
                    }
                }
            }
        }
    }
}

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
#[component]
pub(super) fn GimfihiCandidateRow(
    candidate: GimfihiCandidate,
    show_collisions: bool,
    mut gimfihi_draft_state: Signal<GimfihiWebState>,
    mut gimfihi_committed_state: Signal<GimfihiWebState>,
    mut gimfihi_result: Signal<GimfihiAsyncResultState>,
    base_path: String,
    script: GentufaScript,
    page_find: PageFindContext,
) -> Element {
    let word = candidate.word.clone();
    let score = format_gimfihi_score(candidate.score);
    let existing = candidate
        .collision
        .as_ref()
        .map(|collision| collision.existing_word.clone());
    let row_class = class_names(
        "gimfihi-candidate-row",
        &[("is-highlighted", candidate.highlighted)],
    );
    let base_path_for_highlight = base_path.clone();
    rsx! {
        tr { class: "{row_class}",
            td {
                button {
                    class: "gimfihi-candidate-button",
                    r#type: "button",
                    aria_pressed: pressed_attr(candidate.highlighted),
                    onclick: move |_| {
                        let next = gimfihi_state_with_highlight(
                            &gimfihi_committed_state.read(),
                            &word,
                        );
                        let current_result = gimfihi_result.read().clone();
                        if let Some(highlighted_result) = gimfihi_result_state_with_highlight(
                            &base_path_for_highlight,
                            &next,
                            &current_result,
                        ) {
                            gimfihi_result.set(highlighted_result);
                        }
                        gimfihi_draft_state.set(next.clone());
                        gimfihi_committed_state.set(next);
                    },
                    { render_page_find_text(&page_find, &candidate.word) }
                }
            }
            td { { render_page_find_text(&page_find, &score) } }
            if show_collisions {
                td {
                    { render_gimfihi_dictionary_word_link(existing.as_deref(), &base_path, script, &page_find) }
                }
            }
            td {
                { render_gimfihi_rafsis(candidate.rafsi(), &base_path, script, &page_find) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_preset_options_for_state(
    state: &GimfihiWebState,
) -> Vec<GimfihiPresetOption> {
    all_presets()
        .iter()
        .map(|preset| GimfihiPresetOption {
            value: preset.as_str().to_owned(),
            label: preset.as_str().to_owned(),
            selected: state.preset == Some(*preset),
        })
        .collect()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn gimfihi_language_suggestions() -> Vec<String> {
    let mut languages = BTreeSet::new();
    for preset in all_presets() {
        for entry in preset.entries() {
            languages.insert(entry.language.to_owned());
        }
    }
    languages.into_iter().collect()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_sources_for_preset(preset: GimfihiPreset) -> Vec<GimfihiWebSource> {
    preset
        .entries()
        .iter()
        .map(|entry| GimfihiWebSource {
            language: entry.language.to_owned(),
            weight: None,
            word: String::new(),
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_state_for_selected_preset(
    state: &GimfihiWebState,
    preset: Option<GimfihiPreset>,
    source_words: &BTreeMap<String, String>,
) -> GimfihiWebState {
    let Some(preset) = preset else {
        return gimfihi_state_with_explicit_custom_weights(state);
    };
    let mut words_by_language = source_words.clone();
    words_by_language.extend(state.sources.iter().filter_map(|source| {
        let language = gimfihi_source_language_key(&source.language)?;
        Some((language, source.word.clone()))
    }));
    let mut next = state.clone();
    next.preset = Some(preset);
    next.sources = gimfihi_sources_for_preset(preset)
        .into_iter()
        .map(|mut source| {
            if let Some(word) = gimfihi_source_language_key(&source.language)
                .and_then(|language| words_by_language.get(&language))
            {
                source.word = word.clone();
            }
            source
        })
        .collect();
    next
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_source_word_memory_from_state(
    state: &GimfihiWebState,
) -> BTreeMap<String, String> {
    let mut memory = BTreeMap::new();
    update_gimfihi_source_word_memory(&mut memory, state);
    memory
}

#[requires(true)]
#[ensures(true)]
pub(super) fn update_gimfihi_source_word_memory(
    memory: &mut BTreeMap<String, String>,
    state: &GimfihiWebState,
) {
    for source in &state.sources {
        gimfihi_set_source_word_memory_entry(memory, &source.language, &source.word);
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_set_source_word_memory_entry(
    memory: &mut BTreeMap<String, String>,
    language: &str,
    word: &str,
) {
    let Some(language) = gimfihi_source_language_key(language) else {
        return;
    };
    let word = word.trim();
    if word.is_empty() {
        memory.remove(&language);
    } else {
        memory.insert(language, word.to_owned());
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_remove_source_word_memory_entry(
    memory: &mut BTreeMap<String, String>,
    language: &str,
) {
    if let Some(language) = gimfihi_source_language_key(language) {
        memory.remove(&language);
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|value| !value.is_empty()))]
pub(super) fn gimfihi_source_language_key(language: &str) -> Option<String> {
    let language = language.trim().to_ascii_lowercase();
    (!language.is_empty()).then_some(language)
}

#[requires(true)]
#[ensures(ret.preset.is_none())]
pub(super) fn gimfihi_state_with_explicit_custom_weights(
    state: &GimfihiWebState,
) -> GimfihiWebState {
    let preset = state.preset;
    let mut next = state.clone();
    next.preset = None;
    for source in &mut next.sources {
        if source.weight.is_none() {
            source.weight = Some(gimfihi_preset_weight_text(preset, &source.language));
        }
    }
    next
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_state_with_source_language(
    state: &GimfihiWebState,
    index: usize,
    language: &str,
) -> GimfihiWebState {
    let mut next = state.clone();
    if let Some(source) = next.sources.get_mut(index) {
        source.language = language.to_owned();
    }
    let Some(preset) = state.preset else {
        return next;
    };
    if gimfihi_language_multiset(&next.sources) == gimfihi_preset_language_multiset(preset) {
        next
    } else {
        let mut custom = gimfihi_state_with_explicit_custom_weights(state);
        if let Some(source) = custom.sources.get_mut(index) {
            source.language = language.to_owned();
        }
        custom
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_state_with_source_weight(
    state: &GimfihiWebState,
    index: usize,
    weight: &str,
) -> GimfihiWebState {
    let mut next = state.clone();
    if let Some(source) = next.sources.get_mut(index) {
        source.weight = gimfihi_optional_text(weight);
    }
    next
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_state_with_source_word(
    state: &GimfihiWebState,
    index: usize,
    word: &str,
) -> GimfihiWebState {
    let mut next = state.clone();
    if let Some(source) = next.sources.get_mut(index) {
        source.word = word.to_owned();
    }
    next
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_state_without_source(
    state: &GimfihiWebState,
    index: usize,
) -> GimfihiWebState {
    let mut next = gimfihi_state_with_explicit_custom_weights(state);
    if index < next.sources.len() {
        next.sources.remove(index);
    }
    next
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_state_with_shape_toggled(
    state: &GimfihiWebState,
    shape: GismuShape,
) -> GimfihiWebState {
    let mut next = state.clone();
    if let Some(index) = next.shapes.iter().position(|current| *current == shape) {
        next.shapes.remove(index);
    } else {
        next.shapes.push(shape);
    }
    next
}

#[requires(!highlight.trim().is_empty())]
#[ensures(ret.highlight.as_ref().is_some_and(|value| value == &highlight.trim().to_ascii_lowercase()))]
pub(super) fn gimfihi_state_with_highlight(
    state: &GimfihiWebState,
    highlight: &str,
) -> GimfihiWebState {
    let mut next = state.clone();
    next.highlight = Some(highlight.trim().to_owned());
    normalize_gimfihi_state(&next)
}

#[requires(true)]
#[ensures(ret.count >= 1 && ret.count <= GIMFIHI_MAX_COUNT)]
pub(super) fn gimfihi_load_more_state(state: &GimfihiWebState) -> GimfihiWebState {
    let mut next = state.clone();
    next.count = next.count.saturating_mul(2).clamp(1, GIMFIHI_MAX_COUNT);
    next
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_language_multiset(sources: &[GimfihiWebSource]) -> Vec<String> {
    let mut languages = sources
        .iter()
        .map(|source| source.language.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();
    languages.sort();
    languages
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_preset_language_multiset(preset: GimfihiPreset) -> Vec<String> {
    let mut languages = preset
        .entries()
        .iter()
        .map(|entry| entry.language.to_owned())
        .collect::<Vec<_>>();
    languages.sort();
    languages
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_source_weight_value(
    state: &GimfihiWebState,
    source: &GimfihiWebSource,
) -> String {
    source
        .weight
        .clone()
        .unwrap_or_else(|| gimfihi_preset_weight_text(state.preset, &source.language))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn gimfihi_preset_weight_text(preset: Option<GimfihiPreset>, language: &str) -> String {
    preset
        .and_then(|preset| {
            preset
                .entries()
                .iter()
                .find(|entry| entry.language == language.trim())
                .map(|entry| entry.weight.to_string())
        })
        .unwrap_or_else(|| "1".to_owned())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_state_has_any_source_word(state: &GimfihiWebState) -> bool {
    state
        .sources
        .iter()
        .any(|source| !source.word.trim().is_empty())
}

#[requires(true)]
#[ensures(!gimfihi_state_has_any_source_word(committed_state) -> !ret)]
pub(super) fn gimfihi_result_panel_is_loading(
    committed_state: &GimfihiWebState,
    result_state: &GimfihiAsyncResultState,
) -> bool {
    let result_current = result_state.state.as_ref() == Some(committed_state);
    gimfihi_state_has_any_source_word(committed_state) && (result_state.loading || !result_current)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn gimfihi_result_summary(output: &GimfihiOutput) -> String {
    format!(
        "{} candidates, {} shown",
        format_integer_count(output.filtered_count),
        format_integer_count(output.candidates.len())
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn format_integer_count(value: usize) -> String {
    let digits = value.to_string();
    let mut output = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            output.push(',');
        }
        output.push(ch);
    }
    output
}

#[requires(value.is_finite())]
#[ensures(!ret.is_empty())]
pub(super) fn format_gimfihi_score(value: f64) -> String {
    trim_gimfihi_float(&format!("{value:.6}"))
}

#[requires(!value.is_empty())]
#[ensures(!ret.is_empty())]
pub(super) fn trim_gimfihi_float(value: &str) -> String {
    let trimmed = value.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gimfihi_rafsis(
    rafsis: &[RafsiCandidate],
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    if rafsis.is_empty() {
        return rsx! {
            span { class: "gimfihi-rafsi-empty",
                { render_page_find_text(page_find, "none") }
            }
        };
    }
    rsx! {
        div { class: "gimfihi-rafsi-list",
            for rafsi in rafsis.iter() {
                { render_gimfihi_rafsi_pill(rafsi, base_path, script, page_find) }
            }
        }
    }
}

#[requires(!rafsi.form.is_empty())]
#[ensures(true)]
pub(super) fn render_gimfihi_rafsi_pill(
    rafsi: &RafsiCandidate,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    match rafsi.availability {
        RafsiAvailability::Free => {
            rsx! {
                span { class: "chip rafsi-chip gimfihi-rafsi-pill is-free",
                    { render_page_find_text(page_find, &rafsi.form) }
                }
            }
        }
        RafsiAvailability::OfficialTaken | RafsiAvailability::ExperimentalTaken => {
            let tone_class = match rafsi.availability {
                RafsiAvailability::OfficialTaken => "is-official-taken",
                RafsiAvailability::ExperimentalTaken => "is-experimental-taken",
                RafsiAvailability::Free => "is-free",
            };
            let sources = if rafsi.taken_by.is_empty() {
                vec![String::new()]
            } else {
                rafsi.taken_by.clone()
            };
            rsx! {
                for source in sources.iter() {
                    span { class: "rafsi-split-pill gimfihi-rafsi-pill {tone_class}",
                        span { class: "rafsi-split-left",
                            { render_page_find_text(page_find, &rafsi.form) }
                        }
                        { render_gimfihi_taken_rafsi_source(source, base_path, script, page_find) }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gimfihi_taken_rafsi_source(
    source: &str,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let Some(source) = (!source.is_empty()).then_some(source) else {
        return rsx! { span { class: "rafsi-split-right" } };
    };
    render_gimfihi_dictionary_word_link_with_host(
        "rafsi-split-right dictionary-tooltip-host",
        Some(source),
        base_path,
        script,
        page_find,
    )
}

#[requires(!host_class.is_empty())]
#[ensures(true)]
pub(super) fn render_gimfihi_dictionary_word_link_with_host(
    host_class: &str,
    word: Option<&str>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let Some(word) = word.filter(|word| !word.is_empty()) else {
        return rsx! {};
    };
    let href = vlacku_word_href(base_path, word);
    let display_word = display_lojban_text(script, word);
    let tooltip = dictionary_tooltip_for_word(base_path, word);
    rsx! {
        span { class: "{host_class}",
            { render_text_route_link_with_page_find("dictionary-word-link", &href, base_path, &display_word, page_find) }
            if let Some(card) = &tooltip {
                { render_dictionary_tooltip(card, true, base_path, script) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gimfihi_dictionary_word_link(
    word: Option<&str>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    render_gimfihi_dictionary_word_link_with_host(
        "dictionary-tooltip-host",
        word,
        base_path,
        script,
        page_find,
    )
}

#[requires(!word.is_empty())]
#[ensures(ret.starts_with(base_path) || base_path.is_empty())]
pub(super) fn vlacku_word_href(base_path: &str, word: &str) -> String {
    vlacku_web_url(
        base_path,
        &VlackuWebState {
            mode: VlackuWebMode::Word,
            query: word.to_owned(),
            count: VLACKU_WEB_DEFAULT_COUNT,
            word_types: Vec::new(),
        },
    )
}
