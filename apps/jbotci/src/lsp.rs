//! Language Server Protocol adapter for the `jbotci lsp` stdio subcommand.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::ops::ControlFlow;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering as AtomicOrdering},
};
use std::time::Duration;

use anyhow::{Context, Result};
use async_lsp::lsp_types::{
    self, CompletionItemKind, CompletionItemLabelDetails, CompletionOptions, CompletionResponse,
    CompletionTextEdit, DiagnosticRelatedInformation, DiagnosticServerCapabilities,
    DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
    Documentation, FullDocumentDiagnosticReport, InitializeParams, InitializeResult, InlayHint,
    InlayHintLabel, InlayHintOptions, InlayHintServerCapabilities, Location, NumberOrString, OneOf,
    PositionEncodingKind, PublishDiagnosticsParams, Range, RelatedFullDocumentDiagnosticReport,
    RelatedUnchangedDocumentDiagnosticReport, SemanticTokensFullOptions, SemanticTokensLegend,
    SemanticTokensOptions, ServerCapabilities, ServerInfo, TextDocumentContentChangeEvent,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, TextEdit,
    UnchangedDocumentDiagnosticReport, Url, WorkDoneProgressOptions, notification, request,
};
use async_lsp::router::Router;
use async_lsp::server::LifecycleLayer;
use async_lsp::{ClientSocket, ErrorCode, ResponseError};
#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_diagnostics::{
    Diagnostic as JbotciDiagnostic, DiagnosticPhase, DiagnosticSeverity as JbotciSeverity,
    diagnostic_text_segments_text,
};
use jbotci_ide::{
    CompletionCancellationToken, CompletionDocumentationHandle, CompletionItem as JbotciCompletion,
    CompletionKind, DecorationProfile, DiagnosticSnapshot, DocumentSnapshot, LineIndex,
    MAX_POSITION_VALUE, Position, PositionEncoding, PositionRange, PreparedDocumentAnalysis,
    SemanticTokenKind, completion_documentation_markdown,
};
use jbotci_syntax::{SyntaxExpectationReason, SyntaxExpectationReasonData};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::watch;
use tokio::task::AbortHandle;
use tower::ServiceBuilder;

const DIAGNOSTIC_IDENTIFIER: &str = "jbotci";
const SERVER_NAME: &str = "jbotci";
const COMPLETION_DATA_WORD: &str = "jbotciWord";
const DEBOUNCE_DELAY: Duration = Duration::from_millis(200);
const MIN_LSP_REQUEST_CONCURRENCY: NonZeroUsize = NonZeroUsize::new(2).unwrap();

type DiagnosticsPublisher = Arc<dyn Fn(Url, i32, Arc<DiagnosticSnapshot>) + Send + Sync + 'static>;

#[invariant(true)]
struct ServerState {
    client: ClientSocket,
    documents: DocumentStore,
    position_encoding: PositionEncoding,
    pull_diagnostics: bool,
    structure_inlay_profile: DecorationProfile,
    diagnostics_publisher: DiagnosticsPublisher,
}

#[invariant(true)]
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ServerInitializationOptions {
    structure_inlays: DecorationProfile,
}

#[invariant(true)]
#[derive(Clone)]
struct DocumentStore {
    documents: Arc<Mutex<HashMap<Url, DocumentState>>>,
    next_epoch: Arc<AtomicU64>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PublishedDiagnosticPhase {
    Provisional,
    Confirmed,
}

#[invariant(!result_id.is_empty())]
#[invariant(snapshot.version == i64::from(*version))]
#[derive(Debug)]
struct PublishedDiagnostics {
    result_id: String,
    epoch: u64,
    generation: u64,
    version: i32,
    phase: PublishedDiagnosticPhase,
    snapshot: Arc<DiagnosticSnapshot>,
}

// This is mutable state behind `DocumentStore`'s lock. Its transitions enforce
// text/version/generation coherence; a validating wrapper would prevent the
// in-place updates that the lock exists to protect. `confirmed_snapshot` is
// the only tree feature substrate and may intentionally lag `version`, while
// `latest_diagnostics` is always cleared on edit and may then hold either the
// provisional or confirmed diagnostic phase for the current generation.
#[invariant(true)]
struct DocumentState {
    text_index: LineIndex,
    version: i32,
    epoch: u64,
    generation: u64,
    confirmed_snapshot: Option<Arc<DocumentSnapshot>>,
    confirmed: watch::Sender<Option<Arc<DocumentSnapshot>>>,
    latest_diagnostics: Option<Arc<PublishedDiagnostics>>,
    diagnostics: watch::Sender<Option<Arc<PublishedDiagnostics>>>,
    pending_analysis: Option<AbortHandle>,
}

/// Cancels a detached blocking completion when its protocol future is dropped.
#[invariant(true)]
struct CompletionCancellationGuard {
    cancellation: CompletionCancellationToken,
}

impl Drop for CompletionCancellationGuard {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        self.cancellation.cancel();
    }
}

impl DocumentStore {
    #[requires(true)]
    #[ensures(true)]
    fn new() -> Self {
        Self {
            documents: Arc::new(Mutex::new(HashMap::new())),
            next_epoch: Arc::new(AtomicU64::new(1)),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn open(&self, uri: Url, text: String, version: i32, publisher: DiagnosticsPublisher) {
        let epoch = self
            .next_epoch
            .fetch_update(AtomicOrdering::Relaxed, AtomicOrdering::Relaxed, |epoch| {
                epoch.checked_add(1)
            })
            .expect("document epochs cannot wrap without reusing diagnostic result IDs");
        let generation = {
            let mut documents = self.documents.lock().expect("document store poisoned");
            if let Some(previous) = documents.remove(&uri)
                && let Some(pending) = previous.pending_analysis
            {
                pending.abort();
            }
            let (confirmed, _) = watch::channel(None);
            let (diagnostics, _) = watch::channel(None);
            let generation = 1;
            documents.insert(
                uri.clone(),
                DocumentState {
                    text_index: LineIndex::new(Arc::from(text)),
                    version,
                    epoch,
                    generation,
                    confirmed_snapshot: None,
                    confirmed,
                    latest_diagnostics: None,
                    diagnostics,
                    pending_analysis: None,
                },
            );
            generation
        };
        self.schedule_analysis(uri, generation, publisher);
    }

    /// Apply a versioned notification atomically. Stale versions and inverted ranges are rejected.
    #[requires(true)]
    #[ensures(true)]
    fn change(
        &self,
        uri: &Url,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
        encoding: PositionEncoding,
        publisher: DiagnosticsPublisher,
    ) -> bool {
        let generation = {
            let mut documents = self.documents.lock().expect("document store poisoned");
            let Some(document) = documents.get_mut(uri) else {
                return false;
            };
            if version <= document.version {
                return false;
            }

            let Some(next_index) = apply_content_changes(&document.text_index, changes, encoding)
            else {
                return false;
            };
            if let Some(pending) = document.pending_analysis.take() {
                pending.abort();
            }
            document.text_index = next_index;
            document.version = version;
            document.generation = document
                .generation
                .checked_add(1)
                .expect("strictly increasing i32 versions cannot exhaust a u64 generation");
            document.latest_diagnostics = None;
            document.diagnostics.send_replace(None);
            document.generation
        };
        self.schedule_analysis(uri.clone(), generation, publisher);
        true
    }

    #[requires(true)]
    #[ensures(true)]
    fn close(&self, uri: &Url) -> bool {
        let mut documents = self.documents.lock().expect("document store poisoned");
        let Some(document) = documents.remove(uri) else {
            return false;
        };
        if let Some(pending) = document.pending_analysis {
            pending.abort();
        }
        true
    }

    #[requires(true)]
    #[ensures(true)]
    fn schedule_analysis(&self, uri: Url, generation: u64, publisher: DiagnosticsPublisher) {
        let store = self.clone();
        let task_uri = uri.clone();
        let task = tokio::spawn(async move {
            let debounce_deadline = tokio::time::Instant::now() + DEBOUNCE_DELAY;
            let Some((text, version, confirmed)) = store.analysis_input(&task_uri, generation)
            else {
                return;
            };
            let prepared = match tokio::task::spawn_blocking(move || {
                PreparedDocumentAnalysis::prepare(confirmed.as_deref(), text, i64::from(version))
            })
            .await
            {
                Ok(prepared) => prepared,
                Err(_) => return,
            };
            if let Some(provisional) = prepared.provisional() {
                if !store.install_provisional(
                    &task_uri,
                    generation,
                    version,
                    provisional,
                    &publisher,
                ) {
                    return;
                }
                tokio::time::sleep_until(debounce_deadline).await;
            }
            let snapshot = match tokio::task::spawn_blocking(move || prepared.confirm()).await {
                Ok(snapshot) => Arc::new(snapshot),
                Err(_) => return,
            };
            store.complete_confirmation(task_uri, generation, version, snapshot, publisher);
        });
        let abort_handle = task.abort_handle();

        let mut documents = self.documents.lock().expect("document store poisoned");
        if let Some(document) = documents.get_mut(&uri)
            && document.generation == generation
        {
            document.pending_analysis = Some(abort_handle);
        } else {
            task.abort();
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analysis_input(
        &self,
        uri: &Url,
        generation: u64,
    ) -> Option<(String, i32, Option<Arc<DocumentSnapshot>>)> {
        let documents = self.documents.lock().expect("document store poisoned");
        let document = documents.get(uri)?;
        (document.generation == generation).then(|| {
            (
                document.text_index.text().to_owned(),
                document.version,
                document.confirmed_snapshot.as_ref().map(Arc::clone),
            )
        })
    }

    #[requires(snapshot.version == i64::from(version))]
    #[ensures(true)]
    fn install_provisional(
        &self,
        uri: &Url,
        generation: u64,
        version: i32,
        snapshot: Arc<DiagnosticSnapshot>,
        publisher: &DiagnosticsPublisher,
    ) -> bool {
        let publication = {
            let mut documents = self.documents.lock().expect("document store poisoned");
            let Some(document) = documents.get_mut(uri) else {
                return false;
            };
            if document.generation != generation || document.version != version {
                return false;
            }
            let publication = Arc::new(new!(PublishedDiagnostics {
                result_id: diagnostic_result_id(
                    document.epoch,
                    generation,
                    version,
                    PublishedDiagnosticPhase::Provisional,
                ),
                epoch: document.epoch,
                generation,
                version,
                phase: PublishedDiagnosticPhase::Provisional,
                snapshot,
            }));
            document.latest_diagnostics = Some(Arc::clone(&publication));
            document
                .diagnostics
                .send_replace(Some(Arc::clone(&publication)));
            publication
        };
        publisher(uri.clone(), version, Arc::clone(&publication.snapshot));
        true
    }

    #[requires(snapshot.version == i64::from(version))]
    #[ensures(true)]
    fn complete_confirmation(
        &self,
        uri: Url,
        generation: u64,
        version: i32,
        snapshot: Arc<DocumentSnapshot>,
        publisher: DiagnosticsPublisher,
    ) {
        let publication = {
            let mut documents = self.documents.lock().expect("document store poisoned");
            let Some(document) = documents.get_mut(&uri) else {
                return;
            };
            if document.generation != generation || document.version != version {
                return;
            }
            let diagnostics = Arc::new(DiagnosticSnapshot::from_confirmed(&snapshot));
            let publication = Arc::new(new!(PublishedDiagnostics {
                result_id: diagnostic_result_id(
                    document.epoch,
                    generation,
                    version,
                    PublishedDiagnosticPhase::Confirmed,
                ),
                epoch: document.epoch,
                generation,
                version,
                phase: PublishedDiagnosticPhase::Confirmed,
                snapshot: diagnostics,
            }));
            document.confirmed_snapshot = Some(Arc::clone(&snapshot));
            document.confirmed.send_replace(Some(snapshot));
            document.latest_diagnostics = Some(Arc::clone(&publication));
            document
                .diagnostics
                .send_replace(Some(Arc::clone(&publication)));
            document.pending_analysis = None;
            publication
        };
        publisher(uri, version, Arc::clone(&publication.snapshot));
    }

    /// Return the last confirmed tree, waiting only when a document has never been confirmed.
    #[requires(true)]
    #[ensures(true)]
    async fn snapshot_for_features(&self, uri: &Url) -> Option<Arc<DocumentSnapshot>> {
        let mut confirmed = {
            let documents = self.documents.lock().expect("document store poisoned");
            let document = documents.get(uri)?;
            if let Some(snapshot) = &document.confirmed_snapshot {
                return Some(Arc::clone(snapshot));
            }
            document.confirmed.subscribe()
        };

        loop {
            if let Some(snapshot) = confirmed.borrow_and_update().as_ref() {
                return Some(Arc::clone(snapshot));
            }
            if confirmed.changed().await.is_err() {
                return None;
            }
        }
    }

    /// Return diagnostics for the current or a later non-superseded generation.
    #[requires(true)]
    #[ensures(true)]
    async fn diagnostics_for_current(&self, uri: &Url) -> Option<Arc<PublishedDiagnostics>> {
        let (target_epoch, target_generation, target_version, mut diagnostics) = {
            let documents = self.documents.lock().expect("document store poisoned");
            let document = documents.get(uri)?;
            if let Some(diagnostics) = &document.latest_diagnostics {
                return Some(Arc::clone(diagnostics));
            }
            (
                document.epoch,
                document.generation,
                document.version,
                document.diagnostics.subscribe(),
            )
        };

        loop {
            if let Some(publication) = diagnostics.borrow_and_update().as_ref()
                && publication.epoch == target_epoch
                && publication.generation >= target_generation
                && publication.version >= target_version
            {
                return Some(Arc::clone(publication));
            }
            if diagnostics.changed().await.is_err() {
                return None;
            }
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn diagnostic_result_id(
    epoch: u64,
    generation: u64,
    version: i32,
    phase: PublishedDiagnosticPhase,
) -> String {
    let phase = match phase {
        PublishedDiagnosticPhase::Provisional => "provisional",
        PublishedDiagnosticPhase::Confirmed => "confirmed",
    };
    format!("{epoch}:{generation}:{version}:{phase}")
}

impl ServerState {
    #[requires(true)]
    #[ensures(true)]
    fn new(client: ClientSocket) -> Self {
        Self {
            client,
            documents: DocumentStore::new(),
            position_encoding: PositionEncoding::Utf16,
            pull_diagnostics: true,
            structure_inlay_profile: DecorationProfile::default(),
            diagnostics_publisher: no_diagnostics_publisher(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn initialize(
        &mut self,
        params: InitializeParams,
    ) -> std::result::Result<InitializeResult, ResponseError> {
        let position_encoding = negotiate_position_encoding(&params);
        let pull_diagnostics = supports_pull_diagnostics(&params);
        let diagnostic_refresh = supports_diagnostic_refresh(&params);
        let structure_inlay_profile =
            initialization_structure_inlay_profile(params.initialization_options.as_ref())?;
        self.position_encoding = position_encoding;
        self.pull_diagnostics = pull_diagnostics;
        self.structure_inlay_profile = structure_inlay_profile;
        self.diagnostics_publisher = if pull_diagnostics && diagnostic_refresh {
            pull_diagnostics_refresh_publisher(self.client.clone())
        } else if pull_diagnostics {
            no_diagnostics_publisher()
        } else {
            push_diagnostics_publisher(self.client.clone(), position_encoding)
        };

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                position_encoding: Some(lsp_position_encoding(position_encoding)),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        ..TextDocumentSyncOptions::default()
                    },
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    lsp_types::DiagnosticOptions {
                        identifier: Some(DIAGNOSTIC_IDENTIFIER.to_owned()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
                hover_provider: Some(true.into()),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(true),
                    // Lojban completion has no punctuation trigger. Identifier
                    // typing and explicit client invocation are sufficient.
                    trigger_characters: None,
                    ..CompletionOptions::default()
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensOptions {
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                        legend: semantic_tokens_legend(),
                        range: None,
                        full: Some(SemanticTokensFullOptions::Bool(true)),
                    }
                    .into(),
                ),
                inlay_hint_provider: Some(OneOf::Right(InlayHintServerCapabilities::Options(
                    InlayHintOptions {
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                        resolve_provider: Some(false),
                    },
                ))),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: SERVER_NAME.to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn did_open(&mut self, params: lsp_types::DidOpenTextDocumentParams) {
        let document = params.text_document;
        self.documents.open(
            document.uri,
            document.text,
            document.version,
            Arc::clone(&self.diagnostics_publisher),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn did_change(&mut self, params: lsp_types::DidChangeTextDocumentParams) {
        self.documents.change(
            &params.text_document.uri,
            params.text_document.version,
            params.content_changes,
            self.position_encoding,
            Arc::clone(&self.diagnostics_publisher),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn did_close(&mut self, params: lsp_types::DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.close(&uri);
        if !self.pull_diagnostics {
            let client = self.client.clone();
            let _ = client.notify::<notification::PublishDiagnostics>(
                PublishDiagnosticsParams::new(uri, Vec::new(), None),
            );
        }
    }
}

#[requires(true)]
#[ensures(true)]
pub(super) fn run() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to create the LSP runtime")?;
    let result = runtime.block_on(run_server());
    runtime.shutdown_timeout(Duration::from_millis(100));
    result
}

#[requires(true)]
#[ensures(true)]
async fn run_server() -> Result<()> {
    let (server, _) = async_lsp::MainLoop::new_server(|client| {
        let mut router = Router::new(ServerState::new(client));
        router
            .request::<request::Initialize, _>(|state, params| {
                let result = state.initialize(params);
                async move { result }
            })
            .request::<request::Shutdown, _>(|_, ()| async move { Ok(()) })
            .request::<request::DocumentDiagnosticRequest, _>(|state, params| {
                let documents = state.documents.clone();
                let encoding = state.position_encoding;
                async move { Ok(document_diagnostic(documents, encoding, params).await) }
            })
            .request::<request::HoverRequest, _>(|state, params| {
                let documents = state.documents.clone();
                let encoding = state.position_encoding;
                async move { Ok(document_hover(documents, encoding, params).await) }
            })
            .request::<request::Completion, _>(|state, params| {
                let documents = state.documents.clone();
                let encoding = state.position_encoding;
                async move { Ok(document_completion(documents, encoding, params).await) }
            })
            .request::<request::ResolveCompletionItem, _>(|_, item| async move {
                Ok(resolve_completion_item(item))
            })
            .request::<request::SemanticTokensFullRequest, _>(|state, params| {
                let documents = state.documents.clone();
                let encoding = state.position_encoding;
                async move { Ok(document_semantic_tokens(documents, encoding, params).await) }
            })
            .request::<request::InlayHintRequest, _>(|state, params| {
                let documents = state.documents.clone();
                let encoding = state.position_encoding;
                let profile = state.structure_inlay_profile.clone();
                async move { document_inlay_hints(documents, encoding, profile, params).await }
            })
            .notification::<notification::Initialized>(|_, _| ControlFlow::Continue(()))
            .notification::<notification::Exit>(|_, ()| ControlFlow::Continue(()))
            .notification::<notification::DidOpenTextDocument>(|state, params| {
                state.did_open(params);
                ControlFlow::Continue(())
            })
            .notification::<notification::DidChangeTextDocument>(|state, params| {
                state.did_change(params);
                ControlFlow::Continue(())
            })
            .notification::<notification::DidCloseTextDocument>(|state, params| {
                state.did_close(params);
                ControlFlow::Continue(())
            })
            // LSP requires servers to ignore notifications they do not handle
            // (editors freely send didSave, willSave, workspace notifications,
            // and extensions). async-lsp's default fallback instead breaks the
            // main loop, which kills the server the first time an editor saves
            // a file — so ignore unknown notifications explicitly. Unknown
            // *requests* keep async-lsp's default METHOD_NOT_FOUND response.
            .unhandled_notification(|_, _| ControlFlow::Continue(()));

        ServiceBuilder::new()
            .layer(LifecycleLayer::default())
            .layer(async_lsp::panic::CatchUnwindLayer::default())
            // One blocking analysis request must never consume the only
            // request slot and prevent shutdown on single-core hosts.
            .layer(async_lsp::concurrency::ConcurrencyLayer::new(
                std::thread::available_parallelism()
                    .unwrap_or(MIN_LSP_REQUEST_CONCURRENCY)
                    .max(MIN_LSP_REQUEST_CONCURRENCY),
            ))
            .service(router)
    });

    // Editors wire server stdio through pipes, but also through socket pairs,
    // ptys, or even regular files; async-lsp's zero-copy PipeStdin/PipeStdout
    // fast path only accepts genuine pipes. Try it first on unix and fall back
    // to tokio's thread-backed stdio, which accepts any fd type, so the server
    // runs under every spawning strategy instead of dying at startup.
    #[cfg(unix)]
    {
        use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

        // Bind the lock attempt outside the dispatch match: a partially
        // successful pair must be fully dropped (restoring the original fd
        // flags) before the fallback path reads the same fds, and scrutinee
        // temporaries live for the whole match otherwise.
        let pipes = match (
            async_lsp::stdio::PipeStdin::lock_tokio(),
            async_lsp::stdio::PipeStdout::lock_tokio(),
        ) {
            (Ok(stdin), Ok(stdout)) => Some((stdin, stdout)),
            _ => None,
        };
        match pipes {
            Some((stdin, stdout)) => {
                server
                    .run_buffered(stdin, stdout)
                    .await
                    .context("LSP transport failed")?;
            }
            None => {
                server
                    .run_buffered(
                        tokio::io::stdin().compat(),
                        tokio::io::stdout().compat_write(),
                    )
                    .await
                    .context("LSP transport failed")?;
            }
        }
    }

    #[cfg(not(unix))]
    {
        use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

        server
            .run_buffered(
                tokio::io::stdin().compat(),
                tokio::io::stdout().compat_write(),
            )
            .await
            .context("LSP transport failed")?;
    }

    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn negotiate_position_encoding(params: &InitializeParams) -> PositionEncoding {
    if params
        .capabilities
        .general
        .as_ref()
        .and_then(|general| general.position_encodings.as_ref())
        .is_some_and(|encodings| encodings.contains(&PositionEncodingKind::UTF8))
    {
        PositionEncoding::Utf8
    } else {
        PositionEncoding::Utf16
    }
}

#[requires(true)]
#[ensures(true)]
fn supports_pull_diagnostics(params: &InitializeParams) -> bool {
    params
        .capabilities
        .text_document
        .as_ref()
        .and_then(|text_document| text_document.diagnostic.as_ref())
        .is_some()
}

#[requires(true)]
#[ensures(true)]
fn supports_diagnostic_refresh(params: &InitializeParams) -> bool {
    params
        .capabilities
        .workspace
        .as_ref()
        .and_then(|workspace| workspace.diagnostic.as_ref())
        .and_then(|diagnostic| diagnostic.refresh_support)
        .unwrap_or(false)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| error.code == ErrorCode::INVALID_PARAMS))]
fn initialization_structure_inlay_profile(
    initialization_options: Option<&serde_json::Value>,
) -> std::result::Result<DecorationProfile, ResponseError> {
    let Some(initialization_options) = initialization_options else {
        return Ok(DecorationProfile::default());
    };
    serde_json::from_value::<ServerInitializationOptions>(initialization_options.clone())
        .map(|options| options.structure_inlays)
        .map_err(|error| {
            ResponseError::new(
                ErrorCode::INVALID_PARAMS,
                format!("invalid jbotci initializationOptions: {error}"),
            )
        })
}

#[requires(encoding != PositionEncoding::Utf32)]
#[ensures(true)]
fn lsp_position_encoding(encoding: PositionEncoding) -> PositionEncodingKind {
    match encoding {
        PositionEncoding::Utf8 => PositionEncodingKind::UTF8,
        PositionEncoding::Utf16 => PositionEncodingKind::UTF16,
        PositionEncoding::Utf32 => unreachable!("the server does not negotiate UTF-32"),
    }
}

#[requires(true)]
#[ensures(ret.token_types.len() == SemanticTokenKind::ALL.len())]
#[ensures(ret.token_modifiers.is_empty())]
fn semantic_tokens_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: SemanticTokenKind::ALL
            .iter()
            .map(|kind| lsp_types::SemanticTokenType::new(kind.lsp_name()))
            .collect(),
        // M1 intentionally leaves modifiers empty. Erased-word and elidable-
        // terminator modifiers belong to a later tree-aware milestone.
        token_modifiers: Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn no_diagnostics_publisher() -> DiagnosticsPublisher {
    Arc::new(|_, _, _| {})
}

#[requires(encoding != PositionEncoding::Utf32)]
#[ensures(true)]
fn push_diagnostics_publisher(
    client: ClientSocket,
    encoding: PositionEncoding,
) -> DiagnosticsPublisher {
    Arc::new(move |uri, version, snapshot| {
        let diagnostics = diagnostic_snapshot_to_lsp(&snapshot, &uri, encoding);
        let client = client.clone();
        let _ = client.notify::<notification::PublishDiagnostics>(PublishDiagnosticsParams::new(
            uri,
            diagnostics,
            Some(version),
        ));
    })
}

#[requires(true)]
#[ensures(true)]
fn pull_diagnostics_refresh_publisher(client: ClientSocket) -> DiagnosticsPublisher {
    Arc::new(move |_, _, _| {
        let client = client.clone();
        tokio::spawn(async move {
            let _ = client
                .request::<request::WorkspaceDiagnosticRefresh>(())
                .await;
        });
    })
}

#[requires(true)]
#[ensures(true)]
fn apply_content_changes(
    previous: &LineIndex,
    changes: Vec<TextDocumentContentChangeEvent>,
    encoding: PositionEncoding,
) -> Option<LineIndex> {
    let mut index = previous.clone();
    for change in changes {
        let mut text = index.text().to_owned();
        if let Some(range) = change.range {
            let start = index.byte_offset_for_position(
                Position::new(range.start.line as usize, range.start.character as usize),
                encoding,
            );
            let end = index.byte_offset_for_position(
                Position::new(range.end.line as usize, range.end.character as usize),
                encoding,
            );
            if end < start {
                return None;
            }
            text.replace_range(start..end, &change.text);
        } else {
            text = change.text;
        }
        index = LineIndex::new(Arc::from(text));
    }
    Some(index)
}

#[requires(encoding != PositionEncoding::Utf32)]
#[ensures(true)]
async fn document_hover(
    documents: DocumentStore,
    encoding: PositionEncoding,
    params: lsp_types::HoverParams,
) -> Option<lsp_types::Hover> {
    let position = params.text_document_position_params.position;
    let uri = params.text_document_position_params.text_document.uri;
    let snapshot = documents.snapshot_for_features(&uri).await?;
    let char_offset = snapshot.line_index.char_offset_for_position(
        Position::new(position.line as usize, position.character as usize),
        encoding,
    );
    let hover = snapshot.hover(char_offset)?.into_data();
    let range = snapshot
        .line_index
        .positions_for_span(&hover.span, encoding);
    Some(lsp_types::Hover {
        contents: lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: hover.markdown,
        }),
        range: Some(lsp_range(range)),
    })
}

#[requires(encoding != PositionEncoding::Utf32)]
#[ensures(true)]
async fn document_completion(
    documents: DocumentStore,
    encoding: PositionEncoding,
    params: lsp_types::CompletionParams,
) -> Option<CompletionResponse> {
    let position = params.text_document_position.position;
    let uri = params.text_document_position.text_document.uri;
    let snapshot = documents.snapshot_for_features(&uri).await?;
    let char_offset = snapshot.line_index.char_offset_for_position(
        Position::new(position.line as usize, position.character as usize),
        encoding,
    );
    let cancellation = CompletionCancellationToken::new();
    let _cancel_on_drop = CompletionCancellationGuard {
        cancellation: cancellation.clone(),
    };
    tokio::task::spawn_blocking(move || {
        let items = snapshot.completions_cancellable(char_offset, &cancellation);
        if cancellation.is_cancelled() {
            return None;
        }
        let items = items
            .into_iter()
            .map(|item| completion_to_lsp(&snapshot, encoding, item))
            .collect();
        Some(CompletionResponse::Array(items))
    })
    .await
    .ok()
    .flatten()
}

#[requires(encoding != PositionEncoding::Utf32)]
#[ensures(ret.documentation.is_none())]
fn completion_to_lsp(
    snapshot: &DocumentSnapshot,
    encoding: PositionEncoding,
    item: JbotciCompletion,
) -> lsp_types::CompletionItem {
    let reason_sort_rank = item.reason_sort_rank();
    let item = item.into_data();
    let range = snapshot
        .line_index
        .positions_for_span(&item.replacement_span, encoding);
    let label = item.label;
    lsp_types::CompletionItem {
        label: label.clone(),
        label_details: item
            .short_gloss
            .map(|description| CompletionItemLabelDetails {
                detail: None,
                description: Some(description),
            }),
        kind: Some(completion_item_kind(item.kind)),
        detail: Some(completion_reason_detail(&item.reason)),
        sort_text: Some(format!(
            "{}{}-{label}",
            item.interpretation.sort_rank(),
            reason_sort_rank,
        )),
        filter_text: Some(label.clone()),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range: lsp_range(range),
            // Pause periods are pronunciation/rendering concerns. Completion
            // inserts the morphology word and any separator required by its
            // cursor interpretation.
            new_text: item.replacement_text,
        })),
        data: Some(json!({ (COMPLETION_DATA_WORD): item.documentation.word() })),
        ..lsp_types::CompletionItem::default()
    }
}

#[requires(true)]
#[ensures(true)]
fn resolve_completion_item(mut item: lsp_types::CompletionItem) -> lsp_types::CompletionItem {
    let Some(word) = item
        .data
        .as_ref()
        .and_then(|data| data.get(COMPLETION_DATA_WORD))
        .and_then(serde_json::Value::as_str)
        .filter(|word| !word.is_empty())
    else {
        return item;
    };
    let handle = CompletionDocumentationHandle::new(word.to_owned());
    item.documentation = Some(Documentation::MarkupContent(lsp_types::MarkupContent {
        kind: lsp_types::MarkupKind::Markdown,
        value: completion_documentation_markdown(&handle),
    }));
    item
}

#[requires(true)]
#[ensures(true)]
fn completion_item_kind(kind: CompletionKind) -> CompletionItemKind {
    match kind {
        CompletionKind::Brivla => CompletionItemKind::FUNCTION,
        CompletionKind::ProSumti => CompletionItemKind::VARIABLE,
        CompletionKind::LetterWord | CompletionKind::Cmavo => CompletionItemKind::KEYWORD,
        CompletionKind::Cmevla => CompletionItemKind::VALUE,
        CompletionKind::Terminator => CompletionItemKind::OPERATOR,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn completion_reason_detail(reason: &SyntaxExpectationReason) -> String {
    match reason.as_data() {
        data!(SyntaxExpectationReason::ContinueCurrent { construct }) => {
            format!("continues {construct}")
        }
        data!(SyntaxExpectationReason::StartNested { construct }) => {
            format!("starts {construct}")
        }
        data!(SyntaxExpectationReason::EndThenStart { starts, ends }) if ends.is_empty() => {
            format!("starts {starts}")
        }
        data!(SyntaxExpectationReason::EndThenStart { starts, ends }) => {
            format!("ends {}; starts {starts}", ends.join(", "))
        }
    }
}

#[requires(encoding != PositionEncoding::Utf32)]
#[ensures(true)]
async fn document_semantic_tokens(
    documents: DocumentStore,
    encoding: PositionEncoding,
    params: lsp_types::SemanticTokensParams,
) -> Option<lsp_types::SemanticTokensResult> {
    let snapshot = documents
        .snapshot_for_features(&params.text_document.uri)
        .await?;
    Some(lsp_types::SemanticTokensResult::Tokens(
        lsp_types::SemanticTokens {
            result_id: None,
            data: snapshot_semantic_tokens(&snapshot, encoding),
        },
    ))
}

#[requires(encoding != PositionEncoding::Utf32)]
#[ensures(ret.as_ref().err().is_none_or(|error| error.code == ErrorCode::INVALID_PARAMS))]
async fn document_inlay_hints(
    documents: DocumentStore,
    encoding: PositionEncoding,
    profile: DecorationProfile,
    params: lsp_types::InlayHintParams,
) -> std::result::Result<Option<Vec<InlayHint>>, ResponseError> {
    let start = Position::new(
        params.range.start.line as usize,
        params.range.start.character as usize,
    );
    let end = Position::new(
        params.range.end.line as usize,
        params.range.end.character as usize,
    );
    if end < start {
        return Err(ResponseError::new(
            ErrorCode::INVALID_PARAMS,
            "textDocument/inlayHint range end precedes its start",
        ));
    }
    let uri = params.text_document.uri;
    let Some(snapshot) = documents.snapshot_for_features(&uri).await else {
        return Ok(None);
    };
    let requested_positions = PositionRange::new(start, end);
    let requested_span =
        snapshot
            .line_index
            .span_for_positions(&requested_positions, encoding, None);
    let inlays = snapshot
        .structure_inlays(&profile, &requested_span)
        .into_iter()
        .map(|inlay| {
            let inlay = inlay.into_data();
            let position = snapshot
                .line_index
                .positions_for_span(&inlay.anchor, encoding)
                .start;
            InlayHint {
                position: lsp_types::Position::new(position.line as u32, position.column as u32),
                label: InlayHintLabel::String(inlay.label),
                kind: None,
                text_edits: None,
                tooltip: None,
                padding_left: None,
                padding_right: None,
                data: None,
            }
        })
        .collect();
    Ok(Some(inlays))
}

#[requires(encoding != PositionEncoding::Utf32)]
#[ensures(true)]
fn snapshot_semantic_tokens(
    snapshot: &DocumentSnapshot,
    encoding: PositionEncoding,
) -> Vec<lsp_types::SemanticToken> {
    let mut result = Vec::new();
    let mut previous = None;
    for token in snapshot.semantic_tokens() {
        let positions = snapshot
            .line_index
            .positions_for_span(&token.span, encoding);
        for line in positions.start.line..=positions.end.line {
            let start = if line == positions.start.line {
                positions.start
            } else {
                Position::new(line, 0)
            };
            let end = if line == positions.end.line {
                positions.end
            } else {
                let offsets = snapshot
                    .line_index
                    .offsets_for_position(Position::new(line, MAX_POSITION_VALUE), encoding);
                snapshot
                    .line_index
                    .position_for_byte(offsets.byte, encoding)
            };
            if start.column < end.column {
                push_lsp_semantic_token(&mut result, &mut previous, start, end, token.kind);
            }
        }
    }
    result
}

#[requires(start.line == end.line && start.column < end.column)]
#[ensures(tokens.len() == old(tokens.len()) + 1)]
fn push_lsp_semantic_token(
    tokens: &mut Vec<lsp_types::SemanticToken>,
    previous: &mut Option<Position>,
    start: Position,
    end: Position,
    kind: SemanticTokenKind,
) {
    let (delta_line, delta_start) = previous.map_or((start.line, start.column), |previous| {
        let delta_line = start
            .line
            .checked_sub(previous.line)
            .expect("semantic tokens are ordered by source position");
        let delta_start = if delta_line == 0 {
            start
                .column
                .checked_sub(previous.column)
                .expect("same-line semantic tokens are ordered by source position")
        } else {
            start.column
        };
        (delta_line, delta_start)
    });
    tokens.push(lsp_types::SemanticToken {
        delta_line: delta_line as u32,
        delta_start: delta_start as u32,
        length: (end.column - start.column) as u32,
        token_type: kind.legend_index(),
        token_modifiers_bitset: 0,
    });
    *previous = Some(start);
}

#[requires(true)]
#[ensures(true)]
async fn document_diagnostic(
    documents: DocumentStore,
    encoding: PositionEncoding,
    params: DocumentDiagnosticParams,
) -> DocumentDiagnosticReportResult {
    let uri = params.text_document.uri;
    let Some(publication) = documents.diagnostics_for_current(&uri).await else {
        return full_diagnostic_report(None, Vec::new());
    };
    let result_id = publication.result_id.clone();
    if params.previous_result_id.as_deref() == Some(result_id.as_str()) {
        return DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Unchanged(
            RelatedUnchangedDocumentDiagnosticReport {
                related_documents: None,
                unchanged_document_diagnostic_report: UnchangedDocumentDiagnosticReport {
                    result_id,
                },
            },
        ));
    }
    full_diagnostic_report(
        Some(result_id),
        diagnostic_snapshot_to_lsp(&publication.snapshot, &uri, encoding),
    )
}

#[requires(true)]
#[ensures(true)]
fn full_diagnostic_report(
    result_id: Option<String>,
    diagnostics: Vec<lsp_types::Diagnostic>,
) -> DocumentDiagnosticReportResult {
    DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(
        RelatedFullDocumentDiagnosticReport {
            related_documents: None,
            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                result_id,
                items: diagnostics,
            },
        },
    ))
}

#[requires(encoding != PositionEncoding::Utf32)]
#[ensures(ret.len() == snapshot.diagnostics.len())]
fn diagnostic_snapshot_to_lsp(
    snapshot: &DiagnosticSnapshot,
    uri: &Url,
    encoding: PositionEncoding,
) -> Vec<lsp_types::Diagnostic> {
    snapshot
        .diagnostics(encoding)
        .map(|diagnostic| diagnostic_to_lsp(diagnostic.diagnostic, diagnostic.labels(), uri))
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_to_lsp<'snapshot>(
    diagnostic: &'snapshot JbotciDiagnostic,
    labels: impl ExactSizeIterator<Item = jbotci_ide::ResolvedLabel<'snapshot>>,
    uri: &Url,
) -> lsp_types::Diagnostic {
    let labels = labels.collect::<Vec<_>>();
    let primary = labels
        .iter()
        .find(|resolved| resolved.label.primary)
        .expect("jbotci diagnostic invariants require a primary label");
    let related_information = labels
        .iter()
        .filter(|resolved| !resolved.label.primary)
        .map(|resolved| DiagnosticRelatedInformation {
            location: Location {
                uri: uri.clone(),
                range: lsp_range(resolved.positions),
            },
            message: resolved.label.message.clone(),
        })
        .collect::<Vec<_>>();

    lsp_types::Diagnostic {
        range: lsp_range(primary.positions),
        severity: Some(match diagnostic.severity {
            JbotciSeverity::Error => lsp_types::DiagnosticSeverity::ERROR,
            JbotciSeverity::Warning => lsp_types::DiagnosticSeverity::WARNING,
            JbotciSeverity::Advice => lsp_types::DiagnosticSeverity::HINT,
        }),
        code: Some(NumberOrString::String(diagnostic.code.clone())),
        code_description: None,
        source: Some(match diagnostic.phase {
            DiagnosticPhase::Morphology => "jbotci/morphology".to_owned(),
            DiagnosticPhase::Syntax => "jbotci/syntax".to_owned(),
        }),
        message: plain_diagnostic_message(diagnostic),
        related_information: (!related_information.is_empty()).then_some(related_information),
        tags: None,
        data: None,
    }
}

/// Plain-text diagnostic rendering seam. The hover work can replace this with shared Markdown.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn plain_diagnostic_message(diagnostic: &JbotciDiagnostic) -> String {
    let mut message = diagnostic.message.clone();
    for note in &diagnostic.notes {
        message.push('\n');
        message.push_str(note);
    }
    for note in &diagnostic.styled_notes {
        message.push('\n');
        message.push_str(&diagnostic_text_segments_text(&note.segments));
    }
    message
}

#[requires(true)]
#[ensures(ret.start <= ret.end)]
fn lsp_range(range: jbotci_ide::PositionRange) -> Range {
    Range {
        start: lsp_types::Position::new(range.start.line as u32, range.start.column as u32),
        end: lsp_types::Position::new(range.end.line as u32, range.end.column as u32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dropped_completion_request_cancels_its_worker() {
        let cancellation = CompletionCancellationToken::new();
        {
            let _guard = CompletionCancellationGuard {
                cancellation: cancellation.clone(),
            };
            assert!(!cancellation.is_cancelled());
        }
        assert!(cancellation.is_cancelled());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn initialization_options_parse_structure_profile_and_reject_unknown_profiles() {
        let options = json!({
            "structureInlays": {
                "profile": "raw-brackets",
                "maxNestingDepth": 3,
                "constructs": "bridi-tails"
            }
        });
        let profile = initialization_structure_inlay_profile(Some(&options))
            .expect("documented structure profile");
        assert_eq!(
            serde_json::to_value(profile).expect("profile serialization"),
            options["structureInlays"],
        );

        let unknown = json!({"structureInlays": {"profile": "pandi"}});
        let error = initialization_structure_inlay_profile(Some(&unknown))
            .expect_err("unimplemented profiles must not silently change meaning");
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    }

    #[tokio::test(flavor = "current_thread")]
    #[requires(true)]
    #[ensures(true)]
    async fn superseded_analysis_never_publishes() {
        let store = DocumentStore::new();
        let uri = Url::parse("file:///superseded.jbo").expect("valid test URI");
        let published = Arc::new(Mutex::new(Vec::new()));
        let publisher: DiagnosticsPublisher = {
            let published = Arc::clone(&published);
            Arc::new(move |_, version, _| {
                published
                    .lock()
                    .expect("publication log poisoned")
                    .push(version);
            })
        };

        store.open(
            uri.clone(),
            "mi ku i do".to_owned(),
            1,
            Arc::clone(&publisher),
        );
        assert!(store.change(
            &uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "mi klama".to_owned(),
            }],
            PositionEncoding::Utf16,
            publisher,
        ));

        let snapshot = store
            .snapshot_for_features(&uri)
            .await
            .expect("replacement snapshot must complete");
        assert_eq!(snapshot.version, 2);
        tokio::time::sleep(DEBOUNCE_DELAY).await;
        assert_eq!(
            *published.lock().expect("publication log poisoned"),
            vec![2]
        );
    }

    #[tokio::test(flavor = "current_thread")]
    #[requires(true)]
    #[ensures(true)]
    async fn document_state_serves_provisional_diagnostics_but_confirmed_tree() {
        let store = DocumentStore::new();
        let uri = Url::parse("file:///incremental-state.jbo").expect("valid test URI");
        let publisher = no_diagnostics_publisher();
        let old_source = "mi klama\nni'o\ndo cadzu\nni'o\nmi ku i do";
        let new_source = "mi klama\nni'o\ndo cadzu le zarci\nni'o\nmi ku i do";

        store.open(
            uri.clone(),
            old_source.to_owned(),
            1,
            Arc::clone(&publisher),
        );
        let initial = store
            .diagnostics_for_current(&uri)
            .await
            .expect("initial confirmation");
        assert_eq!(initial.phase, PublishedDiagnosticPhase::Confirmed);

        assert!(store.change(
            &uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: new_source.to_owned(),
            }],
            PositionEncoding::Utf16,
            publisher,
        ));

        let provisional = store
            .diagnostics_for_current(&uri)
            .await
            .expect("provisional diagnostics");
        assert_eq!(provisional.phase, PublishedDiagnosticPhase::Provisional);
        assert!(provisional.result_id.ends_with(":2:provisional"));
        let tree = store
            .snapshot_for_features(&uri)
            .await
            .expect("last confirmed tree");
        assert_eq!(tree.version, 1);

        tokio::time::sleep(DEBOUNCE_DELAY + Duration::from_millis(50)).await;
        let confirmed = store
            .diagnostics_for_current(&uri)
            .await
            .expect("confirming diagnostics");
        assert_eq!(confirmed.phase, PublishedDiagnosticPhase::Confirmed);
        assert!(confirmed.result_id.ends_with(":2:confirmed"));
        assert_eq!(
            provisional.snapshot.diagnostics,
            confirmed.snapshot.diagnostics
        );

        assert!(store.close(&uri));
        store.open(
            uri.clone(),
            new_source.to_owned(),
            2,
            no_diagnostics_publisher(),
        );
        let reopened = store
            .diagnostics_for_current(&uri)
            .await
            .expect("reopened document confirmation");
        assert_ne!(
            reopened.result_id, confirmed.result_id,
            "the epoch prevents resultId reuse when a URI reopens at the same version",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn incremental_changes_use_the_previous_change_index() {
        let initial = LineIndex::new(Arc::from("𝙰«a mi ku"));
        let changed = apply_content_changes(
            &initial,
            vec![
                TextDocumentContentChangeEvent {
                    range: Some(Range::new(
                        lsp_types::Position::new(0, 5),
                        lsp_types::Position::new(0, 7),
                    )),
                    range_length: None,
                    text: "do".to_owned(),
                },
                TextDocumentContentChangeEvent {
                    range: Some(Range::new(
                        lsp_types::Position::new(0, 8),
                        lsp_types::Position::new(0, 10),
                    )),
                    range_length: None,
                    text: "klama".to_owned(),
                },
            ],
            PositionEncoding::Utf16,
        )
        .expect("ordered edits are valid");

        assert_eq!(changed.text(), "𝙰«a do klama");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn multiline_quote_payloads_are_split_into_single_line_semantic_tokens() {
        let snapshot = DocumentSnapshot::new("zoi gy one\ntwo gy".to_owned(), 1);
        assert_eq!(
            snapshot_semantic_tokens(&snapshot, PositionEncoding::Utf16),
            vec![
                lsp_types::SemanticToken {
                    delta_line: 0,
                    delta_start: 0,
                    length: 3,
                    token_type: SemanticTokenKind::QuotationMarker.legend_index(),
                    token_modifiers_bitset: 0,
                },
                lsp_types::SemanticToken {
                    delta_line: 0,
                    delta_start: 4,
                    length: 2,
                    token_type: SemanticTokenKind::QuotationMarker.legend_index(),
                    token_modifiers_bitset: 0,
                },
                lsp_types::SemanticToken {
                    delta_line: 0,
                    delta_start: 3,
                    length: 3,
                    token_type: SemanticTokenKind::String.legend_index(),
                    token_modifiers_bitset: 0,
                },
                lsp_types::SemanticToken {
                    delta_line: 1,
                    delta_start: 0,
                    length: 3,
                    token_type: SemanticTokenKind::String.legend_index(),
                    token_modifiers_bitset: 0,
                },
                lsp_types::SemanticToken {
                    delta_line: 0,
                    delta_start: 4,
                    length: 2,
                    token_type: SemanticTokenKind::QuotationMarker.legend_index(),
                    token_modifiers_bitset: 0,
                },
            ]
        );
    }
}
