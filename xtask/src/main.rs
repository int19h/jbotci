use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitStatus};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result, bail};
use bityzba::{contract_trait, ensures, expensive_requires, invariant, requires};
use clap::{Args, Parser, Subcommand};
use jbotci_morphology::{
    MorphologyOptions, WordWithModifiers, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_output::pretty_brackets;
use jbotci_source::SourceId;
use jbotci_syntax::{
    ParseOptions, SyntaxError, parse_syntax_tree_with_source_and_options, syntax_values_equivalent,
};
use rayon::prelude::*;

#[path = "../../tests/support/fixtures/mod.rs"]
mod fixtures;
mod syntax_compare;

use fixtures::{
    ExpectationStatus, Facet, FacetResult, FixtureBackend, FixtureProfile, FixtureSelector,
    LoadedTestCase, MuplisForm, Provenance, RunSummary, fixture_matches_selector, fixture_paths,
    import_export_file, load_fixture_path, load_profile, validate_fixture_tree, visit_fixture_tree,
};
use syntax_compare::format_syntax_mismatch;

#[derive(Debug, Parser)]
#[command(name = "xtask")]
#[command(about = "Workspace automation for jbotci")]
#[invariant(true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
#[invariant(true)]
enum Command {
    Check,
    Test,
    Clippy,
    Fmt {
        #[arg(long)]
        check: bool,
    },
    FixtureCheck {
        #[arg(default_value = "tests/fixtures")]
        path: PathBuf,
    },
    FixtureImport(FixtureImportArgs),
    FixtureList(FixtureRunArgs),
    FixtureTest(FixtureRunArgs),
}

#[derive(Debug, Args)]
#[invariant(true)]
struct FixtureImportArgs {
    #[arg(long, default_value = ".jbotci-build/v0-fixtures/export.json")]
    input: PathBuf,
    #[arg(long, default_value = "tests/fixtures")]
    output: PathBuf,
    #[arg(long)]
    run_v0: bool,
    #[arg(long, default_value = "../jbotci.v0")]
    v0_root: PathBuf,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct FixtureRunArgs {
    #[arg(long, default_value = "tests/fixtures")]
    root: PathBuf,
    #[arg(long)]
    profile: Option<String>,
    #[arg(long = "facet")]
    facets: Vec<Facet>,
    #[arg(long = "provenance")]
    provenance: Vec<String>,
    #[arg(long = "tag")]
    tags: Vec<String>,
    #[arg(long = "id")]
    ids: Vec<String>,
    #[arg(long = "path-prefix")]
    path_prefixes: Vec<String>,
    #[arg(long = "cll-chapter")]
    cll_chapter: Option<u16>,
    #[arg(long = "cll-section")]
    cll_section: Option<String>,
    #[arg(long = "cll-example")]
    cll_example: Option<String>,
    #[arg(long = "muplis-collection")]
    muplis_collection: Option<String>,
    #[arg(long = "muplis-item")]
    muplis_item: Option<String>,
    #[arg(long = "muplis-form")]
    muplis_form: Option<MuplisForm>,
    #[arg(long, value_name = "N")]
    jobs: Option<usize>,
    #[arg(long, default_value_t = 0, value_name = "N")]
    failure_samples: usize,
}

#[requires(true)]
#[ensures(true)]
fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Check => cargo(&[
            "check",
            "--workspace",
            "--all-targets",
            "-j",
            DEFAULT_TEST_JOBS_TEXT,
        ]),
        Command::Test => cargo(&[
            "test",
            "--workspace",
            "--all-targets",
            "-j",
            DEFAULT_TEST_JOBS_TEXT,
            "--",
            "--test-threads",
            DEFAULT_TEST_JOBS_TEXT,
        ]),
        Command::Clippy => cargo(&[
            "clippy",
            "--workspace",
            "--all-targets",
            "-j",
            DEFAULT_TEST_JOBS_TEXT,
            "--",
            "-D",
            "warnings",
        ]),
        Command::Fmt { check } => {
            if check {
                cargo(&["fmt", "--all", "--", "--check"])
            } else {
                cargo(&["fmt", "--all"])
            }
        }
        Command::FixtureCheck { path } => {
            let summary = validate_fixture_tree(&path)
                .with_context(|| format!("checking fixtures under `{}`", path.display()))?;
            println!(
                "checked {} fixture(s), {} profile(s)",
                summary.fixture_count, summary.profile_count
            );
            Ok(())
        }
        Command::FixtureImport(args) => fixture_import(args),
        Command::FixtureList(args) => fixture_list(args),
        Command::FixtureTest(args) => fixture_test(args),
    }
}

#[requires(true)]
#[ensures(true)]
fn fixture_import(args: FixtureImportArgs) -> Result<()> {
    let input = absolute_path(&args.input)?;
    if args.run_v0 {
        run_v0_exporter(&args.v0_root, &input)?;
    }
    let summary = import_export_file(&input, &args.output).with_context(|| {
        format!(
            "importing `{}` into `{}`",
            input.display(),
            args.output.display()
        )
    })?;
    println!("imported {} fixture(s)", summary.written);
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn fixture_list(args: FixtureRunArgs) -> Result<()> {
    let profile = merged_profile(&args)?;
    visit_fixture_tree(&args.root, |fixture| {
        if fixture_matches_selector(&args.root, &fixture, &profile.selector) {
            if profile.facets.is_empty() {
                println!("{}\t{}", fixture.test_case.id, fixture.path.display());
            } else {
                for facet in &profile.facets {
                    println!(
                        "{}\t{}\t{}",
                        fixture.test_case.id,
                        facet,
                        fixture.path.display()
                    );
                }
            }
        }
        Ok(())
    })
    .with_context(|| format!("loading fixtures under `{}`", args.root.display()))?;
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn fixture_test(args: FixtureRunArgs) -> Result<()> {
    let profile = merged_profile(&args)?;
    let backend = NotImplementedBackend;
    let paths = fixture_paths(&args.root)
        .with_context(|| format!("listing fixtures under `{}`", args.root.display()))?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.jobs.unwrap_or_else(default_fixture_jobs))
        .stack_size(FIXTURE_WORKER_STACK_SIZE)
        .build()
        .context("creating fixture-test thread pool")?;
    let failure_counter = AtomicUsize::new(0);
    let mut summary = pool
        .install(|| {
            run_fixture_test_jobs(
                &args.root,
                &profile,
                &backend,
                &paths,
                args.failure_samples,
                &failure_counter,
            )
        })
        .with_context(|| format!("loading fixtures under `{}`", args.root.display()))?;
    summary.selected_facets = profile.facets.len();
    println!(
        "fixtures={}, facets={}, passed={}, xfailed={}, failed={}, skipped={}",
        summary.selected_fixtures,
        summary.selected_facets,
        summary.passed,
        summary.xfailed,
        summary.failed,
        summary.skipped
    );
    if summary.failed > 0 {
        bail!("fixture-test failed {} facet(s)", summary.failed);
    }
    Ok(())
}

#[requires(profile.is_valid())]
#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|summary| summary.total_results() == summary.selected_fixtures * profile.facets.len()))]
fn run_fixture_test_jobs<B: FixtureBackend + Sync>(
    root: &Path,
    profile: &FixtureProfile,
    backend: &B,
    paths: &[PathBuf],
    failure_samples: usize,
    failure_counter: &AtomicUsize,
) -> Result<RunSummary, fixtures::FixtureError> {
    paths
        .par_iter()
        .map(|path| {
            let fixture = load_fixture_path(path)?;
            let mut summary = RunSummary::default();
            if fixture_matches_selector(root, &fixture, &profile.selector) {
                summary.selected_fixtures = 1;
                for facet in &profile.facets {
                    let result = backend.run(&fixture, *facet);
                    if result.status == fixtures::FacetStatus::Failed {
                        let sample_index = failure_counter.fetch_add(1, Ordering::Relaxed);
                        if sample_index < failure_samples {
                            eprintln!(
                                "{}\t{}\t{}\t{}",
                                fixture.test_case.id,
                                facet,
                                fixture.path.display(),
                                result.message.as_deref().unwrap_or("failed")
                            );
                        }
                    }
                    summary.record_result(&result);
                }
            }
            Ok(summary)
        })
        .try_reduce(RunSummary::default, |mut left, right| {
            left.merge(right);
            Ok(left)
        })
}

#[ensures(ret > 0)]
#[requires(true)]
fn default_fixture_jobs() -> usize {
    DEFAULT_TEST_JOBS
}

// TOML fixtures can contain deeply nested exported syntax trees, and serde's
// TOML decoder needs more stack than Rayon workers get by default.
const FIXTURE_WORKER_STACK_SIZE: usize = 32 * 1024 * 1024;
const DEFAULT_TEST_JOBS: usize = 16;
const DEFAULT_TEST_JOBS_TEXT: &str = "16";

#[requires(true)]
#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(FixtureProfile::is_valid))]
fn merged_profile(args: &FixtureRunArgs) -> Result<FixtureProfile> {
    let mut profile = match &args.profile {
        Some(name) => load_profile(&args.root, name)
            .with_context(|| format!("loading fixture profile `{name}`"))?,
        None => FixtureProfile::default(),
    };
    merge_cli_selector(&mut profile.selector, args);
    if !args.facets.is_empty() {
        profile.facets = args.facets.clone();
    }
    Ok(profile)
}

#[requires(selector.is_valid())]
#[ensures(selector.is_valid())]
fn merge_cli_selector(selector: &mut FixtureSelector, args: &FixtureRunArgs) {
    selector.provenance.extend(args.provenance.clone());
    selector.tags.extend(args.tags.clone());
    selector.ids.extend(args.ids.clone());
    selector.path_prefixes.extend(args.path_prefixes.clone());
    if args.cll_chapter.is_some() || args.cll_section.is_some() || args.cll_example.is_some() {
        let mut cll = selector.cll.take().unwrap_or_default();
        if let Some(chapter) = args.cll_chapter {
            cll.chapter = Some(chapter);
        }
        if let Some(section) = &args.cll_section {
            cll.section_number = Some(section.clone());
        }
        if let Some(example) = &args.cll_example {
            if example.starts_with('c') {
                cll.example_id = Some(example.clone());
            } else {
                cll.example_number = Some(example.clone());
            }
        }
        selector.cll = Some(cll);
    }
    if args.muplis_collection.is_some() || args.muplis_item.is_some() || args.muplis_form.is_some()
    {
        let mut muplis = selector.muplis.take().unwrap_or_default();
        if let Some(collection) = &args.muplis_collection {
            muplis.collection_id = Some(collection.clone());
        }
        if let Some(item) = &args.muplis_item {
            muplis.item_id = Some(item.clone());
        }
        if let Some(form) = args.muplis_form {
            muplis.form = Some(form);
        }
        selector.muplis = Some(muplis);
    }
}

#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|path| path.is_absolute()))]
#[requires(true)]
fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .context("resolving current directory")?
            .join(path))
    }
}

#[requires(v0_root.is_absolute() || v0_root.components().next().is_some())]
#[requires(output.is_absolute() || output.components().next().is_some())]
#[ensures(true)]
fn run_v0_exporter(v0_root: &Path, output: &Path) -> Result<()> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating `{}`", parent.display()))?;
    }
    let status = ProcessCommand::new("cabal")
        .args([
            "--project-file=cabal.host.project",
            "run",
            "exe:v1-fixture-export",
            "--",
            "--output",
        ])
        .arg(output)
        .current_dir(v0_root)
        .status()
        .with_context(|| format!("failed to run v0 exporter in `{}`", v0_root.display()))?;
    check_status(status, "cabal run exe:v1-fixture-export")
}

#[requires(!args.is_empty(), "cargo subcommand arguments must not be empty")]
#[ensures(true)]
fn cargo(args: &[&str]) -> Result<()> {
    let status = ProcessCommand::new("cargo")
        .args(args)
        .status()
        .with_context(|| format!("failed to run `cargo {}`", args.join(" ")))?;
    check_status(status, &format!("cargo {}", args.join(" ")))
}

#[requires(!command.is_empty(), "checked command name must not be empty")]
#[ensures(true)]
fn check_status(status: ExitStatus, command: &str) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        bail!("`{command}` failed with status {status}")
    }
}

#[invariant(true)]
struct NotImplementedBackend;

#[contract_trait]
impl FixtureBackend for NotImplementedBackend {
    #[requires(true)]
    #[ensures(true)]
    fn run(&self, fixture: &LoadedTestCase, facet: Facet) -> FacetResult {
        let Some(status) = expectation_status(fixture, facet) else {
            return FacetResult::skipped(format!("fixture has no {facet} expectation"));
        };
        if matches!(
            status,
            ExpectationStatus::Pending | ExpectationStatus::NotApplicable
        ) {
            return FacetResult::skipped(format!("{facet} expectation is {status:?}"));
        }
        match facet {
            Facet::Morphology => run_morphology_fixture(fixture),
            Facet::Syntax => run_syntax_fixture(fixture),
            Facet::Brackets => run_brackets_fixture(fixture),
            Facet::SyntaxRefs | Facet::Warnings => {
                FacetResult::skipped(format!("{facet} runner is not implemented yet"))
            }
        }
    }
}

#[expensive_requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_brackets_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = fixture
        .test_case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.brackets.as_ref())
    else {
        return FacetResult::skipped("fixture has no brackets expectation");
    };
    let dialect = match fixture.test_case.dialect_definition() {
        Ok(dialect) => dialect,
        Err(error) => return FacetResult::failed(format!("dialect error: {error}")),
    };
    let options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
    let words = match segment_words_with_modifiers_with_options_and_source_id(
        &fixture.test_case.lojban,
        &options,
        Some(SourceId("<fixture>".to_owned())),
    ) {
        Ok(words) => words,
        Err(error) => return FacetResult::failed(format!("morphology error: {error}")),
    };
    let parsed = match parse_syntax_tree_with_source_and_options(
        &words,
        &fixture.test_case.lojban,
        &syntax_options,
    ) {
        Ok(parsed) => parsed,
        Err(error) => return FacetResult::failed(format!("syntax error: {error}")),
    };
    match pretty_brackets(&parsed.parse_tree, &fixture.test_case.lojban) {
        Ok(actual) if brackets_expectation_matches(fixture, &expectation.text, &actual) => {
            FacetResult::passed()
        }
        Ok(actual) => FacetResult::failed(format!(
            "brackets mismatch: expected `{}`, got `{actual}`",
            expectation.text
        )),
        Err(error) => FacetResult::failed(format!("brackets render error: {error}")),
    }
}

#[requires(true)]
#[ensures(true)]
fn brackets_expectation_matches(fixture: &LoadedTestCase, expected: &str, actual: &str) -> bool {
    if expected == actual {
        return true;
    }
    if !fixture_is_cll(fixture) {
        return false;
    }
    normalize_cll_brackets(expected) == normalize_cll_brackets(actual)
}

#[requires(true)]
#[ensures(true)]
fn fixture_is_cll(fixture: &LoadedTestCase) -> bool {
    fixture
        .test_case
        .provenance
        .iter()
        .any(|provenance| matches!(provenance, Provenance::Cll { .. }))
}

#[requires(true)]
#[ensures(true)]
fn normalize_cll_brackets(text: &str) -> String {
    text.chars()
        .filter_map(normalize_cll_bracket_char)
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn normalize_cll_bracket_char(ch: char) -> Option<char> {
    match ch {
        '.' | '-' | '\u{0306}' => None,
        'á' | 'à' | 'Á' | 'À' => Some('a'),
        'é' | 'è' | 'É' | 'È' => Some('e'),
        'í' | 'ì' | 'Í' | 'Ì' => Some('i'),
        'ó' | 'ò' | 'Ó' | 'Ò' => Some('o'),
        'ú' | 'ù' | 'Ú' | 'Ù' => Some('u'),
        'ý' | 'ỳ' | 'Ý' | 'Ỳ' => Some('y'),
        'ĭ' | 'Ĭ' => Some('i'),
        'ŭ' | 'Ŭ' => Some('u'),
        other => Some(other),
    }
}

#[expensive_requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_syntax_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = &fixture.test_case.expectations.syntax else {
        return FacetResult::skipped("fixture has no syntax expectation");
    };
    let dialect = match fixture.test_case.dialect_definition() {
        Ok(dialect) => dialect,
        Err(error) => return FacetResult::failed(format!("dialect error: {error}")),
    };
    let options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
    let words = match segment_words_with_modifiers_with_options_and_source_id(
        &fixture.test_case.lojban,
        &options,
        Some(SourceId("<fixture>".to_owned())),
    ) {
        Ok(words) => words,
        Err(error) => {
            return match expectation.status {
                ExpectationStatus::Failure => {
                    syntax_xfail_result(expectation, ExpectationStatus::Failure, true)
                        .unwrap_or_else(FacetResult::passed)
                }
                ExpectationStatus::Success => {
                    syntax_xfail_result(expectation, ExpectationStatus::Failure, true)
                        .unwrap_or_else(|| {
                            FacetResult::failed(format!(
                                "syntax blocked by morphology error: {error}"
                            ))
                        })
                }
                ExpectationStatus::Pending | ExpectationStatus::NotApplicable => {
                    FacetResult::skipped(format!("syntax expectation is {:?}", expectation.status))
                }
            };
        }
    };

    match parse_syntax_tree_with_source_and_options(
        &words,
        &fixture.test_case.lojban,
        &syntax_options,
    ) {
        Ok(parsed) => match expectation.status {
            ExpectationStatus::Success => {
                let Some(expected_tree) = &expectation.parse_tree else {
                    return FacetResult::failed("syntax success expectation has no parse-tree");
                };
                if syntax_values_equivalent(expected_tree, &parsed.parse_tree) {
                    syntax_xfail_result(expectation, ExpectationStatus::Success, true)
                        .unwrap_or_else(FacetResult::passed)
                } else if expectation.xfail.is_some()
                    && expectation
                        .xfail
                        .as_ref()
                        .is_some_and(|xfail| xfail.accepted_status == ExpectationStatus::Success)
                {
                    FacetResult::failed(
                        "syntax xfail accepted success, but parse-tree did not match",
                    )
                } else {
                    FacetResult::failed(format_syntax_mismatch(expected_tree, &parsed.parse_tree))
                }
            }
            ExpectationStatus::Failure => {
                if expectation
                    .xfail
                    .as_ref()
                    .is_some_and(|xfail| xfail.accepted_status == ExpectationStatus::Success)
                {
                    let Some(expected_tree) = &expectation.parse_tree else {
                        return FacetResult::failed("syntax success xfail has no parse-tree");
                    };
                    if syntax_values_equivalent(expected_tree, &parsed.parse_tree) {
                        syntax_xfail_result(expectation, ExpectationStatus::Success, true)
                            .unwrap_or_else(|| {
                                FacetResult::failed(
                                    "syntax xfail unexpectedly missing accepted success metadata",
                                )
                            })
                    } else {
                        FacetResult::failed(format!(
                            "syntax xfail accepted success, but {}",
                            format_syntax_mismatch(expected_tree, &parsed.parse_tree)
                        ))
                    }
                } else {
                    FacetResult::failed("expected syntax failure, got success")
                }
            }
            ExpectationStatus::Pending | ExpectationStatus::NotApplicable => {
                FacetResult::skipped(format!("syntax expectation is {:?}", expectation.status))
            }
        },
        Err(SyntaxError::NotImplemented) => {
            FacetResult::failed("syntax parser returned NotImplemented")
        }
        Err(SyntaxError::Parse {
            byte_offset,
            reason,
        }) => match expectation.status {
            ExpectationStatus::Success => {
                syntax_xfail_result(expectation, ExpectationStatus::Failure, true).unwrap_or_else(
                    || {
                        FacetResult::failed(format!(
                            "syntax parse error at byte {byte_offset}: {reason}"
                        ))
                    },
                )
            }
            ExpectationStatus::Failure => {
                syntax_xfail_result(expectation, ExpectationStatus::Failure, true)
                    .unwrap_or_else(FacetResult::passed)
            }
            ExpectationStatus::Pending | ExpectationStatus::NotApplicable => {
                FacetResult::skipped(format!("syntax expectation is {:?}", expectation.status))
            }
        },
    }
}

#[ensures(ret.as_ref().is_none_or(FacetResult::is_valid))]
#[requires(true)]
fn syntax_xfail_result(
    expectation: &fixtures::SyntaxExpectation,
    actual_status: ExpectationStatus,
    actual_matches_status_payload: bool,
) -> Option<FacetResult> {
    let xfail = expectation.xfail.as_ref()?;
    if actual_status == expectation.status && actual_matches_status_payload {
        return Some(FacetResult::failed(format!(
            "syntax xfail unexpectedly passed; remove xfail metadata. Reason was: {}",
            xfail.reason
        )));
    }
    if actual_status == xfail.accepted_status && actual_matches_status_payload {
        return Some(FacetResult::xfailed(format!(
            "{}: {}",
            xfail.source, xfail.reason
        )));
    }
    Some(FacetResult::failed(format!(
        "syntax xfail expected {:?}, got {:?}",
        xfail.accepted_status, actual_status
    )))
}

#[expensive_requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_morphology_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = &fixture.test_case.expectations.morphology else {
        return FacetResult::skipped("fixture has no morphology expectation");
    };
    match expectation.status {
        ExpectationStatus::Success => {
            let options = match fixture.test_case.dialect_definition() {
                Ok(dialect) => MorphologyOptions::default().with_dialect_definition(&dialect),
                Err(error) => return FacetResult::failed(format!("dialect error: {error}")),
            };
            match segment_words_with_modifiers_with_options_and_source_id(
                &fixture.test_case.lojban,
                &options,
                Some(SourceId("<fixture>".to_owned())),
            ) {
                Ok(actual) if actual == expectation.words => FacetResult::passed(),
                Ok(actual) => {
                    FacetResult::failed(format_morphology_mismatch(&expectation.words, &actual))
                }
                Err(error) => FacetResult::failed(format!("morphology error: {error}")),
            }
        }
        ExpectationStatus::Failure => {
            let options = match fixture.test_case.dialect_definition() {
                Ok(dialect) => MorphologyOptions::default().with_dialect_definition(&dialect),
                Err(error) => return FacetResult::failed(format!("dialect error: {error}")),
            };
            match segment_words_with_modifiers_with_options_and_source_id(
                &fixture.test_case.lojban,
                &options,
                Some(SourceId("<fixture>".to_owned())),
            ) {
                Ok(actual) => FacetResult::failed(format!(
                    "expected morphology failure, got {} word(s)",
                    actual.len()
                )),
                Err(_) => FacetResult::passed(),
            }
        }
        ExpectationStatus::Pending | ExpectationStatus::NotApplicable => FacetResult::skipped(
            format!("morphology expectation is {:?}", expectation.status),
        ),
    }
}

#[ensures(!ret.is_empty())]
#[requires(true)]
fn format_morphology_mismatch(
    expected: &[WordWithModifiers],
    actual: &[WordWithModifiers],
) -> String {
    let first_difference = expected
        .iter()
        .zip(actual.iter())
        .position(|(left, right)| left != right);
    match first_difference {
        Some(index) => {
            let expected_text = expected[index].to_string();
            let actual_text = actual[index].to_string();
            if expected_text == actual_text {
                format!(
                    "morphology mismatch at word {index}: expected {:#?}, got {:#?} (expected {} word(s), got {} word(s))",
                    expected[index],
                    actual[index],
                    expected.len(),
                    actual.len()
                )
            } else {
                format!(
                    "morphology mismatch at word {index}: expected `{expected_text}`, got `{actual_text}` (expected {} word(s), got {} word(s))",
                    expected.len(),
                    actual.len()
                )
            }
        }
        None => format!(
            "morphology mismatch: expected {} word(s), got {} word(s)",
            expected.len(),
            actual.len()
        ),
    }
}

#[ensures(ret.as_ref().is_none_or(|status| matches!(status, ExpectationStatus::Success | ExpectationStatus::Failure | ExpectationStatus::Pending | ExpectationStatus::NotApplicable)))]
#[requires(true)]
fn expectation_status(fixture: &LoadedTestCase, facet: Facet) -> Option<ExpectationStatus> {
    let expectations = &fixture.test_case.expectations;
    match facet {
        Facet::Morphology => expectations.morphology.as_ref().map(|value| value.status),
        Facet::Syntax => expectations.syntax.as_ref().map(|value| value.status),
        Facet::SyntaxRefs => expectations.syntax_refs.as_ref().map(|value| value.status),
        Facet::Warnings => expectations.warnings.as_ref().map(|value| value.status),
        Facet::Brackets => expectations
            .output
            .as_ref()
            .and_then(|output| output.brackets.as_ref())
            .map(|_| ExpectationStatus::Success),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::requires;

    #[test]
    #[should_panic(expected = "cargo subcommand arguments must not be empty")]
    #[requires(true)]
    #[ensures(true)]
    fn empty_cargo_command_contract_is_reported() {
        let _ = cargo(&[]);
    }
}
