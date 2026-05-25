mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, requires};
use jbotci_source::SourceId;
use support::fixtures::{
    CllSelector, CommandOutputExpectation, ExpectationStatus, Expectations, Facet, FacetResult,
    FixtureBackend, FixtureExport, FixtureSelector, LoadedTestCase, MorphologyExpectation,
    MuplisForm, OutputExpectations, Provenance, SyntaxExpectation, TestCase, TextExpectation,
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
    assert_eq!(value[0]["Bare"]["Cmavo"]["phonemes"], "coĭ");
    assert_eq!(value[0]["Bare"]["Cmavo"]["span"], serde_json::json!([0, 3]));
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
                vlasei: Some(CommandOutputExpectation {
                    json: Some(TextExpectation {
                        text: "[{\"Bare\":{\"Cmavo\":{\"phonemes\":\"coĭ\",\"span\":[0,3]}}}]"
                            .into(),
                    }),
                    ..CommandOutputExpectation::default()
                }),
                gentufa: Some(CommandOutputExpectation {
                    brackets: Some(TextExpectation {
                        text: "[coi]".into(),
                    }),
                    tree: Some(TextExpectation {
                        text: "\"coi\"".into(),
                    }),
                    json: Some(TextExpectation {
                        text: "{}".into(),
                    }),
                }),
            }),
            morphology: Some(MorphologyExpectation {
                status: ExpectationStatus::Success,
                raw: Some(TextExpectation {
                    text: "[WordLike(Bare(Word(Cmavo { phonemes: Phonemes(PhonemesData { text: \"coĭ\" }), span: SourceSpan(SourceSpanData { source_id: None, byte_start: 0, byte_end: 3, char_start: 0, char_end: 3, start: None, end: None }) })))]".into(),
                }),
                diagnostics: vec![],
            }),
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
                gentufa: Some(CommandOutputExpectation {
                    tree: Some(TextExpectation {
                        text: "\"coi\"".into(),
                    }),
                    ..CommandOutputExpectation::default()
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
