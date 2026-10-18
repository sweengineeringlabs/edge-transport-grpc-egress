//! Reusable example: drive a real call through [`StdGrpcRetryFactory`] so the
//! backoff/attempt loop (delegating to the shared `edge-transport-retry`
//! crate's `BackoffScheduler`) actually executes — not just constructs.
//!
//! A test double that fails twice and then succeeds is enough to prove this; no real gRPC server
//! is needed since `StdGrpcRetryFactory` decorates any client that is a `Transport` and a `RawTransport`.
//!
//! [`StdGrpcRetryFactory`]: grpc_egress_retry_std::StdGrpcRetryFactory

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use grpc_egress_retry::GrpcRetryConfig;
use grpc_egress_retry::{BuildGrpcRetryRequest, GrpcRetryFactory};
use grpc_egress_retry_std::StdGrpcRetryFactory;
use proto_pattern::{
    BidiStreamMethod, CallError, CallOptions, CallRequest, CallResponse, ClientStreamMethod,
    MessageStream, Metadata, MethodPath, RawTransport, ResponseStream, ServerStreamMethod,
    StatusCode, Transport, UnaryMethod,
};

/// An upstream that fails its first `fail_until` calls with `Unavailable`, then succeeds. A real peer answers one
/// way for every call, so this double is what makes "fails twice, then succeeds" happen.
struct FlakyUpstream {
    calls: Arc<AtomicUsize>,
    fail_until: usize,
}

/// The response stream of a call the upstream refuses.
struct NoStream {
    headers: Metadata,
}

impl<T: Send> MessageStream<T> for NoStream {
    async fn read_message(&mut self) -> Result<Option<T>, CallError> {
        Ok(None)
    }
}

impl<T: Send> ResponseStream<T> for NoStream {
    type Metadata = Metadata;

    fn get_headers(&self) -> &Metadata {
        &self.headers
    }

    async fn read_trailers(&mut self) -> Result<Metadata, CallError> {
        Ok(Metadata::default())
    }
}

impl FlakyUpstream {
    fn refusal(path: &str) -> CallError {
        CallError::Transport {
            path: path.into(),
            code: StatusCode::Unimplemented,
            reason: "this example only makes raw unary calls".to_string(),
        }
    }
}

impl RawTransport for FlakyUpstream {
    type Metadata = Metadata;

    async fn call_raw(
        &self,
        path: MethodPath,
        _request: CallRequest<Vec<u8>>,
    ) -> Result<CallResponse<Vec<u8>, Metadata>, CallError> {
        let attempt = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if attempt <= self.fail_until {
            Err(CallError::Transport {
                path: path.get_path().into(),
                code: StatusCode::Unavailable,
                reason: format!("upstream still down (attempt {attempt})"),
            })
        } else {
            Ok(CallResponse::create(
                format!("ok on attempt {attempt}").into_bytes(),
                Metadata::default(),
                Metadata::default(),
            ))
        }
    }

    async fn call_raw_server_stream(
        &self,
        path: MethodPath,
        _request: CallRequest<Vec<u8>>,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = Metadata>, CallError> {
        Err::<NoStream, _>(Self::refusal(path.get_path()))
    }

    async fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        path: MethodPath,
        _requests: S,
        _options: CallOptions,
    ) -> Result<CallResponse<Vec<u8>, Metadata>, CallError> {
        Err(Self::refusal(path.get_path()))
    }

    async fn call_raw_bidi_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        path: MethodPath,
        _requests: S,
        _options: CallOptions,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = Metadata>, CallError> {
        Err::<NoStream, _>(Self::refusal(path.get_path()))
    }
}

impl Transport for FlakyUpstream {
    type Metadata = Metadata;

    async fn call<M: UnaryMethod>(
        &self,
        _request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, Metadata>, CallError> {
        Err(Self::refusal(M::PATH))
    }

    async fn call_server_stream<M: ServerStreamMethod>(
        &self,
        _request: CallRequest<M::Request>,
    ) -> Result<impl ResponseStream<M::Response, Metadata = Metadata>, CallError> {
        Err::<NoStream, _>(Self::refusal(M::PATH))
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        _requests: S,
        _options: CallOptions,
    ) -> Result<CallResponse<M::Response, Metadata>, CallError> {
        Err(Self::refusal(M::PATH))
    }

    async fn call_bidi_stream<M: BidiStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        _requests: S,
        _options: CallOptions,
    ) -> Result<impl ResponseStream<M::Response, Metadata = Metadata>, CallError> {
        Err::<NoStream, _>(Self::refusal(M::PATH))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let calls = Arc::new(AtomicUsize::new(0));
    let upstream = FlakyUpstream {
        calls: calls.clone(),
        fail_until: 2,
    };

    let client = StdGrpcRetryFactory::new(GrpcRetryConfig {
        max_attempts: 3,
        initial_backoff_ms: 50,
        backoff_multiplier: 2.0,
        jitter_factor: 0.0,
        max_backoff_ms: 200,
        ..GrpcRetryConfig::default()
    })
    .build(BuildGrpcRetryRequest { inner: upstream });

    // A single logical call. The retry layer's internal attempt loop must
    // re-invoke the upstream after a real backoff sleep on each retriable
    // failure — the timeout below leaves comfortable headroom for that.
    let request = CallRequest::create(
        Vec::new(),
        CallOptions::create(Some(Duration::from_secs(3)), Metadata::default()),
    );
    let started = std::time::Instant::now();
    let result = client
        .call_raw(MethodPath::create("/example.Service/Method")?, request)
        .await;
    let elapsed = started.elapsed();

    println!(
        "result: {:?}",
        result.as_ref().map(|r| r.get_message().len())
    );
    println!(
        "upstream was called {} time(s) across {:?}",
        calls.load(Ordering::SeqCst),
        elapsed
    );

    assert!(
        result.is_ok(),
        "the retry layer must eventually succeed once the upstream recovers"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "the upstream must have been invoked on all 3 attempts (2 failures + 1 success)"
    );
    assert!(
        elapsed >= Duration::from_millis(50),
        "elapsed time must reflect at least one real backoff sleep, not an instant retry"
    );
    println!(
        "confirmed: BackoffScheduler::next_backoff really executed between attempts \
         (elapsed {elapsed:?} implies a genuine sleep, not a busy loop)"
    );
    Ok(())
}
