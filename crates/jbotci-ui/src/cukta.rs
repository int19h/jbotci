use super::*;

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub(super) struct CuktaPageSnapshot {
    pub(super) page: CuktaPageData,
    pub(super) toc_is_pinned: bool,
    pub(super) toc_is_forced_autohide: bool,
    pub(super) toc_overlay_is_visible: bool,
    pub(super) is_resizing: bool,
    pub(super) current_toc_width: f64,
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cukta_page_snapshot(
    cukta_page: Signal<CuktaAsyncPageState>,
    toc_pinned: Signal<bool>,
    toc_forced_autohide: Signal<bool>,
    toc_overlay_visible: Signal<bool>,
    toc_resize: Signal<Option<CuktaTocResizeState>>,
    toc_width: Signal<f64>,
) -> CuktaPageSnapshot {
    CuktaPageSnapshot {
        page: cukta_page.read().page.clone(),
        toc_is_pinned: *toc_pinned.read(),
        toc_is_forced_autohide: *toc_forced_autohide.read(),
        toc_overlay_is_visible: *toc_overlay_visible.read(),
        is_resizing: toc_resize.read().is_some(),
        current_toc_width: clamp_cukta_toc_width(*toc_width.read()),
    }
}

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
#[component]
pub(super) fn CuktaPage(
    cukta_draft_state: Signal<CuktaWebState>,
    cukta_committed_state: Signal<CuktaWebState>,
    cukta_page: Signal<CuktaAsyncPageState>,
    toc_filter: Signal<String>,
    toc_pinned: Signal<bool>,
    toc_expansion: Signal<CuktaTocExpansionState>,
    toc_width: Signal<f64>,
    toc_resize: Signal<Option<CuktaTocResizeState>>,
    toc_overlay_visible: Signal<bool>,
    toc_forced_autohide: Signal<bool>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: String,
    script: GentufaScript,
    page_find: PageFindContext,
) -> Element {
    let snapshot = use_memo(move || {
        cukta_page_snapshot(
            cukta_page,
            toc_pinned,
            toc_forced_autohide,
            toc_overlay_visible,
            toc_resize,
            toc_width,
        )
    });
    let snapshot = snapshot.read().clone();
    render_cukta_page(
        cukta_draft_state,
        cukta_committed_state,
        &snapshot,
        toc_filter,
        toc_pinned,
        toc_expansion,
        toc_width,
        toc_resize,
        toc_overlay_visible,
        pending_cukta_scroll,
        &base_path,
        script,
        &page_find,
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_cukta_page(
    cukta_draft_state: Signal<CuktaWebState>,
    cukta_committed_state: Signal<CuktaWebState>,
    snapshot: &CuktaPageSnapshot,
    mut toc_filter: Signal<String>,
    mut toc_pinned: Signal<bool>,
    toc_expansion: Signal<CuktaTocExpansionState>,
    toc_width: Signal<f64>,
    mut toc_resize: Signal<Option<CuktaTocResizeState>>,
    mut toc_overlay_visible: Signal<bool>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let toc_is_visible = cukta_toc_panel_visible(
        snapshot.toc_is_pinned,
        snapshot.toc_is_forced_autohide,
        snapshot.toc_overlay_is_visible,
    );
    let toc_uses_autohide = snapshot.toc_is_forced_autohide || !snapshot.toc_is_pinned;
    let toc_button_state = cukta_toc_button_state(
        snapshot.toc_is_pinned,
        snapshot.toc_is_forced_autohide,
        snapshot.toc_overlay_is_visible,
    );
    let toc_button_action = cukta_toc_button_action(toc_button_state);
    let toc_button_title = cukta_toc_button_title(toc_button_state);
    let toc_hides_on_leave = cukta_toc_hides_overlay_on_pointer_leave(
        snapshot.toc_is_pinned,
        snapshot.toc_is_forced_autohide,
    );
    let shell_class = class_names(
        "cll-shell",
        &[
            ("cll-toc-autohide", toc_uses_autohide),
            ("cll-toc-visible", toc_is_visible),
            ("cll-is-resizing", snapshot.is_resizing),
        ],
    );
    let shell_style = format!("--cll-sidebar-width:{:.0}px;", snapshot.current_toc_width);
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
                                for node in snapshot.page.toc.iter() {
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
                        match &snapshot.page.page_kind {
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
pub(super) fn clamp_cukta_toc_width(width: f64) -> f64 {
    width.max(cukta_toc_width_min()).min(cukta_toc_width_max())
}

#[requires(true)]
#[ensures(ret > 0.0)]
pub(super) fn cukta_toc_width_min() -> f64 {
    300.0
}

#[requires(true)]
#[ensures(ret > cukta_toc_width_min())]
pub(super) fn cukta_toc_width_max() -> f64 {
    560.0
}

#[requires(true)]
#[ensures(ret >= cukta_toc_width_min())]
#[ensures(ret <= cukta_toc_width_max())]
pub(super) fn default_cukta_toc_width() -> f64 {
    390.0
}

#[requires(true)]
#[ensures(ret >= cukta_toc_width_min())]
#[ensures(ret <= cukta_toc_width_max())]
pub(super) fn load_cukta_toc_width() -> f64 {
    storage_get("jbotci.cukta.toc.width.v1")
        .and_then(|value| value.parse::<f64>().ok())
        .map(clamp_cukta_toc_width)
        .unwrap_or_else(default_cukta_toc_width)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn load_cukta_toc_pinned() -> bool {
    storage_get("jbotci.cukta.toc.pinned.v1").as_deref() != Some("0")
}

#[requires(true)]
#[ensures(true)]
pub(super) fn load_cukta_toc_expansion() -> CuktaTocExpansionState {
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
pub(super) fn json_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
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
pub(super) fn save_cukta_toc_expansion(state: &CuktaTocExpansionState) {
    let value = serde_json::json!({
        "expanded": &state.expanded,
        "collapsed": &state.collapsed,
    });
    session_storage_set("jbotci.cukta.toc.expansion.v1", &value.to_string());
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_cukta_toc_width(width: &mut Signal<f64>, next_width: f64) {
    let next_width = clamp_cukta_toc_width(next_width);
    storage_set("jbotci.cukta.toc.width.v1", &format!("{next_width:.0}"));
    width.set(next_width);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_cukta_toc_pinned(pinned: &mut Signal<bool>, value: bool) {
    storage_set("jbotci.cukta.toc.pinned.v1", if value { "1" } else { "0" });
    pinned.set(value);
}

#[requires(true)]
#[ensures(ret == ((!forced_autohide && pinned) || overlay_visible))]
pub(super) fn cukta_toc_panel_visible(
    pinned: bool,
    forced_autohide: bool,
    overlay_visible: bool,
) -> bool {
    (!forced_autohide && pinned) || overlay_visible
}

#[requires(true)]
#[ensures(cukta_toc_panel_visible(pinned, forced_autohide, overlay_visible) || ret == CuktaTocButtonState::Hidden)]
pub(super) fn cukta_toc_button_state(
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
pub(super) fn cukta_toc_button_action(state: CuktaTocButtonState) -> CuktaTocButtonAction {
    match state {
        CuktaTocButtonState::Hidden => CuktaTocButtonAction::ShowOverlay,
        CuktaTocButtonState::ForcedAutoHideVisible => CuktaTocButtonAction::HideOverlay,
        CuktaTocButtonState::PinnedVisible => CuktaTocButtonAction::Unpin,
        CuktaTocButtonState::UnpinnedVisible => CuktaTocButtonAction::Pin,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn cukta_toc_button_title(state: CuktaTocButtonState) -> &'static str {
    match state {
        CuktaTocButtonState::Hidden => "Show table of contents",
        CuktaTocButtonState::ForcedAutoHideVisible => "Hide table of contents",
        CuktaTocButtonState::PinnedVisible => "Unpin table of contents",
        CuktaTocButtonState::UnpinnedVisible => "Pin table of contents",
    }
}

#[requires(true)]
#[ensures(ret == (forced_autohide || !pinned))]
pub(super) fn cukta_toc_hides_overlay_on_pointer_leave(
    pinned: bool,
    forced_autohide: bool,
) -> bool {
    forced_autohide || !pinned
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cukta_toc_interaction_after_button_action(
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
pub(super) fn apply_cukta_toc_button_action(
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
pub(super) fn render_cukta_toc_button_icon(state: CuktaTocButtonState) -> Element {
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
pub(super) fn render_cukta_toc_node(
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
pub(super) fn render_cukta_toc_number(number: &str, trailing_dot: bool) -> Element {
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
pub(super) fn render_cukta_toc_title(label: &str) -> Element {
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
pub(super) fn toc_node_expanded(
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
pub(super) fn cukta_toc_node_expanded_with_default(
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
pub(super) fn toggle_cukta_toc_node(
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
pub(super) fn cukta_toc_expansion_with_node_state(
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
pub(super) fn render_cukta_section(
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
pub(super) fn render_cukta_section_pager_link(
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
pub(super) fn render_cukta_index(
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
pub(super) fn render_cukta_search(
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
pub(super) fn render_cukta_search_controls(
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
pub(super) fn render_cukta_mode_button(
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
pub(super) fn render_cukta_target_check(
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
pub(super) fn render_cukta_search_card(
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
pub(super) fn render_cll_block(
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
pub(super) fn render_cll_inline(
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
            let href = cll_inline_href(base_path, *kind, target, CllLinkRenderMode::Web);
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
pub(super) fn display_lojban_text(script: GentufaScript, text: &str) -> String {
    render_lojban_text_for_script(text, script, display_lojban_phoneme_options())
        .unwrap_or_else(|_| text.to_owned())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn display_lujvo_fragment(
    script: GentufaScript,
    text: &str,
    kind: LujvoFragmentKind,
) -> String {
    render_lujvo_fragment_for_script(text, kind, script, display_lojban_phoneme_options())
        .unwrap_or_else(|error| format!("⟨{error}: {text}⟩"))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn display_lojban_text_if(
    script: GentufaScript,
    text: &str,
    lojban_context: bool,
) -> String {
    if lojban_context {
        display_lojban_text(script, text)
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(!matches!(ret.mark_stress, StressMark::Acute | StressMark::Caps))]
#[ensures(ret.mark_glides == GlideMark::Breve)]
pub(super) fn display_lojban_phoneme_options() -> PhonemeRenderOptions {
    PhonemeRenderOptions {
        mark_stress: StressMark::None,
        mark_glides: GlideMark::Breve,
    }
}

#[requires(true)]
#[ensures(ret == language.is_some_and(|language| language.eq_ignore_ascii_case("jbo") || language.eq_ignore_ascii_case("lojban")))]
pub(super) fn cll_language_is_lojban(language: Option<&str>) -> bool {
    language.is_some_and(|language| {
        language.eq_ignore_ascii_case("jbo") || language.eq_ignore_ascii_case("lojban")
    })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cll_link_text_is_lojban(kind: CllLinkKind) -> bool {
    matches!(
        kind,
        CllLinkKind::Dictionary | CllLinkKind::Rafsi | CllLinkKind::Parse
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cll_kind_is_lojban(kind: &str) -> bool {
    matches!(kind, "jbo" | "jbophrase" | "veljvo" | "rafsi")
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cll_display_text_for_kind(script: GentufaScript, kind: &str, text: &str) -> String {
    display_lojban_text_if(script, text, cll_kind_is_lojban(kind))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_cll_interlinear(
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
pub(super) fn render_cll_cmavo_list(
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
pub(super) fn render_cll_lojbanization(
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
pub(super) fn render_cll_lujvo_making(
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
pub(super) fn render_cll_ebnf(
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
pub(super) fn render_cll_ebnf_rhs(
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
pub(super) fn render_cll_ebnf_token(
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
pub(super) fn render_cll_ebnf_elidable(
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
pub(super) fn render_cll_ebnf_link(
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
pub(super) fn render_cll_parse_link(class_name: &str, href: &str, base_path: &str) -> Element {
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
pub(super) fn cll_dictionary_tooltip_for_link(
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
pub(super) fn cll_dictionary_tooltip_for_href(
    base_path: &str,
    href: &str,
) -> Option<DictionaryTooltipCard> {
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
pub(super) fn cll_ebnf_elidable_hash_pieces(body: &str) -> Option<(String, String)> {
    let inner = body.strip_prefix('/')?.strip_suffix('/')?;
    let inner_without_hash = inner.strip_suffix('#')?;
    Some((format!("/{inner_without_hash}"), "/".to_owned()))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn cll_table_class(classes: &[String]) -> String {
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
pub(super) fn cll_language_span_class(kind: CllLanguageSpanKind) -> &'static str {
    match kind {
        CllLanguageSpanKind::ForeignPhrase => "spa-cll-foreignphrase",
        CllLanguageSpanKind::JboPhrase => "spa-cll-jbophrase",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn cll_link_kind_class(kind: CllLinkKind) -> &'static str {
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
pub(super) fn cll_table_row_parse_class(row: &[CllTableCell]) -> String {
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
pub(super) fn cll_table_row_parse_group_id(row: &[CllTableCell]) -> Option<String> {
    cll_table_row_parse_group(row).map(|group| group.group_id.clone())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cll_table_row_parse_group(
    row: &[CllTableCell],
) -> Option<&jbotci_cll::CllTableParseGroup> {
    row.first().and_then(|cell| cell.parse_group.as_ref())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn cll_table_cell_parse_link_class(cell: &CllTableCell) -> String {
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
pub(super) fn cll_parse_href(base_path: &str, href: &str) -> String {
    if let Some(query) = href.strip_prefix("../gentufa") {
        format!("{}/gentufa{query}", base_path.trim_end_matches('/'))
    } else {
        href.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cll_ebnf_href(base_path: &str, href: &str) -> String {
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
#[requires(link_mode == CllLinkRenderMode::Web)]
#[ensures(true)]
pub(super) fn cll_inline_href(
    base_path: &str,
    kind: CllLinkKind,
    target: &str,
    link_mode: CllLinkRenderMode,
) -> String {
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
pub(super) fn cukta_section_reference_from_href(href: &str) -> Option<String> {
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
pub(super) fn cukta_anchor_from_href(href: &str) -> Option<String> {
    href.split_once('#')
        .map(|(_, anchor)| anchor)
        .filter(|anchor| !anchor.is_empty())
        .map(str::to_owned)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn scroll_to_cukta_href(href: &str) {
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
pub(super) fn scroll_to_cukta_href(href: &str) {
    let _ = href;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn save_cukta_toc_scroll() {
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
pub(super) fn save_cukta_toc_scroll() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn restore_cukta_toc_scroll() {
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
pub(super) fn restore_cukta_toc_scroll() {}

#[requires(true)]
#[ensures(true)]
pub(super) fn cll_asset_href(base_path: &str, src: &str) -> String {
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
pub(super) fn cll_known_media_href(file_name: &str) -> Option<String> {
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
pub(super) fn cukta_search_draft_for_page(
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
pub(super) fn cukta_draft_mode_options(selected: CuktaWebMode) -> Vec<CuktaModeOption> {
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
pub(super) fn cukta_draft_target_options(
    selected_targets: &[CuktaSearchTarget],
) -> Vec<CuktaTargetOption> {
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
pub(super) fn cukta_search_state_with_query(
    state: &CuktaWebSearchState,
    query: &str,
) -> CuktaWebSearchState {
    CuktaWebSearchState {
        mode: state.mode,
        query: query.to_owned(),
        count: CUKTA_WEB_DEFAULT_COUNT,
        targets: state.targets.clone(),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_cukta_state_immediate(
    mut draft_state: Signal<CuktaWebState>,
    mut committed_state: Signal<CuktaWebState>,
    state: CuktaWebState,
) {
    clear_cukta_search_timer();
    draft_state.set(state.clone());
    committed_state.set(state);
}
