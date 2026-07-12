mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use jbotci_diagnostics::Diagnostic;
use jbotci_source::SourceId;
use support::fixtures::{
    BracketExpectations, CllSelector, CommandOutputExpectation, DiagnosticExpectation,
    ExpectationStatus, Expectations, Facet, FacetResult, FixtureBackend, FixtureError,
    FixtureExport, FixtureSelector, GentufaOutputExpectation, JvozbaExpectation,
    JvozbaFixtureInput, JvozbaFixtureMode, JvozbaOutputExpectation, JvozbaSegmentExpectation,
    JvozbaSegmentKindExpectation, LoadedTestCase, MorphologyExpectation, MuplisForm,
    OutputExpectations, Provenance, RecoveredExpectation, RecoveredTreeExpectation,
    RecoveredTreeRecoveryItemExpectation, RecoveredTreeRecoveryItemKindExpectation,
    ReferenceExpectation, ScriptBracketExpectations, SemanticsExpectations, SyntaxExpectation,
    TersmuOutputExpectation, TestCase, TextExpectation, VlaseiOutputExpectation, XfailExpectation,
    filter_fixtures, fixture_paths, import_export_file, load_fixture_file, load_fixture_path,
    load_fixture_tree, run_fixture_facets, run_fixture_facets_parallel, validate_fixture_tree,
    write_fixture_file,
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
fn recovered_morphology_preserves_strict_first_error_for_failure_fixtures() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let fixtures = load_fixture_tree(&root).expect("fixtures should load");
    let mut checked = 0usize;
    let mut rendered_checked = 0usize;
    for fixture in fixtures {
        let Some(expectation) = &fixture.test_case.expectations.morphology else {
            continue;
        };
        if expectation.status != ExpectationStatus::Failure {
            continue;
        }
        let dialect = fixture
            .test_case
            .dialect_definition()
            .unwrap_or_else(|error| panic!("{} dialect error: {error}", fixture.test_case.id));
        let options =
            jbotci_morphology::MorphologyOptions::default().with_dialect_definition(&dialect);
        let source_id = Some(SourceId("<fixture>".to_owned()));
        let strict =
            jbotci_morphology::segment_words_with_modifiers_with_options_and_source_id_attempt(
                &fixture.test_case.lojban,
                &options,
                source_id.clone(),
            )
            .into_data();
        let strict_error = strict.result.unwrap_err();
        let recovered = jbotci_morphology::segment_words_with_modifiers_recovered_with_options_and_source_id_attempt(
            &fixture.test_case.lojban,
            &options,
            source_id.clone(),
        )
        .into_data()
        .result
        .into_data();
        assert_eq!(
            recovered.errors.first(),
            Some(&strict_error),
            "{}",
            fixture.test_case.id
        );
        let capped = jbotci_morphology::segment_words_with_modifiers_recovered_with_options_and_source_id_attempt(
            &fixture.test_case.lojban,
            &options.clone().with_max_recovery_errors(1),
            source_id.clone(),
        )
        .into_data()
        .result
        .into_data();
        let mut old_diagnostics = strict
            .warnings
            .iter()
            .map(|warning| warning.to_diagnostic(source_id.clone(), &fixture.test_case.lojban))
            .collect::<Vec<_>>();
        old_diagnostics
            .push(strict_error.to_diagnostic(source_id.clone(), &fixture.test_case.lojban));
        let mut new_diagnostics = capped
            .warnings
            .iter()
            .map(|warning| warning.to_diagnostic(source_id.clone(), &fixture.test_case.lojban))
            .collect::<Vec<_>>();
        new_diagnostics.extend(
            capped
                .errors
                .iter()
                .map(|error| error.to_diagnostic(source_id.clone(), &fixture.test_case.lojban)),
        );
        assert_eq!(
            render_fixture_diagnostics(&fixture.test_case.lojban, &new_diagnostics),
            render_fixture_diagnostics(&fixture.test_case.lojban, &old_diagnostics),
            "--max-errors 1 morphology stderr changed for fixture {}",
            fixture.test_case.id,
        );
        rendered_checked += 1;
        checked += 1;
    }
    assert!(checked > 0);
    assert!(rendered_checked > 0);
}

#[requires(true)]
#[ensures(diagnostics.is_empty() -> ret.is_empty())]
#[ensures(!diagnostics.is_empty() -> !ret.is_empty())]
fn render_fixture_diagnostics(source: &str, diagnostics: &[Diagnostic]) -> String {
    jbotci_output::render_diagnostics(
        "<fixture>",
        source,
        diagnostics,
        new!(jbotci_output::DiagnosticRenderOptions {
            color: false,
            detail: jbotci_output::DiagnosticDetailMode::Summary,
            glyphs: jbotci_output::GlyphStyle::Unicode,
            terminal_width: jbotci_output::DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
        }),
    )
    .expect("fixture diagnostics should render")
}

#[cfg(feature = "expensive_contracts")]
#[test]
#[requires(true)]
#[ensures(true)]
fn recovered_morphology_contracts_hold_for_fixture_corpus() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let fixtures = load_fixture_tree(&root).expect("fixtures should load");
    let mut checked = 0usize;
    for fixture in fixtures {
        let dialect = fixture
            .test_case
            .dialect_definition()
            .unwrap_or_else(|error| panic!("{} dialect error: {error}", fixture.test_case.id));
        let options =
            jbotci_morphology::MorphologyOptions::default().with_dialect_definition(&dialect);
        let _ =
            jbotci_morphology::segment_words_with_modifiers_recovered_with_options_and_source_id(
                &fixture.test_case.lojban,
                &options,
                Some(SourceId("<fixture>".to_owned())),
            );
        checked += 1;
    }
    assert!(checked > 0);
}

#[cfg(feature = "expensive_contracts")]
#[test]
#[requires(true)]
#[ensures(true)]
fn domain_import_marker_iff_holds_for_fixture_corpus() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let fixtures = load_fixture_tree(&root).expect("fixtures should load");
    let mut fixtures_checked = 0usize;
    let mut formula_nodes_checked = 0usize;
    let mut marked_nodes = 0usize;

    for fixture in fixtures {
        let Some(expectation) = fixture
            .test_case
            .expectations
            .output
            .as_ref()
            .and_then(|output| output.tersmu.as_ref())
            .filter(|expectation| expectation.status == ExpectationStatus::Success)
            .and_then(|expectation| expectation.json.as_ref())
        else {
            continue;
        };
        let graph: serde_json::Value = serde_json::from_str(&expectation.text)
            .unwrap_or_else(|error| panic!("{} tersmu JSON: {error}", fixture.test_case.id));
        let objects = graph["objects"]
            .as_object()
            .unwrap_or_else(|| panic!("{} tersmu objects", fixture.test_case.id));
        for (id, object) in objects {
            let Some(object) = object.as_object() else {
                panic!("{} object {id} is not a map", fixture.test_case.id);
            };
            let qualifies = object.get("type").and_then(serde_json::Value::as_str)
                == Some("formula")
                && matches!(
                    object.get("operator").and_then(serde_json::Value::as_str),
                    Some("forall" | "pluralForall")
                )
                && object.contains_key("restriction");
            let expected = qualifies.then_some("projective");
            assert_eq!(
                object
                    .get("domainImport")
                    .and_then(serde_json::Value::as_str),
                expected,
                "{} object {id} violates the domainImport iff rule",
                fixture.test_case.id,
            );
            if object.get("type").and_then(serde_json::Value::as_str) == Some("formula") {
                formula_nodes_checked += 1;
            }
            marked_nodes += usize::from(qualifies);
        }
        fixtures_checked += 1;
    }

    println!(
        "fixtures_checked={fixtures_checked} formula_nodes_checked={formula_nodes_checked} marked_nodes={marked_nodes}"
    );
    assert!(fixtures_checked > 0);
    assert!(formula_nodes_checked > 0);
    assert!(marked_nodes > 0);
}

#[cfg(feature = "expensive_contracts")]
#[test]
#[requires(true)]
#[ensures(true)]
fn recovered_syntax_contracts_hold_for_fixture_corpus() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let paths = fixture_paths(&root).expect("fixture paths should load");
    if let Some((start, end)) = recovered_syntax_contract_worker_range() {
        let checked = recovered_syntax_contract_fixture_range(&paths, start, end)
            .expect("fixture chunk should load");
        println!("checked={checked}");
        return;
    }

    let current_exe = std::env::current_exe().expect("current test binary path");
    let mut checked = 0usize;
    for start in (0..paths.len()).step_by(RECOVERED_SYNTAX_CONTRACT_CHUNK_SIZE) {
        let end = paths
            .len()
            .min(start + RECOVERED_SYNTAX_CONTRACT_CHUNK_SIZE);
        let output = Command::new(&current_exe)
            .arg("recovered_syntax_contracts_hold_for_fixture_corpus")
            .arg("--exact")
            .arg("--nocapture")
            .env("RECOVERED_SYNTAX_CONTRACT_START", start.to_string())
            .env("RECOVERED_SYNTAX_CONTRACT_END", end.to_string())
            .output()
            .expect("fixture chunk process should run");
        if !output.status.success() {
            panic!(
                "fixture chunk {start}..{end} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        checked += checked_count_from_test_stdout(&output.stdout);
    }
    assert!(checked > 0);
}

#[cfg(feature = "expensive_contracts")]
const RECOVERED_SYNTAX_CONTRACT_CHUNK_SIZE: usize = 1000;

#[cfg(feature = "expensive_contracts")]
#[requires(true)]
#[ensures(ret.is_none_or(|(start, end)| start <= end))]
fn recovered_syntax_contract_worker_range() -> Option<(usize, usize)> {
    let start = std::env::var("RECOVERED_SYNTAX_CONTRACT_START")
        .ok()?
        .parse()
        .ok()?;
    let end = std::env::var("RECOVERED_SYNTAX_CONTRACT_END")
        .ok()?
        .parse()
        .ok()?;
    (start <= end).then_some((start, end))
}

#[cfg(feature = "expensive_contracts")]
#[requires(start <= end)]
#[ensures(ret.as_ref().is_ok_and(|checked| *checked <= paths.len()) || ret.is_err())]
fn recovered_syntax_contract_fixture_range(
    paths: &[PathBuf],
    start: usize,
    end: usize,
) -> Result<usize, FixtureError> {
    let start = start.min(paths.len());
    let end = end.min(paths.len());
    let mut checked = 0usize;
    for path in &paths[start..end] {
        let fixture = load_fixture_path(path)?;
        let dialect = fixture
            .test_case
            .dialect_definition()
            .unwrap_or_else(|error| panic!("{} dialect error: {error}", fixture.test_case.id));
        let morphology_options =
            jbotci_morphology::MorphologyOptions::default().with_dialect_definition(&dialect);
        let syntax_options = jbotci_syntax::ParseOptions::default()
            .with_dialect_definition(&dialect)
            .with_max_recovery_errors(1);
        let Ok(words) = jbotci_morphology::segment_words_with_modifiers_with_options_and_source_id(
            &fixture.test_case.lojban,
            &morphology_options,
            Some(SourceId("<fixture>".to_owned())),
        ) else {
            continue;
        };
        let Ok(strict) = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            &fixture.test_case.lojban,
            &syntax_options,
        ) else {
            continue;
        };
        let strict = *strict;
        let recovered = jbotci_syntax::parse_syntax_tree_recovered_with_source_and_options(
            &words,
            &fixture.test_case.lojban,
            &syntax_options,
        )
        .into_data();
        assert!(
            recovered.errors.is_empty(),
            "{} strict parse succeeded but recovered API reported errors: {:?}",
            fixture.test_case.id,
            recovered.errors,
        );
        let valid = (*recovered.parse_tree).try_into_valid();
        assert_eq!(
            valid,
            Ok(strict),
            "{} zero-error recovered parse differs from strict parse",
            fixture.test_case.id,
        );
        checked += 1;
    }
    Ok(checked)
}

#[requires(true)]
#[ensures(true)]
fn checked_count_from_test_stdout(stdout: &[u8]) -> usize {
    let stdout = String::from_utf8_lossy(stdout);
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("checked=")?.parse().ok())
        .unwrap_or(0)
}

#[test]
#[requires(true)]
#[ensures(true)]
fn load_fixture_normalizes_crlf_storage_newlines() {
    let temp_root = temp_root("jbotci-fixture-crlf-load-test");
    fs::create_dir_all(&temp_root).expect("temp root");
    let fixture_path = temp_root.join("fixture.toml");
    fs::write(
        &fixture_path,
        "id = \"adhoc.crlf-load\"\r\nlojban = \"\"\"\r\ncoi\r\n.i do klama\"\"\"\r\n",
    )
    .expect("write fixture");

    let test_case = load_fixture_file(&fixture_path).expect("fixture should load");

    assert_eq!(test_case.id, "adhoc.crlf-load");
    assert_eq!(test_case.lojban, "coi\n.i do klama");
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn load_fixture_reads_external_lojban_filename() {
    let temp_root = temp_root("jbotci-fixture-external-source-test");
    let text_dir = temp_root.join("texts");
    fs::create_dir_all(&text_dir).expect("text dir");
    fs::write(text_dir.join("source.txt"), "coi\r\n.i do klama\r\n").expect("write source");
    let fixture_path = temp_root.join("fixture.toml");
    fs::write(
        &fixture_path,
        "id = \"adhoc.external-source\"\nlojban-filename = \"texts/source.txt\"\n",
    )
    .expect("write fixture");

    let test_case = load_fixture_file(&fixture_path).expect("fixture should load");

    assert_eq!(test_case.id, "adhoc.external-source");
    assert_eq!(test_case.lojban, "coi\n.i do klama\n");
    assert_eq!(
        test_case.lojban_filename.as_deref(),
        Some(Path::new("texts/source.txt"))
    );

    write_fixture_file(&fixture_path, &test_case).expect("rewrite fixture");
    let rewritten = fs::read_to_string(&fixture_path).expect("read rewritten fixture");
    assert!(rewritten.contains("lojban-filename = \"texts/source.txt\""));
    assert!(!rewritten.contains("lojban = "));
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn load_fixture_rejects_ambiguous_lojban_sources() {
    let temp_root = temp_root("jbotci-fixture-ambiguous-source-test");
    fs::create_dir_all(&temp_root).expect("temp root");
    let fixture_path = temp_root.join("fixture.toml");
    fs::write(
        &fixture_path,
        "id = \"adhoc.ambiguous-source\"\nlojban = \"coi\"\nlojban-filename = \"coi.txt\"\n",
    )
    .expect("write fixture");

    let error = load_fixture_file(&fixture_path).expect_err("fixture should fail");

    assert!(matches!(error, FixtureError::InvalidLojbanSource { .. }));
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn load_fixture_rejects_unsafe_lojban_filename() {
    let temp_root = temp_root("jbotci-fixture-unsafe-source-test");
    fs::create_dir_all(&temp_root).expect("temp root");
    let fixture_path = temp_root.join("fixture.toml");
    fs::write(
        &fixture_path,
        "id = \"adhoc.unsafe-source\"\nlojban-filename = \"../outside.txt\"\n",
    )
    .expect("write fixture");

    let error = load_fixture_file(&fixture_path).expect_err("fixture should fail");

    assert!(matches!(error, FixtureError::InvalidLojbanSource { .. }));
    let _ = fs::remove_dir_all(temp_root);
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
            lojban_filename: None,
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
            lojban_filename: None,
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
    let mut cll_selector_data = FixtureSelector::default().into_data();
    cll_selector_data.cll = Some(CllSelector {
        chapter: Some(18),
        example_id: Some("c18e3d1".into()),
        ..CllSelector::default()
    });
    let cll_selector = FixtureSelector::from_data(cll_selector_data);
    assert_eq!(filter_fixtures(root, &fixtures, &cll_selector).len(), 1);

    let mut muplis_selector_data = FixtureSelector::default().into_data();
    muplis_selector_data.muplis = Some(support::fixtures::MuplisSelector {
        collection_id: Some("18".into()),
        form: Some(MuplisForm::Front),
        ..support::fixtures::MuplisSelector::default()
    });
    let muplis_selector = FixtureSelector::from_data(muplis_selector_data);
    assert_eq!(filter_fixtures(root, &fixtures, &muplis_selector).len(), 1);

    let mut exact_selector_data = FixtureSelector::default().into_data();
    exact_selector_data
        .paths
        .push("muplis/collection-18/1-front.toml".to_owned());
    let exact_selector = FixtureSelector::from_data(exact_selector_data);
    let exact_matches = filter_fixtures(root, &fixtures, &exact_selector);
    assert_eq!(exact_matches.len(), 1);
    assert_eq!(exact_matches[0].test_case.id, "muplis.18.1.front");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn fixture_loader_ignores_legacy_markers_inside_strings() {
    let temp_root = temp_root("jbotci-fixture-legacy-string-test");
    fs::create_dir_all(&temp_root).expect("temp root");
    let fixture_path = temp_root.join("fixture.toml");
    fs::write(
        &fixture_path,
        r#"
id = "adhoc.legacy-string"
lojban = "coi"
translation-en = "This prose mentions constructor = and kind = \"node\"."

[[provenance]]
kind = "adhoc"

[expectations]
"#,
    )
    .expect("write fixture");

    let loaded = load_fixture_file(&fixture_path).expect("fixture should load");

    assert_eq!(loaded.id, "adhoc.legacy-string");
    fs::remove_dir_all(temp_root).unwrap();
}

#[test]
#[requires(true)]
#[ensures(true)]
fn fixture_loader_rejects_structural_legacy_expectation_keys() {
    let temp_root = temp_root("jbotci-fixture-legacy-key-test");
    fs::create_dir_all(&temp_root).expect("temp root");
    let fixture_path = temp_root.join("fixture.toml");
    fs::write(
        &fixture_path,
        r#"
id = "adhoc.legacy-key"
lojban = "coi"

[[provenance]]
kind = "adhoc"

[expectations.syntax.parse-tree]
kind = "node"
"#,
    )
    .expect("write fixture");

    let error = load_fixture_file(&fixture_path).expect_err("legacy fixture should fail");

    assert!(matches!(
        error,
        FixtureError::LegacyExpectationFormat { .. }
    ));
    fs::remove_dir_all(temp_root).unwrap();
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
            lojban_filename: None,
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
            lojban_filename: None,
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
            lojban_filename: None,
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
            lojban_filename: None,
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
fn morphology_matches_simple_cll_fixture_expectation() {
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
    let actual = jbotci_morphology::segment_words_with_modifiers_with_options_and_source_id(
        &test_case.lojban,
        &jbotci_morphology::MorphologyOptions::default(),
        Some(SourceId("<fixture>".to_owned())),
    )
    .expect("simple fixture should segment");
    assert_eq!(format!("{actual:?}"), expected);
}

#[cfg(not(debug_assertions))]
#[test]
#[requires(true)]
#[ensures(true)]
fn recovered_syntax_first_error_matches_strict_failure_fixtures() {
    run_on_fixture_worker_stack(recovered_syntax_first_error_matches_strict_failure_fixtures_inner);
}

#[requires(true)]
#[ensures(true)]
fn recovered_syntax_first_error_matches_strict_failure_fixtures_inner() {
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let paths = fixture_paths(&fixture_root).expect("fixture paths should load");
    if let Some((start, end)) = recovered_syntax_first_error_worker_range() {
        let checked = recovered_syntax_first_error_fixture_range(&paths, start, end)
            .expect("fixture chunk should load");
        println!("checked={checked}");
        return;
    }

    let current_exe = std::env::current_exe().expect("current test binary path");
    let mut checked = 0usize;
    for start in (0..paths.len()).step_by(RECOVERED_SYNTAX_FIRST_ERROR_CHUNK_SIZE) {
        let end = paths
            .len()
            .min(start + RECOVERED_SYNTAX_FIRST_ERROR_CHUNK_SIZE);
        let output = Command::new(&current_exe)
            .arg("recovered_syntax_first_error_matches_strict_failure_fixtures")
            .arg("--exact")
            .arg("--nocapture")
            .env("RECOVERED_SYNTAX_FIRST_ERROR_START", start.to_string())
            .env("RECOVERED_SYNTAX_FIRST_ERROR_END", end.to_string())
            .output()
            .expect("fixture chunk process should run");
        if !output.status.success() {
            panic!(
                "fixture chunk {start}..{end} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
        checked += checked_count_from_test_stdout(&output.stdout);
    }
    assert!(checked > 0, "expected at least one syntax-failure fixture");
}

const RECOVERED_SYNTAX_FIRST_ERROR_CHUNK_SIZE: usize = 1000;

#[requires(true)]
#[ensures(ret.is_none_or(|(start, end)| start <= end))]
fn recovered_syntax_first_error_worker_range() -> Option<(usize, usize)> {
    let start = std::env::var("RECOVERED_SYNTAX_FIRST_ERROR_START")
        .ok()?
        .parse()
        .ok()?;
    let end = std::env::var("RECOVERED_SYNTAX_FIRST_ERROR_END")
        .ok()?
        .parse()
        .ok()?;
    (start <= end).then_some((start, end))
}

#[requires(start <= end)]
#[ensures(ret.as_ref().is_ok_and(|checked| *checked <= paths.len()) || ret.is_err())]
fn recovered_syntax_first_error_fixture_range(
    paths: &[PathBuf],
    start: usize,
    end: usize,
) -> Result<usize, FixtureError> {
    let start = start.min(paths.len());
    let end = end.min(paths.len());
    let mut checked = 0usize;
    for path in &paths[start..end] {
        let fixture = load_fixture_path(path)?;
        let Some(expectation) = fixture.test_case.expectations.syntax.as_ref() else {
            continue;
        };
        if expectation.status != ExpectationStatus::Failure {
            continue;
        }
        let dialect = fixture
            .test_case
            .dialect_definition()
            .expect("fixture dialect should parse");
        let morphology_options =
            jbotci_morphology::MorphologyOptions::default().with_dialect_definition(&dialect);
        let source_id = Some(SourceId("<fixture>".to_owned()));
        let morphology =
            jbotci_morphology::segment_words_with_modifiers_with_options_and_source_id_attempt(
                &fixture.test_case.lojban,
                &morphology_options,
                source_id.clone(),
            )
            .into_data();
        let Ok(words) = morphology.result else {
            continue;
        };
        let syntax_options =
            jbotci_syntax::ParseOptions::default().with_dialect_definition(&dialect);
        let strict = match jbotci_syntax::parse_syntax_tree_with_source_and_options(
            &words,
            &fixture.test_case.lojban,
            &syntax_options,
        ) {
            Ok(_) => continue,
            Err(error) => error,
        };
        let capped_options = syntax_options.clone().with_max_recovery_errors(1);
        let recovered = jbotci_syntax::parse_syntax_tree_recovered_with_source_and_options(
            &words,
            &fixture.test_case.lojban,
            &capped_options,
        );
        assert_eq!(
            recovered.errors.first(),
            Some(&strict),
            "first recovered syntax error differs for fixture {}",
            fixture.test_case.id,
        );
        let mut old_diagnostics = morphology
            .warnings
            .iter()
            .map(|warning| warning.to_diagnostic(source_id.clone(), &fixture.test_case.lojban))
            .collect::<Vec<_>>();
        old_diagnostics.push(strict.to_diagnostic(source_id.clone(), &fixture.test_case.lojban));
        let mut new_diagnostics = morphology
            .warnings
            .iter()
            .map(|warning| warning.to_diagnostic(source_id.clone(), &fixture.test_case.lojban))
            .collect::<Vec<_>>();
        new_diagnostics.extend(
            recovered
                .errors
                .iter()
                .map(|error| error.to_diagnostic(source_id.clone(), &fixture.test_case.lojban)),
        );
        new_diagnostics.extend(
            recovered
                .warnings
                .iter()
                .map(|warning| warning.to_diagnostic(source_id.clone(), &fixture.test_case.lojban)),
        );
        assert_eq!(
            render_fixture_diagnostics(&fixture.test_case.lojban, &new_diagnostics),
            render_fixture_diagnostics(&fixture.test_case.lojban, &old_diagnostics),
            "--max-errors 1 syntax stderr changed for fixture {}",
            fixture.test_case.id,
        );
        checked += 1;
    }
    Ok(checked)
}

#[cfg(not(debug_assertions))]
#[test]
#[requires(true)]
#[ensures(true)]
fn recovered_syntax_first_error_matches_strict_with_default_recovery_cap() {
    for source in [
        "mi ku i do",
        "mi ku ni'o do",
        "mi cu ku",
        "le ku do",
        "mi viska lo",
        "lu mi ku i do li'u i mi klama",
        "mi ku i mi ku i mi klama",
    ] {
        assert_default_cap_recovered_syntax_first_error_matches_strict(source);
    }
}

#[requires(!source.is_empty())]
#[ensures(true)]
fn assert_default_cap_recovered_syntax_first_error_matches_strict(source: &str) {
    let words = jbotci_morphology::segment_words_with_modifiers_with_options_and_source_id(
        source,
        &jbotci_morphology::MorphologyOptions::default(),
        Some(SourceId("<fixture>".to_owned())),
    )
    .expect("default-cap first-error source has valid morphology");
    let options = jbotci_syntax::ParseOptions::default();
    let strict = jbotci_syntax::parse_syntax_tree_with_source_and_options(&words, source, &options)
        .expect_err("default-cap first-error source should fail strict syntax");
    let recovered = jbotci_syntax::parse_syntax_tree_recovered_with_source_and_options(
        &words, source, &options,
    );
    assert_eq!(
        recovered.errors.first(),
        Some(&strict),
        "first recovered syntax error differs for source {source:?}",
    );
}

#[cfg(not(debug_assertions))]
#[test]
#[requires(true)]
#[ensures(true)]
fn recovered_syntax_recovery_fixtures_match() {
    let fixture_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/adhoc/recovery/syntax");
    let fixtures = load_fixture_tree(&fixture_root).expect("syntax recovery fixtures");
    assert!(!fixtures.is_empty());
    let mut checked = 0usize;
    for fixture in fixtures {
        let Some(expectation) = fixture
            .test_case
            .expectations
            .syntax
            .as_ref()
            .and_then(|syntax| syntax.recovered.as_ref())
        else {
            continue;
        };
        assert_recovered_syntax_expectation(&fixture.test_case, expectation);
        checked += 1;
    }
    assert!(checked > 0, "expected recovered syntax expectations");
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
            lojban_filename: None,
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
        lojban_filename: None,
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
                        sha256: None,
                    }),
                    ..VlaseiOutputExpectation::default()
                }),
                gentufa: Some(GentufaOutputExpectation {
                    brackets: Some(TextExpectation {
                        text: "[coi]".into(),
                        sha256: None,
                    }),
                    tree: Some(TextExpectation {
                        text: "\"coi\"".into(),
                        sha256: None,
                    }),
                    json: Some(TextExpectation {
                        text: "{}".into(),
                        sha256: None,
                    }),
                    show_elided: None,
                }),
                tersmu: Some(TersmuOutputExpectation {
                    story_time: true,
                    json: Some(TextExpectation {
                        text: "{\"version\":\"lojban-semantics-json-1\"}".into(),
                        sha256: None,
                    }),
                    ..TersmuOutputExpectation::default()
                }),
            }),
            morphology: Some(MorphologyExpectation {
                status: ExpectationStatus::Success,
                raw: Some(TextExpectation {
                    text: "[WordLike(PlainWord(Word(Cmavo { phonemes: Phonemes(PhonemesData { text: \"coĭ\" }), span: SourceSpan(SourceSpanData { source_id: None, byte_start: 0, byte_end: 3, char_start: 0, char_end: 3, start: None, end: None }) })))]".into(),
                    sha256: None,
                }),
                diagnostics: vec![],
                recovered: None,
            }),
            jvozba: None,
            syntax: Some(SyntaxExpectation {
                status: ExpectationStatus::Success,
                raw: Some(TextExpectation {
                    text: "TextSyntax { leading_nai: [], leading_cmevla: [], leading_indicators: [], leading_free_modifiers: [], leading_connective: None, paragraphs: [] }".into(),
                    sha256: None,
                }),
                diagnostics: vec![],
                recovered: Some(new!(RecoveredExpectation {
                    status: ExpectationStatus::Success,
                    max_errors: None,
                    diagnostics: vec![],
                    tree: None,
                })),
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
                        sha256: None,
                    }),
                    error: None,
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
    assert!(text.contains("[expectations.output.tersmu]\nstory-time = true\njson = "));
    assert!(text.contains("tree = '\"coi\"'"));
    assert!(text.contains("[expectations.morphology]\nstatus = \"success\"\nraw = "));
    assert!(!text.contains("words = ["));
    assert!(!text.contains("options = "));
    assert!(text.contains("[expectations.syntax]\nstatus = \"success\"\nraw = "));
    assert!(text.contains("[expectations.syntax.recovered]\nstatus = \"success\""));
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
        lojban_filename: None,
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
                            sha256: None,
                        }),
                        cyrillic: Some(TextExpectation {
                            text: "(ми кла́ма)".into(),
                            sha256: None,
                        }),
                        zbalermorna: Some(TextExpectation {
                            text: "zbal".into(),
                            sha256: None,
                        }),
                    })),
                    ..VlaseiOutputExpectation::default()
                }),
                gentufa: Some(GentufaOutputExpectation {
                    show_elided: Some(CommandOutputExpectation {
                        brackets: Some(TextExpectation {
                            text: "(mi kláma vau)".into(),
                            sha256: None,
                        }),
                        tree: Some(TextExpectation {
                            text: "tree".into(),
                            sha256: None,
                        }),
                        json: Some(TextExpectation {
                            text: "{}".into(),
                            sha256: None,
                        }),
                    }),
                    ..GentufaOutputExpectation::default()
                }),
                ..OutputExpectations::default()
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
        lojban_filename: None,
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
        lojban_filename: None,
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
                        sha256: None,
                    }),
                    ..GentufaOutputExpectation::default()
                }),
                tersmu: Some(TersmuOutputExpectation {
                    json: Some(TextExpectation {
                        text: "{\"version\":\"lojban-semantics-json-1\"}".into(),
                        sha256: None,
                    }),
                    ..TersmuOutputExpectation::default()
                }),
                ..OutputExpectations::default()
            }),
            ..Expectations::default()
        },
    };
    let facets = case.available_facets();
    assert!(facets.contains(&Facet::GentufaTree));
    assert!(facets.contains(&Facet::TersmuJson));
    assert!(!facets.contains(&Facet::GentufaBrackets));
    assert_eq!(
        "gentufa-tree".parse::<Facet>().expect("tree facet"),
        Facet::GentufaTree
    );
    assert_eq!(
        "tersmu-json".parse::<Facet>().expect("tersmu facet"),
        Facet::TersmuJson
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
        lojban_filename: None,
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
                            sha256: None,
                        }),
                        cyrillic: Some(TextExpectation {
                            text: "(ми кла́ма)".into(),
                            sha256: None,
                        }),
                        zbalermorna: Some(TextExpectation {
                            text: "zbal".into(),
                            sha256: None,
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
        lojban_filename: None,
        dialect: None,
        translation_en: None,
        gloss_en: None,
        tags: vec![],
        provenance: vec![],
        expectations: Expectations {
            output: Some(OutputExpectations {
                gentufa: Some(GentufaOutputExpectation {
                    show_elided: Some(CommandOutputExpectation {
                        brackets: Some(TextExpectation {
                            text: "()".into(),
                            sha256: None,
                        }),
                        tree: Some(TextExpectation {
                            text: "tree".into(),
                            sha256: None,
                        }),
                        json: Some(TextExpectation {
                            text: "{}".into(),
                            sha256: None,
                        }),
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
        lojban_filename: None,
        dialect: None,
        translation_en: None,
        gloss_en: None,
        tags: vec![],
        provenance: vec![],
        expectations: Expectations {
            semantics: Some(SemanticsExpectations {
                refs: Some(ReferenceExpectation {
                    status: ExpectationStatus::Success,
                    raw: Some(TextExpectation {
                        text: "{}".into(),
                        sha256: None,
                    }),
                    error: None,
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
        lojban_filename: None,
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
                    sha256: None,
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
        lojban_filename: None,
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
    let dialect = test_case
        .dialect_definition()
        .unwrap_or_else(|error| panic!("{} dialect error: {error}", test_case.id));
    let options = jbotci_morphology::MorphologyOptions::default().with_dialect_definition(&dialect);
    let attempt =
        jbotci_morphology::segment_words_with_modifiers_with_options_and_source_id_attempt(
            &test_case.lojban,
            &options,
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
    if let Some(recovered) = &expectation.recovered {
        assert_recovered_morphology_expectation(test_case, recovered);
    }
}

#[requires(!test_case.id.is_empty())]
#[ensures(true)]
fn assert_recovered_morphology_expectation(
    test_case: &TestCase,
    expectation: &RecoveredExpectation,
) {
    let dialect = test_case
        .dialect_definition()
        .unwrap_or_else(|error| panic!("{} dialect error: {error}", test_case.id));
    let options = jbotci_morphology::MorphologyOptions::default().with_dialect_definition(&dialect);
    let attempt =
        jbotci_morphology::segment_words_with_modifiers_recovered_with_options_and_source_id(
            &test_case.lojban,
            &options,
            Some(SourceId("<fixture>".to_owned())),
        );
    let actual_status = if attempt.errors.is_empty() {
        ExpectationStatus::Success
    } else {
        ExpectationStatus::Failure
    };
    assert_eq!(actual_status, expectation.status, "{}", test_case.id);
    let diagnostics = recovered_morphology_diagnostics(test_case, &attempt);
    assert_eq!(diagnostics, expectation.diagnostics, "{}", test_case.id);
}

#[requires(true)]
#[ensures(true)]
fn recovered_syntax_tree_expectation(
    recovered: &jbotci_syntax::RecoveredSyntaxParse,
) -> RecoveredTreeExpectation {
    let mut visitor = RecoveredSyntaxTreeExpectationVisitor::default();
    jbotci_syntax::generated_model::recovered::TreeNode::visit_in_order(
        recovered.parse_tree.as_ref(),
        &mut visitor,
    );
    new!(RecoveredTreeExpectation {
        valid_tokens: visitor.valid_tokens,
        recovery_items: visitor.recovery_items,
    })
}

#[derive(Default)]
#[invariant(true)]
struct RecoveredSyntaxTreeExpectationVisitor {
    valid_tokens: Vec<String>,
    recovery_items: Vec<RecoveredTreeRecoveryItemExpectation>,
}

impl<'tree> jbotci_tree::TreeVisitor<'tree> for RecoveredSyntaxTreeExpectationVisitor {
    type Node = jbotci_syntax::generated_model::recovered::NodeRef<'tree>;
    type Atom = jbotci_syntax::generated_model::recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let jbotci_syntax::generated_model::recovered::AtomRef::Token(token) = atom;
        let token = token.core_word().to_string();
        let token = token
            .split_once(':')
            .map_or(token.as_str(), |(_kind, text)| text)
            .to_owned();
        self.valid_tokens.push(token);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E>(&mut self, item: &'tree E)
    where
        E: jbotci_tree::RecoveryItemState + serde::Serialize,
    {
        let mut byte_spans = Vec::new();
        item.visit_source_spans(&mut |span| {
            byte_spans.push([span.byte_start, span.byte_end]);
        });
        self.recovery_items
            .push(new!(RecoveredTreeRecoveryItemExpectation {
                kind: recovered_tree_item_kind(item.recovery_item_kind()),
                error_index: item.recovery_error_index(),
                byte_spans,
            }));
    }
}

#[requires(true)]
#[ensures(true)]
fn recovered_tree_item_kind(
    kind: jbotci_tree::RecoveryItemKind,
) -> RecoveredTreeRecoveryItemKindExpectation {
    match kind {
        jbotci_tree::RecoveryItemKind::Invalid => RecoveredTreeRecoveryItemKindExpectation::Invalid,
        jbotci_tree::RecoveryItemKind::Missing => RecoveredTreeRecoveryItemKindExpectation::Missing,
    }
}

#[requires(!test_case.id.is_empty())]
#[ensures(true)]
fn recovered_morphology_diagnostics(
    test_case: &TestCase,
    recovered: &jbotci_morphology::RecoveredMorphologySegmentation,
) -> Vec<DiagnosticExpectation> {
    let mut diagnostics = recovered
        .warnings
        .iter()
        .map(|warning| {
            DiagnosticExpectation::from_diagnostic(
                &test_case.lojban,
                &warning.to_diagnostic(Some(SourceId("<fixture>".to_owned())), &test_case.lojban),
            )
        })
        .chain(recovered.errors.iter().map(|error| {
            DiagnosticExpectation::from_diagnostic(
                &test_case.lojban,
                &error.to_diagnostic(Some(SourceId("<fixture>".to_owned())), &test_case.lojban),
            )
        }))
        .collect::<Vec<_>>();
    diagnostics.sort_by_key(|diagnostic| {
        (
            diagnostic.byte_span[0],
            diagnostic.byte_span[1],
            diagnostic.code.clone(),
        )
    });
    diagnostics
}

#[requires(!test_case.id.is_empty())]
#[ensures(true)]
fn assert_recovered_syntax_expectation(test_case: &TestCase, expectation: &RecoveredExpectation) {
    let dialect = test_case
        .dialect_definition()
        .unwrap_or_else(|error| panic!("{} dialect error: {error}", test_case.id));
    let morphology_options =
        jbotci_morphology::MorphologyOptions::default().with_dialect_definition(&dialect);
    let mut syntax_options =
        jbotci_syntax::ParseOptions::default().with_dialect_definition(&dialect);
    if let Some(max_errors) = expectation.max_errors {
        syntax_options = syntax_options.with_max_recovery_errors(max_errors);
    }
    let attempt =
        jbotci_morphology::segment_words_with_modifiers_with_options_and_source_id_attempt(
            &test_case.lojban,
            &morphology_options,
            Some(SourceId("<fixture>".to_owned())),
        )
        .into_data();
    let words = attempt
        .result
        .unwrap_or_else(|error| panic!("{} morphology should parse: {error}", test_case.id));
    let recovered = jbotci_syntax::parse_syntax_tree_recovered_with_source_and_options(
        &words,
        &test_case.lojban,
        &syntax_options,
    );
    let actual_status = if recovered.errors.is_empty() {
        ExpectationStatus::Success
    } else {
        ExpectationStatus::Failure
    };
    assert_eq!(actual_status, expectation.status, "{}", test_case.id);
    let mut diagnostics =
        morphology_warning_diagnostics_from_warnings(test_case, &attempt.warnings);
    diagnostics.extend(recovered_syntax_diagnostics(test_case, &recovered));
    diagnostics.sort_by_key(|diagnostic| {
        (
            diagnostic.byte_span[0],
            diagnostic.byte_span[1],
            diagnostic.code.clone(),
        )
    });
    assert_eq!(diagnostics, expectation.diagnostics, "{}", test_case.id);
    if let Some(expected_tree) = &expectation.tree {
        assert_eq!(
            recovered_syntax_tree_expectation(&recovered),
            *expected_tree,
            "{}",
            test_case.id
        );
    }
}

#[requires(!test_case.id.is_empty())]
#[ensures(true)]
fn morphology_warning_diagnostics_from_warnings(
    test_case: &TestCase,
    warnings: &[jbotci_morphology::MorphologyWarning],
) -> Vec<DiagnosticExpectation> {
    warnings
        .iter()
        .map(|warning| {
            DiagnosticExpectation::from_diagnostic(
                &test_case.lojban,
                &warning.to_diagnostic(Some(SourceId("<fixture>".to_owned())), &test_case.lojban),
            )
        })
        .collect()
}

#[requires(!test_case.id.is_empty())]
#[ensures(true)]
fn recovered_syntax_diagnostics(
    test_case: &TestCase,
    recovered: &jbotci_syntax::RecoveredSyntaxParse,
) -> Vec<DiagnosticExpectation> {
    recovered
        .warnings
        .iter()
        .map(|warning| {
            DiagnosticExpectation::from_diagnostic(
                &test_case.lojban,
                &warning.to_diagnostic(Some(SourceId("<fixture>".to_owned())), &test_case.lojban),
            )
        })
        .chain(recovered.errors.iter().map(|error| {
            DiagnosticExpectation::from_diagnostic(
                &test_case.lojban,
                &error.to_diagnostic(Some(SourceId("<fixture>".to_owned())), &test_case.lojban),
            )
        }))
        .collect()
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

#[requires(true)]
#[ensures(true)]
fn run_on_fixture_worker_stack(test: impl FnOnce() + Send + 'static) {
    let handle = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(test)
        .expect("fixture worker stack test thread should spawn");
    if let Err(panic) = handle.join() {
        std::panic::resume_unwind(panic);
    }
}
