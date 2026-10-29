use super::*;

#[invariant(true)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReferenceTooltipMode {
    Hover,
    Clicked,
}

// The shell owns the card so that scrolling panels cannot clip it.
#[invariant(true)]
#[derive(Clone)]
pub(super) struct ActiveReferenceTooltip {
    marker: HoveredReference,
    tooltip: ReferenceTooltip,
    base_path: String,
    mode: ReferenceTooltipMode,
}

#[requires(true)]
#[ensures(true)]
pub(super) fn show_reference_tooltip(
    event: &MouseEvent,
    open: Signal<Option<HoveredReference>>,
    marker: &ReferenceMarker,
    tooltip: &ReferenceTooltip,
    base_path: &str,
    mode: ReferenceTooltipMode,
) {
    if change_reference_tooltip(open, marker, tooltip, base_path, mode) {
        mark_reference_tooltip_anchor(event);
        if mode == ReferenceTooltipMode::Clicked {
            focus_reference_tooltip_anchor();
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn change_reference_tooltip(
    open: Signal<Option<HoveredReference>>,
    marker: &ReferenceMarker,
    tooltip: &ReferenceTooltip,
    base_path: &str,
    mode: ReferenceTooltipMode,
) -> bool {
    let Some(mut active) = try_consume_context::<Signal<Option<ActiveReferenceTooltip>>>() else {
        return false;
    };
    let current = active.peek();
    if mode == ReferenceTooltipMode::Hover
        && current
            .as_ref()
            .is_some_and(|active| active.mode == ReferenceTooltipMode::Clicked)
    {
        return false;
    }
    let close = mode == ReferenceTooltipMode::Clicked
        && current.as_ref().is_some_and(|active| {
            active.mode == ReferenceTooltipMode::Clicked
                && active.marker.role == marker.role
                && active.marker.label == marker.label
        });
    drop(current);
    if close {
        close_reference_tooltip(open, false);
        return false;
    }
    active.set(Some(ActiveReferenceTooltip {
        marker: HoveredReference {
            role: marker.role,
            label: marker.label.clone(),
        },
        tooltip: tooltip.clone(),
        base_path: base_path.to_owned(),
        mode,
    }));
    set_reference_tooltip_open(open, marker.role, marker.label.clone());
    true
}

#[requires(true)]
#[ensures(true)]
pub(super) fn leave_reference_tooltip(open: Signal<Option<HoveredReference>>) {
    let Some(active) = try_consume_context::<Signal<Option<ActiveReferenceTooltip>>>() else {
        return;
    };
    let hovered = active
        .peek()
        .as_ref()
        .is_some_and(|active| active.mode == ReferenceTooltipMode::Hover);
    if hovered {
        close_reference_tooltip(open, false);
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn close_reference_tooltip(open: Signal<Option<HoveredReference>>, restore_focus: bool) {
    let was_open = open.peek().is_some();
    if restore_focus && was_open {
        focus_reference_tooltip_anchor();
    }
    if was_open {
        clear_reference_tooltip_open(open);
    }
    if let Some(mut active) = try_consume_context::<Signal<Option<ActiveReferenceTooltip>>>() {
        let has_active = active.peek().is_some();
        if has_active {
            active.set(None);
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn focus_reference_tooltip_card() {
    let Some(mut active) = try_consume_context::<Signal<Option<ActiveReferenceTooltip>>>() else {
        return;
    };
    let Some(mut card) = active.peek().clone() else {
        return;
    };
    card.mode = ReferenceTooltipMode::Clicked;
    active.set(Some(card));
    #[cfg(any(target_arch = "wasm32", feature = "desktop"))]
    spawn(async move {
        let _ = document::eval("document.querySelector('#reference-tooltip-overlay .rich-dictionary-tooltip')?.focus({preventScroll:true});").await;
    });
}

#[cfg(any(target_arch = "wasm32", feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn focus_reference_tooltip_anchor() {
    spawn(async move {
        let _ = document::eval(
            "document.querySelector('[data-reference-tooltip-anchor] .reference-tooltip-trigger')?.focus({preventScroll:true});",
        ).await;
    });
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
fn focus_reference_tooltip_anchor() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn mark_reference_tooltip_anchor(event: &MouseEvent) {
    let Some(target) = event
        .data
        .downcast::<web_sys::MouseEvent>()
        .and_then(|event| event.target())
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
    else {
        return;
    };
    let Ok(Some(host)) = target.closest(".reference-tooltip-host") else {
        return;
    };
    let Some(document) = host.owner_document() else {
        return;
    };
    if let Ok(anchors) = document.query_selector_all("[data-reference-tooltip-anchor]") {
        for index in 0..anchors.length() {
            if let Some(anchor) = anchors
                .item(index)
                .and_then(|node| node.dyn_into::<web_sys::Element>().ok())
            {
                let _ = anchor.remove_attribute("data-reference-tooltip-anchor");
            }
        }
    }
    let _ = host.set_attribute("data-reference-tooltip-anchor", "true");
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn mark_reference_tooltip_anchor(_event: &MouseEvent) {}

#[component]
#[requires(true)]
#[ensures(true)]
pub(super) fn ReferenceTooltipLayer(
    open: Signal<Option<HoveredReference>>,
    route: AppRoute,
    script: GentufaScript,
) -> Element {
    let active = use_context::<Signal<Option<ActiveReferenceTooltip>>>();
    use_effect(move || {
        let _ = active.read();
        let _ = open.read();
        schedule_reference_tooltip_position();
    });
    let active = active.read();
    let open = open.read();
    if route != AppRoute::Gentufa {
        return rsx! {};
    }
    match active.as_ref() {
        Some(active) if open.as_ref() == Some(&active.marker) => {
            let mode = match active.mode {
                ReferenceTooltipMode::Hover => "hover",
                ReferenceTooltipMode::Clicked => "clicked",
            };
            rsx! {
                div { class: "reference-tooltip-interaction", "data-reference-tooltip-mode": mode,
                    { render_reference_tooltip(&active.tooltip, &active.base_path, script) }
                }
            }
        }
        _ => rsx! {},
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
fn schedule_reference_tooltip_position() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let callback = Closure::once_into_js(move || {
        if let Some(document) = web_sys::window().and_then(|window| window.document())
            && let Ok(Some(host)) = document.query_selector("[data-reference-tooltip-anchor]")
        {
            activate_dictionary_tooltip_host(&host);
            position_dictionary_tooltip(&host);
        }
    });
    let _ = window.request_animation_frame(callback.unchecked_ref());
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
fn schedule_reference_tooltip_position() {
    spawn(async move {
        let _ = document::eval(
            "document.dispatchEvent(new Event('jbotci-position-reference-tooltip'));",
        )
        .await;
    });
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
fn schedule_reference_tooltip_position() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn clicked_cards_survive_leave_and_hover_but_toggle_and_dismiss() {
        let mut dom = VirtualDom::new(ShellLayerTest);
        let active =
            dom.in_runtime(|| Signal::new_in_scope(None::<ActiveReferenceTooltip>, ScopeId::ROOT));
        let open = dom.in_runtime(|| Signal::new_in_scope(None::<HoveredReference>, ScopeId::ROOT));
        dom.insert_any_root_context(Box::new(active));
        dom.insert_any_root_context(Box::new(open));
        let tooltip = new!(ReferenceTooltip {
            card: None,
            missing_word: Some("klama".to_owned()),
            highlighted_places: Vec::new(),
            definition: Vec::new(),
            notes: Vec::new(),
            rows: Vec::new(),
        });
        let marker = ReferenceMarker {
            role: ReferenceMarkerRole::Reference,
            kind: ReferenceMarkerKind::Reference,
            label: ReferenceLabel::new("k", None, None),
            source: None,
            tooltip: Some(tooltip.clone()),
        };
        let other = ReferenceMarker {
            label: ReferenceLabel::new("b", None, None),
            ..marker.clone()
        };
        dom.in_scope(ScopeId::ROOT, || {
            assert!(change_reference_tooltip(
                open,
                &marker,
                &tooltip,
                "",
                ReferenceTooltipMode::Hover
            ));
            leave_reference_tooltip(open);
            assert!(open.peek().is_none());
            assert!(change_reference_tooltip(
                open,
                &marker,
                &tooltip,
                "",
                ReferenceTooltipMode::Hover
            ));
            assert!(change_reference_tooltip(
                open,
                &marker,
                &tooltip,
                "",
                ReferenceTooltipMode::Clicked
            ));
            leave_reference_tooltip(open);
            assert!(!change_reference_tooltip(
                open,
                &other,
                &tooltip,
                "",
                ReferenceTooltipMode::Hover
            ));
            assert!(reference_tooltip_matches_marker(&marker, &open.peek()));
            assert!(!change_reference_tooltip(
                open,
                &marker,
                &tooltip,
                "",
                ReferenceTooltipMode::Clicked
            ));
            assert!(open.peek().is_none());
            assert!(change_reference_tooltip(
                open,
                &other,
                &tooltip,
                "",
                ReferenceTooltipMode::Clicked
            ));
            close_reference_tooltip(open, false);
            assert!(open.peek().is_none());
            assert!(active.peek().is_none());
        });
    }

    #[allow(non_snake_case)]
    #[requires(true)]
    #[ensures(true)]
    fn ShellLayerTest() -> Element {
        let open = consume_context::<Signal<Option<HoveredReference>>>();
        rsx! {
            div { id: "shell",
                div { id: "panel", "reference label" }
                ReferenceTooltipLayer { open, route: AppRoute::Gentufa, script: GentufaScript::Latin }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reference_card_stays_outside_the_panel_and_follows_open_state() {
        let marker = HoveredReference {
            role: ReferenceMarkerRole::Reference,
            label: ReferenceLabel::new("k", None, None),
        };
        let presentation = ActiveReferenceTooltip {
            marker: marker.clone(),
            tooltip: new!(ReferenceTooltip {
                card: None,
                missing_word: Some("klama".to_owned()),
                highlighted_places: Vec::new(),
                definition: Vec::new(),
                notes: Vec::new(),
                rows: Vec::new(),
            }),
            base_path: String::new(),
            mode: ReferenceTooltipMode::Hover,
        };
        let mut dom = VirtualDom::new(ShellLayerTest);
        let active = dom.in_runtime(|| Signal::new_in_scope(Some(presentation), ScopeId::ROOT));
        let mut open =
            dom.in_runtime(|| Signal::new_in_scope(None::<HoveredReference>, ScopeId::ROOT));
        dom.insert_any_root_context(Box::new(active));
        dom.insert_any_root_context(Box::new(open));
        dom.rebuild_in_place();
        for state in [
            Some(marker.clone()),
            Some(HoveredReference {
                role: ReferenceMarkerRole::Referent,
                ..marker.clone()
            }),
            Some(marker.clone()),
            None,
        ] {
            let visible = state.as_ref() == Some(&marker);
            dom.in_runtime(|| open.set(state));
            dom.render_immediate_to_vec();
            let html = dioxus::ssr::render(&dom);
            assert_eq!(
                html.contains("id=\"reference-tooltip-overlay\""),
                visible,
                "{html}"
            );
            assert!(!html.contains("popover="), "{html}");
            if visible {
                let panel_end = html.find("</div>").unwrap();
                let card = html.find("id=\"reference-tooltip-overlay\"").unwrap();
                assert!(panel_end < card, "{html}");
                assert!(html.contains("klama"), "{html}");
            }
        }
    }
}
