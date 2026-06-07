//! Native llama.cpp embedding backend.

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Once, OnceLock};

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, requires};
use llama_cpp_4::context::LlamaContext;
use llama_cpp_4::context::params::LlamaContextParams;
use llama_cpp_4::llama_backend::LlamaBackend;
use llama_cpp_4::llama_batch::LlamaBatch;
use llama_cpp_4::model::params::LlamaModelParams;
use llama_cpp_4::model::{AddBos, LlamaModel};

use crate::{
    EmbeddingBackend, EmbeddingError, EmbeddingModelSpec, QueryEmbedding, SetupOptions,
    SetupProgress, SetupProgressCallback, SetupProgressPhase, SetupReport,
    build_embedding_pack_with_progress, default_index_root, default_model_root,
    ensure_model_file_with_progress, model_file_path, model_spec, normalize_vector,
    semantic_cukta_output, semantic_vlacku_hits,
};

const N_BATCH: u32 = 2048;
const N_UBATCH: u32 = 2048;
const N_CTX: u32 = 2048 * 32;
const N_PARALLEL: usize = 32;

static BACKEND: OnceLock<Result<LlamaBackend, String>> = OnceLock::new();
static SUPPRESS_LLAMA_LOGS: Once = Once::new();

pub type NativeGemmaEmbeddingBackend = NativeLlamaEmbeddingBackend;

#[derive(Debug)]
#[invariant(true)]
pub struct NativeLlamaEmbeddingBackend {
    model: &'static LlamaModel,
    context: LlamaContext<'static>,
    dimensions: usize,
    max_tokens_per_call: usize,
}

impl NativeLlamaEmbeddingBackend {
    #[requires(path.is_file())]
    #[ensures(ret.as_ref().is_ok_and(|backend| backend.dimensions == spec.dimensions) || ret.is_err())]
    pub fn load(spec: &EmbeddingModelSpec, path: &Path) -> Result<Self, EmbeddingError> {
        let backend = global_backend()?;
        let model = LlamaModel::load_from_file(backend, path, &LlamaModelParams::default())
            .map_err(|source| EmbeddingError::Backend {
                message: format!("llama.cpp failed to load `{}`: {source}", path.display()),
            })?;
        let model = Box::leak(Box::new(model));
        let threads = std::thread::available_parallelism()
            .map(|count| count.get().min(N_PARALLEL))
            .unwrap_or(1)
            .max(1);
        let context_params = LlamaContextParams::default()
            .with_embeddings(true)
            .with_n_ctx(NonZeroU32::new(N_CTX))
            .with_n_batch(N_BATCH)
            .with_n_ubatch(N_UBATCH)
            .with_n_threads(threads as i32)
            .with_n_threads_batch(threads as i32);
        let context = model
            .new_context(backend, context_params)
            .map_err(|source| EmbeddingError::Backend {
                message: format!("llama.cpp failed to create embedding context: {source}"),
            })?;
        let dimensions =
            usize::try_from(model.n_embd_out()).map_err(|_| EmbeddingError::Backend {
                message: "llama.cpp reported invalid embedding dimension".to_owned(),
            })?;
        if dimensions != spec.dimensions {
            return Err(EmbeddingError::DimensionMismatch {
                expected: spec.dimensions,
                actual: dimensions,
            });
        }
        let max_tokens_per_call =
            usize::try_from(context.n_ubatch()).map_err(|_| EmbeddingError::Backend {
                message: "llama.cpp reported invalid n_ubatch".to_owned(),
            })?;
        Ok(Self {
            model,
            context,
            dimensions,
            max_tokens_per_call,
        })
    }

    #[requires(!tokens.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|values| values.len() == self.dimensions) || ret.is_err())]
    fn embed_tokens(
        &mut self,
        tokens: &[llama_cpp_4::token::LlamaToken],
    ) -> Result<Vec<f32>, EmbeddingError> {
        if tokens.len() > self.max_tokens_per_call {
            return Err(EmbeddingError::Backend {
                message: format!(
                    "token window has {} tokens, maximum is {}",
                    tokens.len(),
                    self.max_tokens_per_call
                ),
            });
        }
        let _ = self
            .context
            .clear_kv_cache_seq(Some(0), None, None)
            .map_err(|source| EmbeddingError::Backend {
                message: format!("llama.cpp failed to clear context memory: {source}"),
            })?;
        let mut batch = LlamaBatch::new(tokens.len(), 1);
        for (index, token) in tokens.iter().enumerate() {
            batch
                .add(*token, index as i32, &[0], true)
                .map_err(|source| EmbeddingError::Backend {
                    message: format!("llama.cpp failed to prepare embedding batch: {source}"),
                })?;
        }
        if self.model.has_encoder() && !self.model.has_decoder() {
            self.context
                .encode(&mut batch)
                .map_err(|source| EmbeddingError::Backend {
                    message: format!("llama.cpp embedding encode failed: {source}"),
                })?;
        } else {
            self.context
                .decode(&mut batch)
                .map_err(|source| EmbeddingError::Backend {
                    message: format!("llama.cpp embedding decode failed: {source}"),
                })?;
        }
        let embedding = self
            .context
            .embeddings_seq_ith(0)
            .or_else(|_| self.context.embeddings_ith(tokens.len() as i32 - 1))
            .map_err(|source| EmbeddingError::Backend {
                message: format!("llama.cpp did not return an embedding: {source}"),
            })?;
        if embedding.len() != self.dimensions {
            return Err(EmbeddingError::DimensionMismatch {
                expected: self.dimensions,
                actual: embedding.len(),
            });
        }
        let mut values = embedding.to_vec();
        normalize_vector(&mut values);
        Ok(values)
    }
}

#[requires(true)]
#[ensures(true)]
pub fn suppress_llama_logs_for_cli() {
    SUPPRESS_LLAMA_LOGS.call_once(|| {
        // llama.cpp's default logger writes to stderr. The CLI owns stderr for
        // user-facing diagnostics, so install an explicit silent logger there.
        unsafe {
            llama_cpp_4::log_set(Some(silent_llama_log), std::ptr::null_mut());
        }
    });
}

#[requires(true)]
#[ensures(true)]
unsafe extern "C" fn silent_llama_log(
    _level: core::ffi::c_uint,
    _text: *const core::ffi::c_char,
    _user_data: *mut core::ffi::c_void,
) {
}

#[contract_trait]
impl EmbeddingBackend for NativeLlamaEmbeddingBackend {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|dimensions| *dimensions == self.dimensions) || ret.is_err())]
    fn dimensions(&self) -> Result<usize, EmbeddingError> {
        Ok(self.dimensions)
    }

    #[requires(!input.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|embedding| embedding.values.len() == self.dimensions) || ret.is_err())]
    fn embed(&mut self, input: &str) -> Result<QueryEmbedding, EmbeddingError> {
        let tokens = self
            .model
            .str_to_token(input, AddBos::Always)
            .map_err(|source| EmbeddingError::Backend {
                message: format!("llama.cpp tokenization failed: {source}"),
            })?;
        if tokens.is_empty() {
            return Err(EmbeddingError::Backend {
                message: "cannot embed an empty token sequence".to_owned(),
            });
        }
        if tokens.len() <= self.max_tokens_per_call {
            return Ok(QueryEmbedding {
                values: self.embed_tokens(&tokens)?,
            });
        }
        let mut pooled = vec![0.0; self.dimensions];
        let mut window_count = 0usize;
        let window_size = self.max_tokens_per_call.max(1);
        for window in tokens.chunks(window_size) {
            let embedding = self.embed_tokens(window)?;
            for (accumulator, value) in pooled.iter_mut().zip(embedding.iter()) {
                *accumulator += *value;
            }
            window_count += 1;
        }
        if window_count == 0 {
            return Err(EmbeddingError::Backend {
                message: "cannot pool embeddings from an empty token window list".to_owned(),
            });
        }
        for value in &mut pooled {
            *value /= window_count as f32;
        }
        normalize_vector(&mut pooled);
        Ok(QueryEmbedding { values: pooled })
    }
}

#[requires(true)]
#[ensures(true)]
pub fn setup_embeddings(options: &SetupOptions) -> Result<SetupReport, EmbeddingError> {
    let mut progress = |_| {};
    setup_embeddings_with_progress(options, &mut progress)
}

#[requires(true)]
#[ensures(true)]
pub fn setup_embeddings_with_progress(
    options: &SetupOptions,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<SetupReport, EmbeddingError> {
    let result = setup_embeddings_with_progress_inner(options, progress);
    if let Err(error) = &result {
        progress(SetupProgress::indeterminate(
            SetupProgressPhase::Error,
            "error",
            "Embedding setup failed",
            &error.to_string(),
        ));
    }
    result
}

#[requires(true)]
#[ensures(true)]
fn setup_embeddings_with_progress_inner(
    options: &SetupOptions,
    progress: &mut SetupProgressCallback<'_>,
) -> Result<SetupReport, EmbeddingError> {
    progress(SetupProgress::indeterminate(
        SetupProgressPhase::ResolvingPaths,
        "setup",
        "Preparing setup",
        "Resolving embedding model and index paths.",
    ));
    let spec = model_spec(&options.model_key).ok_or_else(|| EmbeddingError::UnsupportedModel {
        model_key: options.model_key.clone(),
    })?;
    let model_root = options
        .model_dir
        .clone()
        .map(Ok)
        .unwrap_or_else(default_model_root)?;
    let index_root = options
        .index_dir
        .clone()
        .map(Ok)
        .unwrap_or_else(default_index_root)?;
    let model_path = model_file_path(&model_root, &spec);
    ensure_model_file_with_progress(&spec, &model_path, options.force, progress)?;
    progress(SetupProgress::indeterminate(
        SetupProgressPhase::LoadingModel,
        "load",
        "Loading model",
        "Loading embedding model with llama.cpp.",
    ));
    let mut backend = NativeLlamaEmbeddingBackend::load(&spec, &model_path)?;
    let dictionary = jbotci_dictionary_data::english();
    let cll_site =
        jbotci_cll::embedded_cll_site().map_err(|error| EmbeddingError::InvalidIndex {
            message: error.to_string(),
        })?;
    let mut report = build_embedding_pack_with_progress(
        &mut backend,
        dictionary,
        jbotci_cll::cll_search_all_chunks(cll_site),
        &index_root,
        &spec,
        options.force,
        progress,
    )?;
    report.model_path = model_path;
    Ok(report)
}

#[requires(!model_key.is_empty())]
#[ensures(true)]
pub fn load_backend_for_search(
    model_key: &str,
    model_dir: Option<PathBuf>,
) -> Result<NativeLlamaEmbeddingBackend, EmbeddingError> {
    let spec = model_spec(model_key).ok_or_else(|| EmbeddingError::UnsupportedModel {
        model_key: model_key.to_owned(),
    })?;
    let model_root = model_dir.map(Ok).unwrap_or_else(default_model_root)?;
    let model_path = model_file_path(&model_root, &spec);
    if !model_path.is_file() {
        return Err(EmbeddingError::InvalidModel {
            message: format!(
                "embedding model is missing at `{}`; run `jbotci setup --embedding`",
                model_path.display()
            ),
        });
    }
    NativeLlamaEmbeddingBackend::load(&spec, &model_path)
}

#[derive(Debug)]
#[invariant(true)]
pub struct NativeEmbeddingSearchService {
    model_key: String,
    index_root: PathBuf,
    backend: NativeLlamaEmbeddingBackend,
}

impl NativeEmbeddingSearchService {
    #[requires(!model_key.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|service| service.model_key == model_key) || ret.is_err())]
    pub fn load(
        model_key: &str,
        model_dir: Option<PathBuf>,
        index_dir: Option<PathBuf>,
    ) -> Result<Self, EmbeddingError> {
        let index_root = index_dir.map(Ok).unwrap_or_else(default_index_root)?;
        let backend = load_backend_for_search(model_key, model_dir)?;
        Ok(Self {
            model_key: model_key.to_owned(),
            index_root,
            backend,
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn model_key(&self) -> &str {
        &self.model_key
    }

    #[requires(true)]
    #[ensures(ret == self.index_root)]
    pub fn index_root(&self) -> &Path {
        &self.index_root
    }

    #[requires(!query.trim().is_empty())]
    #[requires(count > 0)]
    #[ensures(ret.as_ref().is_ok_and(|hits| hits.len() <= count) || ret.is_err())]
    pub fn semantic_vlacku_hits(
        &mut self,
        query: &str,
        count: usize,
    ) -> Result<Vec<crate::DictionarySemanticHit>, EmbeddingError> {
        semantic_vlacku_hits(
            &mut self.backend,
            query,
            count,
            &self.index_root,
            &self.model_key,
        )
    }

    #[requires(!query.trim().is_empty())]
    #[requires(count > 0)]
    #[ensures(ret.as_ref().is_ok() || ret.is_err())]
    pub fn semantic_cukta_output(
        &mut self,
        chunks: &[jbotci_cll::CllSearchChunk],
        query: &str,
        count: usize,
        targets: jbotci_cll::CuktaTargetFilter,
    ) -> Result<jbotci_cll::CuktaSearchOutput, EmbeddingError> {
        semantic_cukta_output(
            &mut self.backend,
            chunks,
            query,
            count,
            targets,
            &self.index_root,
            &self.model_key,
        )
    }
}

#[requires(true)]
#[ensures(true)]
fn global_backend() -> Result<&'static LlamaBackend, EmbeddingError> {
    match BACKEND.get_or_init(|| LlamaBackend::init().map_err(|error| error.to_string())) {
        Ok(backend) => Ok(backend),
        Err(message) => Err(EmbeddingError::Backend {
            message: format!("llama.cpp backend initialization failed: {message}"),
        }),
    }
}
