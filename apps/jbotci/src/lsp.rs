//! Language Server Protocol adapter for the `jbotci lsp` stdio subcommand.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::ops::ControlFlow;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use async_lsp::lsp_types::{
    self, CompletionItemKind, CompletionItemLabelDetails, CompletionOptions, CompletionResponse,
    CompletionTextEdit, DiagnosticRelatedInformation, DiagnosticServerCapabilities,
    DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
    Documentation, FoldingRangeProviderCapability, FullDocumentDiagnosticReport, InitializeParams,
    InitializeResult, InlayHint, InlayHintLabel, InlayHintOptions, InlayHintServerCapabilities,
    Location, LogMessageParams, MessageType, NumberOrString, OneOf, PositionEncodingKind,
    PublishDiagnosticsParams, Range, RelatedFullDocumentDiagnosticReport,
    RelatedUnchangedDocumentDiagnosticReport, SelectionRangeProviderCapability,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions, ServerCapabilities,
    ServerInfo, TextDocumentContentChangeEvent, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextEdit, UnchangedDocumentDiagnosticReport, Url,
    WorkDoneProgressOptions, notification, request,
};
use async_lsp::router::Router;
use async_lsp::server::LifecycleLayer;
use async_lsp::{ClientSocket, ErrorCode, ResponseError};
#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, expensive_invariant, invariant, new, requires};
use jbotci_diagnostics::{
    Diagnostic as JbotciDiagnostic, DiagnosticPhase, DiagnosticSeverity as JbotciSeverity,
    diagnostic_text_segments_text,
};
use jbotci_ide::{
    COMPLETION_GRAMMAR_TIME_LIMIT, CompletionCancellationToken, CompletionDocumentationHandle,
    CompletionItem as JbotciCompletion, CompletionKind, CompletionResult, CompletionResultData,
    DecorationProfile, DiagnosticSnapshot, DocumentSnapshot, InlayOptions, LineIndex,
    MAX_POSITION_VALUE, Position, PositionEncoding, PositionRange, PreparedDocumentAnalysis,
    SelectionRangeChain, SemanticTokenKind, StructureBracketInlayOptions,
    completion_documentation_markdown,
};
use jbotci_syntax::{SyntaxExpectationReason, SyntaxExpectationReasonData};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::{Semaphore, watch};
use tokio::task::AbortHandle;
use tower::ServiceBuilder;

#[path = "lsp_admission.rs"]
mod lsp_admission;

const DIAGNOSTIC_IDENTIFIER: &str = "jbotci";
const SERVER_NAME: &str = "jbotci";
const COMPLETION_DATA_WORD: &str = "jbotciWord";
const ANALYSIS_FAILED_MESSAGE: &str = "document analysis failed";
const DEBOUNCE_DELAY: Duration = Duration::from_millis(200);
const MIN_WORKER_PERMITS: usize = 2;
const LEGACY_STRUCTURE_INLAYS_WARNING: &str = "initializationOptions.structureInlays is deprecated; use initializationOptions.inlays.structureBrackets instead";

type DiagnosticsPublisher = Arc<dyn Fn(publication::DiagnosticPublication) + Send + Sync + 'static>;

/// The position encoding for one LSP client.
///
/// The values are UTF-8 and UTF-16.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServerPositionEncoding {
    Utf8,
    Utf16,
}

impl ServerPositionEncoding {
    #[requires(true)]
    #[ensures(matches!(ret, PositionEncoding::Utf8 | PositionEncoding::Utf16))]
    fn to_position_encoding(self) -> PositionEncoding {
        match self {
            Self::Utf8 => PositionEncoding::Utf8,
            Self::Utf16 => PositionEncoding::Utf16,
        }
    }
}

/// One LSP client connection.
///
/// The position encoding is UTF-8 or UTF-16.
/// Every other field accepts every value of its type.
#[invariant(true)]
struct ServerState {
    client: ClientSocket,
    documents: DocumentStore,
    position_encoding: ServerPositionEncoding,
    pull_diagnostics: bool,
    inlay_options: InlayOptions,
    completion_grammar_time_limit: Option<Duration>,
    initialization_warnings: Vec<&'static str>,
    diagnostics_publisher: DiagnosticsPublisher,
}

#[invariant(true)]
#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ServerInitializationOptions {
    inlays: Option<InlayInitializationOptions>,
    structure_inlays: Option<DecorationProfile>,
    completion_grammar_time_limit_ms: Option<u64>,
}

impl Default for ServerInitializationOptions {
    #[requires(true)]
    #[ensures(ret.completion_grammar_time_limit_ms.is_some())]
    fn default() -> Self {
        Self {
            inlays: None,
            structure_inlays: None,
            completion_grammar_time_limit_ms: Some(
                u64::try_from(COMPLETION_GRAMMAR_TIME_LIMIT.as_millis())
                    .expect("the default limit fits in milliseconds"),
            ),
        }
    }
}

#[invariant(true)]
#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
struct InlayInitializationOptions {
    structure_brackets: StructureBracketsInitialization,
    word_boundaries: bool,
    rafsi_boundaries: bool,
}

impl Default for InlayInitializationOptions {
    #[requires(true)]
    #[ensures(matches!(ret.structure_brackets, StructureBracketsInitialization::Enabled(true)))]
    #[ensures(!ret.word_boundaries && !ret.rafsi_boundaries)]
    fn default() -> Self {
        Self {
            structure_brackets: StructureBracketsInitialization::Enabled(true),
            word_boundaries: false,
            rafsi_boundaries: false,
        }
    }
}

impl InlayInitializationOptions {
    #[requires(true)]
    #[ensures(ret.word_boundaries == self.word_boundaries)]
    #[ensures(ret.rafsi_boundaries == self.rafsi_boundaries)]
    fn into_options(self) -> InlayOptions {
        InlayOptions {
            structure_brackets: self.structure_brackets.into_options(),
            word_boundaries: self.word_boundaries,
            rafsi_boundaries: self.rafsi_boundaries,
        }
    }
}

#[invariant(true)]
#[invariant(::Enabled(_) => true)]
#[invariant(::Profile { .. } => true)]
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StructureBracketsInitialization {
    Enabled(bool),
    Profile {
        #[serde(flatten)]
        profile: DecorationProfile,
    },
}

impl StructureBracketsInitialization {
    #[requires(true)]
    #[ensures(true)]
    fn into_options(self) -> StructureBracketInlayOptions {
        match self {
            Self::Enabled(enabled) => StructureBracketInlayOptions {
                enabled,
                profile: DecorationProfile::default(),
            },
            Self::Profile { profile } => StructureBracketInlayOptions {
                enabled: true,
                profile,
            },
        }
    }
}

#[invariant(deprecation_warning.is_none_or(|warning| !warning.is_empty()))]
#[derive(Debug)]
struct ParsedInlayInitialization {
    options: InlayOptions,
    completion_grammar_time_limit: Option<Duration>,
    deprecation_warning: Option<&'static str>,
}

/// The open documents for one server.
///
/// `DocumentTable` keeps every open epoch below the next epoch.
/// The permit pool has no second constraint.
#[invariant(true)]
#[derive(Clone)]
struct DocumentStore {
    documents: Arc<Mutex<DocumentTable>>,
    /// Permits for blocking jobs.
    ///
    /// A permit covers one running job.
    /// A request that only waits takes none.
    /// The store is the only owner, so the server has one pool.
    worker_permits: Arc<Semaphore>,
}

/// The open documents and the next epoch.
///
/// The next epoch stays above zero.
/// Every open document uses an epoch below the next epoch.
/// That walk is an expensive contract.
#[invariant(*next_epoch > 0)]
#[expensive_invariant(documents.values().all(|document| document.revision.identity.epoch < *next_epoch))]
struct DocumentTable {
    documents: HashMap<Url, DocumentState>,
    next_epoch: u64,
}

/// The epoch and the generation of one analysis.
///
/// A second open uses a new epoch.
/// An edit uses the next generation in the same epoch.
#[invariant(*epoch > 0 && *generation > 0)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AnalysisIdentity {
    epoch: u64,
    generation: u64,
}

/// The answer of a wait for one document.
#[invariant(true)]
#[invariant(::Ready(_) => true)]
#[derive(Debug)]
enum DocumentWait<T> {
    /// The document is not open.
    Closed,
    /// This generation ended before confirmation.
    Failed,
    /// The document has this value.
    Ready(T),
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PublishedDiagnosticPhase {
    Provisional,
    Confirmed,
}

/// One published diagnostic set for one analysis.
///
/// The result id names this identity, this version, and this phase.
/// The snapshot uses this version.
#[invariant(!result_id.is_empty())]
#[invariant(snapshot.version == i64::from(*version))]
#[invariant(result_id.as_str() == diagnostic_result_id(identity.epoch, identity.generation, *version, *phase))]
#[derive(Debug)]
struct PublishedDiagnostics {
    result_id: String,
    identity: AnalysisIdentity,
    version: i32,
    phase: PublishedDiagnosticPhase,
    snapshot: Arc<DiagnosticSnapshot>,
}

mod publication {
    use std::sync::Arc;

    use async_lsp::lsp_types::Url;
    #[allow(unused_imports)]
    use bityzba::{ensures, invariant, new, requires};
    use jbotci_ide::DiagnosticSnapshot;

    use super::{AnalysisIdentity, DocumentStore};

    /// One diagnostic publication for one analysis.
    ///
    /// The identity can already be stale. `emit` rejects a stale identity.
    /// The snapshot version matches `version`.
    #[invariant(snapshot.version == i64::from(*version))]
    pub(super) struct DiagnosticPublication {
        store: DocumentStore,
        uri: Url,
        identity: AnalysisIdentity,
        version: i32,
        snapshot: Arc<DiagnosticSnapshot>,
    }

    impl DiagnosticPublication {
        #[requires(snapshot.version == i64::from(version))]
        #[ensures(ret.version == version)]
        #[ensures(ret.identity == identity)]
        #[ensures(ret.snapshot.version == i64::from(version))]
        pub(super) fn from_analysis(
            store: DocumentStore,
            uri: Url,
            identity: AnalysisIdentity,
            version: i32,
            snapshot: Arc<DiagnosticSnapshot>,
        ) -> Self {
            new!(DiagnosticPublication {
                store,
                uri,
                identity,
                version,
                snapshot,
            })
        }

        /// Send only when this analysis still owns the document.
        ///
        /// `send` runs before the document-store lock is released.
        /// A close or a reopen takes that same lock.
        #[requires(true)]
        #[ensures(true)]
        pub(super) fn emit(&self, send: impl FnOnce(&Url, i32, &DiagnosticSnapshot)) -> bool {
            let table = self
                .store
                .documents
                .lock()
                .expect("document store poisoned");
            let Some(document) = table.documents.get(&self.uri) else {
                return false;
            };
            if document.revision.identity != self.identity
                || document.revision.version != self.version
            {
                return false;
            }
            send(&self.uri, self.version, &self.snapshot);
            true
        }
    }
}

/// The identity and version of one open document, with its diagnostics.
///
/// Diagnostics are absent, or they use this identity and this version.
#[invariant(match latest_diagnostics.as_ref() {
    Some(diagnostics) => diagnostics.identity == *identity && diagnostics.version == *version,
    None => true,
})]
struct DocumentRevision {
    identity: AnalysisIdentity,
    version: i32,
    latest_diagnostics: Option<Arc<PublishedDiagnostics>>,
}

/// One open document behind the store lock.
///
/// The confirmed tree is absent, or its version is this revision or an older one.
/// A confirmed tree at this revision version has the same text as `text_index`.
/// An older confirmed tree may have different text.
/// Current diagnostics have the same text as `text_index`.
/// `revision` keeps the identity, the version, and the diagnostics together.
/// The failure flag, the watches, and the pending task accept every value.
#[invariant(match confirmed_snapshot.as_ref() {
    Some(snapshot) => snapshot.version <= i64::from(revision.version),
    None => true,
})]
#[expensive_invariant(match confirmed_snapshot.as_ref() {
    Some(snapshot) => {
        snapshot.version != i64::from(revision.version)
            || snapshot.text.as_ref() == text_index.text()
    }
    None => true,
})]
#[expensive_invariant(match revision.latest_diagnostics.as_ref() {
    Some(diagnostics) => diagnostics.snapshot.text.as_ref() == text_index.text(),
    None => true,
})]
struct DocumentState {
    text_index: LineIndex,
    revision: DocumentRevision,
    confirmed_snapshot: Option<Arc<DocumentSnapshot>>,
    /// Wakes a feature request. The value is only a signal.
    confirmed: watch::Sender<()>,
    /// Wakes a diagnostic request. The value is only a signal.
    diagnostics: watch::Sender<()>,
    /// This generation ended before confirmation.
    ///
    /// A newer edit clears this flag.
    analysis_failed: bool,
    pending_analysis: Option<AbortHandle>,
}

impl DocumentState {
    /// Apply one edit and check the snapshot against the revision.
    #[requires(true)]
    #[ensures(true)]
    fn edit(self, edit: impl FnOnce(&mut DocumentStateData)) -> Self {
        let mut data = self.into_data();
        edit(&mut data);
        Self::from_data(data)
    }
}

/// Mark this analysis failed when the task ends before confirmation.
///
/// The identity carries the epoch and the generation.
/// `disarm` runs after the confirmed state is stored and before the publisher runs.
#[invariant(true)]
struct AnalysisExitGuard {
    store: DocumentStore,
    uri: Url,
    identity: AnalysisIdentity,
    armed: bool,
}

impl AnalysisExitGuard {
    #[requires(true)]
    #[ensures(true)]
    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for AnalysisExitGuard {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        self.store.note_analysis_failure(&self.uri, self.identity);
    }
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

#[requires(true)]
#[ensures(ret >= MIN_WORKER_PERMITS)]
#[ensures(ret <= Semaphore::MAX_PERMITS)]
fn worker_permit_limit() -> usize {
    std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(MIN_WORKER_PERMITS)
        .max(MIN_WORKER_PERMITS)
        .min(Semaphore::MAX_PERMITS)
}

/// Run `job` on the blocking pool.
///
/// The permit stays inside the worker.
/// A dropped caller does not release it.
/// The permit ends when `job` returns.
#[requires(true)]
#[ensures(true)]
async fn run_with_worker_permit<T, F>(
    permits: Arc<Semaphore>,
    job: F,
) -> Result<T, tokio::task::JoinError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = Arc::clone(&permits)
        .acquire_owned()
        .await
        .expect("LSP worker semaphore stays open");
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        job()
    })
    .await
}

impl DocumentTable {
    #[requires(true)]
    #[ensures(ret.next_epoch == 1)]
    #[ensures(ret.documents.is_empty())]
    fn empty() -> Self {
        new!(DocumentTable {
            documents: HashMap::new(),
            next_epoch: 1,
        })
    }
}

impl DocumentStore {
    #[requires(worker_permits.available_permits() > 0)]
    #[ensures(true)]
    fn with_worker_permits(worker_permits: Arc<Semaphore>) -> Self {
        DocumentStore {
            documents: Arc::new(Mutex::new(DocumentTable::empty())),
            worker_permits,
        }
    }

    /// Replace the table, apply `edit`, and check the table again.
    ///
    /// Every build checks that the next epoch stays above zero.
    /// The open-epoch walk is an expensive contract.
    /// The check reads the table fields. It does not lock the map.
    #[requires(true)]
    #[ensures(true)]
    fn write_locked_documents<T>(
        mut guard: std::sync::MutexGuard<'_, DocumentTable>,
        edit: impl FnOnce(&mut DocumentTableData) -> T,
    ) -> T {
        let mut data = std::mem::replace(&mut *guard, DocumentTable::empty()).into_data();
        let result = edit(&mut data);
        *guard = DocumentTable::from_data(data);
        result
    }

    #[requires(true)]
    #[ensures(true)]
    fn edit_documents<T>(&self, edit: impl FnOnce(&mut DocumentTableData) -> T) -> T {
        let guard = self.documents.lock().expect("document store poisoned");
        Self::write_locked_documents(guard, edit)
    }

    #[requires(true)]
    #[ensures(true)]
    fn open(
        &self,
        uri: Url,
        text: String,
        version: i32,
        publisher: DiagnosticsPublisher,
        prepare: impl for<'snapshot> FnOnce(
            Option<&'snapshot DocumentSnapshot>,
            String,
            i64,
        ) -> PreparedDocumentAnalysis
        + Send
        + 'static,
    ) {
        let identity = {
            let guard = self.documents.lock().expect("document store poisoned");
            let epoch = guard.next_epoch;
            let next_epoch = epoch
                .checked_add(1)
                .expect("document epochs cannot wrap without reusing diagnostic result IDs");
            Self::write_locked_documents(guard, |data| {
                data.next_epoch = next_epoch;
                if let Some(previous) = data.documents.remove(&uri)
                    && let Some(pending) = previous.into_data().pending_analysis
                {
                    pending.abort();
                }
                let (confirmed, _) = watch::channel(());
                let (diagnostics, _) = watch::channel(());
                let identity = new!(AnalysisIdentity {
                    epoch,
                    generation: 1,
                });
                data.documents.insert(
                    uri.clone(),
                    new!(DocumentState {
                        text_index: LineIndex::new(Arc::from(text)),
                        revision: new!(DocumentRevision {
                            identity,
                            version,
                            latest_diagnostics: None,
                        }),
                        confirmed_snapshot: None,
                        confirmed,
                        diagnostics,
                        analysis_failed: false,
                        pending_analysis: None,
                    }),
                );
                identity
            })
        };
        self.schedule_analysis(uri, identity, publisher, prepare);
    }

    /// Apply a versioned notification atomically. Stale versions and inverted ranges are rejected.
    #[requires(true)]
    #[ensures(true)]
    fn change(
        &self,
        uri: &Url,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
        encoding: ServerPositionEncoding,
        publisher: DiagnosticsPublisher,
        prepare: impl for<'snapshot> FnOnce(
            Option<&'snapshot DocumentSnapshot>,
            String,
            i64,
        ) -> PreparedDocumentAnalysis
        + Send
        + 'static,
    ) -> bool {
        let identity = self.edit_documents(|data| {
            let (epoch, generation, next_index) =
                {
                    let Some(document) = data.documents.get(uri) else {
                        return None;
                    };
                    if version <= document.revision.version {
                        return None;
                    }
                    let Some(next_index) =
                        apply_content_changes(&document.text_index, changes, encoding)
                    else {
                        return None;
                    };
                    let generation =
                        document.revision.identity.generation.checked_add(1).expect(
                            "strictly increasing i32 versions cannot exhaust a u64 generation",
                        );
                    (document.revision.identity.epoch, generation, next_index)
                };
            let document = data
                .documents
                .remove(uri)
                .expect("the document stays open until the edit is stored");
            let document = document.edit(|state| {
                if let Some(pending) = state.pending_analysis.take() {
                    pending.abort();
                }
                state.text_index = next_index;
                state.revision = new!(DocumentRevision {
                    identity: new!(AnalysisIdentity { epoch, generation }),
                    version,
                    latest_diagnostics: None,
                });
                state.analysis_failed = false;
                // `send_replace` wakes a waiter when the value is already the signal.
                state.diagnostics.send_replace(());
                state.confirmed.send_replace(());
            });
            let identity = document.revision.identity;
            data.documents.insert(uri.clone(), document);
            Some(identity)
        });
        let Some(identity) = identity else {
            return false;
        };
        self.schedule_analysis(uri.clone(), identity, publisher, prepare);
        true
    }

    #[requires(true)]
    #[ensures(true)]
    fn close(&self, uri: &Url) -> bool {
        self.edit_documents(|data| {
            let Some(document) = data.documents.remove(uri) else {
                return false;
            };
            if let Some(pending) = document.into_data().pending_analysis {
                pending.abort();
            }
            true
        })
    }

    /// `prepare` turns the current text into an analysis.
    ///
    /// Production passes `PreparedDocumentAnalysis::prepare`.
    /// A test passes its own function.
    /// Generation checks, permits, the exit guard, and publication stay in this function.
    #[requires(true)]
    #[ensures(true)]
    fn schedule_analysis(
        &self,
        uri: Url,
        identity: AnalysisIdentity,
        publisher: DiagnosticsPublisher,
        prepare: impl for<'snapshot> FnOnce(
            Option<&'snapshot DocumentSnapshot>,
            String,
            i64,
        ) -> PreparedDocumentAnalysis
        + Send
        + 'static,
    ) {
        let store = self.clone();
        let task_uri = uri.clone();
        let permits = Arc::clone(&self.worker_permits);
        let task = tokio::spawn(async move {
            let debounce_deadline = tokio::time::Instant::now() + DEBOUNCE_DELAY;
            let Some((text, version, confirmed)) = store.analysis_input(&task_uri, identity) else {
                return;
            };
            // The guard owns every exit after this identity is current.
            // The task disarms the guard after the confirmed state is stored.
            let guard = AnalysisExitGuard {
                store: store.clone(),
                uri: task_uri.clone(),
                identity,
                armed: true,
            };
            let prepared = match run_with_worker_permit(Arc::clone(&permits), move || {
                prepare(confirmed.as_deref(), text, i64::from(version))
            })
            .await
            {
                Ok(prepared) => prepared,
                Err(_) => return,
            };
            if let Some(provisional) = prepared.provisional() {
                if !store.install_provisional(&task_uri, identity, version, provisional, &publisher)
                {
                    return;
                }
                tokio::time::sleep_until(debounce_deadline).await;
            }
            let snapshot = match run_with_worker_permit(permits, move || prepared.confirm()).await {
                Ok(snapshot) => Arc::new(snapshot),
                Err(_) => return,
            };
            if let Some(publication) =
                store.store_confirmation(&task_uri, identity, version, snapshot)
            {
                let outgoing = publication::DiagnosticPublication::from_analysis(
                    store.clone(),
                    task_uri.clone(),
                    identity,
                    version,
                    Arc::clone(&publication.snapshot),
                );
                guard.disarm();
                publisher(outgoing);
            }
        });
        let abort_handle = task.abort_handle();
        let stored = self.edit_documents(|data| {
            let current = data
                .documents
                .get(&uri)
                .is_some_and(|document| document.revision.identity == identity);
            if !current {
                return false;
            }
            let document = data
                .documents
                .remove(&uri)
                .expect("the document stays open until the task is stored");
            let document = document.edit(|state| {
                state.pending_analysis = Some(abort_handle);
            });
            data.documents.insert(uri, document);
            true
        });
        if !stored {
            task.abort();
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analysis_input(
        &self,
        uri: &Url,
        identity: AnalysisIdentity,
    ) -> Option<(String, i32, Option<Arc<DocumentSnapshot>>)> {
        let table = self.documents.lock().expect("document store poisoned");
        let document = table.documents.get(uri)?;
        (document.revision.identity == identity).then(|| {
            (
                document.text_index.text().to_owned(),
                document.revision.version,
                document.confirmed_snapshot.as_ref().map(Arc::clone),
            )
        })
    }

    #[requires(snapshot.version == i64::from(version))]
    #[ensures(true)]
    fn install_provisional(
        &self,
        uri: &Url,
        identity: AnalysisIdentity,
        version: i32,
        snapshot: Arc<DiagnosticSnapshot>,
        publisher: &DiagnosticsPublisher,
    ) -> bool {
        let publication = self.edit_documents(|data| {
            let current = data.documents.get(uri).is_some_and(|document| {
                document.revision.identity == identity && document.revision.version == version
            });
            if !current {
                return None;
            }
            let publication = Arc::new(new!(PublishedDiagnostics {
                result_id: diagnostic_result_id(
                    identity.epoch,
                    identity.generation,
                    version,
                    PublishedDiagnosticPhase::Provisional,
                ),
                identity,
                version,
                phase: PublishedDiagnosticPhase::Provisional,
                snapshot,
            }));
            let document = data
                .documents
                .remove(uri)
                .expect("the document stays open until provisional diagnostics are stored");
            let document = document.edit(|state| {
                state.revision = new!(DocumentRevision {
                    identity,
                    version,
                    latest_diagnostics: Some(Arc::clone(&publication)),
                });
                state.analysis_failed = false;
                state.diagnostics.send_replace(());
            });
            data.documents.insert(uri.clone(), document);
            Some(publication)
        });
        let Some(publication) = publication else {
            return false;
        };
        publisher(publication::DiagnosticPublication::from_analysis(
            self.clone(),
            uri.clone(),
            identity,
            version,
            Arc::clone(&publication.snapshot),
        ));
        true
    }

    #[requires(snapshot.version == i64::from(version))]
    #[ensures(true)]
    fn store_confirmation(
        &self,
        uri: &Url,
        identity: AnalysisIdentity,
        version: i32,
        snapshot: Arc<DocumentSnapshot>,
    ) -> Option<Arc<PublishedDiagnostics>> {
        self.edit_documents(|data| {
            let current = data.documents.get(uri).is_some_and(|document| {
                document.revision.identity == identity && document.revision.version == version
            });
            if !current {
                return None;
            }
            let diagnostics = Arc::new(DiagnosticSnapshot::from_confirmed(&snapshot));
            let publication = Arc::new(new!(PublishedDiagnostics {
                result_id: diagnostic_result_id(
                    identity.epoch,
                    identity.generation,
                    version,
                    PublishedDiagnosticPhase::Confirmed,
                ),
                identity,
                version,
                phase: PublishedDiagnosticPhase::Confirmed,
                snapshot: diagnostics,
            }));
            let document = data
                .documents
                .remove(uri)
                .expect("the document stays open until confirmation is stored");
            let document = document.edit(|state| {
                state.confirmed_snapshot = Some(snapshot);
                state.confirmed.send_replace(());
                state.revision = new!(DocumentRevision {
                    identity,
                    version,
                    latest_diagnostics: Some(Arc::clone(&publication)),
                });
                state.analysis_failed = false;
                state.diagnostics.send_replace(());
                state.pending_analysis = None;
            });
            data.documents.insert(uri.clone(), document);
            Some(publication)
        })
    }

    /// Wake waiters after this generation ends before confirmation.
    ///
    /// A different identity keeps its own result.
    /// A poisoned lock still records the failure, so this drop does not abort the process.
    #[requires(true)]
    #[ensures(true)]
    fn note_analysis_failure(&self, uri: &Url, identity: AnalysisIdentity) {
        let guard = self
            .documents
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Self::write_locked_documents(guard, |data| {
            let Some(document) = data.documents.remove(uri) else {
                return;
            };
            if document.revision.identity != identity {
                data.documents.insert(uri.clone(), document);
                return;
            }
            let document = document.edit(|state| {
                state.analysis_failed = true;
                // `send_replace` wakes a waiter when the value is already the signal.
                state.confirmed.send_replace(());
                state.diagnostics.send_replace(());
            });
            data.documents.insert(uri.clone(), document);
        });
    }

    /// Return the last confirmed tree, waiting only when a document has never been confirmed.
    ///
    /// `Failed` means this generation ended before confirmation.
    /// `Closed` means the document is not open.
    /// A later edit wakes this wait.
    #[requires(true)]
    #[ensures(true)]
    async fn snapshot_for_features(&self, uri: &Url) -> DocumentWait<Arc<DocumentSnapshot>> {
        let mut confirmed = {
            let table = self.documents.lock().expect("document store poisoned");
            let Some(document) = table.documents.get(uri) else {
                return DocumentWait::Closed;
            };
            if let Some(snapshot) = &document.confirmed_snapshot {
                return DocumentWait::Ready(Arc::clone(snapshot));
            }
            if document.analysis_failed {
                return DocumentWait::Failed;
            }
            document.confirmed.subscribe()
        };

        loop {
            if confirmed.changed().await.is_err() {
                return DocumentWait::Closed;
            }
            let table = self.documents.lock().expect("document store poisoned");
            let Some(document) = table.documents.get(uri) else {
                return DocumentWait::Closed;
            };
            if let Some(snapshot) = &document.confirmed_snapshot {
                return DocumentWait::Ready(Arc::clone(snapshot));
            }
            if document.analysis_failed {
                return DocumentWait::Failed;
            }
        }
    }

    /// Return diagnostics for this identity or a later one in the same epoch.
    ///
    /// A stored publication wins over a later failure.
    /// That keeps a provisional publication after the publisher panics.
    #[requires(true)]
    #[ensures(true)]
    async fn diagnostics_for_current(&self, uri: &Url) -> DocumentWait<Arc<PublishedDiagnostics>> {
        let (target_identity, target_version, mut diagnostics) = {
            let table = self.documents.lock().expect("document store poisoned");
            let Some(document) = table.documents.get(uri) else {
                return DocumentWait::Closed;
            };
            if let Some(diagnostics) = &document.revision.latest_diagnostics {
                return DocumentWait::Ready(Arc::clone(diagnostics));
            }
            if document.analysis_failed {
                return DocumentWait::Failed;
            }
            (
                document.revision.identity,
                document.revision.version,
                document.diagnostics.subscribe(),
            )
        };

        loop {
            if diagnostics.changed().await.is_err() {
                return DocumentWait::Closed;
            }
            let table = self.documents.lock().expect("document store poisoned");
            let Some(document) = table.documents.get(uri) else {
                return DocumentWait::Closed;
            };
            if let Some(publication) = &document.revision.latest_diagnostics
                && publication_covers(publication, target_identity, target_version)
            {
                return DocumentWait::Ready(Arc::clone(publication));
            }
            if document.analysis_failed {
                return DocumentWait::Failed;
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn publication_covers(
    publication: &PublishedDiagnostics,
    identity: AnalysisIdentity,
    version: i32,
) -> bool {
    publication.identity.epoch == identity.epoch
        && publication.identity.generation >= identity.generation
        && publication.version >= version
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
            documents: DocumentStore::with_worker_permits(Arc::new(Semaphore::new(
                worker_permit_limit(),
            ))),
            position_encoding: ServerPositionEncoding::Utf16,
            pull_diagnostics: true,
            inlay_options: InlayOptions::default(),
            completion_grammar_time_limit: Some(COMPLETION_GRAMMAR_TIME_LIMIT),
            initialization_warnings: Vec::new(),
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
        let inlay_initialization =
            initialization_inlay_options(params.initialization_options.as_ref())?;
        let inlay_initialization = inlay_initialization.into_data();
        self.position_encoding = position_encoding;
        self.pull_diagnostics = pull_diagnostics;
        self.inlay_options = inlay_initialization.options;
        self.completion_grammar_time_limit = inlay_initialization.completion_grammar_time_limit;
        self.initialization_warnings.clear();
        self.initialization_warnings
            .extend(inlay_initialization.deprecation_warning);
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
                selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
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
            PreparedDocumentAnalysis::prepare,
        );
    }

    #[requires(true)]
    #[ensures(self.initialization_warnings.is_empty())]
    fn initialized(&mut self) {
        for warning in self.initialization_warnings.drain(..) {
            let _ = self
                .client
                .notify::<notification::LogMessage>(LogMessageParams {
                    typ: MessageType::WARNING,
                    message: warning.to_owned(),
                });
        }
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
            PreparedDocumentAnalysis::prepare,
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
                async move { document_diagnostic(documents, encoding, params).await }
            })
            .request::<request::HoverRequest, _>(|state, params| {
                let documents = state.documents.clone();
                let encoding = state.position_encoding;
                async move { document_hover(documents, encoding, params).await }
            })
            .request::<request::SelectionRangeRequest, _>(|state, params| {
                let documents = state.documents.clone();
                let encoding = state.position_encoding;
                async move { document_selection_ranges(documents, encoding, params).await }
            })
            .request::<request::FoldingRangeRequest, _>(|state, params| {
                let documents = state.documents.clone();
                async move { document_folding_ranges(documents, params).await }
            })
            .request::<request::Completion, _>(|state, params| {
                let documents = state.documents.clone();
                let encoding = state.position_encoding;
                let grammar_time_limit = state.completion_grammar_time_limit;
                async move {
                    document_completion(documents, encoding, params, grammar_time_limit).await
                }
            })
            .request::<request::ResolveCompletionItem, _>(|_, item| async move {
                Ok(resolve_completion_item(item))
            })
            .request::<request::SemanticTokensFullRequest, _>(|state, params| {
                let documents = state.documents.clone();
                let encoding = state.position_encoding;
                async move { document_semantic_tokens(documents, encoding, params).await }
            })
            .request::<request::InlayHintRequest, _>(|state, params| {
                let documents = state.documents.clone();
                let encoding = state.position_encoding;
                let options = state.inlay_options.clone();
                async move { document_inlay_hints(documents, encoding, options, params).await }
            })
            .notification::<notification::Initialized>(|state, _| {
                state.initialized();
                ControlFlow::Continue(())
            })
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
            // Admission cancels a request and forwards poll_ready.
            // A worker permit covers one blocking job.
            .service(lsp_admission::request_admission(router))
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
fn negotiate_position_encoding(params: &InitializeParams) -> ServerPositionEncoding {
    if params
        .capabilities
        .general
        .as_ref()
        .and_then(|general| general.position_encodings.as_ref())
        .is_some_and(|encodings| encodings.contains(&PositionEncodingKind::UTF8))
    {
        ServerPositionEncoding::Utf8
    } else {
        ServerPositionEncoding::Utf16
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
fn initialization_inlay_options(
    initialization_options: Option<&serde_json::Value>,
) -> std::result::Result<ParsedInlayInitialization, ResponseError> {
    let Some(initialization_options) = initialization_options else {
        return Ok(new!(ParsedInlayInitialization {
            options: InlayOptions::default(),
            completion_grammar_time_limit: Some(COMPLETION_GRAMMAR_TIME_LIMIT),
            deprecation_warning: None,
        }));
    };
    let parsed =
        serde_json::from_value::<ServerInitializationOptions>(initialization_options.clone())
            .map_err(|error| {
                ResponseError::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("invalid jbotci initializationOptions: {error}"),
                )
            })?;
    let completion_grammar_time_limit = parsed
        .completion_grammar_time_limit_ms
        .map(Duration::from_millis);
    match (parsed.inlays, parsed.structure_inlays) {
        (Some(_), Some(_)) => Err(ResponseError::new(
            ErrorCode::INVALID_PARAMS,
            "invalid jbotci initializationOptions: inlays and deprecated structureInlays cannot both be set",
        )),
        (Some(inlays), None) => Ok(new!(ParsedInlayInitialization {
            options: inlays.into_options(),
            completion_grammar_time_limit: completion_grammar_time_limit,
            deprecation_warning: None,
        })),
        (None, Some(profile)) => Ok(new!(ParsedInlayInitialization {
            options: InlayOptions {
                structure_brackets: StructureBracketInlayOptions {
                    enabled: true,
                    profile,
                },
                word_boundaries: false,
                rafsi_boundaries: false,
            },
            completion_grammar_time_limit: completion_grammar_time_limit,
            deprecation_warning: Some(LEGACY_STRUCTURE_INLAYS_WARNING),
        })),
        (None, None) => Ok(new!(ParsedInlayInitialization {
            options: InlayOptions::default(),
            completion_grammar_time_limit: completion_grammar_time_limit,
            deprecation_warning: None,
        })),
    }
}

#[requires(true)]
#[ensures(true)]
fn lsp_position_encoding(encoding: ServerPositionEncoding) -> PositionEncodingKind {
    match encoding {
        ServerPositionEncoding::Utf8 => PositionEncodingKind::UTF8,
        ServerPositionEncoding::Utf16 => PositionEncodingKind::UTF16,
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
    Arc::new(|publication| {
        publication.emit(|_uri, _version, _snapshot| {});
    })
}

#[requires(true)]
#[ensures(true)]
fn push_diagnostics_publisher(
    client: ClientSocket,
    encoding: ServerPositionEncoding,
) -> DiagnosticsPublisher {
    // `notify` only queues the message. It does not take the document-store lock.
    Arc::new(move |publication| {
        publication.emit(|uri, version, snapshot| {
            let diagnostics = diagnostic_snapshot_to_lsp(snapshot, uri, encoding);
            let _ = client.notify::<notification::PublishDiagnostics>(
                PublishDiagnosticsParams::new(uri.clone(), diagnostics, Some(version)),
            );
        });
    })
}

#[requires(true)]
#[ensures(true)]
fn pull_diagnostics_refresh_publisher(client: ClientSocket) -> DiagnosticsPublisher {
    Arc::new(move |publication| {
        publication.emit(|_uri, _version, _snapshot| {
            let client = client.clone();
            tokio::spawn(async move {
                let _ = client
                    .request::<request::WorkspaceDiagnosticRefresh>(())
                    .await;
            });
        });
    })
}

#[requires(true)]
#[ensures(true)]
fn apply_content_changes(
    previous: &LineIndex,
    changes: Vec<TextDocumentContentChangeEvent>,
    encoding: ServerPositionEncoding,
) -> Option<LineIndex> {
    let encoding = encoding.to_position_encoding();
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

#[requires(true)]
#[ensures(ret.code == ErrorCode::INTERNAL_ERROR)]
#[ensures(ret.message == ANALYSIS_FAILED_MESSAGE)]
fn analysis_failed_response() -> ResponseError {
    ResponseError::new(ErrorCode::INTERNAL_ERROR, ANALYSIS_FAILED_MESSAGE)
}

#[requires(true)]
#[ensures(true)]
async fn ready_snapshot(
    documents: &DocumentStore,
    uri: &Url,
) -> Result<Option<Arc<DocumentSnapshot>>, ResponseError> {
    match documents.snapshot_for_features(uri).await {
        DocumentWait::Closed => Ok(None),
        DocumentWait::Failed => Err(analysis_failed_response()),
        DocumentWait::Ready(snapshot) => Ok(Some(snapshot)),
    }
}

#[requires(true)]
#[ensures(true)]
async fn document_hover(
    documents: DocumentStore,
    encoding: ServerPositionEncoding,
    params: lsp_types::HoverParams,
) -> Result<Option<lsp_types::Hover>, ResponseError> {
    let encoding = encoding.to_position_encoding();
    let position = params.text_document_position_params.position;
    let uri = params.text_document_position_params.text_document.uri;
    let Some(snapshot) = ready_snapshot(&documents, &uri).await? else {
        return Ok(None);
    };
    let char_offset = snapshot.line_index.char_offset_for_position(
        Position::new(position.line as usize, position.character as usize),
        encoding,
    );
    let Some(hover) = snapshot.hover(char_offset) else {
        return Ok(None);
    };
    let hover = hover.into_data();
    let range = snapshot
        .line_index
        .positions_for_span(&hover.span, encoding);
    Ok(Some(lsp_types::Hover {
        contents: lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: hover.markdown,
        }),
        range: Some(lsp_range(range)),
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().ok().is_none_or(|ranges| ranges.as_ref().is_none_or(|ranges| ranges.len() == params.positions.len())))]
#[ensures(ret.as_ref().err().is_none_or(|error| error.code == ErrorCode::INTERNAL_ERROR))]
async fn document_selection_ranges(
    documents: DocumentStore,
    encoding: ServerPositionEncoding,
    params: lsp_types::SelectionRangeParams,
) -> Result<Option<Vec<lsp_types::SelectionRange>>, ResponseError> {
    let Some(snapshot) = ready_snapshot(&documents, &params.text_document.uri).await? else {
        return Ok(None);
    };
    let offsets = params
        .positions
        .iter()
        .map(|position| {
            snapshot.line_index.char_offset_for_position(
                Position::new(position.line as usize, position.character as usize),
                encoding.to_position_encoding(),
            )
        })
        .collect::<Vec<_>>();
    Ok(Some(
        snapshot
            .selection_ranges(&offsets)
            .into_iter()
            .map(|chain| selection_range_to_lsp(&snapshot, encoding, chain))
            .collect(),
    ))
}

#[requires(true)]
#[ensures(true)]
fn selection_range_to_lsp(
    snapshot: &DocumentSnapshot,
    encoding: ServerPositionEncoding,
    chain: SelectionRangeChain,
) -> lsp_types::SelectionRange {
    let encoding = encoding.to_position_encoding();
    chain
        .spans
        .iter()
        .rev()
        .fold(None, |parent, span| {
            Some(Box::new(lsp_types::SelectionRange {
                range: lsp_range(snapshot.line_index.positions_for_span(&span, encoding)),
                parent,
            }))
        })
        .map(|range| *range)
        .expect("selection chain invariants guarantee a document range")
}

#[requires(true)]
#[ensures(ret.as_ref().ok().is_none_or(|ranges| ranges.as_ref().is_none_or(|ranges| ranges.iter().all(|range| range.start_line < range.end_line))))]
#[ensures(ret.as_ref().err().is_none_or(|error| error.code == ErrorCode::INTERNAL_ERROR))]
async fn document_folding_ranges(
    documents: DocumentStore,
    params: lsp_types::FoldingRangeParams,
) -> Result<Option<Vec<lsp_types::FoldingRange>>, ResponseError> {
    let Some(snapshot) = ready_snapshot(&documents, &params.text_document.uri).await? else {
        return Ok(None);
    };
    Ok(Some(
        snapshot
            .folding_ranges()
            .into_iter()
            .map(|range| lsp_types::FoldingRange {
                start_line: range.start_line as u32,
                start_character: None,
                end_line: range.end_line as u32,
                end_character: None,
                kind: None,
                collapsed_text: None,
            })
            .collect(),
    ))
}

#[requires(true)]
#[ensures(true)]
async fn document_completion(
    documents: DocumentStore,
    encoding: ServerPositionEncoding,
    params: lsp_types::CompletionParams,
    grammar_time_limit: Option<Duration>,
) -> Result<Option<CompletionResponse>, ResponseError> {
    let position = params.text_document_position.position;
    let uri = params.text_document_position.text_document.uri;
    let Some(snapshot) = ready_snapshot(&documents, &uri).await? else {
        return Ok(None);
    };
    let worker_permits = Arc::clone(&documents.worker_permits);
    let char_offset = snapshot.line_index.char_offset_for_position(
        Position::new(position.line as usize, position.character as usize),
        encoding.to_position_encoding(),
    );
    let cancellation = CompletionCancellationToken::new();
    let cancel_on_drop = CompletionCancellationGuard {
        cancellation: cancellation.clone(),
    };
    let completion = run_with_worker_permit(worker_permits, move || {
        let result = snapshot.completions_with_grammar_time_limit_and_cancellation(
            char_offset,
            grammar_time_limit,
            &cancellation,
        );
        if cancellation.is_cancelled() {
            return None;
        }
        Some(completion_result_to_lsp(&snapshot, encoding, result))
    })
    .await
    .ok()
    .flatten();
    drop(cancel_on_drop);
    Ok(completion)
}

#[requires(true)]
#[ensures(matches!(ret, CompletionResponse::List(_)))]
fn completion_result_to_lsp(
    snapshot: &DocumentSnapshot,
    encoding: ServerPositionEncoding,
    result: CompletionResult,
) -> CompletionResponse {
    let (items, is_incomplete) = match result.into_data() {
        data!(CompletionResult::Complete { items }) => (items, false),
        data!(CompletionResult::Incomplete { items, reason: _ }) => (items, true),
    };
    CompletionResponse::List(lsp_types::CompletionList {
        is_incomplete,
        items: items
            .into_iter()
            .map(|item| completion_to_lsp(snapshot, encoding, item))
            .collect(),
    })
}

#[requires(true)]
#[ensures(ret.documentation.is_none())]
fn completion_to_lsp(
    snapshot: &DocumentSnapshot,
    encoding: ServerPositionEncoding,
    item: JbotciCompletion,
) -> lsp_types::CompletionItem {
    let encoding = encoding.to_position_encoding();
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
        detail: Some(
            if matches!(
                item.provenance.as_data(),
                data!(jbotci_ide::CompletionProvenance::GrammarUnavailable)
            ) {
                "morphology only: grammar completion did not finish".to_owned()
            } else {
                completion_reason_detail(&item.reason)
            },
        ),
        // With an empty prefix the client renders these two blocks exactly.
        // With typed text, VS Code's fuzzy score can dominate sortText and
        // interleave them; that is accepted client behavior.
        sort_text: Some(format!("{}·{label}", u8::from(!item.document_local))),
        filter_text: Some(label.clone()),
        preselect: item.preselect.then_some(true),
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

#[requires(true)]
#[ensures(true)]
async fn document_semantic_tokens(
    documents: DocumentStore,
    encoding: ServerPositionEncoding,
    params: lsp_types::SemanticTokensParams,
) -> Result<Option<lsp_types::SemanticTokensResult>, ResponseError> {
    let Some(snapshot) = ready_snapshot(&documents, &params.text_document.uri).await? else {
        return Ok(None);
    };
    Ok(Some(lsp_types::SemanticTokensResult::Tokens(
        lsp_types::SemanticTokens {
            result_id: None,
            data: snapshot_semantic_tokens(&snapshot, encoding),
        },
    )))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| error.code == ErrorCode::INVALID_PARAMS || error.code == ErrorCode::INTERNAL_ERROR))]
async fn document_inlay_hints(
    documents: DocumentStore,
    encoding: ServerPositionEncoding,
    options: InlayOptions,
    params: lsp_types::InlayHintParams,
) -> std::result::Result<Option<Vec<InlayHint>>, ResponseError> {
    let encoding = encoding.to_position_encoding();
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
    let Some(snapshot) = ready_snapshot(&documents, &uri).await? else {
        return Ok(None);
    };
    let requested_positions = PositionRange::new(start, end);
    let requested_span =
        snapshot
            .line_index
            .span_for_positions(&requested_positions, encoding, None);
    let inlays = snapshot
        .inlays(&options, &requested_span)
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

#[requires(true)]
#[ensures(true)]
fn snapshot_semantic_tokens(
    snapshot: &DocumentSnapshot,
    encoding: ServerPositionEncoding,
) -> Vec<lsp_types::SemanticToken> {
    let encoding = encoding.to_position_encoding();
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
    encoding: ServerPositionEncoding,
    params: DocumentDiagnosticParams,
) -> Result<DocumentDiagnosticReportResult, ResponseError> {
    let uri = params.text_document.uri;
    let publication = match documents.diagnostics_for_current(&uri).await {
        DocumentWait::Closed => return Ok(full_diagnostic_report(None, Vec::new())),
        DocumentWait::Failed => return Err(analysis_failed_response()),
        DocumentWait::Ready(publication) => publication,
    };
    let result_id = publication.result_id.clone();
    if params.previous_result_id.as_deref() == Some(result_id.as_str()) {
        return Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Unchanged(RelatedUnchangedDocumentDiagnosticReport {
                related_documents: None,
                unchanged_document_diagnostic_report: UnchangedDocumentDiagnosticReport {
                    result_id,
                },
            }),
        ));
    }
    Ok(full_diagnostic_report(
        Some(result_id),
        diagnostic_snapshot_to_lsp(&publication.snapshot, &uri, encoding),
    ))
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

#[requires(true)]
#[ensures(ret.len() == snapshot.diagnostics.len())]
fn diagnostic_snapshot_to_lsp(
    snapshot: &DiagnosticSnapshot,
    uri: &Url,
    encoding: ServerPositionEncoding,
) -> Vec<lsp_types::Diagnostic> {
    let encoding = encoding.to_position_encoding();
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
            DiagnosticPhase::SemanticProjection => "jbotci/semantic-projection".to_owned(),
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
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

    use super::*;

    impl DocumentStore {
        #[requires(true)]
        #[ensures(true)]
        fn new() -> Self {
            Self::with_worker_permits(Arc::new(Semaphore::new(worker_permit_limit())))
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn document_state_with_snapshot(
        version: i32,
        snapshot: Option<Arc<DocumentSnapshot>>,
    ) -> Result<DocumentState, DocumentStateInvariantError> {
        let (confirmed, _) = watch::channel(());
        let (diagnostics, _) = watch::channel(());
        bityzba::try_new!(DocumentState {
            text_index: LineIndex::new(Arc::from("coi")),
            revision: new!(DocumentRevision {
                identity: new!(AnalysisIdentity {
                    epoch: 1,
                    generation: 1,
                }),
                version,
                latest_diagnostics: None,
            }),
            confirmed_snapshot: snapshot,
            confirmed,
            diagnostics,
            analysis_failed: false,
            pending_analysis: None,
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn document_state_rejects_a_confirmed_snapshot_from_the_future() {
        let lagging = Arc::new(DocumentSnapshot::new("coi".to_owned(), 1));
        let current = Arc::new(DocumentSnapshot::new("coi".to_owned(), 2));
        let future = Arc::new(DocumentSnapshot::new("coi".to_owned(), 3));

        let older = document_state_with_snapshot(2, Some(Arc::clone(&lagging)))
            .expect("an older confirmed tree is valid");
        assert_eq!(older.revision.version, 2);
        assert_eq!(
            older
                .confirmed_snapshot
                .as_ref()
                .expect("lagging tree")
                .version,
            1
        );
        let same = document_state_with_snapshot(2, Some(current))
            .expect("a confirmed tree at the revision version is valid");
        assert_eq!(
            same.confirmed_snapshot
                .as_ref()
                .expect("current tree")
                .version,
            2
        );
        let absent = document_state_with_snapshot(2, None).expect("no confirmed tree is valid");
        assert!(absent.confirmed_snapshot.is_none());
        assert!(
            document_state_with_snapshot(2, Some(future)).is_err(),
            "a confirmed tree newer than the revision is rejected"
        );
    }

    #[requires(!text.is_empty())]
    #[requires(epoch > 0)]
    #[ensures(ret.version == version)]
    #[expensive_ensures(ret.snapshot.text.as_ref() == text)]
    fn published_diagnostics_for_text(
        text: &str,
        version: i32,
        epoch: u64,
    ) -> Arc<PublishedDiagnostics> {
        let snapshot = Arc::new(DocumentSnapshot::new(text.to_owned(), i64::from(version)));
        let diagnostics = Arc::new(DiagnosticSnapshot::from_confirmed(&snapshot));
        let identity = new!(AnalysisIdentity {
            epoch,
            generation: 1,
        });
        Arc::new(new!(PublishedDiagnostics {
            result_id: diagnostic_result_id(
                identity.epoch,
                identity.generation,
                version,
                PublishedDiagnosticPhase::Confirmed,
            ),
            identity,
            version,
            phase: PublishedDiagnosticPhase::Confirmed,
            snapshot: diagnostics,
        }))
    }

    #[requires(!text.is_empty())]
    #[requires(epoch > 0)]
    #[ensures(true)]
    fn try_document_state(
        text: &str,
        version: i32,
        epoch: u64,
        snapshot: Option<Arc<DocumentSnapshot>>,
        latest_diagnostics: Option<Arc<PublishedDiagnostics>>,
    ) -> Result<DocumentState, DocumentStateInvariantError> {
        let (confirmed, _) = watch::channel(());
        let (diagnostics, _) = watch::channel(());
        bityzba::try_new!(DocumentState {
            text_index: LineIndex::new(Arc::from(text)),
            revision: new!(DocumentRevision {
                identity: new!(AnalysisIdentity {
                    epoch,
                    generation: 1,
                }),
                version,
                latest_diagnostics,
            }),
            confirmed_snapshot: snapshot,
            confirmed,
            diagnostics,
            analysis_failed: false,
            pending_analysis: None,
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn document_state_requires_the_current_text_at_the_current_version() {
        let stale = Arc::new(DocumentSnapshot::new("stale".to_owned(), 7));
        let same_version = try_document_state("current", 7, 1, Some(stale), None);
        if cfg!(feature = "expensive_contracts") {
            assert!(
                same_version.is_err(),
                "same-version text must match the document"
            );
        } else {
            assert!(
                same_version.is_ok(),
                "the text compare runs with expensive contracts"
            );
        }

        let stale_diagnostics = published_diagnostics_for_text("stale", 7, 1);
        let current_diagnostics =
            try_document_state("current", 7, 1, None, Some(stale_diagnostics));
        if cfg!(feature = "expensive_contracts") {
            assert!(
                current_diagnostics.is_err(),
                "current diagnostics must match the document text"
            );
        } else {
            assert!(
                current_diagnostics.is_ok(),
                "the text compare runs with expensive contracts"
            );
        }

        let older = Arc::new(DocumentSnapshot::new("stale".to_owned(), 6));
        let older_state = try_document_state("current", 7, 1, Some(older), None)
            .expect("an older snapshot may use different text");
        assert_eq!(older_state.text_index.text(), "current");
        assert_eq!(
            older_state
                .confirmed_snapshot
                .as_ref()
                .expect("older tree")
                .text
                .as_ref(),
            "stale"
        );

        let matching = published_diagnostics_for_text("current", 7, 1);
        let current_tree = Arc::new(DocumentSnapshot::new("current".to_owned(), 7));
        let accepted = try_document_state("current", 7, 1, Some(current_tree), Some(matching))
            .expect("matching current text is valid");
        assert_eq!(accepted.revision.version, 7);
        assert_eq!(accepted.text_index.text(), "current");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn document_table_rejects_an_open_epoch_that_is_not_below_the_next_epoch() {
        assert!(
            bityzba::try_new!(DocumentTable {
                documents: HashMap::new(),
                next_epoch: 0,
            })
            .is_err(),
            "the next epoch stays above zero"
        );
        let uri = Url::parse("file:///epoch.jbo").expect("valid test URI");
        let document = document_state_with_snapshot(1, None).expect("document");
        assert_eq!(document.revision.identity.epoch, 1);
        let mut documents = HashMap::new();
        documents.insert(uri.clone(), document);
        let equal_epoch = bityzba::try_new!(DocumentTable {
            documents,
            next_epoch: 1,
        });
        if cfg!(feature = "expensive_contracts") {
            assert!(
                equal_epoch.is_err(),
                "an open epoch equal to the next epoch is rejected"
            );
        } else {
            assert!(
                equal_epoch.is_ok(),
                "the epoch relation runs with expensive contracts"
            );
        }
        let document = document_state_with_snapshot(1, None).expect("document");
        let mut documents = HashMap::new();
        documents.insert(uri, document);
        let table = bityzba::try_new!(DocumentTable {
            documents,
            next_epoch: 2,
        })
        .expect("an open epoch below the next epoch is valid");
        assert_eq!(table.next_epoch, 2);
        assert_eq!(
            table
                .documents
                .values()
                .next()
                .expect("open document")
                .revision
                .identity
                .epoch,
            1
        );
    }

    #[tokio::test(flavor = "current_thread")]
    #[requires(true)]
    #[ensures(true)]
    async fn an_open_document_keeps_its_epoch_below_the_next_epoch() {
        let store = DocumentStore::new();
        let first = Url::parse("file:///epoch-one.jbo").expect("valid test URI");
        let second = Url::parse("file:///epoch-two.jbo").expect("valid test URI");
        store.open(
            first.clone(),
            "coi".to_owned(),
            1,
            no_diagnostics_publisher(),
            PreparedDocumentAnalysis::prepare,
        );
        store.open(
            second.clone(),
            "coi".to_owned(),
            1,
            no_diagnostics_publisher(),
            PreparedDocumentAnalysis::prepare,
        );
        let table = store.documents.lock().expect("document store poisoned");
        let first_epoch = table
            .documents
            .get(&first)
            .expect("first document")
            .revision
            .identity
            .epoch;
        let second_epoch = table
            .documents
            .get(&second)
            .expect("second document")
            .revision
            .identity
            .epoch;
        assert_eq!(first_epoch, 1);
        assert_eq!(second_epoch, 2);
        assert_eq!(table.next_epoch, 3);
        assert!(first_epoch < table.next_epoch);
        assert!(second_epoch < table.next_epoch);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn published_diagnostics_reject_a_result_id_for_another_analysis() {
        let snapshot = Arc::new(DocumentSnapshot::new("coi".to_owned(), 1));
        let diagnostics = Arc::new(DiagnosticSnapshot::from_confirmed(&snapshot));
        let identity = new!(AnalysisIdentity {
            epoch: 4,
            generation: 2,
        });
        let published = new!(PublishedDiagnostics {
            result_id: diagnostic_result_id(
                identity.epoch,
                identity.generation,
                1,
                PublishedDiagnosticPhase::Confirmed,
            ),
            identity,
            version: 1,
            phase: PublishedDiagnosticPhase::Confirmed,
            snapshot: Arc::clone(&diagnostics),
        });
        assert_eq!(published.result_id, "4:2:1:confirmed");
        assert!(
            bityzba::try_new!(PublishedDiagnostics {
                result_id: "other".to_owned(),
                identity,
                version: 1,
                phase: PublishedDiagnosticPhase::Confirmed,
                snapshot: diagnostics,
            })
            .is_err(),
            "the result id must name this identity, version, and phase"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn completion_limit_initialization_distinguishes_default_unlimited_and_zero() {
        let default = initialization_inlay_options(None).expect("default initialization");
        assert_eq!(
            default.completion_grammar_time_limit,
            Some(COMPLETION_GRAMMAR_TIME_LIMIT)
        );
        let missing = initialization_inlay_options(Some(&json!({}))).expect("missing limit");
        assert_eq!(
            missing.completion_grammar_time_limit,
            Some(COMPLETION_GRAMMAR_TIME_LIMIT)
        );
        let unlimited =
            initialization_inlay_options(Some(&json!({"completionGrammarTimeLimitMs": null})))
                .expect("unlimited");
        assert_eq!(unlimited.completion_grammar_time_limit, None);
        let zero = initialization_inlay_options(Some(&json!({"completionGrammarTimeLimitMs": 0})))
            .expect("zero limit");
        assert_eq!(zero.completion_grammar_time_limit, Some(Duration::ZERO));
        assert!(
            initialization_inlay_options(Some(&json!({"completionGrammarTimeLimitMs": -1})))
                .is_err()
        );
    }

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
    fn initialization_options_toggle_inlay_kinds_independently() {
        for structure_brackets in [false, true] {
            for word_boundaries in [false, true] {
                for rafsi_boundaries in [false, true] {
                    let initialization = initialization_inlay_options(Some(&json!({
                        "inlays": {
                            "structureBrackets": structure_brackets,
                            "wordBoundaries": word_boundaries,
                            "rafsiBoundaries": rafsi_boundaries,
                        }
                    })))
                    .expect("documented kind-keyed inlay options");
                    assert_eq!(
                        initialization.options.structure_brackets.enabled,
                        structure_brackets,
                    );
                    assert_eq!(initialization.options.word_boundaries, word_boundaries);
                    assert_eq!(initialization.options.rafsi_boundaries, rafsi_boundaries);
                    assert!(initialization.deprecation_warning.is_none());
                }
            }
        }

        let initialization = initialization_inlay_options(Some(&json!({
            "inlays": {
                "structureBrackets": {
                    "profile": "raw-brackets",
                    "maxNestingDepth": 3,
                    "constructs": "bridi-tails"
                },
                "wordBoundaries": true,
                "rafsiBoundaries": false
            }
        })))
        .expect("structure profile is the structureBrackets kind value");
        assert!(initialization.options.structure_brackets.enabled);
        assert_eq!(
            serde_json::to_value(&initialization.options.structure_brackets.profile)
                .expect("profile serialization"),
            json!({
                "profile": "raw-brackets",
                "maxNestingDepth": 3,
                "constructs": "bridi-tails"
            }),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn legacy_structure_inlays_shape_is_deprecated_but_unambiguous() {
        let legacy_profile = json!({
            "profile": "raw-brackets",
            "maxNestingDepth": 3,
            "constructs": "bridi-tails"
        });
        let initialization = initialization_inlay_options(Some(&json!({
            "structureInlays": legacy_profile.clone()
        })))
        .expect("legacy structure profile remains accepted for one release");
        assert_eq!(
            initialization.deprecation_warning,
            Some(LEGACY_STRUCTURE_INLAYS_WARNING),
        );
        assert!(initialization.options.structure_brackets.enabled);
        assert!(!initialization.options.word_boundaries);
        assert!(!initialization.options.rafsi_boundaries);
        assert_eq!(
            serde_json::to_value(&initialization.options.structure_brackets.profile)
                .expect("profile serialization"),
            legacy_profile,
        );

        let conflict = initialization_inlay_options(Some(&json!({
            "inlays": {"structureBrackets": false},
            "structureInlays": {"profile": "raw-brackets"}
        })))
        .expect_err("new and legacy shapes must not compete silently");
        assert_eq!(conflict.code, ErrorCode::INVALID_PARAMS);

        let unknown = json!({"inlays": {"structureBrackets": {"profile": "pandi"}}});
        let error = initialization_inlay_options(Some(&unknown))
            .expect_err("unimplemented profiles must not silently change meaning");
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
    }

    #[tokio::test(flavor = "current_thread")]
    #[requires(true)]
    #[ensures(true)]
    async fn a_dropped_worker_future_keeps_the_permit_until_the_job_ends() {
        let permits = Arc::new(Semaphore::new(1));
        let started = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let started_job = Arc::clone(&started);
        let release_job = Arc::clone(&release);
        let worker = tokio::spawn(run_with_worker_permit(Arc::clone(&permits), move || {
            started_job.store(true, AtomicOrdering::SeqCst);
            while !release_job.load(AtomicOrdering::SeqCst) {
                std::thread::sleep(Duration::from_millis(1));
            }
        }));
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !started.load(AtomicOrdering::SeqCst) {
            assert!(std::time::Instant::now() < deadline, "worker did not start");
            tokio::task::yield_now().await;
        }
        assert_eq!(permits.available_permits(), 0);
        worker.abort();
        while !worker.is_finished() {
            assert!(
                std::time::Instant::now() < deadline,
                "aborted worker future did not end"
            );
            tokio::task::yield_now().await;
        }
        assert_eq!(permits.available_permits(), 0);
        release.store(true, AtomicOrdering::SeqCst);
        let release_deadline = std::time::Instant::now() + Duration::from_secs(2);
        while permits.available_permits() == 0 {
            assert!(
                std::time::Instant::now() < release_deadline,
                "worker kept the permit after the job ended"
            );
            tokio::task::yield_now().await;
        }
    }

    #[requires(true)]
    #[ensures(true)]
    async fn wait_for_signal(receiver: std::sync::mpsc::Receiver<()>, failure: &'static str) {
        tokio::task::spawn_blocking(move || receiver.recv_timeout(Duration::from_secs(5)))
            .await
            .expect("signal wait")
            .expect(failure);
    }

    #[requires(true)]
    #[ensures(true)]
    fn poll_now<F>(future: std::pin::Pin<&mut F>) -> std::task::Poll<F::Output>
    where
        F: std::future::Future,
    {
        let waker = std::task::Waker::noop();
        let mut context = std::task::Context::from_waker(&waker);
        future.poll(&mut context)
    }

    #[requires(true)]
    #[ensures(true)]
    fn expect_ready<T>(wait: DocumentWait<T>, failure: &'static str) -> T {
        match wait {
            DocumentWait::Ready(value) => value,
            DocumentWait::Closed => panic!("{failure}: the document is closed"),
            DocumentWait::Failed => panic!("{failure}: the analysis failed"),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_analysis_failed(error: &ResponseError) {
        assert_eq!(error.code, ErrorCode::INTERNAL_ERROR);
        assert_eq!(error.message, ANALYSIS_FAILED_MESSAGE);
    }

    #[requires(true)]
    #[ensures(true)]
    fn text_document(uri: &Url) -> lsp_types::TextDocumentIdentifier {
        lsp_types::TextDocumentIdentifier { uri: uri.clone() }
    }

    #[requires(true)]
    #[ensures(true)]
    fn position_params(uri: &Url) -> lsp_types::TextDocumentPositionParams {
        lsp_types::TextDocumentPositionParams {
            text_document: text_document(uri),
            position: lsp_types::Position::new(0, 0),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_empty_diagnostic_report(report: DocumentDiagnosticReportResult) {
        match report {
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
                assert!(full.full_document_diagnostic_report.result_id.is_none());
                assert!(full.full_document_diagnostic_report.items.is_empty());
            }
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Unchanged(_))
            | DocumentDiagnosticReportResult::Partial(_) => {
                panic!("a closed document is an empty full report")
            }
        }
    }

    #[tokio::test(flavor = "current_thread")]
    #[requires(true)]
    #[ensures(true)]
    async fn a_panicked_analysis_ends_feature_and_diagnostic_waits() {
        let store = DocumentStore::new();
        let uri = Url::parse("file:///panicked-analysis.jbo").expect("valid test URI");
        let (arrived_tx, arrived_rx) = std::sync::mpsc::sync_channel(1);
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        store.open(
            uri.clone(),
            "mi klama".to_owned(),
            1,
            no_diagnostics_publisher(),
            move |_confirmed: Option<&DocumentSnapshot>, _text, _version| {
                arrived_tx
                    .send(())
                    .expect("the test waits for the analysis job");
                release_rx
                    .recv()
                    .expect("the test releases the analysis job");
                panic!("test analysis failure");
            },
        );
        wait_for_signal(arrived_rx, "analysis did not reach the failure").await;
        {
            let table = store.documents.lock().expect("document store poisoned");
            let document = table.documents.get(&uri).expect("open document");
            assert!(!document.analysis_failed);
            assert!(document.confirmed_snapshot.is_none());
            assert!(document.revision.latest_diagnostics.is_none());
        }
        let mut feature = std::pin::pin!(store.snapshot_for_features(&uri));
        let mut diagnostics = std::pin::pin!(store.diagnostics_for_current(&uri));
        assert!(
            poll_now(feature.as_mut()).is_pending(),
            "the feature request must wait before the failure"
        );
        assert!(
            poll_now(diagnostics.as_mut()).is_pending(),
            "the diagnostic request must wait before the failure"
        );

        release_tx.send(()).expect("release the analysis failure");
        let feature = tokio::time::timeout(Duration::from_secs(5), feature)
            .await
            .expect("feature wait ends");
        assert!(matches!(feature, DocumentWait::Failed));
        let diagnostics = tokio::time::timeout(Duration::from_secs(5), diagnostics)
            .await
            .expect("diagnostic wait ends");
        assert!(matches!(diagnostics, DocumentWait::Failed));
    }

    #[tokio::test(flavor = "current_thread")]
    #[requires(true)]
    #[ensures(true)]
    async fn a_failed_analysis_is_an_internal_error_for_every_waiting_request() {
        let store = DocumentStore::new();
        let missing = Url::parse("file:///closed-analysis.jbo").expect("valid test URI");
        let closed = document_diagnostic(
            store.clone(),
            ServerPositionEncoding::Utf16,
            DocumentDiagnosticParams {
                text_document: text_document(&missing),
                identifier: None,
                previous_result_id: None,
                work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
                partial_result_params: lsp_types::PartialResultParams::default(),
            },
        )
        .await
        .expect("a closed document is an empty report");
        assert_empty_diagnostic_report(closed);
        let closed_hover = document_hover(
            store.clone(),
            ServerPositionEncoding::Utf16,
            lsp_types::HoverParams {
                text_document_position_params: position_params(&missing),
                work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
            },
        )
        .await
        .expect("a closed document has no hover");
        assert!(closed_hover.is_none());

        let uri = Url::parse("file:///failed-analysis.jbo").expect("valid test URI");
        let (arrived_tx, arrived_rx) = std::sync::mpsc::sync_channel(1);
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        store.open(
            uri.clone(),
            "mi klama".to_owned(),
            1,
            no_diagnostics_publisher(),
            move |_confirmed: Option<&DocumentSnapshot>, _text, _version| {
                arrived_tx
                    .send(())
                    .expect("the test waits for the analysis job");
                release_rx
                    .recv()
                    .expect("the test releases the analysis job");
                panic!("test analysis failure");
            },
        );
        wait_for_signal(arrived_rx, "analysis did not reach the failure").await;
        release_tx.send(()).expect("release the analysis failure");
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let failed = {
                let table = store.documents.lock().expect("document store poisoned");
                let document = table.documents.get(&uri).expect("the document stays open");
                document.analysis_failed
            };
            if failed {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "analysis did not record the failure"
            );
            tokio::task::yield_now().await;
        }

        let hover = document_hover(
            store.clone(),
            ServerPositionEncoding::Utf16,
            lsp_types::HoverParams {
                text_document_position_params: position_params(&uri),
                work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
            },
        )
        .await
        .expect_err("hover reports the failed analysis");
        assert_analysis_failed(&hover);

        let selection = document_selection_ranges(
            store.clone(),
            ServerPositionEncoding::Utf16,
            lsp_types::SelectionRangeParams {
                text_document: text_document(&uri),
                positions: vec![lsp_types::Position::new(0, 0)],
                work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
                partial_result_params: lsp_types::PartialResultParams::default(),
            },
        )
        .await
        .expect_err("selection range reports the failed analysis");
        assert_analysis_failed(&selection);

        let folding = document_folding_ranges(
            store.clone(),
            lsp_types::FoldingRangeParams {
                text_document: text_document(&uri),
                work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
                partial_result_params: lsp_types::PartialResultParams::default(),
            },
        )
        .await
        .expect_err("folding range reports the failed analysis");
        assert_analysis_failed(&folding);

        let completion = document_completion(
            store.clone(),
            ServerPositionEncoding::Utf16,
            lsp_types::CompletionParams {
                text_document_position: position_params(&uri),
                work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
                partial_result_params: lsp_types::PartialResultParams::default(),
                context: None,
            },
            None,
        )
        .await
        .expect_err("completion reports the failed analysis");
        assert_analysis_failed(&completion);

        let tokens = document_semantic_tokens(
            store.clone(),
            ServerPositionEncoding::Utf16,
            lsp_types::SemanticTokensParams {
                work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
                partial_result_params: lsp_types::PartialResultParams::default(),
                text_document: text_document(&uri),
            },
        )
        .await
        .expect_err("semantic tokens report the failed analysis");
        assert_analysis_failed(&tokens);

        let inlays = document_inlay_hints(
            store.clone(),
            ServerPositionEncoding::Utf16,
            InlayOptions::default(),
            lsp_types::InlayHintParams {
                work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
                text_document: text_document(&uri),
                range: Range::new(
                    lsp_types::Position::new(0, 0),
                    lsp_types::Position::new(0, 0),
                ),
            },
        )
        .await
        .expect_err("inlay hints report the failed analysis");
        assert_analysis_failed(&inlays);

        let diagnostics = document_diagnostic(
            store.clone(),
            ServerPositionEncoding::Utf16,
            DocumentDiagnosticParams {
                text_document: text_document(&uri),
                identifier: None,
                previous_result_id: None,
                work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
                partial_result_params: lsp_types::PartialResultParams::default(),
            },
        )
        .await
        .expect_err("diagnostics report the failed analysis");
        assert_analysis_failed(&diagnostics);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_publisher_panic_after_provisional_publication_ends_the_feature_wait() {
        let store = DocumentStore::new();
        let uri = Url::parse("file:///publisher-panic.jbo").expect("valid test URI");
        let old_source = "mi klama\nni'o\ndo cadzu\nni'o\nmi ku i do";
        let new_source = "mi klama\nni'o\ndo cadzu le zarci\nni'o\nmi ku i do";
        store.open(
            uri.clone(),
            old_source.to_owned(),
            1,
            no_diagnostics_publisher(),
            PreparedDocumentAnalysis::prepare,
        );
        let initial = expect_ready(
            store.diagnostics_for_current(&uri).await,
            "initial confirmation",
        );
        assert_eq!(initial.phase, PublishedDiagnosticPhase::Confirmed);

        let (job_arrived_tx, job_arrived_rx) = std::sync::mpsc::sync_channel(1);
        let (job_release_tx, job_release_rx) = std::sync::mpsc::channel();
        let (arrived_tx, arrived_rx) = std::sync::mpsc::sync_channel(1);
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let release_rx = Mutex::new(Some(release_rx));
        let publisher: DiagnosticsPublisher = Arc::new(move |_publication| {
            arrived_tx
                .send(())
                .expect("the test waits for the publisher");
            let release = release_rx
                .lock()
                .expect("publisher release poisoned")
                .take()
                .expect("the publisher runs once");
            release.recv().expect("the test releases the publisher");
            panic!("test publisher failure");
        });
        assert!(store.change(
            &uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: new_source.to_owned(),
            }],
            ServerPositionEncoding::Utf16,
            publisher,
            move |confirmed, text, version| {
                job_arrived_tx
                    .send(())
                    .expect("the test waits for the analysis job");
                job_release_rx
                    .recv()
                    .expect("the test releases the analysis job");
                PreparedDocumentAnalysis::prepare(confirmed, text, version)
            },
        ));
        wait_for_signal(job_arrived_rx, "analysis did not reach the hold").await;
        {
            // The edit cleared diagnostics. The previous tree is still stored.
            // Remove that tree so the feature request has to wait.
            store.edit_documents(|data| {
                let document = data.documents.remove(&uri).expect("open document");
                assert!(!document.analysis_failed);
                assert!(document.revision.latest_diagnostics.is_none());
                let document = document.edit(|state| {
                    state.confirmed_snapshot = None;
                });
                data.documents.insert(uri.clone(), document);
            });
        }
        let mut feature = std::pin::pin!(store.snapshot_for_features(&uri));
        let mut diagnostics = std::pin::pin!(store.diagnostics_for_current(&uri));
        assert!(
            poll_now(feature.as_mut()).is_pending(),
            "the feature request must wait before publication"
        );
        assert!(
            poll_now(diagnostics.as_mut()).is_pending(),
            "the diagnostic request must wait before publication"
        );
        job_release_tx.send(()).expect("release the analysis hold");
        wait_for_signal(arrived_rx, "publisher did not arrive").await;
        {
            let table = store.documents.lock().expect("document store poisoned");
            let document = table.documents.get(&uri).expect("open document");
            assert!(!document.analysis_failed, "the failure has not happened");
            assert!(
                document.confirmed_snapshot.is_none(),
                "the feature request has no tree"
            );
            let published = document
                .revision
                .latest_diagnostics
                .as_ref()
                .expect("provisional diagnostics");
            assert_eq!(published.phase, PublishedDiagnosticPhase::Provisional);
            assert!(published.result_id.ends_with(":2:provisional"));
        }
        // Provisional publication does not confirm a tree.
        // The feature request is still waiting when the publisher panics.
        assert!(
            poll_now(feature.as_mut()).is_pending(),
            "the feature request must still wait after provisional publication"
        );
        // `latest_diagnostics` already holds the provisional publication.
        // A new diagnostic request returns that publication. It is not waiting.
        let mut current = std::pin::pin!(store.diagnostics_for_current(&uri));
        let provisional = match poll_now(current.as_mut()) {
            std::task::Poll::Ready(wait) => expect_ready(wait, "provisional diagnostics"),
            std::task::Poll::Pending => {
                panic!("diagnostics already have a provisional publication")
            }
        };
        assert_eq!(provisional.phase, PublishedDiagnosticPhase::Provisional);
        assert!(provisional.result_id.ends_with(":2:provisional"));

        release_tx.send(()).expect("release the publisher");
        let feature = tokio::time::timeout(Duration::from_secs(5), feature)
            .await
            .expect("feature wait ends");
        assert!(matches!(feature, DocumentWait::Failed));
        // The waiter subscribed before publication.
        // The failure does not remove `latest_diagnostics`.
        // The waiter therefore keeps the provisional publication.
        let waited = expect_ready(
            tokio::time::timeout(Duration::from_secs(5), diagnostics)
                .await
                .expect("diagnostic wait ends"),
            "the provisional publication",
        );
        assert_eq!(waited.result_id, provisional.result_id);
        assert_eq!(waited.phase, PublishedDiagnosticPhase::Provisional);
        let after = expect_ready(
            tokio::time::timeout(Duration::from_secs(5), store.diagnostics_for_current(&uri))
                .await
                .expect("diagnostic read ends"),
            "the provisional publication remains",
        );
        assert_eq!(after.result_id, provisional.result_id);

        let report = document_diagnostic(
            store.clone(),
            ServerPositionEncoding::Utf16,
            DocumentDiagnosticParams {
                text_document: text_document(&uri),
                identifier: None,
                previous_result_id: None,
                work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
                partial_result_params: lsp_types::PartialResultParams::default(),
            },
        )
        .await
        .expect("a publisher panic keeps the provisional report");
        match report {
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
                assert_eq!(
                    full.full_document_diagnostic_report.result_id.as_deref(),
                    Some(provisional.result_id.as_str())
                );
            }
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Unchanged(_))
            | DocumentDiagnosticReportResult::Partial(_) => {
                panic!("the provisional answer is a full report")
            }
        }
        let hover = document_hover(
            store.clone(),
            ServerPositionEncoding::Utf16,
            lsp_types::HoverParams {
                text_document_position_params: position_params(&uri),
                work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
            },
        )
        .await
        .expect_err("a publisher panic leaves no confirmed tree");
        assert_analysis_failed(&hover);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn an_old_provisional_publication_does_not_emit_after_reopen() {
        old_publication_does_not_emit_after_reopen(PublishedDiagnosticPhase::Provisional).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn an_old_confirmed_publication_does_not_emit_after_reopen() {
        old_publication_does_not_emit_after_reopen(PublishedDiagnosticPhase::Confirmed).await;
    }

    /// Hold one publication, reopen that URI at the same version, then release it.
    ///
    /// The old analysis must not emit. The reopened document has a new epoch.
    #[requires(true)]
    #[ensures(true)]
    async fn old_publication_does_not_emit_after_reopen(held: PublishedDiagnosticPhase) {
        let store = DocumentStore::new();
        let uri = Url::parse(&format!("file:///reopen-{held:?}.jbo")).expect("valid test URI");
        let old_source = "mi klama\nni'o\ndo cadzu\nni'o\nmi ku i do";
        let new_source = "mi klama\nni'o\ndo cadzu le zarci\nni'o\nmi ku i do";
        store.open(
            uri.clone(),
            old_source.to_owned(),
            1,
            no_diagnostics_publisher(),
            PreparedDocumentAnalysis::prepare,
        );
        let initial = expect_ready(
            store.diagnostics_for_current(&uri).await,
            "initial confirmation",
        );
        assert_eq!(initial.phase, PublishedDiagnosticPhase::Confirmed);

        let (arrived_tx, arrived_rx) = std::sync::mpsc::sync_channel(1);
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (outcome_tx, outcome_rx) = std::sync::mpsc::channel();
        let release_rx = Mutex::new(Some(release_rx));
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let emitted = Arc::new(Mutex::new(Vec::<(i32, String)>::new()));
        let emitted_log = Arc::clone(&emitted);
        let publisher: DiagnosticsPublisher = Arc::new(move |publication| {
            let call = calls.fetch_add(1, AtomicOrdering::SeqCst);
            let hold = match held {
                PublishedDiagnosticPhase::Provisional => call == 0,
                PublishedDiagnosticPhase::Confirmed => call == 1,
            };
            if hold {
                arrived_tx
                    .send(())
                    .expect("the test waits for the publisher");
                let release = release_rx
                    .lock()
                    .expect("publisher release poisoned")
                    .take()
                    .expect("the held publisher runs once");
                release.recv().expect("the test releases the publisher");
            }
            let log = Arc::clone(&emitted_log);
            let sent = publication.emit(move |_uri, version, snapshot| {
                if hold {
                    log.lock()
                        .expect("publication log poisoned")
                        .push((version, snapshot.text.to_string()));
                }
            });
            if hold {
                outcome_tx
                    .send(sent)
                    .expect("the test waits for the emission");
            }
        });
        assert!(store.change(
            &uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: new_source.to_owned(),
            }],
            ServerPositionEncoding::Utf16,
            publisher,
            PreparedDocumentAnalysis::prepare,
        ));
        wait_for_signal(arrived_rx, "publisher did not arrive").await;
        {
            let table = store.documents.lock().expect("document store poisoned");
            let document = table.documents.get(&uri).expect("open document");
            let published = document
                .revision
                .latest_diagnostics
                .as_ref()
                .expect("the held publication is stored");
            assert_eq!(published.phase, held);
            assert_eq!(published.version, 2);
        }
        assert!(store.close(&uri));
        store.open(
            uri.clone(),
            new_source.to_owned(),
            2,
            no_diagnostics_publisher(),
            PreparedDocumentAnalysis::prepare,
        );
        release_tx.send(()).expect("release the publisher");
        let sent =
            tokio::task::spawn_blocking(move || outcome_rx.recv_timeout(Duration::from_secs(5)))
                .await
                .expect("emission wait")
                .expect("the old publisher finishes");
        assert!(!sent, "an old analysis emitted after reopen");
        assert!(
            emitted.lock().expect("publication log poisoned").is_empty(),
            "an old analysis recorded an emission after reopen"
        );
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
            Arc::new(move |publication| {
                publication.emit(|_uri, version, _snapshot| {
                    published
                        .lock()
                        .expect("publication log poisoned")
                        .push(version);
                });
            })
        };

        store.open(
            uri.clone(),
            "mi ku i do".to_owned(),
            1,
            Arc::clone(&publisher),
            PreparedDocumentAnalysis::prepare,
        );
        assert!(store.change(
            &uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "mi klama".to_owned(),
            }],
            ServerPositionEncoding::Utf16,
            publisher,
            PreparedDocumentAnalysis::prepare,
        ));

        let snapshot = expect_ready(
            store.snapshot_for_features(&uri).await,
            "replacement snapshot must complete",
        );
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
            PreparedDocumentAnalysis::prepare,
        );
        let initial = expect_ready(
            store.diagnostics_for_current(&uri).await,
            "initial confirmation",
        );
        assert_eq!(initial.phase, PublishedDiagnosticPhase::Confirmed);

        assert!(store.change(
            &uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: new_source.to_owned(),
            }],
            ServerPositionEncoding::Utf16,
            publisher,
            PreparedDocumentAnalysis::prepare,
        ));

        let provisional = expect_ready(
            store.diagnostics_for_current(&uri).await,
            "provisional diagnostics",
        );
        assert_eq!(provisional.phase, PublishedDiagnosticPhase::Provisional);
        assert!(provisional.result_id.ends_with(":2:provisional"));
        let tree = expect_ready(
            store.snapshot_for_features(&uri).await,
            "last confirmed tree",
        );
        assert_eq!(tree.version, 1);
        {
            let table = store.documents.lock().expect("document store poisoned");
            let document = table.documents.get(&uri).expect("open document");
            assert_eq!(document.revision.version, 2);
        }

        let mut confirmations = {
            let table = store.documents.lock().expect("document store poisoned");
            table
                .documents
                .get(&uri)
                .expect("open document")
                .confirmed
                .subscribe()
        };
        tokio::time::timeout(Duration::from_secs(30), async {
            loop {
                let confirmed = {
                    let table = store.documents.lock().expect("document store poisoned");
                    table
                        .documents
                        .get(&uri)
                        .expect("open document")
                        .confirmed_snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.version == 2)
                };
                if confirmed {
                    break;
                }
                confirmations
                    .changed()
                    .await
                    .expect("document remains open while confirmation runs");
            }
        })
        .await
        .expect("confirmation must complete within the CI-twin boundary");
        let confirmed = expect_ready(
            store.diagnostics_for_current(&uri).await,
            "confirming diagnostics",
        );
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
            PreparedDocumentAnalysis::prepare,
        );
        let reopened = expect_ready(
            store.diagnostics_for_current(&uri).await,
            "reopened document confirmation",
        );
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
            ServerPositionEncoding::Utf16,
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
            snapshot_semantic_tokens(&snapshot, ServerPositionEncoding::Utf16),
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
