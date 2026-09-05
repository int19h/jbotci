use super::*;

#[requires(true)]
#[ensures(true)]
fn parse(source: &str, compounds: bool, elided: bool) -> GentufaSuccess {
    let request = GentufaWebRequest {
        text: source.to_owned(),
        options: GentufaWebOptions {
            show_compounds: compounds,
            show_elided: elided,
            show_glosses: true,
            ..GentufaWebOptions::default()
        },
    };
    let GentufaWebResult::Success(success) = parse_gentufa_for_web(&request) else {
        panic!("expected a success projection: {source}");
    };
    success
}

#[test]
#[requires(true)]
#[ensures(true)]
fn compound_blocks_have_one_documentation_host_and_preserve_other_projections() {
    for source in [
        "batke zei uidje",
        "bapuba klama",
        "la pa da cu klama",
        ".abu cu klama",
        "klama zei tavla",
        "mi klama",
    ] {
        for elided in [false, true] {
            let enabled = parse(source, true, elided);
            let disabled = parse(source, false, elided);
            assert_eq!(enabled.tree_rows, disabled.tree_rows, "Tree: {source}");
            assert_eq!(enabled.ipa_text, disabled.ipa_text);
            assert_eq!(enabled.brackets_text, disabled.brackets_text);
            assert_eq!(enabled.bracket_fragments, disabled.bracket_fragments);
            assert_eq!(
                enabled.blocks_layout.max_col,
                disabled.blocks_layout.max_col
            );
            if source == "batke zei uidje" {
                let compounds: Vec<_> = enabled
                    .blocks_layout
                    .blocks
                    .iter()
                    .filter(|block| block.compound_kind.is_some())
                    .collect();
                assert_eq!(compounds.len(), 1);
                let block = compounds[0];
                assert_eq!(block.compound_kind, Some(GentufaCompoundKind::Zei));
                assert!(block.is_leaf);
                assert_eq!(block.col_span, 3);
                let mut members: Vec<_> = disabled
                    .blocks_layout
                    .blocks
                    .iter()
                    .filter(|block| block.is_leaf && block.role.is_normal())
                    .collect();
                members.sort_by_key(|block| block.col);
                assert_eq!(
                    block.display_text,
                    members
                        .iter()
                        .map(|block| block.display_text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                assert_eq!(
                    block.glosses,
                    ["button (graphical user interface element)".to_owned()]
                );
                assert_eq!(
                    enabled
                        .blocks_layout
                        .blocks
                        .iter()
                        .filter(|block| block
                            .tooltip
                            .as_ref()
                            .is_some_and(|card| card.word == "batke zei uidje"))
                        .count(),
                    1
                );
                assert_eq!(
                    disabled
                        .blocks_layout
                        .blocks
                        .iter()
                        .filter(|block| block.is_leaf && block.role.is_normal())
                        .count(),
                    3
                );
            }
            if source == "klama zei tavla" || source == ".abu cu klama" {
                assert_eq!(enabled.blocks_layout, disabled.blocks_layout);
            }
            // Both annotation passes obey the same host resolution, including
            // the normal structural fallback for BU and duplicate elided ancestors.
            for block in enabled
                .blocks_layout
                .blocks
                .iter()
                .filter(|block| block.tooltip.is_some())
            {
                assert_eq!(
                    enabled
                        .blocks_layout
                        .blocks
                        .iter()
                        .filter(|other| other.tooltip == block.tooltip
                            && other.span == block.span
                            && other.role == block.role)
                        .count(),
                    1,
                    "{source}"
                );
            }
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn straddling_compound_preserves_the_remaining_number_and_its_annotations() {
    for elided in [false, true] {
        let enabled = parse("mi re pa moi", true, elided);
        let disabled = parse("mi re pa moi", false, elided);
        assert_eq!(
            enabled.blocks_layout.max_col,
            disabled.blocks_layout.max_col
        );
        let retained = enabled
            .blocks_layout
            .blocks
            .iter()
            .find(|block| block.is_leaf && block.raw_text == "re")
            .expect("the number donor must retain its remaining re leaf");
        let original = disabled
            .blocks_layout
            .blocks
            .iter()
            .find(|block| block.is_leaf && block.raw_text == "re")
            .unwrap();
        assert_eq!(retained.display_text, original.display_text);
        assert_eq!(retained.glosses, original.glosses);
        assert_eq!(retained.tooltip, original.tooltip);
        assert_eq!(retained.span, original.span);
        assert_eq!(retained.token_kind, original.token_kind);
        let compounds = enabled
            .blocks_layout
            .blocks
            .iter()
            .filter(|block| block.compound_kind.is_some())
            .collect::<Vec<_>>();
        assert_eq!(compounds.len(), 1);
        assert!(compounds[0].is_leaf);
        assert_eq!(compounds[0].raw_text, "pa moi");
        assert_eq!(compounds[0].col_span, 2);
        assert!(
            compounds[0]
                .glosses
                .iter()
                .any(|gloss| gloss == "first" || gloss == "is first among")
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn recovered_projection_keeps_error_barriers_and_later_compounds() {
    for source in ["ba pu mi ku i do", "mi ku i je do"] {
        for elided in [false, true] {
            let enabled = parse(source, true, elided);
            let disabled = parse(source, false, elided);
            let errors = enabled
                .blocks_layout
                .blocks
                .iter()
                .filter(|block| block.role.is_error())
                .collect::<Vec<_>>();
            assert!(!errors.is_empty());
            let compounds = enabled
                .blocks_layout
                .blocks
                .iter()
                .filter(|block| block.compound_kind.is_some())
                .collect::<Vec<_>>();
            assert!(!compounds.is_empty(), "{source}");
            for compound in compounds {
                let span = compound.span.unwrap();
                assert!(errors.iter().all(|error| {
                    let error = error.span.unwrap();
                    error.byte_end <= span.byte_start || span.byte_end <= error.byte_start
                }));
            }
            assert_eq!(enabled.tree_rows, disabled.tree_rows);
            assert_eq!(enabled.bracket_fragments, disabled.bracket_fragments);
            assert_eq!(
                enabled.blocks_layout.max_col,
                disabled.blocks_layout.max_col
            );
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn compound_rewrite_preserves_enriched_reference_endpoints_and_content() {
    let source = "la pa da cu klama .i ri tavla";
    for elided in [false, true] {
        let enabled = parse(source, true, elided);
        let disabled = parse(source, false, elided);
        assert!(
            enabled
                .blocks_layout
                .blocks
                .iter()
                .any(|block| block.compound_kind.is_some())
        );
        let mut before = disabled
            .blocks_layout
            .blocks
            .iter()
            .flat_map(|block| &block.ref_markers)
            .map(|marker| serde_json::to_string(marker).unwrap())
            .collect::<Vec<_>>();
        let mut after = enabled
            .blocks_layout
            .blocks
            .iter()
            .flat_map(|block| &block.ref_markers)
            .map(|marker| serde_json::to_string(marker).unwrap())
            .collect::<Vec<_>>();
        assert!(!before.is_empty());
        before.sort();
        after.sort();
        assert_eq!(before, after);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn compounds_state_defaults_and_routes_survive_old_payloads() {
    assert!(GentufaWebState::default().show_compounds);
    assert!(GentufaWebOptions::default().show_compounds);
    let mut old_state = serde_json::to_value(GentufaWebState::default()).unwrap();
    old_state.as_object_mut().unwrap().remove("show-compounds");
    assert!(
        serde_json::from_value::<GentufaWebState>(old_state)
            .unwrap()
            .show_compounds
    );
    let mut old_request = serde_json::to_value(GentufaWebRequest::default()).unwrap();
    old_request["options"]
        .as_object_mut()
        .unwrap()
        .remove("show-compounds");
    assert!(
        serde_json::from_value::<GentufaWebRequest>(old_request)
            .unwrap()
            .options
            .show_compounds
    );
    for query in [
        "text=batke+zei+uidje&compounds=false",
        "text=&compounds=false",
        "compounds=false",
    ] {
        let state = parse_gentufa_web_route("/gentufa", query);
        assert!(!state.show_compounds);
        let url = gentufa_web_url("", &state);
        assert!(url.contains("compounds=false"));
        assert_eq!(
            parse_gentufa_web_route("/gentufa", url.split_once('?').unwrap().1),
            state
        );
        for format in [GentufaExportFormat::Svg, GentufaExportFormat::Png] {
            let url = gentufa_export_url("", &state, format, GentufaScript::Latin);
            assert_eq!(
                parse_gentufa_web_export_request(url.split_once('?').unwrap().1).state,
                state
            );
        }
    }
    for query in ["", "compounds=true", "compounds=invalid"] {
        let state = parse_gentufa_web_route("/gentufa", query);
        assert!(state.show_compounds);
        assert!(!gentufa_web_url("", &state).contains("compounds"));
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn client_and_server_exports_share_the_resolved_compound_layout() {
    for compounds in [false, true] {
        let success = parse("batke zei uidje", compounds, false);
        let state = GentufaWebState {
            text: "batke zei uidje".to_owned(),
            show_compounds: compounds,
            show_glosses: true,
            ..GentufaWebState::default()
        };
        let mut expected_png_dimensions = None;
        for format in [GentufaExportFormat::Svg, GentufaExportFormat::Png] {
            let client = render_gentufa_blocks_web_export(
                &success.blocks_layout,
                true,
                GentufaScript::Latin,
                format,
            )
            .unwrap();
            let server =
                render_gentufa_state_web_export(&state, GentufaScript::Latin, format).unwrap();
            assert_eq!(client, server);
            if let Some(expected) = expected_png_dimensions {
                assert_eq!((client.width.unwrap(), client.height.unwrap()), expected);
            } else {
                let svg = std::str::from_utf8(&client.bytes).unwrap();
                let document = roxmltree::Document::parse(svg).unwrap();
                let root = document.root_element();
                let width = root.attribute("width").unwrap().parse::<f32>().unwrap();
                let height = root.attribute("height").unwrap().parse::<f32>().unwrap();
                expected_png_dimensions = Some((
                    (width * DEFAULT_GENTUFA_PNG_SCALE).ceil() as usize,
                    (height * DEFAULT_GENTUFA_PNG_SCALE).ceil() as usize,
                ));
                if compounds {
                    assert_eq!(svg.matches("button").count(), 1);
                }
            }
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn projection_selects_the_coalesced_layout_only_when_a_compound_was_applied() {
    for (source, has_compound) in [("mi pa moi klama", true), ("mi klama", false)] {
        for show_compounds in [false, true] {
            for show_elided in [false, true] {
                let options = GentufaWebOptions {
                    show_compounds,
                    show_elided,
                    ..GentufaWebOptions::default()
                };
                let morphology = analyze_gentufa_morphology_source(source, &options).unwrap();
                let analysis = complete_gentufa_source_analysis(source, &options, morphology);
                let data!(GentufaSourceAnalysis {
                    morphology,
                    parse: recovery,
                    ..
                }) = analysis.into_data();
                let words = morphology.into_data().words;
                let data!(SyntaxRecoveryParse::Valid { parse: valid }) = recovery.into_data()
                else {
                    panic!("{source} parses without recovery");
                };
                let projection = generated_model_gentufa_blocks_projection(
                    &valid.parse_tree,
                    source,
                    &words,
                    &gentufa_blocks_projection_options(&options),
                )
                .unwrap();
                assert_eq!(
                    projection.coalesced.is_some(),
                    has_compound && show_compounds,
                    "{source} compounds={show_compounds}"
                );
                assert!(
                    projection
                        .bare
                        .blocks
                        .iter()
                        .all(|block| block.compound_kind.is_none())
                );
                assert_eq!(
                    projection
                        .bare
                        .blocks
                        .iter()
                        .any(|block| block.role.is_elided()),
                    show_elided
                );
                let expected = parse(source, show_compounds, show_elided).blocks_layout;
                assert_eq!(projection.into_blocks_layout(), expected);
            }
        }
    }
}
