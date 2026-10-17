//! Reusable example: wrap an already-built client in retry and circuit breaker with
//! [`StdGrpcResilientFactory`].
//!
//! The decorator never builds the client: the composition root does that and applies the decorator
//! on top. A test double stands in for the client, so no server is needed. The double
//! fails its first two calls and then succeeds; the retry layer inside the decorator must call it
//! again after a real backoff until it does.
//!
//! [`StdGrpcResilientFactory`]: grpc_egress_resilient_std::StdGrpcResilientFactory

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use grpc_egress_resilient::ResilienceConfig;
use grpc_egress_resilient::{BuildGrpcResilientRequest, GrpcResilientFactory};
use grpc_egress_resilient_std::StdGrpcResilientFactory;
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

    let config = ResilienceConfig {
        max_attempts: 3,
        initial_backoff_ms: 20,
        backoff_multiplier: 2.0,
        jitter_factor: 0.0,
        max_backoff_ms: 100,
        rate_limit_max_attempts: 1,
        rate_limit_initial_backoff_ms: 100,
        rate_limit_max_backoff_ms: 100,
        failure_threshold: 5,
        cool_down_seconds: 1,
        half_open_probe_count: 1,
    };
    let client = StdGrpcResilientFactory::new(config)
        .build(BuildGrpcResilientRequest { inner: upstream })?;

    let request = CallRequest::create(
        Vec::new(),
        CallOptions::create(Some(Duration::from_secs(3)), Metadata::default()),
    );
    let started = std::time::Instant::now();
    let response = client
        .call_raw(MethodPath::create("/example.Service/Method")?, request)
        .await?;
    let elapsed = started.elapsed();

    println!(
        "response: {}",
        String::from_utf8_lossy(response.get_message())
    );
    println!(
        "upstream was called {} time(s) across {:?}",
        calls.load(Ordering::SeqCst),
        elapsed
    );

    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "two failures and one success must reach the upstream"
    );
    assert!(
        elapsed >= Duration::from_millis(20),
        "the retries must be separated by a real backoff"
    );
    Ok(())
}
