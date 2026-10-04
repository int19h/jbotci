//! Native llama.cpp embedding backend.

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Once, OnceLock};

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use llama_cpp_4::context::LlamaContext;
use llama_cpp_4::context::params::LlamaContextParams;
use llama_cpp_4::llama_backend::LlamaBackend;
use llama_cpp_4::llama_batch::LlamaBatch;
use llama_cpp_4::model::params::LlamaModelParams;
use llama_cpp_4::model::{AddBos, LlamaModel};

use crate::{
    EmbeddingBackend, EmbeddingError, EmbeddingModelSpec, QueryEmbedding, SetupOptions,
    SetupProgress, SetupProgressCallback, SetupProgressPhase, SetupReport, UsePrecomputed,
    build_embedding_pack_with_progress, default_index_root, default_model_root,
    download_precomputed_embedding_pack_with_progress, ensure_model_file_with_progress,
    model_file_path, model_spec, normalize_vector, reuse_existing_embedding_pack_with_progress,
    semantic_cukta_output, semantic_vlacku_hits,
};

const N_BATCH: u32 = 2048;
const N_UBATCH: u32 = 2048;
const N_CTX: u32 = 2048;
const N_PARALLEL: usize = 32;
/// The most inputs one decode call carries in `embed_batch`. The token budget
/// stays N_UBATCH whatever this is, so it costs no memory; it only lets many
/// short inputs (most dictionary entries are tens of tokens) share a call.
/// 64 was the fastest setting measured on Metal (about twice one sequence per
/// call); on the CPU it neither helps nor hurts.
const N_SEQ_MAX: u32 = 64;

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
            .with_n_seq_max(N_SEQ_MAX)
            // One shared cache, so a call carrying many sequences can still
            // give any one of them the whole N_CTX.
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

/// One token window of one input in `embed_batch`.
#[invariant(!tokens.is_empty())]
#[derive(Debug)]
struct BatchWindow {
    input_index: usize,
    tokens: Vec<llama_cpp_4::token::LlamaToken>,
}

impl NativeLlamaEmbeddingBackend {
    /// Tokenize `input` and split it into windows of at most one call's
    /// token budget, exactly as `embed` does.
    #[requires(!input.is_empty())]
    #[ensures(ret.as_ref().is_ok_and(|windows| !windows.is_empty()) || ret.is_err())]
    fn token_windows(
        &self,
        input: &str,
    ) -> Result<Vec<Vec<llama_cpp_4::token::LlamaToken>>, EmbeddingError> {
        let tokens = self
            .owned
            .model()
            .str_to_token(input, AddBos::Always)
            .map_err(|source| EmbeddingError::Backend {
                message: format!("llama.cpp tokenization failed: {source}"),
            })?;
        if tokens.is_empty() {
            return Err(EmbeddingError::Backend {
                message: "cannot embed an empty token sequence".to_owned(),
            });
        }
        Ok(tokens
            .chunks(self.max_tokens_per_call.max(1))
            .map(<[_]>::to_vec)
            .collect())
    }

    /// Embed `windows` in one decode call, one sequence per window, returning
    /// each window's normalized embedding in order.
    #[requires(!windows.is_empty() && windows.len() <= N_SEQ_MAX as usize)]
    #[requires(windows.iter().map(|window| window.tokens.len()).sum::<usize>() <= self.max_tokens_per_call)]
    #[ensures(ret.as_ref().is_ok_and(|embeddings| {
        embeddings.len() == windows.len()
            && embeddings.iter().all(|embedding| embedding.len() == self.dimensions)
    }) || ret.is_err())]
    fn embed_window_group(
        &mut self,
        windows: &[&BatchWindow],
    ) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let has_encoder_only =
            self.owned.model().has_encoder() && !self.owned.model().has_decoder();
        let total_tokens = windows
            .iter()
            .map(|window| window.tokens.len())
            .sum::<usize>();
        let dimensions = self.dimensions;
        let context = self.owned.context_mut();
        clear_all_sequences(context)?;
        // The second argument is the number of sequence ids per token, and
        // each token here belongs to exactly one sequence.
        let mut batch = LlamaBatch::new(total_tokens, 1);
        // Batch index of each sequence's last token, for models whose GGUF
        // metadata asks for no pooling: there the last token's output is the
        // embedding, as in `embed_tokens`.
        let mut last_token_indices = Vec::with_capacity(windows.len());
        for (sequence, window) in windows.iter().enumerate() {
            for (position, token) in window.tokens.iter().enumerate() {
                batch
                    .add(*token, position as i32, &[sequence as i32], true)
                    .map_err(|source| EmbeddingError::Backend {
                        message: format!("llama.cpp failed to prepare embedding batch: {source}"),
                    })?;
            }
            last_token_indices.push(batch.n_tokens() - 1);
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
        let mut embeddings = Vec::with_capacity(windows.len());
        for (sequence, last_token_index) in last_token_indices.into_iter().enumerate() {
            let embedding = context
                .embeddings_seq_ith(sequence as i32)
                .or_else(|_| context.embeddings_ith(last_token_index))
                .map_err(|source| EmbeddingError::Backend {
                    message: format!("llama.cpp did not return an embedding: {source}"),
                })?;
            if embedding.len() != dimensions {
                return Err(EmbeddingError::DimensionMismatch {
                    expected: dimensions,
                    actual: embedding.len(),
                });
            }
            let mut values = embedding.to_vec();
            normalize_vector(&mut values);
            embeddings.push(values);
        }
        Ok(embeddings)
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
        normalize_vector(&mut pooled);
        Ok(QueryEmbedding { values: pooled })
    }

    /// Embed many inputs with several windows per decode call.
    ///
    /// Each input is tokenized and windowed exactly as `embed` does it, and
    /// its windows are pooled the same way (a single window as is, several by
    /// token-weighted mean). Only the packing differs: consecutive windows
    /// share a call up to N_SEQ_MAX sequences and the N_UBATCH token budget,
    /// each in its own sequence, so attention never crosses inputs. The
    /// results differ slightly from `embed` (lowest cosine 0.9985
    /// in the 200-document probe), because the arithmetic is grouped
    /// differently; a different CPU build changes them about as much.
    #[requires(inputs.iter().all(|input| !input.is_empty()))]
    #[ensures(ret.as_ref().is_ok_and(|embeddings| {
        embeddings.len() == inputs.len()
            && embeddings.iter().all(|embedding| embedding.values.len() == self.dimensions)
    }) || ret.is_err())]
    fn embed_batch(&mut self, inputs: &[&str]) -> Result<Vec<QueryEmbedding>, EmbeddingError> {
        let mut windows = Vec::new();
        for (input_index, input) in inputs.iter().enumerate() {
            windows.extend(self.token_windows(input)?.into_iter().map(|tokens| {
                new!(BatchWindow {
                    input_index: input_index,
                    tokens: tokens
                })
            }));
        }
        let mut pooled = vec![vec![0.0f32; self.dimensions]; inputs.len()];
        let mut token_counts = vec![0usize; inputs.len()];
        let mut start = 0;
        while start < windows.len() {
            let mut end = start;
            let mut tokens = 0;
            while end < windows.len()
                && end - start < N_SEQ_MAX as usize
                && tokens + windows[end].tokens.len() <= self.max_tokens_per_call
            {
                tokens += windows[end].tokens.len();
                end += 1;
            }
            let group = windows[start..end].iter().collect::<Vec<_>>();
            let embeddings = self.embed_window_group(&group)?;
            for (window, embedding) in group.iter().zip(embeddings) {
                let weight = window.tokens.len() as f32;
                for (accumulator, value) in pooled[window.input_index].iter_mut().zip(&embedding) {
                    *accumulator += value * weight;
                }
                token_counts[window.input_index] += window.tokens.len();
            }
            start = end;
        }
        Ok(pooled
            .into_iter()
            .zip(token_counts)
            .map(|(mut values, token_count)| {
                // A single window is already normalized, and dividing by its
                // own weight undoes the weighting, so one rule covers both
                // the single-window and the multi-window case of `embed`.
                for value in &mut values {
                    *value /= token_count as f32;
                }
                normalize_vector(&mut values);
                QueryEmbedding { values }
            })
            .collect())
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
    let dictionary = jbotci_dictionary_data::english();
    let cll_site =
        jbotci_cll::embedded_cll_site().map_err(|error| EmbeddingError::InvalidIndex {
            message: error.to_string(),
        })?;
    let cll_chunks = jbotci_cll::cll_search_all_chunks(cll_site);

    if !options.force
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
        options.force,
        options.skip_validation,
        progress,
    )?;
    match options.use_precomputed {
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
    let mut report = build_embedding_pack_with_progress(
        &mut backend,
        dictionary,
        cll_chunks,
        &index_root,
        &spec,
        options.force,
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

    /// Batched embedding must agree with one-at-a-time `embed`, which is what
    /// queries use at search time. The inputs exercise every packing path: a
    /// long input of varied text spanning several windows (so a dropped,
    /// doubled or misweighted window changes its vector), short inputs that
    /// share a call with its tail window, and more short inputs than one
    /// call's sequence limit. Needs a real GGUF model, which CI does not
    /// have: run with `--ignored` and point `JBOTCI_TEST_GGUF_MODEL` at an
    /// F2LLM 80m Q4_K_M file.
    #[test]
    #[ignore = "needs a GGUF model file; set JBOTCI_TEST_GGUF_MODEL and run with --ignored"]
    #[requires(true)]
    #[ensures(true)]
    fn embed_batch_agrees_with_embed() {
        let model_path = PathBuf::from(
            std::env::var_os("JBOTCI_TEST_GGUF_MODEL")
                .expect("JBOTCI_TEST_GGUF_MODEL must name the F2LLM 80m GGUF model"),
        );
        let spec = model_spec("f2llm-v2-80m-q4-k-m-320").expect("the 80m model is registered");
        let mut backend =
            NativeLlamaEmbeddingBackend::load(&spec, &model_path).expect("the model loads");
        let entries = jbotci_dictionary_data::english().entries();

        // Varied text, long enough for at least three windows.
        let mut long = String::new();
        for entry in entries {
            long.push_str(entry.definition);
            long.push('\n');
            if backend
                .token_windows(&long)
                .map_or(0, |windows| windows.len())
                >= 3
            {
                break;
            }
        }
        let windows = backend.token_windows(&long).expect("tokenizes");
        assert!(
            windows.len() >= 3,
            "the long input must span several windows"
        );
        let first = backend
            .embed_tokens(&windows[0])
            .expect("first window embeds");
        let last = backend
            .embed_tokens(windows.last().expect("has windows"))
            .expect("last window embeds");
        let window_cosine = first.iter().zip(&last).map(|(a, b)| a * b).sum::<f32>();
        assert!(
            window_cosine < 0.95,
            "the long input's windows must differ for the test to see pooling errors: {window_cosine}"
        );

        // More short inputs than one call carries, after the long input.
        let mut inputs = vec![long.as_str()];
        inputs.extend(
            entries
                .iter()
                .map(|entry| entry.word)
                .filter(|word| !word.is_empty())
                .take(N_SEQ_MAX as usize + 6),
        );
        let batched = backend.embed_batch(&inputs).expect("batch embeds");
        let single = inputs
            .iter()
            .map(|input| backend.embed(input).expect("single embeds").values)
            .collect::<Vec<_>>();
        let cosine = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
        for (index, (input, batched)) in inputs.iter().zip(&batched).enumerate() {
            let own = cosine(&batched.values, &single[index]);
            assert!(own > 0.998, "{input:.40}: cosine with embed {own}");
            let nearest_other = single
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .map(|(_, other)| cosine(&batched.values, other))
                .fold(f32::MIN, f32::max);
            assert!(
                own > nearest_other,
                "{input:.40}: closer to another input ({nearest_other}) than to its own embed ({own})"
            );
        }
    }
}
