use super::*;

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn vlacku_jvozba_available() -> bool {
    web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|width| width.as_f64())
        .map_or(true, |width| width >= VLACKU_JVOZBA_MIN_WIDTH_PX)
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret)]
pub(super) fn vlacku_jvozba_available() -> bool {
    true
}

#[requires(true)]
#[ensures(true)]
pub(super) fn update_vlacku_jvozba_availability(mut available: Signal<bool>) {
    let next = vlacku_jvozba_available();
    if *available.read() != next {
        available.set(next);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn cukta_toc_forced_autohide_active() -> bool {
    web_sys::window()
        .and_then(|window| window.inner_width().ok())
        .and_then(|width| width.as_f64())
        .map_or(false, |width| width <= CUKTA_TOC_FORCED_AUTOHIDE_WIDTH_PX)
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(!ret)]
pub(super) fn cukta_toc_forced_autohide_active() -> bool {
    false
}

#[requires(true)]
#[ensures(true)]
pub(super) fn update_cukta_toc_forced_autohide(mut forced_autohide: Signal<bool>) {
    let next = cukta_toc_forced_autohide_active();
    if *forced_autohide.read() != next {
        forced_autohide.set(next);
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn update_topbar_layout(
    mut settings_layout: Signal<TopbarSettingsLayout>,
    mut settings_open: Signal<bool>,
    mut nav_layout: Signal<TopbarNavLayout>,
    next_layout: TopbarLayout,
) {
    if *settings_layout.read() != next_layout.settings {
        settings_layout.set(next_layout.settings);
    }
    if *nav_layout.read() != next_layout.nav {
        nav_layout.set(next_layout.nav);
    }
    if next_layout.settings == TopbarSettingsLayout::BothInline && *settings_open.read() {
        settings_open.set(false);
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_topbar_settings_layout_measure(
    settings_layout: Signal<TopbarSettingsLayout>,
    settings_open: Signal<bool>,
    nav_layout: Signal<TopbarNavLayout>,
) {
    platform::schedule_visual_measure_task(move || async move {
        update_topbar_layout(
            settings_layout,
            settings_open,
            nav_layout,
            measure_topbar_settings_layout_scheduled().await,
        );
    });
}

#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_topbar_active_nav_sync() {
    platform::schedule_visual_measure_task(|| async {
        scroll_active_topbar_nav_into_view_scheduled().await;
    });
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) async fn scroll_active_topbar_nav_into_view_scheduled() {
    scroll_active_topbar_nav_into_view();
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn scroll_active_topbar_nav_into_view_scheduled() {
    let _ = document::eval(
        r#"
        const active = document.querySelector('.app-topbar-nav-carousel-track [data-topbar-nav-active="true"]');
        if (active) {
            active.scrollIntoView({ block: "nearest", inline: "center" });
        }
        return null;
        "#,
    )
    .await;
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn scroll_active_topbar_nav_into_view_scheduled() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) async fn measure_topbar_settings_layout_scheduled() -> TopbarLayout {
    measure_topbar_settings_layout()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn measure_topbar_settings_layout_scheduled() -> TopbarLayout {
    let mut layout = None;
    for delay_ms in [0, 16, 64] {
        platform::sleep_ms(delay_ms).await;
        layout = measure_topbar_settings_layout_desktop().await;
        if layout.is_some() {
            break;
        }
    }
    layout.unwrap_or(new!(TopbarLayout {
        settings: TopbarSettingsLayout::BothInline,
        nav: TopbarNavLayout::Full,
    }))
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn measure_topbar_settings_layout_scheduled() -> TopbarLayout {
    new!(TopbarLayout {
        settings: TopbarSettingsLayout::BothInline,
        nav: TopbarNavLayout::Full,
    })
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_topbar_settings_layout_after_fonts_ready(
    document: &web_sys::Document,
    settings_layout: Signal<TopbarSettingsLayout>,
    settings_open: Signal<bool>,
    nav_layout: Signal<TopbarNavLayout>,
) {
    platform::schedule_after_fonts_ready(document, move || async move {
        schedule_topbar_settings_layout_measure(settings_layout, settings_open, nav_layout);
    });
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn measure_topbar_settings_layout() -> TopbarLayout {
    topbar_layout_from_probe_fits(|selector| topbar_probe_fits(selector))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[invariant(true)]
pub(super) struct TopbarLayoutMetrics {
    pub(super) available_width: f64,
    pub(super) both_full_required_width: f64,
    pub(super) theme_full_required_width: f64,
    pub(super) none_full_required_width: f64,
    pub(super) both_carousel_required_width: f64,
    pub(super) theme_carousel_required_width: f64,
    pub(super) none_carousel_required_width: f64,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn measure_topbar_settings_layout_desktop() -> Option<TopbarLayout> {
    let metrics: TopbarLayoutMetrics = document::eval(
        r#"
        const inner = document.querySelector(".app-topbar-inner");
        const stylesReady = () => {
            const shell = document.querySelector(".spa-shell.app-page");
            if (!shell) {
                return false;
            }
            const shellStyle = window.getComputedStyle(shell);
            return String(shellStyle.getPropertyValue("--topbar-bg") || "").trim().length > 0;
        };
        if (!stylesReady()) {
            return null;
        }
        const widthFor = (parent, selector) => {
            const element = parent && parent.querySelector(selector);
            if (!element) {
                return 0;
            }
            const style = window.getComputedStyle(element);
            if (style.display === "none" || style.visibility === "hidden") {
                return 0;
            }
            const rect = element.getBoundingClientRect();
            return Math.max(Number(element.scrollWidth || 0), rect.width);
        };
        const centerWidthFor = (parent) => {
            const center = parent && parent.querySelector(".app-topbar-center");
            if (!center) {
                return 0;
            }
            const style = window.getComputedStyle(center);
            if (style.display === "none" || style.visibility === "hidden") {
                return 0;
            }
            const dots = center.querySelector(".app-topbar-activity-dots");
            if (!dots) {
                return 0;
            }
            const rect = dots.getBoundingClientRect();
            return Math.max(Number(dots.scrollWidth || 0), rect.width);
        };
        const columnGapFor = (element) => {
            if (!element) {
                return 0;
            }
            const value = Number.parseFloat(window.getComputedStyle(element).columnGap || "0");
            return Number.isFinite(value) && value >= 0 ? value : 0;
        };
        const requiredFor = (selector) => {
            if (!inner) {
                return 0;
            }
            const probe = document.querySelector(selector);
            if (!probe) {
                return 0;
            }
            const probeRect = probe.getBoundingClientRect();
            const probeWidth = Math.max(Number(probe.scrollWidth || 0), probeRect.width);
            const centerWidth = centerWidthFor(inner);
            const rightWidth = widthFor(inner, ".app-topbar-right");
            const visibleColumns = 1 + (centerWidth > 0 ? 1 : 0) + (rightWidth > 0 ? 1 : 0);
            return probeWidth + centerWidth + rightWidth + (visibleColumns - 1) * columnGapFor(inner);
        };
        const availableWidth = inner ? inner.getBoundingClientRect().width : 0;
        const bothFullRequiredWidth = requiredFor(".app-topbar-fit-probe-both-full");
        const rightWidth = widthFor(inner, ".app-topbar-right");
        if (!inner || availableWidth <= 0 || bothFullRequiredWidth <= 0 || rightWidth <= 0) {
            return null;
        }
        return {
            available_width: availableWidth,
            both_full_required_width: bothFullRequiredWidth,
            theme_full_required_width: requiredFor(".app-topbar-fit-probe-theme-full"),
            none_full_required_width: requiredFor(".app-topbar-fit-probe-none-full"),
            both_carousel_required_width: requiredFor(".app-topbar-fit-probe-both-carousel"),
            theme_carousel_required_width: requiredFor(".app-topbar-fit-probe-theme-carousel"),
            none_carousel_required_width: requiredFor(".app-topbar-fit-probe-none-carousel"),
        };
        "#,
    )
    .join()
    .await
    .ok()?;
    Some(topbar_layout_from_metrics(metrics))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn topbar_layout_from_metrics(metrics: TopbarLayoutMetrics) -> TopbarLayout {
    topbar_layout_from_probe_fits(|selector| {
        let required_width = match selector {
            ".app-topbar-fit-probe-both-full" => metrics.both_full_required_width,
            ".app-topbar-fit-probe-theme-full" => metrics.theme_full_required_width,
            ".app-topbar-fit-probe-none-full" => metrics.none_full_required_width,
            ".app-topbar-fit-probe-both-carousel" => metrics.both_carousel_required_width,
            ".app-topbar-fit-probe-theme-carousel" => metrics.theme_carousel_required_width,
            ".app-topbar-fit-probe-none-carousel" => metrics.none_carousel_required_width,
            _ => metrics.none_carousel_required_width,
        };
        required_width <= metrics.available_width + 1.0
    })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn topbar_layout_from_probe_fits(fits: impl Fn(&str) -> bool) -> TopbarLayout {
    let candidates = [
        new!(TopbarLayout {
            settings: TopbarSettingsLayout::BothInline,
            nav: TopbarNavLayout::Full,
        }),
        new!(TopbarLayout {
            settings: TopbarSettingsLayout::ThemeInline,
            nav: TopbarNavLayout::Full,
        }),
        new!(TopbarLayout {
            settings: TopbarSettingsLayout::NoneInline,
            nav: TopbarNavLayout::Full,
        }),
        new!(TopbarLayout {
            settings: TopbarSettingsLayout::BothInline,
            nav: TopbarNavLayout::Carousel,
        }),
        new!(TopbarLayout {
            settings: TopbarSettingsLayout::ThemeInline,
            nav: TopbarNavLayout::Carousel,
        }),
        new!(TopbarLayout {
            settings: TopbarSettingsLayout::NoneInline,
            nav: TopbarNavLayout::Carousel,
        }),
    ];
    for candidate in candidates {
        if fits(topbar_layout_probe_selector(candidate)) {
            return candidate;
        }
    }
    new!(TopbarLayout {
        settings: TopbarSettingsLayout::NoneInline,
        nav: TopbarNavLayout::Carousel,
    })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn topbar_layout_probe_selector(layout: TopbarLayout) -> &'static str {
    match (layout.settings, layout.nav) {
        (TopbarSettingsLayout::BothInline, TopbarNavLayout::Full) => {
            ".app-topbar-fit-probe-both-full"
        }
        (TopbarSettingsLayout::ThemeInline, TopbarNavLayout::Full) => {
            ".app-topbar-fit-probe-theme-full"
        }
        (TopbarSettingsLayout::NoneInline, TopbarNavLayout::Full) => {
            ".app-topbar-fit-probe-none-full"
        }
        (TopbarSettingsLayout::BothInline, TopbarNavLayout::Carousel) => {
            ".app-topbar-fit-probe-both-carousel"
        }
        (TopbarSettingsLayout::ThemeInline, TopbarNavLayout::Carousel) => {
            ".app-topbar-fit-probe-theme-carousel"
        }
        (TopbarSettingsLayout::NoneInline, TopbarNavLayout::Carousel) => {
            ".app-topbar-fit-probe-none-carousel"
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(!selector.is_empty())]
#[ensures(true)]
pub(super) fn topbar_probe_fits(selector: &str) -> bool {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return true;
    };
    if !topbar_styles_ready(&document) {
        return true;
    }
    let Some(inner) = document.query_selector(".app-topbar-inner").ok().flatten() else {
        return true;
    };
    let Some(probe) = document.query_selector(selector).ok().flatten() else {
        return true;
    };
    let available_width = inner.get_bounding_client_rect().width();
    let center_width = topbar_center_content_width(&inner);
    let right_width = topbar_visible_width(&inner, ".app-topbar-right");
    let visible_columns = 1.0
        + if center_width > 0.0 { 1.0 } else { 0.0 }
        + if right_width > 0.0 { 1.0 } else { 0.0 };
    let required_width = element_layout_width(&probe)
        + center_width
        + right_width
        + (visible_columns - 1.0) * topbar_column_gap(&inner);
    required_width <= available_width + 1.0
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn topbar_styles_ready(document: &web_sys::Document) -> bool {
    let Some(shell) = document
        .query_selector(".spa-shell.app-page")
        .ok()
        .flatten()
    else {
        return false;
    };
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Some(style) = window.get_computed_style(&shell).ok().flatten() else {
        return false;
    };
    style
        .get_property_value("--topbar-bg")
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0.0)]
pub(super) fn topbar_center_content_width(parent: &web_sys::Element) -> f64 {
    let Some(center) = parent.query_selector(".app-topbar-center").ok().flatten() else {
        return 0.0;
    };
    let Some(window) = web_sys::window() else {
        return 0.0;
    };
    let Some(style) = window.get_computed_style(&center).ok().flatten() else {
        return 0.0;
    };
    if style.get_property_value("display").ok().as_deref() == Some("none")
        || style.get_property_value("visibility").ok().as_deref() == Some("hidden")
    {
        return 0.0;
    }
    center
        .query_selector(".app-topbar-activity-dots")
        .ok()
        .flatten()
        .map_or(0.0, |dots| element_layout_width(&dots))
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0.0)]
pub(super) fn topbar_visible_width(parent: &web_sys::Element, selector: &str) -> f64 {
    let Some(element) = parent.query_selector(selector).ok().flatten() else {
        return 0.0;
    };
    let Some(window) = web_sys::window() else {
        return 0.0;
    };
    let Some(style) = window.get_computed_style(&element).ok().flatten() else {
        return 0.0;
    };
    if style.get_property_value("display").ok().as_deref() == Some("none")
        || style.get_property_value("visibility").ok().as_deref() == Some("hidden")
    {
        return 0.0;
    }
    element_layout_width(&element)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0.0)]
pub(super) fn element_layout_width(element: &web_sys::Element) -> f64 {
    f64::from(element.scroll_width()).max(element.get_bounding_client_rect().width())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0.0)]
pub(super) fn topbar_column_gap(element: &web_sys::Element) -> f64 {
    web_sys::window()
        .and_then(|window| window.get_computed_style(element).ok().flatten())
        .and_then(|style| style.get_property_value("column-gap").ok())
        .and_then(|value| value.trim_end_matches("px").parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(0.0)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_page_find_match_scroll(match_index: usize) {
    platform::schedule_layout_task_after_delay(0, move || async move {
        scroll_page_find_match_scheduled(match_index).await;
    });
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) async fn scroll_page_find_match_scheduled(match_index: usize) {
    scroll_page_find_match(match_index);
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn scroll_page_find_match_scheduled(match_index: usize) {
    scroll_page_find_match_desktop(match_index).await;
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn scroll_page_find_match_scheduled(_match_index: usize) {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn scroll_page_find_match(match_index: usize) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let selector = format!(r#"[data-page-find-match-index="{match_index}"]"#);
    if let Ok(Some(element)) = document.query_selector(&selector) {
        element.scroll_into_view();
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn event_is_page_find_shortcut(event: &web_sys::Event) -> bool {
    let key = js_event_string_property(event, "key")
        .unwrap_or_default()
        .to_lowercase();
    let ctrl_key = js_event_bool_property(event, "ctrlKey");
    let meta_key = js_event_bool_property(event, "metaKey");
    let alt_key = js_event_bool_property(event, "altKey");
    (ctrl_key || meta_key) && !alt_key && key == "f"
}

#[cfg(target_arch = "wasm32")]
#[requires(!name.is_empty())]
#[ensures(true)]
pub(super) fn js_event_string_property(event: &web_sys::Event, name: &str) -> Option<String> {
    js_sys::Reflect::get(event.as_ref(), &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_string())
}

#[cfg(target_arch = "wasm32")]
#[requires(!name.is_empty())]
#[ensures(true)]
pub(super) fn js_event_bool_property(event: &web_sys::Event, name: &str) -> bool {
    js_sys::Reflect::get(event.as_ref(), &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn focus_page_find_input() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(element) = document.get_element_by_id(PAGE_FIND_INPUT_ID) else {
        return;
    };
    if let Ok(input) = element.dyn_into::<web_sys::HtmlInputElement>() {
        let _ = input.focus();
        input.select();
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn scroll_active_topbar_nav_into_view() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(Some(element)) = document
        .query_selector(r#".app-topbar-nav-carousel-track [data-topbar-nav-active="true"]"#)
    else {
        return;
    };
    let options = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        options.as_ref(),
        &JsValue::from_str("block"),
        &JsValue::from_str("nearest"),
    );
    let _ = js_sys::Reflect::set(
        options.as_ref(),
        &JsValue::from_str("inline"),
        &JsValue::from_str("center"),
    );
    if let Ok(function) =
        js_sys::Reflect::get(element.as_ref(), &JsValue::from_str("scrollIntoView"))
            .and_then(|value| value.dyn_into::<js_sys::Function>())
    {
        let _ = function.call1(element.as_ref(), options.as_ref());
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn scroll_page_find_match_desktop(match_index: usize) {
    let script = format!(
        r#"
        const element = document.querySelector('[data-page-find-match-index="{match_index}"]');
        if (element) {{
            element.scrollIntoView({{ block: "center", inline: "nearest" }});
        }}
        return null;
        "#
    );
    let _ = document::eval(&script).await;
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_theme_switch(
    mut settings: Signal<UserSettings>,
    current: ThemeMode,
) -> Element {
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
pub(super) fn render_script_switch(
    mut settings: Signal<UserSettings>,
    current: GentufaScript,
) -> Element {
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
pub(super) fn render_dialect_control(
    mut dialect: Signal<String>,
    dialect_settings: DialectSettings,
    mut picker_open: Signal<bool>,
) -> Element {
    let formula_text = dialect.read().clone();
    let picker_is_open = *picker_open.read();
    let picker_names = gentufa_picker_dialect_names(&dialect_settings);
    let selected_references = dialect_formula_top_level_references(&formula_text)
        .into_iter()
        .collect::<BTreeSet<_>>();
    rsx! {
        div { class: "gentufa-dialect-control",
            button {
                class: "gentufa-dialect-label",
                r#type: "button",
                aria_expanded: if picker_is_open { "true" } else { "false" },
                onclick: move |_| {
                    let next = !*picker_open.read();
                    picker_open.set(next);
                },
                "Dialect:"
            }
            div { class: "gentufa-dialect-input-shell",
                div { class: "gentufa-dialect-formula-wrap",
                    pre {
                        class: "settings-dialect-definition-highlight gentufa-dialect-formula-highlight",
                        aria_hidden: "true",
                        { render_dialect_highlight(&formula_text) }
                    }
                    textarea {
                        class: "settings-text-input settings-dialect-definition gentufa-dialect-formula-input",
                        rows: "1",
                        value: "{formula_text}",
                        placeholder: "baseline (CLL + xorlo + LTR-magic)",
                        spellcheck: "false",
                        aria_label: "Dialect formula",
                        oninput: move |event| {
                            dialect.set(event.value());
                        },
                    }
                }
                if picker_is_open {
                    div { class: "gentufa-dialect-picker",
                        for name in picker_names.iter() {
                            {
                                let item_name = name.clone();
                                let checked = selected_references.contains(name);
                                rsx! {
                                    label { class: "gentufa-dialect-picker-row",
                                        input {
                                            r#type: "checkbox",
                                            checked,
                                            onchange: move |_| {
                                                let current = dialect.read().clone();
                                                let next = if checked {
                                                    remove_dialect_formula_reference(&item_name, &current)
                                                } else {
                                                    add_dialect_formula_reference(&item_name, &current)
                                                };
                                                dialect.set(next);
                                            },
                                        }
                                        span { "{name}" }
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
pub(super) fn gentufa_picker_dialect_names(settings: &DialectSettings) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut names = Vec::new();
    for name in builtin_dialect_names() {
        if builtin_dialect_shows_in_gentufa(settings, name) && seen.insert(name.to_owned()) {
            names.push(name.to_owned());
        }
    }
    for custom in &settings.custom_dialects {
        let name = custom.name.trim();
        if custom.show_in_gentufa
            && dialect_name_shows_in_gentufa_picker(name)
            && seen.insert(name.to_owned())
        {
            names.push(name.to_owned());
        }
    }
    names
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_dialect_highlight(text: &str) -> Element {
    let tokens = dialect_highlight_tokens(text);
    rsx! {
        for token in tokens.iter() {
            span { class: "{token.class_name}", "{token.text}" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn dialect_highlight_tokens(text: &str) -> Vec<DialectHighlightToken> {
    let mut tokens = Vec::new();
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        let character = chars[index];
        if character.is_whitespace() {
            let start = index;
            while chars.get(index).is_some_and(|value| value.is_whitespace()) {
                index += 1;
            }
            tokens.push(dialect_highlight_token(
                "dialect-token-space",
                chars[start..index].iter().collect(),
            ));
        } else if matches!(character, '(' | ')') {
            tokens.push(dialect_highlight_token(
                "dialect-token-paren",
                character.to_string(),
            ));
            index += 1;
        } else {
            let start = index;
            while chars
                .get(index)
                .is_some_and(|value| !value.is_whitespace() && !matches!(*value, '(' | ')'))
            {
                index += 1;
            }
            let token_text = chars[start..index].iter().collect::<String>();
            let class_name = dialect_highlight_class(&token_text);
            tokens.push(dialect_highlight_token(class_name, token_text));
        }
    }
    if tokens.is_empty() {
        tokens.push(dialect_highlight_token(
            "dialect-token-empty",
            String::new(),
        ));
    }
    tokens
}

#[requires(!class_name.is_empty())]
#[ensures(ret.class_name == class_name)]
pub(super) fn dialect_highlight_token(class_name: &str, text: String) -> DialectHighlightToken {
    DialectHighlightToken {
        class_name: class_name.to_owned(),
        text,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn dialect_highlight_class(token: &str) -> &'static str {
    if token.starts_with('+') || token.starts_with('-') {
        "dialect-token-feature"
    } else if token == "↦" || token == "->" || token == "↔" || token == "<->" || token == "🣐"
    {
        "dialect-token-operator"
    } else if find_builtin_dialect(token).is_some() {
        "dialect-token-reference"
    } else {
        "dialect-token-word"
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(module = "/assets/embeddings.js")]
extern "C" {
    #[wasm_bindgen(js_name = jbotciEmbeddingConfigureWorker)]
    fn js_embedding_configure_worker(worker_url: &str);

    #[wasm_bindgen(js_name = jbotciEmbeddingConfigureOrtAssets)]
    fn js_embedding_configure_ort_assets(module_url: &str, wasm_mjs_url: &str, wasm_url: &str);

    #[wasm_bindgen(js_name = jbotciEmbeddingConfigureRemoteBase)]
    fn js_embedding_configure_remote_base(remote_base_url: &str);

    #[wasm_bindgen(js_name = jbotciEmbeddingConfigureCatalog)]
    fn js_embedding_configure_catalog(catalog_json: &str);

    #[wasm_bindgen(js_name = jbotciEmbeddingConfigureModel)]
    fn js_embedding_configure_model(model_key: &str);

    #[wasm_bindgen(js_name = jbotciEmbeddingPreferredModelKey)]
    fn js_embedding_preferred_model_key() -> String;

    #[wasm_bindgen(js_name = jbotciEmbeddingStatus)]
    fn js_embedding_status() -> js_sys::Promise;

    #[wasm_bindgen(js_name = jbotciEmbeddingSetup)]
    fn js_embedding_setup(corpus_json: &str, remote_base_url: &str) -> js_sys::Promise;

    #[wasm_bindgen(js_name = jbotciEmbeddingRemove)]
    fn js_embedding_remove() -> js_sys::Promise;

    #[wasm_bindgen(js_name = jbotciEmbeddingCancel)]
    fn js_embedding_cancel(channel: &str);

    #[wasm_bindgen(js_name = jbotciEmbeddingSearch)]
    fn js_embedding_search(
        channel: &str,
        corpus_id: &str,
        query: &str,
        limit: usize,
        kind_filters_json: &str,
    ) -> js_sys::Promise;
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(module = "/assets/worker-client.js")]
extern "C" {
    #[wasm_bindgen(js_name = jbotciWorkerClientAssetPin)]
    fn js_worker_client_asset_pin();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(module = "/assets/model-catalog.js")]
extern "C" {
    #[wasm_bindgen(js_name = jbotciModelCatalogAssetPin)]
    fn js_model_catalog_asset_pin();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(module = "/assets/compute.js")]
extern "C" {
    #[wasm_bindgen(js_name = jbotciComputeConfigureWorker)]
    fn js_compute_configure_worker(worker_url: &str);

    #[wasm_bindgen(js_name = jbotciComputeCancel)]
    fn js_compute_cancel(channel: &str);

    #[wasm_bindgen(js_name = jbotciComputeRequest)]
    fn js_compute_request(channel: &str, request_json: &str) -> js_sys::Promise;
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = jbotciComputeHandle)]
#[requires(true)]
#[ensures(true)]
pub fn jbotci_compute_handle(request_json: &str) -> Result<String, JsValue> {
    web_compute_handle(request_json).map_err(|error| JsValue::from_str(&error))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = jbotciWorkerReady)]
#[requires(true)]
#[ensures(true)]
pub fn jbotci_worker_ready() -> js_sys::Promise {
    js_sys::Promise::resolve(&JsValue::UNDEFINED)
}

#[requires(!request_json.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|json| !json.is_empty()) || ret.is_err())]
pub(super) fn web_compute_handle(request_json: &str) -> Result<String, String> {
    jbotci_web_core::run_web_compute_request_json(request_json).map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
#[requires(!worker_url.is_empty())]
#[ensures(true)]
pub(super) fn configure_embedding_worker_url(worker_url: &str) {
    js_embedding_configure_worker(worker_url);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!worker_url.is_empty())]
#[ensures(true)]
pub(super) fn configure_embedding_worker_url(worker_url: &str) {
    let _ = worker_url;
}

#[cfg(target_arch = "wasm32")]
#[requires(!module_url.is_empty())]
#[requires(!wasm_mjs_url.is_empty())]
#[requires(!wasm_url.is_empty())]
#[ensures(true)]
pub(super) fn configure_embedding_ort_assets(module_url: &str, wasm_mjs_url: &str, wasm_url: &str) {
    js_embedding_configure_ort_assets(module_url, wasm_mjs_url, wasm_url);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!module_url.is_empty())]
#[requires(!wasm_mjs_url.is_empty())]
#[requires(!wasm_url.is_empty())]
#[ensures(true)]
pub(super) fn configure_embedding_ort_assets(module_url: &str, wasm_mjs_url: &str, wasm_url: &str) {
    let _ = (module_url, wasm_mjs_url, wasm_url);
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn web_embeddings_base_url() -> &'static str {
    match BUILD_WEB_EMBEDDINGS_BASE_URL {
        Some(base_url) if !base_url.trim().is_empty() => base_url.trim(),
        _ => DEFAULT_WEB_EMBEDDINGS_BASE_URL,
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(!remote_base_url.is_empty())]
#[ensures(true)]
pub(super) fn configure_embedding_remote_base_url(remote_base_url: &str) {
    js_embedding_configure_remote_base(remote_base_url);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!remote_base_url.is_empty())]
#[ensures(true)]
pub(super) fn configure_embedding_remote_base_url(remote_base_url: &str) {
    let _ = remote_base_url;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn configure_embedding_model_catalog() {
    js_embedding_configure_catalog(&browser_embedding_model_catalog_json());
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn configure_embedding_model_catalog() {}

#[cfg(target_arch = "wasm32")]
#[requires(is_supported_embedding_model_key(model_key))]
#[ensures(true)]
pub(super) fn configure_embedding_model_key(model_key: &str) {
    js_embedding_configure_model(model_key);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(is_supported_embedding_model_key(model_key))]
#[ensures(true)]
pub(super) fn configure_embedding_model_key(model_key: &str) {
    let _ = model_key;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn pin_worker_client_asset() {
    js_worker_client_asset_pin();
    js_model_catalog_asset_pin();
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn pin_worker_client_asset() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(is_supported_embedding_model_key(&ret))]
pub(super) fn preferred_embedding_model_key() -> String {
    let key = js_embedding_preferred_model_key();
    if is_supported_embedding_model_key(&key) {
        key
    } else {
        F2LLM_330M_MODEL_KEY.to_owned()
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(is_supported_embedding_model_key(&ret))]
pub(super) fn preferred_embedding_model_key() -> String {
    F2LLM_NATIVE_330M_MODEL_KEY.to_owned()
}

#[cfg(target_arch = "wasm32")]
#[requires(!worker_url.is_empty())]
#[ensures(true)]
pub(super) fn configure_compute_worker_url(worker_url: &str) {
    js_compute_configure_worker(worker_url);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!worker_url.is_empty())]
#[ensures(true)]
pub(super) fn configure_compute_worker_url(worker_url: &str) {
    let _ = worker_url;
}

#[requires(true)]
#[ensures(true)]
pub(super) async fn refresh_embedding_settings(mut settings: Signal<EmbeddingSettingsState>) {
    configure_embedding_model_key(&settings.read().selected_model_key);
    match embedding_status_json().await {
        Ok(json) => settings.set(embedding_settings_from_json(&json, "Embeddings are ready.")),
        Err(error) => {
            let previous = settings.read().clone();
            settings.set(embedding_settings_error_state(
                &previous,
                "unavailable",
                error,
            ));
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) async fn setup_embeddings(mut settings: Signal<EmbeddingSettingsState>) {
    configure_embedding_model_key(&settings.read().selected_model_key);
    let corpus_json = match embedding_setup_corpus_json().await {
        Ok(json) => json,
        Err(error) => {
            let previous = settings.read().clone();
            settings.set(embedding_settings_error_state(&previous, "error", error));
            return;
        }
    };
    match embedding_setup_json(&corpus_json).await {
        Ok(json) => settings.set(embedding_settings_from_json(&json, "Embeddings are ready.")),
        Err(error) => {
            let previous = settings.read().clone();
            settings.set(embedding_settings_error_state(&previous, "error", error));
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn embedding_setup_corpus_json() -> Result<String, String> {
    embedding_corpus_json_from_compute_worker().await
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|json| json.is_empty()) || ret.is_err())]
pub(super) async fn embedding_setup_corpus_json() -> Result<String, String> {
    Ok(String::new())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn embedding_corpus_json_from_compute_worker() -> Result<String, String> {
    let response = compute_request(
        COMPUTE_CHANNEL_EMBEDDINGS,
        WebComputeRequest::EmbeddingCorpusJson,
    )
    .await?;
    let WebComputeResponse::EmbeddingCorpusJson { json } = response else {
        return Err("compute worker returned the wrong embedding corpus response".to_owned());
    };
    Ok(json)
}

#[requires(true)]
#[ensures(true)]
pub(super) async fn poll_embedding_settings_while_busy(
    mut settings: Signal<EmbeddingSettingsState>,
) {
    loop {
        platform::sleep_ms(350).await;
        if !settings.read().busy {
            break;
        }
        if let Ok(json) = embedding_status_json().await {
            let mut next = embedding_settings_from_json(&json, "Embeddings are being prepared.");
            next.busy = true;
            settings.set(next);
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) async fn remove_embeddings(mut settings: Signal<EmbeddingSettingsState>) {
    configure_embedding_model_key(&settings.read().selected_model_key);
    match embedding_remove_json().await {
        Ok(json) => settings.set(embedding_settings_from_json(
            &json,
            "Embeddings were removed.",
        )),
        Err(error) => {
            let previous = settings.read().clone();
            settings.set(embedding_settings_error_state(&previous, "error", error));
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) async fn load_vlacku_semantic_result(
    state: VlackuWebState,
) -> VlackuSemanticResultState {
    let limit = vlacku_semantic_worker_limit(&state);
    let normalized_state = normalize_vlacku_state(&state);
    match embedding_search_json(
        EMBEDDING_CHANNEL_VLACKU_SEMANTIC,
        "vlacku-en",
        &state.query,
        limit,
        &normalized_state.word_types,
    )
    .await
    {
        Ok(json) => {
            let (hits, message) = parse_vlacku_semantic_search_json(&json);
            VlackuSemanticResultState {
                state: Some(state),
                hits,
                message,
                loading: false,
            }
        }
        Err(error) => VlackuSemanticResultState {
            state: Some(state),
            hits: Vec::new(),
            message: Some(error),
            loading: false,
        },
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn spawn_vlacku_semantic_loading_message(
    mut result_signal: Signal<VlackuSemanticResultState>,
    state: VlackuWebState,
) {
    spawn(async move {
        platform::sleep_ms(SEMANTIC_LOADING_MESSAGE_DELAY_MS).await;
        if embedding_status_is_loading_model().await {
            result_signal.with_mut(|current| {
                if current.loading && current.state.as_ref() == Some(&state) {
                    current.message = Some("Loading semantic search model.".to_owned());
                }
            });
        }
    });
}

#[requires(true)]
#[ensures(ret >= 1 && ret <= VLACKU_WEB_MAX_COUNT)]
pub(super) fn vlacku_semantic_worker_limit(state: &VlackuWebState) -> usize {
    let normalized_state = normalize_vlacku_state(state);
    normalized_state
        .count
        .saturating_add(1)
        .min(VLACKU_WEB_MAX_COUNT)
}

#[requires(true)]
#[ensures(true)]
pub(super) async fn load_cukta_semantic_result(
    state: CuktaWebSearchState,
) -> CuktaSemanticResultState {
    let limit = cukta_semantic_worker_limit(&state);
    let kind_filters = cukta_semantic_worker_kind_filters(&state);
    match embedding_search_json(
        EMBEDDING_CHANNEL_CUKTA_SEMANTIC,
        "cukta-cll",
        &state.query,
        limit,
        &kind_filters,
    )
    .await
    {
        Ok(json) => {
            let (hits, message) = parse_cukta_semantic_search_json(&json);
            CuktaSemanticResultState {
                state: Some(state),
                hits,
                message,
                loading: false,
            }
        }
        Err(error) => CuktaSemanticResultState {
            state: Some(state),
            hits: Vec::new(),
            message: Some(error),
            loading: false,
        },
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn spawn_cukta_semantic_loading_message(
    mut result_signal: Signal<CuktaSemanticResultState>,
    state: CuktaWebSearchState,
) {
    spawn(async move {
        platform::sleep_ms(SEMANTIC_LOADING_MESSAGE_DELAY_MS).await;
        if embedding_status_is_loading_model().await {
            result_signal.with_mut(|current| {
                if current.loading && current.state.as_ref() == Some(&state) {
                    current.message = Some("Loading semantic search model.".to_owned());
                }
            });
        }
    });
}

#[requires(true)]
#[ensures(ret >= 1 && ret <= CUKTA_WEB_MAX_COUNT)]
pub(super) fn cukta_semantic_worker_limit(state: &CuktaWebSearchState) -> usize {
    state
        .count
        .clamp(1, CUKTA_WEB_MAX_COUNT)
        .saturating_add(1)
        .min(CUKTA_WEB_MAX_COUNT)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn cukta_semantic_worker_kind_filters(state: &CuktaWebSearchState) -> Vec<String> {
    let mut filters = Vec::new();
    for target in &state.targets {
        match target {
            CuktaSearchTarget::Section => push_unique_filter(&mut filters, "section"),
            CuktaSearchTarget::Paragraph => push_unique_filter(&mut filters, "paragraph"),
            CuktaSearchTarget::Example => push_unique_filter(&mut filters, "example"),
        }
    }
    if filters.is_empty() {
        filters.extend(
            ["section", "paragraph", "example"]
                .into_iter()
                .map(str::to_owned),
        );
    }
    filters
}

#[requires(!filter.is_empty())]
#[ensures(filters.iter().any(|candidate| candidate == filter))]
pub(super) fn push_unique_filter(filters: &mut Vec<String>, filter: &str) {
    if !filters.iter().any(|candidate| candidate == filter) {
        filters.push(filter.to_owned());
    }
}

#[requires(!message.is_empty())]
#[ensures(matches!(ret.page_kind, CuktaPageKind::Error { .. }))]
pub(super) fn cukta_loading_page_data(message: &str) -> CuktaPageData {
    CuktaPageData {
        toc: Vec::new(),
        current_section_id: None,
        page_kind: CuktaPageKind::Error {
            message: message.to_owned(),
        },
    }
}

#[requires(!message.is_empty())]
#[ensures(ret.message.as_ref().is_some_and(|value| value == message))]
pub(super) fn vlacku_loading_result(state: &VlackuWebState, message: &str) -> VlackuWebResult {
    VlackuWebResult {
        state: state.clone(),
        cards: Vec::new(),
        word_type_options: vlacku_word_type_options(&state.word_types),
        dictionary_info: None,
        has_more: false,
        message: Some(message.to_owned()),
        errors: Vec::new(),
    }
}

#[requires(true)]
#[ensures(ret.errors.is_empty())]
pub(super) fn gimfihi_empty_result(state: &GimfihiWebState) -> GimfihiWebResult {
    let state = normalize_gimfihi_state(state);
    GimfihiWebResult {
        preset_options: gimfihi_preset_options_for_state(&state),
        language_suggestions: gimfihi_language_suggestions(),
        state,
        output: None,
        errors: Vec::new(),
    }
}

#[requires(true)]
#[ensures(ret.state.as_ref().is_some_and(|current| current == state))]
#[ensures(!ret.loading)]
pub(super) fn gimfihi_idle_result_state(state: &GimfihiWebState) -> GimfihiAsyncResultState {
    GimfihiAsyncResultState {
        state: Some(state.clone()),
        result: gimfihi_empty_result(state),
        meta: None,
        loading: false,
        error: None,
    }
}

#[requires(!message.is_empty())]
#[ensures(ret.error.as_ref().is_some_and(|error| error == message))]
pub(super) fn gentufa_async_error_state(
    state: GentufaWebState,
    request: GentufaWebRequest,
    message: &str,
) -> GentufaAsyncPageState {
    GentufaAsyncPageState {
        state: Some(state),
        request: Some(request),
        result: GentufaWebResult::Error(GentufaError {
            phase: None,
            message: message.to_owned(),
            diagnostics: Vec::new(),
        }),
        meta: None,
        loading: false,
        error: Some(message.to_owned()),
    }
}

#[requires(!message.is_empty())]
#[ensures(ret.error.as_ref().is_some_and(|error| error == message))]
pub(super) fn cukta_async_error_state(state: CuktaWebState, message: &str) -> CuktaAsyncPageState {
    CuktaAsyncPageState {
        state: Some(state),
        page: cukta_loading_page_data(message),
        meta: None,
        loading: false,
        error: Some(message.to_owned()),
    }
}

#[requires(!message.is_empty())]
#[ensures(ret.error.as_ref().is_some_and(|error| error == message))]
pub(super) fn vlacku_async_error_state(
    state: &VlackuWebState,
    message: &str,
) -> VlackuAsyncResultState {
    VlackuAsyncResultState {
        state: Some(state.clone()),
        result: vlacku_loading_result(state, message),
        meta: None,
        loading: false,
        error: Some(message.to_owned()),
    }
}

#[requires(!message.is_empty())]
#[ensures(ret.error.as_ref().is_some_and(|error| error == message))]
pub(super) fn gimfihi_async_error_state(
    state: &GimfihiWebState,
    message: &str,
) -> GimfihiAsyncResultState {
    GimfihiAsyncResultState {
        state: Some(state.clone()),
        result: gimfihi_empty_result(state),
        meta: None,
        loading: false,
        error: Some(message.to_owned()),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn gimfihi_generation_cache_key(state: &GimfihiWebState) -> String {
    let mut key_state = normalize_gimfihi_state(state);
    key_state.highlight = None;
    gimfihi_web_url("", &key_state)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_cached_result_for_state(
    base_path: &str,
    state: &GimfihiWebState,
    cached: GimfihiAsyncResultState,
) -> Option<GimfihiAsyncResultState> {
    let normalized = normalize_gimfihi_state(state);
    let output = cached.result.output.as_ref()?;
    if let Some(highlight) = normalized.highlight.as_deref()
        && output.winner.as_deref() != Some(highlight)
        && !output
            .candidates
            .iter()
            .any(|candidate| candidate.word == highlight)
    {
        return None;
    }
    let highlighted_output = gimfihi_output_with_highlight(output, normalized.highlight.as_deref());
    let result = GimfihiWebResult {
        state: normalized.clone(),
        output: Some(highlighted_output.clone()),
        preset_options: gimfihi_preset_options_for_state(&normalized),
        language_suggestions: gimfihi_language_suggestions(),
        errors: cached.result.errors.clone(),
    };
    Some(GimfihiAsyncResultState {
        state: Some(normalized.clone()),
        result,
        meta: Some(build_gimfihi_page_meta_from_output(
            base_path,
            &normalized,
            &highlighted_output,
        )),
        loading: false,
        error: None,
    })
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gimfihi_result_state_with_highlight(
    base_path: &str,
    state: &GimfihiWebState,
    current: &GimfihiAsyncResultState,
) -> Option<GimfihiAsyncResultState> {
    let normalized = normalize_gimfihi_state(state);
    let output = current.result.output.as_ref()?;
    let highlight = normalized.highlight.as_deref()?;
    if output.winner.as_deref() != Some(highlight)
        && !output
            .candidates
            .iter()
            .any(|candidate| candidate.word == highlight)
    {
        return None;
    }
    let highlighted_output = gimfihi_output_with_highlight(output, Some(highlight));
    let result = GimfihiWebResult {
        state: normalized.clone(),
        output: Some(highlighted_output.clone()),
        preset_options: gimfihi_preset_options_for_state(&normalized),
        language_suggestions: gimfihi_language_suggestions(),
        errors: current.result.errors.clone(),
    };
    Some(GimfihiAsyncResultState {
        state: Some(normalized.clone()),
        result,
        meta: Some(build_gimfihi_page_meta_from_output(
            base_path,
            &normalized,
            &highlighted_output,
        )),
        loading: false,
        error: None,
    })
}

#[requires(true)]
#[ensures(ret.candidates.len() == output.candidates.len())]
pub(super) fn gimfihi_output_with_highlight(
    output: &GimfihiOutput,
    highlight: Option<&str>,
) -> GimfihiOutput {
    let requested = highlight
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);
    let selected = requested
        .filter(|value| {
            output
                .candidates
                .iter()
                .any(|candidate| &candidate.word == value)
        })
        .or_else(|| output.winner.clone());
    let mut next = output.clone();
    next.highlighted_word = selected.clone();
    for candidate in &mut next.candidates {
        candidate.highlighted = selected
            .as_ref()
            .is_some_and(|highlighted| highlighted == &candidate.word);
    }
    next
}

#[requires(true)]
#[ensures(!ret || state.mode == VlackuWebMode::Meaning)]
pub(super) fn vlacku_semantic_result_is_pending(
    state: &VlackuWebState,
    semantic: &VlackuSemanticResultState,
) -> bool {
    state.mode == VlackuWebMode::Meaning
        && !state.query.trim().is_empty()
        && (semantic.state.as_ref() != Some(state) || semantic.loading)
}

#[requires(vlacku_semantic_result_is_pending(state, semantic))]
#[ensures(page.state.as_ref() == Some(state))]
#[ensures(page.loading)]
#[ensures(page.error.is_none())]
pub(super) fn apply_vlacku_semantic_pending_page(
    page: &mut VlackuAsyncResultState,
    base_path: &str,
    state: &VlackuWebState,
    semantic: &VlackuSemanticResultState,
) -> PageMeta {
    let meta = build_page_meta(base_path, &WebRoute::Vlacku(state.clone()));
    page.state = Some(state.clone());
    page.meta = Some(meta.clone());
    page.loading = true;
    page.error = None;
    if semantic.state.as_ref() == Some(state)
        && let Some(message) = &semantic.message
    {
        page.result = vlacku_loading_result(state, message);
    }
    meta
}

#[requires(true)]
#[ensures(true)]
pub(super) fn vlacku_compute_request(
    base_path: &str,
    state: &VlackuWebState,
    semantic: &VlackuSemanticResultState,
) -> WebComputeRequest {
    if state.mode != VlackuWebMode::Meaning {
        return WebComputeRequest::VlackuPage {
            base_path: base_path.to_owned(),
            state: state.clone(),
        };
    }
    let loading = vlacku_semantic_result_is_pending(state, semantic);
    let message = if semantic.state.as_ref() == Some(state) {
        semantic.message.clone()
    } else {
        None
    };
    let hits = if !loading && semantic.state.as_ref() == Some(state) {
        semantic.hits.clone()
    } else {
        Vec::new()
    };
    WebComputeRequest::VlackuSemanticPage {
        base_path: base_path.to_owned(),
        state: state.clone(),
        hits,
        message,
        loading,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn cukta_compute_request(
    base_path: &str,
    state: &CuktaWebState,
    semantic: &CuktaSemanticResultState,
) -> WebComputeRequest {
    let CuktaWebView::Search(search_state) = &state.view else {
        return WebComputeRequest::CuktaPage {
            base_path: base_path.to_owned(),
            state: state.clone(),
        };
    };
    if search_state.mode != CuktaWebMode::Meaning {
        return WebComputeRequest::CuktaPage {
            base_path: base_path.to_owned(),
            state: state.clone(),
        };
    }
    let loading = !search_state.query.trim().is_empty()
        && (semantic.state.as_ref() != Some(search_state) || semantic.loading);
    let message = if semantic.state.as_ref() == Some(search_state) {
        semantic.message.clone()
    } else {
        None
    };
    let hits = if !loading && semantic.state.as_ref() == Some(search_state) {
        semantic.hits.clone()
    } else {
        Vec::new()
    };
    WebComputeRequest::CuktaSemanticPage {
        base_path: base_path.to_owned(),
        state: state.clone(),
        hits,
        message,
        loading,
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn embedding_status_json() -> Result<String, String> {
    promise_to_string(js_embedding_status()).await
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn embedding_status_json() -> Result<String, String> {
    platform::run_native_task(native_embedding_status_json_result).await
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
pub(super) async fn embedding_status_json() -> Result<String, String> {
    Err("Native embeddings are not available for this platform yet.".to_owned())
}

#[requires(true)]
#[ensures(true)]
pub(super) async fn embedding_status_is_loading_model() -> bool {
    let Ok(json) = embedding_status_json().await else {
        return false;
    };
    let value = serde_json::from_str::<serde_json::Value>(&json).unwrap_or(serde_json::Value::Null);
    json_string(&value, "status").as_deref() == Some("loading-model")
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn embedding_setup_json(corpus_json: &str) -> Result<String, String> {
    promise_to_string(js_embedding_setup(corpus_json, web_embeddings_base_url())).await
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn embedding_setup_json(corpus_json: &str) -> Result<String, String> {
    let _ = corpus_json;
    let model_key = load_embedding_model_key();
    platform::run_native_task(move || native_embedding_setup_json_result(model_key)).await
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
pub(super) async fn embedding_setup_json(_corpus_json: &str) -> Result<String, String> {
    Err("Native embeddings are not available for this platform yet.".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn embedding_remove_json() -> Result<String, String> {
    promise_to_string(js_embedding_remove()).await
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn embedding_remove_json() -> Result<String, String> {
    let model_key = load_embedding_model_key();
    platform::run_native_task(move || native_embedding_remove_json_result(model_key)).await
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
pub(super) async fn embedding_remove_json() -> Result<String, String> {
    Err("Native embeddings are not available for this platform yet.".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn embedding_search_json(
    channel: &str,
    corpus_id: &str,
    query: &str,
    limit: usize,
    kind_filters: &[String],
) -> Result<String, String> {
    configure_embedding_model_key(&load_embedding_model_key());
    let kind_filters_json = serde_json::to_string(kind_filters).unwrap_or_else(|_| "[]".to_owned());
    promise_to_string(js_embedding_search(
        channel,
        corpus_id,
        query,
        limit,
        &kind_filters_json,
    ))
    .await
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn embedding_search_json(
    channel: &str,
    corpus_id: &str,
    query: &str,
    limit: usize,
    kind_filters: &[String],
) -> Result<String, String> {
    let _ = channel;
    let model_key = load_embedding_model_key();
    let corpus_id = corpus_id.to_owned();
    let query = query.to_owned();
    let kind_filters = kind_filters.to_owned();
    platform::run_native_task(move || {
        native_embedding_search_json_result(&model_key, &corpus_id, &query, limit, &kind_filters)
    })
    .await
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_some())]
pub(super) async fn embedding_search_json(
    _channel: &str,
    _corpus_id: &str,
    _query: &str,
    _limit: usize,
    _kind_filters: &[String],
) -> Result<String, String> {
    Err(SEMANTIC_SEARCH_SETUP_MESSAGE.to_owned())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
static NATIVE_EMBEDDING_SEARCH_WORKER: OnceLock<Mutex<Option<NativeEmbeddingSearchWorkerHandle>>> =
    OnceLock::new();

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
static NATIVE_EMBEDDING_SETUP_PROGRESS: OnceLock<Mutex<Option<jbotci_embeddings::SetupProgress>>> =
    OnceLock::new();

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|progress| !progress.kind.is_empty()))]
pub(super) fn native_embedding_setup_progress() -> Option<jbotci_embeddings::SetupProgress> {
    NATIVE_EMBEDDING_SETUP_PROGRESS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|progress| progress.clone())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!progress.kind.is_empty())]
#[ensures(true)]
pub(super) fn set_native_embedding_setup_progress(progress: jbotci_embeddings::SetupProgress) {
    if let Ok(mut stored) = NATIVE_EMBEDDING_SETUP_PROGRESS
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        *stored = Some(progress);
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn clear_native_embedding_setup_progress() {
    if let Ok(mut stored) = NATIVE_EMBEDDING_SETUP_PROGRESS
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        *stored = None;
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone)]
#[invariant(true)]
pub(super) struct NativeEmbeddingSearchWorkerHandle {
    pub(super) sender: std::sync::mpsc::Sender<NativeEmbeddingSearchCommand>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug)]
#[invariant(::Search { .. } => true)]
#[invariant(::Clear { .. } => true)]
pub(super) enum NativeEmbeddingSearchCommand {
    Search {
        model_key: String,
        corpus_id: String,
        query: String,
        count: usize,
        kind_filters: Vec<String>,
        response: std::sync::mpsc::Sender<Result<String, String>>,
    },
    Clear {
        response: std::sync::mpsc::Sender<Result<(), String>>,
    },
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl NativeEmbeddingSearchWorkerHandle {
    #[requires(!model_key.is_empty())]
    #[requires(!corpus_id.is_empty())]
    #[requires(count > 0)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
    fn search(
        &self,
        model_key: &str,
        corpus_id: &str,
        query: &str,
        count: usize,
        kind_filters: &[String],
    ) -> Result<String, String> {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.sender
            .send(NativeEmbeddingSearchCommand::Search {
                model_key: model_key.to_owned(),
                corpus_id: corpus_id.to_owned(),
                query: query.to_owned(),
                count,
                kind_filters: kind_filters.to_owned(),
                response: sender,
            })
            .map_err(|_| "native embedding search worker is unavailable".to_owned())?;
        receiver
            .recv()
            .map_err(|_| "native embedding search worker stopped before replying".to_owned())?
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
    fn clear(&self) -> Result<(), String> {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.sender
            .send(NativeEmbeddingSearchCommand::Clear { response: sender })
            .map_err(|_| "native embedding search worker is unavailable".to_owned())?;
        receiver
            .recv()
            .map_err(|_| "native embedding search worker stopped before replying".to_owned())?
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn native_embedding_status_json_result() -> Result<String, String> {
    let model_key = load_embedding_model_key();
    let spec = jbotci_embeddings::model_spec(&model_key)
        .ok_or_else(|| format!("unsupported native embedding model `{model_key}`"))?;
    let model_root = jbotci_embeddings::default_model_root().map_err(|error| error.to_string())?;
    let index_root = jbotci_embeddings::default_index_root().map_err(|error| error.to_string())?;
    let model_path = jbotci_embeddings::model_file_path(&model_root, &spec);
    let model_bytes = std::fs::metadata(&model_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let model_present = model_path.is_file() && model_bytes == spec.native_size_bytes;
    let pack_result = jbotci_embeddings::load_latest_pack(&index_root, &model_key);
    let index_bytes = pack_result
        .as_ref()
        .ok()
        .and_then(|(pack_dir, _)| directory_size(pack_dir).ok())
        .unwrap_or(0);
    let setup_progress = native_embedding_setup_progress();
    let (status, detail) = if let Some(progress) = &setup_progress {
        ("preparing", progress.detail.clone())
    } else if !model_path.is_file() {
        (
            "missing-model",
            format!(
                "No native embedding model is installed at `{}`.",
                model_path.display()
            ),
        )
    } else if !model_present {
        (
            "invalid-model",
            format!(
                "The installed native embedding model has {} bytes; expected {}.",
                model_bytes, spec.native_size_bytes
            ),
        )
    } else if let Err(error) = &pack_result {
        ("missing-index", error.to_string())
    } else {
        (
            "ready",
            "Native embeddings are ready for semantic search.".to_owned(),
        )
    };
    let mut json = serde_json::json!({
        "selectedModelKey": model_key,
        "effectiveModelKey": spec.model_key,
        "modelKey": spec.model_key,
        "modelLabel": embedding_model_label(&model_key),
        "modelBytes": model_bytes,
        "modelDtype": "Q4_K_M",
        "modelDevice": "llama.cpp",
        "indexBytes": index_bytes,
        "status": status,
        "detail": detail,
    });
    if let Some(progress) = setup_progress
        && let Ok(progress_value) = serde_json::to_value(progress)
    {
        json["progress"] = progress_value;
    }
    Ok(json.to_string())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!model_key.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn native_embedding_setup_json_result(model_key: String) -> Result<String, String> {
    let options = jbotci_embeddings::SetupOptions {
        model_key,
        force: false,
        index_dir: None,
        model_dir: None,
        ..jbotci_embeddings::SetupOptions::default()
    };
    clear_native_embedding_setup_progress();
    let mut progress = |progress| {
        set_native_embedding_setup_progress(progress);
    };
    let setup_result =
        jbotci_embeddings::native::setup_embeddings_with_progress(&options, &mut progress);
    clear_native_embedding_setup_progress();
    setup_result.map_err(|error| error.to_string())?;
    native_clear_embedding_search_service()?;
    native_embedding_status_json_result()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!model_key.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn native_embedding_remove_json_result(model_key: String) -> Result<String, String> {
    native_clear_embedding_search_service()?;
    let Some(spec) = jbotci_embeddings::model_spec(&model_key) else {
        return Err(format!("unsupported native embedding model `{model_key}`"));
    };
    let model_root = jbotci_embeddings::default_model_root().map_err(|error| error.to_string())?;
    let model_path = jbotci_embeddings::model_file_path(&model_root, &spec);
    if let Some(model_dir) = model_path.parent() {
        remove_dir_if_exists(model_dir)?;
    }
    let index_root = jbotci_embeddings::default_index_root().map_err(|error| error.to_string())?;
    let model_index_dir = index_root
        .join(jbotci_embeddings::INDEX_BASE_VERSION)
        .join("models")
        .join(&model_key);
    remove_dir_if_exists(&model_index_dir)?;
    remove_model_from_native_catalog(&index_root, &model_key)?;
    native_embedding_status_json_result()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!model_key.is_empty())]
#[requires(!corpus_id.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn native_embedding_search_json_result(
    model_key: &str,
    corpus_id: &str,
    query: &str,
    limit: usize,
    kind_filters: &[String],
) -> Result<String, String> {
    if query.trim().is_empty() {
        return Ok(serde_json::json!({ "hits": [] }).to_string());
    }
    let count = limit.max(1);
    native_embedding_search_worker_handle()?.search(
        model_key,
        corpus_id,
        query,
        count,
        kind_filters,
    )
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(service.model_key() == model_key)]
#[requires(!model_key.is_empty())]
#[requires(!corpus_id.is_empty())]
#[requires(!query.trim().is_empty())]
#[requires(count > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn native_embedding_worker_search_json(
    service: &mut jbotci_embeddings::native::NativeEmbeddingSearchService,
    model_key: &str,
    corpus_id: &str,
    query: &str,
    count: usize,
    kind_filters: &[String],
) -> Result<String, String> {
    match corpus_id {
        jbotci_embeddings::VLACKU_CORPUS_ID => {
            native_embedding_vlacku_search_json(service, query, count)
        }
        jbotci_embeddings::CUKTA_CORPUS_ID => {
            native_embedding_cukta_search_json(service, query, count, kind_filters)
        }
        _ => Err(format!("unsupported semantic corpus `{corpus_id}`")),
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!query.trim().is_empty())]
#[requires(count > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn native_embedding_vlacku_search_json(
    service: &mut jbotci_embeddings::native::NativeEmbeddingSearchService,
    query: &str,
    count: usize,
) -> Result<String, String> {
    let hits = service
        .semantic_vlacku_hits(query, count)
        .map_err(native_embedding_search_setup_error)?
        .into_iter()
        .map(|hit| {
            serde_json::json!({
                "id": hit.entry_index,
                "score": hit.score,
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({ "hits": hits }).to_string())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!query.trim().is_empty())]
#[requires(count > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn native_embedding_cukta_search_json(
    service: &mut jbotci_embeddings::native::NativeEmbeddingSearchService,
    query: &str,
    count: usize,
    kind_filters: &[String],
) -> Result<String, String> {
    let site = embedded_cll_site().map_err(|error| error.to_string())?;
    let chunks = jbotci_cll::cll_search_all_chunks(site);
    let targets = native_cukta_target_filter(kind_filters);
    let output = service
        .semantic_cukta_output(chunks, query, count, targets)
        .map_err(native_embedding_search_setup_error)?;
    let hits = output
        .matches
        .into_iter()
        .map(|hit| {
            let chunk_index = chunks
                .iter()
                .position(|chunk| chunk == &hit.chunk)
                .ok_or_else(|| "native CLL semantic search returned an unknown chunk".to_owned())?;
            let score = hit.similarity.ok_or_else(|| {
                "native CLL semantic search returned a hit without similarity".to_owned()
            })?;
            Ok(serde_json::json!({
                "id": chunk_index,
                "score": score,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(serde_json::json!({
        "hits": hits,
        "message": output.message,
    })
    .to_string())
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn native_embedding_search_worker_cell()
-> &'static Mutex<Option<NativeEmbeddingSearchWorkerHandle>> {
    NATIVE_EMBEDDING_SEARCH_WORKER.get_or_init(|| Mutex::new(None))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn native_embedding_search_worker_handle()
-> Result<NativeEmbeddingSearchWorkerHandle, String> {
    let mut guard = native_embedding_search_worker_cell()
        .lock()
        .map_err(|_| "native embedding search worker lock was poisoned".to_owned())?;
    if let Some(handle) = guard.as_ref() {
        return Ok(handle.clone());
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("jbotci-native-embedding-search".to_owned())
        .spawn(move || native_embedding_search_worker_loop(receiver))
        .map_err(|error| format!("failed to spawn native embedding search worker: {error}"))?;
    let handle = NativeEmbeddingSearchWorkerHandle { sender };
    *guard = Some(handle.clone());
    Ok(handle)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn native_clear_embedding_search_service() -> Result<(), String> {
    native_embedding_search_worker_handle()?.clear()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn native_embedding_search_worker_loop(
    receiver: std::sync::mpsc::Receiver<NativeEmbeddingSearchCommand>,
) {
    let mut service: Option<jbotci_embeddings::native::NativeEmbeddingSearchService> = None;
    while let Ok(command) = receiver.recv() {
        match command {
            NativeEmbeddingSearchCommand::Search {
                model_key,
                corpus_id,
                query,
                count,
                kind_filters,
                response,
            } => {
                let result = native_embedding_search_worker_command(
                    &mut service,
                    &model_key,
                    &corpus_id,
                    &query,
                    count,
                    &kind_filters,
                );
                let _ = response.send(result);
            }
            NativeEmbeddingSearchCommand::Clear { response } => {
                service = None;
                let _ = response.send(Ok(()));
            }
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(!model_key.is_empty())]
#[requires(!corpus_id.is_empty())]
#[requires(count > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn native_embedding_search_worker_command(
    service: &mut Option<jbotci_embeddings::native::NativeEmbeddingSearchService>,
    model_key: &str,
    corpus_id: &str,
    query: &str,
    count: usize,
    kind_filters: &[String],
) -> Result<String, String> {
    if service
        .as_ref()
        .is_none_or(|service| service.model_key() != model_key)
    {
        *service = Some(
            jbotci_embeddings::native::NativeEmbeddingSearchService::load(model_key, None, None)
                .map_err(native_embedding_search_setup_error)?,
        );
    }
    let service = service
        .as_mut()
        .ok_or_else(|| "native embedding search service was not initialized".to_owned())?;
    native_embedding_worker_search_json(service, model_key, corpus_id, query, count, kind_filters)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn native_cukta_target_filter(kind_filters: &[String]) -> jbotci_cll::CuktaTargetFilter {
    if kind_filters.is_empty() {
        return jbotci_cll::CuktaTargetFilter::default();
    }
    let sections = kind_filters
        .iter()
        .any(|filter| matches!(filter.trim(), "section" | "sections"));
    let paragraphs = kind_filters
        .iter()
        .any(|filter| matches!(filter.trim(), "paragraph" | "paragraphs"));
    let examples = kind_filters
        .iter()
        .any(|filter| matches!(filter.trim(), "example" | "examples"));
    if !sections && !paragraphs && !examples {
        return jbotci_cll::CuktaTargetFilter::default();
    }
    jbotci_cll::CuktaTargetFilter {
        sections,
        paragraphs,
        examples,
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn native_embedding_search_setup_error(
    error: jbotci_embeddings::EmbeddingError,
) -> String {
    match error {
        jbotci_embeddings::EmbeddingError::MissingCompatiblePack { .. }
        | jbotci_embeddings::EmbeddingError::InvalidModel { .. } => {
            SEMANTIC_SEARCH_SETUP_MESSAGE.to_owned()
        }
        other => other.to_string(),
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn remove_model_from_native_catalog(
    index_root: &Path,
    model_key: &str,
) -> Result<(), String> {
    let catalog_path =
        jbotci_embeddings::catalog_path(index_root).map_err(|error| error.to_string())?;
    if !catalog_path.is_file() {
        return Ok(());
    }
    let bytes = std::fs::read(&catalog_path)
        .map_err(|error| format!("failed to read `{}`: {error}", catalog_path.display()))?;
    let mut catalog: jbotci_embeddings::EmbeddingCatalog = serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse `{}`: {error}", catalog_path.display()))?;
    catalog.models.retain(|model| model.model_key != model_key);
    let bytes = serde_json::to_vec_pretty(&catalog)
        .map_err(|error| format!("failed to serialize `{}`: {error}", catalog_path.display()))?;
    std::fs::write(&catalog_path, bytes)
        .map_err(|error| format!("failed to write `{}`: {error}", catalog_path.display()))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) fn remove_dir_if_exists(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(path)
        .map_err(|error| format!("failed to remove `{}`: {error}", path.display()))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn directory_size(path: &Path) -> Result<u64, String> {
    if !path.exists() {
        return Ok(0);
    }
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("failed to inspect `{}`: {error}", path.display()))?;
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    let mut total = 0u64;
    for entry in std::fs::read_dir(path)
        .map_err(|error| format!("failed to list `{}`: {error}", path.display()))?
    {
        let entry =
            entry.map_err(|error| format!("failed to read `{}` entry: {error}", path.display()))?;
        total = total.saturating_add(directory_size(&entry.path())?);
    }
    Ok(total)
}

#[cfg(target_arch = "wasm32")]
#[requires(!channel.is_empty())]
#[requires(!request_json.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn compute_request_json(
    channel: &str,
    request_json: &str,
) -> Result<String, String> {
    promise_to_string(js_compute_request(channel, request_json)).await
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!channel.is_empty())]
#[requires(!request_json.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn compute_request_json(
    channel: &str,
    request_json: &str,
) -> Result<String, String> {
    let _ = channel;
    jbotci_web_core::run_web_compute_request_json(request_json).map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
#[requires(!channel.is_empty())]
#[ensures(true)]
pub(super) fn cancel_compute_channel(channel: &str) {
    js_compute_cancel(channel);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!channel.is_empty())]
#[ensures(true)]
pub(super) fn cancel_compute_channel(channel: &str) {
    let _ = channel;
}

#[cfg(target_arch = "wasm32")]
#[requires(!channel.is_empty())]
#[ensures(true)]
pub(super) fn cancel_embedding_channel(channel: &str) {
    js_embedding_cancel(channel);
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(!channel.is_empty())]
#[ensures(true)]
pub(super) fn cancel_embedding_channel(channel: &str) {
    let _ = channel;
}

#[requires(!channel.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn compute_request(
    channel: &str,
    request: WebComputeRequest,
) -> Result<WebComputeResponse, String> {
    let request_json = serde_json::to_string(&request).map_err(|error| error.to_string())?;
    let response_json = compute_request_json(channel, &request_json).await?;
    serde_json::from_str(&response_json).map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
pub(super) async fn promise_to_string(promise: js_sys::Promise) -> Result<String, String> {
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(js_value_to_string)?;
    value
        .as_string()
        .ok_or_else(|| "embedding worker returned a non-string response".to_owned())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn js_value_to_string(value: JsValue) -> String {
    value.as_string().unwrap_or_else(|| {
        js_sys::JSON::stringify(&value)
            .ok()
            .and_then(|text| text.as_string())
            .unwrap_or_else(|| "embedding worker request failed".to_owned())
    })
}

#[requires(true)]
#[ensures(!ret.status.is_empty())]
pub(super) fn embedding_settings_from_json(
    json: &str,
    fallback_detail: &str,
) -> EmbeddingSettingsState {
    let value = serde_json::from_str::<serde_json::Value>(json).unwrap_or(serde_json::Value::Null);
    let mut selected_model_key = json_string(&value, "selectedModelKey")
        .filter(|key| is_supported_embedding_model_key(key))
        .unwrap_or_else(load_embedding_model_key);
    let effective_model_key = json_string(&value, "effectiveModelKey")
        .or_else(|| json_string(&value, "modelKey"))
        .filter(|key| is_supported_embedding_model_key(key))
        .unwrap_or_else(|| selected_model_key.clone());
    let webgpu_available = value
        .get("webGpuAvailable")
        .and_then(serde_json::Value::as_bool);
    if webgpu_available == Some(false) && selected_model_key != F2LLM_80M_MODEL_KEY {
        selected_model_key = F2LLM_80M_MODEL_KEY.to_owned();
        save_embedding_model_key(&selected_model_key);
        configure_embedding_model_key(&selected_model_key);
    }
    let selected_model_label = embedding_model_label(&selected_model_key).to_owned();
    let status = json_string(&value, "status").unwrap_or_else(|| "unknown".to_owned());
    let detail = json_string(&value, "detail")
        .or_else(|| json_string(&value, "message"))
        .unwrap_or_else(|| fallback_detail.to_owned());
    let model_size = value
        .get("modelBytes")
        .and_then(serde_json::Value::as_u64)
        .map(human_bytes)
        .unwrap_or_else(|| "unknown".to_owned());
    let model_runtime = match (
        json_string(&value, "modelDtype"),
        json_string(&value, "modelDevice"),
    ) {
        (Some(dtype), Some(device)) => Some(format!("{dtype}/{device}")),
        (Some(dtype), None) => Some(dtype),
        _ => None,
    };
    let model_size = match model_runtime {
        Some(runtime) if model_size != "unknown" => format!("{model_size} ({runtime})"),
        Some(runtime) => runtime,
        None => model_size,
    };
    let model_size = json_string(&value, "modelLabel")
        .filter(|label| !label.is_empty())
        .map(|label| format!("{label}, {model_size}"))
        .unwrap_or(model_size);
    let index_size = value
        .get("indexBytes")
        .and_then(serde_json::Value::as_u64)
        .map(human_bytes)
        .unwrap_or_else(|| "unknown".to_owned());
    let progress = value.get("progress");
    let progress_kind = progress
        .and_then(|progress| json_string(progress, "kind"))
        .filter(|kind| !kind.is_empty());
    let progress_label = progress
        .and_then(|progress| json_string(progress, "label"))
        .filter(|label| !label.is_empty());
    let progress_loaded = progress
        .and_then(|progress| progress.get("loaded"))
        .and_then(serde_json::Value::as_u64);
    let progress_total = progress
        .and_then(|progress| progress.get("total"))
        .and_then(serde_json::Value::as_u64);
    let progress_percent = progress
        .and_then(|progress| progress.get("percent"))
        .and_then(serde_json::Value::as_u64)
        .map(|percent| percent.min(100) as u8);
    EmbeddingSettingsState {
        selected_model_key,
        selected_model_label,
        effective_model_key,
        webgpu_available,
        status,
        detail,
        model_size,
        index_size,
        progress_kind,
        progress_label,
        progress_loaded,
        progress_total,
        progress_percent,
        busy: false,
        remove_confirmation_open: false,
    }
}

#[requires(true)]
#[ensures(is_supported_embedding_model_key(&ret))]
pub(super) fn load_embedding_model_key() -> String {
    storage_get(EMBEDDING_MODEL_STORAGE_KEY)
        .filter(|key| is_supported_embedding_model_key(key))
        .unwrap_or_else(preferred_embedding_model_key)
}

#[requires(is_supported_embedding_model_key(model_key))]
#[ensures(true)]
pub(super) fn save_embedding_model_key(model_key: &str) {
    storage_set(EMBEDDING_MODEL_STORAGE_KEY, model_key);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn is_supported_embedding_model_key(model_key: &str) -> bool {
    embedding_model_options()
        .iter()
        .any(|option| option.key == model_key)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn embedding_model_label(model_key: &str) -> &'static str {
    embedding_model_options()
        .iter()
        .find(|option| option.key == model_key)
        .map(|option| option.label)
        .unwrap_or("F2LLM v2 330M")
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn embedding_model_options() -> &'static [EmbeddingModelOption] {
    WEB_EMBEDDING_MODEL_OPTIONS
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn embedding_model_options() -> &'static [EmbeddingModelOption] {
    NATIVE_EMBEDDING_MODEL_OPTIONS
}

#[requires(!status.is_empty())]
#[requires(true)]
#[ensures(!ret.status.is_empty())]
pub(super) fn embedding_settings_error_state(
    previous: &EmbeddingSettingsState,
    status: &str,
    detail: String,
) -> EmbeddingSettingsState {
    let detail = if detail.is_empty() {
        "Embedding request failed.".to_owned()
    } else {
        detail
    };
    EmbeddingSettingsState {
        selected_model_key: previous.selected_model_key.clone(),
        selected_model_label: previous.selected_model_label.clone(),
        effective_model_key: previous.effective_model_key.clone(),
        webgpu_available: previous.webgpu_available,
        status: status.to_owned(),
        detail,
        model_size: "unknown".to_owned(),
        index_size: "unknown".to_owned(),
        progress_kind: None,
        progress_label: None,
        progress_loaded: None,
        progress_total: None,
        progress_percent: None,
        busy: false,
        remove_confirmation_open: false,
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_vlacku_semantic_search_json(
    json: &str,
) -> (Vec<VlackuSemanticSearchHit>, Option<String>) {
    let value = serde_json::from_str::<serde_json::Value>(json).unwrap_or(serde_json::Value::Null);
    let message = json_string(&value, "message");
    let hits = value
        .get("hits")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|hit| {
            Some(VlackuSemanticSearchHit {
                entry_index: hit.get("id")?.as_u64()? as usize,
                score: hit.get("score")?.as_f64()? as f32,
            })
        })
        .collect();
    (hits, message)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn parse_cukta_semantic_search_json(
    json: &str,
) -> (Vec<CuktaSemanticSearchHit>, Option<String>) {
    let value = serde_json::from_str::<serde_json::Value>(json).unwrap_or(serde_json::Value::Null);
    let message = json_string(&value, "message");
    let hits = value
        .get("hits")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|hit| {
            Some(CuktaSemanticSearchHit {
                chunk_index: hit.get("id")?.as_u64()? as usize,
                score: hit.get("score")?.as_f64()? as f32,
            })
        })
        .collect();
    (hits, message)
}

#[requires(true)]
#[ensures(true)]
pub(super) fn json_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn human_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes < 1024 * 1024 {
        format!("{bytes} B")
    } else {
        format!("{:.1} MiB", bytes as f64 / MIB)
    }
}

#[requires(!name.is_empty())]
#[ensures(true)]
pub(super) fn render_disabled(name: &str) -> Element {
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
pub(super) fn repeated_parse_tree_template(count: usize) -> String {
    format!("repeat({count}, max-content)")
}

#[requires(true)]
#[ensures(true)]
pub(super) fn tree_row_is_elided(row: &GentufaTreeRow) -> bool {
    !row.cells.is_empty() && row.cells.iter().all(|cell| cell.is_elided)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn block_class(block: &GentufaBlock) -> String {
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
pub(super) fn web_options(
    settings: UserSettings,
    display: GentufaDisplayState,
    view_mode: GentufaWebViewMode,
    dialect: String,
    dialect_settings: &DialectSettings,
) -> GentufaWebOptions {
    let dialect = resolved_dialect_formula_for_request(dialect_settings, &dialect);
    GentufaWebOptions {
        dialect: if dialect.trim().is_empty() {
            None
        } else {
            Some(dialect)
        },
        view_mode,
        script: settings.script,
        show_elided: display.show_elided,
        show_glosses: display.show_glosses,
        show_definitions: false,
        error_context_depth: settings.error_context_depth,
        phonemes: PhonemeRenderOptions {
            mark_stress: settings.stress,
            mark_glides: settings.glides,
        },
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn resolved_dialect_formula_for_request(
    settings: &DialectSettings,
    dialect: &str,
) -> String {
    if dialect.trim().is_empty() {
        return String::new();
    }
    parse_dialect_selection_formula(settings, dialect)
        .map(|definition| dialect_definition_to_text(&definition))
        .unwrap_or_else(|_| dialect.to_owned())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_theme(settings: &mut Signal<UserSettings>, theme: ThemeMode) {
    let mut next = *settings.read();
    next.theme = theme;
    settings.set(next);
    save_settings(&next);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_script(settings: &mut Signal<UserSettings>, script: GentufaScript) {
    let mut next = *settings.read();
    next.script = script;
    settings.set(next);
    save_settings(&next);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_stress_mark(settings: &mut Signal<UserSettings>, stress: StressMark) {
    let mut next = *settings.read();
    next.stress = stress;
    settings.set(next);
    save_settings(&next);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_glide_mark(settings: &mut Signal<UserSettings>, glides: GlideMark) {
    let mut next = *settings.read();
    next.glides = glides;
    settings.set(next);
    save_settings(&next);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn set_error_context_depth(settings: &mut Signal<UserSettings>, depth: usize) {
    let mut next = *settings.read();
    next.error_context_depth = depth;
    settings.set(next);
    save_settings(&next);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn toggle_elided(display: &mut Signal<GentufaDisplayState>) {
    let mut next = *display.read();
    next.show_elided = !next.show_elided;
    display.set(next);
}

#[requires(true)]
#[ensures(true)]
pub(super) fn toggle_glosses(display: &mut Signal<GentufaDisplayState>) {
    let mut next = *display.read();
    next.show_glosses = !next.show_glosses;
    display.set(next);
}

#[requires(true)]
#[ensures(active -> ret.contains("active"))]
#[ensures(loading -> ret.contains("is-loading"))]
pub(super) fn topbar_link_class(active: bool, loading: bool) -> String {
    class_names(
        "app-topbar-link",
        &[("active", active), ("is-loading", loading)],
    )
}

#[requires(true)]
#[ensures(active -> ret.contains("is-active"))]
pub(super) fn topbar_activity_class(active: bool) -> String {
    class_names(
        "app-topbar-center app-topbar-activity",
        &[("is-active", active)],
    )
}

#[requires(true)]
#[ensures(active -> ret.contains("active"))]
pub(super) fn view_tab_class(active: bool) -> &'static str {
    if active {
        "view-tab active"
    } else {
        "view-tab"
    }
}

#[requires(true)]
#[ensures(active -> ret.contains("is-active"))]
pub(super) fn theme_button_class(active: bool) -> &'static str {
    if active {
        "theme-btn is-active"
    } else {
        "theme-btn"
    }
}

#[requires(true)]
#[ensures(active -> ret.contains("is-active"))]
pub(super) fn orthography_button_class(active: bool, zbalermorna: bool) -> &'static str {
    match (active, zbalermorna) {
        (true, true) => "theme-btn orthography-btn is-zbalermorna is-active",
        (true, false) => "theme-btn orthography-btn is-active",
        (false, true) => "theme-btn orthography-btn is-zbalermorna",
        (false, false) => "theme-btn orthography-btn",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn pressed_attr(active: bool) -> &'static str {
    if active { "true" } else { "false" }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn theme_class(theme: ThemeMode) -> &'static str {
    match theme {
        ThemeMode::Auto => "auto",
        ThemeMode::Day => "day",
        ThemeMode::Night => "night",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn script_class(script: GentufaScript) -> &'static str {
    match script {
        GentufaScript::Latin => "latin",
        GentufaScript::Cyrillic => "cyrillic",
        GentufaScript::Zbalermorna => "zbalermorna",
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn install_browser_dom_handlers(
    jvozba_available: Signal<bool>,
    topbar_settings_layout: Signal<TopbarSettingsLayout>,
    topbar_settings_open: Signal<bool>,
    topbar_nav_layout: Signal<TopbarNavLayout>,
    cukta_toc_forced_autohide: Signal<bool>,
) {
    let should_install = BROWSER_STATE_HANDLERS_INSTALLED.with(|installed| {
        if installed.get() {
            false
        } else {
            installed.set(true);
            true
        }
    });
    if !should_install {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let tooltip_pointer_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        position_dictionary_tooltip_from_event(&event);
    }) as Box<dyn FnMut(_)>);
    let _ = document.add_event_listener_with_callback(
        "mouseover",
        tooltip_pointer_closure.as_ref().unchecked_ref(),
    );
    tooltip_pointer_closure.forget();

    let tooltip_focus_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        position_dictionary_tooltip_from_event(&event);
    }) as Box<dyn FnMut(_)>);
    let _ = document.add_event_listener_with_callback(
        "focusin",
        tooltip_focus_closure.as_ref().unchecked_ref(),
    );
    tooltip_focus_closure.forget();

    let page_find_keydown_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if event_is_page_find_shortcut(&event) {
            event.prevent_default();
            focus_page_find_input();
        }
    }) as Box<dyn FnMut(_)>);
    let _ = document.add_event_listener_with_callback_and_bool(
        "keydown",
        page_find_keydown_closure.as_ref().unchecked_ref(),
        true,
    );
    page_find_keydown_closure.forget();

    let resize_layout = topbar_settings_layout;
    let resize_open = topbar_settings_open;
    let resize_nav_layout = topbar_nav_layout;
    let resize_jvozba_available = jvozba_available;
    let resize_cukta_toc_forced_autohide = cukta_toc_forced_autohide;
    let resize_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        schedule_gentufa_block_reference_layout();
        schedule_gentufa_tree_layout();
        schedule_topbar_settings_layout_measure(resize_layout, resize_open, resize_nav_layout);
        update_vlacku_jvozba_availability(resize_jvozba_available);
        update_cukta_toc_forced_autohide(resize_cukta_toc_forced_autohide);
        schedule_vlacku_jvozba_pane_metrics_sync();
    }) as Box<dyn FnMut(_)>);
    let _ =
        window.add_event_listener_with_callback("resize", resize_closure.as_ref().unchecked_ref());
    resize_closure.forget();

    let load_layout = topbar_settings_layout;
    let load_open = topbar_settings_open;
    let load_nav_layout = topbar_nav_layout;
    let load_jvozba_available = jvozba_available;
    let load_cukta_toc_forced_autohide = cukta_toc_forced_autohide;
    let window_load_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        schedule_gentufa_block_reference_layout();
        schedule_gentufa_tree_layout();
        schedule_topbar_settings_layout_measure(load_layout, load_open, load_nav_layout);
        update_vlacku_jvozba_availability(load_jvozba_available);
        update_cukta_toc_forced_autohide(load_cukta_toc_forced_autohide);
        schedule_vlacku_jvozba_pane_metrics_sync();
    }) as Box<dyn FnMut(_)>);
    let _ = window
        .add_event_listener_with_callback("load", window_load_closure.as_ref().unchecked_ref());
    window_load_closure.forget();

    let stylesheet_layout = topbar_settings_layout;
    let stylesheet_open = topbar_settings_open;
    let stylesheet_nav_layout = topbar_nav_layout;
    let stylesheet_load_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if event_target_is_stylesheet_link(&event) {
            schedule_gentufa_block_reference_layout();
            schedule_gentufa_tree_layout();
            schedule_topbar_settings_layout_measure(
                stylesheet_layout,
                stylesheet_open,
                stylesheet_nav_layout,
            );
            schedule_vlacku_jvozba_pane_metrics_sync();
        }
    }) as Box<dyn FnMut(_)>);
    let _ = document.add_event_listener_with_callback_and_bool(
        "load",
        stylesheet_load_closure.as_ref().unchecked_ref(),
        true,
    );
    stylesheet_load_closure.forget();
    schedule_gentufa_block_reference_layout_after_fonts_ready(&document);
    schedule_gentufa_tree_layout_after_fonts_ready(&document);
    schedule_topbar_settings_layout_after_fonts_ready(
        &document,
        topbar_settings_layout,
        topbar_settings_open,
        topbar_nav_layout,
    );
    schedule_vlacku_jvozba_pane_metrics_after_fonts_ready(&document);

    let document_scroll_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        save_current_scroll_position();
    }) as Box<dyn FnMut(_)>);
    let _ = document.add_event_listener_with_callback_and_bool(
        "scroll",
        document_scroll_closure.as_ref().unchecked_ref(),
        true,
    );
    document_scroll_closure.forget();

    let window_scroll_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        save_current_scroll_position();
    }) as Box<dyn FnMut(_)>);
    let _ = window
        .add_event_listener_with_callback("scroll", window_scroll_closure.as_ref().unchecked_ref());
    window_scroll_closure.forget();
    restore_scroll_for_current_url();
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn install_browser_dom_handlers(
    jvozba_available: Signal<bool>,
    topbar_settings_layout: Signal<TopbarSettingsLayout>,
    topbar_settings_open: Signal<bool>,
    topbar_nav_layout: Signal<TopbarNavLayout>,
    cukta_toc_forced_autohide: Signal<bool>,
) {
    if DESKTOP_DOM_HANDLERS_INSTALLED.set(()).is_err() {
        return;
    }
    install_desktop_tooltip_bridge();
    spawn(async move {
        let mut eval = document::eval(
            r#"
            window.addEventListener("keydown", (event) => {
                if ((event.ctrlKey || event.metaKey) && !event.altKey && String(event.key || "").toLowerCase() === "f") {
                    event.preventDefault();
                    const input = document.getElementById("app-page-find-input");
                    if (input) {
                        input.focus();
                        if (typeof input.select === "function") {
                            input.select();
                        }
                    }
                }
            }, true);
            const sendLayout = () => {
                try {
                    dioxus.send("layout");
                } catch (_error) {
                }
            };
            const scheduleLayout = () => requestAnimationFrame(sendLayout);
            window.addEventListener("resize", scheduleLayout);
            window.addEventListener("load", sendLayout);
            for (const link of Array.from(document.querySelectorAll('link[rel~="stylesheet"]'))) {
                link.addEventListener("load", scheduleLayout, { once: true });
            }
            if (document.fonts && document.fonts.ready) {
                document.fonts.ready.then(sendLayout).catch(() => {});
            }
            scheduleLayout();
            await new Promise(() => {});
            "#,
        );
        while eval.recv::<String>().await.is_ok() {
            schedule_gentufa_block_reference_layout();
            schedule_gentufa_tree_layout();
            schedule_topbar_settings_layout_measure(
                topbar_settings_layout,
                topbar_settings_open,
                topbar_nav_layout,
            );
            update_vlacku_jvozba_availability(jvozba_available);
            update_cukta_toc_forced_autohide(cukta_toc_forced_autohide);
            schedule_vlacku_jvozba_pane_metrics_sync();
        }
    });
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
pub(super) fn install_browser_dom_handlers(
    jvozba_available: Signal<bool>,
    topbar_settings_layout: Signal<TopbarSettingsLayout>,
    topbar_settings_open: Signal<bool>,
    topbar_nav_layout: Signal<TopbarNavLayout>,
    cukta_toc_forced_autohide: Signal<bool>,
) {
    let _ = (
        jvozba_available,
        topbar_settings_layout,
        topbar_settings_open,
        topbar_nav_layout,
        cukta_toc_forced_autohide,
    );
}

#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_gentufa_textarea_resize() {
    platform::schedule_layout_task_after_delay(0, || async {
        resize_gentufa_textarea();
    });
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn resize_gentufa_textarea() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Some(element) = document.get_element_by_id("gentufa-text") else {
        return;
    };
    let Some(textarea) = element.dyn_ref::<web_sys::HtmlTextAreaElement>() else {
        return;
    };
    let textarea_html: &web_sys::HtmlElement = textarea.unchecked_ref();
    let style = textarea_html.style();
    let _ = style.remove_property("height");
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn resize_gentufa_textarea() {}

#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_gentufa_block_reference_layout() {
    platform::schedule_layout_passes(
        GENTUFA_BLOCK_REFERENCE_LAYOUT_DELAY_MS,
        GENTUFA_BLOCK_REFERENCE_LAYOUT_FRAME_PASSES,
        || async {
            adjust_gentufa_block_reference_layout_scheduled().await;
        },
    );
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) async fn adjust_gentufa_block_reference_layout_scheduled() {
    adjust_gentufa_block_reference_layout();
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn adjust_gentufa_block_reference_layout_scheduled() {
    adjust_gentufa_block_reference_layout_desktop().await;
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn adjust_gentufa_block_reference_layout_scheduled() {}

#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_gentufa_tree_layout() {
    platform::schedule_layout_passes(
        GENTUFA_TREE_LAYOUT_DELAY_MS,
        GENTUFA_TREE_LAYOUT_FRAME_PASSES,
        || async {
            layout_gentufa_tree_lines_scheduled().await;
        },
    );
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) async fn layout_gentufa_tree_lines_scheduled() {
    layout_gentufa_tree_lines();
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn layout_gentufa_tree_lines_scheduled() {
    layout_gentufa_tree_lines_desktop().await;
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn layout_gentufa_tree_lines_scheduled() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_gentufa_block_reference_layout_after_fonts_ready(
    document: &web_sys::Document,
) {
    platform::schedule_after_fonts_ready(document, || async {
        adjust_gentufa_block_reference_layout();
    });
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_gentufa_tree_layout_after_fonts_ready(document: &web_sys::Document) {
    platform::schedule_after_fonts_ready(document, || async {
        layout_gentufa_tree_lines();
    });
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_gentufa_tree_layout_after_fonts_ready(document: &()) {
    let _ = document;
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[invariant(true)]
pub(super) struct GentufaTreeLineAnchor {
    pub(super) parent_id: Option<usize>,
    pub(super) depth: usize,
    pub(super) label_left: f64,
    pub(super) label_center_y: f64,
    pub(super) row_top: f64,
    pub(super) row_bottom: f64,
}

#[requires(true)]
#[ensures(true)]
pub(super) fn gentufa_tree_line_paths(
    ordered_anchors: &[(usize, GentufaTreeLineAnchor)],
    table_bottom: f64,
) -> Vec<String> {
    let mut parents_with_children = BTreeSet::new();
    for (_, anchor) in ordered_anchors {
        if let Some(parent_id) = anchor.parent_id {
            parents_with_children.insert(parent_id);
        }
    }
    let mut paths = Vec::new();
    for (index, (node_id, anchor)) in ordered_anchors.iter().enumerate() {
        if !parents_with_children.contains(node_id) {
            continue;
        }
        let end_y = ordered_anchors
            .iter()
            .skip(index + 1)
            .find_map(|(_, candidate)| {
                (candidate.depth <= anchor.depth).then_some(candidate.row_top)
            })
            .unwrap_or(table_bottom.max(anchor.row_bottom));
        if end_y <= anchor.label_center_y {
            continue;
        }
        paths.push(gentufa_tree_line_path_data(
            anchor.label_left,
            anchor.label_center_y,
            end_y,
        ));
    }
    paths
}

#[requires(end_y >= start_y)]
#[ensures(!ret.is_empty())]
pub(super) fn gentufa_tree_line_path_data(x: f64, start_y: f64, end_y: f64) -> String {
    format!("M {x:.3} {start_y:.3} V {end_y:.3}")
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn layout_gentufa_tree_lines() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(Some(wrap)) = document.query_selector(".parse-page .table-wrap") else {
        return;
    };
    let Ok(Some(svg)) = wrap.query_selector(".tree-lines") else {
        return;
    };
    let Ok(Some(table)) = wrap.query_selector(".parse-table") else {
        clear_svg_children(&svg);
        return;
    };
    let Some(wrap_html) = wrap.dyn_ref::<web_sys::HtmlElement>() else {
        return;
    };
    let Some(table_html) = table.dyn_ref::<web_sys::HtmlElement>() else {
        return;
    };
    clear_svg_children(&svg);
    let wrap_rect = wrap.get_bounding_client_rect();
    let table_rect = table.get_bounding_client_rect();
    let scroll_left = f64::from(wrap_html.scroll_left());
    let scroll_top = f64::from(wrap_html.scroll_top());
    let width = f64::from(wrap_html.scroll_width())
        .max(f64::from(table_html.scroll_width()))
        .max(table_rect.right() - wrap_rect.left() + scroll_left);
    let height = f64::from(wrap_html.scroll_height())
        .max(f64::from(table_html.scroll_height()))
        .max(table_rect.bottom() - wrap_rect.top() + scroll_top);
    if width <= 0.0 || height <= 0.0 {
        return;
    }
    let _ = svg.set_attribute("width", &format!("{width:.3}"));
    let _ = svg.set_attribute("height", &format!("{height:.3}"));
    let _ = svg.set_attribute("viewBox", &format!("0 0 {width:.3} {height:.3}"));
    let Ok(row_nodes) = table.query_selector_all("tbody tr.tree-row") else {
        return;
    };
    let mut ordered_anchors = Vec::new();
    for index in 0..row_nodes.length() {
        let Some(node) = row_nodes.item(index) else {
            continue;
        };
        let Ok(row) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(node_id) = element_usize_attr(&row, "data-node-id") else {
            continue;
        };
        let Some(anchor) = tree_line_anchor_for_row(&row, &wrap, wrap_html) else {
            continue;
        };
        ordered_anchors.push((node_id, anchor));
    }
    let table_bottom = table_rect.bottom() - wrap_rect.top() + scroll_top;
    for path_data in gentufa_tree_line_paths(&ordered_anchors, table_bottom) {
        append_gentufa_tree_line_path(&document, &svg, &path_data);
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
pub(super) struct DesktopGentufaTreeMetrics {
    pub(super) width: f64,
    pub(super) height: f64,
    pub(super) table_bottom: f64,
    pub(super) anchors: Vec<DesktopGentufaTreeAnchorMetrics>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[invariant(true)]
pub(super) struct DesktopGentufaTreeAnchorMetrics {
    pub(super) node_id: usize,
    pub(super) parent_id: Option<usize>,
    pub(super) depth: usize,
    pub(super) label_left: f64,
    pub(super) label_center_y: f64,
    pub(super) row_top: f64,
    pub(super) row_bottom: f64,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[invariant(true)]
pub(super) struct DesktopGentufaTreeLayout {
    pub(super) width: f64,
    pub(super) height: f64,
    pub(super) paths: Vec<String>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn layout_gentufa_tree_lines_desktop() {
    let Some(metrics) = measure_gentufa_tree_layout_desktop().await else {
        return;
    };
    let layout = gentufa_tree_layout_from_metrics(metrics);
    apply_gentufa_tree_layout_desktop(layout).await;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn gentufa_tree_layout_from_metrics(
    metrics: DesktopGentufaTreeMetrics,
) -> DesktopGentufaTreeLayout {
    let ordered_anchors = metrics
        .anchors
        .into_iter()
        .map(|anchor| {
            (
                anchor.node_id,
                GentufaTreeLineAnchor {
                    parent_id: anchor.parent_id,
                    depth: anchor.depth,
                    label_left: anchor.label_left,
                    label_center_y: anchor.label_center_y,
                    row_top: anchor.row_top,
                    row_bottom: anchor.row_bottom,
                },
            )
        })
        .collect::<Vec<_>>();
    DesktopGentufaTreeLayout {
        width: metrics.width,
        height: metrics.height,
        paths: gentufa_tree_line_paths(&ordered_anchors, metrics.table_bottom),
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn measure_gentufa_tree_layout_desktop() -> Option<DesktopGentufaTreeMetrics> {
    document::eval(
        r#"
        const wrap = document.querySelector(".parse-page .table-wrap");
        const svg = wrap && wrap.querySelector(".tree-lines");
        if (!wrap || !svg) {
            return null;
        }
        const table = wrap.querySelector(".parse-table");
        if (!table) {
            return {
                width: 0,
                height: 0,
                table_bottom: 0,
                anchors: [],
            };
        }
        const wrapRect = wrap.getBoundingClientRect();
        const tableRect = table.getBoundingClientRect();
        const scrollLeft = Number(wrap.scrollLeft || 0);
        const scrollTop = Number(wrap.scrollTop || 0);
        const width = Math.max(
            Number(wrap.scrollWidth || 0),
            Number(table.scrollWidth || 0),
            tableRect.right - wrapRect.left + scrollLeft,
        );
        const height = Math.max(
            Number(wrap.scrollHeight || 0),
            Number(table.scrollHeight || 0),
            tableRect.bottom - wrapRect.top + scrollTop,
        );
        const parseOptionalInt = (value) => {
            if (value === null || value === undefined || value === "") {
                return null;
            }
            const parsed = Number.parseInt(value, 10);
            return Number.isFinite(parsed) ? parsed : null;
        };
        const anchors = [];
        for (const row of Array.from(table.querySelectorAll("tbody tr.tree-row"))) {
            const nodeId = parseOptionalInt(row.getAttribute("data-node-id"));
            const depth = parseOptionalInt(row.getAttribute("data-depth"));
            const label = row.querySelector(".node-label");
            if (nodeId === null || depth === null || !label) {
                continue;
            }
            const labelRect = label.getBoundingClientRect();
            const rowRect = row.getBoundingClientRect();
            anchors.push({
                node_id: nodeId,
                parent_id: parseOptionalInt(row.getAttribute("data-parent-id")),
                depth,
                label_left: labelRect.left - wrapRect.left + scrollLeft,
                label_center_y: labelRect.top - wrapRect.top + scrollTop + labelRect.height / 2,
                row_top: rowRect.top - wrapRect.top + scrollTop,
                row_bottom: rowRect.bottom - wrapRect.top + scrollTop,
            });
        }
        return {
            width,
            height,
            table_bottom: tableRect.bottom - wrapRect.top + scrollTop,
            anchors,
        };
        "#,
    )
    .join()
    .await
    .ok()
    .flatten()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn apply_gentufa_tree_layout_desktop(layout: DesktopGentufaTreeLayout) {
    let Ok(layout_json) = serde_json::to_string(&layout) else {
        return;
    };
    let script = format!(
        r#"
        const layout = {layout_json};
        const svg = document.querySelector(".parse-page .table-wrap .tree-lines");
        if (svg) {{
            while (svg.firstChild) {{
                svg.removeChild(svg.firstChild);
            }}
            if (Number(layout.width) > 0 && Number(layout.height) > 0) {{
                svg.setAttribute("width", Number(layout.width).toFixed(3));
                svg.setAttribute("height", Number(layout.height).toFixed(3));
                svg.setAttribute("viewBox", `0 0 ${{Number(layout.width).toFixed(3)}} ${{Number(layout.height).toFixed(3)}}`);
                for (const d of layout.paths) {{
                    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
                    path.setAttribute("class", "tree-line");
                    path.setAttribute("d", d);
                    svg.appendChild(path);
                }}
            }}
        }}
        return null;
        "#
    );
    let _ = document::eval(&script).await;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn clear_svg_children(svg: &web_sys::Element) {
    while let Some(child) = svg.first_child() {
        let _ = svg.remove_child(&child);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn tree_line_anchor_for_row(
    row: &web_sys::Element,
    wrap: &web_sys::Element,
    wrap_html: &web_sys::HtmlElement,
) -> Option<GentufaTreeLineAnchor> {
    let label = row.query_selector(".node-label").ok().flatten()?;
    let label_rect = label.get_bounding_client_rect();
    let row_rect = row.get_bounding_client_rect();
    let wrap_rect = wrap.get_bounding_client_rect();
    let scroll_left = f64::from(wrap_html.scroll_left());
    let scroll_top = f64::from(wrap_html.scroll_top());
    Some(GentufaTreeLineAnchor {
        parent_id: element_usize_attr(row, "data-parent-id"),
        depth: element_usize_attr(row, "data-depth")?,
        label_left: label_rect.left() - wrap_rect.left() + scroll_left,
        label_center_y: label_rect.top() - wrap_rect.top() + scroll_top + label_rect.height() / 2.0,
        row_top: row_rect.top() - wrap_rect.top() + scroll_top,
        row_bottom: row_rect.bottom() - wrap_rect.top() + scroll_top,
    })
}

#[cfg(target_arch = "wasm32")]
#[requires(!d.is_empty())]
#[ensures(true)]
pub(super) fn append_gentufa_tree_line_path(
    document: &web_sys::Document,
    svg: &web_sys::Element,
    d: &str,
) {
    let Ok(path) = document.create_element_ns(Some("http://www.w3.org/2000/svg"), "path") else {
        return;
    };
    let _ = path.set_attribute("class", "tree-line");
    let _ = path.set_attribute("d", d);
    let _ = svg.append_child(&path);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn event_target_is_stylesheet_link(event: &web_sys::Event) -> bool {
    let Some(element) = event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
    else {
        return false;
    };
    if !element.tag_name().eq_ignore_ascii_case("link") {
        return false;
    }
    element.get_attribute("rel").is_some_and(|rel| {
        rel.split_ascii_whitespace()
            .any(|part| part.eq_ignore_ascii_case("stylesheet"))
    })
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn adjust_gentufa_block_reference_layout() {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(nodes) = document.query_selector_all(".parse-page .block") else {
        return;
    };
    let mut blocks = Vec::new();
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(block) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        reset_block_reference_fit_width(&block);
        blocks.push(block);
    }
    reset_block_reference_height_sizers(&document);
    for block in &blocks {
        adjust_block_reference_fit_width(block);
    }
    let row_heights = measured_block_row_heights(&document);
    if row_heights.is_empty() {
        return;
    }
    let mut row_growths = vec![0.0; row_heights.len()];
    let mut indexed_blocks = blocks
        .into_iter()
        .filter_map(|block| {
            let (row, row_span, bottom_row) = block_row_range_for_element(&block)?;
            Some((bottom_row, row, row_span, block))
        })
        .collect::<Vec<_>>();
    indexed_blocks.sort_by_key(|(bottom_row, row, _, _)| (*bottom_row, *row));
    for (_, row, row_span, block) in indexed_blocks {
        if let Some((bottom_row, deficit)) =
            block_reference_height_growth(&block, row, row_span, &row_growths)
            && bottom_row < row_growths.len()
        {
            row_growths[bottom_row] += deficit;
        }
    }
    apply_block_reference_height_sizers(&document, &row_heights, &row_growths);
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn adjust_gentufa_block_reference_layout_desktop() {
    let Some(fit_metrics) = measure_block_reference_fit_metrics_desktop().await else {
        return;
    };
    let fit_updates = block_reference_fit_updates(fit_metrics);
    apply_block_reference_fit_updates_desktop(&fit_updates).await;
    let Some(height_metrics) = measure_block_reference_height_metrics_desktop().await else {
        return;
    };
    if height_metrics.row_heights.is_empty() {
        return;
    }
    let row_growths = block_reference_row_growths(&height_metrics);
    apply_block_reference_height_updates_desktop(BlockReferenceHeightUpdates {
        row_heights: height_metrics.row_heights,
        row_growths,
    })
    .await;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn measure_block_reference_fit_metrics_desktop()
-> Option<Vec<BlockReferenceFitMetrics>> {
    document::eval(
        r#"
        const parseMetrics = [];
        const rectFor = (element) => {
            const rect = element.getBoundingClientRect();
            return {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            };
        };
        for (const block of Array.from(document.querySelectorAll(".parse-page .block"))) {
            block.style.removeProperty("--block-reference-fit-width");
        }
        for (const sizer of Array.from(document.querySelectorAll(".parse-page .block-row-height-sizer"))) {
            sizer.style.removeProperty("height");
            sizer.style.removeProperty("min-height");
        }
        for (const block of Array.from(document.querySelectorAll(".parse-page .block"))) {
            const blockId = block.getAttribute("data-block-id") || "";
            const label = block.querySelector(".block-label-text");
            const referenceTarget = block.querySelector(".block-ref-target");
            if (!blockId || !label || !referenceTarget) {
                continue;
            }
            const blockRect = block.getBoundingClientRect();
            const labelRect = label.getBoundingClientRect();
            let referenceRight = null;
            let referenceBottom = null;
            for (const element of Array.from(referenceTarget.querySelectorAll(".ref-var, .ref-var *"))) {
                const rect = element.getBoundingClientRect();
                referenceRight = referenceRight === null ? rect.right : Math.max(referenceRight, rect.right);
                referenceBottom = referenceBottom === null ? rect.bottom : Math.max(referenceBottom, rect.bottom);
            }
            parseMetrics.push({
                block_id: blockId,
                current_width: blockRect.width,
                block_left: blockRect.left,
                label_left: labelRect.left,
                label_top: labelRect.top,
                label_width: labelRect.width,
                reference_right: referenceRight,
                reference_bottom: referenceBottom,
            });
        }
        return parseMetrics;
        "#,
    )
    .join()
    .await
    .ok()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn apply_block_reference_fit_updates_desktop(updates: &[BlockReferenceFitUpdate]) {
    if updates.is_empty() {
        return;
    }
    let Ok(updates_json) = serde_json::to_string(updates) else {
        return;
    };
    let script = format!(
        r#"
        const updates = {updates_json};
        const blocks = new Map(Array.from(document.querySelectorAll(".parse-page .block")).map(
            (block) => [block.getAttribute("data-block-id") || "", block],
        ));
        for (const update of updates) {{
            const block = blocks.get(String(update.block_id));
            if (!block) {{
                continue;
            }}
            block.style.setProperty("--block-reference-fit-width", `${{Number(update.fit_width).toFixed(2)}}px`);
        }}
        return null;
        "#
    );
    let _ = document::eval(&script).await;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn measure_block_reference_height_metrics_desktop()
-> Option<BlockReferenceHeightLayoutMetrics> {
    document::eval(
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
        const parseRequiredInt = (value) => {
            const parsed = Number.parseInt(value || "", 10);
            return Number.isFinite(parsed) && parsed >= 0 ? parsed : null;
        };
        const rowHeights = [];
        for (const probe of Array.from(document.querySelectorAll(".parse-page .block-row-height-probe"))) {
            const row = parseRequiredInt(probe.getAttribute("data-block-row"));
            if (row === null) {
                continue;
            }
            while (rowHeights.length <= row) {
                rowHeights.push(0);
            }
            rowHeights[row] = probe.getBoundingClientRect().height;
        }
        const blocks = [];
        for (const block of Array.from(document.querySelectorAll(".parse-page .block"))) {
            const blockId = block.getAttribute("data-block-id") || "";
            const row = parseRequiredInt(block.getAttribute("data-row"));
            const rowSpanRaw = parseRequiredInt(block.getAttribute("data-rowspan"));
            const label = block.querySelector(".block-label-text");
            const referenceTarget = block.querySelector(".block-ref-target");
            if (!blockId || row === null || !label || !referenceTarget) {
                continue;
            }
            const rowSpan = Math.max(1, rowSpanRaw === null ? 1 : rowSpanRaw);
            const blockRect = block.getBoundingClientRect();
            const labelRect = label.getBoundingClientRect();
            blocks.push({
                block_id: blockId,
                row,
                row_span: rowSpan,
                block_top: blockRect.top,
                block_height: blockRect.height,
                label_top: labelRect.top,
                label_left: labelRect.left,
                label_right: labelRect.right,
                reference_target_rect: rectFor(referenceTarget),
                reference_line_rects: Array.from(referenceTarget.querySelectorAll(".ref-line")).map(rectFor),
            });
        }
        return {
            row_heights: rowHeights,
            blocks,
        };
        "#,
    )
    .join()
    .await
    .ok()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn apply_block_reference_height_updates_desktop(
    updates: BlockReferenceHeightUpdates,
) {
    let Ok(updates_json) = serde_json::to_string(&updates) else {
        return;
    };
    let script = format!(
        r#"
        const updates = {updates_json};
        for (const sizer of Array.from(document.querySelectorAll(".parse-page .block-row-height-sizer"))) {{
            const row = Number.parseInt(sizer.getAttribute("data-block-row") || "", 10);
            if (!Number.isFinite(row)) {{
                continue;
            }}
            const growth = Number(updates.row_growths[row] || 0);
            const baseHeight = Number(updates.row_heights[row] || 0);
            if (!(growth > 0) || !(baseHeight >= 0)) {{
                continue;
            }}
            const targetHeight = baseHeight + growth;
            const value = `${{targetHeight.toFixed(2)}}px`;
            sizer.style.setProperty("height", value);
            sizer.style.setProperty("min-height", value);
        }}
        return null;
        "#
    );
    let _ = document::eval(&script).await;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn reset_block_reference_fit_width(block: &web_sys::Element) {
    let Some(block) = block.dyn_ref::<web_sys::HtmlElement>() else {
        return;
    };
    let _ = block.style().remove_property("--block-reference-fit-width");
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn reset_block_reference_height_sizers(document: &web_sys::Document) {
    let Ok(nodes) = document.query_selector_all(".parse-page .block-row-height-sizer") else {
        return;
    };
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(html) = element.dyn_ref::<web_sys::HtmlElement>() else {
            continue;
        };
        let style = html.style();
        let _ = style.remove_property("height");
        let _ = style.remove_property("min-height");
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn measured_block_row_heights(document: &web_sys::Document) -> Vec<f64> {
    let Ok(nodes) = document.query_selector_all(".parse-page .block-row-height-probe") else {
        return Vec::new();
    };
    let mut row_heights = Vec::new();
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(row) = element_usize_attr(&element, "data-block-row") else {
            continue;
        };
        if row >= row_heights.len() {
            row_heights.resize(row + 1, 0.0);
        }
        row_heights[row] = element.get_bounding_client_rect().height();
    }
    row_heights
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn block_row_range_for_element(
    block: &web_sys::Element,
) -> Option<(usize, usize, usize)> {
    let row = element_usize_attr(block, "data-row")?;
    let row_span = element_usize_attr(block, "data-rowspan")?.max(1);
    Some((row, row_span, row + row_span.saturating_sub(1)))
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn element_usize_attr(element: &web_sys::Element, name: &str) -> Option<usize> {
    element.get_attribute(name)?.parse::<usize>().ok()
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn block_reference_height_growth(
    block: &web_sys::Element,
    row: usize,
    row_span: usize,
    row_growths: &[f64],
) -> Option<(usize, f64)> {
    let bottom_row = row + row_span.saturating_sub(1);
    if bottom_row >= row_growths.len() {
        return None;
    }
    let label_text = block_label_text_for_block(block)?;
    let block_rect = block.get_bounding_client_rect();
    let label_rect = label_text.get_bounding_client_rect();
    let reference_bottoms = reference_bottoms_for_block(block, &label_rect, block_rect.top())?;
    let existing_growth = row_growths[row..=bottom_row].iter().sum::<f64>();
    let containment_deficit = reference_containment_deficit(
        reference_bottoms.stack_bottom,
        block_rect.height(),
        existing_growth,
    );
    let label_deficit = reference_bottoms
        .overlapping_label_bottom
        .map(|reference_bottom| {
            reference_clearance_deficit(
                reference_bottom,
                label_rect.top() - block_rect.top(),
                existing_growth,
            )
        })
        .unwrap_or(0.0);
    let deficit = containment_deficit.max(label_deficit);
    if deficit > 0.0 {
        Some((bottom_row, deficit))
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[invariant(true)]
pub(super) struct ReferenceBottoms {
    pub(super) stack_bottom: f64,
    pub(super) overlapping_label_bottom: Option<f64>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
pub(super) struct BlockReferenceFitMetrics {
    pub(super) block_id: String,
    pub(super) current_width: f64,
    pub(super) block_left: f64,
    pub(super) label_left: f64,
    pub(super) label_top: f64,
    pub(super) label_width: f64,
    pub(super) reference_right: Option<f64>,
    pub(super) reference_bottom: Option<f64>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[invariant(true)]
pub(super) struct BlockReferenceFitUpdate {
    pub(super) block_id: String,
    pub(super) fit_width: f64,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
pub(super) struct BlockReferenceHeightLayoutMetrics {
    pub(super) row_heights: Vec<f64>,
    pub(super) blocks: Vec<BlockReferenceHeightMetrics>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
pub(super) struct BlockReferenceHeightMetrics {
    pub(super) block_id: String,
    pub(super) row: usize,
    pub(super) row_span: usize,
    pub(super) block_top: f64,
    pub(super) block_height: f64,
    pub(super) label_top: f64,
    pub(super) label_left: f64,
    pub(super) label_right: f64,
    pub(super) reference_target_rect: Option<ReferenceRect>,
    pub(super) reference_line_rects: Vec<ReferenceRect>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[invariant(true)]
pub(super) struct BlockReferenceHeightUpdates {
    pub(super) row_heights: Vec<f64>,
    pub(super) row_growths: Vec<f64>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn block_reference_fit_updates(
    metrics: Vec<BlockReferenceFitMetrics>,
) -> Vec<BlockReferenceFitUpdate> {
    metrics
        .into_iter()
        .filter_map(|metric| {
            block_reference_fit_width_from_metrics(&metric).map(|fit_width| {
                BlockReferenceFitUpdate {
                    block_id: metric.block_id,
                    fit_width,
                }
            })
        })
        .collect()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.is_none_or(|width| width.is_finite() && width > metric.current_width))]
pub(super) fn block_reference_fit_width_from_metrics(
    metric: &BlockReferenceFitMetrics,
) -> Option<f64> {
    let reference_right = metric.reference_right?;
    let reference_bottom = metric.reference_bottom?;
    let reference_right_in_block = reference_right - metric.block_left;
    if reference_right_in_block <= 0.0 {
        return None;
    }
    let reference_fit_width = reference_right_in_block + BLOCK_REFERENCE_LABEL_GAP_PX;
    let overlap_fit_width = if reference_bottom > metric.label_top {
        let desired_text_left = reference_right + BLOCK_REFERENCE_LABEL_GAP_PX;
        if desired_text_left > metric.label_left {
            (reference_right_in_block + BLOCK_REFERENCE_LABEL_GAP_PX) * 2.0 + metric.label_width
        } else {
            0.0
        }
    } else {
        0.0
    };
    let fit_width = metric
        .current_width
        .max(reference_fit_width)
        .max(overlap_fit_width);
    (fit_width.is_finite() && fit_width > metric.current_width).then_some(fit_width)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.len() == metrics.row_heights.len())]
#[ensures(ret.iter().all(|growth| growth.is_finite() && *growth >= 0.0))]
pub(super) fn block_reference_row_growths(metrics: &BlockReferenceHeightLayoutMetrics) -> Vec<f64> {
    let mut row_growths = vec![0.0; metrics.row_heights.len()];
    let mut indexed_blocks = metrics
        .blocks
        .iter()
        .filter_map(|block| {
            let bottom_row = block.row + block.row_span.saturating_sub(1);
            Some((bottom_row, block.row, block.row_span, block))
        })
        .collect::<Vec<_>>();
    indexed_blocks.sort_by_key(|(bottom_row, row, _, _)| (*bottom_row, *row));
    for (_, _, _, block) in indexed_blocks {
        if let Some((bottom_row, deficit)) =
            block_reference_height_growth_from_metrics(block, &row_growths)
            && bottom_row < row_growths.len()
        {
            row_growths[bottom_row] += deficit;
        }
    }
    row_growths
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn block_reference_height_growth_from_metrics(
    block: &BlockReferenceHeightMetrics,
    row_growths: &[f64],
) -> Option<(usize, f64)> {
    let bottom_row = block.row + block.row_span.saturating_sub(1);
    if bottom_row >= row_growths.len() {
        return None;
    }
    let reference_bottoms = reference_bottoms_for_block_metrics(block)?;
    let existing_growth = row_growths[block.row..=bottom_row].iter().sum::<f64>();
    let containment_deficit = reference_containment_deficit(
        reference_bottoms.stack_bottom,
        block.block_height,
        existing_growth,
    );
    let label_deficit = reference_bottoms
        .overlapping_label_bottom
        .map(|reference_bottom| {
            reference_clearance_deficit(
                reference_bottom,
                block.label_top - block.block_top,
                existing_growth,
            )
        })
        .unwrap_or(0.0);
    let deficit = containment_deficit.max(label_deficit);
    (deficit > 0.0).then_some((bottom_row, deficit))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn reference_bottoms_for_block_metrics(
    block: &BlockReferenceHeightMetrics,
) -> Option<ReferenceBottoms> {
    if block.reference_line_rects.is_empty() {
        return block
            .reference_target_rect
            .map(|rect| reference_bottoms_for_rect(rect, block));
    }
    let mut stack_bottom = None;
    let mut overlapping_label_bottom = None;
    for rect in &block.reference_line_rects {
        let line_bottom = rect.bottom - block.block_top;
        stack_bottom = Some(stack_bottom.unwrap_or(f64::NEG_INFINITY).max(line_bottom));
        if horizontal_ranges_overlap(rect.left, rect.right, block.label_left, block.label_right) {
            overlapping_label_bottom = Some(
                overlapping_label_bottom
                    .unwrap_or(f64::NEG_INFINITY)
                    .max(line_bottom),
            );
        }
    }
    stack_bottom.map(|stack_bottom| ReferenceBottoms {
        stack_bottom,
        overlapping_label_bottom,
    })
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn reference_bottoms_for_rect(
    rect: ReferenceRect,
    block: &BlockReferenceHeightMetrics,
) -> ReferenceBottoms {
    let stack_bottom = rect.bottom - block.block_top;
    let overlapping_label_bottom =
        horizontal_ranges_overlap(rect.left, rect.right, block.label_left, block.label_right)
            .then_some(stack_bottom);
    ReferenceBottoms {
        stack_bottom,
        overlapping_label_bottom,
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn reference_bottoms_for_block(
    block: &web_sys::Element,
    label_rect: &web_sys::DomRect,
    block_top: f64,
) -> Option<ReferenceBottoms> {
    let reference_target = block_reference_target_for_block(block)?;
    let Ok(line_nodes) = reference_target.query_selector_all(".ref-line") else {
        return reference_bottoms_for_element(&reference_target, label_rect, block_top);
    };
    if line_nodes.length() == 0 {
        return reference_bottoms_for_element(&reference_target, label_rect, block_top);
    }
    let mut stack_bottom = None;
    let mut overlapping_label_bottom = None;
    for index in 0..line_nodes.length() {
        let Some(node) = line_nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let rect = element.get_bounding_client_rect();
        let line_bottom = rect.bottom() - block_top;
        stack_bottom = Some(stack_bottom.unwrap_or(f64::NEG_INFINITY).max(line_bottom));
        if horizontal_ranges_overlap(
            rect.left(),
            rect.right(),
            label_rect.left(),
            label_rect.right(),
        ) {
            overlapping_label_bottom = Some(
                overlapping_label_bottom
                    .unwrap_or(f64::NEG_INFINITY)
                    .max(line_bottom),
            );
        }
    }
    stack_bottom.map(|stack_bottom| ReferenceBottoms {
        stack_bottom,
        overlapping_label_bottom,
    })
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn reference_bottoms_for_element(
    element: &web_sys::Element,
    label_rect: &web_sys::DomRect,
    block_top: f64,
) -> Option<ReferenceBottoms> {
    let rect = element.get_bounding_client_rect();
    let stack_bottom = rect.bottom() - block_top;
    let overlapping_label_bottom = horizontal_ranges_overlap(
        rect.left(),
        rect.right(),
        label_rect.left(),
        label_rect.right(),
    )
    .then_some(stack_bottom);
    Some(ReferenceBottoms {
        stack_bottom,
        overlapping_label_bottom,
    })
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn apply_block_reference_height_sizers(
    document: &web_sys::Document,
    row_heights: &[f64],
    row_growths: &[f64],
) {
    let Ok(nodes) = document.query_selector_all(".parse-page .block-row-height-sizer") else {
        return;
    };
    for index in 0..nodes.length() {
        let Some(node) = nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let Some(row) = element_usize_attr(&element, "data-block-row") else {
            continue;
        };
        let Some(growth) = row_growths.get(row).copied() else {
            continue;
        };
        if growth <= 0.0 {
            continue;
        }
        let Some(base_height) = row_heights.get(row).copied() else {
            continue;
        };
        let Some(html) = element.dyn_ref::<web_sys::HtmlElement>() else {
            continue;
        };
        let target_height = base_height + growth;
        let value = format!("{target_height:.2}px");
        let style = html.style();
        let _ = style.set_property("height", &value);
        let _ = style.set_property("min-height", &value);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn adjust_block_reference_fit_width(block: &web_sys::Element) {
    let Some(block_html) = block.dyn_ref::<web_sys::HtmlElement>() else {
        return;
    };
    let Some(label_text) = block_label_text_for_block(block) else {
        return;
    };
    let Some(reference_target) = block_reference_target_for_block(block) else {
        return;
    };
    let Ok(reference_nodes) = reference_target.query_selector_all(".ref-var, .ref-var *") else {
        return;
    };
    let text_rect = label_text.get_bounding_client_rect();
    let block_rect = block.get_bounding_client_rect();
    let mut reference_right = f64::NEG_INFINITY;
    let mut reference_bottom = f64::NEG_INFINITY;
    for index in 0..reference_nodes.length() {
        let Some(node) = reference_nodes.item(index) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        let rect = element.get_bounding_client_rect();
        reference_right = reference_right.max(rect.right());
        reference_bottom = reference_bottom.max(rect.bottom());
    }
    if !reference_right.is_finite() || !reference_bottom.is_finite() {
        return;
    }
    let reference_right_in_block = reference_right - block_rect.left();
    if reference_right_in_block <= 0.0 {
        return;
    }
    let reference_fit_width = reference_right_in_block + BLOCK_REFERENCE_LABEL_GAP_PX;
    let overlap_fit_width = if reference_bottom > text_rect.top() {
        let desired_text_left = reference_right + BLOCK_REFERENCE_LABEL_GAP_PX;
        if desired_text_left > text_rect.left() {
            (reference_right_in_block + BLOCK_REFERENCE_LABEL_GAP_PX) * 2.0 + text_rect.width()
        } else {
            0.0
        }
    } else {
        0.0
    };
    let current_width = block_rect.width();
    let fit_width = current_width
        .max(reference_fit_width)
        .max(overlap_fit_width);
    if !fit_width.is_finite() || fit_width <= current_width {
        return;
    }
    let _ = block_html
        .style()
        .set_property("--block-reference-fit-width", &format!("{fit_width:.2}px"));
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn block_label_text_for_block(block: &web_sys::Element) -> Option<web_sys::Element> {
    block.query_selector(".block-label-text").ok().flatten()
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn block_reference_target_for_block(
    block: &web_sys::Element,
) -> Option<web_sys::Element> {
    block.query_selector(".block-ref-target").ok().flatten()
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn position_dictionary_tooltip_from_event(event: &web_sys::Event) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    if !topbar_styles_ready(&document) {
        return;
    }
    let Some(target) = event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
    else {
        return;
    };
    let Ok(Some(host)) = target.closest(".dictionary-tooltip-host, .reference-tooltip-host") else {
        return;
    };
    activate_dictionary_tooltip_host(&host);
    position_dictionary_tooltip(&host);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn activate_dictionary_tooltip_host(active_host: &web_sys::Element) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(hosts) =
        document.query_selector_all(".dictionary-tooltip-host, .reference-tooltip-host")
    else {
        return;
    };
    for index in 0..hosts.length() {
        let Some(node) = hosts.item(index) else {
            continue;
        };
        let Ok(host) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if js_sys::Object::is(host.as_ref(), active_host.as_ref()) {
            clear_dictionary_tooltip_immediate_hide(&host);
        } else {
            hide_dictionary_tooltip_immediately(&host);
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn hide_dictionary_tooltip_immediately(host: &web_sys::Element) {
    let Some(tooltip) = dictionary_tooltip_element_for_host(host) else {
        return;
    };
    let style = tooltip.style();
    let _ = tooltip.remove_attribute("data-jbotci-position-ready");
    let _ = style.set_property("visibility", "hidden");
    let _ = style.set_property("pointer-events", "none");
    let _ = style.set_property("transition", "none");
    let _ = style.remove_property("transform");
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn clear_dictionary_tooltip_immediate_hide(host: &web_sys::Element) {
    let Some(tooltip) = dictionary_tooltip_element_for_host(host) else {
        return;
    };
    let style = tooltip.style();
    let _ = tooltip.remove_attribute("data-jbotci-position-ready");
    let _ = style.remove_property("visibility");
    let _ = style.remove_property("pointer-events");
    let _ = style.remove_property("transform");
    let _ = style.remove_property("transition");
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn dictionary_tooltip_element_for_host(
    host: &web_sys::Element,
) -> Option<web_sys::HtmlElement> {
    host.query_selector(".rich-reference-tooltip-stack")
        .ok()
        .flatten()
        .or_else(|| {
            host.query_selector(".rich-dictionary-tooltip")
                .ok()
                .flatten()
        })
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn position_dictionary_tooltip(host: &web_sys::Element) {
    let Some(tooltip_html) = dictionary_tooltip_element_for_host(host) else {
        return;
    };
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let _ = tooltip_html.remove_attribute("data-jbotci-position-ready");
    let host_rect = host.get_bounding_client_rect();
    let tooltip_rect = tooltip_html.get_bounding_client_rect();
    let viewport_width = window
        .inner_width()
        .ok()
        .and_then(|width| width.as_f64())
        .unwrap_or(1.0);
    let viewport_height = window
        .inner_height()
        .ok()
        .and_then(|height| height.as_f64())
        .unwrap_or(1.0);
    let viewport_top = dictionary_tooltip_visible_top(&document);
    let position = platform::place_tooltip(
        platform::Rect {
            left: host_rect.left(),
            top: host_rect.top(),
            width: host_rect.width().max(0.0),
            height: host_rect.height().max(0.0),
        },
        platform::Size {
            width: tooltip_rect.width(),
            height: tooltip_rect.height(),
        },
        platform::Viewport {
            top: viewport_top,
            width: viewport_width,
            height: viewport_height,
        },
        DICTIONARY_TOOLTIP_VIEWPORT_MARGIN_PX,
        DICTIONARY_TOOLTIP_HOST_GAP_PX,
    );
    let style = tooltip_html.style();
    let _ = style.set_property(
        "--dictionary-tooltip-left",
        &format!("{:.2}px", position.left),
    );
    let _ = style.set_property(
        "--dictionary-tooltip-top",
        &format!("{:.2}px", position.top),
    );
    let _ = style.set_property("left", &format!("{:.2}px", position.left));
    let _ = style.set_property("top", &format!("{:.2}px", position.top));
    let _ = tooltip_html.set_attribute("data-jbotci-position-ready", "true");
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret >= 0.0)]
pub(super) fn dictionary_tooltip_visible_top(document: &web_sys::Document) -> f64 {
    let topbar_bottom = document
        .query_selector(".app-topbar")
        .ok()
        .flatten()
        .map(|element| element.get_bounding_client_rect().bottom())
        .unwrap_or(0.0);
    let app_scroll_top = document
        .query_selector("[data-app-scroll='main']")
        .ok()
        .flatten()
        .map(|element| element.get_bounding_client_rect().top())
        .unwrap_or(0.0);
    topbar_bottom.max(app_scroll_top).max(0.0)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[invariant(true)]
pub(super) struct DesktopTooltipMeasure {
    pub(super) id: String,
    pub(super) host_rect: ReferenceRect,
    pub(super) tooltip_size: platform::Size,
    pub(super) viewport: platform::Viewport,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[invariant(true)]
pub(super) struct DesktopTooltipPlacement {
    pub(super) id: String,
    pub(super) left: f64,
    pub(super) top: f64,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn install_desktop_tooltip_bridge() {
    spawn(async move {
        let mut eval = document::eval(
            r#"
            let nextTooltipId = 1;
            const hostSelector = ".dictionary-tooltip-host, .reference-tooltip-host";
            const stylesReady = () => {
                const shell = document.querySelector(".spa-shell.app-page");
                if (!shell) {
                    return false;
                }
                const shellStyle = window.getComputedStyle(shell);
                return String(shellStyle.getPropertyValue("--topbar-bg") || "").trim().length > 0;
            };
            const tooltipForHost = (host) => {
                for (const child of Array.from(host.children)) {
                    if (
                        child.classList &&
                        (child.classList.contains("rich-reference-tooltip-stack") ||
                            child.classList.contains("rich-dictionary-tooltip"))
                    ) {
                        return child;
                    }
                }
                return host.querySelector(".rich-reference-tooltip-stack, .rich-dictionary-tooltip");
            };
            const rectFor = (element) => {
                const rect = element.getBoundingClientRect();
                return {
                    left: rect.left,
                    top: rect.top,
                    right: rect.right,
                    bottom: rect.bottom,
                };
            };
            const rectTop = (selector) => {
                const element = document.querySelector(selector);
                return element ? element.getBoundingClientRect().top : 0;
            };
            const rectBottom = (selector) => {
                const element = document.querySelector(selector);
                return element ? element.getBoundingClientRect().bottom : 0;
            };
            const visibleViewportTop = () => Math.max(
                0,
                rectBottom(".app-topbar"),
                rectTop("[data-app-scroll='main']"),
            );
            const hideInactiveTooltip = (host) => {
                const tooltip = tooltipForHost(host);
                if (!tooltip) {
                    return;
                }
                tooltip.removeAttribute("data-jbotci-position-ready");
                tooltip.style.setProperty("visibility", "hidden");
                tooltip.style.setProperty("pointer-events", "none");
                tooltip.style.setProperty("transition", "none");
                tooltip.style.removeProperty("transform");
            };
            const activateHost = (activeHost) => {
                for (const host of Array.from(document.querySelectorAll(hostSelector))) {
                    const tooltip = tooltipForHost(host);
                    if (!tooltip) {
                        continue;
                    }
                    if (host === activeHost) {
                        tooltip.removeAttribute("data-jbotci-position-ready");
                        tooltip.style.removeProperty("visibility");
                        tooltip.style.removeProperty("pointer-events");
                        tooltip.style.removeProperty("transform");
                        tooltip.style.removeProperty("transition");
                    } else {
                        hideInactiveTooltip(host);
                    }
                }
            };
            const hostForId = (id) => Array.from(document.querySelectorAll(hostSelector)).find(
                (host) => host.dataset.jbotciTooltipId === String(id),
            );
            const measureHost = (target) => {
                if (!stylesReady()) {
                    return;
                }
                const element = target instanceof Element ? target : target && target.parentElement;
                const host = element && element.closest ? element.closest(hostSelector) : null;
                if (!host) {
                    return;
                }
                if (!host.dataset.jbotciTooltipId) {
                    host.dataset.jbotciTooltipId = String(nextTooltipId++);
                }
                const tooltip = tooltipForHost(host);
                if (!tooltip) {
                    return;
                }
                activateHost(host);
                const tooltipRect = tooltip.getBoundingClientRect();
                dioxus.send({
                    id: host.dataset.jbotciTooltipId,
                    host_rect: rectFor(host),
                    tooltip_size: {
                        width: tooltipRect.width,
                        height: tooltipRect.height,
                    },
                    viewport: {
                        top: visibleViewportTop(),
                        width: Number(window.innerWidth || 1),
                        height: Number(window.innerHeight || 1),
                    },
                });
            };
            const scheduleMeasure = (event) => {
                const target = event.target;
                requestAnimationFrame(() => requestAnimationFrame(() => measureHost(target)));
            };
            document.addEventListener("mouseover", scheduleMeasure, true);
            document.addEventListener("focusin", scheduleMeasure, true);
            document.addEventListener("click", scheduleMeasure, true);
            (async () => {
                while (true) {
                    const placement = await dioxus.recv();
                    const host = hostForId(placement.id);
                    if (!host) {
                        continue;
                    }
                    const tooltip = tooltipForHost(host);
                    if (!tooltip) {
                        continue;
                    }
                    const left = `${Number(placement.left).toFixed(2)}px`;
                    const top = `${Number(placement.top).toFixed(2)}px`;
                    tooltip.style.setProperty("--dictionary-tooltip-left", left);
                    tooltip.style.setProperty("--dictionary-tooltip-top", top);
                    tooltip.style.setProperty("left", left);
                    tooltip.style.setProperty("top", top);
                    tooltip.setAttribute("data-jbotci-position-ready", "true");
                }
            })();
            await new Promise(() => {});
            "#,
        );
        while let Ok(measure) = eval.recv::<DesktopTooltipMeasure>().await {
            let position = platform::place_tooltip(
                platform_rect_from_reference_rect(measure.host_rect),
                measure.tooltip_size,
                measure.viewport,
                DICTIONARY_TOOLTIP_VIEWPORT_MARGIN_PX,
                DICTIONARY_TOOLTIP_HOST_GAP_PX,
            );
            let _ = eval.send(DesktopTooltipPlacement {
                id: measure.id,
                left: position.left,
                top: position.top,
            });
        }
    });
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn sync_document_head(meta: &PageMeta) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let head_model = build_page_head(meta);
    if let Ok(nodes) = document.query_selector_all("[data-jbotci-meta='1']") {
        for index in 0..nodes.length() {
            if let Some(node) = nodes.item(index)
                && let Some(parent) = node.parent_node()
            {
                let _ = parent.remove_child(&node);
            }
        }
    }
    let Ok(Some(head)) = document.query_selector("head") else {
        return;
    };
    let canonical_url = absolute_href_for_client(&head_model.canonical_url);
    let manifest_href = absolute_href_for_client(&head_model.manifest_href);
    let icon_href = absolute_href_for_client(&head_model.icon_href);
    let apple_touch_icon_href = absolute_href_for_client(&head_model.apple_touch_icon_href);
    append_meta_name(&document, &head, "application-name", "jbotci");
    append_meta_name(&document, &head, "apple-mobile-web-app-capable", "yes");
    append_meta_name(&document, &head, "apple-mobile-web-app-title", "jbotci");
    append_meta_name(&document, &head, "mobile-web-app-capable", "yes");
    append_meta_name_with_extra(
        &document,
        &head,
        "theme-color",
        &head_model.light_theme_color,
        &[("media", "(prefers-color-scheme: light)")],
    );
    append_meta_name_with_extra(
        &document,
        &head,
        "theme-color",
        &head_model.dark_theme_color,
        &[("media", "(prefers-color-scheme: dark)")],
    );
    append_link(&document, &head, "manifest", &manifest_href);
    append_link(&document, &head, "icon", &icon_href);
    append_link(&document, &head, "shortcut icon", &icon_href);
    append_link(&document, &head, "apple-touch-icon", &apple_touch_icon_href);
    append_meta_name(&document, &head, "description", &head_model.description);
    append_link(&document, &head, "canonical", &canonical_url);
    append_meta_property(&document, &head, "og:title", &head_model.title);
    append_meta_property(&document, &head, "og:description", &head_model.description);
    append_meta_property(&document, &head, "og:type", "website");
    append_meta_property(&document, &head, "og:url", &canonical_url);
    append_meta_name(&document, &head, "twitter:title", &head_model.title);
    append_meta_name(
        &document,
        &head,
        "twitter:description",
        &head_model.description,
    );
    append_meta_name(&document, &head, "twitter:card", &head_model.twitter_card);
    if let Some(image) = &head_model.image {
        let image_url = absolute_href_for_client(&image.href);
        append_meta_property(&document, &head, "og:image", &image_url);
        append_meta_name(&document, &head, "twitter:image", &image_url);
        append_meta_property(&document, &head, "og:image:width", &image.width.to_string());
        append_meta_property(
            &document,
            &head,
            "og:image:height",
            &image.height.to_string(),
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn sync_document_head(meta: &PageMeta) {
    let _ = meta;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn append_meta_name(
    document: &web_sys::Document,
    head: &web_sys::Element,
    name: &str,
    content: &str,
) {
    append_meta_name_with_extra(document, head, name, content, &[]);
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn append_meta_name_with_extra(
    document: &web_sys::Document,
    head: &web_sys::Element,
    name: &str,
    content: &str,
    extra: &[(&str, &str)],
) {
    if let Ok(element) = document.create_element("meta") {
        let _ = element.set_attribute("data-jbotci-meta", "1");
        let _ = element.set_attribute("name", name);
        let _ = element.set_attribute("content", content);
        for (key, value) in extra {
            let _ = element.set_attribute(key, value);
        }
        let _ = head.append_child(&element);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn append_meta_property(
    document: &web_sys::Document,
    head: &web_sys::Element,
    property: &str,
    content: &str,
) {
    if let Ok(element) = document.create_element("meta") {
        let _ = element.set_attribute("data-jbotci-meta", "1");
        let _ = element.set_attribute("property", property);
        let _ = element.set_attribute("content", content);
        let _ = head.append_child(&element);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn append_link(
    document: &web_sys::Document,
    head: &web_sys::Element,
    rel: &str,
    href: &str,
) {
    if let Ok(element) = document.create_element("link") {
        let _ = element.set_attribute("data-jbotci-meta", "1");
        let _ = element.set_attribute("rel", rel);
        let _ = element.set_attribute("href", href);
        let _ = head.append_child(&element);
    }
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn absolute_href_for_client(href: &str) -> String {
    if href.starts_with('/') {
        if let Some(window) = web_sys::window()
            && let Ok(origin) = window.location().origin()
        {
            return format!("{}{}", origin.trim_end_matches('/'), href);
        }
    }
    href.to_owned()
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(ret.starts_with('/'))]
pub(super) fn current_path() -> String {
    web_sys::window()
        .and_then(|window| window.location().pathname().ok())
        .filter(|path| path.starts_with('/'))
        .unwrap_or_else(|| "/vlacku".to_owned())
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(ret.starts_with('/'))]
pub(super) fn current_path() -> String {
    "/vlacku".to_owned()
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn set_brivla_toggle_indeterminate(indeterminate: bool) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let Ok(Some(element)) = document.query_selector("input[data-brivla-toggle='1']") else {
        return;
    };
    if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
        input.set_indeterminate(indeterminate);
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn set_brivla_toggle_indeterminate(_indeterminate: bool) {}

#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_vlacku_jvozba_pane_metrics_sync() {
    platform::schedule_layout_passes(0, VLACKU_JVOZBA_LAYOUT_FRAME_PASSES, || async {
        sync_vlacku_jvozba_pane_metrics_scheduled().await;
    });
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) async fn sync_vlacku_jvozba_pane_metrics_scheduled() {
    sync_vlacku_jvozba_pane_metrics();
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn sync_vlacku_jvozba_pane_metrics_scheduled() {
    sync_vlacku_jvozba_pane_metrics_desktop().await;
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn sync_vlacku_jvozba_pane_metrics_scheduled() {}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn schedule_vlacku_jvozba_pane_metrics_after_fonts_ready(document: &web_sys::Document) {
    platform::schedule_after_fonts_ready(document, || async {
        schedule_vlacku_jvozba_pane_metrics_sync();
    });
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn sync_vlacku_jvozba_pane_metrics() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Ok(Some(pane)) = document.query_selector("[data-jvozba-pane='1']") else {
        return;
    };
    let Some(pane) = pane.dyn_ref::<web_sys::HtmlElement>() else {
        return;
    };
    let topbar_bottom = document
        .query_selector(".app-topbar")
        .ok()
        .flatten()
        .map(|element| element.get_bounding_client_rect().bottom())
        .unwrap_or(0.0);
    let form_bottom = document
        .query_selector(".vlacku-page .dictionary-form .dictionary-query-row")
        .ok()
        .flatten()
        .map(|element| element.get_bounding_client_rect().bottom());
    let anchor_top = document
        .query_selector("[data-jvozba-pane-anchor='1']")
        .ok()
        .flatten()
        .map(|element| element.get_bounding_client_rect().top());
    let viewport_height = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(720.0);
    let app_scroll_container = document
        .query_selector("[data-app-scroll='main']")
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok());
    let app_scroll_top = app_scroll_container
        .as_ref()
        .map(|main| main.scroll_top().max(0))
        .unwrap_or(0);
    let app_scrollbar_gutter_width = app_scroll_container
        .as_ref()
        .map(|main| (main.offset_width() - main.client_width()).max(0))
        .unwrap_or(0);
    let fallback_top = form_bottom.unwrap_or(topbar_bottom).max(topbar_bottom) + 12.0;
    let layout = platform::compute_jvozba_pane_layout(
        anchor_top,
        app_scroll_top,
        fallback_top,
        topbar_bottom,
        viewport_height,
        app_scrollbar_gutter_width,
        VLACKU_JVOZBA_HEIGHT_SCALE,
    );
    let style = pane.style();
    let _ = style.set_property("--jvozba-pane-top", &format!("{}px", layout.top));
    let _ = style.set_property("--jvozba-pane-bottom", &format!("{}px", layout.bottom));
    let _ = style.set_property("--jvozba-pane-height", &format!("{}px", layout.height));
    let _ = style.set_property(
        "--app-scrollbar-gutter-width",
        &format!("{}px", layout.scrollbar_gutter_width),
    );
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn sync_vlacku_jvozba_pane_metrics() {
    spawn(async move {
        sync_vlacku_jvozba_pane_metrics_desktop().await;
    });
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "desktop")))]
#[requires(true)]
#[ensures(true)]
pub(super) fn sync_vlacku_jvozba_pane_metrics() {}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[invariant(true)]
pub(super) struct JvozbaPaneMetrics {
    pub(super) topbar_bottom: f64,
    pub(super) form_bottom: Option<f64>,
    pub(super) anchor_top: Option<f64>,
    pub(super) viewport_height: f64,
    pub(super) app_scroll_top: i32,
    pub(super) app_scrollbar_gutter_width: i32,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(ret.top >= metrics.topbar_bottom)]
pub(super) fn jvozba_pane_layout_from_metrics(
    metrics: JvozbaPaneMetrics,
) -> platform::JvozbaPaneLayout {
    let fallback_top = metrics
        .form_bottom
        .unwrap_or(metrics.topbar_bottom)
        .max(metrics.topbar_bottom)
        + 12.0;
    platform::compute_jvozba_pane_layout(
        metrics.anchor_top,
        metrics.app_scroll_top,
        fallback_top,
        metrics.topbar_bottom,
        metrics.viewport_height,
        metrics.app_scrollbar_gutter_width,
        VLACKU_JVOZBA_HEIGHT_SCALE,
    )
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn sync_vlacku_jvozba_pane_metrics_desktop() {
    let Some(metrics) = measure_vlacku_jvozba_pane_metrics_desktop().await else {
        return;
    };
    let layout = jvozba_pane_layout_from_metrics(metrics);
    apply_vlacku_jvozba_pane_layout_desktop(layout).await;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn measure_vlacku_jvozba_pane_metrics_desktop() -> Option<JvozbaPaneMetrics> {
    document::eval(
        r#"
        if (!document.querySelector("[data-jvozba-pane='1']")) {
            return null;
        }
        const rectBottom = (selector) => {
            const element = document.querySelector(selector);
            return element ? element.getBoundingClientRect().bottom : null;
        };
        const rectTop = (selector) => {
            const element = document.querySelector(selector);
            return element ? element.getBoundingClientRect().top : null;
        };
        const appScroll = document.querySelector("[data-app-scroll='main']");
        return {
            topbar_bottom: rectBottom(".app-topbar") ?? 0,
            form_bottom: rectBottom(".vlacku-page .dictionary-form .dictionary-query-row"),
            anchor_top: rectTop("[data-jvozba-pane-anchor='1']"),
            viewport_height: Number(window.innerHeight || 720),
            app_scroll_top: appScroll ? Math.max(0, Number(appScroll.scrollTop || 0)) : 0,
            app_scrollbar_gutter_width: appScroll ? Math.max(0, Number(appScroll.offsetWidth || 0) - Number(appScroll.clientWidth || 0)) : 0,
        };
        "#,
    )
    .join()
    .await
    .ok()
    .flatten()
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[requires(true)]
#[ensures(true)]
pub(super) async fn apply_vlacku_jvozba_pane_layout_desktop(layout: platform::JvozbaPaneLayout) {
    let Ok(layout_json) = serde_json::to_string(&layout) else {
        return;
    };
    let script = format!(
        r#"
        const layout = {layout_json};
        const pane = document.querySelector("[data-jvozba-pane='1']");
        if (pane) {{
            pane.style.setProperty("--jvozba-pane-top", `${{Number(layout.top).toFixed(2)}}px`);
            pane.style.setProperty("--jvozba-pane-bottom", `${{Number(layout.bottom).toFixed(2)}}px`);
            pane.style.setProperty("--jvozba-pane-height", `${{Number(layout.height).toFixed(2)}}px`);
            pane.style.setProperty("--app-scrollbar-gutter-width", `${{Number(layout.scrollbar_gutter_width)}}px`);
        }}
        return null;
        "#
    );
    let _ = document::eval(&script).await;
}

#[cfg(target_arch = "wasm32")]
#[requires(true)]
#[ensures(true)]
pub(super) fn measure_vlacku_jvozba_item_height(index: usize) -> Option<usize> {
    let document = web_sys::window()?.document()?;
    let selector = format!("[data-jvozba-item-index='{index}']");
    let element = document.query_selector(&selector).ok().flatten()?;
    Some(element.get_bounding_client_rect().height().round().max(1.0) as usize)
}

#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
pub(super) fn measure_vlacku_jvozba_item_height(_index: usize) -> Option<usize> {
    None
}
