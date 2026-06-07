mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, requires};
use jbotci_source::SourceId;
use support::fixtures::{
    BracketExpectations, CllSelector, CommandOutputExpectation, DiagnosticExpectation,
    ExpectationStatus, Expectations, Facet, FacetResult, FixtureBackend, FixtureExport,
    FixtureSelector, GentufaOutputExpectation, JvozbaExpectation, JvozbaFixtureInput,
    JvozbaFixtureMode, JvozbaOutputExpectation, JvozbaSegmentExpectation,
    JvozbaSegmentKindExpectation, LoadedTestCase, MorphologyExpectation, MuplisForm,
    OutputExpectations, Provenance, ReferenceExpectation, ScriptBracketExpectations,
    SemanticsExpectations, SyntaxExpectation, TestCase, TextExpectation, VlaseiOutputExpectation,
    XfailExpectation, filter_fixtures, import_export_file, load_fixture_file, load_fixture_tree,
    run_fixture_facets, run_fixture_facets_parallel, validate_fixture_tree, write_fixture_file,
};

#[test]
#[requires(true)]
#[ensures(true)]
fn loads_smoke_fixture() {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/adhoc/smoke/coi.toml");
    let test_case = load_fixture_file(fixture_path).expect("fixture should load");
    assert_eq!(test_case.id, "adhoc.smoke.coi");
    assert_eq!(test_case.lojban, "coi");
    assert!(test_case.tags.contains(&"smoke".to_owned()));
    let vlasei_json = test_case
        .expectations
        .output
        .expect("output expectation")
        .vlasei
        .expect("vlasei output")
        .json
        .expect("vlasei JSON")
        .text;
    let value: serde_json::Value = serde_json::from_str(&vlasei_json).expect("vlasei JSON");
    assert_eq!(value[0]["PlainWord"]["Cmavo"]["phonemes"], "coĭ");
    assert_eq!(
        value[0]["PlainWord"]["Cmavo"]["span"],
        serde_json::json!([0, 3])
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn profile_filters_cll_chapter_and_muplis_form() {
    let root = Path::new("tests/fixtures");
    let cll = loaded_case(
        "tests/fixtures/cll/chapter-18/section-18.3/c18e3d1.toml",
        TestCase {
            id: "cll.18.3.c18e3d1".into(),
            lojban: "coi".into(),
            dialect: None,
            translation_en: None,
            gloss_en: None,
            tags: vec![],
            provenance: vec![Provenance::Cll {
                chapter: 18,
                section_number: "18.3".into(),
                section_id: "c18s3".into(),
                example_number: Some("18.12".into()),
                example_id: Some("c18e3d1".into()),
                source_path: Some("vendor/cll/chapters/18.xml".into()),
            }],
            expectations: Expectations::default(),
        },
    );
    let muplis = loaded_case(
        "tests/fixtures/muplis/collection-18/1-front.toml",
        TestCase {
            id: "muplis.18.1.front".into(),
            lojban: "coi".into(),
            dialect: None,
            translation_en: None,
            gloss_en: None,
            tags: vec![],
            provenance: vec![Provenance::Muplis {
                collection_id: "18".into(),
                item_id: Some("1".into()),
                form: Some(MuplisForm::Front),
                url: None,
            }],
            expectations: Expectations::default(),
        },
    );
    let fixtures = vec![cll, muplis];
    let cll_selector = FixtureSelector {
        cll: Some(CllSelector {
            chapter: Some(18),
            example_id: Some("c18e3d1".into()),
            ..CllSelector::default()
        }),
        ..FixtureSelector::default()
    };
    assert_eq!(filter_fixtures(root, &fixtures, &cll_selector).len(), 1);

    let muplis_selector = FixtureSelector {
        muplis: Some(support::fixtures::MuplisSelector {
            collection_id: Some("18".into()),
            form: Some(MuplisForm::Front),
            ..support::fixtures::MuplisSelector::default()
        }),
        ..FixtureSelector::default()
    };
    assert_eq!(filter_fixtures(root, &fixtures, &muplis_selector).len(), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn fake_runner_counts_failures() {
    #[invariant(true)]
    struct FakeBackend;
    #[contract_trait]
    impl FixtureBackend for FakeBackend {
        #[requires(true)]
        #[ensures(true)]
        fn run(&self, _fixture: &LoadedTestCase, facet: Facet) -> FacetResult {
            match facet {
                Facet::Morphology => FacetResult::passed(),
                Facet::Syntax => FacetResult::failed("syntax failed"),
                _ => FacetResult::skipped("not selected"),
            }
        }
    }

    let case = loaded_case(
        "tests/fixtures/adhoc/smoke/coi.toml",
        TestCase {
            id: "adhoc.smoke.coi".into(),
            lojban: "coi".into(),
            dialect: None,
            translation_en: None,
            gloss_en: None,
            tags: vec!["smoke".into()],
            provenance: vec![Provenance::Adhoc { description: None }],
            expectations: Expectations::default(),
        },
    );
    let fixtures = vec![&case];
    let summary = run_fixture_facets(&FakeBackend, &fixtures, &[Facet::Morphology, Facet::Syntax]);
    assert_eq!(summary.passed, 1);
    assert_eq!(summary.failed, 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn fake_runner_counts_xfails() {
    #[invariant(true)]
    struct FakeBackend;
    #[contract_trait]
    impl FixtureBackend for FakeBackend {
        #[requires(true)]
        #[ensures(true)]
        fn run(&self, _fixture: &LoadedTestCase, facet: Facet) -> FacetResult {
            match facet {
                Facet::Syntax => FacetResult::xfailed("known v0 xfail"),
                _ => FacetResult::passed(),
            }
        }
    }

    let case = loaded_case(
        "tests/fixtures/adhoc/xfail.toml",
        TestCase {
            id: "adhoc.xfail".into(),
            lojban: "coi".into(),
            dialect: None,
            translation_en: None,
            gloss_en: None,
            tags: vec![],
            provenance: vec![Provenance::Adhoc { description: None }],
            expectations: Expectations::default(),
        },
    );
    let fixtures = vec![&case];
    let summary = run_fixture_facets(&FakeBackend, &fixtures, &[Facet::Syntax]);
    assert_eq!(summary.xfailed, 1);
    assert_eq!(summary.failed, 0);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn parallel_runner_matches_serial_summary() {
    #[invariant(true)]
    struct FakeBackend;
    #[contract_trait]
    impl FixtureBackend for FakeBackend {
        #[requires(true)]
        #[ensures(true)]
        fn run(&self, fixture: &LoadedTestCase, facet: Facet) -> FacetResult {
            match (&fixture.test_case.id[..], facet) {
                ("adhoc.first", Facet::Morphology) => FacetResult::passed(),
                ("adhoc.second", Facet::Morphology) => FacetResult::failed("mismatch"),
                _ => FacetResult::skipped("not selected"),
            }
        }
    }

    let first = loaded_case(
        "tests/fixtures/adhoc/first.toml",
        TestCase {
            id: "adhoc.first".into(),
            lojban: "coi".into(),
            dialect: None,
            translation_en: None,
            gloss_en: None,
            tags: vec![],
            provenance: vec![Provenance::Adhoc { description: None }],
            expectations: Expectations::default(),
        },
    );
    let second = loaded_case(
        "tests/fixtures/adhoc/second.toml",
        TestCase {
            id: "adhoc.second".into(),
            lojban: "co'o".into(),
            dialect: None,
            translation_en: None,
            gloss_en: None,
            tags: vec![],
            provenance: vec![Provenance::Adhoc { description: None }],
            expectations: Expectations::default(),
        },
    );
    let fixtures = vec![&first, &second];
    let facets = [Facet::Morphology, Facet::Syntax];
    assert_eq!(
        run_fixture_facets_parallel(&FakeBackend, &fixtures, &facets),
        run_fixture_facets(&FakeBackend, &fixtures, &facets)
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn morphology_raw_matches_simple_cll_fixture() {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cll/chapter-05/section-5.1/c5e1d1.toml");
    let test_case = load_fixture_file(fixture_path).expect("fixture should load");
    let expected = test_case
        .expectations
        .morphology
        .expect("morphology expectation")
        .raw
        .expect("morphology raw")
        .text;
    let actual = jbotci_morphology::segment_words_with_modifiers_with_source_id(
        &test_case.lojban,
        SourceId("<fixture>".to_owned()),
    )
    .expect("simple fixture should segment");
    assert_eq!(format!("{actual:?}"), expected);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn camxes_compatible_morphology_fixtures_match() {
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/adhoc/morphology/camxes-compatible");
    let fixtures = load_fixture_tree(&fixture_root).expect("camxes-compatible morphology fixtures");
    assert!(!fixtures.is_empty());
    for fixture in fixtures {
        let Some(expectation) = fixture.test_case.expectations.morphology.as_ref() else {
            continue;
        };
        assert_morphology_expectation(&fixture.test_case, expectation);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn jvozba_fixtures_validate_output_and_parse_back() {
    let fixture_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/adhoc/jvozba");
    let fixtures = load_fixture_tree(&fixture_root).expect("jvozba fixtures");
    assert!(!fixtures.is_empty());
    for fixture in fixtures {
        let Some(expectation) = fixture.test_case.expectations.jvozba.as_ref() else {
            continue;
        };
        assert_jvozba_expectation(&fixture.test_case.id, expectation);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn import_writes_toml_fixture() {
    let temp_root = temp_root("jbotci-fixtures-import-test");
    fs::create_dir_all(&temp_root).expect("temp root");
    let input = temp_root.join("export.json");
    let output = temp_root.join("fixtures");
    let export = FixtureExport {
        schema_version: 1,
        cases: vec![TestCase {
            id: "adhoc.import".into(),
            lojban: "coi".into(),
            dialect: Some("(case-insensitive)".into()),
            translation_en: None,
            gloss_en: None,
            tags: vec!["generated".into()],
            provenance: vec![Provenance::Adhoc {
                description: Some("test".into()),
            }],
            expectations: Expectations::default(),
        }],
    };
    fs::write(&input, serde_json::to_string(&export).expect("json")).expect("write export");
    let summary = import_export_file(&input, &output).expect("import");
    assert_eq!(summary.written, 1);
    let fixtures = load_fixture_tree(&output).expect("fixtures");
    assert_eq!(fixtures.len(), 1);
    assert_eq!(
        fixtures[0].test_case.dialect.as_deref(),
        Some("(case-insensitive)")
    );
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn fixture_check_rejects_invalid_dialect_formula() {
    let temp_root = temp_root("jbotci-fixtures-invalid-dialect-test");
    let fixture_root = temp_root.join("fixtures");
    fs::create_dir_all(fixture_root.join("adhoc")).expect("temp fixture root");
    fs::write(
        fixture_root.join("adhoc").join("bad.toml"),
        "id = \"adhoc.bad\"\nlojban = \"coi\"\ndialect = \"(no-cgv)\"\n",
    )
    .expect("write invalid fixture");
    let error = validate_fixture_tree(&fixture_root).expect_err("invalid dialect");
    assert!(error.to_string().contains("invalid dialect formula"));
    assert!(error.to_string().contains("no-cgv"));
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn fixture_check_rejects_invalid_xfail_metadata() {
    let temp_root = temp_root("jbotci-fixtures-invalid-xfail-test");
    let fixture_root = temp_root.join("fixtures");
    fs::create_dir_all(fixture_root.join("adhoc")).expect("temp fixture root");
    fs::write(
        fixture_root.join("adhoc").join("bad.toml"),
        "id = \"adhoc.bad\"\nlojban = \"coi\"\n\n[expectations.syntax]\nstatus = \"success\"\nxfail = { source = \"\", reason = \"\", accepted-status = \"success\" }\n",
    )
    .expect("write invalid fixture");
    let error = validate_fixture_tree(&fixture_root).expect_err("invalid xfail");
    assert!(error.to_string().contains("invalid syntax xfail metadata"));
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn writer_keeps_tree_and_output_values() {
    let temp_root = temp_root("jbotci-fixtures-writer-test");
    fs::create_dir_all(&temp_root).expect("temp root");
    let fixture_path = temp_root.join("fixture.toml");
    let test_case = TestCase {
        id: "adhoc.syntax".into(),
        lojban: "coi".into(),
        dialect: Some("(case-insensitive)".into()),
        translation_en: None,
        gloss_en: None,
        tags: vec![],
        provenance: vec![],
        expectations: Expectations {
            output: Some(OutputExpectations {
                vlasei: Some(VlaseiOutputExpectation {
                    json: Some(TextExpectation {
                        text: "[{\"PlainWord\":{\"Cmavo\":{\"phonemes\":\"coĭ\",\"span\":[0,3]}}}]"
                            .into(),
                    }),
                    ..VlaseiOutputExpectation::default()
                }),
                gentufa: Some(GentufaOutputExpectation {
                    brackets: Some(TextExpectation {
                        text: "[coi]".into(),
                    }),
                    tree: Some(TextExpectation {
                        text: "\"coi\"".into(),
                    }),
                    json: Some(TextExpectation {
                        text: "{}".into(),
                    }),
                    show_elided: None,
                }),
            }),
            morphology: Some(MorphologyExpectation {
                status: ExpectationStatus::Success,
                raw: Some(TextExpectation {
                    text: "[WordLike(PlainWord(Word(Cmavo { phonemes: Phonemes(PhonemesData { text: \"coĭ\" }), span: SourceSpan(SourceSpanData { source_id: None, byte_start: 0, byte_end: 3, char_start: 0, char_end: 3, start: None, end: None }) })))]".into(),
                }),
                diagnostics: vec![],
            }),
            jvozba: None,
            syntax: Some(SyntaxExpectation {
                status: ExpectationStatus::Success,
                raw: Some(TextExpectation {
                    text: "TextSyntax { leading_nai: [], leading_cmevla: [], leading_indicators: [], leading_free_modifiers: [], leading_connective: None, paragraphs: [] }".into(),
                }),
                diagnostics: vec![],
                xfail: Some(XfailExpectation {
                    source: "test".into(),
                    reason: "intentional writer coverage".into(),
                    accepted_status: ExpectationStatus::Failure,
                }),
            }),
            semantics: Some(SemanticsExpectations {
                refs: Some(ReferenceExpectation {
                    status: ExpectationStatus::Success,
                    raw: Some(TextExpectation {
                        text: "{\"frames\":[],\"assignments\":[],\"relation-places\":[],\"references\":[]}".into(),
                    }),
                }),
            }),
        },
    };
    write_fixture_file(&fixture_path, &test_case).expect("write fixture");
    let text = fs::read_to_string(&fixture_path).expect("read fixture");
    assert!(
        text.starts_with(
            "id = \"adhoc.syntax\"\nlojban = \"coi\"\ndialect = \"(case-insensitive)\""
        )
    );
    assert!(text.contains("[expectations.output.vlasei]\njson = "));
    assert!(text.contains("[expectations.output.gentufa]\nbrackets = \"[coi]\""));
    assert!(text.contains("tree = '\"coi\"'"));
    assert!(text.contains("[expectations.morphology]\nstatus = \"success\"\nraw = "));
    assert!(!text.contains("words = ["));
    assert!(!text.contains("options = "));
    assert!(text.contains("[expectations.syntax]\nstatus = \"success\"\nraw = "));
    assert!(text.contains("[expectations.semantics.refs]\nstatus = \"success\"\nraw = "));
    assert!(!text.contains("parse-tree"));
    assert!(
        text.contains(
            "xfail = { source = \"test\", reason = \"intentional writer coverage\", accepted-status = \"failure\" }"
        )
    );
    assert!(!text.contains("[expectations.morphology.words"));
    assert!(!text.contains("[expectations.syntax.parse-tree"));
    assert_eq!(
        load_fixture_file(&fixture_path).expect("load fixture"),
        test_case
    );
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn writer_round_trips_script_brackets_and_show_elided_profile() {
    let temp_root = temp_root("jbotci-fixtures-script-writer-test");
    fs::create_dir_all(&temp_root).expect("temp root");
    let fixture_path = temp_root.join("fixture.toml");
    let test_case = TestCase {
        id: "adhoc.script-output".into(),
        lojban: "mi klama".into(),
        dialect: None,
        translation_en: None,
        gloss_en: None,
        tags: vec![],
        provenance: vec![],
        expectations: Expectations {
            output: Some(OutputExpectations {
                vlasei: Some(VlaseiOutputExpectation {
                    brackets: Some(BracketExpectations::Scripts(ScriptBracketExpectations {
                        latin: Some(TextExpectation {
                            text: "(mi kláma)".into(),
                        }),
                        cyrillic: Some(TextExpectation {
                            text: "(ми кла́ма)".into(),
                        }),
                        zbalermorna: Some(TextExpectation {
                            text: "zbal".into(),
                        }),
                    })),
                    ..VlaseiOutputExpectation::default()
                }),
                gentufa: Some(GentufaOutputExpectation {
                    show_elided: Some(CommandOutputExpectation {
                        brackets: Some(TextExpectation {
                            text: "(mi kláma vau)".into(),
                        }),
                        tree: Some(TextExpectation {
                            text: "tree".into(),
                        }),
                        json: Some(TextExpectation { text: "{}".into() }),
                    }),
                    ..GentufaOutputExpectation::default()
                }),
            }),
            ..Expectations::default()
        },
    };
    write_fixture_file(&fixture_path, &test_case).expect("write fixture");
    let text = fs::read_to_string(&fixture_path).expect("read fixture");
    assert!(text.contains("[expectations.output.vlasei.brackets]\nlatin = "));
    assert!(text.contains("cyrillic = "));
    assert!(text.contains("zbalermorna = "));
    assert!(text.contains("[expectations.output.gentufa.show-elided]\nbrackets = "));
    assert_eq!(
        load_fixture_file(&fixture_path).expect("load fixture"),
        test_case
    );
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn writer_round_trips_jvozba_expectation() {
    let temp_root = temp_root("jbotci-fixtures-jvozba-writer-test");
    fs::create_dir_all(&temp_root).expect("temp root");
    let fixture_path = temp_root.join("fixture.toml");
    let test_case = TestCase {
        id: "adhoc.jvozba.writer".into(),
        lojban: "fulta ismu".into(),
        dialect: None,
        translation_en: None,
        gloss_en: None,
        tags: vec!["jvozba".into()],
        provenance: vec![],
        expectations: Expectations {
            jvozba: Some(JvozbaExpectation {
                status: ExpectationStatus::Success,
                mode: JvozbaFixtureMode::Lujvo,
                inputs: vec![
                    JvozbaFixtureInput::Word {
                        text: "fulta".into(),
                    },
                    JvozbaFixtureInput::Word {
                        text: "ismu".into(),
                    },
                ],
                output: Some(JvozbaOutputExpectation {
                    word: "fuly'ismu".into(),
                    segments: vec![
                        JvozbaSegmentExpectation {
                            kind: JvozbaSegmentKindExpectation::Rafsi,
                            text: "ful".into(),
                        },
                        JvozbaSegmentExpectation {
                            kind: JvozbaSegmentKindExpectation::Hyphen,
                            text: "y'".into(),
                        },
                        JvozbaSegmentExpectation {
                            kind: JvozbaSegmentKindExpectation::Rafsi,
                            text: "ismu".into(),
                        },
                    ],
                }),
                error: None,
            }),
            ..Expectations::default()
        },
    };
    write_fixture_file(&fixture_path, &test_case).expect("write fixture");
    let text = fs::read_to_string(&fixture_path).expect("read fixture");
    assert!(text.contains("[expectations.jvozba]\nstatus = \"success\""));
    assert!(text.contains("mode = \"lujvo\""));
    assert!(text.contains("kind = \"fixed-rafsi\"") || text.contains("kind = \"word\""));
    assert_eq!(
        load_fixture_file(&fixture_path).expect("load fixture"),
        test_case
    );
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn available_facets_include_tree_expectations() {
    let case = TestCase {
        id: "adhoc.tree".into(),
        lojban: "coi".into(),
        dialect: None,
        translation_en: None,
        gloss_en: None,
        tags: vec![],
        provenance: vec![],
        expectations: Expectations {
            output: Some(OutputExpectations {
                gentufa: Some(GentufaOutputExpectation {
                    tree: Some(TextExpectation {
                        text: "\"coi\"".into(),
                    }),
                    ..GentufaOutputExpectation::default()
                }),
                ..OutputExpectations::default()
            }),
            ..Expectations::default()
        },
    };
    let facets = case.available_facets();
    assert!(facets.contains(&Facet::GentufaTree));
    assert!(!facets.contains(&Facet::GentufaBrackets));
    assert_eq!(
        "gentufa-tree".parse::<Facet>().expect("tree facet"),
        Facet::GentufaTree
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn legacy_vlasei_brackets_load_as_latin_facet() {
    let source = r#"
id = "adhoc.legacy-brackets"
lojban = "mi klama"

[expectations.output.vlasei]
brackets = "(mi kláma)"
"#;
    let case: TestCase = toml::from_str(source).expect("legacy fixture");
    let facets = case.available_facets();
    assert!(facets.contains(&Facet::VlaseiBrackets));
    assert!(!facets.contains(&Facet::VlaseiBracketsCyrillic));
    let brackets = case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.vlasei.as_ref())
        .and_then(|vlasei| vlasei.brackets.as_ref())
        .expect("brackets expectation");
    assert_eq!(
        brackets
            .expectation_for_script(jbotci_orthography::LojbanScript::Latin)
            .map(|expectation| expectation.text.as_str()),
        Some("(mi kláma)")
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn available_facets_include_script_bracket_expectations() {
    let case = TestCase {
        id: "adhoc.script-brackets".into(),
        lojban: "mi klama".into(),
        dialect: None,
        translation_en: None,
        gloss_en: None,
        tags: vec![],
        provenance: vec![],
        expectations: Expectations {
            output: Some(OutputExpectations {
                vlasei: Some(VlaseiOutputExpectation {
                    brackets: Some(BracketExpectations::Scripts(ScriptBracketExpectations {
                        latin: Some(TextExpectation {
                            text: "(mi kláma)".into(),
                        }),
                        cyrillic: Some(TextExpectation {
                            text: "(ми кла́ма)".into(),
                        }),
                        zbalermorna: Some(TextExpectation {
                            text: "zbal".into(),
                        }),
                    })),
                    ..VlaseiOutputExpectation::default()
                }),
                ..OutputExpectations::default()
            }),
            ..Expectations::default()
        },
    };
    let facets = case.available_facets();
    assert!(facets.contains(&Facet::VlaseiBrackets));
    assert!(facets.contains(&Facet::VlaseiBracketsCyrillic));
    assert!(facets.contains(&Facet::VlaseiBracketsZbalermorna));
    assert_eq!(
        "vlasei-brackets-cyrillic"
            .parse::<Facet>()
            .expect("cyrillic facet"),
        Facet::VlaseiBracketsCyrillic
    );
    assert_eq!(
        Facet::VlaseiBracketsZbalermorna.to_string(),
        "vlasei-brackets-zbalermorna"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn available_facets_include_gentufa_show_elided_expectations() {
    let case = TestCase {
        id: "adhoc.show-elided".into(),
        lojban: "mi klama".into(),
        dialect: None,
        translation_en: None,
        gloss_en: None,
        tags: vec![],
        provenance: vec![],
        expectations: Expectations {
            output: Some(OutputExpectations {
                gentufa: Some(GentufaOutputExpectation {
                    show_elided: Some(CommandOutputExpectation {
                        brackets: Some(TextExpectation { text: "()".into() }),
                        tree: Some(TextExpectation {
                            text: "tree".into(),
                        }),
                        json: Some(TextExpectation { text: "{}".into() }),
                    }),
                    ..GentufaOutputExpectation::default()
                }),
                ..OutputExpectations::default()
            }),
            ..Expectations::default()
        },
    };
    let facets = case.available_facets();
    assert!(facets.contains(&Facet::GentufaBracketsShowElided));
    assert!(facets.contains(&Facet::GentufaTreeShowElided));
    assert!(facets.contains(&Facet::GentufaJsonShowElided));
    assert_eq!(
        "gentufa-json-show-elided"
            .parse::<Facet>()
            .expect("show-elided facet"),
        Facet::GentufaJsonShowElided
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn available_facets_include_semantics_refs_expectations() {
    let case = TestCase {
        id: "adhoc.refs".into(),
        lojban: "mi klama do".into(),
        dialect: None,
        translation_en: None,
        gloss_en: None,
        tags: vec![],
        provenance: vec![],
        expectations: Expectations {
            semantics: Some(SemanticsExpectations {
                refs: Some(ReferenceExpectation {
                    status: ExpectationStatus::Success,
                    raw: Some(TextExpectation { text: "{}".into() }),
                }),
            }),
            ..Expectations::default()
        },
    };
    let facets = case.available_facets();
    assert!(facets.contains(&Facet::SemanticsRefs));
    assert_eq!(
        "semantics-refs".parse::<Facet>().expect("refs facet"),
        Facet::SemanticsRefs
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn available_facets_include_jvozba_expectations() {
    let case = TestCase {
        id: "adhoc.jvozba".into(),
        lojban: "fulta ismu".into(),
        dialect: None,
        translation_en: None,
        gloss_en: None,
        tags: vec![],
        provenance: vec![],
        expectations: Expectations {
            jvozba: Some(JvozbaExpectation {
                status: ExpectationStatus::Failure,
                mode: JvozbaFixtureMode::Lujvo,
                inputs: vec![
                    JvozbaFixtureInput::FixedRafsi {
                        text: "kerl".into(),
                    },
                    JvozbaFixtureInput::FixedRafsi { text: "u'u".into() },
                    JvozbaFixtureInput::Word {
                        text: "kerlo".into(),
                    },
                ],
                output: None,
                error: Some(TextExpectation {
                    text: "Could not build a valid lujvo from the supplied inputs.".into(),
                }),
            }),
            ..Expectations::default()
        },
    };
    let facets = case.available_facets();
    assert!(facets.contains(&Facet::Jvozba));
    assert_eq!(
        "jvozba".parse::<Facet>().expect("jvozba facet"),
        Facet::Jvozba
    );
    assert_eq!(Facet::Jvozba.to_string(), "jvozba");
}

#[test]
#[should_panic]
#[requires(true)]
#[ensures(true)]
fn write_fixture_rejects_invalid_metadata_by_contract() {
    let test_case = TestCase {
        id: String::new(),
        lojban: "coi".into(),
        dialect: None,
        translation_en: None,
        gloss_en: None,
        tags: vec![],
        provenance: vec![],
        expectations: Expectations::default(),
    };
    let fixture_path = temp_root("jbotci-invalid-fixture-contract").join("invalid.toml");
    let _ = write_fixture_file(fixture_path, &test_case);
}

#[requires(!test_case.id.is_empty())]
#[ensures(true)]
fn assert_morphology_expectation(test_case: &TestCase, expectation: &MorphologyExpectation) {
    let attempt =
        jbotci_morphology::segment_words_with_modifiers_with_options_and_source_id_attempt(
            &test_case.lojban,
            &jbotci_morphology::MorphologyOptions::default(),
            Some(SourceId("<fixture>".to_owned())),
        );
    let data = attempt.into_data();
    let mut diagnostics = data
        .warnings
        .iter()
        .map(|warning| {
            DiagnosticExpectation::from_diagnostic(
                &test_case.lojban,
                &warning.to_diagnostic(Some(SourceId("<fixture>".to_owned())), &test_case.lojban),
            )
        })
        .collect::<Vec<_>>();
    match (expectation.status, data.result) {
        (ExpectationStatus::Success, Ok(words)) => {
            if let Some(raw) = &expectation.raw {
                assert_eq!(format!("{words:?}"), raw.text, "{}", test_case.id);
            }
        }
        (ExpectationStatus::Failure, Err(error)) => {
            diagnostics.push(DiagnosticExpectation::from_diagnostic(
                &test_case.lojban,
                &error.to_diagnostic(Some(SourceId("<fixture>".to_owned())), &test_case.lojban),
            ));
        }
        (ExpectationStatus::Success, Err(error)) => {
            panic!("{} should parse, got {error}", test_case.id);
        }
        (ExpectationStatus::Failure, Ok(words)) => {
            panic!("{} should fail, got {words:?}", test_case.id);
        }
        (ExpectationStatus::Pending | ExpectationStatus::NotApplicable, _) => {
            panic!("{} has unsupported morphology status", test_case.id);
        }
    }
    assert_eq!(diagnostics, expectation.diagnostics, "{}", test_case.id);
}

#[requires(!id.is_empty())]
#[requires(true)]
#[ensures(true)]
fn assert_jvozba_expectation(id: &str, expectation: &JvozbaExpectation) {
    let inputs = expectation
        .inputs
        .iter()
        .map(to_jvozba_input)
        .collect::<Vec<_>>();
    let result = jbotci_jvozba::build_best_jvozba_detailed(
        to_jvozba_mode(expectation.mode),
        jbotci_dictionary_data::english(),
        &inputs,
    );
    match expectation.status {
        ExpectationStatus::Success => {
            let actual = result
                .unwrap_or_else(|error| panic!("jvozba fixture {id} should succeed, got {error}"));
            let expected = expectation
                .output
                .as_ref()
                .unwrap_or_else(|| panic!("jvozba fixture {id} missing output expectation"));
            assert_eq!(actual.word, expected.word, "{id}");
            assert_segments_match(id, &actual.segments, &expected.segments);
            assert_jvozba_output_parses_back(id, expectation.mode, expected);
        }
        ExpectationStatus::Failure => {
            let error = result.expect_err("jvozba fixture should fail").to_string();
            let expected = expectation
                .error
                .as_ref()
                .unwrap_or_else(|| panic!("jvozba fixture {id} missing error expectation"));
            assert_eq!(error, expected.text, "{id}");
        }
        ExpectationStatus::Pending | ExpectationStatus::NotApplicable => {
            panic!("jvozba fixture {id} has unsupported status");
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn to_jvozba_mode(mode: JvozbaFixtureMode) -> jbotci_jvozba::JvozbaMode {
    match mode {
        JvozbaFixtureMode::Lujvo => jbotci_jvozba::JvozbaMode::Lujvo,
        JvozbaFixtureMode::Cmevla => jbotci_jvozba::JvozbaMode::Cmevla,
    }
}

#[requires(true)]
#[ensures(true)]
fn to_jvozba_input(input: &JvozbaFixtureInput) -> jbotci_jvozba::JvozbaInput {
    match input {
        JvozbaFixtureInput::Word { text } => jbotci_jvozba::JvozbaInput::Word(text.clone()),
        JvozbaFixtureInput::FixedRafsi { text } => {
            jbotci_jvozba::JvozbaInput::FixedRafsi(text.clone())
        }
    }
}

#[requires(!id.is_empty())]
#[requires(true)]
#[ensures(true)]
fn assert_segments_match(
    id: &str,
    actual: &[jbotci_jvozba::JvozbaSegment],
    expected: &[JvozbaSegmentExpectation],
) {
    assert_eq!(actual.len(), expected.len(), "{id}: segment count");
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(
            to_fixture_segment_kind(actual.kind),
            expected.kind,
            "{id}: segment kind for {}",
            expected.text
        );
        assert_eq!(actual.text, expected.text, "{id}: segment text");
    }
}

#[requires(true)]
#[ensures(true)]
fn to_fixture_segment_kind(kind: jbotci_jvozba::JvozbaSegmentKind) -> JvozbaSegmentKindExpectation {
    match kind {
        jbotci_jvozba::JvozbaSegmentKind::Rafsi => JvozbaSegmentKindExpectation::Rafsi,
        jbotci_jvozba::JvozbaSegmentKind::Hyphen => JvozbaSegmentKindExpectation::Hyphen,
    }
}

#[requires(!id.is_empty())]
#[ensures(true)]
fn assert_jvozba_output_parses_back(
    id: &str,
    mode: JvozbaFixtureMode,
    expected: &JvozbaOutputExpectation,
) {
    let words = jbotci_morphology::segment_words_with_modifiers(&expected.word)
        .unwrap_or_else(|error| panic!("jvozba fixture {id} output did not parse: {error}"));
    let [word_like] = words.as_slice() else {
        panic!("jvozba fixture {id} output did not parse as one word");
    };
    let word = word_like
        .bare_word()
        .unwrap_or_else(|| panic!("jvozba fixture {id} output was not a bare word"));
    match mode {
        JvozbaFixtureMode::Lujvo => {
            assert_eq!(word.kind(), jbotci_morphology::WordKind::Lujvo, "{id}");
            let parts = word
                .lujvo_parts()
                .unwrap_or_else(|| panic!("jvozba fixture {id} output lacks lujvo parts"));
            assert_eq!(parts.len(), expected.segments.len(), "{id}");
            for (part, segment) in parts.iter().zip(&expected.segments) {
                assert!(
                    jbotci_morphology::canonical_text_eq(part.phonemes().as_str(), &segment.text),
                    "{id}: parsed part `{}` did not match expected `{}`",
                    part.phonemes().as_str(),
                    segment.text
                );
            }
        }
        JvozbaFixtureMode::Cmevla => {
            assert_eq!(word.kind(), jbotci_morphology::WordKind::Cmevla, "{id}");
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn loaded_case(path: &str, test_case: TestCase) -> LoadedTestCase {
    LoadedTestCase {
        path: PathBuf::from(path),
        test_case,
    }
}

#[requires(true)]
#[ensures(true)]
fn temp_root(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{}-{}",
        prefix,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ))
}
