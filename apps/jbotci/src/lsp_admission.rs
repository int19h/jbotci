//! Cancel an LSP request without a stop of the read loop.
//!
//! The main loop calls `poll_ready` before it reads the next message.
//! This layer forwards that call.
//! It does not wait for a worker permit.
//! `call` starts the inner request at once.
//! The request future also watches `$/cancelRequest`.

use std::collections::HashMap;
use std::future::Future;
use std::ops::ControlFlow;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use async_lsp::lsp_types::notification::Notification;
use async_lsp::lsp_types::{CancelParams, NumberOrString};
use async_lsp::{
    AnyEvent, AnyNotification, AnyRequest, ErrorCode, LspService, ResponseError,
    Result as LspResult,
};
#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use tokio::sync::oneshot;
use tower::Service;

/// One cancel slot for one request id.
///
/// The token identifies that request.
/// A newer request with the same id stores a new token.
#[invariant(*token > 0)]
struct CancelSlot {
    token: u64,
    sender: oneshot::Sender<()>,
}

/// The cancel tokens and the live cancel senders.
///
/// The next token stays above zero.
/// Every stored token is below the next token.
/// The counter does not wrap.
#[invariant(*next_token > 0)]
#[invariant(entries.values().all(|slot| slot.token < *next_token))]
struct CancelBook {
    next_token: u64,
    entries: HashMap<NumberOrString, CancelSlot>,
}

/// Removes one cancel map entry when dropped.
///
/// Any book, id, and token can be stored.
#[invariant(true)]
struct CancelRemoval {
    book: Arc<Mutex<CancelBook>>,
    id: NumberOrString,
    token: u64,
}

impl Drop for CancelRemoval {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        let mut book = self.book.lock().expect("LSP cancel map lock poisoned");
        // Drop removes the entry for this token.
        // A newer entry for the same id stays in the map.
        CancelBook::edit(&mut *book, |data| {
            if data
                .entries
                .get(&self.id)
                .is_some_and(|slot| slot.token == self.token)
            {
                data.entries.remove(&self.id);
            }
        });
    }
}

impl CancelBook {
    #[requires(true)]
    #[ensures(ret.next_token == 1)]
    #[ensures(ret.entries.is_empty())]
    fn empty() -> Self {
        new!(CancelBook {
            next_token: 1,
            entries: HashMap::new(),
        })
    }

    /// Replace the book, apply `edit`, and check the token relation again.
    #[requires(true)]
    #[ensures(true)]
    fn edit<T>(book: &mut Self, edit: impl FnOnce(&mut CancelBookData) -> T) -> T {
        let mut data = std::mem::replace(book, Self::empty()).into_data();
        let result = edit(&mut data);
        *book = Self::from_data(data);
        result
    }
}

/// Removes one cancel entry on drop.
///
/// The token stays above zero.
/// The removal record uses that same token.
#[invariant(*token > 0 && removal.token == *token)]
struct CancelGuard {
    token: u64,
    removal: CancelRemoval,
}

/// Forwards each request and keeps the cancel book.
///
/// `CancelBook` checks the token. This layer adds no second constraint.
#[invariant(true)]
pub(super) struct RequestAdmission<S> {
    service: S,
    cancels: Arc<Mutex<CancelBook>>,
}

#[requires(true)]
#[ensures(true)]
pub(super) fn request_admission<S>(service: S) -> RequestAdmission<S> {
    RequestAdmission {
        service,
        cancels: Arc::new(Mutex::new(CancelBook::empty())),
    }
}

#[requires(true)]
#[ensures(ret.code == ErrorCode::REQUEST_CANCELLED)]
#[ensures(ret.message == "Client cancelled the request")]
fn cancelled_response() -> ResponseError {
    ResponseError::new(ErrorCode::REQUEST_CANCELLED, "Client cancelled the request")
}

#[requires(true)]
#[ensures(ret.code == ErrorCode::INTERNAL_ERROR)]
#[ensures(ret.message == CANCEL_TOKENS_EXHAUSTED_MESSAGE)]
fn cancel_tokens_exhausted_response() -> ResponseError {
    ResponseError::new(ErrorCode::INTERNAL_ERROR, CANCEL_TOKENS_EXHAUSTED_MESSAGE)
}

impl<S> RequestAdmission<S> {
    /// Register one cancel token for `id`.
    ///
    /// `None` means the counter is exhausted.
    /// The book does not change in that case.
    #[requires(true)]
    #[ensures(true)]
    fn register_cancel(&self, id: NumberOrString) -> Option<(oneshot::Receiver<()>, CancelGuard)> {
        let mut book = self.cancels.lock().expect("LSP cancel map lock poisoned");
        let token = book.next_token;
        let next_token = token.checked_add(1)?;
        let (sender, receiver) = oneshot::channel();
        CancelBook::edit(&mut *book, |data| {
            data.next_token = next_token;
            data.entries
                .insert(id.clone(), new!(CancelSlot { token, sender }));
        });
        drop(book);
        Some((
            receiver,
            new!(CancelGuard {
                token,
                removal: CancelRemoval {
                    book: Arc::clone(&self.cancels),
                    id,
                    token,
                },
            }),
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    fn cancel_request(&self, id: &NumberOrString) {
        let mut book = self.cancels.lock().expect("LSP cancel map lock poisoned");
        CancelBook::edit(&mut *book, |data| {
            if let Some(slot) = data.entries.remove(id) {
                let _ = slot.into_data().sender.send(());
            }
        });
    }
}

const CANCEL_TOKENS_EXHAUSTED_MESSAGE: &str = "cancel token space is exhausted";

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
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ResponseError>> {
        // A pending result here stops the next read and the next response write.
        // This layer adds no wait. The inner router stays ready.
        self.service.poll_ready(cx)
    }

    #[requires(true)]
    #[ensures(true)]
    fn call(&mut self, req: AnyRequest) -> <Self as Service<AnyRequest>>::Future {
        let Some((cancel, guard)) = self.register_cancel(req.id.clone()) else {
            return Box::pin(async { Err(cancel_tokens_exhausted_response()) });
        };
        let inner = self.service.call(req);
        Box::pin(async move {
            // The guard stays in this future.
            // The future drop removes the cancel entry for this token.
            let _guard = guard;
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
                result = inner => result,
            }
        })
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
        self.service.notify(notif)
    }

    #[requires(true)]
    #[ensures(true)]
    fn emit(&mut self, event: AnyEvent) -> ControlFlow<LspResult<()>> {
        self.service.emit(event)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
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

    use super::request_admission;

    /// A test service that counts calls and then waits.
    ///
    /// The count and the watch have no value constraint.
    #[invariant(true)]
    struct PendingService {
        started: Arc<AtomicUsize>,
        ready: watch::Receiver<bool>,
    }

    #[requires(true)]
    #[ensures(true)]
    fn pending_service(started: Arc<AtomicUsize>, ready: watch::Receiver<bool>) -> PendingService {
        PendingService { started, ready }
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
    async fn a_pending_request_can_be_cancelled_while_the_reader_stays_ready() {
        let started = Arc::new(AtomicUsize::new(0));
        let (ready_tx, ready_rx) = watch::channel(false);
        let mut admission = request_admission(pending_service(Arc::clone(&started), ready_rx));
        let waker = Waker::noop();
        let mut cx = Context::from_waker(&waker);
        assert!(admission.poll_ready(&mut cx).is_ready());

        let mut first = admission.call(request(1, request::HoverRequest::METHOD));
        let mut second = admission.call(request(2, request::FoldingRangeRequest::METHOD));
        let mut third = admission.call(request(3, request::HoverRequest::METHOD));
        let mut shutdown = admission.call(request(4, request::Shutdown::METHOD));
        // Every call starts at once. None of them waits for a worker permit.
        assert_eq!(started.load(Ordering::SeqCst), 4);
        assert!(admission.poll_ready(&mut cx).is_ready());
        assert!(first.as_mut().poll(&mut cx).is_pending());
        assert!(second.as_mut().poll(&mut cx).is_pending());
        assert!(third.as_mut().poll(&mut cx).is_pending());
        assert!(shutdown.as_mut().poll(&mut cx).is_pending());
        assert!(matches!(
            admission.notify(notification("textDocument/didChange")),
            ControlFlow::Continue(())
        ));
        assert!(admission.poll_ready(&mut cx).is_ready());

        assert!(matches!(
            admission.notify(cancel(1)),
            ControlFlow::Continue(())
        ));
        let cancelled = first.await.expect_err("cancelled request");
        assert_eq!(cancelled.code, ErrorCode::REQUEST_CANCELLED);
        assert_eq!(cancelled.message, "Client cancelled the request");

        ready_tx.send(true).expect("test watch stays open");
        assert!(second.await.expect("second request").is_null());
        assert!(third.await.expect("third request").is_null());
        assert!(shutdown.await.expect("shutdown request").is_null());
    }

    #[tokio::test(flavor = "current_thread")]
    #[requires(true)]
    #[ensures(true)]
    async fn dropped_request_removes_only_its_cancel_entry() {
        let started = Arc::new(AtomicUsize::new(0));
        let (_ready_tx, ready_rx) = watch::channel(false);
        let mut admission = request_admission(pending_service(Arc::clone(&started), ready_rx));
        let first = admission.call(request(7, request::HoverRequest::METHOD));
        assert_eq!(cancel_token(&admission, 7), Some(1));
        let second = admission.call(request(7, request::HoverRequest::METHOD));
        assert_eq!(cancel_token(&admission, 7), Some(2));
        drop(first);
        assert_eq!(cancel_token(&admission, 7), Some(2));
        drop(second);
        assert_eq!(cancel_token(&admission, 7), None);
    }

    #[tokio::test(flavor = "current_thread")]
    #[requires(true)]
    #[ensures(true)]
    async fn exhausted_cancel_tokens_leave_the_live_request_in_place() {
        let started = Arc::new(AtomicUsize::new(0));
        let (_ready_tx, ready_rx) = watch::channel(false);
        let mut admission = request_admission(pending_service(Arc::clone(&started), ready_rx));
        set_next_cancel_token(&admission, u64::MAX - 1);

        let first = admission.call(request(7, request::HoverRequest::METHOD));
        assert_eq!(cancel_token(&admission, 7), Some(u64::MAX - 1));
        assert_eq!(started.load(Ordering::SeqCst), 1);

        let same_id = admission.call(request(7, request::HoverRequest::METHOD));
        let same_id = same_id.await.expect_err("the counter is exhausted");
        assert_eq!(same_id.code, ErrorCode::INTERNAL_ERROR);
        assert_eq!(same_id.message, "cancel token space is exhausted");
        assert_eq!(cancel_token(&admission, 7), Some(u64::MAX - 1));

        let other = admission.call(request(8, request::HoverRequest::METHOD));
        let other = other.await.expect_err("the counter is exhausted");
        assert_eq!(other.code, ErrorCode::INTERNAL_ERROR);
        assert_eq!(other.message, "cancel token space is exhausted");
        assert_eq!(started.load(Ordering::SeqCst), 1);
        assert_eq!(cancel_token(&admission, 7), Some(u64::MAX - 1));
        assert_eq!(cancel_token(&admission, 8), None);

        drop(first);
        assert_eq!(cancel_token(&admission, 7), None);
        assert_eq!(cancel_token(&admission, 8), None);
    }

    #[requires(next > 0)]
    #[ensures(true)]
    fn set_next_cancel_token(admission: &super::RequestAdmission<PendingService>, next: u64) {
        let mut book = admission
            .cancels
            .lock()
            .expect("LSP cancel map lock poisoned");
        super::CancelBook::edit(&mut *book, |data| {
            assert!(data.entries.is_empty());
            data.next_token = next;
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cancel_book_rejects_a_token_that_is_not_below_the_next_token() {
        let (low_sender, _low_receiver) = tokio::sync::oneshot::channel();
        let mut entries = HashMap::new();
        entries.insert(
            NumberOrString::Number(1),
            new!(super::CancelSlot {
                token: 1,
                sender: low_sender,
            }),
        );
        let accepted = bityzba::try_new!(super::CancelBook {
            next_token: 2,
            entries,
        })
        .expect("a stored token below the next token is valid");
        assert_eq!(accepted.next_token, 2);

        let (high_sender, _high_receiver) = tokio::sync::oneshot::channel();
        let mut entries = HashMap::new();
        entries.insert(
            NumberOrString::Number(1),
            new!(super::CancelSlot {
                token: 2,
                sender: high_sender,
            }),
        );
        assert!(
            bityzba::try_new!(super::CancelBook {
                next_token: 2,
                entries,
            })
            .is_err(),
            "a stored token must stay below the next token"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn cancel_token(admission: &super::RequestAdmission<PendingService>, id: i32) -> Option<u64> {
        admission
            .cancels
            .lock()
            .expect("LSP cancel map lock poisoned")
            .entries
            .get(&NumberOrString::Number(id))
            .map(|slot| slot.token)
    }
}
