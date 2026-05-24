use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitStatus};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result, bail};
use bityzba::{contract_trait, ensures, invariant, requires};
use clap::{Args, Parser, Subcommand};
use jbotci_dictionary::import::parse_lensisku_json;
use jbotci_morphology::{
    MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_output::{
    BracketRenderOptions, JsonRenderOptions, TreeRenderOptions,
    compact_morphology_json_string_with_options, compact_syntax_json_string_with_options,
    pretty_brackets, pretty_morphology_brackets_with_options, pretty_morphology_tree_with_options,
    pretty_tree_with_options,
};
use jbotci_source::SourceId;
use jbotci_syntax::{
    ParseOptions, SyntaxError, SyntaxWarning, parse_syntax_tree_with_source_and_options,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[path = "../../tests/support/fixtures/mod.rs"]
mod fixtures;

use fixtures::{
    ExpectationStatus, Facet, FacetResult, FixtureBackend, FixtureProfile, FixtureSelector,
    LoadedTestCase, MuplisForm, Provenance, RunSummary, fixture_matches_selector, fixture_paths,
    import_export_file, load_fixture_path, load_profile, validate_fixture_tree, visit_fixture_tree,
    write_fixture_file,
};

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
#[invariant(::Fmt => true)]
#[invariant(::FixtureCheck => true)]
#[invariant(::FixtureImport(..) => true)]
#[invariant(::FixtureList(..) => true)]
#[invariant(::FixtureRewrite(..) => true)]
#[invariant(::FixtureVectorStats(..) => true)]
#[invariant(::FixtureTest(..) => true)]
#[invariant(::VendorDictionary(..) => true)]
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
    FixtureRewrite(FixtureRewriteArgs),
    FixtureVectorStats(FixtureVectorStatsArgs),
    FixtureTest(FixtureRunArgs),
    VendorDictionary(VendorDictionaryArgs),
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
    #[arg(long, hide = true)]
    chunk_worker: bool,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct FixtureRewriteArgs {
    #[arg(default_value = "tests/fixtures")]
    roots: Vec<PathBuf>,
    #[arg(long, hide = true)]
    chunk_worker: bool,
    #[arg(long = "path", hide = true)]
    paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct FixtureVectorStatsArgs {
    #[arg(long, default_value = "tests/fixtures")]
    root: PathBuf,
    #[arg(long, value_name = "N")]
    jobs: Option<usize>,
    #[arg(long, default_value_t = 1, value_name = "N")]
    min_count: usize,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct VendorDictionaryArgs {
    #[arg(long, default_value = "https://lensisku.lojban.org")]
    base_url: String,
    #[arg(long, default_value = "en")]
    language: String,
    #[arg(long, default_value = "json")]
    format: String,
    #[arg(long, default_value = "vendor/lensisku")]
    output: PathBuf,
    #[arg(long)]
    check: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[invariant(true)]
struct CachedExport {
    language_tag: String,
    language_realname: String,
    format: String,
    filename: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[invariant(true)]
struct DictionaryMetadata<'a> {
    language_tag: &'a str,
    language_realname: &'a str,
    format: &'a str,
    filename: &'a str,
    metadata_url: &'a str,
    download_url: &'a str,
    lensisku_created_at: &'a str,
    sha256: &'a str,
    entry_count: usize,
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
        Command::FixtureRewrite(args) => fixture_rewrite(args),
        Command::FixtureVectorStats(args) => fixture_vector_stats(args),
        Command::FixtureTest(args) => fixture_test(args),
        Command::VendorDictionary(args) => vendor_dictionary(args),
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
fn vendor_dictionary(args: VendorDictionaryArgs) -> Result<()> {
    let base_url = args.base_url.trim_end_matches('/').to_owned();
    let metadata_url = format!("{base_url}/api/export/cached");
    let exports_text = fetch_text(&metadata_url)
        .with_context(|| format!("fetching Lensisku export metadata from `{metadata_url}`"))?;
    let exports = serde_json::from_str::<Vec<CachedExport>>(&exports_text)
        .with_context(|| format!("parsing Lensisku export metadata from `{metadata_url}`"))?;
    let export = exports
        .iter()
        .find(|export| export.language_tag == args.language && export.format == args.format)
        .cloned()
        .with_context(|| {
            format!(
                "finding cached Lensisku export for language `{}` and format `{}`",
                args.language, args.format
            )
        })?;

    let download_url = format!(
        "{base_url}/api/export/cached/{}/{}",
        export.language_tag, export.format
    );
    let dictionary_text = fetch_text(&download_url)
        .with_context(|| format!("fetching Lensisku dictionary from `{download_url}`"))?;
    let imported = parse_lensisku_json(&dictionary_text)
        .with_context(|| format!("validating Lensisku dictionary from `{download_url}`"))?;
    let pretty_json = pretty_json(&dictionary_text)
        .with_context(|| format!("pretty-printing Lensisku dictionary from `{download_url}`"))?;
    let sha256 = sha256_hex(pretty_json.as_bytes());
    let metadata = DictionaryMetadata {
        language_tag: &export.language_tag,
        language_realname: &export.language_realname,
        format: &export.format,
        filename: &export.filename,
        metadata_url: &metadata_url,
        download_url: &download_url,
        lensisku_created_at: &export.created_at,
        sha256: &sha256,
        entry_count: imported.entries.len(),
    };
    let metadata_text =
        toml::to_string_pretty(&metadata).context("rendering dictionary metadata")?;

    if args.check {
        println!(
            "validated {} Lensisku entries from {} created at {}",
            imported.entries.len(),
            download_url,
            export.created_at
        );
        return Ok(());
    }

    fs::create_dir_all(&args.output)
        .with_context(|| format!("creating `{}`", args.output.display()))?;
    let dictionary_path = args.output.join(&export.filename);
    let metadata_path = args.output.join("dictionary-en.metadata.toml");
    fs::write(&dictionary_path, pretty_json)
        .with_context(|| format!("writing `{}`", dictionary_path.display()))?;
    fs::write(&metadata_path, metadata_text)
        .with_context(|| format!("writing `{}`", metadata_path.display()))?;
    println!(
        "vendored {} Lensisku entries into `{}`",
        imported.entries.len(),
        dictionary_path.display()
    );
    Ok(())
}

#[requires(!url.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
fn fetch_text(url: &str) -> Result<String> {
    let text = ureq::get(url)
        .call()
        .with_context(|| format!("GET `{url}`"))?
        .body_mut()
        .with_config()
        .limit(64 * 1024 * 1024)
        .read_to_string()
        .with_context(|| format!("reading response body from `{url}`"))?;
    Ok(text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.ends_with('\n')))]
fn pretty_json(input: &str) -> Result<String> {
    let value = serde_json::from_str::<serde_json::Value>(input)?;
    let mut text = serde_json::to_string_pretty(&value)?;
    text.push('\n');
    Ok(text)
}

#[requires(true)]
#[ensures(ret.len() == 64)]
fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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
fn fixture_rewrite(args: FixtureRewriteArgs) -> Result<()> {
    let handle = std::thread::Builder::new()
        .stack_size(FIXTURE_WORKER_STACK_SIZE)
        .spawn(move || fixture_rewrite_inner(args))
        .context("spawning fixture-rewrite worker")?;
    match handle.join() {
        Ok(result) => result,
        Err(_) => bail!("fixture-rewrite worker panicked"),
    }
}

#[requires(true)]
#[ensures(true)]
fn fixture_rewrite_inner(args: FixtureRewriteArgs) -> Result<()> {
    if args.chunk_worker {
        let summary = fixture_rewrite_paths(args.paths, false)?;
        println!(
            "fixtures={}, rewritten={}",
            summary.processed, summary.rewritten
        );
        return Ok(());
    }
    fixture_rewrite_subprocess_chunks(args.roots)
}

#[requires(true)]
#[ensures(true)]
fn fixture_rewrite_subprocess_chunks(roots: Vec<PathBuf>) -> Result<()> {
    let mut paths = Vec::new();
    for root in &roots {
        paths.extend(
            fixture_paths(root)
                .with_context(|| format!("listing fixtures under `{}`", root.display()))?,
        );
    }
    let total = paths.len();
    let exe = std::env::current_exe().context("resolving xtask executable")?;
    let mut summary = RewriteSummary::default();
    for chunk in paths.chunks(FIXTURE_REWRITE_SUBPROCESS_CHUNK_SIZE) {
        let output = fixture_rewrite_chunk_output(&exe, chunk)?;
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let chunk_summary = parse_fixture_rewrite_summary(&stdout)?;
        summary.merge(chunk_summary);
        if !output.status.success() {
            bail!(
                "fixture-rewrite worker failed with status {}; stdout: {}",
                output.status,
                stdout.trim()
            );
        }
        if total > 0 && should_report_fixture_rewrite_progress(summary.processed, total) {
            eprintln!(
                "fixture-rewrite: {}/{} processed, {} changed",
                summary.processed, total, summary.rewritten
            );
        }
    }
    println!("rewrote {} fixture(s)", summary.rewritten);
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|output| !output.stdout.is_empty() || !output.status.success()))]
fn fixture_rewrite_chunk_output(exe: &Path, chunk: &[PathBuf]) -> Result<std::process::Output> {
    let mut command = ProcessCommand::new(exe);
    command.arg("fixture-rewrite").arg("--chunk-worker");
    for path in chunk {
        command.arg("--path").arg(path);
    }
    command.output().context("running fixture-rewrite worker")
}

#[requires(true)]
#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|summary| summary.processed >= summary.rewritten))]
fn parse_fixture_rewrite_summary(stdout: &str) -> Result<RewriteSummary> {
    let line = stdout
        .lines()
        .rev()
        .find(|line| line.starts_with("fixtures="))
        .ok_or_else(|| anyhow::anyhow!("fixture-rewrite worker did not print a summary"))?;
    let mut summary = RewriteSummary::default();
    for part in line.split(", ") {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        let value = value
            .parse::<usize>()
            .with_context(|| format!("parsing fixture-rewrite summary value `{value}`"))?;
        match key {
            "fixtures" => summary.processed = value,
            "rewritten" => summary.rewritten = value,
            _ => {}
        }
    }
    Ok(summary)
}

#[derive(Debug, Default, Clone, Copy)]
#[invariant(true)]
struct RewriteSummary {
    processed: usize,
    rewritten: usize,
}

impl RewriteSummary {
    #[requires(other.processed >= other.rewritten)]
    #[ensures(self.processed >= self.rewritten)]
    fn merge(&mut self, other: Self) {
        self.processed += other.processed;
        self.rewritten += other.rewritten;
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|summary| summary.processed >= summary.rewritten))]
fn fixture_rewrite_paths(paths: Vec<PathBuf>, report_progress: bool) -> Result<RewriteSummary> {
    let mut rewritten = 0usize;
    let total = paths.len();
    for (index, path) in paths.into_iter().enumerate() {
        let processed = index + 1;
        if report_progress && total > 0 && should_report_fixture_rewrite_progress(processed, total)
        {
            eprintln!("fixture-rewrite: {processed}/{total} processed, {rewritten} changed");
        }
        let before = fs::read_to_string(&path)
            .with_context(|| format!("reading fixture `{}`", path.display()))?;
        let mut fixture = load_fixture_path(&path)
            .with_context(|| format!("loading fixture `{}`", path.display()))?;
        refresh_fixture_expectations(&mut fixture)
            .with_context(|| format!("refreshing fixture `{}`", path.display()))?;
        write_fixture_file(&path, &fixture.test_case)
            .with_context(|| format!("rewriting fixture `{}`", path.display()))?;
        let after = fs::read_to_string(&path)
            .with_context(|| format!("reading rewritten fixture `{}`", path.display()))?;
        if before != after {
            rewritten += 1;
        }
    }
    Ok(RewriteSummary {
        processed: total,
        rewritten,
    })
}

#[requires(total > 0)]
#[ensures(processed == total -> ret)]
fn should_report_fixture_rewrite_progress(processed: usize, total: usize) -> bool {
    processed == 1 || processed == total || processed.is_multiple_of(100)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn refresh_fixture_expectations(fixture: &mut LoadedTestCase) -> Result<()> {
    let dialect = fixture.test_case.dialect_definition()?;
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
    let words = segment_words_with_modifiers_with_options_and_source_id(
        &fixture.test_case.lojban,
        &morphology_options,
        Some(SourceId("<fixture>".to_owned())),
    );
    if let Some(morphology) = &mut fixture.test_case.expectations.morphology
        && morphology.status == ExpectationStatus::Success
    {
        let morphology_words = words.clone()?;
        morphology.raw = Some(text_expectation(format_debug_value(&morphology_words)));
        let vlasei = ensure_vlasei_output(&mut fixture.test_case.expectations);
        vlasei.json = Some(text_expectation(
            compact_morphology_json_string_with_options(
                &morphology_words,
                JsonRenderOptions {
                    indent: 0,
                    ..JsonRenderOptions::default()
                },
            )?,
        ));
        vlasei.brackets = Some(text_expectation(pretty_morphology_brackets_with_options(
            &morphology_words,
            &fixture.test_case.lojban,
            BracketRenderOptions {
                color: false,
                ..BracketRenderOptions::default()
            },
        )?));
        vlasei.tree = Some(text_expectation(pretty_morphology_tree_with_options(
            &morphology_words,
            &fixture.test_case.lojban,
            TreeRenderOptions {
                color: false,
                indent: 2,
                show_spans: true,
                ..TreeRenderOptions::default()
            },
        )?));
    }
    let refresh_syntax = fixture
        .test_case
        .expectations
        .syntax
        .as_ref()
        .is_some_and(syntax_accepts_success_tree_refresh);
    let refresh_warnings = fixture
        .test_case
        .expectations
        .warnings
        .as_ref()
        .is_some_and(|warnings| warnings.status == ExpectationStatus::Success);
    let refresh_tree = fixture
        .test_case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.gentufa.as_ref())
        .is_some_and(|output| output.tree.is_some());
    let refresh_brackets = fixture
        .test_case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.gentufa.as_ref())
        .is_some_and(|output| output.brackets.is_some());
    if refresh_syntax || refresh_warnings || refresh_tree || refresh_brackets {
        let syntax_words = words?;
        if let Ok(parsed) = parse_syntax_tree_with_source_and_options(
            &syntax_words,
            &fixture.test_case.lojban,
            &syntax_options,
        ) {
            if refresh_syntax {
                if let Some(syntax) = &mut fixture.test_case.expectations.syntax {
                    syntax.raw = Some(text_expectation(format_debug_value(&parsed.parse_tree)));
                }
                let gentufa = ensure_gentufa_output(&mut fixture.test_case.expectations);
                gentufa.json = Some(text_expectation(compact_syntax_json_string_with_options(
                    &parsed.parse_tree,
                    JsonRenderOptions {
                        indent: 0,
                        ..JsonRenderOptions::default()
                    },
                )?));
                gentufa.tree = Some(text_expectation(pretty_tree_with_options(
                    &parsed.parse_tree,
                    &fixture.test_case.lojban,
                    TreeRenderOptions {
                        color: false,
                        indent: 2,
                        show_spans: true,
                        ..TreeRenderOptions::default()
                    },
                )?));
            }
            if refresh_warnings && let Some(warnings) = &mut fixture.test_case.expectations.warnings
            {
                warnings.items =
                    warning_expectation_items(&fixture.test_case.lojban, &parsed.warnings);
            }
            if refresh_tree
                && let Some(output) = &mut fixture.test_case.expectations.output
                && let Some(gentufa) = &mut output.gentufa
                && let Some(tree) = &mut gentufa.tree
            {
                tree.text = pretty_tree_with_options(
                    &parsed.parse_tree,
                    &fixture.test_case.lojban,
                    TreeRenderOptions {
                        color: false,
                        indent: 2,
                        show_spans: true,
                        ..TreeRenderOptions::default()
                    },
                )?;
            }
            if refresh_brackets
                && let Some(output) = &mut fixture.test_case.expectations.output
                && let Some(gentufa) = &mut output.gentufa
                && let Some(brackets) = &mut gentufa.brackets
            {
                brackets.text = pretty_brackets(&parsed.parse_tree, &fixture.test_case.lojban)?;
            }
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn ensure_vlasei_output(
    expectations: &mut fixtures::Expectations,
) -> &mut fixtures::CommandOutputExpectation {
    expectations
        .output
        .get_or_insert_with(Default::default)
        .vlasei
        .get_or_insert_with(Default::default)
}

#[requires(true)]
#[ensures(true)]
fn ensure_gentufa_output(
    expectations: &mut fixtures::Expectations,
) -> &mut fixtures::CommandOutputExpectation {
    expectations
        .output
        .get_or_insert_with(Default::default)
        .gentufa
        .get_or_insert_with(Default::default)
}

#[requires(true)]
#[ensures(true)]
fn text_expectation(text: String) -> fixtures::TextExpectation {
    fixtures::TextExpectation { text }
}

#[requires(true)]
#[ensures(true)]
fn format_debug_value<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

#[requires(true)]
#[ensures(true)]
fn debug_value_matches<T: std::fmt::Debug>(value: &T, expected: &str) -> bool {
    let mut writer = DebugMatchWriter {
        expected,
        offset: 0,
    };
    if fmt::write(&mut writer, format_args!("{value:?}")).is_err() {
        return false;
    }
    writer.offset == expected.len()
}

#[derive(Debug)]
#[invariant(true)]
struct DebugMatchWriter<'expected> {
    expected: &'expected str,
    offset: usize,
}

impl fmt::Write for DebugMatchWriter<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let end = self.offset.saturating_add(text.len());
        if self.expected.get(self.offset..end) == Some(text) {
            self.offset = end;
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn format_debug_prefix<T: std::fmt::Debug>(value: &T) -> String {
    let mut writer = DebugPrefixWriter {
        output: String::new(),
        truncated: false,
    };
    let _ = fmt::write(&mut writer, format_args!("{value:?}"));
    if writer.truncated {
        writer.output.push_str("...");
    }
    writer.output
}

#[derive(Debug, Default)]
#[invariant(true)]
struct DebugPrefixWriter {
    output: String,
    truncated: bool,
}

impl fmt::Write for DebugPrefixWriter {
    #[requires(true)]
    #[ensures(true)]
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self.truncated {
            return Err(fmt::Error);
        }
        let remaining = DEBUG_MISMATCH_LIMIT.saturating_sub(self.output.chars().count());
        let mut chars = text.chars();
        self.output.extend(chars.by_ref().take(remaining));
        if chars.next().is_some() {
            self.truncated = true;
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn syntax_accepts_success_tree_refresh(syntax: &fixtures::SyntaxExpectation) -> bool {
    match syntax.status {
        ExpectationStatus::Success => syntax
            .xfail
            .as_ref()
            .is_none_or(|xfail| xfail.accepted_status == ExpectationStatus::Success),
        ExpectationStatus::Failure => syntax
            .xfail
            .as_ref()
            .is_some_and(|xfail| xfail.accepted_status == ExpectationStatus::Success),
        ExpectationStatus::Pending | ExpectationStatus::NotApplicable => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn fixture_test(args: FixtureRunArgs) -> Result<()> {
    let profile = merged_profile(&args)?;
    let backend = NotImplementedBackend;
    let mut paths = fixture_paths(&args.root)
        .with_context(|| format!("listing fixtures under `{}`", args.root.display()))?;
    let jobs = args.jobs.unwrap_or_else(default_fixture_jobs);
    if !args.chunk_worker && should_spawn_fixture_test_chunks(&profile) {
        return fixture_test_subprocess_chunks(&args, &profile, &paths, jobs);
    }
    paths.retain(|path| path_matches_prefix_selector(&args.root, path, &profile.selector));
    let failure_counter = AtomicUsize::new(0);
    let mut summary = RunSummary::default();
    for chunk in paths.chunks(FIXTURE_TEST_CHUNK_SIZE) {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .stack_size(FIXTURE_WORKER_STACK_SIZE)
            .build()
            .context("creating fixture-test thread pool")?;
        let chunk_summary = pool
            .install(|| {
                run_fixture_test_jobs(
                    &args.root,
                    &profile,
                    &backend,
                    chunk,
                    args.failure_samples,
                    &failure_counter,
                )
            })
            .with_context(|| format!("loading fixtures under `{}`", args.root.display()))?;
        summary.merge(chunk_summary);
        drop(pool);
        trim_fixture_worker_heap();
    }
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
#[ensures(true)]
fn should_spawn_fixture_test_chunks(profile: &FixtureProfile) -> bool {
    profile.facets.iter().any(|facet| {
        matches!(
            facet,
            Facet::Syntax | Facet::VlaseiTree | Facet::GentufaTree
        )
    })
}

#[requires(profile.is_valid())]
#[ensures(true)]
fn fixture_test_subprocess_chunks(
    args: &FixtureRunArgs,
    profile: &FixtureProfile,
    paths: &[PathBuf],
    jobs: usize,
) -> Result<()> {
    let exe = std::env::current_exe().context("resolving xtask executable")?;
    let selected_paths = paths
        .iter()
        .filter(|path| path_matches_prefix_selector(&args.root, path, &profile.selector))
        .collect::<Vec<_>>();
    let mut summary = RunSummary::default();
    for chunk in selected_paths.chunks(FIXTURE_TEST_SUBPROCESS_CHUNK_SIZE) {
        let output = fixture_test_chunk_output(&exe, args, profile, chunk, jobs)?;
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let chunk_summary = parse_fixture_test_summary(&stdout)?;
        let child_failed_without_fixture_failures =
            !output.status.success() && chunk_summary.failed == 0;
        summary.merge(chunk_summary);
        if child_failed_without_fixture_failures {
            bail!(
                "fixture-test worker failed with status {}; stdout: {}",
                output.status,
                stdout.trim()
            );
        }
    }
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
#[requires(jobs > 0)]
#[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|output| !output.stdout.is_empty() || !output.status.success()))]
fn fixture_test_chunk_output(
    exe: &Path,
    args: &FixtureRunArgs,
    profile: &FixtureProfile,
    chunk: &[&PathBuf],
    jobs: usize,
) -> Result<std::process::Output> {
    let mut command = ProcessCommand::new(exe);
    command
        .arg("fixture-test")
        .arg("--root")
        .arg(&args.root)
        .arg("--jobs")
        .arg(jobs.to_string())
        .arg("--failure-samples")
        .arg(args.failure_samples.to_string())
        .arg("--chunk-worker");
    for facet in &profile.facets {
        command.arg("--facet").arg(facet.to_string());
    }
    append_selector_args(&mut command, &profile.selector);
    for path in chunk {
        let path = *path;
        let relative = path.strip_prefix(&args.root).unwrap_or(path);
        command
            .arg("--path-prefix")
            .arg(relative.to_string_lossy().to_string());
    }
    command.output().context("running fixture-test worker")
}

#[requires(selector.is_valid())]
#[ensures(true)]
fn append_selector_args(command: &mut ProcessCommand, selector: &FixtureSelector) {
    for value in &selector.provenance {
        command.arg("--provenance").arg(value);
    }
    for value in &selector.tags {
        command.arg("--tag").arg(value);
    }
    for value in &selector.ids {
        command.arg("--id").arg(value);
    }
    if let Some(cll) = &selector.cll {
        if let Some(chapter) = cll.chapter {
            command.arg("--cll-chapter").arg(chapter.to_string());
        }
        if let Some(section) = &cll.section_number {
            command.arg("--cll-section").arg(section);
        }
        if let Some(example) = &cll.example_id {
            command.arg("--cll-example").arg(example);
        } else if let Some(example) = &cll.example_number {
            command.arg("--cll-example").arg(example);
        }
    }
    if let Some(muplis) = &selector.muplis {
        if let Some(collection) = &muplis.collection_id {
            command.arg("--muplis-collection").arg(collection);
        }
        if let Some(item) = &muplis.item_id {
            command.arg("--muplis-item").arg(item);
        }
        if let Some(form) = &muplis.form {
            command.arg("--muplis-form").arg(form.to_string());
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|summary| summary.total_results() == summary.passed + summary.failed + summary.skipped + summary.xfailed) || ret.is_err())]
fn parse_fixture_test_summary(stdout: &str) -> Result<RunSummary> {
    let line = stdout
        .lines()
        .rev()
        .find(|line| line.starts_with("fixtures="))
        .ok_or_else(|| anyhow::anyhow!("fixture-test worker did not print a summary"))?;
    let mut summary = RunSummary::default();
    for part in line.split(", ") {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        let value = value
            .parse::<usize>()
            .with_context(|| format!("parsing fixture-test summary value `{value}`"))?;
        match key {
            "fixtures" => summary.selected_fixtures = value,
            "facets" => summary.selected_facets = value,
            "passed" => summary.passed = value,
            "xfailed" => summary.xfailed = value,
            "failed" => summary.failed = value,
            "skipped" => summary.skipped = value,
            _ => {}
        }
    }
    Ok(summary)
}

#[derive(Debug, Default)]
#[invariant(true)]
struct VectorStats {
    selected: usize,
    parsed: usize,
    failed: usize,
    fields: BTreeMap<String, FieldLengths>,
}

#[derive(Debug, Default)]
#[invariant(true)]
struct FieldLengths {
    lengths: Vec<usize>,
}

impl VectorStats {
    #[requires(true)]
    #[ensures(self.selected >= old(self.selected))]
    fn merge(&mut self, other: VectorStats) {
        self.selected += other.selected;
        self.parsed += other.parsed;
        self.failed += other.failed;
        for (field, mut lengths) in other.fields {
            self.fields
                .entry(field)
                .or_default()
                .lengths
                .append(&mut lengths.lengths);
        }
    }

    #[requires(!field.is_empty())]
    #[ensures(self.fields.contains_key(field))]
    fn record_field_length(&mut self, field: &str, length: usize) {
        self.fields
            .entry(field.to_owned())
            .or_default()
            .lengths
            .push(length);
    }
}

#[requires(true)]
#[ensures(true)]
fn fixture_vector_stats(args: FixtureVectorStatsArgs) -> Result<()> {
    let paths = fixture_paths(&args.root)
        .with_context(|| format!("listing fixtures under `{}`", args.root.display()))?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(args.jobs.unwrap_or_else(default_fixture_jobs))
        .stack_size(FIXTURE_WORKER_STACK_SIZE)
        .build()
        .context("creating fixture-vector-stats thread pool")?;
    let stats = pool
        .install(|| {
            paths
                .par_iter()
                .map(|path| fixture_vector_stats_for_path(path))
                .try_reduce(VectorStats::default, |mut left, right| {
                    left.merge(right);
                    Ok(left)
                })
        })
        .with_context(|| format!("loading fixtures under `{}`", args.root.display()))?;
    print_vector_stats(&stats, args.min_count);
    Ok(())
}

#[requires(path.components().next().is_some())]
#[ensures(true)]
fn fixture_vector_stats_for_path(path: &Path) -> Result<VectorStats> {
    let fixture = load_fixture_path(path)?;
    let mut stats = VectorStats {
        selected: 1,
        ..VectorStats::default()
    };
    let dialect = match fixture.test_case.dialect_definition() {
        Ok(dialect) => dialect,
        Err(_) => {
            stats.failed = 1;
            return Ok(stats);
        }
    };
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
    let words = match segment_words_with_modifiers_with_options_and_source_id(
        &fixture.test_case.lojban,
        &morphology_options,
        Some(SourceId("<fixture>".to_owned())),
    ) {
        Ok(words) => words,
        Err(_) => {
            stats.failed = 1;
            return Ok(stats);
        }
    };
    let parsed = match parse_syntax_tree_with_source_and_options(
        &words,
        &fixture.test_case.lojban,
        &syntax_options,
    ) {
        Ok(parsed) => parsed,
        Err(_) => {
            stats.failed = 1;
            return Ok(stats);
        }
    };
    let value = serde_json::to_value(&parsed.parse_tree).context("serializing parse tree")?;
    stats.parsed = 1;
    record_json_array_lengths(&value, &mut Vec::new(), &mut stats);
    Ok(stats)
}

#[requires(true)]
#[ensures(true)]
fn record_json_array_lengths(
    value: &serde_json::Value,
    path: &mut Vec<String>,
    stats: &mut VectorStats,
) {
    match value {
        serde_json::Value::Array(items) => {
            if let Some(field) = vector_field_path(path) {
                stats.record_field_length(&field, items.len());
            }
            path.push("[]".to_owned());
            for item in items {
                record_json_array_lengths(item, path, stats);
            }
            path.pop();
        }
        serde_json::Value::Object(object) => {
            for (key, item) in object {
                path.push(key.clone());
                record_json_array_lengths(item, path, stats);
                path.pop();
            }
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
    }
}

#[requires(true)]
#[ensures(true)]
fn vector_field_path(path: &[String]) -> Option<String> {
    let last = path.last()?;
    if last == "span" || last == "source_span" {
        return None;
    }
    Some(path.join("."))
}

#[requires(true)]
#[ensures(true)]
fn print_vector_stats(stats: &VectorStats, min_count: usize) {
    println!(
        "fixtures={}, parsed={}, failed={}",
        stats.selected, stats.parsed, stats.failed
    );
    println!("field\tcount\tmin\tp50\tp90\tp95\tp99\tmax\tavg");
    for (field, lengths) in &stats.fields {
        if lengths.lengths.len() < min_count {
            continue;
        }
        let summary = summarize_lengths(&lengths.lengths);
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.2}",
            field,
            summary.count,
            summary.min,
            summary.p50,
            summary.p90,
            summary.p95,
            summary.p99,
            summary.max,
            summary.average
        );
    }
}

#[derive(Debug)]
#[invariant(true)]
struct LengthSummary {
    count: usize,
    min: usize,
    p50: usize,
    p90: usize,
    p95: usize,
    p99: usize,
    max: usize,
    average: f64,
}

#[requires(!lengths.is_empty())]
#[ensures(ret.count == lengths.len())]
fn summarize_lengths(lengths: &[usize]) -> LengthSummary {
    let mut sorted = lengths.to_vec();
    sorted.sort_unstable();
    let sum = sorted.iter().sum::<usize>();
    LengthSummary {
        count: sorted.len(),
        min: sorted[0],
        p50: percentile(&sorted, 50),
        p90: percentile(&sorted, 90),
        p95: percentile(&sorted, 95),
        p99: percentile(&sorted, 99),
        max: *sorted
            .last()
            .expect("precondition guarantees non-empty lengths"),
        average: sum as f64 / sorted.len() as f64,
    }
}

#[requires(!sorted.is_empty())]
#[requires(percentile <= 100)]
#[ensures(ret >= sorted[0])]
fn percentile(sorted: &[usize], percentile: usize) -> usize {
    let index = ((sorted.len() - 1) * percentile).div_ceil(100);
    sorted[index]
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
            if !path_matches_prefix_selector(root, path, &profile.selector) {
                return Ok(RunSummary::default());
            }
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
            trim_fixture_worker_heap();
            Ok(summary)
        })
        .try_reduce(RunSummary::default, |mut left, right| {
            left.merge(right);
            Ok(left)
        })
}

#[requires(selector.is_valid())]
#[ensures(true)]
fn path_matches_prefix_selector(root: &Path, path: &Path, selector: &FixtureSelector) -> bool {
    if selector.path_prefixes.is_empty() {
        return true;
    }
    let relative = path.strip_prefix(root).unwrap_or(path);
    let relative_text = relative.to_string_lossy();
    selector
        .path_prefixes
        .iter()
        .any(|prefix| relative_text.starts_with(prefix))
}

#[requires(true)]
#[ensures(true)]
fn trim_fixture_worker_heap() {
    // Raw/tree fixture facets create very large transient strings. glibc often
    // keeps those freed arenas mapped, so long corpus sweeps can hit the
    // process memory limit even when no Rust values are retained.
    #[cfg(target_os = "linux")]
    unsafe {
        unsafe extern "C" {
            fn malloc_trim(pad: usize) -> std::ffi::c_int;
        }
        let _ = malloc_trim(0);
    }
}

#[ensures(ret > 0)]
#[requires(true)]
fn default_fixture_jobs() -> usize {
    DEFAULT_TEST_JOBS
}

// TOML fixtures can contain deeply nested exported syntax trees, and serde's
// TOML decoder needs more stack than Rayon workers get by default.
const FIXTURE_WORKER_STACK_SIZE: usize = 32 * 1024 * 1024;
const FIXTURE_TEST_CHUNK_SIZE: usize = 8;
const FIXTURE_TEST_SUBPROCESS_CHUNK_SIZE: usize = 64;
const FIXTURE_REWRITE_SUBPROCESS_CHUNK_SIZE: usize = 64;
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
            Facet::Warnings => run_warnings_fixture(fixture),
            Facet::VlaseiBrackets => run_vlasei_brackets_fixture(fixture),
            Facet::VlaseiTree => run_vlasei_tree_fixture(fixture),
            Facet::VlaseiJson => run_vlasei_json_fixture(fixture),
            Facet::GentufaBrackets => run_gentufa_brackets_fixture(fixture),
            Facet::GentufaTree => run_gentufa_tree_fixture(fixture),
            Facet::GentufaJson => run_gentufa_json_fixture(fixture),
        }
    }
}

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_vlasei_brackets_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = fixture
        .test_case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.vlasei.as_ref())
        .and_then(|output| output.brackets.as_ref())
    else {
        return FacetResult::skipped("fixture has no vlasei brackets expectation");
    };
    let dialect = match fixture.test_case.dialect_definition() {
        Ok(dialect) => dialect,
        Err(error) => return FacetResult::failed(format!("dialect error: {error}")),
    };
    let options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let words = match segment_words_with_modifiers_with_options_and_source_id(
        &fixture.test_case.lojban,
        &options,
        Some(SourceId("<fixture>".to_owned())),
    ) {
        Ok(words) => words,
        Err(error) => return FacetResult::failed(format!("morphology error: {error}")),
    };
    match pretty_morphology_brackets_with_options(
        &words,
        &fixture.test_case.lojban,
        BracketRenderOptions {
            color: false,
            ..BracketRenderOptions::default()
        },
    ) {
        Ok(actual) if actual == expectation.text => FacetResult::passed(),
        Ok(actual) => FacetResult::failed(format_text_mismatch(
            "vlasei brackets",
            &expectation.text,
            &actual,
        )),
        Err(error) => FacetResult::failed(format!("vlasei brackets render error: {error}")),
    }
}

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_vlasei_tree_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = fixture
        .test_case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.vlasei.as_ref())
        .and_then(|output| output.tree.as_ref())
    else {
        return FacetResult::skipped("fixture has no vlasei tree expectation");
    };
    let dialect = match fixture.test_case.dialect_definition() {
        Ok(dialect) => dialect,
        Err(error) => return FacetResult::failed(format!("dialect error: {error}")),
    };
    let options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let words = match segment_words_with_modifiers_with_options_and_source_id(
        &fixture.test_case.lojban,
        &options,
        Some(SourceId("<fixture>".to_owned())),
    ) {
        Ok(words) => words,
        Err(error) => return FacetResult::failed(format!("morphology error: {error}")),
    };
    match pretty_morphology_tree_with_options(
        &words,
        &fixture.test_case.lojban,
        TreeRenderOptions {
            color: false,
            indent: 2,
            show_spans: true,
            ..TreeRenderOptions::default()
        },
    ) {
        Ok(actual) if actual == expectation.text => FacetResult::passed(),
        Ok(actual) => FacetResult::failed(format_text_mismatch(
            "vlasei tree",
            &expectation.text,
            &actual,
        )),
        Err(error) => FacetResult::failed(format!("vlasei tree render error: {error}")),
    }
}

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_vlasei_json_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = fixture
        .test_case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.vlasei.as_ref())
        .and_then(|output| output.json.as_ref())
    else {
        return FacetResult::skipped("fixture has no vlasei JSON expectation");
    };
    let dialect = match fixture.test_case.dialect_definition() {
        Ok(dialect) => dialect,
        Err(error) => return FacetResult::failed(format!("dialect error: {error}")),
    };
    let options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let words = match segment_words_with_modifiers_with_options_and_source_id(
        &fixture.test_case.lojban,
        &options,
        Some(SourceId("<fixture>".to_owned())),
    ) {
        Ok(words) => words,
        Err(error) => return FacetResult::failed(format!("morphology error: {error}")),
    };
    match compact_morphology_json_string_with_options(
        &words,
        JsonRenderOptions {
            indent: 0,
            ..JsonRenderOptions::default()
        },
    ) {
        Ok(actual) if actual == expectation.text => FacetResult::passed(),
        Ok(actual) => FacetResult::failed(format_text_mismatch(
            "vlasei JSON",
            &expectation.text,
            &actual,
        )),
        Err(error) => FacetResult::failed(format!("vlasei JSON render error: {error}")),
    }
}

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_gentufa_brackets_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = fixture
        .test_case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.gentufa.as_ref())
        .and_then(|output| output.brackets.as_ref())
    else {
        return FacetResult::skipped("fixture has no gentufa brackets expectation");
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

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_gentufa_tree_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = fixture
        .test_case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.gentufa.as_ref())
        .and_then(|output| output.tree.as_ref())
    else {
        return FacetResult::skipped("fixture has no gentufa tree expectation");
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
    match pretty_tree_with_options(
        &parsed.parse_tree,
        &fixture.test_case.lojban,
        TreeRenderOptions {
            color: false,
            indent: 2,
            show_spans: true,
            ..TreeRenderOptions::default()
        },
    ) {
        Ok(actual) if actual == expectation.text => FacetResult::passed(),
        Ok(actual) => FacetResult::failed(format_text_mismatch(
            "gentufa tree",
            &expectation.text,
            &actual,
        )),
        Err(error) => FacetResult::failed(format!("tree render error: {error}")),
    }
}

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_gentufa_json_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = fixture
        .test_case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.gentufa.as_ref())
        .and_then(|output| output.json.as_ref())
    else {
        return FacetResult::skipped("fixture has no gentufa JSON expectation");
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
    match compact_syntax_json_string_with_options(
        &parsed.parse_tree,
        JsonRenderOptions {
            indent: 0,
            ..JsonRenderOptions::default()
        },
    ) {
        Ok(actual) if actual == expectation.text => FacetResult::passed(),
        Ok(actual) => FacetResult::failed(format_text_mismatch(
            "gentufa JSON",
            &expectation.text,
            &actual,
        )),
        Err(error) => FacetResult::failed(format!("gentufa JSON render error: {error}")),
    }
}

#[requires(!label.is_empty())]
#[ensures(!ret.is_empty())]
fn format_text_mismatch(label: &str, expected: &str, actual: &str) -> String {
    format!(
        "{label} mismatch: expected `{}`, got `{}`",
        truncate_for_mismatch(expected),
        truncate_for_mismatch(actual)
    )
}

#[requires(true)]
#[ensures(ret.len() <= text.len() + 3)]
fn truncate_for_mismatch(text: &str) -> String {
    let mut output = text.chars().take(DEBUG_MISMATCH_LIMIT).collect::<String>();
    if output.len() < text.len() {
        output.push_str("...");
    }
    output
}

const DEBUG_MISMATCH_LIMIT: usize = 512;

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

#[requires(fixture.test_case.is_valid_fixture_metadata())]
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
                let Some(expected_raw) = &expectation.raw else {
                    return FacetResult::failed("syntax success expectation has no raw tree");
                };
                if debug_value_matches(&parsed.parse_tree, &expected_raw.text) {
                    syntax_xfail_result(expectation, ExpectationStatus::Success, true)
                        .unwrap_or_else(FacetResult::passed)
                } else if expectation.xfail.is_some()
                    && expectation
                        .xfail
                        .as_ref()
                        .is_some_and(|xfail| xfail.accepted_status == ExpectationStatus::Success)
                {
                    FacetResult::failed("syntax xfail accepted success, but raw tree did not match")
                } else {
                    FacetResult::failed(format_text_mismatch(
                        "syntax raw",
                        &expected_raw.text,
                        &format_debug_prefix(&parsed.parse_tree),
                    ))
                }
            }
            ExpectationStatus::Failure => {
                if expectation
                    .xfail
                    .as_ref()
                    .is_some_and(|xfail| xfail.accepted_status == ExpectationStatus::Success)
                {
                    let Some(expected_raw) = &expectation.raw else {
                        return FacetResult::failed("syntax success xfail has no raw tree");
                    };
                    if debug_value_matches(&parsed.parse_tree, &expected_raw.text) {
                        syntax_xfail_result(expectation, ExpectationStatus::Success, true)
                            .unwrap_or_else(|| {
                                FacetResult::failed(
                                    "syntax xfail unexpectedly missing accepted success metadata",
                                )
                            })
                    } else {
                        FacetResult::failed(format!(
                            "syntax xfail accepted success, but {}",
                            format_text_mismatch(
                                "syntax raw",
                                &expected_raw.text,
                                &format_debug_prefix(&parsed.parse_tree),
                            )
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

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_warnings_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = &fixture.test_case.expectations.warnings else {
        return FacetResult::skipped("fixture has no warnings expectation");
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
                ExpectationStatus::Failure => FacetResult::passed(),
                ExpectationStatus::Success => {
                    FacetResult::failed(format!("warnings blocked by morphology error: {error}"))
                }
                ExpectationStatus::Pending | ExpectationStatus::NotApplicable => {
                    FacetResult::skipped(format!(
                        "warnings expectation is {:?}",
                        expectation.status
                    ))
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
                let expected = &expectation.items;
                let actual = warning_expectation_items(&fixture.test_case.lojban, &parsed.warnings);
                if *expected == actual {
                    FacetResult::passed()
                } else {
                    FacetResult::failed(format!(
                        "warnings mismatch: expected {expected:?}, got {actual:?}"
                    ))
                }
            }
            ExpectationStatus::Failure => {
                FacetResult::failed("expected warnings parse failure, got success")
            }
            ExpectationStatus::Pending | ExpectationStatus::NotApplicable => {
                FacetResult::skipped(format!("warnings expectation is {:?}", expectation.status))
            }
        },
        Err(error) => match expectation.status {
            ExpectationStatus::Failure => FacetResult::passed(),
            ExpectationStatus::Success => {
                FacetResult::failed(format!("warnings syntax error: {error}"))
            }
            ExpectationStatus::Pending | ExpectationStatus::NotApplicable => {
                FacetResult::skipped(format!("warnings expectation is {:?}", expectation.status))
            }
        },
    }
}

#[requires(true)]
#[ensures(ret.len() == warnings.len())]
fn warning_expectation_items(
    source: &str,
    warnings: &[SyntaxWarning],
) -> Vec<fixtures::WarningItemExpectation> {
    warnings
        .iter()
        .map(|warning| warning_expectation_item(source, warning))
        .collect()
}

#[requires(true)]
#[ensures(!ret.anchor_text.is_empty())]
fn warning_expectation_item(
    source: &str,
    warning: &SyntaxWarning,
) -> fixtures::WarningItemExpectation {
    let span = warning_anchor_span(warning);
    let anchor_text = source
        .get(span[0]..span[1])
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| warning.anchor.to_string());
    fixtures::WarningItemExpectation {
        kind: warning.kind,
        anchor_index: warning.anchor_index,
        anchor_text,
        span,
    }
}

#[requires(true)]
#[ensures(ret[0] <= ret[1])]
fn warning_anchor_span(warning: &SyntaxWarning) -> [usize; 2] {
    let mut spans = warning.anchor.source_spans();
    spans.sort_by_key(|span| span.byte_start);
    let Some(first) = spans.first() else {
        return [0, 0];
    };
    let last = spans.last().expect("first span exists");
    [first.byte_start, last.byte_end]
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

#[requires(fixture.test_case.is_valid_fixture_metadata())]
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
                Ok(actual)
                    if expectation
                        .raw
                        .as_ref()
                        .is_some_and(|raw| debug_value_matches(&actual, &raw.text)) =>
                {
                    FacetResult::passed()
                }
                Ok(actual) => FacetResult::failed(format_text_mismatch(
                    "morphology raw",
                    &expectation
                        .raw
                        .as_ref()
                        .map(|raw| raw.text.as_str())
                        .unwrap_or_default(),
                    &format_debug_prefix(&actual),
                )),
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
                Err(error) => {
                    if let Some(expected) = &expectation.error {
                        let actual = error.to_string();
                        if actual.contains(expected) {
                            FacetResult::passed()
                        } else {
                            FacetResult::failed(format!(
                                "morphology error mismatch: expected substring `{expected}`, got `{actual}`"
                            ))
                        }
                    } else {
                        FacetResult::passed()
                    }
                }
            }
        }
        ExpectationStatus::Pending | ExpectationStatus::NotApplicable => FacetResult::skipped(
            format!("morphology expectation is {:?}", expectation.status),
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
        Facet::Warnings => expectations.warnings.as_ref().map(|value| value.status),
        Facet::VlaseiBrackets => expectations
            .output
            .as_ref()
            .and_then(|output| output.vlasei.as_ref())
            .and_then(|output| output.brackets.as_ref())
            .map(|_| ExpectationStatus::Success),
        Facet::VlaseiTree => expectations
            .output
            .as_ref()
            .and_then(|output| output.vlasei.as_ref())
            .and_then(|output| output.tree.as_ref())
            .map(|_| ExpectationStatus::Success),
        Facet::VlaseiJson => expectations
            .output
            .as_ref()
            .and_then(|output| output.vlasei.as_ref())
            .and_then(|output| output.json.as_ref())
            .map(|_| ExpectationStatus::Success),
        Facet::GentufaBrackets => expectations
            .output
            .as_ref()
            .and_then(|output| output.gentufa.as_ref())
            .and_then(|output| output.brackets.as_ref())
            .map(|_| ExpectationStatus::Success),
        Facet::GentufaTree => expectations
            .output
            .as_ref()
            .and_then(|output| output.gentufa.as_ref())
            .and_then(|output| output.tree.as_ref())
            .map(|_| ExpectationStatus::Success),
        Facet::GentufaJson => expectations
            .output
            .as_ref()
            .and_then(|output| output.gentufa.as_ref())
            .and_then(|output| output.json.as_ref())
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
