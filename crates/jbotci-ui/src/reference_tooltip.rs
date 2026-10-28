use super::*;

// The shell owns the card so that scrolling panels cannot clip it.
#[invariant(true)]
#[derive(Clone)]
pub(super) struct ActiveReferenceTooltip {
    marker: HoveredReference,
    tooltip: ReferenceTooltip,
    base_path: String,
    script: GentufaScript,
}

#[requires(true)]
#[ensures(true)]
pub(super) fn show_reference_tooltip(
    event: &MouseEvent,
    marker: &ReferenceMarker,
    tooltip: &ReferenceTooltip,
    base_path: &str,
    script: GentufaScript,
) {
    let Some(mut active) = try_consume_context::<Signal<Option<ActiveReferenceTooltip>>>() else {
        return;
    };
    mark_reference_tooltip_anchor(event);
    active.set(Some(ActiveReferenceTooltip {
        marker: HoveredReference {
            role: marker.role,
            label: marker.label.clone(),
        },
        tooltip: tooltip.clone(),
        base_path: base_path.to_owned(),
        script,
    }));
}

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
            render_reference_tooltip(&active.tooltip, &active.base_path, active.script)
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

    #[allow(non_snake_case)]
    #[requires(true)]
    #[ensures(true)]
    fn ShellLayerTest() -> Element {
        let open = consume_context::<Signal<Option<HoveredReference>>>();
        rsx! {
            div { id: "shell",
                div { id: "panel", "reference label" }
                ReferenceTooltipLayer { open, route: AppRoute::Gentufa }
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
            script: GentufaScript::Latin,
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
