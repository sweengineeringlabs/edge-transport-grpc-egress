//! The contract of `GrpcResilientFactory`: an implementor builds a client that is also a
//! `BreakerObservable` from the breaker domain, and refuses an invalid policy with a
//! `ResilientTransportError`.

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
use std::error::Error;

use futures::future::BoxFuture;
use grpc_egress_breaker_api::{
    BreakerDomainError, BreakerObservable, BreakerState, ObserveStateRequest, ObserveStateResponse,
};
use grpc_egress_resilient_api::{
    BuildGrpcResilientRequest, GrpcResilientFactory, ResilientTransportError,
};

/// A client that answers every raw call with an empty success, refuses what else it is asked, and reports a closed
/// breaker.
struct ClosedClient;

/// The response stream of a call `ClosedClient` refuses.
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

impl ClosedClient {
    fn refusal(path: &str) -> CallError {
        CallError::Transport {
            path: path.into(),
            code: StatusCode::Unimplemented,
            reason: "the test client only answers raw unary calls".to_string(),
        }
    }
}

impl Transport for ClosedClient {
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

impl RawTransport for ClosedClient {
    type Metadata = Metadata;

    async fn call_raw(
        &self,
        request: CallRawRequest,
    ) -> Result<CallRawResponse<Metadata>, CallError> {
        let (_path, message, options) = request.into_parts();
        let _request = CallRequest::new(message, options);
        Ok(CallRawResponse::new(CallResponse::new(
            Vec::new(),
            Metadata::default(),
            Metadata::default(),
        )))
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

impl BreakerObservable for ClosedClient {
    fn state(
        &self,
        _req: ObserveStateRequest,
    ) -> BoxFuture<'_, Result<ObserveStateResponse, BreakerDomainError>> {
        Box::pin(async { Ok(ObserveStateResponse::new(BreakerState::Closed)) })
    }
}

/// A factory that wraps nothing: it checks the policy and hands back [`ClosedClient`].
struct CheckingFactory;

impl GrpcResilientFactory for CheckingFactory {
    fn build<T: Transport + RawTransport + Send + Sync + 'static>(
        &self,
        _req: BuildGrpcResilientRequest<T>,
    ) -> Result<impl Transport + RawTransport + BreakerObservable + use<T>, ResilientTransportError>
    {
        Ok(ClosedClient)
    }
}

/// @covers: GrpcResilientFactory::build
#[tokio::test]
async fn test_build_valid_policy_returns_a_client_that_can_be_called_and_observed(
) -> Result<(), Box<dyn Error>> {
    let factory = CheckingFactory;

    let client = factory.build(BuildGrpcResilientRequest {
        inner: ClosedClient,
    })?;

    let answered = client
        .call_raw(
            CallRawRequestBuilder::new()
                .with_path(MethodPath::create("/pkg.Service/Method")?)
                .with_message(Vec::new())
                .with_options(CallOptions::default())
                .build(),
        )
        .await
        .map(CallRawResponse::into_response)?;
    let observed = client.state(ObserveStateRequest::new()).await?;
    assert!(answered.get_message().is_empty());
    assert_eq!(observed.state(), BreakerState::Closed);
    Ok(())
}

/// @covers: GrpcResilientFactory::build
#[test]
fn test_build_invalid_policy_returns_invalid_resilience_error() {
    assert!(grpc_egress_retry_api::GrpcRetryConfig::try_new(
        grpc_egress_retry_api::BuildGrpcRetryConfigRequest {
            max_attempts: 0,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_backoff_ms: 5000,
            rate_limit_max_attempts: 2,
            rate_limit_initial_backoff_ms: 1000,
            rate_limit_max_backoff_ms: 10000,
        }
    )
    .is_err());
}
