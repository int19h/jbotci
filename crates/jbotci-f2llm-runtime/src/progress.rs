use std::fmt;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires};
use serde::{Deserialize, Serialize};

use crate::RuntimeFuture;

#[invariant(::LoadingManifest => true)]
#[invariant(::LoadingTokenizer => true)]
#[invariant(::LoadingModel => true)]
#[invariant(::Embedding => true)]
#[invariant(::WritingPack => true)]
#[invariant(::Validating => true)]
#[invariant(::Complete => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProgressPhase {
    LoadingManifest,
    LoadingTokenizer,
    LoadingModel,
    Embedding,
    WritingPack,
    Validating,
    Complete,
}

#[invariant(::Model => true)]
#[invariant(::Corpus => true)]
#[invariant(::Pack => true)]
#[invariant(::Validation => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProgressKind {
    Model,
    Corpus,
    Pack,
    Validation,
}

#[invariant(*total > 0)]
#[invariant(*loaded <= *total)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressCounter {
    pub kind: ProgressKind,
    pub loaded: u64,
    pub total: u64,
}

#[invariant(!status.is_empty())]
#[invariant(!detail.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressEvent {
    pub phase: ProgressPhase,
    pub status: String,
    pub detail: String,
    pub progress: Option<ProgressCounter>,
}

impl ProgressEvent {
    #[requires(!status.is_empty())]
    #[requires(!detail.is_empty())]
    #[ensures(ret.progress.is_none())]
    pub fn indeterminate(phase: ProgressPhase, status: String, detail: String) -> Self {
        new!(ProgressEvent {
            phase: phase,
            status: status,
            detail: detail,
            progress: None,
        })
    }

    #[requires(!status.is_empty())]
    #[requires(!detail.is_empty())]
    #[requires(total > 0)]
    #[requires(loaded <= total)]
    #[ensures(ret.progress.as_ref().is_some_and(|progress| progress.loaded == loaded))]
    pub fn determinate(
        phase: ProgressPhase,
        status: String,
        detail: String,
        kind: ProgressKind,
        loaded: u64,
        total: u64,
    ) -> Self {
        new!(ProgressEvent {
            phase: phase,
            status: status,
            detail: detail,
            progress: Some(new!(ProgressCounter {
                kind: kind,
                loaded: loaded,
                total: total,
            })),
        })
    }
}

#[invariant(!message.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressError {
    pub message: String,
}

impl ProgressError {
    #[requires(!message.is_empty())]
    #[ensures(!ret.message.is_empty())]
    pub fn new(message: String) -> Self {
        new!(ProgressError { message: message })
    }
}

impl fmt::Display for ProgressError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "progress sink rejected an event: {}",
            self.message
        )
    }
}

impl std::error::Error for ProgressError {}

#[contract_trait]
pub trait ProgressSink {
    /// Reports progress with async backpressure and fallible callback semantics.
    #[requires(true)]
    #[ensures(true)]
    fn report<'a>(
        &'a mut self,
        event: &'a ProgressEvent,
    ) -> RuntimeFuture<'a, Result<(), ProgressError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    #[invariant(true)]
    struct ObjectSafeProgressSink;

    #[contract_trait]
    impl ProgressSink for ObjectSafeProgressSink {
        fn report<'a>(
            &'a mut self,
            _event: &'a ProgressEvent,
        ) -> RuntimeFuture<'a, Result<(), ProgressError>> {
            Box::pin(async {
                Err(ProgressError::new(
                    "deliberate progress rejection".to_owned(),
                ))
            })
        }
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn progress_events_preserve_current_callback_shape() {
        let event = ProgressEvent::determinate(
            ProgressPhase::LoadingModel,
            "loading-model".to_owned(),
            "Loading tensor shards.".to_owned(),
            ProgressKind::Model,
            3,
            8,
        );
        let value = serde_json::to_value(&event).expect("serialize progress");
        assert_eq!(value["status"], "loading-model");
        assert_eq!(value["detail"], "Loading tensor shards.");
        assert_eq!(value["progress"]["kind"], "model");
        assert_eq!(value["progress"]["loaded"], 3);
        assert_eq!(value["progress"]["total"], 8);
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn progress_deserialization_rejects_invalid_counters() {
        let invalid = r#"{
            "phase":"embedding",
            "status":"embedding",
            "detail":"Embedding corpus.",
            "progress":{"kind":"corpus","loaded":9,"total":8}
        }"#;
        assert!(serde_json::from_str::<ProgressEvent>(invalid).is_err());
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn progress_sink_is_object_safe_async_and_fallible() {
        let mut sink = ObjectSafeProgressSink;
        let sink: &mut dyn ProgressSink = &mut sink;
        let event = ProgressEvent::indeterminate(
            ProgressPhase::LoadingManifest,
            "loading-manifest".to_owned(),
            "Loading the artifact manifest.".to_owned(),
        );
        drop(sink.report(&event));
    }
}
