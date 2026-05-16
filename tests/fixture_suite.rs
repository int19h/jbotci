mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use jbotci_morphology::{WordKind, WordLike, WordWithModifiers};
use jbotci_syntax::{SyntaxField, SyntaxValue};
use support::fixtures::{
    CllSelector, ExpectationStatus, Expectations, Facet, FacetResult, FixtureBackend,
    FixtureExport, FixtureSelector, LoadedTestCase, MorphologyExpectation, MuplisForm,
    OutputExpectation, Provenance, SyntaxExpectation, TestCase, TextExpectation, filter_fixtures,
    import_export_file, load_fixture_file, load_fixture_tree, run_fixture_facets,
    write_fixture_file,
};

#[test]
fn loads_smoke_fixture() {
    let fixture_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/adhoc/smoke/coi.toml");
    let test_case = load_fixture_file(fixture_path).expect("fixture should load");
    assert_eq!(test_case.id, "adhoc.smoke.coi");
    assert_eq!(test_case.lojban, "coi");
    assert!(test_case.tags.contains(&"smoke".to_owned()));
    assert_eq!(
        test_case
            .expectations
            .morphology
            .expect("morphology expectation")
            .words[0]
            .base_word_kind(),
        Some(WordKind::Cmavo)
    );
}

#[test]
fn profile_filters_cll_chapter_and_muplis_form() {
    let root = Path::new("tests/fixtures");
    let cll = loaded_case(
        "tests/fixtures/cll/chapter-18/section-18.3/c18e3d1.toml",
        TestCase {
            id: "cll.18.3.c18e3d1".into(),
            lojban: "coi".into(),
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
fn fake_runner_counts_failures() {
    struct FakeBackend;
    impl FixtureBackend for FakeBackend {
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
    assert_eq!(load_fixture_tree(&output).expect("fixtures").len(), 1);
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn writer_keeps_tree_and_words_as_values() {
    let temp_root = temp_root("jbotci-fixtures-writer-test");
    fs::create_dir_all(&temp_root).expect("temp root");
    let fixture_path = temp_root.join("fixture.toml");
    let word = WordWithModifiers::BaseWord {
        word_like: Box::new(WordLike::Bare {
            word: Box::new(jbotci_morphology::Word {
                kind: WordKind::Cmavo,
                phonemes: "coi".into(),
                span: jbotci_source_span(),
                surface_override: None,
                dialect_transform: None,
            }),
        }),
    };
    let test_case = TestCase {
        id: "adhoc.syntax".into(),
        lojban: "coi".into(),
        translation_en: None,
        gloss_en: None,
        tags: vec![],
        provenance: vec![],
        expectations: Expectations {
            output: Some(OutputExpectation {
                brackets: Some(TextExpectation {
                    text: "[coi]".into(),
                }),
            }),
            morphology: Some(MorphologyExpectation {
                status: ExpectationStatus::Success,
                words: vec![word],
                error: None,
            }),
            syntax: Some(SyntaxExpectation {
                status: ExpectationStatus::Success,
                parse_tree: Some(SyntaxValue::node(
                    "LojbanText",
                    vec![SyntaxField {
                        name: Some("paragraphs".into()),
                        value: SyntaxValue::List { items: vec![] },
                    }],
                )),
                error: None,
            }),
            ..Expectations::default()
        },
    };
    write_fixture_file(&fixture_path, &test_case).expect("write fixture");
    let text = fs::read_to_string(&fixture_path).expect("read fixture");
    assert!(text.contains("[expectations.output]\nbrackets = \"[coi]\""));
    assert!(text.contains("[expectations.morphology]\nstatus = \"success\"\nwords = ["));
    assert!(text.contains("[expectations.syntax]\nstatus = \"success\"\nparse-tree = {"));
    assert!(!text.contains("[expectations.morphology.words"));
    assert!(!text.contains("[expectations.syntax.parse-tree"));
    assert_eq!(
        load_fixture_file(&fixture_path).expect("load fixture"),
        test_case
    );
    let _ = fs::remove_dir_all(temp_root);
}

fn loaded_case(path: &str, test_case: TestCase) -> LoadedTestCase {
    LoadedTestCase {
        path: PathBuf::from(path),
        test_case,
    }
}

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

fn jbotci_source_span() -> jbotci_source::SourceSpan {
    jbotci_source::SourceSpan::new(None, 0, 3, 0, 3).expect("valid span")
}

trait WordWithModifiersExpectationExt {
    fn base_word_kind(&self) -> Option<WordKind>;
}

impl WordWithModifiersExpectationExt for WordWithModifiers {
    fn base_word_kind(&self) -> Option<WordKind> {
        match self {
            WordWithModifiers::BaseWord { word_like } => match word_like.as_ref() {
                WordLike::Bare { word } => Some(word.kind),
                _ => None,
            },
            _ => None,
        }
    }
}
