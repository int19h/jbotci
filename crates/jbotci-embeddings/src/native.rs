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
    SetupProgress, SetupProgressCallback, SetupProgressPhase, SetupReport, UsePrecomputed,
    build_embedding_pack_with_reuse, default_index_root, default_model_root,
    download_precomputed_embedding_pack_with_progress, ensure_model_file_with_progress,
    model_file_path, model_spec, normalize_vector, reuse_existing_embedding_pack_with_progress,
    semantic_cukta_output, semantic_vlacku_hits, validate_native_reuse_source_with_adoption,
};

const N_BATCH: u32 = crate::NATIVE_MAX_WINDOW_TOKENS;
const N_UBATCH: u32 = crate::NATIVE_MAX_WINDOW_TOKENS;
const N_CTX: u32 = crate::NATIVE_MAX_WINDOW_TOKENS;
const N_PARALLEL: usize = 32;

static BACKEND: OnceLock<Result<LlamaBackend, String>> = OnceLock::new();
static SUPPRESS_LLAMA_LOGS: Once = Once::new();

#[derive(Debug)]
#[invariant(true)]
struct OwnedLlamaContext {
    // Rust drops struct fields in declaration order. The llama context carries
    // a reference into model storage, so context must be dropped before model.
    context: LlamaContext<'static>,
    model: Box<LlamaModel>,
}

impl OwnedLlamaContext {
    #[requires(path.is_file())]
    #[ensures(true)]
    fn load(path: &Path, context_params: LlamaContextParams) -> Result<Self, EmbeddingError> {
        let backend = global_backend()?;
        let model = Box::new(
            LlamaModel::load_from_file(backend, path, &LlamaModelParams::default()).map_err(
                |source| EmbeddingError::Backend {
                    message: format!("llama.cpp failed to load `{}`: {source}", path.display()),
                },
            )?,
        );
        let model_ref: &'static LlamaModel = {
            let ptr: *const LlamaModel = model.as_ref();
            // Safety: the model is heap-allocated in a Box whose allocation is
            // stable after moves. OwnedLlamaContext stores the context before
            // the model so drop order destroys context first, exposes no cloned
            // owner, and never returns this extended reference to callers.
            unsafe { &*ptr }
        };
        let context = model_ref
            .new_context(backend, context_params)
            .map_err(|source| EmbeddingError::Backend {
                message: format!("llama.cpp failed to create embedding context: {source}"),
            })?;
        Ok(Self { context, model })
    }

    #[requires(true)]
    #[ensures(true)]
    fn model(&self) -> &LlamaModel {
        &self.model
    }

    #[requires(true)]
    #[ensures(true)]
    fn context_mut(&mut self) -> &mut LlamaContext<'static> {
        &mut self.context
    }
}

#[derive(Debug)]
#[invariant(true)]
pub struct NativeLlamaEmbeddingBackend {
    owned: OwnedLlamaContext,
    dimensions: usize,
    max_tokens_per_call: usize,
}

impl NativeLlamaEmbeddingBackend {
    #[requires(path.is_file())]
    #[ensures(ret.as_ref().is_ok_and(|backend| backend.dimensions == spec.dimensions) || ret.is_err())]
    pub fn load(spec: &EmbeddingModelSpec, path: &Path) -> Result<Self, EmbeddingError> {
        let threads = std::thread::available_parallelism()
            .map(|count| count.get().min(N_PARALLEL))
            .unwrap_or(1)
            .max(1);
        let context_params = LlamaContextParams::default()
            .with_embeddings(true)
            .with_n_ctx(NonZeroU32::new(N_CTX))
            .with_n_batch(N_BATCH)
            .with_n_ubatch(N_UBATCH)
            // Use the same single-sequence call for documents and queries.
            .with_n_seq_max(1)
            .with_kv_unified(true)
            .with_n_threads(threads as i32)
            .with_n_threads_batch(threads as i32);
        let owned = OwnedLlamaContext::load(path, context_params)?;
        let dimensions =
            usize::try_from(owned.model().n_embd_out()).map_err(|_| EmbeddingError::Backend {
                message: "llama.cpp reported invalid embedding dimension".to_owned(),
            })?;
        if dimensions != spec.dimensions {
            return Err(EmbeddingError::DimensionMismatch {
                expected: spec.dimensions,
                actual: dimensions,
            });
        }
        let max_tokens_per_call =
            usize::try_from(owned.context.n_ubatch()).map_err(|_| EmbeddingError::Backend {
                message: "llama.cpp reported invalid n_ubatch".to_owned(),
            })?;
        if max_tokens_per_call != crate::NATIVE_MAX_WINDOW_TOKENS as usize {
            return Err(EmbeddingError::InvalidModel {
                message: "llama.cpp window size differs from the native vector function".to_owned(),
            });
        }
        Ok(Self {
            owned,
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
        let has_encoder_only =
            self.owned.model().has_encoder() && !self.owned.model().has_decoder();
        let context = self.owned.context_mut();
        clear_all_sequences(context)?;
        let mut batch = LlamaBatch::new(tokens.len(), 1);
        for (index, token) in tokens.iter().enumerate() {
            batch
                .add(*token, index as i32, &[0], true)
                .map_err(|source| EmbeddingError::Backend {
                    message: format!("llama.cpp failed to prepare embedding batch: {source}"),
                })?;
        }
        if has_encoder_only {
            context
                .encode(&mut batch)
                .map_err(|source| EmbeddingError::Backend {
                    message: format!("llama.cpp embedding encode failed: {source}"),
                })?;
        } else {
            context
                .decode(&mut batch)
                .map_err(|source| EmbeddingError::Backend {
                    message: format!("llama.cpp embedding decode failed: {source}"),
                })?;
        }
        let embedding = context
            .embeddings_seq_ith(0)
            .or_else(|_| context.embeddings_ith(tokens.len() as i32 - 1))
            .map_err(|source| EmbeddingError::Backend {
                message: format!("llama.cpp did not return an embedding: {source}"),
            })?;
        let mut values = embedding.to_vec();
        normalize_vector(&mut values, self.dimensions)?;
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
    _level: llama_cpp_sys_4::ggml_log_level,
    _text: *const core::ffi::c_char,
    _user_data: *mut core::ffi::c_void,
) {
}

/// Forget every sequence's cells before a new call.
///
/// `seq_rm` over all sequences frees the cells without writing to the cache;
/// `clear_kv_cache` would also zero the whole buffer (hundreds of MiB for the
/// larger models) on every search query.
#[requires(true)]
#[ensures(true)]
fn clear_all_sequences(context: &mut LlamaContext<'static>) -> Result<(), EmbeddingError> {
    context
        .clear_kv_cache_seq(None, None, None)
        .map(|_| ())
        .map_err(|source| EmbeddingError::Backend {
            message: format!("llama.cpp failed to clear context memory: {source}"),
        })
}

impl NativeLlamaEmbeddingBackend {
    /// Tokenize `input` and split it into windows of at most one call's
    /// token budget, exactly as `embed` does.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|windows| !windows.is_empty()) || ret.is_err())]
    fn token_windows(
        &self,
        input: &str,
    ) -> Result<Vec<Vec<llama_cpp_4::token::LlamaToken>>, EmbeddingError> {
        let tokens = crate::tokenize_native_input(input, |normalized| {
            self.owned
                .model()
                .str_to_token(normalized, AddBos::Never)
                .map_err(|source| EmbeddingError::Backend {
                    message: format!("llama.cpp tokenization failed: {source}"),
                })
        })?;
        let rule = jbotci_f2llm_runtime::TokenWindowRule::new(self.max_tokens_per_call)
            .map_err(|message| EmbeddingError::Backend { message })?;
        Ok(rule.split(&tokens, self.owned.model().token_eos()))
    }
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
        let windows = self.token_windows(input)?;
        if let [window] = windows.as_slice() {
            return Ok(QueryEmbedding {
                values: self.embed_tokens(window)?,
            });
        }
        let mut pooled = vec![0.0; self.dimensions];
        let mut token_count = 0usize;
        for window in &windows {
            let embedding = self.embed_tokens(window)?;
            for (accumulator, value) in pooled.iter_mut().zip(embedding.iter()) {
                *accumulator += *value * window.len() as f32;
            }
            token_count += window.len();
        }
        // Weight windows by token count so a short tail window contributes in
        // proportion to its share of the source sequence.
        for value in &mut pooled {
            *value /= token_count as f32;
        }
        normalize_vector(&mut pooled, self.dimensions)?;
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
    if options.reuse_index_dir.is_some() && options.skip_validation {
        return Err(EmbeddingError::InvalidIndex {
            message: "--reuse-index-dir conflicts with --skip-validation".to_owned(),
        });
    }
    let source = options
        .reuse_index_dir
        .as_ref()
        .map(|root| {
            validate_native_reuse_source_with_adoption(root, &spec, options.adopt_legacy_pack)
        })
        .transpose()?;
    let model_path = model_file_path(&model_root, &spec);
    let dictionary = jbotci_dictionary_data::english();
    let cll_site =
        jbotci_cll::embedded_cll_site().map_err(|error| EmbeddingError::InvalidIndex {
            message: error.to_string(),
        })?;
    let cll_chunks = jbotci_cll::cll_search_all_chunks(cll_site);

    if source.is_none()
        && !options.force
        && let Some(mut report) = reuse_existing_embedding_pack_with_progress(
            dictionary,
            cll_chunks,
            &index_root,
            &spec,
            progress,
        )?
    {
        ensure_model_file_with_progress(
            &spec,
            &model_path,
            false,
            options.skip_validation,
            progress,
        )?;
        report.model_path = Some(model_path);
        return Ok(report);
    }

    ensure_model_file_with_progress(
        &spec,
        &model_path,
        options.force && options.reuse_index_dir.is_none(),
        options.skip_validation,
        progress,
    )?;
    match if source.is_some() {
        UsePrecomputed::Never
    } else {
        options.use_precomputed
    } {
        UsePrecomputed::Always => {
            let mut report = download_precomputed_embedding_pack_with_progress(
                dictionary,
                cll_chunks,
                &index_root,
                &spec,
                &options.precomputed_base_url,
                progress,
            )?;
            report.model_path = Some(model_path);
            return Ok(report);
        }
        UsePrecomputed::Auto => {
            match download_precomputed_embedding_pack_with_progress(
                dictionary,
                cll_chunks,
                &index_root,
                &spec,
                &options.precomputed_base_url,
                progress,
            ) {
                Ok(mut report) => {
                    report.model_path = Some(model_path);
                    return Ok(report);
                }
                Err(error) => {
                    progress(SetupProgress::indeterminate(
                        SetupProgressPhase::Indexing,
                        "index",
                        "Indexing locally",
                        &format!("Precomputed embedding index unavailable: {error}"),
                    ));
                }
            }
        }
        UsePrecomputed::Never => {}
    }
    progress(SetupProgress::indeterminate(
        SetupProgressPhase::LoadingModel,
        "load",
        "Loading model",
        "Loading embedding model with llama.cpp.",
    ));
    let mut backend = NativeLlamaEmbeddingBackend::load(&spec, &model_path)?;
    let mut report = build_embedding_pack_with_reuse(
        &mut backend,
        dictionary,
        cll_chunks,
        &index_root,
        &spec,
        options.force || source.is_some(),
        source,
        (!options.skip_validation).then(|| spec.native_sha256.clone()),
        progress,
    )?;
    report.model_path = Some(model_path);
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
    crate::load_validated_search_backend(&spec, &model_path, NativeLlamaEmbeddingBackend::load)
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
        crate::load_latest_pack(&index_root, model_key)?;
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

    /// Ranked entries among those `entry_allowed` accepts; see
    /// [`crate::semantic_vlacku_hits_filtered`].
    #[requires(!query.trim().is_empty())]
    #[requires(count > 0)]
    #[ensures(ret.as_ref().is_ok_and(|hits| hits.len() <= count) || ret.is_err())]
    pub fn semantic_vlacku_hits_filtered<F>(
        &mut self,
        query: &str,
        count: usize,
        entry_allowed: F,
    ) -> Result<Vec<crate::DictionarySemanticHit>, EmbeddingError>
    where
        F: FnMut(usize) -> bool,
    {
        crate::semantic_vlacku_hits_filtered(
            &mut self.backend,
            query,
            count,
            &self.index_root,
            &self.model_key,
            entry_allowed,
        )
    }

    #[requires(!query.trim().is_empty())]
    #[ensures(true)]
    pub fn semantic_cukta_output(
        &mut self,
        chunks: &[jbotci_cll::CllSearchChunk],
        query: &str,
        window: jbotci_cll::CuktaSearchWindow,
        targets: jbotci_cll::CuktaTargetFilter,
    ) -> Result<jbotci_cll::CuktaSearchOutput, EmbeddingError> {
        semantic_cukta_output(
            &mut self.backend,
            chunks,
            query,
            window,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires the pinned 80M GGUF file"]
    #[requires(true)]
    #[ensures(true)]
    fn native_and_browser_windows_match_for_pinned_80m_tokenizer() {
        let path = std::env::var_os("JBOTCI_F2LLM_80M_TOKENIZER_GGUF").expect("80M GGUF path");
        let spec = model_spec("f2llm-v2-80m-q4-k-m-320").unwrap();
        assert_eq!(
            crate::sha256_hex_file(Path::new(&path)).unwrap(),
            spec.native_sha256
        );
        let mut backend = NativeLlamaEmbeddingBackend::load(&spec, Path::new(&path)).unwrap();
        assert!(!backend.owned.model().get_vocab().get_add_bos());
        assert!(backend.owned.model().add_eos_token());
        assert_eq!(backend.owned.model().token_eos().0, 151645);
        let fixture: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../jbotci-f2llm-runtime/testdata/tokenizer/pinned.json"
        ))
        .unwrap();
        let browser = jbotci_f2llm_runtime::QwenByteBpeTokenizer::from_compact_json(
            &serde_json::to_vec(&fixture["tokenizer"]).unwrap(),
        )
        .unwrap();
        for max_length in [8, 512, 2048] {
            backend.max_tokens_per_call = max_length;
            for case in fixture["cases"].as_array().unwrap() {
                let text = case["text"].as_str().unwrap();
                let native = backend
                    .token_windows(text)
                    .unwrap()
                    .into_iter()
                    .map(|w| {
                        w.into_iter()
                            .map(|token| token.0 as u32)
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    native,
                    browser.token_windows(text, max_length).unwrap(),
                    "{text:?}, {max_length}"
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn search_load_hashes_the_installed_file_before_loading_llama() {
        let root = tempfile::tempdir().unwrap();
        let spec = model_spec("f2llm-v2-80m-q4-k-m-320").unwrap();
        let path = model_file_path(root.path(), &spec);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // A sparse file has the pinned length without any model weights.
        // Search must reject its hash before it initializes llama.cpp.
        std::fs::File::create(&path)
            .unwrap()
            .set_len(spec.native_size_bytes)
            .unwrap();
        let error =
            load_backend_for_search(&spec.model_key, Some(root.path().to_owned())).unwrap_err();
        assert!(matches!(error, EmbeddingError::InvalidModel { .. }));
        assert!(error.to_string().contains("SHA-256 mismatch"));
    }
}
