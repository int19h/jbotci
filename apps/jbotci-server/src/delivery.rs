//! Response deadlines remain active after Hyper takes ownership of a frame.
use axum::{
    body::{Body, Bytes, HttpBody},
    extract::{ConnectInfo, Extension},
    http::{Request, Response},
    middleware::Next,
    serve::{IncomingStream, Listener},
};
use bityzba::{invariant, requires};
use http_body_util::{BodyExt, LengthLimitError, Limited};
use std::{
    future::Future,
    io,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    time::{Instant, Sleep},
};

/// The absolute operation deadline created at the HTTP boundary.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct OperationDeadline(pub(crate) Instant);

#[invariant(true)]
#[derive(Debug, Clone)]
pub(crate) struct Connection(Arc<Mutex<DeliveryState>>);

#[invariant(true)]
#[derive(Debug)]
struct DeliveryState {
    deadlines: Vec<Arc<Instant>>,
    timer: Option<Pin<Box<Sleep>>>,
    waker: Option<Waker>,
}

impl Connection {
    #[requires(true)]
    #[ensures(true)]
    fn new() -> Self {
        Self(Arc::new(Mutex::new(DeliveryState {
            deadlines: Vec::new(),
            timer: None,
            waker: None,
        })))
    }
    #[requires(true)]
    #[ensures(true)]
    fn register(&self, deadline: Instant) -> Arc<Registration> {
        let deadline = Arc::new(deadline);
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        state.deadlines.push(Arc::clone(&deadline));
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
        Arc::new(Registration {
            connection: self.clone(),
            deadline,
        })
    }
    #[requires(true)]
    #[ensures(true)]
    fn poll_deadline(&self, cx: &mut Context<'_>) -> io::Result<()> {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        state.waker = Some(cx.waker().clone());
        let Some(deadline) = state.deadlines.iter().map(|value| **value).min() else {
            state.timer = None;
            return Ok(());
        };
        let timer = state
            .timer
            .get_or_insert_with(|| Box::pin(tokio::time::sleep_until(deadline)));
        if timer.deadline() != deadline {
            timer.as_mut().reset(deadline);
        }
        if timer.as_mut().poll(cx).is_ready() {
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "response delivery deadline expired",
            ))
        } else {
            Ok(())
        }
    }
}

#[invariant(true)]
struct Registration {
    connection: Connection,
    deadline: Arc<Instant>,
}
impl Drop for Registration {
    #[requires(true)]
    #[ensures(true)]
    fn drop(&mut self) {
        let mut state = self
            .connection
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state
            .deadlines
            .retain(|deadline| !Arc::ptr_eq(deadline, &self.deadline));
        if let Some(waker) = state.waker.take() {
            waker.wake();
        }
    }
}

#[invariant(true)]
pub(crate) struct DeadlineListener<L>(pub(crate) L);
#[invariant(true)]
pub(crate) struct DeadlineStream<S> {
    stream: S,
    connection: Connection,
}
impl<L: Listener> Listener for DeadlineListener<L> {
    type Io = DeadlineStream<L::Io>;
    type Addr = L::Addr;
    #[requires(true)]
    #[ensures(true)]
    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        let (stream, address) = Listener::accept(&mut self.0).await;
        (
            DeadlineStream {
                stream,
                connection: Connection::new(),
            },
            address,
        )
    }
    #[requires(true)]
    #[ensures(true)]
    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.0.local_addr()
    }
}
impl<L: Listener> axum::extract::connect_info::Connected<IncomingStream<'_, DeadlineListener<L>>>
    for Connection
{
    #[requires(true)]
    #[ensures(true)]
    fn connect_info(stream: IncomingStream<'_, DeadlineListener<L>>) -> Self {
        stream.io().connection.clone()
    }
}
impl<S: AsyncRead + Unpin> AsyncRead for DeadlineStream<S> {
    #[requires(true)]
    #[ensures(true)]
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.connection.poll_deadline(cx)?;
        Pin::new(&mut self.stream).poll_read(cx, buffer)
    }
}
impl<S: AsyncWrite + Unpin> AsyncWrite for DeadlineStream<S> {
    #[requires(true)]
    #[ensures(true)]
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.connection.poll_deadline(cx)?;
        Pin::new(&mut self.stream).poll_write(cx, bytes)
    }
    // Hyper keeps body owners in its queue when the transport supports vectored writes.
    // Its flattened path copies those bytes and releases their owners before delivery.
    #[requires(true)]
    #[ensures(true)]
    fn is_write_vectored(&self) -> bool {
        self.stream.is_write_vectored()
    }
    #[requires(true)]
    #[ensures(true)]
    fn poll_write_vectored(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffers: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.connection.poll_deadline(cx)?;
        Pin::new(&mut self.stream).poll_write_vectored(cx, buffers)
    }
    #[requires(true)]
    #[ensures(true)]
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.connection.poll_deadline(cx)?;
        Pin::new(&mut self.stream).poll_flush(cx)
    }
    #[requires(true)]
    #[ensures(true)]
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

#[invariant(true)]
struct DeliveredBytes {
    bytes: Bytes,
    _registration: Arc<Registration>,
}
impl AsRef<[u8]> for DeliveredBytes {
    #[requires(true)]
    #[ensures(ret.len() == self.bytes.len())]
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}
#[invariant(true)]
struct DeliveryBody {
    inner: Body,
    registration: Arc<Registration>,
}
impl HttpBody for DeliveryBody {
    type Data = Bytes;
    type Error = axum::Error;
    #[requires(true)]
    #[ensures(true)]
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Bytes>, axum::Error>>> {
        let registration = Arc::clone(&self.registration);
        Pin::new(&mut self.inner).poll_frame(cx).map(|frame| {
            frame.map(|frame| {
                frame.map(|frame| {
                    frame.map_data(|bytes| {
                        Bytes::from_owner(DeliveredBytes {
                            bytes,
                            _registration: registration,
                        })
                    })
                })
            })
        })
    }
    #[requires(true)]
    #[ensures(true)]
    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }
    #[requires(true)]
    #[ensures(true)]
    fn size_hint(&self) -> http_body::SizeHint {
        self.inner.size_hint()
    }
}

/// A connection deadline closes the transport and releases retained frame owners.
/// This also closes other streams on an HTTP/2 connection with an expired response.
#[requires(true)]
#[ensures(true)]
pub(crate) async fn enforce(
    Extension(state): Extension<Arc<crate::AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let connection = request
        .extensions()
        .get::<ConnectInfo<Connection>>()
        .map(|info| info.0.clone());
    let path = request.uri().path();
    let health = path == "/api/health"
        || crate::strip_base_path(path, &state.base_path).as_deref() == Some("/api/health");
    let bounded_api = crate::is_api_request_path(path, &state.base_path) && !health;
    let Some(deadline) = Instant::now().checked_add(state.tool_services.execution.policy.deadline)
    else {
        return crate::execution::work_error_response(crate::work::WorkError::Panicked {
            lane: crate::work::WorkLane::Compute,
        });
    };
    request.extensions_mut().insert(OperationDeadline(deadline));
    let response = next.run(request).await;
    let response = if bounded_api {
        limit_api_response(
            response,
            jbotci_output::budget::ByteLimit::new(
                state.tool_services.execution.policy.response_bytes,
            )
            .expect("validated response limit"),
            deadline,
        )
        .await
    } else {
        response
    };
    let (response, deadline) = response_delivery(
        response,
        deadline,
        state.tool_services.execution.policy.error_delivery,
    );
    let Some(connection) = connection else {
        return response;
    };
    let (parts, inner) = response.into_parts();
    Response::from_parts(
        parts,
        Body::new(DeliveryBody {
            inner,
            registration: connection.register(deadline),
        }),
    )
}

/// Only a coded refusal can use time after the operation deadline.
#[requires(true)]
#[ensures(true)]
fn response_delivery(
    mut response: Response<Body>,
    operation_deadline: Instant,
    error_delivery: std::time::Duration,
) -> (Response<Body>, Instant) {
    use crate::error::ServerErrorCode;
    let coded_refusal = matches!(
        response.extensions().get::<ServerErrorCode>(),
        Some(ServerErrorCode::Busy | ServerErrorCode::Deadline)
    );
    if !coded_refusal && Instant::now() >= operation_deadline {
        response = crate::execution::work_error_response(crate::work::WorkError::TimedOut {
            lane: crate::work::WorkLane::Compute,
        });
    }
    let coded_refusal = matches!(
        response.extensions().get::<ServerErrorCode>(),
        Some(ServerErrorCode::Busy | ServerErrorCode::Deadline)
    );
    let deadline = if coded_refusal {
        operation_deadline
            .checked_add(error_delivery)
            .unwrap_or(operation_deadline)
    } else {
        operation_deadline
    };
    (response, deadline)
}

/// Collect bounded frames before sending headers. Frame owners retain admission.
/// Framework rejections use this bound even when no operation encoder ran.
#[requires(true)]
#[ensures(true)]
async fn limit_api_response(
    response: Response<Body>,
    limit: jbotci_output::budget::ByteLimit,
    deadline: Instant,
) -> Response<Body> {
    use crate::{
        error::ServerErrorCode,
        work::{WorkError, WorkLane},
    };
    if response.extensions().get::<ServerErrorCode>().is_some() {
        return response;
    }
    if Instant::now() >= deadline {
        return crate::execution::work_error_response(WorkError::TimedOut {
            lane: WorkLane::Compute,
        });
    }
    let (parts, body) = response.into_parts();
    match tokio::time::timeout_at(deadline, Limited::new(body, limit.get()).collect()).await {
        Ok(Ok(body)) => Response::from_parts(parts, Body::new(body)),
        Ok(Err(error)) if error.is::<LengthLimitError>() => {
            crate::execution::work_error_response(WorkError::OutputLimit)
        }
        Ok(Err(error)) => {
            eprintln!("API response body failed: {error}");
            crate::execution::work_error_response(WorkError::Panicked {
                lane: WorkLane::Compute,
            })
        }
        Err(_) => crate::execution::work_error_response(WorkError::TimedOut {
            lane: WorkLane::Compute,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use tokio::net::{TcpListener, TcpStream};
    #[invariant(true)]
    struct BlockedWrite {
        stream: tokio::io::DuplexStream,
        buffered: Arc<tokio::sync::Notify>,
    }
    const OWNED_BODY_PROBE: &[u8] = b"retained-response-owner";
    impl AsyncRead for BlockedWrite {
        #[requires(true)]
        #[ensures(true)]
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buffer: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            Pin::new(&mut self.stream).poll_read(cx, buffer)
        }
    }
    impl AsyncWrite for BlockedWrite {
        #[requires(true)]
        #[ensures(true)]
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            bytes: &[u8],
        ) -> Poll<io::Result<usize>> {
            if bytes
                .windows(OWNED_BODY_PROBE.len())
                .any(|window| window == OWNED_BODY_PROBE)
            {
                self.buffered.notify_one();
            }
            Poll::Pending
        }
        #[requires(true)]
        #[ensures(true)]
        fn is_write_vectored(&self) -> bool {
            self.stream.is_write_vectored()
        }
        #[requires(true)]
        #[ensures(true)]
        fn poll_write_vectored(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buffers: &[io::IoSlice<'_>],
        ) -> Poll<io::Result<usize>> {
            if buffers.iter().any(|bytes| {
                bytes
                    .windows(OWNED_BODY_PROBE.len())
                    .any(|window| window == OWNED_BODY_PROBE)
            }) {
                self.buffered.notify_one();
            }
            Poll::Pending
        }
        #[requires(true)]
        #[ensures(true)]
        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Pending
        }
        #[requires(true)]
        #[ensures(true)]
        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
    #[invariant(true)]
    struct OneConnection(Option<BlockedWrite>);
    impl Listener for OneConnection {
        type Io = BlockedWrite;
        type Addr = SocketAddr;
        #[requires(true)]
        #[ensures(true)]
        async fn accept(&mut self) -> (Self::Io, Self::Addr) {
            match self.0.take() {
                Some(stream) => (stream, "127.0.0.1:0".parse().unwrap()),
                None => std::future::pending().await,
            }
        }
        #[requires(true)]
        #[ensures(true)]
        fn local_addr(&self) -> io::Result<Self::Addr> {
            Ok("127.0.0.1:0".parse().unwrap())
        }
    }

    #[tokio::test(start_paused = true)]
    #[requires(true)]
    #[ensures(true)]
    async fn hyper_retains_admission_while_socket_output_is_blocked() {
        use tokio::io::AsyncWriteExt;
        let execution = crate::execution::ServerExecution::new(crate::ServerPolicy::default());
        let context = execution.admit(None).await.unwrap();
        let response = Arc::new(Mutex::new(Some(context)));
        let app = axum::Router::new().route(
            "/",
            axum::routing::get(move |ConnectInfo(connection): ConnectInfo<Connection>| {
                let context = response.lock().unwrap().take().unwrap();
                async move {
                    let deadline = context.deadline;
                    Response::new(Body::new(DeliveryBody {
                        inner: crate::execution::owned_body(
                            OWNED_BODY_PROBE.to_vec(),
                            Some(context),
                        ),
                        registration: connection.register(deadline),
                    }))
                }
            }),
        );
        let buffered = Arc::new(tokio::sync::Notify::new());
        let (mut client, stream) = tokio::io::duplex(128);
        let stream = BlockedWrite {
            stream,
            buffered: Arc::clone(&buffered),
        };
        // Use the same deadline stream as the production listener, with deterministic backpressure.
        let listener = OneConnection(Some(stream));
        let server = tokio::spawn(async move {
            axum::serve(
                DeadlineListener(listener),
                app.into_make_service_with_connect_info::<Connection>(),
            )
            .await
            .unwrap();
        });
        client
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        buffered.notified().await;
        assert_eq!(execution.governor.snapshot().compute_active, 1);
        tokio::time::advance(execution.policy.deadline).await;
        for _ in 0..100 {
            if execution.governor.snapshot().compute_active == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(execution.governor.snapshot().compute_active, 0);
        server.abort();
    }

    #[requires(true)]
    #[ensures(true)]
    async fn local_client_response(late_normal: bool) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let policy = crate::ServerPolicy::default().with_data(bityzba::data! {
            deadline: std::time::Duration::from_millis(20),
            error_delivery: std::time::Duration::from_secs(1),
        });
        let state = Arc::new(crate::AppState::new(
            crate::ServerConfig::new(
                "127.0.0.1:0".parse().unwrap(),
                "/",
                std::path::PathBuf::new(),
            ),
            crate::production_page_meta_builder,
            policy,
        ));
        let app = axum::Router::new()
            .route(
                "/api/probe",
                axum::routing::get(
                    move |Extension(state): Extension<Arc<crate::AppState>>,
                          Extension(deadline): Extension<OperationDeadline>| async move {
                        if late_normal {
                            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
                            Response::new(Body::from("LATE NORMAL RESULT"))
                        } else {
                            let context = state
                                .tool_services
                                .execution
                                .admit(Some(deadline.0))
                                .await
                                .unwrap();
                            match context
                                .run(|| {
                                    std::thread::sleep(std::time::Duration::from_millis(60));
                                    "LATE NORMAL RESULT"
                                })
                                .await
                            {
                                Ok(text) => Response::new(Body::from(text)),
                                Err(error) => crate::execution::work_error_response(error),
                            }
                        }
                    },
                ),
            )
            .layer(axum::middleware::from_fn(enforce))
            .layer(Extension(state));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                DeadlineListener(listener),
                app.into_make_service_with_connect_info::<Connection>(),
            )
            .await
            .unwrap();
        });
        let mut client = TcpStream::connect(address).await.unwrap();
        client
            .write_all(b"GET /api/probe HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut bytes = Vec::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.read_to_end(&mut bytes),
        )
        .await
        .unwrap()
        .unwrap();
        server.abort();
        String::from_utf8(bytes).unwrap()
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn local_client_receives_the_coded_timeout_after_the_operation_deadline() {
        let response = local_client_response(false).await;
        assert!(response.starts_with("HTTP/1.1 504"), "{response}");
        assert!(response.contains("x-jbotci-error: work_deadline_exceeded"));
        assert!(response.contains(crate::error::ServerErrorCode::Deadline.message()));
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn error_delivery_window_never_delivers_a_late_normal_result() {
        let response = local_client_response(true).await;
        assert!(response.starts_with("HTTP/1.1 504"), "{response}");
        assert!(response.contains("x-jbotci-error: work_deadline_exceeded"));
        assert!(!response.contains("LATE NORMAL RESULT"));
    }

    #[tokio::test(start_paused = true)]
    #[requires(true)]
    #[ensures(true)]
    async fn refusal_window_is_anchored_and_does_not_extend_success_delivery() {
        let deadline = Instant::now() + std::time::Duration::from_secs(20);
        let window = std::time::Duration::from_secs(1);
        let (_, normal_deadline) =
            response_delivery(Response::new(Body::from("normal")), deadline, window);
        assert_eq!(normal_deadline, deadline);
        tokio::time::advance(std::time::Duration::from_secs(20)).await;
        let (_, refusal_deadline) = response_delivery(
            crate::execution::work_error_response(crate::work::WorkError::Overloaded {
                lane: crate::work::WorkLane::Compute,
            }),
            deadline,
            window,
        );
        assert_eq!(refusal_deadline, deadline + window);
        tokio::time::advance(window).await;
        let (_, expired_deadline) = response_delivery(
            crate::execution::work_error_response(crate::work::WorkError::TimedOut {
                lane: crate::work::WorkLane::Compute,
            }),
            deadline,
            window,
        );
        assert_eq!(expired_deadline, refusal_deadline);
        assert!(expired_deadline <= Instant::now());
    }

    #[tokio::test(start_paused = true)]
    #[requires(true)]
    #[ensures(true)]
    async fn yielded_frames_retain_the_deadline_until_transport_releases_them() {
        let connection = Connection::new();
        let registration = connection.register(Instant::now() + std::time::Duration::from_secs(20));
        let mut body = DeliveryBody {
            inner: Body::from("bounded response"),
            registration,
        };
        let frame = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
            .await
            .unwrap()
            .unwrap();
        drop(body);
        assert_eq!(connection.0.lock().unwrap().deadlines.len(), 1);
        let waker = Waker::noop();
        let mut cx = Context::from_waker(waker);
        assert!(connection.poll_deadline(&mut cx).is_ok());
        tokio::time::advance(std::time::Duration::from_secs(20)).await;
        assert_eq!(
            connection.poll_deadline(&mut cx).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        drop(frame);
        assert!(connection.0.lock().unwrap().deadlines.is_empty());
        assert!(connection.poll_deadline(&mut cx).is_ok());
    }

    #[tokio::test(start_paused = true)]
    #[requires(true)]
    #[ensures(true)]
    async fn an_expired_response_stops_socket_writes() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let connect = TcpStream::connect(listener.local_addr().unwrap());
        let accept = listener.accept();
        let (client, server) = tokio::join!(connect, accept);
        let _client = client.unwrap();
        let (stream, _) = server.unwrap();
        let connection = Connection::new();
        let _registration =
            connection.register(Instant::now() + std::time::Duration::from_secs(20));
        let mut stream = DeadlineStream { stream, connection };
        tokio::time::advance(std::time::Duration::from_secs(20)).await;
        let error = tokio::io::AsyncWriteExt::write_all(&mut stream, b"small local probe")
            .await
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    }
    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn api_body_limits_count_utf8_and_keep_the_reserved_error_complete() {
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        let response = limit_api_response(
            Response::new(Body::from("é\n")),
            jbotci_output::budget::ByteLimit::new(3).unwrap(),
            deadline,
        )
        .await;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            axum::body::to_bytes(response.into_body(), 64)
                .await
                .unwrap(),
            "é\n"
        );
        let response = limit_api_response(
            Response::new(Body::from("é\n")),
            jbotci_output::budget::ByteLimit::new(2).unwrap(),
            deadline,
        )
        .await;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            response.headers()["x-jbotci-error"],
            "output_limit_exceeded"
        );
        let response = limit_api_response(
            response,
            jbotci_output::budget::ByteLimit::new(1).unwrap(),
            deadline,
        )
        .await;
        assert_eq!(
            axum::body::to_bytes(response.into_body(), 4096)
                .await
                .unwrap(),
            crate::error::ServerErrorCode::OutputLimit.message()
        );
    }

    #[tokio::test]
    #[requires(true)]
    #[ensures(true)]
    async fn api_collection_preserves_the_admission_owner_across_several_frames() {
        let policy = crate::ServerPolicy::default().with_data(bityzba::data! { response_bytes: 4 });
        let execution = crate::execution::ServerExecution::new(policy);
        let context = execution.admit(None).await.unwrap();
        let deadline = context.deadline;
        let mut original = crate::execution::owned_body(b"body".to_vec(), Some(context));
        let mut bytes = std::future::poll_fn(|cx| Pin::new(&mut original).poll_frame(cx))
            .await
            .unwrap()
            .unwrap()
            .into_data()
            .unwrap();
        drop(original);
        let (sender, receiver) =
            futures_channel::mpsc::unbounded::<Result<Bytes, std::convert::Infallible>>();
        sender.unbounded_send(Ok(bytes.split_to(2))).unwrap();
        sender.unbounded_send(Ok(bytes)).unwrap();
        drop(sender);
        let response = limit_api_response(
            Response::new(Body::from_stream(receiver)),
            jbotci_output::budget::ByteLimit::new(4).unwrap(),
            deadline,
        )
        .await;
        assert_eq!(execution.governor.snapshot().compute_active, 1);
        let mut body = response.into_body();
        let frame = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
            .await
            .unwrap()
            .unwrap()
            .into_data()
            .unwrap();
        drop(body);
        assert_eq!(execution.governor.snapshot().compute_active, 1);
        drop(frame);
        assert_eq!(execution.governor.snapshot().compute_active, 0);
    }

    #[tokio::test(start_paused = true)]
    #[requires(true)]
    #[ensures(true)]
    async fn stalled_api_bodies_expire_before_headers_and_release_the_source() {
        let (sender, receiver) =
            futures_channel::mpsc::unbounded::<Result<Bytes, std::convert::Infallible>>();
        let response = limit_api_response(
            Response::new(Body::from_stream(receiver)),
            jbotci_output::budget::ByteLimit::new(128).unwrap(),
            Instant::now() + std::time::Duration::from_secs(1),
        )
        .await;
        assert_eq!(response.status(), axum::http::StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(
            response.headers()["x-jbotci-error"],
            "work_deadline_exceeded"
        );
        assert!(sender.is_closed());
    }
}
