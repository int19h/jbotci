use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitStatus};
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context, Result, bail};
use bityzba::{contract_trait, ensures, invariant, requires};
use clap::{Args, Parser, Subcommand};
use jbotci_diagnostics::{Diagnostic, DiagnosticSeverity};
use jbotci_dictionary::import::parse_lensisku_json;
use jbotci_morphology::{
    MorphologyError, MorphologyOptions, MorphologyWarning,
    segment_words_with_modifiers_with_options_and_source_id,
    segment_words_with_modifiers_with_options_and_source_id_attempt,
};
use jbotci_output::{
    BracketRenderOptions, JsonRenderOptions, TreeRenderOptions,
    compact_morphology_json_string_with_options, compact_syntax_json_string_with_options,
    pretty_brackets, pretty_morphology_brackets_with_options, pretty_morphology_tree_with_options,
    pretty_tree_with_options,
};
use jbotci_semantics::references::{
    FixturePlaceSlot, FixtureReferenceTarget, FixtureSpanKey, ReferenceFixtureProjection,
    analyze_references,
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
#[invariant(::RefsV0Parity(..) => true)]
#[invariant(::FixtureVectorStats(..) => true)]
#[invariant(::FixtureTest(..) => true)]
#[invariant(::VendorDictionary(..) => true)]
#[invariant(::BuildWebRelease(..) => true)]
#[invariant(::ServeWebRelease(..) => true)]
#[invariant(::ExportWebEmbeddingCorpus(..) => true)]
#[invariant(::BuildWebEmbeddings(..) => true)]
#[invariant(::DistServer(..) => true)]
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
    RefsV0Parity(RefsV0ParityArgs),
    FixtureVectorStats(FixtureVectorStatsArgs),
    FixtureTest(FixtureRunArgs),
    VendorDictionary(VendorDictionaryArgs),
    BuildWebRelease(BuildWebReleaseArgs),
    ServeWebRelease(ServeWebReleaseArgs),
    ExportWebEmbeddingCorpus(ExportWebEmbeddingCorpusArgs),
    BuildWebEmbeddings(BuildWebEmbeddingsArgs),
    DistServer(DistServerArgs),
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
    #[arg(short = 'j', long, value_name = "N")]
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
    #[arg(long)]
    migrate_morphology_diagnostics: bool,
    #[arg(long)]
    add_semantics_refs: bool,
    #[arg(long, hide = true)]
    chunk_worker: bool,
    #[arg(long = "path", hide = true)]
    paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct RefsV0ParityArgs {
    #[arg(long)]
    input: PathBuf,
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

#[derive(Debug, Args)]
#[invariant(true)]
struct BuildWebReleaseArgs {
    #[arg(long)]
    base_path: Option<String>,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct ServeWebReleaseArgs {
    #[arg(long, default_value_t = 8081)]
    port: u16,
    #[arg(long, default_value_t = false, num_args = 0..=1, default_missing_value = "true")]
    open: bool,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct ExportWebEmbeddingCorpusArgs {
    #[arg(long, default_value = ".jbotci-build/web-embedding-corpus.json")]
    output: PathBuf,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct BuildWebEmbeddingsArgs {
    #[arg(long, default_value = ".jbotci-build/jbotci-web/public")]
    web_dist: PathBuf,
    #[arg(long)]
    corpus: Option<PathBuf>,
    #[arg(long = "dtype", default_values_t = ["q4".to_owned(), "q8".to_owned()])]
    dtypes: Vec<String>,
    #[arg(long, default_value = "transformers")]
    backend: String,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct DistServerArgs {
    #[arg(long, default_value = ".jbotci-build/jbotci-web")]
    out_dir: PathBuf,
    #[arg(long, default_value = "/jbotci")]
    base_path: String,
    #[arg(long)]
    skip_web_bundle: bool,
    #[arg(long)]
    skip_web_embeddings: bool,
    #[arg(long = "embedding-dtype", default_values_t = ["q4".to_owned(), "q8".to_owned()])]
    embedding_dtypes: Vec<String>,
    #[arg(long, default_value = "transformers")]
    embedding_backend: String,
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
        Command::RefsV0Parity(args) => refs_v0_parity(args),
        Command::FixtureVectorStats(args) => fixture_vector_stats(args),
        Command::FixtureTest(args) => fixture_test(args),
        Command::VendorDictionary(args) => vendor_dictionary(args),
        Command::BuildWebRelease(args) => build_web_release(args),
        Command::ServeWebRelease(args) => serve_web_release(args),
        Command::ExportWebEmbeddingCorpus(args) => export_web_embedding_corpus(args),
        Command::BuildWebEmbeddings(args) => build_web_embeddings(args),
        Command::DistServer(args) => dist_server(args),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn build_web_release(args: BuildWebReleaseArgs) -> Result<()> {
    let mut command = dx_web_release_command("build");
    if let Some(base_path) = args.base_path {
        command.arg("--base-path").arg(base_path);
    }
    let status = command.status().context("failed to run `dx build`")?;
    check_status(status, "dx build --web --release --debug-symbols=false")
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn serve_web_release(args: ServeWebReleaseArgs) -> Result<()> {
    let status = dx_web_release_command("serve")
        .arg("--port")
        .arg(args.port.to_string())
        .arg("--open")
        .arg(args.open.to_string())
        .status()
        .context("failed to run `dx serve`")?;
    check_status(status, "dx serve --web --release --debug-symbols=false")
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn export_web_embedding_corpus(args: ExportWebEmbeddingCorpusArgs) -> Result<()> {
    let output = absolute_path(&args.output)?;
    write_web_embedding_corpus(&output)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn build_web_embeddings(args: BuildWebEmbeddingsArgs) -> Result<()> {
    let web_dist = absolute_path(&args.web_dist)?;
    let corpus = match args.corpus {
        Some(path) => absolute_path(&path)?,
        None => {
            let output = absolute_path(Path::new(".jbotci-build/web-embedding-corpus.json"))?;
            write_web_embedding_corpus(&output)?;
            output
        }
    };
    build_web_embedding_assets(&web_dist, &corpus, &args.dtypes, &args.backend)
}

#[requires(matches!(subcommand, "build" | "bundle" | "serve"))]
#[ensures(true)]
fn dx_web_release_command(subcommand: &str) -> ProcessCommand {
    let mut command = ProcessCommand::new("dx");
    command
        .arg(subcommand)
        .arg("--web")
        .arg("--release")
        .arg("-p")
        .arg("jbotci-web")
        // Dioxus 0.7.x can emit DWARF that makes wasm-opt abort during release web builds.
        .arg("--debug-symbols=false");
    command
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn dist_server(args: DistServerArgs) -> Result<()> {
    let out_dir = absolute_path(&args.out_dir)?;
    if !args.skip_web_bundle {
        if out_dir.exists() {
            fs::remove_dir_all(&out_dir)
                .with_context(|| format!("removing old web bundle `{}`", out_dir.display()))?;
        }
        run_dx_bundle(&out_dir, &args.base_path)?;
    }
    let web_dist = web_dist_dir(&out_dir)?;
    if !args.skip_web_embeddings {
        let corpus = absolute_path(Path::new(".jbotci-build/web-embedding-corpus.json"))?;
        write_web_embedding_corpus(&corpus)?;
        build_web_embedding_assets(
            &web_dist,
            &corpus,
            &args.embedding_dtypes,
            &args.embedding_backend,
        )?;
    }
    build_server_binary()
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_dx_bundle(out_dir: &Path, base_path: &str) -> Result<()> {
    let base_path = normalized_dist_base_path(base_path);
    let dioxus_public = Path::new("target/dx/jbotci-web/release/web/public");
    if dioxus_public.exists() {
        fs::remove_dir_all(dioxus_public)
            .with_context(|| format!("removing old Dioxus output `{}`", dioxus_public.display()))?;
    }
    let mut command = dx_web_release_command("bundle");
    command
        .arg("--out-dir")
        .arg(out_dir)
        .arg("--base-path")
        .arg(base_path);
    let status = command.status().context("failed to run `dx bundle`")?;
    check_status(status, "dx bundle --web --release --debug-symbols=false")
}

#[requires(true)]
#[ensures(!ret.starts_with('/'))]
fn normalized_dist_base_path(base_path: &str) -> String {
    base_path.trim().trim_matches('/').to_owned()
}

#[requires(out_dir.is_absolute())]
#[ensures(ret.as_ref().is_ok_and(|path| path.is_dir()) || ret.is_err())]
fn web_dist_dir(out_dir: &Path) -> Result<PathBuf> {
    let candidates = [out_dir.join("public"), out_dir.to_path_buf()];
    candidates
        .into_iter()
        .find(|candidate| candidate.join("index.html").is_file())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "could not find Dioxus web `index.html` under `{}`",
                out_dir.display()
            )
        })
}

#[requires(web_dist.is_dir())]
#[requires(corpus.is_file())]
#[requires(!dtypes.is_empty())]
#[requires(!backend.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn build_web_embedding_assets(
    web_dist: &Path,
    corpus: &Path,
    dtypes: &[String],
    backend: &str,
) -> Result<()> {
    ensure_node_embedding_dependencies()?;
    let output = web_dist
        .join("assets")
        .join("embeddings")
        .join("web")
        .join("v1");
    let mut command = ProcessCommand::new("node");
    command
        .arg("tools/embedding-pack/build-web-embeddings.mjs")
        .arg("--input")
        .arg(corpus)
        .arg("--out")
        .arg(&output)
        .arg("--backend")
        .arg(backend);
    for dtype in dtypes {
        command.arg("--dtype").arg(dtype);
    }
    let status = command.status().with_context(|| {
        format!(
            "failed to run web embedding builder for `{}`",
            output.display()
        )
    })?;
    check_status(status, "node tools/embedding-pack/build-web-embeddings.mjs")
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn write_web_embedding_corpus(output: &Path) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating `{}`", parent.display()))?;
    }
    fs::write(
        output,
        jbotci_embedding_inputs::embedding_input_corpus_json(),
    )
    .with_context(|| format!("writing web embedding corpus `{}`", output.display()))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn ensure_node_embedding_dependencies() -> Result<()> {
    let dependency =
        Path::new("tools/embedding-pack/node_modules/@huggingface/transformers/package.json");
    if dependency.is_file() {
        return Ok(());
    }
    let status = ProcessCommand::new("npm")
        .arg("ci")
        .arg("--prefix")
        .arg("tools/embedding-pack")
        .status()
        .context("failed to run `npm ci --prefix tools/embedding-pack`")?;
    check_status(status, "npm ci --prefix tools/embedding-pack")
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn build_server_binary() -> Result<()> {
    let status = ProcessCommand::new("cargo")
        .arg("build")
        .arg("-p")
        .arg("jbotci-server")
        .arg("--release")
        .status()
        .context("failed to run server release build")?;
    check_status(status, "cargo build -p jbotci-server --release")
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
fn refs_v0_parity(args: RefsV0ParityArgs) -> Result<()> {
    let text = fs::read_to_string(&args.input)
        .with_context(|| format!("reading `{}`", args.input.display()))?;
    let export = serde_json::from_str::<V0RefsExport>(&text)
        .with_context(|| format!("parsing `{}`", args.input.display()))?;
    let mut failures = ParityFailures::default();
    let mut checked = 0usize;
    let mut skipped = 0usize;
    for case in &export.cases {
        let Some(refs) = &case.syntax_refs else {
            skipped += 1;
            continue;
        };
        if !refs.has_facts() {
            skipped += 1;
            continue;
        }
        if v0_refs_case_is_outside_syntax_ref_gate(case) {
            skipped += 1;
            continue;
        }
        checked += 1;
        if checked == 1 || checked.is_multiple_of(100) {
            eprintln!(
                "refs-v0-parity: checked {checked} case(s), current {}",
                case.id
            );
        }
        match v1_reference_projection_for_v0_case(case) {
            Ok(projection) => {
                compare_v0_reference_facts(case, refs, &projection, &mut failures);
            }
            Err(error) => failures.push(format!("{}: {error}", case.id)),
        }
        trim_fixture_worker_heap();
    }
    println!(
        "v0 refs parity: schema={}, checked={}, skipped={}, failures={}",
        export.schema_version, checked, skipped, failures.count
    );
    if failures.count > 0 {
        let sample = failures.samples.join("\n");
        bail!("v0 refs parity failed:\n{sample}");
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn v0_refs_case_is_outside_syntax_ref_gate(case: &V0RefsCase) -> bool {
    // The disposable gate is intentionally limited to curated CLL examples.
    // The free-form corpus and muplis chapter-18 corpus include parser
    // divergences and v0 higher-order Lean-semantic facts that are outside the
    // SyntaxRef-style syntax-tree reference overlay validated here.
    !case.id.starts_with("cll.")
}

const V0_PARITY_FAILURE_SAMPLE_LIMIT: usize = 50;

#[derive(Debug, Default)]
#[invariant(true)]
struct ParityFailures {
    count: usize,
    samples: Vec<String>,
}

impl ParityFailures {
    #[requires(!message.is_empty())]
    #[ensures(self.count == old(self.count) + 1)]
    fn push(&mut self, message: String) {
        self.count += 1;
        if self.samples.len() < V0_PARITY_FAILURE_SAMPLE_LIMIT {
            self.samples.push(message);
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
struct V0RefsExport {
    #[serde(rename = "schema-version")]
    schema_version: u16,
    cases: Vec<V0RefsCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
struct V0RefsCase {
    id: String,
    lojban: String,
    #[allow(dead_code)]
    #[serde(default)]
    provenance: Vec<serde_json::Value>,
    #[serde(default)]
    dialect: Option<String>,
    #[serde(default, rename = "syntax-refs")]
    syntax_refs: Option<V0SyntaxRefs>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
struct V0SyntaxRefs {
    #[serde(default, rename = "argument-assignments")]
    argument_assignments: Vec<V0ArgumentAssignmentFact>,
    #[serde(default, rename = "relation-places")]
    relation_places: Vec<V0RelationPlaceFact>,
    #[serde(default, rename = "pro-argument-targets")]
    pro_argument_targets: Vec<V0LabelledSpan>,
    #[serde(default, rename = "pro-argument-sources")]
    pro_argument_sources: Vec<V0LabelledSpan>,
    #[serde(default, rename = "pro-predicate-targets")]
    pro_predicate_targets: Vec<V0LabelledSpan>,
    #[serde(default, rename = "pro-predicate-sources")]
    pro_predicate_sources: Vec<V0LabelledSpan>,
    #[serde(default, rename = "pro-relation-targets")]
    pro_relation_targets: Vec<V0LabelledSpan>,
    #[serde(default, rename = "pro-relation-sources")]
    pro_relation_sources: Vec<V0LabelledSpan>,
    #[serde(default, rename = "pro-utterance-targets")]
    pro_utterance_targets: Vec<V0LabelledSpan>,
}

impl V0SyntaxRefs {
    #[requires(true)]
    #[ensures(true)]
    fn has_facts(&self) -> bool {
        !self.argument_assignments.is_empty()
            || !self.relation_places.is_empty()
            || !self.pro_argument_targets.is_empty()
            || !self.pro_argument_sources.is_empty()
            || !self.pro_predicate_targets.is_empty()
            || !self.pro_predicate_sources.is_empty()
            || !self.pro_relation_targets.is_empty()
            || !self.pro_relation_sources.is_empty()
            || !self.pro_utterance_targets.is_empty()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
struct V0ArgumentAssignmentFact {
    argument: FixtureSpanKey,
    relation: Option<FixtureSpanKey>,
    #[serde(rename = "place-index")]
    place_index: Option<u8>,
    #[allow(dead_code)]
    label: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
struct V0RelationPlaceFact {
    relation: FixtureSpanKey,
    place: u8,
    argument: FixtureSpanKey,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[invariant(true)]
struct V0LabelledSpan {
    node: FixtureSpanKey,
    label: String,
}

#[requires(!case.lojban.is_empty())]
#[ensures(true)]
fn v1_reference_projection_for_v0_case(case: &V0RefsCase) -> Result<ReferenceFixtureProjection> {
    let dialect = match &case.dialect {
        Some(formula) => jbotci_dialect::parse_dialect_definition(formula)
            .map_err(|error| anyhow::anyhow!("invalid dialect `{formula}`: {}", error.message()))?,
        None => jbotci_dialect::DialectDefinition::baseline(),
    };
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
    let words = segment_words_with_modifiers_with_options_and_source_id(
        &case.lojban,
        &morphology_options,
        Some(SourceId("<v0-refs>".to_owned())),
    )
    .with_context(|| format!("{}: morphology failed", case.id))?;
    let parsed = parse_syntax_tree_with_source_and_options(&words, &case.lojban, &syntax_options)
        .with_context(|| format!("{}: syntax failed", case.id))?;
    let analysis = analyze_references(&parsed.parse_tree)
        .with_context(|| format!("{}: reference analysis failed", case.id))?;
    Ok(analysis.fixture_projection())
}

#[requires(true)]
#[ensures(true)]
fn compare_v0_reference_facts(
    case: &V0RefsCase,
    refs: &V0SyntaxRefs,
    projection: &ReferenceFixtureProjection,
    failures: &mut ParityFailures,
) {
    for assignment in &refs.argument_assignments {
        if !projection_contains_v0_assignment(projection, assignment) {
            failures.push(format!(
                "{}: missing argument assignment argument={:?} relation={:?} place={:?}",
                case.id, assignment.argument, assignment.relation, assignment.place_index
            ));
        }
    }
    for relation_place in &refs.relation_places {
        if !projection_contains_v0_relation_place(projection, relation_place) {
            failures.push(format!(
                "{}: missing relation place relation={:?} place={} argument={:?}",
                case.id, relation_place.relation, relation_place.place, relation_place.argument
            ));
        }
    }
    compare_labelled_reference_pairs(
        case,
        "pro-argument",
        &refs.pro_argument_sources,
        &combined_argument_targets(refs),
        projection,
        failures,
    );
    compare_labelled_reference_pairs(
        case,
        "pro-predicate",
        &refs.pro_predicate_sources,
        &refs.pro_predicate_targets,
        projection,
        failures,
    );
    compare_labelled_reference_pairs(
        case,
        "pro-relation",
        &refs.pro_relation_sources,
        &refs.pro_relation_targets,
        projection,
        failures,
    );
}

#[requires(true)]
#[ensures(true)]
fn combined_argument_targets(refs: &V0SyntaxRefs) -> Vec<V0LabelledSpan> {
    let mut targets = refs.pro_argument_targets.clone();
    targets.extend(refs.pro_utterance_targets.clone());
    targets
}

#[requires(true)]
#[ensures(true)]
fn projection_contains_v0_assignment(
    projection: &ReferenceFixtureProjection,
    expected: &V0ArgumentAssignmentFact,
) -> bool {
    projection.assignments.iter().any(|actual| {
        (actual.argument == expected.argument
            || assignment_argument_references(projection, actual, &expected.argument))
            && expected
                .relation
                .as_ref()
                .is_none_or(|relation| assignment_matches_relation(actual, relation))
            && expected
                .place_index
                .is_none_or(|place| assignment_reaches_numbered_place(projection, actual, place))
    })
}

#[requires(true)]
#[ensures(true)]
fn assignment_argument_references(
    projection: &ReferenceFixtureProjection,
    assignment: &jbotci_semantics::references::FixtureArgumentAssignment,
    expected_argument: &FixtureSpanKey,
) -> bool {
    projection.references.iter().any(|edge| {
        edge.source == assignment.argument
            && reference_target_contains(&edge.target, expected_argument)
    })
}

#[requires(true)]
#[ensures(true)]
fn projection_contains_v0_relation_place(
    projection: &ReferenceFixtureProjection,
    expected: &V0RelationPlaceFact,
) -> bool {
    projection.assignments.iter().any(|actual| {
        actual.argument == expected.argument
            && assignment_matches_relation(actual, &expected.relation)
            && assignment_reaches_numbered_place(projection, actual, expected.place)
    })
}

#[requires(true)]
#[ensures(true)]
fn assignment_matches_relation(
    assignment: &jbotci_semantics::references::FixtureArgumentAssignment,
    relation: &FixtureSpanKey,
) -> bool {
    assignment.relation.as_ref() == Some(relation)
        || assignment.relation_unit.as_ref() == Some(relation)
        || assignment.frame_node == *relation
        || assignment
            .relation
            .as_ref()
            .is_some_and(|actual| span_is_suffix_of(actual, relation))
        || assignment
            .relation_unit
            .as_ref()
            .is_some_and(|actual| span_is_suffix_of(actual, relation))
        || span_is_suffix_of(&assignment.frame_node, relation)
}

#[requires(true)]
#[ensures(true)]
fn span_is_suffix_of(actual: &FixtureSpanKey, expected: &FixtureSpanKey) -> bool {
    actual.offset >= expected.offset
        && actual.offset + actual.length == expected.offset + expected.length
        && actual.length < expected.length
}

#[requires(true)]
#[ensures(true)]
fn assignment_reaches_numbered_place(
    projection: &ReferenceFixtureProjection,
    assignment: &jbotci_semantics::references::FixtureArgumentAssignment,
    place: u8,
) -> bool {
    let mut visited = Vec::new();
    slot_reaches_numbered_place(
        projection,
        assignment.frame,
        &assignment.slot,
        place,
        &mut visited,
    )
}

#[requires(true)]
#[ensures(true)]
fn slot_reaches_numbered_place(
    projection: &ReferenceFixtureProjection,
    frame: usize,
    slot: &FixturePlaceSlot,
    place: u8,
    visited: &mut Vec<(usize, FixturePlaceSlot)>,
) -> bool {
    if *slot == (FixturePlaceSlot::Numbered { place }) {
        return true;
    }
    let visit_key = (frame, slot.clone());
    if visited.contains(&visit_key) {
        return false;
    }
    visited.push(visit_key);
    let Some(frame_data) = projection
        .frames
        .iter()
        .find(|candidate| candidate.index == frame)
    else {
        return false;
    };
    match &frame_data.propagation {
        jbotci_semantics::references::FixturePlaceFramePropagation::None => false,
        jbotci_semantics::references::FixturePlaceFramePropagation::Forward { inner } => {
            slot_reaches_numbered_place(projection, *inner, slot, place, visited)
        }
        jbotci_semantics::references::FixturePlaceFramePropagation::Conversion {
            inner,
            converted_place,
        } => {
            let converted = convert_fixture_slot(slot.clone(), *converted_place);
            slot_reaches_numbered_place(projection, *inner, &converted, place, visited)
        }
        jbotci_semantics::references::FixturePlaceFramePropagation::Jai { inner } => match slot {
            FixturePlaceSlot::Fai => slot_reaches_numbered_place(
                projection,
                *inner,
                &FixturePlaceSlot::Numbered { place: 1 },
                place,
                visited,
            ),
            FixturePlaceSlot::Numbered { place: slot_place } if *slot_place > 1 => {
                slot_reaches_numbered_place(projection, *inner, slot, place, visited)
            }
            FixturePlaceSlot::Numbered { .. } | FixturePlaceSlot::Modal { .. } => false,
        },
        jbotci_semantics::references::FixturePlaceFramePropagation::Connected { branches } => {
            branches.iter().any(|branch| {
                slot_reaches_numbered_place(projection, *branch, slot, place, visited)
            })
        }
        jbotci_semantics::references::FixturePlaceFramePropagation::Compound {
            head,
            modifiers,
        } => {
            slot_reaches_numbered_place(projection, *head, slot, place, visited)
                || (matches!(slot, FixturePlaceSlot::Numbered { place: 1 })
                    && modifiers.iter().any(|modifier| {
                        slot_reaches_numbered_place(projection, *modifier, slot, place, visited)
                    }))
        }
        jbotci_semantics::references::FixturePlaceFramePropagation::Co { leading, trailing } => {
            slot_reaches_numbered_place(projection, *trailing, slot, place, visited)
                || (matches!(slot, FixturePlaceSlot::Numbered { place: 1 })
                    && slot_reaches_numbered_place(projection, *leading, slot, place, visited))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn convert_fixture_slot(slot: FixturePlaceSlot, converted_place: u8) -> FixturePlaceSlot {
    match slot {
        FixturePlaceSlot::Numbered { place: 1 } => FixturePlaceSlot::Numbered {
            place: converted_place,
        },
        FixturePlaceSlot::Numbered { place } if place == converted_place => {
            FixturePlaceSlot::Numbered { place: 1 }
        }
        other => other,
    }
}

#[requires(!label.is_empty())]
#[ensures(true)]
fn compare_labelled_reference_pairs(
    case: &V0RefsCase,
    label: &str,
    sources: &[V0LabelledSpan],
    targets: &[V0LabelledSpan],
    projection: &ReferenceFixtureProjection,
    failures: &mut ParityFailures,
) {
    let targets_by_label = targets_by_label(targets);
    for source in sources {
        let Some(targets) = targets_by_label.get(&source.label) else {
            failures.push(format!(
                "{}: v0 {label} source {:?} label `{}` has no exported target",
                case.id, source.node, source.label
            ));
            continue;
        };
        if !targets.iter().any(|target| {
            projection_contains_reference_edge(projection, &source.node, &target.node)
        }) {
            failures.push(format!(
                "{}: missing {label} reference source={:?} label=`{}` targets={:?}",
                case.id, source.node, source.label, targets
            ));
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn targets_by_label(targets: &[V0LabelledSpan]) -> BTreeMap<String, Vec<V0LabelledSpan>> {
    let mut grouped = BTreeMap::new();
    for target in targets {
        grouped
            .entry(target.label.clone())
            .or_insert_with(Vec::new)
            .push(target.clone());
    }
    grouped
}

#[requires(true)]
#[ensures(true)]
fn projection_contains_reference_edge(
    projection: &ReferenceFixtureProjection,
    source: &FixtureSpanKey,
    target: &FixtureSpanKey,
) -> bool {
    projection
        .references
        .iter()
        .any(|edge| edge.source == *source && reference_target_contains(&edge.target, target))
}

#[requires(true)]
#[ensures(true)]
fn reference_target_contains(target: &FixtureReferenceTarget, expected: &FixtureSpanKey) -> bool {
    match target {
        FixtureReferenceTarget::ResolvedNode { node } => node == expected,
        FixtureReferenceTarget::ResolvedFrame { frame_node, .. } => frame_node == expected,
        FixtureReferenceTarget::AmbiguousNodes { nodes } => {
            nodes.iter().any(|node| node == expected)
        }
        FixtureReferenceTarget::Unresolved { .. } | FixtureReferenceTarget::Vague { .. } => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn fixture_rewrite(args: FixtureRewriteArgs) -> Result<()> {
    fixture_rewrite_inner(args)
}

#[requires(true)]
#[ensures(true)]
fn fixture_rewrite_inner(args: FixtureRewriteArgs) -> Result<()> {
    if args.chunk_worker {
        let summary = fixture_rewrite_paths(
            args.paths,
            false,
            args.migrate_morphology_diagnostics,
            args.add_semantics_refs,
        )?;
        println!(
            "fixtures={}, rewritten={}",
            summary.processed, summary.rewritten
        );
        return Ok(());
    }
    if !args.paths.is_empty() {
        let summary = fixture_rewrite_paths(
            args.paths,
            true,
            args.migrate_morphology_diagnostics,
            args.add_semantics_refs,
        )?;
        println!("rewrote {} fixture(s)", summary.rewritten);
        return Ok(());
    }
    fixture_rewrite_subprocess_chunks(
        args.roots,
        args.migrate_morphology_diagnostics,
        args.add_semantics_refs,
    )
}

#[requires(true)]
#[ensures(true)]
fn fixture_rewrite_subprocess_chunks(
    roots: Vec<PathBuf>,
    migrate_morphology_diagnostics: bool,
    add_semantics_refs: bool,
) -> Result<()> {
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
        let output = fixture_rewrite_chunk_output(
            &exe,
            chunk,
            migrate_morphology_diagnostics,
            add_semantics_refs,
        )?;
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
fn fixture_rewrite_chunk_output(
    exe: &Path,
    chunk: &[PathBuf],
    migrate_morphology_diagnostics: bool,
    add_semantics_refs: bool,
) -> Result<std::process::Output> {
    let mut command = ProcessCommand::new(exe);
    command.arg("fixture-rewrite").arg("--chunk-worker");
    if migrate_morphology_diagnostics {
        command.arg("--migrate-morphology-diagnostics");
    }
    if add_semantics_refs {
        command.arg("--add-semantics-refs");
    }
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
fn fixture_rewrite_paths(
    paths: Vec<PathBuf>,
    report_progress: bool,
    migrate_morphology_diagnostics: bool,
    add_semantics_refs: bool,
) -> Result<RewriteSummary> {
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
        if migrate_morphology_diagnostics {
            migrate_legacy_morphology_diagnostics(&mut fixture).with_context(|| {
                format!(
                    "migrating morphology diagnostics in fixture `{}`",
                    path.display()
                )
            })?;
        } else {
            refresh_fixture_expectations(&mut fixture, add_semantics_refs)
                .with_context(|| format!("refreshing fixture `{}`", path.display()))?;
        }
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
fn migrate_legacy_morphology_diagnostics(fixture: &mut LoadedTestCase) -> Result<()> {
    let migrate_morphology = fixture
        .test_case
        .expectations
        .morphology
        .as_ref()
        .is_some_and(expectation_has_legacy_morphology_placeholder);
    let migrate_syntax = fixture
        .test_case
        .expectations
        .syntax
        .as_ref()
        .is_some_and(expectation_has_legacy_morphology_placeholder);
    let migrate_success_morphology_now_failure = fixture
        .test_case
        .expectations
        .morphology
        .as_ref()
        .is_some_and(|morphology| morphology.status == ExpectationStatus::Success);
    let refresh_morphology_failure_diagnostics = fixture
        .test_case
        .expectations
        .morphology
        .as_ref()
        .is_some_and(|morphology| {
            morphology.status == ExpectationStatus::Failure
                && diagnostics_are_morphology(&morphology.diagnostics)
        });
    let migrate_syntax_parse_blocked_by_morphology = fixture
        .test_case
        .expectations
        .syntax
        .as_ref()
        .is_some_and(|syntax| {
            syntax.status == ExpectationStatus::Failure
                && syntax_has_single_parse_diagnostic(&syntax.diagnostics)
        });
    let refresh_syntax_blocking_morphology_diagnostics = fixture
        .test_case
        .expectations
        .syntax
        .as_ref()
        .is_some_and(|syntax| {
            syntax.status == ExpectationStatus::Failure
                && diagnostics_are_morphology(&syntax.diagnostics)
        });
    let should_migrate = migrate_morphology
        || migrate_syntax
        || migrate_success_morphology_now_failure
        || refresh_morphology_failure_diagnostics
        || migrate_syntax_parse_blocked_by_morphology
        || refresh_syntax_blocking_morphology_diagnostics;
    if !should_migrate {
        return Ok(());
    }

    let dialect = fixture.test_case.dialect_definition()?;
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
    let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
        &fixture.test_case.lojban,
        &morphology_options,
        Some(SourceId("<fixture>".to_owned())),
    );
    let attempt = attempt.into_data();
    let morphology_warning_diagnostics = morphology_warning_diagnostic_expectation_items(
        &fixture.test_case.lojban,
        &attempt.warnings,
    );

    match attempt.result {
        Err(error) => {
            let diagnostic = error.to_diagnostic(
                Some(SourceId("<fixture>".to_owned())),
                &fixture.test_case.lojban,
            );
            let mut diagnostics = morphology_warning_diagnostics;
            diagnostics.extend(diagnostic_expectation_items(
                &fixture.test_case.lojban,
                std::slice::from_ref(&diagnostic),
            ));
            if migrate_morphology
                || migrate_success_morphology_now_failure
                || refresh_morphology_failure_diagnostics
            {
                fixture
                    .test_case
                    .expectations
                    .morphology
                    .as_mut()
                    .expect("morphology expectation was checked")
                    .status = ExpectationStatus::Failure;
                let morphology = fixture
                    .test_case
                    .expectations
                    .morphology
                    .as_mut()
                    .expect("morphology expectation was checked");
                morphology.raw = None;
                morphology.diagnostics = diagnostics.clone();
                clear_vlasei_output(&mut fixture.test_case.expectations);
            }
            if migrate_syntax
                || migrate_syntax_parse_blocked_by_morphology
                || refresh_syntax_blocking_morphology_diagnostics
            {
                fixture
                    .test_case
                    .expectations
                    .syntax
                    .as_mut()
                    .expect("syntax expectation was checked")
                    .status = ExpectationStatus::Failure;
                let syntax = fixture
                    .test_case
                    .expectations
                    .syntax
                    .as_mut()
                    .expect("syntax expectation was checked");
                syntax.raw = None;
                syntax.diagnostics = diagnostics;
            }
        }
        Ok(words) => {
            if migrate_morphology
                || migrate_success_morphology_now_failure
                || refresh_morphology_failure_diagnostics
            {
                refresh_morphology_success_expectations(
                    fixture,
                    &words,
                    &morphology_warning_diagnostics,
                )?;
            }
            if migrate_syntax
                || migrate_syntax_parse_blocked_by_morphology
                || refresh_syntax_blocking_morphology_diagnostics
            {
                refresh_syntax_after_morphology_success(
                    fixture,
                    &words,
                    &syntax_options,
                    &morphology_warning_diagnostics,
                )?;
            }
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn refresh_morphology_success_expectations(
    fixture: &mut LoadedTestCase,
    words: &[jbotci_morphology::WordLike],
    diagnostics: &[fixtures::DiagnosticExpectation],
) -> Result<()> {
    let morphology = fixture
        .test_case
        .expectations
        .morphology
        .as_mut()
        .expect("morphology expectation was checked");
    morphology.status = ExpectationStatus::Success;
    morphology.raw = Some(text_expectation(format_debug_value(words)));
    morphology.diagnostics = diagnostics.to_vec();
    let vlasei = ensure_vlasei_output(&mut fixture.test_case.expectations);
    vlasei.json = Some(text_expectation(
        compact_morphology_json_string_with_options(
            words,
            JsonRenderOptions {
                indent: 0,
                ..JsonRenderOptions::default()
            },
        )?,
    ));
    vlasei.brackets = Some(text_expectation(pretty_morphology_brackets_with_options(
        words,
        &fixture.test_case.lojban,
        BracketRenderOptions {
            color: false,
            ..BracketRenderOptions::default()
        },
    )?));
    vlasei.tree = Some(text_expectation(pretty_morphology_tree_with_options(
        words,
        &fixture.test_case.lojban,
        TreeRenderOptions {
            color: false,
            indent: 2,
            show_spans: true,
            ..TreeRenderOptions::default()
        },
    )?));
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn refresh_syntax_after_morphology_success(
    fixture: &mut LoadedTestCase,
    words: &[jbotci_morphology::WordLike],
    syntax_options: &ParseOptions,
    morphology_diagnostics: &[fixtures::DiagnosticExpectation],
) -> Result<()> {
    let source = fixture.test_case.lojban.clone();
    let syntax = fixture
        .test_case
        .expectations
        .syntax
        .as_mut()
        .expect("syntax expectation was checked");
    match parse_syntax_tree_with_source_and_options(words, &source, syntax_options) {
        Ok(parsed) => {
            syntax.status = ExpectationStatus::Success;
            syntax.raw = Some(text_expectation(format_debug_value(&parsed.parse_tree)));
            let mut diagnostics = morphology_diagnostics.to_vec();
            diagnostics.extend(syntax_warning_diagnostic_expectation_items(
                &source,
                &parsed.warnings,
            ));
            syntax.diagnostics = diagnostics;
        }
        Err(error) => {
            syntax.status = ExpectationStatus::Failure;
            syntax.raw = None;
            let mut diagnostics = morphology_diagnostics.to_vec();
            diagnostics.extend(syntax_error_diagnostic_expectation_items(&source, &error));
            syntax.diagnostics = diagnostics;
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn syntax_has_single_parse_diagnostic(diagnostics: &[fixtures::DiagnosticExpectation]) -> bool {
    matches!(
        diagnostics,
        [diagnostic]
            if diagnostic.severity == DiagnosticSeverity::Error
                && diagnostic.code == "syntax.parse"
    )
}

#[requires(true)]
#[ensures(ret -> !diagnostics.is_empty())]
fn diagnostics_are_morphology(diagnostics: &[fixtures::DiagnosticExpectation]) -> bool {
    !diagnostics.is_empty()
        && diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code.starts_with("morphology."))
}

#[requires(true)]
#[ensures(expectations.output.as_ref().and_then(|output| output.vlasei.as_ref()).is_none())]
fn clear_vlasei_output(expectations: &mut fixtures::Expectations) {
    let Some(output) = &mut expectations.output else {
        return;
    };
    output.vlasei = None;
    if output.gentufa.is_none() {
        expectations.output = None;
    }
}

#[requires(true)]
#[ensures(true)]
fn expectation_has_legacy_morphology_placeholder<T>(expectation: &T) -> bool
where
    T: HasDiagnosticExpectations,
{
    expectation.status() == ExpectationStatus::Failure
        && is_legacy_morphology_placeholder(expectation.diagnostics())
}

#[contract_trait]
trait HasDiagnosticExpectations {
    #[requires(true)]
    #[ensures(matches!(ret, ExpectationStatus::Success | ExpectationStatus::Failure | ExpectationStatus::Pending | ExpectationStatus::NotApplicable))]
    fn status(&self) -> ExpectationStatus;

    #[requires(true)]
    #[ensures(true)]
    fn diagnostics(&self) -> &[fixtures::DiagnosticExpectation];
}

#[contract_trait]
impl HasDiagnosticExpectations for fixtures::MorphologyExpectation {
    fn status(&self) -> ExpectationStatus {
        self.status
    }

    fn diagnostics(&self) -> &[fixtures::DiagnosticExpectation] {
        &self.diagnostics
    }
}

#[contract_trait]
impl HasDiagnosticExpectations for fixtures::SyntaxExpectation {
    fn status(&self) -> ExpectationStatus {
        self.status
    }

    fn diagnostics(&self) -> &[fixtures::DiagnosticExpectation] {
        &self.diagnostics
    }
}

#[requires(true)]
#[ensures(true)]
fn is_legacy_morphology_placeholder(diagnostics: &[fixtures::DiagnosticExpectation]) -> bool {
    matches!(
        diagnostics,
        [diagnostic]
            if diagnostic.severity == DiagnosticSeverity::Error
                && diagnostic.code == "morphology.invalid"
                && diagnostic.byte_span == [0, 0]
                && diagnostic.source_text.is_empty()
                && diagnostic.message.as_deref() == Some("invalid morphology")
                && diagnostic.word_index.is_none()
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn refresh_fixture_expectations(
    fixture: &mut LoadedTestCase,
    add_semantics_refs: bool,
) -> Result<()> {
    let dialect = fixture.test_case.dialect_definition()?;
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
    let words = segment_words_with_modifiers_with_options_and_source_id(
        &fixture.test_case.lojban,
        &morphology_options,
        Some(SourceId("<fixture>".to_owned())),
    );
    if let Some(morphology) = &mut fixture.test_case.expectations.morphology {
        if morphology.status == ExpectationStatus::Failure
            && let Err(error) = &words
        {
            let diagnostic = error.to_diagnostic(
                Some(SourceId("<fixture>".to_owned())),
                &fixture.test_case.lojban,
            );
            morphology.diagnostics = diagnostic_expectation_items(
                &fixture.test_case.lojban,
                std::slice::from_ref(&diagnostic),
            );
        } else if morphology.status == ExpectationStatus::Success {
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
    }
    let refresh_syntax = fixture
        .test_case
        .expectations
        .syntax
        .as_ref()
        .is_some_and(syntax_accepts_success_tree_refresh);
    let refresh_syntax_failure = fixture
        .test_case
        .expectations
        .syntax
        .as_ref()
        .is_some_and(|syntax| syntax.status == ExpectationStatus::Failure);
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
    let existing_semantics_refs_success = fixture
        .test_case
        .expectations
        .semantics
        .as_ref()
        .and_then(|semantics| semantics.refs.as_ref())
        .is_some_and(|refs| refs.status == ExpectationStatus::Success);
    let add_semantics_refs_for_fixture = add_semantics_refs
        && fixture
            .test_case
            .expectations
            .syntax
            .as_ref()
            .is_some_and(|syntax| syntax.status == ExpectationStatus::Success);
    let refresh_semantics_refs = existing_semantics_refs_success || add_semantics_refs_for_fixture;
    if refresh_syntax
        || refresh_syntax_failure
        || refresh_tree
        || refresh_brackets
        || refresh_semantics_refs
    {
        let syntax_words = match &words {
            Ok(words) => words.clone(),
            Err(error) => {
                if refresh_syntax_failure
                    && let Some(syntax) = &mut fixture.test_case.expectations.syntax
                {
                    let diagnostic = error.to_diagnostic(
                        Some(SourceId("<fixture>".to_owned())),
                        &fixture.test_case.lojban,
                    );
                    syntax.diagnostics = diagnostic_expectation_items(
                        &fixture.test_case.lojban,
                        std::slice::from_ref(&diagnostic),
                    );
                }
                if existing_semantics_refs_success {
                    bail!("semantics refs blocked by morphology error: {error}");
                }
                return Ok(());
            }
        };
        match parse_syntax_tree_with_source_and_options(
            &syntax_words,
            &fixture.test_case.lojban,
            &syntax_options,
        ) {
            Ok(parsed) => {
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
                if refresh_semantics_refs {
                    let refs = analyze_references(&parsed.parse_tree)
                        .context("analyzing semantic references")?;
                    let raw = refs
                        .fixture_projection_json()
                        .context("rendering semantic refs fixture projection")?;
                    let refs = ensure_semantics_refs(&mut fixture.test_case.expectations);
                    refs.status = ExpectationStatus::Success;
                    refs.raw = Some(text_expectation(raw));
                }
            }
            Err(error) => {
                if refresh_syntax_failure
                    && let Some(syntax) = &mut fixture.test_case.expectations.syntax
                {
                    let diagnostic = error.to_diagnostic(
                        Some(SourceId("<fixture>".to_owned())),
                        &fixture.test_case.lojban,
                    );
                    let diagnostics = diagnostic_expectation_items(
                        &fixture.test_case.lojban,
                        std::slice::from_ref(&diagnostic),
                    );
                    syntax.diagnostics = diagnostics;
                }
                if existing_semantics_refs_success {
                    bail!("semantics refs blocked by syntax error: {error}");
                }
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
fn ensure_semantics_refs(
    expectations: &mut fixtures::Expectations,
) -> &mut fixtures::ReferenceExpectation {
    expectations
        .semantics
        .get_or_insert_with(Default::default)
        .refs
        .get_or_insert(fixtures::ReferenceExpectation {
            status: ExpectationStatus::Success,
            raw: None,
        })
}

#[requires(true)]
#[ensures(true)]
fn text_expectation(text: String) -> fixtures::TextExpectation {
    fixtures::TextExpectation { text }
}

#[requires(true)]
#[ensures(ret.len() == diagnostics.len())]
fn diagnostic_expectation_items(
    source: &str,
    diagnostics: &[Diagnostic],
) -> Vec<fixtures::DiagnosticExpectation> {
    diagnostics
        .iter()
        .map(|diagnostic| fixtures::DiagnosticExpectation::from_diagnostic(source, diagnostic))
        .collect()
}

#[requires(true)]
#[ensures(ret.len() == warnings.len())]
fn syntax_warning_diagnostic_expectation_items(
    source: &str,
    warnings: &[SyntaxWarning],
) -> Vec<fixtures::DiagnosticExpectation> {
    warnings
        .iter()
        .map(|warning| {
            let diagnostic = warning.to_diagnostic(Some(SourceId("<fixture>".to_owned())), source);
            fixtures::DiagnosticExpectation::from_diagnostic(source, &diagnostic)
        })
        .collect()
}

#[requires(true)]
#[ensures(ret.len() == 1)]
fn syntax_error_diagnostic_expectation_items(
    source: &str,
    error: &SyntaxError,
) -> Vec<fixtures::DiagnosticExpectation> {
    let diagnostic = error.to_diagnostic(Some(SourceId("<fixture>".to_owned())), source);
    diagnostic_expectation_items(source, std::slice::from_ref(&diagnostic))
}

#[requires(true)]
#[ensures(ret.len() == 1)]
fn morphology_error_diagnostic_expectation_items(
    source: &str,
    error: &MorphologyError,
) -> Vec<fixtures::DiagnosticExpectation> {
    let diagnostic = error.to_diagnostic(Some(SourceId("<fixture>".to_owned())), source);
    diagnostic_expectation_items(source, std::slice::from_ref(&diagnostic))
}

#[requires(true)]
#[ensures(ret.len() == warnings.len())]
fn morphology_warning_diagnostic_expectation_items(
    source: &str,
    warnings: &[MorphologyWarning],
) -> Vec<fixtures::DiagnosticExpectation> {
    warnings
        .iter()
        .map(|warning| {
            let diagnostic = warning.to_diagnostic(Some(SourceId("<fixture>".to_owned())), source);
            fixtures::DiagnosticExpectation::from_diagnostic(source, &diagnostic)
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn format_debug_value<T: std::fmt::Debug + ?Sized>(value: &T) -> String {
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
            Facet::Syntax | Facet::SemanticsRefs | Facet::VlaseiTree | Facet::GentufaTree
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
            Facet::SemanticsRefs => run_semantics_refs_fixture(fixture),
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

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_semantics_refs_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = fixture
        .test_case
        .expectations
        .semantics
        .as_ref()
        .and_then(|semantics| semantics.refs.as_ref())
    else {
        return FacetResult::skipped("fixture has no semantic refs expectation");
    };
    if matches!(
        expectation.status,
        ExpectationStatus::Pending | ExpectationStatus::NotApplicable
    ) {
        return FacetResult::skipped(format!(
            "semantic refs expectation is {:?}",
            expectation.status
        ));
    }
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
    let actual = match analyze_references(&parsed.parse_tree) {
        Ok(analysis) => match analysis.fixture_projection_json() {
            Ok(raw) => Ok(raw),
            Err(error) => Err(format!("semantic refs render error: {error}")),
        },
        Err(error) => Err(format!("semantic refs error: {error}")),
    };
    match actual {
        Ok(actual) if expectation.status == ExpectationStatus::Success => {
            let Some(expected_raw) = &expectation.raw else {
                return FacetResult::failed("semantic refs success expectation has no raw value");
            };
            if actual == expected_raw.text {
                FacetResult::passed()
            } else {
                FacetResult::failed(format_text_mismatch(
                    "semantic refs",
                    &expected_raw.text,
                    &actual,
                ))
            }
        }
        Ok(actual) => FacetResult::failed(format!(
            "semantic refs unexpectedly succeeded with `{}`",
            truncate_for_mismatch(&actual)
        )),
        Err(error) if expectation.status == ExpectationStatus::Failure => FacetResult::failed(
            format!("semantic refs failure expectations are not supported: {error}"),
        ),
        Err(error) => FacetResult::failed(format!("semantic refs error: {error}")),
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
    let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
        &fixture.test_case.lojban,
        &options,
        Some(SourceId("<fixture>".to_owned())),
    );
    let attempt = attempt.into_data();
    let morphology_warning_diagnostics = morphology_warning_diagnostic_expectation_items(
        &fixture.test_case.lojban,
        &attempt.warnings,
    );
    let words = match attempt.result {
        Ok(words) => words,
        Err(error) => {
            return match expectation.status {
                ExpectationStatus::Failure => {
                    if !expectation.diagnostics.is_empty() {
                        let mut actual = morphology_warning_diagnostics.clone();
                        actual.extend(morphology_error_diagnostic_expectation_items(
                            &fixture.test_case.lojban,
                            &error,
                        ));
                        if expectation.diagnostics == actual {
                            syntax_xfail_result(expectation, ExpectationStatus::Failure, true)
                                .unwrap_or_else(FacetResult::passed)
                        } else {
                            FacetResult::failed(format!(
                                "syntax-blocking morphology diagnostics mismatch: expected {:?}, got {actual:?}",
                                expectation.diagnostics
                            ))
                        }
                    } else {
                        syntax_xfail_result(expectation, ExpectationStatus::Failure, true)
                            .unwrap_or_else(FacetResult::passed)
                    }
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
                if !expectation.diagnostics.is_empty() {
                    let mut actual = morphology_warning_diagnostics.clone();
                    actual.extend(syntax_warning_diagnostic_expectation_items(
                        &fixture.test_case.lojban,
                        &parsed.warnings,
                    ));
                    if expectation.diagnostics != actual {
                        return FacetResult::failed(format!(
                            "syntax diagnostics mismatch: expected {:?}, got {actual:?}",
                            expectation.diagnostics
                        ));
                    }
                }
                if expectation.raw.is_none() && !expectation.diagnostics.is_empty() {
                    syntax_xfail_result(expectation, ExpectationStatus::Success, true)
                        .unwrap_or_else(FacetResult::passed)
                } else {
                    let Some(expected_raw) = &expectation.raw else {
                        return FacetResult::failed("syntax success expectation has no raw tree");
                    };
                    if debug_value_matches(&parsed.parse_tree, &expected_raw.text) {
                        syntax_xfail_result(expectation, ExpectationStatus::Success, true)
                            .unwrap_or_else(FacetResult::passed)
                    } else if expectation.xfail.is_some()
                        && expectation.xfail.as_ref().is_some_and(|xfail| {
                            xfail.accepted_status == ExpectationStatus::Success
                        })
                    {
                        FacetResult::failed(
                            "syntax xfail accepted success, but raw tree did not match",
                        )
                    } else {
                        FacetResult::failed(format_text_mismatch(
                            "syntax raw",
                            &expected_raw.text,
                            &format_debug_prefix(&parsed.parse_tree),
                        ))
                    }
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
        Err(error @ SyntaxError::Parse { .. }) => match expectation.status {
            ExpectationStatus::Success => {
                syntax_xfail_result(expectation, ExpectationStatus::Failure, true)
                    .unwrap_or_else(|| FacetResult::failed(format!("syntax parse error: {error}")))
            }
            ExpectationStatus::Failure => {
                if !expectation.diagnostics.is_empty() {
                    let mut actual = morphology_warning_diagnostics.clone();
                    actual.extend(syntax_error_diagnostic_expectation_items(
                        &fixture.test_case.lojban,
                        &error,
                    ));
                    if expectation.diagnostics == actual {
                        syntax_xfail_result(expectation, ExpectationStatus::Failure, true)
                            .unwrap_or_else(FacetResult::passed)
                    } else {
                        FacetResult::failed(format!(
                            "syntax diagnostics mismatch: expected {:?}, got {actual:?}",
                            expectation.diagnostics
                        ))
                    }
                } else {
                    syntax_xfail_result(expectation, ExpectationStatus::Failure, true)
                        .unwrap_or_else(FacetResult::passed)
                }
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
            let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
                &fixture.test_case.lojban,
                &options,
                Some(SourceId("<fixture>".to_owned())),
            );
            let attempt = attempt.into_data();
            match attempt.result {
                Ok(actual) => {
                    if !expectation.diagnostics.is_empty() {
                        let diagnostics = morphology_warning_diagnostic_expectation_items(
                            &fixture.test_case.lojban,
                            &attempt.warnings,
                        );
                        if expectation.diagnostics != diagnostics {
                            return FacetResult::failed(format!(
                                "morphology diagnostics mismatch: expected {:?}, got {diagnostics:?}",
                                expectation.diagnostics
                            ));
                        }
                    }
                    if expectation.raw.is_none() && !expectation.diagnostics.is_empty() {
                        return FacetResult::passed();
                    }
                    if expectation
                        .raw
                        .as_ref()
                        .is_some_and(|raw| debug_value_matches(&actual, &raw.text))
                    {
                        FacetResult::passed()
                    } else {
                        FacetResult::failed(format_text_mismatch(
                            "morphology raw",
                            expectation
                                .raw
                                .as_ref()
                                .map(|raw| raw.text.as_str())
                                .unwrap_or_default(),
                            &format_debug_prefix(&actual),
                        ))
                    }
                }
                Err(error) => {
                    if !expectation.diagnostics.is_empty() {
                        let actual = morphology_error_diagnostic_expectation_items(
                            &fixture.test_case.lojban,
                            &error,
                        );
                        if expectation.diagnostics == actual {
                            FacetResult::passed()
                        } else {
                            FacetResult::failed(format!(
                                "morphology diagnostics mismatch: expected {:?}, got {actual:?}",
                                expectation.diagnostics
                            ))
                        }
                    } else {
                        FacetResult::failed(format!("morphology error: {error}"))
                    }
                }
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
                    if !expectation.diagnostics.is_empty() {
                        let actual = morphology_error_diagnostic_expectation_items(
                            &fixture.test_case.lojban,
                            &error,
                        );
                        return if expectation.diagnostics == actual {
                            FacetResult::passed()
                        } else {
                            FacetResult::failed(format!(
                                "morphology diagnostics mismatch: expected {:?}, got {actual:?}",
                                expectation.diagnostics
                            ))
                        };
                    }
                    FacetResult::passed()
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
        Facet::SemanticsRefs => expectations
            .semantics
            .as_ref()
            .and_then(|semantics| semantics.refs.as_ref())
            .map(|value| value.status),
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
