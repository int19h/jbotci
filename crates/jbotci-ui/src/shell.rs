use super::*;

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
pub(super) fn App() -> Element {
    rsx! {
        Router::<JbotciRoute> {}
    }
}

#[requires(true)]
#[ensures(!ret.title.is_empty())]
pub(super) fn route_document_meta(base_path: &str, route: &JbotciRoute) -> PageMeta {
    build_page_meta(base_path, &route.web_route)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn document_title_from_meta(meta: &PageMeta) -> String {
    meta.title.clone()
}

#[requires(true)]
#[ensures(true)]
pub(super) fn apply_document_meta(mut document_meta: Signal<PageMeta>, meta: PageMeta) {
    sync_document_head(&meta);
    document_meta.set(meta);
}

#[requires(true)]
#[ensures(ret.contains("STIX Two Math"))]
#[ensures(ret.contains("STIX Two Text"))]
pub(super) fn font_face_css() -> String {
    format!(
        r#"
@font-face {{
  font-family: "Noto Sans";
  src: url("{noto_sans}") format("truetype");
  font-weight: 100 900;
  font-stretch: 62.5% 100%;
  font-style: normal;
  font-display: swap;
}}

@font-face {{
  font-family: "Noto Sans";
  src: url("{noto_sans_italic}") format("truetype");
  font-weight: 100 900;
  font-stretch: 62.5% 100%;
  font-style: italic;
  font-display: swap;
}}

@font-face {{
  font-family: "STIX Two Math";
  src: url("{stix_two_math}") format("truetype");
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}}

@font-face {{
  font-family: "STIX Two Text";
  src: url("{stix_two_text}") format("truetype");
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}}

@font-face {{
  font-family: "STIX Two Text";
  src: url("{stix_two_text_bold}") format("truetype");
  font-weight: 700;
  font-style: normal;
  font-display: swap;
}}

@font-face {{
  font-family: "Crisa";
  src: url("{crisa}") format("opentype");
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}}
"#,
        noto_sans = NOTO_SANS,
        noto_sans_italic = NOTO_SANS_ITALIC,
        stix_two_math = STIX_TWO_MATH,
        stix_two_text = STIX_TWO_TEXT,
        stix_two_text_bold = STIX_TWO_TEXT_BOLD,
        crisa = CRISA,
    )
}

#[requires(true)]
#[ensures(ret.contains(".app-topbar-brand-logo"))]
#[ensures(ret.contains(".rich-dictionary-tooltip"))]
pub(super) fn critical_startup_css() -> &'static str {
    r#"
.app-topbar-brand-logo {
  display: block;
  height: 1.9rem;
  width: auto;
}

.rich-dictionary-tooltip,
.rich-reference-tooltip-stack {
  position: fixed;
  left: 0;
  top: 0;
  visibility: hidden;
  pointer-events: none;
}
"#
}

#[allow(non_snake_case)]
#[requires(true)]
#[ensures(true)]
pub(super) fn AppShell() -> Element {
    let current_route_location = use_route::<JbotciRoute>();
    let route = use_signal(|| current_route_location.app_route());
    let base_path = router_base_path();
    let initial_document_meta = route_document_meta(&base_path, &current_route_location);
    let document_meta = use_signal(move || initial_document_meta.clone());
    let app_history = history();
    let settings = use_signal(load_settings);
    let initial_dialect_settings = load_dialect_settings();
    let initial_settings_dialect_selection =
        initial_dialect_settings_selection(&initial_dialect_settings);
    let mut dialect_settings = use_signal(move || initial_dialect_settings.clone());
    let mut settings_dialect_selection =
        use_signal(move || initial_settings_dialect_selection.clone());
    let settings_dialect_qr_uri = use_signal(|| None::<String>);
    let gentufa_dialect_picker_open = use_signal(|| false);
    let mut settings_johau_import_seen = use_signal(|| None::<String>);
    let embedding_settings = use_signal(EmbeddingSettingsState::default);
    let activity = use_signal(AsyncActivityState::default);
    let activity_indicator_visible = use_signal(|| false);
    let activity_indicator_delay_task = use_signal(|| None::<Task>);
    let topbar_settings_layout = use_signal(|| TopbarSettingsLayout::BothInline);
    let topbar_settings_open = use_signal(|| false);
    let topbar_nav_layout = use_signal(|| TopbarNavLayout::Full);
    let mut page_find_state = use_signal(PageFindState::default);
    let initial_gentufa = initial_gentufa_state(&current_route_location);
    let initial_gentufa_has_text = initial_gentufa_text_explicit(&current_route_location);
    let initial_gentufa_input_text = if initial_gentufa_has_text {
        initial_gentufa.text.clone()
    } else {
        String::new()
    };
    let initial_gentufa_parsed_text =
        if initial_gentufa.text.is_empty() && !initial_gentufa_has_text {
            DEFAULT_GENTUFA_TEXT.to_owned()
        } else {
            initial_gentufa.text.clone()
        };
    let initial_gentufa_dialect = initial_gentufa.dialect.clone().unwrap_or_default();
    let initial_gentufa_view_mode = initial_gentufa.view_mode;
    let initial_gentufa_display = GentufaDisplayState {
        show_elided: initial_gentufa.show_elided,
        show_glosses: initial_gentufa.show_glosses,
        show_compounds: initial_gentufa.show_compounds,
    };
    let view_mode = use_signal(move || initial_gentufa_view_mode);
    let gentufa_display = use_signal(move || initial_gentufa_display);
    let parsed_text_explicit = use_signal(move || initial_gentufa_has_text);
    let gentufa_url_write_intent = use_signal(|| GentufaUrlWriteIntent::ReplaceCurrent);
    let initial_cukta = initial_cukta_state(&current_route_location);
    let cukta_draft_state = use_signal(|| initial_cukta.clone());
    let cukta_committed_state = use_signal(|| initial_cukta);
    let cukta_toc_filter = use_signal(String::new);
    let cukta_toc_pinned = use_signal(load_cukta_toc_pinned);
    let cukta_toc_expansion = use_signal(load_cukta_toc_expansion);
    let cukta_toc_width = use_signal(load_cukta_toc_width);
    let cukta_toc_resize = use_signal(|| None::<CuktaTocResizeState>);
    let cukta_toc_overlay_visible = use_signal(|| false);
    let cukta_toc_forced_autohide = use_signal(cukta_toc_forced_autohide_active);
    let initial_vlacku = initial_vlacku_state(&current_route_location);
    let vlacku_draft_state = use_signal(|| initial_vlacku.clone());
    let vlacku_committed_state = use_signal(|| initial_vlacku);
    let pending_vlacku_scroll_restore = use_signal(|| None::<i32>);
    let vlacku_semantic_result = use_signal(VlackuSemanticResultState::default);
    let vlacku_result = use_signal(VlackuAsyncResultState::default);
    let vlacku_result_task = use_signal(|| None::<LatestAsyncTask>);
    let vlacku_semantic_task = use_signal(|| None::<LatestAsyncTask>);
    let initial_gimfihi = initial_gimfihi_state(&current_route_location);
    let initial_gimfihi_source_words = initial_gimfihi.clone();
    let gimfihi_source_word_memory =
        use_signal(move || gimfihi_source_word_memory_from_state(&initial_gimfihi_source_words));
    let gimfihi_draft_state = use_signal(|| initial_gimfihi.clone());
    let gimfihi_committed_state = use_signal(|| initial_gimfihi);
    let gimfihi_result = use_signal(GimfihiAsyncResultState::default);
    let gimfihi_result_cache = use_signal(BTreeMap::<String, GimfihiAsyncResultState>::new);
    let gimfihi_result_task = use_signal(|| None::<LatestAsyncTask>);
    let cukta_semantic_result = use_signal(CuktaSemanticResultState::default);
    let cukta_page = use_signal(CuktaAsyncPageState::default);
    let cukta_page_task = use_signal(|| None::<LatestAsyncTask>);
    let cukta_semantic_task = use_signal(|| None::<LatestAsyncTask>);
    let initial_pending_cukta_scroll = current_cukta_pending_scroll(&current_route_location);
    let pending_cukta_scroll = use_signal(move || initial_pending_cukta_scroll.clone());
    let initial_last_route_for_scroll = current_route_location.clone();
    let mut last_route_for_scroll = use_signal(move || initial_last_route_for_scroll.clone());
    let initial_last_page_find_route = current_route_location.app_route();
    let mut last_page_find_route = use_signal(move || initial_last_page_find_route);
    let jvozba_pane = use_signal(load_vlacku_jvozba_pane_state);
    let jvozba_available = use_signal(vlacku_jvozba_available);
    let jvozba_drag = use_signal(|| None::<VlackuJvozbaDragState>);
    let initial_input_text = initial_gentufa_input_text;
    let initial_parsed_text = initial_gentufa_parsed_text;
    let initial_dialect = initial_gentufa_dialect.clone();
    let initial_parsed_dialect = initial_gentufa_dialect;
    let input_text = use_signal(move || initial_input_text.clone());
    let parsed_text = use_signal(move || initial_parsed_text.clone());
    let dialect = use_signal(move || initial_dialect.clone());
    let parsed_dialect = use_signal(move || initial_parsed_dialect.clone());
    let reference_hover = use_signal(ReferenceHoverState::default);
    let reference_tooltip_open = use_signal(|| None::<HoveredReference>);
    let gentufa_page = use_signal(GentufaAsyncPageState::default);
    let gentufa_page_task = use_signal(|| None::<LatestAsyncTask>);
    let gentufa_diagnostics_open = use_signal(|| true);
    let gentufa_active_diagnostic = use_signal(|| None::<ActiveDiagnosticTarget>);
    let gentufa_input_diagnostic_tooltip = use_signal(|| None::<DiagnosticInputTooltip>);
    let export_task = use_signal(|| None::<LatestAsyncTask>);
    let mut pending_local_route_writes = use_signal(PendingLocalRouteWrites::default);

    let settings_value = *settings.read();
    let dialect_settings_value = dialect_settings.read().clone();
    let settings_dialect_selection_value = settings_dialect_selection.read().clone();
    let embedding_settings_value = embedding_settings.read().clone();
    let activity_value = activity.read().clone();
    let activity_indicator_visible_value = *activity_indicator_visible.read();
    let route_value = *route.read();
    let view_mode_value = *view_mode.read();
    let gentufa_display_value = *gentufa_display.read();
    let parsed_text_value = parsed_text.read().clone();
    let parsed_dialect_value = parsed_dialect.read().clone();
    let parsed_text_explicit_value = *parsed_text_explicit.read();
    let gentufa_url_write_intent_value = *gentufa_url_write_intent.read();
    let gentufa_page_value = gentufa_page.read().clone();
    let document_meta_value = document_meta.read().clone();
    let document_title = document_title_from_meta(&document_meta_value);
    let result = gentufa_page_value.result.clone();
    let gentufa_request = gentufa_page_value.request.clone();
    let cukta_committed_state_value = cukta_committed_state.read().clone();
    let cukta_page_value = cukta_page.read().clone();
    let vlacku_committed_state_value = vlacku_committed_state.read().clone();
    let vlacku_result_value = vlacku_result.read().clone();
    let gimfihi_committed_state_value = gimfihi_committed_state.read().clone();
    let gimfihi_result_value = gimfihi_result.read().clone();
    let page_find_state_value = page_find_state.read().clone();
    let current_page_find_route_state = page_find_state_value.route_state(route_value).clone();
    let page_find_entries = if current_page_find_route_state.query.is_empty() {
        Vec::new()
    } else {
        page_find_entries_for_route(
            route_value,
            &cukta_page_value,
            &vlacku_committed_state_value,
            &vlacku_result_value,
            &gimfihi_committed_state_value,
            &gimfihi_result_value,
            &result,
            gentufa_request.as_ref(),
            view_mode_value,
            gentufa_display_value,
            settings_value,
            &dialect_settings_value,
            &settings_dialect_selection_value,
            &embedding_settings_value,
            settings_value.script,
        )
    };
    let page_find_index =
        build_page_find_index(&current_page_find_route_state.query, &page_find_entries);
    let page_find_context = PageFindContext::new(&page_find_index, &current_page_find_route_state);
    let committed_gentufa_state = gentufa_state_from_parts(
        &parsed_text_value,
        &parsed_dialect_value,
        view_mode_value,
        gentufa_display_value,
        parsed_text_explicit_value,
    );
    let gentufa_url_inputs = new!(GentufaUrlInputs {
        active_route: route_value,
        current_route: current_route_location.clone(),
        state: committed_gentufa_state.clone(),
        text_explicit: parsed_text_explicit_value,
        intent: gentufa_url_write_intent_value,
    });
    let gentufa_compute_inputs = GentufaComputeInputs {
        route: route_value,
        settings: settings_value,
        dialect_settings: dialect_settings_value.clone(),
        display: gentufa_display_value,
        view_mode: view_mode_value,
        text: parsed_text_value.clone(),
        dialect_text: parsed_dialect_value.clone(),
        text_explicit: parsed_text_explicit_value,
    };
    let gentufa_layout_inputs = GentufaLayoutInputs {
        route: route_value,
        parsed_text_len: parsed_text_value.len(),
        parsed_dialect_len: parsed_dialect_value.len(),
        display: gentufa_display_value,
        view_mode: view_mode_value,
    };
    let topbar_cukta_route =
        JbotciRoute::from_web_route(WebRoute::Cukta(cukta_committed_state_value.clone()), false);
    let topbar_vlacku_route = JbotciRoute::from_web_route(
        WebRoute::Vlacku(vlacku_committed_state_value.clone()),
        false,
    );
    let topbar_gimfihi_route = JbotciRoute::from_web_route(
        WebRoute::Gimfihi(gimfihi_committed_state_value.clone()),
        false,
    );
    let topbar_gentufa_route =
        gentufa_route_for_committed_state(&committed_gentufa_state, parsed_text_explicit_value);
    let topbar_settings_route = JbotciRoute::from_web_route(WebRoute::Settings, false);
    install_browser_dom_handlers(
        jvozba_available,
        topbar_settings_layout,
        topbar_settings_open,
        topbar_nav_layout,
        cukta_toc_forced_autohide,
    );
    let scroll_base_path = base_path.clone();
    let scroll_route_location = current_route_location.clone();
    use_effect(use_reactive(
        (&scroll_route_location,),
        move |(location,)| {
            let previous = last_route_for_scroll.read().clone();
            if previous == location {
                return;
            }
            let scroll_already_pending = pending_cukta_scroll.read().is_some();
            if !scroll_already_pending {
                if let Some(scroll) =
                    cukta_pending_scroll_for_route_change(&scroll_base_path, &location)
                {
                    let mut pending = pending_cukta_scroll;
                    pending.set(Some(scroll));
                }
            }
            last_route_for_scroll.set(location);
        },
    ));
    let document_meta_route_location = current_route_location.clone();
    let document_meta_base_path = base_path.clone();
    use_effect(use_reactive(
        (&document_meta_route_location,),
        move |(location,)| {
            let meta = route_document_meta(&document_meta_base_path, &location);
            apply_document_meta(document_meta, meta);
        },
    ));
    let sync_route_location = current_route_location.clone();
    use_effect(use_reactive((&sync_route_location,), move |(location,)| {
        let is_local_route_write =
            pending_local_route_writes.with_mut(|pending| pending.consume(&location));
        apply_web_route_to_client_state(
            &location,
            is_local_route_write,
            route,
            cukta_draft_state,
            cukta_committed_state,
            vlacku_draft_state,
            vlacku_committed_state,
            gimfihi_draft_state,
            gimfihi_committed_state,
            gimfihi_source_word_memory,
            input_text,
            parsed_text,
            parsed_text_explicit,
            dialect,
            parsed_dialect,
            view_mode,
            gentufa_display,
        );
    }));
    use_effect(move || {
        let current = *route.read();
        let previous = *last_page_find_route.read();
        if previous == current {
            return;
        }
        page_find_state.with_mut(|state| {
            reset_page_find_active(state.route_state_mut(previous));
            reset_page_find_active(state.route_state_mut(current));
        });
        last_page_find_route.set(current);
    });
    let page_find_signature = page_find_index.signature;
    let page_find_match_count = page_find_index.matches.len();
    use_effect(use_reactive(
        &(route_value, page_find_signature, page_find_match_count),
        move |(route, signature, match_count)| {
            page_find_state.with_mut(|state| {
                sync_page_find_result_signature(state, route, signature, match_count);
            });
        },
    ));
    let page_find_scroll_request = current_page_find_route_state.scroll_request;
    let page_find_active_index = page_find_context.active_index;
    use_effect(use_reactive(
        &(
            route_value,
            page_find_scroll_request,
            page_find_active_index,
        ),
        move |(_route, scroll_request, active_index)| {
            if scroll_request > 0
                && let Some(active_index) = active_index
            {
                schedule_page_find_match_scroll(active_index);
            }
        },
    ));
    use_effect(move || {
        pin_worker_client_asset();
        configure_embedding_worker_url(&format!("{EMBEDDING_WORKER_JS}"));
        configure_embedding_ort_assets(
            &format!("{ORT_WASM_MIN_MJS}"),
            &format!("{ORT_WASM_SIMD_THREADED_MJS}"),
            &format!("{ORT_WASM_SIMD_THREADED_WASM}"),
        );
        configure_embedding_remote_base_url(web_embeddings_base_url());
        configure_embedding_model_catalog();
        configure_embedding_model_key(&embedding_settings.read().selected_model_key);
        configure_compute_worker_url(&format!("{COMPUTE_WORKER_JS}"));
    });
    use_effect(move || {
        let active = activity.read().is_active();
        let mut visible = activity_indicator_visible;
        let mut delay_task = activity_indicator_delay_task;
        if !active {
            if let Some(task) = delay_task.write().take() {
                task.cancel();
            }
            visible.set(false);
            return;
        }
        if *visible.read() || delay_task.read().is_some() {
            return;
        }
        let activity_for_delay = activity;
        let mut visible_for_delay = visible;
        let mut delay_task_for_delay = delay_task;
        let task = spawn(async move {
            platform::sleep_ms(ASYNC_ACTIVITY_INDICATOR_DELAY_MS).await;
            if activity_for_delay.read().is_active() {
                visible_for_delay.set(true);
            }
            delay_task_for_delay.set(None);
        });
        delay_task.set(Some(task));
    });
    use_effect(move || {
        if *route.read() == AppRoute::Settings {
            spawn_tracked(activity, AsyncTaskKind::Settings, async move {
                refresh_embedding_settings(embedding_settings).await;
            });
        }
    });
    let settings_route_location = current_route_location.clone();
    use_effect(use_reactive(
        (&settings_route_location,),
        move |(location,)| {
            if location.app_route() != AppRoute::Settings {
                return;
            }
            let Some(raw_johau) = query_param(&location.settings_query, "johau") else {
                return;
            };
            if settings_johau_import_seen.read().as_deref() == Some(raw_johau.as_str()) {
                return;
            }
            settings_johau_import_seen.set(Some(raw_johau.clone()));
            let current_settings = dialect_settings.read().clone();
            if let Ok((selected_name, next_settings)) =
                import_johau_dialect_settings(&raw_johau, &current_settings)
            {
                save_dialect_settings(&next_settings);
                dialect_settings.set(next_settings);
                settings_dialect_selection.set(selected_name);
            }
        },
    ));
    let gentufa_base_path = base_path.clone();
    use_effect(use_reactive((&gentufa_compute_inputs,), move |(inputs,)| {
        if inputs.route != AppRoute::Gentufa {
            cancel_compute_channel(COMPUTE_CHANNEL_GENTUFA);
            cancel_latest_task(gentufa_page_task);
            return;
        }
        let state = gentufa_state_from_parts(
            &inputs.text,
            &inputs.dialect_text,
            inputs.view_mode,
            inputs.display,
            inputs.text_explicit,
        );
        let request = GentufaWebRequest {
            text: inputs.text.clone(),
            options: web_options(
                inputs.settings,
                inputs.display,
                inputs.view_mode,
                inputs.dialect_text.clone(),
                &inputs.dialect_settings,
            ),
        };
        let mut page_signal = gentufa_page;
        page_signal.with_mut(|page| {
            page.state = Some(state.clone());
            page.request = Some(request.clone());
            page.loading = true;
            page.error = None;
        });
        let base_path = gentufa_base_path.clone();
        let mut result_signal = gentufa_page;
        cancel_compute_channel(COMPUTE_CHANNEL_GENTUFA);
        spawn_latest_tracked(
            gentufa_page_task,
            activity,
            AsyncTaskKind::Gentufa,
            async move {
                let response = compute_request(
                    COMPUTE_CHANNEL_GENTUFA,
                    WebComputeRequest::GentufaPage {
                        base_path,
                        state: state.clone(),
                        request: request.clone(),
                    },
                )
                .await;
                match response {
                    Ok(WebComputeResponse::GentufaPage { result, meta }) => {
                        result_signal.set(GentufaAsyncPageState {
                            state: Some(state),
                            request: Some(request),
                            result,
                            meta: Some(meta.clone()),
                            loading: false,
                            error: None,
                        });
                        apply_document_meta(document_meta, meta);
                        schedule_gentufa_block_reference_layout();
                        schedule_gentufa_tree_layout();
                    }
                    Ok(_) => {
                        result_signal.set(gentufa_async_error_state(
                            state,
                            request,
                            "compute worker returned the wrong gentufa response",
                        ));
                    }
                    Err(error) => {
                        result_signal.set(gentufa_async_error_state(state, request, &error));
                    }
                }
            },
        );
    }));
    use_effect(move || {
        let state = vlacku_committed_state.read().clone();
        let mut result_signal = vlacku_semantic_result;
        if *route.read() != AppRoute::Vlacku
            || state.mode != VlackuWebMode::Meaning
            || state.query.trim().is_empty()
        {
            cancel_embedding_channel(EMBEDDING_CHANNEL_VLACKU_SEMANTIC);
            cancel_latest_task(vlacku_semantic_task);
            result_signal.set(VlackuSemanticResultState::default());
            return;
        }
        result_signal.set(VlackuSemanticResultState {
            state: Some(state.clone()),
            hits: Vec::new(),
            message: None,
            loading: true,
        });
        cancel_embedding_channel(EMBEDDING_CHANNEL_VLACKU_SEMANTIC);
        spawn_latest_tracked(
            vlacku_semantic_task,
            activity,
            AsyncTaskKind::Vlacku,
            async move {
                spawn_vlacku_semantic_loading_message(result_signal, state.clone());
                let result = load_vlacku_semantic_result(state).await;
                result_signal.set(result);
            },
        );
    });
    let vlacku_page_base_path = base_path.clone();
    use_effect(move || {
        if *route.read() != AppRoute::Vlacku {
            cancel_compute_channel(COMPUTE_CHANNEL_VLACKU);
            cancel_latest_task(vlacku_result_task);
            return;
        }
        let state = vlacku_committed_state.read().clone();
        let semantic = vlacku_semantic_result.read().clone();
        let mut page_signal = vlacku_result;
        if vlacku_semantic_result_is_pending(&state, &semantic) {
            cancel_compute_channel(COMPUTE_CHANNEL_VLACKU);
            cancel_latest_task(vlacku_result_task);
            let meta = page_signal.with_mut(|page| {
                apply_vlacku_semantic_pending_page(page, &vlacku_page_base_path, &state, &semantic)
            });
            apply_document_meta(document_meta, meta);
            return;
        }
        let request = vlacku_compute_request(&vlacku_page_base_path, &state, &semantic);
        page_signal.with_mut(|page| {
            page.state = Some(state.clone());
            page.loading = true;
            page.error = None;
        });
        let mut result_signal = vlacku_result;
        cancel_compute_channel(COMPUTE_CHANNEL_VLACKU);
        spawn_latest_tracked(
            vlacku_result_task,
            activity,
            AsyncTaskKind::Vlacku,
            async move {
                let response = compute_request(COMPUTE_CHANNEL_VLACKU, request).await;
                match response {
                    Ok(WebComputeResponse::VlackuPage { result, meta }) => {
                        result_signal.set(VlackuAsyncResultState {
                            state: Some(state),
                            result,
                            meta: Some(meta.clone()),
                            loading: false,
                            error: None,
                        });
                        apply_document_meta(document_meta, meta);
                    }
                    Ok(_) => {
                        result_signal.set(vlacku_async_error_state(
                            &state,
                            "compute worker returned the wrong vlacku response",
                        ));
                    }
                    Err(error) => {
                        result_signal.set(vlacku_async_error_state(&state, &error));
                    }
                }
                schedule_vlacku_jvozba_pane_metrics_sync();
            },
        );
    });
    let gimfihi_page_base_path = base_path.clone();
    use_effect(move || {
        if *route.read() != AppRoute::Gimfihi {
            cancel_compute_channel(COMPUTE_CHANNEL_GIMFIHI);
            cancel_latest_task(gimfihi_result_task);
            return;
        }
        let state = gimfihi_committed_state.read().clone();
        if !gimfihi_state_has_any_source_word(&state) {
            cancel_compute_channel(COMPUTE_CHANNEL_GIMFIHI);
            cancel_latest_task(gimfihi_result_task);
            let mut idle_result_signal = gimfihi_result;
            idle_result_signal.set(gimfihi_idle_result_state(&state));
            return;
        }
        let cache_key = gimfihi_generation_cache_key(&state);
        if let Some(cached) = gimfihi_result_cache.read().get(&cache_key).cloned()
            && let Some(cached_result) =
                gimfihi_cached_result_for_state(&gimfihi_page_base_path, &state, cached)
        {
            cancel_compute_channel(COMPUTE_CHANNEL_GIMFIHI);
            cancel_latest_task(gimfihi_result_task);
            if let Some(meta) = cached_result.meta.clone() {
                apply_document_meta(document_meta, meta);
            }
            let mut cached_result_signal = gimfihi_result;
            cached_result_signal.set(cached_result);
            return;
        }
        let mut page_signal = gimfihi_result;
        page_signal.with_mut(|page| {
            page.state = Some(state.clone());
            page.loading = true;
            page.error = None;
        });
        let mut result_signal = gimfihi_result;
        let mut cache_signal = gimfihi_result_cache;
        let request = WebComputeRequest::GimfihiPage {
            base_path: gimfihi_page_base_path.clone(),
            state: state.clone(),
        };
        cancel_compute_channel(COMPUTE_CHANNEL_GIMFIHI);
        spawn_latest_tracked(
            gimfihi_result_task,
            activity,
            AsyncTaskKind::Gimfihi,
            async move {
                let response = compute_request(COMPUTE_CHANNEL_GIMFIHI, request).await;
                match response {
                    Ok(WebComputeResponse::GimfihiPage { result, meta }) => {
                        let next = GimfihiAsyncResultState {
                            state: Some(state),
                            result,
                            meta: Some(meta.clone()),
                            loading: false,
                            error: None,
                        };
                        // The cache projection requires a candidate output so it
                        // can reapply highlights. Caching an output-less error
                        // response would miss that projection and immediately
                        // relaunch this effect for the same committed state.
                        if next.result.output.is_some() {
                            cache_signal.with_mut(|cache| {
                                cache.insert(cache_key, next.clone());
                                while cache.len() > 16 {
                                    if let Some(first_key) = cache.keys().next().cloned() {
                                        cache.remove(&first_key);
                                    } else {
                                        break;
                                    }
                                }
                            });
                        }
                        result_signal.set(next);
                        apply_document_meta(document_meta, meta);
                    }
                    Ok(_) => {
                        result_signal.set(gimfihi_async_error_state(
                            &state,
                            "compute worker returned the wrong gimfihi response",
                        ));
                    }
                    Err(error) => {
                        result_signal.set(gimfihi_async_error_state(&state, &error));
                    }
                }
            },
        );
    });
    use_effect(move || {
        let mut result_signal = cukta_semantic_result;
        let state = cukta_committed_state.read().clone();
        let search_state = match state.view {
            CuktaWebView::Search(search_state)
                if search_state.mode == CuktaWebMode::Meaning
                    && !search_state.query.trim().is_empty() =>
            {
                search_state
            }
            _ => {
                cancel_embedding_channel(EMBEDDING_CHANNEL_CUKTA_SEMANTIC);
                cancel_latest_task(cukta_semantic_task);
                result_signal.set(CuktaSemanticResultState::default());
                return;
            }
        };
        if *route.read() != AppRoute::Cukta {
            cancel_embedding_channel(EMBEDDING_CHANNEL_CUKTA_SEMANTIC);
            cancel_latest_task(cukta_semantic_task);
            result_signal.set(CuktaSemanticResultState::default());
            return;
        }
        result_signal.set(CuktaSemanticResultState {
            state: Some(search_state.clone()),
            hits: Vec::new(),
            message: None,
            loading: true,
        });
        cancel_embedding_channel(EMBEDDING_CHANNEL_CUKTA_SEMANTIC);
        spawn_latest_tracked(
            cukta_semantic_task,
            activity,
            AsyncTaskKind::Cukta,
            async move {
                spawn_cukta_semantic_loading_message(result_signal, search_state.clone());
                let result = load_cukta_semantic_result(search_state).await;
                result_signal.set(result);
            },
        );
    });
    let cukta_page_base_path = base_path.clone();
    use_effect(move || {
        if *route.read() != AppRoute::Cukta {
            cancel_compute_channel(COMPUTE_CHANNEL_CUKTA);
            cancel_latest_task(cukta_page_task);
            return;
        }
        let state = cukta_committed_state.read().clone();
        let semantic = cukta_semantic_result.read().clone();
        let request = cukta_compute_request(&cukta_page_base_path, &state, &semantic);
        let mut page_signal = cukta_page;
        page_signal.with_mut(|page| {
            page.state = Some(state.clone());
            page.loading = true;
            page.error = None;
        });
        let mut result_signal = cukta_page;
        cancel_compute_channel(COMPUTE_CHANNEL_CUKTA);
        spawn_latest_tracked(
            cukta_page_task,
            activity,
            AsyncTaskKind::Cukta,
            async move {
                let response = compute_request(COMPUTE_CHANNEL_CUKTA, request).await;
                match response {
                    Ok(WebComputeResponse::CuktaPage { page, meta }) => {
                        result_signal.set(CuktaAsyncPageState {
                            state: Some(state),
                            page,
                            meta: Some(meta.clone()),
                            loading: false,
                            error: None,
                        });
                        apply_document_meta(document_meta, meta);
                    }
                    Ok(_) => {
                        result_signal.set(cukta_async_error_state(
                            state,
                            "compute worker returned the wrong cukta response",
                        ));
                    }
                    Err(error) => {
                        result_signal.set(cukta_async_error_state(state, &error));
                    }
                }
            },
        );
    });
    let cukta_scroll_route = route;
    let cukta_scroll_state = cukta_committed_state;
    let cukta_scroll_page = cukta_page;
    let mut cukta_scroll_pending = pending_cukta_scroll;
    use_effect(move || {
        if cukta_scroll_pending.read().is_none() {
            return;
        }
        if *cukta_scroll_route.read() != AppRoute::Cukta {
            return;
        }
        let page_ready = {
            let state = cukta_scroll_state.read();
            let page = cukta_scroll_page.read();
            cukta_page_ready_for_scroll(&page, &state)
        };
        if !page_ready {
            return;
        }
        if let Some(scroll) = cukta_scroll_pending.write().take() {
            apply_cukta_pending_scroll(scroll);
        }
    });
    let vlacku_url_history = app_history.clone();
    let vlacku_url_route_location = current_route_location.clone();
    let mut vlacku_url_scroll_restore = pending_vlacku_scroll_restore;
    use_effect(move || {
        if *route.read() == AppRoute::Vlacku {
            let state = vlacku_committed_state.read().clone();
            let restore_scroll_y = vlacku_url_scroll_restore.write().take();
            schedule_vlacku_url_push(
                vlacku_url_history.clone(),
                pending_local_route_writes,
                &vlacku_url_route_location,
                &state,
                restore_scroll_y,
            );
        }
    });
    let gimfihi_url_route_location = current_route_location.clone();
    let gimfihi_url_history = app_history.clone();
    use_effect(move || {
        if *route.read() == AppRoute::Gimfihi {
            let state = gimfihi_committed_state.read().clone();
            push_gimfihi_url(
                gimfihi_url_history.clone(),
                pending_local_route_writes,
                &gimfihi_url_route_location,
                &state,
            );
        }
    });
    let cukta_url_route_location = current_route_location.clone();
    let cukta_url_history = app_history.clone();
    use_effect(move || {
        if *route.read() == AppRoute::Cukta {
            let state = cukta_committed_state.read().clone();
            push_cukta_url(
                cukta_url_history.clone(),
                pending_local_route_writes,
                &cukta_url_route_location,
                &state,
            );
        }
    });
    let gentufa_url_history = app_history.clone();
    let mut gentufa_url_intent_for_effect = gentufa_url_write_intent;
    use_effect(use_reactive((&gentufa_url_inputs,), move |(inputs,)| {
        if !gentufa_url_sync_allowed(inputs.active_route, &inputs.current_route) {
            set_gentufa_url_write_intent_if_changed(
                &mut gentufa_url_intent_for_effect,
                inputs.intent,
                GentufaUrlWriteIntent::ReplaceCurrent,
            );
            return;
        }
        sync_gentufa_committed_url(
            gentufa_url_history.clone(),
            pending_local_route_writes,
            &inputs.current_route,
            &inputs.state,
            inputs.text_explicit,
            inputs.intent,
            gentufa_url_intent_for_effect,
        );
    }));
    use_effect(move || {
        if *route.read() == AppRoute::Vlacku {
            let state = vlacku_draft_state.read().clone();
            let pane_open = jvozba_pane.read().open;
            let pane_available = *jvozba_available.read();
            set_brivla_toggle_indeterminate(vlacku_brivla_filter_indeterminate(&state.word_types));
            let _ = (pane_open, pane_available);
            schedule_vlacku_jvozba_pane_metrics_sync();
        }
    });
    use_effect(move || {
        if *route.read() == AppRoute::Cukta {
            restore_cukta_toc_scroll();
        }
    });
    use_effect(move || {
        let _ = (
            *route.read(),
            settings.read().theme,
            settings.read().script,
            activity.read().is_active(),
            *topbar_settings_layout.read(),
            *topbar_nav_layout.read(),
        );
        schedule_topbar_settings_layout_measure(
            topbar_settings_layout,
            topbar_settings_open,
            topbar_nav_layout,
        );
    });
    use_effect(move || {
        let _ = (*route.read(), *topbar_nav_layout.read());
        schedule_topbar_active_nav_sync();
        if *route.read() == AppRoute::Vlacku {
            schedule_vlacku_jvozba_pane_metrics_sync();
        }
    });
    use_effect(use_reactive((&gentufa_layout_inputs,), move |(inputs,)| {
        if inputs.route == AppRoute::Gentufa {
            schedule_gentufa_block_reference_layout();
            schedule_gentufa_tree_layout();
        }
    }));
    use_effect(move || {
        if *route.read() == AppRoute::Gentufa {
            let _ = input_text.read().len();
            schedule_gentufa_textarea_resize();
        }
    });
    let app_class = format!(
        "spa-shell app-page theme-{} orthography-{}",
        theme_class(settings_value.theme),
        script_class(settings_value.script)
    );
    let manifest_href = static_asset_href_with_base_path(&base_path, MANIFEST_ASSET_PATH);
    let favicon_href = static_asset_href_with_base_path(&base_path, FAVICON_ASSET_PATH);
    let apple_touch_icon_href =
        static_asset_href_with_base_path(&base_path, APPLE_TOUCH_ICON_ASSET_PATH);

    rsx! {
        document::Title { "{document_title}" }
        style { "{font_face_css()}\n{critical_startup_css()}" }
        document::Stylesheet { href: MAIN_CSS }
        if cfg!(target_arch = "wasm32") {
            document::Link { rel: "modulepreload", href: COMPUTE_WORKER_JS }
            document::Link { rel: "modulepreload", href: EMBEDDING_WORKER_JS }
            document::Link { rel: "manifest", href: "{manifest_href}" }
        }
        document::Link { rel: "icon", r#type: "image/png", href: "{favicon_href}" }
        document::Link { rel: "shortcut icon", r#type: "image/png", href: "{favicon_href}" }
        document::Link { rel: "apple-touch-icon", href: "{apple_touch_icon_href}" }
        div { class: "{app_class}",
            { render_topbar(
                route_value,
                settings,
                settings_value,
                topbar_cukta_route,
                topbar_vlacku_route,
                topbar_gimfihi_route,
                topbar_gentufa_route,
                topbar_settings_route,
                &base_path,
                pending_cukta_scroll,
                *topbar_settings_layout.read(),
                topbar_settings_open,
                *topbar_nav_layout.read(),
                page_find_state,
                &page_find_context,
                &activity_value,
                activity_indicator_visible_value,
            ) }
            main { class: "spa-main", "data-app-scroll": "main",
                div { class: "spa-stack",
                    {
                        match route_value {
                            AppRoute::Gentufa => rsx! {
                                GentufaPage {
                                    input_text,
                                    dialect,
                                    dialect_settings: dialect_settings_value.clone(),
                                    dialect_picker_open: gentufa_dialect_picker_open,
                                    parsed_text_explicit,
                                    parsed_text,
                                    parsed_dialect,
                                    url_write_intent: gentufa_url_write_intent,
                                    result: result.clone(),
                                    request: gentufa_request.clone(),
                                    diagnostics_open: gentufa_diagnostics_open,
                                    active_diagnostic: gentufa_active_diagnostic,
                                    input_diagnostic_tooltip: gentufa_input_diagnostic_tooltip,
                                    pending_cukta_scroll,
                                    base_path: base_path.clone(),
                                    view_mode,
                                    view_mode_value,
                                    display: gentufa_display,
                                    display_value: gentufa_display_value,
                                    settings: settings_value,
                                    reference_hover,
                                    reference_tooltip_open,
                                    activity,
                                    export_task,
                                    page_find: page_find_context.clone(),
                                }
                            },
                            AppRoute::Settings => rsx! {
                                SettingsPage {
                                    settings,
                                    dialect_settings,
                                    selected_dialect: settings_dialect_selection,
                                    qr_uri: settings_dialect_qr_uri,
                                    embedding_settings,
                                    activity,
                                    page_find: page_find_context.clone(),
                                }
                            },
                            AppRoute::Cukta => rsx! {
                                CuktaPage {
                                    cukta_draft_state,
                                    cukta_committed_state,
                                    cukta_page,
                                    toc_filter: cukta_toc_filter,
                                    toc_pinned: cukta_toc_pinned,
                                    toc_expansion: cukta_toc_expansion,
                                    toc_width: cukta_toc_width,
                                    toc_resize: cukta_toc_resize,
                                    toc_overlay_visible: cukta_toc_overlay_visible,
                                    toc_forced_autohide: cukta_toc_forced_autohide,
                                    pending_cukta_scroll,
                                    base_path: base_path.clone(),
                                    script: settings_value.script,
                                    page_find: page_find_context.clone(),
                                }
                            },
                            AppRoute::Vlacku => rsx! {
                                VlackuPage {
                                    vlacku_draft_state,
                                    vlacku_committed_state,
                                    vlacku_result,
                                    jvozba_pane,
                                    jvozba_available,
                                    jvozba_drag,
                                    pending_cukta_scroll,
                                    pending_vlacku_scroll_restore,
                                    base_path: base_path.clone(),
                                    script: settings_value.script,
                                    page_find: page_find_context.clone(),
                                }
                            },
                            AppRoute::Gimfihi => rsx! {
                                GimfihiPage {
                                    gimfihi_draft_state,
                                    gimfihi_committed_state,
                                    gimfihi_result,
                                    gimfihi_source_word_memory,
                                    base_path: base_path.clone(),
                                    script: settings_value.script,
                                    page_find: page_find_context.clone(),
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_topbar(
    route: AppRoute,
    settings: Signal<UserSettings>,
    current: UserSettings,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gimfihi_route: JbotciRoute,
    gentufa_route: JbotciRoute,
    settings_route: JbotciRoute,
    base_path: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
    settings_layout: TopbarSettingsLayout,
    settings_open: Signal<bool>,
    nav_layout: TopbarNavLayout,
    page_find_state: Signal<PageFindState>,
    page_find: &PageFindContext,
    activity: &AsyncActivityState,
    activity_visible: bool,
) -> Element {
    let cukta_loading = activity_visible && activity.has_kind(AsyncTaskKind::Cukta);
    let vlacku_loading = activity_visible && activity.has_kind(AsyncTaskKind::Vlacku);
    let gimfihi_loading = activity_visible && activity.has_kind(AsyncTaskKind::Gimfihi);
    let gentufa_loading = activity_visible && activity.has_kind(AsyncTaskKind::Gentufa);
    let activity_class = topbar_activity_class(activity_visible);
    let header_class = topbar_header_class(settings_layout, *settings_open.read(), nav_layout);
    let show_theme_inline = settings_layout.shows_theme_inline();
    let show_script_inline = settings_layout.shows_script_inline();
    let topbar_home_href = deployment_root_href(base_path);
    let logo_title = logo_title_text();
    rsx! {
        header { class: "{header_class}",
            div { class: "app-topbar-inner spa-topbar-inner",
                div { class: "app-topbar-left",
                    a {
                        class: "app-topbar-brand",
                        href: "{topbar_home_href}",
                        aria_label: "jbotci home",
                        title: "{logo_title}",
                        img { class: "app-topbar-brand-logo", src: LOGO, alt: "jbotci" }
                    }
                    { render_topbar_settings_button(settings, current, settings_route.clone(), settings_layout, settings_open) }
                    if show_theme_inline {
                        span { class: "app-topbar-theme app-topbar-theme-mode",
                            { render_theme_switch(settings, current.theme) }
                        }
                    }
                    if show_script_inline {
                        span { class: "app-topbar-theme app-topbar-orthography",
                            { render_script_switch(settings, current.script) }
                        }
                    }
                    match nav_layout {
                        TopbarNavLayout::Full => {
                            { render_topbar_nav(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gimfihi_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
                        }
                        TopbarNavLayout::Carousel => {
                            { render_topbar_nav_carousel(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gimfihi_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
                        }
                    }
                }
                { render_topbar_fit_probes(
                    settings,
                    current,
                    route,
                    cukta_loading,
                    vlacku_loading,
                    gimfihi_loading,
                    gentufa_loading,
                    cukta_route,
                    vlacku_route,
                    gimfihi_route,
                    gentufa_route,
                    base_path,
                    pending_cukta_scroll,
                ) }
                div { class: "{activity_class}", role: "status", aria_live: "polite",
                    span { class: "sr-only",
                        if activity_visible {
                            "Working"
                        }
                    }
                    span { class: "app-topbar-activity-dots", aria_hidden: "true",
                        span { class: "app-topbar-activity-dot" }
                        span { class: "app-topbar-activity-dot" }
                        span { class: "app-topbar-activity-dot" }
                    }
                }
                div { class: "app-topbar-right",
                    { render_page_find_control(route, page_find_state, page_find) }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_topbar_nav(
    route: AppRoute,
    cukta_loading: bool,
    vlacku_loading: bool,
    gimfihi_loading: bool,
    gentufa_loading: bool,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gimfihi_route: JbotciRoute,
    gentufa_route: JbotciRoute,
    base_path: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
) -> Element {
    let topbar_cukta_scroll_target = route_href_with_base_path(base_path, &cukta_route);
    let topbar_cukta_click_route = cukta_route.clone();
    rsx! {
        nav { class: "spa-nav", aria_label: "Primary navigation",
            Link {
                class: topbar_link_class(route == AppRoute::Cukta, cukta_loading),
                to: cukta_route,
                aria_current: if route == AppRoute::Cukta { "page" } else { "false" },
                onclick_only: true,
                onclick: move |_| {
                    push_route_with_cukta_scroll_intent(
                        pending_cukta_scroll,
                        Some(cukta_stored_pending_scroll(topbar_cukta_scroll_target.clone())),
                        topbar_cukta_click_route.clone(),
                    );
                },
                span { class: "app-topbar-link-label", "cukta" }
            }
            Link {
                class: topbar_link_class(route == AppRoute::Vlacku, vlacku_loading),
                to: vlacku_route,
                aria_current: if route == AppRoute::Vlacku { "page" } else { "false" },
                span { class: "app-topbar-link-label", "vlacku" }
            }
            Link {
                class: topbar_link_class(route == AppRoute::Gentufa, gentufa_loading),
                to: gentufa_route,
                aria_current: if route == AppRoute::Gentufa { "page" } else { "false" },
                span { class: "app-topbar-link-label", "gentufa" }
                span { class: "app-topbar-link-dots", aria_hidden: "true",
                    span { class: "app-topbar-link-dot" }
                    span { class: "app-topbar-link-dot" }
                    span { class: "app-topbar-link-dot" }
                }
            }
            Link {
                class: topbar_link_class(route == AppRoute::Gimfihi, gimfihi_loading),
                to: gimfihi_route,
                aria_current: if route == AppRoute::Gimfihi { "page" } else { "false" },
                span { class: "app-topbar-link-label", "gimfi'i" }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(true)]
#[ensures(true)]
pub(super) fn render_topbar_nav_carousel(
    route: AppRoute,
    cukta_loading: bool,
    vlacku_loading: bool,
    gimfihi_loading: bool,
    gentufa_loading: bool,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gimfihi_route: JbotciRoute,
    gentufa_route: JbotciRoute,
    base_path: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
) -> Element {
    let [first_route, second_route, third_route, fourth_route] = topbar_carousel_routes(route);
    rsx! {
        nav { class: "spa-nav app-topbar-nav-carousel", aria_label: "Primary navigation",
            div { class: "app-topbar-nav-carousel-track",
                { render_topbar_nav_carousel_link(
                    first_route,
                    route,
                    topbar_carousel_route_slot_class(first_route, route),
                    cukta_loading,
                    vlacku_loading,
                    gimfihi_loading,
                    gentufa_loading,
                    cukta_route.clone(),
                    vlacku_route.clone(),
                    gimfihi_route.clone(),
                    gentufa_route.clone(),
                    base_path,
                    pending_cukta_scroll,
                ) }
                { render_topbar_nav_carousel_link(
                    second_route,
                    route,
                    topbar_carousel_route_slot_class(second_route, route),
                    cukta_loading,
                    vlacku_loading,
                    gimfihi_loading,
                    gentufa_loading,
                    cukta_route.clone(),
                    vlacku_route.clone(),
                    gimfihi_route.clone(),
                    gentufa_route.clone(),
                    base_path,
                    pending_cukta_scroll,
                ) }
                { render_topbar_nav_carousel_link(
                    third_route,
                    route,
                    topbar_carousel_route_slot_class(third_route, route),
                    cukta_loading,
                    vlacku_loading,
                    gimfihi_loading,
                    gentufa_loading,
                    cukta_route.clone(),
                    vlacku_route.clone(),
                    gimfihi_route.clone(),
                    gentufa_route.clone(),
                    base_path,
                    pending_cukta_scroll,
                ) }
                { render_topbar_nav_carousel_link(
                    fourth_route,
                    route,
                    topbar_carousel_route_slot_class(fourth_route, route),
                    cukta_loading,
                    vlacku_loading,
                    gimfihi_loading,
                    gentufa_loading,
                    cukta_route,
                    vlacku_route,
                    gimfihi_route,
                    gentufa_route,
                    base_path,
                    pending_cukta_scroll,
                ) }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(target != AppRoute::Settings)]
#[requires(!slot_class.is_empty())]
#[ensures(true)]
pub(super) fn render_topbar_nav_carousel_link(
    target: AppRoute,
    active_route: AppRoute,
    slot_class: &'static str,
    cukta_loading: bool,
    vlacku_loading: bool,
    gimfihi_loading: bool,
    gentufa_loading: bool,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gimfihi_route: JbotciRoute,
    gentufa_route: JbotciRoute,
    base_path: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
) -> Element {
    let active = target == active_route;
    let loading = topbar_carousel_route_loading(
        target,
        cukta_loading,
        vlacku_loading,
        gimfihi_loading,
        gentufa_loading,
    );
    let class = topbar_carousel_link_class(active, loading, slot_class);
    let aria_current = if active { "page" } else { "false" };
    let data_active = if active { "true" } else { "false" };
    let label = topbar_carousel_route_label(target);
    let target_route = match target {
        AppRoute::Cukta => cukta_route,
        AppRoute::Vlacku => vlacku_route,
        AppRoute::Gimfihi => gimfihi_route,
        AppRoute::Gentufa => gentufa_route,
        AppRoute::Settings => return rsx! {},
    };
    let href = route_href_with_base_path(base_path, &target_route);
    let pending_scroll = if target == AppRoute::Cukta {
        Some(cukta_stored_pending_scroll(href.clone()))
    } else {
        None
    };
    let click_route = target_route.clone();
    rsx! {
        a {
            key: "{label}",
            class: "{class}",
            href: "{href}",
            aria_current,
            "data-topbar-nav-active": data_active,
            onclick: move |event| {
                if !event.modifiers().is_empty() {
                    return;
                }
                event.prevent_default();
                push_route_with_cukta_scroll_intent(
                    pending_cukta_scroll,
                    pending_scroll.clone(),
                    click_route.clone(),
                );
            },
            span { class: "app-topbar-link-label", "{label}" }
            if target == AppRoute::Gentufa {
                span { class: "app-topbar-link-dots", aria_hidden: "true",
                    span { class: "app-topbar-link-dot" }
                    span { class: "app-topbar-link-dot" }
                    span { class: "app-topbar-link-dot" }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(true)]
#[ensures(true)]
pub(super) fn render_topbar_nav_carousel_probe(
    route: AppRoute,
    cukta_loading: bool,
    vlacku_loading: bool,
    gimfihi_loading: bool,
    gentufa_loading: bool,
) -> Element {
    let [first_route, second_route, third_route, fourth_route] = topbar_carousel_routes(route);
    let first_label = topbar_carousel_route_label(first_route);
    let second_label = topbar_carousel_route_label(second_route);
    let third_label = topbar_carousel_route_label(third_route);
    let fourth_label = topbar_carousel_route_label(fourth_route);
    rsx! {
        nav { class: "spa-nav app-topbar-nav-carousel", aria_label: "Primary navigation",
            div { class: "app-topbar-nav-carousel-track",
                span {
                    class: topbar_carousel_link_class(
                        first_route == route,
                        topbar_carousel_route_loading(first_route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading),
                        topbar_carousel_route_slot_class(first_route, route),
                    ),
                    "data-topbar-nav-active": if first_route == route { "true" } else { "false" },
                    span { class: "app-topbar-link-label", "{first_label}" }
                }
                span {
                    class: topbar_carousel_link_class(
                        second_route == route,
                        topbar_carousel_route_loading(second_route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading),
                        topbar_carousel_route_slot_class(second_route, route),
                    ),
                    "data-topbar-nav-active": if second_route == route { "true" } else { "false" },
                    span { class: "app-topbar-link-label", "{second_label}" }
                }
                span {
                    class: topbar_carousel_link_class(
                        third_route == route,
                        topbar_carousel_route_loading(third_route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading),
                        topbar_carousel_route_slot_class(third_route, route),
                    ),
                    "data-topbar-nav-active": if third_route == route { "true" } else { "false" },
                    span { class: "app-topbar-link-label", "{third_label}" }
                }
                span {
                    class: topbar_carousel_link_class(
                        fourth_route == route,
                        topbar_carousel_route_loading(fourth_route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading),
                        topbar_carousel_route_slot_class(fourth_route, route),
                    ),
                    "data-topbar-nav-active": if fourth_route == route { "true" } else { "false" },
                    span { class: "app-topbar-link-label", "{fourth_label}" }
                }
            }
        }
    }
}

#[requires(true)]
#[requires(!slot_class.is_empty())]
#[ensures(!ret.is_empty())]
pub(super) fn topbar_carousel_link_class(
    active: bool,
    loading: bool,
    slot_class: &'static str,
) -> String {
    let base = format!("app-topbar-link app-topbar-carousel-link {slot_class}");
    class_names(&base, &[("active", active), ("is-loading", loading)])
}

#[requires(true)]
#[ensures(!ret.contains(&AppRoute::Settings))]
#[ensures(route == AppRoute::Settings || ret.contains(&route))]
#[ensures(ret[0] == AppRoute::Cukta)]
#[ensures(ret[3] == AppRoute::Gimfihi)]
pub(super) fn topbar_carousel_routes(route: AppRoute) -> [AppRoute; 4] {
    let _ = route;
    TOPBAR_NAV_ROUTES
}

#[requires(target != AppRoute::Settings)]
#[ensures(!ret.is_empty())]
pub(super) fn topbar_carousel_route_slot_class(
    target: AppRoute,
    active_route: AppRoute,
) -> &'static str {
    if target == active_route {
        "is-current-slot"
    } else {
        "is-adjacent"
    }
}

#[requires(route != AppRoute::Settings)]
#[ensures(true)]
pub(super) fn topbar_carousel_route_loading(
    route: AppRoute,
    cukta_loading: bool,
    vlacku_loading: bool,
    gimfihi_loading: bool,
    gentufa_loading: bool,
) -> bool {
    match route {
        AppRoute::Cukta => cukta_loading,
        AppRoute::Vlacku => vlacku_loading,
        AppRoute::Gimfihi => gimfihi_loading,
        AppRoute::Gentufa => gentufa_loading,
        AppRoute::Settings => false,
    }
}

#[requires(route != AppRoute::Settings)]
#[ensures(!ret.is_empty())]
pub(super) fn topbar_carousel_route_label(route: AppRoute) -> &'static str {
    match route {
        AppRoute::Cukta => "cukta",
        AppRoute::Vlacku => "vlacku",
        AppRoute::Gimfihi => "gimfi'i",
        AppRoute::Gentufa => "gentufa",
        AppRoute::Settings => "",
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_page_find_control(
    route: AppRoute,
    mut page_find_state: Signal<PageFindState>,
    page_find: &PageFindContext,
) -> Element {
    let query = page_find.query.clone();
    let placeholder = page_find_placeholder(route);
    let match_count = page_find.match_count;
    let counter = page_find_counter_text(page_find.active_index, match_count, !query.is_empty());
    let controls_disabled = match_count == 0;
    let query_for_keydown = query.clone();
    rsx! {
        div { class: "page-find-control", role: "search",
            span { class: "page-find-icon", aria_hidden: "true",
                svg { view_box: "0 0 20 20",
                    circle { cx: "8.5", cy: "8.5", r: "5.5" }
                    path { d: "M12.5 12.5L17 17" }
                }
            }
            input {
                id: PAGE_FIND_INPUT_ID,
                class: "page-find-input",
                r#type: "search",
                aria_label: "Find on this page",
                placeholder,
                spellcheck: "false",
                value: "{query}",
                oninput: move |event| {
                    let next_query = event.value();
                    page_find_state.with_mut(|state| {
                        set_page_find_query(
                            state,
                            route,
                            next_query,
                            PageFindRouteQueryUpdate::Replace,
                        );
                    });
                },
                onkeydown: move |event| {
                    let key = event.data().key();
                    if key == Key::Enter {
                        event.prevent_default();
                        let direction = if event.data().modifiers().contains(Modifiers::SHIFT) {
                            PageFindDirection::Previous
                        } else {
                            PageFindDirection::Next
                        };
                        page_find_state.with_mut(|state| {
                            update_page_find_active(state, route, direction, match_count);
                        });
                    } else if key == Key::Escape && !query_for_keydown.is_empty() {
                        event.prevent_default();
                        page_find_state.with_mut(|state| {
                            set_page_find_query(
                                state,
                                route,
                                String::new(),
                                PageFindRouteQueryUpdate::Clear,
                            );
                        });
                    }
                },
            }
            span { class: "page-find-actions",
                if !query.is_empty() {
                    button {
                        class: "page-find-button page-find-clear",
                        r#type: "button",
                        aria_label: "Clear page find",
                        title: "Clear",
                        onclick: move |_| {
                            page_find_state.with_mut(|state| {
                                set_page_find_query(
                                    state,
                                    route,
                                    String::new(),
                                    PageFindRouteQueryUpdate::Clear,
                                );
                            });
                        },
                        svg {
                            class: "page-find-button-icon",
                            view_box: "0 0 20 20",
                            "aria-hidden": "true",
                            path {
                                d: "M5 5L15 15M15 5L5 15",
                                fill: "none",
                                stroke: "currentColor",
                                stroke_width: "2.2",
                                stroke_linecap: "round",
                            }
                        }
                    }
                }
                button {
                    class: "page-find-button page-find-prev",
                    r#type: "button",
                    aria_label: "Previous page find match",
                    title: "Previous",
                    disabled: controls_disabled,
                    onclick: move |_| {
                        page_find_state.with_mut(|state| {
                            update_page_find_active(
                                state,
                                route,
                                PageFindDirection::Previous,
                                match_count,
                            );
                        });
                    },
                    svg {
                        class: "page-find-button-icon",
                        view_box: "0 0 20 20",
                        "aria-hidden": "true",
                        path {
                            d: "M12.5 5L7.5 10L12.5 15",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2.2",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                        }
                    }
                }
                if !counter.is_empty() {
                    span { class: "page-find-count", aria_live: "polite", "{counter}" }
                }
                button {
                    class: "page-find-button page-find-next",
                    r#type: "button",
                    aria_label: "Next page find match",
                    title: "Next",
                    disabled: controls_disabled,
                    onclick: move |_| {
                        page_find_state.with_mut(|state| {
                            update_page_find_active(
                                state,
                                route,
                                PageFindDirection::Next,
                                match_count,
                            );
                        });
                    },
                    svg {
                        class: "page-find-button-icon",
                        view_box: "0 0 20 20",
                        "aria-hidden": "true",
                        path {
                            d: "M7.5 5L12.5 10L7.5 15",
                            fill: "none",
                            stroke: "currentColor",
                            stroke_width: "2.2",
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                        }
                    }
                }
            }
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn page_find_placeholder(route: AppRoute) -> &'static str {
    match route {
        AppRoute::Cukta => "Find in section",
        AppRoute::Vlacku => "Find in cards",
        AppRoute::Gimfihi => "Find in candidates",
        AppRoute::Gentufa => "Find in output",
        AppRoute::Settings => "Find in settings",
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn page_find_counter_text(
    active_index: Option<usize>,
    match_count: usize,
    query_present: bool,
) -> String {
    if !query_present {
        String::new()
    } else if match_count == 0 {
        "0/0".to_owned()
    } else {
        let current = active_index.map_or(1, |index| index + 1);
        format!("{current}/{match_count}")
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_topbar_settings_button(
    settings: Signal<UserSettings>,
    current: UserSettings,
    settings_route: JbotciRoute,
    settings_layout: TopbarSettingsLayout,
    mut settings_open: Signal<bool>,
) -> Element {
    let menu_open = *settings_open.read() && settings_layout.uses_popout();
    let button_class = topbar_settings_button_class(menu_open);
    rsx! {
        div { class: "app-topbar-settings",
            if settings_layout.uses_popout() {
                button {
                    class: "{button_class}",
                    r#type: "button",
                    aria_label: "Settings",
                    aria_expanded: if menu_open { "true" } else { "false" },
                    aria_controls: "app-topbar-settings-menu",
                    title: "Settings",
                    onclick: move |_| settings_open.set(!menu_open),
                    span { class: "app-topbar-settings-icon", aria_hidden: "true", "⚙" }
                }
                if menu_open {
                    { render_topbar_settings_menu(settings, current, settings_route, settings_layout) }
                }
            } else {
                Link {
                    class: "{button_class}",
                    to: settings_route,
                    aria_label: "Settings",
                    title: "Settings",
                    span { class: "app-topbar-settings-icon", aria_hidden: "true", "⚙" }
                }
            }
        }
    }
}

#[requires(settings_layout.uses_popout())]
#[ensures(true)]
pub(super) fn render_topbar_settings_menu(
    settings: Signal<UserSettings>,
    current: UserSettings,
    settings_route: JbotciRoute,
    settings_layout: TopbarSettingsLayout,
) -> Element {
    rsx! {
        div {
            id: "app-topbar-settings-menu",
            class: "app-topbar-settings-menu",
            role: "dialog",
            aria_label: "Settings",
            if !settings_layout.shows_theme_inline() {
                div { class: "app-topbar-settings-menu-row",
                    { render_theme_switch(settings, current.theme) }
                }
            }
            if !settings_layout.shows_script_inline() {
                div { class: "app-topbar-settings-menu-row",
                    { render_script_switch(settings, current.script) }
                }
            }
            div { class: "app-topbar-settings-menu-row",
                Link {
                    class: "app-topbar-settings-all",
                    to: settings_route,
                    "All settings"
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(true)]
#[ensures(true)]
pub(super) fn render_topbar_fit_probes(
    settings: Signal<UserSettings>,
    current: UserSettings,
    route: AppRoute,
    cukta_loading: bool,
    vlacku_loading: bool,
    gimfihi_loading: bool,
    gentufa_loading: bool,
    cukta_route: JbotciRoute,
    vlacku_route: JbotciRoute,
    gimfihi_route: JbotciRoute,
    gentufa_route: JbotciRoute,
    base_path: &str,
    pending_cukta_scroll: Signal<Option<CuktaPendingScroll>>,
) -> Element {
    rsx! {
        div {
            class: "app-topbar-fit-probes",
            aria_hidden: "true",
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-both-full",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                span { class: "app-topbar-theme app-topbar-theme-mode",
                    { render_theme_switch(settings, current.theme) }
                }
                span { class: "app-topbar-theme app-topbar-orthography",
                    { render_script_switch(settings, current.script) }
                }
                { render_topbar_nav(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gimfihi_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-theme-full",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                span { class: "app-topbar-theme app-topbar-theme-mode",
                    { render_theme_switch(settings, current.theme) }
                }
                { render_topbar_nav(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gimfihi_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-none-full",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                { render_topbar_nav(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading, cukta_route.clone(), vlacku_route.clone(), gimfihi_route.clone(), gentufa_route.clone(), base_path, pending_cukta_scroll) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-both-carousel",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                span { class: "app-topbar-theme app-topbar-theme-mode",
                    { render_theme_switch(settings, current.theme) }
                }
                span { class: "app-topbar-theme app-topbar-orthography",
                    { render_script_switch(settings, current.script) }
                }
                { render_topbar_nav_carousel_probe(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-theme-carousel",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                span { class: "app-topbar-theme app-topbar-theme-mode",
                    { render_theme_switch(settings, current.theme) }
                }
                { render_topbar_nav_carousel_probe(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading) }
            }
            div { class: "app-topbar-fit-probe app-topbar-fit-probe-none-carousel",
                { render_topbar_probe_brand() }
                { render_topbar_probe_settings_button() }
                { render_topbar_nav_carousel_probe(route, cukta_loading, vlacku_loading, gimfihi_loading, gentufa_loading) }
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_topbar_probe_brand() -> Element {
    rsx! {
        span { class: "app-topbar-brand app-topbar-brand-probe",
            img { class: "app-topbar-brand-logo", src: LOGO, alt: "" }
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_topbar_probe_settings_button() -> Element {
    rsx! {
        span { class: "app-topbar-settings",
            span { class: "app-topbar-settings-toggle", aria_hidden: "true",
                span { class: "app-topbar-settings-icon", "⚙" }
            }
        }
    }
}

#[invariant(!short.is_empty())]
#[invariant(!href.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BuildCommitInfo {
    pub(super) short: String,
    pub(super) href: String,
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|commit| !commit.href.is_empty()))]
pub(super) fn build_commit_info() -> Option<BuildCommitInfo> {
    let Some(full_commit) = BUILD_GIT_COMMIT else {
        return None;
    };
    let Some(short_commit) = BUILD_GIT_COMMIT_SHORT else {
        return None;
    };
    Some(new!(BuildCommitInfo {
        short: short_commit.to_owned(),
        href: format!("https://codeberg.org/int_19h/jbotci/commit/{full_commit}"),
    }))
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn logo_title_text() -> String {
    build_commit_info()
        .map(|commit| format!("jbotci #{}", commit.short))
        .unwrap_or_else(|| "jbotci".to_owned())
}

#[requires(true)]
#[ensures(true)]
pub(super) fn render_settings_commit_link(page_find: &PageFindContext) -> Element {
    let Some(commit) = build_commit_info() else {
        return rsx! {};
    };
    let label = format!("commit {}", commit.short);
    rsx! {
        a {
            class: "settings-commit-link",
            href: "{commit.href}",
            title: "Git commit from which this version of jbotci was built.",
            aria_label: "Build commit {commit.short}",
            { render_page_find_text(page_find, &label) }
        }
    }
}

#[requires(commit.chars().all(|character| character.is_ascii_hexdigit()))]
#[ensures(ret.chars().count() == commit.chars().count())]
pub(super) fn math_monospace_git_commit(commit: &str) -> String {
    commit.chars().map(math_monospace_hex_char).collect()
}

#[requires(character.is_ascii_hexdigit())]
#[ensures(true)]
pub(super) fn math_monospace_hex_char(character: char) -> char {
    const DIGITS: [char; 10] = ['𝟶', '𝟷', '𝟸', '𝟹', '𝟺', '𝟻', '𝟼', '𝟽', '𝟾', '𝟿'];
    const HEX_LETTERS: [char; 6] = ['𝚊', '𝚋', '𝚌', '𝚍', '𝚎', '𝚏'];
    if character.is_ascii_digit() {
        DIGITS[(character as u8 - b'0') as usize]
    } else {
        HEX_LETTERS[(character.to_ascii_lowercase() as u8 - b'a') as usize]
    }
}

impl TopbarSettingsLayout {
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn shows_theme_inline(self) -> bool {
        matches!(self, Self::BothInline | Self::ThemeInline)
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn shows_script_inline(self) -> bool {
        matches!(self, Self::BothInline)
    }

    #[requires(true)]
    #[ensures(ret == !self.shows_script_inline())]
    pub(super) fn uses_popout(self) -> bool {
        !self.shows_script_inline()
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn topbar_header_class(
    settings_layout: TopbarSettingsLayout,
    settings_open: bool,
    nav_layout: TopbarNavLayout,
) -> String {
    format!(
        "app-topbar spa-topbar {} {}{}",
        match settings_layout {
            TopbarSettingsLayout::BothInline => "topbar-settings-both-inline",
            TopbarSettingsLayout::ThemeInline => "topbar-settings-theme-inline",
            TopbarSettingsLayout::NoneInline => "topbar-settings-none-inline",
        },
        match nav_layout {
            TopbarNavLayout::Full => "topbar-nav-full",
            TopbarNavLayout::Carousel => "topbar-nav-carousel",
        },
        if settings_open && settings_layout.uses_popout() {
            " topbar-settings-open"
        } else {
            ""
        }
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(super) fn topbar_settings_button_class(open: bool) -> &'static str {
    if open {
        "app-topbar-settings-toggle is-open"
    } else {
        "app-topbar-settings-toggle"
    }
}
