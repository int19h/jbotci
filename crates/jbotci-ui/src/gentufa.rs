use super::*;

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gentufa_input(
    mut input_text: Signal<String>,
    result: &GentufaWebResult,
    request: Option<&GentufaWebRequest>,
    active_diagnostic: Option<usize>,
    mut active_diagnostic_signal: Signal<Option<usize>>,
    mut diagnostic_tooltip: Signal<Option<DiagnosticInputTooltip>>,
    diagnostic_tooltip_value: Option<DiagnosticInputTooltip>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
) -> Element {
    let text = input_text.read().clone();
    let content_sizer_text = gentufa_textarea_content_sizer_text(&text);
    let diagnostics = current_gentufa_input_diagnostics(&text, result, request);
    rsx! {
        div { class: "gentufa-input-editor",
            div { class: "gentufa-text-sizer", aria_hidden: "true", "{content_sizer_text}" }
            div { class: "gentufa-text-sizer", aria_hidden: "true", "{DEFAULT_GENTUFA_TEXT}" }
            { render_gentufa_diagnostic_overlay(
                &text,
                diagnostics,
                active_diagnostic,
                diagnostic_tooltip,
            ) }
            textarea {
                id: "gentufa-text",
                aria_label: "Lojban text",
                placeholder: "{DEFAULT_GENTUFA_TEXT}",
                value: "{text}",
                spellcheck: "false",
                oninput: move |event| {
                    input_text.set(event.value());
                    active_diagnostic_signal.set(None);
                    diagnostic_tooltip.set(None);
                    schedule_gentufa_textarea_resize();
                },
            }
            { render_gentufa_diagnostic_input_tooltip(
                diagnostic_tooltip_value,
                diagnostics,
                &text,
                active_diagnostic_signal,
                pending_cukta_scroll,
                base_path,
                script,
            ) }
        }
    }
}

#[requires(true)]
#[ensures(!source.ends_with('\n') || ret.ends_with(' '))]
#[ensures(source.ends_with('\n') || ret == source)]
pub(super) fn gentufa_textarea_content_sizer_text(source: &str) -> String {
    if source.ends_with('\n') {
        format!("{source} ")
    } else {
        source.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gentufa_diagnostic_overlay(
    text: &str,
    diagnostics: &[Diagnostic],
    active_diagnostic: Option<usize>,
    diagnostic_tooltip: Signal<Option<DiagnosticInputTooltip>>,
) -> Element {
    if diagnostics.is_empty() {
        return rsx! {};
    }
    let fragments = diagnostic_overlay_fragments(text, diagnostics, active_diagnostic);
    rsx! {
        div { class: "gentufa-text-overlay", aria_hidden: "true",
            for fragment in fragments.iter() {
                { render_gentufa_diagnostic_overlay_fragment(
                    fragment,
                    diagnostic_tooltip,
                ) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gentufa_diagnostic_overlay_fragment(
    fragment: &DiagnosticOverlayFragment,
    diagnostic_tooltip: Signal<Option<DiagnosticInputTooltip>>,
) -> Element {
    let diagnostic_index = fragment.diagnostic_index;
    let mut over_tooltip = diagnostic_tooltip;
    let mut enter_tooltip = diagnostic_tooltip;
    let mut move_tooltip = diagnostic_tooltip;
    let mut out_tooltip = diagnostic_tooltip;
    let mut leave_tooltip = diagnostic_tooltip;
    rsx! {
        span {
            class: "{fragment.class_name}",
            "data-selection-start": "{fragment.selection_start}",
            onmouseover: move |event| {
                if let Some(diagnostic_index) = diagnostic_index {
                    let coordinates = event.data().client_coordinates();
                    over_tooltip.set(Some(new!(DiagnosticInputTooltip {
                        diagnostic_index,
                        x: coordinates.x,
                        y: coordinates.y,
                    })));
                }
            },
            onmouseenter: move |event| {
                if let Some(diagnostic_index) = diagnostic_index {
                    let coordinates = event.data().client_coordinates();
                    enter_tooltip.set(Some(new!(DiagnosticInputTooltip {
                        diagnostic_index,
                        x: coordinates.x,
                        y: coordinates.y,
                    })));
                }
            },
            onmousemove: move |event| {
                if let Some(diagnostic_index) = diagnostic_index {
                    let coordinates = event.data().client_coordinates();
                    move_tooltip.set(Some(new!(DiagnosticInputTooltip {
                        diagnostic_index,
                        x: coordinates.x,
                        y: coordinates.y,
                    })));
                }
            },
            onmouseout: move |_| {
                if diagnostic_index.is_some() {
                    out_tooltip.set(None);
                }
            },
            onmouseleave: move |_| {
                if diagnostic_index.is_some() {
                    leave_tooltip.set(None);
                }
            },
            onmousedown: move |event| {
                event.prevent_default();
                let coordinates = event.data().client_coordinates();
                place_gentufa_textarea_caret_from_overlay_click(
                    coordinates.x,
                    coordinates.y,
                );
            },
            "{fragment.text}"
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gentufa_diagnostic_input_tooltip(
    tooltip: Option<DiagnosticInputTooltip>,
    diagnostics: &[Diagnostic],
    source: &str,
    active_diagnostic: Signal<Option<usize>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
) -> Element {
    let Some(tooltip) = tooltip else {
        return rsx! {};
    };
    let Some(diagnostic) = diagnostics.get(tooltip.diagnostic_index) else {
        return rsx! {};
    };
    let style = format!(
        "--diagnostic-tooltip-x: {:.2}px; --diagnostic-tooltip-y: {:.2}px;",
        tooltip.x, tooltip.y
    );
    rsx! {
        div { class: "gentufa-diagnostic-input-tooltip", style: "{style}",
            { render_diagnostic_card(
                tooltip.diagnostic_index,
                diagnostic,
                source,
                active_diagnostic,
                pending_cukta_scroll,
                base_path,
                script,
                None,
            ) }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn place_gentufa_textarea_caret_from_overlay_click(x: f64, y: f64) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(selection_start) = diagnostic_overlay_selection_offset_from_point(&document, x, y)
    else {
        return;
    };
    let Some(textarea) = document
        .get_element_by_id("gentufa-text")
        .and_then(|element| element.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
    else {
        return;
    };
    let _ = textarea.focus();
    let _ = textarea.set_selection_start(Some(selection_start));
    let _ = textarea.set_selection_end(Some(selection_start));
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn place_gentufa_textarea_caret_from_overlay_click(_x: f64, _y: f64) {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_overlay_selection_offset_from_point(
    document: &web_sys::Document,
    x: f64,
    y: f64,
) -> Option<u32> {
    diagnostic_overlay_caret_position_offset_from_point(document, x, y)
        .or_else(|| diagnostic_overlay_caret_range_offset_from_point(document, x, y))
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_overlay_caret_position_offset_from_point(
    document: &web_sys::Document,
    x: f64,
    y: f64,
) -> Option<u32> {
    let document_value = document.as_ref();
    let function = js_sys::Reflect::get(
        document_value,
        &wasm_bindgen::JsValue::from_str("caretPositionFromPoint"),
    )
    .ok()?
    .dyn_into::<js_sys::Function>()
    .ok()?;
    let position = function
        .call2(
            document_value,
            &wasm_bindgen::JsValue::from_f64(x),
            &wasm_bindgen::JsValue::from_f64(y),
        )
        .ok()?;
    if position.is_null() || position.is_undefined() {
        return None;
    }
    let node = js_sys::Reflect::get(&position, &wasm_bindgen::JsValue::from_str("offsetNode"))
        .ok()?
        .dyn_into::<web_sys::Node>()
        .ok()?;
    let offset = js_sys::Reflect::get(&position, &wasm_bindgen::JsValue::from_str("offset"))
        .ok()?
        .as_f64()? as u32;
    diagnostic_overlay_selection_offset_from_node_offset(node, offset)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_overlay_caret_range_offset_from_point(
    document: &web_sys::Document,
    x: f64,
    y: f64,
) -> Option<u32> {
    let document_value = document.as_ref();
    let function = js_sys::Reflect::get(
        document_value,
        &wasm_bindgen::JsValue::from_str("caretRangeFromPoint"),
    )
    .ok()?
    .dyn_into::<js_sys::Function>()
    .ok()?;
    let range = function
        .call2(
            document_value,
            &wasm_bindgen::JsValue::from_f64(x),
            &wasm_bindgen::JsValue::from_f64(y),
        )
        .ok()?;
    if range.is_null() || range.is_undefined() {
        return None;
    }
    let node = js_sys::Reflect::get(&range, &wasm_bindgen::JsValue::from_str("startContainer"))
        .ok()?
        .dyn_into::<web_sys::Node>()
        .ok()?;
    let offset = js_sys::Reflect::get(&range, &wasm_bindgen::JsValue::from_str("startOffset"))
        .ok()?
        .as_f64()? as u32;
    diagnostic_overlay_selection_offset_from_node_offset(node, offset)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_overlay_selection_offset_from_node_offset(
    node: web_sys::Node,
    offset: u32,
) -> Option<u32> {
    let mut element = node
        .dyn_ref::<web_sys::Element>()
        .cloned()
        .or_else(|| node.parent_element());
    while let Some(current) = element {
        if let Some(start) = current
            .get_attribute("data-selection-start")
            .and_then(|value| value.parse::<u32>().ok())
        {
            return Some(start.saturating_add(offset));
        }
        element = current.parent_element();
    }
    None
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_result(
    result: &GentufaWebResult,
    request: Option<&GentufaWebRequest>,
    diagnostics_open: Signal<bool>,
    diagnostics_open_value: bool,
    active_diagnostic: Signal<Option<usize>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    view_mode: Signal<GentufaWebViewMode>,
    view_mode_value: GentufaWebViewMode,
    display: Signal<GentufaDisplayState>,
    display_value: GentufaDisplayState,
    settings_value: UserSettings,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    activity: Signal<AsyncActivityState>,
    export_task: Signal<Option<LatestAsyncTask>>,
    page_find: &PageFindContext,
) -> Element {
    match result {
        GentufaWebResult::Blank => rsx! {},
        GentufaWebResult::Error(error) => render_error(
            error,
            request,
            diagnostics_open,
            diagnostics_open_value,
            active_diagnostic,
            pending_cukta_scroll,
            base_path,
            settings_value.script,
            page_find,
        ),
        GentufaWebResult::Success(success) => render_success(
            success,
            request,
            diagnostics_open,
            diagnostics_open_value,
            active_diagnostic,
            pending_cukta_scroll,
            base_path,
            view_mode,
            view_mode_value,
            display,
            display_value,
            settings_value,
            reference_hover,
            reference_tooltip_open,
            activity,
            export_task,
            page_find,
        ),
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_error(
    error: &GentufaError,
    request: Option<&GentufaWebRequest>,
    diagnostics_open: Signal<bool>,
    diagnostics_open_value: bool,
    active_diagnostic: Signal<Option<usize>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let source = gentufa_request_source(request);
    rsx! {
        section { class: "result-section error-section",
            { render_diagnostics_pane(
                &error.diagnostics,
                source,
                Some(error.message.as_str()),
                diagnostics_open,
                diagnostics_open_value,
                active_diagnostic,
                pending_cukta_scroll,
                base_path,
                script,
                Some(page_find),
            ) }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_success(
    success: &GentufaSuccess,
    request: Option<&GentufaWebRequest>,
    diagnostics_open: Signal<bool>,
    diagnostics_open_value: bool,
    active_diagnostic: Signal<Option<usize>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    view_mode: Signal<GentufaWebViewMode>,
    view_mode_value: GentufaWebViewMode,
    display: Signal<GentufaDisplayState>,
    display_value: GentufaDisplayState,
    settings_value: UserSettings,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    activity: Signal<AsyncActivityState>,
    export_task: Signal<Option<LatestAsyncTask>>,
    page_find: &PageFindContext,
) -> Element {
    let reference_hover_value = reference_hover.read().clone();
    let source = gentufa_request_source(request);
    rsx! {
        section { class: "result-section",
            { render_reference_overlay(&reference_hover_value) }
            { render_surface_output(success, settings_value.script, page_find) }
            { render_diagnostics_pane(
                &success.diagnostics,
                source,
                None,
                diagnostics_open,
                diagnostics_open_value,
                active_diagnostic,
                pending_cukta_scroll,
                base_path,
                settings_value.script,
                Some(page_find),
            ) }
            div { class: "view-toolbar",
                { render_view_tabs(view_mode, view_mode_value) }
                { render_output_controls(display, display_value) }
            }
            match view_mode_value {
                GentufaWebViewMode::Blocks => rsx! {
                    { render_blocks(success, display_value.show_glosses, settings_value.script, reference_hover, reference_tooltip_open, activity, export_task, page_find) }
                },
                GentufaWebViewMode::Tree => rsx! {
                    { render_tree(success, reference_hover, reference_tooltip_open, settings_value.script, page_find) }
                },
                GentufaWebViewMode::Ipa => rsx! {
                    { render_ipa_output(success, page_find) }
                },
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_surface_output(
    success: &GentufaSuccess,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        div { class: "brackets-section",
            div { class: "brackets-output-stack",
                pre { class: "brackets-output compact-output",
                    span { class: "brackets-output-markup",
                        for fragment in success.bracket_fragments.iter() {
                            { render_bracket_fragment(fragment, script, page_find) }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_bracket_fragment(
    fragment: &GentufaBracketFragment,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    match fragment {
        GentufaBracketFragment::Text { text, elided } => {
            if *elided {
                rsx! { s { { render_page_find_text(page_find, text) } } }
            } else {
                render_page_find_text(page_find, text)
            }
        }
        GentufaBracketFragment::Span {
            color,
            href,
            tooltip,
            children,
        } => {
            let style = color
                .as_ref()
                .map(|color| format!("color: {color};"))
                .unwrap_or_default();
            if let Some(href) = href {
                let base_path = router_base_path();
                let route = jbotci_route_from_href(&base_path, href);
                if let Some(card) = tooltip {
                    rsx! {
                        span {
                            class: "bracket-fragment bracket-word dictionary-tooltip-host",
                            style: "{style}",
                            if let Some(route) = route {
                                Link { class: "bracket-word-link", to: route,
                                    for child in children.iter() {
                                        { render_bracket_fragment(child, script, page_find) }
                                    }
                                }
                            } else {
                                a { class: "bracket-word-link", href: "{href}",
                                    for child in children.iter() {
                                        { render_bracket_fragment(child, script, page_find) }
                                    }
                                }
                            }
                            { render_dictionary_tooltip(card, false, &base_path, script) }
                        }
                    }
                } else {
                    if let Some(route) = route {
                        rsx! {
                            Link {
                                class: "bracket-fragment bracket-word",
                                style: "{style}",
                                to: route,
                                for child in children.iter() {
                                    { render_bracket_fragment(child, script, page_find) }
                                }
                            }
                        }
                    } else {
                        rsx! {
                            a {
                                class: "bracket-fragment bracket-word",
                                style: "{style}",
                                href: "{href}",
                                for child in children.iter() {
                                    { render_bracket_fragment(child, script, page_find) }
                                }
                            }
                        }
                    }
                }
            } else {
                rsx! {
                    span { class: "bracket-fragment", style: "{style}",
                        for child in children.iter() {
                            { render_bracket_fragment(child, script, page_find) }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_diagnostics_pane(
    diagnostics: &[Diagnostic],
    source: &str,
    fallback_error: Option<&str>,
    mut diagnostics_open: Signal<bool>,
    diagnostics_open_value: bool,
    active_diagnostic: Signal<Option<usize>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: Option<&PageFindContext>,
) -> Element {
    let fallback_error = fallback_error.filter(|message| !message.is_empty());
    if diagnostics.is_empty() && fallback_error.is_none() {
        return rsx! {};
    }
    let counts = diagnostic_counts(diagnostics, fallback_error);
    let title = diagnostic_pane_title(counts);
    let toggle_label = diagnostics_toggle_label(diagnostics_open_value);
    rsx! {
        section { class: "gentufa-diagnostics-pane", role: "alert", aria_live: "polite",
            div { class: "gentufa-diagnostics-header",
                h2 { class: "gentufa-diagnostics-title",
                    { render_optional_page_find_text(page_find, &title) }
                }
                button {
                    class: "gentufa-diagnostics-toggle",
                    r#type: "button",
                    aria_expanded: if diagnostics_open_value { "true" } else { "false" },
                    onclick: move |_| diagnostics_open.set(!diagnostics_open_value),
                    { render_optional_page_find_text(page_find, toggle_label) }
                }
            }
            if diagnostics_open_value {
                div { class: "gentufa-diagnostics-list",
                    if diagnostics.is_empty() {
                        if let Some(message) = fallback_error {
                            article { class: "gentufa-diagnostic-card is-error",
                                div { class: "gentufa-diagnostic-main",
                                    span { class: "gentufa-diagnostic-severity",
                                        { render_optional_page_find_text(page_find, "error") }
                                    }
                                    span { class: "gentufa-diagnostic-message",
                                        { render_optional_page_find_text(page_find, message) }
                                    }
                                }
                            }
                        }
                    } else {
                        for (index, diagnostic) in diagnostics.iter().enumerate() {
                            { render_diagnostic_card(
                                index,
                                diagnostic,
                                source,
                                active_diagnostic,
                                pending_cukta_scroll,
                                base_path,
                                script,
                                page_find,
                            ) }
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_diagnostic_card(
    index: usize,
    diagnostic: &Diagnostic,
    source: &str,
    active_diagnostic: Signal<Option<usize>>,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: Option<&PageFindContext>,
) -> Element {
    let mut enter_active = active_diagnostic;
    let mut leave_active = active_diagnostic;
    let card_class = diagnostic_card_class(diagnostic);
    let primary = diagnostic.primary_label();
    let location = diagnostic_label_location(source, primary);
    let location_text = format!(
        "{}:{}: {}",
        location.line, location.column, diagnostic.message
    );
    let context_labels = diagnostic_context_labels(diagnostic);
    let styled_notes = diagnostic_styled_notes_for_web(diagnostic);
    let plain_notes = diagnostic_plain_note_segments_for_web(diagnostic);
    let primary_detail_segments = diagnostic_primary_detail_parts(diagnostic);
    rsx! {
        article {
            class: "{card_class}",
            onmouseenter: move |_| enter_active.set(Some(index)),
            onmouseleave: move |_| leave_active.set(None),
            div { class: "gentufa-diagnostic-main",
                span { class: "gentufa-diagnostic-severity",
                    { render_optional_page_find_text(page_find, diagnostic_severity_text(diagnostic.severity)) }
                }
                code { class: "gentufa-diagnostic-code",
                    { render_optional_page_find_text(page_find, &diagnostic.code) }
                }
                span { class: "gentufa-diagnostic-message",
                    { render_optional_page_find_text(page_find, &location_text) }
                }
            }
            for label in context_labels {
                { render_diagnostic_context_label(label, page_find) }
            }
            if !primary_detail_segments.is_empty() {
                div { class: "gentufa-diagnostic-primary-detail",
                    for segment in primary_detail_segments.iter() {
                        { render_diagnostic_text_part(segment, pending_cukta_scroll, base_path, script, page_find) }
                    }
                }
            }
            if !plain_notes.is_empty() || !styled_notes.is_empty() {
                div { class: "gentufa-diagnostic-notes",
                    for note in plain_notes.iter() {
                        div { class: "gentufa-diagnostic-note",
                            for segment in note.iter() {
                                { render_diagnostic_text_part(segment, pending_cukta_scroll, base_path, script, page_find) }
                            }
                        }
                    }
                    for note in styled_notes {
                        { render_styled_diagnostic_note(note, pending_cukta_scroll, base_path, script, page_find) }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_styled_diagnostic_note(
    note: &DiagnosticStyledNote,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: Option<&PageFindContext>,
) -> Element {
    let class_name = diagnostic_styled_note_class(note);
    rsx! {
        div { class: "{class_name}",
            for segment in note.segments.iter() {
                { render_diagnostic_note_segment(segment, pending_cukta_scroll, base_path, script, page_find) }
            }
        }
    }
}

#[requires(!segment.text.is_empty())]
#[ensures(true)]
pub(super) fn render_diagnostic_note_segment(
    segment: &DiagnosticTextSegment,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: Option<&PageFindContext>,
) -> Element {
    let parts = diagnostic_text_segment_render_parts(std::slice::from_ref(segment));
    rsx! {
        for part in parts.iter() {
            { render_diagnostic_text_part(part, pending_cukta_scroll, base_path, script, page_find) }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn current_gentufa_input_diagnostics<'a>(
    input_text: &str,
    result: &'a GentufaWebResult,
    request: Option<&GentufaWebRequest>,
) -> &'a [Diagnostic] {
    if diagnostics_decorate_current_input(input_text, request) {
        gentufa_result_diagnostics(result)
    } else {
        &[]
    }
}

#[requires(true)]
#[ensures(ret -> request.is_some())]
pub(super) fn diagnostics_decorate_current_input(
    input_text: &str,
    request: Option<&GentufaWebRequest>,
) -> bool {
    request.is_some_and(|request| request.text == input_text)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gentufa_result_diagnostics(result: &GentufaWebResult) -> &[Diagnostic] {
    match result {
        GentufaWebResult::Blank => &[],
        GentufaWebResult::Success(success) => &success.diagnostics,
        GentufaWebResult::Error(error) => &error.diagnostics,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gentufa_request_source(request: Option<&GentufaWebRequest>) -> &str {
    request.map_or("", |request| request.text.as_str())
}

#[requires(true)]
#[ensures(ret.errors + ret.warnings >= diagnostics.len() || fallback_error.is_some())]
pub(super) fn diagnostic_counts(
    diagnostics: &[Diagnostic],
    fallback_error: Option<&str>,
) -> DiagnosticCounts {
    if diagnostics.is_empty() && fallback_error.is_some() {
        return new!(DiagnosticCounts {
            errors: 1,
            warnings: 0,
        });
    }
    let mut errors = 0;
    let mut warnings = 0;
    for diagnostic in diagnostics {
        match diagnostic.severity {
            DiagnosticSeverity::Error => errors += 1,
            DiagnosticSeverity::Warning | DiagnosticSeverity::Advice => warnings += 1,
        }
    }
    new!(DiagnosticCounts { errors, warnings })
}

#[requires(true)]
#[ensures(ret.contains("Diagnostics"))]
pub(super) fn diagnostic_pane_title(counts: DiagnosticCounts) -> String {
    format!(
        "Diagnostics: {}, {}",
        plural_count(counts.errors, "error", "errors"),
        plural_count(counts.warnings, "warning", "warnings")
    )
}

#[requires(!singular.is_empty())]
#[requires(!plural.is_empty())]
#[ensures(!ret.is_empty())]
pub(super) fn plural_count(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostics_toggle_label(open: bool) -> &'static str {
    if open { "Hide" } else { "Show" }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_card_class(diagnostic: &Diagnostic) -> String {
    class_names(
        "gentufa-diagnostic-card",
        &[
            ("is-error", diagnostic.severity == DiagnosticSeverity::Error),
            (
                "is-warning",
                diagnostic.severity != DiagnosticSeverity::Error,
            ),
        ],
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_severity_text(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Advice => "advice",
    }
}

#[requires(true)]
#[ensures(ret.line > 0)]
#[ensures(ret.column > 0)]
pub(super) fn diagnostic_label_location(
    source: &str,
    label: &DiagnosticLabel,
) -> DiagnosticSourceLocation {
    source_location_for_char_offset(source, label.span.char_start)
}

#[requires(true)]
#[ensures(ret.line > 0)]
#[ensures(ret.column > 0)]
pub(super) fn source_location_for_char_offset(
    source: &str,
    char_offset: usize,
) -> DiagnosticSourceLocation {
    let mut line = 1;
    let mut column = 1;
    for (index, character) in source.chars().enumerate() {
        if index == char_offset {
            return new!(DiagnosticSourceLocation { line, column });
        }
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    new!(DiagnosticSourceLocation { line, column })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_primary_detail_parts(
    diagnostic: &Diagnostic,
) -> Vec<DiagnosticTextRenderPart> {
    let label = diagnostic.primary_label();
    if label.message != diagnostic.message && label.message.starts_with("expected:") {
        return diagnostic_text_segment_render_parts(&label.message_segments);
    }
    diagnostic_expected_detail_parts_from_detailed_note(diagnostic).unwrap_or_else(|| {
        if label.message != diagnostic.message {
            diagnostic_text_segment_render_parts(&label.message_segments)
        } else {
            Vec::new()
        }
    })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_primary_detail_display_text(diagnostic: &Diagnostic) -> Option<String> {
    let parts = diagnostic_primary_detail_parts(diagnostic);
    (!parts.is_empty()).then(|| diagnostic_text_parts_text(&parts))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|parts| !parts.is_empty()))]
pub(super) fn diagnostic_expected_detail_parts_from_detailed_note(
    diagnostic: &Diagnostic,
) -> Option<Vec<DiagnosticTextRenderPart>> {
    let note = diagnostic.styled_notes.iter().find(|note| {
        matches!(note.mode, jbotci_diagnostics::DiagnosticNoteMode::Detailed)
            && diagnostic_styled_note_text(note)
                .trim_start()
                .starts_with("needs one of:")
    })?;
    let mut output = vec![
        diagnostic_text_render_part(DiagnosticTextRole::Keyword, "expected".to_owned(), None),
        diagnostic_text_render_part(DiagnosticTextRole::Plain, " ".to_owned(), None),
    ];
    let mut heading_seen = false;
    let mut skipping_heading_tail = false;
    let mut at_line_start = true;
    let mut pending_separator = false;
    let mut content_started = false;

    for segment in &note.segments {
        for part in diagnostic_text_segment_render_parts(std::slice::from_ref(segment)) {
            let mut index = 0usize;
            if !heading_seen {
                if part.role == DiagnosticTextRole::Keyword && part.text == "needs one of" {
                    heading_seen = true;
                    skipping_heading_tail = true;
                }
                continue;
            }
            while index < part.text.len() {
                let Some(character) = part.text[index..].chars().next() else {
                    break;
                };
                if skipping_heading_tail {
                    index += character.len_utf8();
                    if character == '\n' {
                        skipping_heading_tail = false;
                        at_line_start = true;
                    }
                    continue;
                }
                if character == '\n' {
                    index += character.len_utf8();
                    if content_started {
                        pending_separator = true;
                    }
                    at_line_start = true;
                    continue;
                }
                if at_line_start {
                    if character.is_whitespace() {
                        index += character.len_utf8();
                        continue;
                    }
                    if character == '-' {
                        index += character.len_utf8();
                        if index < part.text.len()
                            && part.text[index..]
                                .chars()
                                .next()
                                .is_some_and(char::is_whitespace)
                        {
                            let next = part.text[index..]
                                .chars()
                                .next()
                                .expect("checked above that a character is present");
                            index += next.len_utf8();
                        }
                        continue;
                    }
                    if pending_separator {
                        output.push(diagnostic_text_render_part(
                            DiagnosticTextRole::Punctuation,
                            ", ".to_owned(),
                            None,
                        ));
                        pending_separator = false;
                    }
                    at_line_start = false;
                }
                let start = index;
                index += character.len_utf8();
                while index < part.text.len() {
                    let next = part.text[index..]
                        .chars()
                        .next()
                        .expect("index is inside the current text part");
                    if next == '\n' {
                        break;
                    }
                    index += next.len_utf8();
                }
                output.push(diagnostic_text_render_part(
                    part.role,
                    part.text[start..index].to_owned(),
                    part.link.clone(),
                ));
                content_started = true;
            }
        }
    }

    if heading_seen && content_started {
        Some(merge_diagnostic_text_parts(output))
    } else {
        None
    }
}

#[requires(true)]
#[ensures(ret.iter().all(|label| !label.primary))]
pub(super) fn diagnostic_context_labels(diagnostic: &Diagnostic) -> Vec<&DiagnosticLabel> {
    diagnostic
        .labels
        .iter()
        .filter(|label| !label.primary)
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_diagnostic_context_label(
    label: &DiagnosticLabel,
    page_find: Option<&PageFindContext>,
) -> Element {
    let descriptor = diagnostic_context_descriptor(&label.message);
    rsx! {
        div { class: "gentufa-diagnostic-context",
            em {
                if let Some(descriptor) = descriptor {
                    { render_optional_page_find_text(page_find, "while parsing ") }
                    span { class: "gentufa-diagnostic-context-descriptor",
                        { render_optional_page_find_text(page_find, descriptor) }
                    }
                } else {
                    { render_optional_page_find_text(page_find, &label.message) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_context_descriptor(message: &str) -> Option<&str> {
    message.strip_prefix("while parsing ")
}

#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_plain_note_segments_for_web(
    diagnostic: &Diagnostic,
) -> Vec<Vec<DiagnosticTextRenderPart>> {
    diagnostic
        .notes
        .iter()
        .enumerate()
        .filter(|(_, note)| !note.is_empty() && !diagnostic_plain_note_is_hidden(note))
        .map(|(index, note)| {
            let segments = diagnostic
                .note_segments
                .get(index)
                .filter(|segments| !segments.is_empty())
                .cloned()
                .unwrap_or_else(|| {
                    vec![DiagnosticTextSegment::new(
                        DiagnosticTextRole::Plain,
                        note.clone(),
                    )]
                });
            diagnostic_text_segment_render_parts(&segments)
        })
        .collect()
}

#[requires(true)]
#[ensures(text.starts_with("expected one of:") -> ret)]
pub(super) fn diagnostic_plain_note_is_hidden(text: &str) -> bool {
    text.trim_start().starts_with("expected one of:")
}

#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_styled_notes_for_web(
    diagnostic: &Diagnostic,
) -> Vec<&DiagnosticStyledNote> {
    diagnostic
        .styled_notes
        .iter()
        .filter(|note| !diagnostic_styled_note_is_hidden(note))
        .collect()
}

#[requires(true)]
#[ensures(matches!(note.mode, jbotci_diagnostics::DiagnosticNoteMode::Summary) && diagnostic_styled_note_text(note).trim_start().starts_with("expected one of:") -> ret)]
pub(super) fn diagnostic_styled_note_is_hidden(note: &DiagnosticStyledNote) -> bool {
    matches!(note.mode, jbotci_diagnostics::DiagnosticNoteMode::Summary)
        && diagnostic_styled_note_text(note)
            .trim_start()
            .starts_with("expected one of:")
}

#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_styled_note_text(note: &DiagnosticStyledNote) -> String {
    diagnostic_text_segments_text(&note.segments)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_text_parts_text(parts: &[DiagnosticTextRenderPart]) -> String {
    parts.iter().fold(String::new(), |mut text, part| {
        text.push_str(&part.text);
        text
    })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_styled_note_class(note: &DiagnosticStyledNote) -> String {
    class_names(
        "gentufa-diagnostic-note gentufa-diagnostic-styled-note",
        &[
            (
                "is-always",
                matches!(note.mode, jbotci_diagnostics::DiagnosticNoteMode::Always),
            ),
            (
                "is-summary",
                matches!(note.mode, jbotci_diagnostics::DiagnosticNoteMode::Summary),
            ),
            (
                "is-detailed",
                matches!(note.mode, jbotci_diagnostics::DiagnosticNoteMode::Detailed),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_text_role_class(role: DiagnosticTextRole) -> &'static str {
    match role {
        DiagnosticTextRole::Construct => "diagnostic-text diagnostic-text-construct",
        DiagnosticTextRole::SpecificWord => "diagnostic-text diagnostic-text-specific-word",
        DiagnosticTextRole::Selmaho => "diagnostic-text diagnostic-text-selmaho",
        DiagnosticTextRole::WordCategory => "diagnostic-text diagnostic-text-word-category",
        DiagnosticTextRole::Keyword => "diagnostic-text diagnostic-text-keyword",
        DiagnosticTextRole::Punctuation => "diagnostic-text diagnostic-text-punctuation",
        DiagnosticTextRole::Plain => "diagnostic-text diagnostic-text-plain",
    }
}

#[requires(segments.iter().all(|segment| !segment.text.is_empty()))]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_text_segment_render_parts(
    segments: &[DiagnosticTextSegment],
) -> Vec<DiagnosticTextRenderPart> {
    merge_diagnostic_text_parts(
        segments
            .iter()
            .map(|segment| {
                diagnostic_text_render_part(
                    segment.role,
                    segment.text.clone(),
                    segment.link.clone(),
                )
            })
            .collect(),
    )
}

#[requires(!text.is_empty())]
#[ensures(!ret.text.is_empty())]
pub(super) fn diagnostic_text_render_part(
    role: DiagnosticTextRole,
    text: String,
    link: Option<DiagnosticTextLink>,
) -> DiagnosticTextRenderPart {
    new!(DiagnosticTextRenderPart { role, text, link })
}

#[requires(true)]
#[ensures(ret.iter().all(|part| !part.text.is_empty()))]
pub(super) fn merge_diagnostic_text_parts(
    parts: Vec<DiagnosticTextRenderPart>,
) -> Vec<DiagnosticTextRenderPart> {
    let mut merged = Vec::<DiagnosticTextRenderPart>::new();
    for part in parts {
        if let Some(previous) = merged.last()
            && previous.role == part.role
            && previous.link == part.link
        {
            let mut previous_data = merged
                .pop()
                .expect("last text part was checked above")
                .into_data();
            previous_data.text.push_str(&part.text);
            merged.push(DiagnosticTextRenderPart::from_data(previous_data));
            continue;
        }
        merged.push(part);
    }
    merged
}

#[requires(!part.text.is_empty())]
#[ensures(true)]
pub(super) fn render_diagnostic_text_part(
    part: &DiagnosticTextRenderPart,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    base_path: &str,
    script: GentufaScript,
    page_find: Option<&PageFindContext>,
) -> Element {
    let class_name = diagnostic_text_role_class(part.role);
    let href = diagnostic_text_part_href(part, base_path);
    let label = diagnostic_display_text_part_for_script(part, script);
    if let Some(href) = href {
        render_diagnostic_text_link(
            class_name,
            &href,
            base_path,
            &label,
            pending_cukta_scroll,
            page_find,
        )
    } else {
        rsx! {
            span { class: "{class_name}",
                { render_optional_page_find_text(page_find, &label) }
            }
        }
    }
}

#[requires(!part.text.is_empty())]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_display_text_part_for_script(
    part: &DiagnosticTextRenderPart,
    script: GentufaScript,
) -> String {
    if part.role == DiagnosticTextRole::SpecificWord {
        display_lojban_text(script, &part.text)
    } else {
        part.text.clone()
    }
}

#[requires(!class_name.is_empty())]
#[requires(!href.is_empty())]
#[requires(!label.is_empty())]
#[ensures(true)]
pub(super) fn render_diagnostic_text_link(
    class_name: &str,
    href: &str,
    base_path: &str,
    label: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    page_find: Option<&PageFindContext>,
) -> Element {
    let class_name = format!("{class_name} diagnostic-text-link");
    if let Some(route) = jbotci_route_from_href(base_path, href) {
        let pending_scroll = cukta_pending_scroll_for_explicit_route_link(base_path, &route);
        let click_route = route.clone();
        rsx! {
            Link {
                class: "{class_name}",
                to: route,
                onclick_only: true,
                onclick: move |_| {
                    if let Some(pending_scroll) = pending_scroll.clone() {
                        push_route_with_cukta_scroll_intent(
                            pending_cukta_scroll,
                            Some(pending_scroll),
                            click_route.clone(),
                        );
                    }
                },
                { render_optional_page_find_text(page_find, label) }
            }
        }
    } else {
        rsx! {
            a { class: "{class_name}", href: "{href}",
                { render_optional_page_find_text(page_find, label) }
            }
        }
    }
}

#[requires(!part.text.is_empty())]
#[ensures(ret.as_ref().is_none_or(|href| !href.is_empty()))]
pub(super) fn diagnostic_text_part_href(
    part: &DiagnosticTextRenderPart,
    base_path: &str,
) -> Option<String> {
    part.link
        .as_ref()
        .map(|link| diagnostic_text_link_href(base_path, link))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_text_link_href(base_path: &str, link: &DiagnosticTextLink) -> String {
    if let Some(word) = link.vlacku_word() {
        diagnostic_vlacku_href(base_path, word)
    } else if let Some((section_id, anchor)) = link.cll_section() {
        diagnostic_cukta_section_href(base_path, section_id, anchor)
    } else if let Some(rule_name) = link.ebnf_rule() {
        diagnostic_ebnf_rule_href(base_path, rule_name)
    } else {
        unreachable!("diagnostic text link variants are exhaustive")
    }
}

#[requires(!word.is_empty())]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_vlacku_href(base_path: &str, word: &str) -> String {
    format!("{}/vlacku/{word}", base_path.trim_end_matches('/'))
}

#[requires(!section_id.is_empty())]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_cukta_section_href(
    base_path: &str,
    section_id: &str,
    anchor: Option<&str>,
) -> String {
    let mut href = format!(
        "{}/cukta/section/{section_id}",
        base_path.trim_end_matches('/')
    );
    if let Some(anchor) = anchor.filter(|anchor| !anchor.is_empty()) {
        href.push('#');
        href.push_str(anchor.trim_start_matches('#'));
    }
    href
}

#[requires(!rule_name.is_empty())]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_ebnf_rule_href(base_path: &str, rule_name: &str) -> String {
    diagnostic_cukta_section_href(
        base_path,
        "section-EBNF",
        Some(jbotci_cll::ebnf_rule_anchor_id(rule_name).as_str()),
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_overlay_fragments(
    text: &str,
    diagnostics: &[Diagnostic],
    active_diagnostic: Option<usize>,
) -> Vec<DiagnosticOverlayFragment> {
    let chars = text.chars().collect::<Vec<_>>();
    let mut fragments = Vec::new();
    let mut run_text = String::new();
    let mut run_class = String::new();
    let mut run_selection_start = 0u32;
    let mut run_diagnostic_index = None;
    let mut selection_offset = 0u32;

    for index in 0..=chars.len() {
        if has_diagnostic_caret_at(index, chars.len(), diagnostics, active_diagnostic) {
            flush_diagnostic_overlay_run(
                &mut fragments,
                &mut run_text,
                &mut run_class,
                &mut run_selection_start,
                &mut run_diagnostic_index,
            );
            push_diagnostic_overlay_carets(
                &mut fragments,
                index,
                chars.len(),
                selection_offset,
                diagnostics,
                active_diagnostic,
            );
        }
        let Some(character) = chars.get(index) else {
            break;
        };
        let mark =
            diagnostic_overlay_mark_for_char(index, chars.len(), diagnostics, active_diagnostic);
        let class_name = diagnostic_overlay_class(mark, diagnostics);
        let diagnostic_index = mark.map(|mark| mark.diagnostic_index);
        if !run_text.is_empty()
            && (run_class != class_name || run_diagnostic_index != diagnostic_index)
        {
            flush_diagnostic_overlay_run(
                &mut fragments,
                &mut run_text,
                &mut run_class,
                &mut run_selection_start,
                &mut run_diagnostic_index,
            );
        }
        if run_text.is_empty() {
            run_class = class_name;
            run_selection_start = selection_offset;
            run_diagnostic_index = diagnostic_index;
        }
        run_text.push(*character);
        selection_offset += character.len_utf16() as u32;
    }
    flush_diagnostic_overlay_run(
        &mut fragments,
        &mut run_text,
        &mut run_class,
        &mut run_selection_start,
        &mut run_diagnostic_index,
    );
    mark_active_context_overlay_groups(&mut fragments);
    fragments
}

#[requires(true)]
#[ensures(run_text.is_empty())]
pub(super) fn flush_diagnostic_overlay_run(
    fragments: &mut Vec<DiagnosticOverlayFragment>,
    run_text: &mut String,
    run_class: &mut String,
    run_selection_start: &mut u32,
    run_diagnostic_index: &mut Option<usize>,
) {
    if run_text.is_empty() {
        return;
    }
    fragments.push(new!(DiagnosticOverlayFragment {
        text: std::mem::take(run_text),
        class_name: std::mem::take(run_class),
        selection_start: *run_selection_start,
        diagnostic_index: *run_diagnostic_index,
    }));
    *run_selection_start = 0;
    *run_diagnostic_index = None;
}

#[requires(true)]
#[ensures(true)]
pub(super) fn mark_active_context_overlay_groups(fragments: &mut [DiagnosticOverlayFragment]) {
    let mut group_start = None;
    for index in 0..=fragments.len() {
        let in_group = fragments
            .get(index)
            .is_some_and(|fragment| diagnostic_overlay_fragment_is_active_context(fragment));
        match (group_start, in_group) {
            (None, true) => group_start = Some(index),
            (Some(start), false) => {
                mark_active_context_overlay_group(fragments, start, index);
                group_start = None;
            }
            _ => {}
        }
    }
}

#[requires(start < end)]
#[requires(end <= fragments.len())]
#[ensures(true)]
pub(super) fn mark_active_context_overlay_group(
    fragments: &mut [DiagnosticOverlayFragment],
    start: usize,
    end: usize,
) {
    if let Some(first) = fragments.get_mut(start) {
        append_diagnostic_overlay_fragment_css_class(first, "is-active-context-start");
    }
    if let Some(last) = fragments.get_mut(end - 1) {
        append_diagnostic_overlay_fragment_css_class(last, "is-active-context-end");
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_overlay_fragment_is_active_context(
    fragment: &DiagnosticOverlayFragment,
) -> bool {
    css_class_contains(&fragment.class_name, "is-active-context")
        || css_class_contains(&fragment.class_name, "is-active-context-token")
}

#[requires(!class_to_add.is_empty())]
#[ensures(css_class_contains(&fragment.class_name, class_to_add))]
pub(super) fn append_diagnostic_overlay_fragment_css_class(
    fragment: &mut DiagnosticOverlayFragment,
    class_to_add: &str,
) {
    if css_class_contains(&fragment.class_name, class_to_add) {
        return;
    }
    let mut data = fragment.clone().into_data();
    append_css_class(&mut data.class_name, class_to_add);
    *fragment = DiagnosticOverlayFragment::from_data(data);
}

#[requires(!class_name.is_empty())]
#[requires(!class_to_add.is_empty())]
#[ensures(css_class_contains(class_name, class_to_add))]
pub(super) fn append_css_class(class_name: &mut String, class_to_add: &str) {
    if css_class_contains(class_name, class_to_add) {
        return;
    }
    class_name.push(' ');
    class_name.push_str(class_to_add);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn css_class_contains(class_name: &str, expected: &str) -> bool {
    class_name
        .split_whitespace()
        .any(|class_name| class_name == expected)
}

#[requires(index <= char_len)]
#[ensures(true)]
pub(super) fn has_diagnostic_caret_at(
    index: usize,
    char_len: usize,
    diagnostics: &[Diagnostic],
    active_diagnostic: Option<usize>,
) -> bool {
    diagnostics
        .iter()
        .enumerate()
        .any(|(diagnostic_index, diagnostic)| {
            diagnostic.labels.iter().any(|label| {
                diagnostic_label_is_visible_in_overlay(diagnostic_index, label, active_diagnostic)
                    && label_span_char_range(label, char_len) == (index, index)
            })
        })
}

#[requires(index <= char_len)]
#[ensures(true)]
pub(super) fn push_diagnostic_overlay_carets(
    fragments: &mut Vec<DiagnosticOverlayFragment>,
    index: usize,
    char_len: usize,
    selection_offset: u32,
    diagnostics: &[Diagnostic],
    active_diagnostic: Option<usize>,
) {
    for (diagnostic_index, diagnostic) in diagnostics.iter().enumerate() {
        for label in &diagnostic.labels {
            if !diagnostic_label_is_visible_in_overlay(diagnostic_index, label, active_diagnostic) {
                continue;
            }
            if label_span_char_range(label, char_len) != (index, index) {
                continue;
            }
            let role = if label.primary {
                if active_diagnostic == Some(diagnostic_index) {
                    DiagnosticOverlayRole::ActivePrimary
                } else {
                    DiagnosticOverlayRole::Primary
                }
            } else {
                DiagnosticOverlayRole::ActiveContextPrefix
            };
            let mark = Some(DiagnosticOverlayMark {
                diagnostic_index,
                role,
            });
            fragments.push(new!(DiagnosticOverlayFragment {
                text: String::new(),
                class_name: diagnostic_overlay_caret_class(mark, diagnostics),
                selection_start: selection_offset,
                diagnostic_index: mark.map(|mark| mark.diagnostic_index),
            }));
        }
    }
}

#[requires(index < char_len)]
#[ensures(true)]
pub(super) fn diagnostic_overlay_mark_for_char(
    index: usize,
    char_len: usize,
    diagnostics: &[Diagnostic],
    active_diagnostic: Option<usize>,
) -> Option<DiagnosticOverlayMark> {
    if let Some(active_index) = active_diagnostic
        && let Some(active) = diagnostics.get(active_index)
    {
        if label_contains_char(active.primary_label(), index, char_len) {
            return Some(DiagnosticOverlayMark {
                diagnostic_index: active_index,
                role: DiagnosticOverlayRole::ActivePrimary,
            });
        }
        if active_context_range_contains_char(active, index, char_len) {
            return Some(DiagnosticOverlayMark {
                diagnostic_index: active_index,
                role: DiagnosticOverlayRole::ActiveContextPrefix,
            });
        }
    }
    primary_overlay_mark_for_char(index, char_len, diagnostics)
}

#[requires(index < char_len)]
#[ensures(true)]
pub(super) fn active_context_range_contains_char(
    diagnostic: &Diagnostic,
    index: usize,
    char_len: usize,
) -> bool {
    let (primary_start, primary_end) = label_span_char_range(diagnostic.primary_label(), char_len);
    diagnostic.labels.iter().any(|label| {
        if label.primary {
            return false;
        }
        let (context_start, _) = label_span_char_range(label, char_len);
        let start = context_start.min(primary_start);
        let end = primary_end.max(primary_start);
        start <= index && index < end
    })
}

#[requires(index < char_len)]
#[ensures(true)]
pub(super) fn primary_overlay_mark_for_char(
    index: usize,
    char_len: usize,
    diagnostics: &[Diagnostic],
) -> Option<DiagnosticOverlayMark> {
    diagnostics
        .iter()
        .enumerate()
        .filter(|(_, diagnostic)| diagnostic.severity == DiagnosticSeverity::Error)
        .find(|(_, diagnostic)| label_contains_char(diagnostic.primary_label(), index, char_len))
        .map(|(diagnostic_index, _)| DiagnosticOverlayMark {
            diagnostic_index,
            role: DiagnosticOverlayRole::Primary,
        })
        .or_else(|| {
            diagnostics
                .iter()
                .enumerate()
                .find(|(_, diagnostic)| {
                    label_contains_char(diagnostic.primary_label(), index, char_len)
                })
                .map(|(diagnostic_index, _)| DiagnosticOverlayMark {
                    diagnostic_index,
                    role: DiagnosticOverlayRole::Primary,
                })
        })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn diagnostic_label_is_visible_in_overlay(
    diagnostic_index: usize,
    label: &DiagnosticLabel,
    active_diagnostic: Option<usize>,
) -> bool {
    label.primary || active_diagnostic == Some(diagnostic_index)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn label_contains_char(label: &DiagnosticLabel, index: usize, char_len: usize) -> bool {
    let (start, end) = label_span_char_range(label, char_len);
    start <= index && index < end
}

#[requires(true)]
#[ensures(ret.0 <= ret.1)]
#[ensures(ret.1 <= char_len)]
pub(super) fn label_span_char_range(label: &DiagnosticLabel, char_len: usize) -> (usize, usize) {
    let start = label.span.char_start.min(char_len);
    let end = label.span.char_end.min(char_len).max(start);
    (start, end)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_overlay_class(
    mark: Option<DiagnosticOverlayMark>,
    diagnostics: &[Diagnostic],
) -> String {
    let Some(mark) = mark else {
        return "gentufa-diagnostic-overlay-fragment".to_owned();
    };
    diagnostic_overlay_mark_class("gentufa-diagnostic-overlay-fragment", mark, diagnostics)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_overlay_caret_class(
    mark: Option<DiagnosticOverlayMark>,
    diagnostics: &[Diagnostic],
) -> String {
    let Some(mark) = mark else {
        return "gentufa-diagnostic-overlay-caret".to_owned();
    };
    diagnostic_overlay_mark_class("gentufa-diagnostic-overlay-caret", mark, diagnostics)
}

#[requires(!base.is_empty())]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_overlay_mark_class(
    base: &str,
    mark: DiagnosticOverlayMark,
    diagnostics: &[Diagnostic],
) -> String {
    let severity = diagnostics
        .get(mark.diagnostic_index)
        .map(|diagnostic| diagnostic.severity)
        .unwrap_or(DiagnosticSeverity::Warning);
    class_names(
        base,
        &[
            ("has-diagnostic", true),
            ("is-error", severity == DiagnosticSeverity::Error),
            ("is-warning", severity != DiagnosticSeverity::Error),
            (
                "is-active-primary",
                mark.role == DiagnosticOverlayRole::ActivePrimary,
            ),
            (
                "is-active-context",
                mark.role == DiagnosticOverlayRole::ActiveContextPrefix,
            ),
            (
                "is-active-context-token",
                mark.role == DiagnosticOverlayRole::ActivePrimary
                    && diagnostics
                        .get(mark.diagnostic_index)
                        .is_some_and(|diagnostic| {
                            diagnostic.labels.iter().any(|label| !label.primary)
                        }),
            ),
        ],
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn diagnostic_tooltip_text(diagnostic: &Diagnostic) -> String {
    let message = diagnostic_primary_detail_display_text(diagnostic)
        .unwrap_or_else(|| diagnostic.message.clone());
    format!("{}: {message}", diagnostic.code)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_view_tabs(
    mut view_mode: Signal<GentufaWebViewMode>,
    current: GentufaWebViewMode,
) -> Element {
    rsx! {
        div { class: "view-tabs",
            button {
                class: view_tab_class(current == GentufaWebViewMode::Blocks),
                r#type: "button",
                aria_current: if current == GentufaWebViewMode::Blocks { "page" } else { "false" },
                onclick: move |_| {
                    view_mode.set(GentufaWebViewMode::Blocks);
                },
                "Blocks"
            }
            button {
                class: view_tab_class(current == GentufaWebViewMode::Tree),
                r#type: "button",
                aria_current: if current == GentufaWebViewMode::Tree { "page" } else { "false" },
                onclick: move |_| {
                    view_mode.set(GentufaWebViewMode::Tree);
                },
                "Tree"
            }
            button {
                class: view_tab_class(current == GentufaWebViewMode::Ipa),
                r#type: "button",
                aria_current: if current == GentufaWebViewMode::Ipa { "page" } else { "false" },
                onclick: move |_| {
                    view_mode.set(GentufaWebViewMode::Ipa);
                },
                "IPA"
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_output_controls(
    display: Signal<GentufaDisplayState>,
    current: GentufaDisplayState,
) -> Element {
    rsx! {
        div { class: "controls output-controls",
            { render_gloss_checkbox(display, current.show_glosses) }
            { render_elided_checkbox(display, current.show_elided) }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gloss_checkbox(
    mut display: Signal<GentufaDisplayState>,
    checked: bool,
) -> Element {
    rsx! {
        label {
            input {
                r#type: "checkbox",
                checked,
                onchange: move |_| toggle_glosses(&mut display),
            }
            " gloss"
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_elided_checkbox(
    mut display: Signal<GentufaDisplayState>,
    checked: bool,
) -> Element {
    rsx! {
        label {
            input {
                r#type: "checkbox",
                checked,
                onchange: move |_| toggle_elided(&mut display),
            }
            " elided"
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_ipa_output(success: &GentufaSuccess, page_find: &PageFindContext) -> Element {
    rsx! {
        section { class: "ipa-view",
            pre { class: "ipa-tab-output",
                { render_page_find_text(page_find, &success.ipa_text) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_blocks(
    success: &GentufaSuccess,
    show_glosses: bool,
    script: GentufaScript,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    activity: Signal<AsyncActivityState>,
    export_task: Signal<Option<LatestAsyncTask>>,
    page_find: &PageFindContext,
) -> Element {
    let column_count = success.blocks_layout.max_col.max(1);
    let column_template = repeated_parse_tree_template(column_count);
    let row_template = blocks_grid_row_template(success.blocks_layout.max_row, show_glosses);
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
                    onscroll: move |_| {
                        refresh_reference_hover(
                            reference_hover,
                            ReferenceHoverRefreshReason::ViewportShift,
                        )
                    },
                    div {
                        class: "{container_class}",
                        "data-elided": "0",
                        "data-col-count": "{column_count}",
                        div {
                            class: "blocks-grid",
                            style: "grid-template-columns: {column_template}; grid-template-rows: {row_template};",
                            for row in 0..success.blocks_layout.max_row {
                                { render_block_row_height_probe(row, column_count) }
                            }
                            for block in success.blocks_layout.blocks.iter() {
                                { render_block_reference_height_sizer(block) }
                                { render_block(block, reference_hover, reference_tooltip_open, export_anchor_id, &success.blocks_layout, show_glosses, script, activity, export_task, page_find) }
                            }
                            if show_glosses {
                                for block in success.blocks_layout.blocks.iter().filter(|block| block.is_leaf) {
                                    { render_gloss_block(block, gloss_row, page_find) }
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
#[ensures(ret >= block.row)]
pub(super) fn block_bottom_row(block: &GentufaBlock) -> usize {
    block.row + block.row_span.saturating_sub(1)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn block_has_incoming_reference(block: &GentufaBlock) -> bool {
    block
        .ref_markers
        .iter()
        .any(|marker| matches!(marker.role, ReferenceMarkerRole::Referent))
}

#[requires(true)]
#[ensures(ret == block_has_incoming_reference(block))]
pub(super) fn block_needs_reference_height_sizer(block: &GentufaBlock) -> bool {
    block_has_incoming_reference(block)
}

#[requires(true)]
#[ensures(ret >= 0.0)]
pub(super) fn reference_clearance_deficit(
    reference_bottom: f64,
    label_top: f64,
    existing_growth: f64,
) -> f64 {
    let clearance = label_top + existing_growth - reference_bottom;
    (BLOCK_REFERENCE_LABEL_GAP_PX - clearance).max(0.0)
}

#[requires(true)]
#[ensures(ret >= 0.0)]
pub(super) fn reference_containment_deficit(
    reference_bottom: f64,
    block_height: f64,
    existing_growth: f64,
) -> f64 {
    (reference_bottom + BLOCK_REFERENCE_CONTAINMENT_GAP_PX - block_height - existing_growth)
        .max(0.0)
}

#[requires(left_start <= left_end)]
#[requires(right_start <= right_end)]
#[ensures(true)]
pub(super) fn horizontal_ranges_overlap(
    left_start: f64,
    left_end: f64,
    right_start: f64,
    right_end: f64,
) -> bool {
    left_start < right_end && right_start < left_end
}

#[requires(true)]
#[ensures(ret.left == rect.left)]
pub(super) fn platform_rect_from_reference_rect(rect: ReferenceRect) -> platform::Rect {
    platform::Rect {
        left: rect.left,
        top: rect.top,
        width: (rect.right - rect.left).max(0.0),
        height: (rect.bottom - rect.top).max(0.0),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn blocks_grid_row_template(row_count: usize, show_glosses: bool) -> String {
    let mut tracks = Vec::with_capacity(row_count + usize::from(show_glosses));
    for _ in 0..row_count {
        tracks.push("minmax(var(--blocks-compact-min-height), auto)");
    }
    if show_glosses {
        tracks.push("auto");
    }
    if tracks.is_empty() {
        "auto".to_owned()
    } else {
        tracks.join(" ")
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_block_row_height_probe(row: usize, column_count: usize) -> Element {
    let grid_row = row + 1;
    let style = format!("grid-row: {grid_row} / span 1; grid-column: 1 / span {column_count};");
    rsx! {
        span {
            key: "row-probe-{row}",
            class: "block-row-height-probe",
            style: "{style}",
            "data-block-row": "{row}",
            aria_hidden: "true",
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_block_reference_height_sizer(block: &GentufaBlock) -> Element {
    if !block_needs_reference_height_sizer(block) {
        return rsx! {};
    }

    let bottom_row = block_bottom_row(block);
    let row = bottom_row + 1;
    let col = block.col + 1;
    let style = format!(
        "grid-row: {row} / span 1; grid-column: {col} / span {};",
        block.col_span
    );
    rsx! {
        span {
            key: "edge-height-{block.block_id}",
            class: "block-row-height-sizer",
            style: "{style}",
            "data-block-row": "{bottom_row}",
            aria_hidden: "true",
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_block(
    block: &GentufaBlock,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    export_anchor_id: Option<&str>,
    export_layout: &GentufaBlocksLayout,
    export_show_glosses: bool,
    export_script: GentufaScript,
    activity: Signal<AsyncActivityState>,
    export_task: Signal<Option<LatestAsyncTask>>,
    page_find: &PageFindContext,
) -> Element {
    let row = block.row + 1;
    let col = block.col + 1;
    let classes = block_class(block);
    let hover_state = reference_hover.read().clone();
    let tooltip_open_state = reference_tooltip_open.read().clone();
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
    let style = format!(
        "grid-row: {row} / span {}; grid-column: {col} / span {}; --block-color: {}; background-color: {};",
        block.row_span, block.col_span, block.color, block.color
    );
    let native_title = block_native_title(block, export_script);
    let is_export_anchor = export_anchor_id == Some(block.block_id.as_str());
    let export_controls =
        is_export_anchor.then(|| (export_layout.clone(), export_show_glosses, export_script));
    let token_kind = block
        .token_kind
        .map(|kind| kind.to_string())
        .unwrap_or_default();
    rsx! {
        div {
            key: "{block.block_id}",
            class: "{classes}",
            style: "{style}",
            "data-block-id": "{block.block_id}",
            "data-row": "{block.row}",
            "data-rowspan": "{block.row_span}",
            "data-col": "{block.col}",
            "data-colspan": "{block.col_span}",
            "data-color": "{block.color}",
            "data-token-kind": "{token_kind}",
            "data-raw-text": "{block.raw_text}",
            "data-label": "{block.label}",
            "data-node-type": "{block.node_types.join(\" \")}",
            if block.ref_markers.iter().any(|marker| marker.role == ReferenceMarkerRole::Referent) {
                span { class: "{incoming_class}",
                    for marker in block.ref_markers.iter().filter(|marker| marker.role == ReferenceMarkerRole::Referent) {
                        span { class: "ref-math ref-line",
                            { render_ref_marker(marker, reference_hover, reference_tooltip_open, &hover_state, &tooltip_open_state, export_script) }
                        }
                    }
                }
            }
            if let Some(card) = &block.tooltip {
                {
                    let base_path = router_base_path();
                    rsx! {
                        span { class: "block-label",
                            span { class: "block-label-tooltip dictionary-tooltip-host",
                                title: "{native_title}",
                                if let Some(route) = jbotci_route_from_href(&base_path, &card.href) {
                                    Link { class: "block-label-link", to: route,
                                        span { class: "block-label-text",
                                            { render_elidable_page_find_text(page_find, &block.label, block.is_elided) }
                                        }
                                    }
                                } else {
                                    a { class: "block-label-link", href: "{card.href}",
                                        span { class: "block-label-text",
                                            { render_elidable_page_find_text(page_find, &block.label, block.is_elided) }
                                        }
                                    }
                                }
                                { render_dictionary_tooltip(card, false, &base_path, export_script) }
                            }
                        }
                    }
                }
            } else {
                span { class: "block-label", title: "{native_title}",
                    span { class: "block-label-text",
                        { render_elidable_page_find_text(page_find, &block.label, block.is_elided) }
                    }
                }
            }
            if block.ref_markers.iter().any(|marker| marker.role == ReferenceMarkerRole::Reference) {
                span { class: "block-ref-source",
                    span { class: "ref-math",
                        for marker in block.ref_markers.iter().filter(|marker| marker.role == ReferenceMarkerRole::Reference) {
                            span { class: "ref-arrow", "→" }
                            { render_ref_marker(marker, reference_hover, reference_tooltip_open, &hover_state, &tooltip_open_state, export_script) }
                        }
                    }
                }
            }
            if let Some((export_layout, export_show_glosses, export_script)) = export_controls {
                {
                    let svg_layout = export_layout.clone();
                    let png_layout = export_layout.clone();
                    let svg_activity = activity;
                    let png_activity = activity;
                    let svg_export_task = export_task;
                    let png_export_task = export_task;
                    rsx! {
                span { class: "blocks-svg-link",
                    button {
                        class: "export-link",
                        r#type: "button",
                        onclick: move |_| {
                            let layout = svg_layout.clone();
                            cancel_compute_channel(COMPUTE_CHANNEL_EXPORT);
                            spawn_latest_tracked(svg_export_task, svg_activity, AsyncTaskKind::Export, async move {
                                download_gentufa_blocks_svg(layout, export_show_glosses, export_script).await;
                            });
                        },
                        "SVG"
                    }
                    button {
                        class: "export-link",
                        r#type: "button",
                        onclick: move |_| {
                            let layout = png_layout.clone();
                            cancel_compute_channel(COMPUTE_CHANNEL_EXPORT);
                            spawn_latest_tracked(png_export_task, png_activity, AsyncTaskKind::Export, async move {
                                download_gentufa_blocks_png(layout, export_show_glosses, export_script).await;
                            });
                        },
                        "PNG"
                    }
                }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(matches!(script, GentufaScript::Zbalermorna) -> ret.is_empty())]
pub(super) fn block_native_title(block: &GentufaBlock, script: GentufaScript) -> &str {
    if matches!(script, GentufaScript::Zbalermorna) {
        ""
    } else {
        &block.label
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_ref_marker(
    marker: &ReferenceMarker,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    hover_state: &ReferenceHoverState,
    tooltip_open_state: &Option<HoveredReference>,
    script: GentufaScript,
) -> Element {
    let view = reference_marker_view_model(marker, hover_state).into_data();
    let class = view.class;
    let role = view.role_attr;
    let base = view.base_key;
    let label = view.full_key;
    let kind = view.kind;
    if let Some(tooltip) = &marker.tooltip {
        let host_class = reference_tooltip_host_class(marker, tooltip_open_state);
        let base_path = router_base_path();
        let enter_hover = reference_hover;
        let leave_hover = reference_hover;
        let leave_tooltip_open = reference_tooltip_open;
        let click_tooltip_open = reference_tooltip_open;
        let enter_role = marker.role;
        let enter_label = marker.label.clone();
        let click_role = marker.role;
        let click_label = marker.label.clone();
        rsx! {
            span {
                class: "{host_class}",
                onmouseenter: move |_| set_reference_hover(enter_hover, enter_role, enter_label.clone()),
                onmouseleave: move |_| {
                    clear_reference_hover(leave_hover);
                    clear_reference_tooltip_open(leave_tooltip_open);
                },
                onclick: move |_| set_reference_tooltip_open(click_tooltip_open, click_role, click_label.clone()),
                span {
                    class: "{class}",
                    "data-ref-role": "{role}",
                    "data-ref-kind": "{kind}",
                    "data-ref-label": "{label}",
                    "data-ref-base": "{base}",
                    { render_reference_label(&marker.label) }
                }
                { render_reference_tooltip(tooltip, &base_path, script) }
            }
        }
    } else {
        let enter_hover = reference_hover;
        let leave_hover = reference_hover;
        let leave_tooltip_open = reference_tooltip_open;
        let enter_role = marker.role;
        let enter_label = marker.label.clone();
        rsx! {
            span {
                class: "{class}",
                "data-ref-role": "{role}",
                "data-ref-kind": "{kind}",
                "data-ref-label": "{label}",
                "data-ref-base": "{base}",
                onmouseenter: move |_| set_reference_hover(enter_hover, enter_role, enter_label.clone()),
                onmouseleave: move |_| {
                    clear_reference_hover(leave_hover);
                    clear_reference_tooltip_open(leave_tooltip_open);
                },
                { render_reference_label(&marker.label) }
            }
        }
    }
}

#[invariant(!self.class.is_empty())]
#[invariant(!self.role_attr.is_empty())]
#[invariant(!self.kind.is_empty())]
#[invariant(!self.base_key.is_empty())]
#[invariant(!self.full_key.is_empty())]
#[invariant(self.native_title.is_none())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReferenceMarkerViewModel {
    class: String,
    role_attr: &'static str,
    kind: String,
    base_key: String,
    full_key: String,
    pub(super) has_tooltip: bool,
    pub(super) native_title: Option<String>,
}

#[requires(true)]
#[ensures(ret.native_title.is_none())]
pub(super) fn reference_marker_view_model(
    marker: &ReferenceMarker,
    hover_state: &ReferenceHoverState,
) -> ReferenceMarkerViewModel {
    new!(ReferenceMarkerViewModel {
        class: reference_marker_class(marker, hover_state),
        role_attr: reference_role_attr(marker.role),
        kind: marker.kind.as_str().to_owned(),
        base_key: marker.label.base_key(),
        full_key: marker.label.full_key(),
        has_tooltip: marker.tooltip.is_some(),
        native_title: None,
    })
}

#[requires(true)]
#[ensures(ret.contains("reference-tooltip-host"))]
pub(super) fn reference_tooltip_host_class(
    marker: &ReferenceMarker,
    tooltip_open_state: &Option<HoveredReference>,
) -> String {
    class_names(
        "reference-tooltip-host",
        &[(
            "is-open",
            reference_tooltip_matches_marker(marker, tooltip_open_state),
        )],
    )
}

#[requires(true)]
#[ensures(true)]
pub(super) async fn download_gentufa_blocks_svg(
    layout: GentufaBlocksLayout,
    show_glosses: bool,
    script: GentufaScript,
) {
    let _ = download_gentufa_blocks_svg_result(layout, show_glosses, script).await;
}

#[requires(true)]
#[ensures(true)]
pub(super) async fn download_gentufa_blocks_png(
    layout: GentufaBlocksLayout,
    show_glosses: bool,
    script: GentufaScript,
) {
    let _ = download_gentufa_blocks_png_result(layout, show_glosses, script).await;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn download_gentufa_blocks_svg_result(
    layout: GentufaBlocksLayout,
    show_glosses: bool,
    script: GentufaScript,
) -> Result<(), String> {
    let response = compute_request(
        COMPUTE_CHANNEL_EXPORT,
        WebComputeRequest::GentufaBlocksSvg {
            layout,
            show_glosses,
            script,
        },
    )
    .await?;
    let WebComputeResponse::GentufaBlocksSvg { svg } = response else {
        return Err("compute worker returned the wrong SVG export response".to_owned());
    };
    download_browser_bytes(
        "jbotci-blocks.svg",
        "image/svg+xml;charset=utf-8",
        svg.as_bytes(),
    )
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn download_gentufa_blocks_svg_result(
    layout: GentufaBlocksLayout,
    show_glosses: bool,
    script: GentufaScript,
) -> Result<(), String> {
    let response = compute_request(
        COMPUTE_CHANNEL_EXPORT,
        WebComputeRequest::GentufaBlocksSvg {
            layout,
            show_glosses,
            script,
        },
    )
    .await?;
    let WebComputeResponse::GentufaBlocksSvg { svg } = response else {
        return Err("compute worker returned the wrong SVG export response".to_owned());
    };
    save_native_bytes("jbotci-blocks.svg", svg.as_bytes())
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
pub(super) async fn download_gentufa_blocks_svg_result(
    _layout: GentufaBlocksLayout,
    _show_glosses: bool,
    _script: GentufaScript,
) -> Result<(), String> {
    Err("gentufa SVG export is not available for this platform yet".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn download_gentufa_blocks_png_result(
    layout: GentufaBlocksLayout,
    show_glosses: bool,
    script: GentufaScript,
) -> Result<(), String> {
    let response = compute_request(
        COMPUTE_CHANNEL_EXPORT,
        WebComputeRequest::GentufaBlocksPng {
            layout,
            show_glosses,
            script,
        },
    )
    .await?;
    let WebComputeResponse::GentufaBlocksPng { bytes } = response else {
        return Err("compute worker returned the wrong PNG export response".to_owned());
    };
    download_browser_bytes("jbotci-blocks.png", "image/png", &bytes)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn download_gentufa_blocks_png_result(
    layout: GentufaBlocksLayout,
    show_glosses: bool,
    script: GentufaScript,
) -> Result<(), String> {
    let response = compute_request(
        COMPUTE_CHANNEL_EXPORT,
        WebComputeRequest::GentufaBlocksPng {
            layout,
            show_glosses,
            script,
        },
    )
    .await?;
    let WebComputeResponse::GentufaBlocksPng { bytes } = response else {
        return Err("compute worker returned the wrong PNG export response".to_owned());
    };
    save_native_bytes("jbotci-blocks.png", &bytes)
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
pub(super) async fn download_gentufa_blocks_png_result(
    _layout: GentufaBlocksLayout,
    _show_glosses: bool,
    _script: GentufaScript,
) -> Result<(), String> {
    Err("gentufa PNG export is not available for this platform yet".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[requires(!file_name.is_empty())]
#[requires(!mime_type.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn download_browser_bytes(
    file_name: &str,
    mime_type: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let Some(window) = web_sys::window() else {
        return Err("browser window is unavailable".to_owned());
    };
    let Some(document) = window.document() else {
        return Err("browser document is unavailable".to_owned());
    };
    let Some(body) = document.body() else {
        return Err("document body is unavailable".to_owned());
    };
    let parts = js_sys::Array::new();
    let bytes = js_sys::Uint8Array::from(bytes);
    parts.push(&bytes);
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime_type);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(parts.as_ref(), &options)
        .map_err(js_value_to_string)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js_value_to_string)?;
    let anchor = document
        .create_element("a")
        .map_err(js_value_to_string)?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "created anchor element had an unexpected DOM type".to_owned())?;
    anchor.set_href(&url);
    anchor.set_download(file_name);
    let anchor_html: &web_sys::HtmlElement = anchor.unchecked_ref();
    let _ = anchor_html.style().set_property("display", "none");
    body.append_child(anchor.unchecked_ref())
        .map_err(js_value_to_string)?;
    anchor_html.click();
    let _ = body.remove_child(anchor.unchecked_ref());
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!file_name.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn save_native_bytes(file_name: &str, bytes: &[u8]) -> Result<(), String> {
    let Some(path) = rfd::FileDialog::new().set_file_name(file_name).save_file() else {
        return Ok(());
    };
    std::fs::write(&path, bytes)
        .map_err(|error| format!("failed to write `{}`: {error}", path.display()))
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_gloss_block(
    block: &GentufaBlock,
    gloss_row: usize,
    page_find: &PageFindContext,
) -> Element {
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
            div { class: "gloss-list",
                { render_page_find_text(page_find, text) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_tree(
    success: &GentufaSuccess,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    rsx! {
        div { class: "table-view",
            div { class: "table-wrap",
                svg { class: "tree-lines", "aria-hidden": "true" }
                table { class: "parse-table spa-gentufa-table",
                        tbody {
                            for row in success.tree_rows.iter() {
                            { render_tree_row(row, reference_hover, reference_tooltip_open, script, page_find) }
                            }
                        }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_tree_row(
    row: &GentufaTreeRow,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    script: GentufaScript,
    page_find: &PageFindContext,
) -> Element {
    let row_class = class_names(
        "tree-row",
        &[
            ("elided-row", tree_row_is_elided(row)),
            ("tree-leaf", !row.has_children),
        ],
    );
    let parent_id = row
        .parent_id
        .map(|parent_id| parent_id.to_string())
        .unwrap_or_default();
    let indent_count = row.guides.len() + 1;
    let style = format!(
        "--row-color: {}; --block-color: {}; --indent-count: {};",
        row.color, row.color, indent_count
    );
    let hover_state = reference_hover.read().clone();
    let tooltip_open_state = reference_tooltip_open.read().clone();
    let incoming_markers = row
        .ref_markers
        .iter()
        .filter(|marker| marker.role == ReferenceMarkerRole::Referent)
        .collect::<Vec<_>>();
    let outgoing_markers = row
        .ref_markers
        .iter()
        .filter(|marker| marker.role == ReferenceMarkerRole::Reference)
        .collect::<Vec<_>>();
    let current_guide_class = class_names(
        "indent-block tree-current-guide",
        &[
            ("has-parent", !row.guides.is_empty()),
            ("line-bottom", row.has_children),
        ],
    );
    rsx! {
        tr {
            class: "{row_class}",
            style: "{style}",
            "data-node-id": "{row.node_id}",
            "data-parent-id": "{parent_id}",
            "data-depth": "{row.depth}",
            "data-color": "{row.color}",
            td { class: "col-node",
                span { class: "indent-stack",
                    for guide in row.guides.iter() {
                        { render_tree_guide(guide) }
                    }
                    span { class: "{current_guide_class}", style: "--block-color: {row.color};" }
                }
                div { class: "node-cell",
                    span { class: "node-content",
                        span { class: "node-label", style: "--block-color: {row.color};",
                            { render_page_find_text(page_find, &row.label) }
                        }
                    }
                }
            }
            { render_tree_edge_cell(incoming_markers, reference_hover, reference_tooltip_open, &hover_state, &tooltip_open_state, script) }
            td { class: "col-text",
                div { class: "cell-pad tree-text-cell",
                    for cell in row.cells.iter() {
                        { render_tree_cell(cell, page_find) }
                    }
                    { render_tree_outgoing_edges(outgoing_markers, reference_hover, reference_tooltip_open, &hover_state, &tooltip_open_state, script) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_tree_guide(guide: &GentufaTreeGuide) -> Element {
    let class = class_names(
        "indent-block tree-guide",
        &[
            ("line-top", guide.line_top),
            ("line-bottom", guide.line_bottom),
        ],
    );
    rsx! {
        span { class: "{class}", style: "--block-color: {guide.color};" }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_tree_edge_cell(
    markers: Vec<&ReferenceMarker>,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    hover_state: &ReferenceHoverState,
    tooltip_open_state: &Option<HoveredReference>,
    script: GentufaScript,
) -> Element {
    let has_markers = !markers.is_empty();
    rsx! {
        td { class: "col-edge col-edge-in",
            div { class: "cell-pad edge-cell",
                for marker in markers {
                    { render_ref_marker(marker, reference_hover, reference_tooltip_open, hover_state, tooltip_open_state, script) }
                }
                if has_markers {
                    span { class: "ref-arrow edge-arrow", "→" }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_tree_outgoing_edges(
    markers: Vec<&ReferenceMarker>,
    reference_hover: Signal<ReferenceHoverState>,
    reference_tooltip_open: Signal<Option<HoveredReference>>,
    hover_state: &ReferenceHoverState,
    tooltip_open_state: &Option<HoveredReference>,
    script: GentufaScript,
) -> Element {
    let has_markers = !markers.is_empty();
    rsx! {
        if has_markers {
            span { class: "tree-outgoing-edge edge-cell",
                span { class: "ref-arrow edge-arrow", "→" }
                for marker in markers {
                    { render_ref_marker(marker, reference_hover, reference_tooltip_open, hover_state, tooltip_open_state, script) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_tree_cell(cell: &GentufaCell, page_find: &PageFindContext) -> Element {
    let class = if cell.is_elided {
        "token is-elided"
    } else {
        "token"
    };
    rsx! {
        span { class: "{class}",
            span { class: "token-raw lojban-text",
                { render_elidable_page_find_text(page_find, &cell.text, cell.is_elided) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_elidable_text(text: &str, elided: bool) -> Element {
    if elided {
        rsx! { s { "{text}" } }
    } else {
        rsx! { "{text}" }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_elidable_page_find_text(
    page_find: &PageFindContext,
    text: &str,
    elided: bool,
) -> Element {
    if elided {
        rsx! { s { { render_page_find_text(page_find, text) } } }
    } else {
        render_page_find_text(page_find, text)
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_reference_label(label: &ReferenceLabel) -> Element {
    let slot_text = label.slot.as_ref().map(reference_slot_display_text);
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
pub(super) fn math_alphanumeric_stem(stem: &str) -> String {
    let mut output = String::new();
    for ch in stem.chars() {
        push_math_alphanumeric_char(&mut output, ch);
    }
    output
}

#[requires(true)]
#[ensures(true)]
pub(super) fn push_math_alphanumeric_char(output: &mut String, ch: char) {
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
pub(super) fn normalized_reference_stem_char(ch: char) -> Option<char> {
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
pub(super) fn is_reference_stem_combining_mark(ch: char) -> bool {
    matches!(ch, '\u{0301}' | '\u{0306}')
}

#[requires(true)]
#[ensures(true)]
pub(super) fn math_alphanumeric_ascii_char(ch: char) -> Option<char> {
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
pub(super) fn render_reference_overlay(state: &ReferenceHoverState) -> Element {
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
pub(super) fn set_reference_hover(
    mut reference_hover: Signal<ReferenceHoverState>,
    role: ReferenceMarkerRole,
    label: ReferenceLabel,
) {
    let hovered = HoveredReference { role, label };
    let current = reference_hover.read().clone();
    let measured_overlay = measure_reference_overlay(&hovered);
    let overlay = reference_overlay_for_measurement_request(
        &current,
        &hovered,
        &measured_overlay,
        reference_overlay_measurement_is_async(),
    );
    let measurement_id = next_reference_hover_measurement_id(&current);
    reference_hover.set(ReferenceHoverState {
        hovered: Some(hovered.clone()),
        overlay,
        measurement_id,
    });
    schedule_reference_overlay_measure(reference_hover, hovered, measurement_id);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn clear_reference_hover(mut reference_hover: Signal<ReferenceHoverState>) {
    let measurement_id = next_reference_hover_measurement_id(&reference_hover.read());
    reference_hover.set(ReferenceHoverState {
        hovered: None,
        overlay: None,
        measurement_id,
    });
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_reference_tooltip_open(
    mut reference_tooltip_open: Signal<Option<HoveredReference>>,
    role: ReferenceMarkerRole,
    label: ReferenceLabel,
) {
    reference_tooltip_open.set(Some(HoveredReference { role, label }));
}

#[requires(true)]
#[ensures(true)]
pub(super) fn clear_reference_tooltip_open(
    mut reference_tooltip_open: Signal<Option<HoveredReference>>,
) {
    reference_tooltip_open.set(None);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn refresh_reference_hover(
    mut reference_hover: Signal<ReferenceHoverState>,
    reason: ReferenceHoverRefreshReason,
) {
    let async_measurement = reference_overlay_measurement_is_async();
    if !reference_hover_refresh_requires_measurement(reason, async_measurement) {
        return;
    }
    let current = reference_hover.read().clone();
    let Some(hovered) = current.hovered.clone() else {
        return;
    };
    let measured_overlay = measure_reference_overlay(&hovered);
    let overlay = reference_overlay_for_measurement_request(
        &current,
        &hovered,
        &measured_overlay,
        async_measurement,
    );
    let measurement_id = next_reference_hover_measurement_id(&current);
    reference_hover.set(ReferenceHoverState {
        hovered: Some(hovered.clone()),
        overlay,
        measurement_id,
    });
    schedule_reference_overlay_measure(reference_hover, hovered, measurement_id);
}

#[requires(true)]
#[ensures(ret >= state.measurement_id)]
pub(super) fn next_reference_hover_measurement_id(state: &ReferenceHoverState) -> u64 {
    state.measurement_id.saturating_add(1)
}

#[requires(true)]
#[ensures(!async_measurement || !matches!(reason, ReferenceHoverRefreshReason::PointerMove) || !ret)]
pub(super) fn reference_hover_refresh_requires_measurement(
    reason: ReferenceHoverRefreshReason,
    async_measurement: bool,
) -> bool {
    !(async_measurement && matches!(reason, ReferenceHoverRefreshReason::PointerMove))
}

#[requires(true)]
#[ensures(measured_overlay.is_some() -> ret.as_ref() == measured_overlay.as_ref())]
#[ensures(!async_measurement && measured_overlay.is_none() -> ret.is_none())]
pub(super) fn reference_overlay_for_measurement_request(
    current: &ReferenceHoverState,
    hovered: &HoveredReference,
    measured_overlay: &Option<ArrowOverlay>,
    async_measurement: bool,
) -> Option<ArrowOverlay> {
    if let Some(overlay) = measured_overlay {
        return Some(overlay.clone());
    }
    if async_measurement && current.hovered.as_ref() == Some(hovered) {
        current.overlay.clone()
    } else {
        None
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret)]
pub(super) fn reference_overlay_measurement_is_async() -> bool {
    true
}

#[cfg(any(
    target_arch = "wasm32",
    all(not(target_arch = "wasm32"), not(feature = "desktop"))
))]
#[requires(true)]
#[ensures(!ret)]
pub(super) fn reference_overlay_measurement_is_async() -> bool {
    false
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_reference_overlay_measure(
    mut reference_hover: Signal<ReferenceHoverState>,
    hovered: HoveredReference,
    measurement_id: u64,
) {
    spawn(async move {
        let overlay = measure_reference_overlay_desktop(&hovered).await;
        reference_hover.with_mut(|state| {
            if state.measurement_id == measurement_id && state.hovered.as_ref() == Some(&hovered) {
                state.overlay = overlay;
            }
        });
    });
}

#[cfg(any(
    target_arch = "wasm32",
    all(not(target_arch = "wasm32"), not(feature = "desktop"))
))]
#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_reference_overlay_measure(
    _reference_hover: Signal<ReferenceHoverState>,
    _hovered: HoveredReference,
    _measurement_id: u64,
) {
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn reference_marker_class(
    marker: &ReferenceMarker,
    state: &ReferenceHoverState,
) -> String {
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
pub(super) fn reference_role_attr(role: ReferenceMarkerRole) -> &'static str {
    match role {
        ReferenceMarkerRole::Reference => "reference",
        ReferenceMarkerRole::Referent => "referent",
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn reference_matches_hover(
    marker: &ReferenceMarker,
    state: &ReferenceHoverState,
) -> bool {
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

#[requires(true)]
#[ensures(true)]
pub(super) fn reference_tooltip_matches_marker(
    marker: &ReferenceMarker,
    opened: &Option<HoveredReference>,
) -> bool {
    opened
        .as_ref()
        .is_some_and(|opened| marker.role == opened.role && marker.label == opened.label)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn measure_reference_overlay(hovered: &HoveredReference) -> Option<ArrowOverlay> {
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
    Some(new!(ArrowOverlay {
        width: window
            .inner_width()
            .ok()
            .and_then(|width| width.as_f64())
            .unwrap_or(1.0)
            .max(1.0),
        height: window
            .inner_height()
            .ok()
            .and_then(|height| height.as_f64())
            .unwrap_or(1.0)
            .max(1.0),
        paths,
    }))
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.is_none())]
pub(super) fn measure_reference_overlay(_hovered: &HoveredReference) -> Option<ArrowOverlay> {
    None
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
pub(super) struct DesktopReferenceOverlayMetrics {
    width: f64,
    height: f64,
    markers: Vec<DesktopReferenceMarkerMetrics>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
pub(super) struct DesktopReferenceMarkerMetrics {
    role: String,
    base: String,
    label: String,
    rect: ReferenceRect,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn measure_reference_overlay_desktop(
    hovered: &HoveredReference,
) -> Option<ArrowOverlay> {
    let metrics: DesktopReferenceOverlayMetrics = document::eval(
        r#"
        const rectFor = (element) => {
            const rect = element.getBoundingClientRect();
            return {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            };
        };
        return {
            width: Number(window.innerWidth || 1),
            height: Number(window.innerHeight || 1),
            markers: Array.from(document.querySelectorAll(".parse-page .ref-var[data-ref-role]")).map((element) => ({
                role: element.getAttribute("data-ref-role") || "",
                base: element.getAttribute("data-ref-base") || "",
                label: element.getAttribute("data-ref-label") || "",
                rect: rectFor(element),
            })),
        };
        "#,
    )
    .join()
    .await
    .ok()?;
    reference_overlay_from_marker_metrics(hovered, metrics)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn reference_overlay_from_marker_metrics(
    hovered: &HoveredReference,
    metrics: DesktopReferenceOverlayMetrics,
) -> Option<ArrowOverlay> {
    let base_key = hovered.label.base_key();
    let full_key = hovered.label.full_key();
    let mut sources = Vec::new();
    let mut targets = Vec::new();
    for marker in metrics.markers {
        if marker.base != base_key {
            continue;
        }
        if marker.role == "reference" {
            sources.push(marker.rect);
        } else if marker.role == "referent"
            && (hovered.role == ReferenceMarkerRole::Reference || marker.label == full_key)
        {
            targets.push(marker.rect);
        }
    }
    let mut paths = reference_arrow_paths(&sources, &targets);
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        None
    } else {
        Some(new!(ArrowOverlay {
            width: metrics.width.max(1.0),
            height: metrics.height.max(1.0),
            paths: paths,
        }))
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn reference_rect_from_element(element: &web_sys::Element) -> ReferenceRect {
    let rect = element.get_bounding_client_rect();
    new!(ReferenceRect {
        left: rect.left(),
        top: rect.top(),
        right: rect.right(),
        bottom: rect.bottom(),
    })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn reference_arrow_paths(
    sources: &[ReferenceRect],
    targets: &[ReferenceRect],
) -> Vec<String> {
    let mut paths = Vec::new();
    for source in sources {
        for target in targets {
            paths.push(reference_arrow_path(*source, *target));
        }
    }
    paths
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn reference_arrow_path(source: ReferenceRect, target: ReferenceRect) -> String {
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

#[requires(true)]
#[ensures(true)]
pub(super) fn rect_anchor_toward(from: ReferenceRect, to: ReferenceRect) -> (f64, f64) {
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
