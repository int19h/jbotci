use super::*;

#[invariant(self.line > 0)]
#[invariant(self.column > 0)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiagnosticSourceLocation {
    pub(super) line: usize,
    pub(super) column: usize,
}

#[invariant(self.errors <= usize::MAX - self.warnings)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DiagnosticCounts {
    pub(super) errors: usize,
    pub(super) warnings: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub(super) enum DiagnosticOverlayRole {
    Primary,
    ActivePrimary,
    ActiveContextPrefix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub(super) struct DiagnosticOverlayMark {
    pub(super) diagnostic_index: usize,
    pub(super) role: DiagnosticOverlayRole,
}

#[invariant(self.class_name.split_whitespace().next().is_some())]
#[invariant(self.diagnostic_index.is_none() || css_class_contains(&self.class_name, "has-diagnostic"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiagnosticOverlayFragment {
    pub(super) text: String,
    pub(super) class_name: String,
    pub(super) selection_start: u32,
    pub(super) diagnostic_index: Option<usize>,
}

#[invariant(self.x.is_finite())]
#[invariant(self.y.is_finite())]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DiagnosticInputTooltip {
    pub(super) diagnostic_index: usize,
    pub(super) x: f64,
    pub(super) y: f64,
}

#[invariant(!self.text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiagnosticTextRenderPart {
    pub(super) role: DiagnosticTextRole,
    pub(super) text: String,
    pub(super) link: Option<DiagnosticTextLink>,
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
