//! Reusable example: wrap an already-built client in retry and circuit breaker with
//! [`StdGrpcResilientFactory`].
//!
//! The decorator never builds the client: the composition root does that and applies the decorator
//! on top. A test double stands in for the client, so no server is needed. The double
//! fails its first two calls and then succeeds; the retry layer inside the decorator must call it
//! again after a real backoff until it does.
//!
//! [`StdGrpcResilientFactory`]: grpc_egress_resilient_spi::StdGrpcResilientFactory

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

use grpc_egress_resilient::ResilienceConfig;
use grpc_egress_resilient::{BuildGrpcResilientRequest, GrpcResilientFactory};
use grpc_egress_resilient_spi::StdGrpcResilientFactory;

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

    let config = ResilienceConfig::from_policies(
        grpc_egress_retry::GrpcRetryConfig::try_new(
            grpc_egress_retry::BuildGrpcRetryConfigRequest {
                max_attempts: 3,
                initial_backoff_ms: 20,
                backoff_multiplier: 2.0,
                jitter_factor: 0.0,
                max_backoff_ms: 100,
                rate_limit_max_attempts: 1,
                rate_limit_initial_backoff_ms: 100,
                rate_limit_max_backoff_ms: 100,
            },
        )?,
        grpc_egress_breaker::GrpcBreakerConfig::try_new(true, 5, 1, 1)?,
    );
    let client = StdGrpcResilientFactory::new(config)
        .build(BuildGrpcResilientRequest { inner: upstream })?;

    let request = CallRequest::new(
        Vec::new(),
        CallOptions::create(Some(Duration::from_secs(3)), Metadata::default()),
    );
    let started = std::time::Instant::now();
    let response = client
        .call_raw({
            let (message, options) = request.into_parts();
            CallRawRequestBuilder::new()
                .with_path(MethodPath::create("/example.Service/Method")?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;
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
