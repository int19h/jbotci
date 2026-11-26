//! Admit each LSP request without a stop of the read loop.
//!
//! The main loop calls `poll_ready` before it reads the next message.
//! During that call, the main loop does not poll accepted requests.
//! `poll_ready` stays ready after the worker pool is full.
//! The request future waits for a worker permit.
//! The main loop can read, cancel, and write during that wait.

use std::collections::HashMap;
use std::future::Future;
use std::num::NonZeroUsize;
use std::ops::ControlFlow;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use async_lsp::lsp_types::notification::Notification;
use async_lsp::lsp_types::request::{self, Request};
use async_lsp::lsp_types::{CancelParams, NumberOrString};
use async_lsp::{
    AnyEvent, AnyNotification, AnyRequest, ErrorCode, LspService, ResponseError,
    Result as LspResult,
};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use tokio::sync::{Semaphore, oneshot};
use tower::Service;

const MIN_HEAVY_REQUESTS: usize = 2;

#[invariant(*heavy_limit >= MIN_HEAVY_REQUESTS && *heavy_limit <= Semaphore::MAX_PERMITS)]
#[invariant(heavy_permits.available_permits() <= *heavy_limit)]
pub(super) struct RequestAdmission<S> {
    service: Arc<Mutex<S>>,
    cancels: Mutex<HashMap<NumberOrString, oneshot::Sender<()>>>,
    heavy_permits: Arc<Semaphore>,
    heavy_limit: usize,
}

#[requires(true)]
#[ensures(ret >= MIN_HEAVY_REQUESTS)]
#[ensures(ret <= Semaphore::MAX_PERMITS)]
fn heavy_request_limit() -> usize {
    std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(MIN_HEAVY_REQUESTS)
        .max(MIN_HEAVY_REQUESTS)
}

#[requires(true)]
#[ensures(ret.heavy_limit == heavy_request_limit())]
pub(super) fn request_admission<S>(service: S) -> RequestAdmission<S> {
    request_admission_with_limit(service, heavy_request_limit())
}

#[requires(heavy_limit >= MIN_HEAVY_REQUESTS && heavy_limit <= Semaphore::MAX_PERMITS)]
#[ensures(ret.heavy_limit == heavy_limit)]
#[ensures(ret.heavy_permits.available_permits() == heavy_limit)]
fn request_admission_with_limit<S>(service: S, heavy_limit: usize) -> RequestAdmission<S> {
    new!(RequestAdmission {
        service: Arc::new(Mutex::new(service)),
        cancels: Mutex::new(HashMap::new()),
        heavy_permits: Arc::new(Semaphore::new(heavy_limit)),
        heavy_limit,
    })
}

#[requires(true)]
#[ensures(true)]
fn request_uses_analysis_worker(method: &str) -> bool {
    matches!(
        method,
        request::DocumentDiagnosticRequest::METHOD
            | request::HoverRequest::METHOD
            | request::SelectionRangeRequest::METHOD
            | request::FoldingRangeRequest::METHOD
            | request::Completion::METHOD
            | request::SemanticTokensFullRequest::METHOD
            | request::InlayHintRequest::METHOD
    )
}

#[requires(true)]
#[ensures(ret.code == ErrorCode::REQUEST_CANCELLED)]
#[ensures(ret.message == "Client cancelled the request")]
fn cancelled_response() -> ResponseError {
    ResponseError::new(ErrorCode::REQUEST_CANCELLED, "Client cancelled the request")
}

impl<S> RequestAdmission<S> {
    #[requires(true)]
    #[ensures(true)]
    fn register_cancel(&self, id: NumberOrString) -> oneshot::Receiver<()> {
        let (sender, receiver) = oneshot::channel();
        let mut cancels = self.cancels.lock().expect("LSP cancel map lock poisoned");
        cancels.retain(|_, sender| !sender.is_closed());
        cancels.insert(id, sender);
        receiver
    }

    #[requires(true)]
    #[ensures(true)]
    fn cancel_request(&self, id: &NumberOrString) {
        let mut cancels = self.cancels.lock().expect("LSP cancel map lock poisoned");
        if let Some(sender) = cancels.remove(id) {
            let _ = sender.send(());
        }
    }
}

impl<S> Service<AnyRequest> for RequestAdmission<S>
where
    S: Service<AnyRequest, Response = serde_json::Value, Error = ResponseError> + Send + 'static,
    S::Future: Send,
{
    type Response = serde_json::Value;
    type Error = ResponseError;
    type Future = Pin<Box<dyn Future<Output = Result<serde_json::Value, ResponseError>> + Send>>;

    #[requires(true)]
    #[ensures(matches!(ret, Poll::Ready(Ok(()))))]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), ResponseError>> {
        // A pending result here stops the next read and the next response write.
        Poll::Ready(Ok(()))
    }

    #[requires(true)]
    #[ensures(true)]
    fn call(&mut self, req: AnyRequest) -> <Self as Service<AnyRequest>>::Future {
        let heavy = request_uses_analysis_worker(&req.method);
        let cancel = self.register_cancel(req.id.clone());
        let permits = Arc::clone(&self.heavy_permits);
        let service = Arc::clone(&self.service);
        Box::pin(async move {
            // A dropped sender is not a client cancel. The work future must continue.
            let cancel_requested = async move {
                match cancel.await {
                    Ok(()) => (),
                    Err(_) => std::future::pending::<()>().await,
                }
            };
            tokio::select! {
                biased;
                () = cancel_requested => Err(cancelled_response()),
                result = run_admitted(heavy, permits, service, req) => result,
            }
        })
    }
}

#[requires(true)]
#[ensures(true)]
async fn run_admitted<S>(
    heavy: bool,
    permits: Arc<Semaphore>,
    service: Arc<Mutex<S>>,
    req: AnyRequest,
) -> Result<serde_json::Value, ResponseError>
where
    S: Service<AnyRequest, Response = serde_json::Value, Error = ResponseError> + Send,
    S::Future: Send,
{
    if heavy {
        let permit = permits
            .acquire_owned()
            .await
            .expect("LSP worker semaphore stays open");
        // The guard ends before the inner future waits.
        // A waiting analysis request does not block shutdown or the next notification.
        let fut = {
            let mut service = service.lock().expect("LSP request admission lock poisoned");
            service.call(req)
        };
        let result = fut.await;
        drop(permit);
        result
    } else {
        let fut = {
            let mut service = service.lock().expect("LSP request admission lock poisoned");
            service.call(req)
        };
        fut.await
    }
}

impl<S> LspService for RequestAdmission<S>
where
    S: Service<AnyRequest, Response = serde_json::Value, Error = ResponseError>
        + LspService
        + Send
        + 'static,
    S::Future: Send,
{
    #[requires(true)]
    #[ensures(true)]
    fn notify(&mut self, notif: AnyNotification) -> ControlFlow<LspResult<()>> {
        if notif.method == async_lsp::lsp_types::notification::Cancel::METHOD {
            if let Ok(params) = serde_json::from_value::<CancelParams>(notif.params) {
                self.cancel_request(&params.id);
            }
            return ControlFlow::Continue(());
        }
        self.service
            .lock()
            .expect("LSP request admission lock poisoned")
            .notify(notif)
    }

    #[requires(true)]
    #[ensures(true)]
    fn emit(&mut self, event: AnyEvent) -> ControlFlow<LspResult<()>> {
        self.service
            .lock()
            .expect("LSP request admission lock poisoned")
            .emit(event)
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::ops::ControlFlow;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::{Context, Poll, Waker};

    use async_lsp::lsp_types::NumberOrString;
    use async_lsp::lsp_types::request::{self, Request};
    use async_lsp::{
        AnyEvent, AnyNotification, AnyRequest, ErrorCode, LspService, ResponseError,
        Result as LspResult,
    };
    #[allow(unused_imports)]
    use bityzba::{ensures, invariant, new, requires};
    use serde_json::{Value, json};
    use tokio::sync::watch;
    use tower::Service;

    use super::request_admission_with_limit;

    #[invariant(started.load(Ordering::Relaxed) >= *started_floor)]
    struct PendingService {
        started_floor: usize,
        started: Arc<AtomicUsize>,
        ready: watch::Receiver<bool>,
    }

    #[requires(true)]
    #[ensures(ret.started_floor == 0)]
    fn pending_service(started: Arc<AtomicUsize>, ready: watch::Receiver<bool>) -> PendingService {
        new!(PendingService {
            started_floor: 0,
            started,
            ready,
        })
    }

    impl Service<AnyRequest> for PendingService {
        type Response = Value;
        type Error = ResponseError;
        type Future = Pin<Box<dyn Future<Output = Result<Value, ResponseError>> + Send>>;

        #[requires(true)]
        #[ensures(matches!(ret, Poll::Ready(Ok(()))))]
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), ResponseError>> {
            Poll::Ready(Ok(()))
        }

        #[requires(true)]
        #[ensures(true)]
        fn call(&mut self, _req: AnyRequest) -> <Self as Service<AnyRequest>>::Future {
            self.started.fetch_add(1, Ordering::SeqCst);
            let mut ready = self.ready.clone();
            Box::pin(async move {
                loop {
                    if *ready.borrow() {
                        return Ok(Value::Null);
                    }
                    if ready.changed().await.is_err() {
                        return Ok(Value::Null);
                    }
                }
            })
        }
    }

    impl LspService for PendingService {
        #[requires(true)]
        #[ensures(matches!(ret, ControlFlow::Continue(())))]
        fn notify(&mut self, _notif: AnyNotification) -> ControlFlow<LspResult<()>> {
            ControlFlow::Continue(())
        }

        #[requires(true)]
        #[ensures(matches!(ret, ControlFlow::Continue(())))]
        fn emit(&mut self, _event: AnyEvent) -> ControlFlow<LspResult<()>> {
            ControlFlow::Continue(())
        }
    }

    #[requires(id > 0)]
    #[requires(!method.is_empty())]
    #[ensures(ret.id == NumberOrString::Number(id))]
    #[ensures(ret.method == method)]
    fn request(id: i32, method: &str) -> AnyRequest {
        serde_json::from_value(json!({
            "id": id,
            "method": method,
            "params": {}
        }))
        .expect("test request deserializes")
    }

    #[requires(id > 0)]
    #[ensures(ret.method == "$/cancelRequest")]
    fn cancel(id: i32) -> AnyNotification {
        serde_json::from_value(json!({
            "method": "$/cancelRequest",
            "params": { "id": id }
        }))
        .expect("test cancel deserializes")
    }

    #[requires(!method.is_empty())]
    #[ensures(ret.method == method)]
    fn notification(method: &str) -> AnyNotification {
        serde_json::from_value(json!({
            "method": method,
            "params": {}
        }))
        .expect("test notification deserializes")
    }

    #[tokio::test(flavor = "current_thread")]
    #[requires(true)]
    #[ensures(true)]
    async fn full_pool_still_reads_cancel_and_a_cheap_request() {
        let started = Arc::new(AtomicUsize::new(0));
        let (ready_tx, ready_rx) = watch::channel(false);
        let mut admission =
            request_admission_with_limit(pending_service(Arc::clone(&started), ready_rx), 2);
        let waker = Waker::noop();
        let mut cx = Context::from_waker(&waker);
        assert!(admission.poll_ready(&mut cx).is_ready());

        let mut first = admission.call(request(1, request::HoverRequest::METHOD));
        let mut second = admission.call(request(2, request::HoverRequest::METHOD));
        let mut third = admission.call(request(3, request::HoverRequest::METHOD));
        let mut shutdown = admission.call(request(4, request::Shutdown::METHOD));
        assert!(admission.poll_ready(&mut cx).is_ready());

        assert!(first.as_mut().poll(&mut cx).is_pending());
        assert!(second.as_mut().poll(&mut cx).is_pending());
        assert_eq!(started.load(Ordering::SeqCst), 2);
        assert!(admission.poll_ready(&mut cx).is_ready());
        assert!(matches!(
            admission.notify(notification("textDocument/didChange")),
            ControlFlow::Continue(())
        ));
        assert!(third.as_mut().poll(&mut cx).is_pending());
        assert_eq!(started.load(Ordering::SeqCst), 2);
        assert!(shutdown.as_mut().poll(&mut cx).is_pending());
        assert_eq!(started.load(Ordering::SeqCst), 3);

        assert!(matches!(
            admission.notify(cancel(1)),
            ControlFlow::Continue(())
        ));
        let cancelled = first.await.expect_err("cancelled request");
        assert_eq!(cancelled.code, ErrorCode::REQUEST_CANCELLED);
        assert_eq!(cancelled.message, "Client cancelled the request");

        assert!(third.as_mut().poll(&mut cx).is_pending());
        assert_eq!(started.load(Ordering::SeqCst), 4);
        ready_tx.send(true).expect("test watch stays open");
        assert!(second.await.expect("second request").is_null());
        assert!(third.await.expect("third request").is_null());
        assert!(shutdown.await.expect("shutdown request").is_null());
    }
}
