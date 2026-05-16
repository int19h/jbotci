use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitStatus};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use rayon::prelude::*;

#[path = "../../tests/support/fixtures.rs"]
mod fixtures;

use fixtures::{
    ExpectationStatus, Facet, FacetResult, FixtureBackend, FixtureProfile, FixtureSelector,
    LoadedTestCase, MuplisForm, RunSummary, fixture_matches_selector, fixture_paths,
    import_export_file, load_fixture_path, load_profile, validate_fixture_tree, visit_fixture_tree,
};

#[derive(Debug, Parser)]
#[command(name = "xtask")]
#[command(about = "Workspace automation for jbotci")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Check => cargo(&["check", "--workspace", "--all-targets"]),
        Command::Test => cargo(&["test", "--workspace", "--all-targets"]),
        Command::Clippy => cargo(&[
            "clippy",
            "--workspace",
            "--all-targets",
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

fn fixture_test(args: FixtureRunArgs) -> Result<()> {
    let profile = merged_profile(&args)?;
    let backend = NotImplementedBackend;
    let paths = fixture_paths(&args.root)
        .with_context(|| format!("listing fixtures under `{}`", args.root.display()))?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.jobs.unwrap_or_else(default_fixture_jobs))
        .build()
        .context("creating fixture-test thread pool")?;
    let mut summary = pool
        .install(|| run_fixture_test_jobs(&args.root, &profile, &backend, &paths))
        .with_context(|| format!("loading fixtures under `{}`", args.root.display()))?;
    summary.selected_facets = profile.facets.len();
    println!(
        "fixtures={}, facets={}, passed={}, failed={}, skipped={}",
        summary.selected_fixtures,
        summary.selected_facets,
        summary.passed,
        summary.failed,
        summary.skipped
    );
    if summary.failed > 0 {
        bail!("fixture-test failed {} facet(s)", summary.failed);
    }
    Ok(())
}

fn run_fixture_test_jobs<B: FixtureBackend + Sync>(
    root: &Path,
    profile: &FixtureProfile,
    backend: &B,
    paths: &[PathBuf],
) -> Result<RunSummary, fixtures::FixtureError> {
    paths
        .par_iter()
        .map(|path| {
            let fixture = load_fixture_path(path)?;
            let mut summary = RunSummary::default();
            if fixture_matches_selector(root, &fixture, &profile.selector) {
                summary.selected_fixtures = 1;
                for facet in &profile.facets {
                    summary.record_result(&backend.run(&fixture, *facet));
                }
            }
            Ok(summary)
        })
        .try_reduce(RunSummary::default, |mut left, right| {
            left.merge(right);
            Ok(left)
        })
}

fn default_fixture_jobs() -> usize {
    std::thread::available_parallelism().map_or(1, usize::from)
}

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

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .context("resolving current directory")?
            .join(path))
    }
}

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

fn cargo(args: &[&str]) -> Result<()> {
    let status = ProcessCommand::new("cargo")
        .args(args)
        .status()
        .with_context(|| format!("failed to run `cargo {}`", args.join(" ")))?;
    check_status(status, &format!("cargo {}", args.join(" ")))
}

fn check_status(status: ExitStatus, command: &str) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        bail!("`{command}` failed with status {status}")
    }
}

struct NotImplementedBackend;

impl FixtureBackend for NotImplementedBackend {
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
            Facet::Morphology | Facet::Syntax => {
                FacetResult::failed(format!("{facet} runner is not implemented yet"))
            }
            Facet::SyntaxRefs | Facet::Warnings | Facet::Brackets => {
                FacetResult::skipped(format!("{facet} runner is not implemented yet"))
            }
        }
    }
}

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
