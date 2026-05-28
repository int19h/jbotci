//! EmbeddingGemma model and vector-pack support.

use std::cmp::Ordering;
use std::env;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, requires};
use directories::ProjectDirs;
use jbotci_cll::{
    CllSearchChunk, CllSearchMatch, CuktaSearchMode, CuktaSearchOutput, CuktaTargetFilter,
    clamp_cukta_result_count, cll_search_all_chunks,
};
use jbotci_dictionary::{Dictionary, DictionaryEntry};
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
pub const DEFAULT_MODEL_KEY: &str = "embedding-gemma-300m-q4-768";
pub const DEFAULT_MODEL_REVISION: &str = "8dd0ca2a66a8f14470acb0e2a71f801afbc5fb73";
pub const DEFAULT_MODEL_DIMENSIONS: usize = 768;
pub const DEFAULT_INPUT_FORMAT_VERSION: &str = "egemma-v0-parity-1";
pub const DEFAULT_VECTOR_SHARD_TARGET_BYTES: usize = 8 * 1024 * 1024;
pub const VLACKU_CORPUS_ID: &str = "vlacku-en";
pub const CUKTA_CORPUS_ID: &str = "cukta-cll";
pub const RETRIEVAL_QUERY_PREFIX: &str = "task: search result | query: ";
pub const RETRIEVAL_DOCUMENT_PREFIX: &str = "title: {title} | text: {text}";

const DEFAULT_HF_ENDPOINT: &str = "https://huggingface.co";
const DEFAULT_GGUF_REPO: &str = "ggml-org/embeddinggemma-300M-qat-q4_0-GGUF";
const DEFAULT_GGUF_FILE: &str = "embeddinggemma-300M-qat-Q4_0.gguf";
const DEFAULT_GGUF_SIZE: u64 = 277_852_192;
const DEFAULT_GGUF_SHA256: &str =
    "50d28e22432a148f6f8a86eab3700f92add5d1f54baf7790675a2a4dadbccf26";
const DEFAULT_WEB_MODEL: &str = "onnx-community/embeddinggemma-300m-ONNX";
const DEFAULT_WEB_DTYPE: &str = "q4";
const LLAMA_CPP_4_RUNTIME_VERSION: &str = "0.3.0";

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
    pub native_hf_repo: String,
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
    pub fn default_embedding_gemma() -> Self {
        Self {
            model_key: DEFAULT_MODEL_KEY.to_owned(),
            model_revision: DEFAULT_MODEL_REVISION.to_owned(),
            native_hf_repo: DEFAULT_GGUF_REPO.to_owned(),
            native_hf_file: DEFAULT_GGUF_FILE.to_owned(),
            native_size_bytes: DEFAULT_GGUF_SIZE,
            native_sha256: DEFAULT_GGUF_SHA256.to_owned(),
            web_model: DEFAULT_WEB_MODEL.to_owned(),
            web_dtype: DEFAULT_WEB_DTYPE.to_owned(),
            dimensions: DEFAULT_MODEL_DIMENSIONS,
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_some_and(|spec| spec.model_key == model_key) || ret.is_none())]
pub fn model_spec(model_key: &str) -> Option<EmbeddingModelSpec> {
    (model_key == DEFAULT_MODEL_KEY).then(EmbeddingModelSpec::default_embedding_gemma)
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct CllEmbeddingItem {
    pub chunk_index: usize,
    pub input_hash: String,
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
    pub index_dir: Option<PathBuf>,
    pub model_dir: Option<PathBuf>,
}

impl Default for SetupOptions {
    #[requires(true)]
    #[ensures(ret.model_key == DEFAULT_MODEL_KEY)]
    fn default() -> Self {
        Self {
            model_key: DEFAULT_MODEL_KEY.to_owned(),
            force: false,
            index_dir: None,
            model_dir: None,
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
        spec.model_revision,
        spec.native_hf_file
    )
}

#[requires(true)]
#[ensures(ret.starts_with(RETRIEVAL_QUERY_PREFIX))]
pub fn build_retrieval_query_input(content: &str) -> String {
    format!("{RETRIEVAL_QUERY_PREFIX}{content}")
}

#[requires(true)]
#[ensures(ret.contains(" | text: "))]
pub fn build_retrieval_document_input(content: &str, title: &str) -> String {
    let safe_title = if title.trim().is_empty() {
        "none"
    } else {
        title
    };
    RETRIEVAL_DOCUMENT_PREFIX
        .replace("{title}", safe_title)
        .replace("{text}", content)
}

#[requires(true)]
#[ensures(ret.contains(&entry.word))]
pub fn dictionary_embedding_input(entry: &DictionaryEntry<'_>) -> String {
    let mut body_parts = Vec::new();
    let definition = replace_dollar_markup_with_placeholder(entry.definition);
    if !definition.trim().is_empty() {
        body_parts.push(definition);
    }
    let glosses = entry
        .gloss_keywords
        .iter()
        .map(|keyword| match keyword.meaning {
            Some(meaning) => format!("{} ({meaning})", keyword.word),
            None => keyword.word.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("; ");
    if !glosses.trim().is_empty() {
        body_parts.push(glosses);
    }
    build_retrieval_document_input(&body_parts.join("\n"), entry.word)
}

#[requires(true)]
#[ensures(true)]
fn replace_dollar_markup_with_placeholder(input: &str) -> String {
    let mut output = String::new();
    let mut rest = input;
    loop {
        let Some(start) = rest.find('$') else {
            output.push_str(rest);
            break;
        };
        output.push_str(&rest[..start]);
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('$') else {
            output.push('$');
            output.push_str(after_start);
            break;
        };
        let inside = &after_start[..end];
        if inside.trim().is_empty() {
            output.push_str("$ $");
        } else {
            output.push_str("place");
        }
        rest = &after_start[end + 1..];
    }
    output
}

#[requires(true)]
#[ensures(ret.contains(&chunk.label))]
pub fn cll_embedding_input(chunk: &CllSearchChunk) -> String {
    build_retrieval_document_input(&chunk.text, &chunk.label)
}

#[requires(true)]
#[ensures(ret.len() == 64)]
pub fn sha256_hex_bytes(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes))
}

#[requires(path.is_file())]
#[ensures(ret.as_ref().is_ok_and(|value| value.len() == 64) || ret.is_err())]
pub fn sha256_hex_file(path: &Path) -> Result<String, EmbeddingError> {
    let mut file = File::open(path).map_err(|source| EmbeddingError::Io {
        context: format!("failed to open `{}`", path.display()),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buf).map_err(|source| EmbeddingError::Io {
            context: format!("failed to read `{}`", path.display()),
            source,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
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
    if query.len() != dimensions || limit == 0 {
        return Vec::new();
    }
    let mut hits = values
        .chunks_exact(dimensions)
        .take(row_count)
        .enumerate()
        .map(|(row_index, vector)| VectorHit {
            row_index,
            score: dot_product(vector, query),
        })
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.row_index.cmp(&right.row_index))
    });
    hits.truncate(limit);
    hits
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

#[requires(path.is_file())]
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
#[ensures(ret.len() == 64)]
pub fn dictionary_fingerprint(dictionary: &Dictionary<'_>) -> String {
    let mut hasher = Sha256::new();
    for entry in dictionary.entries() {
        hasher.update(entry.word.as_bytes());
        hasher.update([0]);
        hasher.update(entry.definition_id.0.to_le_bytes());
        hasher.update([0]);
        hasher.update(dictionary_embedding_input(entry).as_bytes());
        hasher.update([0]);
    }
    hex_digest(hasher.finalize())
}

#[requires(true)]
#[ensures(ret.len() == 64)]
pub fn cll_fingerprint(chunks: &[CllSearchChunk]) -> String {
    let mut hasher = Sha256::new();
    for chunk in chunks {
        hasher.update(chunk.label.as_bytes());
        hasher.update([0]);
        hasher.update(cll_embedding_input(chunk).as_bytes());
        hasher.update([0]);
    }
    hex_digest(hasher.finalize())
}

#[requires(!path.as_os_str().is_empty())]
#[ensures(true)]
pub fn ensure_model_file(
    spec: &EmbeddingModelSpec,
    path: &Path,
    force: bool,
) -> Result<(), EmbeddingError> {
    if path.is_file() && !force {
        validate_model_file(spec, path)?;
        return Ok(());
    }
    download_model_file(spec, path)?;
    validate_model_file(spec, path)
}

#[requires(path.is_file())]
#[ensures(true)]
pub fn validate_model_file(spec: &EmbeddingModelSpec, path: &Path) -> Result<(), EmbeddingError> {
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
    let sha256 = sha256_hex_file(path)?;
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
    ensure_parent_dir(path)?;
    let url = model_download_url(spec);
    let partial_path = path.with_extension("downloadInProgress");
    let mut request = ureq::get(&url);
    if let Ok(token) = env::var(HF_TOKEN_ENV)
        && !token.trim().is_empty()
    {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    let response = request.call().map_err(|source| EmbeddingError::Http {
        message: format!("failed to download `{url}`: {source}"),
    })?;
    let mut reader = response.into_body().into_reader();
    let mut writer =
        BufWriter::new(
            File::create(&partial_path).map_err(|source| EmbeddingError::Io {
                context: format!("failed to create `{}`", partial_path.display()),
                source,
            })?,
        );
    std::io::copy(&mut reader, &mut writer).map_err(|source| EmbeddingError::Io {
        context: format!("failed to write `{}`", partial_path.display()),
        source,
    })?;
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
        DEFAULT_INPUT_FORMAT_VERSION,
        &spec.model_revision,
        &dictionary_fingerprint,
        &cll_fingerprint,
    );
    let final_pack_root = pack_root(index_root, &spec.model_key, &pack_id);
    if final_pack_root.join("manifest.json").is_file() && !force {
        write_catalog(index_root, spec, &pack_id)?;
        return Ok(SetupReport {
            index_root: index_root.to_owned(),
            model_path: PathBuf::new(),
            pack_id,
            dictionary_rows: dictionary.entries().len(),
            cll_rows: cll_chunks.len(),
        });
    }
    let temp_pack_root = final_pack_root.with_extension("tmp");
    if temp_pack_root.exists() {
        fs::remove_dir_all(&temp_pack_root).map_err(|source| EmbeddingError::Io {
            context: format!("failed to remove `{}`", temp_pack_root.display()),
            source,
        })?;
    }
    fs::create_dir_all(&temp_pack_root).map_err(|source| EmbeddingError::Io {
        context: format!("failed to create `{}`", temp_pack_root.display()),
        source,
    })?;

    let dictionary_corpus = write_dictionary_corpus(
        backend,
        dictionary,
        &temp_pack_root,
        &pack_id,
        dimensions,
        &dictionary_fingerprint,
    )?;
    let cll_corpus = write_cll_corpus(
        backend,
        cll_chunks,
        &temp_pack_root,
        &pack_id,
        dimensions,
        &cll_fingerprint,
    )?;
    let manifest = EmbeddingPackManifest {
        schema_version: INDEX_SCHEMA_VERSION,
        model_key: spec.model_key.clone(),
        model_revision: spec.model_revision.clone(),
        pack_id: pack_id.clone(),
        input_format_version: DEFAULT_INPUT_FORMAT_VERSION.to_owned(),
        built_by: EmbeddingRuntime {
            runtime: "llama-cpp-4".to_owned(),
            version: LLAMA_CPP_4_RUNTIME_VERSION.to_owned(),
        },
        dimensions,
        element_type: "f32le".to_owned(),
        normalized: true,
        distance: "dot".to_owned(),
        compatible_query_runtimes: vec![EmbeddingRuntime {
            runtime: "llama-cpp-4".to_owned(),
            version: LLAMA_CPP_4_RUNTIME_VERSION.to_owned(),
        }],
        corpora: vec![dictionary_corpus, cll_corpus],
    };
    write_json_file(&temp_pack_root.join("manifest.json"), &manifest)?;
    validate_pack_dir(&temp_pack_root)?;
    if final_pack_root.exists() {
        fs::remove_dir_all(&final_pack_root).map_err(|source| EmbeddingError::Io {
            context: format!("failed to remove `{}`", final_pack_root.display()),
            source,
        })?;
    }
    ensure_parent_dir(&final_pack_root)?;
    fs::rename(&temp_pack_root, &final_pack_root).map_err(|source| EmbeddingError::Io {
        context: format!(
            "failed to publish `{}` as `{}`",
            temp_pack_root.display(),
            final_pack_root.display()
        ),
        source,
    })?;
    write_catalog(index_root, spec, &pack_id)?;
    Ok(SetupReport {
        index_root: index_root.to_owned(),
        model_path: PathBuf::new(),
        pack_id,
        dictionary_rows: dictionary.entries().len(),
        cll_rows: cll_chunks.len(),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|corpus| corpus.corpus_id == VLACKU_CORPUS_ID) || ret.is_err())]
fn write_dictionary_corpus<B: EmbeddingBackend>(
    backend: &mut B,
    dictionary: &Dictionary<'_>,
    pack_dir: &Path,
    pack_id: &str,
    dimensions: usize,
    fingerprint: &str,
) -> Result<CorpusManifest, EmbeddingError> {
    let corpus_dir = pack_dir.join("corpora").join(VLACKU_CORPUS_ID);
    let items = dictionary
        .entries()
        .iter()
        .enumerate()
        .map(|(entry_index, entry)| {
            let input = dictionary_embedding_input(entry);
            DictionaryEmbeddingItem {
                entry_index,
                word: entry.word.to_owned(),
                definition_id: entry.definition_id.0,
                input_hash: sha256_hex_bytes(input.as_bytes()),
            }
        })
        .collect::<Vec<_>>();
    let mut values = Vec::with_capacity(items.len() * dimensions);
    for entry in dictionary.entries() {
        let mut embedding = backend.embed(&dictionary_embedding_input(entry))?.values;
        if embedding.len() != dimensions {
            return Err(EmbeddingError::DimensionMismatch {
                expected: dimensions,
                actual: embedding.len(),
            });
        }
        normalize_vector(&mut embedding);
        values.extend_from_slice(&embedding);
    }
    write_corpus_files(
        &corpus_dir,
        &format!("corpora/{VLACKU_CORPUS_ID}"),
        &items,
        values.as_slice(),
        dimensions,
        VLACKU_CORPUS_ID,
        pack_id,
        fingerprint,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|corpus| corpus.corpus_id == CUKTA_CORPUS_ID) || ret.is_err())]
fn write_cll_corpus<B: EmbeddingBackend>(
    backend: &mut B,
    chunks: &[CllSearchChunk],
    pack_dir: &Path,
    pack_id: &str,
    dimensions: usize,
    fingerprint: &str,
) -> Result<CorpusManifest, EmbeddingError> {
    let corpus_dir = pack_dir.join("corpora").join(CUKTA_CORPUS_ID);
    let items = chunks
        .iter()
        .enumerate()
        .map(|(chunk_index, chunk)| {
            let input = cll_embedding_input(chunk);
            CllEmbeddingItem {
                chunk_index,
                input_hash: sha256_hex_bytes(input.as_bytes()),
            }
        })
        .collect::<Vec<_>>();
    let mut values = Vec::with_capacity(items.len() * dimensions);
    for chunk in chunks {
        let mut embedding = backend.embed(&cll_embedding_input(chunk))?.values;
        if embedding.len() != dimensions {
            return Err(EmbeddingError::DimensionMismatch {
                expected: dimensions,
                actual: embedding.len(),
            });
        }
        normalize_vector(&mut embedding);
        values.extend_from_slice(&embedding);
    }
    write_corpus_files(
        &corpus_dir,
        &format!("corpora/{CUKTA_CORPUS_ID}"),
        &items,
        values.as_slice(),
        dimensions,
        CUKTA_CORPUS_ID,
        pack_id,
        fingerprint,
    )
}

#[allow(clippy::too_many_arguments)]
#[requires(dimensions > 0)]
#[ensures(ret.as_ref().is_ok_and(|corpus| corpus.row_count > 0) || ret.is_err())]
fn write_corpus_files<T: Serialize>(
    corpus_dir: &Path,
    url_prefix: &str,
    items: &[T],
    values: &[f32],
    dimensions: usize,
    corpus_id: &str,
    _pack_id: &str,
    fingerprint: &str,
) -> Result<CorpusManifest, EmbeddingError> {
    let items_path = corpus_dir.join("items.json");
    write_json_file(&items_path, &items)?;
    let items_sha256 = sha256_hex_file(&items_path)?;
    let shards = write_vector_shards(
        corpus_dir,
        url_prefix,
        values,
        dimensions,
        NonZeroUsize::new(DEFAULT_VECTOR_SHARD_TARGET_BYTES).expect("shard size is nonzero"),
    )?;
    Ok(CorpusManifest {
        corpus_id: corpus_id.to_owned(),
        input_format_version: DEFAULT_INPUT_FORMAT_VERSION.to_owned(),
        fingerprint: fingerprint.to_owned(),
        row_count: items.len(),
        dimensions,
        items_url: format!("{}/items.json", url_prefix.trim_end_matches('/')),
        items_sha256,
        shards,
    })
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
    let catalog: EmbeddingCatalog = read_json_file(&catalog_path(index_root)?)?;
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
    let manifest: EmbeddingPackManifest = read_json_file(&pack_dir.join("manifest.json"))?;
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
    let items_path = pack_dir.join(&corpus.items_url);
    let items: Vec<DictionaryEmbeddingItem> = read_json_file(&items_path)?;
    if sha256_hex_file(&items_path)? != corpus.items_sha256 {
        return Err(EmbeddingError::InvalidIndex {
            message: format!("items file `{}` SHA-256 mismatch", items_path.display()),
        });
    }
    let matrix = read_vector_shards(&pack_dir, corpus, manifest.dimensions)?;
    let mut query_embedding = backend.embed(&build_retrieval_query_input(query))?.values;
    normalize_vector(&mut query_embedding);
    let hits = top_vector_hits(
        &matrix,
        manifest.dimensions,
        &query_embedding,
        corpus.row_count,
        count.max(1),
    );
    Ok(hits
        .into_iter()
        .filter_map(|hit| {
            let item = items.get(hit.row_index)?;
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
    let items_path = pack_dir.join(&corpus.items_url);
    let items: Vec<CllEmbeddingItem> = read_json_file(&items_path)?;
    if sha256_hex_file(&items_path)? != corpus.items_sha256 {
        return Err(EmbeddingError::InvalidIndex {
            message: format!("items file `{}` SHA-256 mismatch", items_path.display()),
        });
    }
    let matrix = read_vector_shards(&pack_dir, corpus, manifest.dimensions)?;
    let mut query_embedding = backend.embed(&build_retrieval_query_input(query))?.values;
    normalize_vector(&mut query_embedding);
    let hits = top_vector_hits(
        &matrix,
        manifest.dimensions,
        &query_embedding,
        corpus.row_count,
        corpus.row_count,
    );
    let mut matches = Vec::new();
    for hit in hits {
        let Some(item) = items.get(hit.row_index) else {
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
    build_embedding_pack(
        backend,
        dictionary,
        cll_search_all_chunks(cll_site),
        &index_root,
        &spec,
        options.force,
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
            let mut values = (0..self.dimensions)
                .map(|index| {
                    let byte = input.as_bytes()[index % input.len()];
                    f32::from(byte) + index as f32
                })
                .collect::<Vec<_>>();
            normalize_vector(&mut values);
            Ok(QueryEmbedding { values })
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn retrieval_prefixes_match_v0() {
        assert_eq!(
            build_retrieval_query_input("klama"),
            "task: search result | query: klama"
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
        let mut backend = FakeBackend { dimensions: 4 };
        let spec = EmbeddingModelSpec {
            dimensions: 4,
            ..EmbeddingModelSpec::default_embedding_gemma()
        };
        let report =
            build_embedding_pack(&mut backend, entries, cll_chunks, dir.path(), &spec, false)
                .expect("build pack");
        let manifest_path =
            pack_root(dir.path(), &spec.model_key, &report.pack_id).join("manifest.json");
        let manifest: EmbeddingPackManifest = read_json_file(&manifest_path).expect("manifest");
        assert_eq!(manifest.dimensions, 4);
        assert_eq!(manifest.corpora.len(), 2);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fake_backend_pack_supports_semantic_search() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dictionary = jbotci_dictionary_data::english();
        let cll_site = jbotci_cll::embedded_cll_site().expect("embedded CLL");
        let cll_chunks = &cll_site.search_chunks[..4];
        let spec = EmbeddingModelSpec {
            dimensions: 4,
            ..EmbeddingModelSpec::default_embedding_gemma()
        };
        build_embedding_pack(
            &mut FakeBackend { dimensions: 4 },
            dictionary,
            cll_chunks,
            dir.path(),
            &spec,
            false,
        )
        .expect("build fixture pack");

        let hits = semantic_vlacku_hits(
            &mut FakeBackend { dimensions: 4 },
            "go somewhere",
            3,
            dir.path(),
            &spec.model_key,
        )
        .expect("semantic vlacku search");
        assert_eq!(hits.len(), 3);

        let output = semantic_cukta_output(
            &mut FakeBackend { dimensions: 4 },
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
    }
}
