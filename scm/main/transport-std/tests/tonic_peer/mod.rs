//! A real `tonic` server for the tests of what a client sends and reads back: it records the metadata of every
//! request it receives, answers with headers and trailers that hold several values under one key and binary
//! entries, can fail with a status carrying such trailers, can hold a call open and notice when the client
//! cancels it, and can stream messages and then stay silent.
//!
//! The server is `tonic`'s own, so the client is checked against an independent implementation of the wire format.
//! Methods (all under `/meta.Peer/`): `Reflect` (unary), `Fail` (unary, fails with the code of the request's
//! `fail_with`), `Cancelled` (unary, fails `CANCELLED`), `Hold` (unary, answers after the request's `sleep_ms`),
//! `Tick` (server streaming: 1, 2, then silence).

use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll};
use std::time::Duration;

use futures::{Stream, StreamExt as _};
use http::{Request as HttpRequest, Response as HttpResponse};
use proto_test_peer::{Count, Say};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::body::Body;
use tonic::codegen::{BoxFuture, Service};
use tonic::metadata::{MetadataMap, MetadataValue};
use tonic::server::{Grpc, NamedService, ServerStreamingService, UnaryService};
use tonic::transport::Server;
use tonic::{Code, Request, Response, Status};
use tonic_prost::ProstCodec;

/// The text and binary metadata one request carried.
#[derive(Debug, Clone, Default)]
pub struct SeenRequest {
    /// Every text entry as `(key, value)`, in the order received.
    pub text: Vec<(String, String)>,
    /// Every binary entry as `(key, bytes)`, in the order received.
    pub binary: Vec<(String, Vec<u8>)>,
}

impl SeenRequest {
    /// The values the request carried under `key`.
    pub fn get_values(&self, key: &str) -> Vec<&str> {
        self.text
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    /// The byte values the request carried under the binary `key`.
    pub fn get_binary_values(&self, key: &str) -> Vec<&[u8]> {
        self.binary
            .iter()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.as_slice())
            .collect()
    }
}

/// What the server has seen.
#[derive(Debug, Default)]
struct State {
    requests: Mutex<Vec<SeenRequest>>,
    holding: AtomicUsize,
    released: AtomicUsize,
}

/// Counts a held call in while it runs and out when the server drops its future (the call ended, or the client
/// cancelled it).
struct HoldGuard {
    state: Arc<State>,
}

impl HoldGuard {
    fn enter(state: &Arc<State>) -> Self {
        state.holding.fetch_add(1, Ordering::SeqCst);
        Self {
            state: Arc::clone(state),
        }
    }
}

impl Drop for HoldGuard {
    fn drop(&mut self) {
        self.state.holding.fetch_sub(1, Ordering::SeqCst);
        self.state.released.fetch_add(1, Ordering::SeqCst);
    }
}

/// The running server.
pub struct TonicPeer {
    address: SocketAddr,
    state: Arc<State>,
    stop: Option<oneshot::Sender<()>>,
}

impl TonicPeer {
    /// Start the server on a port the operating system assigns.
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|e| panic!("cannot bind a port: {e}"));
        let address = listener
            .local_addr()
            .unwrap_or_else(|e| panic!("no local address: {e}"));
        let state = Arc::new(State::default());
        let service = PeerService {
            state: Arc::clone(&state),
        };
        let (stop, stopped) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = Server::builder()
                .add_service(service)
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                    let _ = stopped.await;
                })
                .await;
        });
        Self {
            address,
            state,
            stop: Some(stop),
        }
    }

    /// The `http://` endpoint of the server.
    pub fn get_endpoint(&self) -> String {
        format!("http://{}", self.address)
    }

    /// The metadata of every request received, in order.
    pub fn get_seen_requests(&self) -> Vec<SeenRequest> {
        self.state
            .requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// The number of `Hold` calls whose handler is running now.
    pub fn get_holding(&self) -> usize {
        self.state.holding.load(Ordering::SeqCst)
    }

    /// The number of `Hold` calls whose handler has ended, because it finished or because the client cancelled.
    pub fn get_released(&self) -> usize {
        self.state.released.load(Ordering::SeqCst)
    }

    /// Wait until `condition` holds, for at most `limit`; `false` when it did not.
    pub async fn wait_until(&self, limit: Duration, condition: impl Fn(&Self) -> bool) -> bool {
        let deadline = tokio::time::Instant::now() + limit;
        while tokio::time::Instant::now() < deadline {
            if condition(self) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        condition(self)
    }
}

impl Drop for TonicPeer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

#[derive(Clone)]
struct PeerService {
    state: Arc<State>,
}

impl NamedService for PeerService {
    const NAME: &'static str = "meta.Peer";
}

impl Service<HttpRequest<Body>> for PeerService {
    type Response = HttpResponse<Body>;
    type Error = std::convert::Infallible;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: HttpRequest<Body>) -> Self::Future {
        let state = Arc::clone(&self.state);
        let path = req.uri().path().to_string();
        Box::pin(async move {
            let unary = || Grpc::new(ProstCodec::<Say, Say>::default());
            let response = match path.as_str() {
                "/meta.Peer/Reflect" => {
                    unary()
                        .unary(Handler::create(state, Method::Reflect), req)
                        .await
                }
                "/meta.Peer/Fail" => {
                    unary()
                        .unary(Handler::create(state, Method::Fail), req)
                        .await
                }
                "/meta.Peer/Cancelled" => {
                    unary()
                        .unary(Handler::create(state, Method::Cancelled), req)
                        .await
                }
                "/meta.Peer/Hold" => {
                    unary()
                        .unary(Handler::create(state, Method::Hold), req)
                        .await
                }
                "/meta.Peer/Tick" => {
                    Grpc::new(ProstCodec::<Count, Say>::default())
                        .server_streaming(TickHandler { state }, req)
                        .await
                }
                _ => Status::unimplemented("no such method").into_http(),
            };
            Ok(response)
        })
    }
}

#[derive(Clone, Copy)]
enum Method {
    Reflect,
    Fail,
    Cancelled,
    Hold,
}

struct Handler {
    state: Arc<State>,
    method: Method,
}

impl Handler {
    fn create(state: Arc<State>, method: Method) -> Self {
        Self { state, method }
    }

    /// Two values under one key and a binary entry that is not UTF-8 and needs padding: what a map of one text
    /// value per key cannot carry.
    fn fill(metadata: &mut MetadataMap) {
        metadata.append("x-multi", MetadataValue::from_static("first"));
        metadata.append("x-multi", MetadataValue::from_static("second"));
        metadata.append_bin(
            "x-blob-bin",
            MetadataValue::from_bytes(&[0xff, 0x00, 0xfe, 0x01]),
        );
        metadata.append_bin("x-blob-bin", MetadataValue::from_bytes(&[0x80]));
    }

    fn record(state: &State, metadata: &MetadataMap) {
        let mut seen = SeenRequest::default();
        for entry in metadata.iter() {
            match entry {
                tonic::metadata::KeyAndValueRef::Ascii(key, value) => {
                    if let Ok(text) = value.to_str() {
                        seen.text.push((key.as_str().to_string(), text.to_string()));
                    }
                }
                tonic::metadata::KeyAndValueRef::Binary(key, value) => {
                    if let Ok(bytes) = value.to_bytes() {
                        seen.binary.push((key.as_str().to_string(), bytes.to_vec()));
                    }
                }
            }
        }
        state
            .requests
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(seen);
    }
}

impl UnaryService<Say> for Handler {
    type Response = Say;
    type Future = BoxFuture<Response<Say>, Status>;

    fn call(&mut self, request: Request<Say>) -> Self::Future {
        let state = Arc::clone(&self.state);
        let method = self.method;
        Box::pin(async move {
            Self::record(&state, request.metadata());
            let say = request.into_inner();
            match method {
                Method::Reflect => {
                    let mut response = Response::new(say);
                    Self::fill(response.metadata_mut());
                    Ok(response)
                }
                Method::Fail => {
                    let mut trailers = MetadataMap::new();
                    Self::fill(&mut trailers);
                    Err(Status::with_details_and_metadata(
                        Code::from_i32(say.fail_with),
                        "the peer says no",
                        tonic::codegen::Bytes::from_static(&[9, 8, 7]),
                        trailers,
                    ))
                }
                Method::Cancelled => Err(Status::cancelled("the peer cancelled the call")),
                Method::Hold => {
                    let _guard = HoldGuard::enter(&state);
                    tokio::time::sleep(Duration::from_millis(u64::from(say.sleep_ms))).await;
                    Ok(Response::new(say))
                }
            }
        })
    }
}

struct TickHandler {
    state: Arc<State>,
}

type CountStream = Pin<Box<dyn Stream<Item = Result<Count, Status>> + Send>>;

impl ServerStreamingService<Say> for TickHandler {
    type Response = Count;
    type ResponseStream = CountStream;
    type Future = BoxFuture<Response<CountStream>, Status>;

    fn call(&mut self, request: Request<Say>) -> Self::Future {
        Handler::record(&self.state, request.metadata());
        Box::pin(async move {
            let first_two = futures::stream::iter([Ok(Count { value: 1 }), Ok(Count { value: 2 })]);
            let stream: CountStream = Box::pin(first_two.chain(futures::stream::pending()));
            Ok(Response::new(stream))
        })
    }
}
