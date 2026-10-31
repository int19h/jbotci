//! Admission and operation context shared by server adapters.
use crate::error::ServerErrorCode;
use crate::policy::ServerPolicy;
use crate::work::{WorkError, WorkGovernor, WorkLease, run_admitted};
use bityzba::{invariant, new, requires};
use std::sync::Arc;
use tokio::time::Instant;

#[invariant(governor.limits() == policy.work)]
#[derive(Debug)]
pub(crate) struct ServerExecution {
    pub(crate) policy: ServerPolicy,
    pub(crate) governor: Arc<WorkGovernor>,
}
impl ServerExecution {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn new(policy: ServerPolicy) -> Arc<Self> {
        Arc::new(new!(ServerExecution {
            governor: Arc::new(WorkGovernor::new(policy.work)),
            policy,
        }))
    }
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn deadline(&self, earlier: Option<Instant>) -> Result<Instant, WorkError> {
        let configured =
            Instant::now()
                .checked_add(self.policy.deadline)
                .ok_or(WorkError::Panicked {
                    lane: crate::work::WorkLane::Compute,
                })?;
        Ok(earlier.map_or(configured, |earlier| earlier.min(configured)))
    }
    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn admit(
        self: &Arc<Self>,
        earlier: Option<Instant>,
    ) -> Result<AdmittedWorkContext, WorkError> {
        let deadline = self.deadline(earlier)?;
        let lease = self.governor.acquire_compute(deadline).await?;
        Ok(new!(AdmittedWorkContext {
            execution: Arc::clone(self),
            lease,
            deadline,
        }))
    }
}

#[invariant(execution.governor.owns_compute(lease))]
#[derive(Debug, Clone)]
pub(crate) struct AdmittedWorkContext {
    pub(crate) execution: Arc<ServerExecution>,
    pub(crate) lease: WorkLease,
    pub(crate) deadline: Instant,
}
impl AdmittedWorkContext {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn run<T, F>(&self, job: F) -> Result<T, WorkError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        run_admitted(self.lease.clone(), self.deadline, job).await
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn work_error_code(error: &WorkError) -> ServerErrorCode {
    ServerErrorCode::from(error)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn work_error_response(error: WorkError) -> axum::response::Response {
    response_for_code(ServerErrorCode::from(&error), None)
}

#[requires(true)]
#[ensures(true)]
fn response_for_code(
    code: ServerErrorCode,
    context: Option<AdmittedWorkContext>,
) -> axum::response::Response {
    let body = match context {
        Some(context) => owned_body(code.message().as_bytes().to_vec(), Some(context)),
        None => axum::body::Body::from(code.message()),
    };
    axum::http::Response::builder()
        .status(code.status())
        .header("x-jbotci-error", code.as_str())
        .header("content-type", "text/plain; charset=utf-8")
        .extension(code)
        .body(body)
        .expect("fixed error response")
}

#[invariant(context.as_ref().is_none_or(|context| {
    bytes.len() <= context.execution.policy.response_bytes
        || ServerErrorCode::ALL.iter().any(|code| code.message().as_bytes() == bytes.as_slice())
}))]
struct ResponseBytes {
    bytes: Vec<u8>,
    context: Option<AdmittedWorkContext>,
}
impl AsRef<[u8]> for ResponseBytes {
    #[requires(true)]
    #[ensures(ret.len() == self.bytes.len())]
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

#[requires(context.as_ref().is_none_or(|context| {
    bytes.len() <= context.execution.policy.response_bytes
        || ServerErrorCode::ALL.iter().any(|code| code.message().as_bytes() == bytes.as_slice())
}))]
#[ensures(true)]
pub(crate) fn owned_body(bytes: Vec<u8>, context: Option<AdmittedWorkContext>) -> axum::body::Body {
    axum::body::Body::from(axum::body::Bytes::from_owner(new!(ResponseBytes {
        bytes,
        context,
    })))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn json_response<T: serde::Serialize>(
    status: axum::http::StatusCode,
    value: &T,
    policy: &ServerPolicy,
    context: Option<AdmittedWorkContext>,
) -> axum::response::Response {
    use axum::http::Response;
    let limit =
        jbotci_output::budget::ByteLimit::new(policy.response_bytes).expect("validated policy");
    match jbotci_output::budget::bounded_json_with_text_budget(
        value,
        jbotci_output::budget::ByteLimit::new(policy.text_bytes).expect("validated text limit"),
        limit,
    ) {
        Ok(bytes) => Response::builder()
            .status(status)
            .header("content-type", "application/json")
            .body(owned_body(bytes, context))
            .expect("JSON response"),
        Err(error) => {
            let code = if error.io_error_kind() == Some(std::io::ErrorKind::FileTooLarge) {
                ServerErrorCode::OutputLimit
            } else {
                eprintln!("JSON response encoding failed: {error}");
                ServerErrorCode::Failed
            };
            response_for_code(code, context)
        }
    }
}

impl AdmittedWorkContext {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) async fn run_keeping<T, F>(
        &self,
        keepalive: Option<crate::work::WorkKeepalive>,
        job: F,
    ) -> Result<T, WorkError>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        self.run(move || {
            let _keepalive = keepalive;
            job()
        })
        .await
    }
}

/// Stable reasons for operation and output budget failures.
#[requires(true)]
#[ensures(true)]
pub(crate) fn operation_error_code(error: &anyhow::Error) -> Option<ServerErrorCode> {
    for cause in error.chain() {
        if cause
            .downcast_ref::<jbotci_output::budget::OutputLimitError>()
            .is_some()
            || cause
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::FileTooLarge)
            || cause
                .downcast_ref::<serde_json::Error>()
                .is_some_and(|error| {
                    error.io_error_kind() == Some(std::io::ErrorKind::FileTooLarge)
                })
            || cause
                .downcast_ref::<jbotci_output::OutputError>()
                .is_some_and(|error| matches!(error, jbotci_output::OutputError::Limit { .. }))
            || cause
                .downcast_ref::<jbotci_cll::CllError>()
                .is_some_and(|error| matches!(error, jbotci_cll::CllError::OutputLimit { .. }))
            || cause
                .downcast_ref::<jbotci_web_core::WebComputeError>()
                .is_some_and(|error| error.is_output_limit())
            || cause
                .downcast_ref::<jbotci_gentufa::GentufaExportError>()
                .is_some_and(|error| matches!(error, jbotci_gentufa::GentufaExportError::Limit(_)))
        {
            return Some(ServerErrorCode::OutputLimit);
        }
        if cause
            .downcast_ref::<crate::policy::RequestLimitError>()
            .is_some_and(|error| matches!(error, crate::policy::RequestLimitError::CompoundParts))
            || cause
                .downcast_ref::<jbotci_search::vlacku::DictionaryCardLimitError>()
                .is_some()
            || cause
                .downcast_ref::<jbotci_jvozba::JvozbaError>()
                .is_some_and(|error| {
                    matches!(error, jbotci_jvozba::JvozbaError::TooMuchWork { .. })
                })
        {
            return Some(ServerErrorCode::OperationLimit);
        }
    }
    None
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(crate) fn operation_error_message(code: ServerErrorCode) -> &'static str {
    code.message()
}

#[requires(true)]
#[ensures(true)]
pub(crate) async fn operation_error_response(
    error: anyhow::Error,
    context: AdmittedWorkContext,
) -> axum::response::Response {
    if let Some(code) = operation_error_code(&error) {
        return response_for_code(code, Some(context));
    }
    let limit = jbotci_output::budget::ByteLimit::new(
        context
            .execution
            .policy
            .text_bytes
            .min(context.execution.policy.response_bytes),
    )
    .expect("positive text and response limits");
    match context
        .run(move || jbotci_output::budget::bounded_format(format_args!("{error}"), limit))
        .await
    {
        Ok(Ok(message)) => axum::http::Response::builder()
            .status(axum::http::StatusCode::BAD_REQUEST)
            .header("content-type", "text/plain; charset=utf-8")
            .body(owned_body(message.into_bytes(), Some(context)))
            .expect("bounded operation error response"),
        Ok(Err(_)) => response_for_code(ServerErrorCode::OutputLimit, Some(context)),
        Err(error) => response_for_code(ServerErrorCode::from(&error), Some(context)),
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) async fn admitted_json_response<T: serde::Serialize + Send + 'static>(
    status: axum::http::StatusCode,
    value: T,
    context: AdmittedWorkContext,
) -> axum::response::Response {
    let owner = context.clone();
    match context
        .run(move || json_response(status, &value, &owner.execution.policy, Some(owner.clone())))
        .await
    {
        Ok(response) => response,
        Err(error) => work_error_response(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::to_bytes, http::StatusCode};
    use bityzba::data;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn json_quota_errors_keep_the_canonical_output_code_without_an_inner_cause() {
        let error = serde_json::Error::io(std::io::Error::from(std::io::ErrorKind::FileTooLarge));
        let error = anyhow::Error::new(error).context("JSON encoding failed");
        assert_eq!(
            operation_error_code(&error),
            Some(ServerErrorCode::OutputLimit)
        );
    }

    #[tokio::test(start_paused = true)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_ready_normal_result_is_dropped_if_the_caller_resumes_after_its_deadline() {
        use std::{
            future::Future,
            sync::atomic::{AtomicBool, Ordering},
            task::Poll,
        };
        let execution = ServerExecution::new(ServerPolicy::default());
        let context = execution.admit(None).await.unwrap();
        let finished = Arc::new(AtomicBool::new(false));
        let worker_finished = Arc::clone(&finished);
        let (release, wait) = std::sync::mpsc::channel();
        let mut result = Box::pin(context.run(move || {
            wait.recv().unwrap();
            worker_finished.store(true, Ordering::Release);
            "late normal result"
        }));
        std::future::poll_fn(|cx| {
            assert!(result.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        release.send(()).unwrap();
        while !finished.load(Ordering::Acquire) {
            tokio::task::yield_now().await;
        }
        tokio::time::advance(execution.policy.deadline).await;
        assert!(matches!(result.await, Err(WorkError::TimedOut { .. })));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn json_responses_enforce_text_and_wire_boundaries_and_keep_the_lease() {
        let value = serde_json::json!({"text": "é\n"});
        let expected = serde_json::to_vec(&value).unwrap();
        let policy = ServerPolicy::default().with_data(data! {
            text_bytes: "é\n".len(), response_bytes: expected.len(),
        });
        let execution = ServerExecution::new(policy);
        let context = execution.admit(None).await.unwrap();
        let response = admitted_json_response(StatusCode::OK, value.clone(), context).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(execution.governor.snapshot().compute_active, 1);
        let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
        assert_eq!(bytes.as_ref(), expected);
        assert_eq!(execution.governor.snapshot().compute_active, 1);
        drop(bytes);
        assert_eq!(execution.governor.snapshot().compute_active, 0);
        for policy in [
            execution
                .policy
                .clone()
                .with_data(data! { text_bytes: "é\n".len() - 1 }),
            execution
                .policy
                .clone()
                .with_data(data! { response_bytes: expected.len() - 1 }),
        ] {
            let response = json_response(StatusCode::OK, &value, &policy, None);
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(
                response.headers()["x-jbotci-error"],
                "output_limit_exceeded"
            );
        }
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn dropping_an_admitted_caller_keeps_its_running_worker_in_the_slot() {
        let execution = ServerExecution::new(ServerPolicy::default());
        let context = execution.admit(None).await.unwrap();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (finished_tx, finished_rx) = tokio::sync::oneshot::channel();
        let caller = tokio::spawn(async move {
            context
                .run(move || {
                    started_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    let _ = finished_tx.send(());
                })
                .await
        });
        started_rx.await.unwrap();
        caller.abort();
        let _ = caller.await;
        assert_eq!(execution.governor.snapshot().compute_active, 1);
        release_tx.send(()).unwrap();
        finished_rx.await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while execution.governor.snapshot().compute_active != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn admitted_panics_release_capacity_and_expired_work_never_runs() {
        let execution = ServerExecution::new(ServerPolicy::default());
        let context = execution.admit(None).await.unwrap();
        assert!(matches!(
            context.run(|| panic!("local test panic")).await,
            Err(WorkError::Panicked { .. })
        ));
        assert_eq!(execution.governor.snapshot().compute_active, 1);
        let context = context.with_data(data! { deadline: Instant::now() });
        assert!(matches!(
            context.run(|| panic!("expired work ran")).await,
            Err(WorkError::TimedOut { .. })
        ));
        drop(context);
        assert_eq!(execution.governor.snapshot().compute_active, 0);
    }
    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn operation_errors_use_both_text_and_wire_limits() {
        for (text_bytes, response_bytes, fits) in [(2, 2, true), (1, 2, false), (2, 1, false)] {
            let policy = ServerPolicy::default().with_data(data! {
                text_bytes: text_bytes, response_bytes: response_bytes,
            });
            let execution = ServerExecution::new(policy);
            let response = operation_error_response(
                anyhow::anyhow!("é"),
                execution.admit(None).await.unwrap(),
            )
            .await;
            if fits {
                assert_eq!(response.status(), StatusCode::BAD_REQUEST);
                assert_eq!(to_bytes(response.into_body(), 64).await.unwrap(), "é");
            } else {
                assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
                assert_eq!(
                    response.headers()["x-jbotci-error"],
                    "output_limit_exceeded"
                );
                drop(response);
            }
            assert_eq!(execution.governor.snapshot().compute_active, 0);
        }
    }
}
