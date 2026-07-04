use super::*;

#[test]
#[requires(true)]
#[ensures(true)]
fn pwa_manifest_uses_root_routes_and_separate_maskable_icons() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../assets/manifest.webmanifest"))
            .expect("PWA manifest is valid JSON");

    assert_eq!(manifest["id"], "/");
    assert_eq!(manifest["start_url"], "/vlacku");
    assert_eq!(manifest["scope"], "/");
    assert_eq!(manifest["display"], "standalone");
    assert_eq!(
        manifest["display_override"],
        serde_json::json!(["tabbed", "minimal-ui", "standalone"])
    );

    let protocol_handlers = manifest["protocol_handlers"]
        .as_array()
        .expect("protocol_handlers is an array");
    assert!(protocol_handlers.iter().any(|handler| {
        handler.get("protocol").and_then(serde_json::Value::as_str) == Some("web+johau")
            && handler.get("url").and_then(serde_json::Value::as_str) == Some("/settings?johau=%s")
    }));

    let icons = manifest["icons"].as_array().expect("icons is an array");
    let has_icon = |src: &str, sizes: &str, content_type: &str, purpose: &str| {
        icons.iter().any(|icon| {
            icon.get("src").and_then(serde_json::Value::as_str) == Some(src)
                && icon.get("sizes").and_then(serde_json::Value::as_str) == Some(sizes)
                && icon.get("type").and_then(serde_json::Value::as_str) == Some(content_type)
                && icon.get("purpose").and_then(serde_json::Value::as_str) == Some(purpose)
        })
    };
    assert!(has_icon(
        "/assets/icons/jbotci-icon-192.png",
        "192x192",
        "image/png",
        "any"
    ));
    assert!(has_icon(
        "/assets/icons/jbotci-icon-512.png",
        "512x512",
        "image/png",
        "any"
    ));
    assert!(has_icon(
        "/assets/icons/jbotci-icon.svg",
        "any",
        "image/svg+xml",
        "any"
    ));
    assert!(has_icon(
        "/assets/icons/jbotci-icon-maskable-192.png",
        "192x192",
        "image/png",
        "maskable"
    ));
    assert!(has_icon(
        "/assets/icons/jbotci-icon-maskable-512.png",
        "512x512",
        "image/png",
        "maskable"
    ));
    assert_eq!(
        icons
            .iter()
            .filter(|icon| {
                icon.get("purpose").and_then(serde_json::Value::as_str) == Some("maskable")
            })
            .count(),
        2
    );
    assert!(!include_bytes!("../assets/icons/jbotci-icon-maskable-192.png").is_empty());
    assert!(!include_bytes!("../assets/icons/jbotci-icon-maskable-512.png").is_empty());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn git_commit_display_uses_math_monospace_hex() {
    assert_eq!(math_monospace_git_commit("f4a90c1"), "𝚏𝟺𝚊𝟿𝟶𝚌𝟷");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn topbar_carousel_routes_keep_all_primary_pages_in_display_order() {
    assert_eq!(topbar_carousel_routes(AppRoute::Cukta), TOPBAR_NAV_ROUTES);
    assert_eq!(topbar_carousel_routes(AppRoute::Vlacku), TOPBAR_NAV_ROUTES);
    assert_eq!(topbar_carousel_routes(AppRoute::Gimfihi), TOPBAR_NAV_ROUTES);
    assert_eq!(topbar_carousel_routes(AppRoute::Gentufa), TOPBAR_NAV_ROUTES);
    assert_eq!(
        topbar_carousel_routes(AppRoute::Settings),
        TOPBAR_NAV_ROUTES
    );
    assert_eq!(TOPBAR_NAV_ROUTES.last(), Some(&AppRoute::Gimfihi));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_sources_for_preset_populates_rows() {
    let sources = gimfihi_sources_for_preset(GimfihiPreset::Ilmen12);

    assert_eq!(sources.len(), 12);
    assert_eq!(sources[0].language, "cmn");
    assert_eq!(sources[11].language, "fra");
    assert!(sources.iter().all(|source| source.weight.is_none()));
    assert!(sources.iter().all(|source| source.word.is_empty()));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_language_edit_clears_preset_and_preserves_visible_weight() {
    let state = GimfihiWebState::default();

    let next = gimfihi_state_with_source_language(&state, 0, "jpn");

    assert_eq!(next.preset, None);
    assert_eq!(next.sources[0].language, "jpn");
    assert_eq!(next.sources[0].weight.as_deref(), Some("347"));
    assert_eq!(next.sources[1].weight.as_deref(), Some("196"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_selected_preset_preserves_matching_words() {
    let mut state = GimfihiWebState::default();
    state.sources[0].word = "uan".to_owned();
    state.sources[2].word = "ekspekt".to_owned();
    let memory = gimfihi_source_word_memory_from_state(&state);

    let next = gimfihi_state_for_selected_preset(&state, Some(GimfihiPreset::Ilmen6), &memory);

    assert_eq!(next.preset, Some(GimfihiPreset::Ilmen6));
    assert_eq!(next.sources[0].language, "eng");
    assert_eq!(next.sources[0].word, "ekspekt");
    assert_eq!(next.sources[1].language, "cmn");
    assert_eq!(next.sources[1].word, "uan");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_preset_switch_restores_words_for_returning_languages() {
    let mut state = GimfihiWebState::default();
    state.sources[4].word = "predpologa".to_owned();
    let mut memory = gimfihi_source_word_memory_from_state(&state);

    let ilmen6 = gimfihi_state_for_selected_preset(&state, Some(GimfihiPreset::Ilmen6), &memory);
    update_gimfihi_source_word_memory(&mut memory, &ilmen6);
    let restored =
        gimfihi_state_for_selected_preset(&ilmen6, Some(GimfihiPreset::Data1995), &memory);

    assert_eq!(restored.sources[4].language, "rus");
    assert_eq!(restored.sources[4].word, "predpologa");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_source_word_memory_tracks_explicit_language_edits() {
    let mut memory = BTreeMap::new();
    gimfihi_set_source_word_memory_entry(&mut memory, "rus", "predpologa");

    gimfihi_remove_source_word_memory_entry(&mut memory, "rus");
    gimfihi_set_source_word_memory_entry(&mut memory, "jpn", "dentou");

    assert!(!memory.contains_key("rus"));
    assert_eq!(memory.get("jpn").map(String::as_str), Some("dentou"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_source_word_memory_removes_empty_words() {
    let mut memory = BTreeMap::new();
    gimfihi_set_source_word_memory_entry(&mut memory, "rus", "predpologa");

    gimfihi_set_source_word_memory_entry(&mut memory, "rus", "   ");

    assert!(!memory.contains_key("rus"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_idle_result_state_is_current_and_not_loading() {
    let state = GimfihiWebState::default();

    let result = gimfihi_idle_result_state(&state);

    assert_eq!(result.state.as_ref(), Some(&state));
    assert!(!result.loading);
    assert!(result.result.output.is_none());
    assert!(!gimfihi_result_panel_is_loading(&state, &result));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_blank_committed_state_never_shows_loading() {
    let state = GimfihiWebState::default();
    let mut stale_result = GimfihiAsyncResultState::default();

    assert!(!gimfihi_result_panel_is_loading(&state, &stale_result));

    stale_result.loading = true;
    assert!(!gimfihi_result_panel_is_loading(&state, &stale_result));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_nonblank_committed_state_shows_loading_for_stale_result() {
    let mut state = GimfihiWebState::default();
    state.sources[0].word = "uan".to_owned();
    let stale_result = GimfihiAsyncResultState::default();

    assert!(gimfihi_result_panel_is_loading(&state, &stale_result));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_load_more_state_doubles_and_clamps_count() {
    let mut state = GimfihiWebState {
        count: 20,
        ..GimfihiWebState::default()
    };

    assert_eq!(gimfihi_load_more_state(&state).count, 40);

    state.count = GIMFIHI_MAX_COUNT;
    assert_eq!(gimfihi_load_more_state(&state).count, GIMFIHI_MAX_COUNT);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_result_summary_reports_candidates_and_shown_count() {
    let output = GimfihiOutput {
        resolved_sources: Vec::new(),
        candidate_count: 20_000,
        filtered_count: 12_222,
        winner: None,
        highlighted_word: None,
        candidates: vec![GimfihiCandidate {
            word: "traco".to_owned(),
            score: 0.0,
            source_scores: Vec::new(),
            collision: None,
            rafsi: None,
            highlighted: false,
        }],
    };

    assert_eq!(
        gimfihi_result_summary(&output),
        "12,222 candidates, 1 shown"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_initial_state_hydrates_from_route() {
    let mut state = GimfihiWebState::default();
    state.highlight = Some("nanpe".to_owned());
    let route = JbotciRoute::from_web_route(WebRoute::Gimfihi(state.clone()), false);

    assert_eq!(initial_gimfihi_state(&route), state);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_highlight_helper_preserves_row_selection_in_state() {
    let state = GimfihiWebState::default();

    let next = gimfihi_state_with_highlight(&state, "Nanpe");

    assert_eq!(next.highlight.as_deref(), Some("nanpe"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_generation_cache_key_ignores_highlight() {
    let mut left = GimfihiWebState::default();
    left.highlight = Some("traco".to_owned());
    let mut right = left.clone();
    right.highlight = Some("kanpe".to_owned());

    assert_eq!(
        gimfihi_generation_cache_key(&left),
        gimfihi_generation_cache_key(&right)
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gimfihi_result_cache_misses_highlight_outside_cached_rows() {
    let mut state = GimfihiWebState::default();
    state.highlight = Some("nanpe".to_owned());
    let output = GimfihiOutput {
        resolved_sources: Vec::new(),
        candidate_count: 2,
        filtered_count: 2,
        winner: Some("traco".to_owned()),
        highlighted_word: Some("traco".to_owned()),
        candidates: vec![GimfihiCandidate {
            word: "traco".to_owned(),
            score: 0.0,
            source_scores: Vec::new(),
            collision: None,
            rafsi: None,
            highlighted: true,
        }],
    };
    let cached = GimfihiAsyncResultState {
        state: Some(state.clone()),
        result: GimfihiWebResult {
            state: state.clone(),
            output: Some(output),
            preset_options: Vec::new(),
            language_suggestions: Vec::new(),
            errors: Vec::new(),
        },
        meta: None,
        loading: false,
        error: None,
    };

    assert!(gimfihi_cached_result_for_state("", &state, cached).is_none());
}

#[requires(true)]
#[ensures(true)]
fn page_find_entry_texts(entries: &[PageFindTextEntry]) -> Vec<String> {
    entries.iter().map(|entry| entry.text.clone()).collect()
}

#[test]
#[requires(true)]
#[ensures(true)]
fn page_find_matching_handles_empty_overlap_and_unicode_ranges() {
    assert!(page_find_match_ranges("banana", "").is_empty());
    assert!(page_find_match_ranges("", "ana").is_empty());

    let overlapping = page_find_match_ranges("banana", "ana");
    assert_eq!(overlapping.len(), 1);
    assert_eq!(overlapping[0].byte_start, 1);
    assert_eq!(overlapping[0].byte_end, 4);

    let unicode_text = "İS";
    let unicode = page_find_match_ranges(unicode_text, "i\u{307}s");
    assert_eq!(unicode.len(), 1);
    assert!(unicode_text.is_char_boundary(unicode[0].byte_start));
    assert!(unicode_text.is_char_boundary(unicode[0].byte_end));
    assert_eq!(
        &unicode_text[unicode[0].byte_start..unicode[0].byte_end],
        unicode_text
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn page_find_text_keys_are_content_stable() {
    let mut entries = Vec::new();
    push_page_find_entry(&mut entries, "alpha");
    push_page_find_entry(&mut entries, "beta");
    push_page_find_entry(&mut entries, "alpha");

    let first_alpha = entries[0].key;
    let beta = entries[1].key;
    let second_alpha = entries[2].key;
    assert_eq!(first_alpha.content_hash, second_alpha.content_hash);
    assert_ne!(first_alpha.occurrence, second_alpha.occurrence);

    let mut shifted = Vec::new();
    push_page_find_entry(&mut shifted, "inserted");
    push_page_find_entry(&mut shifted, "alpha");
    push_page_find_entry(&mut shifted, "beta");
    push_page_find_entry(&mut shifted, "alpha");

    assert_eq!(shifted[1].key, first_alpha);
    assert_eq!(shifted[2].key, beta);
    assert_eq!(shifted[3].key, second_alpha);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn page_find_route_state_remembers_queries_and_resets_active_selection() {
    let mut state = PageFindState::default();

    set_page_find_query(
        &mut state,
        AppRoute::Cukta,
        "broda".to_owned(),
        PageFindRouteQueryUpdate::Replace,
    );
    update_page_find_active(&mut state, AppRoute::Cukta, PageFindDirection::Next, 3);

    set_page_find_query(
        &mut state,
        AppRoute::Vlacku,
        "valsi".to_owned(),
        PageFindRouteQueryUpdate::Replace,
    );

    assert_eq!(state.cukta.query, "broda");
    assert_eq!(state.cukta.active_index, Some(0));
    assert_eq!(state.vlacku.query, "valsi");
    assert_eq!(state.vlacku.active_index, None);

    set_page_find_query(
        &mut state,
        AppRoute::Cukta,
        "brode".to_owned(),
        PageFindRouteQueryUpdate::Replace,
    );
    assert_eq!(state.cukta.query, "brode");
    assert_eq!(state.cukta.active_index, None);

    state.cukta = state.cukta.clone().with_data(data! {
        active_index: Some(2),
        result_signature: 10,
    });
    sync_page_find_result_signature(&mut state, AppRoute::Cukta, 11, 3);
    assert_eq!(state.cukta.active_index, None);

    state.cukta = state.cukta.clone().with_data(data! {
        active_index: Some(5),
        result_signature: 11,
    });
    sync_page_find_result_signature(&mut state, AppRoute::Cukta, 11, 3);
    assert_eq!(state.cukta.active_index, None);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn topbar_layout_resolver_prefers_full_nav_then_compact_settings_then_carousel() {
    let layout =
        topbar_layout_from_probe_fits(|selector| selector == ".app-topbar-fit-probe-both-full");
    assert_eq!(layout.settings, TopbarSettingsLayout::BothInline);
    assert_eq!(layout.nav, TopbarNavLayout::Full);

    let layout =
        topbar_layout_from_probe_fits(|selector| selector == ".app-topbar-fit-probe-theme-full");
    assert_eq!(layout.settings, TopbarSettingsLayout::ThemeInline);
    assert_eq!(layout.nav, TopbarNavLayout::Full);

    let layout =
        topbar_layout_from_probe_fits(|selector| selector == ".app-topbar-fit-probe-both-carousel");
    assert_eq!(layout.settings, TopbarSettingsLayout::BothInline);
    assert_eq!(layout.nav, TopbarNavLayout::Carousel);

    let layout = topbar_layout_from_probe_fits(|_selector| false);
    assert_eq!(layout.settings, TopbarSettingsLayout::NoneInline);
    assert_eq!(layout.nav, TopbarNavLayout::Carousel);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn page_find_collects_cukta_content_but_not_toc() {
    let page = CuktaPageData {
        toc: vec![CuktaTocNode {
            node_id: "toc-hidden".to_owned(),
            number_label: None,
            label: "TOC hidden label".to_owned(),
            href: "/cukta#toc-hidden".to_owned(),
            active: false,
            section_id: None,
            current: false,
            children: Vec::new(),
        }],
        current_section_id: None,
        page_kind: CuktaPageKind::Section {
            section_heading: "Section heading".to_owned(),
            section_parse_href: None,
            chapter_title: None,
            previous_section: None,
            next_section: None,
            chapter_prelude_blocks: Vec::new(),
            blocks: vec![CllBlock::Paragraph {
                anchor_id: None,
                role: None,
                inlines: vec![CllInline::Text("Visible CLL body".to_owned())],
                text: "Visible CLL body".to_owned(),
            }],
        },
    };
    let mut entries = Vec::new();

    collect_cukta_page_find_entries(&mut entries, &page, GentufaScript::Latin);
    let texts = page_find_entry_texts(&entries);

    assert!(texts.contains(&"Section heading".to_owned()));
    assert!(texts.contains(&"Visible CLL body".to_owned()));
    assert!(!texts.contains(&"TOC hidden label".to_owned()));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn page_find_collects_vlacku_card_text_without_controls() {
    let result = VlackuWebResult {
        state: VlackuWebState::default(),
        cards: vec![VlackuWebCard {
            rank: 1,
            word: "broda".to_owned(),
            display_word: "broda".to_owned(),
            word_type: "gismu".to_owned(),
            word_type_key: "gismu".to_owned(),
            selmaho: Some("BRIVLA".to_owned()),
            author: Some(new!(VlackuWebAuthor {
                username: "alice".to_owned(),
                realname: Some("Alice A.".to_owned()),
            })),
            ipa: Some("bɾoda".to_owned()),
            similarity: Some(0.42),
            votes: VlackuVoteDisplay::Known("+7".to_owned()),
            rafsi: vec!["bro".to_owned()],
            glosses: vec!["predicate".to_owned()],
            definition_source: "definition source".to_owned(),
            definition: vec![
                new!(VlackuInline::Text("definition body ".to_owned())),
                new!(VlackuInline::WordRef {
                    label: "klama".to_owned(),
                    href: "/vlacku?mode=word&q=klama".to_owned(),
                    can_add_to_jvozba: true,
                }),
            ],
            notes: vec![new!(VlackuInline::Text("note body".to_owned()))],
            etymology: vec![new!(VlackuInline::Text("etymology body".to_owned()))],
            decomposition: vec![VlackuCompositionPiece {
                kind: VlackuCompositionPieceKind::Rafsi,
                surface: "bro".to_owned(),
                display_surface: "bro".to_owned(),
                source: Some("broda".to_owned()),
                display_source: Some("broda".to_owned()),
                source_href: None,
                source_is_surface: false,
            }],
            can_add_to_jvozba: true,
        }],
        word_type_options: Vec::new(),
        dictionary_info: None,
        has_more: true,
        message: Some("semantic message".to_owned()),
        errors: vec!["visible error".to_owned()],
    };
    let mut entries = Vec::new();

    collect_vlacku_page_find_entries(&mut entries, &result, GentufaScript::Latin);
    let texts = page_find_entry_texts(&entries);

    for expected in [
        "semantic message",
        "visible error",
        "broda",
        "/bɾoda/",
        "bro",
        "by alice (Alice A.)",
        "gismu",
        "BRIVLA",
        "42%",
        "+7",
        "definition body ",
        "klama",
        "predicate",
        "note body",
        "etymology: ",
        "etymology body",
        "Load more",
    ] {
        assert!(texts.contains(&expected.to_owned()), "missing {expected}");
    }
    assert!(!texts.contains(&"Dictionary query".to_owned()));
    assert!(!texts.contains(&"jvozba".to_owned()));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn page_find_collects_gentufa_outputs_and_excludes_edge_labels() {
    let edge_marker = ReferenceMarker {
        role: ReferenceMarkerRole::Reference,
        kind: ReferenceMarkerKind::Reference,
        label: ReferenceLabel::new("edgeonly", None, None),
        source: None,
        tooltip: None,
    };
    let success = GentufaSuccess {
        ipa_text: "ipa-visible".to_owned(),
        surface_text: String::new(),
        brackets_text: "bracket visible".to_owned(),
        bracket_fragments: vec![GentufaBracketFragment::Text {
            text: "bracket visible".to_owned(),
            elided: false,
        }],
        blocks_layout: new!(GentufaBlocksLayout {
            blocks: vec![new!(GentufaBlock {
                block_id: "block-1".to_owned(),
                node_ids: vec![1],
                label: "block label".to_owned(),
                is_leaf: true,
                is_elided: false,
                token_kind: None,
                ref_markers: Vec::new(),
                span: None,
                node_types: Vec::new(),
                ancestors: Vec::new(),
                col: 0,
                col_span: 1,
                row: 0,
                row_span: 1,
                color: "#cccccc".to_owned(),
                parent_color: None,
                raw_text: "block label".to_owned(),
                display_text: "block label".to_owned(),
                transform: None,
                glosses: vec!["block gloss".to_owned()],
                definition: None,
                computed_gloss: None,
                tooltip: None,
            })],
            max_col: 1,
            max_row: 1,
        }),
        tree_rows: vec![GentufaTreeRow {
            node_id: 1,
            parent_id: None,
            depth: 0,
            label: "tree category".to_owned(),
            color: "#cccccc".to_owned(),
            guides: Vec::new(),
            has_children: false,
            cells: vec![GentufaCell {
                text: "tree token".to_owned(),
                is_word: true,
                quoted: false,
                tooltip: None,
                is_elided: false,
                transform: None,
            }],
            computed_gloss: None,
            ref_markers: vec![edge_marker],
            glosses: Vec::new(),
            definition: None,
            rafsi_breakdown: Vec::new(),
        }],
        diagnostics: Vec::new(),
        features: WebFeatureAvailability::default(),
    };
    let mut tree_entries = Vec::new();
    collect_gentufa_page_find_entries(
        &mut tree_entries,
        &GentufaWebResult::Success(success.clone()),
        None,
        GentufaWebViewMode::Tree,
        GentufaDisplayState {
            show_elided: false,
            show_glosses: true,
        },
        GentufaScript::Latin,
    );
    let tree_texts = page_find_entry_texts(&tree_entries);
    assert!(tree_texts.contains(&"bracket visible".to_owned()));
    assert!(tree_texts.contains(&"tree category".to_owned()));
    assert!(tree_texts.contains(&"tree token".to_owned()));
    assert!(!tree_texts.contains(&"edgeonly".to_owned()));
    assert!(!tree_texts.contains(&"edge-only-kind".to_owned()));
    assert!(!tree_texts.contains(&"ipa-visible".to_owned()));

    let mut block_entries = Vec::new();
    collect_gentufa_page_find_entries(
        &mut block_entries,
        &GentufaWebResult::Success(success.clone()),
        None,
        GentufaWebViewMode::Blocks,
        GentufaDisplayState {
            show_elided: false,
            show_glosses: true,
        },
        GentufaScript::Latin,
    );
    let block_texts = page_find_entry_texts(&block_entries);
    assert!(block_texts.contains(&"block label".to_owned()));
    assert!(block_texts.contains(&"block gloss".to_owned()));

    let mut ipa_entries = Vec::new();
    collect_gentufa_page_find_entries(
        &mut ipa_entries,
        &GentufaWebResult::Success(success),
        None,
        GentufaWebViewMode::Ipa,
        GentufaDisplayState {
            show_elided: false,
            show_glosses: true,
        },
        GentufaScript::Latin,
    );
    let ipa_texts = page_find_entry_texts(&ipa_entries);
    assert!(ipa_texts.contains(&"ipa-visible".to_owned()));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn page_find_collects_settings_static_text_without_editable_values() {
    let dialect_settings = DialectSettings {
        custom_dialects: vec![CustomDialect {
            name: "custom-visible".to_owned(),
            definition: "()".to_owned(),
            show_in_gentufa: true,
        }],
        hidden_builtin_gentufa_dialects: BTreeSet::new(),
    };
    let mut entries = Vec::new();

    collect_settings_page_find_entries(
        &mut entries,
        UserSettings {
            theme: ThemeMode::Day,
            script: GentufaScript::Latin,
            stress: StressMark::None,
            glides: GlideMark::None,
            error_context_depth: 2,
        },
        &dialect_settings,
        "custom-visible",
        &EmbeddingSettingsState::default(),
    );
    let texts = page_find_entry_texts(&entries);

    for expected in [
        "Settings",
        "Semantic search",
        "Embedding model",
        "Status",
        "Download",
        "Parsing",
        "Error context depth",
        "Output",
        "Stress",
        "none",
        "Glides",
        "Lojban dialects",
        "Builtins",
        "Custom",
        "custom-visible",
        "Name",
        "Show in gentufa",
        "Definition",
        "Definition is valid.",
    ] {
        assert!(texts.contains(&expected.to_owned()), "missing {expected}");
    }
    assert!(!texts.contains(&"()".to_owned()));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn async_activity_tracks_overlapping_tasks_by_id() {
    let mut activity = AsyncActivityState::default();

    let gentufa_id = activity.begin(AsyncTaskKind::Gentufa);
    let cukta_id = activity.begin(AsyncTaskKind::Cukta);

    assert_ne!(gentufa_id, cukta_id);
    assert!(activity.is_active());
    assert!(activity.has_kind(AsyncTaskKind::Gentufa));
    assert!(activity.has_kind(AsyncTaskKind::Cukta));

    assert!(activity.finish(gentufa_id));
    assert!(activity.is_active());
    assert!(!activity.has_kind(AsyncTaskKind::Gentufa));
    assert!(activity.has_kind(AsyncTaskKind::Cukta));

    assert!(activity.finish(cukta_id));
    assert!(!activity.is_active());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn async_activity_finish_is_idempotent_for_cleanup_paths() {
    let mut activity = AsyncActivityState::default();
    let task_id = activity.begin(AsyncTaskKind::Export);

    assert!(activity.finish(task_id));
    assert!(!activity.finish(task_id));
    assert!(!activity.finish(task_id + 1));
    assert!(!activity.is_active());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn embedding_settings_parse_native_progress_payload() {
    let json = serde_json::json!({
        "selectedModelKey": F2LLM_NATIVE_330M_MODEL_KEY,
        "effectiveModelKey": F2LLM_NATIVE_330M_MODEL_KEY,
        "status": "preparing",
        "detail": "Indexing dictionary.",
        "progress": {
            "kind": "index",
            "label": "Indexing dictionary",
            "loaded": 3,
            "total": 10,
            "percent": 30
        }
    });

    let state = embedding_settings_from_json(&json.to_string(), "fallback");

    assert_eq!(state.progress_kind.as_deref(), Some("index"));
    assert_eq!(state.progress_loaded, Some(3));
    assert_eq!(state.progress_total, Some(10));
    assert_eq!(state.progress_percent, Some(30));
    assert_eq!(
        embedding_progress_display_label(&state),
        "Indexing dictionary 3/10 rows (30%)"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn native_embedding_model_options_cover_f2llm_size_family() {
    let keys = NATIVE_EMBEDDING_MODEL_OPTIONS
        .iter()
        .map(|option| option.key)
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        vec![
            F2LLM_NATIVE_80M_MODEL_KEY,
            F2LLM_NATIVE_160M_MODEL_KEY,
            F2LLM_NATIVE_330M_MODEL_KEY,
            F2LLM_NATIVE_0_6B_MODEL_KEY,
        ]
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn browser_embedding_catalog_serializes_runtime_specs() {
    let catalog: serde_json::Value =
        serde_json::from_str(&browser_embedding_model_catalog_json()).unwrap();
    assert_eq!(catalog["defaultMobileModelKey"], F2LLM_80M_MODEL_KEY);
    assert_eq!(catalog["defaultDesktopModelKey"], F2LLM_330M_MODEL_KEY);
    assert_eq!(catalog["wasmFallbackModelKey"], F2LLM_80M_MODEL_KEY);
    let models = catalog["models"].as_object().unwrap();
    assert_eq!(models.len(), 4);
    let default_model = &models[F2LLM_330M_MODEL_KEY];
    assert_eq!(
        default_model["customRuntime"]["artifactBaseUrl"],
        "https://assets.jbotci.app/models/f2llm-v2-330m-webgpu/v1"
    );
    assert_eq!(default_model["dimensions"], 896);
    assert!(default_model["wasmRuntime"].is_null());
    let fallback_model = &models[F2LLM_80M_MODEL_KEY];
    assert_eq!(
        fallback_model["wasmRuntime"]["onnxUrl"],
        "https://assets.jbotci.app/models/f2llm-v2-80m-onnx-q4/v1/model_q4.onnx"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn embedding_progress_display_formats_byte_progress() {
    let state = EmbeddingSettingsState {
        progress_kind: Some("download".to_owned()),
        progress_label: Some("Downloading model".to_owned()),
        progress_loaded: Some(1024),
        progress_total: Some(2048),
        progress_percent: Some(50),
        ..EmbeddingSettingsState::default()
    };

    assert_eq!(
        embedding_progress_display_label(&state),
        "Downloading model 1024 B / 2048 B (50%)"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reference_hover_measurement_id_is_monotonic_and_saturating() {
    let mut state = ReferenceHoverState::default();
    assert_eq!(next_reference_hover_measurement_id(&state), 1);
    state.measurement_id = 41;
    assert_eq!(next_reference_hover_measurement_id(&state), 42);
    state.measurement_id = u64::MAX;
    assert_eq!(next_reference_hover_measurement_id(&state), u64::MAX);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reference_hover_pointer_moves_do_not_request_async_measurement() {
    assert!(!reference_hover_refresh_requires_measurement(
        ReferenceHoverRefreshReason::PointerMove,
        true
    ));
    assert!(reference_hover_refresh_requires_measurement(
        ReferenceHoverRefreshReason::ViewportShift,
        true
    ));
    assert!(reference_hover_refresh_requires_measurement(
        ReferenceHoverRefreshReason::PointerMove,
        false
    ));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reference_hover_keeps_overlay_during_same_target_async_measurement() {
    let hovered = HoveredReference {
        role: ReferenceMarkerRole::Reference,
        label: ReferenceLabel::new("b", Some(1), None),
    };
    let overlay = new!(ArrowOverlay {
        width: 100.0,
        height: 80.0,
        paths: vec!["M 1.00 2.00 L 3.00 4.00".to_owned()],
    });
    let state = ReferenceHoverState {
        hovered: Some(hovered.clone()),
        overlay: Some(overlay.clone()),
        measurement_id: 7,
    };
    assert_eq!(
        reference_overlay_for_measurement_request(&state, &hovered, &None, true),
        Some(overlay.clone())
    );

    let other_hovered = HoveredReference {
        role: ReferenceMarkerRole::Referent,
        label: hovered.label.clone(),
    };
    assert_eq!(
        reference_overlay_for_measurement_request(&state, &other_hovered, &None, true),
        None
    );

    let measured_overlay = Some(new!(ArrowOverlay {
        width: 120.0,
        height: 90.0,
        paths: vec!["M 5.00 6.00 L 7.00 8.00".to_owned()],
    }));
    assert_eq!(
        reference_overlay_for_measurement_request(&state, &hovered, &measured_overlay, true),
        measured_overlay
    );
    assert_eq!(
        reference_overlay_for_measurement_request(&state, &hovered, &None, false),
        None
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn diagnostic_location_uses_one_indexed_line_column() {
    let source = "coi\nmi broda";
    let diagnostic = test_diagnostic(
        source,
        DiagnosticSeverity::Error,
        "syntax.unexpected-cmavo",
        "unexpected cmavo",
        4,
        6,
        "expected selbri",
    );

    let location = diagnostic_label_location(source, diagnostic.primary_label());

    assert_eq!(location.line, 2);
    assert_eq!(location.column, 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn diagnostic_location_uses_character_offsets_for_unicode() {
    let source = "coi\nzo'é mi";
    let diagnostic = test_diagnostic(
        source,
        DiagnosticSeverity::Error,
        "morphology.invalid",
        "invalid morphology",
        7,
        8,
        "invalid character",
    );

    let location = diagnostic_label_location(source, diagnostic.primary_label());

    assert_eq!(location.line, 2);
    assert_eq!(location.column, 4);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn diagnostic_pane_title_counts_errors_and_warning_like_diagnostics() {
    let source = "coi";
    let diagnostics = vec![
        test_diagnostic(
            source,
            DiagnosticSeverity::Error,
            "syntax.unexpected-cmavo",
            "unexpected cmavo",
            0,
            1,
            "expected text",
        ),
        test_diagnostic(
            source,
            DiagnosticSeverity::Warning,
            "syntax.warning.experimental",
            "experimental syntax",
            1,
            2,
            "experimental",
        ),
        test_diagnostic(
            source,
            DiagnosticSeverity::Advice,
            "syntax.advice",
            "syntax advice",
            2,
            3,
            "advice",
        ),
    ];

    let title = diagnostic_pane_title(diagnostic_counts(&diagnostics, None));

    assert_eq!(title, "Diagnostics: 1 error, 2 warnings");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn stale_gentufa_input_disables_decorations() {
    let source = "coi";
    let diagnostic = test_diagnostic(
        source,
        DiagnosticSeverity::Error,
        "syntax.unexpected-cmavo",
        "unexpected cmavo",
        0,
        1,
        "expected text",
    );
    let request = GentufaWebRequest {
        text: source.to_owned(),
        options: GentufaWebOptions::default(),
    };
    let result = GentufaWebResult::Error(GentufaError {
        phase: None,
        message: "unexpected cmavo".to_owned(),
        diagnostics: vec![diagnostic],
    });

    assert_eq!(
        current_gentufa_input_diagnostics(source, &result, Some(&request)).len(),
        1
    );
    assert!(current_gentufa_input_diagnostics("coi mi", &result, Some(&request)).is_empty());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn active_overlay_context_prefix_extends_to_primary_span() {
    let source = "mi broda";
    let diagnostic = test_diagnostic(
        source,
        DiagnosticSeverity::Error,
        "syntax.unexpected-cmavo",
        "unexpected cmavo",
        3,
        8,
        "expected selbri",
    );
    let context_span = jbotci_diagnostics::source_span_from_char_offsets(None, source, 0, 2)
        .expect("test context span is valid");
    let mut labels = diagnostic.labels.clone();
    labels.push(DiagnosticLabel::new(
        context_span,
        "while parsing sumti".to_owned(),
        false,
    ));
    let diagnostic = diagnostic.with_data(data! { labels: labels });
    let diagnostics = vec![diagnostic];

    let fragments = diagnostic_overlay_fragments(source, &diagnostics, Some(0));
    let context_prefix = fragments
        .iter()
        .find(|fragment| fragment.text == "mi ")
        .expect("context prefix should include text up to the primary span");
    let primary = fragments
        .iter()
        .find(|fragment| fragment.text == "broda")
        .expect("primary span should be a separate fragment");

    assert!(has_css_class(
        &context_prefix.class_name,
        "is-active-context"
    ));
    assert!(has_css_class(
        &context_prefix.class_name,
        "is-active-context-start"
    ));
    assert!(!has_css_class(
        &context_prefix.class_name,
        "is-active-context-end"
    ));
    assert!(!has_css_class(
        &context_prefix.class_name,
        "is-active-primary"
    ));
    assert!(has_css_class(&primary.class_name, "is-active-primary"));
    assert!(has_css_class(
        &primary.class_name,
        "is-active-context-token"
    ));
    assert!(!has_css_class(
        &primary.class_name,
        "is-active-context-start"
    ));
    assert!(has_css_class(&primary.class_name, "is-active-context-end"));
    assert!(!has_css_class(&primary.class_name, "is-active-context"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn diagnostic_overlay_selection_offsets_are_utf16_offsets() {
    let source = "a 😀 broda";
    let diagnostic = test_diagnostic(
        source,
        DiagnosticSeverity::Error,
        "syntax.unexpected-cmavo",
        "unexpected cmavo",
        4,
        9,
        "expected selbri",
    );
    let context_span = jbotci_diagnostics::source_span_from_char_offsets(None, source, 0, 1)
        .expect("test context span is valid");
    let mut labels = diagnostic.labels.clone();
    labels.push(DiagnosticLabel::new(
        context_span,
        "while parsing sumti".to_owned(),
        false,
    ));
    let diagnostic = diagnostic.with_data(data! { labels: labels });
    let diagnostics = vec![diagnostic];

    let fragments = diagnostic_overlay_fragments(source, &diagnostics, Some(0));
    let primary = fragments
        .iter()
        .find(|fragment| fragment.text == "broda")
        .expect("primary span should be present");

    assert_eq!(
        primary.selection_start,
        "a 😀 ".encode_utf16().count() as u32
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn styled_diagnostic_notes_include_detailed_needs_one_of() {
    let source = "coi";
    let diagnostic = test_diagnostic(
        source,
        DiagnosticSeverity::Error,
        "syntax.unexpected-cmavo",
        "unexpected cmavo",
        0,
        1,
        "expected text",
    )
    .with_styled_notes(vec![DiagnosticStyledNote::new(
        jbotci_diagnostics::DiagnosticNoteMode::Detailed,
        vec![
            DiagnosticTextSegment::new(DiagnosticTextRole::Keyword, "needs one of".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ":\n".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, "- ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Construct, "selbri".to_owned()),
        ],
    )]);

    let notes = diagnostic_styled_notes_for_web(&diagnostic);
    let note_text = notes[0]
        .segments
        .iter()
        .fold(String::new(), |mut text, segment| {
            text.push_str(&segment.text);
            text
        });

    assert_eq!(notes.len(), 1);
    assert!(note_text.contains("needs one of:"));
    assert!(note_text.contains("selbri"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn diagnostic_tooltip_uses_primary_detail_when_available() {
    let source = "coi";
    let diagnostic = test_diagnostic(
        source,
        DiagnosticSeverity::Error,
        "syntax.unexpected-cmavo",
        "unexpected cmavo",
        0,
        1,
        "expected free modifier, SE",
    );

    assert_eq!(
        diagnostic_tooltip_text(&diagnostic),
        "syntax.unexpected-cmavo: expected free modifier, SE"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn diagnostic_tooltip_prefers_structured_expected_headline() {
    let source = "li nu";
    let diagnostic = test_diagnostic(
        source,
        DiagnosticSeverity::Error,
        "syntax.unexpected-cmavo",
        "unexpected cmavo",
        3,
        5,
        "expected: free modifier or mex",
    )
    .with_styled_notes(vec![DiagnosticStyledNote::new(
        jbotci_diagnostics::DiagnosticNoteMode::Detailed,
        vec![
            DiagnosticTextSegment::new(DiagnosticTextRole::Keyword, "needs one of".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ":\n".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, "- ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Construct, "free modifier".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " (".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::WordCategory, "LERFU".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ")\n".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, "- ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Construct, "mex".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " (".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Selmaho, "PA".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ")".to_owned()),
        ],
    )]);

    assert_eq!(
        diagnostic_tooltip_text(&diagnostic),
        "syntax.unexpected-cmavo: expected: free modifier or mex"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn diagnostic_tooltip_uses_detailed_expectation_order_and_lerfu_name() {
    let source = "coi";
    let diagnostic = test_diagnostic(
        source,
        DiagnosticSeverity::Error,
        "syntax.unexpected-cmavo",
        "unexpected cmavo",
        0,
        1,
        "expected SE, free modifier, LERFU",
    )
    .with_styled_notes(vec![DiagnosticStyledNote::new(
        jbotci_diagnostics::DiagnosticNoteMode::Detailed,
        vec![
            DiagnosticTextSegment::new(DiagnosticTextRole::Keyword, "needs one of".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ":\n".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, "- ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Construct, "free modifier".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " (".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::WordCategory, "LERFU".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " or ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Selmaho, "COI".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ")\n".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, "- ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::WordCategory, "BRIVLA".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " or ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Selmaho, "SE".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " [".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Keyword, "continues".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Construct, "sumti".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, "]".to_owned()),
        ],
    )]);

    assert_eq!(
        diagnostic_tooltip_text(&diagnostic),
        "syntax.unexpected-cmavo: expected free modifier (LERFU or COI), BRIVLA or SE [continues sumti]"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn web_diagnostics_hide_redundant_expected_summary_notes() {
    let source = "coi";
    let diagnostic = test_diagnostic(
        source,
        DiagnosticSeverity::Error,
        "syntax.unexpected-cmavo",
        "unexpected cmavo",
        0,
        1,
        "expected text",
    )
    .with_data(data! {
        notes: vec![
            "expected one of: BRIVLA, SE".to_owned(),
            "another note".to_owned(),
        ],
    })
    .with_styled_notes(vec![
        DiagnosticStyledNote::new(
            jbotci_diagnostics::DiagnosticNoteMode::Summary,
            vec![
                DiagnosticTextSegment::new(
                    DiagnosticTextRole::Keyword,
                    "expected one of".to_owned(),
                ),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ": ".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Selmaho, "SE".to_owned()),
            ],
        ),
        DiagnosticStyledNote::new(
            jbotci_diagnostics::DiagnosticNoteMode::Detailed,
            vec![
                DiagnosticTextSegment::new(DiagnosticTextRole::Keyword, "needs one of".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ":\n".to_owned()),
                DiagnosticTextSegment::new(DiagnosticTextRole::Construct, "sumti".to_owned()),
            ],
        ),
    ]);

    let plain_notes = diagnostic_plain_note_segments_for_web(&diagnostic);
    let styled_notes = diagnostic_styled_notes_for_web(&diagnostic);

    assert_eq!(plain_notes.len(), 1);
    assert_eq!(diagnostic_text_parts_text(&plain_notes[0]), "another note");
    assert_eq!(styled_notes.len(), 1);
    assert_eq!(
        diagnostic_styled_note_text(styled_notes[0]),
        "needs one of:\nsumti"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn diagnostic_model_segments_style_expectation_terms() {
    let segments = jbotci_diagnostics::diagnostic_text_segments(
        "expected forethought selbri connection, linked arguments, FIhO modal, VUhU operator, statement, SE, LERFU, fe'e",
    );
    let parts = diagnostic_text_segment_render_parts(&segments);

    assert!(
        parts
            .iter()
            .any(|part| { part.role == DiagnosticTextRole::Keyword && part.text == "expected" })
    );
    assert!(parts.iter().any(|part| {
        part.role == DiagnosticTextRole::Construct && part.text == "forethought selbri connection"
    }));
    assert!(parts.iter().any(|part| {
        part.role == DiagnosticTextRole::Construct && part.text == "linked arguments"
    }));
    assert!(
        parts.iter().any(|part| {
            part.role == DiagnosticTextRole::Construct && part.text == "FIhO modal"
        })
    );
    assert!(parts.iter().any(|part| {
        part.role == DiagnosticTextRole::Construct && part.text == "VUhU operator"
    }));
    assert!(
        parts
            .iter()
            .any(|part| { part.role == DiagnosticTextRole::Construct && part.text == "statement" })
    );
    assert!(
        parts
            .iter()
            .any(|part| { part.role == DiagnosticTextRole::Selmaho && part.text == "SE" })
    );
    assert!(
        parts
            .iter()
            .any(|part| { part.role == DiagnosticTextRole::WordCategory && part.text == "LERFU" })
    );
    assert!(
        parts
            .iter()
            .any(|part| { part.role == DiagnosticTextRole::SpecificWord && part.text == "fe'e" })
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn selected_script_renders_visible_lojban_text_only() {
    assert_eq!(
        display_lojban_text(GentufaScript::Cyrillic, "mi klama le zarci"),
        "ми клама ле зарши"
    );
    assert_eq!(display_lojban_text(GentufaScript::Cyrillic, "coi"), "шой");
    assert_eq!(
        display_lojban_text(GentufaScript::Zbalermorna, "coi"),
        "\u{ed86}\u{eda8}"
    );
    assert_eq!(
        display_lojban_text(GentufaScript::Cyrillic, "hello!"),
        "hello!"
    );
    assert_eq!(
        display_lojban_text_if(GentufaScript::Cyrillic, "mi klama", false),
        "mi klama"
    );
    assert_eq!(
        cll_display_text_for_kind(GentufaScript::Cyrillic, "jbo", "mi klama"),
        "ми клама"
    );
    assert_eq!(
        cll_display_text_for_kind(GentufaScript::Cyrillic, "natlang", "I go"),
        "I go"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn dictionary_tooltip_position_keeps_normal_above_host_placement() {
    let position = platform::place_tooltip(
        platform::Rect {
            left: 240.0,
            top: 300.0,
            width: 20.0,
            height: 20.0,
        },
        platform::Size {
            width: 160.0,
            height: 120.0,
        },
        platform::Viewport {
            top: 40.0,
            width: 640.0,
            height: 480.0,
        },
        DICTIONARY_TOOLTIP_VIEWPORT_MARGIN_PX,
        DICTIONARY_TOOLTIP_HOST_GAP_PX,
    );

    assert_eq!(position.top, 172.0);
    assert_eq!(position.left, 170.0);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn dictionary_tooltip_position_clamps_oversized_stack_below_visible_top() {
    let position = platform::place_tooltip(
        platform::Rect {
            left: 240.0,
            top: 300.0,
            width: 20.0,
            height: 20.0,
        },
        platform::Size {
            width: 160.0,
            height: 460.0,
        },
        platform::Viewport {
            top: 56.0,
            width: 640.0,
            height: 480.0,
        },
        DICTIONARY_TOOLTIP_VIEWPORT_MARGIN_PX,
        DICTIONARY_TOOLTIP_HOST_GAP_PX,
    );

    assert_eq!(position.top, 64.0);
    assert_eq!(position.left, 170.0);
}

#[requires(!selector.is_empty())]
#[ensures(!ret.is_empty())]
fn css_rule<'a>(css: &'a str, selector: &str) -> &'a str {
    let selector_start = css.find(selector).expect("CSS selector");
    let rule_tail = &css[selector_start..];
    let rule_end = rule_tail.find('}').expect("CSS rule end");
    &rule_tail[..rule_end]
}

#[test]
#[requires(true)]
#[ensures(true)]
fn css_font_stacks_cover_ui_controls_and_lojban_fallbacks() {
    let css = include_str!("../assets/main.css");
    let root_rule = css_rule(css, ":root");
    assert!(
        root_rule.contains(
            "--ui-font: \"Noto Sans\", \"STIX Two Math\", \"Crisa\", Verdana, sans-serif;"
        )
    );
    assert!(root_rule.contains(
        "--lojban-font: \"Crisa\", \"Noto Sans\", \"STIX Two Math\", Verdana, sans-serif;"
    ));
    assert!(root_rule.contains(
        "--math-font: \"STIX Two Text\", \"STIX Two Math\", \"Noto Sans\", \"Crisa\", math, serif;"
    ));
    assert!(root_rule.contains(
        "--math-symbol-font: \"STIX Two Math\", \"Noto Sans\", \"Crisa\", Verdana, sans-serif;"
    ));
    assert!(
        root_rule.contains("--code-font: \"Noto Sans\", \"STIX Two Math\", \"Crisa\", monospace;")
    );
    assert!(root_rule.contains("font-family: var(--ui-font);"));
    let form_controls_rule = css_rule(css, "button,");
    assert!(form_controls_rule.contains("input,"));
    assert!(form_controls_rule.contains("textarea"));
    assert!(form_controls_rule.contains("font: inherit;"));

    let blocks_rule = css_rule(css, ".spa-shell.app-page");
    assert!(blocks_rule.contains("--blocks-font: var(--ui-font);"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn zbalermorna_linked_lojban_css_uses_lojban_font() {
    let css = include_str!("../assets/main.css");
    let selectors = [
        ".app-page.orthography-zbalermorna .parse-page .brackets-output",
        ".app-page.orthography-zbalermorna .parse-page .diagnostic-text-specific-word",
        ".app-page.orthography-zbalermorna .dictionary-page .dictionary-word-link",
        ".app-page.orthography-zbalermorna .reference-resolution-tooltip .reference-row-target",
        ".app-page.orthography-zbalermorna .rich-dictionary-tooltip .tooltip-inline-link",
        ".app-page.orthography-zbalermorna .cll-page .spa-cll-link-dictionary",
        ".app-page.orthography-zbalermorna .cll-page .spa-cll-link-parse",
        ".app-page.orthography-zbalermorna .cll-page .spa-cll-jbophrase",
    ];

    for selector in selectors {
        let rule = css_rule(css, selector);
        assert!(
            rule.contains("font-family: var(--lojban-font);"),
            "{selector}"
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn zbalermorna_block_native_titles_are_suppressed() {
    let block = test_gentufa_block(0, 1, &[]);

    assert_eq!(block_native_title(&block, GentufaScript::Latin), "test");
    assert_eq!(block_native_title(&block, GentufaScript::Cyrillic), "test");
    assert_eq!(block_native_title(&block, GentufaScript::Zbalermorna), "");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn diagnostic_specific_words_render_with_selected_script() {
    let word = diagnostic_test_part(DiagnosticTextRole::SpecificWord, "fe'e");
    let selmaho = diagnostic_test_part(DiagnosticTextRole::Selmaho, "FAhA");

    let rendered_word = diagnostic_display_text_part_for_script(&word, GentufaScript::Cyrillic);

    assert_ne!(rendered_word, "fe'e");
    assert!(rendered_word.contains('ф'));
    assert_eq!(
        diagnostic_display_text_part_for_script(&selmaho, GentufaScript::Cyrillic),
        "FAhA"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cll_link_kinds_identify_lojban_link_text() {
    assert!(cll_link_text_is_lojban(CllLinkKind::Dictionary));
    assert!(cll_link_text_is_lojban(CllLinkKind::Rafsi));
    assert!(cll_link_text_is_lojban(CllLinkKind::Parse));
    assert!(!cll_link_text_is_lojban(CllLinkKind::Section));
    assert!(!cll_link_text_is_lojban(CllLinkKind::External));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn diagnostic_input_tooltip_card_uses_opaque_background_variable() {
    let css = include_str!("../assets/main.css");
    let selector = ".parse-page .gentufa-diagnostic-input-tooltip .gentufa-diagnostic-card";
    let selector_start = css.find(selector).expect("tooltip card selector");
    let rule_tail = &css[selector_start..];
    let rule_end = rule_tail.find('}').expect("tooltip card rule end");
    let rule = &rule_tail[..rule_end];

    assert!(css.contains("--diagnostic-tooltip-card-bg: var(--app-surface-0);"));
    assert!(css.contains("--diagnostic-tooltip-card-bg: var(--app-surface-2);"));
    assert!(rule.contains("background: var(--diagnostic-tooltip-card-bg);"));
    let background_line = rule
        .lines()
        .find(|line| line.trim_start().starts_with("background:"))
        .expect("tooltip card background declaration");
    assert!(!background_line.contains("transparent"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn diagnostic_token_links_follow_cukta_and_vlacku_conventions() {
    let word = diagnostic_test_part(DiagnosticTextRole::SpecificWord, "fe'e");
    let selmaho = diagnostic_test_part(DiagnosticTextRole::Selmaho, "BAI");
    let category = diagnostic_test_part(DiagnosticTextRole::WordCategory, "BRIVLA");
    let construct = diagnostic_test_part(DiagnosticTextRole::Construct, "sumti");
    let statement = diagnostic_test_part(DiagnosticTextRole::Construct, "statement");

    assert_eq!(
        diagnostic_text_part_href(&word, "/jbotci").as_deref(),
        Some("/jbotci/vlacku/fe'e")
    );
    assert_eq!(
        diagnostic_text_part_href(&selmaho, "/jbotci").as_deref(),
        Some("/jbotci/cukta/section/section-index#BAI")
    );
    assert_eq!(
        diagnostic_text_part_href(&category, "/jbotci").as_deref(),
        Some("/jbotci/cukta/section/section-morphology-brivla")
    );
    assert_eq!(
        diagnostic_text_part_href(&construct, "/jbotci").as_deref(),
        Some("/jbotci/cukta/section/section-EBNF#ebnf-rule-sumti")
    );
    assert_eq!(
        diagnostic_text_part_href(&statement, "/jbotci").as_deref(),
        Some("/jbotci/cukta/section/section-EBNF#ebnf-rule-statement")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn diagnostic_construct_links_cover_new_syntax_constructs() {
    for (construct, rule) in [
        ("forethought bridi connection", "gek-sentence"),
        ("forethought sumti connection", "sumti-4"),
        ("forethought selbri connection", "selbri-6"),
        ("forethought mex", "mex"),
        ("termset", "termset"),
        ("place tag", "term"),
        ("quantifier", "quantifier"),
        ("linked arguments", "linkargs"),
        ("operator", "operator"),
        ("word-sequence quote", "sumti-6"),
        ("FIhO modal", "tense-modal"),
        ("VUhU operator", "mex-operator"),
    ] {
        let part = diagnostic_test_part(DiagnosticTextRole::Construct, construct);
        let expected_href = diagnostic_ebnf_rule_href("/jbotci", rule);

        assert_eq!(
            diagnostic_text_part_href(&part, "/jbotci").as_deref(),
            Some(expected_href.as_str()),
            "unexpected link for diagnostic construct {construct:?}",
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_search_debounce_is_longer_than_url_debounce() {
    assert_eq!(VLACKU_SEARCH_DEBOUNCE_MS, 900);
    assert_eq!(CUKTA_SEARCH_DEBOUNCE_MS, VLACKU_SEARCH_DEBOUNCE_MS);
    assert!(VLACKU_SEARCH_DEBOUNCE_MS > VLACKU_URL_DEBOUNCE_MS);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pending_local_route_writes_consume_exact_route_once() {
    let route = parse_test_route("", "/vlacku/klama");
    let mut pending = PendingLocalRouteWrites::default();

    pending.record(&route);

    assert!(pending.consume(&route));
    assert!(!pending.consume(&route));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pending_local_route_writes_do_not_consume_nonmatching_routes() {
    let route = parse_test_route("", "/vlacku/klama");
    let other = parse_test_route("", "/vlacku/ciska");
    let mut pending = PendingLocalRouteWrites::default();

    pending.record(&route);

    assert!(!pending.consume(&other));
    assert!(pending.consume(&route));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pending_local_route_writes_consume_duplicate_targets_together() {
    let route = parse_test_route("", "/gentufa?text=coi");
    let mut pending = PendingLocalRouteWrites::default();

    pending.record(&route);
    pending.record(&route);

    assert!(pending.consume(&route));
    assert!(!pending.consume(&route));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn pending_local_gentufa_writes_match_router_normalized_routes() {
    let target = JbotciRoute::from_web_route(
        WebRoute::Gentufa(GentufaWebState {
            text: " coi ".to_owned(),
            dialect: Some(" (cbm) ".to_owned()),
            view_mode: GentufaWebViewMode::Blocks,
            show_elided: false,
            show_glosses: false,
        }),
        true,
    );
    let reported = parse_test_route("", "/gentufa?text=coi&dialect=%28cbm%29");
    let mut pending = PendingLocalRouteWrites::default();

    pending.record(&target);

    assert!(pending.consume(&reported));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn document_title_uses_route_default_meta() {
    let route = parse_test_route("", "/settings");
    let meta = route_document_meta("", &route);

    assert_eq!(document_title_from_meta(&meta), "Settings");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn document_title_uses_result_meta_when_available() {
    let meta = new!(PageMeta {
        title: "coi - jbotci gentufa".to_owned(),
        description: "Gentufa parse result.".to_owned(),
        canonical_url: "/gentufa?text=coi".to_owned(),
        image: None,
    });

    assert_eq!(document_title_from_meta(&meta), "coi - jbotci gentufa");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_url_target_uses_committed_parse_state() {
    let draft_text = "mi klama";
    let committed_state = gentufa_state_from_parts(
        "coi",
        "",
        GentufaWebViewMode::Blocks,
        GentufaDisplayState {
            show_elided: false,
            show_glosses: false,
        },
        true,
    );

    let target = gentufa_route_for_committed_state(&committed_state, true);

    assert_eq!(target.to_string(), "/gentufa?text=coi");
    assert!(!target.to_string().contains(draft_text));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_url_sync_requires_gentufa_browser_location() {
    let gentufa_route = parse_test_route("", "/gentufa?text=coi");
    let vlacku_route = parse_test_route("", "/vlacku/klama");
    let cukta_route = parse_test_route("", "/cukta/search?q=klama");

    assert!(gentufa_url_sync_allowed(AppRoute::Gentufa, &gentufa_route));
    assert!(!gentufa_url_sync_allowed(AppRoute::Gentufa, &vlacku_route));
    assert!(!gentufa_url_sync_allowed(AppRoute::Gentufa, &cukta_route));
    assert!(!gentufa_url_sync_allowed(AppRoute::Vlacku, &gentufa_route));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_parse_intent_pushes_changed_route() {
    let current = parse_test_route("", "/gentufa?text=coi");
    let target_state = gentufa_state_from_parts(
        "mi klama",
        "",
        GentufaWebViewMode::Blocks,
        GentufaDisplayState {
            show_elided: false,
            show_glosses: false,
        },
        true,
    );
    let target = gentufa_route_for_committed_state(&target_state, true);

    assert_eq!(
        gentufa_url_history_action(&current, &target, GentufaUrlWriteIntent::PushParse),
        GentufaUrlHistoryAction::PushParse
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_display_changes_replace_current_route() {
    let current = parse_test_route("", "/gentufa?text=coi");
    let target_state = gentufa_state_from_parts(
        "coi",
        "",
        GentufaWebViewMode::Tree,
        GentufaDisplayState {
            show_elided: false,
            show_glosses: false,
        },
        true,
    );
    let target = gentufa_route_for_committed_state(&target_state, true);

    assert_eq!(
        gentufa_url_history_action(&current, &target, GentufaUrlWriteIntent::ReplaceCurrent),
        GentufaUrlHistoryAction::ReplaceCurrent
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_matching_route_has_no_url_write() {
    let current = parse_test_route("", "/gentufa?text=coi&view=tree");
    let target_state = gentufa_state_from_parts(
        "coi",
        "",
        GentufaWebViewMode::Tree,
        GentufaDisplayState {
            show_elided: false,
            show_glosses: false,
        },
        true,
    );
    let target = gentufa_route_for_committed_state(&target_state, true);

    assert_eq!(
        gentufa_url_history_action(&current, &target, GentufaUrlWriteIntent::PushParse),
        GentufaUrlHistoryAction::NoWrite
    );
    assert_eq!(
        gentufa_url_history_action(&current, &target, GentufaUrlWriteIntent::ReplaceCurrent),
        GentufaUrlHistoryAction::NoWrite
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_noop_sync_clears_pending_parse_intent() {
    assert_eq!(
        gentufa_url_intent_after_sync_action(
            GentufaUrlWriteIntent::PushParse,
            GentufaUrlHistoryAction::NoWrite,
        ),
        GentufaUrlWriteIntent::ReplaceCurrent
    );
    assert_eq!(
        gentufa_url_intent_after_sync_action(
            GentufaUrlWriteIntent::ReplaceCurrent,
            GentufaUrlHistoryAction::NoWrite,
        ),
        GentufaUrlWriteIntent::ReplaceCurrent
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_changed_parse_intent_survives_until_route_matches() {
    assert_eq!(
        gentufa_url_intent_after_sync_action(
            GentufaUrlWriteIntent::PushParse,
            GentufaUrlHistoryAction::PushParse,
        ),
        GentufaUrlWriteIntent::PushParse
    );
    assert_eq!(
        gentufa_url_intent_after_sync_action(
            GentufaUrlWriteIntent::ReplaceCurrent,
            GentufaUrlHistoryAction::ReplaceCurrent,
        ),
        GentufaUrlWriteIntent::ReplaceCurrent
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn local_route_writes_still_update_active_page_selection() {
    let route = parse_test_route("", "/gentufa?text=coi");

    let action = route_location_sync_action(&route, true);

    assert_eq!(action.app_route, AppRoute::Gentufa);
    assert!(!action.hydrate_route_bound_state);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn same_app_route_does_not_need_signal_update() {
    assert!(!app_route_update_needed(
        AppRoute::Gentufa,
        AppRoute::Gentufa
    ));
    assert!(app_route_update_needed(AppRoute::Gentufa, AppRoute::Vlacku));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn browser_route_changes_update_page_selection_and_hydrate_state() {
    let route = parse_test_route("", "/vlacku?mode=smuni&q=nonsense");

    let action = route_location_sync_action(&route, false);

    assert_eq!(action.app_route, AppRoute::Vlacku);
    assert!(action.hydrate_route_bound_state);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_search_query_draft_resets_count_and_preserves_controls() {
    let state = CuktaWebSearchState {
        mode: CuktaWebMode::Word,
        query: "klama".to_owned(),
        count: 80,
        targets: vec![CuktaSearchTarget::Example],
    };

    let next = cukta_search_state_with_query(&state, "ciska");

    assert_eq!(next.mode, CuktaWebMode::Word);
    assert_eq!(next.query, "ciska");
    assert_eq!(next.count, CUKTA_WEB_DEFAULT_COUNT);
    assert_eq!(next.targets, vec![CuktaSearchTarget::Example]);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn output_settings_parse_cli_mark_names() {
    assert_eq!(parse_stress_mark("none"), Some(StressMark::None));
    assert_eq!(parse_stress_mark("acute"), Some(StressMark::Acute));
    assert_eq!(parse_stress_mark("caps"), Some(StressMark::Caps));
    assert_eq!(parse_stress_mark("uppercase"), None);
    assert_eq!(stress_mark_storage_value(StressMark::Caps), "caps");

    assert_eq!(parse_glide_mark("none"), Some(GlideMark::None));
    assert_eq!(parse_glide_mark("breve"), Some(GlideMark::Breve));
    assert_eq!(parse_glide_mark("acute"), None);
    assert_eq!(glide_mark_storage_value(GlideMark::Breve), "breve");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
#[requires(true)]
#[ensures(true)]
fn vlacku_jvozba_is_available_without_browser_width() {
    assert!(vlacku_jvozba_available());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_dictionary_tooltip_helpers_cover_inline_and_ebnf_links() {
    let inline_card = cll_dictionary_tooltip_for_link("", CllLinkKind::Dictionary, "klama")
        .expect("dictionary CLL links should have tooltips");
    assert_eq!(inline_card.display_word, "klama");

    let ebnf_card = cll_dictionary_tooltip_for_href("", "../vlacku/klama")
        .expect("EBNF vlacku links should have tooltips");
    assert_eq!(ebnf_card.display_word, "klama");

    assert!(cll_dictionary_tooltip_for_link("", CllLinkKind::Rafsi, "kla").is_some());
    assert!(cll_dictionary_tooltip_for_href("", "../vlacku?mode=rafsi&q=kla").is_some());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_ebnf_section_links_use_v1_routes() {
    let href = cll_ebnf_href("/jbotci", "section/section-index#BAI");

    assert_eq!(href, "/jbotci/cukta/section/section-index#BAI");
    assert_eq!(
        cukta_section_reference_from_href(&href),
        Some("section-index".to_owned())
    );
    assert_eq!(cukta_anchor_from_href(&href), Some("BAI".to_owned()));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_hash_scroll_target_requires_cukta_route_and_anchor() {
    assert_eq!(
        cukta_hash_scroll_target(
            "/cukta/section/section-index",
            "",
            Some("#KEhE"),
            AppRoute::Cukta,
        ),
        Some("/cukta/section/section-index#KEhE".to_owned())
    );
    assert_eq!(
        cukta_hash_scroll_target(
            "/jbotci/cukta/section/section-index",
            "?q=unused",
            Some("KEhE"),
            AppRoute::Cukta,
        ),
        Some("/jbotci/cukta/section/section-index?q=unused#KEhE".to_owned())
    );
    assert_eq!(
        cukta_hash_scroll_target("/gentufa", "", Some("#KEhE"), AppRoute::Gentufa),
        None
    );
    assert_eq!(
        cukta_hash_scroll_target(
            "/cukta/section/section-index",
            "",
            Some("#"),
            AppRoute::Cukta,
        ),
        None
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_navigation_scroll_distinguishes_history_topbar_and_fresh_links() {
    assert_eq!(
        cukta_pending_scroll_for_navigation(
            AppRoute::Cukta,
            "/cukta/section/section-index#NAI",
            true,
            false,
        ),
        Some(cukta_anchor_pending_scroll(
            "/cukta/section/section-index#NAI".to_owned()
        ))
    );
    assert_eq!(
        cukta_pending_scroll_for_navigation(
            AppRoute::Cukta,
            "/cukta/section/section-index",
            false,
            true,
        ),
        Some(cukta_stored_pending_scroll(
            "/cukta/section/section-index".to_owned()
        ))
    );
    assert_eq!(
        cukta_pending_scroll_for_navigation(
            AppRoute::Cukta,
            "/cukta/section/section-index",
            false,
            false,
        ),
        Some(cukta_top_pending_scroll())
    );
    assert_eq!(
        cukta_pending_scroll_for_navigation(AppRoute::Gentufa, "/gentufa", false, true),
        None
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_route_links_preserve_anchor_scroll_intent() {
    let anchor_route = parse_test_route("", "/cukta/section/section-index#KE");
    assert_eq!(
        cukta_pending_scroll_for_route_link("", &anchor_route),
        cukta_anchor_pending_scroll("/cukta/section/section-index#KE".to_owned())
    );

    let section_route = parse_test_route("", "/cukta/section/section-index");
    assert_eq!(
        cukta_pending_scroll_for_route_link("", &section_route),
        cukta_top_pending_scroll()
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn explicit_route_links_only_set_cukta_scroll_intent_for_cukta_routes() {
    let anchor_route = parse_test_route("", "/cukta/section/section-index#KE");
    assert_eq!(
        cukta_pending_scroll_for_explicit_route_link("", &anchor_route),
        Some(cukta_anchor_pending_scroll(
            "/cukta/section/section-index#KE".to_owned()
        ))
    );

    let section_route = parse_test_route("", "/cukta/section/section-index");
    assert_eq!(
        cukta_pending_scroll_for_explicit_route_link("", &section_route),
        Some(cukta_top_pending_scroll())
    );

    let prefixed_section_route =
        parse_test_route("/jbotci", "/jbotci/cukta/section/section-index#KE");
    assert_eq!(
        cukta_pending_scroll_for_explicit_route_link("/jbotci", &prefixed_section_route),
        Some(cukta_anchor_pending_scroll(
            "/jbotci/cukta/section/section-index#KE".to_owned()
        ))
    );

    let gentufa_route = parse_test_route("", "/gentufa");
    assert_eq!(
        cukta_pending_scroll_for_explicit_route_link("", &gentufa_route),
        None
    );

    let vlacku_route = parse_test_route("", "/vlacku/klama");
    assert_eq!(
        cukta_pending_scroll_for_explicit_route_link("", &vlacku_route),
        None
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_history_route_changes_restore_stored_scroll_even_with_hash() {
    let anchor_route = parse_test_route("", "/cukta/section/section-index#KE");
    assert_eq!(
        cukta_pending_scroll_for_route_change("", &anchor_route),
        Some(cukta_stored_pending_scroll(
            "/cukta/section/section-index#KE".to_owned()
        ))
    );

    let prefixed_anchor_route =
        parse_test_route("/jbotci", "/jbotci/cukta/section/section-index#KE");
    assert_eq!(
        cukta_pending_scroll_for_route_change("/jbotci", &prefixed_anchor_route),
        Some(cukta_stored_pending_scroll(
            "/jbotci/cukta/section/section-index#KE".to_owned()
        ))
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_scroll_waits_for_matching_rendered_page() {
    let state = CuktaWebState {
        view: CuktaWebView::Index,
    };
    let ready_page = CuktaAsyncPageState {
        state: Some(state.clone()),
        page: cukta_loading_page_data("Loaded CLL page."),
        meta: None,
        loading: false,
        error: None,
    };
    assert!(cukta_page_ready_for_scroll(&ready_page, &state));

    let mut loading_page = ready_page.clone();
    loading_page.loading = true;
    assert!(!cukta_page_ready_for_scroll(&loading_page, &state));

    let mut error_page = ready_page.clone();
    error_page.error = Some("failed".to_owned());
    assert!(!cukta_page_ready_for_scroll(&error_page, &state));

    let other_state = CuktaWebState {
        view: CuktaWebView::Search(CuktaWebSearchState::default()),
    };
    assert!(!cukta_page_ready_for_scroll(&ready_page, &other_state));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn block_bottom_row_uses_leaf_span_bottom() {
    let tall_leaf = test_gentufa_block(0, 3, &[ReferenceMarkerRole::Referent]);

    assert_eq!(block_bottom_row(&tall_leaf), 2);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reference_height_sizer_requires_incoming_reference() {
    let outgoing = test_gentufa_block(0, 1, &[ReferenceMarkerRole::Reference]);
    let incoming = test_gentufa_block(0, 1, &[ReferenceMarkerRole::Referent]);
    let plain = test_gentufa_block(0, 1, &[]);

    assert!(!block_needs_reference_height_sizer(&outgoing));
    assert!(block_needs_reference_height_sizer(&incoming));
    assert!(!block_needs_reference_height_sizer(&plain));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reference_marker_view_model_omits_native_title() {
    let hover_state = ReferenceHoverState::default();
    let plain = test_reference_marker(ReferenceMarkerRole::Reference, 0);
    let plain_view = reference_marker_view_model(&plain, &hover_state);
    assert_eq!(plain_view.native_title, None);
    assert!(!plain_view.has_tooltip);

    let mut rich = test_reference_marker(ReferenceMarkerRole::Reference, 0);
    rich.tooltip = Some(test_reference_tooltip());
    let rich_view = reference_marker_view_model(&rich, &hover_state);
    assert_eq!(rich_view.native_title, None);
    assert!(rich_view.has_tooltip);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reference_tooltip_host_class_opens_only_for_clicked_marker() {
    let marker = test_reference_marker(ReferenceMarkerRole::Reference, 0);
    assert_eq!(
        reference_tooltip_host_class(&marker, &None),
        "reference-tooltip-host"
    );

    let opened = Some(HoveredReference {
        role: marker.role,
        label: marker.label.clone(),
    });
    assert_eq!(
        reference_tooltip_host_class(&marker, &opened),
        "reference-tooltip-host is-open"
    );

    let other_role = Some(HoveredReference {
        role: ReferenceMarkerRole::Referent,
        label: marker.label.clone(),
    });
    assert_eq!(
        reference_tooltip_host_class(&marker, &other_role),
        "reference-tooltip-host"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reference_tooltip_row_view_model_separates_slot_and_target_text() {
    let row = new!(ReferenceTooltipRow {
        label: ReferenceLabel::new("k", None, Some(ReferenceSlotLabel::Numbered(1))),
        target_text: "lo mlatu be mi".to_owned(),
    });
    let view = reference_tooltip_row_view_model(&row);
    assert_eq!(view.slot_text.as_deref(), Some("𝟣"));
    assert_eq!(view.target_text, "lo mlatu be mi");

    let discourse_row = new!(ReferenceTooltipRow {
        label: ReferenceLabel::new("ko'a", Some(1), None),
        target_text: "mi".to_owned(),
    });
    let discourse_view = reference_tooltip_row_view_model(&discourse_row);
    assert_eq!(discourse_view.slot_text, None);
    assert_eq!(discourse_view.target_text, "mi");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn blocks_grid_row_template_uses_compact_rows() {
    assert_eq!(
        blocks_grid_row_template(3, true),
        "minmax(var(--blocks-compact-min-height), auto) minmax(var(--blocks-compact-min-height), auto) minmax(var(--blocks-compact-min-height), auto) auto"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reference_clearance_deficit_only_reports_needed_growth() {
    assert_eq!(reference_clearance_deficit(20.0, 40.0, 0.0), 0.0);
    assert_eq!(
        reference_clearance_deficit(20.0, 24.0, 0.0),
        BLOCK_REFERENCE_LABEL_GAP_PX - 4.0
    );
    assert_eq!(reference_clearance_deficit(20.0, 24.0, 4.0), 0.0);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn reference_containment_deficit_only_reports_block_overflow() {
    assert_eq!(reference_containment_deficit(20.0, 32.0, 0.0), 0.0);
    assert_eq!(
        reference_containment_deficit(36.0, 32.0, 0.0),
        4.0 + BLOCK_REFERENCE_CONTAINMENT_GAP_PX
    );
    assert_eq!(reference_containment_deficit(36.0, 32.0, 5.0), 0.0);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn horizontal_ranges_overlap_requires_shared_interior() {
    assert!(horizontal_ranges_overlap(0.0, 10.0, 5.0, 15.0));
    assert!(!horizontal_ranges_overlap(0.0, 10.0, 10.0, 15.0));
    assert!(!horizontal_ranges_overlap(0.0, 10.0, 11.0, 15.0));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_semantic_worker_limit_bounds_unfiltered_results() {
    let state = VlackuWebState {
        mode: VlackuWebMode::Meaning,
        query: "klama".to_owned(),
        count: 20,
        word_types: Vec::new(),
    };
    assert_eq!(vlacku_semantic_worker_limit(&state), 21);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_semantic_worker_limit_bounds_filtered_results() {
    let state = VlackuWebState {
        mode: VlackuWebMode::Meaning,
        query: "klama".to_owned(),
        count: 20,
        word_types: vec!["gismu".to_owned()],
    };
    assert_eq!(vlacku_semantic_worker_limit(&state), 21);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_semantic_worker_limit_clamps_unfiltered_results() {
    let state = VlackuWebState {
        mode: VlackuWebMode::Meaning,
        query: "klama".to_owned(),
        count: usize::MAX,
        word_types: Vec::new(),
    };
    assert_eq!(vlacku_semantic_worker_limit(&state), VLACKU_WEB_MAX_COUNT);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_load_more_state_only_expands_count() {
    let state = VlackuWebState {
        mode: VlackuWebMode::Rafsi,
        query: "kla".to_owned(),
        count: 20,
        word_types: vec!["gismu".to_owned()],
    };

    let next = vlacku_load_more_state(&state);

    assert_eq!(next.mode, state.mode);
    assert_eq!(next.query, state.query);
    assert_eq!(next.word_types, state.word_types);
    assert_eq!(next.count, 40);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_load_more_state_clamps_count() {
    let state = VlackuWebState {
        mode: VlackuWebMode::Word,
        query: "klama".to_owned(),
        count: VLACKU_WEB_MAX_COUNT,
        word_types: Vec::new(),
    };

    let next = vlacku_load_more_state(&state);

    assert_eq!(next.count, VLACKU_WEB_MAX_COUNT);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn stable_jvozba_pane_top_uses_anchor_at_unscrolled_position() {
    let top_at_page_top = platform::stable_jvozba_pane_top(Some(242.0), 0, 46.0, 34.0);
    let top_after_scroll = platform::stable_jvozba_pane_top(Some(-658.0), 900, 46.0, 34.0);

    assert_eq!(top_at_page_top, 242.0);
    assert_eq!(top_after_scroll, top_at_page_top);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn stable_jvozba_pane_top_uses_fallback_until_results_render() {
    let top = platform::stable_jvozba_pane_top(None, 900, 46.0, 34.0);

    assert_eq!(top, 46.0);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_semantic_result_is_pending_for_stale_or_loading_results() {
    let state = VlackuWebState {
        mode: VlackuWebMode::Meaning,
        query: "klama".to_owned(),
        count: 20,
        word_types: Vec::new(),
    };
    let semantic = VlackuSemanticResultState::default();

    assert!(vlacku_semantic_result_is_pending(&state, &semantic));

    let loading = VlackuSemanticResultState {
        state: Some(state.clone()),
        hits: Vec::new(),
        message: Some("Loading semantic search model.".to_owned()),
        loading: true,
    };
    assert!(vlacku_semantic_result_is_pending(&state, &loading));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_semantic_pending_page_preserves_existing_result() {
    let previous_state = VlackuWebState {
        mode: VlackuWebMode::Meaning,
        query: "klama".to_owned(),
        count: 20,
        word_types: Vec::new(),
    };
    let state = VlackuWebState {
        mode: VlackuWebMode::Meaning,
        query: "klama!".to_owned(),
        count: 20,
        word_types: Vec::new(),
    };
    let mut page = VlackuAsyncResultState {
        state: Some(previous_state.clone()),
        result: vlacku_loading_result(&previous_state, "Previous result remains visible."),
        meta: None,
        loading: false,
        error: None,
    };
    let semantic = VlackuSemanticResultState::default();

    let meta = apply_vlacku_semantic_pending_page(&mut page, "/jbotci", &state, &semantic);

    assert_eq!(page.state.as_ref(), Some(&state));
    assert!(page.loading);
    assert!(page.error.is_none());
    assert_eq!(
        page.result.message.as_deref(),
        Some("Previous result remains visible.")
    );
    assert_eq!(meta.title, "klama! - jbotci vlacku");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_semantic_pending_page_shows_explicit_loading_message() {
    let state = VlackuWebState {
        mode: VlackuWebMode::Meaning,
        query: "klama".to_owned(),
        count: 20,
        word_types: Vec::new(),
    };
    let mut page = VlackuAsyncResultState {
        state: Some(state.clone()),
        result: vlacku_loading_result(&state, "Previous result remains visible."),
        meta: None,
        loading: false,
        error: None,
    };
    let semantic = VlackuSemanticResultState {
        state: Some(state.clone()),
        hits: Vec::new(),
        message: Some("Loading semantic search model.".to_owned()),
        loading: true,
    };

    apply_vlacku_semantic_pending_page(&mut page, "/jbotci", &state, &semantic);

    assert_eq!(
        page.result.message.as_deref(),
        Some("Loading semantic search model.")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_semantic_result_ready_uses_compute_worker_path() {
    let state = VlackuWebState {
        mode: VlackuWebMode::Meaning,
        query: "klama".to_owned(),
        count: 20,
        word_types: Vec::new(),
    };
    let semantic = VlackuSemanticResultState {
        state: Some(state.clone()),
        hits: Vec::new(),
        message: None,
        loading: false,
    };

    assert!(!vlacku_semantic_result_is_pending(&state, &semantic));

    let request = vlacku_compute_request("/jbotci", &state, &semantic);
    assert!(matches!(
        request,
        WebComputeRequest::VlackuSemanticPage { loading: false, .. }
    ));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn top_level_routes_accept_trailing_slashes() {
    assert_eq!(
        parse_test_route("/jbotci", "/jbotci/cukta/").app_route(),
        AppRoute::Cukta
    );
    assert_eq!(
        parse_test_route("/jbotci", "/jbotci/vlacku/").app_route(),
        AppRoute::Vlacku
    );
    assert_eq!(
        parse_test_route("/jbotci", "/jbotci/gimfihi/").app_route(),
        AppRoute::Gimfihi
    );
    assert_eq!(
        parse_test_route("/jbotci", "/jbotci/settings/").app_route(),
        AppRoute::Settings
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn typed_routes_preserve_canonical_url_contract() {
    assert_eq!(parse_test_route("", "/").to_string(), "/vlacku");

    let gentufa = parse_test_route("/jbotci", "/jbotci/gentufa?text=coi&view=tree&glosses=true");
    assert_eq!(
        gentufa.to_string(),
        "/gentufa?text=coi&view=tree&glosses=true"
    );
    assert!(gentufa.gentufa_text_explicit);

    let settings = parse_test_route("", "/settings?johau=lojban");
    assert_eq!(settings.to_string(), "/settings?johau=lojban");
    assert_eq!(settings.settings_query, "johau=lojban");

    let cukta_search = parse_test_route("", "/cukta/search?q=klama&target=example&count=40");
    assert_eq!(
        cukta_search.to_string(),
        "/cukta/search?q=klama&count=40&target=example"
    );

    let cukta_section = parse_test_route("", "/cukta/section/chapter-abstractions#section-example");
    assert_eq!(
        cukta_section.to_string(),
        "/cukta/section/chapter-abstractions#section-example"
    );

    assert_eq!(parse_test_route("", "/vlacku").to_string(), "/vlacku");
    assert_eq!(
        parse_test_route("", "/vlacku/klama").to_string(),
        "/vlacku/klama"
    );
    assert_eq!(
        parse_test_route("/jbotci", "/vlacku/klama").to_string(),
        "/vlacku/klama"
    );
    assert_eq!(
        parse_test_route("/jbotci", "/jbotci/vlacku/klama").to_string(),
        "/vlacku/klama"
    );
    assert_eq!(
        parse_test_route("", "/vlacku/%2Fma.*%2F").to_string(),
        "/vlacku/%2Fma.%2A%2F"
    );

    let gimfihi = parse_test_route(
        "",
        "/gimfihi?preset=1995&source=cmn%3A%3A&source=hin%3A%3A&source=eng%3A%3A&source=spa%3A%3A&source=rus%3A%3A&source=ara%3A%3A&shape=ccvcv&shape=cvccv&letters=source&check-collisions=all&require-free-short-rafsi=false&count=20",
    );
    assert_eq!(gimfihi.app_route(), AppRoute::Gimfihi);
    assert!(gimfihi.to_string().starts_with("/gimfihi?"));

    for alias in ["/gimfi'i", "/gimfi%27i"] {
        let route = parse_test_route("", alias);
        assert_eq!(route.app_route(), AppRoute::Gimfihi);
        assert!(route.to_string().starts_with("/gimfihi?"));
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn typed_routes_accept_dioxus_route_strings() {
    assert_eq!(JbotciRoute::from_str("").unwrap().to_string(), "/vlacku");
    assert_eq!(
        JbotciRoute::from_str("gentufa?text=coi")
            .unwrap()
            .to_string(),
        "/gentufa?text=coi"
    );
    assert_eq!(
        JbotciRoute::from_str("settings?johau=lojban")
            .unwrap()
            .to_string(),
        "/settings?johau=lojban"
    );
    assert_eq!(
        JbotciRoute::from_str("cukta/section/chapter-abstractions#section-example")
            .unwrap()
            .to_string(),
        "/cukta/section/chapter-abstractions#section-example"
    );
    assert_eq!(
        JbotciRoute::from_str("vlacku/klama").unwrap().to_string(),
        "/vlacku/klama"
    );
    assert_eq!(
        JbotciRoute::from_str("vlacku/%2Fma.*%2F")
            .unwrap()
            .to_string(),
        "/vlacku/%2Fma.%2A%2F"
    );
    assert_eq!(
            JbotciRoute::from_str(
                "gimfihi?preset=1995&source=cmn%3A%3A&source=hin%3A%3A&source=eng%3A%3A&source=spa%3A%3A&source=rus%3A%3A&source=ara%3A%3A&shape=ccvcv&shape=cvccv&letters=source&check-collisions=all&require-free-short-rafsi=false&count=20",
            )
            .unwrap()
            .app_route(),
            AppRoute::Gimfihi
        );
    assert!(JbotciRoute::from_str("assets/compute-worker.js").is_err());
}

#[test]
#[requires(true)]
#[ensures(true)]
fn deployment_root_href_targets_router_prefix_root() {
    assert_eq!(deployment_root_href(""), "/");
    assert_eq!(deployment_root_href("/"), "/");
    assert_eq!(deployment_root_href("/jbotci"), "/jbotci/");
    assert_eq!(deployment_root_href("/jbotci/"), "/jbotci/");
}

#[requires(true)]
#[ensures(true)]
fn parse_test_route(base_path: &str, href: &str) -> JbotciRoute {
    jbotci_route_from_href(base_path, href).expect("test route should parse")
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_toc_hidden_button_opens_overlay_without_pinning() {
    let state = CuktaTocInteractionState {
        pinned: false,
        overlay_visible: false,
    };
    let button_state = cukta_toc_button_state(state.pinned, false, state.overlay_visible);

    assert_eq!(button_state, CuktaTocButtonState::Hidden);
    assert_eq!(
        cukta_toc_button_action(button_state),
        CuktaTocButtonAction::ShowOverlay
    );
    assert_eq!(
        cukta_toc_interaction_after_button_action(state, cukta_toc_button_action(button_state)),
        CuktaTocInteractionState {
            pinned: false,
            overlay_visible: true,
        }
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_toc_forced_visible_button_hides_overlay_without_changing_pin() {
    let state = CuktaTocInteractionState {
        pinned: true,
        overlay_visible: true,
    };
    let button_state = cukta_toc_button_state(state.pinned, true, state.overlay_visible);

    assert_eq!(button_state, CuktaTocButtonState::ForcedAutoHideVisible);
    assert_eq!(
        cukta_toc_button_action(button_state),
        CuktaTocButtonAction::HideOverlay
    );
    assert_eq!(
        cukta_toc_interaction_after_button_action(state, cukta_toc_button_action(button_state)),
        CuktaTocInteractionState {
            pinned: true,
            overlay_visible: false,
        }
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_toc_pinned_visible_button_unpins_and_keeps_overlay_visible() {
    let state = CuktaTocInteractionState {
        pinned: true,
        overlay_visible: false,
    };
    let button_state = cukta_toc_button_state(state.pinned, false, state.overlay_visible);

    assert_eq!(button_state, CuktaTocButtonState::PinnedVisible);
    assert_eq!(
        cukta_toc_button_action(button_state),
        CuktaTocButtonAction::Unpin
    );
    assert_eq!(
        cukta_toc_interaction_after_button_action(state, cukta_toc_button_action(button_state)),
        CuktaTocInteractionState {
            pinned: false,
            overlay_visible: true,
        }
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_toc_unpinned_visible_button_pins_and_returns_to_pinned_layout() {
    let state = CuktaTocInteractionState {
        pinned: false,
        overlay_visible: true,
    };
    let button_state = cukta_toc_button_state(state.pinned, false, state.overlay_visible);

    assert_eq!(button_state, CuktaTocButtonState::UnpinnedVisible);
    assert_eq!(
        cukta_toc_button_action(button_state),
        CuktaTocButtonAction::Pin
    );
    assert_eq!(
        cukta_toc_interaction_after_button_action(state, cukta_toc_button_action(button_state)),
        CuktaTocInteractionState {
            pinned: true,
            overlay_visible: false,
        }
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_toc_manual_expansion_survives_active_default_changes() {
    let state = CuktaTocExpansionState::default();
    assert!(!cukta_toc_node_expanded_with_default(
        "chapter-tour",
        false,
        &state
    ));

    let state = cukta_toc_expansion_with_node_state(&state, "chapter-tour", false, true);

    assert!(cukta_toc_node_expanded_with_default(
        "chapter-tour",
        false,
        &state
    ));
    assert!(cukta_toc_node_expanded_with_default(
        "chapter-tour",
        true,
        &state
    ));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn cukta_toc_default_matching_toggle_prunes_override() {
    let state = CuktaTocExpansionState::default();
    let state = cukta_toc_expansion_with_node_state(&state, "chapter-tour", false, true);
    let state = cukta_toc_expansion_with_node_state(&state, "chapter-tour", false, false);

    assert!(state.expanded.is_empty());
    assert!(state.collapsed.is_empty());
    assert!(!cukta_toc_node_expanded_with_default(
        "chapter-tour",
        false,
        &state
    ));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_jvozba_storage_reads_v0_json_items() {
    let raw = r#"
            [
              {"kind":"word","value":"cmene","indentLevel":2},
              {"kind":"rafsi","value":"vla","source":"valsi"}
            ]
        "#;

    let items = parse_vlacku_jvozba_items(raw);
    assert_eq!(
        items,
        vec![
            VlackuJvozbaItem {
                kind: VlackuJvozbaItemKind::Word,
                value: "cmene".to_owned(),
                source: None,
                indent_level: 2,
            },
            VlackuJvozbaItem {
                kind: VlackuJvozbaItemKind::FixedRafsi,
                value: "vla".to_owned(),
                source: Some("valsi".to_owned()),
                indent_level: 0,
            },
        ]
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_jvozba_storage_migrates_legacy_newline_items() {
    let items = parse_vlacku_jvozba_items("word\tcmene\nrafsi\tvla\nbad\tno\nword\t");

    assert_eq!(
        items,
        vec![
            VlackuJvozbaItem {
                kind: VlackuJvozbaItemKind::Word,
                value: "cmene".to_owned(),
                source: None,
                indent_level: 0,
            },
            VlackuJvozbaItem {
                kind: VlackuJvozbaItemKind::FixedRafsi,
                value: "vla".to_owned(),
                source: None,
                indent_level: 0,
            },
        ]
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlacku_jvozba_storage_writes_v0_json_shape() {
    let raw = format_vlacku_jvozba_items(&[VlackuJvozbaItem {
        kind: VlackuJvozbaItemKind::FixedRafsi,
        value: "vla".to_owned(),
        source: Some("valsi".to_owned()),
        indent_level: 1,
    }]);

    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&raw).expect("valid json"),
        serde_json::json!([
            {"kind":"rafsi","value":"vla","source":"valsi","indentLevel":1}
        ])
    );
}

#[requires(row_span > 0)]
#[ensures(ret.row == row)]
fn test_gentufa_block(
    row: usize,
    row_span: usize,
    marker_roles: &[ReferenceMarkerRole],
) -> GentufaBlock {
    new!(GentufaBlock {
        block_id: format!("test-{row}"),
        node_ids: Vec::new(),
        label: "test".to_owned(),
        is_leaf: true,
        is_elided: false,
        token_kind: None,
        ref_markers: marker_roles
            .iter()
            .enumerate()
            .map(|(index, role)| test_reference_marker(*role, index))
            .collect(),
        span: None,
        node_types: Vec::new(),
        ancestors: Vec::new(),
        col: 0,
        col_span: 1,
        row,
        row_span,
        color: "#ffffff".to_owned(),
        parent_color: None,
        raw_text: "test".to_owned(),
        display_text: "test".to_owned(),
        transform: None,
        glosses: Vec::new(),
        definition: None,
        computed_gloss: None,
        tooltip: None,
    })
}

#[requires(true)]
#[ensures(ret.role == role)]
fn test_reference_marker(role: ReferenceMarkerRole, index: usize) -> ReferenceMarker {
    ReferenceMarker {
        role,
        kind: ReferenceMarkerKind::Reference,
        label: ReferenceLabel::new("b", Some(index + 1), None),
        source: None,
        tooltip: None,
    }
}

#[requires(true)]
#[ensures(true)]
fn test_reference_tooltip() -> ReferenceTooltip {
    new!(ReferenceTooltip {
        card: None,
        missing_word: Some("b".to_owned()),
        highlighted_places: Vec::new(),
        definition: Vec::new(),
        notes: Vec::new(),
        rows: Vec::new(),
    })
}

#[requires(char_start <= char_end)]
#[requires(!code.is_empty())]
#[requires(!message.is_empty())]
#[requires(!label.is_empty())]
#[ensures(!ret.labels.is_empty())]
fn test_diagnostic(
    source: &str,
    severity: DiagnosticSeverity,
    code: &str,
    message: &str,
    char_start: usize,
    char_end: usize,
    label: &str,
) -> Diagnostic {
    let span =
        jbotci_diagnostics::source_span_from_char_offsets(None, source, char_start, char_end)
            .expect("test span is valid");
    Diagnostic::new(
        severity,
        jbotci_diagnostics::DiagnosticPhase::Syntax,
        code.to_owned(),
        message.to_owned(),
        vec![DiagnosticLabel::new(span, label.to_owned(), true)],
        Vec::new(),
        None,
    )
}

#[requires(!text.is_empty())]
#[ensures(ret.role == role)]
fn diagnostic_test_part(role: DiagnosticTextRole, text: &str) -> DiagnosticTextRenderPart {
    diagnostic_text_segment_render_parts(&[DiagnosticTextSegment::new(role, text.to_owned())])
        .into_iter()
        .next()
        .expect("single diagnostic segment renders to a part")
}

#[requires(true)]
#[ensures(true)]
fn has_css_class(class_name: &str, expected: &str) -> bool {
    class_name
        .split_whitespace()
        .any(|class_name| class_name == expected)
}
