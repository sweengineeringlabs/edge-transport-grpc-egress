//! Reusable example: drive a real call through [`StdGrpcRetryFactory`] so the
//! backoff/attempt loop (delegating to the shared `edge-transport-retry`
//! crate's `BackoffScheduler`) actually executes — not just constructs.
//!
//! A test double that fails twice and then succeeds is enough to prove this; no real gRPC server
//! is needed since `StdGrpcRetryFactory` decorates any client that is a `Transport` and a `RawTransport`.
//!
//! [`StdGrpcRetryFactory`]: grpc_egress_retry_std::StdGrpcRetryFactory

use proto_pattern::{
    BidiStreamMethod, CallBidiStreamRequest, CallBidiStreamResponse, CallClientStreamRequest,
    CallClientStreamResponse, CallError, CallOptions, CallRawBidiStreamRequest,
    CallRawBidiStreamResponse, CallRawClientStreamRequest, CallRawClientStreamResponse,
    CallRawRequest, CallRawRequestBuilder, CallRawResponse, CallRawServerStreamRequest,
    CallRawServerStreamResponse, CallRequest, CallResponse, CallServerStreamRequest,
    CallServerStreamResponse, ClientStreamMethod, GetHeadersRequest, GetHeadersResponse,
    MessageStream, Metadata, MethodPath, RawTransport, ReadMessageRequest, ReadMessageResponse,
    ReadTrailersRequest, ReadTrailersResponse, ResponseStream, ServerStreamMethod, StatusCode,
    Transport, UnaryMethod,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use grpc_egress_retry::GrpcRetryConfig;
use grpc_egress_retry::{BuildGrpcRetryRequest, GrpcRetryFactory};
use grpc_egress_retry_std::StdGrpcRetryFactory;

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
    async fn read_message(
        &mut self,
        _request: ReadMessageRequest,
    ) -> Result<ReadMessageResponse<T>, CallError> {
        Ok(ReadMessageResponse::new(None))
    }
}

impl<T: Send> ResponseStream<T> for NoStream {
    type Metadata = Metadata;

    fn get_headers(
        &self,
        _request: GetHeadersRequest,
    ) -> Result<GetHeadersResponse<'_, Metadata>, CallError> {
        Ok(GetHeadersResponse::new(&self.headers))
    }

    async fn read_trailers(
        &mut self,
        _request: ReadTrailersRequest,
    ) -> Result<ReadTrailersResponse<Metadata>, CallError> {
        Ok(ReadTrailersResponse::new(Metadata::default()))
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
        request: CallRawRequest,
    ) -> Result<CallRawResponse<Metadata>, CallError> {
        let (path, message, options) = request.into_parts();
        let _request = CallRequest::new(message, options);
        {
            let attempt = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            if attempt <= self.fail_until {
                Err(CallError::Transport {
                    path: path.get_path().into(),
                    code: StatusCode::Unavailable,
                    reason: format!("upstream still down (attempt {attempt})"),
                })
            } else {
                Ok(CallResponse::new(
                    format!("ok on attempt {attempt}").into_bytes(),
                    Metadata::default(),
                    Metadata::default(),
                ))
            }
        }
        .map(CallRawResponse::new)
    }

    async fn call_raw_server_stream(
        &self,
        request: CallRawServerStreamRequest,
    ) -> Result<
        CallRawServerStreamResponse<impl ResponseStream<Vec<u8>, Metadata = Metadata>>,
        CallError,
    > {
        let (path, message, options) = request.into_parts();
        let _request = CallRequest::new(message, options);
        { Err::<NoStream, _>(Self::refusal(path.get_path())) }.map(CallRawServerStreamResponse::new)
    }

    async fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        request: CallRawClientStreamRequest<S>,
    ) -> Result<CallRawClientStreamResponse<Metadata>, CallError> {
        let (path, _requests, _options) = request.into_parts();
        { Err(Self::refusal(path.get_path())) }.map(CallRawClientStreamResponse::new)
    }

    async fn call_raw_bidi_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        request: CallRawBidiStreamRequest<S>,
    ) -> Result<
        CallRawBidiStreamResponse<impl ResponseStream<Vec<u8>, Metadata = Metadata>>,
        CallError,
    > {
        let (path, _requests, _options) = request.into_parts();
        { Err::<NoStream, _>(Self::refusal(path.get_path())) }.map(CallRawBidiStreamResponse::new)
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
        request: CallServerStreamRequest<M::Request>,
    ) -> Result<
        CallServerStreamResponse<impl ResponseStream<M::Response, Metadata = Metadata>>,
        CallError,
    > {
        let (message, options) = request.into_parts();
        let _request = CallRequest::new(message, options);
        { Err::<NoStream, _>(Self::refusal(M::PATH)) }.map(CallServerStreamResponse::new)
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        request: CallClientStreamRequest<S>,
    ) -> Result<CallClientStreamResponse<M::Response, Metadata>, CallError> {
        let (_requests, _options) = request.into_parts();
        { Err(Self::refusal(M::PATH)) }.map(CallClientStreamResponse::new)
    }

    async fn call_bidi_stream<M: BidiStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        request: CallBidiStreamRequest<S>,
    ) -> Result<
        CallBidiStreamResponse<impl ResponseStream<M::Response, Metadata = Metadata>>,
        CallError,
    > {
        let (_requests, _options) = request.into_parts();
        { Err::<NoStream, _>(Self::refusal(M::PATH)) }.map(CallBidiStreamResponse::new)
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
    let request = CallRequest::new(
        Vec::new(),
        CallOptions::create(Some(Duration::from_secs(3)), Metadata::default()),
    );
    let started = std::time::Instant::now();
    let result = client
        .call_raw({
            let (message, options) = request.into_parts();
            CallRawRequestBuilder::new()
                .with_path(MethodPath::create("/example.Service/Method")?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response);
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
