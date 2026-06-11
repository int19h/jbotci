use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitStatus, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use bityzba::{contract_trait, ensures, invariant, requires};
use clap::{Args, Parser, Subcommand, ValueEnum};
use clx::progress::{ProgressJobBuilder, ProgressStatus};
use jbotci_diagnostics::{Diagnostic, DiagnosticSeverity};
use jbotci_dictionary::import::parse_lensisku_json;
use jbotci_morphology::{
    MorphologyError, MorphologyOptions, MorphologyWarning,
    segment_words_with_modifiers_with_options_and_source_id,
    segment_words_with_modifiers_with_options_and_source_id_attempt, word_like_syntax_eq,
};
use jbotci_output::{
    BracketRenderOptions, JsonRenderOptions, LojbanScript, TreeRenderOptions,
    compact_morphology_json_string_with_options, compact_syntax_json_string_with_options,
    pretty_brackets, pretty_brackets_with_options, pretty_morphology_brackets_with_options,
    pretty_morphology_tree_with_options, pretty_tree_with_options,
};
use jbotci_semantics::references::{
    FixturePlaceSlot, FixtureReferenceTarget, FixtureSpanKey, ReferenceFixtureProjection,
    analyze_references,
};
use jbotci_source::SourceId;
use jbotci_syntax::{
    ParseOptions, SyntaxError, SyntaxWarning, parse_syntax_tree_with_source_and_options,
    syntax_tree_eq_ignoring_spans,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

const DIOXUS_WEB_RELEASE_DIR: &str = "target/dx/jbotci-app/release/web";
const DIOXUS_WEB_PUBLIC_INPUT_DIR: &str = "target/jbotci-web-public";
const SHARED_UI_ASSET_DIR: &str = "crates/jbotci-ui/assets";
const RELEASE_SERVICE_WORKER_FILE_NAME: &str = "service-worker.js";
const WEB_ASSET_SYNC_TEMP_DIR: &str = "target/jbotci-web-public-sync";
const R2_CATALOG_CACHE_CONTROL: &str = "public, max-age=300";
const R2_IMMUTABLE_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";
const F2LLM_VECTOR_PACK_OUT_DIR: &str = ".jbotci-build/r2-web-embeddings-f2llm";
const F2LLM_MODEL_ARTIFACT_ROOT_DIR: &str = ".jbotci-build/f2llm-webgpu-models";
const F2LLM_ONNX_FALLBACK_R2_PREFIX: &str = "models/f2llm-v2-80m-onnx-q4/v1";
const F2LLM_EMBEDDINGS_R2_PREFIX: &str = "embeddings/web/v1";
const F2LLM_REMOTE_CATALOG_URL: &str = "https://assets.jbotci.app/embeddings/web/v1/catalog.json";
const GGUF_VECTOR_PACK_OUT_DIR: &str = ".jbotci-build/r2-gguf-embeddings";
const GGUF_INDEX_STAGE_DIR: &str = ".jbotci-build/native-gguf-index";
const GGUF_EMBEDDINGS_R2_PREFIX: &str = "embeddings/gguf/v1";
const GGUF_REMOTE_CATALOG_URL: &str = "https://assets.jbotci.app/embeddings/gguf/v1/catalog.json";
const GGUF_80M_MODEL_KEY: &str = "f2llm-v2-80m-q4-k-m-320";
const GGUF_160M_MODEL_KEY: &str = "f2llm-v2-160m-q4-k-m-640";
const GGUF_DEFAULT_MODEL_KEY: &str = "f2llm-v2-330m-q4-k-m-896";
const GGUF_0_6B_MODEL_KEY: &str = "f2llm-v2-0.6b-q4-k-m-1024";
const GGUF_MODEL_KEYS: &[&str] = &[
    GGUF_80M_MODEL_KEY,
    GGUF_160M_MODEL_KEY,
    GGUF_DEFAULT_MODEL_KEY,
    GGUF_0_6B_MODEL_KEY,
];
const F2LLM_VECTOR_SPACE_KEY: &str = "jbotci-browser-f2llm-q4-f16-windowed-512-v1";
const F2LLM_MAX_SEQUENCE_LENGTH: usize = 512;
const R2_UPLOAD_PARALLELISM: usize = 4;
const DEFAULT_WIKI_SOURCE_URL: &str = "https://mw.lojban.org";
const DEFAULT_WIKI_OUTPUT_DIR: &str = "vendor/lojban-wiki";
const DEFAULT_WIKI_BATCH_SIZE: usize = 50;
const DEFAULT_WIKI_DELAY_MS: u64 = 1000;
const DEFAULT_WIKI_TIMEOUT_MS: u64 = 120_000;
const DEFAULT_WIKI_RETRIES: usize = 8;
const DEFAULT_WIKI_MAXLAG: usize = 5;
const WIKI_USER_AGENT: &str = "jbotci-wiki-vendor/0.1 (https://codeberg.org/int_19h/jbotci)";
const WIKI_HTTP_BODY_LIMIT_BYTES: u64 = 256 * 1024 * 1024;
const F2LLM_80M_MODEL_KEY: &str = "f2llm-v2-80m-q4-320";
const F2LLM_160M_MODEL_KEY: &str = "f2llm-v2-160m-q4-640";
const F2LLM_330M_MODEL_KEY: &str = "f2llm-v2-330m-q4-896";
const F2LLM_0_6B_MODEL_KEY: &str = "f2llm-v2-0.6b-q4-1024";
const F2LLM_80M_MODEL_ID: &str = "codefuse-ai/F2LLM-v2-80M";
const F2LLM_80M_Q4_ONNX: &str = "/home/int19h.linux/git/jbotci-f2llm-quant/artifacts/f2llm-v2-80m-q4-hqq32-transformersjs/onnx/model_q4.onnx";
const F2LLM_80M_DIMENSIONS: usize = 320;
const F2LLM_MODEL_SPECS: &[F2LlmAssetSpec] = &[
    F2LlmAssetSpec {
        id: "80m",
        model_key: F2LLM_80M_MODEL_KEY,
        model_id: F2LLM_80M_MODEL_ID,
        q4_onnx: F2LLM_80M_Q4_ONNX,
        dimensions: F2LLM_80M_DIMENSIONS,
        webgpu_artifact_dir_name: "f2llm-v2-80m-webgpu",
        webgpu_r2_prefix: "models/f2llm-v2-80m-webgpu/v1",
        include_wasm_runtime: true,
    },
    F2LlmAssetSpec {
        id: "160m",
        model_key: F2LLM_160M_MODEL_KEY,
        model_id: "codefuse-ai/F2LLM-v2-160M",
        q4_onnx: "/home/int19h.linux/git/jbotci-f2llm-quant/artifacts/f2llm-v2-160m-q4-640-q4-hqq32-transformersjs/onnx/model_q4.onnx",
        dimensions: 640,
        webgpu_artifact_dir_name: "f2llm-v2-160m-webgpu",
        webgpu_r2_prefix: "models/f2llm-v2-160m-webgpu/v1",
        include_wasm_runtime: false,
    },
    F2LlmAssetSpec {
        id: "330m",
        model_key: F2LLM_330M_MODEL_KEY,
        model_id: "codefuse-ai/F2LLM-v2-330M",
        q4_onnx: "/home/int19h.linux/git/jbotci-f2llm-quant/artifacts/f2llm-v2-330m-q4-896-q4-hqq32-transformersjs/onnx/model_q4.onnx",
        dimensions: 896,
        webgpu_artifact_dir_name: "f2llm-v2-330m-webgpu",
        webgpu_r2_prefix: "models/f2llm-v2-330m-webgpu/v1",
        include_wasm_runtime: false,
    },
    F2LlmAssetSpec {
        id: "0.6b",
        model_key: F2LLM_0_6B_MODEL_KEY,
        model_id: "codefuse-ai/F2LLM-v2-0.6B",
        q4_onnx: "/home/int19h.linux/git/jbotci-f2llm-quant/artifacts/f2llm-v2-0_6b-q4-1024-q4-hqq32-transformersjs/onnx/model_q4.onnx",
        dimensions: 1024,
        webgpu_artifact_dir_name: "f2llm-v2-0.6b-webgpu",
        webgpu_r2_prefix: "models/f2llm-v2-0.6b-webgpu/v1",
        include_wasm_runtime: false,
    },
];
static WEB_ASSET_COPY_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[path = "../../tests/support/fixtures/mod.rs"]
mod fixtures;

use fixtures::{
    ExpectationStatus, Facet, FacetResult, FixtureBackend, FixtureProfile, FixtureSelector,
    LoadedTestCase, MuplisForm, Provenance, RunSummary, fixture_matches_selector, fixture_paths,
    import_export_file, load_fixture_path, load_profile, validate_fixture_tree, visit_fixture_tree,
    write_fixture_file,
};

#[derive(Debug, Clone, Copy)]
#[invariant(true)]
struct F2LlmAssetSpec {
    id: &'static str,
    model_key: &'static str,
    model_id: &'static str,
    q4_onnx: &'static str,
    dimensions: usize,
    webgpu_artifact_dir_name: &'static str,
    webgpu_r2_prefix: &'static str,
    include_wasm_runtime: bool,
}

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
#[invariant(::VendorWiki(..) => true)]
#[invariant(::BuildWebRelease(..) => true)]
#[invariant(::ExportWebEmbeddingCorpus(..) => true)]
#[invariant(::BuildWebEmbeddings(..) => true)]
#[invariant(::BuildF2LlmWebgpuModel(..) => true)]
#[invariant(::BuildF2LlmWebgpuVectors(..) => true)]
#[invariant(::BuildGgufEmbeddings(..) => true)]
#[invariant(::DistServer(..) => true)]
#[invariant(::PublishWebEmbeddingsR2(..) => true)]
#[invariant(::PublishF2LlmWebgpuR2(..) => true)]
#[invariant(::PublishGgufEmbeddingsR2(..) => true)]
#[invariant(::RenderDockerBuild(..) => true)]
#[invariant(::RenderDockerRun(..) => true)]
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
    VendorWiki(VendorWikiArgs),
    BuildWebRelease(BuildWebReleaseArgs),
    ExportWebEmbeddingCorpus(ExportWebEmbeddingCorpusArgs),
    BuildWebEmbeddings(BuildWebEmbeddingsArgs),
    #[command(name = "build-f2llm-webgpu-model")]
    BuildF2LlmWebgpuModel(BuildF2LlmWebgpuModelArgs),
    #[command(name = "build-f2llm-webgpu-vectors")]
    BuildF2LlmWebgpuVectors(BuildF2LlmWebgpuVectorsArgs),
    #[command(name = "build-gguf-embeddings")]
    BuildGgufEmbeddings(BuildGgufEmbeddingsArgs),
    DistServer(DistServerArgs),
    PublishWebEmbeddingsR2(PublishWebEmbeddingsR2Args),
    #[command(name = "publish-f2llm-webgpu-r2")]
    PublishF2LlmWebgpuR2(PublishF2LlmWebgpuR2Args),
    #[command(name = "publish-gguf-embeddings-r2")]
    PublishGgufEmbeddingsR2(PublishGgufEmbeddingsR2Args),
    RenderDockerBuild(RenderDockerBuildArgs),
    RenderDockerRun(RenderDockerRunArgs),
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
    #[arg(long, value_name = "N")]
    failure_samples: Option<usize>,
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
struct VendorWikiArgs {
    #[arg(long, default_value = DEFAULT_WIKI_SOURCE_URL)]
    source_url: String,
    #[arg(long)]
    api_url: Option<String>,
    #[arg(long)]
    rest_url: Option<String>,
    #[arg(long, visible_alias = "output-dir", default_value = DEFAULT_WIKI_OUTPUT_DIR)]
    output: PathBuf,
    #[arg(long, default_value_t = DEFAULT_WIKI_BATCH_SIZE, value_name = "N")]
    batch_size: usize,
    #[arg(long, default_value_t = DEFAULT_WIKI_DELAY_MS, value_name = "MS")]
    delay_ms: u64,
    #[arg(long, default_value_t = DEFAULT_WIKI_TIMEOUT_MS, value_name = "MS")]
    timeout_ms: u64,
    #[arg(long, default_value_t = DEFAULT_WIKI_RETRIES, value_name = "N")]
    retries: usize,
    #[arg(long, default_value_t = DEFAULT_WIKI_MAXLAG, value_name = "SECONDS")]
    maxlag: usize,
    #[arg(long, default_value = WIKI_USER_AGENT)]
    user_agent: String,
    #[arg(long)]
    check: bool,
    #[arg(long, hide = true, value_name = "N")]
    limit_pages: Option<usize>,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct BuildWebReleaseArgs {
    #[arg(long)]
    base_path: Option<String>,
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
struct BuildF2LlmWebgpuModelArgs {
    #[arg(long, default_value = F2LLM_80M_Q4_ONNX)]
    q4_onnx: PathBuf,
    #[arg(long, default_value = F2LLM_80M_MODEL_KEY)]
    model_key: String,
    #[arg(long, default_value = F2LLM_80M_MODEL_ID)]
    model_id: String,
    #[arg(long)]
    model_root: Option<PathBuf>,
    #[arg(
        long,
        default_value = ".jbotci-build/f2llm-webgpu-models/f2llm-v2-80m-webgpu/v1"
    )]
    out_dir: PathBuf,
    #[arg(long)]
    stage: Option<PathBuf>,
    #[arg(long, default_value_t = 4 * 1024 * 1024)]
    shard_size: usize,
    #[arg(long, default_value = "python3")]
    python: String,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct BuildF2LlmWebgpuVectorsArgs {
    #[arg(long, default_value = F2LLM_80M_Q4_ONNX)]
    q4_onnx: PathBuf,
    #[arg(long, default_value = F2LLM_80M_MODEL_KEY)]
    model_key: String,
    #[arg(long, default_value = F2LLM_80M_MODEL_ID)]
    model_id: String,
    #[arg(long, default_value_t = F2LLM_80M_DIMENSIONS)]
    dimensions: usize,
    #[arg(long)]
    include_wasm_runtime: bool,
    #[arg(long)]
    tokenizer_dir: Option<PathBuf>,
    #[arg(long, default_value = F2LLM_VECTOR_PACK_OUT_DIR)]
    out_dir: PathBuf,
    #[arg(long)]
    stage: Option<PathBuf>,
    #[arg(long)]
    corpus: Option<PathBuf>,
    #[arg(long, default_value_t = 8)]
    batch_size: usize,
    #[arg(long, default_value = "python3")]
    python: String,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct BuildGgufEmbeddingsArgs {
    #[arg(long, default_value = GGUF_VECTOR_PACK_OUT_DIR)]
    out_dir: PathBuf,
    #[arg(long, default_value = GGUF_INDEX_STAGE_DIR)]
    index_dir: PathBuf,
    #[arg(long)]
    model_dir: Option<PathBuf>,
    #[arg(long = "model")]
    models: Vec<String>,
    #[arg(long)]
    skip_validation: bool,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct DistServerArgs {
    #[arg(long, default_value = ".jbotci-build/jbotci-web")]
    out_dir: PathBuf,
    #[arg(long, default_value = "/")]
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

#[derive(Debug, Args)]
#[invariant(true)]
struct PublishWebEmbeddingsR2Args {
    #[arg(long, default_value = "jbotci-web-assets")]
    bucket: String,
    #[arg(long, default_value = "embeddings/web/v1")]
    prefix: String,
    #[arg(long, default_value = ".jbotci-build/r2-web-embeddings")]
    out_dir: PathBuf,
    #[arg(long)]
    corpus: Option<PathBuf>,
    #[arg(long = "embedding-dtype", alias = "dtype", default_values_t = ["q4".to_owned(), "q8".to_owned()])]
    embedding_dtypes: Vec<String>,
    #[arg(long, default_value = "transformers")]
    backend: String,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct PublishF2LlmWebgpuR2Args {
    #[arg(long, default_value = "jbotci-web-assets")]
    bucket: String,
    #[arg(long, default_value = F2LLM_EMBEDDINGS_R2_PREFIX)]
    embedding_prefix: String,
    #[arg(long, default_value = F2LLM_MODEL_ARTIFACT_ROOT_DIR)]
    model_out_root: PathBuf,
    #[arg(long, default_value = F2LLM_VECTOR_PACK_OUT_DIR)]
    vector_out_dir: PathBuf,
    #[arg(long)]
    corpus: Option<PathBuf>,
    #[arg(long)]
    tokenizer_dir: Option<PathBuf>,
    #[arg(long, default_value_t = 8)]
    batch_size: usize,
    #[arg(long, default_value = "python3")]
    python: String,
    #[arg(long, default_value = F2LLM_REMOTE_CATALOG_URL)]
    remote_catalog_url: String,
    #[arg(long)]
    skip_build: bool,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct PublishGgufEmbeddingsR2Args {
    #[arg(long, default_value = "jbotci-web-assets")]
    bucket: String,
    #[arg(long, default_value = GGUF_EMBEDDINGS_R2_PREFIX)]
    prefix: String,
    #[arg(long, default_value = GGUF_VECTOR_PACK_OUT_DIR)]
    out_dir: PathBuf,
    #[arg(long, default_value = GGUF_INDEX_STAGE_DIR)]
    index_dir: PathBuf,
    #[arg(long)]
    model_dir: Option<PathBuf>,
    #[arg(long = "model")]
    models: Vec<String>,
    #[arg(long, default_value = GGUF_REMOTE_CATALOG_URL)]
    remote_catalog_url: String,
    #[arg(long)]
    skip_build: bool,
    #[arg(long)]
    skip_validation: bool,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct RenderDockerBuildArgs {
    #[arg(long, value_enum, default_value = "auto")]
    engine: ContainerEngineArg,
    #[arg(long, default_value = "jbotci-render:local")]
    image: String,
    #[arg(long, default_value = "/")]
    base_path: String,
    #[arg(long, default_value = "https://assets.jbotci.app/embeddings/web/v1")]
    web_embeddings_base_url: String,
    #[arg(long)]
    no_cache: bool,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct RenderDockerRunArgs {
    #[arg(long, value_enum, default_value = "auto")]
    engine: ContainerEngineArg,
    #[arg(long, default_value = "jbotci-render:local")]
    image: String,
    #[arg(long, default_value_t = 8080)]
    host_port: u16,
    #[arg(long, default_value_t = 10000)]
    container_port: u16,
    #[arg(long, default_value = "/")]
    base_path: String,
    #[arg(long, default_value = "https://assets.jbotci.app/embeddings/web/v1")]
    web_embeddings_base_url: String,
    #[arg(long)]
    no_build: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[invariant(true)]
#[invariant(::Auto => true)]
#[invariant(::Docker => true)]
#[invariant(::Podman => true)]
enum ContainerEngineArg {
    Auto,
    Docker,
    Podman,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Docker => true)]
#[invariant(::Podman => true)]
enum ContainerEngine {
    Docker,
    Podman,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct R2UploadObject {
    local_path: PathBuf,
    object_key: String,
    content_type: &'static str,
    content_encoding: Option<&'static str>,
    cache_control: &'static str,
}

impl ContainerEngineArg {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn resolve(self) -> Result<ContainerEngine> {
        match self {
            Self::Docker => Ok(ContainerEngine::Docker),
            Self::Podman => Ok(ContainerEngine::Podman),
            Self::Auto => {
                if container_engine_available(ContainerEngine::Docker.command_name()) {
                    Ok(ContainerEngine::Docker)
                } else if container_engine_available(ContainerEngine::Podman.command_name()) {
                    Ok(ContainerEngine::Podman)
                } else {
                    bail!("could not find `docker` or `podman` in PATH")
                }
            }
        }
    }
}

impl ContainerEngine {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn command_name(self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
        }
    }
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

#[derive(Debug, Clone)]
#[invariant(true)]
struct WikiVendorConfig {
    source_url: String,
    api_url: String,
    rest_url: String,
    output: PathBuf,
    batch_size: usize,
    delay: Duration,
    timeout: Duration,
    retries: usize,
    maxlag: usize,
    user_agent: String,
    check: bool,
    limit_pages: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[invariant(true)]
struct WikiNamespace {
    id: i64,
    name: String,
    canonical: Option<String>,
    content: bool,
    case: Option<String>,
    subpages: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[invariant(true)]
struct WikiRevisionMetadata {
    revid: u64,
    parentid: Option<u64>,
    timestamp: String,
    user: Option<String>,
    userid: Option<u64>,
    comment: String,
    size: Option<u64>,
    sha1: Option<String>,
    contentmodel: String,
    contentformat: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[invariant(true)]
struct WikiPageMetadataFile {
    pageid: u64,
    ns: i64,
    title: String,
    touched: Option<String>,
    lastrevid: Option<u64>,
    length: Option<u64>,
    redirect: bool,
    protection: serde_json::Value,
    revision: Option<WikiRevisionMetadata>,
    source_sha256: String,
    parsoid_html_sha256: String,
    source_path: String,
    parsoid_html_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[invariant(true)]
struct WikiPageRemoteMetadata {
    pageid: u64,
    ns: i64,
    title: String,
    touched: Option<String>,
    lastrevid: Option<u64>,
    length: Option<u64>,
    redirect: bool,
    protection: serde_json::Value,
    revision: Option<WikiRevisionMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[invariant(true)]
struct WikiPageIndexEntry {
    pageid: u64,
    ns: i64,
    title: String,
    redirect: bool,
    revid: Option<u64>,
    timestamp: Option<String>,
    model: String,
    bytes: usize,
    #[serde(default)]
    source_sha256: String,
    #[serde(default)]
    parsoid_html_sha256: String,
    meta: String,
    source: String,
    #[serde(default)]
    parsoid_html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[invariant(true)]
struct WikiPageFetchError {
    pageid: u64,
    ns: i64,
    title: String,
    lastrevid: Option<u64>,
    error: String,
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct WikiFetchedPage {
    pageid: u64,
    ns: i64,
    title: String,
    redirect: bool,
    touched: Option<String>,
    lastrevid: Option<u64>,
    length: Option<u64>,
    protection: serde_json::Value,
    revision: WikiRevisionMetadata,
    source: String,
    parsoid_html: String,
    source_sha256: String,
    parsoid_html_sha256: String,
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct WikiFetchedSourceRevision {
    pageid: u64,
    ns: i64,
    title: String,
    touched: Option<String>,
    lastrevid: Option<u64>,
    length: Option<u64>,
    redirect: bool,
    protection: serde_json::Value,
    revision: WikiRevisionMetadata,
    source: String,
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct WikiParsoidHtml {
    revid: u64,
    html: String,
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct WikiRecentChangesSummary {
    changed_pageids: BTreeSet<u64>,
    needs_full_reconcile: bool,
    change_count: usize,
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct WikiVendorReport {
    checked: bool,
    output: PathBuf,
    pages: usize,
    fetched: usize,
    kept: usize,
    failed: usize,
    removed: usize,
    media_files: usize,
    source_bytes: usize,
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct WikiPageWriteReport {
    entry: WikiPageIndexEntry,
    source_bytes: usize,
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct WikiPagePaths {
    base: PathBuf,
    meta: PathBuf,
    source: PathBuf,
    parsoid_html: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct DirectoryDigestRow {
    digest: String,
    path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct UtcDate {
    year: i64,
    month: u32,
    day: u32,
}

#[derive(Debug, Clone, Copy)]
#[invariant(true)]
struct WikiSnapshotMetadataRenderArgs<'a> {
    config: &'a WikiVendorConfig,
    siteinfo: &'a serde_json::Value,
    started_at: &'a str,
    finished_at: &'a str,
    plan: &'a WikiSnapshotPlan,
    page_count: usize,
    failed_pages: &'a [WikiPageFetchError],
    source_bytes: usize,
    media_manifest: &'a serde_json::Value,
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct WikiExistingSnapshot {
    fetched_at: String,
    pages: Vec<WikiPageIndexEntry>,
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct WikiSnapshotPlan {
    keep: Vec<WikiPageIndexEntry>,
    fetch: Vec<WikiPageRemoteMetadata>,
    removed: Vec<WikiPageIndexEntry>,
    source: WikiSnapshotPlanSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::RecentChanges => true)]
#[invariant(::FullReconcile => true)]
enum WikiSnapshotPlanSource {
    RecentChanges,
    FullReconcile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum WikiProgressPhase {
    Siteinfo,
    Check,
    Planning,
    RecentChanges,
    Metadata,
    Pages,
    Media,
    Writing,
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct WikiProgressUpdate {
    phase: WikiProgressPhase,
    current: usize,
    total: usize,
    detail: String,
}

#[derive(Debug)]
#[invariant(true)]
struct WikiProgressReporter {
    job: Option<std::sync::Arc<clx::progress::ProgressJob>>,
    determinate: bool,
    last_log: Instant,
}

#[derive(Debug)]
#[invariant(true)]
struct WikiHttpClient {
    agent: ureq::Agent,
    api_url: String,
    rest_url: String,
    delay: Duration,
    retries: usize,
    maxlag: usize,
    next_request_at: Option<Instant>,
    retry_count: usize,
}

#[derive(Debug)]
#[invariant(true)]
struct WikiHttpResponse {
    status: u16,
    retry_after: Option<String>,
    body: String,
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
        Command::VendorWiki(args) => vendor_wiki(args),
        Command::BuildWebRelease(args) => build_web_release(args),
        Command::ExportWebEmbeddingCorpus(args) => export_web_embedding_corpus(args),
        Command::BuildWebEmbeddings(args) => build_web_embeddings(args),
        Command::BuildF2LlmWebgpuModel(args) => build_f2llm_webgpu_model(args),
        Command::BuildF2LlmWebgpuVectors(args) => build_f2llm_webgpu_vectors(args),
        Command::BuildGgufEmbeddings(args) => build_gguf_embeddings(args),
        Command::DistServer(args) => dist_server(args),
        Command::PublishWebEmbeddingsR2(args) => publish_web_embeddings_r2(args),
        Command::PublishF2LlmWebgpuR2(args) => publish_f2llm_webgpu_r2(args),
        Command::PublishGgufEmbeddingsR2(args) => publish_gguf_embeddings_r2(args),
        Command::RenderDockerBuild(args) => render_docker_build(args),
        Command::RenderDockerRun(args) => render_docker_run(args),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn build_web_release(args: BuildWebReleaseArgs) -> Result<()> {
    clean_dioxus_web_release_output()?;
    prepare_dioxus_web_public_input()?;
    let mut command = dx_web_release_command("build");
    if let Some(base_path) = args.base_path {
        set_dioxus_base_path_env(&mut command, &base_path);
        command.arg("--base-path").arg(base_path);
    }
    let status = command.status().context("failed to run `dx build`")?;
    check_status(
        status,
        "dx build --web --release --debug-symbols=false --inject-loading-scripts=false",
    )?;
    write_release_service_worker(&Path::new(DIOXUS_WEB_RELEASE_DIR).join("public"))
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

#[requires(!args.python.trim().is_empty())]
#[requires(args.shard_size > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn build_f2llm_webgpu_model(args: BuildF2LlmWebgpuModelArgs) -> Result<()> {
    let q4_onnx = absolute_path(&args.q4_onnx)?;
    if !q4_onnx.is_file() {
        bail!("F2LLM q4 ONNX model `{}` does not exist", q4_onnx.display());
    }
    let out_dir = absolute_path(&args.out_dir)?;
    let mut command = ProcessCommand::new(&args.python);
    command
        .arg("tools/embedding-pack/f2llm/export-webgpu-from-onnx-q4.py")
        .arg("--onnx-model")
        .arg(&q4_onnx)
        .arg("--model-key")
        .arg(&args.model_key)
        .arg("--source-model")
        .arg(&args.model_id)
        .arg("--out")
        .arg(&out_dir)
        .arg("--shard-size")
        .arg(args.shard_size.to_string());
    if let Some(model_root) = args.model_root {
        command.arg("--model-root").arg(absolute_path(&model_root)?);
    }
    if let Some(stage) = args.stage {
        command.arg("--stage").arg(absolute_path(&stage)?);
    }
    let status = command.status().with_context(|| {
        format!(
            "failed to build F2LLM WebGPU artifact at `{}`",
            out_dir.display()
        )
    })?;
    check_status(
        status,
        "python3 tools/embedding-pack/f2llm/export-webgpu-from-onnx-q4.py",
    )
}

#[requires(!args.python.trim().is_empty())]
#[requires(args.batch_size > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn build_f2llm_webgpu_vectors(args: BuildF2LlmWebgpuVectorsArgs) -> Result<()> {
    let q4_onnx = absolute_path(&args.q4_onnx)?;
    let out_dir = absolute_path(&args.out_dir)?;
    let corpus = ensure_web_embedding_corpus(args.corpus.as_deref())?;
    run_f2llm_vector_builder(
        &args.python,
        &q4_onnx,
        args.tokenizer_dir.as_deref(),
        &out_dir,
        args.stage.as_deref(),
        &corpus,
        args.batch_size,
        &args.model_key,
        &args.model_id,
        args.dimensions,
        args.include_wasm_runtime,
    )?;
    run_f2llm_vector_validator(
        &args.python,
        &q4_onnx,
        args.tokenizer_dir.as_deref(),
        &out_dir,
        &corpus,
        &args.model_key,
        args.dimensions,
        args.include_wasm_runtime,
    )
}

#[requires(!args.bucket.trim().is_empty())]
#[requires(!args.embedding_prefix.trim().is_empty())]
#[requires(!args.python.trim().is_empty())]
#[requires(args.batch_size > 0)]
#[requires(!args.remote_catalog_url.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn publish_f2llm_webgpu_r2(args: PublishF2LlmWebgpuR2Args) -> Result<()> {
    let model_out_root = absolute_path(&args.model_out_root)?;
    let vector_out_dir = absolute_path(&args.vector_out_dir)?;
    let corpus = ensure_web_embedding_corpus(args.corpus.as_deref())?;
    if !args.skip_build {
        build_all_f2llm_webgpu_assets(
            &args.python,
            &model_out_root,
            &vector_out_dir,
            &corpus,
            args.tokenizer_dir.as_deref(),
            args.batch_size,
        )?;
        build_f2llm_onnx_fallback_asset(&model_out_root)?;
    } else {
        validate_all_f2llm_vector_packs(
            &args.python,
            &vector_out_dir,
            &corpus,
            args.tokenizer_dir.as_deref(),
        )?;
    }

    for spec in F2LLM_MODEL_SPECS {
        let model_dir = f2llm_model_artifact_out_dir(&model_out_root, spec);
        let model_objects = r2_upload_tree_objects(&model_dir, spec.webgpu_r2_prefix)?;
        put_r2_objects(&args.bucket, &model_objects)?;
    }

    let onnx_fallback_dir = f2llm_onnx_fallback_out_dir(&model_out_root);
    let onnx_objects = r2_upload_tree_objects(&onnx_fallback_dir, F2LLM_ONNX_FALLBACK_R2_PREFIX)?;
    put_r2_objects(&args.bucket, &onnx_objects)?;

    let vector_objects =
        r2_upload_objects_without_catalog(&vector_out_dir, &args.embedding_prefix)?;
    put_r2_objects(&args.bucket, &vector_objects)?;

    let merged_catalog_dir = absolute_path(Path::new(".jbotci-build/r2-f2llm-merged-catalog"))?;
    fs::create_dir_all(&merged_catalog_dir)
        .with_context(|| format!("creating `{}`", merged_catalog_dir.display()))?;
    let remote_catalog = serde_json::from_str::<serde_json::Value>(
        &fetch_text(&args.remote_catalog_url).with_context(|| {
            format!("fetching remote catalog from `{}`", args.remote_catalog_url)
        })?,
    )
    .with_context(|| format!("parsing remote catalog from `{}`", args.remote_catalog_url))?;
    let local_catalog = read_json_file(&vector_out_dir.join("catalog.json"))?;
    let f2llm_model_keys = F2LLM_MODEL_SPECS
        .iter()
        .map(|spec| spec.model_key.to_owned())
        .collect::<BTreeSet<_>>();
    let merged_catalog =
        merge_embedding_catalog_models(remote_catalog, local_catalog, &f2llm_model_keys)?;
    let catalog_path = merged_catalog_dir.join("catalog.json");
    write_json_file(&catalog_path, &merged_catalog)?;
    let embedding_prefix = normalize_r2_prefix(&args.embedding_prefix)?;
    let catalog_object =
        r2_upload_object_for_key(&merged_catalog_dir, &embedding_prefix, "catalog.json")?;
    put_r2_object(&args.bucket, &catalog_object)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn build_gguf_embeddings(args: BuildGgufEmbeddingsArgs) -> Result<()> {
    let out_dir = absolute_path(&args.out_dir)?;
    let index_dir = absolute_path(&args.index_dir)?;
    let models = selected_gguf_model_keys(&args.models)?;
    remove_path_if_exists(&out_dir)?;
    fs::create_dir_all(out_dir.join("models"))
        .with_context(|| format!("creating `{}`", out_dir.join("models").display()))?;
    let mut catalog_models = Vec::new();
    for model in &models {
        let model_index_dir = index_dir.join(model);
        remove_path_if_exists(&model_index_dir)?;
        fs::create_dir_all(&model_index_dir)
            .with_context(|| format!("creating `{}`", model_index_dir.display()))?;
        let mut command = ProcessCommand::new("cargo");
        command
            .arg("run")
            .arg("--release")
            .arg("-p")
            .arg("jbotci")
            .arg("--")
            .arg("setup")
            .arg("--embedding")
            .arg("--force")
            .arg("--use-precomputed")
            .arg("never")
            .arg("--model")
            .arg(model)
            .arg("--index-dir")
            .arg(&model_index_dir);
        if let Some(model_dir) = &args.model_dir {
            command.arg("--model-dir").arg(absolute_path(model_dir)?);
        }
        if args.skip_validation {
            command.arg("--skip-validation");
        }
        let status = command
            .status()
            .context("failed to run `cargo run --release -p jbotci -- setup --embedding`")?;
        check_status(
            status,
            "cargo run --release -p jbotci -- setup --embedding --use-precomputed never",
        )?;
        let index_version_dir = model_index_dir.join("v1");
        if !index_version_dir.is_dir() {
            bail!(
                "native embedding setup did not create `{}`",
                index_version_dir.display()
            );
        }
        copy_dir_recursive(
            &index_version_dir.join("models").join(model),
            &out_dir.join("models").join(model),
            "GGUF embedding R2 model tree",
        )?;
        let catalog = read_json_file(&index_version_dir.join("catalog.json"))?;
        let catalog_model = catalog
            .get("models")
            .and_then(serde_json::Value::as_array)
            .and_then(|models| {
                models
                    .iter()
                    .find(|catalog_model| {
                        catalog_model
                            .get("model_key")
                            .and_then(serde_json::Value::as_str)
                            == Some(model.as_str())
                    })
                    .cloned()
            })
            .with_context(|| format!("native embedding catalog is missing model `{model}`"))?;
        catalog_models.push(catalog_model);
    }
    write_json_file(
        &out_dir.join("catalog.json"),
        &serde_json::json!({
            "schema_version": 1,
            "models": catalog_models,
        }),
    )?;
    validate_native_gguf_r2_tree(&out_dir)
}

#[requires(!args.bucket.trim().is_empty())]
#[requires(!args.prefix.trim().is_empty())]
#[requires(!args.remote_catalog_url.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn publish_gguf_embeddings_r2(args: PublishGgufEmbeddingsR2Args) -> Result<()> {
    let out_dir = absolute_path(&args.out_dir)?;
    let selected_models = selected_gguf_model_keys(&args.models)?;
    if !args.skip_build {
        build_gguf_embeddings(BuildGgufEmbeddingsArgs {
            out_dir: args.out_dir.clone(),
            index_dir: args.index_dir,
            model_dir: args.model_dir,
            models: selected_models.to_vec(),
            skip_validation: args.skip_validation,
        })?;
    } else {
        validate_native_gguf_r2_tree(&out_dir)?;
    }
    let pack_objects = r2_upload_native_objects_without_catalog(&out_dir, &args.prefix)?;
    put_r2_objects(&args.bucket, &pack_objects)?;

    let remote_catalog =
        fetch_optional_json_catalog(&args.remote_catalog_url).with_context(|| {
            format!(
                "fetching remote native embedding catalog from `{}`",
                args.remote_catalog_url
            )
        })?;
    let local_catalog = read_json_file(&out_dir.join("catalog.json"))?;
    let local_model_keys = embedding_catalog_model_keys(&local_catalog)?;
    let replacement_keys = if args.models.is_empty() {
        local_model_keys
    } else {
        selected_models.into_iter().collect()
    };
    let merged_catalog =
        merge_embedding_catalog_models(remote_catalog, local_catalog, &replacement_keys)?;
    let merged_catalog_dir = absolute_path(Path::new(".jbotci-build/r2-gguf-merged-catalog"))?;
    fs::create_dir_all(&merged_catalog_dir)
        .with_context(|| format!("creating `{}`", merged_catalog_dir.display()))?;
    let catalog_path = merged_catalog_dir.join("catalog.json");
    write_json_file(&catalog_path, &merged_catalog)?;
    let prefix = normalize_r2_prefix(&args.prefix)?;
    let catalog_object = r2_upload_object_for_key(&merged_catalog_dir, &prefix, "catalog.json")?;
    put_r2_object(&args.bucket, &catalog_object)
}

#[requires(!args.bucket.trim().is_empty())]
#[requires(!args.prefix.trim().is_empty())]
#[requires(!args.embedding_dtypes.is_empty())]
#[requires(!args.backend.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn publish_web_embeddings_r2(args: PublishWebEmbeddingsR2Args) -> Result<()> {
    let output = absolute_path(&args.out_dir)?;
    let corpus = match args.corpus {
        Some(path) => absolute_path(&path)?,
        None => {
            let output = absolute_path(Path::new(".jbotci-build/web-embedding-corpus.json"))?;
            write_web_embedding_corpus(&output)?;
            output
        }
    };
    build_web_embedding_assets_to(&output, &corpus, &args.embedding_dtypes, &args.backend)?;
    let objects = r2_upload_objects(&output, &args.prefix)?;
    put_r2_objects(&args.bucket, &objects)?;
    Ok(())
}

#[requires(matches!(subcommand, "build" | "bundle"))]
#[ensures(true)]
fn dx_web_release_command(subcommand: &str) -> ProcessCommand {
    let mut command = ProcessCommand::new("dx");
    command
        .arg(subcommand)
        .arg("--web")
        .arg("--release")
        .arg("-p")
        .arg("jbotci-app")
        // Dioxus 0.7.x can emit DWARF that makes wasm-opt abort during release web builds.
        .arg("--debug-symbols=false")
        .arg("--inject-loading-scripts=false");
    command
}

#[requires(true)]
#[ensures(true)]
fn set_dioxus_base_path_env(command: &mut ProcessCommand, base_path: &str) {
    if let Some(asset_root) = dioxus_asset_root(base_path) {
        command.env("DIOXUS_ASSET_ROOT", asset_root);
    } else {
        command.env_remove("DIOXUS_ASSET_ROOT");
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|root| root.starts_with('/') && root.len() > 1))]
#[ensures(ret.as_ref().is_none_or(|root| !root.ends_with('/')))]
fn dioxus_asset_root(base_path: &str) -> Option<String> {
    let trimmed = base_path.trim().trim_matches('/');
    if trimmed.is_empty() {
        None
    } else {
        Some(format!("/{trimmed}"))
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn clean_dioxus_web_release_output() -> Result<()> {
    let release_dir = Path::new(DIOXUS_WEB_RELEASE_DIR);
    if release_dir.exists() {
        fs::remove_dir_all(release_dir).with_context(|| {
            format!(
                "removing old Dioxus release web output `{}`",
                release_dir.display()
            )
        })?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn prepare_dioxus_web_public_input() -> Result<()> {
    remove_obsolete_web_public_assets(&dioxus_web_public_input_dir())?;
    copy_stable_web_assets_to_public(&dioxus_web_public_input_dir())
}

#[requires(true)]
#[ensures(!ret.as_os_str().is_empty())]
fn dioxus_web_public_input_dir() -> PathBuf {
    PathBuf::from(DIOXUS_WEB_PUBLIC_INPUT_DIR)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn remove_obsolete_web_public_assets(public_dir: &Path) -> Result<()> {
    remove_obsolete_web_public_dir(public_dir, Path::new("assets/generated"))?;
    remove_obsolete_web_public_file(public_dir, Path::new("manifest.webmanifest"))?;
    remove_obsolete_web_public_file(public_dir, Path::new("assets/manifest.webmanifest"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn remove_obsolete_web_public_dir(public_dir: &Path, relative: &Path) -> Result<()> {
    let path = public_dir.join(relative);
    match fs::remove_dir_all(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "removing obsolete web public asset directory `{}`",
                path.display()
            )
        }),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn remove_obsolete_web_public_file(public_dir: &Path, relative: &Path) -> Result<()> {
    let path = public_dir.join(relative);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "removing obsolete web public asset file `{}`",
                path.display()
            )
        }),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_stable_web_assets_to_public(public_dir: &Path) -> Result<()> {
    copy_stable_web_asset_file(
        public_dir,
        Path::new("manifest.webmanifest"),
        Path::new("assets/manifest.webmanifest"),
    )?;
    copy_stable_web_asset_dir(public_dir, Path::new("icons"), Path::new("assets/icons"))?;
    copy_stable_web_asset_dir(
        public_dir,
        Path::new("cll/media"),
        Path::new("assets/cll/media"),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_stable_web_asset_file(
    public_dir: &Path,
    source_relative: &Path,
    target_relative: &Path,
) -> Result<()> {
    let source = Path::new(SHARED_UI_ASSET_DIR).join(source_relative);
    let target = public_dir.join(target_relative);
    copy_web_asset_file_atomically(&source, &target, "stable web asset")
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_stable_web_asset_dir(
    public_dir: &Path,
    source_relative: &Path,
    target_relative: &Path,
) -> Result<()> {
    let source_dir = Path::new(SHARED_UI_ASSET_DIR).join(source_relative);
    let target_dir = public_dir.join(target_relative);
    copy_flat_web_asset_dir(&source_dir, &target_dir, "stable web asset")
}

#[requires(source_dir.is_dir())]
#[requires(!description.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_flat_web_asset_dir(source_dir: &Path, target_dir: &Path, description: &str) -> Result<()> {
    fs::create_dir_all(target_dir).with_context(|| {
        format!(
            "creating {description} directory `{}`",
            target_dir.display()
        )
    })?;
    let mut source_file_names = BTreeSet::new();
    for entry in fs::read_dir(source_dir)
        .with_context(|| format!("reading {description}s from `{}`", source_dir.display()))?
    {
        let entry = entry
            .with_context(|| format!("reading {description} under `{}`", source_dir.display()))?;
        if !entry
            .file_type()
            .with_context(|| format!("reading file type for `{}`", entry.path().display()))?
            .is_file()
        {
            continue;
        }
        let file_name = entry.file_name();
        source_file_names.insert(file_name.clone());
        let target = target_dir.join(file_name);
        copy_web_asset_file_atomically(&entry.path(), &target, description)?;
    }
    remove_obsolete_flat_web_asset_files(target_dir, &source_file_names, description)
}

#[requires(source.is_file())]
#[requires(!description.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_web_asset_file_atomically(source: &Path, target: &Path, description: &str) -> Result<()> {
    let parent = target
        .parent()
        .with_context(|| format!("{description} target `{}` has no parent", target.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating {description} directory `{}`", parent.display()))?;
    let temp_dir = web_asset_sync_temp_dir(target);
    fs::create_dir_all(&temp_dir).with_context(|| {
        format!(
            "creating temporary {description} directory `{}`",
            temp_dir.display()
        )
    })?;
    let file_name = target.file_name().with_context(|| {
        format!(
            "{description} target `{}` has no file name",
            target.display()
        )
    })?;
    let temp_path = temp_dir.join(format!(
        "{}-{}-{}.tmp",
        std::process::id(),
        WEB_ASSET_COPY_COUNTER.fetch_add(1, Ordering::Relaxed),
        file_name.to_string_lossy()
    ));
    fs::copy(source, &temp_path).with_context(|| {
        format!(
            "copying {description} `{}` to temporary file `{}`",
            source.display(),
            temp_path.display()
        )
    })?;
    match fs::rename(&temp_path, target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            fs::remove_file(target).with_context(|| {
                format!(
                    "removing old {description} `{}` before replace",
                    target.display()
                )
            })?;
            fs::rename(&temp_path, target).with_context(|| {
                format!(
                    "moving temporary {description} `{}` to `{}`",
                    temp_path.display(),
                    target.display()
                )
            })
        }
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            Err(error).with_context(|| {
                format!(
                    "moving temporary {description} `{}` to `{}`",
                    temp_path.display(),
                    target.display()
                )
            })
        }
    }
}

#[requires(!contents.is_empty())]
#[requires(!description.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn write_web_asset_text_atomically(target: &Path, contents: &str, description: &str) -> Result<()> {
    let parent = target
        .parent()
        .with_context(|| format!("{description} target `{}` has no parent", target.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("creating {description} directory `{}`", parent.display()))?;
    let temp_dir = web_asset_sync_temp_dir(target);
    fs::create_dir_all(&temp_dir).with_context(|| {
        format!(
            "creating temporary {description} directory `{}`",
            temp_dir.display()
        )
    })?;
    let file_name = target.file_name().with_context(|| {
        format!(
            "{description} target `{}` has no file name",
            target.display()
        )
    })?;
    let temp_path = temp_dir.join(format!(
        "{}-{}-{}.tmp",
        std::process::id(),
        WEB_ASSET_COPY_COUNTER.fetch_add(1, Ordering::Relaxed),
        file_name.to_string_lossy()
    ));
    fs::write(&temp_path, contents).with_context(|| {
        format!(
            "writing {description} temporary file `{}`",
            temp_path.display()
        )
    })?;
    match fs::rename(&temp_path, target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            fs::remove_file(target).with_context(|| {
                format!(
                    "removing old {description} `{}` before replace",
                    target.display()
                )
            })?;
            fs::rename(&temp_path, target).with_context(|| {
                format!(
                    "moving temporary {description} `{}` to `{}`",
                    temp_path.display(),
                    target.display()
                )
            })
        }
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            Err(error).with_context(|| {
                format!(
                    "moving temporary {description} `{}` to `{}`",
                    temp_path.display(),
                    target.display()
                )
            })
        }
    }
}

#[requires(true)]
#[ensures(!ret.as_os_str().is_empty())]
fn web_asset_sync_temp_dir(target: &Path) -> PathBuf {
    let public_input_dir = dioxus_web_public_input_dir();
    if target.starts_with(&public_input_dir) {
        return PathBuf::from(WEB_ASSET_SYNC_TEMP_DIR);
    }
    target
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".jbotci-asset-sync")
}

#[requires(!description.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn remove_obsolete_flat_web_asset_files(
    target_dir: &Path,
    source_file_names: &BTreeSet<std::ffi::OsString>,
    description: &str,
) -> Result<()> {
    let entries = match fs::read_dir(target_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "reading {description} target directory `{}`",
                    target_dir.display()
                )
            });
        }
    };
    for entry in entries {
        let entry = entry
            .with_context(|| format!("reading {description} under `{}`", target_dir.display()))?;
        if source_file_names.contains(&entry.file_name()) {
            continue;
        }
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("reading file type for `{}`", entry.path().display())
                });
            }
        };
        if file_type.is_file() || file_type.is_symlink() {
            match fs::remove_file(entry.path()) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!(
                            "removing obsolete {description} `{}`",
                            entry.path().display()
                        )
                    });
                }
            }
        }
    }
    Ok(())
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
    server_bundle_path(&out_dir).map(|_| ())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_dx_bundle(out_dir: &Path, base_path: &str) -> Result<()> {
    clean_dioxus_web_release_output()?;
    prepare_dioxus_web_public_input()?;
    let mut command = ProcessCommand::new("dx");
    set_dioxus_base_path_env(&mut command, base_path);
    command
        .arg("bundle")
        .arg("--out-dir")
        .arg(out_dir)
        .arg("@client")
        .arg("--web")
        .arg("-p")
        .arg("jbotci-app")
        .arg("--release")
        .arg("--debug-symbols=false")
        .arg("--inject-loading-scripts=false")
        .arg("--base-path")
        .arg(base_path)
        .arg("@server")
        .arg("--server")
        .arg("-p")
        .arg("jbotci-server")
        .arg("--release");
    let status = command.status().context("failed to run `dx bundle`")?;
    check_status(
        status,
        "dx bundle @client --web -p jbotci-app --release @server --server -p jbotci-server --release",
    )?;
    let web_dist = web_dist_dir(out_dir)?;
    write_release_service_worker(&web_dist)?;
    server_bundle_path(out_dir)?;
    Ok(())
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

#[requires(out_dir.is_absolute())]
#[ensures(ret.as_ref().is_ok_and(|path| path.is_file()) || ret.is_err())]
fn server_bundle_path(out_dir: &Path) -> Result<PathBuf> {
    let server = out_dir.join("server");
    if server.is_file() {
        Ok(server)
    } else {
        bail!(
            "could not find Dioxus server bundle executable at `{}`",
            server.display()
        )
    }
}

#[requires(!args.image.trim().is_empty())]
#[requires(!args.web_embeddings_base_url.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_docker_build(args: RenderDockerBuildArgs) -> Result<()> {
    let engine = args.engine.resolve()?;
    let engine_command = engine.command_name();
    let git_commit = current_git_commit()?;
    let mut command = ProcessCommand::new(engine_command);
    command.arg("build");
    if args.no_cache {
        command.arg("--no-cache");
    }
    command
        .arg("-f")
        .arg("deploy/render/Dockerfile")
        .arg("-t")
        .arg(&args.image)
        .arg("--build-arg")
        .arg(format!("BASE_PATH={}", args.base_path))
        .arg("--build-arg")
        .arg(format!(
            "WEB_EMBEDDINGS_BASE_URL={}",
            args.web_embeddings_base_url
        ))
        .arg("--build-arg")
        .arg(format!("RENDER_GIT_COMMIT={git_commit}"))
        .arg(".");
    let status = command
        .status()
        .context("failed to run Render Docker build")?;
    check_status(
        status,
        &format!("{engine_command} build -f deploy/render/Dockerfile"),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|commit| is_git_commit_hash(commit)) || ret.is_err())]
fn current_git_commit() -> Result<String> {
    let output = ProcessCommand::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .context("failed to run `git rev-parse HEAD` for Render Docker build")?;
    if !output.status.success() {
        bail!("`git rev-parse HEAD` failed while preparing Render Docker build");
    }
    let commit = String::from_utf8(output.stdout)
        .context("`git rev-parse HEAD` did not return UTF-8 output")?
        .trim()
        .to_owned();
    if !is_git_commit_hash(&commit) {
        bail!(
            "`git rev-parse HEAD` returned `{commit}`, expected a 40-character hexadecimal Git commit hash"
        );
    }
    Ok(commit)
}

#[requires(true)]
#[ensures(true)]
fn is_git_commit_hash(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|character| character.is_ascii_hexdigit())
}

#[requires(!args.image.trim().is_empty())]
#[requires(!args.web_embeddings_base_url.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_docker_run(args: RenderDockerRunArgs) -> Result<()> {
    if !args.no_build {
        render_docker_build(RenderDockerBuildArgs {
            engine: args.engine,
            image: args.image.clone(),
            base_path: args.base_path.clone(),
            web_embeddings_base_url: args.web_embeddings_base_url.clone(),
            no_cache: false,
        })?;
    }
    let engine = args.engine.resolve()?;
    let engine_command = engine.command_name();
    let host_url = format!("http://127.0.0.1:{}", args.host_port);
    println!("running {} on {}", args.image, host_url);
    let status = ProcessCommand::new(engine_command)
        .arg("run")
        .arg("--rm")
        .arg("-p")
        .arg(format!(
            "127.0.0.1:{}:{}",
            args.host_port, args.container_port
        ))
        .arg("-e")
        .arg("IP=0.0.0.0")
        .arg("-e")
        .arg(format!("PORT={}", args.container_port))
        .arg("-e")
        .arg(format!(
            "DIOXUS_ASSET_ROOT={}",
            dioxus_runtime_asset_root(&args.base_path)
        ))
        .arg("-e")
        .arg("DIOXUS_PUBLIC_PATH=/opt/jbotci/public")
        .arg("-e")
        .arg(format!(
            "JBOTCI_WEB_EMBEDDINGS_BASE_URL={}",
            args.web_embeddings_base_url
        ))
        .arg(&args.image)
        .status()
        .context("failed to run Render Docker image")?;
    check_status(status, &format!("{engine_command} run jbotci-render"))
}

#[requires(!command_name.trim().is_empty())]
#[ensures(true)]
fn container_engine_available(command_name: &str) -> bool {
    ProcessCommand::new(command_name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[requires(true)]
#[ensures(ret.starts_with('/'))]
fn dioxus_runtime_asset_root(base_path: &str) -> String {
    let trimmed = base_path.trim().trim_matches('/');
    if trimmed.is_empty() {
        "/".to_owned()
    } else {
        format!("/{trimmed}")
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn write_release_service_worker(public_dir: &Path) -> Result<()> {
    let precache_paths = release_service_worker_precache_paths(public_dir)?;
    let cache_version = release_service_worker_cache_version(public_dir, &precache_paths)?;
    let contents = render_release_service_worker(&cache_version, &precache_paths)?;
    write_web_asset_text_atomically(
        &public_dir.join(RELEASE_SERVICE_WORKER_FILE_NAME),
        &contents,
        "release service worker",
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|paths| paths.iter().all(|path| !path.starts_with('/'))) || ret.is_err())]
#[ensures(ret.as_ref().is_ok_and(|paths| paths.windows(2).all(|pair| pair[0] <= pair[1])) || ret.is_err())]
fn release_service_worker_precache_paths(public_dir: &Path) -> Result<Vec<String>> {
    if !public_dir.is_dir() {
        bail!(
            "release web public directory `{}` does not exist",
            public_dir.display()
        );
    }
    let mut paths = Vec::new();
    for entry in WalkDir::new(public_dir) {
        let entry = entry
            .with_context(|| format!("walking release web output `{}`", public_dir.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = web_relative_asset_path(public_dir, entry.path())?;
        if release_service_worker_should_precache(&relative) {
            paths.push(relative);
        }
    }
    paths.sort();
    Ok(paths)
}

#[requires(root.is_dir())]
#[requires(path.is_file())]
#[ensures(ret.as_ref().is_ok_and(|path| !path.is_empty() && !path.starts_with('/')) || ret.is_err())]
fn web_relative_asset_path(root: &Path, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(root).with_context(|| {
        format!(
            "making `{}` relative to `{}`",
            path.display(),
            root.display()
        )
    })?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            std::path::Component::Normal(part) => {
                let text = part.to_str().with_context(|| {
                    format!("release web asset path `{}` is not utf-8", path.display())
                })?;
                parts.push(text);
            }
            _ => bail!(
                "release web asset path `{}` is not normalized",
                path.display()
            ),
        }
    }
    if parts.is_empty() {
        bail!(
            "release web asset path `{}` has no relative components",
            path.display()
        );
    }
    Ok(parts.join("/"))
}

#[requires(!path.is_empty())]
#[ensures(true)]
fn release_service_worker_should_precache(path: &str) -> bool {
    path != RELEASE_SERVICE_WORKER_FILE_NAME
        && !path.ends_with(".br")
        && !path.ends_with(".gz")
        && !path.ends_with(".map")
        && !path.starts_with("assets/embeddings/")
}

#[requires(public_dir.is_dir())]
#[requires(paths.iter().all(|path| !path.is_empty() && !path.starts_with('/')))]
#[ensures(ret.as_ref().is_ok_and(|version| !version.is_empty()) || ret.is_err())]
fn release_service_worker_cache_version(public_dir: &Path, paths: &[String]) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(RELEASE_SERVICE_WORKER_TEMPLATE.as_bytes());
    hasher.update([0]);
    for path in paths {
        let bytes = fs::read(public_dir.join(path))
            .with_context(|| format!("reading release web asset `{path}` for cache version"))?;
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(&bytes);
        hasher.update([0xff]);
    }
    let hash = format!("{:x}", hasher.finalize());
    Ok(hash[..16].to_owned())
}

#[requires(!cache_version.is_empty())]
#[requires(precache_paths.iter().all(|path| !path.is_empty() && !path.starts_with('/')))]
#[ensures(ret.as_ref().is_ok_and(|script| script.contains(cache_version)) || ret.is_err())]
fn render_release_service_worker(cache_version: &str, precache_paths: &[String]) -> Result<String> {
    let cache_version_json = serde_json::to_string(cache_version)?;
    let precache_paths_json = serde_json::to_string(precache_paths)?;
    Ok(RELEASE_SERVICE_WORKER_TEMPLATE
        .replace("__CACHE_VERSION_JSON__", &cache_version_json)
        .replace("__PRECACHE_PATHS_JSON__", &precache_paths_json))
}

const RELEASE_SERVICE_WORKER_TEMPLATE: &str = r#"const CACHE_VERSION = __CACHE_VERSION_JSON__;
const STATIC_CACHE_NAME = `jbotci-static-${CACHE_VERSION}`;
const RUNTIME_CACHE_NAME = `jbotci-runtime-${CACHE_VERSION}`;
const CURRENT_CACHE_NAMES = new Set([STATIC_CACHE_NAME, RUNTIME_CACHE_NAME]);
const PRECACHE_PATHS = __PRECACHE_PATHS_JSON__;

const SCOPE_URL = new URL(self.registration.scope);
if (!SCOPE_URL.pathname.endsWith("/")) {
  SCOPE_URL.pathname = `${SCOPE_URL.pathname}/`;
}
const APP_SHELL_URL = new URL("index.html", SCOPE_URL).href;
const PRECACHE_URLS = new Set(
  PRECACHE_PATHS.map((path) => new URL(path, SCOPE_URL).href),
);

self.addEventListener("install", (event) => {
  event.waitUntil((async () => {
    const cache = await caches.open(STATIC_CACHE_NAME);
    await cache.addAll(
      PRECACHE_PATHS.map((path) => new Request(new URL(path, SCOPE_URL), {
        cache: "default",
      })),
    );
    await self.skipWaiting();
  })());
});

self.addEventListener("activate", (event) => {
  event.waitUntil((async () => {
    const cacheNames = await caches.keys();
    await Promise.all(cacheNames.map((name) => {
      if (name.startsWith("jbotci-") && !CURRENT_CACHE_NAMES.has(name)) {
        return caches.delete(name);
      }
      return Promise.resolve(false);
    }));
    await self.clients.claim();
  })());
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET") {
    return;
  }

  const url = new URL(request.url);
  if (url.origin !== self.location.origin) {
    return;
  }

  const relativePath = relativeScopedPath(url);
  if (relativePath === null) {
    return;
  }

  if (isApiRequest(relativePath)) {
    event.respondWith(networkOnlyJson(request));
    return;
  }

  if (isEmbeddingAssetRequest(relativePath)) {
    return;
  }

  if (request.mode === "navigate") {
    event.respondWith(networkFirst(request, RUNTIME_CACHE_NAME, APP_SHELL_URL));
    return;
  }

  if (PRECACHE_URLS.has(url.href)) {
    event.respondWith(networkFirst(request, STATIC_CACHE_NAME, null));
    return;
  }

  if (isStaticOrCoreRequest(relativePath)) {
    event.respondWith(networkFirst(request, RUNTIME_CACHE_NAME, null));
  }
});

function relativeScopedPath(url) {
  if (!url.pathname.startsWith(SCOPE_URL.pathname)) {
    return null;
  }
  return url.pathname.slice(SCOPE_URL.pathname.length);
}

function isApiRequest(relativePath) {
  return relativePath === "api" || relativePath.startsWith("api/");
}

function isEmbeddingAssetRequest(relativePath) {
  return relativePath.startsWith("assets/embeddings/");
}

function isStaticOrCoreRequest(relativePath) {
  return relativePath === ""
    || relativePath === "index.html"
    || relativePath === "manifest.webmanifest"
    || relativePath === "service-worker.js"
    || relativePath.startsWith("assets/");
}

async function networkFirst(request, cacheName, fallbackUrl) {
  const cache = await caches.open(cacheName);
  try {
    const response = await fetch(request);
    if (response.ok && response.type !== "opaque") {
      await cache.put(request, response.clone());
    }
    return response;
  } catch (error) {
    const cached = await caches.match(request);
    if (cached) {
      return cached;
    }
    if (fallbackUrl !== null) {
      const fallback = await caches.match(fallbackUrl);
      if (fallback) {
        return fallback;
      }
    }
    return offlineTextResponse();
  }
}

async function networkOnlyJson(request) {
  try {
    return await fetch(request);
  } catch (error) {
    return new Response(JSON.stringify({
      error: "offline",
      message: "jbotci is offline and this API request is not cached.",
    }), {
      status: 503,
      headers: {
        "Content-Type": "application/json; charset=utf-8",
      },
    });
  }
}

function offlineTextResponse() {
  return new Response("jbotci is offline and this resource is not cached.", {
    status: 503,
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
    },
  });
}
"#;

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
    let output = web_dist
        .join("assets")
        .join("embeddings")
        .join("web")
        .join("v1");
    build_web_embedding_assets_to(&output, corpus, dtypes, backend)
}

#[requires(corpus.is_file())]
#[requires(!dtypes.is_empty())]
#[requires(!backend.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn build_web_embedding_assets_to(
    output: &Path,
    corpus: &Path,
    dtypes: &[String],
    backend: &str,
) -> Result<()> {
    let _ = (dtypes, backend);
    build_all_f2llm_webgpu_assets(
        "python3",
        Path::new(F2LLM_MODEL_ARTIFACT_ROOT_DIR),
        output,
        corpus,
        None,
        8,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|path| path.is_file()) || ret.is_err())]
fn ensure_web_embedding_corpus(corpus: Option<&Path>) -> Result<PathBuf> {
    match corpus {
        Some(path) => {
            let path = absolute_path(path)?;
            if !path.is_file() {
                bail!("web embedding corpus `{}` does not exist", path.display());
            }
            Ok(path)
        }
        None => {
            let output = absolute_path(Path::new(".jbotci-build/web-embedding-corpus.json"))?;
            write_web_embedding_corpus(&output)?;
            Ok(output)
        }
    }
}

#[requires(root.components().next().is_some())]
#[requires(!spec.webgpu_artifact_dir_name.is_empty())]
#[ensures(ret.ends_with("v1"))]
fn f2llm_model_artifact_out_dir(root: &Path, spec: &F2LlmAssetSpec) -> PathBuf {
    root.join(spec.webgpu_artifact_dir_name).join("v1")
}

#[requires(root.components().next().is_some())]
#[ensures(ret.ends_with("v1"))]
fn f2llm_onnx_fallback_out_dir(root: &Path) -> PathBuf {
    root.join("f2llm-v2-80m-onnx-q4").join("v1")
}

#[requires(!python.trim().is_empty())]
#[requires(batch_size > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn build_all_f2llm_webgpu_assets(
    python: &str,
    model_out_root: &Path,
    vector_out_dir: &Path,
    corpus: &Path,
    tokenizer_dir: Option<&Path>,
    batch_size: usize,
) -> Result<()> {
    let vector_parts_root = vector_out_dir.with_file_name(format!(
        "{}.parts",
        vector_out_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("r2-web-embeddings-f2llm")
    ));
    fs::remove_dir_all(&vector_parts_root).ok();
    fs::create_dir_all(&vector_parts_root)
        .with_context(|| format!("creating `{}`", vector_parts_root.display()))?;
    let mut part_dirs = Vec::new();
    for spec in F2LLM_MODEL_SPECS {
        let q4_onnx = absolute_path(Path::new(spec.q4_onnx))?;
        build_f2llm_webgpu_model(BuildF2LlmWebgpuModelArgs {
            q4_onnx: q4_onnx.clone(),
            model_key: spec.model_key.to_owned(),
            model_id: spec.model_id.to_owned(),
            model_root: None,
            out_dir: f2llm_model_artifact_out_dir(model_out_root, spec),
            stage: None,
            shard_size: 4 * 1024 * 1024,
            python: python.to_owned(),
        })?;
        let part_dir = vector_parts_root.join(spec.id);
        run_f2llm_vector_builder(
            python,
            &q4_onnx,
            tokenizer_dir,
            &part_dir,
            None,
            corpus,
            batch_size,
            spec.model_key,
            spec.model_id,
            spec.dimensions,
            spec.include_wasm_runtime,
        )?;
        run_f2llm_vector_validator(
            python,
            &q4_onnx,
            tokenizer_dir,
            &part_dir,
            corpus,
            spec.model_key,
            spec.dimensions,
            spec.include_wasm_runtime,
        )?;
        part_dirs.push(part_dir);
    }
    merge_f2llm_vector_pack_parts(&part_dirs, vector_out_dir)
}

#[requires(!python.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_all_f2llm_vector_packs(
    python: &str,
    vector_out_dir: &Path,
    corpus: &Path,
    tokenizer_dir: Option<&Path>,
) -> Result<()> {
    for spec in F2LLM_MODEL_SPECS {
        let q4_onnx = absolute_path(Path::new(spec.q4_onnx))?;
        run_f2llm_vector_validator(
            python,
            &q4_onnx,
            tokenizer_dir,
            vector_out_dir,
            corpus,
            spec.model_key,
            spec.dimensions,
            spec.include_wasm_runtime,
        )?;
    }
    Ok(())
}

#[requires(!part_dirs.is_empty())]
#[requires(part_dirs.iter().all(|path| path.is_dir()))]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn merge_f2llm_vector_pack_parts(part_dirs: &[PathBuf], out_dir: &Path) -> Result<()> {
    let stage = out_dir.with_file_name(format!(
        "{}.staging",
        out_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("r2-web-embeddings-f2llm")
    ));
    fs::remove_dir_all(&stage).ok();
    fs::create_dir_all(stage.join("models"))
        .with_context(|| format!("creating `{}`", stage.display()))?;
    let mut models = Vec::new();
    for part_dir in part_dirs {
        let catalog = read_json_file(&part_dir.join("catalog.json"))?;
        let part_models = catalog
            .get("models")
            .and_then(serde_json::Value::as_array)
            .context("F2LLM vector part catalog `models` must be an array")?;
        for model in part_models {
            let model_key = json_string_field(model, "model_key")?;
            copy_dir_recursive(
                &part_dir.join("models").join(model_key),
                &stage.join("models").join(model_key),
                "F2LLM vector pack model",
            )?;
            models.push(model.clone());
        }
    }
    write_json_file(
        &stage.join("catalog.json"),
        &serde_json::json!({
            "schema_version": 1,
            "models": models,
        }),
    )?;
    promote_directory(&stage, out_dir)
}

#[requires(source.is_dir())]
#[requires(!description.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_dir_recursive(source: &Path, target: &Path, description: &str) -> Result<()> {
    fs::remove_dir_all(target).ok();
    for entry in WalkDir::new(source) {
        let entry = entry.with_context(|| format!("walking `{}`", source.display()))?;
        let relative = entry.path().strip_prefix(source).with_context(|| {
            format!(
                "making `{}` relative to `{}`",
                entry.path().display(),
                source.display()
            )
        })?;
        let target_path = target.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target_path).with_context(|| {
                format!(
                    "creating {description} directory `{}`",
                    target_path.display()
                )
            })?;
        } else if entry.file_type().is_file() {
            let parent = target_path
                .parent()
                .with_context(|| format!("target `{}` has no parent", target_path.display()))?;
            fs::create_dir_all(parent).with_context(|| {
                format!("creating {description} directory `{}`", parent.display())
            })?;
            fs::copy(entry.path(), &target_path).with_context(|| {
                format!(
                    "copying {description} `{}` to `{}`",
                    entry.path().display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

#[requires(stage.is_dir())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn promote_directory(stage: &Path, output: &Path) -> Result<()> {
    let backup = output.with_file_name(format!(
        "{}.previous",
        output
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("output")
    ));
    fs::remove_dir_all(&backup).ok();
    if output.exists() {
        fs::rename(output, &backup).with_context(|| {
            format!(
                "moving previous output `{}` to `{}`",
                output.display(),
                backup.display()
            )
        })?;
    }
    fs::rename(stage, output)
        .with_context(|| format!("promoting `{}` to `{}`", stage.display(), output.display()))?;
    fs::remove_dir_all(&backup).ok();
    Ok(())
}

#[requires(root.components().next().is_some())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn build_f2llm_onnx_fallback_asset(root: &Path) -> Result<()> {
    let spec = F2LLM_MODEL_SPECS
        .iter()
        .find(|spec| spec.include_wasm_runtime)
        .context("F2LLM model table must contain a WASM fallback model")?;
    let source = absolute_path(Path::new(spec.q4_onnx))?;
    let output = f2llm_onnx_fallback_out_dir(root);
    let stage = output.with_file_name("v1.staging");
    fs::remove_dir_all(&stage).ok();
    fs::create_dir_all(&stage).with_context(|| format!("creating `{}`", stage.display()))?;
    let model_target = stage.join("model_q4.onnx");
    fs::copy(&source, &model_target).with_context(|| {
        format!(
            "copying F2LLM ONNX fallback `{}` to `{}`",
            source.display(),
            model_target.display()
        )
    })?;
    let bytes =
        fs::read(&model_target).with_context(|| format!("reading `{}`", model_target.display()))?;
    write_json_file(
        &stage.join("manifest.json"),
        &serde_json::json!({
            "schema_version": 1,
            "runtime": "jbotci-onnxruntime-web-f2llm",
            "artifact_version": "0.2.0",
            "model_key": spec.model_key,
            "source_model": spec.model_id,
            "model_url": "model_q4.onnx",
            "model_byte_length": bytes.len(),
            "model_sha256": sha256_hex(&bytes),
            "max_sequence_length": F2LLM_MAX_SEQUENCE_LENGTH,
            "dimensions": spec.dimensions,
        }),
    )?;
    promote_directory(&stage, &output)
}

#[requires(!python.trim().is_empty())]
#[requires(true)]
#[requires(batch_size > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
#[allow(clippy::too_many_arguments)]
fn run_f2llm_vector_builder(
    python: &str,
    q4_onnx: &Path,
    tokenizer_dir: Option<&Path>,
    out_dir: &Path,
    stage: Option<&Path>,
    corpus: &Path,
    batch_size: usize,
    model_key: &str,
    model_id: &str,
    dimensions: usize,
    include_wasm_runtime: bool,
) -> Result<()> {
    if !q4_onnx.is_file() {
        bail!("F2LLM q4 ONNX model `{}` does not exist", q4_onnx.display());
    }
    if !corpus.is_file() {
        bail!("web embedding corpus `{}` does not exist", corpus.display());
    }
    let mut command = ProcessCommand::new(python);
    command
        .arg("tools/embedding-pack/f2llm/build-vector-pack.py")
        .arg("--input")
        .arg(corpus)
        .arg("--out")
        .arg(out_dir)
        .arg("--q4-onnx")
        .arg(q4_onnx)
        .arg("--model-key")
        .arg(model_key)
        .arg("--model-id")
        .arg(model_id)
        .arg("--dimensions")
        .arg(dimensions.to_string())
        .arg("--vector-space-key")
        .arg(F2LLM_VECTOR_SPACE_KEY)
        .arg("--max-sequence-length")
        .arg(F2LLM_MAX_SEQUENCE_LENGTH.to_string())
        .arg("--batch-size")
        .arg(batch_size.to_string());
    if include_wasm_runtime {
        command.arg("--include-wasm-runtime");
    }
    if let Some(tokenizer_dir) = tokenizer_dir {
        command
            .arg("--tokenizer-dir")
            .arg(absolute_path(tokenizer_dir)?);
    }
    if let Some(stage) = stage {
        command.arg("--stage").arg(absolute_path(stage)?);
    }
    let status = command.status().with_context(|| {
        format!(
            "failed to build F2LLM vector pack at `{}`",
            out_dir.display()
        )
    })?;
    check_status(
        status,
        "python3 tools/embedding-pack/f2llm/build-vector-pack.py",
    )
}

#[requires(!python.trim().is_empty())]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
#[allow(clippy::too_many_arguments)]
fn run_f2llm_vector_validator(
    python: &str,
    q4_onnx: &Path,
    tokenizer_dir: Option<&Path>,
    out_dir: &Path,
    corpus: &Path,
    model_key: &str,
    dimensions: usize,
    include_wasm_runtime: bool,
) -> Result<()> {
    if !q4_onnx.is_file() {
        bail!("F2LLM q4 ONNX model `{}` does not exist", q4_onnx.display());
    }
    if !out_dir.is_dir() {
        bail!("F2LLM vector pack `{}` does not exist", out_dir.display());
    }
    if !corpus.is_file() {
        bail!("web embedding corpus `{}` does not exist", corpus.display());
    }
    let mut command = ProcessCommand::new(python);
    command
        .arg("tools/embedding-pack/f2llm/validate-vector-pack.py")
        .arg("--pack")
        .arg(out_dir)
        .arg("--corpus")
        .arg(corpus)
        .arg("--q4-onnx")
        .arg(q4_onnx)
        .arg("--model-key")
        .arg(model_key)
        .arg("--dimensions")
        .arg(dimensions.to_string())
        .arg("--vector-space-key")
        .arg(F2LLM_VECTOR_SPACE_KEY)
        .arg("--max-sequence-length")
        .arg(F2LLM_MAX_SEQUENCE_LENGTH.to_string());
    if include_wasm_runtime {
        command.arg("--include-wasm-runtime");
    }
    if let Some(tokenizer_dir) = tokenizer_dir {
        command
            .arg("--tokenizer-dir")
            .arg(absolute_path(tokenizer_dir)?);
    }
    let status = command.status().with_context(|| {
        format!(
            "failed to validate F2LLM vector pack at `{}`",
            out_dir.display()
        )
    })?;
    check_status(
        status,
        "python3 tools/embedding-pack/f2llm/validate-vector-pack.py",
    )
}

#[requires(build_root.is_dir())]
#[requires(!prefix.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|objects| !objects.is_empty()) || ret.is_err())]
fn r2_upload_objects(build_root: &Path, prefix: &str) -> Result<Vec<R2UploadObject>> {
    let prefix = normalize_r2_prefix(prefix)?;
    let mut pack_objects = catalog_referenced_r2_object_keys(build_root)?
        .into_iter()
        .map(|relative_key| r2_upload_object_for_key(build_root, &prefix, &relative_key))
        .collect::<Result<Vec<_>>>()?;
    let catalog = r2_upload_object_for_key(build_root, &prefix, "catalog.json")?;
    pack_objects.push(catalog);
    Ok(pack_objects)
}

#[requires(build_root.is_dir())]
#[requires(!prefix.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|objects| !objects.iter().any(|object| object.object_key.ends_with("/catalog.json"))) || ret.is_err())]
fn r2_upload_objects_without_catalog(
    build_root: &Path,
    prefix: &str,
) -> Result<Vec<R2UploadObject>> {
    let prefix = normalize_r2_prefix(prefix)?;
    catalog_referenced_r2_object_keys(build_root)?
        .into_iter()
        .map(|relative_key| r2_upload_object_for_key(build_root, &prefix, &relative_key))
        .collect()
}

#[requires(build_root.is_dir())]
#[requires(!prefix.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|objects| !objects.iter().any(|object| object.object_key.ends_with("/catalog.json"))) || ret.is_err())]
fn r2_upload_native_objects_without_catalog(
    build_root: &Path,
    prefix: &str,
) -> Result<Vec<R2UploadObject>> {
    let prefix = normalize_r2_prefix(prefix)?;
    native_catalog_referenced_r2_object_keys(build_root)?
        .into_iter()
        .map(|relative_key| r2_upload_object_for_key(build_root, &prefix, &relative_key))
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|keys| !keys.is_empty()) || ret.is_err())]
fn selected_gguf_model_keys(models: &[String]) -> Result<Vec<String>> {
    let allowed = GGUF_MODEL_KEYS.iter().copied().collect::<BTreeSet<_>>();
    let selected = if models.is_empty() {
        GGUF_MODEL_KEYS
            .iter()
            .map(|model| (*model).to_owned())
            .collect::<Vec<_>>()
    } else {
        let mut seen = BTreeSet::new();
        let mut selected = Vec::new();
        for model in models {
            if !allowed.contains(model.as_str()) {
                bail!(
                    "unsupported native GGUF embedding model `{}`; supported models are: {}",
                    model,
                    GGUF_MODEL_KEYS.join(", ")
                );
            }
            if seen.insert(model.as_str()) {
                selected.push(model.clone());
            }
        }
        selected
    };
    Ok(selected)
}

#[requires(catalog.get("models").is_some())]
#[ensures(ret.as_ref().is_ok_and(|keys| !keys.is_empty()) || ret.is_err())]
fn embedding_catalog_model_keys(catalog: &serde_json::Value) -> Result<BTreeSet<String>> {
    let models = catalog
        .get("models")
        .and_then(serde_json::Value::as_array)
        .context("embedding catalog `models` must be an array")?;
    let mut keys = BTreeSet::new();
    for model in models {
        keys.insert(json_string_field(model, "model_key")?.to_owned());
    }
    if keys.is_empty() {
        bail!("embedding catalog must contain at least one model");
    }
    Ok(keys)
}

#[requires(build_root.is_dir())]
#[requires(!prefix.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|objects| objects.iter().all(|object| object.object_key.starts_with(prefix.trim().trim_matches('/')))) || ret.is_err())]
fn r2_upload_tree_objects(build_root: &Path, prefix: &str) -> Result<Vec<R2UploadObject>> {
    let prefix = normalize_r2_prefix(prefix)?;
    let mut objects = Vec::new();
    for entry in WalkDir::new(build_root) {
        let entry = entry.with_context(|| format!("walking `{}`", build_root.display()))?;
        if !entry.file_type().is_file() || path_has_extension(entry.path(), "br") {
            continue;
        }
        let relative = entry.path().strip_prefix(build_root).with_context(|| {
            format!(
                "making upload path `{}` relative to `{}`",
                entry.path().display(),
                build_root.display()
            )
        })?;
        let relative_key = relative_path_to_object_key(relative)?;
        objects.push(r2_upload_object_for_key(
            build_root,
            &prefix,
            &relative_key,
        )?);
    }
    objects.sort_by(|left, right| {
        let left_manifest = left.object_key.ends_with("/manifest.json");
        let right_manifest = right.object_key.ends_with("/manifest.json");
        left_manifest
            .cmp(&right_manifest)
            .then_with(|| left.object_key.cmp(&right.object_key))
    });
    Ok(objects)
}

#[requires(build_root.is_dir())]
#[ensures(ret.as_ref().is_ok_and(|keys| !keys.is_empty() && !keys.contains(&"catalog.json".to_owned())) || ret.is_err())]
fn catalog_referenced_r2_object_keys(build_root: &Path) -> Result<Vec<String>> {
    let catalog = read_json_file(&build_root.join("catalog.json"))?;
    let mut keys = BTreeSet::new();
    let models = catalog
        .get("models")
        .and_then(serde_json::Value::as_array)
        .context("web embedding catalog `models` must be an array")?;
    for model in models {
        let vector_spaces = model
            .get("vector_spaces")
            .and_then(serde_json::Value::as_array)
            .context("web embedding catalog `vector_spaces` must be an array")?;
        for vector_space in vector_spaces {
            let manifest_key = json_string_field(vector_space, "manifest_url")?;
            let manifest_dir_key = manifest_key
                .strip_suffix("/manifest.json")
                .context("web embedding manifest_url must end with `/manifest.json`")?;
            keys.insert(manifest_key.to_owned());
            let manifest = read_json_file(&object_key_local_path(build_root, manifest_key)?)?;
            let corpora = manifest
                .get("corpora")
                .and_then(serde_json::Value::as_array)
                .context("web embedding manifest `corpora` must be an array")?;
            for corpus in corpora {
                let items_key = join_relative_object_key(
                    manifest_dir_key,
                    json_string_field(corpus, "items_url")?,
                )?;
                let vector_key = join_relative_object_key(
                    manifest_dir_key,
                    json_string_field(corpus, "vector_url")?,
                )?;
                keys.insert(items_key);
                keys.insert(vector_key);
            }
        }
    }
    Ok(keys.into_iter().collect())
}

#[requires(build_root.is_dir())]
#[ensures(ret.as_ref().is_ok_and(|keys| !keys.is_empty() && !keys.contains(&"catalog.json".to_owned())) || ret.is_err())]
fn native_catalog_referenced_r2_object_keys(build_root: &Path) -> Result<Vec<String>> {
    let catalog = read_json_file(&build_root.join("catalog.json"))?;
    let mut keys = BTreeSet::new();
    let models = catalog
        .get("models")
        .and_then(serde_json::Value::as_array)
        .context("native embedding catalog `models` must be an array")?;
    for model in models {
        let manifest_key = json_string_field(model, "manifest_url")?;
        let manifest_dir_key = manifest_key
            .strip_suffix("/manifest.json")
            .context("native embedding manifest_url must end with `/manifest.json`")?;
        keys.insert(manifest_key.to_owned());
        let manifest = read_json_file(&object_key_local_path(build_root, manifest_key)?)?;
        let corpora = manifest
            .get("corpora")
            .and_then(serde_json::Value::as_array)
            .context("native embedding manifest `corpora` must be an array")?;
        for corpus in corpora {
            let items_key = join_relative_object_key(
                manifest_dir_key,
                json_string_field(corpus, "items_url")?,
            )?;
            keys.insert(items_key);
            let shards = corpus
                .get("shards")
                .and_then(serde_json::Value::as_array)
                .context("native embedding corpus `shards` must be an array")?;
            for shard in shards {
                let shard_key =
                    join_relative_object_key(manifest_dir_key, json_string_field(shard, "url")?)?;
                keys.insert(shard_key);
            }
        }
    }
    Ok(keys.into_iter().collect())
}

#[requires(build_root.is_dir())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_native_gguf_r2_tree(build_root: &Path) -> Result<()> {
    let keys = native_catalog_referenced_r2_object_keys(build_root)?;
    for key in keys {
        let path = object_key_local_path(build_root, &key)?;
        if !path.is_file() {
            bail!(
                "native GGUF embedding object `{}` is missing under `{}`",
                key,
                build_root.display()
            );
        }
    }
    Ok(())
}

#[requires(path.components().next().is_some())]
#[ensures(ret.as_ref().is_ok_and(|value| value.is_object() || value.is_array() || value.is_null() || value.is_boolean() || value.is_number() || value.is_string()) || ret.is_err())]
fn read_json_file(path: &Path) -> Result<serde_json::Value> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading JSON file `{}`", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing JSON file `{}`", path.display()))
}

#[requires(path.components().next().is_some())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn write_json_file(path: &Path, value: &serde_json::Value) -> Result<()> {
    let mut text = serde_json::to_string_pretty(value)
        .with_context(|| format!("rendering JSON for `{}`", path.display()))?;
    text.push('\n');
    fs::write(path, text).with_context(|| format!("writing JSON file `{}`", path.display()))
}

#[requires(!model_key.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|value| value.get("models").and_then(serde_json::Value::as_array).is_some()) || ret.is_err())]
fn merge_embedding_catalog(
    remote_catalog: serde_json::Value,
    replacement_catalog: serde_json::Value,
    model_key: &str,
) -> Result<serde_json::Value> {
    merge_embedding_catalog_models(
        remote_catalog,
        replacement_catalog,
        &BTreeSet::from([model_key.to_owned()]),
    )
}

#[requires(!model_keys.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|value| value.get("models").and_then(serde_json::Value::as_array).is_some()) || ret.is_err())]
fn merge_embedding_catalog_models(
    remote_catalog: serde_json::Value,
    replacement_catalog: serde_json::Value,
    model_keys: &BTreeSet<String>,
) -> Result<serde_json::Value> {
    if remote_catalog
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        bail!("remote embedding catalog schema_version must be 1");
    }
    if replacement_catalog
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        bail!("replacement embedding catalog schema_version must be 1");
    }
    let remote_models = remote_catalog
        .get("models")
        .and_then(serde_json::Value::as_array)
        .context("remote embedding catalog `models` must be an array")?;
    let replacement_models = replacement_catalog
        .get("models")
        .and_then(serde_json::Value::as_array)
        .context("replacement embedding catalog `models` must be an array")?;
    let mut replacements = BTreeMap::new();
    for model in replacement_models {
        let Some(model_key) = model.get("model_key").and_then(serde_json::Value::as_str) else {
            continue;
        };
        if !model_keys.contains(model_key) {
            continue;
        }
        if replacements
            .insert(model_key.to_owned(), model.clone())
            .is_some()
        {
            bail!("replacement embedding catalog contains multiple `{model_key}` entries");
        }
    }
    for model_key in model_keys {
        if !replacements.contains_key(model_key) {
            bail!("replacement embedding catalog does not contain `{model_key}`");
        }
    }
    let mut merged = Vec::new();
    let mut inserted = BTreeSet::new();
    for model in remote_models {
        if let Some(model_key) = model.get("model_key").and_then(serde_json::Value::as_str)
            && model_keys.contains(model_key)
        {
            if inserted.insert(model_key.to_owned()) {
                merged.push(replacements[model_key].clone());
            }
            continue;
        }
        merged.push(model.clone());
    }
    for model_key in model_keys {
        if !inserted.contains(model_key) {
            merged.push(replacements[model_key].clone());
        }
    }
    Ok(serde_json::json!({
        "schema_version": 1,
        "models": merged,
    }))
}

#[requires(!url.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|value| value.get("models").and_then(serde_json::Value::as_array).is_some()) || ret.is_err())]
fn fetch_optional_json_catalog(url: &str) -> Result<serde_json::Value> {
    match ureq::get(url).call() {
        Ok(mut response) => {
            let text = response
                .body_mut()
                .with_config()
                .limit(64 * 1024 * 1024)
                .read_to_string()
                .with_context(|| format!("reading response body from `{url}`"))?;
            let value: serde_json::Value = serde_json::from_str(&text)
                .with_context(|| format!("parsing JSON from `{url}`"))?;
            if value
                .get("models")
                .and_then(serde_json::Value::as_array)
                .is_none()
            {
                bail!("remote catalog from `{url}` must contain a `models` array");
            }
            Ok(value)
        }
        Err(ureq::Error::StatusCode(404)) => Ok(serde_json::json!({
            "schema_version": 1,
            "models": [],
        })),
        Err(error) => Err(error).with_context(|| format!("GET `{url}`")),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| !value.is_empty()) || ret.is_err())]
fn json_string_field<'a>(value: &'a serde_json::Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .with_context(|| format!("web embedding JSON field `{field}` must be a non-empty string"))
}

#[requires(!base.trim().is_empty())]
#[requires(!relative.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|key| !key.starts_with('/')) || ret.is_err())]
fn join_relative_object_key(base: &str, relative: &str) -> Result<String> {
    let base = normalize_relative_object_key(base)?;
    let relative = normalize_relative_object_key(relative)?;
    Ok(format!("{base}/{relative}"))
}

#[requires(!key.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|key| !key.starts_with('/') && !key.ends_with('/')) || ret.is_err())]
fn normalize_relative_object_key(key: &str) -> Result<String> {
    let normalized = key.trim().trim_matches('/');
    if normalized.is_empty() {
        bail!("R2 object key must not be empty")
    }
    if normalized
        .split('/')
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        bail!("R2 object key `{key}` must not contain empty, `.` or `..` path components")
    }
    Ok(normalized.to_owned())
}

#[requires(build_root.is_dir())]
#[requires(!relative_key.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|path| path.starts_with(build_root)) || ret.is_err())]
fn object_key_local_path(build_root: &Path, relative_key: &str) -> Result<PathBuf> {
    let mut path = build_root.to_path_buf();
    for component in normalize_relative_object_key(relative_key)?.split('/') {
        path.push(component);
    }
    Ok(path)
}

#[requires(build_root.is_dir())]
#[requires(!prefix.trim().is_empty())]
#[requires(!relative_key.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|object| object.object_key.starts_with(prefix)) || ret.is_err())]
fn r2_upload_object_for_key(
    build_root: &Path,
    prefix: &str,
    relative_key: &str,
) -> Result<R2UploadObject> {
    let relative_key = normalize_relative_object_key(relative_key)?;
    let local_uncompressed = object_key_local_path(build_root, &relative_key)?;
    if !local_uncompressed.is_file() {
        bail!(
            "web embedding upload object `{}` does not exist under `{}`",
            relative_key,
            build_root.display()
        );
    }
    let brotli_path = brotli_sidecar_path(&local_uncompressed)?;
    let (local_path, content_encoding) = if brotli_path.is_file() {
        (brotli_path, Some("br"))
    } else {
        (local_uncompressed, None)
    };
    Ok(R2UploadObject {
        local_path,
        content_type: r2_content_type(&relative_key),
        content_encoding,
        cache_control: r2_cache_control(&relative_key),
        object_key: format!("{prefix}/{relative_key}"),
    })
}

#[requires(!prefix.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|prefix| !prefix.starts_with('/') && !prefix.ends_with('/')) || ret.is_err())]
fn normalize_r2_prefix(prefix: &str) -> Result<String> {
    let normalized = prefix.trim().trim_matches('/');
    if normalized.is_empty() {
        bail!("R2 prefix must not be empty")
    }
    if normalized
        .split('/')
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        bail!("R2 prefix `{prefix}` must not contain empty, `.` or `..` path components")
    }
    Ok(normalized.to_owned())
}

#[requires(path.components().next().is_some())]
#[ensures(ret.as_ref().is_ok_and(|key| !key.starts_with('/')) || ret.is_err())]
fn relative_path_to_object_key(path: &Path) -> Result<String> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(name) => {
                let Some(name) = name.to_str() else {
                    bail!("R2 object path `{}` is not valid UTF-8", path.display())
                };
                components.push(name.to_owned());
            }
            _ => bail!(
                "R2 object path `{}` must be relative and contain only normal components",
                path.display()
            ),
        }
    }
    if components.is_empty() {
        bail!("R2 object path must not be empty")
    }
    Ok(components.join("/"))
}

#[requires(path.file_name().is_some())]
#[ensures(ret.as_ref().is_ok_and(|path| path.file_name().is_some_and(|name| name.to_string_lossy().ends_with(".br"))) || ret.is_err())]
fn brotli_sidecar_path(path: &Path) -> Result<PathBuf> {
    let Some(file_name) = path.file_name() else {
        bail!("path `{}` has no file name", path.display())
    };
    let mut sidecar_file_name = file_name.to_os_string();
    sidecar_file_name.push(".br");
    Ok(path.with_file_name(sidecar_file_name))
}

#[requires(!path.trim().is_empty())]
#[ensures(!ret.is_empty())]
fn r2_content_type(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("json") => "application/json; charset=utf-8",
        Some("f32") => "application/octet-stream",
        Some("f16") => "application/octet-stream",
        _ => "application/octet-stream",
    }
}

#[requires(!path.trim().is_empty())]
#[ensures(!ret.is_empty())]
fn r2_cache_control(path: &str) -> &'static str {
    if path == "catalog.json" || path.ends_with("/manifest.json") || path == "manifest.json" {
        R2_CATALOG_CACHE_CONTROL
    } else {
        R2_IMMUTABLE_CACHE_CONTROL
    }
}

#[requires(path.components().next().is_some())]
#[requires(!extension.is_empty())]
#[ensures(true)]
fn path_has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|actual| actual.to_str())
        .is_some_and(|actual| actual == extension)
}

#[requires(!bucket.trim().is_empty())]
#[requires(!objects.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn put_r2_objects(bucket: &str, objects: &[R2UploadObject]) -> Result<()> {
    let (boundary_objects, data_objects): (Vec<_>, Vec<_>) = objects
        .iter()
        .partition(|object| r2_upload_boundary_object(&object.object_key));
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(R2_UPLOAD_PARALLELISM)
        .build()
        .context("creating R2 upload thread pool")?;
    pool.install(|| {
        data_objects
            .par_iter()
            .try_for_each(|object| put_r2_object(bucket, object))
    })?;
    for object in boundary_objects {
        put_r2_object(bucket, object)?;
    }
    Ok(())
}

#[requires(!object_key.trim().is_empty())]
#[ensures(true)]
fn r2_upload_boundary_object(object_key: &str) -> bool {
    object_key == "catalog.json"
        || object_key.ends_with("/catalog.json")
        || object_key == "manifest.json"
        || object_key.ends_with("/manifest.json")
}

#[requires(!bucket.trim().is_empty())]
#[requires(!object.object_key.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn put_r2_object(bucket: &str, object: &R2UploadObject) -> Result<()> {
    let target = format!("{}/{}", bucket.trim(), object.object_key);
    let mut command = ProcessCommand::new("npx");
    command
        .arg("--yes")
        .arg("wrangler")
        .arg("r2")
        .arg("object")
        .arg("put")
        .arg(target)
        .arg("--file")
        .arg(&object.local_path)
        .arg("--content-type")
        .arg(object.content_type)
        .arg("--cache-control")
        .arg(object.cache_control)
        .arg("--remote")
        .arg("--force");
    if let Some(content_encoding) = object.content_encoding {
        command.arg("--content-encoding").arg(content_encoding);
    }
    let status = command.status().with_context(|| {
        format!(
            "uploading `{}` to R2 object `{}`",
            object.local_path.display(),
            object.object_key
        )
    })?;
    check_status(status, "npx --yes wrangler r2 object put")
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

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn vendor_wiki(args: VendorWikiArgs) -> Result<()> {
    let mut progress = WikiProgressReporter::new(true);
    let result = vendor_wiki_inner(args, &mut progress);
    match result {
        Ok(report) => {
            progress.finish();
            if report.checked {
                return Ok(());
            }
            println!(
                "vendored {} wiki page(s) into `{}` ({} fetched, {} kept, {} failed, {} removed, {}, {} media records)",
                report.pages,
                report.output.display(),
                report.fetched,
                report.kept,
                report.failed,
                report.removed,
                human_bytes(report.source_bytes as u64),
                report.media_files
            );
            Ok(())
        }
        Err(error) => {
            progress.fail();
            Err(error)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn vendor_wiki_inner(
    args: VendorWikiArgs,
    progress: &mut WikiProgressReporter,
) -> Result<WikiVendorReport> {
    let config = wiki_vendor_config(args)?;
    let output = absolute_path(&config.output)?;
    let mut client = WikiHttpClient::new(&config);

    progress.update(&WikiProgressUpdate {
        phase: WikiProgressPhase::Siteinfo,
        current: 0,
        total: 0,
        detail: format!("GET {}", config.api_url),
    });
    let siteinfo = fetch_wiki_siteinfo(&mut client).context("fetching wiki siteinfo")?;
    let namespaces = wiki_namespaces_from_siteinfo(&siteinfo).context("parsing wiki namespaces")?;

    if config.check {
        progress.update(&WikiProgressUpdate {
            phase: WikiProgressPhase::Check,
            current: 0,
            total: 0,
            detail: format!("validating {}", output.display()),
        });
        let existing = validate_existing_wiki_snapshot(&output)?;
        println!(
            "validated wiki snapshot at `{}` with {} page(s); remote {} is reachable",
            output.display(),
            existing.pages.len(),
            config.api_url
        );
        return Ok(WikiVendorReport {
            checked: true,
            output,
            pages: existing.pages.len(),
            fetched: 0,
            kept: existing.pages.len(),
            failed: 0,
            removed: 0,
            media_files: 0,
            source_bytes: existing.pages.iter().map(|page| page.bytes).sum(),
        });
    }

    let started_at = rfc3339_now()?;
    let existing = load_existing_wiki_snapshot(&output)?;
    let plan = build_wiki_snapshot_plan(
        &mut client,
        &config,
        &namespaces,
        existing.as_ref(),
        &output,
        progress,
    )?;

    progress.update(&WikiProgressUpdate {
        phase: WikiProgressPhase::Writing,
        current: 0,
        total: 0,
        detail: "preparing temporary snapshot directory".to_owned(),
    });
    let stage = wiki_stage_dir(&output)?;
    let backup = wiki_backup_dir(&output)?;
    remove_path_if_exists(&stage)?;
    remove_path_if_exists(&backup)?;
    fs::create_dir_all(&stage).with_context(|| format!("creating `{}`", stage.display()))?;
    fs::create_dir_all(stage.join("pages"))
        .with_context(|| format!("creating `{}`", stage.join("pages").display()))?;
    fs::create_dir_all(stage.join("media"))
        .with_context(|| format!("creating `{}`", stage.join("media").display()))?;

    write_json_file(&stage.join("siteinfo.json"), &siteinfo)?;
    write_json_file(
        &stage.join("namespaces.json"),
        &serde_json::to_value(&namespaces).context("serializing wiki namespaces")?,
    )?;

    let mut page_entries = Vec::new();
    let mut source_bytes = 0usize;
    let page_total = plan.keep.len() + plan.fetch.len();
    for (index, entry) in plan.keep.iter().enumerate() {
        copy_wiki_page_entry(&output, &stage, entry)?;
        source_bytes += entry.bytes;
        page_entries.push(entry.clone());
        progress.update(&WikiProgressUpdate {
            phase: WikiProgressPhase::Pages,
            current: index + 1,
            total: page_total,
            detail: wiki_page_progress_detail(
                plan.keep.len(),
                0,
                0,
                plan.removed.len(),
                client.retry_count,
                &entry.title,
            ),
        });
    }

    let mut fetched = 0usize;
    let mut failed_pages = Vec::new();
    for (offset, metadata) in plan.fetch.iter().enumerate() {
        match fetch_wiki_page(&mut client, metadata).with_context(|| {
            format!(
                "fetching Parsoid HTML and source for page `{}` ({})",
                metadata.title, metadata.pageid
            )
        }) {
            Ok(fetched_page) => {
                let report = write_wiki_page(&stage, &fetched_page)?;
                source_bytes += report.source_bytes;
                page_entries.push(report.entry);
                fetched += 1;
            }
            Err(error) => {
                eprintln!(
                    "[wiki] skipped page {} `{}` because the wiki did not return a source/Parsoid pair: {error:#}",
                    metadata.pageid, metadata.title
                );
                failed_pages.push(wiki_page_fetch_error(metadata, &error));
            }
        }
        progress.update(&WikiProgressUpdate {
            phase: WikiProgressPhase::Pages,
            current: plan.keep.len() + offset + 1,
            total: page_total,
            detail: wiki_page_progress_detail(
                plan.keep.len(),
                fetched,
                failed_pages.len(),
                plan.removed.len(),
                client.retry_count,
                &metadata.title,
            ),
        });
    }
    page_entries.sort_by(compare_wiki_page_index_entries);
    write_json_file(
        &stage.join("pages").join("index.json"),
        &serde_json::to_value(&page_entries).context("serializing wiki page index")?,
    )?;
    write_json_file(
        &stage.join("pages").join("errors.json"),
        &serde_json::to_value(&failed_pages).context("serializing wiki page fetch errors")?,
    )?;

    progress.update(&WikiProgressUpdate {
        phase: WikiProgressPhase::Media,
        current: 0,
        total: 0,
        detail: "fetching upload metadata".to_owned(),
    });
    let media_manifest = fetch_wiki_media_manifest(&mut client, progress)?;
    write_json_file(&stage.join("media").join("manifest.json"), &media_manifest)?;

    let finished_at = rfc3339_now()?;
    let media_files = media_manifest
        .get("files")
        .and_then(serde_json::Value::as_array)
        .map_or(0, Vec::len);
    let snapshot = render_wiki_snapshot_metadata(&WikiSnapshotMetadataRenderArgs {
        config: &config,
        siteinfo: &siteinfo,
        started_at: &started_at,
        finished_at: &finished_at,
        plan: &plan,
        page_count: page_entries.len(),
        failed_pages: &failed_pages,
        source_bytes,
        media_manifest: &media_manifest,
    });
    write_json_file(&stage.join("snapshot.json"), &snapshot)?;
    fs::write(
        stage.join("README.md"),
        render_wiki_snapshot_readme(
            &config,
            page_entries.len(),
            failed_pages.len(),
            source_bytes,
            &media_manifest,
        ),
    )
    .with_context(|| format!("writing `{}`", stage.join("README.md").display()))?;

    progress.update(&WikiProgressUpdate {
        phase: WikiProgressPhase::Writing,
        current: 0,
        total: 0,
        detail: "computing digests".to_owned(),
    });
    let digest_rows = directory_digest_rows(&stage)?;
    fs::write(
        stage.join("DIGESTS.sha256"),
        render_directory_digests(&digest_rows),
    )
    .with_context(|| format!("writing `{}`", stage.join("DIGESTS.sha256").display()))?;
    replace_wiki_snapshot_directory(&output, &stage, &backup)?;

    Ok(WikiVendorReport {
        checked: false,
        output,
        pages: page_entries.len(),
        fetched,
        kept: plan.keep.len(),
        failed: failed_pages.len(),
        removed: plan.removed.len(),
        media_files,
        source_bytes,
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|config| !config.source_url.is_empty()) || ret.is_err())]
fn wiki_vendor_config(args: VendorWikiArgs) -> Result<WikiVendorConfig> {
    if args.batch_size == 0 || args.batch_size > 500 {
        bail!("--batch-size must be in the range 1..=500");
    }
    if args.delay_ms < 250 {
        bail!("--delay-ms must be at least 250");
    }
    if args.timeout_ms == 0 {
        bail!("--timeout-ms must be positive");
    }
    if args.retries == 0 {
        bail!("--retries must be positive");
    }
    let source_url = normalize_wiki_base_url(&args.source_url)?;
    let api_url = args
        .api_url
        .map(|url| normalize_wiki_base_url(&url))
        .transpose()?
        .unwrap_or_else(|| format!("{source_url}/api.php"));
    let rest_url = args
        .rest_url
        .map(|url| normalize_wiki_base_url(&url))
        .transpose()?
        .unwrap_or_else(|| format!("{source_url}/rest.php/v1"));
    Ok(WikiVendorConfig {
        source_url,
        api_url,
        rest_url,
        output: args.output,
        batch_size: args.batch_size,
        delay: Duration::from_millis(args.delay_ms),
        timeout: Duration::from_millis(args.timeout_ms),
        retries: args.retries,
        maxlag: args.maxlag,
        user_agent: args.user_agent,
        check: args.check,
        limit_pages: args.limit_pages,
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|url| !url.ends_with('/')) || ret.is_err())]
fn normalize_wiki_base_url(url: &str) -> Result<String> {
    let trimmed = url.trim().trim_end_matches('/').to_owned();
    if trimmed.is_empty() {
        bail!("wiki URL must not be empty");
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        bail!("wiki URL `{trimmed}` must start with http:// or https://");
    }
    Ok(trimmed)
}

impl WikiProgressReporter {
    #[requires(true)]
    #[ensures(enabled -> ret.job.is_some() || clx::progress::is_disabled())]
    fn new(enabled: bool) -> Self {
        if !enabled || clx::progress::is_disabled() {
            return Self {
                job: None,
                determinate: false,
                last_log: Instant::now(),
            };
        }
        let job = ProgressJobBuilder::new()
            .body("{{ spinner() }} {{ message }} {{ detail | flex }}")
            .prop("message", "Wiki snapshot")
            .prop("detail", "")
            .start();
        Self {
            job: Some(job),
            determinate: false,
            last_log: Instant::now(),
        }
    }

    #[requires(update.total == 0 || update.current <= update.total)]
    #[ensures(true)]
    fn update(&mut self, update: &WikiProgressUpdate) {
        if let Some(job) = &self.job {
            if update.total > 0 {
                if !self.determinate {
                    job.set_body("{{ spinner() }} {{ message }} {{ detail | flex }} {{ progress_bar(width=20) }}");
                    self.determinate = true;
                }
                job.progress_total(update.total);
                job.progress_current(update.current);
            } else if self.determinate {
                job.set_body("{{ spinner() }} {{ message }} {{ detail | flex }}");
                self.determinate = false;
            }
            job.message(wiki_progress_phase_label(update.phase));
            job.prop("detail", &update.detail);
            return;
        }

        let now = Instant::now();
        if update.total == 0
            || update.current == update.total
            || now.duration_since(self.last_log) >= Duration::from_secs(30)
        {
            eprintln!(
                "[wiki] {}: {}",
                wiki_progress_phase_label(update.phase),
                update.detail
            );
            self.last_log = now;
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(&mut self) {
        if let Some(job) = &self.job {
            job.set_status(ProgressStatus::Done);
            clx::progress::stop_clear();
        }
        self.job = None;
    }

    #[requires(true)]
    #[ensures(true)]
    fn fail(&mut self) {
        if let Some(job) = &self.job {
            job.set_status(ProgressStatus::Failed);
            clx::progress::stop_clear();
        }
        self.job = None;
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn wiki_progress_phase_label(phase: WikiProgressPhase) -> &'static str {
    match phase {
        WikiProgressPhase::Siteinfo => "Wiki siteinfo",
        WikiProgressPhase::Check => "Wiki check",
        WikiProgressPhase::Planning => "Wiki planning",
        WikiProgressPhase::RecentChanges => "Wiki recentchanges",
        WikiProgressPhase::Metadata => "Wiki metadata",
        WikiProgressPhase::Pages => "Wiki pages",
        WikiProgressPhase::Media => "Wiki media",
        WikiProgressPhase::Writing => "Wiki writing",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn wiki_page_progress_detail(
    kept: usize,
    fetched: usize,
    failed: usize,
    removed: usize,
    retries: usize,
    current_title: &str,
) -> String {
    let title = truncate_progress_text(current_title, 80);
    format!(
        "kept {kept}, fetched {fetched}, failed {failed}, removed {removed}, retries {retries}, {title}"
    )
}

#[requires(max_chars > 0)]
#[ensures(ret.chars().count() <= max_chars)]
fn truncate_progress_text(text: &str, max_chars: usize) -> String {
    let mut result = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index + 1 >= max_chars {
            result.push('~');
            return result;
        }
        result.push(ch);
    }
    result
}

impl WikiHttpClient {
    #[requires(!config.api_url.trim().is_empty())]
    #[requires(!config.rest_url.trim().is_empty())]
    #[ensures(ret.retries == config.retries)]
    fn new(config: &WikiVendorConfig) -> Self {
        let agent_config = ureq::Agent::config_builder()
            .timeout_global(Some(config.timeout))
            .http_status_as_error(false)
            .user_agent(config.user_agent.clone())
            .build();
        Self {
            agent: ureq::Agent::new_with_config(agent_config),
            api_url: config.api_url.clone(),
            rest_url: config.rest_url.clone(),
            delay: config.delay,
            retries: config.retries,
            maxlag: config.maxlag,
            next_request_at: None,
            retry_count: 0,
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|value| value.is_object()) || ret.is_err())]
    fn api_json(&mut self, params: &[(&str, String)]) -> Result<serde_json::Value> {
        let mut full_params = Vec::with_capacity(params.len() + 4);
        full_params.push(("format", "json".to_owned()));
        full_params.push(("formatversion", "2".to_owned()));
        full_params.push(("utf8", "1".to_owned()));
        if self.maxlag > 0 {
            full_params.push(("maxlag", self.maxlag.to_string()));
        }
        full_params.extend(params.iter().map(|(key, value)| (*key, value.clone())));

        for attempt in 1..=self.retries {
            let response = self.get_with_query(&self.api_url.clone(), &full_params);
            let response = match response {
                Ok(response) => response,
                Err(error) if attempt < self.retries => {
                    self.backoff(attempt, None);
                    self.retry_count += 1;
                    eprintln!("[wiki] API request failed: {error}; retrying");
                    continue;
                }
                Err(error) => return Err(error),
            };
            if !is_success_status(response.status) {
                if attempt < self.retries && is_transient_status(response.status) {
                    self.backoff(attempt, response.retry_after.as_deref());
                    self.retry_count += 1;
                    continue;
                }
                bail!(
                    "HTTP {} from MediaWiki API: {}",
                    response.status,
                    truncate_progress_text(&response.body, 500)
                );
            }
            let value = serde_json::from_str::<serde_json::Value>(&response.body)
                .context("parsing MediaWiki API JSON response")?;
            if let Some(error) = value.get("error") {
                let code = error
                    .get("code")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                let info = error
                    .get("info")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("MediaWiki API error");
                if attempt < self.retries && is_transient_mediawiki_error(code) {
                    let retry_after = error
                        .get("lag")
                        .and_then(serde_json::Value::as_f64)
                        .map(|lag| lag.max(1.0));
                    self.backoff_seconds(attempt, retry_after);
                    self.retry_count += 1;
                    continue;
                }
                bail!("MediaWiki API error {code}: {info}");
            }
            return Ok(value);
        }
        bail!("MediaWiki API retry loop exhausted")
    }

    #[requires(!path.trim().is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
    fn rest_text(&mut self, path: &str) -> Result<String> {
        let url = format!("{}/{}", self.rest_url, path.trim_start_matches('/'));
        for attempt in 1..=self.retries {
            let response = self.get_with_query(&url, &[]);
            let response = match response {
                Ok(response) => response,
                Err(error) if attempt < self.retries => {
                    self.backoff(attempt, None);
                    self.retry_count += 1;
                    eprintln!("[wiki] REST request failed: {error}; retrying");
                    continue;
                }
                Err(error) => return Err(error),
            };
            if is_success_status(response.status) {
                return Ok(response.body);
            }
            if attempt < self.retries && is_transient_rest_status(response.status) {
                self.backoff(attempt, response.retry_after.as_deref());
                self.retry_count += 1;
                continue;
            }
            bail!(
                "HTTP {} from MediaWiki REST endpoint `{url}`: {}",
                response.status,
                truncate_progress_text(&response.body, 500)
            );
        }
        bail!("MediaWiki REST retry loop exhausted")
    }

    #[requires(!url.trim().is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|response| response.status > 0) || ret.is_err())]
    fn get_with_query(&mut self, url: &str, params: &[(&str, String)]) -> Result<WikiHttpResponse> {
        self.wait_for_rate_limit();
        let mut request = self.agent.get(url);
        for (key, value) in params {
            request = request.query(*key, value);
        }
        let mut response = request.call().with_context(|| format!("GET `{url}`"))?;
        let status = response.status().as_u16();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let body = response
            .body_mut()
            .with_config()
            .limit(WIKI_HTTP_BODY_LIMIT_BYTES)
            .read_to_string()
            .with_context(|| format!("reading response body from `{url}`"))?;
        Ok(WikiHttpResponse {
            status,
            retry_after,
            body,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn wait_for_rate_limit(&mut self) {
        if let Some(next_request_at) = self.next_request_at {
            let now = Instant::now();
            if next_request_at > now {
                thread::sleep(next_request_at.duration_since(now));
            }
        }
        self.next_request_at = Some(Instant::now() + self.delay);
    }

    #[requires(attempt > 0)]
    #[ensures(true)]
    fn backoff(&self, attempt: usize, retry_after: Option<&str>) {
        let retry_after = retry_after
            .and_then(retry_after_header_duration)
            .unwrap_or(Duration::ZERO);
        thread::sleep(retry_after.max(exponential_backoff_duration(attempt)));
    }

    #[requires(attempt > 0)]
    #[ensures(true)]
    fn backoff_seconds(&self, attempt: usize, retry_after_seconds: Option<f64>) {
        let retry_after = retry_after_seconds
            .filter(|seconds| seconds.is_finite() && *seconds > 0.0)
            .map(Duration::from_secs_f64)
            .unwrap_or(Duration::ZERO);
        thread::sleep(retry_after.max(exponential_backoff_duration(attempt)));
    }
}

#[requires(status > 0)]
#[ensures(true)]
fn is_success_status(status: u16) -> bool {
    (200..300).contains(&status)
}

#[requires(status > 0)]
#[ensures(true)]
fn is_transient_status(status: u16) -> bool {
    matches!(
        status,
        408 | 425 | 429 | 500 | 502 | 503 | 504 | 520 | 521 | 522 | 523 | 524
    )
}

#[requires(status > 0)]
#[ensures(true)]
fn is_transient_rest_status(status: u16) -> bool {
    matches!(
        status,
        408 | 425 | 429 | 502 | 503 | 504 | 520 | 521 | 522 | 523 | 524
    )
}

#[requires(true)]
#[ensures(true)]
fn is_transient_mediawiki_error(code: &str) -> bool {
    matches!(
        code,
        "maxlag" | "readonly" | "internal_api_error_DBQueryError" | "ratelimited"
    )
}

#[requires(attempt > 0)]
#[ensures(ret >= Duration::from_secs(1))]
fn exponential_backoff_duration(attempt: usize) -> Duration {
    let seconds = attempt.saturating_mul(attempt).min(120) as u64;
    Duration::from_secs(seconds.max(1))
}

#[requires(true)]
#[ensures(ret.is_none_or(|duration| duration >= Duration::ZERO))]
fn retry_after_header_duration(value: &str) -> Option<Duration> {
    value.trim().parse::<u64>().ok().map(Duration::from_secs)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| value.get("query").is_some()) || ret.is_err())]
fn fetch_wiki_siteinfo(client: &mut WikiHttpClient) -> Result<serde_json::Value> {
    client.api_json(&[
        ("action", "query".to_owned()),
        ("meta", "siteinfo".to_owned()),
        (
            "siprop",
            "general|namespaces|namespacealiases|statistics|extensions|libraries|extensiontags|functionhooks|variables|magicwords"
                .to_owned(),
        ),
    ])
}

#[requires(siteinfo.get("query").is_some())]
#[ensures(ret.as_ref().is_ok_and(|namespaces| namespaces.windows(2).all(|pair| pair[0].id <= pair[1].id)) || ret.is_err())]
fn wiki_namespaces_from_siteinfo(siteinfo: &serde_json::Value) -> Result<Vec<WikiNamespace>> {
    let namespaces = siteinfo
        .get("query")
        .and_then(|query| query.get("namespaces"))
        .and_then(serde_json::Value::as_object)
        .context("siteinfo query.namespaces must be an object")?;
    let mut result = namespaces
        .values()
        .map(wiki_namespace_from_value)
        .collect::<Result<Vec<_>>>()?;
    result.sort_by_key(|namespace| namespace.id);
    Ok(result)
}

#[requires(value.is_object())]
#[ensures(ret.as_ref().is_ok_and(|namespace| namespace.id >= -2) || ret.is_err())]
fn wiki_namespace_from_value(value: &serde_json::Value) -> Result<WikiNamespace> {
    let id = wiki_json_i64(value, "id")?;
    Ok(WikiNamespace {
        id,
        name: wiki_json_optional_string(value, "name").unwrap_or_default(),
        canonical: wiki_json_optional_string(value, "canonical"),
        content: value
            .get("content")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        case: wiki_json_optional_string(value, "case"),
        subpages: value
            .get("subpages")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
    })
}

#[requires(config.batch_size > 0)]
#[ensures(ret.as_ref().is_ok_and(|pages| pages.windows(2).all(|pair| compare_wiki_remote_metadata(&pair[0], &pair[1]) != std::cmp::Ordering::Greater)) || ret.is_err())]
fn fetch_all_wiki_page_metadata(
    client: &mut WikiHttpClient,
    config: &WikiVendorConfig,
    namespaces: &[WikiNamespace],
    progress: &mut WikiProgressReporter,
) -> Result<Vec<WikiPageRemoteMetadata>> {
    let mut pages = Vec::new();
    let content_namespaces = namespaces
        .iter()
        .filter(|namespace| namespace.id >= 0)
        .collect::<Vec<_>>();
    for namespace in content_namespaces {
        fetch_namespace_wiki_page_metadata(client, config, namespace, &mut pages, progress)?;
        if config
            .limit_pages
            .is_some_and(|limit_pages| pages.len() >= limit_pages)
        {
            pages.truncate(config.limit_pages.unwrap_or(pages.len()));
            break;
        }
    }
    pages.sort_by(compare_wiki_remote_metadata);
    Ok(pages)
}

#[requires(config.batch_size > 0)]
#[requires(namespace.id >= 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn fetch_namespace_wiki_page_metadata(
    client: &mut WikiHttpClient,
    config: &WikiVendorConfig,
    namespace: &WikiNamespace,
    pages: &mut Vec<WikiPageRemoteMetadata>,
    progress: &mut WikiProgressReporter,
) -> Result<()> {
    let start_count = pages.len();
    let mut api_continue = BTreeMap::<String, String>::new();
    loop {
        let mut params = vec![
            ("action", "query".to_owned()),
            ("generator", "allpages".to_owned()),
            ("gapnamespace", namespace.id.to_string()),
            ("gaplimit", config.batch_size.to_string()),
            ("gapfilterredir", "all".to_owned()),
            ("prop", "revisions|info".to_owned()),
            (
                "rvprop",
                "ids|timestamp|user|userid|comment|size|sha1|contentmodel".to_owned(),
            ),
            ("rvslots", "main".to_owned()),
            ("inprop", "protection".to_owned()),
        ];
        params.extend(
            api_continue
                .iter()
                .map(|(key, value)| (key.as_str(), value.clone())),
        );
        let payload = client.api_json(&params).with_context(|| {
            format!(
                "fetching metadata for namespace {} `{}`",
                namespace.id, namespace.name
            )
        })?;
        pages.extend(parse_wiki_api_pages(&payload)?);
        progress.update(&WikiProgressUpdate {
            phase: WikiProgressPhase::Metadata,
            current: pages.len(),
            total: 0,
            detail: format!(
                "namespace {} `{}`: {} page metadata records",
                namespace.id,
                namespace.name,
                pages.len() - start_count
            ),
        });
        if config
            .limit_pages
            .is_some_and(|limit_pages| pages.len() >= limit_pages)
        {
            break;
        }
        api_continue = wiki_continue_map(&payload)?;
        if api_continue.is_empty() {
            break;
        }
    }
    Ok(())
}

#[requires(!pageids.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|pages| pages.len() <= pageids.len()) || ret.is_err())]
fn fetch_wiki_page_metadata_by_pageids(
    client: &mut WikiHttpClient,
    pageids: &BTreeSet<u64>,
) -> Result<Vec<WikiPageRemoteMetadata>> {
    let pageids = pageids.iter().copied().collect::<Vec<_>>();
    let mut pages = Vec::new();
    for chunk in pageids.chunks(50) {
        let pageids_text = chunk
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join("|");
        let payload = client.api_json(&[
            ("action", "query".to_owned()),
            ("pageids", pageids_text),
            ("prop", "revisions|info".to_owned()),
            (
                "rvprop",
                "ids|timestamp|user|userid|comment|size|sha1|contentmodel".to_owned(),
            ),
            ("rvslots", "main".to_owned()),
            ("inprop", "protection".to_owned()),
        ])?;
        pages.extend(parse_wiki_api_pages(&payload)?);
    }
    pages.sort_by(compare_wiki_remote_metadata);
    Ok(pages)
}

#[requires(value.is_object())]
#[ensures(ret.as_ref().is_ok_and(|pages| pages.iter().all(|page| page.pageid > 0)) || ret.is_err())]
fn parse_wiki_api_pages(value: &serde_json::Value) -> Result<Vec<WikiPageRemoteMetadata>> {
    let Some(query) = value.get("query") else {
        return Ok(Vec::new());
    };
    let Some(pages) = query.get("pages") else {
        return Ok(Vec::new());
    };
    let pages = pages
        .as_array()
        .context("MediaWiki query.pages must be an array")?;
    pages
        .iter()
        .filter(|page| page.get("missing").is_none())
        .map(wiki_page_remote_metadata_from_value)
        .collect()
}

#[requires(value.is_object())]
#[ensures(ret.as_ref().is_ok_and(|page| page.pageid > 0) || ret.is_err())]
fn wiki_page_remote_metadata_from_value(
    value: &serde_json::Value,
) -> Result<WikiPageRemoteMetadata> {
    let revision = value
        .get("revisions")
        .and_then(serde_json::Value::as_array)
        .and_then(|revisions| revisions.first())
        .map(wiki_revision_metadata_from_value)
        .transpose()?;
    Ok(WikiPageRemoteMetadata {
        pageid: wiki_json_u64(value, "pageid")?,
        ns: wiki_json_i64(value, "ns")?,
        title: wiki_json_string(value, "title")?.to_owned(),
        touched: wiki_json_optional_string(value, "touched"),
        lastrevid: wiki_json_optional_u64(value, "lastrevid"),
        length: wiki_json_optional_u64(value, "length"),
        redirect: value.get("redirect").is_some(),
        protection: value
            .get("protection")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Array(Vec::new())),
        revision,
    })
}

#[requires(metadata.pageid > 0)]
#[ensures(ret.pageid == metadata.pageid)]
fn wiki_page_fetch_error(
    metadata: &WikiPageRemoteMetadata,
    error: &anyhow::Error,
) -> WikiPageFetchError {
    WikiPageFetchError {
        pageid: metadata.pageid,
        ns: metadata.ns,
        title: metadata.title.clone(),
        lastrevid: metadata.lastrevid,
        error: truncate_progress_text(&format!("{error:#}"), 2000),
    }
}

#[requires(value.is_object())]
#[ensures(ret.as_ref().is_ok_and(|revision| revision.revid > 0) || ret.is_err())]
fn wiki_revision_metadata_from_value(value: &serde_json::Value) -> Result<WikiRevisionMetadata> {
    let slot = value
        .get("slots")
        .and_then(|slots| slots.get("main"))
        .unwrap_or(value);
    Ok(WikiRevisionMetadata {
        revid: wiki_json_u64(value, "revid")?,
        parentid: wiki_json_optional_u64(value, "parentid"),
        timestamp: wiki_json_string(value, "timestamp")?.to_owned(),
        user: wiki_json_optional_string(value, "user"),
        userid: wiki_json_optional_u64(value, "userid"),
        comment: wiki_json_optional_string(value, "comment").unwrap_or_default(),
        size: wiki_json_optional_u64(value, "size"),
        sha1: wiki_json_optional_string(value, "sha1"),
        contentmodel: wiki_json_optional_string(slot, "contentmodel")
            .or_else(|| wiki_json_optional_string(value, "contentmodel"))
            .unwrap_or_else(|| "unknown".to_owned()),
        contentformat: wiki_json_optional_string(slot, "contentformat"),
    })
}

#[requires(!metadata.title.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|page| page.revision.revid > 0) || ret.is_err())]
fn fetch_wiki_page(
    client: &mut WikiHttpClient,
    metadata: &WikiPageRemoteMetadata,
) -> Result<WikiFetchedPage> {
    let parsoid = fetch_wiki_parsoid_html(client, &metadata.title)?;
    let source = fetch_wiki_source_revision(client, parsoid.revid)?;
    if !wiki_parsoid_source_pair_is_valid(&parsoid, &source) {
        bail!(
            "source revision {} is not a valid pair for Parsoid revision {} on `{}`",
            source.revision.revid,
            parsoid.revid,
            metadata.title
        );
    }
    let source_sha256 = sha256_hex(source.source.as_bytes());
    let parsoid_html_sha256 = sha256_hex(parsoid.html.as_bytes());
    Ok(WikiFetchedPage {
        pageid: source.pageid,
        ns: source.ns,
        title: source.title,
        redirect: source.redirect,
        touched: source.touched,
        lastrevid: source.lastrevid,
        length: source.length,
        protection: source.protection,
        revision: source.revision,
        source: source.source,
        parsoid_html: parsoid.html,
        source_sha256,
        parsoid_html_sha256,
    })
}

#[requires(!title.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|parsoid| parsoid.revid > 0 && !parsoid.html.is_empty()) || ret.is_err())]
fn fetch_wiki_parsoid_html(client: &mut WikiHttpClient, title: &str) -> Result<WikiParsoidHtml> {
    let path = format!("page/{}/with_html", wiki_rest_title_segment(title));
    let text = client.rest_text(&path)?;
    let value = serde_json::from_str::<serde_json::Value>(&text)
        .with_context(|| format!("parsing Parsoid with_html response for `{title}`"))?;
    let revid = value
        .get("latest")
        .and_then(|latest| latest.get("id"))
        .and_then(serde_json::Value::as_u64)
        .or_else(|| value.get("id").and_then(serde_json::Value::as_u64))
        .with_context(|| format!("Parsoid with_html response for `{title}` has no latest.id"))?;
    let html = wiki_json_string(&value, "html")?.to_owned();
    Ok(WikiParsoidHtml { revid, html })
}

#[requires(revid > 0)]
#[ensures(ret.as_ref().is_ok_and(|source| source.revision.revid == revid) || ret.is_err())]
fn fetch_wiki_source_revision(
    client: &mut WikiHttpClient,
    revid: u64,
) -> Result<WikiFetchedSourceRevision> {
    let payload = client.api_json(&[
        ("action", "query".to_owned()),
        ("revids", revid.to_string()),
        ("prop", "revisions|info".to_owned()),
        (
            "rvprop",
            "ids|timestamp|user|userid|comment|size|sha1|content|contentmodel".to_owned(),
        ),
        ("rvslots", "main".to_owned()),
        ("inprop", "protection".to_owned()),
    ])?;
    let page = payload
        .get("query")
        .and_then(|query| query.get("pages"))
        .and_then(serde_json::Value::as_array)
        .and_then(|pages| pages.first())
        .context("source revision query returned no page")?;
    let mut metadata = wiki_page_remote_metadata_from_value(page)?;
    let revision_value = page
        .get("revisions")
        .and_then(serde_json::Value::as_array)
        .and_then(|revisions| revisions.first())
        .context("source revision query returned no revision")?;
    let source = wiki_revision_source_text(revision_value)?;
    let revision = metadata
        .revision
        .take()
        .context("source revision query returned no revision metadata")?;
    if revision.revid != revid {
        bail!(
            "source revision query requested {revid} but returned {}",
            revision.revid
        );
    }
    Ok(WikiFetchedSourceRevision {
        pageid: metadata.pageid,
        ns: metadata.ns,
        title: metadata.title,
        touched: metadata.touched,
        lastrevid: metadata.lastrevid,
        length: metadata.length,
        redirect: metadata.redirect,
        protection: metadata.protection,
        revision,
        source,
    })
}

#[requires(value.is_object())]
#[ensures(ret.as_ref().is_ok() || ret.is_err())]
fn wiki_revision_source_text(value: &serde_json::Value) -> Result<String> {
    let slot = value
        .get("slots")
        .and_then(|slots| slots.get("main"))
        .unwrap_or(value);
    slot.get("content")
        .or_else(|| slot.get("*"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .context("revision source content is missing")
}

#[requires(true)]
#[ensures(true)]
fn has_parsoid_html_metadata(html: &str) -> bool {
    html.contains("mw:htmlVersion") || html.contains("mw:html:version")
}

#[requires(parsoid.revid > 0)]
#[requires(source.revision.revid > 0)]
#[ensures(true)]
fn wiki_parsoid_source_pair_is_valid(
    parsoid: &WikiParsoidHtml,
    source: &WikiFetchedSourceRevision,
) -> bool {
    source.revision.revid == parsoid.revid && has_parsoid_html_metadata(&parsoid.html)
}

#[requires(!title.is_empty())]
#[ensures(!ret.is_empty())]
fn wiki_rest_title_segment(title: &str) -> String {
    percent_encode_path_segment(&title.replace(' ', "_"))
}

#[requires(true)]
#[ensures(!ret.contains('/'))]
fn percent_encode_path_segment(value: &str) -> String {
    let mut output = String::new();
    for byte in value.as_bytes() {
        if path_segment_byte_is_unreserved(*byte) {
            output.push(char::from(*byte));
        } else {
            output.push('%');
            output.push(hex_digit(byte >> 4));
            output.push(hex_digit(byte & 0x0f));
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn path_segment_byte_is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

#[requires(value < 16)]
#[ensures(ret.is_ascii_hexdigit())]
fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'A' + (value - 10)),
        _ => unreachable!("hex digit contract requires value < 16"),
    }
}

#[requires(value.is_object())]
#[requires(!field.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn wiki_json_string<'a>(value: &'a serde_json::Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .filter(|text| !text.is_empty())
        .with_context(|| format!("MediaWiki JSON field `{field}` must be a non-empty string"))
}

#[requires(value.is_object())]
#[requires(!field.is_empty())]
#[ensures(true)]
fn wiki_json_optional_string(value: &serde_json::Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

#[requires(value.is_object())]
#[requires(!field.is_empty())]
#[ensures(ret.as_ref().is_ok() || ret.is_err())]
fn wiki_json_u64(value: &serde_json::Value, field: &str) -> Result<u64> {
    value
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .with_context(|| format!("MediaWiki JSON field `{field}` must be an unsigned integer"))
}

#[requires(value.is_object())]
#[requires(!field.is_empty())]
#[ensures(true)]
fn wiki_json_optional_u64(value: &serde_json::Value, field: &str) -> Option<u64> {
    value.get(field).and_then(serde_json::Value::as_u64)
}

#[requires(value.is_object())]
#[requires(!field.is_empty())]
#[ensures(ret.as_ref().is_ok() || ret.is_err())]
fn wiki_json_i64(value: &serde_json::Value, field: &str) -> Result<i64> {
    value
        .get(field)
        .and_then(serde_json::Value::as_i64)
        .with_context(|| format!("MediaWiki JSON field `{field}` must be an integer"))
}

#[requires(value.is_object())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn wiki_continue_map(value: &serde_json::Value) -> Result<BTreeMap<String, String>> {
    let Some(continue_value) = value.get("continue") else {
        return Ok(BTreeMap::new());
    };
    let object = continue_value
        .as_object()
        .context("MediaWiki continue value must be an object")?;
    let mut result = BTreeMap::new();
    for (key, value) in object {
        if let Some(value) = value.as_str() {
            result.insert(key.clone(), value.to_owned());
        } else if let Some(value) = value.as_i64() {
            result.insert(key.clone(), value.to_string());
        }
    }
    Ok(result)
}

#[requires(true)]
#[ensures(true)]
fn compare_wiki_remote_metadata(
    left: &WikiPageRemoteMetadata,
    right: &WikiPageRemoteMetadata,
) -> std::cmp::Ordering {
    left.ns
        .cmp(&right.ns)
        .then_with(|| left.title.cmp(&right.title))
        .then_with(|| left.pageid.cmp(&right.pageid))
}

#[requires(true)]
#[ensures(true)]
fn compare_wiki_page_index_entries(
    left: &WikiPageIndexEntry,
    right: &WikiPageIndexEntry,
) -> std::cmp::Ordering {
    left.ns
        .cmp(&right.ns)
        .then_with(|| left.title.cmp(&right.title))
        .then_with(|| left.pageid.cmp(&right.pageid))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|manifest| manifest.get("files").and_then(serde_json::Value::as_array).is_some()) || ret.is_err())]
fn fetch_wiki_media_manifest(
    client: &mut WikiHttpClient,
    progress: &mut WikiProgressReporter,
) -> Result<serde_json::Value> {
    let mut files = Vec::new();
    let mut api_continue = BTreeMap::<String, String>::new();
    loop {
        let mut params = vec![
            ("action", "query".to_owned()),
            ("list", "allimages".to_owned()),
            ("ailimit", "500".to_owned()),
            (
                "aiprop",
                "timestamp|user|userid|url|size|sha1|mime|mediatype|metadata|commonmetadata|extmetadata"
                    .to_owned(),
            ),
        ];
        params.extend(
            api_continue
                .iter()
                .map(|(key, value)| (key.as_str(), value.clone())),
        );
        let payload = client
            .api_json(&params)
            .context("fetching upload metadata")?;
        let batch = payload
            .get("query")
            .and_then(|query| query.get("allimages"))
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        files.extend(batch);
        progress.update(&WikiProgressUpdate {
            phase: WikiProgressPhase::Media,
            current: files.len(),
            total: 0,
            detail: format!("{} upload metadata records", files.len()),
        });
        api_continue = wiki_continue_map(&payload)?;
        if api_continue.is_empty() {
            break;
        }
    }
    files.sort_by(compare_wiki_media_files);
    let summary = summarize_wiki_media_files(&files);
    Ok(serde_json::json!({
        "fetchedAt": rfc3339_now()?,
        "note": "Upload binaries are not vendored here; this manifest preserves MediaWiki file metadata and canonical URLs.",
        "summary": summary,
        "files": files,
    }))
}

#[requires(true)]
#[ensures(true)]
fn compare_wiki_media_files(
    left: &serde_json::Value,
    right: &serde_json::Value,
) -> std::cmp::Ordering {
    let left_name = left
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let right_name = right
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    left_name.cmp(right_name)
}

#[requires(true)]
#[ensures(ret.is_object())]
fn summarize_wiki_media_files(files: &[serde_json::Value]) -> serde_json::Value {
    let total_bytes = files.iter().map(wiki_media_file_size).sum::<u64>();
    let image_files = files
        .iter()
        .filter(|file| {
            file.get("mime")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|mime| mime.starts_with("image/"))
        })
        .collect::<Vec<_>>();
    let image_bytes = image_files
        .iter()
        .map(|file| wiki_media_file_size(file))
        .sum::<u64>();
    serde_json::json!({
        "count": files.len(),
        "totalBytes": total_bytes,
        "imageCount": image_files.len(),
        "imageBytes": image_bytes,
        "byMime": summarize_wiki_media_by(files, "mime"),
        "byMediaType": summarize_wiki_media_by(files, "mediatype"),
    })
}

#[requires(true)]
#[ensures(true)]
fn wiki_media_file_size(file: &serde_json::Value) -> u64 {
    file.get("size")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0)
}

#[requires(!field.is_empty())]
#[ensures(ret.is_array())]
fn summarize_wiki_media_by(files: &[serde_json::Value], field: &str) -> serde_json::Value {
    let mut rows = BTreeMap::<String, (usize, u64)>::new();
    for file in files {
        let key = file
            .get(field)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let entry = rows.entry(key).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += wiki_media_file_size(file);
    }
    let mut rows = rows
        .into_iter()
        .map(|(key, (count, bytes))| {
            serde_json::json!({
                "key": key,
                "count": count,
                "bytes": bytes,
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        let left_bytes = left
            .get("bytes")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let right_bytes = right
            .get("bytes")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        right_bytes.cmp(&left_bytes).then_with(|| {
            left.get("key")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .cmp(
                    right
                        .get("key")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(""),
                )
        })
    });
    serde_json::Value::Array(rows)
}

#[requires(config.batch_size > 0)]
#[ensures(ret.as_ref().is_ok_and(|plan| plan.keep.iter().all(|page| page.pageid > 0) && plan.fetch.iter().all(|page| page.pageid > 0)) || ret.is_err())]
fn build_wiki_snapshot_plan(
    client: &mut WikiHttpClient,
    config: &WikiVendorConfig,
    namespaces: &[WikiNamespace],
    existing: Option<&WikiExistingSnapshot>,
    output: &Path,
    progress: &mut WikiProgressReporter,
) -> Result<WikiSnapshotPlan> {
    progress.update(&WikiProgressUpdate {
        phase: WikiProgressPhase::Planning,
        current: 0,
        total: 0,
        detail: "choosing update strategy".to_owned(),
    });

    if let Some(existing) = existing
        && config.limit_pages.is_none()
        && existing_wiki_snapshot_supports_parsoid(output, existing)
        && let Some(oldest_recentchange) = fetch_oldest_recentchange_timestamp(client)?
        && recentchanges_covers_snapshot(&existing.fetched_at, &oldest_recentchange)
    {
        progress.update(&WikiProgressUpdate {
            phase: WikiProgressPhase::RecentChanges,
            current: 0,
            total: 0,
            detail: format!("fetching changes since {}", existing.fetched_at),
        });
        let recentchanges = fetch_wiki_recentchanges_since(client, &existing.fetched_at)?;
        if !recentchanges.needs_full_reconcile {
            let changed_metadata = if recentchanges.changed_pageids.is_empty() {
                Vec::new()
            } else {
                fetch_wiki_page_metadata_by_pageids(client, &recentchanges.changed_pageids)?
            };
            if changed_metadata.len() == recentchanges.changed_pageids.len() {
                return Ok(build_recentchanges_wiki_snapshot_plan(
                    existing,
                    &changed_metadata,
                    recentchanges.change_count,
                ));
            }
        }
    }

    progress.update(&WikiProgressUpdate {
        phase: WikiProgressPhase::Metadata,
        current: 0,
        total: 0,
        detail: "running full page metadata reconciliation".to_owned(),
    });
    let remote_pages = fetch_all_wiki_page_metadata(client, config, namespaces, progress)?;
    Ok(build_full_wiki_snapshot_plan(
        existing,
        output,
        &remote_pages,
        config.limit_pages.is_some(),
    ))
}

#[requires(true)]
#[ensures(true)]
fn existing_wiki_snapshot_supports_parsoid(output: &Path, existing: &WikiExistingSnapshot) -> bool {
    existing_wiki_snapshot_has_no_page_errors(output)
        && existing.pages.iter().all(|entry| {
            !entry.parsoid_html.is_empty()
                && !entry.source_sha256.is_empty()
                && !entry.parsoid_html_sha256.is_empty()
                && output.join(&entry.meta).is_file()
                && output.join(&entry.source).is_file()
                && output.join(&entry.parsoid_html).is_file()
        })
}

#[requires(true)]
#[ensures(true)]
fn existing_wiki_snapshot_has_no_page_errors(output: &Path) -> bool {
    let errors_path = output.join("pages").join("errors.json");
    if !errors_path.exists() {
        return true;
    }
    fs::read_to_string(&errors_path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.as_array().map(Vec::is_empty))
        .unwrap_or(false)
}

#[requires(true)]
#[ensures(true)]
fn recentchanges_covers_snapshot(snapshot_fetched_at: &str, oldest_recentchange_at: &str) -> bool {
    !snapshot_fetched_at.trim().is_empty()
        && !oldest_recentchange_at.trim().is_empty()
        && snapshot_fetched_at >= oldest_recentchange_at
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn fetch_oldest_recentchange_timestamp(client: &mut WikiHttpClient) -> Result<Option<String>> {
    let payload = client.api_json(&[
        ("action", "query".to_owned()),
        ("list", "recentchanges".to_owned()),
        ("rcdir", "newer".to_owned()),
        ("rclimit", "1".to_owned()),
        ("rcprop", "timestamp".to_owned()),
    ])?;
    Ok(payload
        .get("query")
        .and_then(|query| query.get("recentchanges"))
        .and_then(serde_json::Value::as_array)
        .and_then(|changes| changes.first())
        .and_then(|change| change.get("timestamp"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned))
}

#[requires(!since.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn fetch_wiki_recentchanges_since(
    client: &mut WikiHttpClient,
    since: &str,
) -> Result<WikiRecentChangesSummary> {
    let mut changed_pageids = BTreeSet::new();
    let mut needs_full_reconcile = false;
    let mut change_count = 0usize;
    let mut api_continue = BTreeMap::<String, String>::new();
    loop {
        let mut params = vec![
            ("action", "query".to_owned()),
            ("list", "recentchanges".to_owned()),
            ("rcstart", since.to_owned()),
            ("rcdir", "newer".to_owned()),
            ("rclimit", "500".to_owned()),
            (
                "rcprop",
                "title|ids|timestamp|type|loginfo|flags|sizes|sha1".to_owned(),
            ),
        ];
        params.extend(
            api_continue
                .iter()
                .map(|(key, value)| (key.as_str(), value.clone())),
        );
        let payload = client
            .api_json(&params)
            .context("fetching MediaWiki recentchanges")?;
        let changes = payload
            .get("query")
            .and_then(|query| query.get("recentchanges"))
            .and_then(serde_json::Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        for change in changes {
            change_count += 1;
            if wiki_recentchange_requires_full_reconcile(change) {
                needs_full_reconcile = true;
            }
            if let Some(pageid) = change.get("pageid").and_then(serde_json::Value::as_u64) {
                if pageid > 0 {
                    changed_pageids.insert(pageid);
                }
            } else if !wiki_recentchange_is_media_only(change) {
                needs_full_reconcile = true;
            }
        }
        api_continue = wiki_continue_map(&payload)?;
        if api_continue.is_empty() {
            break;
        }
    }
    Ok(WikiRecentChangesSummary {
        changed_pageids,
        needs_full_reconcile,
        change_count,
    })
}

#[requires(change.is_object())]
#[ensures(true)]
fn wiki_recentchange_requires_full_reconcile(change: &serde_json::Value) -> bool {
    let change_type = change
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if change_type != "log" {
        return false;
    }
    let log_type = change
        .get("logtype")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    matches!(log_type, "delete" | "move" | "import" | "merge")
}

#[requires(change.is_object())]
#[ensures(true)]
fn wiki_recentchange_is_media_only(change: &serde_json::Value) -> bool {
    change
        .get("logtype")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|log_type| matches!(log_type, "upload"))
}

#[requires(true)]
#[ensures(ret.source == WikiSnapshotPlanSource::RecentChanges)]
fn build_recentchanges_wiki_snapshot_plan(
    existing: &WikiExistingSnapshot,
    changed_metadata: &[WikiPageRemoteMetadata],
    change_count: usize,
) -> WikiSnapshotPlan {
    let changed = changed_metadata
        .iter()
        .map(|metadata| (metadata.pageid, metadata))
        .collect::<BTreeMap<_, _>>();
    let mut keep = Vec::new();
    let mut fetch = Vec::new();
    for entry in &existing.pages {
        if let Some(metadata) = changed.get(&entry.pageid) {
            if wiki_page_entry_matches_remote_metadata(entry, metadata) {
                keep.push(entry.clone());
            } else {
                fetch.push((*metadata).clone());
            }
        } else {
            keep.push(entry.clone());
        }
    }
    let existing_pageids = existing
        .pages
        .iter()
        .map(|entry| entry.pageid)
        .collect::<BTreeSet<_>>();
    for metadata in changed_metadata {
        if !existing_pageids.contains(&metadata.pageid) {
            fetch.push(metadata.clone());
        }
    }
    eprintln!(
        "[wiki] recentchanges fast path: {change_count} change(s), {} page(s) to fetch",
        fetch.len()
    );
    WikiSnapshotPlan {
        keep,
        fetch,
        removed: Vec::new(),
        source: WikiSnapshotPlanSource::RecentChanges,
    }
}

#[requires(true)]
#[ensures(ret.source == WikiSnapshotPlanSource::FullReconcile)]
fn build_full_wiki_snapshot_plan(
    existing: Option<&WikiExistingSnapshot>,
    output: &Path,
    remote_pages: &[WikiPageRemoteMetadata],
    limited: bool,
) -> WikiSnapshotPlan {
    let existing_by_pageid = existing
        .map(|snapshot| {
            snapshot
                .pages
                .iter()
                .map(|entry| (entry.pageid, entry))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let remote_pageids = remote_pages
        .iter()
        .map(|metadata| metadata.pageid)
        .collect::<BTreeSet<_>>();
    let mut keep = Vec::new();
    let mut fetch = Vec::new();
    for metadata in remote_pages {
        if let Some(entry) = existing_by_pageid.get(&metadata.pageid)
            && wiki_existing_entry_complete(output, entry)
            && wiki_page_entry_matches_remote_metadata(entry, metadata)
        {
            keep.push((*entry).clone());
            continue;
        }
        fetch.push(metadata.clone());
    }
    let removed = if limited {
        Vec::new()
    } else {
        existing
            .map(|snapshot| {
                snapshot
                    .pages
                    .iter()
                    .filter(|entry| !remote_pageids.contains(&entry.pageid))
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    WikiSnapshotPlan {
        keep,
        fetch,
        removed,
        source: WikiSnapshotPlanSource::FullReconcile,
    }
}

#[requires(entry.pageid > 0)]
#[ensures(true)]
fn wiki_existing_entry_complete(output: &Path, entry: &WikiPageIndexEntry) -> bool {
    !entry.parsoid_html.is_empty()
        && !entry.source_sha256.is_empty()
        && !entry.parsoid_html_sha256.is_empty()
        && output.join(&entry.meta).is_file()
        && output.join(&entry.source).is_file()
        && output.join(&entry.parsoid_html).is_file()
}

#[requires(entry.pageid > 0)]
#[requires(metadata.pageid > 0)]
#[ensures(true)]
fn wiki_page_entry_matches_remote_metadata(
    entry: &WikiPageIndexEntry,
    metadata: &WikiPageRemoteMetadata,
) -> bool {
    entry.pageid == metadata.pageid
        && entry.ns == metadata.ns
        && entry.title == metadata.title
        && entry.redirect == metadata.redirect
        && entry.revid == metadata.revision.as_ref().map(|revision| revision.revid)
        && entry.timestamp
            == metadata
                .revision
                .as_ref()
                .map(|revision| revision.timestamp.clone())
        && entry.model == wiki_remote_metadata_model(metadata)
        && entry.bytes == wiki_remote_metadata_bytes(metadata)
}

#[requires(metadata.pageid > 0)]
#[ensures(!ret.is_empty())]
fn wiki_remote_metadata_model(metadata: &WikiPageRemoteMetadata) -> String {
    metadata
        .revision
        .as_ref()
        .map(|revision| revision.contentmodel.clone())
        .unwrap_or_else(|| "unknown".to_owned())
}

#[requires(metadata.pageid > 0)]
#[ensures(true)]
fn wiki_remote_metadata_bytes(metadata: &WikiPageRemoteMetadata) -> usize {
    metadata
        .revision
        .as_ref()
        .and_then(|revision| revision.size)
        .or(metadata.length)
        .and_then(|bytes| usize::try_from(bytes).ok())
        .unwrap_or(0)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn load_existing_wiki_snapshot(output: &Path) -> Result<Option<WikiExistingSnapshot>> {
    if !output.exists() {
        return Ok(None);
    }
    if !output.is_dir() {
        bail!(
            "wiki output `{}` exists but is not a directory",
            output.display()
        );
    }
    Ok(Some(read_existing_wiki_snapshot(output)?))
}

#[requires(output.is_dir())]
#[ensures(ret.as_ref().is_ok_and(|snapshot| !snapshot.fetched_at.is_empty()) || ret.is_err())]
fn read_existing_wiki_snapshot(output: &Path) -> Result<WikiExistingSnapshot> {
    let snapshot_value = read_json_file(&output.join("snapshot.json"))
        .with_context(|| format!("reading wiki snapshot metadata from `{}`", output.display()))?;
    let fetched_at = snapshot_value
        .get("fetchedAt")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .context("wiki snapshot.json must contain fetchedAt")?;
    let index_value = read_json_file(&output.join("pages").join("index.json"))
        .with_context(|| format!("reading wiki page index from `{}`", output.display()))?;
    let pages = serde_json::from_value::<Vec<WikiPageIndexEntry>>(index_value)
        .context("parsing wiki page index")?;
    Ok(WikiExistingSnapshot { fetched_at, pages })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|snapshot| !snapshot.fetched_at.is_empty()) || ret.is_err())]
fn validate_existing_wiki_snapshot(output: &Path) -> Result<WikiExistingSnapshot> {
    if !output.is_dir() {
        bail!(
            "wiki snapshot directory `{}` does not exist",
            output.display()
        );
    }
    let snapshot = read_existing_wiki_snapshot(output)?;
    for entry in &snapshot.pages {
        if !wiki_existing_entry_complete(output, entry) {
            bail!(
                "wiki page {} `{}` is missing required source, Parsoid HTML, metadata, or digest fields",
                entry.pageid,
                entry.title
            );
        }
    }
    validate_directory_digests(output)?;
    Ok(snapshot)
}

#[requires(stage.is_dir())]
#[requires(page.pageid > 0)]
#[ensures(ret.as_ref().is_ok_and(|report| report.entry.pageid == page.pageid) || ret.is_err())]
fn write_wiki_page(stage: &Path, page: &WikiFetchedPage) -> Result<WikiPageWriteReport> {
    let paths = wiki_page_paths(page.pageid);
    let absolute_base = stage.join(&paths.base);
    fs::create_dir_all(&absolute_base)
        .with_context(|| format!("creating `{}`", absolute_base.display()))?;
    let source_path = stage.join(&paths.source);
    let html_path = stage.join(&paths.parsoid_html);
    let meta_path = stage.join(&paths.meta);
    fs::write(&source_path, &page.source)
        .with_context(|| format!("writing `{}`", source_path.display()))?;
    fs::write(&html_path, &page.parsoid_html)
        .with_context(|| format!("writing `{}`", html_path.display()))?;
    let source_relative = relative_path_string(&paths.source);
    let html_relative = relative_path_string(&paths.parsoid_html);
    let meta = WikiPageMetadataFile {
        pageid: page.pageid,
        ns: page.ns,
        title: page.title.clone(),
        touched: page.touched.clone(),
        lastrevid: page.lastrevid,
        length: page.length,
        redirect: page.redirect,
        protection: page.protection.clone(),
        revision: Some(page.revision.clone()),
        source_sha256: page.source_sha256.clone(),
        parsoid_html_sha256: page.parsoid_html_sha256.clone(),
        source_path: source_relative.clone(),
        parsoid_html_path: html_relative.clone(),
    };
    write_json_file(
        &meta_path,
        &serde_json::to_value(&meta).context("serializing wiki page metadata")?,
    )?;
    let source_bytes = page.source.len();
    let entry = WikiPageIndexEntry {
        pageid: page.pageid,
        ns: page.ns,
        title: page.title.clone(),
        redirect: page.redirect,
        revid: Some(page.revision.revid),
        timestamp: Some(page.revision.timestamp.clone()),
        model: page.revision.contentmodel.clone(),
        bytes: source_bytes,
        source_sha256: page.source_sha256.clone(),
        parsoid_html_sha256: page.parsoid_html_sha256.clone(),
        meta: relative_path_string(&paths.meta),
        source: source_relative,
        parsoid_html: html_relative,
    };
    Ok(WikiPageWriteReport {
        entry,
        source_bytes,
    })
}

#[requires(source_root.is_dir())]
#[requires(entry.pageid > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_wiki_page_entry(
    source_root: &Path,
    target_root: &Path,
    entry: &WikiPageIndexEntry,
) -> Result<()> {
    copy_wiki_snapshot_file(source_root, target_root, &entry.meta)?;
    copy_wiki_snapshot_file(source_root, target_root, &entry.source)?;
    copy_wiki_snapshot_file(source_root, target_root, &entry.parsoid_html)
}

#[requires(source_root.is_dir())]
#[requires(!relative.trim().is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn copy_wiki_snapshot_file(source_root: &Path, target_root: &Path, relative: &str) -> Result<()> {
    let source = source_root.join(relative);
    let target = target_root.join(relative);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating `{}`", parent.display()))?;
    }
    fs::copy(&source, &target)
        .with_context(|| format!("copying `{}` to `{}`", source.display(), target.display()))?;
    Ok(())
}

#[requires(pageid > 0)]
#[ensures(!ret.base.as_os_str().is_empty())]
fn wiki_page_paths(pageid: u64) -> WikiPagePaths {
    let base = PathBuf::from(format!("pages/by-id/{pageid:08}"));
    WikiPagePaths {
        meta: base.join("meta.json"),
        source: base.join("source.wiki"),
        parsoid_html: base.join("parsoid.html"),
        base,
    }
}

#[requires(path.components().next().is_some())]
#[ensures(!ret.contains('\\'))]
fn relative_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|path| path.parent().is_some()) || ret.is_err())]
fn wiki_stage_dir(output: &Path) -> Result<PathBuf> {
    Ok(wiki_sibling_work_dir(output, "tmp")?)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|path| path.parent().is_some()) || ret.is_err())]
fn wiki_backup_dir(output: &Path) -> Result<PathBuf> {
    Ok(wiki_sibling_work_dir(output, "previous")?)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|path| path.parent().is_some()) || ret.is_err())]
fn wiki_sibling_work_dir(output: &Path, suffix: &str) -> Result<PathBuf> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .context("wiki output path must have a final path component")?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_nanos();
    Ok(parent.join(format!(".{name}.{suffix}-{}-{nonce}", std::process::id())))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn remove_path_if_exists(path: &Path) -> Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("removing `{}`", path.display())),
    }
}

#[requires(stage.is_dir())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn replace_wiki_snapshot_directory(output: &Path, stage: &Path, backup: &Path) -> Result<()> {
    remove_path_if_exists(backup)?;
    if output.exists() {
        fs::rename(output, backup).with_context(|| {
            format!(
                "moving previous wiki snapshot `{}` to `{}`",
                output.display(),
                backup.display()
            )
        })?;
    }
    match fs::rename(stage, output) {
        Ok(()) => {
            remove_path_if_exists(backup)?;
            Ok(())
        }
        Err(error) => {
            if backup.exists() {
                let _ = fs::rename(backup, output);
            }
            Err(error).with_context(|| {
                format!(
                    "promoting wiki snapshot `{}` to `{}`",
                    stage.display(),
                    output.display()
                )
            })
        }
    }
}

#[requires(root.is_dir())]
#[ensures(ret.as_ref().is_ok_and(|rows| rows.windows(2).all(|pair| pair[0].path <= pair[1].path)) || ret.is_err())]
fn directory_digest_rows(root: &Path) -> Result<Vec<DirectoryDigestRow>> {
    let mut rows = Vec::new();
    for entry in WalkDir::new(root).sort_by_file_name() {
        let entry = entry.with_context(|| format!("walking `{}`", root.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .with_context(|| format!("computing relative path under `{}`", root.display()))?;
        let path = relative_path_string(relative);
        if path == "DIGESTS.sha256" {
            continue;
        }
        let bytes = fs::read(entry.path())
            .with_context(|| format!("reading `{}` for digest", entry.path().display()))?;
        rows.push(DirectoryDigestRow {
            digest: sha256_hex(&bytes),
            path,
        });
    }
    rows.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(rows)
}

#[requires(true)]
#[ensures(ret.ends_with('\n'))]
fn render_directory_digests(rows: &[DirectoryDigestRow]) -> String {
    let mut text = String::new();
    for row in rows {
        text.push_str(&row.digest);
        text.push_str("  ");
        text.push_str(&row.path);
        text.push('\n');
    }
    text
}

#[requires(root.is_dir())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_directory_digests(root: &Path) -> Result<()> {
    let digest_path = root.join("DIGESTS.sha256");
    let expected = fs::read_to_string(&digest_path)
        .with_context(|| format!("reading `{}`", digest_path.display()))?;
    let actual = render_directory_digests(&directory_digest_rows(root)?);
    if expected != actual {
        bail!(
            "wiki snapshot digest manifest `{}` is stale",
            digest_path.display()
        );
    }
    Ok(())
}

#[requires(!args.started_at.trim().is_empty())]
#[requires(!args.finished_at.trim().is_empty())]
#[ensures(ret.is_object())]
fn render_wiki_snapshot_metadata(args: &WikiSnapshotMetadataRenderArgs<'_>) -> serde_json::Value {
    serde_json::json!({
        "sourceUrl": args.config.source_url,
        "apiUrl": args.config.api_url,
        "restUrl": args.config.rest_url,
        "startedAt": args.started_at,
        "fetchedAt": args.finished_at,
        "mediaWikiGenerator": args.siteinfo
            .get("query")
            .and_then(|query| query.get("general"))
            .and_then(|general| general.get("generator"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown"),
        "requestPolicy": {
            "batchSize": args.config.batch_size,
            "delayMs": args.config.delay.as_millis(),
            "timeoutMs": args.config.timeout.as_millis(),
            "retries": args.config.retries,
            "maxlag": args.config.maxlag,
            "limitPages": args.config.limit_pages,
        },
        "incremental": {
            "source": match args.plan.source {
                WikiSnapshotPlanSource::RecentChanges => "recentchanges",
                WikiSnapshotPlanSource::FullReconcile => "full-reconcile",
            },
            "kept": args.plan.keep.len(),
            "fetched": args.plan.fetch.len(),
            "removed": args.plan.removed.len(),
        },
        "pages": {
            "count": args.page_count,
            "failed": args.failed_pages.len(),
            "currentRevisionContentBytes": args.source_bytes,
        },
        "failedPages": args.failed_pages,
        "media": args.media_manifest.get("summary").cloned().unwrap_or(serde_json::Value::Null),
    })
}

#[requires(true)]
#[ensures(ret.ends_with('\n'))]
fn render_wiki_snapshot_readme(
    config: &WikiVendorConfig,
    page_count: usize,
    failed_page_count: usize,
    source_bytes: usize,
    media_manifest: &serde_json::Value,
) -> String {
    let media_summary = media_manifest
        .get("summary")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let media_count = media_summary
        .get("count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let media_bytes = media_summary
        .get("totalBytes")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    format!(
        "# Lojban Wiki Snapshot\n\n\
This directory is a vendored current-revision Parsoid/source snapshot of {}.\n\n\
The snapshot is generated by `cargo xtask vendor-wiki`. Uploaded media binaries are not included.\n\n\
## Contents\n\n\
- `siteinfo.json`: MediaWiki site metadata.\n\
- `namespaces.json`: normalized namespace metadata.\n\
- `pages/index.json`: page manifest with stable page-id paths.\n\
- `pages/errors.json`: pages that the wiki listed but could not return as a source/Parsoid pair.\n\
- `pages/by-id/*/source.wiki`: exact raw source for the stored revision.\n\
- `pages/by-id/*/parsoid.html`: annotated Parsoid HTML for the same revision.\n\
- `pages/by-id/*/meta.json`: page and revision metadata.\n\
- `media/manifest.json`: upload metadata and canonical URLs, without binaries.\n\
- `DIGESTS.sha256`: SHA-256 manifest for all other snapshot files.\n\n\
The current snapshot contains {} stored page(s), {} page fetch failure(s), {}, and {} media record(s) totaling {}.\n",
        config.source_url,
        page_count,
        failed_page_count,
        human_bytes(source_bytes as u64),
        media_count,
        human_bytes(media_bytes)
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.ends_with('Z')) || ret.is_err())]
fn rfc3339_now() -> Result<String> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_secs();
    Ok(unix_seconds_to_rfc3339(seconds))
}

#[requires(true)]
#[ensures(ret.ends_with('Z'))]
fn unix_seconds_to_rfc3339(seconds: u64) -> String {
    let days = seconds / 86_400;
    let second_of_day = seconds % 86_400;
    let date = civil_from_days(days as i64);
    let hour = second_of_day / 3_600;
    let minute = (second_of_day % 3_600) / 60;
    let second = second_of_day % 60;
    format!(
        "{:04}-{:02}-{:02}T{hour:02}:{minute:02}:{second:02}Z",
        date.year, date.month, date.day
    )
}

#[requires(true)]
#[ensures((1..=12).contains(&ret.month))]
#[ensures((1..=31).contains(&ret.day))]
fn civil_from_days(days_since_unix_epoch: i64) -> UtcDate {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    UtcDate {
        year,
        month: month as u32,
        day: day as u32,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn human_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
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
    sumti_assignments: Vec<V0SumtiAssignmentFact>,
    #[serde(default, rename = "relation-places")]
    selbri_places: Vec<V0SelbriPlaceFact>,
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
        !self.sumti_assignments.is_empty()
            || !self.selbri_places.is_empty()
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
struct V0SumtiAssignmentFact {
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
struct V0SelbriPlaceFact {
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
    for assignment in &refs.sumti_assignments {
        if !projection_contains_v0_assignment(projection, assignment) {
            failures.push(format!(
                "{}: missing argument assignment argument={:?} relation={:?} place={:?}",
                case.id, assignment.argument, assignment.relation, assignment.place_index
            ));
        }
    }
    for relation_place in &refs.selbri_places {
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
    expected: &V0SumtiAssignmentFact,
) -> bool {
    projection.assignments.iter().any(|actual| {
        (actual.sumti == expected.argument
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
    assignment: &jbotci_semantics::references::FixtureSumtiAssignment,
    expected_argument: &FixtureSpanKey,
) -> bool {
    projection.references.iter().any(|edge| {
        edge.source == assignment.sumti
            && reference_target_contains(&edge.target, expected_argument)
    })
}

#[requires(true)]
#[ensures(true)]
fn projection_contains_v0_relation_place(
    projection: &ReferenceFixtureProjection,
    expected: &V0SelbriPlaceFact,
) -> bool {
    projection.assignments.iter().any(|actual| {
        actual.sumti == expected.argument
            && assignment_matches_relation(actual, &expected.relation)
            && assignment_reaches_numbered_place(projection, actual, expected.place)
    })
}

#[requires(true)]
#[ensures(true)]
fn assignment_matches_relation(
    assignment: &jbotci_semantics::references::FixtureSumtiAssignment,
    relation: &FixtureSpanKey,
) -> bool {
    assignment.selbri.as_ref() == Some(relation)
        || assignment.tanru_unit.as_ref() == Some(relation)
        || assignment.frame_node == *relation
        || assignment
            .selbri
            .as_ref()
            .is_some_and(|actual| span_is_suffix_of(actual, relation))
        || assignment
            .tanru_unit
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
    assignment: &jbotci_semantics::references::FixtureSumtiAssignment,
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
        jbotci_semantics::references::FixturePlaceFramePropagation::ConnectiveBranches {
            branches,
        } => branches
            .iter()
            .any(|branch| slot_reaches_numbered_place(projection, *branch, slot, place, visited)),
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
    vlasei.brackets = Some(fixtures::BracketExpectations::latin(text_expectation(
        pretty_morphology_brackets_with_options(
            words,
            &fixture.test_case.lojban,
            BracketRenderOptions {
                color: false,
                ..BracketRenderOptions::default()
            },
        )?,
    )));
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
    let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
        &fixture.test_case.lojban,
        &morphology_options,
        Some(SourceId("<fixture>".to_owned())),
    )
    .into_data();
    let morphology_warning_diagnostics = morphology_warning_diagnostic_expectation_items(
        &fixture.test_case.lojban,
        &attempt.warnings,
    );
    let words = attempt.result;
    if let Some(morphology) = &mut fixture.test_case.expectations.morphology {
        if morphology.status == ExpectationStatus::Failure
            && let Err(error) = &words
        {
            let diagnostic = error.to_diagnostic(
                Some(SourceId("<fixture>".to_owned())),
                &fixture.test_case.lojban,
            );
            let mut diagnostics = morphology_warning_diagnostics.clone();
            diagnostics.extend(diagnostic_expectation_items(
                &fixture.test_case.lojban,
                std::slice::from_ref(&diagnostic),
            ));
            morphology.diagnostics = diagnostics;
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
            vlasei.brackets = Some(fixtures::BracketExpectations::latin(text_expectation(
                pretty_morphology_brackets_with_options(
                    &morphology_words,
                    &fixture.test_case.lojban,
                    BracketRenderOptions {
                        color: false,
                        ..BracketRenderOptions::default()
                    },
                )?,
            )));
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
                    let mut diagnostics = morphology_warning_diagnostics.clone();
                    diagnostics.extend(diagnostic_expectation_items(
                        &fixture.test_case.lojban,
                        std::slice::from_ref(&diagnostic),
                    ));
                    syntax.diagnostics = diagnostics;
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
                        if !syntax.diagnostics.is_empty() {
                            let mut diagnostics = morphology_warning_diagnostics.clone();
                            diagnostics.extend(syntax_warning_diagnostic_expectation_items(
                                &fixture.test_case.lojban,
                                &parsed.warnings,
                            ));
                            syntax.diagnostics = diagnostics;
                        }
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
                    let mut diagnostics = morphology_warning_diagnostics.clone();
                    diagnostics.extend(diagnostic_expectation_items(
                        &fixture.test_case.lojban,
                        std::slice::from_ref(&diagnostic),
                    ));
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
) -> &mut fixtures::VlaseiOutputExpectation {
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
) -> &mut fixtures::GentufaOutputExpectation {
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
    paths.retain(|path| path_matches_prefix_selector(&args.root, path, &profile.selector));
    warn_if_large_debug_fixture_test(&args, &profile, paths.len());
    if !args.chunk_worker && should_spawn_fixture_test_chunks(&profile) {
        return fixture_test_subprocess_chunks(&args, &profile, &paths, jobs);
    }
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
    print_fixture_test_summary(&summary);
    if summary.failed > 0 && !args.chunk_worker {
        bail!("fixture-test failed {} facet(s)", summary.failed);
    }
    Ok(())
}

#[requires(profile.is_valid())]
#[ensures(true)]
fn warn_if_large_debug_fixture_test(
    args: &FixtureRunArgs,
    profile: &FixtureProfile,
    selected_path_count: usize,
) {
    if !cfg!(debug_assertions) || args.chunk_worker {
        return;
    }
    let test_count = selected_path_count.saturating_mul(profile.facets.len());
    if test_count > DEBUG_LARGE_FIXTURE_TEST_WARNING_THRESHOLD {
        eprintln!(
            "warning: running {test_count} fixture facet test(s) in a debug xtask build is very slow; use `cargo run -r -p xtask-full -- fixture-test ...` for large runs, and reserve debug fixture tests for targeted debugging."
        );
    }
}

#[requires(true)]
#[ensures(true)]
fn print_fixture_test_summary(summary: &RunSummary) {
    println!(
        "fixtures={}, facets={}, passed={}, xfailed={}, failed={}, skipped={}",
        summary.selected_fixtures,
        summary.selected_facets,
        summary.passed,
        summary.xfailed,
        summary.failed,
        summary.skipped
    );
}

#[requires(profile.is_valid())]
#[ensures(true)]
fn should_spawn_fixture_test_chunks(profile: &FixtureProfile) -> bool {
    profile.facets.iter().any(|facet| {
        matches!(
            facet,
            Facet::Syntax
                | Facet::SemanticsRefs
                | Facet::VlaseiTree
                | Facet::GentufaTree
                | Facet::GentufaTreeShowElided
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
    let mut remaining_failure_samples = args.failure_samples;
    for chunk in selected_paths.chunks(FIXTURE_TEST_SUBPROCESS_CHUNK_SIZE) {
        let output =
            fixture_test_chunk_output(&exe, args, profile, chunk, jobs, remaining_failure_samples)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() {
            bail!(
                "fixture-test worker failed with status {}; stdout: {}",
                output.status,
                stdout.trim()
            );
        }
        let chunk_summary = parse_fixture_test_summary(&stdout)?;
        if let Some(remaining) = &mut remaining_failure_samples {
            *remaining = remaining.saturating_sub(chunk_summary.failed);
        }
        summary.merge(chunk_summary);
    }
    summary.selected_facets = profile.facets.len();
    print_fixture_test_summary(&summary);
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
    failure_samples: Option<usize>,
) -> Result<std::process::Output> {
    let mut command = ProcessCommand::new(exe);
    command
        .arg("fixture-test")
        .arg("--root")
        .arg(&args.root)
        .arg("--jobs")
        .arg(jobs.to_string())
        .arg("--chunk-worker");
    if let Some(failure_samples) = failure_samples {
        command
            .arg("--failure-samples")
            .arg(failure_samples.to_string());
    }
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
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .context("running fixture-test worker")
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
    failure_samples: Option<usize>,
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
                        if should_print_fixture_failure(failure_samples, sample_index) {
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

#[requires(true)]
#[ensures(true)]
fn should_print_fixture_failure(failure_samples: Option<usize>, sample_index: usize) -> bool {
    failure_samples.is_none_or(|limit| sample_index < limit)
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
const DEBUG_LARGE_FIXTURE_TEST_WARNING_THRESHOLD: usize = 100;
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
            Facet::Jvozba => run_jvozba_fixture(fixture),
            Facet::Syntax => run_syntax_fixture(fixture),
            Facet::SemanticsRefs => run_semantics_refs_fixture(fixture),
            Facet::VlaseiBrackets => {
                run_vlasei_brackets_fixture(fixture, LojbanScript::Latin, "vlasei brackets")
            }
            Facet::VlaseiBracketsCyrillic => run_vlasei_brackets_fixture(
                fixture,
                LojbanScript::Cyrillic,
                "vlasei brackets cyrillic",
            ),
            Facet::VlaseiBracketsZbalermorna => run_vlasei_brackets_fixture(
                fixture,
                LojbanScript::Zbalermorna,
                "vlasei brackets zbalermorna",
            ),
            Facet::VlaseiTree => run_vlasei_tree_fixture(fixture),
            Facet::VlaseiJson => run_vlasei_json_fixture(fixture),
            Facet::GentufaBrackets => run_gentufa_brackets_fixture(fixture),
            Facet::GentufaTree => run_gentufa_tree_fixture(fixture),
            Facet::GentufaJson => run_gentufa_json_fixture(fixture),
            Facet::GentufaBracketsShowElided => run_gentufa_brackets_show_elided_fixture(fixture),
            Facet::GentufaTreeShowElided => run_gentufa_tree_show_elided_fixture(fixture),
            Facet::GentufaJsonShowElided => run_gentufa_json_show_elided_fixture(fixture),
        }
    }
}

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_vlasei_brackets_fixture(
    fixture: &LoadedTestCase,
    script: LojbanScript,
    label: &str,
) -> FacetResult {
    let Some(expectation) = fixture
        .test_case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.vlasei.as_ref())
        .and_then(|output| output.brackets.as_ref())
        .and_then(|brackets| brackets.expectation_for_script(script))
    else {
        return FacetResult::skipped(format!("fixture has no {label} expectation"));
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
            script,
            ..BracketRenderOptions::default()
        },
    ) {
        Ok(actual) if actual == expectation.text => {
            run_vlasei_brackets_round_trip(fixture, &options, &words, &actual)
        }
        Ok(actual) => FacetResult::failed(format_text_mismatch(label, &expectation.text, &actual)),
        Err(error) => FacetResult::failed(format!("{label} render error: {error}")),
    }
}

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_vlasei_brackets_round_trip(
    fixture: &LoadedTestCase,
    options: &MorphologyOptions,
    expected_words: &[jbotci_morphology::WordLike],
    rendered: &str,
) -> FacetResult {
    let actual_words = match segment_words_with_modifiers_with_options_and_source_id(
        rendered,
        options,
        Some(SourceId("<fixture-round-trip>".to_owned())),
    ) {
        Ok(words) => words,
        Err(error) => {
            return FacetResult::failed(format!(
                "vlasei brackets round-trip morphology error: {error}"
            ));
        }
    };
    if actual_words.len() != expected_words.len() {
        return FacetResult::failed(format!(
            "vlasei brackets round-trip word count mismatch for {}: expected {}, got {}",
            fixture.test_case.id,
            expected_words.len(),
            actual_words.len()
        ));
    }
    for (index, (expected, actual)) in expected_words.iter().zip(actual_words.iter()).enumerate() {
        if !word_like_syntax_eq(expected, actual) {
            return FacetResult::failed(format!(
                "vlasei brackets round-trip word mismatch for {} at index {index}: expected {expected:?}, got {actual:?}",
                fixture.test_case.id
            ));
        }
    }
    FacetResult::passed()
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
    match pretty_brackets_with_options(
        &parsed.parse_tree,
        &fixture.test_case.lojban,
        BracketRenderOptions {
            color: false,
            ..BracketRenderOptions::default()
        },
    ) {
        Ok(actual) if brackets_expectation_matches(fixture, &expectation.text, &actual) => {
            run_gentufa_brackets_round_trip(fixture, &options, &syntax_options, &parsed, &actual)
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
fn run_gentufa_brackets_round_trip(
    fixture: &LoadedTestCase,
    morphology_options: &MorphologyOptions,
    syntax_options: &ParseOptions,
    expected: &jbotci_syntax::SyntaxParse,
    rendered: &str,
) -> FacetResult {
    let words = match segment_words_with_modifiers_with_options_and_source_id(
        rendered,
        morphology_options,
        Some(SourceId("<fixture-round-trip>".to_owned())),
    ) {
        Ok(words) => words,
        Err(error) => {
            return FacetResult::failed(format!(
                "gentufa brackets round-trip morphology error: {error}"
            ));
        }
    };
    let actual =
        match parse_syntax_tree_with_source_and_options(words.as_ref(), rendered, syntax_options) {
            Ok(parsed) => parsed,
            Err(error) => {
                return FacetResult::failed(format!(
                    "gentufa brackets round-trip syntax error: {error}"
                ));
            }
        };
    if syntax_tree_eq_ignoring_spans(&expected.parse_tree, &actual.parse_tree) {
        FacetResult::passed()
    } else {
        FacetResult::failed(format!(
            "gentufa brackets round-trip syntax tree mismatch for {}",
            fixture.test_case.id
        ))
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
fn run_gentufa_brackets_show_elided_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = fixture
        .test_case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.gentufa.as_ref())
        .and_then(|output| output.show_elided.as_ref())
        .and_then(|output| output.brackets.as_ref())
    else {
        return FacetResult::skipped("fixture has no gentufa brackets show-elided expectation");
    };
    let parsed = match parse_gentufa_fixture_tree(fixture) {
        Ok(parsed) => parsed,
        Err(error) => return FacetResult::failed(error),
    };
    match pretty_brackets_with_options(
        &parsed.parse_tree,
        &fixture.test_case.lojban,
        BracketRenderOptions {
            color: false,
            show_elided: true,
            ..BracketRenderOptions::default()
        },
    ) {
        Ok(actual) if actual == expectation.text => FacetResult::passed(),
        Ok(actual) => FacetResult::failed(format_text_mismatch(
            "gentufa brackets show-elided",
            &expectation.text,
            &actual,
        )),
        Err(error) => FacetResult::failed(format!(
            "gentufa brackets show-elided render error: {error}"
        )),
    }
}

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_gentufa_tree_show_elided_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = fixture
        .test_case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.gentufa.as_ref())
        .and_then(|output| output.show_elided.as_ref())
        .and_then(|output| output.tree.as_ref())
    else {
        return FacetResult::skipped("fixture has no gentufa tree show-elided expectation");
    };
    let parsed = match parse_gentufa_fixture_tree(fixture) {
        Ok(parsed) => parsed,
        Err(error) => return FacetResult::failed(error),
    };
    match pretty_tree_with_options(
        &parsed.parse_tree,
        &fixture.test_case.lojban,
        TreeRenderOptions {
            color: false,
            indent: 2,
            show_spans: true,
            show_elided: true,
            ..TreeRenderOptions::default()
        },
    ) {
        Ok(actual) if actual == expectation.text => FacetResult::passed(),
        Ok(actual) => FacetResult::failed(format_text_mismatch(
            "gentufa tree show-elided",
            &expectation.text,
            &actual,
        )),
        Err(error) => {
            FacetResult::failed(format!("gentufa tree show-elided render error: {error}"))
        }
    }
}

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_gentufa_json_show_elided_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = fixture
        .test_case
        .expectations
        .output
        .as_ref()
        .and_then(|output| output.gentufa.as_ref())
        .and_then(|output| output.show_elided.as_ref())
        .and_then(|output| output.json.as_ref())
    else {
        return FacetResult::skipped("fixture has no gentufa JSON show-elided expectation");
    };
    let parsed = match parse_gentufa_fixture_tree(fixture) {
        Ok(parsed) => parsed,
        Err(error) => return FacetResult::failed(error),
    };
    match compact_syntax_json_string_with_options(
        &parsed.parse_tree,
        JsonRenderOptions {
            indent: 0,
            show_elided: true,
            ..JsonRenderOptions::default()
        },
    ) {
        Ok(actual) if actual == expectation.text => FacetResult::passed(),
        Ok(actual) => FacetResult::failed(format_text_mismatch(
            "gentufa JSON show-elided",
            &expectation.text,
            &actual,
        )),
        Err(error) => {
            FacetResult::failed(format!("gentufa JSON show-elided render error: {error}"))
        }
    }
}

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.is_empty()))]
fn parse_gentufa_fixture_tree(
    fixture: &LoadedTestCase,
) -> std::result::Result<jbotci_syntax::SyntaxParse, String> {
    let dialect = fixture
        .test_case
        .dialect_definition()
        .map_err(|error| format!("dialect error: {error}"))?;
    let options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
    let words = segment_words_with_modifiers_with_options_and_source_id(
        &fixture.test_case.lojban,
        &options,
        Some(SourceId("<fixture>".to_owned())),
    )
    .map_err(|error| format!("morphology error: {error}"))?;
    parse_syntax_tree_with_source_and_options(&words, &fixture.test_case.lojban, &syntax_options)
        .map_err(|error| format!("syntax error: {error}"))
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

#[requires(fixture.test_case.is_valid_fixture_metadata())]
#[ensures(ret.is_valid())]
fn run_jvozba_fixture(fixture: &LoadedTestCase) -> FacetResult {
    let Some(expectation) = &fixture.test_case.expectations.jvozba else {
        return FacetResult::skipped("fixture has no jvozba expectation");
    };
    let inputs = expectation
        .inputs
        .iter()
        .map(jvozba_input_from_fixture)
        .collect::<Vec<_>>();
    let result = jbotci_jvozba::build_best_jvozba_detailed(
        jvozba_mode_from_fixture(expectation.mode),
        jbotci_dictionary_data::english(),
        &inputs,
    );
    match expectation.status {
        ExpectationStatus::Success => {
            let actual = match result {
                Ok(actual) => actual,
                Err(error) => {
                    return FacetResult::failed(format!(
                        "jvozba should succeed, got error: {error}"
                    ));
                }
            };
            let Some(expected) = expectation.output.as_ref() else {
                return FacetResult::failed("jvozba success expectation has no output");
            };
            if actual.word != expected.word {
                return FacetResult::failed(format_text_mismatch(
                    "jvozba word",
                    &expected.word,
                    &actual.word,
                ));
            }
            if let Some(message) = jvozba_segments_mismatch(&actual.segments, &expected.segments) {
                return FacetResult::failed(message);
            }
            if let Some(message) = jvozba_parse_back_mismatch(expectation.mode, expected) {
                return FacetResult::failed(message);
            }
            FacetResult::passed()
        }
        ExpectationStatus::Failure => match result {
            Ok(actual) => FacetResult::failed(format!(
                "expected jvozba failure, got `{}`",
                truncate_for_mismatch(&actual.word)
            )),
            Err(error) => {
                if let Some(expected) = expectation.error.as_ref() {
                    let actual = error.to_string();
                    if actual == expected.text {
                        FacetResult::passed()
                    } else {
                        FacetResult::failed(format_text_mismatch(
                            "jvozba error",
                            &expected.text,
                            &actual,
                        ))
                    }
                } else {
                    FacetResult::passed()
                }
            }
        },
        ExpectationStatus::Pending | ExpectationStatus::NotApplicable => {
            FacetResult::skipped(format!("jvozba expectation is {:?}", expectation.status))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn jvozba_mode_from_fixture(mode: fixtures::JvozbaFixtureMode) -> jbotci_jvozba::JvozbaMode {
    match mode {
        fixtures::JvozbaFixtureMode::Lujvo => jbotci_jvozba::JvozbaMode::Lujvo,
        fixtures::JvozbaFixtureMode::Cmevla => jbotci_jvozba::JvozbaMode::Cmevla,
    }
}

#[requires(true)]
#[ensures(true)]
fn jvozba_input_from_fixture(input: &fixtures::JvozbaFixtureInput) -> jbotci_jvozba::JvozbaInput {
    match input {
        fixtures::JvozbaFixtureInput::Word { text } => {
            jbotci_jvozba::JvozbaInput::Word(text.clone())
        }
        fixtures::JvozbaFixtureInput::FixedRafsi { text } => {
            jbotci_jvozba::JvozbaInput::FixedRafsi(text.clone())
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn jvozba_segments_mismatch(
    actual: &[jbotci_jvozba::JvozbaSegment],
    expected: &[fixtures::JvozbaSegmentExpectation],
) -> Option<String> {
    if actual.len() != expected.len() {
        return Some(format!(
            "jvozba segment count mismatch: expected {}, got {}",
            expected.len(),
            actual.len()
        ));
    }
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        let actual_kind = jvozba_segment_kind_to_fixture(actual.kind);
        if actual_kind != expected.kind {
            return Some(format!(
                "jvozba segment {index} kind mismatch for `{}`: expected {:?}, got {:?}",
                expected.text, expected.kind, actual_kind
            ));
        }
        if actual.text != expected.text {
            return Some(format!(
                "jvozba segment {index} text mismatch: expected `{}`, got `{}`",
                truncate_for_mismatch(&expected.text),
                truncate_for_mismatch(&actual.text)
            ));
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn jvozba_segment_kind_to_fixture(
    kind: jbotci_jvozba::JvozbaSegmentKind,
) -> fixtures::JvozbaSegmentKindExpectation {
    match kind {
        jbotci_jvozba::JvozbaSegmentKind::Rafsi => fixtures::JvozbaSegmentKindExpectation::Rafsi,
        jbotci_jvozba::JvozbaSegmentKind::Hyphen => fixtures::JvozbaSegmentKindExpectation::Hyphen,
    }
}

#[requires(true)]
#[ensures(true)]
fn jvozba_parse_back_mismatch(
    mode: fixtures::JvozbaFixtureMode,
    expected: &fixtures::JvozbaOutputExpectation,
) -> Option<String> {
    let words = match jbotci_morphology::segment_words_with_modifiers(&expected.word) {
        Ok(words) => words,
        Err(error) => return Some(format!("jvozba output did not parse back: {error}")),
    };
    let [word_like] = words.as_slice() else {
        return Some(format!(
            "jvozba output parsed back as {} word(s), expected one",
            words.len()
        ));
    };
    let Some(word) = word_like.bare_word() else {
        return Some("jvozba output parsed back as a non-bare word".to_owned());
    };
    match mode {
        fixtures::JvozbaFixtureMode::Lujvo => {
            if word.kind() != jbotci_morphology::WordKind::Lujvo {
                return Some(format!(
                    "jvozba output parsed back as {}, expected lujvo",
                    word.kind()
                ));
            }
            let Some(parts) = word.lujvo_parts() else {
                return Some("jvozba output parsed back without lujvo parts".to_owned());
            };
            if parts.len() != expected.segments.len() {
                return Some(format!(
                    "jvozba parse-back part count mismatch: expected {}, got {}",
                    expected.segments.len(),
                    parts.len()
                ));
            }
            for (index, (part, segment)) in parts.iter().zip(&expected.segments).enumerate() {
                if !jbotci_morphology::canonical_text_eq(part.phonemes().as_str(), &segment.text) {
                    return Some(format!(
                        "jvozba parse-back part {index} mismatch: expected `{}`, got `{}`",
                        truncate_for_mismatch(&segment.text),
                        truncate_for_mismatch(part.phonemes().as_str())
                    ));
                }
            }
            None
        }
        fixtures::JvozbaFixtureMode::Cmevla => {
            if word.kind() == jbotci_morphology::WordKind::Cmevla {
                None
            } else {
                Some(format!(
                    "jvozba output parsed back as {}, expected cmevla",
                    word.kind()
                ))
            }
        }
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
        Facet::Jvozba => expectations.jvozba.as_ref().map(|value| value.status),
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
            .and_then(|brackets| brackets.expectation_for_script(LojbanScript::Latin))
            .map(|_| ExpectationStatus::Success),
        Facet::VlaseiBracketsCyrillic => expectations
            .output
            .as_ref()
            .and_then(|output| output.vlasei.as_ref())
            .and_then(|output| output.brackets.as_ref())
            .and_then(|brackets| brackets.expectation_for_script(LojbanScript::Cyrillic))
            .map(|_| ExpectationStatus::Success),
        Facet::VlaseiBracketsZbalermorna => expectations
            .output
            .as_ref()
            .and_then(|output| output.vlasei.as_ref())
            .and_then(|output| output.brackets.as_ref())
            .and_then(|brackets| brackets.expectation_for_script(LojbanScript::Zbalermorna))
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
        Facet::GentufaBracketsShowElided => expectations
            .output
            .as_ref()
            .and_then(|output| output.gentufa.as_ref())
            .and_then(|output| output.show_elided.as_ref())
            .and_then(|output| output.brackets.as_ref())
            .map(|_| ExpectationStatus::Success),
        Facet::GentufaTreeShowElided => expectations
            .output
            .as_ref()
            .and_then(|output| output.gentufa.as_ref())
            .and_then(|output| output.show_elided.as_ref())
            .and_then(|output| output.tree.as_ref())
            .map(|_| ExpectationStatus::Success),
        Facet::GentufaJsonShowElided => expectations
            .output
            .as_ref()
            .and_then(|output| output.gentufa.as_ref())
            .and_then(|output| output.show_elided.as_ref())
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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dioxus_asset_root_is_only_set_for_non_root_base_paths() {
        assert_eq!(dioxus_asset_root("/"), None);
        assert_eq!(dioxus_asset_root(""), None);
        assert_eq!(dioxus_asset_root(" / "), None);
        assert_eq!(dioxus_asset_root("/jbotci"), Some("/jbotci".to_owned()));
        assert_eq!(dioxus_asset_root("jbotci/"), Some("/jbotci".to_owned()));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dioxus_runtime_asset_root_keeps_root_explicit_for_render() {
        assert_eq!(dioxus_runtime_asset_root("/"), "/");
        assert_eq!(dioxus_runtime_asset_root(""), "/");
        assert_eq!(dioxus_runtime_asset_root("/jbotci"), "/jbotci");
        assert_eq!(dioxus_runtime_asset_root("jbotci/"), "/jbotci");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn wiki_rest_title_encoding_uses_single_path_segment() {
        assert_eq!(
            wiki_rest_title_segment("The Complete Lojban Language"),
            "The_Complete_Lojban_Language"
        );
        assert_eq!(
            wiki_rest_title_segment("\"Sixteen Rules\" issue"),
            "%22Sixteen_Rules%22_issue"
        );
        assert_eq!(
            wiki_rest_title_segment("¡Bienvenido!/Español"),
            "%C2%A1Bienvenido%21%2FEspa%C3%B1ol"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn wiki_page_metadata_parser_accepts_empty_generator_response() {
        let pages = parse_wiki_api_pages(&serde_json::json!({
            "batchcomplete": true
        }))
        .unwrap();

        assert!(pages.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn wiki_recentchanges_window_is_safe_only_when_snapshot_is_retained() {
        assert!(recentchanges_covers_snapshot(
            "2026-06-07T12:00:00Z",
            "2025-11-17T15:08:48Z"
        ));
        assert!(recentchanges_covers_snapshot(
            "2026-06-07T12:00:00Z",
            "2026-06-07T12:00:00Z"
        ));
        assert!(!recentchanges_covers_snapshot(
            "2025-01-01T00:00:00Z",
            "2025-11-17T15:08:48Z"
        ));
        assert!(!recentchanges_covers_snapshot("", "2025-11-17T15:08:48Z"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn wiki_full_reconcile_plan_keeps_fetches_and_removes_by_metadata() {
        let root = std::env::temp_dir().join(format!(
            "jbotci-xtask-wiki-plan-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let kept = test_wiki_page_entry(1, 10, "Kept");
        let changed = test_wiki_page_entry(2, 20, "Changed");
        let removed = test_wiki_page_entry(3, 30, "Removed");
        write_test_wiki_entry_files(&root, &kept);
        write_test_wiki_entry_files(&root, &changed);
        write_test_wiki_entry_files(&root, &removed);
        let existing = WikiExistingSnapshot {
            fetched_at: "2026-06-07T12:00:00Z".to_owned(),
            pages: vec![kept.clone(), changed.clone(), removed.clone()],
        };
        let remote = vec![
            test_wiki_remote_metadata(1, 10, "Kept"),
            test_wiki_remote_metadata(2, 21, "Changed"),
            test_wiki_remote_metadata(4, 40, "New"),
        ];

        let plan = build_full_wiki_snapshot_plan(Some(&existing), &root, &remote, false);

        assert_eq!(plan.source, WikiSnapshotPlanSource::FullReconcile);
        assert_eq!(
            plan.keep
                .iter()
                .map(|entry| entry.pageid)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            plan.fetch
                .iter()
                .map(|metadata| metadata.pageid)
                .collect::<Vec<_>>(),
            vec![2, 4]
        );
        assert_eq!(
            plan.removed
                .iter()
                .map(|entry| entry.pageid)
                .collect::<Vec<_>>(),
            vec![3]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn wiki_parsoid_pair_validation_requires_matching_revision_and_metadata() {
        let source = test_wiki_source_revision(122741);
        let valid = WikiParsoidHtml {
            revid: 122741,
            html: r#"<html about="x" mw:htmlVersion="2.4.0"></html>"#.to_owned(),
        };
        let wrong_revision = WikiParsoidHtml {
            revid: 122742,
            html: valid.html.clone(),
        };
        let missing_metadata = WikiParsoidHtml {
            revid: 122741,
            html: "<html></html>".to_owned(),
        };

        assert!(wiki_parsoid_source_pair_is_valid(&valid, &source));
        assert!(!wiki_parsoid_source_pair_is_valid(&wrong_revision, &source));
        assert!(!wiki_parsoid_source_pair_is_valid(
            &missing_metadata,
            &source
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn wiki_progress_detail_reports_counts_retries_and_current_title() {
        let detail = wiki_page_progress_detail(12, 3, 1, 1, 2, "The Complete Lojban Language");
        assert!(detail.contains("kept 12"));
        assert!(detail.contains("fetched 3"));
        assert!(detail.contains("failed 1"));
        assert!(detail.contains("removed 1"));
        assert!(detail.contains("retries 2"));
        assert!(detail.contains("The Complete Lojban Language"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn wiki_page_fetch_error_records_metadata_and_message() {
        let metadata = test_wiki_remote_metadata(104, 74046, "cipra/jbo");
        let error = anyhow::anyhow!("missing content for page ID 104");

        let record = wiki_page_fetch_error(&metadata, &error);

        assert_eq!(record.pageid, 104);
        assert_eq!(record.ns, 0);
        assert_eq!(record.title, "cipra/jbo");
        assert_eq!(record.lastrevid, Some(74046));
        assert!(record.error.contains("missing content"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn wiki_rest_retry_policy_does_not_retry_page_render_500() {
        assert!(is_transient_status(500));
        assert!(!is_transient_rest_status(500));
        assert!(is_transient_rest_status(503));
        assert!(is_transient_rest_status(429));
    }

    #[requires(pageid > 0)]
    #[requires(revid > 0)]
    #[requires(!title.is_empty())]
    #[ensures(ret.pageid == pageid)]
    fn test_wiki_page_entry(pageid: u64, revid: u64, title: &str) -> WikiPageIndexEntry {
        let paths = wiki_page_paths(pageid);
        WikiPageIndexEntry {
            pageid,
            ns: 0,
            title: title.to_owned(),
            redirect: false,
            revid: Some(revid),
            timestamp: Some("2026-06-07T12:00:00Z".to_owned()),
            model: "wikitext".to_owned(),
            bytes: 42,
            source_sha256: "a".repeat(64),
            parsoid_html_sha256: "b".repeat(64),
            meta: relative_path_string(&paths.meta),
            source: relative_path_string(&paths.source),
            parsoid_html: relative_path_string(&paths.parsoid_html),
        }
    }

    #[requires(pageid > 0)]
    #[requires(revid > 0)]
    #[requires(!title.is_empty())]
    #[ensures(ret.pageid == pageid)]
    fn test_wiki_remote_metadata(pageid: u64, revid: u64, title: &str) -> WikiPageRemoteMetadata {
        WikiPageRemoteMetadata {
            pageid,
            ns: 0,
            title: title.to_owned(),
            touched: Some("2026-06-07T12:00:00Z".to_owned()),
            lastrevid: Some(revid),
            length: Some(42),
            redirect: false,
            protection: serde_json::Value::Array(Vec::new()),
            revision: Some(test_wiki_revision(revid)),
        }
    }

    #[requires(revid > 0)]
    #[ensures(ret.revid == revid)]
    fn test_wiki_revision(revid: u64) -> WikiRevisionMetadata {
        WikiRevisionMetadata {
            revid,
            parentid: Some(revid - 1),
            timestamp: "2026-06-07T12:00:00Z".to_owned(),
            user: Some("Test".to_owned()),
            userid: Some(1),
            comment: "test".to_owned(),
            size: Some(42),
            sha1: Some("sha1".to_owned()),
            contentmodel: "wikitext".to_owned(),
            contentformat: Some("text/x-wiki".to_owned()),
        }
    }

    #[requires(revid > 0)]
    #[ensures(ret.revision.revid == revid)]
    fn test_wiki_source_revision(revid: u64) -> WikiFetchedSourceRevision {
        WikiFetchedSourceRevision {
            pageid: 1,
            ns: 0,
            title: "Test".to_owned(),
            touched: Some("2026-06-07T12:00:00Z".to_owned()),
            lastrevid: Some(revid),
            length: Some(42),
            redirect: false,
            protection: serde_json::Value::Array(Vec::new()),
            revision: test_wiki_revision(revid),
            source: "test source".to_owned(),
        }
    }

    #[requires(root.components().next().is_some())]
    #[requires(entry.pageid > 0)]
    #[ensures(true)]
    fn write_test_wiki_entry_files(root: &Path, entry: &WikiPageIndexEntry) {
        for relative in [&entry.meta, &entry.source, &entry.parsoid_html] {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "test").unwrap();
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn explicit_container_engine_args_resolve_without_path_probe() {
        assert_eq!(
            ContainerEngineArg::Docker.resolve().expect("docker engine"),
            ContainerEngine::Docker
        );
        assert_eq!(
            ContainerEngineArg::Podman.resolve().expect("podman engine"),
            ContainerEngine::Podman
        );
        assert_eq!(ContainerEngine::Docker.command_name(), "docker");
        assert_eq!(ContainerEngine::Podman.command_name(), "podman");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn r2_prefix_is_normalized_without_allowing_escape_components() {
        assert_eq!(
            normalize_r2_prefix("/embeddings/web/v1/").unwrap(),
            "embeddings/web/v1"
        );
        assert!(normalize_r2_prefix("/").is_err());
        assert!(normalize_r2_prefix("embeddings/../v1").is_err());
        assert!(normalize_r2_prefix("embeddings//v1").is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn r2_upload_plan_writes_only_prefixed_objects_and_catalog_last() {
        let root = std::env::temp_dir().join(format!(
            "jbotci-xtask-r2-plan-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let pack =
            root.join("models/model/spaces/transformers-js-q4/packs/pack/corpora/dictionary");
        fs::create_dir_all(&pack).unwrap();
        fs::write(
            root.join("catalog.json"),
            r#"{
  "schema_version": 1,
  "models": [
    {
      "model_key": "model",
      "vector_spaces": [
        {
          "vector_space_key": "transformers-js-q4",
          "latest_pack_id": "pack",
          "manifest_url": "models/model/spaces/transformers-js-q4/packs/pack/manifest.json",
          "compatible_query_runtimes": []
        }
      ]
    }
  ]
}
"#,
        )
        .unwrap();
        fs::write(root.join("catalog.json.br"), "compressed catalog").unwrap();
        fs::write(
            root.join("models/model/spaces/transformers-js-q4/packs/pack/manifest.json"),
            r#"{
  "schema_version": 1,
  "corpora": [
    {
      "corpus_id": "dictionary",
      "items_url": "corpora/dictionary/items.json",
      "vector_url": "corpora/dictionary/vectors.f32"
    }
  ]
}
"#,
        )
        .unwrap();
        fs::write(
            root.join("models/model/spaces/transformers-js-q4/packs/pack/manifest.json.br"),
            "compressed manifest",
        )
        .unwrap();
        fs::write(pack.join("items.json"), "[]").unwrap();
        fs::write(pack.join("items.json.br"), "compressed items").unwrap();
        fs::write(pack.join("vectors.f32"), [0_u8, 1, 2, 3]).unwrap();
        fs::write(pack.join("vectors.f32.br"), "compressed vectors").unwrap();
        fs::write(root.join("stale.json"), "{}").unwrap();
        fs::write(root.join("stale.json.br"), "compressed stale").unwrap();

        let objects = r2_upload_objects(&root, "/embeddings/web/v1/").unwrap();
        let keys = objects
            .iter()
            .map(|object| object.object_key.as_str())
            .collect::<Vec<_>>();

        assert!(keys.iter().all(|key| key.starts_with("embeddings/web/v1/")));
        assert!(!keys.iter().any(|key| key.ends_with(".br")));
        assert!(!keys.iter().any(|key| key.ends_with("stale.json")));
        assert_eq!(keys.last().copied(), Some("embeddings/web/v1/catalog.json"));
        assert!(keys.iter().any(|key| key.ends_with("/manifest.json")));
        assert!(keys.iter().any(|key| key.ends_with("/items.json")));
        assert!(keys.iter().any(|key| key.ends_with("/vectors.f32")));

        let catalog = objects.last().unwrap();
        assert_eq!(catalog.content_type, "application/json; charset=utf-8");
        assert_eq!(catalog.content_encoding, Some("br"));
        assert_eq!(catalog.cache_control, R2_CATALOG_CACHE_CONTROL);
        assert!(catalog.local_path.ends_with("catalog.json.br"));

        let vectors = objects
            .iter()
            .find(|object| object.object_key.ends_with("/vectors.f32"))
            .unwrap();
        assert_eq!(vectors.content_type, "application/octet-stream");
        assert_eq!(vectors.content_encoding, Some("br"));
        assert_eq!(vectors.cache_control, R2_IMMUTABLE_CACHE_CONTROL);
        assert!(vectors.local_path.ends_with("vectors.f32.br"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn native_r2_upload_plan_follows_manifest_items_and_shards() {
        let root = std::env::temp_dir().join(format!(
            "jbotci-xtask-native-r2-plan-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let pack = root.join("models/model/packs/pack/corpora/vlacku-en");
        fs::create_dir_all(&pack).unwrap();
        fs::write(
            root.join("catalog.json"),
            r#"{
  "schema_version": 1,
  "models": [
    {
      "model_key": "model",
      "latest_pack_id": "pack",
      "manifest_url": "models/model/packs/pack/manifest.json"
    }
  ]
}
"#,
        )
        .unwrap();
        fs::write(
            root.join("models/model/packs/pack/manifest.json"),
            r#"{
  "schema_version": 1,
  "corpora": [
    {
      "corpus_id": "vlacku-en",
      "items_url": "corpora/vlacku-en/items.json",
      "shards": [
        {
          "url": "corpora/vlacku-en/vectors-0000.f32",
          "byte_len": 4,
          "sha256": "unused"
        }
      ]
    }
  ]
}
"#,
        )
        .unwrap();
        fs::write(pack.join("items.json"), "[]").unwrap();
        fs::write(pack.join("items.json.br"), "compressed items").unwrap();
        fs::write(pack.join("vectors-0000.f32"), [0_u8, 1, 2, 3]).unwrap();
        fs::write(pack.join("vectors-0000.f32.br"), "compressed vectors").unwrap();
        fs::write(root.join("stale.json"), "{}").unwrap();

        let objects = r2_upload_native_objects_without_catalog(&root, GGUF_EMBEDDINGS_R2_PREFIX)
            .expect("native R2 plan");
        let keys = objects
            .iter()
            .map(|object| object.object_key.as_str())
            .collect::<Vec<_>>();

        assert!(
            keys.iter()
                .all(|key| key.starts_with("embeddings/gguf/v1/"))
        );
        assert!(!keys.iter().any(|key| key.ends_with("catalog.json")));
        assert!(!keys.iter().any(|key| key.ends_with("stale.json")));
        assert!(keys.iter().any(|key| key.ends_with("/manifest.json")));
        assert!(keys.iter().any(|key| key.ends_with("/items.json")));
        assert!(keys.iter().any(|key| key.ends_with("/vectors-0000.f32")));

        let vectors = objects
            .iter()
            .find(|object| object.object_key.ends_with("/vectors-0000.f32"))
            .unwrap();
        assert_eq!(vectors.content_type, "application/octet-stream");
        assert_eq!(vectors.content_encoding, Some("br"));
        assert_eq!(vectors.cache_control, R2_IMMUTABLE_CACHE_CONTROL);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn r2_upload_policy_uses_short_catalog_cache_and_immutable_pack_cache() {
        assert_eq!(
            r2_content_type("catalog.json"),
            "application/json; charset=utf-8"
        );
        assert_eq!(
            r2_content_type("models/model/spaces/q4/packs/pack/corpora/dictionary/vectors.f32"),
            "application/octet-stream"
        );
        assert_eq!(
            r2_content_type("models/model/spaces/q4/packs/pack/corpora/dictionary/vectors.f16"),
            "application/octet-stream"
        );
        assert_eq!(r2_cache_control("catalog.json"), R2_CATALOG_CACHE_CONTROL);
        assert_eq!(
            r2_cache_control("models/model/spaces/q4/packs/pack/manifest.json"),
            R2_CATALOG_CACHE_CONTROL
        );
        assert_eq!(
            r2_cache_control("models/model/spaces/q4/packs/pack/corpora/dictionary/vectors.f16"),
            R2_IMMUTABLE_CACHE_CONTROL
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedding_catalog_merge_preserves_gemma_and_replaces_f2llm() {
        let remote = serde_json::json!({
            "schema_version": 1,
            "models": [
                {
                    "model_key": "embedding-gemma-300m-q4-768",
                    "vector_spaces": [{"vector_space_key": "gemma-q4"}]
                },
                {
                    "model_key": F2LLM_80M_MODEL_KEY,
                    "vector_spaces": [{"vector_space_key": "old"}]
                }
            ]
        });
        let replacement = serde_json::json!({
            "schema_version": 1,
            "models": [
                {
                    "model_key": F2LLM_80M_MODEL_KEY,
                    "vector_spaces": [{"vector_space_key": "new"}]
                }
            ]
        });

        let merged = merge_embedding_catalog(remote, replacement, F2LLM_80M_MODEL_KEY).unwrap();
        let models = merged["models"].as_array().unwrap();

        assert_eq!(models.len(), 2);
        assert_eq!(models[0]["model_key"], "embedding-gemma-300m-q4-768");
        assert_eq!(models[1]["model_key"], F2LLM_80M_MODEL_KEY);
        assert_eq!(models[1]["vector_spaces"][0]["vector_space_key"], "new");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedding_catalog_merge_replaces_all_f2llm_models() {
        let remote = serde_json::json!({
            "schema_version": 1,
            "models": [
                {"model_key": "embedding-gemma-300m-q4-768"},
                {"model_key": F2LLM_80M_MODEL_KEY, "vector_spaces": [{"vector_space_key": "old-80m"}]},
                {"model_key": F2LLM_330M_MODEL_KEY, "vector_spaces": [{"vector_space_key": "old-330m"}]}
            ]
        });
        let replacement = serde_json::json!({
            "schema_version": 1,
            "models": [
                {"model_key": F2LLM_80M_MODEL_KEY, "vector_spaces": [{"vector_space_key": "new-80m"}]},
                {"model_key": F2LLM_160M_MODEL_KEY, "vector_spaces": [{"vector_space_key": "new-160m"}]},
                {"model_key": F2LLM_330M_MODEL_KEY, "vector_spaces": [{"vector_space_key": "new-330m"}]},
                {"model_key": F2LLM_0_6B_MODEL_KEY, "vector_spaces": [{"vector_space_key": "new-0.6b"}]}
            ]
        });
        let model_keys = F2LLM_MODEL_SPECS
            .iter()
            .map(|spec| spec.model_key.to_owned())
            .collect::<BTreeSet<_>>();

        let merged = merge_embedding_catalog_models(remote, replacement, &model_keys).unwrap();
        let models = merged["models"].as_array().unwrap();
        let merged_keys = models
            .iter()
            .map(|model| model["model_key"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(merged_keys[0], "embedding-gemma-300m-q4-768");
        assert_eq!(merged_keys[1], F2LLM_80M_MODEL_KEY);
        assert_eq!(merged_keys[2], F2LLM_330M_MODEL_KEY);
        assert!(merged_keys.contains(&F2LLM_160M_MODEL_KEY));
        assert!(merged_keys.contains(&F2LLM_0_6B_MODEL_KEY));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn f2llm_model_upload_plan_uploads_manifest_last() {
        let root = std::env::temp_dir().join(format!(
            "jbotci-xtask-f2llm-model-r2-plan-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("tensors/layer")).unwrap();
        fs::write(root.join("manifest.json"), "{}").unwrap();
        fs::write(root.join("manifest.json.br"), "compressed manifest").unwrap();
        fs::write(root.join("tokenizer.abc.compact.json"), "{}").unwrap();
        fs::write(root.join("tensors/layer/data.abc.bin"), [1_u8, 2, 3]).unwrap();

        let objects = r2_upload_tree_objects(&root, F2LLM_MODEL_SPECS[0].webgpu_r2_prefix).unwrap();
        let keys = objects
            .iter()
            .map(|object| object.object_key.as_str())
            .collect::<Vec<_>>();

        assert!(
            keys.iter()
                .all(|key| key.starts_with(F2LLM_MODEL_SPECS[0].webgpu_r2_prefix))
        );
        assert_eq!(
            keys.last().copied(),
            Some("models/f2llm-v2-80m-webgpu/v1/manifest.json")
        );
        let manifest = objects.last().unwrap();
        assert_eq!(manifest.content_encoding, Some("br"));
        assert_eq!(manifest.cache_control, R2_CATALOG_CACHE_CONTROL);
        let tensor = objects
            .iter()
            .find(|object| object.object_key.ends_with("data.abc.bin"))
            .unwrap();
        assert_eq!(tensor.cache_control, R2_IMMUTABLE_CACHE_CONTROL);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn flat_web_asset_copy_prunes_obsolete_files_without_replacing_target_directory() {
        let root = std::env::temp_dir().join(format!(
            "jbotci-xtask-flat-assets-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(source.join("current.txt"), "current").unwrap();
        fs::write(target.join("current.txt"), "old").unwrap();
        fs::write(target.join("obsolete.txt"), "obsolete").unwrap();
        fs::create_dir(target.join("nested")).unwrap();

        copy_flat_web_asset_dir(&source, &target, "test asset").unwrap();

        assert_eq!(
            fs::read_to_string(target.join("current.txt")).unwrap(),
            "current"
        );
        assert!(!target.join("obsolete.txt").exists());
        assert!(target.join("nested").is_dir());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn release_service_worker_precache_excludes_sidecars_and_embeddings() {
        let root = std::env::temp_dir().join(format!(
            "jbotci-xtask-sw-assets-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let public = root.join("public");
        fs::create_dir_all(public.join("assets/embeddings/web/v1")).unwrap();
        fs::create_dir_all(public.join("assets/icons")).unwrap();
        fs::write(public.join("index.html"), "<!doctype html>").unwrap();
        fs::write(public.join("assets/manifest.webmanifest"), "{}").unwrap();
        fs::write(public.join("service-worker.js"), "old").unwrap();
        fs::write(public.join("assets/app.js"), "app").unwrap();
        fs::write(public.join("assets/app.js.br"), "compressed").unwrap();
        fs::write(public.join("assets/app.js.map"), "sourcemap").unwrap();
        fs::write(public.join("assets/icons/jbotci-icon-512.png"), "icon").unwrap();
        fs::write(
            public.join("assets/embeddings/web/v1/catalog.json"),
            "embedding",
        )
        .unwrap();

        let paths = release_service_worker_precache_paths(&public).unwrap();

        assert_eq!(
            paths,
            vec![
                "assets/app.js".to_owned(),
                "assets/icons/jbotci-icon-512.png".to_owned(),
                "assets/manifest.webmanifest".to_owned(),
                "index.html".to_owned(),
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn release_service_worker_script_uses_network_first_and_jbotci_cache_prefix() {
        let paths = vec![
            "assets/manifest.webmanifest".to_owned(),
            "index.html".to_owned(),
        ];
        let script = render_release_service_worker("abc123", &paths).unwrap();

        assert!(script.contains("const CACHE_VERSION = \"abc123\";"));
        assert!(script.contains("networkFirst(request, RUNTIME_CACHE_NAME, APP_SHELL_URL)"));
        assert!(script.contains("name.startsWith(\"jbotci-\")"));
        assert!(script.contains("\"assets/manifest.webmanifest\""));
    }
}
