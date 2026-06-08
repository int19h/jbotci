//! Native and web embedding model, vector-pack, and semantic search support.

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use std::env;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, requires};
use directories::ProjectDirs;
use jbotci_cll::{
    CllSearchChunk, CllSearchMatch, CuktaSearchMode, CuktaSearchOutput, CuktaTargetFilter,
    clamp_cukta_result_count, cll_search_all_chunks,
};
use jbotci_dictionary::Dictionary;
pub use jbotci_embedding_inputs::{
    CUKTA_CORPUS_ID, DEFAULT_INPUT_FORMAT_VERSION, DEFAULT_MODEL_DIMENSIONS, DEFAULT_MODEL_KEY,
    DEFAULT_MODEL_REVISION, RETRIEVAL_DOCUMENT_PREFIX, RETRIEVAL_QUERY_PREFIX, VLACKU_CORPUS_ID,
    build_retrieval_document_input, build_retrieval_query_input, cll_embedding_input,
    cll_fingerprint, dictionary_embedding_input, dictionary_embedding_kind, dictionary_fingerprint,
    sha256_hex_bytes,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[cfg(feature = "native-llama")]
pub mod native;

pub const EMBEDDING_INDEX_DIR_ENV: &str = "JBOTCI_EMBEDDING_INDEX_DIR";
pub const EMBEDDING_MODEL_DIR_ENV: &str = "JBOTCI_EMBEDDING_MODEL_DIR";
pub const HF_ENDPOINT_ENV: &str = "HF_ENDPOINT";
pub const HF_TOKEN_ENV: &str = "HF_TOKEN";
pub const INDEX_SCHEMA_VERSION: u32 = 1;
pub const INDEX_BASE_VERSION: &str = "v1";
pub const DEFAULT_VECTOR_SHARD_TARGET_BYTES: usize = 8 * 1024 * 1024;
pub const DEFAULT_GGUF_EMBEDDINGS_BASE_URL: &str = "https://assets.jbotci.app/embeddings/gguf/v1";

const NATIVE_PARTIAL_BUILD_SCHEMA_VERSION: u32 = 1;
const NATIVE_PARTIAL_BUILD_SOURCE: &str = "native-partial";
const NATIVE_PARTIAL_BUILD_FILE: &str = "native-local-build.json";
const NATIVE_VECTOR_CHUNK_ROWS: usize = 256;
const DEFAULT_HF_ENDPOINT: &str = "https://huggingface.co";
const DEFAULT_WEB_DTYPE: &str = "q4";
const LLAMA_CPP_4_RUNTIME_VERSION: &str = "0.3.0";

#[derive(Debug, Clone, Copy)]
#[invariant(true)]
struct NativeF2LlmModel {
    model_key: &'static str,
    model_revision: &'static str,
    input_format_version: &'static str,
    native_hf_repo: &'static str,
    native_hf_revision: &'static str,
    native_hf_file: &'static str,
    native_size_bytes: u64,
    native_sha256: &'static str,
    web_model: &'static str,
    dimensions: usize,
}

const NATIVE_F2LLM_MODELS: &[NativeF2LlmModel] = &[
    NativeF2LlmModel {
        model_key: "f2llm-v2-80m-q4-k-m-320",
        model_revision: "f4a16a11c9f5c8c7e22694653de6ce75430f4538",
        input_format_version: "f2llm-v2-80m-q4-k-m-v0",
        native_hf_repo: "mradermacher/F2LLM-v2-80M-GGUF",
        native_hf_revision: "f39be191bbbd6d6f13894d53d63c8291d7b31182",
        native_hf_file: "F2LLM-v2-80M.Q4_K_M.gguf",
        native_size_bytes: 79_111_968,
        native_sha256: "46e6279206856868adf680ce25f23f6d4846610d0f54f6c527647ab48478a813",
        web_model: "codefuse-ai/F2LLM-v2-80M",
        dimensions: 320,
    },
    NativeF2LlmModel {
        model_key: "f2llm-v2-160m-q4-k-m-640",
        model_revision: "60229594f9498ae44c553f1f8cebf32559bc8577",
        input_format_version: "f2llm-v2-160m-q4-k-m-v0",
        native_hf_repo: "mradermacher/F2LLM-v2-160M-GGUF",
        native_hf_revision: "a1f45469b2b9b3a2d0df7150fad56f65a37b8937",
        native_hf_file: "F2LLM-v2-160M.Q4_K_M.gguf",
        native_size_bytes: 151_808_704,
        native_sha256: "19db6aebaacc6d4f496cfa6226c31fca84e8f72935ee6444a3e25dc1dcc90645",
        web_model: "codefuse-ai/F2LLM-v2-160M",
        dimensions: 640,
    },
    NativeF2LlmModel {
        model_key: DEFAULT_MODEL_KEY,
        model_revision: DEFAULT_MODEL_REVISION,
        input_format_version: DEFAULT_INPUT_FORMAT_VERSION,
        native_hf_repo: "mradermacher/F2LLM-v2-330M-GGUF",
        native_hf_revision: "03158c3a78ea1c7a7eea2d6829c49e3f1d63f85f",
        native_hf_file: "F2LLM-v2-330M.Q4_K_M.gguf",
        native_size_bytes: 286_198_400,
        native_sha256: "7f3c03769de1436ad1f9014cb2872d2f7b5d8aa5f2322796c5070867c84dc254",
        web_model: "codefuse-ai/F2LLM-v2-330M",
        dimensions: DEFAULT_MODEL_DIMENSIONS,
    },
    NativeF2LlmModel {
        model_key: "f2llm-v2-0.6b-q4-k-m-1024",
        model_revision: "2b4159091278275a3b00d2c39095754d59d7d7de",
        input_format_version: "f2llm-v2-0.6b-q4-k-m-v0",
        native_hf_repo: "mradermacher/F2LLM-v2-0.6B-GGUF",
        native_hf_revision: "641cb8859b59035469cb7d93fbd96f61cd5be4a7",
        native_hf_file: "F2LLM-v2-0.6B.Q4_K_M.gguf",
        native_size_bytes: 396_706_560,
        native_sha256: "6f106cf54671f2aadf32c5f0da600c5a7ac430404b6ccda097cb7c7a45021f4b",
        web_model: "codefuse-ai/F2LLM-v2-0.6B",
        dimensions: 1024,
    },
];

static DICTIONARY_CORPUS_CACHE: OnceLock<
    Mutex<HashMap<LoadedCorpusCacheKey, Arc<LoadedCorpus<DictionaryEmbeddingItem>>>>,
> = OnceLock::new();
static CLL_CORPUS_CACHE: OnceLock<
    Mutex<HashMap<LoadedCorpusCacheKey, Arc<LoadedCorpus<CllEmbeddingItem>>>>,
> = OnceLock::new();

#[derive(Debug, Error)]
#[invariant(true)]
#[invariant(::Environment { .. } => true)]
#[invariant(::Io { .. } => true)]
#[invariant(::Json { .. } => true)]
#[invariant(::Http { .. } => true)]
#[invariant(::InvalidModel { .. } => true)]
#[invariant(::InvalidIndex { .. } => true)]
#[invariant(::UnsupportedModel { .. } => true)]
#[invariant(::MissingCompatiblePack { .. } => true)]
#[invariant(::DimensionMismatch { .. } => true)]
#[invariant(::Backend { .. } => true)]
pub enum EmbeddingError {
    #[error("{message}")]
    Environment { message: String },
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{context}: {source}")]
    Json {
        context: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("{message}")]
    Http { message: String },
    #[error("{message}")]
    InvalidModel { message: String },
    #[error("{message}")]
    InvalidIndex { message: String },
    #[error("unsupported embedding model `{model_key}`")]
    UnsupportedModel { model_key: String },
    #[error(
        "no compatible embedding vector pack found for `{model_key}`; run `jbotci setup --embedding`"
    )]
    MissingCompatiblePack { model_key: String },
    #[error("embedding dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    #[error("{message}")]
    Backend { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct EmbeddingModelSpec {
    pub model_key: String,
    pub model_revision: String,
    pub input_format_version: String,
    pub native_hf_repo: String,
    pub native_hf_revision: String,
    pub native_hf_file: String,
    pub native_size_bytes: u64,
    pub native_sha256: String,
    pub web_model: String,
    pub web_dtype: String,
    pub dimensions: usize,
}

impl EmbeddingModelSpec {
    #[requires(true)]
    #[ensures(ret.model_key == DEFAULT_MODEL_KEY)]
    pub fn default_f2llm() -> Self {
        native_f2llm_model_spec(
            NATIVE_F2LLM_MODELS
                .iter()
                .find(|model| model.model_key == DEFAULT_MODEL_KEY)
                .expect("native F2LLM model table includes the default model"),
        )
    }
}

#[requires(!model.model_key.is_empty())]
#[ensures(ret.model_key == model.model_key)]
fn native_f2llm_model_spec(model: &NativeF2LlmModel) -> EmbeddingModelSpec {
    EmbeddingModelSpec {
        model_key: model.model_key.to_owned(),
        model_revision: model.model_revision.to_owned(),
        input_format_version: model.input_format_version.to_owned(),
        native_hf_repo: model.native_hf_repo.to_owned(),
        native_hf_revision: model.native_hf_revision.to_owned(),
        native_hf_file: model.native_hf_file.to_owned(),
        native_size_bytes: model.native_size_bytes,
        native_sha256: model.native_sha256.to_owned(),
        web_model: model.web_model.to_owned(),
        web_dtype: DEFAULT_WEB_DTYPE.to_owned(),
        dimensions: model.dimensions,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_some_and(|spec| spec.model_key == model_key) || ret.is_none())]
pub fn model_spec(model_key: &str) -> Option<EmbeddingModelSpec> {
    NATIVE_F2LLM_MODELS
        .iter()
        .find(|model| model.model_key == model_key)
        .map(native_f2llm_model_spec)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct EmbeddingCatalogModel {
    pub model_key: String,
    pub latest_pack_id: String,
    pub manifest_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct EmbeddingCatalog {
    pub schema_version: u32,
    pub models: Vec<EmbeddingCatalogModel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct EmbeddingRuntime {
    pub runtime: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct VectorShardManifest {
    pub url: String,
    pub byte_len: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct CorpusManifest {
    pub corpus_id: String,
    pub input_format_version: String,
    pub fingerprint: String,
    pub row_count: usize,
    pub dimensions: usize,
    pub items_url: String,
    pub items_sha256: String,
    pub shards: Vec<VectorShardManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct EmbeddingPackManifest {
    pub schema_version: u32,
    pub model_key: String,
    pub model_revision: String,
    pub pack_id: String,
    pub input_format_version: String,
    pub built_by: EmbeddingRuntime,
    pub dimensions: usize,
    pub element_type: String,
    pub normalized: bool,
    pub distance: String,
    pub compatible_query_runtimes: Vec<EmbeddingRuntime>,
    pub corpora: Vec<CorpusManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct DictionaryEmbeddingItem {
    pub entry_index: usize,
    pub word: String,
    pub definition_id: u64,
    pub input_hash: String,
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct CllEmbeddingItem {
    pub chunk_index: usize,
    pub input_hash: String,
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct EmbeddingBuildRow<T> {
    item: T,
    input: String,
    input_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
struct NativePartialBuildCheckpoint {
    schema_version: u32,
    source: String,
    pack_id: String,
    model_key: String,
    model_revision: String,
    input_format_version: String,
    built_by: EmbeddingRuntime,
    dimensions: usize,
    element_type: String,
    normalized: bool,
    distance: String,
    chunk_rows: usize,
    dictionary_fingerprint: String,
    dictionary_rows: usize,
    cll_fingerprint: String,
    cll_rows: usize,
    corpora: Vec<NativePartialCorpus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
struct NativePartialCorpus {
    corpus_id: String,
    input_format_version: String,
    fingerprint: String,
    row_count: usize,
    dimensions: usize,
    items_url: String,
    items_sha256: String,
    shards: Vec<NativePartialShard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
struct NativePartialShard {
    url: String,
    byte_len: u64,
    sha256: String,
    row_start: usize,
    row_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[invariant(true)]
struct LoadedCorpusCacheKey {
    pack_dir: PathBuf,
    model_key: String,
    pack_id: String,
    corpus_id: String,
    items_sha256: String,
    vector_shards_hash: String,
    row_count: usize,
    dimensions: usize,
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct LoadedCorpus<T> {
    items: Vec<T>,
    values: Vec<f32>,
    row_count: usize,
    dimensions: usize,
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct VectorHit {
    pub row_index: usize,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct DictionarySemanticHit {
    pub entry_index: usize,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct QueryEmbedding {
    pub values: Vec<f32>,
}

#[contract_trait]
pub trait EmbeddingBackend {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|dimensions| *dimensions > 0) || ret.is_err())]
    fn dimensions(&self) -> Result<usize, EmbeddingError>;

    #[requires(!input.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|embedding| !embedding.values.is_empty()) || ret.is_err())]
    fn embed(&mut self, input: &str) -> Result<QueryEmbedding, EmbeddingError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct SetupOptions {
    pub model_key: String,
    pub force: bool,
    pub use_precomputed: UsePrecomputed,
    pub skip_validation: bool,
    pub index_dir: Option<PathBuf>,
    pub model_dir: Option<PathBuf>,
    pub precomputed_base_url: String,
}

impl Default for SetupOptions {
    #[requires(true)]
    #[ensures(ret.model_key == DEFAULT_MODEL_KEY)]
    fn default() -> Self {
        Self {
            model_key: DEFAULT_MODEL_KEY.to_owned(),
            force: false,
            use_precomputed: UsePrecomputed::Auto,
            skip_validation: false,
            index_dir: None,
            model_dir: None,
            precomputed_base_url: DEFAULT_GGUF_EMBEDDINGS_BASE_URL.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(::Auto => true)]
#[invariant(::Always => true)]
#[invariant(::Never => true)]
pub enum UsePrecomputed {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(::Reused => true)]
#[invariant(::DownloadedPrecomputed => true)]
#[invariant(::BuiltLocal => true)]
pub enum SetupIndexSource {
    Reused,
    DownloadedPrecomputed,
    BuiltLocal,
}

impl SetupIndexSource {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reused => "reused",
            Self::DownloadedPrecomputed => "downloaded-precomputed",
            Self::BuiltLocal => "built-local",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct SetupReport {
    pub index_root: PathBuf,
    pub model_path: PathBuf,
    pub pack_id: String,
    pub dictionary_rows: usize,
    pub cll_rows: usize,
    pub index_source: SetupIndexSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(::ResolvingPaths => true)]
#[invariant(::DownloadingModel => true)]
#[invariant(::DownloadingIndex => true)]
#[invariant(::ValidatingModel => true)]
#[invariant(::ValidatingIndex => true)]
#[invariant(::LoadingModel => true)]
#[invariant(::Indexing => true)]
#[invariant(::WritingIndex => true)]
#[invariant(::ReusingIndex => true)]
#[invariant(::Complete => true)]
#[invariant(::Error => true)]
pub enum SetupProgressPhase {
    ResolvingPaths,
    DownloadingModel,
    DownloadingIndex,
    ValidatingModel,
    ValidatingIndex,
    LoadingModel,
    Indexing,
    WritingIndex,
    ReusingIndex,
    Complete,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct SetupProgress {
    pub phase: SetupProgressPhase,
    pub kind: String,
    pub label: String,
    pub detail: String,
    pub loaded: Option<u64>,
    pub total: Option<u64>,
    pub percent: Option<u8>,
}

impl SetupProgress {
    #[requires(!kind.is_empty())]
    #[requires(!label.is_empty())]
    #[requires(!detail.is_empty())]
    #[ensures(ret.kind == kind)]
    pub fn indeterminate(phase: SetupProgressPhase, kind: &str, label: &str, detail: &str) -> Self {
        Self {
            phase,
            kind: kind.to_owned(),
            label: label.to_owned(),
            detail: detail.to_owned(),
            loaded: None,
            total: None,
            percent: None,
        }
    }

    #[requires(!kind.is_empty())]
    #[requires(!label.is_empty())]
    #[requires(!detail.is_empty())]
    #[requires(loaded <= total)]
    #[ensures(ret.loaded == Some(loaded))]
    #[ensures(ret.total == Some(total))]
    pub fn determinate(
        phase: SetupProgressPhase,
        kind: &str,
        label: &str,
        detail: &str,
        loaded: u64,
        total: u64,
    ) -> Self {
        Self {
            phase,
            kind: kind.to_owned(),
            label: label.to_owned(),
            detail: detail.to_owned(),
            loaded: Some(loaded),
            total: Some(total),
            percent: progress_percent(loaded, total),
        }
    }
}

#[requires(loaded <= total)]
#[ensures(ret.is_none_or(|percent| percent <= 100))]
pub fn progress_percent(loaded: u64, total: u64) -> Option<u8> {
    if total == 0 {
        return None;
    }
    Some(((loaded.saturating_mul(100)) / total).min(100) as u8)
}

pub type SetupProgressCallback<'a> = dyn FnMut(SetupProgress) + 'a;

#[derive(Debug, Clone, Default)]
#[invariant(true)]
struct ReusableVectorRows {
    rows_by_input_hash: HashMap<String, Vec<f32>>,
}

impl ReusableVectorRows {
    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|row| row.len() == dimensions))]
    fn row(&self, input_hash: &str, dimensions: usize) -> Option<&[f32]> {
        self.rows_by_input_hash
            .get(input_hash)
            .filter(|row| row.len() == dimensions)
            .map(Vec::as_slice)
    }
}

#[derive(Debug, Clone, Default)]
#[invariant(true)]
struct ReusablePackRows {
    dictionary: ReusableVectorRows,
    cll: ReusableVectorRows,
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|path| !path.as_os_str().is_empty()) || ret.is_err())]
pub fn default_model_root() -> Result<PathBuf, EmbeddingError> {
    if let Some(value) = non_empty_env_path(EMBEDDING_MODEL_DIR_ENV) {
        return Ok(value);
    }
    let project = project_dirs()?;
    Ok(project.cache_dir().join("embeddings").join("models"))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|path| !path.as_os_str().is_empty()) || ret.is_err())]
pub fn default_index_root() -> Result<PathBuf, EmbeddingError> {
    if let Some(value) = non_empty_env_path(EMBEDDING_INDEX_DIR_ENV) {
        return Ok(value);
    }
    let project = project_dirs()?;
    Ok(project.data_dir().join("embeddings").join("indexes"))
}

#[requires(true)]
#[ensures(true)]
fn project_dirs() -> Result<ProjectDirs, EmbeddingError> {
    ProjectDirs::from("org", "int19h", "jbotci").ok_or_else(|| EmbeddingError::Environment {
        message: format!(
            "Unable to resolve app directories: set {EMBEDDING_MODEL_DIR_ENV} or {EMBEDDING_INDEX_DIR_ENV}."
        ),
    })
}

#[requires(true)]
#[ensures(true)]
fn non_empty_env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .map(PathBuf::from)
        .filter(|value| !value.as_os_str().is_empty())
}

#[requires(!spec.model_key.is_empty())]
#[ensures(!ret.as_os_str().is_empty())]
pub fn model_file_path(root: &Path, spec: &EmbeddingModelSpec) -> PathBuf {
    root.join(&spec.model_key).join(&spec.native_hf_file)
}

#[requires(!model_key.is_empty())]
#[ensures(!ret.as_os_str().is_empty())]
pub fn pack_root(index_root: &Path, model_key: &str, pack_id: &str) -> PathBuf {
    index_root
        .join(INDEX_BASE_VERSION)
        .join("models")
        .join(model_key)
        .join("packs")
        .join(pack_id)
}

#[requires(!model_key.is_empty())]
#[ensures(!ret.as_os_str().is_empty())]
pub fn model_packs_root(index_root: &Path, model_key: &str) -> PathBuf {
    index_root
        .join(INDEX_BASE_VERSION)
        .join("models")
        .join(model_key)
        .join("packs")
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|path| path.ends_with("catalog.json")) || ret.is_err())]
pub fn catalog_path(index_root: &Path) -> Result<PathBuf, EmbeddingError> {
    Ok(index_root.join(INDEX_BASE_VERSION).join("catalog.json"))
}

#[requires(!spec.native_hf_repo.is_empty())]
#[requires(!spec.native_hf_revision.is_empty())]
#[requires(!spec.native_hf_file.is_empty())]
#[ensures(ret.contains(&spec.native_hf_repo))]
pub fn model_download_url(spec: &EmbeddingModelSpec) -> String {
    let endpoint = env::var(HF_ENDPOINT_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_HF_ENDPOINT.to_owned());
    format!(
        "{}/{}/resolve/{}/{}",
        endpoint.trim_end_matches('/'),
        spec.native_hf_repo,
        spec.native_hf_revision,
        spec.native_hf_file
    )
}

#[requires(!path.as_os_str().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|value| value.len() == 64) || ret.is_err())]
pub fn sha256_hex_file(path: &Path) -> Result<String, EmbeddingError> {
    let mut progress = |_| {};
    sha256_hex_file_with_progress(
        path,
        SetupProgressPhase::ValidatingModel,
        "validate",
        "Validating file",
        "Checking file SHA-256.",
        None,
        &mut progress,
    )
}

#[allow(clippy::too_many_arguments)]
#[requires(!path.as_os_str().is_empty())]
#[requires(!kind.is_empty())]
#[requires(!label.is_empty())]
#[requires(!detail.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|value| value.len() == 64) || ret.is_err())]
fn sha256_hex_file_with_progress(
    path: &Path,
    phase: SetupProgressPhase,
    kind: &str,
    label: &str,
    detail: &str,
    total: Option<u64>,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<String, EmbeddingError> {
    let mut file = File::open(path).map_err(|source| EmbeddingError::Io {
        context: format!("failed to open `{}`", path.display()),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut loaded = 0u64;
    if let Some(total) = total {
        progress(SetupProgress::determinate(
            phase, kind, label, detail, 0, total,
        ));
    }
    loop {
        let read = file.read(&mut buf).map_err(|source| EmbeddingError::Io {
            context: format!("failed to read `{}`", path.display()),
            source,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
        if let Some(total) = total {
            loaded = loaded.saturating_add(read as u64);
            progress(SetupProgress::determinate(
                phase,
                kind,
                label,
                detail,
                loaded.min(total),
                total,
            ));
        }
    }
    Ok(hex_digest(hasher.finalize()))
}

#[requires(true)]
#[ensures(ret.len() == 64)]
fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub fn normalize_vector(values: &mut [f32]) {
    let magnitude = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for value in values {
            *value /= magnitude;
        }
    }
}

#[requires(left.len() == right.len())]
#[ensures(true)]
pub fn dot_product(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum()
}

#[requires(dimensions > 0)]
#[requires(values.len() % dimensions == 0)]
#[ensures(ret.len() <= row_count)]
pub fn top_vector_hits(
    values: &[f32],
    dimensions: usize,
    query: &[f32],
    row_count: usize,
    limit: usize,
) -> Vec<VectorHit> {
    top_vector_hits_by_row(values, dimensions, query, row_count, limit, |_| true)
}

#[requires(dimensions > 0)]
#[requires(values.len() % dimensions == 0)]
#[ensures(ret.len() <= row_count)]
fn top_vector_hits_by_row<F>(
    values: &[f32],
    dimensions: usize,
    query: &[f32],
    row_count: usize,
    limit: usize,
    mut row_allowed: F,
) -> Vec<VectorHit>
where
    F: FnMut(usize) -> bool,
{
    if query.len() != dimensions || limit == 0 {
        return Vec::new();
    }
    let effective_row_count = row_count.min(values.len() / dimensions);
    let mut hits = if limit >= effective_row_count {
        values
            .chunks_exact(dimensions)
            .take(effective_row_count)
            .enumerate()
            .filter(|(row_index, _)| row_allowed(*row_index))
            .map(|(row_index, vector)| VectorHit {
                row_index,
                score: dot_product(vector, query),
            })
            .collect::<Vec<_>>()
    } else {
        let mut hits = Vec::with_capacity(limit);
        for (row_index, vector) in values
            .chunks_exact(dimensions)
            .take(effective_row_count)
            .enumerate()
        {
            if !row_allowed(row_index) {
                continue;
            }
            let candidate = VectorHit {
                row_index,
                score: dot_product(vector, query),
            };
            if hits.len() < limit {
                hits.push(candidate);
                continue;
            }
            if let Some(worst_index) = worst_vector_hit_index(&hits)
                && compare_vector_hits_best_first(&candidate, &hits[worst_index]) == Ordering::Less
            {
                hits[worst_index] = candidate;
            }
        }
        hits
    };
    hits.sort_by(compare_vector_hits_best_first);
    hits.truncate(limit);
    hits
}

#[requires(true)]
#[ensures(true)]
fn compare_vector_hits_best_first(left: &VectorHit, right: &VectorHit) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.row_index.cmp(&right.row_index))
}

#[requires(true)]
#[ensures(ret.is_none_or(|index| index < hits.len()))]
fn worst_vector_hit_index(hits: &[VectorHit]) -> Option<usize> {
    hits.iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| compare_vector_hits_best_first(left, right))
        .map(|(index, _)| index)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|_| path.is_file()) || ret.is_err())]
pub fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), EmbeddingError> {
    ensure_parent_dir(path)?;
    let file = File::create(path).map_err(|source| EmbeddingError::Io {
        context: format!("failed to create `{}`", path.display()),
        source,
    })?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value).map_err(|source| EmbeddingError::Json {
        context: format!("failed to serialize `{}`", path.display()),
        source,
    })?;
    writer
        .write_all(b"\n")
        .map_err(|source| EmbeddingError::Io {
            context: format!("failed to write `{}`", path.display()),
            source,
        })?;
    writer.flush().map_err(|source| EmbeddingError::Io {
        context: format!("failed to flush `{}`", path.display()),
        source,
    })?;
    write_brotli_sibling(path)?;
    Ok(())
}

#[requires(!path.as_os_str().is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn write_json_file_atomically<T: Serialize>(path: &Path, value: &T) -> Result<(), EmbeddingError> {
    ensure_parent_dir(path)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| EmbeddingError::Io {
            context: format!("failed to create temporary name for `{}`", path.display()),
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no file name"),
        })?;
    let temp_path = path.with_file_name(format!("{file_name}.tmp"));
    let file = File::create(&temp_path).map_err(|source| EmbeddingError::Io {
        context: format!("failed to create `{}`", temp_path.display()),
        source,
    })?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value).map_err(|source| EmbeddingError::Json {
        context: format!("failed to serialize `{}`", temp_path.display()),
        source,
    })?;
    writer
        .write_all(b"\n")
        .map_err(|source| EmbeddingError::Io {
            context: format!("failed to write `{}`", temp_path.display()),
            source,
        })?;
    writer.flush().map_err(|source| EmbeddingError::Io {
        context: format!("failed to flush `{}`", temp_path.display()),
        source,
    })?;
    rename_replacing(&temp_path, path)?;
    Ok(())
}

#[requires(!path.as_os_str().is_empty())]
#[ensures(true)]
pub fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, EmbeddingError> {
    let file = File::open(path).map_err(|source| EmbeddingError::Io {
        context: format!("failed to open `{}`", path.display()),
        source,
    })?;
    serde_json::from_reader(BufReader::new(file)).map_err(|source| EmbeddingError::Json {
        context: format!("failed to parse `{}`", path.display()),
        source,
    })
}

#[requires(path.is_file())]
#[ensures(true)]
pub fn write_brotli_sibling(path: &Path) -> Result<(), EmbeddingError> {
    let bytes = fs::read(path).map_err(|source| EmbeddingError::Io {
        context: format!("failed to read `{}`", path.display()),
        source,
    })?;
    let br_path = PathBuf::from(format!("{}.br", path.display()));
    let file = File::create(&br_path).map_err(|source| EmbeddingError::Io {
        context: format!("failed to create `{}`", br_path.display()),
        source,
    })?;
    let mut compressor = brotli::CompressorWriter::new(BufWriter::new(file), 4096, 5, 22);
    compressor
        .write_all(&bytes)
        .map_err(|source| EmbeddingError::Io {
            context: format!("failed to compress `{}`", path.display()),
            source,
        })?;
    compressor.flush().map_err(|source| EmbeddingError::Io {
        context: format!("failed to flush `{}`", br_path.display()),
        source,
    })?;
    Ok(())
}

#[requires(dimensions > 0)]
#[requires(values.len() % dimensions == 0)]
#[ensures(ret.as_ref().is_ok_and(|shards| !shards.is_empty() || values.is_empty()) || ret.is_err())]
pub fn write_vector_shards(
    corpus_dir: &Path,
    url_prefix: &str,
    values: &[f32],
    dimensions: usize,
    shard_target_bytes: NonZeroUsize,
) -> Result<Vec<VectorShardManifest>, EmbeddingError> {
    fs::create_dir_all(corpus_dir).map_err(|source| EmbeddingError::Io {
        context: format!("failed to create `{}`", corpus_dir.display()),
        source,
    })?;
    let row_bytes = dimensions * std::mem::size_of::<f32>();
    let rows_per_shard = (shard_target_bytes.get() / row_bytes).max(1);
    let mut output = Vec::new();
    for (shard_index, shard_rows) in values.chunks(rows_per_shard * dimensions).enumerate() {
        let file_name = format!("vectors-{shard_index:04}.f32");
        let path = corpus_dir.join(&file_name);
        let mut file =
            BufWriter::new(File::create(&path).map_err(|source| EmbeddingError::Io {
                context: format!("failed to create `{}`", path.display()),
                source,
            })?);
        for value in shard_rows {
            file.write_all(&value.to_le_bytes())
                .map_err(|source| EmbeddingError::Io {
                    context: format!("failed to write `{}`", path.display()),
                    source,
                })?;
        }
        file.flush().map_err(|source| EmbeddingError::Io {
            context: format!("failed to flush `{}`", path.display()),
            source,
        })?;
        let byte_len = fs::metadata(&path)
            .map_err(|source| EmbeddingError::Io {
                context: format!("failed to inspect `{}`", path.display()),
                source,
            })?
            .len();
        let sha256 = sha256_hex_file(&path)?;
        write_brotli_sibling(&path)?;
        output.push(VectorShardManifest {
            url: format!("{}/{}", url_prefix.trim_end_matches('/'), file_name),
            byte_len,
            sha256,
        });
    }
    Ok(output)
}

#[requires(dimensions > 0)]
#[requires(values.len() % dimensions == 0)]
#[requires(!file_name.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|shard| shard.byte_len > 0 || values.is_empty()) || ret.is_err())]
fn write_vector_chunk_file(
    corpus_dir: &Path,
    url_prefix: &str,
    file_name: &str,
    values: &[f32],
    dimensions: usize,
) -> Result<VectorShardManifest, EmbeddingError> {
    fs::create_dir_all(corpus_dir).map_err(|source| EmbeddingError::Io {
        context: format!("failed to create `{}`", corpus_dir.display()),
        source,
    })?;
    let path = corpus_dir.join(file_name);
    let temp_path = corpus_dir.join(format!("{file_name}.tmp"));
    let mut file =
        BufWriter::new(
            File::create(&temp_path).map_err(|source| EmbeddingError::Io {
                context: format!("failed to create `{}`", temp_path.display()),
                source,
            })?,
        );
    for value in values {
        file.write_all(&value.to_le_bytes())
            .map_err(|source| EmbeddingError::Io {
                context: format!("failed to write `{}`", temp_path.display()),
                source,
            })?;
    }
    file.flush().map_err(|source| EmbeddingError::Io {
        context: format!("failed to flush `{}`", temp_path.display()),
        source,
    })?;
    rename_replacing(&temp_path, &path)?;
    let byte_len = fs::metadata(&path)
        .map_err(|source| EmbeddingError::Io {
            context: format!("failed to inspect `{}`", path.display()),
            source,
        })?
        .len();
    let expected_byte_len = values.len() * std::mem::size_of::<f32>();
    if byte_len != expected_byte_len as u64 {
        return Err(EmbeddingError::InvalidIndex {
            message: format!(
                "vector shard `{}` is {} bytes, expected {}",
                path.display(),
                byte_len,
                expected_byte_len
            ),
        });
    }
    let sha256 = sha256_hex_file(&path)?;
    write_brotli_sibling(&path)?;
    Ok(VectorShardManifest {
        url: format!("{}/{}", url_prefix.trim_end_matches('/'), file_name),
        byte_len,
        sha256,
    })
}

#[requires(dimensions > 0)]
#[ensures(true)]
pub fn read_vector_shards(
    pack_dir: &Path,
    corpus: &CorpusManifest,
    dimensions: usize,
) -> Result<Vec<f32>, EmbeddingError> {
    if corpus.dimensions != dimensions {
        return Err(EmbeddingError::DimensionMismatch {
            expected: dimensions,
            actual: corpus.dimensions,
        });
    }
    let mut bytes = Vec::new();
    for shard in &corpus.shards {
        let path = pack_dir.join(shard.url.trim_start_matches('/'));
        let shard_bytes = fs::read(&path).map_err(|source| EmbeddingError::Io {
            context: format!("failed to read `{}`", path.display()),
            source,
        })?;
        if shard_bytes.len() as u64 != shard.byte_len {
            return Err(EmbeddingError::InvalidIndex {
                message: format!("vector shard `{}` size mismatch", path.display()),
            });
        }
        if sha256_hex_bytes(&shard_bytes) != shard.sha256 {
            return Err(EmbeddingError::InvalidIndex {
                message: format!("vector shard `{}` SHA-256 mismatch", path.display()),
            });
        }
        bytes.extend_from_slice(&shard_bytes);
    }
    if bytes.len() % std::mem::size_of::<f32>() != 0 {
        return Err(EmbeddingError::InvalidIndex {
            message: "vector bytes are not aligned to f32".to_owned(),
        });
    }
    let values = bytes
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .collect::<Vec<_>>();
    if values.len() != corpus.row_count * dimensions {
        return Err(EmbeddingError::InvalidIndex {
            message: format!(
                "vector matrix for `{}` has {} f32 values, expected {}",
                corpus.corpus_id,
                values.len(),
                corpus.row_count * dimensions
            ),
        });
    }
    Ok(values)
}

#[requires(dimensions > 0)]
#[ensures(ret.as_ref().is_ok_and(|values| values.len() == shard.row_count * dimensions) || ret.is_err())]
fn read_native_partial_vector_shard(
    pack_dir: &Path,
    shard: &NativePartialShard,
    dimensions: usize,
) -> Result<Vec<f32>, EmbeddingError> {
    let path = pack_dir.join(shard.url.trim_start_matches('/'));
    let shard_bytes = fs::read(&path).map_err(|source| EmbeddingError::Io {
        context: format!("failed to read `{}`", path.display()),
        source,
    })?;
    if shard_bytes.len() as u64 != shard.byte_len {
        return Err(EmbeddingError::InvalidIndex {
            message: format!("vector shard `{}` size mismatch", path.display()),
        });
    }
    if sha256_hex_bytes(&shard_bytes) != shard.sha256 {
        return Err(EmbeddingError::InvalidIndex {
            message: format!("vector shard `{}` SHA-256 mismatch", path.display()),
        });
    }
    if shard_bytes.len() % std::mem::size_of::<f32>() != 0 {
        return Err(EmbeddingError::InvalidIndex {
            message: "partial vector bytes are not aligned to f32".to_owned(),
        });
    }
    let values = shard_bytes
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .collect::<Vec<_>>();
    if values.len() != shard.row_count * dimensions {
        return Err(EmbeddingError::InvalidIndex {
            message: format!(
                "partial vector shard `{}` has {} f32 values, expected {}",
                path.display(),
                values.len(),
                shard.row_count * dimensions
            ),
        });
    }
    Ok(values)
}

#[requires(!path.as_os_str().is_empty())]
#[requires(!description.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|_| path.is_file()) || ret.is_err())]
fn require_index_file(path: &Path, description: &str) -> Result<(), EmbeddingError> {
    if path.is_file() {
        return Ok(());
    }
    Err(EmbeddingError::InvalidIndex {
        message: format!("{description} `{}` is missing", path.display()),
    })
}

#[requires(true)]
#[ensures(true)]
fn ensure_parent_dir(path: &Path) -> Result<(), EmbeddingError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| EmbeddingError::Io {
            context: format!("failed to create `{}`", parent.display()),
            source,
        })?;
    }
    Ok(())
}

#[requires(!source.as_os_str().is_empty())]
#[requires(!destination.as_os_str().is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn rename_replacing(source: &Path, destination: &Path) -> Result<(), EmbeddingError> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            fs::remove_file(destination).map_err(|error| EmbeddingError::Io {
                context: format!("failed to remove `{}`", destination.display()),
                source: error,
            })?;
            fs::rename(source, destination).map_err(|error| EmbeddingError::Io {
                context: format!(
                    "failed to move `{}` to `{}`",
                    source.display(),
                    destination.display()
                ),
                source: error,
            })
        }
        Err(error) => Err(EmbeddingError::Io {
            context: format!(
                "failed to move `{}` to `{}`",
                source.display(),
                destination.display()
            ),
            source: error,
        }),
    }
}

#[requires(!path.as_os_str().is_empty())]
#[requires(!suffix.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|sibling| sibling.parent() == path.parent()) || ret.is_err())]
fn sibling_path_with_suffix(path: &Path, suffix: &str) -> Result<PathBuf, EmbeddingError> {
    let file_name = path.file_name().ok_or_else(|| EmbeddingError::Io {
        context: format!("failed to create sibling path for `{}`", path.display()),
        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no file name"),
    })?;
    let mut sibling_name = file_name.to_os_string();
    sibling_name.push(format!(".{suffix}"));
    Ok(path.with_file_name(sibling_name))
}

#[requires(!input_format_version.is_empty())]
#[ensures(!ret.is_empty())]
pub fn deterministic_pack_id(
    input_format_version: &str,
    model_revision: &str,
    dictionary_fingerprint: &str,
    cll_fingerprint: &str,
) -> String {
    format!(
        "{}-{}-{}-{}",
        input_format_version,
        short_fingerprint(model_revision),
        short_fingerprint(dictionary_fingerprint),
        short_fingerprint(cll_fingerprint)
    )
}

#[requires(!value.is_empty())]
#[ensures(ret.len() <= 12)]
fn short_fingerprint(value: &str) -> String {
    value.chars().take(12).collect()
}

#[requires(true)]
#[ensures(ret.runtime == "llama-cpp-4")]
fn native_embedding_runtime() -> EmbeddingRuntime {
    EmbeddingRuntime {
        runtime: "llama-cpp-4".to_owned(),
        version: LLAMA_CPP_4_RUNTIME_VERSION.to_owned(),
    }
}

#[requires(true)]
#[ensures(!ret.as_os_str().is_empty())]
fn native_work_pack_root(work_root: &Path) -> PathBuf {
    work_root.join("pack")
}

#[requires(true)]
#[ensures(!ret.as_os_str().is_empty())]
fn native_partial_checkpoint_path(work_root: &Path) -> PathBuf {
    work_root.join(NATIVE_PARTIAL_BUILD_FILE)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn remove_dir_all_if_exists(path: &Path) -> Result<(), EmbeddingError> {
    if !path.exists() {
        return Ok(());
    }
    fs::remove_dir_all(path).map_err(|source| EmbeddingError::Io {
        context: format!("failed to remove `{}`", path.display()),
        source,
    })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn native_vector_chunk_file_name(chunk_index: usize) -> String {
    format!("vectors-{chunk_index:04}.f32")
}

#[requires(!corpus_id.is_empty())]
#[ensures(!ret.is_empty())]
fn native_vector_chunk_url(corpus_id: &str, chunk_index: usize) -> String {
    format!(
        "corpora/{corpus_id}/{}",
        native_vector_chunk_file_name(chunk_index)
    )
}

#[allow(clippy::too_many_arguments)]
#[requires(dimensions > 0)]
#[ensures(ret.schema_version == NATIVE_PARTIAL_BUILD_SCHEMA_VERSION)]
fn initial_native_partial_checkpoint(
    spec: &EmbeddingModelSpec,
    pack_id: &str,
    dimensions: usize,
    dictionary_fingerprint: &str,
    dictionary_rows: usize,
    cll_fingerprint: &str,
    cll_rows: usize,
) -> NativePartialBuildCheckpoint {
    NativePartialBuildCheckpoint {
        schema_version: NATIVE_PARTIAL_BUILD_SCHEMA_VERSION,
        source: NATIVE_PARTIAL_BUILD_SOURCE.to_owned(),
        pack_id: pack_id.to_owned(),
        model_key: spec.model_key.clone(),
        model_revision: spec.model_revision.clone(),
        input_format_version: spec.input_format_version.clone(),
        built_by: native_embedding_runtime(),
        dimensions,
        element_type: "f32le".to_owned(),
        normalized: true,
        distance: "dot".to_owned(),
        chunk_rows: NATIVE_VECTOR_CHUNK_ROWS,
        dictionary_fingerprint: dictionary_fingerprint.to_owned(),
        dictionary_rows,
        cll_fingerprint: cll_fingerprint.to_owned(),
        cll_rows,
        corpora: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
#[requires(dimensions > 0)]
#[ensures(true)]
fn load_compatible_native_partial_checkpoint(
    work_root: &Path,
    pack_root: &Path,
    spec: &EmbeddingModelSpec,
    pack_id: &str,
    dimensions: usize,
    dictionary_fingerprint: &str,
    dictionary_rows: usize,
    cll_fingerprint: &str,
    cll_rows: usize,
) -> Option<NativePartialBuildCheckpoint> {
    let path = native_partial_checkpoint_path(work_root);
    if !path.is_file() {
        return None;
    }
    let checkpoint: NativePartialBuildCheckpoint = read_json_file(&path).ok()?;
    if !native_partial_header_matches(
        &checkpoint,
        spec,
        pack_id,
        dimensions,
        dictionary_fingerprint,
        dictionary_rows,
        cll_fingerprint,
        cll_rows,
    ) {
        return None;
    }
    let mut seen_corpora = HashMap::new();
    for corpus in &checkpoint.corpora {
        if seen_corpora.insert(corpus.corpus_id.clone(), ()).is_some()
            || !native_partial_corpus_is_compatible(pack_root, &checkpoint, corpus, dimensions)
        {
            return None;
        }
    }
    Some(checkpoint)
}

#[allow(clippy::too_many_arguments)]
#[requires(dimensions > 0)]
#[ensures(true)]
fn native_partial_header_matches(
    checkpoint: &NativePartialBuildCheckpoint,
    spec: &EmbeddingModelSpec,
    pack_id: &str,
    dimensions: usize,
    dictionary_fingerprint: &str,
    dictionary_rows: usize,
    cll_fingerprint: &str,
    cll_rows: usize,
) -> bool {
    checkpoint.schema_version == NATIVE_PARTIAL_BUILD_SCHEMA_VERSION
        && checkpoint.source == NATIVE_PARTIAL_BUILD_SOURCE
        && checkpoint.pack_id == pack_id
        && checkpoint.model_key == spec.model_key
        && checkpoint.model_revision == spec.model_revision
        && checkpoint.input_format_version == spec.input_format_version
        && checkpoint.built_by == native_embedding_runtime()
        && checkpoint.dimensions == dimensions
        && checkpoint.element_type == "f32le"
        && checkpoint.normalized
        && checkpoint.distance == "dot"
        && checkpoint.chunk_rows == NATIVE_VECTOR_CHUNK_ROWS
        && checkpoint.dictionary_fingerprint == dictionary_fingerprint
        && checkpoint.dictionary_rows == dictionary_rows
        && checkpoint.cll_fingerprint == cll_fingerprint
        && checkpoint.cll_rows == cll_rows
}

#[requires(dimensions > 0)]
#[ensures(true)]
fn native_partial_corpus_is_compatible(
    pack_root: &Path,
    checkpoint: &NativePartialBuildCheckpoint,
    corpus: &NativePartialCorpus,
    dimensions: usize,
) -> bool {
    let (expected_fingerprint, expected_rows) = match corpus.corpus_id.as_str() {
        VLACKU_CORPUS_ID => (
            checkpoint.dictionary_fingerprint.as_str(),
            checkpoint.dictionary_rows,
        ),
        CUKTA_CORPUS_ID => (checkpoint.cll_fingerprint.as_str(), checkpoint.cll_rows),
        _ => return false,
    };
    if corpus.input_format_version != checkpoint.input_format_version
        || corpus.fingerprint != expected_fingerprint
        || corpus.dimensions != dimensions
        || corpus.items_url != format!("corpora/{}/items.json", corpus.corpus_id)
        || corpus.row_count != expected_rows
    {
        return false;
    }
    let items_path = pack_root.join(&corpus.items_url);
    if !items_path.is_file() {
        return false;
    }
    if sha256_hex_file(&items_path)
        .ok()
        .is_none_or(|sha256| sha256 != corpus.items_sha256)
    {
        return false;
    }
    let mut seen_starts = HashMap::new();
    for shard in &corpus.shards {
        if seen_starts.insert(shard.row_start, ()).is_some() {
            return false;
        }
        if !native_partial_shard_is_compatible(pack_root, corpus, shard, dimensions) {
            return false;
        }
    }
    true
}

#[requires(dimensions > 0)]
#[ensures(true)]
fn native_partial_shard_is_compatible(
    pack_root: &Path,
    corpus: &NativePartialCorpus,
    shard: &NativePartialShard,
    dimensions: usize,
) -> bool {
    if shard.row_count == 0
        || shard.row_start >= corpus.row_count
        || shard.row_start % NATIVE_VECTOR_CHUNK_ROWS != 0
    {
        return false;
    }
    let chunk_index = shard.row_start / NATIVE_VECTOR_CHUNK_ROWS;
    let expected_row_count =
        NATIVE_VECTOR_CHUNK_ROWS.min(corpus.row_count.saturating_sub(shard.row_start));
    let expected_byte_len = expected_row_count * dimensions * std::mem::size_of::<f32>();
    shard.row_count == expected_row_count
        && shard.byte_len == expected_byte_len as u64
        && shard.url == native_vector_chunk_url(&corpus.corpus_id, chunk_index)
        && read_native_partial_vector_shard(pack_root, shard, dimensions).is_ok()
}

#[requires(!checkpoint_path.as_os_str().is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn write_native_partial_checkpoint(
    checkpoint_path: &Path,
    checkpoint: &NativePartialBuildCheckpoint,
) -> Result<(), EmbeddingError> {
    write_json_file_atomically(checkpoint_path, checkpoint)
}

#[requires(!corpus_id.is_empty())]
#[ensures(true)]
fn native_partial_shards_by_row_start(
    checkpoint: &NativePartialBuildCheckpoint,
    corpus_id: &str,
) -> HashMap<usize, NativePartialShard> {
    checkpoint
        .corpora
        .iter()
        .find(|corpus| corpus.corpus_id == corpus_id)
        .map(|corpus| {
            corpus
                .shards
                .iter()
                .map(|shard| (shard.row_start, shard.clone()))
                .collect()
        })
        .unwrap_or_default()
}

#[requires(!corpus_id.is_empty())]
#[ensures(ret.corpus_id == corpus_id)]
fn native_partial_corpus_from_shards(
    corpus_id: &str,
    input_format_version: &str,
    fingerprint: &str,
    row_count: usize,
    dimensions: usize,
    items_url: &str,
    items_sha256: &str,
    shards_by_start: &HashMap<usize, NativePartialShard>,
) -> NativePartialCorpus {
    let mut shards = shards_by_start.values().cloned().collect::<Vec<_>>();
    shards.sort_by_key(|shard| shard.row_start);
    NativePartialCorpus {
        corpus_id: corpus_id.to_owned(),
        input_format_version: input_format_version.to_owned(),
        fingerprint: fingerprint.to_owned(),
        row_count,
        dimensions,
        items_url: items_url.to_owned(),
        items_sha256: items_sha256.to_owned(),
        shards,
    }
}

#[requires(!corpus.corpus_id.is_empty())]
#[ensures(true)]
fn upsert_native_partial_corpus(
    checkpoint: &mut NativePartialBuildCheckpoint,
    corpus: NativePartialCorpus,
) {
    if let Some(existing) = checkpoint
        .corpora
        .iter_mut()
        .find(|existing| existing.corpus_id == corpus.corpus_id)
    {
        *existing = corpus;
    } else {
        checkpoint.corpora.push(corpus);
    }
    checkpoint
        .corpora
        .sort_by(|left, right| left.corpus_id.cmp(&right.corpus_id));
}

#[requires(true)]
#[ensures(ret.url == shard.url)]
fn vector_shard_from_native_partial(shard: &NativePartialShard) -> VectorShardManifest {
    VectorShardManifest {
        url: shard.url.clone(),
        byte_len: shard.byte_len,
        sha256: shard.sha256.clone(),
    }
}

#[requires(true)]
#[ensures(ret.url == shard.url)]
fn native_partial_shard_from_vector_shard(
    shard: &VectorShardManifest,
    row_start: usize,
    row_count: usize,
) -> NativePartialShard {
    NativePartialShard {
        url: shard.url.clone(),
        byte_len: shard.byte_len,
        sha256: shard.sha256.clone(),
        row_start,
        row_count,
    }
}

#[requires(!path.as_os_str().is_empty())]
#[ensures(true)]
pub fn ensure_model_file(
    spec: &EmbeddingModelSpec,
    path: &Path,
    force: bool,
) -> Result<(), EmbeddingError> {
    let mut progress = |_| {};
    ensure_model_file_with_progress(spec, path, force, false, &mut progress)
}

#[requires(!path.as_os_str().is_empty())]
#[ensures(true)]
pub fn ensure_model_file_with_progress(
    spec: &EmbeddingModelSpec,
    path: &Path,
    force: bool,
    skip_hash_validation: bool,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<(), EmbeddingError> {
    if path.is_file() && !force {
        validate_model_file_with_progress(spec, path, skip_hash_validation, progress)?;
        return Ok(());
    }
    download_model_file_with_progress(spec, path, progress)?;
    validate_model_file_with_progress(spec, path, skip_hash_validation, progress)
}

#[requires(path.is_file())]
#[ensures(true)]
pub fn validate_model_file(spec: &EmbeddingModelSpec, path: &Path) -> Result<(), EmbeddingError> {
    let mut progress = |_| {};
    validate_model_file_with_progress(spec, path, false, &mut progress)
}

#[requires(path.is_file())]
#[ensures(true)]
pub fn validate_model_file_with_progress(
    spec: &EmbeddingModelSpec,
    path: &Path,
    skip_hash_validation: bool,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<(), EmbeddingError> {
    let metadata = fs::metadata(path).map_err(|source| EmbeddingError::Io {
        context: format!("failed to inspect `{}`", path.display()),
        source,
    })?;
    if metadata.len() != spec.native_size_bytes {
        return Err(EmbeddingError::InvalidModel {
            message: format!(
                "model `{}` is {} bytes, expected {}",
                path.display(),
                metadata.len(),
                spec.native_size_bytes
            ),
        });
    }
    if skip_hash_validation {
        progress(SetupProgress::determinate(
            SetupProgressPhase::ValidatingModel,
            "validate",
            "Validating model",
            "Checked embedding model file size; SHA-256 validation was skipped.",
            metadata.len(),
            metadata.len(),
        ));
        return Ok(());
    }
    let sha256 = sha256_hex_file_with_progress(
        path,
        SetupProgressPhase::ValidatingModel,
        "validate",
        "Validating model",
        "Checking embedding model SHA-256.",
        Some(metadata.len()),
        progress,
    )?;
    if sha256 != spec.native_sha256 {
        return Err(EmbeddingError::InvalidModel {
            message: format!(
                "model `{}` SHA-256 mismatch: got {sha256}, expected {}",
                path.display(),
                spec.native_sha256
            ),
        });
    }
    Ok(())
}

#[requires(!spec.native_hf_repo.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|_| path.is_file()) || ret.is_err())]
pub fn download_model_file(spec: &EmbeddingModelSpec, path: &Path) -> Result<(), EmbeddingError> {
    let mut progress = |_| {};
    download_model_file_with_progress(spec, path, &mut progress)
}

#[requires(!spec.native_hf_repo.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|_| path.is_file()) || ret.is_err())]
pub fn download_model_file_with_progress(
    spec: &EmbeddingModelSpec,
    path: &Path,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<(), EmbeddingError> {
    ensure_parent_dir(path)?;
    let url = model_download_url(spec);
    progress(SetupProgress::indeterminate(
        SetupProgressPhase::DownloadingModel,
        "download",
        "Downloading model",
        "Downloading embedding model.",
    ));
    let partial_path = sibling_path_with_suffix(path, "downloadInProgress")?;
    let mut request = ureq::get(&url);
    if let Ok(token) = env::var(HF_TOKEN_ENV)
        && !token.trim().is_empty()
    {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    let response = request.call().map_err(|source| EmbeddingError::Http {
        message: format!("failed to download `{url}`: {source}"),
    })?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let mut reader = response.into_body().into_reader();
    let mut writer =
        BufWriter::new(
            File::create(&partial_path).map_err(|source| EmbeddingError::Io {
                context: format!("failed to create `{}`", partial_path.display()),
                source,
            })?,
        );
    let mut loaded = 0u64;
    if let Some(total) = total {
        progress(SetupProgress::determinate(
            SetupProgressPhase::DownloadingModel,
            "download",
            "Downloading model",
            "Downloading embedding model.",
            0,
            total,
        ));
    }
    let mut buf = [0u8; 1024 * 1024];
    loop {
        let read = reader.read(&mut buf).map_err(|source| EmbeddingError::Io {
            context: format!("failed to read `{url}`"),
            source,
        })?;
        if read == 0 {
            break;
        }
        writer
            .write_all(&buf[..read])
            .map_err(|source| EmbeddingError::Io {
                context: format!("failed to write `{}`", partial_path.display()),
                source,
            })?;
        if let Some(total) = total {
            loaded = loaded.saturating_add(read as u64);
            progress(SetupProgress::determinate(
                SetupProgressPhase::DownloadingModel,
                "download",
                "Downloading model",
                "Downloading embedding model.",
                loaded.min(total),
                total,
            ));
        }
    }
    writer.flush().map_err(|source| EmbeddingError::Io {
        context: format!("failed to flush `{}`", partial_path.display()),
        source,
    })?;
    fs::rename(&partial_path, path).map_err(|source| EmbeddingError::Io {
        context: format!(
            "failed to move `{}` to `{}`",
            partial_path.display(),
            path.display()
        ),
        source,
    })?;
    Ok(())
}

#[requires(true)]
#[ensures(true)]
pub fn build_embedding_pack<B: EmbeddingBackend>(
    backend: &mut B,
    dictionary: &Dictionary<'_>,
    cll_chunks: &[CllSearchChunk],
    index_root: &Path,
    spec: &EmbeddingModelSpec,
    force: bool,
) -> Result<SetupReport, EmbeddingError> {
    let mut progress = |_| {};
    build_embedding_pack_with_progress(
        backend,
        dictionary,
        cll_chunks,
        index_root,
        spec,
        force,
        &mut progress,
    )
}

#[requires(true)]
#[ensures(true)]
pub fn build_embedding_pack_with_progress<B: EmbeddingBackend>(
    backend: &mut B,
    dictionary: &Dictionary<'_>,
    cll_chunks: &[CllSearchChunk],
    index_root: &Path,
    spec: &EmbeddingModelSpec,
    force: bool,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<SetupReport, EmbeddingError> {
    let dimensions = backend.dimensions()?;
    if dimensions != spec.dimensions {
        return Err(EmbeddingError::DimensionMismatch {
            expected: spec.dimensions,
            actual: dimensions,
        });
    }
    let dictionary_fingerprint = dictionary_fingerprint(dictionary);
    let cll_fingerprint = cll_fingerprint(cll_chunks);
    let pack_id = deterministic_pack_id(
        &spec.input_format_version,
        &spec.model_revision,
        &dictionary_fingerprint,
        &cll_fingerprint,
    );
    let final_pack_root = pack_root(index_root, &spec.model_key, &pack_id);
    let dictionary_rows = dictionary.entries().len();
    let cll_rows = cll_chunks.len();
    let total_rows = (dictionary_rows + cll_rows) as u64;
    if final_pack_root.join("manifest.json").is_file() && !force {
        progress(SetupProgress::determinate(
            SetupProgressPhase::ReusingIndex,
            "index",
            "Reusing embedding index",
            "Using an existing compatible embedding vector pack.",
            total_rows,
            total_rows,
        ));
        write_catalog(index_root, spec, &pack_id)?;
        progress(SetupProgress::determinate(
            SetupProgressPhase::Complete,
            "complete",
            "Embedding setup complete",
            "Native embeddings are ready for semantic search.",
            total_rows,
            total_rows,
        ));
        return Ok(SetupReport {
            index_root: index_root.to_owned(),
            model_path: PathBuf::new(),
            pack_id,
            dictionary_rows,
            cll_rows,
            index_source: SetupIndexSource::Reused,
        });
    }
    let work_root = sibling_path_with_suffix(&final_pack_root, "tmp")?;
    let work_pack_root = native_work_pack_root(&work_root);
    let checkpoint_path = native_partial_checkpoint_path(&work_root);
    let mut checkpoint = load_compatible_native_partial_checkpoint(
        &work_root,
        &work_pack_root,
        spec,
        &pack_id,
        dimensions,
        &dictionary_fingerprint,
        dictionary_rows,
        &cll_fingerprint,
        cll_rows,
    );
    if checkpoint.is_none() {
        remove_dir_all_if_exists(&work_root)?;
        checkpoint = Some(initial_native_partial_checkpoint(
            spec,
            &pack_id,
            dimensions,
            &dictionary_fingerprint,
            dictionary_rows,
            &cll_fingerprint,
            cll_rows,
        ));
    }
    fs::create_dir_all(&work_pack_root).map_err(|source| EmbeddingError::Io {
        context: format!("failed to create `{}`", work_pack_root.display()),
        source,
    })?;
    let mut checkpoint = checkpoint.expect("checkpoint was initialized");
    write_native_partial_checkpoint(&checkpoint_path, &checkpoint)?;
    let reusable_rows = load_reusable_native_rows(index_root, &spec.model_key, dimensions);
    let mut completed_rows = 0u64;
    emit_indexing_progress("Indexing dictionary", completed_rows, total_rows, progress);

    let dictionary_corpus = write_dictionary_corpus(
        backend,
        dictionary,
        &work_pack_root,
        &checkpoint_path,
        &mut checkpoint,
        &pack_id,
        dimensions,
        &dictionary_fingerprint,
        reusable_rows.as_ref().map(|rows| &rows.dictionary),
        &mut completed_rows,
        total_rows,
        progress,
    )?;
    emit_indexing_progress("Indexing CLL", completed_rows, total_rows, progress);
    let cll_corpus = write_cll_corpus(
        backend,
        cll_chunks,
        &work_pack_root,
        &checkpoint_path,
        &mut checkpoint,
        &pack_id,
        dimensions,
        &cll_fingerprint,
        reusable_rows.as_ref().map(|rows| &rows.cll),
        &mut completed_rows,
        total_rows,
        progress,
    )?;
    progress(SetupProgress::indeterminate(
        SetupProgressPhase::WritingIndex,
        "write",
        "Writing embedding index",
        "Writing embedding vector pack metadata.",
    ));
    let manifest = EmbeddingPackManifest {
        schema_version: INDEX_SCHEMA_VERSION,
        model_key: spec.model_key.clone(),
        model_revision: spec.model_revision.clone(),
        pack_id: pack_id.clone(),
        input_format_version: spec.input_format_version.clone(),
        built_by: native_embedding_runtime(),
        dimensions,
        element_type: "f32le".to_owned(),
        normalized: true,
        distance: "dot".to_owned(),
        compatible_query_runtimes: vec![native_embedding_runtime()],
        corpora: vec![dictionary_corpus, cll_corpus],
    };
    write_json_file(&work_pack_root.join("manifest.json"), &manifest)?;
    validate_pack_dir(&work_pack_root)?;
    if final_pack_root.exists() {
        fs::remove_dir_all(&final_pack_root).map_err(|source| EmbeddingError::Io {
            context: format!("failed to remove `{}`", final_pack_root.display()),
            source,
        })?;
    }
    ensure_parent_dir(&final_pack_root)?;
    fs::rename(&work_pack_root, &final_pack_root).map_err(|source| EmbeddingError::Io {
        context: format!(
            "failed to publish `{}` as `{}`",
            work_pack_root.display(),
            final_pack_root.display()
        ),
        source,
    })?;
    write_catalog(index_root, spec, &pack_id)?;
    let _ = fs::remove_dir_all(&work_root);
    progress(SetupProgress::determinate(
        SetupProgressPhase::Complete,
        "complete",
        "Embedding setup complete",
        "Native embeddings are ready for semantic search.",
        total_rows,
        total_rows,
    ));
    Ok(SetupReport {
        index_root: index_root.to_owned(),
        model_path: PathBuf::new(),
        pack_id,
        dictionary_rows,
        cll_rows,
        index_source: SetupIndexSource::BuiltLocal,
    })
}

#[requires(!label.is_empty())]
#[requires(completed_rows <= total_rows)]
#[ensures(true)]
fn emit_indexing_progress(
    label: &str,
    completed_rows: u64,
    total_rows: u64,
    progress: &mut SetupProgressCallback<'_>,
) {
    if total_rows == 0 {
        progress(SetupProgress::indeterminate(
            SetupProgressPhase::Indexing,
            "index",
            label,
            "Preparing embedding index.",
        ));
        return;
    }
    progress(SetupProgress::determinate(
        SetupProgressPhase::Indexing,
        "index",
        label,
        &format!("{label}: {completed_rows}/{total_rows} rows."),
        completed_rows,
        total_rows,
    ));
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|corpus| corpus.corpus_id == VLACKU_CORPUS_ID) || ret.is_err())]
fn write_dictionary_corpus<B: EmbeddingBackend>(
    backend: &mut B,
    dictionary: &Dictionary<'_>,
    pack_dir: &Path,
    checkpoint_path: &Path,
    checkpoint: &mut NativePartialBuildCheckpoint,
    pack_id: &str,
    dimensions: usize,
    fingerprint: &str,
    reusable_rows: Option<&ReusableVectorRows>,
    completed_rows: &mut u64,
    total_rows: u64,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<CorpusManifest, EmbeddingError> {
    let rows = dictionary
        .entries()
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| {
            let input = dictionary_embedding_input(entry);
            let input_hash = sha256_hex_bytes(input.as_bytes());
            let item = DictionaryEmbeddingItem {
                entry_index,
                word: entry.word.to_owned(),
                definition_id: entry.definition_id.0,
                input_hash: input_hash.clone(),
                kind: dictionary_embedding_kind(entry),
            };
            EmbeddingBuildRow {
                item,
                input,
                input_hash,
            }
        })
        .collect::<Vec<_>>();
    write_chunked_corpus(
        backend,
        &rows,
        pack_dir,
        checkpoint_path,
        checkpoint,
        VLACKU_CORPUS_ID,
        "Indexing dictionary",
        dimensions,
        pack_id,
        fingerprint,
        reusable_rows,
        completed_rows,
        total_rows,
        progress,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|corpus| corpus.corpus_id == CUKTA_CORPUS_ID) || ret.is_err())]
fn write_cll_corpus<B: EmbeddingBackend>(
    backend: &mut B,
    chunks: &[CllSearchChunk],
    pack_dir: &Path,
    checkpoint_path: &Path,
    checkpoint: &mut NativePartialBuildCheckpoint,
    pack_id: &str,
    dimensions: usize,
    fingerprint: &str,
    reusable_rows: Option<&ReusableVectorRows>,
    completed_rows: &mut u64,
    total_rows: u64,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<CorpusManifest, EmbeddingError> {
    let rows = chunks
        .iter()
        .enumerate()
        .map(|(chunk_index, chunk)| {
            let input = cll_embedding_input(chunk);
            let input_hash = sha256_hex_bytes(input.as_bytes());
            let item = CllEmbeddingItem {
                chunk_index,
                input_hash: input_hash.clone(),
            };
            EmbeddingBuildRow {
                item,
                input,
                input_hash,
            }
        })
        .collect::<Vec<_>>();
    write_chunked_corpus(
        backend,
        &rows,
        pack_dir,
        checkpoint_path,
        checkpoint,
        CUKTA_CORPUS_ID,
        "Indexing CLL",
        dimensions,
        pack_id,
        fingerprint,
        reusable_rows,
        completed_rows,
        total_rows,
        progress,
    )
}

#[allow(clippy::too_many_arguments)]
#[requires(dimensions > 0)]
#[ensures(ret.as_ref().is_ok_and(|corpus| corpus.row_count == rows.len()) || ret.is_err())]
fn write_chunked_corpus<B, T>(
    backend: &mut B,
    rows: &[EmbeddingBuildRow<T>],
    pack_dir: &Path,
    checkpoint_path: &Path,
    checkpoint: &mut NativePartialBuildCheckpoint,
    corpus_id: &str,
    progress_label: &str,
    dimensions: usize,
    _pack_id: &str,
    fingerprint: &str,
    reusable_rows: Option<&ReusableVectorRows>,
    completed_rows: &mut u64,
    total_rows: u64,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<CorpusManifest, EmbeddingError>
where
    B: EmbeddingBackend,
    T: Clone + Serialize,
{
    let corpus_dir = pack_dir.join("corpora").join(corpus_id);
    let url_prefix = format!("corpora/{corpus_id}");
    let items_path = corpus_dir.join("items.json");
    let items = rows.iter().map(|row| row.item.clone()).collect::<Vec<_>>();
    write_json_file(&items_path, &items)?;
    let items_sha256 = sha256_hex_file(&items_path)?;
    let items_url = format!("{url_prefix}/items.json");
    let mut partial_shards = native_partial_shards_by_row_start(checkpoint, corpus_id);
    let initial_corpus = native_partial_corpus_from_shards(
        corpus_id,
        &checkpoint.input_format_version,
        fingerprint,
        rows.len(),
        dimensions,
        &items_url,
        &items_sha256,
        &partial_shards,
    );
    upsert_native_partial_corpus(checkpoint, initial_corpus);
    write_native_partial_checkpoint(checkpoint_path, checkpoint)?;

    let mut shards = Vec::new();
    for (chunk_index, chunk_rows) in rows.chunks(NATIVE_VECTOR_CHUNK_ROWS).enumerate() {
        let row_start = chunk_index * NATIVE_VECTOR_CHUNK_ROWS;
        let row_count = chunk_rows.len();
        if let Some(partial_shard) = partial_shards.get(&row_start)
            && partial_shard.row_count == row_count
        {
            let _ = read_native_partial_vector_shard(pack_dir, partial_shard, dimensions)?;
            shards.push(vector_shard_from_native_partial(partial_shard));
            *completed_rows = completed_rows.saturating_add(row_count as u64);
            emit_indexing_progress(progress_label, *completed_rows, total_rows, progress);
            continue;
        }

        let mut values = Vec::with_capacity(row_count * dimensions);
        for row in chunk_rows {
            if let Some(reusable) =
                reusable_rows.and_then(|rows| rows.row(&row.input_hash, dimensions))
            {
                values.extend_from_slice(reusable);
            } else {
                let mut embedding = backend.embed(&row.input)?.values;
                if embedding.len() != dimensions {
                    return Err(EmbeddingError::DimensionMismatch {
                        expected: dimensions,
                        actual: embedding.len(),
                    });
                }
                normalize_vector(&mut embedding);
                values.extend_from_slice(&embedding);
            }
            *completed_rows = completed_rows.saturating_add(1);
            emit_indexing_progress(progress_label, *completed_rows, total_rows, progress);
        }
        let file_name = native_vector_chunk_file_name(chunk_index);
        let shard = write_vector_chunk_file(
            &corpus_dir,
            &url_prefix,
            &file_name,
            values.as_slice(),
            dimensions,
        )?;
        let partial_shard = native_partial_shard_from_vector_shard(&shard, row_start, row_count);
        partial_shards.insert(row_start, partial_shard);
        shards.push(shard);
        let partial_corpus = native_partial_corpus_from_shards(
            corpus_id,
            &checkpoint.input_format_version,
            fingerprint,
            rows.len(),
            dimensions,
            &items_url,
            &items_sha256,
            &partial_shards,
        );
        upsert_native_partial_corpus(checkpoint, partial_corpus);
        write_native_partial_checkpoint(checkpoint_path, checkpoint)?;
    }
    Ok(CorpusManifest {
        corpus_id: corpus_id.to_owned(),
        input_format_version: checkpoint.input_format_version.clone(),
        fingerprint: fingerprint.to_owned(),
        row_count: rows.len(),
        dimensions,
        items_url,
        items_sha256,
        shards,
    })
}

#[requires(true)]
#[ensures(true)]
pub fn reuse_existing_embedding_pack_with_progress(
    dictionary: &Dictionary<'_>,
    cll_chunks: &[CllSearchChunk],
    index_root: &Path,
    spec: &EmbeddingModelSpec,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<Option<SetupReport>, EmbeddingError> {
    let dictionary_fingerprint = dictionary_fingerprint(dictionary);
    let cll_fingerprint = cll_fingerprint(cll_chunks);
    let pack_id = deterministic_pack_id(
        &spec.input_format_version,
        &spec.model_revision,
        &dictionary_fingerprint,
        &cll_fingerprint,
    );
    let pack_dir = pack_root(index_root, &spec.model_key, &pack_id);
    if !pack_dir.join("manifest.json").is_file() {
        return Ok(None);
    }
    progress(SetupProgress::indeterminate(
        SetupProgressPhase::ValidatingIndex,
        "validate",
        "Validating embedding index",
        "Checking existing embedding vector pack.",
    ));
    validate_pack_dir(&pack_dir)?;
    let manifest: EmbeddingPackManifest = read_json_file(&pack_dir.join("manifest.json"))?;
    validate_native_pack_manifest(
        &manifest,
        spec,
        &pack_id,
        &dictionary_fingerprint,
        dictionary.entries().len(),
        &cll_fingerprint,
        cll_chunks.len(),
    )?;
    write_catalog(index_root, spec, &pack_id)?;
    let total_rows = (dictionary.entries().len() + cll_chunks.len()) as u64;
    progress(SetupProgress::determinate(
        SetupProgressPhase::ReusingIndex,
        "index",
        "Reusing embedding index",
        "Using an existing compatible embedding vector pack.",
        total_rows,
        total_rows,
    ));
    progress(SetupProgress::determinate(
        SetupProgressPhase::Complete,
        "complete",
        "Embedding setup complete",
        "Native embeddings are ready for semantic search.",
        total_rows,
        total_rows,
    ));
    Ok(Some(SetupReport {
        index_root: index_root.to_owned(),
        model_path: PathBuf::new(),
        pack_id,
        dictionary_rows: dictionary.entries().len(),
        cll_rows: cll_chunks.len(),
        index_source: SetupIndexSource::Reused,
    }))
}

#[requires(!base_url.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|report| report.index_source == SetupIndexSource::DownloadedPrecomputed) || ret.is_err())]
pub fn download_precomputed_embedding_pack_with_progress(
    dictionary: &Dictionary<'_>,
    cll_chunks: &[CllSearchChunk],
    index_root: &Path,
    spec: &EmbeddingModelSpec,
    base_url: &str,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<SetupReport, EmbeddingError> {
    let dictionary_fingerprint = dictionary_fingerprint(dictionary);
    let cll_fingerprint = cll_fingerprint(cll_chunks);
    let expected_pack_id = deterministic_pack_id(
        &spec.input_format_version,
        &spec.model_revision,
        &dictionary_fingerprint,
        &cll_fingerprint,
    );
    let catalog_url = remote_pack_url(base_url, "catalog.json")?;
    progress(SetupProgress::indeterminate(
        SetupProgressPhase::DownloadingIndex,
        "download",
        "Downloading precomputed index",
        "Fetching native embedding catalog.",
    ));
    let catalog: EmbeddingCatalog = fetch_json_url(&catalog_url)?;
    if catalog.schema_version != INDEX_SCHEMA_VERSION {
        return Err(EmbeddingError::InvalidIndex {
            message: format!(
                "remote embedding catalog schema version {} is unsupported",
                catalog.schema_version
            ),
        });
    }
    let model = catalog
        .models
        .iter()
        .find(|model| model.model_key == spec.model_key)
        .ok_or_else(|| EmbeddingError::MissingCompatiblePack {
            model_key: spec.model_key.clone(),
        })?;
    let manifest_url = remote_pack_url(base_url, &model.manifest_url)?;
    progress(SetupProgress::indeterminate(
        SetupProgressPhase::DownloadingIndex,
        "download",
        "Downloading precomputed index",
        "Fetching native embedding vector pack manifest.",
    ));
    let manifest: EmbeddingPackManifest = fetch_json_url(&manifest_url)?;
    validate_native_pack_manifest(
        &manifest,
        spec,
        &expected_pack_id,
        &dictionary_fingerprint,
        dictionary.entries().len(),
        &cll_fingerprint,
        cll_chunks.len(),
    )?;
    if model.latest_pack_id != manifest.pack_id {
        return Err(EmbeddingError::InvalidIndex {
            message: format!(
                "remote catalog points to pack `{}`, but manifest has pack `{}`",
                model.latest_pack_id, manifest.pack_id
            ),
        });
    }

    let final_pack_root = pack_root(index_root, &spec.model_key, &manifest.pack_id);
    let work_pack_root = sibling_path_with_suffix(&final_pack_root, "downloadInProgress")?;
    remove_dir_all_if_exists(&work_pack_root)?;
    fs::create_dir_all(&work_pack_root).map_err(|source| EmbeddingError::Io {
        context: format!("failed to create `{}`", work_pack_root.display()),
        source,
    })?;
    write_json_file(&work_pack_root.join("manifest.json"), &manifest)?;
    let remote_paths = native_pack_remote_paths(&manifest)?;
    for relative_path in remote_paths {
        let url = remote_pack_child_url(base_url, &model.manifest_url, &relative_path)?;
        let destination = work_pack_root.join(&relative_path);
        download_url_to_file_with_progress(
            &url,
            &destination,
            SetupProgressPhase::DownloadingIndex,
            "Downloading precomputed index",
            &format!("Downloading {relative_path}."),
            progress,
        )?;
    }
    progress(SetupProgress::indeterminate(
        SetupProgressPhase::ValidatingIndex,
        "validate",
        "Validating embedding index",
        "Checking downloaded embedding vector pack.",
    ));
    validate_pack_dir(&work_pack_root)?;
    if final_pack_root.exists() {
        remove_dir_all_if_exists(&final_pack_root)?;
    }
    ensure_parent_dir(&final_pack_root)?;
    fs::rename(&work_pack_root, &final_pack_root).map_err(|source| EmbeddingError::Io {
        context: format!(
            "failed to publish `{}` as `{}`",
            work_pack_root.display(),
            final_pack_root.display()
        ),
        source,
    })?;
    write_catalog(index_root, spec, &manifest.pack_id)?;
    let total_rows = (dictionary.entries().len() + cll_chunks.len()) as u64;
    progress(SetupProgress::determinate(
        SetupProgressPhase::Complete,
        "complete",
        "Embedding setup complete",
        "Native embeddings are ready for semantic search.",
        total_rows,
        total_rows,
    ));
    Ok(SetupReport {
        index_root: index_root.to_owned(),
        model_path: PathBuf::new(),
        pack_id: manifest.pack_id,
        dictionary_rows: dictionary.entries().len(),
        cll_rows: cll_chunks.len(),
        index_source: SetupIndexSource::DownloadedPrecomputed,
    })
}

#[requires(!base_url.trim().is_empty())]
#[requires(!relative_path.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|url| url.starts_with(base_url.trim().trim_end_matches('/'))) || ret.is_err())]
fn remote_pack_url(base_url: &str, relative_path: &str) -> Result<String, EmbeddingError> {
    let _ = safe_pack_relative_path(relative_path)?;
    Ok(format!(
        "{}/{}",
        base_url.trim().trim_end_matches('/'),
        relative_path.trim_start_matches('/')
    ))
}

#[requires(!base_url.trim().is_empty())]
#[requires(!manifest_relative_path.is_empty())]
#[requires(!child_relative_path.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|url| url.starts_with(base_url.trim().trim_end_matches('/'))) || ret.is_err())]
fn remote_pack_child_url(
    base_url: &str,
    manifest_relative_path: &str,
    child_relative_path: &str,
) -> Result<String, EmbeddingError> {
    let _ = safe_pack_relative_path(manifest_relative_path)?;
    let _ = safe_pack_relative_path(child_relative_path)?;
    let child_relative_path = child_relative_path.trim_start_matches('/');
    let relative_path = if let Some((manifest_parent, _)) = manifest_relative_path.rsplit_once('/')
    {
        if manifest_parent.is_empty() {
            child_relative_path.to_owned()
        } else {
            format!("{manifest_parent}/{child_relative_path}")
        }
    } else {
        child_relative_path.to_owned()
    };
    remote_pack_url(base_url, &relative_path)
}

#[requires(!value.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|path| !path.as_os_str().is_empty()) || ret.is_err())]
fn safe_pack_relative_path(value: &str) -> Result<PathBuf, EmbeddingError> {
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(EmbeddingError::InvalidIndex {
            message: format!("remote pack path `{value}` must be relative"),
        });
    }
    if path
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(EmbeddingError::InvalidIndex {
            message: format!("remote pack path `{value}` is not a normalized relative path"),
        });
    }
    Ok(path.to_owned())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|paths| !paths.contains(&"manifest.json".to_owned())) || ret.is_err())]
fn native_pack_remote_paths(
    manifest: &EmbeddingPackManifest,
) -> Result<Vec<String>, EmbeddingError> {
    let mut paths = BTreeSet::new();
    for corpus in &manifest.corpora {
        let items_path = safe_pack_relative_path(&corpus.items_url)?;
        paths.insert(items_path.to_string_lossy().into_owned());
        for shard in &corpus.shards {
            let shard_path = safe_pack_relative_path(&shard.url)?;
            paths.insert(shard_path.to_string_lossy().into_owned());
        }
    }
    Ok(paths.into_iter().collect())
}

#[allow(clippy::too_many_arguments)]
#[requires(!expected_pack_id.is_empty())]
#[requires(spec.dimensions > 0)]
#[ensures(true)]
fn validate_native_pack_manifest(
    manifest: &EmbeddingPackManifest,
    spec: &EmbeddingModelSpec,
    expected_pack_id: &str,
    dictionary_fingerprint: &str,
    dictionary_rows: usize,
    cll_fingerprint: &str,
    cll_rows: usize,
) -> Result<(), EmbeddingError> {
    if manifest.schema_version != INDEX_SCHEMA_VERSION {
        return Err(EmbeddingError::InvalidIndex {
            message: format!(
                "embedding pack schema version {} is unsupported",
                manifest.schema_version
            ),
        });
    }
    if manifest.model_key != spec.model_key || manifest.model_revision != spec.model_revision {
        return Err(EmbeddingError::InvalidIndex {
            message: format!(
                "embedding pack is for `{}` revision `{}`, expected `{}` revision `{}`",
                manifest.model_key, manifest.model_revision, spec.model_key, spec.model_revision
            ),
        });
    }
    if manifest.pack_id != expected_pack_id {
        return Err(EmbeddingError::InvalidIndex {
            message: format!(
                "embedding pack id `{}` does not match expected `{expected_pack_id}`",
                manifest.pack_id
            ),
        });
    }
    if manifest.input_format_version != spec.input_format_version {
        return Err(EmbeddingError::InvalidIndex {
            message: format!(
                "embedding pack input format `{}` is unsupported for `{}`",
                manifest.input_format_version, spec.model_key
            ),
        });
    }
    if manifest.dimensions != spec.dimensions {
        return Err(EmbeddingError::DimensionMismatch {
            expected: spec.dimensions,
            actual: manifest.dimensions,
        });
    }
    if manifest.element_type != "f32le" || !manifest.normalized || manifest.distance != "dot" {
        return Err(EmbeddingError::InvalidIndex {
            message: "embedding pack must contain normalized f32le dot-product vectors".to_owned(),
        });
    }
    if !manifest
        .compatible_query_runtimes
        .iter()
        .any(|runtime| runtime == &native_embedding_runtime())
    {
        return Err(EmbeddingError::MissingCompatiblePack {
            model_key: spec.model_key.clone(),
        });
    }
    let dictionary = manifest_corpus(manifest, VLACKU_CORPUS_ID)?;
    validate_native_corpus_manifest(
        dictionary,
        dictionary_fingerprint,
        dictionary_rows,
        spec.dimensions,
        &spec.input_format_version,
    )?;
    let cll = manifest_corpus(manifest, CUKTA_CORPUS_ID)?;
    validate_native_corpus_manifest(
        cll,
        cll_fingerprint,
        cll_rows,
        spec.dimensions,
        &spec.input_format_version,
    )?;
    Ok(())
}

#[requires(dimensions > 0)]
#[requires(!input_format_version.is_empty())]
#[ensures(true)]
fn validate_native_corpus_manifest(
    corpus: &CorpusManifest,
    fingerprint: &str,
    rows: usize,
    dimensions: usize,
    input_format_version: &str,
) -> Result<(), EmbeddingError> {
    if corpus.input_format_version != input_format_version {
        return Err(EmbeddingError::InvalidIndex {
            message: format!(
                "corpus `{}` input format `{}` is unsupported",
                corpus.corpus_id, corpus.input_format_version
            ),
        });
    }
    if corpus.fingerprint != fingerprint {
        return Err(EmbeddingError::InvalidIndex {
            message: format!("corpus `{}` fingerprint mismatch", corpus.corpus_id),
        });
    }
    if corpus.row_count != rows {
        return Err(EmbeddingError::InvalidIndex {
            message: format!(
                "corpus `{}` has {} rows, expected {rows}",
                corpus.corpus_id, corpus.row_count
            ),
        });
    }
    if corpus.dimensions != dimensions {
        return Err(EmbeddingError::DimensionMismatch {
            expected: dimensions,
            actual: corpus.dimensions,
        });
    }
    let _ = safe_pack_relative_path(&corpus.items_url)?;
    for shard in &corpus.shards {
        let _ = safe_pack_relative_path(&shard.url)?;
    }
    Ok(())
}

#[requires(!url.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn fetch_json_url<T: DeserializeOwned>(url: &str) -> Result<T, EmbeddingError> {
    let response = ureq::get(url)
        .call()
        .map_err(|source| EmbeddingError::Http {
            message: format!("failed to fetch `{url}`: {source}"),
        })?;
    serde_json::from_reader(response.into_body().into_reader()).map_err(|source| {
        EmbeddingError::Json {
            context: format!("failed to parse `{url}`"),
            source,
        }
    })
}

#[requires(!url.is_empty())]
#[requires(!destination.as_os_str().is_empty())]
#[requires(!label.is_empty())]
#[requires(!detail.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|_| destination.is_file()) || ret.is_err())]
fn download_url_to_file_with_progress(
    url: &str,
    destination: &Path,
    phase: SetupProgressPhase,
    label: &str,
    detail: &str,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<(), EmbeddingError> {
    ensure_parent_dir(destination)?;
    progress(SetupProgress::indeterminate(
        phase, "download", label, detail,
    ));
    let response = ureq::get(url)
        .call()
        .map_err(|source| EmbeddingError::Http {
            message: format!("failed to download `{url}`: {source}"),
        })?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let temp_path = sibling_path_with_suffix(destination, "downloadInProgress")?;
    let mut reader = response.into_body().into_reader();
    let mut writer =
        BufWriter::new(
            File::create(&temp_path).map_err(|source| EmbeddingError::Io {
                context: format!("failed to create `{}`", temp_path.display()),
                source,
            })?,
        );
    let mut loaded = 0u64;
    if let Some(total) = total {
        progress(SetupProgress::determinate(
            phase, "download", label, detail, 0, total,
        ));
    }
    let mut buf = [0u8; 1024 * 1024];
    loop {
        let read = reader.read(&mut buf).map_err(|source| EmbeddingError::Io {
            context: format!("failed to read `{url}`"),
            source,
        })?;
        if read == 0 {
            break;
        }
        writer
            .write_all(&buf[..read])
            .map_err(|source| EmbeddingError::Io {
                context: format!("failed to write `{}`", temp_path.display()),
                source,
            })?;
        if let Some(total) = total {
            loaded = loaded.saturating_add(read as u64);
            progress(SetupProgress::determinate(
                phase,
                "download",
                label,
                detail,
                loaded.min(total),
                total,
            ));
        }
    }
    writer.flush().map_err(|source| EmbeddingError::Io {
        context: format!("failed to flush `{}`", temp_path.display()),
        source,
    })?;
    rename_replacing(&temp_path, destination)
}

#[requires(true)]
#[ensures(true)]
fn write_catalog(
    index_root: &Path,
    spec: &EmbeddingModelSpec,
    pack_id: &str,
) -> Result<(), EmbeddingError> {
    let path = catalog_path(index_root)?;
    let mut catalog = if path.is_file() {
        read_json_file::<EmbeddingCatalog>(&path)?
    } else {
        EmbeddingCatalog {
            schema_version: INDEX_SCHEMA_VERSION,
            models: Vec::new(),
        }
    };
    let manifest_url = format!("models/{}/packs/{}/manifest.json", spec.model_key, pack_id);
    if let Some(model) = catalog
        .models
        .iter_mut()
        .find(|model| model.model_key == spec.model_key)
    {
        model.latest_pack_id = pack_id.to_owned();
        model.manifest_url = manifest_url;
    } else {
        catalog.models.push(EmbeddingCatalogModel {
            model_key: spec.model_key.clone(),
            latest_pack_id: pack_id.to_owned(),
            manifest_url,
        });
    }
    write_json_file(&path, &catalog)
}

#[requires(path.is_dir())]
#[ensures(true)]
pub fn validate_pack_dir(path: &Path) -> Result<(), EmbeddingError> {
    let manifest_path = path.join("manifest.json");
    require_index_file(&manifest_path, "embedding pack manifest")?;
    let manifest: EmbeddingPackManifest = read_json_file(&manifest_path)?;
    if manifest.schema_version != INDEX_SCHEMA_VERSION {
        return Err(EmbeddingError::InvalidIndex {
            message: format!(
                "unsupported index schema version {}",
                manifest.schema_version
            ),
        });
    }
    for corpus in &manifest.corpora {
        let items_path = path.join(&corpus.items_url);
        require_index_file(&items_path, "embedding corpus items file")?;
        let items_sha256 = sha256_hex_file(&items_path)?;
        if items_sha256 != corpus.items_sha256 {
            return Err(EmbeddingError::InvalidIndex {
                message: format!("items file `{}` SHA-256 mismatch", items_path.display()),
            });
        }
        let _ = read_vector_shards(path, corpus, manifest.dimensions)?;
    }
    Ok(())
}

#[requires(!model_key.is_empty())]
#[ensures(true)]
pub fn load_latest_pack(
    index_root: &Path,
    model_key: &str,
) -> Result<(PathBuf, EmbeddingPackManifest), EmbeddingError> {
    let catalog_path = catalog_path(index_root)?;
    if !catalog_path.is_file() {
        return Err(EmbeddingError::MissingCompatiblePack {
            model_key: model_key.to_owned(),
        });
    }
    let catalog: EmbeddingCatalog = read_json_file(&catalog_path)?;
    let model = catalog
        .models
        .iter()
        .find(|model| model.model_key == model_key)
        .ok_or_else(|| EmbeddingError::MissingCompatiblePack {
            model_key: model_key.to_owned(),
        })?;
    let pack_dir = index_root
        .join(INDEX_BASE_VERSION)
        .join(&model.manifest_url);
    let pack_dir = pack_dir
        .parent()
        .ok_or_else(|| EmbeddingError::InvalidIndex {
            message: format!("manifest URL `{}` has no parent", model.manifest_url),
        })?;
    let manifest_path = pack_dir.join("manifest.json");
    require_index_file(&manifest_path, "embedding pack manifest")?;
    let manifest: EmbeddingPackManifest = read_json_file(&manifest_path)?;
    if !manifest
        .compatible_query_runtimes
        .iter()
        .any(|runtime| runtime.runtime == "llama-cpp-4")
    {
        return Err(EmbeddingError::MissingCompatiblePack {
            model_key: model_key.to_owned(),
        });
    }
    Ok((pack_dir.to_owned(), manifest))
}

#[requires(dimensions > 0)]
#[ensures(true)]
fn load_reusable_native_rows(
    index_root: &Path,
    model_key: &str,
    dimensions: usize,
) -> Option<ReusablePackRows> {
    if !catalog_path(index_root).ok()?.is_file() {
        return None;
    }
    let (pack_dir, manifest) = load_latest_pack(index_root, model_key).ok()?;
    if manifest.dimensions != dimensions {
        return None;
    }
    let dictionary = manifest_corpus(&manifest, VLACKU_CORPUS_ID)
        .ok()
        .and_then(|corpus| {
            load_reusable_corpus_rows::<DictionaryEmbeddingItem, _>(
                &pack_dir,
                corpus,
                dimensions,
                |item| item.input_hash.as_str(),
            )
            .ok()
        })
        .unwrap_or_default();
    let cll = manifest_corpus(&manifest, CUKTA_CORPUS_ID)
        .ok()
        .and_then(|corpus| {
            load_reusable_corpus_rows::<CllEmbeddingItem, _>(
                &pack_dir,
                corpus,
                dimensions,
                |item| item.input_hash.as_str(),
            )
            .ok()
        })
        .unwrap_or_default();
    Some(ReusablePackRows { dictionary, cll })
}

#[requires(dimensions > 0)]
#[ensures(true)]
fn load_reusable_corpus_rows<T, F>(
    pack_dir: &Path,
    corpus: &CorpusManifest,
    dimensions: usize,
    input_hash: F,
) -> Result<ReusableVectorRows, EmbeddingError>
where
    T: DeserializeOwned,
    F: Fn(&T) -> &str,
{
    let items: Vec<T> = read_json_file(&pack_dir.join(&corpus.items_url))?;
    let values = read_vector_shards(pack_dir, corpus, dimensions)?;
    let mut rows_by_input_hash = HashMap::new();
    for (row_index, item) in items.iter().enumerate() {
        let start = row_index * dimensions;
        let end = start + dimensions;
        if end <= values.len() {
            rows_by_input_hash
                .entry(input_hash(item).to_owned())
                .or_insert_with(|| values[start..end].to_vec());
        }
    }
    Ok(ReusableVectorRows { rows_by_input_hash })
}

#[requires(true)]
#[ensures(true)]
fn dictionary_corpus_cache()
-> &'static Mutex<HashMap<LoadedCorpusCacheKey, Arc<LoadedCorpus<DictionaryEmbeddingItem>>>> {
    DICTIONARY_CORPUS_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[requires(true)]
#[ensures(true)]
fn cll_corpus_cache()
-> &'static Mutex<HashMap<LoadedCorpusCacheKey, Arc<LoadedCorpus<CllEmbeddingItem>>>> {
    CLL_CORPUS_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|corpus| corpus.dimensions == manifest.dimensions) || ret.is_err())]
fn load_cached_dictionary_corpus(
    pack_dir: &Path,
    manifest: &EmbeddingPackManifest,
    corpus: &CorpusManifest,
) -> Result<Arc<LoadedCorpus<DictionaryEmbeddingItem>>, EmbeddingError> {
    load_cached_corpus(dictionary_corpus_cache(), pack_dir, manifest, corpus)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|corpus| corpus.dimensions == manifest.dimensions) || ret.is_err())]
fn load_cached_cll_corpus(
    pack_dir: &Path,
    manifest: &EmbeddingPackManifest,
    corpus: &CorpusManifest,
) -> Result<Arc<LoadedCorpus<CllEmbeddingItem>>, EmbeddingError> {
    load_cached_corpus(cll_corpus_cache(), pack_dir, manifest, corpus)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|corpus| corpus.dimensions == manifest.dimensions) || ret.is_err())]
fn load_cached_corpus<T>(
    cache: &Mutex<HashMap<LoadedCorpusCacheKey, Arc<LoadedCorpus<T>>>>,
    pack_dir: &Path,
    manifest: &EmbeddingPackManifest,
    corpus: &CorpusManifest,
) -> Result<Arc<LoadedCorpus<T>>, EmbeddingError>
where
    T: DeserializeOwned,
{
    let key = loaded_corpus_cache_key(pack_dir, manifest, corpus);
    if let Some(cached) = cache
        .lock()
        .map_err(|_| EmbeddingError::InvalidIndex {
            message: "embedding corpus cache lock is poisoned".to_owned(),
        })?
        .get(&key)
        .cloned()
    {
        return Ok(cached);
    }

    let loaded = Arc::new(load_corpus_from_disk(
        pack_dir,
        corpus,
        manifest.dimensions,
    )?);
    let mut cache = cache.lock().map_err(|_| EmbeddingError::InvalidIndex {
        message: "embedding corpus cache lock is poisoned".to_owned(),
    })?;
    Ok(Arc::clone(
        cache.entry(key).or_insert_with(|| Arc::clone(&loaded)),
    ))
}

#[requires(true)]
#[ensures(ret.dimensions == manifest.dimensions)]
fn loaded_corpus_cache_key(
    pack_dir: &Path,
    manifest: &EmbeddingPackManifest,
    corpus: &CorpusManifest,
) -> LoadedCorpusCacheKey {
    LoadedCorpusCacheKey {
        pack_dir: pack_dir.to_owned(),
        model_key: manifest.model_key.clone(),
        pack_id: manifest.pack_id.clone(),
        corpus_id: corpus.corpus_id.clone(),
        items_sha256: corpus.items_sha256.clone(),
        vector_shards_hash: vector_shard_manifest_fingerprint(&corpus.shards),
        row_count: corpus.row_count,
        dimensions: manifest.dimensions,
    }
}

#[requires(true)]
#[ensures(ret.len() == 64)]
fn vector_shard_manifest_fingerprint(shards: &[VectorShardManifest]) -> String {
    let mut hasher = Sha256::new();
    for shard in shards {
        hasher.update(shard.url.as_bytes());
        hasher.update([0]);
        hasher.update(shard.byte_len.to_le_bytes());
        hasher.update([0]);
        hasher.update(shard.sha256.as_bytes());
        hasher.update([0]);
    }
    hex_digest(hasher.finalize())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|corpus| corpus.dimensions == dimensions) || ret.is_err())]
fn load_corpus_from_disk<T>(
    pack_dir: &Path,
    corpus: &CorpusManifest,
    dimensions: usize,
) -> Result<LoadedCorpus<T>, EmbeddingError>
where
    T: DeserializeOwned,
{
    let items_path = pack_dir.join(&corpus.items_url);
    require_index_file(&items_path, "embedding corpus items file")?;
    let items: Vec<T> = read_json_file(&items_path)?;
    if sha256_hex_file(&items_path)? != corpus.items_sha256 {
        return Err(EmbeddingError::InvalidIndex {
            message: format!("items file `{}` SHA-256 mismatch", items_path.display()),
        });
    }
    if items.len() != corpus.row_count {
        return Err(EmbeddingError::InvalidIndex {
            message: format!(
                "items file `{}` has {} rows, expected {}",
                items_path.display(),
                items.len(),
                corpus.row_count
            ),
        });
    }
    let values = read_vector_shards(pack_dir, corpus, dimensions)?;
    Ok(LoadedCorpus {
        items,
        values,
        row_count: corpus.row_count,
        dimensions,
    })
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_vlacku_hits<B: EmbeddingBackend>(
    backend: &mut B,
    query: &str,
    count: usize,
    index_root: &Path,
    model_key: &str,
) -> Result<Vec<DictionarySemanticHit>, EmbeddingError> {
    let (pack_dir, manifest) = load_latest_pack(index_root, model_key)?;
    let corpus = manifest_corpus(&manifest, VLACKU_CORPUS_ID)?;
    let loaded = load_cached_dictionary_corpus(&pack_dir, &manifest, corpus)?;
    let mut query_embedding = backend.embed(&build_retrieval_query_input(query))?.values;
    normalize_vector(&mut query_embedding);
    let hits = top_vector_hits(
        &loaded.values,
        loaded.dimensions,
        &query_embedding,
        loaded.row_count,
        count.max(1),
    );
    Ok(hits
        .into_iter()
        .filter_map(|hit| {
            let item = loaded.items.get(hit.row_index)?;
            Some(DictionarySemanticHit {
                entry_index: item.entry_index,
                score: hit.score,
            })
        })
        .collect())
}

#[requires(true)]
#[ensures(true)]
pub fn semantic_cukta_output<B: EmbeddingBackend>(
    backend: &mut B,
    chunks: &[CllSearchChunk],
    query: &str,
    count: usize,
    targets: CuktaTargetFilter,
    index_root: &Path,
    model_key: &str,
) -> Result<CuktaSearchOutput, EmbeddingError> {
    let count = clamp_cukta_result_count(count);
    if !targets.sections && !targets.paragraphs && !targets.examples {
        return Ok(CuktaSearchOutput {
            mode: CuktaSearchMode::Meaning,
            query: query.to_owned(),
            count,
            matches: Vec::new(),
            message: Some("Select at least one search target.".to_owned()),
            has_more: false,
        });
    }
    let (pack_dir, manifest) = load_latest_pack(index_root, model_key)?;
    let corpus = manifest_corpus(&manifest, CUKTA_CORPUS_ID)?;
    let loaded = load_cached_cll_corpus(&pack_dir, &manifest, corpus)?;
    let mut query_embedding = backend.embed(&build_retrieval_query_input(query))?.values;
    normalize_vector(&mut query_embedding);
    let hit_limit = count.saturating_add(1).min(loaded.row_count);
    let hits = top_vector_hits_by_row(
        &loaded.values,
        loaded.dimensions,
        &query_embedding,
        loaded.row_count,
        hit_limit,
        |row_index| {
            loaded
                .items
                .get(row_index)
                .and_then(|item| chunks.get(item.chunk_index))
                .is_some_and(|chunk| chunk_allowed(chunk, targets))
        },
    );
    let mut matches = Vec::new();
    for hit in hits {
        let Some(item) = loaded.items.get(hit.row_index) else {
            continue;
        };
        let Some(chunk) = chunks.get(item.chunk_index) else {
            continue;
        };
        if !chunk_allowed(chunk, targets) {
            continue;
        }
        matches.push(CllSearchMatch {
            rank: matches.len() + 1,
            similarity: Some(hit.score),
            chunk: chunk.clone(),
        });
        if matches.len() > count {
            break;
        }
    }
    let has_more = matches.len() > count;
    matches.truncate(count);
    let message = matches.is_empty().then(|| "No matches found.".to_owned());
    Ok(CuktaSearchOutput {
        mode: CuktaSearchMode::Meaning,
        query: query.to_owned(),
        count,
        matches,
        message,
        has_more,
    })
}

#[requires(true)]
#[ensures(true)]
fn chunk_allowed(chunk: &CllSearchChunk, targets: CuktaTargetFilter) -> bool {
    match chunk.kind {
        jbotci_cll::CllSearchChunkKind::Section => targets.sections,
        jbotci_cll::CllSearchChunkKind::Paragraph => targets.paragraphs,
        jbotci_cll::CllSearchChunkKind::Example => targets.examples,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|corpus| corpus.corpus_id == corpus_id) || ret.is_err())]
fn manifest_corpus<'a>(
    manifest: &'a EmbeddingPackManifest,
    corpus_id: &str,
) -> Result<&'a CorpusManifest, EmbeddingError> {
    manifest
        .corpora
        .iter()
        .find(|corpus| corpus.corpus_id == corpus_id)
        .ok_or_else(|| EmbeddingError::InvalidIndex {
            message: format!("manifest is missing corpus `{corpus_id}`"),
        })
}

#[requires(true)]
#[ensures(true)]
pub fn setup_embeddings_with_backend<B: EmbeddingBackend>(
    backend: &mut B,
    options: &SetupOptions,
) -> Result<SetupReport, EmbeddingError> {
    let mut progress = |_| {};
    setup_embeddings_with_backend_and_progress(backend, options, &mut progress)
}

#[requires(true)]
#[ensures(true)]
pub fn setup_embeddings_with_backend_and_progress<B: EmbeddingBackend>(
    backend: &mut B,
    options: &SetupOptions,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<SetupReport, EmbeddingError> {
    let spec = model_spec(&options.model_key).ok_or_else(|| EmbeddingError::UnsupportedModel {
        model_key: options.model_key.clone(),
    })?;
    let index_root = options
        .index_dir
        .clone()
        .map(Ok)
        .unwrap_or_else(default_index_root)?;
    let dictionary = jbotci_dictionary_data::english();
    let cll_site =
        jbotci_cll::embedded_cll_site().map_err(|error| EmbeddingError::InvalidIndex {
            message: error.to_string(),
        })?;
    build_embedding_pack_with_progress(
        backend,
        dictionary,
        cll_search_all_chunks(cll_site),
        &index_root,
        &spec,
        options.force,
        progress,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{contract_trait, ensures, requires};

    #[derive(Debug)]
    #[invariant(true)]
    struct FakeBackend {
        dimensions: usize,
        calls: usize,
    }

    #[derive(Debug)]
    #[invariant(true)]
    struct FailingBackend {
        dimensions: usize,
        calls: usize,
        fail_after: usize,
    }

    #[requires(dimensions > 0)]
    #[requires(!input.is_empty())]
    #[ensures(ret.len() == dimensions)]
    fn fake_embedding_values(input: &str, dimensions: usize) -> Vec<f32> {
        let mut values = (0..dimensions)
            .map(|index| {
                let byte = input.as_bytes()[index % input.len()];
                f32::from(byte) + index as f32
            })
            .collect::<Vec<_>>();
        normalize_vector(&mut values);
        values
    }

    #[contract_trait]
    impl EmbeddingBackend for FakeBackend {
        #[requires(true)]
        #[ensures(ret.as_ref().is_ok_and(|dimensions| *dimensions > 0) || ret.is_err())]
        fn dimensions(&self) -> Result<usize, EmbeddingError> {
            Ok(self.dimensions)
        }

        #[requires(!input.is_empty())]
        #[ensures(ret.as_ref().is_ok_and(|embedding| embedding.values.len() == self.dimensions) || ret.is_err())]
        fn embed(&mut self, input: &str) -> Result<QueryEmbedding, EmbeddingError> {
            self.calls += 1;
            let values = fake_embedding_values(input, self.dimensions);
            Ok(QueryEmbedding { values })
        }
    }

    #[contract_trait]
    impl EmbeddingBackend for FailingBackend {
        #[requires(true)]
        #[ensures(ret.as_ref().is_ok_and(|dimensions| *dimensions > 0) || ret.is_err())]
        fn dimensions(&self) -> Result<usize, EmbeddingError> {
            Ok(self.dimensions)
        }

        #[requires(!input.is_empty())]
        #[ensures(ret.as_ref().is_ok_and(|embedding| embedding.values.len() == self.dimensions) || ret.is_err())]
        fn embed(&mut self, input: &str) -> Result<QueryEmbedding, EmbeddingError> {
            if self.calls >= self.fail_after {
                return Err(EmbeddingError::Backend {
                    message: format!("intentional embedding failure after {}", self.fail_after),
                });
            }
            self.calls += 1;
            let values = fake_embedding_values(input, self.dimensions);
            Ok(QueryEmbedding { values })
        }
    }

    #[requires(true)]
    #[ensures(ret.dimensions == 4)]
    fn test_embedding_spec() -> EmbeddingModelSpec {
        EmbeddingModelSpec {
            dimensions: 4,
            ..EmbeddingModelSpec::default_f2llm()
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|path| !path.as_os_str().is_empty()) || ret.is_err())]
    fn test_work_root(
        index_root: &Path,
        spec: &EmbeddingModelSpec,
        dictionary: &Dictionary<'_>,
        cll_chunks: &[CllSearchChunk],
    ) -> Result<PathBuf, EmbeddingError> {
        let pack_id = deterministic_pack_id(
            &spec.input_format_version,
            &spec.model_revision,
            &dictionary_fingerprint(dictionary),
            &cll_fingerprint(cll_chunks),
        );
        sibling_path_with_suffix(&pack_root(index_root, &spec.model_key, &pack_id), "tmp")
    }

    #[requires(true)]
    #[ensures(true)]
    fn first_positive_index_progress(progress: &[SetupProgress]) -> Option<u64> {
        progress
            .iter()
            .filter(|progress| progress.phase == SetupProgressPhase::Indexing)
            .filter_map(|progress| progress.loaded)
            .find(|loaded| *loaded > 0)
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn retrieval_prefixes_match_f2llm() {
        assert_eq!(
            build_retrieval_query_input("klama"),
            "Instruct: Given a question, retrieve passages that can help answer the question.\nQuery: klama"
        );
        assert_eq!(
            build_retrieval_document_input("goer", "klama"),
            "title: klama | text: goer"
        );
        assert_eq!(
            build_retrieval_document_input("goer", " "),
            "title: none | text: goer"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn default_native_model_url_uses_pinned_gguf_revision() {
        let spec = EmbeddingModelSpec::default_f2llm();
        assert_eq!(spec.model_revision, DEFAULT_MODEL_REVISION);
        assert_eq!(spec.input_format_version, DEFAULT_INPUT_FORMAT_VERSION);
        assert_ne!(spec.native_hf_revision, spec.model_revision);
        assert_eq!(
            model_download_url(&spec),
            "https://huggingface.co/mradermacher/F2LLM-v2-330M-GGUF/resolve/03158c3a78ea1c7a7eea2d6829c49e3f1d63f85f/F2LLM-v2-330M.Q4_K_M.gguf"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn native_model_table_supports_web_size_family() {
        let cases = [
            ("f2llm-v2-80m-q4-k-m-320", 320, 79_111_968),
            ("f2llm-v2-160m-q4-k-m-640", 640, 151_808_704),
            (DEFAULT_MODEL_KEY, DEFAULT_MODEL_DIMENSIONS, 286_198_400),
            ("f2llm-v2-0.6b-q4-k-m-1024", 1024, 396_706_560),
        ];
        for (model_key, dimensions, size_bytes) in cases {
            let spec = model_spec(model_key).expect("native model spec");
            assert_eq!(spec.model_key, model_key);
            assert_eq!(spec.dimensions, dimensions);
            assert_eq!(spec.native_size_bytes, size_bytes);
            assert!(spec.input_format_version.contains("q4-k-m"));
            assert_eq!(spec.web_dtype, "q4");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn temporary_sibling_paths_preserve_dotted_pack_ids() {
        let path = Path::new("models/f2llm-v2-0.6b-q4-k-m-1024/packs/f2llm-v2-0.6b-pack");
        let sibling = sibling_path_with_suffix(path, "tmp").expect("sibling path");
        assert_eq!(
            sibling,
            PathBuf::from("models/f2llm-v2-0.6b-q4-k-m-1024/packs/f2llm-v2-0.6b-pack.tmp")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn progress_percent_handles_empty_and_complete_totals() {
        assert_eq!(progress_percent(0, 0), None);
        assert_eq!(progress_percent(25, 100), Some(25));
        assert_eq!(progress_percent(100, 100), Some(100));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn missing_catalog_returns_missing_compatible_pack() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = load_latest_pack(dir.path(), DEFAULT_MODEL_KEY).expect_err("missing catalog");
        assert!(matches!(
            error,
            EmbeddingError::MissingCompatiblePack { .. }
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn catalog_pointing_to_missing_manifest_returns_invalid_index() {
        let dir = tempfile::tempdir().expect("tempdir");
        let spec = EmbeddingModelSpec::default_f2llm();
        write_catalog(dir.path(), &spec, "missing-pack").expect("catalog");

        let error = load_latest_pack(dir.path(), &spec.model_key).expect_err("missing manifest");
        assert!(matches!(error, EmbeddingError::InvalidIndex { .. }));
        assert!(error.to_string().contains("manifest"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_pack_missing_items_or_shards_returns_pathful_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pack_dir = dir.path().join("pack");
        std::fs::create_dir_all(&pack_dir).expect("pack dir");
        let missing_items_manifest = EmbeddingPackManifest {
            schema_version: INDEX_SCHEMA_VERSION,
            model_key: DEFAULT_MODEL_KEY.to_owned(),
            model_revision: DEFAULT_MODEL_REVISION.to_owned(),
            pack_id: "pack".to_owned(),
            input_format_version: DEFAULT_INPUT_FORMAT_VERSION.to_owned(),
            built_by: EmbeddingRuntime {
                runtime: "test".to_owned(),
                version: "test".to_owned(),
            },
            dimensions: 4,
            element_type: "f32le".to_owned(),
            normalized: true,
            distance: "dot".to_owned(),
            compatible_query_runtimes: vec![EmbeddingRuntime {
                runtime: "llama-cpp-4".to_owned(),
                version: LLAMA_CPP_4_RUNTIME_VERSION.to_owned(),
            }],
            corpora: vec![CorpusManifest {
                corpus_id: VLACKU_CORPUS_ID.to_owned(),
                input_format_version: DEFAULT_INPUT_FORMAT_VERSION.to_owned(),
                fingerprint: "f".repeat(64),
                row_count: 0,
                dimensions: 4,
                items_url: "corpora/vlacku-en/items.json".to_owned(),
                items_sha256: "0".repeat(64),
                shards: Vec::new(),
            }],
        };
        write_json_file(&pack_dir.join("manifest.json"), &missing_items_manifest)
            .expect("missing items manifest");
        let error = validate_pack_dir(&pack_dir).expect_err("missing items");
        assert!(matches!(error, EmbeddingError::InvalidIndex { .. }));
        assert!(error.to_string().contains("items.json"));

        let items_dir = pack_dir.join("corpora").join(VLACKU_CORPUS_ID);
        std::fs::create_dir_all(&items_dir).expect("items dir");
        let items: Vec<DictionaryEmbeddingItem> = Vec::new();
        let items_path = items_dir.join("items.json");
        write_json_file(&items_path, &items).expect("items");
        let items_sha256 = sha256_hex_file(&items_path).expect("items sha");
        let missing_shard_manifest = EmbeddingPackManifest {
            corpora: vec![CorpusManifest {
                items_sha256,
                shards: vec![VectorShardManifest {
                    url: format!("corpora/{VLACKU_CORPUS_ID}/vectors-0000.f32"),
                    byte_len: 0,
                    sha256: "0".repeat(64),
                }],
                ..missing_items_manifest.corpora[0].clone()
            }],
            ..missing_items_manifest
        };
        write_json_file(&pack_dir.join("manifest.json"), &missing_shard_manifest)
            .expect("missing shard manifest");
        let error = validate_pack_dir(&pack_dir).expect_err("missing shard");
        assert!(error.to_string().contains("vectors-0000.f32"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_embedding_input_uses_title_definition_and_glosses() {
        let dictionary = jbotci_dictionary_data::english();
        let entry = dictionary
            .entries()
            .iter()
            .find(|entry| entry.word == "klama")
            .expect("klama entry");
        let input = dictionary_embedding_input(entry);

        assert!(input.starts_with("title: klama | text: "));
        assert!(input.contains("come"));
        assert!(input.contains("go"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pack_paths_include_schema_model_and_pack() {
        let root = Path::new("/tmp/jbotci-index");
        assert_eq!(
            catalog_path(root).expect("catalog"),
            root.join("v1/catalog.json")
        );
        assert_eq!(
            pack_root(root, DEFAULT_MODEL_KEY, "pack-a"),
            root.join("v1")
                .join("models")
                .join(DEFAULT_MODEL_KEY)
                .join("packs")
                .join("pack-a")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vectors_rank_by_dot_product_then_row_index() {
        let values = vec![1.0, 0.0, 0.8, 0.6, 1.0, 0.0];
        let hits = top_vector_hits(&values, 2, &[1.0, 0.0], 3, 3);
        assert_eq!(
            hits.iter().map(|hit| hit.row_index).collect::<Vec<_>>(),
            [0, 2, 1]
        );

        let limited_hits = top_vector_hits(&values, 2, &[1.0, 0.0], 3, 2);
        assert_eq!(
            limited_hits
                .iter()
                .map(|hit| hit.row_index)
                .collect::<Vec<_>>(),
            [0, 2]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn filtered_vectors_rank_before_allocating_full_result_set() {
        let values = vec![1.0, 0.0, 0.7, 0.0, 0.9, 0.0, 0.8, 0.0];
        let hits =
            top_vector_hits_by_row(&values, 2, &[1.0, 0.0], 4, 2, |row_index| row_index != 0);
        assert_eq!(
            hits.iter().map(|hit| hit.row_index).collect::<Vec<_>>(),
            [2, 3]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vector_shards_round_trip_and_validate_hashes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus_dir = dir.path().join("corpora/test");
        let values = vec![1.0, 0.0, 0.0, 1.0];
        let shards = write_vector_shards(
            &corpus_dir,
            "corpora/test",
            &values,
            2,
            NonZeroUsize::new(8).expect("nonzero"),
        )
        .expect("write shards");
        let corpus = CorpusManifest {
            corpus_id: "test".to_owned(),
            input_format_version: DEFAULT_INPUT_FORMAT_VERSION.to_owned(),
            fingerprint: "f".repeat(64),
            row_count: 2,
            dimensions: 2,
            items_url: "items.json".to_owned(),
            items_sha256: "0".repeat(64),
            shards,
        };
        let actual = read_vector_shards(dir.path(), &corpus, 2).expect("read shards");
        assert_eq!(actual, values);
        assert_eq!(corpus.shards.len(), 2);
        assert!(corpus.shards.iter().all(|shard| shard.byte_len == 8));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vector_shards_reject_dimension_and_hash_mismatches() {
        let dir = tempfile::tempdir().expect("tempdir");
        let corpus_dir = dir.path().join("corpora/test");
        let values = vec![1.0, 0.0, 0.0, 1.0];
        let shards = write_vector_shards(
            &corpus_dir,
            "corpora/test",
            &values,
            2,
            NonZeroUsize::new(16).expect("nonzero"),
        )
        .expect("write shards");
        let corpus = CorpusManifest {
            corpus_id: "test".to_owned(),
            input_format_version: DEFAULT_INPUT_FORMAT_VERSION.to_owned(),
            fingerprint: "f".repeat(64),
            row_count: 2,
            dimensions: 2,
            items_url: "items.json".to_owned(),
            items_sha256: "0".repeat(64),
            shards,
        };

        let dimension_error = read_vector_shards(dir.path(), &corpus, 3).expect_err("dimension");
        assert!(matches!(
            dimension_error,
            EmbeddingError::DimensionMismatch {
                expected: 3,
                actual: 2
            }
        ));

        let mut bad_hash = corpus.clone();
        bad_hash.shards[0].sha256 = "1".repeat(64);
        let hash_error = read_vector_shards(dir.path(), &bad_hash, 2).expect_err("hash");
        assert!(hash_error.to_string().contains("SHA-256 mismatch"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fake_backend_builds_pack_manifest() {
        let dir = tempfile::tempdir().expect("tempdir");
        let entries = jbotci_dictionary_data::english();
        let cll_site = jbotci_cll::embedded_cll_site().expect("embedded CLL");
        let cll_chunks = &cll_site.search_chunks[..2];
        let mut backend = FakeBackend {
            dimensions: 4,
            calls: 0,
        };
        let spec = EmbeddingModelSpec {
            dimensions: 4,
            ..EmbeddingModelSpec::default_f2llm()
        };
        let report =
            build_embedding_pack(&mut backend, entries, cll_chunks, dir.path(), &spec, false)
                .expect("build pack");
        assert_eq!(report.index_source, SetupIndexSource::BuiltLocal);
        let manifest_path =
            pack_root(dir.path(), &spec.model_key, &report.pack_id).join("manifest.json");
        let manifest: EmbeddingPackManifest = read_json_file(&manifest_path).expect("manifest");
        assert_eq!(manifest.dimensions, 4);
        assert_eq!(manifest.corpora.len(), 2);
        let vlacku_corpus = manifest
            .corpora
            .iter()
            .find(|corpus| corpus.corpus_id == VLACKU_CORPUS_ID)
            .expect("vlacku corpus");
        let items_path =
            pack_root(dir.path(), &spec.model_key, &report.pack_id).join(&vlacku_corpus.items_url);
        let items: Vec<DictionaryEmbeddingItem> =
            read_json_file(&items_path).expect("dictionary items");
        assert!(items.iter().any(|item| item.kind == "gismu"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn existing_pack_reuse_reports_reused_source() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dictionary = jbotci_dictionary_data::english();
        let cll_site = jbotci_cll::embedded_cll_site().expect("embedded CLL");
        let cll_chunks = &cll_site.search_chunks[..2];
        let spec = test_embedding_spec();
        build_embedding_pack(
            &mut FakeBackend {
                dimensions: 4,
                calls: 0,
            },
            dictionary,
            cll_chunks,
            dir.path(),
            &spec,
            false,
        )
        .expect("build pack");

        let mut progress = |_| {};
        let report = reuse_existing_embedding_pack_with_progress(
            dictionary,
            cll_chunks,
            dir.path(),
            &spec,
            &mut progress,
        )
        .expect("reuse check")
        .expect("existing pack");
        assert_eq!(report.index_source, SetupIndexSource::Reused);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn remote_pack_child_urls_are_manifest_relative() {
        let url = remote_pack_child_url(
            "https://assets.example/embeddings/gguf/v1",
            "models/f2llm/packs/pack-a/manifest.json",
            "corpora/vlacku-en/items.json",
        )
        .expect("child URL");
        assert_eq!(
            url,
            "https://assets.example/embeddings/gguf/v1/models/f2llm/packs/pack-a/corpora/vlacku-en/items.json"
        );

        let error = remote_pack_child_url(
            "https://assets.example/embeddings/gguf/v1",
            "models/f2llm/packs/pack-a/manifest.json",
            "../items.json",
        )
        .expect_err("reject parent traversal");
        assert!(matches!(error, EmbeddingError::InvalidIndex { .. }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn force_rebuild_reuses_existing_native_rows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dictionary = jbotci_dictionary_data::english();
        let cll_site = jbotci_cll::embedded_cll_site().expect("embedded CLL");
        let cll_chunks = &cll_site.search_chunks[..3];
        let spec = EmbeddingModelSpec {
            dimensions: 4,
            ..EmbeddingModelSpec::default_f2llm()
        };
        let mut first = FakeBackend {
            dimensions: 4,
            calls: 0,
        };
        let total_rows = dictionary.entries().len() + cll_chunks.len();
        let mut first_progress = Vec::new();
        {
            let mut progress = |progress| first_progress.push(progress);
            build_embedding_pack_with_progress(
                &mut first,
                dictionary,
                cll_chunks,
                dir.path(),
                &spec,
                false,
                &mut progress,
            )
            .expect("initial pack");
        }
        assert_eq!(first.calls, total_rows);
        assert!(first_progress.iter().any(|progress| {
            progress.phase == SetupProgressPhase::Indexing
                && progress.loaded == Some(total_rows as u64)
                && progress.total == Some(total_rows as u64)
        }));

        let mut second = FakeBackend {
            dimensions: 4,
            calls: 0,
        };
        let mut second_progress = Vec::new();
        {
            let mut progress = |progress| second_progress.push(progress);
            build_embedding_pack_with_progress(
                &mut second,
                dictionary,
                cll_chunks,
                dir.path(),
                &spec,
                true,
                &mut progress,
            )
            .expect("force rebuild");
        }
        assert_eq!(second.calls, 0);
        assert!(second_progress.iter().any(|progress| {
            progress.phase == SetupProgressPhase::Indexing
                && progress.loaded == Some(total_rows as u64)
                && progress.total == Some(total_rows as u64)
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn native_setup_resumes_completed_dictionary_chunks_after_failure() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dictionary = jbotci_dictionary_data::english();
        let cll_site = jbotci_cll::embedded_cll_site().expect("embedded CLL");
        let cll_chunks = &cll_site.search_chunks[..3];
        let spec = test_embedding_spec();
        let fail_after = NATIVE_VECTOR_CHUNK_ROWS + 17;
        let mut failing = FailingBackend {
            dimensions: 4,
            calls: 0,
            fail_after,
        };
        let mut failed_progress = Vec::new();
        {
            let mut progress = |progress| failed_progress.push(progress);
            let error = build_embedding_pack_with_progress(
                &mut failing,
                dictionary,
                cll_chunks,
                dir.path(),
                &spec,
                false,
                &mut progress,
            )
            .expect_err("interrupted build");
            assert!(matches!(error, EmbeddingError::Backend { .. }));
        }
        assert_eq!(failing.calls, fail_after);
        let work_root =
            test_work_root(dir.path(), &spec, dictionary, cll_chunks).expect("work root");
        assert!(
            native_partial_checkpoint_path(&work_root).is_file(),
            "interrupted build should leave a checkpoint"
        );

        let total_rows = dictionary.entries().len() + cll_chunks.len();
        let mut retry = FakeBackend {
            dimensions: 4,
            calls: 0,
        };
        let mut retry_progress = Vec::new();
        {
            let mut progress = |progress| retry_progress.push(progress);
            build_embedding_pack_with_progress(
                &mut retry,
                dictionary,
                cll_chunks,
                dir.path(),
                &spec,
                false,
                &mut progress,
            )
            .expect("resumed build");
        }
        assert_eq!(retry.calls, total_rows - NATIVE_VECTOR_CHUNK_ROWS);
        assert_eq!(
            first_positive_index_progress(&retry_progress),
            Some(NATIVE_VECTOR_CHUNK_ROWS as u64)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn native_setup_resumes_dictionary_and_cll_chunks_after_failure() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dictionary = jbotci_dictionary_data::english();
        let cll_site = jbotci_cll::embedded_cll_site().expect("embedded CLL");
        assert!(cll_site.search_chunks.len() > NATIVE_VECTOR_CHUNK_ROWS + 17);
        let cll_chunks = &cll_site.search_chunks[..NATIVE_VECTOR_CHUNK_ROWS + 17];
        let spec = test_embedding_spec();
        let dictionary_rows = dictionary.entries().len();
        let fail_after = dictionary_rows + NATIVE_VECTOR_CHUNK_ROWS + 9;
        let mut failing = FailingBackend {
            dimensions: 4,
            calls: 0,
            fail_after,
        };
        {
            let mut progress = |_| {};
            let error = build_embedding_pack_with_progress(
                &mut failing,
                dictionary,
                cll_chunks,
                dir.path(),
                &spec,
                false,
                &mut progress,
            )
            .expect_err("interrupted CLL build");
            assert!(matches!(error, EmbeddingError::Backend { .. }));
        }
        assert_eq!(failing.calls, fail_after);

        let total_rows = dictionary_rows + cll_chunks.len();
        let reusable_rows = dictionary_rows + NATIVE_VECTOR_CHUNK_ROWS;
        let mut retry = FakeBackend {
            dimensions: 4,
            calls: 0,
        };
        build_embedding_pack(&mut retry, dictionary, cll_chunks, dir.path(), &spec, false)
            .expect("resumed CLL build");
        assert_eq!(retry.calls, total_rows - reusable_rows);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn native_setup_ignores_incompatible_partial_checkpoint() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dictionary = jbotci_dictionary_data::english();
        let cll_site = jbotci_cll::embedded_cll_site().expect("embedded CLL");
        let cll_chunks = &cll_site.search_chunks[..3];
        let spec = test_embedding_spec();
        let mut failing = FailingBackend {
            dimensions: 4,
            calls: 0,
            fail_after: NATIVE_VECTOR_CHUNK_ROWS + 1,
        };
        {
            let mut progress = |_| {};
            build_embedding_pack_with_progress(
                &mut failing,
                dictionary,
                cll_chunks,
                dir.path(),
                &spec,
                false,
                &mut progress,
            )
            .expect_err("interrupted build");
        }
        let work_root =
            test_work_root(dir.path(), &spec, dictionary, cll_chunks).expect("work root");
        let checkpoint_path = native_partial_checkpoint_path(&work_root);
        let mut checkpoint: NativePartialBuildCheckpoint =
            read_json_file(&checkpoint_path).expect("checkpoint");
        checkpoint.model_revision.push_str("-stale");
        write_native_partial_checkpoint(&checkpoint_path, &checkpoint).expect("corrupt checkpoint");

        let total_rows = dictionary.entries().len() + cll_chunks.len();
        let mut retry = FakeBackend {
            dimensions: 4,
            calls: 0,
        };
        build_embedding_pack(&mut retry, dictionary, cll_chunks, dir.path(), &spec, false)
            .expect("rebuilt after incompatible checkpoint");
        assert_eq!(retry.calls, total_rows);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn native_chunked_pack_vectors_match_direct_row_embeddings() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dictionary = jbotci_dictionary_data::english();
        let cll_site = jbotci_cll::embedded_cll_site().expect("embedded CLL");
        let cll_chunks = &cll_site.search_chunks[..NATIVE_VECTOR_CHUNK_ROWS + 3];
        let spec = test_embedding_spec();
        let report = build_embedding_pack(
            &mut FakeBackend {
                dimensions: 4,
                calls: 0,
            },
            dictionary,
            cll_chunks,
            dir.path(),
            &spec,
            false,
        )
        .expect("chunked pack");

        let pack_dir = pack_root(dir.path(), &spec.model_key, &report.pack_id);
        let manifest: EmbeddingPackManifest =
            read_json_file(&pack_dir.join("manifest.json")).expect("manifest");
        let dictionary_corpus =
            manifest_corpus(&manifest, VLACKU_CORPUS_ID).expect("dictionary corpus");
        let cll_corpus = manifest_corpus(&manifest, CUKTA_CORPUS_ID).expect("CLL corpus");

        let actual_dictionary =
            read_vector_shards(&pack_dir, dictionary_corpus, spec.dimensions).expect("dictionary");
        let expected_dictionary = dictionary
            .entries()
            .iter()
            .flat_map(|entry| fake_embedding_values(&dictionary_embedding_input(entry), 4))
            .collect::<Vec<_>>();
        assert_eq!(actual_dictionary, expected_dictionary);

        let actual_cll = read_vector_shards(&pack_dir, cll_corpus, spec.dimensions).expect("CLL");
        let expected_cll = cll_chunks
            .iter()
            .flat_map(|chunk| fake_embedding_values(&cll_embedding_input(chunk), 4))
            .collect::<Vec<_>>();
        assert_eq!(actual_cll, expected_cll);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fake_backend_pack_supports_semantic_search() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dictionary = jbotci_dictionary_data::english();
        let cll_site = jbotci_cll::embedded_cll_site().expect("embedded CLL");
        let cll_chunks = &cll_site.search_chunks[..4];
        assert!(
            cll_chunks
                .iter()
                .filter(|chunk| chunk.kind == jbotci_cll::CllSearchChunkKind::Section)
                .count()
                >= 2
        );
        let spec = EmbeddingModelSpec {
            dimensions: 4,
            ..EmbeddingModelSpec::default_f2llm()
        };
        build_embedding_pack(
            &mut FakeBackend {
                dimensions: 4,
                calls: 0,
            },
            dictionary,
            cll_chunks,
            dir.path(),
            &spec,
            false,
        )
        .expect("build fixture pack");

        let hits = semantic_vlacku_hits(
            &mut FakeBackend {
                dimensions: 4,
                calls: 0,
            },
            "go somewhere",
            3,
            dir.path(),
            &spec.model_key,
        )
        .expect("semantic vlacku search");
        assert_eq!(hits.len(), 3);

        let output = semantic_cukta_output(
            &mut FakeBackend {
                dimensions: 4,
                calls: 0,
            },
            cll_chunks,
            "grammar",
            2,
            CuktaTargetFilter {
                sections: true,
                paragraphs: true,
                examples: true,
            },
            dir.path(),
            &spec.model_key,
        )
        .expect("semantic cukta search");
        assert_eq!(output.matches.len(), 2);

        let section_output = semantic_cukta_output(
            &mut FakeBackend {
                dimensions: 4,
                calls: 0,
            },
            cll_chunks,
            "grammar",
            1,
            CuktaTargetFilter {
                sections: true,
                paragraphs: false,
                examples: false,
            },
            dir.path(),
            &spec.model_key,
        )
        .expect("semantic cukta section search");
        assert_eq!(section_output.matches.len(), 1);
        assert!(section_output.has_more);
        assert!(
            section_output
                .matches
                .iter()
                .all(|item| item.chunk.kind == jbotci_cll::CllSearchChunkKind::Section)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn semantic_vlacku_search_reuses_cached_loaded_corpus() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dictionary = jbotci_dictionary_data::english();
        let cll_site = jbotci_cll::embedded_cll_site().expect("embedded CLL");
        let cll_chunks = &cll_site.search_chunks[..1];
        let spec = EmbeddingModelSpec {
            dimensions: 4,
            model_revision: "cache-reuse-test-revision".to_owned(),
            ..EmbeddingModelSpec::default_f2llm()
        };
        build_embedding_pack(
            &mut FakeBackend {
                dimensions: 4,
                calls: 0,
            },
            dictionary,
            cll_chunks,
            dir.path(),
            &spec,
            false,
        )
        .expect("build fixture pack");

        let first_hits = semantic_vlacku_hits(
            &mut FakeBackend {
                dimensions: 4,
                calls: 0,
            },
            "go somewhere",
            2,
            dir.path(),
            &spec.model_key,
        )
        .expect("initial semantic vlacku search");
        assert_eq!(first_hits.len(), 2);

        let (pack_dir, manifest) =
            load_latest_pack(dir.path(), &spec.model_key).expect("latest pack");
        let corpus = manifest_corpus(&manifest, VLACKU_CORPUS_ID).expect("vlacku corpus");
        std::fs::remove_file(pack_dir.join(&corpus.items_url)).expect("remove items");
        for shard in &corpus.shards {
            std::fs::remove_file(pack_dir.join(&shard.url)).expect("remove vector shard");
        }

        let second_hits = semantic_vlacku_hits(
            &mut FakeBackend {
                dimensions: 4,
                calls: 0,
            },
            "go somewhere",
            2,
            dir.path(),
            &spec.model_key,
        )
        .expect("cached semantic vlacku search");
        assert_eq!(second_hits, first_hits);
    }
}
