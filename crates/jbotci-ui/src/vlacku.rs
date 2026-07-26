use super::*;

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub(super) struct VlackuPageSnapshot {
    pub(super) committed_state: VlackuWebState,
    pub(super) result_state: VlackuAsyncResultState,
    pub(super) draft_state: VlackuWebState,
    pub(super) word_type_options: Vec<VlackuWordTypeOption>,
    pub(super) jvozba_available: bool,
    pub(super) jvozba_pane: VlackuJvozbaPaneState,
}

#[requires(true)]
#[ensures(true)]
pub(super) fn vlacku_page_snapshot(
    vlacku_draft_state: Signal<VlackuWebState>,
    vlacku_committed_state: Signal<VlackuWebState>,
    vlacku_result: Signal<VlackuAsyncResultState>,
    jvozba_available: Signal<bool>,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
) -> VlackuPageSnapshot {
    let draft_state = vlacku_draft_state.peek().clone();
    let word_type_options = vlacku_word_type_options(&draft_state.word_types);
    VlackuPageSnapshot {
        committed_state: vlacku_committed_state.read().clone(),
        result_state: vlacku_result.read().clone(),
        draft_state,
        word_type_options,
        jvozba_available: *jvozba_available.read(),
        jvozba_pane: jvozba_pane.read().clone(),
    }
}

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
#[component]
pub(super) fn VlackuPage(
    vlacku_draft_state: Signal<VlackuWebState>,
    vlacku_committed_state: Signal<VlackuWebState>,
    vlacku_result: Signal<VlackuAsyncResultState>,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: Signal<bool>,
    jvozba_drag: Signal<Option<VlackuJvozbaDragState>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    pending_vlacku_scroll_restore: Signal<Option<i32>>,
    base_path: String,
    script: GentufaScript,
    page_find: PageFindContext,
) -> Element {
    let snapshot = use_memo(move || {
        vlacku_page_snapshot(
            vlacku_draft_state,
            vlacku_committed_state,
            vlacku_result,
            jvozba_available,
            jvozba_pane,
        )
    });
    let snapshot = snapshot.read().clone();
    render_vlacku_page(
        vlacku_draft_state,
        vlacku_committed_state,
        &snapshot,
        jvozba_pane,
        jvozba_drag,
        pending_cukta_scroll,
        pending_vlacku_scroll_restore,
        &base_path,
        script,
        &page_find,
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_vlacku_page(
    vlacku_draft_state: Signal<VlackuWebState>,
    vlacku_committed_state: Signal<VlackuWebState>,
    snapshot: &VlackuPageSnapshot,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_drag: Signal<Option<VlackuJvozbaDragState>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    pending_vlacku_scroll_restore: Signal<Option<i32>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let result = if snapshot.result_state.state.as_ref() == Some(&snapshot.committed_state) {
        snapshot.result_state.result.clone()
    } else {
        vlacku_loading_result(&snapshot.committed_state, "Loading dictionary results.")
    };
    let jvozba_open = snapshot.jvozba_available && snapshot.jvozba_pane.open;
    let shell_class = class_names(
        "dictionary-shell",
        &[
            ("dictionary-jvozba-available", snapshot.jvozba_available),
            ("dictionary-jvozba-hints-active", jvozba_open),
        ],
    );
    rsx! {
        section { class: "spa-page dictionary-page vlacku-page",
            h1 { class: "sr-only", "jbotci vlacku" }
            div { class: "{shell_class}",
                { render_vlacku_controls(vlacku_draft_state, vlacku_committed_state, &snapshot.draft_state, &snapshot.word_type_options) }
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
                        { render_vlacku_body(&result, vlacku_draft_state, vlacku_committed_state, jvozba_pane, snapshot.jvozba_available, pending_cukta_scroll, pending_vlacku_scroll_restore, base_path, script, page_find) }
                    }
                    if snapshot.jvozba_available {
                        { render_vlacku_jvozba_pane(jvozba_pane, jvozba_drag, script) }
                    }
                }
            }
        }
    }
}

#[requires(!class_name.is_empty())]
#[ensures(true)]
pub(super) fn render_semantic_search_message(
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
pub(super) fn render_vlacku_controls(
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
pub(super) fn render_vlacku_mode_button(
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
pub(super) fn render_vlacku_word_type_controls(
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
pub(super) fn render_word_type_filter_value(
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
pub(super) fn render_word_type_filter(
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
pub(super) fn render_vlacku_body(
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
pub(super) fn render_dictionary_info(info: &VlackuDictionaryInfo) -> Element {
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
pub(super) fn render_text_route_link(
    class_name: &str,
    href: &str,
    base_path: &str,
    label: &str,
) -> Element {
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
pub(super) fn render_text_route_link_with_page_find(
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
pub(super) fn render_dictionary_count_node(node: &VlackuDictionaryCountNode) -> Element {
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
pub(super) fn render_vlacku_card(
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
pub(super) fn render_dictionary_tooltip(
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
                                let display_surface = display_lujvo_fragment(
                                    script,
                                    &piece.display_surface,
                                    LujvoFragmentKind::Rafsi,
                                );
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
pub(super) fn render_reference_tooltip(
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
pub(super) fn render_reference_dictionary_card(
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
                                let display_surface = display_lujvo_fragment(
                                    script,
                                    &piece.display_surface,
                                    LujvoFragmentKind::Rafsi,
                                );
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
pub(super) fn render_reference_tooltip_inline_spans(
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
pub(super) fn render_reference_tooltip_row(row: &ReferenceTooltipRow) -> Element {
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
pub(super) struct ReferenceTooltipRowViewModel {
    pub(super) slot_text: Option<String>,
    pub(super) target_text: String,
}

#[requires(true)]
#[ensures(ret.target_text == row.target_text)]
pub(super) fn reference_tooltip_row_view_model(
    row: &ReferenceTooltipRow,
) -> ReferenceTooltipRowViewModel {
    new!(ReferenceTooltipRowViewModel {
        slot_text: row.label.slot.as_ref().map(reference_slot_display_text),
        target_text: row.target_text.clone(),
    })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_reference_base_label(label: &ReferenceLabel) -> Element {
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
pub(super) fn render_tooltip_inline_spans(
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
pub(super) fn render_vlacku_headword_line(
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
pub(super) fn render_vlacku_headword_action(
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
pub(super) fn render_vlacku_decomposition_inline(
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
pub(super) fn render_vlacku_inline_separator(text: &str) -> Element {
    rsx! { span { class: "dictionary-word-inline-separator", "{text}" } }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_copy_icon() -> Element {
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
#[requires(true)]
#[ensures(true)]
pub(super) fn copy_text_to_clipboard(text: &str) {
    js_copy_text_to_clipboard(text);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn copy_text_to_clipboard(text: &str) {
    let _ = copy_text_to_clipboard_result(text);
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn copy_text_to_clipboard_result(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    clipboard
        .set_text(text.to_owned())
        .map_err(|error| error.to_string())
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
pub(super) fn copy_text_to_clipboard_result(_text: &str) -> Result<(), String> {
    Err("Native clipboard is not available for this platform yet.".to_owned())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn vlacku_author_credit_text(author: &VlackuWebAuthor) -> String {
    match author.realname.as_deref() {
        Some(realname) if !realname.trim().is_empty() => {
            format!("by {} ({realname})", author.username)
        }
        _ => format!("by {}", author.username),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_vlacku_author_credit(
    author: &VlackuWebAuthor,
    page_find: &PageFindContext,
) -> Element {
    let credit = vlacku_author_credit_text(author);
    rsx! {
        span { class: "dictionary-author-credit",
            { render_page_find_text(page_find, &credit) }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_vlacku_metadata_pill(
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
pub(super) fn render_vlacku_selmaho_segment(
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
pub(super) fn word_type_tag_class(word_type_key: &str) -> String {
    format!(
        "dictionary-meta-segment dictionary-word-type-tag {}",
        vlacku_word_type_tag_class(word_type_key)
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn vlacku_word_type_tag_class(word_type_key: &str) -> &'static str {
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
pub(super) fn render_vlacku_word_action(
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
pub(super) fn render_vote_display(
    votes: &VlackuVoteDisplay,
    page_find: &PageFindContext,
) -> Element {
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
pub(super) fn render_composition_piece(
    piece: &VlackuCompositionPiece,
    jvozba_pane: Signal<VlackuJvozbaPaneState>,
    jvozba_available: bool,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    match piece.kind {
        VlackuCompositionPieceKind::Hyphen => {
            let display_surface = display_lujvo_fragment(
                script,
                &piece.display_surface,
                LujvoFragmentKind::BondingHyphen,
            );
            rsx! {
                span { class: "dictionary-word-inline-separator",
                    { render_page_find_text(page_find, &display_surface) }
                }
            }
        }
        VlackuCompositionPieceKind::Rafsi => {
            let display_surface =
                display_lujvo_fragment(script, &piece.display_surface, LujvoFragmentKind::Rafsi);
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
pub(super) fn render_vlacku_rafsi_add_piece(
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
pub(super) fn render_rafsi_pill(
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
    let display_rafsi = display_lujvo_fragment(script, rafsi, LujvoFragmentKind::Rafsi);
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
pub(super) fn render_inline_spans(
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
pub(super) fn render_vlacku_inline_word_ref(
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
pub(super) fn resolved_href_with_base_path(base_path: &str, href: &str) -> String {
    if href.starts_with('/') {
        format!("{}{}", base_path.trim_end_matches('/'), href)
    } else {
        href.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_vlacku_math(math: &VlackuMath) -> Element {
    rsx! {
        span { class: "spa-cll-math", dangerous_inner_html: "{math.markup}" }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_vlacku_jvozba_pane(
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
pub(super) fn render_jvozba_item(
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
pub(super) fn render_jvozba_item_chip(item: &VlackuJvozbaItem, script: GentufaScript) -> Element {
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
pub(super) fn render_jvozba_output(output: &VlackuJvozbaOutput, script: GentufaScript) -> Element {
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
pub(super) fn vlacku_mode_class(active: bool) -> &'static str {
    if active {
        "dictionary-mode-toggle active"
    } else {
        "dictionary-mode-toggle"
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn vlacku_mode_title(mode: VlackuWebMode, disabled: bool) -> &'static str {
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
pub(super) fn vlacku_query_placeholder(mode: VlackuWebMode) -> &'static str {
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
pub(super) fn toggle_vlacku_word_type(
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
pub(super) fn format_similarity(value: f32) -> String {
    format!("{:.0}%", value * 100.0)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn vote_class(value: &str) -> &'static str {
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
pub(super) fn vote_title(value: &str) -> &'static str {
    if value == "∞" {
        "Official baseline lexicon word. The infinity marker replaces the raw Lensisku community tally for officialdata entries."
    } else {
        "Community upvote/downvote tally from Lensisku contributors."
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parsed_vote_value(value: &str) -> Option<i32> {
    value.trim_start_matches('+').parse().ok()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn vlacku_jvozba_mode_class(active: bool) -> &'static str {
    if active {
        "dictionary-jvozba-mode-toggle active"
    } else {
        "dictionary-jvozba-mode-toggle"
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn jvozba_segment_class(tone: VlackuJvozbaSegmentTone) -> &'static str {
    match tone {
        VlackuJvozbaSegmentTone::RafsiA => "dictionary-jvozba-output-segment is-rafsi-a",
        VlackuJvozbaSegmentTone::RafsiB => "dictionary-jvozba-output-segment is-rafsi-b",
        VlackuJvozbaSegmentTone::Hyphen => "dictionary-jvozba-output-segment is-hyphen",
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn add_vlacku_jvozba_word(
    jvozba_pane: &mut Signal<VlackuJvozbaPaneState>,
    value: String,
) {
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
pub(super) fn add_vlacku_jvozba_rafsi(
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
pub(super) fn set_vlacku_jvozba_mode(
    jvozba_pane: &mut Signal<VlackuJvozbaPaneState>,
    mode: VlackuJvozbaMode,
) {
    let mut next = jvozba_pane.read().clone();
    next.mode = mode;
    set_vlacku_jvozba_pane(jvozba_pane, next);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn move_vlacku_jvozba_item(
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
pub(super) fn remove_vlacku_jvozba_item(
    jvozba_pane: &mut Signal<VlackuJvozbaPaneState>,
    index: usize,
) {
    let mut next = jvozba_pane.read().clone();
    if index < next.items.len() {
        next.items.remove(index);
        set_vlacku_jvozba_pane(jvozba_pane, next);
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn clear_vlacku_jvozba_items(jvozba_pane: &mut Signal<VlackuJvozbaPaneState>) {
    let mut next = jvozba_pane.read().clone();
    next.items.clear();
    set_vlacku_jvozba_pane(jvozba_pane, next);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn start_vlacku_jvozba_drag(
    jvozba_drag: &mut Signal<Option<VlackuJvozbaDragState>>,
    index: usize,
) {
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
pub(super) fn set_vlacku_jvozba_drag_target(
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
pub(super) fn commit_vlacku_jvozba_drag(
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
pub(super) fn finish_vlacku_jvozba_drag(
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
pub(super) fn class_names(base: &str, conditional: &[(&str, bool)]) -> String {
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
pub(super) fn set_vlacku_jvozba_pane(
    jvozba_pane: &mut Signal<VlackuJvozbaPaneState>,
    state: VlackuJvozbaPaneState,
) {
    save_vlacku_jvozba_pane_state(&state);
    jvozba_pane.set(state);
}
