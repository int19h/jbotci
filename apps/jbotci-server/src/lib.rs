//! HTTP server for the jbotci web app and API integrations.

#[cfg(test)]
use jbotci_web_core::GentufaWebResult;

mod delivery;
mod error;
mod execution;
use execution::{AdmittedWorkContext, ServerExecution};
pub use policy::{PolicyError, ServerPolicy};
mod policy;
mod work;

mod discord;
mod mcp;

pub use discord::register_discord_commands_from_env;

use std::future::Future;
use std::net::SocketAddr;
use std::num::NonZeroUsize;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use axum::body::Body;
use axum::extract::Extension;
use axum::http::header::{
    self, ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE, HOST, HeaderMap,
    HeaderValue, LOCATION, VARY,
};
use axum::http::{HeaderName, Method, Request, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires, try_new};
use dioxus::server::FullstackState;
use jbotci_cli::{
    GimfihiSourceWordKind, ToolCuktaRequest, ToolEmbeddingSearchService, ToolExecutionContext,
    ToolGimfihiRequest, ToolRenderedOutput, ToolStatus, ToolVlackuRequest,
};
use jbotci_cll::{
    CuktaSearchOutput, CuktaSearchWindow, CuktaTargetFilter, cll_search_all_chunks,
    embedded_cll_site,
};
use jbotci_embeddings::{DictionarySemanticHit, load_latest_pack, model_spec};
use jbotci_search::vlacku::{VlackuSearchOptions, dictionary_entry_passes_vlacku_entry_filters};
use jbotci_web_core::{
    FAVICON_ASSET_PATH, GentufaExportFormat, GentufaWebRequest, MANIFEST_ASSET_PATH,
    META_BLOCK_END, META_BLOCK_START, PageMeta, WebFeatureAvailability, WebRoute,
    parse_gentufa_web_export_request, parse_web_route, web_route_url,
};
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;
use tower::ServiceExt;

use crate::work::{WorkError, WorkKeepalive, WorkLane};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

#[invariant(base_path.starts_with('/'), "server base path must be absolute")]
#[invariant(
    base_path.as_str() == "/" || !base_path.ends_with('/'),
    "server base path must not have a trailing slash except at root"
)]
#[invariant(
    base_path.trim() == base_path.as_str(),
    "server base path must not have surrounding whitespace"
)]
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub address: SocketAddr,
    pub base_path: String,
    pub public_dir: PathBuf,
}

impl ServerConfig {
    #[requires(true)]
    #[ensures(ret.base_path.starts_with('/'))]
    #[ensures(ret.base_path.as_str() == "/" || !ret.base_path.ends_with('/'))]
    pub fn new(address: SocketAddr, base_path: &str, public_dir: PathBuf) -> Self {
        new!(ServerConfig {
            address: address,
            base_path: normalize_base_path(base_path),
            public_dir: public_dir,
        })
    }
}

#[invariant(base_path.starts_with('/'), "server state base path must be absolute")]
#[invariant(
    base_path.as_str() == "/" || !base_path.ends_with('/'),
    "server state base path must not have a trailing slash except at root"
)]
#[invariant(
    base_path.trim() == base_path.as_str(),
    "server state base path must not have surrounding whitespace"
)]
#[derive(Debug, Clone)]
pub(crate) struct AppState {
    base_path: String,
    public_dir: PathBuf,
    tool_services: ToolServices,
    /// The Discord application, when this deployment configured one.
    discord: Option<Arc<discord::interaction::DiscordService>>,
    page_meta_builder: PageMetaBuilder,
}

type PageMetaTaskResult = std::result::Result<PageMeta, WorkError>;
type PageMetaFuture = Pin<Box<dyn Future<Output = PageMetaTaskResult> + Send>>;
type PageMetaBuilder = fn(String, WebRoute, AdmittedWorkContext) -> PageMetaFuture;

const UNHASHED_WEB_MODULE_ASSET_PATHS: &[&str] = &[
    "/assets/app-module-ready.js",
    "/assets/compute-worker.js",
    "/assets/embedding-worker.js",
    "/assets/model-catalog.js",
];
const WASM_BINDGEN_STABLE_MODULE_ASSET_NAMES: &[&str] = &[
    "app-module-ready.js",
    "compute.js",
    "embeddings.js",
    "model-catalog.js",
    "worker-client.js",
];

impl AppState {
    #[requires(true)]
    #[ensures(ret.base_path.starts_with('/'))]
    fn new(config: ServerConfig, page_meta_builder: PageMetaBuilder, policy: ServerPolicy) -> Self {
        let config = config.into_data();
        let tool_services = ToolServices::with_execution(ServerExecution::new(policy));
        let discord = discord::configured_service(tool_services.clone());
        new!(AppState {
            base_path: config.base_path,
            public_dir: config.public_dir,
            tool_services,
            discord,
            page_meta_builder,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn tool_services(&self) -> ToolServices {
        self.tool_services.clone()
    }

    /// The Discord application, when this deployment has one configured.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn discord_service(&self) -> Option<Arc<discord::interaction::DiscordService>> {
        self.discord.clone()
    }
}

#[derive(Debug, Clone)]
#[invariant(true)]
pub(crate) struct ToolServices {
    pub(crate) execution: Arc<ServerExecution>,
    embedding_worker: mpsc::Sender<AdmittedEmbeddingJob>,
}

/// A meaning-search query: never blank, so every worker job is a real search.
#[invariant(!text.trim().is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchQuery {
    text: String,
}

impl SearchQuery {
    #[requires(true)]
    #[ensures(ret.is_some() == !text.trim().is_empty())]
    pub(crate) fn new(text: &str) -> Option<Self> {
        try_new!(SearchQuery {
            text: text.to_owned()
        })
        .ok()
    }

    #[requires(true)]
    #[ensures(!ret.trim().is_empty())]
    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }
}

/// Why a typed meaning search produced no result.
#[invariant(::Unavailable { reason } => !reason.trim().is_empty())]
#[invariant(::Lane(_) => true)]
#[invariant(::Failed { message } => !message.trim().is_empty())]
#[derive(Debug, PartialEq)]
pub(crate) enum SemanticSearchError {
    /// This server has no usable embedding index; the reason is stable until
    /// restart.
    Unavailable { reason: String },
    /// Admission or execution failed. The error describes the work, not the index.
    Lane(WorkError),
    /// The search itself failed.
    Failed { message: String },
}

impl SemanticSearchError {
    #[requires(true)]
    #[ensures(true)]
    fn unavailable(reason: String) -> Self {
        new!(SemanticSearchError::Unavailable {
            reason: non_blank(reason, "meaning search is unavailable on this server"),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn failed(message: String) -> Self {
        new!(SemanticSearchError::Failed {
            message: non_blank(message, "meaning search failed"),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn lane(error: WorkError) -> Self {
        new!(SemanticSearchError::Lane(error))
    }
}

impl std::fmt::Display for SemanticSearchError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_data() {
            bityzba::data!(SemanticSearchError::Unavailable { reason }) => {
                formatter.write_str(reason)
            }
            bityzba::data!(SemanticSearchError::Lane(error)) => write!(formatter, "{error}"),
            bityzba::data!(SemanticSearchError::Failed { message }) => {
                write!(formatter, "meaning search failed: {message}")
            }
        }
    }
}

impl std::error::Error for SemanticSearchError {}

#[requires(!fallback.trim().is_empty())]
#[ensures(!ret.trim().is_empty())]
fn non_blank(text: String, fallback: &str) -> String {
    if text.trim().is_empty() {
        fallback.to_owned()
    } else {
        text
    }
}

/// Work for the single embedding worker thread. Each job carries the reply
/// channel of its own result type, so a dictionary-hits request can only ever
/// receive dictionary hits. The rendered variants serve the CLI-shaped
/// MCP/REST tools; the typed variants serve Discord, which presents the hits
/// itself.
#[invariant(true)]
struct AdmittedEmbeddingJob {
    job: EmbeddingJob,
    context: AdmittedWorkContext,
}

#[invariant(::Cukta { .. } => true)]
#[invariant(::Vlacku { .. } => true)]
#[invariant(::VlackuHits { .. } => true)]
#[invariant(::CuktaSearch { .. } => true)]
enum EmbeddingJob {
    Cukta {
        request: ToolCuktaRequest,
        limits: jbotci_cli::ToolExecutionLimits,
        reply: oneshot::Sender<Result<ToolRenderedOutput>>,
    },
    Vlacku {
        request: ToolVlackuRequest,
        limits: jbotci_cli::ToolExecutionLimits,
        reply: oneshot::Sender<Result<ToolRenderedOutput>>,
    },
    VlackuHits {
        query: SearchQuery,
        count: NonZeroUsize,
        /// Entry filters applied while ranking, so the fetch stays bounded
        /// by `count`.
        options: VlackuSearchOptions,
        reply:
            oneshot::Sender<std::result::Result<Vec<DictionarySemanticHit>, SemanticSearchError>>,
        /// What the caller's delivery stands on while this job runs. A search
        /// that has started keeps it, model loading included, until the job
        /// is evaluated or discarded, however long the caller waits.
        keepalive: Option<WorkKeepalive>,
    },
    CuktaSearch {
        query: SearchQuery,
        /// The stretch of the ranking the caller wants. The search ranks to
        /// the end of it and copies the book's text for it alone.
        window: CuktaSearchWindow,
        targets: CuktaTargetFilter,
        reply: oneshot::Sender<std::result::Result<CuktaSearchOutput, SemanticSearchError>>,
        keepalive: Option<WorkKeepalive>,
    },
}

impl EmbeddingJob {
    /// Whether the requester stopped waiting (deadline passed, connection
    /// gone). Such a job is discarded before any expensive evaluation.
    #[requires(true)]
    #[ensures(true)]
    fn abandoned(&self) -> bool {
        match self {
            Self::Cukta { reply, .. } | Self::Vlacku { reply, .. } => reply.is_closed(),
            Self::VlackuHits { reply, .. } => reply.is_closed(),
            Self::CuktaSearch { reply, .. } => reply.is_closed(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn expire(self) {
        let error = WorkError::TimedOut {
            lane: WorkLane::Compute,
        };
        match self {
            Self::Cukta { reply, .. } | Self::Vlacku { reply, .. } => {
                let _ = reply.send(Err(anyhow!(error)));
            }
            Self::VlackuHits { reply, .. } => {
                let _ = reply.send(Err(SemanticSearchError::lane(error)));
            }
            Self::CuktaSearch { reply, .. } => {
                let _ = reply.send(Err(SemanticSearchError::lane(error)));
            }
        }
    }

    /// Run against the worker's cache and deliver the result; a reply nobody
    /// awaits any more is dropped.
    #[requires(true)]
    #[ensures(true)]
    fn run(self, cache: &mut EmbeddingSearchCache) {
        match self {
            Self::Cukta {
                request,
                limits,
                reply,
            } => {
                let _ = reply.send(run_with_embedding_cache(cache, |context| {
                    jbotci_cli::run_tool_cukta_with_limits(request, Some(context), &limits)
                }));
            }
            Self::Vlacku {
                request,
                limits,
                reply,
            } => {
                let _ = reply.send(run_with_embedding_cache(cache, |context| {
                    jbotci_cli::run_tool_vlacku_with_limits(request, Some(context), &limits)
                }));
            }
            Self::VlackuHits {
                query,
                count,
                options,
                reply,
                keepalive,
            } => {
                // Dropped with the job, so the delivery it stands for outlives
                // every caller that gave up while this ran.
                let _keepalive = keepalive;
                let result = loaded_embedding_service(cache)
                    .map_err(SemanticSearchError::unavailable)
                    .and_then(|service| {
                        let dictionary = jbotci_dictionary_data::english();
                        service
                            .semantic_vlacku_hits_window_filtered(
                                query.as_str(),
                                options.skip,
                                count.get(),
                                |entry_index| {
                                    dictionary.entries().get(entry_index).is_some_and(|entry| {
                                        dictionary_entry_passes_vlacku_entry_filters(
                                            entry, &options,
                                        )
                                    })
                                },
                            )
                            .map_err(|error| SemanticSearchError::failed(error.to_string()))
                    });
                let _ = reply.send(result);
            }
            Self::CuktaSearch {
                query,
                window,
                targets,
                reply,
                keepalive,
            } => {
                let _keepalive = keepalive;
                let result = embedded_cll_site()
                    .map_err(|error| SemanticSearchError::failed(error.to_string()))
                    .and_then(|site| {
                        let service = loaded_embedding_service(cache)
                            .map_err(SemanticSearchError::unavailable)?;
                        service
                            .semantic_cukta_output(
                                cll_search_all_chunks(site),
                                query.as_str(),
                                window,
                                targets,
                            )
                            .map_err(|error| SemanticSearchError::failed(error.to_string()))
                    });
                let _ = reply.send(result);
            }
        }
    }
}

#[invariant(!message.trim().is_empty())]
#[derive(Debug, Clone)]
struct CachedEmbeddingError {
    message: String,
}

#[invariant(!value.trim().is_empty())]
#[derive(Debug, Clone)]
struct ServerEmbeddingModelKey {
    value: String,
}

impl ServerEmbeddingModelKey {
    #[requires(true)]
    #[ensures(!ret.trim().is_empty())]
    fn as_str(&self) -> &str {
        &self.value
    }
}

#[invariant(::Unloaded { .. } => true)]
#[invariant(::Loaded { .. } => true)]
#[invariant(::Unavailable { .. } => true)]
#[derive(Debug)]
enum EmbeddingSearchCache {
    Unloaded { model_key: ServerEmbeddingModelKey },
    Loaded { service: ToolEmbeddingSearchService },
    Unavailable { error: CachedEmbeddingError },
}

impl ToolServices {
    #[cfg(test)]
    #[requires(true)]
    #[ensures(true)]
    fn new() -> Self {
        Self::with_execution(ServerExecution::new(ServerPolicy::default()))
    }

    #[requires(true)]
    #[ensures(true)]
    fn with_execution(execution: Arc<ServerExecution>) -> Self {
        let startup = configured_embedding_search_startup();
        Self::with_worker_and_execution(execution, move || embedding_job_runner(startup))
    }

    /// Build the native evaluator on its own thread to preserve cache affinity.
    #[requires(true)]
    #[ensures(true)]
    fn with_worker_and_execution<B, H>(execution: Arc<ServerExecution>, build_handler: B) -> Self
    where
        B: FnOnce() -> H + Send + 'static,
        H: FnMut(EmbeddingJob),
    {
        let queue_capacity = execution.policy.work.compute_workers;
        let (sender, mut receiver) = mpsc::channel::<AdmittedEmbeddingJob>(queue_capacity);
        thread::Builder::new()
            .name("jbotci-embedding-search".to_owned())
            .spawn(move || {
                let mut handler = build_handler();
                while let Some(envelope) = receiver.blocking_recv() {
                    let AdmittedEmbeddingJob { job, context } = envelope;
                    let _context = context;
                    if job.abandoned() {
                        continue;
                    }
                    if Instant::now() >= _context.deadline {
                        job.expire();
                        continue;
                    }
                    handler(job);
                }
            })
            .expect("embedding search worker thread starts");
        Self {
            execution,
            embedding_worker: sender,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn run_cukta_admitted(
        &self,
        request: ToolCuktaRequest,
        context: &AdmittedWorkContext,
    ) -> Result<ToolRenderedOutput> {
        if request.uses_semantic_search() {
            let (reply, received) = oneshot::channel();
            self.admit_with_context(
                EmbeddingJob::Cukta {
                    request,
                    reply,
                    limits: context.execution.policy.tool_limits(),
                },
                context,
            )
            .await
            .map_err(anyhow::Error::new)?;
            await_rendered_reply(received, context.deadline).await
        } else {
            let limits = context.execution.policy.tool_limits();
            context
                .run(move || jbotci_cli::run_tool_cukta_with_limits(request, None, &limits))
                .await?
        }
    }

    #[cfg(test)]
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub(crate) async fn run_vlacku(
        &self,
        request: ToolVlackuRequest,
    ) -> Result<ToolRenderedOutput> {
        let context = self.execution.admit(None).await?;
        self.run_vlacku_admitted(request, &context).await
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn run_vlacku_admitted(
        &self,
        request: ToolVlackuRequest,
        context: &AdmittedWorkContext,
    ) -> Result<ToolRenderedOutput> {
        if request.uses_semantic_search() {
            let (reply, received) = oneshot::channel();
            self.admit_with_context(
                EmbeddingJob::Vlacku {
                    request,
                    reply,
                    limits: context.execution.policy.tool_limits(),
                },
                context,
            )
            .await
            .map_err(anyhow::Error::new)?;
            await_rendered_reply(received, context.deadline).await
        } else {
            let limits = context.execution.policy.tool_limits();
            context
                .run(move || jbotci_cli::run_tool_vlacku_with_limits(request, None, &limits))
                .await?
        }
    }

    /// Ranked dictionary entries for a meaning query, filtered by the entry
    /// filters in `options` while ranking. `deadline` bounds both the wait
    /// for the worker and the result; `keepalive` is what the started search
    /// holds onto whether or not anyone is still waiting for it.
    #[cfg(test)]
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|hits| hits.len() <= count.get()) || ret.is_err())]
    pub(crate) async fn semantic_vlacku_hits(
        &self,
        query: SearchQuery,
        count: NonZeroUsize,
        options: VlackuSearchOptions,
        deadline: Option<Instant>,
        keepalive: Option<WorkKeepalive>,
    ) -> std::result::Result<Vec<DictionarySemanticHit>, SemanticSearchError> {
        let context = self
            .execution
            .admit(deadline)
            .await
            .map_err(SemanticSearchError::lane)?;
        self.semantic_vlacku_hits_admitted(query, count, options, &context, keepalive)
            .await
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn semantic_vlacku_hits_admitted(
        &self,
        query: SearchQuery,
        count: NonZeroUsize,
        options: VlackuSearchOptions,
        context: &AdmittedWorkContext,
        keepalive: Option<WorkKeepalive>,
    ) -> std::result::Result<Vec<DictionarySemanticHit>, SemanticSearchError> {
        let (reply, received) = oneshot::channel();
        self.admit_with_context(
            EmbeddingJob::VlackuHits {
                query,
                count,
                options,
                reply,
                keepalive,
            },
            context,
        )
        .await?;
        await_typed_reply(received, Some(context.deadline)).await
    }

    /// Meaning search over the CLL from the server's embedding index, for the
    /// stretch of the ranking `window` names.

    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn semantic_cukta_search_admitted(
        &self,
        query: SearchQuery,
        window: CuktaSearchWindow,
        targets: CuktaTargetFilter,
        context: &AdmittedWorkContext,
        keepalive: Option<WorkKeepalive>,
    ) -> std::result::Result<CuktaSearchOutput, SemanticSearchError> {
        let (reply, received) = oneshot::channel();
        self.admit_with_context(
            EmbeddingJob::CuktaSearch {
                query,
                window,
                targets,
                reply,
                keepalive,
            },
            context,
        )
        .await?;
        await_typed_reply(received, Some(context.deadline)).await
    }

    #[requires(true)]
    #[ensures(true)]
    async fn admit_with_context(
        &self,
        job: EmbeddingJob,
        context: &AdmittedWorkContext,
    ) -> std::result::Result<(), SemanticSearchError> {
        let lane = WorkLane::Embedding;
        let deadline = Some(context.deadline);
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(SemanticSearchError::lane(WorkError::TimedOut { lane }));
        }
        let not_running = || SemanticSearchError::lane(WorkError::Panicked { lane });
        let permit = match deadline {
            Some(deadline) => {
                match tokio::time::timeout_at(deadline, self.embedding_worker.reserve()).await {
                    Ok(Ok(permit)) => permit,
                    Ok(Err(_closed)) => return Err(not_running()),
                    Err(_elapsed) => {
                        return Err(SemanticSearchError::lane(WorkError::TimedOut { lane }));
                    }
                }
            }
            None => self
                .embedding_worker
                .reserve()
                .await
                .map_err(|_closed| not_running())?,
        };
        permit.send(AdmittedEmbeddingJob {
            job,
            context: context.clone(),
        });
        Ok(())
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
async fn await_rendered_reply(
    received: oneshot::Receiver<Result<ToolRenderedOutput>>,
    deadline: Instant,
) -> Result<ToolRenderedOutput> {
    tokio::time::timeout_at(deadline, received)
        .await
        .map_err(|_| {
            anyhow!(WorkError::TimedOut {
                lane: WorkLane::Compute
            })
        })?
        .map_err(|_| {
            anyhow!(WorkError::Panicked {
                lane: WorkLane::Embedding
            })
        })?
}

/// Await a typed reply; past the deadline the caller gives up (the job is
/// then discarded by the worker before evaluation, or its result dropped).
#[requires(true)]
#[ensures(true)]
async fn await_typed_reply<T>(
    received: oneshot::Receiver<std::result::Result<T, SemanticSearchError>>,
    deadline: Option<Instant>,
) -> std::result::Result<T, SemanticSearchError> {
    let outcome = match deadline {
        Some(deadline) => match tokio::time::timeout_at(deadline, received).await {
            Ok(outcome) => outcome,
            Err(_elapsed) => {
                return Err(SemanticSearchError::lane(WorkError::TimedOut {
                    lane: WorkLane::Embedding,
                }));
            }
        },
        None => received.await,
    };
    outcome.map_err(|_stopped| {
        SemanticSearchError::lane(WorkError::Panicked {
            lane: WorkLane::Embedding,
        })
    })?
}

/// The production job handler: owns the embedding cache for the worker's
/// lifetime and evaluates jobs against it.
#[requires(true)]
#[ensures(true)]
fn embedding_job_runner(
    startup: std::result::Result<ServerEmbeddingModelKey, CachedEmbeddingError>,
) -> impl FnMut(EmbeddingJob) {
    let mut cache = embedding_search_cache_from_startup(startup);
    move |job: EmbeddingJob| job.run(&mut cache)
}

/// Load the embedding service on first use; afterwards the cache is either
/// `Loaded` or permanently `Unavailable` with the reason.
#[requires(true)]
#[ensures(!matches!(cache, EmbeddingSearchCache::Unloaded { .. }))]
fn ensure_embedding_service_loaded(cache: &mut EmbeddingSearchCache) {
    if let EmbeddingSearchCache::Unloaded { model_key } = cache {
        let model_key = model_key.clone();
        *cache = match load_server_embedding_search_service(model_key.as_str()) {
            Ok(service) => EmbeddingSearchCache::Loaded { service },
            Err(error) => EmbeddingSearchCache::Unavailable {
                error: new!(CachedEmbeddingError {
                    message: format!(
                        "server embedding artifacts for `{}` are unavailable: {error}",
                        model_key.as_str()
                    ),
                }),
            },
        };
    }
}

/// The loaded embedding service, or the cached reason it is unavailable.
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.trim().is_empty()))]
fn loaded_embedding_service(
    cache: &mut EmbeddingSearchCache,
) -> std::result::Result<&mut ToolEmbeddingSearchService, String> {
    ensure_embedding_service_loaded(cache);
    match cache {
        EmbeddingSearchCache::Loaded { service } => Ok(service),
        EmbeddingSearchCache::Unavailable { error } => Err(error.message.clone()),
        EmbeddingSearchCache::Unloaded { .. } => {
            unreachable!("embedding search cache is initialized")
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_with_embedding_cache<T>(
    cache: &mut EmbeddingSearchCache,
    runner: impl FnOnce(&mut ToolExecutionContext<'_>) -> Result<T>,
) -> Result<T> {
    ensure_embedding_service_loaded(cache);
    match cache {
        EmbeddingSearchCache::Loaded { service } => {
            let mut context = ToolExecutionContext::with_embedding_search(service);
            runner(&mut context)
        }
        EmbeddingSearchCache::Unavailable { error } => {
            let mut context =
                ToolExecutionContext::embedding_search_unavailable(error.message.clone());
            runner(&mut context)
        }
        EmbeddingSearchCache::Unloaded { .. } => {
            unreachable!("embedding search cache is initialized")
        }
    }
}

#[requires(true)]
#[ensures(
    ret.as_ref().is_ok_and(|model_key| !model_key.as_str().trim().is_empty())
        || ret.as_ref().err().is_some_and(|error| !error.message.trim().is_empty())
)]
fn configured_embedding_search_startup()
-> std::result::Result<ServerEmbeddingModelKey, CachedEmbeddingError> {
    match configured_embedding_model_key() {
        Ok(model_key) => Ok(model_key),
        Err(message) => Err(new!(CachedEmbeddingError { message })),
    }
}

#[requires(true)]
#[ensures(
    ret.as_ref().is_ok_and(|model_key| !model_key.as_str().trim().is_empty())
        || ret.as_ref().err().is_some_and(|message| !message.trim().is_empty())
)]
fn configured_embedding_model_key() -> std::result::Result<ServerEmbeddingModelKey, String> {
    let model_key = std::env::var(SERVER_EMBEDDING_MODEL_KEY_ENV)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| SERVER_DEFAULT_EMBEDDING_MODEL_KEY.to_owned());
    if model_spec(&model_key).is_none() {
        return Err(format!(
            "unsupported server embedding model `{model_key}` from {SERVER_EMBEDDING_MODEL_KEY_ENV}"
        ));
    }
    Ok(new!(ServerEmbeddingModelKey { value: model_key }))
}

#[requires(true)]
#[ensures(true)]
fn embedding_search_cache_from_startup(
    startup: std::result::Result<ServerEmbeddingModelKey, CachedEmbeddingError>,
) -> EmbeddingSearchCache {
    match startup {
        Ok(model_key) => EmbeddingSearchCache::Unloaded { model_key },
        Err(error) => EmbeddingSearchCache::Unavailable { error },
    }
}

#[requires(!model_key.trim().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|service| service.model_key() == model_key) || ret.is_err())]
fn load_server_embedding_search_service(model_key: &str) -> Result<ToolEmbeddingSearchService> {
    let service = ToolEmbeddingSearchService::load(model_key, None, None)?;
    load_latest_pack(service.index_root(), service.model_key())?;
    Ok(service)
}

#[derive(Debug, Clone, Serialize)]
#[invariant(true)]
struct HealthResponse {
    status: &'static str,
    features: WebFeatureAvailability,
}

const SERVICE_WORKER_ASSET_PATH: &str = "/service-worker.js";
const DIOXUS_PUBLIC_PATH_ENV: &str = "DIOXUS_PUBLIC_PATH";
const SERVER_EMBEDDING_MODEL_KEY_ENV: &str = "JBOTCI_SERVER_EMBEDDING_MODEL_KEY";
pub const SERVER_DEFAULT_EMBEDDING_MODEL_KEY: &str = "f2llm-v2-80m-q4-k-m-320";

#[requires(true)]
#[ensures(ret.base_path.starts_with('/'))]
pub fn config_from_env() -> ServerConfig {
    let address = dioxus::cli_config::fullstack_address_or_localhost();
    let public_dir = public_dir_from_env_or_exe(
        std::env::var_os(DIOXUS_PUBLIC_PATH_ENV).map(PathBuf::from),
        std::env::current_exe().ok().as_deref(),
    );
    ServerConfig::new(
        address,
        dioxus::cli_config::base_path().as_deref().unwrap_or("/"),
        public_dir,
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub async fn run_server(config: ServerConfig) -> Result<()> {
    let policy = ServerPolicy::from_env()?;
    let listener = tokio::net::TcpListener::bind(config.address)
        .await
        .with_context(|| format!("failed to bind `{}`", config.address))?;
    axum::serve(
        delivery::DeadlineListener(listener),
        router_with_policy(config, policy)
            .into_make_service_with_connect_info::<delivery::Connection>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("server failed")?;
    Ok(())
}

#[requires(true)]
#[ensures(true)]
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let ctrl_c = tokio::signal::ctrl_c();
        let terminate = async {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut signal) => {
                    signal.recv().await;
                }
                Err(error) => {
                    eprintln!("failed to install SIGTERM handler: {error}");
                    std::future::pending::<()>().await;
                }
            }
        };
        tokio::select! {
            result = ctrl_c => {
                if let Err(error) = result {
                    eprintln!("failed to install Ctrl-C handler: {error}");
                }
            }
            () = terminate => {}
        }
    }

    #[cfg(not(unix))]
    {
        if let Err(error) = tokio::signal::ctrl_c().await {
            eprintln!("failed to install Ctrl-C handler: {error}");
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub fn router(config: ServerConfig) -> Router {
    router_with_policy(config, ServerPolicy::default())
}

#[requires(true)]
#[ensures(true)]
pub fn router_with_policy(config: ServerConfig, policy: ServerPolicy) -> Router {
    router_with_builder_and_policy(config, production_page_meta_builder, policy)
}

#[requires(true)]
#[ensures(true)]
fn router_with_builder_and_policy(
    config: ServerConfig,
    page_meta_builder: PageMetaBuilder,
    policy: ServerPolicy,
) -> Router {
    router_with_state(Arc::new(AppState::new(config, page_meta_builder, policy)))
}

#[requires(true)]
#[ensures(true)]
fn router_with_state(state: Arc<AppState>) -> Router {
    // The MCP spec's Origin-validation MUST protects localhost servers with
    // ambient authority from DNS rebinding. It does not apply to this public,
    // stateless endpoint whose tools are nonmutating pure functions, so open
    // CORS is a deliberate owner-adjudicated deviation. Never enable
    // credentials or restore a same-origin check here. A self-hosted localhost
    // instance can be invoked by any website its operator visits; the worst
    // case for these tools is CPU consumption.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            HeaderName::from_static("mcp-protocol-version"),
            HeaderName::from_static("mcp-session-id"),
            HeaderName::from_static("last-event-id"),
        ])
        .max_age(Duration::from_secs(86400));
    let public_browser_api = Router::<FullstackState>::new()
        .route("/api/health", get(health))
        // Deliberately public: REST tools have the same exposure posture as
        // MCP. #284 tracks completing parity for the remaining CLI tools.
        .route("/api/gentufa", post(gentufa))
        .route("/api/gimfihi", post(gimfihi))
        .route("/mcp", get(mcp::mcp_get).post(mcp::mcp_post))
        .layer(cors);
    Router::<FullstackState>::new()
        .merge(public_browser_api)
        .route("/discord", post(discord::discord_post))
        .fallback(static_or_spa)
        .layer(axum::middleware::from_fn(delivery::enforce))
        .layer(Extension(Arc::clone(&state)))
        .with_state(FullstackState::headless())
}

#[requires(true)]
#[ensures(ret.starts_with('/'))]
fn normalize_base_path(base_path: &str) -> String {
    let trimmed = base_path.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return "/".to_owned();
    }
    let with_leading = if trimmed.starts_with('/') {
        trimmed.to_owned()
    } else {
        format!("/{trimmed}")
    };
    with_leading.trim_end_matches('/').to_owned()
}

#[requires(true)]
#[ensures(!ret.as_os_str().is_empty())]
fn public_dir_from_env_or_exe(
    env_public_path: Option<PathBuf>,
    current_exe: Option<&Path>,
) -> PathBuf {
    env_public_path.unwrap_or_else(|| {
        current_exe
            .and_then(Path::parent)
            .map(|parent| parent.join("public"))
            .unwrap_or_else(|| PathBuf::from("public"))
    })
}

#[requires(true)]
#[ensures(true)]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        features: WebFeatureAvailability::default(),
    })
}

#[requires(true)]
#[ensures(true)]
async fn gentufa(
    Extension(state): Extension<Arc<AppState>>,
    Extension(deadline): Extension<delivery::OperationDeadline>,
    Json(request): Json<GentufaWebRequest>,
) -> Response<Body> {
    let execution = &state.tool_services.execution;
    if let Err(error) = execution.policy.validate_sources(
        std::iter::once(request.text.as_str()).chain(request.options.dialect.as_deref()),
    ) {
        return plain_response(StatusCode::BAD_REQUEST, &error.to_string());
    }
    let context = match execution.admit(Some(deadline.0)).await {
        Ok(context) => context,
        Err(error) => return execution::work_error_response(error),
    };
    let render_limits = context.execution.policy.tool_limits().render;
    let result = match context
        .run(move || {
            jbotci_web_core::parse_gentufa_for_web_with_limits(&request, Some(render_limits))
        })
        .await
    {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            return execution::operation_error_response(anyhow!(error), context).await;
        }
        Err(error) => return execution::work_error_response(error),
    };
    execution::admitted_json_response(StatusCode::OK, result, context).await
}

#[requires(true)]
#[ensures(true)]
async fn gimfihi(
    Extension(state): Extension<Arc<AppState>>,
    Extension(deadline): Extension<delivery::OperationDeadline>,
    Json(request): Json<ToolGimfihiRequest>,
) -> Response<Body> {
    use crate::policy::ServerRequest;
    let execution = &state.tool_services.execution;
    if let Err(error) = request.validate(&execution.policy) {
        return plain_response(StatusCode::BAD_REQUEST, &error.to_string());
    }
    let context = match execution.admit(Some(deadline.0)).await {
        Ok(context) => context,
        Err(error) => return execution::work_error_response(error),
    };
    let mut limits = context.execution.policy.tool_limits();
    limits.render = jbotci_output::budget::RenderLimits::new(
        jbotci_output::budget::ByteLimit::new(
            limits
                .render
                .text
                .get()
                .min(context.execution.policy.response_bytes),
        )
        .expect("positive overlapping limits"),
        limits.render.projection,
    );
    match context
        .run(move || {
            jbotci_cli::run_tool_gimfihi_with_limits(request, GimfihiSourceWordKind::Ipa, &limits)
        })
        .await
    {
        Ok(Ok(output)) => rest_tool_output_admitted(output, context).await,
        Ok(Err(error)) => execution::operation_error_response(error, context).await,
        Err(error) => execution::work_error_response(error),
    }
}

#[requires(true)]
#[ensures(true)]
async fn rest_tool_output_admitted(
    output: ToolRenderedOutput,
    context: AdmittedWorkContext,
) -> Response<Body> {
    let owner = context.clone();
    match context
        .run(move || build_rest_tool_output(output, owner))
        .await
    {
        Ok(response) => response,
        Err(error) => execution::work_error_response(error),
    }
}
#[requires(true)]
#[ensures(true)]
fn build_rest_tool_output(
    output: ToolRenderedOutput,
    context: AdmittedWorkContext,
) -> Response<Body> {
    use std::io::Write;
    let status = match output.status {
        ToolStatus::Success => StatusCode::OK,
        ToolStatus::ValidMissing => StatusCode::NOT_FOUND,
        ToolStatus::InvalidInput => StatusCode::BAD_REQUEST,
        ToolStatus::Failure => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let policy = &context.execution.policy;
    let content_type = if output.status.is_success() {
        output.content_type.as_deref()
    } else {
        Some("text/plain; charset=utf-8")
    };
    let mut builder = Response::builder().status(status);
    if let Some(content_type) = content_type {
        builder = builder.header(CONTENT_TYPE, content_type);
    }
    let bytes = if output.status.is_success() {
        let raw_limit =
            if content_type.is_some_and(|content_type| content_type.starts_with("image/")) {
                policy.diagram.max_bytes
            } else {
                policy.text_bytes
            };
        if output.stdout.len() > raw_limit.min(policy.response_bytes) {
            return execution::work_error_response(WorkError::OutputLimit);
        }
        output.stdout
    } else {
        let limit =
            jbotci_output::budget::ByteLimit::new(policy.text_bytes.min(policy.response_bytes))
                .expect("positive overlapping limits");
        let mut writer = jbotci_output::budget::OutputBudget::new(limit).writer();
        let result = writer.write_all(output.stderr.as_bytes()).and_then(|_| {
            if let Ok(stdout) = std::str::from_utf8(&output.stdout)
                && !stdout.trim().is_empty()
            {
                writer.write_all(stdout.as_bytes())
            } else {
                Ok(())
            }
        });
        if result.is_err() {
            return execution::work_error_response(WorkError::OutputLimit);
        }
        writer.into_bytes()
    };
    builder
        .body(execution::owned_body(bytes, Some(context)))
        .expect("REST tool response")
}

#[requires(true)]
#[ensures(true)]
async fn static_or_spa(
    Extension(state): Extension<Arc<AppState>>,
    Extension(deadline): Extension<delivery::OperationDeadline>,
    headers: HeaderMap,
    uri: Uri,
) -> Response<Body> {
    let request_path = uri.path();
    if request_path == "/favicon.ico" {
        if let Some(response) = static_dir_response(
            &state.public_dir,
            FAVICON_ASSET_PATH,
            accept_encoding_header(&headers),
        )
        .await
        {
            return response;
        }
        return Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())
            .expect("favicon response builder is valid");
    }
    if is_api_request_path(request_path, &state.base_path) {
        return plain_response(StatusCode::NOT_FOUND, "not found");
    }
    if request_path == "/" || (state.base_path != "/" && request_path == state.base_path) {
        let location = landing_page_location(&state.base_path);
        return redirect_response(&location);
    }
    if let Some(format) = gentufa_export_format_for_request_path(request_path, &state.base_path) {
        return gentufa_export_response(
            format,
            uri.query().unwrap_or_default().to_owned(),
            Arc::clone(&state.tool_services.execution),
            deadline.0,
        )
        .await;
    }
    let Some(asset_path) = asset_path_for_request(request_path, &state.base_path) else {
        return plain_response(StatusCode::NOT_FOUND, "not found");
    };
    if asset_path == "/index.html" {
        return match spa_index_response(&state, &headers, &uri, deadline.0).await {
            Ok(Some(response)) => response,
            Ok(None) => plain_response(StatusCode::NOT_FOUND, "not found"),
            Err(error) => page_meta_task_failure_response(request_path, &error),
        };
    }
    if let Some(response) = static_dir_response(
        &state.public_dir,
        &asset_path,
        accept_encoding_header(&headers),
    )
    .await
    {
        return response;
    }
    plain_response(StatusCode::NOT_FOUND, "not found")
}

#[requires(true)]
#[ensures(true)]
async fn spa_index_response(
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
    deadline: Instant,
) -> std::result::Result<Option<Response<Body>>, WorkError> {
    let Some(bytes) = load_index_html_bytes(state).await else {
        return Ok(None);
    };
    let html = String::from_utf8_lossy(&bytes);
    let logical_path = strip_base_path(uri.path(), &state.base_path).unwrap_or_else(|| {
        if uri.path().starts_with('/') {
            uri.path().to_owned()
        } else {
            format!("/{}", uri.path())
        }
    });
    let route = parse_web_route(&logical_path, uri.query().unwrap_or_default());
    use crate::policy::ServerRequest;
    if let Err(error) = route.validate(&state.tool_services.execution.policy) {
        return Ok(Some(plain_response(
            StatusCode::BAD_REQUEST,
            &error.to_string(),
        )));
    }
    let context = state.tool_services.execution.admit(Some(deadline)).await?;
    let meta = (state.page_meta_builder)(state.base_path.clone(), route, context.clone()).await?;
    let render = context.execution.policy.tool_limits().render;
    let limits = jbotci_output::budget::RenderLimits::new(
        jbotci_output::budget::ByteLimit::new(render.text.get().min(render.projection.get()))
            .expect("positive text and response limits"),
        render.projection,
    );
    let html = html.into_owned();
    let origin = request_origin(headers);
    let rendered = context
        .run(move || apply_spa_head_metadata_with_limits(&html, origin.as_deref(), &meta, limits))
        .await??;
    Ok(Some(asset_response(
        StatusCode::OK,
        "/index.html",
        None,
        execution::owned_body(rendered.into_bytes(), Some(context)),
    )))
}

#[requires(true)]
#[ensures(true)]
async fn gentufa_export_response(
    format: GentufaExportFormat,
    query: String,
    execution: Arc<ServerExecution>,
    deadline: Instant,
) -> Response<Body> {
    let request = parse_gentufa_web_export_request(&query);
    if let Err(error) = execution.policy.validate_sources(
        std::iter::once(request.state.text.as_str()).chain(request.state.dialect.as_deref()),
    ) {
        return plain_response(StatusCode::BAD_REQUEST, &error.to_string());
    }
    let context = match execution.admit(Some(deadline)).await {
        Ok(context) => context,
        Err(error) => return execution::work_error_response(error),
    };
    let limits = context.execution.policy.tool_limits();
    let diagram = limits.diagram.with_data(bityzba::data! { max_bytes: limits.diagram.max_bytes.min(context.execution.policy.response_bytes) });
    let task = context
        .run(move || {
            jbotci_web_core::render_gentufa_state_web_export_with_limits(
                &request.state,
                request.script,
                format,
                Some(limits.render),
                Some(diagram),
            )
        })
        .await;
    let export = match task {
        Ok(Ok(export)) => export,
        Ok(Err(error)) => {
            return execution::operation_error_response(anyhow!(error), context).await;
        }
        Err(error) => return execution::work_error_response(error),
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, export.content_type)
        .header(CACHE_CONTROL, "no-cache")
        .body(execution::owned_body(export.bytes, Some(context)))
        .expect("gentufa export response builder is valid")
}

#[requires(base_path.starts_with('/'))]
#[ensures(true)]
async fn build_page_meta_blocking(
    base_path: String,
    route: WebRoute,
    context: AdmittedWorkContext,
) -> PageMetaTaskResult {
    let limits = context.execution.policy.tool_limits();
    context
        .run(move || {
            jbotci_web_core::blocking::build_computed_page_meta_with_limits(
                &base_path,
                &route,
                limits.render,
                limits.diagram,
            )
        })
        .await?
        .map_err(|error| {
            if error.is_output_limit() {
                WorkError::OutputLimit
            } else {
                eprintln!("metadata computation failed: {error}");
                WorkError::Panicked {
                    lane: crate::work::WorkLane::Compute,
                }
            }
        })
}

#[requires(base_path.starts_with('/'))]
#[ensures(true)]
fn production_page_meta_builder(
    base_path: String,
    route: WebRoute,
    context: AdmittedWorkContext,
) -> PageMetaFuture {
    Box::pin(build_page_meta_blocking(base_path, route, context))
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
async fn run_page_meta_task(
    context: AdmittedWorkContext,
    task: impl FnOnce() -> PageMeta + Send + 'static,
) -> PageMetaTaskResult {
    context.run(task).await
}

#[requires(request_path.starts_with('/'))]
#[ensures(true)]
fn page_meta_task_failure_response(request_path: &str, error: &WorkError) -> Response<Body> {
    // The public response intentionally omits panic and task internals. The
    // server-side report keeps the affected route path (but not its possibly
    // sensitive query) together with Tokio's panic/cancellation diagnosis.
    eprintln!("SPA metadata task failed for request path `{request_path}`: {error}");
    execution::work_error_response(error.clone())
}

#[requires(true)]
#[ensures(true)]
async fn load_index_html_bytes(state: &AppState) -> Option<Vec<u8>> {
    let index_path = state.public_dir.join("index.html");
    if let Ok(bytes) = tokio::fs::read(index_path).await {
        return Some(bytes);
    }
    None
}

#[requires(path.starts_with('/'))]
#[requires(base_path.starts_with('/'))]
#[ensures(ret.as_ref().is_none_or(|path| path.starts_with('/')))]
fn asset_path_for_request(path: &str, base_path: &str) -> Option<String> {
    if path.starts_with("/assets/") && has_file_extension(path) {
        return Some(path.to_owned());
    }
    let stripped = strip_base_path(path, base_path)?;
    if stripped == "/" || is_spa_route_path(&stripped) || !has_file_extension(&stripped) {
        return Some("/index.html".to_owned());
    }
    Some(stripped)
}

#[requires(path.starts_with('/'))]
#[requires(base_path.starts_with('/'))]
#[ensures(true)]
fn gentufa_export_format_for_request_path(
    path: &str,
    base_path: &str,
) -> Option<GentufaExportFormat> {
    match strip_base_path(path, base_path).as_deref() {
        Some("/gentufa.png") => Some(GentufaExportFormat::Png),
        Some("/gentufa.svg") => Some(GentufaExportFormat::Svg),
        _ => None,
    }
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn is_spa_route_path(path: &str) -> bool {
    path == "/gentufa"
        || path.starts_with("/gentufa/")
        || path == "/vlacku"
        || path.starts_with("/vlacku/")
        || path == "/gimfihi"
        || path == "/gimfi'i"
        || path == "/gimfi%27i"
        || path == "/cukta"
        || path.starts_with("/cukta/")
        || path == "/settings"
        || path.starts_with("/settings/")
}

#[requires(path.starts_with('/'))]
#[requires(base_path.starts_with('/'))]
#[ensures(true)]
fn is_api_request_path(path: &str, base_path: &str) -> bool {
    if path == "/mcp" || path == "/discord" || path.starts_with("/api/") {
        return true;
    }
    strip_base_path(path, base_path).is_some_and(|stripped| {
        stripped == "/mcp" || stripped == "/discord" || stripped.starts_with("/api/")
    })
}

#[requires(path.starts_with('/'))]
#[requires(base_path.starts_with('/'))]
#[ensures(ret.as_ref().is_none_or(|path| path.starts_with('/')))]
fn strip_base_path(path: &str, base_path: &str) -> Option<String> {
    if base_path == "/" {
        return Some(path.to_owned());
    }
    if path == base_path {
        return Some("/".to_owned());
    }
    let prefix = format!("{base_path}/");
    path.strip_prefix(&prefix).map(|rest| format!("/{rest}"))
}

#[requires(base_path.starts_with('/'))]
#[ensures(ret.starts_with('/'))]
fn landing_page_location(base_path: &str) -> String {
    web_route_url(base_path, &WebRoute::default())
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn has_file_extension(path: &str) -> bool {
    path.rsplit_once('/')
        .map(|(_, file_name)| file_name.contains('.'))
        .unwrap_or(false)
}

#[requires(true)]
#[ensures(true)]
fn accept_encoding_header(headers: &HeaderMap) -> Option<HeaderValue> {
    headers.get(ACCEPT_ENCODING).cloned()
}

#[requires(asset_path.starts_with('/'))]
#[ensures(true)]
async fn static_dir_response(
    static_dir: &Path,
    asset_path: &str,
    accept_encoding: Option<HeaderValue>,
) -> Option<Response<Body>> {
    safe_relative_path(asset_path)?;
    let mut request = Request::builder().uri(asset_path);
    if let Some(value) = accept_encoding {
        request = request.header(ACCEPT_ENCODING, value);
    }
    let response = ServeDir::new(static_dir)
        .precompressed_br()
        .oneshot(request.body(Body::empty()).ok()?)
        .await
        .ok()?;
    if response.status() != StatusCode::OK {
        return None;
    }
    let mut response = response.map(Body::new);
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(content_type_for_path(asset_path)),
    );
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static(cache_control_for_path(asset_path)),
    );
    append_accept_encoding_vary(headers);
    Some(response)
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn safe_relative_path(path: &str) -> Option<PathBuf> {
    let mut output = PathBuf::new();
    for component in Path::new(path.trim_start_matches('/')).components() {
        match component {
            Component::Normal(part) => output.push(part),
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => return None,
        }
    }
    Some(output)
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn asset_response(
    status: StatusCode,
    path: &str,
    content_encoding: Option<&str>,
    body: Body,
) -> Response<Body> {
    let mut response = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, content_type_for_path(path))
        .header(CACHE_CONTROL, cache_control_for_path(path));
    if let Some(encoding) = content_encoding {
        response = response.header(CONTENT_ENCODING, encoding);
    }
    response
        .body(body)
        .expect("asset response builder is valid")
}

#[requires(true)]
#[ensures(
    headers
        .get(VARY)
        .and_then(|value| value.to_str().ok())
        .is_some_and(vary_mentions_accept_encoding)
)]
fn append_accept_encoding_vary(headers: &mut HeaderMap) {
    match headers.get(VARY).and_then(|value| value.to_str().ok()) {
        Some(value) if vary_mentions_accept_encoding(value) => {}
        Some(value) => {
            let merged = format!("{value}, Accept-Encoding");
            headers.insert(
                VARY,
                HeaderValue::from_str(&merged).expect("merged Vary header is valid"),
            );
        }
        None => {
            headers.insert(VARY, HeaderValue::from_static("Accept-Encoding"));
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn vary_mentions_accept_encoding(value: &str) -> bool {
    value
        .split(',')
        .any(|name| name.trim().eq_ignore_ascii_case("accept-encoding"))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limits.projection.get()) || ret.is_err())]
fn apply_spa_head_metadata_with_limits(
    html: &str,
    origin: Option<&str>,
    meta: &PageMeta,
    limits: jbotci_output::budget::RenderLimits,
) -> std::result::Result<String, WorkError> {
    let scope = jbotci_output::budget::TextScope::new(limits.projection);
    let without_old_block = match html.find(META_BLOCK_START).and_then(|start| {
        html[start..]
            .find(META_BLOCK_END)
            .map(|end| (start, start + end + META_BLOCK_END.len()))
    }) {
        Some((start, end)) => scope.format(format_args!("{}{}", &html[..start], &html[end..])),
        None => scope.copy(html),
    };
    let title_range = without_old_block
        .find("<title")
        .and_then(|open_start| {
            without_old_block[open_start..]
                .find('>')
                .map(|end| open_start + end + 1)
        })
        .and_then(|start| {
            without_old_block[start..]
                .find("</title>")
                .map(|end| (start, start + end))
        });
    let inserted_title = title_range.is_some();
    let with_title = match title_range {
        Some((start, end)) => {
            let mut output = scope.buffer();
            output.push_str(&without_old_block[..start]);
            for character in meta.title.chars() {
                match character {
                    '&' => output.push_str("&amp;"),
                    '<' => output.push_str("&lt;"),
                    '>' => output.push_str("&gt;"),
                    character => output.push(character),
                }
            }
            output.push_str(&without_old_block[end..]);
            output.into_piece()
        }
        None => without_old_block,
    };
    let block = jbotci_web_core::render_page_head_metadata_block_with_limit(
        origin,
        meta,
        !inserted_title,
        limits.text,
    )
    .map_err(|_| WorkError::OutputLimit)?;
    let rendered = match with_title.find("</head>") {
        Some(head_end) => scope.format(format_args!(
            "{}{}{}",
            &with_title[..head_end],
            block,
            &with_title[head_end..]
        )),
        None => scope.format(format_args!("{with_title}{block}")),
    };
    scope.finish(rendered).map_err(|_| WorkError::OutputLimit)
}

#[requires(true)]
#[ensures(true)]
fn request_origin(headers: &HeaderMap) -> Option<String> {
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(HOST))
        .and_then(|value| value.to_str().ok())?
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| *value == "http" || *value == "https")
        .unwrap_or("http");
    Some(format!("{scheme}://{host}"))
}

#[requires(path.starts_with('/'))]
#[ensures(!ret.is_empty())]
fn content_type_for_path(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, extension)| extension) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        Some("json") | Some("webmanifest") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("otf") => "font/otf",
        Some("ttf") => "font/ttf",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("f32") => "application/octet-stream",
        _ => "application/octet-stream",
    }
}

#[requires(path.starts_with('/'))]
#[ensures(!ret.is_empty())]
fn cache_control_for_path(path: &str) -> &'static str {
    if path == "/index.html"
        || path == MANIFEST_ASSET_PATH
        || path == SERVICE_WORKER_ASSET_PATH
        || path == "/assets/embeddings/web/v1/catalog.json"
        || is_unhashed_web_module_asset(path)
    {
        "no-cache"
    } else {
        "public, max-age=31536000, immutable"
    }
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn is_unhashed_web_module_asset(path: &str) -> bool {
    UNHASHED_WEB_MODULE_ASSET_PATHS.contains(&path)
        || is_unhashed_wasm_bindgen_web_module_asset(path)
}

#[requires(path.starts_with('/'))]
#[ensures(true)]
fn is_unhashed_wasm_bindgen_web_module_asset(path: &str) -> bool {
    let Some(relative_path) = path.strip_prefix("/wasm/snippets/") else {
        return false;
    };
    let Some((snippet_dir, asset_path)) = relative_path.split_once("/assets/") else {
        return false;
    };
    snippet_dir.starts_with("jbotci-ui-")
        && !snippet_dir.contains('/')
        && !asset_path.contains('/')
        && WASM_BINDGEN_STABLE_MODULE_ASSET_NAMES.contains(&asset_path)
}

#[requires(!location.is_empty())]
#[ensures(true)]
fn redirect_response(location: &str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::FOUND)
        .header(
            LOCATION,
            HeaderValue::from_str(location).expect("normalized path is valid header text"),
        )
        .body(Body::empty())
        .expect("redirect response builder is valid")
}

#[requires(true)]
#[ensures(true)]
fn plain_response(status: StatusCode, message: &str) -> Response<Body> {
    (status, message.to_owned()).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::header::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
        ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_MAX_AGE,
        ACCESS_CONTROL_REQUEST_HEADERS, ACCESS_CONTROL_REQUEST_METHOD, ORIGIN,
    };
    use axum::http::{Method, Request};
    #[allow(unused_imports)]
    use bityzba::{ensures, invariant, requires};
    use ed25519_dalek::{Signer, SigningKey};
    use jbotci_cli::{
        ToolAlineSaliences, ToolCollisionScope, ToolGimfihiFormat, ToolGimfihiScorer,
        ToolGimfihiSource,
    };
    use jbotci_web_core::parse_gentufa_for_web;
    use std::sync::atomic::{AtomicU64, Ordering};
    use tower::ServiceExt;

    #[invariant(default.is_finite())]
    #[invariant(!description.is_empty())]
    #[derive(Debug, serde::Deserialize)]
    struct NumericSchemaProperty {
        default: f64,
        description: String,
        #[serde(default)]
        minimum: Option<f64>,
        #[serde(default)]
        maximum: Option<f64>,
    }

    #[invariant(!default.is_empty())]
    #[invariant(!description.is_empty())]
    #[derive(Debug, serde::Deserialize)]
    struct EnumSchemaProperty {
        default: String,
        description: String,
    }

    #[invariant(true)]
    #[derive(Debug, serde::Deserialize)]
    struct SalienceSchemaProperties {
        syllabic: NumericSchemaProperty,
        place: NumericSchemaProperty,
        manner: NumericSchemaProperty,
        voice: NumericSchemaProperty,
        nasal: NumericSchemaProperty,
        retroflex: NumericSchemaProperty,
        lateral: NumericSchemaProperty,
        aspirated: NumericSchemaProperty,
        high: NumericSchemaProperty,
        back: NumericSchemaProperty,
        round: NumericSchemaProperty,
        long: NumericSchemaProperty,
    }

    #[invariant(!description.is_empty())]
    #[derive(Debug, serde::Deserialize)]
    struct SaliencesSchemaProperty {
        default: ToolAlineSaliences,
        description: String,
        properties: SalienceSchemaProperties,
    }

    #[invariant(true)]
    #[derive(Debug, serde::Deserialize)]
    struct GimfihiSchemaProperties {
        scorer: EnumSchemaProperty,
        #[serde(rename = "c-sub")]
        c_sub: NumericSchemaProperty,
        #[serde(rename = "c-exp")]
        c_exp: NumericSchemaProperty,
        #[serde(rename = "c-skip")]
        c_skip: NumericSchemaProperty,
        #[serde(rename = "c-vwl")]
        c_vwl: NumericSchemaProperty,
        #[serde(rename = "c-flank")]
        c_flank: NumericSchemaProperty,
        normalizer: EnumSchemaProperty,
        saliences: SaliencesSchemaProperty,
    }

    #[invariant(true)]
    #[derive(Debug, serde::Deserialize)]
    struct GimfihiSchemaProjection {
        properties: GimfihiSchemaProperties,
    }

    static TEST_STATIC_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
    static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[requires(true)]
    #[ensures(ret.is_dir())]
    fn test_static_dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let ordinal = TEST_STATIC_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "jbotci-server-spa-test-{}-{nanos}-{ordinal}",
            std::process::id(),
        ));
        std::fs::create_dir_all(&dir).expect("create test static dir");
        std::fs::write(
            dir.join("index.html"),
            "<!doctype html><html><head><title>jbotci</title></head><body><div id=\"main\"></div></body></html>",
        )
        .expect("write test index");
        dir
    }

    #[requires(true)]
    #[ensures(ret.base_path.starts_with('/'))]
    fn test_config(public_dir: PathBuf) -> ServerConfig {
        test_config_with_base_path("/jbotci", public_dir)
    }

    #[requires(base_path.starts_with('/'))]
    #[ensures(ret.base_path.starts_with('/'))]
    fn test_config_with_base_path(base_path: &str, public_dir: PathBuf) -> ServerConfig {
        ServerConfig::new(
            "127.0.0.1:0".parse().expect("valid test address"),
            base_path,
            public_dir,
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn panicking_page_meta_builder(
        _base_path: String,
        _route: WebRoute,
        context: AdmittedWorkContext,
    ) -> PageMetaFuture {
        Box::pin(run_page_meta_task(context, || -> PageMeta {
            panic!("deterministic metadata worker failure")
        }))
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_public_cors_response(response: &Response<Body>) {
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("*"))
        );
        assert!(
            response
                .headers()
                .get(ACCESS_CONTROL_ALLOW_CREDENTIALS)
                .is_none()
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_public_cors_preflight(response: &Response<Body>) {
        assert_eq!(response.status(), StatusCode::OK);
        assert_public_cors_response(response);
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_METHODS),
            Some(&HeaderValue::from_static("GET,POST"))
        );
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_HEADERS),
            Some(&HeaderValue::from_static(
                "content-type,authorization,mcp-protocol-version,mcp-session-id,last-event-id"
            ))
        );
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_MAX_AGE),
            Some(&HeaderValue::from_static("86400"))
        );
    }

    #[requires(true)]
    #[ensures(true)]
    async fn response_text(response: Response<Body>) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        String::from_utf8(bytes.to_vec()).expect("utf-8 body")
    }

    #[requires(true)]
    #[ensures(true)]
    async fn response_bytes(response: Response<Body>) -> Vec<u8> {
        to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body")
            .to_vec()
    }

    #[requires(uri.starts_with('/'))]
    #[ensures(true)]
    async fn post_json(app: Router, uri: &str, value: serde_json::Value) -> Response<Body> {
        app.oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(value.to_string()))
                .expect("request"),
        )
        .await
        .expect("response")
    }

    #[requires(true)]
    #[ensures(true)]
    async fn response_json(response: Response<Body>) -> serde_json::Value {
        serde_json::from_str(&response_text(response).await).expect("response JSON")
    }

    #[requires(true)]
    #[ensures(ret == (!name.is_empty()
        && name.len() <= 64
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')))]
    fn external_api_identifier_is_safe(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= 64
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    }

    #[requires(!name.is_empty())]
    #[ensures(ret.is_object())]
    fn tool_input_schema<'a>(tools: &'a [serde_json::Value], name: &str) -> &'a serde_json::Value {
        &tools
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("missing tool `{name}`"))["inputSchema"]
    }

    #[requires(true)]
    #[requires(!key.is_empty())]
    #[ensures(true)]
    fn json_contains_key(value: &serde_json::Value, key: &str) -> bool {
        match value {
            serde_json::Value::Object(object) => object.iter().any(|(object_key, object_value)| {
                object_key == key || json_contains_key(object_value, key)
            }),
            serde_json::Value::Array(items) => {
                items.iter().any(|item| json_contains_key(item, key))
            }
            _ => false,
        }
    }

    #[requires(schema.is_object())]
    #[requires(!property.is_empty())]
    #[ensures(true)]
    fn assert_string_enum_property(schema: &serde_json::Value, property: &str, expected: &[&str]) {
        assert_string_enum_property_at(&schema["properties"][property], expected);
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_string_enum_property_at(property_schema: &serde_json::Value, expected: &[&str]) {
        // Documented enums render as an inline `oneOf` of `{const, description}`
        // (per-variant docs); a plain `enum` array is the undocumented fallback.
        let actual = if let Some(variants) = property_schema["oneOf"].as_array() {
            // Documented enums must also declare a top-level `type: string`, so
            // schema viewers do not present the field as untyped ("any").
            assert_eq!(property_schema["type"], "string", "{property_schema}");
            variants
                .iter()
                .map(|variant| {
                    assert_eq!(variant["type"], "string", "{variant}");
                    variant["const"].as_str().expect("const string")
                })
                .collect::<Vec<_>>()
        } else {
            assert_eq!(property_schema["type"], "string", "{property_schema}");
            property_schema["enum"]
                .as_array()
                .expect("enum values")
                .iter()
                .map(|value| value.as_str().expect("enum string"))
                .collect::<Vec<_>>()
        };
        assert_eq!(actual, expected, "{property_schema}");
        assert!(property_schema.get("$ref").is_none(), "{property_schema}");
    }

    #[requires(schema.is_object())]
    #[requires(!property.is_empty())]
    #[ensures(true)]
    fn assert_boolean_default_property(schema: &serde_json::Value, property: &str, default: bool) {
        let property_schema = &schema["properties"][property];
        assert_eq!(property_schema["type"], "boolean", "{property_schema}");
        assert_eq!(property_schema["default"], default, "{property_schema}");
        assert!(
            !serde_json::to_string(property_schema)
                .expect("schema JSON")
                .contains("null"),
            "{property_schema}"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn test_discord_signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    #[requires(true)]
    #[ensures(true)]
    fn lock_test_env() -> std::sync::MutexGuard<'static, ()> {
        TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[requires(!public_key_hex.is_empty())]
    #[ensures(true)]
    fn configure_discord_test_env(public_key_hex: &str) {
        // SAFETY: Discord route tests hold TEST_ENV_LOCK while mutating process-wide env vars.
        unsafe {
            std::env::set_var("DISCORD_PUBLIC_KEY", public_key_hex);
            std::env::set_var("JBOTCI_DISCORD_DISABLE_FOLLOWUP", "1");
            std::env::set_var("JBOTCI_PUBLIC_BASE_URL", "https://jbotci.app");
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn set_server_embedding_model_key_env(value: Option<&str>) {
        // SAFETY: Server embedding env tests hold TEST_ENV_LOCK while mutating process-wide env vars.
        unsafe {
            if let Some(value) = value {
                std::env::set_var(SERVER_EMBEDDING_MODEL_KEY_ENV, value);
            } else {
                std::env::remove_var(SERVER_EMBEDDING_MODEL_KEY_ENV);
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.len() == bytes.len() * 2)]
    fn hex_bytes(bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    }

    #[requires(!body.is_empty())]
    #[requires(!timestamp.is_empty())]
    #[ensures(!ret.0.is_empty() && !ret.1.is_empty())]
    fn discord_signature_headers(
        key: &SigningKey,
        body: &str,
        timestamp: &str,
    ) -> (String, String) {
        let mut signed = Vec::with_capacity(timestamp.len() + body.len());
        signed.extend_from_slice(timestamp.as_bytes());
        signed.extend_from_slice(body.as_bytes());
        let signature = key.sign(&signed);
        (timestamp.to_owned(), hex_bytes(&signature.to_bytes()))
    }

    #[requires(true)]
    #[ensures(true)]
    async fn post_signed_discord(
        app: Router,
        value: serde_json::Value,
        key: &SigningKey,
    ) -> Response<Body> {
        post_signed_discord_body(app, value.to_string(), key).await
    }

    #[requires(!body.is_empty())]
    #[ensures(true)]
    async fn post_signed_discord_body(
        app: Router,
        body: String,
        key: &SigningKey,
    ) -> Response<Body> {
        let (timestamp, signature) = discord_signature_headers(key, &body, "1710000000");
        app.oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/discord")
                .header("X-Signature-Timestamp", timestamp)
                .header("X-Signature-Ed25519", signature)
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("response")
    }

    #[requires(!name.is_empty())]
    #[requires(!value.is_empty())]
    #[ensures(ret.is_object())]
    fn discord_string_option(name: &str, value: &str) -> serde_json::Value {
        serde_json::json!({
            "name": name,
            "type": 3,
            "value": value
        })
    }

    #[requires(!subcommand.is_empty())]
    #[ensures(ret.is_object())]
    fn discord_interaction(subcommand: &str, options: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({
            "type": 2,
            "id": "555555555555555555",
            "application_id": "123456789",
            "token": "test-token",
            "member": { "user": { "id": "123456789012345678" } },
            "data": {
                "name": "jbotci",
                "options": [
                    {
                        "name": subcommand,
                        "type": 1,
                        "options": options
                    }
                ]
            }
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn base_path_normalization_keeps_single_root() {
        assert_eq!(normalize_base_path(""), "/");
        assert_eq!(normalize_base_path("/"), "/");
        assert_eq!(normalize_base_path("jbotci/"), "/jbotci");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn server_embedding_model_defaults_to_80m() {
        let _env_guard = lock_test_env();
        set_server_embedding_model_key_env(None);

        assert_eq!(
            configured_embedding_model_key()
                .expect("server default model key")
                .as_str(),
            SERVER_DEFAULT_EMBEDDING_MODEL_KEY
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn server_embedding_model_env_overrides_default() {
        let _env_guard = lock_test_env();
        set_server_embedding_model_key_env(Some(" f2llm-v2-160m-q4-k-m-640 "));

        assert_eq!(
            configured_embedding_model_key()
                .expect("server configured model key")
                .as_str(),
            "f2llm-v2-160m-q4-k-m-640"
        );

        set_server_embedding_model_key_env(None);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_server_embedding_model_key_returns_cached_semantic_error() {
        let _env_guard = lock_test_env();
        set_server_embedding_model_key_env(Some("not-a-model"));
        let mut cache = embedding_search_cache_from_startup(configured_embedding_search_startup());
        set_server_embedding_model_key_env(None);

        let output = run_vlacku_job(meaning_vlacku_request("go"), &mut cache).expect("tool output");

        assert_eq!(output.status, jbotci_cli::ToolStatus::InvalidInput);
        assert!(
            output
                .stderr
                .contains("unsupported server embedding model `not-a-model`")
        );
        assert!(matches!(cache, EmbeddingSearchCache::Unavailable { .. }));

        // The typed Discord operations report the same cached reason, typed
        // as unavailability, instead of silently degrading to another search.
        let (reply, received) = oneshot::channel();
        EmbeddingJob::VlackuHits {
            query: SearchQuery::new("go").expect("query"),
            count: NonZeroUsize::new(5).expect("count"),
            options: VlackuSearchOptions::default(),
            reply,
            keepalive: None,
        }
        .run(&mut cache);
        let hits = received.blocking_recv().expect("reply");
        assert!(
            hits.as_ref().is_err_and(|error| {
                matches!(error.as_data(), bityzba::data!(SemanticSearchError::Unavailable { reason }) if reason.contains("unsupported server embedding model `not-a-model`"))
            }),
            "{hits:?}"
        );
        let (reply, received) = oneshot::channel();
        EmbeddingJob::CuktaSearch {
            query: SearchQuery::new("tanru").expect("query"),
            window: CuktaSearchWindow::first(5),
            targets: CuktaTargetFilter::default(),
            reply,
            keepalive: None,
        }
        .run(&mut cache);
        let search = received.blocking_recv().expect("reply");
        assert!(
            search.as_ref().is_err_and(|error| {
                matches!(error.as_data(), bityzba::data!(SemanticSearchError::Unavailable { reason }) if reason.contains("unsupported server embedding model `not-a-model`"))
            }),
            "{search:?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedding_cache_reuses_cached_unavailable_state() {
        let mut cache = EmbeddingSearchCache::Unavailable {
            error: new!(CachedEmbeddingError {
                message: "cached embedding load failed".to_owned(),
            }),
        };
        let output =
            run_vlacku_job(meaning_vlacku_request("goer"), &mut cache).expect("tool output");

        assert_eq!(output.status, jbotci_cli::ToolStatus::InvalidInput);
        assert_eq!(output.stderr, "vlacku: cached embedding load failed\n");
        assert!(matches!(cache, EmbeddingSearchCache::Unavailable { .. }));
    }

    #[requires(true)]
    #[ensures(true)]
    fn meaning_vlacku_request(query: &str) -> jbotci_cli::ToolVlackuRequest {
        jbotci_cli::ToolVlackuRequest {
            mode: jbotci_cli::ToolVlackuMode::Meaning,
            query: query.to_owned(),
            count: Some(1),
            word_types: Vec::new(),
            min_votes: None,
            min_similarity: None,
            decompose_lujvo: true,
            show_etymology: false,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_vlacku_job(
        request: jbotci_cli::ToolVlackuRequest,
        cache: &mut EmbeddingSearchCache,
    ) -> Result<ToolRenderedOutput> {
        let (reply, received) = oneshot::channel();
        EmbeddingJob::Vlacku {
            request,
            reply,
            limits: ServerPolicy::default().tool_limits(),
        }
        .run(cache);
        received.blocking_recv().expect("reply delivered")
    }

    #[requires(true)]
    #[ensures(true)]
    fn search_query() -> SearchQuery {
        SearchQuery::new("go").expect("query")
    }

    #[requires(true)]
    #[ensures(true)]
    fn hits_with_deadline(
        services: &ToolServices,
        deadline: Duration,
    ) -> impl std::future::Future<
        Output = std::result::Result<Vec<DictionarySemanticHit>, SemanticSearchError>,
    > + '_ {
        services.semantic_vlacku_hits(
            search_query(),
            NonZeroUsize::new(5).expect("count"),
            VlackuSearchOptions::default(),
            Some(Instant::now() + deadline),
            None,
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn reply_empty_hits(job: EmbeddingJob) {
        if let EmbeddingJob::VlackuHits {
            reply, keepalive, ..
        } = job
        {
            // The reply is sent first, exactly as the real evaluator ends:
            // what the job retained is released only as the job is dropped.
            let _ = reply.send(Ok(Vec::new()));
            drop(keepalive);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    #[requires(true)]
    #[ensures(true)]
    async fn native_work_shares_admission_and_cancelled_waiters_do_not_run() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let processed = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_rx) = oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let execution = ServerExecution::new(ServerPolicy::default());
        let services = ToolServices::with_worker_and_execution(Arc::clone(&execution), {
            let processed = Arc::clone(&processed);
            move || {
                let mut started_tx = Some(started_tx);
                move |job| {
                    processed.fetch_add(1, Ordering::SeqCst);
                    if let Some(started_tx) = started_tx.take() {
                        let _ = started_tx.send(());
                        let _ = release_rx.recv();
                    }
                    reply_empty_hits(job);
                }
            }
        });
        let first_services = services.clone();
        let first = tokio::spawn(async move {
            hits_with_deadline(&first_services, Duration::from_secs(10)).await
        });
        started_rx.await.unwrap();
        let mut queued = Vec::new();
        for expected in 1..=8 {
            let services = services.clone();
            queued.push(tokio::spawn(async move {
                hits_with_deadline(&services, Duration::from_secs(10)).await
            }));
            wait_for_compute_waiters(&execution, expected).await;
        }
        assert_eq!(
            hits_with_deadline(&services, Duration::from_secs(1)).await,
            Err(SemanticSearchError::lane(WorkError::Overloaded {
                lane: WorkLane::Compute
            }))
        );
        assert_eq!(processed.load(Ordering::SeqCst), 1);
        queued.pop().unwrap().abort();
        wait_for_compute_waiters(&execution, 7).await;
        let context = execution
            .admit(Some(Instant::now() + Duration::from_millis(20)))
            .await;
        assert!(matches!(
            context,
            Err(WorkError::WaitTimedOut {
                lane: WorkLane::Compute
            })
        ));
        assert_eq!(execution.governor.snapshot().compute_waiting, 7);
        for caller in queued {
            caller.abort();
        }
        wait_for_compute_waiters(&execution, 0).await;
        assert_eq!(processed.load(Ordering::SeqCst), 1);
        release_tx.send(()).unwrap();
        assert_eq!(first.await.unwrap(), Ok(Vec::new()));
        assert_eq!(
            hits_with_deadline(&services, Duration::from_secs(10)).await,
            Ok(Vec::new())
        );
        assert_eq!(processed.load(Ordering::SeqCst), 2);
    }

    #[requires(expected <= 8)]
    #[ensures(execution.governor.snapshot().compute_waiting == expected)]
    async fn wait_for_compute_waiters(execution: &ServerExecution, expected: usize) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while execution.governor.snapshot().compute_waiting != expected {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("admission reached the expected queue count");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn native_reply_does_not_release_the_worker_lease_before_the_handler_exits() {
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let policy = ServerPolicy::default();
        let work = policy.work.with_data(bityzba::data! {compute_queue: 0});
        let execution = ServerExecution::new(policy.with_data(bityzba::data! { work: work }));
        let services = ToolServices::with_worker_and_execution(Arc::clone(&execution), move || {
            move |job| {
                reply_empty_hits(job);
                let _ = release_rx.recv();
            }
        });
        assert_eq!(
            hits_with_deadline(&services, Duration::from_secs(2)).await,
            Ok(Vec::new())
        );
        assert_eq!(execution.governor.snapshot().compute_active, 1);
        let state = AppState::new(
            test_config(test_static_dir()),
            production_page_meta_builder,
            execution.policy.clone(),
        )
        .with_data(bityzba::data! {tool_services: services, discord: None});
        let app = router_with_state(Arc::new(state));
        for uri in [
            "/jbotci/gentufa.png?text=mi+klama",
            "/jbotci/vlacku?query=klama",
        ] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE, "{uri}");
            assert_eq!(response.headers()["x-jbotci-error"], "server_busy");
            assert_eq!(
                response_text(response).await,
                "The server is busy. Try again later."
            );
        }
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/gentufa")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&GentufaWebRequest {
                            text: "mi klama".into(),
                            options: Default::default(),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let response = app.oneshot(Request::builder().method("POST").uri("/mcp").body(Body::from(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"vlasei","arguments":{"text":"klama"}}}"#)).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let value: serde_json::Value =
            serde_json::from_str(&response_text(response).await).unwrap();
        assert_eq!(value["result"]["isError"], true);
        assert_eq!(
            value["result"]["structuredContent"]["error"]["code"],
            "server_busy"
        );
        assert_eq!(execution.governor.snapshot().compute_active, 1);
        release_tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while execution.governor.snapshot().compute_active != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 3)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_started_meaning_search_holds_its_delivery_until_it_ends() {
        // The embedding lane runs its own worker thread, so a caller that
        // gives up (a cold model load outlasting a Discord deadline, say)
        // must not release what the running search stands for: the same
        // interaction could then be admitted and run a second time.
        use std::sync::atomic::{AtomicBool, Ordering};

        /// Stands in for a delivery: records the moment it is let go.
        #[invariant(true)]
        struct Probe {
            released: Arc<AtomicBool>,
        }

        impl Drop for Probe {
            #[requires(true)]
            #[ensures(true)]
            fn drop(&mut self) {
                self.released.store(true, Ordering::Release);
            }
        }

        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let started = Arc::new(AtomicBool::new(false));
        let services = ToolServices::with_worker_and_execution(
            ServerExecution::new(ServerPolicy::default()),
            {
                let started = Arc::clone(&started);
                move || {
                    move |job| {
                        started.store(true, Ordering::Release);
                        // However long this takes, it is work that has begun.
                        let _ = release_rx.recv();
                        reply_empty_hits(job);
                    }
                }
            },
        );

        let released = Arc::new(AtomicBool::new(false));
        let keepalive: WorkKeepalive = Arc::new(Probe {
            released: Arc::clone(&released),
        });
        let outcome = services
            .semantic_vlacku_hits(
                search_query(),
                NonZeroUsize::new(5).expect("count"),
                VlackuSearchOptions::default(),
                Some(Instant::now() + Duration::from_millis(80)),
                Some(Arc::clone(&keepalive)),
            )
            .await;
        assert_eq!(
            outcome,
            Err(SemanticSearchError::lane(WorkError::TimedOut {
                lane: WorkLane::Embedding
            })),
            "the caller stopped waiting"
        );
        assert!(started.load(Ordering::Acquire), "the search had started");
        // The caller lets go of everything it held; the search has not.
        drop(keepalive);
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert!(
            !released.load(Ordering::Acquire),
            "the running search still stands for its delivery"
        );

        release_tx.send(()).expect("the worker is waiting");
        for _ in 0..400 {
            if released.load(Ordering::Acquire) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert!(
            released.load(Ordering::Acquire),
            "and lets it go once the work really ends"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_cold_model_load_times_out_callers_without_claiming_unavailability() {
        let services = ToolServices::with_worker_and_execution(
            ServerExecution::new(ServerPolicy::default()),
            || {
                let mut loaded = false;
                move |job| {
                    if !loaded {
                        // The first job pays the model load.
                        thread::sleep(Duration::from_millis(300));
                        loaded = true;
                    }
                    reply_empty_hits(job);
                }
            },
        );
        let during_load = hits_with_deadline(&services, Duration::from_millis(50)).await;
        assert_eq!(
            during_load,
            Err(SemanticSearchError::lane(WorkError::TimedOut {
                lane: WorkLane::Embedding
            }))
        );
        let message = during_load.unwrap_err().to_string();
        assert!(
            !message.contains("unavailable") && !message.contains("not running"),
            "{message}"
        );
        let after_load = hits_with_deadline(&services, Duration::from_secs(10)).await;
        assert_eq!(after_load, Ok(Vec::new()));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn tool_services_runs_nonsemantic_vlacku_without_embedding_context() {
        let services = ToolServices::new();
        let output = services
            .run_vlacku(jbotci_cli::ToolVlackuRequest {
                mode: jbotci_cli::ToolVlackuMode::Word,
                query: "klama".to_owned(),
                count: Some(1),
                word_types: Vec::new(),
                min_votes: None,
                min_similarity: None,
                decompose_lujvo: true,
                show_etymology: false,
            })
            .await
            .expect("tool output");

        assert_eq!(output.status, jbotci_cli::ToolStatus::Success);
        assert!(output.stderr.is_empty());
        assert!(output.stdout_text().expect("text").contains("klama"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn root_absolute_assets_work_with_non_root_base_path() {
        assert_eq!(
            asset_path_for_request("/assets/app.js", "/jbotci").as_deref(),
            Some("/assets/app.js")
        );
        assert_eq!(
            asset_path_for_request("/jbotci/assets/manifest.webmanifest", "/jbotci").as_deref(),
            Some("/assets/manifest.webmanifest")
        );
        assert_eq!(
            asset_path_for_request("/jbotci/service-worker.js", "/jbotci").as_deref(),
            Some("/service-worker.js")
        );
        assert_eq!(
            asset_path_for_request("/jbotci/cukta", "/jbotci").as_deref(),
            Some("/index.html")
        );
        assert_eq!(
            asset_path_for_request("/jbotci/cukta/section/11.9", "/jbotci").as_deref(),
            Some("/index.html")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pwa_root_assets_are_not_cached_as_immutable() {
        assert_eq!(
            cache_control_for_path("/assets/manifest.webmanifest"),
            "no-cache"
        );
        assert_eq!(cache_control_for_path("/service-worker.js"), "no-cache");
        assert_eq!(
            cache_control_for_path("/assets/embedding-worker.js"),
            "no-cache"
        );
        assert_eq!(
            cache_control_for_path("/assets/app-module-ready.js"),
            "no-cache"
        );
        assert_eq!(
            cache_control_for_path(
                "/wasm/snippets/jbotci-ui-b86b7932992b1bcb/assets/worker-client.js"
            ),
            "no-cache"
        );
        assert_eq!(
            cache_control_for_path(
                "/wasm/snippets/jbotci-ui-b86b7932992b1bcb/assets/model-catalog.js"
            ),
            "no-cache"
        );
        assert_eq!(
            cache_control_for_path(
                "/wasm/snippets/other-crate-b86b7932992b1bcb/assets/worker-client.js"
            ),
            "public, max-age=31536000, immutable"
        );
        assert_eq!(
            cache_control_for_path("/assets/icons/jbotci-icon-192.png"),
            "public, max-age=31536000, immutable"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn public_dir_uses_dioxus_env_before_exe_adjacent_default() {
        assert_eq!(
            public_dir_from_env_or_exe(
                Some(PathBuf::from("/tmp/jbotci-public")),
                Some(Path::new("/opt/jbotci/server")),
            ),
            PathBuf::from("/tmp/jbotci-public")
        );
        assert_eq!(
            public_dir_from_env_or_exe(None, Some(Path::new("/opt/jbotci/server"))),
            PathBuf::from("/opt/jbotci/public")
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn health_returns_availability_and_features_route_is_removed() {
        let app = router(test_config(test_static_dir()));
        let health = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("health response");
        assert_eq!(health.status(), StatusCode::OK);
        let health_json: serde_json::Value =
            serde_json::from_str(&response_text(health).await).expect("health JSON");
        assert_eq!(health_json["status"], "ok");
        assert_eq!(health_json["features"]["gentufa"], true);
        assert!(
            health_json["features"].get("lean").is_none(),
            "health must not advertise the retired Lean semantic output capability",
        );

        let features = app
            .oneshot(
                Request::builder()
                    .uri("/api/features")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("features response");
        assert_eq!(features.status(), StatusCode::NOT_FOUND);
        assert_eq!(response_text(features).await, "not found");
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn mcp_preflight_allows_public_browser_protocol_headers() {
        let response = router(test_config(test_static_dir()))
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/mcp")
                    .header(ORIGIN, "https://example.org")
                    .header(ACCESS_CONTROL_REQUEST_METHOD, "POST")
                    .header(
                        ACCESS_CONTROL_REQUEST_HEADERS,
                        "content-type, authorization, mcp-protocol-version, mcp-session-id, last-event-id",
                    )
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_public_cors_preflight(&response);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn api_preflight_allows_public_browser_protocol_headers() {
        let response = router(test_config(test_static_dir()))
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/api/gentufa")
                    .header(ORIGIN, "https://example.org")
                    .header(ACCESS_CONTROL_REQUEST_METHOD, "POST")
                    .header(
                        ACCESS_CONTROL_REQUEST_HEADERS,
                        "content-type, authorization, mcp-protocol-version, mcp-session-id, last-event-id",
                    )
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_public_cors_preflight(&response);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn mcp_post_accepts_public_browser_origin() {
        let response = router(test_config(test_static_dir()))
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/mcp")
                    .header(ORIGIN, "https://example.org")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": 1,
                            "method": "tools/list"
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_public_cors_response(&response);
        let json = response_json(response).await;
        assert!(
            json["result"]["tools"]
                .as_array()
                .is_some_and(|tools| !tools.is_empty())
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn mcp_get_405_is_visible_to_public_browser_origins() {
        let response = router(test_config(test_static_dir()))
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/mcp")
                    .header(ORIGIN, "https://example.org")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_public_cors_response(&response);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn api_response_allows_public_browser_origin() {
        let response = router(test_config(test_static_dir()))
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/health")
                    .header(ORIGIN, "https://example.org")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_public_cors_response(&response);
        let json = response_json(response).await;
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn discord_route_remains_outside_public_cors() {
        let response = router(test_config(test_static_dir()))
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/discord")
                    .header(ORIGIN, "https://example.org")
                    .body(Body::from(serde_json::json!({ "type": 1 }).to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(
            response
                .headers()
                .get(ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_none()
        );
        assert!(
            response
                .headers()
                .get(ACCESS_CONTROL_ALLOW_CREDENTIALS)
                .is_none()
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn gentufa_api_matches_direct_parser() {
        let app = router(test_config(test_static_dir()));
        let request = GentufaWebRequest {
            text: "mi klama".to_owned(),
            options: Default::default(),
        };
        let expected = parse_gentufa_for_web(&request);
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/gentufa")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&request).expect("request JSON"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let actual: GentufaWebResult = serde_json::from_slice(&bytes).expect("response JSON");
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn gimfihi_rest_api_matches_typed_tool_surface() {
        let app = router(test_config(test_static_dir()));
        let request = ToolGimfihiRequest {
            sources: vec![ToolGimfihiSource {
                language: "eng".to_owned(),
                word: "ˈkla.ma".to_owned(),
                weight: Some(100),
            }],
            scorer: ToolGimfihiScorer::Phonetic,
            c_vwl: 8.0,
            shapes: vec!["ccvcv".to_owned()],
            check_collisions: ToolCollisionScope::None,
            count: Some(1),
            highlight: Some("klama".to_owned()),
            format: ToolGimfihiFormat::Json,
            ..ToolGimfihiRequest::default()
        };
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/gimfihi")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&request).expect("request JSON"),
                    ))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CONTENT_TYPE),
            Some(&HeaderValue::from_static("application/json; charset=utf-8"))
        );
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let output: jbotci_cli::test_harness::GimfihiOutput =
            serde_json::from_slice(&bytes).expect("typed gimfihi output");
        assert_eq!(
            output.scorer,
            jbotci_cli::test_harness::GimfihiScorer::Phonetic
        );
        assert_eq!(
            output
                .phonetic_parameters
                .as_ref()
                .map(|parameters| parameters.c_vwl),
            Some(8.0)
        );
        let identity = output
            .candidates
            .iter()
            .find(|candidate| candidate.word == "klama")
            .expect("highlighted identity");
        assert_eq!(identity.source_scores[0].raw_score, 1.0);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn gimfihi_rest_rejects_unknown_salience_by_name() {
        let response = router(test_config(test_static_dir()))
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/gimfihi")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"sources":[],"saliences":{"mystery":5}}"#))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(response_text(response).await.contains("mystery"));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn missing_api_route_does_not_fall_back_to_spa() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/missing")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/api/cukta")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn mcp_initialize_and_tool_listing_follow_protocol_shape() {
        let app = router(test_config(test_static_dir()));
        let initialize = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "clientInfo": {
                        "name": "test",
                        "version": "0"
                    }
                }
            }),
        )
        .await;
        assert_eq!(initialize.status(), StatusCode::OK);
        let initialize_json = response_json(initialize).await;
        assert_eq!(initialize_json["result"]["protocolVersion"], "2025-06-18");
        assert!(initialize_json["result"]["capabilities"]["tools"].is_object());
        let instructions = initialize_json["result"]["instructions"]
            .as_str()
            .expect("server instructions");
        assert!(instructions.contains("jbotci is a Lojban toolkit"));
        assert!(instructions.contains("`gentufa` to parse a sentence's grammar"));

        let tools = post_json(
            app,
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list"
            }),
        )
        .await;
        assert_eq!(tools.status(), StatusCode::OK);
        let tools_json = response_json(tools).await;
        let tools_array = tools_json["result"]["tools"]
            .as_array()
            .expect("tools array");
        let names = tools_array
            .iter()
            .map(|tool| tool["name"].as_str().expect("tool name"))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec!["cukta", "vlacku", "jvozba", "vlasei", "gentufa", "gimfihi"]
        );
        assert!(
            names
                .iter()
                .all(|name| external_api_identifier_is_safe(name))
        );
        for tool in tools_array {
            assert_eq!(tool["inputSchema"]["additionalProperties"], false);
            assert_eq!(tool["annotations"]["readOnlyHint"], true);
            assert_eq!(tool["annotations"]["destructiveHint"], false);
            assert_eq!(tool["annotations"]["openWorldHint"], false);
            assert!(
                !json_contains_key(&tool["inputSchema"], "$ref"),
                "{}",
                tool["inputSchema"]
            );
        }

        let gentufa_schema = tool_input_schema(tools_array, "gentufa");
        assert!(gentufa_schema["properties"]["text"].is_object());
        assert!(gentufa_schema["properties"]["show-defs"].is_object());
        assert_string_enum_property(
            gentufa_schema,
            "format",
            &["tree", "brackets", "raw", "json", "svg", "png"],
        );
        assert_boolean_default_property(gentufa_schema, "show-refs", true);
        assert_boolean_default_property(gentufa_schema, "show-elided", false);
        assert_boolean_default_property(gentufa_schema, "show-glosses", false);
        assert_boolean_default_property(gentufa_schema, "show-compounds", true);
        let gentufa_schema_text = serde_json::to_string(&gentufa_schema).expect("schema JSON");
        for stale_name in [
            "lojban",
            "outputFormat",
            "includeWordDefinitions",
            "lojbanDialect",
        ] {
            assert!(!gentufa_schema_text.contains(stale_name), "{stale_name}");
        }

        let vlasei_schema = tool_input_schema(tools_array, "vlasei");
        assert_string_enum_property(
            vlasei_schema,
            "format",
            &["tree", "brackets", "ipa", "raw", "json"],
        );
        // Morphology has no place-structure references, so vlasei exposes no
        // `show-refs` field (it is meaningful only for the syntax parse).
        assert!(vlasei_schema["properties"]["show-refs"].is_null());

        let cukta_schema = tool_input_schema(tools_array, "cukta");
        assert_string_enum_property(
            cukta_schema,
            "mode",
            &["meaning", "word", "section", "example", "toc"],
        );
        assert_string_enum_property(cukta_schema, "format", &["markdown", "html", "raw"]);
        assert!(cukta_schema["properties"]["query"].is_object());
        // The search-result-kind filter is named to disambiguate it from the
        // `section`/`example` navigation modes; its items are content kinds.
        assert!(cukta_schema["properties"]["targets"].is_null());
        let kinds = &cukta_schema["properties"]["search-result-kinds"];
        assert_eq!(kinds["type"], "array", "{kinds}");
        assert_string_enum_property_at(&kinds["items"], &["section", "paragraph", "example"]);
        assert!(
            !serde_json::to_string(&cukta_schema)
                .expect("schema JSON")
                .contains("action")
        );

        let vlacku_schema = tool_input_schema(tools_array, "vlacku");
        assert_string_enum_property(
            vlacku_schema,
            "mode",
            &["word", "rafsi", "lujvo", "sound", "meaning"],
        );
        assert!(vlacku_schema["properties"]["query"].is_object());
        let vlacku_schema_text = serde_json::to_string(&vlacku_schema).expect("schema JSON");
        for stale_name in [
            "exactLojbanWord",
            "semanticallySimilarTo",
            "soundsLike",
            "exactRafsi",
        ] {
            assert!(!vlacku_schema_text.contains(stale_name), "{stale_name}");
        }

        let jvozba_schema = tool_input_schema(tools_array, "jvozba");
        assert_string_enum_property(jvozba_schema, "mode", &["lujvo", "cmevla"]);
        assert_string_enum_property(
            &jvozba_schema["properties"]["parts"]["items"],
            "kind",
            &["word", "fixed-rafsi"],
        );

        let gimfihi_schema = tool_input_schema(tools_array, "gimfihi");
        assert_string_enum_property(gimfihi_schema, "format", &["table", "json"]);
        assert_string_enum_property(
            gimfihi_schema,
            "check-collisions",
            &["all", "official", "none"],
        );
        // Sources are a typed array of objects, fully inlined (no `$ref`).
        let gimfihi_source = &gimfihi_schema["properties"]["sources"]["items"];
        assert!(gimfihi_source["properties"]["language"].is_object());
        assert!(gimfihi_source["properties"]["word"].is_object());
        assert!(gimfihi_source["properties"]["weight"].is_object());
        let projection: GimfihiSchemaProjection =
            serde_json::from_value(gimfihi_schema.clone()).expect("typed gimfihi schema");
        let defaults = jbotci_cli::test_harness::AlineParameters::default();
        assert_eq!(projection.properties.scorer.default, "classic");
        assert_eq!(projection.properties.normalizer.default, "source-side");
        assert_eq!(projection.properties.c_sub.default, defaults.c_sub);
        assert_eq!(projection.properties.c_exp.default, defaults.c_exp);
        assert_eq!(projection.properties.c_skip.default, defaults.c_skip);
        assert_eq!(projection.properties.c_vwl.default, defaults.c_vwl);
        assert_eq!(projection.properties.c_flank.default, defaults.c_flank);
        assert_eq!(projection.properties.c_skip.maximum, Some(0.0));
        assert_eq!(projection.properties.c_vwl.minimum, Some(0.0));
        assert_eq!(projection.properties.c_flank.maximum, Some(0.0));
        for description in [
            &projection.properties.scorer.description,
            &projection.properties.c_sub.description,
            &projection.properties.c_exp.description,
            &projection.properties.c_skip.description,
            &projection.properties.c_vwl.description,
            &projection.properties.c_flank.description,
            &projection.properties.normalizer.description,
            &projection.properties.saliences.description,
        ] {
            assert!(
                description.contains("docs/gismu-phonetic-medoid.md"),
                "{description}"
            );
        }
        let salience_defaults = jbotci_cli::test_harness::AlineSaliences::default();
        assert_eq!(
            projection.properties.saliences.default,
            ToolAlineSaliences::default()
        );
        for (property, expected) in [
            (
                &projection.properties.saliences.properties.syllabic,
                salience_defaults.syllabic,
            ),
            (
                &projection.properties.saliences.properties.place,
                salience_defaults.place,
            ),
            (
                &projection.properties.saliences.properties.manner,
                salience_defaults.manner,
            ),
            (
                &projection.properties.saliences.properties.voice,
                salience_defaults.voice,
            ),
            (
                &projection.properties.saliences.properties.nasal,
                salience_defaults.nasal,
            ),
            (
                &projection.properties.saliences.properties.retroflex,
                salience_defaults.retroflex,
            ),
            (
                &projection.properties.saliences.properties.lateral,
                salience_defaults.lateral,
            ),
            (
                &projection.properties.saliences.properties.aspirated,
                salience_defaults.aspirated,
            ),
            (
                &projection.properties.saliences.properties.high,
                salience_defaults.high,
            ),
            (
                &projection.properties.saliences.properties.back,
                salience_defaults.back,
            ),
            (
                &projection.properties.saliences.properties.round,
                salience_defaults.round,
            ),
            (
                &projection.properties.saliences.properties.long,
                salience_defaults.long,
            ),
        ] {
            assert_eq!(property.default, expected);
            assert_eq!(property.minimum, Some(0.0));
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mcp_indentation_abuse_is_rejected_in_constrained_process() {
        const CHILD_FLAG: &str = "JBOTCI_INDENTATION_PROBE_CHILD";
        if std::env::var_os(CHILD_FLAG).is_none() {
            // A removed bound must fail this child instead of exhausting the test host.
            let output = std::process::Command::new("prlimit")
                .args(["--as=2147483648", "--cpu=20", "--"])
                .arg(std::env::current_exe().expect("test executable"))
                .args([
                    "--exact",
                    "tests::mcp_indentation_abuse_is_rejected_in_constrained_process",
                    "--nocapture",
                ])
                .env(CHILD_FLAG, "1")
                .output()
                .expect("start constrained local probe");
            assert!(
                output.status.success(),
                "status={}\n{}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
            let app = router(test_config(test_static_dir()));
            for name in ["gentufa", "vlasei"] {
                for format in ["tree", "json"] {
                    for indent in [9usize, 1_073_741_824, usize::MAX] {
                        let response = post_json(app.clone(), "/mcp", serde_json::json!({
                            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                            "params": {"name": name, "arguments": {"text": "mi klama", "format": format, "indent": indent}}
                        })).await;
                        assert_eq!(response.status(), StatusCode::OK);
                        let body = response_json(response).await;
                        assert_eq!(body["result"]["isError"], true, "{name} {format}: {body}");
                        let error = body["result"]["content"][0]["text"].as_str().unwrap();
                        assert!(error.contains("Invalid tool arguments"), "{error}");
                        assert!(error.contains("between 0 and 8"), "{error}");
                    }
                }
            }
        });
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn mcp_calls_return_text_structured_json_images_and_tool_errors() {
        let app = router(test_config(test_static_dir()));
        let text = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "text",
                "method": "tools/call",
                "params": {
                    "name": "gentufa",
                    "arguments": {
                        "text": "mi klama"
                    }
                }
            }),
        )
        .await;
        assert_eq!(text.status(), StatusCode::OK);
        let text_json = response_json(text).await;
        assert_eq!(text_json["result"]["content"][0]["type"], "text");
        assert!(
            text_json["result"]["content"][0]["text"]
                .as_str()
                .expect("text content")
                .trim()
                .len()
                > 0
        );

        // `word`/`rafsi` modes accept plain text, `*`/`?` globs, and `/regex/`;
        // there are no separate glob/regex modes.
        for (id, mode, query, expected_text) in [
            ("vlacku-word", "word", "klama", "klama"),
            ("vlacku-word-regex", "word", "/^klama$/", "klama"),
            ("vlacku-rafsi-glob", "rafsi", "kla*", "kla"),
            ("vlacku-sound", "sound", "klama", "klama"),
        ] {
            let vlacku = post_json(
                app.clone(),
                "/mcp",
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": "tools/call",
                    "params": {
                        "name": "vlacku",
                        "arguments": {
                            "mode": mode,
                            "query": query,
                            "count": 1
                        }
                    }
                }),
            )
            .await;
            assert_eq!(vlacku.status(), StatusCode::OK);
            let vlacku_json = response_json(vlacku).await;
            assert_ne!(vlacku_json["result"]["isError"], true);
            assert!(
                vlacku_json["result"]["content"][0]["text"]
                    .as_str()
                    .expect("vlacku text")
                    .contains(expected_text)
            );
        }

        let structured = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "json",
                "method": "tools/call",
                "params": {
                    "name": "gentufa",
                    "arguments": {
                        "text": "mi klama",
                        "format": "json"
                    }
                }
            }),
        )
        .await;
        assert_eq!(structured.status(), StatusCode::OK);
        let structured_json = response_json(structured).await;
        assert_eq!(structured_json["result"]["content"][0]["type"], "text");
        // JSON formats are returned as readable text only (no duplicate
        // `structuredContent`); the text itself parses as JSON.
        assert!(structured_json["result"]["structuredContent"].is_null());
        let gentufa_parsed: serde_json::Value = serde_json::from_str(
            structured_json["result"]["content"][0]["text"]
                .as_str()
                .expect("gentufa json text"),
        )
        .expect("gentufa json content parses");
        assert!(gentufa_parsed.is_object(), "{gentufa_parsed}");

        let vlasei_structured = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "vlasei-json",
                "method": "tools/call",
                "params": {
                    "name": "vlasei",
                    "arguments": {
                        "text": "coi",
                        "format": "json"
                    }
                }
            }),
        )
        .await;
        assert_eq!(vlasei_structured.status(), StatusCode::OK);
        let vlasei_structured_json = response_json(vlasei_structured).await;
        assert_eq!(
            vlasei_structured_json["result"]["content"][0]["type"],
            "text"
        );
        // Text-only JSON; the content parses as the morphology array.
        assert!(
            vlasei_structured_json["result"]["structuredContent"].is_null(),
            "{}",
            vlasei_structured_json["result"]
        );
        let vlasei_parsed: serde_json::Value = serde_json::from_str(
            vlasei_structured_json["result"]["content"][0]["text"]
                .as_str()
                .expect("vlasei json text"),
        )
        .expect("vlasei json content parses");
        assert!(vlasei_parsed.is_array(), "{vlasei_parsed}");

        let vlasei = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "vlasei-bare-dialect",
                "method": "tools/call",
                "params": {
                    "name": "vlasei",
                    "arguments": {
                        "text": "coi",
                        "dialect": "case-insensitive"
                    }
                }
            }),
        )
        .await;
        assert_eq!(vlasei.status(), StatusCode::OK);
        let vlasei_json = response_json(vlasei).await;
        assert_ne!(vlasei_json["result"]["isError"], true, "{vlasei_json}");

        let removed_cgv_alias = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "vlasei-removed-cgv-alias",
                "method": "tools/call",
                "params": {
                    "name": "vlasei",
                    "arguments": {
                        "text": "coi",
                        "dialect": "(allow-cgv)"
                    }
                }
            }),
        )
        .await;
        assert_eq!(removed_cgv_alias.status(), StatusCode::OK);
        let removed_cgv_alias_json = response_json(removed_cgv_alias).await;
        assert_eq!(removed_cgv_alias_json["result"]["isError"], true);
        assert!(
            removed_cgv_alias_json["result"]["content"][0]["text"]
                .as_str()
                .expect("error text")
                .contains("Unknown dialect reference: allow-cgv")
        );

        let gimfihi_highlight = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "gimfihi-highlight",
                "method": "tools/call",
                "params": {
                    "name": "gimfihi",
                    "arguments": {
                        "preset": "1995",
                        "sources": [
                            {"language": "cmn", "word": "uan"},
                            {"language": "hin", "word": "rakan"},
                            {"language": "eng", "word": "ekspekt"},
                            {"language": "spa", "word": "esper"},
                            {"language": "rus", "word": "predpologa"},
                            {"language": "ara", "word": "mulud"}
                        ],
                        "check-collisions": "none",
                        "count": 1,
                        "highlight": "nanpe"
                    }
                }
            }),
        )
        .await;
        assert_eq!(gimfihi_highlight.status(), StatusCode::OK);
        let gimfihi_highlight_json = response_json(gimfihi_highlight).await;
        let gimfihi_text = gimfihi_highlight_json["result"]["content"][0]["text"]
            .as_str()
            .expect("gimfihi text");
        assert!(gimfihi_text.contains("*     nanpe"), "{gimfihi_text}");

        let image = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "png",
                "method": "tools/call",
                "params": {
                    "name": "gentufa",
                    "arguments": {
                        "text": "mi klama",
                        "format": "png"
                    }
                }
            }),
        )
        .await;
        assert_eq!(image.status(), StatusCode::OK);
        let image_json = response_json(image).await;
        assert_eq!(image_json["result"]["content"][0]["type"], "image");
        assert_eq!(image_json["result"]["content"][0]["mimeType"], "image/png");
        assert!(
            image_json["result"]["content"][0]["data"]
                .as_str()
                .expect("image data")
                .len()
                > 100
        );

        // SVG is served as text (its XML source), not an image content block, so
        // harnesses that cannot render SVG images can still read/reuse it.
        let svg = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "svg",
                "method": "tools/call",
                "params": {
                    "name": "gentufa",
                    "arguments": {
                        "text": "mi klama",
                        "format": "svg"
                    }
                }
            }),
        )
        .await;
        assert_eq!(svg.status(), StatusCode::OK);
        let svg_json = response_json(svg).await;
        let svg_block = &svg_json["result"]["content"][0];
        assert_eq!(svg_block["type"], "text");
        assert!(svg_block.get("data").is_none(), "{svg_block}");
        assert!(
            svg_block["text"]
                .as_str()
                .expect("svg text")
                .contains("<svg"),
            "{svg_block}"
        );

        let unknown = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "unknown",
                "method": "tools/call",
                "params": {
                    "name": "not-a-tool",
                    "arguments": {}
                }
            }),
        )
        .await;
        assert_eq!(unknown.status(), StatusCode::OK);
        let unknown_json = response_json(unknown).await;
        assert_eq!(unknown_json["result"]["isError"], true);
        assert!(
            unknown_json["result"]["content"][0]["text"]
                .as_str()
                .expect("error text")
                .contains("Unknown tool")
        );

        let invalid_args = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "invalid",
                "method": "tools/call",
                "params": {
                    "name": "vlacku",
                    "arguments": {
                        "query": "klama",
                        "unexpected": true
                    }
                }
            }),
        )
        .await;
        assert_eq!(invalid_args.status(), StatusCode::OK);
        let invalid_args_json = response_json(invalid_args).await;
        assert_eq!(invalid_args_json["result"]["isError"], true);
        assert!(
            invalid_args_json["result"]["content"][0]["text"]
                .as_str()
                .expect("error text")
                .contains("unknown field")
        );

        let old_vlacku_union_args = post_json(
            app,
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "old-vlacku-union",
                "method": "tools/call",
                "params": {
                    "name": "vlacku",
                    "arguments": {
                        "exactLojbanWord": "klama"
                    }
                }
            }),
        )
        .await;
        assert_eq!(old_vlacku_union_args.status(), StatusCode::OK);
        let old_vlacku_union_json = response_json(old_vlacku_union_args).await;
        assert_eq!(old_vlacku_union_json["result"]["isError"], true);
        let old_vlacku_union_text = old_vlacku_union_json["result"]["content"][0]["text"]
            .as_str()
            .expect("error text");
        assert!(old_vlacku_union_text.contains("unknown field"));
        assert!(!old_vlacku_union_text.contains("Invalid Lojban word"));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn mcp_exposes_lojban_grammar_resource() {
        let app = router(test_config(test_static_dir()));

        let initialize = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": { "protocolVersion": "2025-06-18" }
            }),
        )
        .await;
        let initialize_json = response_json(initialize).await;
        assert!(initialize_json["result"]["capabilities"]["resources"].is_object());

        let list = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({ "jsonrpc": "2.0", "id": 2, "method": "resources/list" }),
        )
        .await;
        assert_eq!(list.status(), StatusCode::OK);
        let list_json = response_json(list).await;
        let resources = list_json["result"]["resources"]
            .as_array()
            .expect("resources array");
        let grammar = resources
            .iter()
            .find(|resource| resource["name"] == "lojban-grammar")
            .expect("grammar resource listed");
        let uri = grammar["uri"].as_str().expect("resource uri");
        assert!(
            grammar["description"]
                .as_str()
                .is_some_and(|d| !d.is_empty())
        );

        let read = post_json(
            app.clone(),
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "resources/read",
                "params": { "uri": uri }
            }),
        )
        .await;
        assert_eq!(read.status(), StatusCode::OK);
        let read_json = response_json(read).await;
        let contents = &read_json["result"]["contents"][0];
        assert_eq!(contents["uri"], uri);
        let text = contents["text"].as_str().expect("grammar text");
        // Spot-check that the real grammar made it through verbatim.
        assert!(
            text.contains("bridi-tail ="),
            "{}",
            &text[..text.len().min(200)]
        );
        assert!(text.contains("selma'o"));

        let unknown = post_json(
            app,
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "resources/read",
                "params": { "uri": "jbotci:///grammar/does-not-exist" }
            }),
        )
        .await;
        let unknown_json = response_json(unknown).await;
        assert_eq!(unknown_json["error"]["code"], -32602);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn mcp_tools_call_tolerates_reserved_meta_field() {
        // The MCP spec reserves a sibling `_meta` object in `tools/call` params
        // (e.g. `progressToken`), which the Claude Agent SDK and other clients
        // send. A conformant server must not reject the call because of it.
        let app = router(test_config(test_static_dir()));
        let response = post_json(
            app,
            "/mcp",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": "meta",
                "method": "tools/call",
                "params": {
                    "name": "gentufa",
                    "arguments": { "text": "mi klama" },
                    "_meta": { "progressToken": "abc-123" }
                }
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let json = response_json(response).await;
        assert!(json.get("error").is_none(), "{json}");
        let result = &json["result"];
        assert_ne!(result["isError"], true, "{result}");
        assert_eq!(result["content"][0]["type"], "text");
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn mcp_routes_do_not_fall_back_to_spa() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/mcp")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/jbotci/mcp")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/jbotci/discord")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn discord_rejects_missing_and_invalid_signatures() {
        let _env_guard = lock_test_env();
        let key = test_discord_signing_key();
        let public_key = hex_bytes(&key.verifying_key().to_bytes());
        configure_discord_test_env(&public_key);
        let app = router(test_config(test_static_dir()));

        let missing = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/discord")
                    .body(Body::from(serde_json::json!({ "type": 1 }).to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);

        let invalid = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/discord")
                    .header("X-Signature-Timestamp", "1710000000")
                    .header("X-Signature-Ed25519", "00".repeat(64))
                    .body(Body::from(serde_json::json!({ "type": 1 }).to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(invalid.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn discord_refuses_a_body_that_is_not_an_interaction() {
        let _env_guard = lock_test_env();
        let key = test_discord_signing_key();
        let public_key = hex_bytes(&key.verifying_key().to_bytes());
        configure_discord_test_env(&public_key);
        let app = router(test_config(test_static_dir()));

        // A signed body that is not JSON is not an interaction to answer: it
        // is a bad request, and saying so is more honest than replying in the
        // channel about jbotci's own parser.
        let response = post_signed_discord_body(app.clone(), "{".to_owned(), &key).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Valid JSON that is not an interaction jbotci handles is answered
        // privately, because Discord expects an interaction response.
        let response =
            post_signed_discord_body(app, serde_json::json!({ "type": 9 }).to_string(), &key).await;
        assert_eq!(response.status(), StatusCode::OK);
        let json = response_json(response).await;
        assert_eq!(json["type"], 4);
        assert!(
            json["data"]["content"]
                .as_str()
                .expect("message content")
                .contains("not one jbotci understands")
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn discord_accepts_signed_ping_and_subcommands() {
        let _env_guard = lock_test_env();
        let key = test_discord_signing_key();
        let public_key = hex_bytes(&key.verifying_key().to_bytes());
        configure_discord_test_env(&public_key);
        let app = router(test_config(test_static_dir()));

        let ping = post_signed_discord(app.clone(), serde_json::json!({ "type": 1 }), &key).await;
        assert_eq!(ping.status(), StatusCode::OK);
        let ping_json = response_json(ping).await;
        assert_eq!(ping_json["type"], 1);

        for interaction in [
            discord_interaction("gentufa", vec![discord_string_option("text", "mi klama")]),
            discord_interaction("vlasei", vec![discord_string_option("text", "mi klama")]),
            discord_interaction("vlacku", vec![discord_string_option("query", "klama")]),
            discord_interaction(
                "cukta",
                vec![
                    discord_string_option("mode", "word"),
                    discord_string_option("query", "klama"),
                ],
            ),
            discord_interaction(
                "jvozba",
                vec![discord_string_option("parts", "klama bajra")],
            ),
            discord_interaction("gimfihi", vec![discord_string_option("sources", "en:go")]),
        ] {
            let response = post_signed_discord(app.clone(), interaction, &key).await;
            assert_eq!(response.status(), StatusCode::OK);
            let json = response_json(response).await;
            assert_eq!(json["type"], 5);
        }
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn discord_rejects_url_unsafe_interaction_identifiers() {
        let _env_guard = lock_test_env();
        let key = test_discord_signing_key();
        let public_key = hex_bytes(&key.verifying_key().to_bytes());
        configure_discord_test_env(&public_key);
        let app = router(test_config(test_static_dir()));
        // An identifier that could travel into a URL path is not read at
        // all, so nothing it names is ever requested.
        for (field, value) in [
            ("application_id", "123/456"),
            ("token", "bad/token"),
            ("id", "not-a-snowflake"),
        ] {
            let mut interaction =
                discord_interaction("vlasei", vec![discord_string_option("text", "coi")]);
            interaction[field] = serde_json::json!(value);

            let response = post_signed_discord(app.clone(), interaction, &key).await;
            assert_eq!(response.status(), StatusCode::OK);
            let json = response_json(response).await;
            assert_eq!(json["type"], 4, "{field}");
            assert!(
                json["data"]["content"]
                    .as_str()
                    .expect("message content")
                    .contains("not one jbotci understands"),
                "{field}: {json}"
            );
        }

        let mut interaction =
            discord_interaction("vlasei", vec![discord_string_option("text", "coi")]);
        interaction["token"] = serde_json::json!("bad/token");
        let response = post_signed_discord(app, interaction, &key).await;
        assert_eq!(response.status(), StatusCode::OK);
        let json = response_json(response).await;
        assert_eq!(json["type"], 4);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn discord_command_registration_keeps_its_installation_contexts() {
        // A bulk overwrite replaces every field of the command, so the
        // registration must carry the contexts the installed command has:
        // guild and user installs, usable in direct messages.
        let payload = discord::discord_command_registration();
        assert_eq!(payload["name"], "jbotci");
        assert_eq!(payload["integration_types"], serde_json::json!([0, 1]));
        assert_eq!(payload["contexts"], serde_json::json!([0, 1, 2]));
        assert_eq!(payload["dm_permission"], serde_json::json!(true));
        let subcommands = payload["options"]
            .as_array()
            .expect("subcommand array")
            .iter()
            .map(|option| option["name"].as_str().expect("subcommand name"))
            .collect::<Vec<_>>();
        assert_eq!(
            subcommands,
            vec![
                "gentufa", "vlasei", "vlatai", "vlacku", "cukta", "jvozba", "gimfihi"
            ]
        );
        assert!(
            subcommands
                .iter()
                .all(|name| external_api_identifier_is_safe(name))
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn embedding_assets_return_404_without_static_dir() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/embeddings/web/v1/catalog.json")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn embedding_assets_serve_catalog_and_vectors_from_static_dir() {
        let static_dir = test_static_dir();
        std::fs::create_dir_all(static_dir.join(
            "assets/embeddings/web/v1/models/test/spaces/space/packs/pack/corpora/vlacku-en",
        ))
        .expect("create embedding asset dir");
        std::fs::write(
            static_dir.join("assets/embeddings/web/v1/catalog.json"),
            "{\"schema_version\":1,\"models\":[]}\n",
        )
        .expect("write catalog");
        std::fs::write(
            static_dir
                .join("assets/embeddings/web/v1/models/test/spaces/space/packs/pack/corpora/vlacku-en/vectors.f32"),
            [0u8, 0, 0, 0],
        )
        .expect("write vector file");
        let app = router(test_config(static_dir));
        let catalog = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/assets/embeddings/web/v1/catalog.json")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("catalog response");
        assert_eq!(catalog.status(), StatusCode::OK);
        assert_eq!(
            catalog
                .headers()
                .get(CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("no-cache")
        );
        assert_eq!(
            response_text(catalog).await,
            "{\"schema_version\":1,\"models\":[]}\n"
        );

        let shard = app
            .oneshot(
                Request::builder()
                    .uri("/assets/embeddings/web/v1/models/test/spaces/space/packs/pack/corpora/vlacku-en/vectors.f32")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("shard response");
        assert_eq!(shard.status(), StatusCode::OK);
        assert_eq!(
            shard
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("application/octet-stream")
        );
        assert_eq!(response_bytes(shard).await, vec![0, 0, 0, 0]);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn static_assets_prefer_brotli_when_accepted() {
        let static_dir = test_static_dir();
        std::fs::create_dir_all(static_dir.join("assets")).expect("create assets dir");
        std::fs::write(static_dir.join("assets/app.js"), "plain").expect("write asset");
        std::fs::write(static_dir.join("assets/app.js.br"), "brotli").expect("write br asset");
        let app = router(test_config(static_dir));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/app.js")
                    .header(ACCEPT_ENCODING, "gzip, br")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_ENCODING)
                .and_then(|value| value.to_str().ok()),
            Some("br")
        );
        assert_eq!(
            response
                .headers()
                .get(VARY)
                .and_then(|value| value.to_str().ok()),
            Some("Accept-Encoding")
        );
        assert_eq!(
            response
                .headers()
                .get(CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("public, max-age=31536000, immutable")
        );
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/javascript; charset=utf-8")
        );
        assert_eq!(response_text(response).await, "brotli");
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn static_assets_skip_brotli_without_accept_encoding() {
        let static_dir = test_static_dir();
        std::fs::create_dir_all(static_dir.join("assets")).expect("create assets dir");
        std::fs::write(static_dir.join("assets/app.js"), "plain").expect("write asset");
        std::fs::write(static_dir.join("assets/app.js.br"), "brotli").expect("write br asset");
        let app = router(test_config(static_dir));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/app.js")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get(CONTENT_ENCODING).is_none());
        assert_eq!(
            response
                .headers()
                .get(VARY)
                .and_then(|value| value.to_str().ok()),
            Some("Accept-Encoding")
        );
        assert_eq!(
            response
                .headers()
                .get(CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("public, max-age=31536000, immutable")
        );
        assert_eq!(response_text(response).await, "plain");
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn missing_static_asset_returns_404() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/missing.js")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn missing_spa_index_returns_404() {
        let static_dir = test_static_dir();
        std::fs::remove_file(static_dir.join("index.html")).expect("remove test index");
        let app =
            router_with_page_meta_builder(test_config(static_dir), panicking_page_meta_builder);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/gentufa?text=mi+klama")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response_text(response).await, "not found");
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn embedding_assets_reject_path_traversal() {
        let static_dir = test_static_dir();
        let secret_path = static_dir
            .parent()
            .expect("test root has parent")
            .join("secret.txt");
        std::fs::write(&secret_path, "secret").expect("write secret");
        let app = router(test_config(static_dir));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/embeddings/../secret.txt")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn root_redirects_to_dictionary_route() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok()),
            Some("/jbotci/vlacku"),
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn root_base_redirects_to_unprefixed_dictionary_route() {
        let app = router(test_config_with_base_path("/", test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::FOUND);
        assert_eq!(
            response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok()),
            Some("/vlacku"),
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn root_favicon_serves_v0_png_icon() {
        let static_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/jbotci-ui");
        let app = router(test_config(static_dir));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/favicon.ico")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("image/png"),
        );
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn gentufa_export_routes_render_png_and_svg() {
        let app = router(test_config(test_static_dir()));
        let png = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/jbotci/gentufa.png?text=mi+klama")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("PNG response");
        assert_eq!(png.status(), StatusCode::OK);
        assert_eq!(
            png.headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("image/png")
        );
        let png_bytes = response_bytes(png).await;
        assert!(png_bytes.starts_with(b"\x89PNG\r\n\x1a\n"));

        let svg = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/gentufa.svg?text=mi+klama&script=cyrillic")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("SVG response");
        assert_eq!(svg.status(), StatusCode::OK);
        assert_eq!(
            svg.headers()
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("image/svg+xml; charset=utf-8")
        );
        let svg_body = response_text(svg).await;
        assert!(svg_body.contains("<svg"));
        assert!(svg_body.contains("ми"));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn gentufa_export_route_returns_bad_request_for_parse_error() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/gentufa.png?text=perhaps")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response_text(response).await;
        assert!(body.contains("morphology.invalid-apostrophe"));
        assert!(body.contains("\nreason:"));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn spa_gentufa_metadata_is_rendered_with_png_social_image() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/gentufa?text=mi+klama")
                    .header(HOST, "example.test")
                    .header("x-forwarded-proto", "https")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_text(response).await;

        assert!(body.contains("<title>mi klama - jbotci gentufa</title>"));
        assert!(body.contains("name=\"description\""));
        assert!(
            body.contains("<link rel=\"manifest\" href=\"/jbotci/assets/manifest.webmanifest\">")
        );
        assert!(body.contains(
            "<link rel=\"apple-touch-icon\" href=\"/jbotci/assets/icons/apple-touch-icon.png\">"
        ));
        assert!(!body.contains("Parse succeeded:"));
        assert!(body.contains(
            "property=\"og:url\" content=\"https://example.test/jbotci/gentufa?text=mi+klama\""
        ));
        assert!(body.contains("name=\"twitter:card\" content=\"summary_large_image\""));
        assert!(body.contains(
            "property=\"og:image\" content=\"https://example.test/jbotci/gentufa.png?text=mi+klama\""
        ));
        assert!(body.contains(
            "name=\"twitter:image\" content=\"https://example.test/jbotci/gentufa.png?text=mi+klama\""
        ));
        assert!(!body.contains("gentufa.svg"));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn reported_jboponei_gentufa_url_renders_spa_metadata() {
        let app = router(test_config_with_base_path("/", test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(concat!(
                        "/gentufa?text=i+lo+drasmu+cu+na+jai+fasnu+ri%27a+po%21%21++",
                        "lo%27e+finti+cu+jai+gau+smuni+ei+dai+do%27e+lo+cnino+terbri",
                        "&dialect=%28jboponei%29",
                    ))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = response_text(response).await;
        assert!(body.contains("<title>i lo drasmu cu na jai fasnu"));
        assert!(body.contains("- jbotci gentufa</title>"));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn metadata_worker_panic_returns_sanitized_500() {
        let app = router_with_page_meta_builder(
            test_config(test_static_dir()),
            panicking_page_meta_builder,
        );
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/gentufa?text=mi+klama")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = response_text(response).await;
        assert_eq!(body, crate::error::ServerErrorCode::Failed.message());
        assert!(!body.contains("deterministic metadata worker failure"));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn spa_cukta_and_vlacku_metadata_include_canonical_social_tags() {
        let app = router(test_config(test_static_dir()));
        let cukta = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/jbotci/cukta/index")
                    .header(HOST, "example.test")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let cukta_body = response_text(cukta).await;
        assert!(cukta_body.contains("<title>jbotci CLL - CLL index</title>"));
        assert!(
            cukta_body
                .contains("Browse indexed CLL terms and jump directly into the embedded book.")
        );
        assert!(
            cukta_body
                .contains("property=\"og:url\" content=\"http://example.test/jbotci/cukta/index\"")
        );
        assert!(cukta_body.contains("name=\"twitter:title\" content=\"jbotci CLL - CLL index\""));

        let vlacku = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/jbotci/vlacku/klama")
                    .header(HOST, "example.test")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let vlacku_body = response_text(vlacku).await;
        assert!(vlacku_body.contains("<title>klama - jbotci vlacku</title>"));
        assert!(vlacku_body.contains("comes/goes"));
        assert!(
            vlacku_body.contains(
                "property=\"og:url\" content=\"http://example.test/jbotci/vlacku/klama\""
            )
        );

        let gimfihi = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/gimfihi?preset=1995&source=cmn%3A%3Auan&source=hin%3A%3Arakan&source=eng%3A%3Aekspekt&source=spa%3A%3Aesper&source=rus%3A%3Apredpologa&source=ara%3A%3Amulud&shape=ccvcv&shape=cvccv&letters=source&check-collisions=none&require-free-short-rafsi=false&count=5&highlight=nanpe")
                    .header(HOST, "example.test")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let gimfihi_body = response_text(gimfihi).await;
        assert!(gimfihi_body.contains("<title>nanpe - jbotci gimfi"));
        assert!(gimfihi_body.contains("nanpe = cmn:uan ×347"));
        assert!(
            gimfihi_body
                .contains("property=\"og:url\" content=\"http://example.test/jbotci/gimfihi?")
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn spa_gimfihi_metadata_renders_original_ipa_in_shared_page_meta() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(concat!(
                        "/jbotci/gimfihi?preset=1995&source=cmn%3A%3Auan",
                        "&source=hin%3A%3Arakan&source=eng%3A%3A%5Bekspekt%5D",
                        "&source=spa%3A%3Aesper&source=rus%3A%3Apredpologa",
                        "&source=ara%3A%3Amulud&shape=ccvcv&shape=cvccv",
                        "&letters=source&check-collisions=none",
                        "&require-free-short-rafsi=false&count=5&highlight=nanpe",
                    ))
                    .header(HOST, "example.test")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_text(response).await;

        assert_eq!(body.matches("eng:[ekspekt] ×160").count(), 3);
        assert!(!body.contains("eng:ekspekt ×160"));
        assert!(body.contains("name=\"description\" content=\""));
        assert!(body.contains("property=\"og:description\" content=\""));
        assert!(body.contains("&amp;source=eng%3A%3A%5Bekspekt%5D"));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn spa_unknown_route_uses_default_dictionary_metadata() {
        let app = router(test_config(test_static_dir()));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/unknown-route")
                    .header(HOST, "example.test")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_text(response).await;
        assert!(body.contains("<title>jbotci vlacku</title>"));
        assert!(body.contains("property=\"og:url\" content=\"http://example.test/jbotci/vlacku\""));
    }
    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn exports_and_computed_metadata_use_each_shared_diagram_limit() {
        let defaults = ServerPolicy::default();
        let public_dir = test_static_dir();
        for diagram in [
            defaults.diagram.with_data(bityzba::data! { max_blocks: 1 }),
            defaults
                .diagram
                .with_data(bityzba::data! { max_columns: 1 }),
            defaults.diagram.with_data(bityzba::data! { max_pixels: 1 }),
            defaults.diagram.with_data(bityzba::data! { max_bytes: 1 }),
        ] {
            let app = router_with_policy(
                test_config(public_dir.clone()),
                defaults
                    .clone()
                    .with_data(bityzba::data! { diagram: diagram }),
            );
            for path in ["gentufa.png", "gentufa.svg", "gentufa"] {
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .uri(format!("/jbotci/{path}?text=lo+nanmu+cu+klama"))
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                assert_eq!(
                    response.status(),
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "{path} bypassed {diagram:?}"
                );
                assert_eq!(
                    response.headers()["x-jbotci-error"],
                    "output_limit_exceeded"
                );
                assert_eq!(
                    response_text(response).await,
                    crate::error::ServerErrorCode::OutputLimit.message()
                );
            }
        }
        std::fs::remove_dir_all(public_dir).unwrap();
    }
    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn a_native_worker_panic_releases_its_lease_and_returns_the_stable_failure() {
        let execution = ServerExecution::new(ServerPolicy::default());
        let services = ToolServices::with_worker_and_execution(execution.clone(), || {
            |_job| panic!("deterministic native worker failure")
        });
        let expected = SemanticSearchError::lane(WorkError::Panicked {
            lane: WorkLane::Embedding,
        });
        assert_eq!(
            hits_with_deadline(&services, Duration::from_secs(2)).await,
            Err(SemanticSearchError::lane(WorkError::Panicked {
                lane: WorkLane::Embedding
            }))
        );
        tokio::time::timeout(Duration::from_secs(2), async {
            while execution.governor.snapshot().compute_active != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            hits_with_deadline(&services, Duration::from_secs(2)).await,
            Err(expected)
        );
        assert_eq!(execution.governor.snapshot().compute_active, 0);
    }
    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn image_exports_use_the_diagram_budget_while_metadata_uses_the_text_budget() {
        let policy = ServerPolicy::default().with_data(bityzba::data! { text_bytes: 1 });
        let public_dir = test_static_dir();
        let app = router_with_policy(test_config(public_dir.clone()), policy.clone());
        for format in ["png", "svg"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/jbotci/gentufa.{format}?text=mi+klama"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "HTTP {format}");
            assert!(
                response.headers()[CONTENT_TYPE]
                    .to_str()
                    .unwrap()
                    .starts_with("image/")
            );
            let bytes = to_bytes(response.into_body(), policy.response_bytes)
                .await
                .unwrap();
            assert!(!bytes.is_empty());
            assert!(bytes.len() <= policy.diagram.max_bytes);
            drop(bytes);
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(Method::POST)
                        .uri("/mcp")
                        .header(CONTENT_TYPE, "application/json")
                        .body(Body::from(
                            serde_json::json!({
                                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                                "params": {"name": "gentufa", "arguments": {
                                    "text": "mi klama", "format": format, "show-defs": false,
                                }},
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let json = response_json(response).await;
            assert_ne!(
                json["result"]["isError"],
                serde_json::json!(true),
                "MCP {format}: {json}"
            );
            let content = &json["result"]["content"][0];
            if format == "png" {
                assert_eq!(content["type"], "image");
                assert!(!content["data"].as_str().unwrap().is_empty());
            } else {
                assert_eq!(content["type"], "text");
                assert!(content["text"].as_str().unwrap().contains("<svg"));
            }
        }
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/gentufa?text=mi+klama")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response.headers()["x-jbotci-error"],
            "output_limit_exceeded"
        );
        std::fs::remove_dir_all(public_dir).unwrap();
    }
    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn framework_errors_use_the_wire_limit_while_health_and_assets_stay_available() {
        let public_dir = test_static_dir();
        std::fs::create_dir_all(public_dir.join("assets")).unwrap();
        std::fs::write(public_dir.join("assets/limit-test.js"), "static contents").unwrap();
        let policy = ServerPolicy::default().with_data(bityzba::data! { response_bytes: 1 });
        let app = router_with_policy(test_config(public_dir.clone()), policy);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/gentufa")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"text":null}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            response.headers()["x-jbotci-error"],
            "output_limit_exceeded"
        );
        assert_eq!(
            response_text(response).await,
            crate::error::ServerErrorCode::OutputLimit.message()
        );
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_json(response).await["status"], "ok");
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/jbotci/assets/limit-test.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_text(response).await, "static contents");
        std::fs::remove_dir_all(public_dir).unwrap();
    }
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
fn router_with_page_meta_builder(
    config: ServerConfig,
    page_meta_builder: PageMetaBuilder,
) -> Router {
    router_with_builder_and_policy(config, page_meta_builder, ServerPolicy::default())
}
