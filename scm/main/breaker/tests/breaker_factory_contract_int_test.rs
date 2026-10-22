//! The contract of `GrpcBreakerFactory`: an implementor returns a client that is a `Transport`, a `RawTransport`
//! and a `BreakerObservable`, or refuses the policy with a `BreakerDomainError`.

use proto_pattern::{
    BidiStreamMethod, CallBidiStreamRequest, CallBidiStreamResponse, CallClientStreamRequest,
    CallClientStreamResponse, CallError, CallOptions, CallRawBidiStreamRequest,
    CallRawBidiStreamRequestBuilder, CallRawBidiStreamResponse, CallRawClientStreamRequest,
    CallRawClientStreamRequestBuilder, CallRawClientStreamResponse, CallRawRequest,
    CallRawRequestBuilder, CallRawResponse, CallRawServerStreamRequest,
    CallRawServerStreamRequestBuilder, CallRawServerStreamResponse, CallRequest, CallResponse,
    CallServerStreamRequest, CallServerStreamResponse, ClientStreamMethod, DecodeError,
    EncodeError, GetHeadersRequest, GetHeadersResponse, MessageStream, Metadata, Method,
    MethodPath, Payload, RawTransport, ReadMessageRequest, ReadMessageResponse,
    ReadTrailersRequest, ReadTrailersResponse, ResponseStream, ServerStreamMethod, StatusCode,
    Transport, UnaryMethod,
};
use std::error::Error;

use futures::future::BoxFuture;
use grpc_egress_breaker::{
    BreakerDomainError, BreakerObservable, BreakerState, BuildGrpcBreakerRequest,
    GrpcBreakerConfig, GrpcBreakerFactory, ObserveStateRequest, ObserveStateResponse,
};

/// A message that is its own bytes.
#[derive(Debug, PartialEq)]
struct Bytes(Vec<u8>);

impl Payload for Bytes {
    const FULL_NAME: &'static str = "test.Bytes";

    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        out.extend_from_slice(&self.0);
        Ok(())
    }

    fn estimate_encoded_len(&self) -> usize {
        self.0.len()
    }

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        Ok(Self(bytes.to_vec()))
    }
}

struct Ping;

impl Method for Ping {
    type Request = Bytes;
    type Response = Bytes;
    const PATH: &'static str = "/test.Peer/Ping";
}

impl UnaryMethod for Ping {}

/// The stream of a transport that never streams: it refuses to open, so none of these is ever read.
struct NoStream(Metadata);

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
        Ok(GetHeadersResponse::new(&self.0))
    }

    async fn read_trailers(
        &mut self,
        _request: ReadTrailersRequest,
    ) -> Result<ReadTrailersResponse<Metadata>, CallError> {
        Ok(ReadTrailersResponse::new(Metadata::default()))
    }
}

/// A client that answers every unary call, typed or raw, with an empty success and refuses to stream.
struct Quiet;

impl Quiet {
    fn refusal(path: &str) -> CallError {
        CallError::Transport {
            path: path.into(),
            code: StatusCode::Unimplemented,
            reason: "this client does not stream".to_string(),
        }
    }
}

impl Transport for Quiet {
    type Metadata = Metadata;

    async fn call<M: UnaryMethod>(
        &self,
        _request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, Metadata>, CallError> {
        let empty = M::Response::decode(&[]).map_err(CallError::Decode)?;
        Ok(CallResponse::new(
            empty,
            Metadata::default(),
            Metadata::default(),
        ))
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

impl RawTransport for Quiet {
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

/// The wrapped client, reporting a closed breaker: both traits forward to the client it wraps.
struct Observed<T>(T);

impl<T: Transport + RawTransport> Transport for Observed<T> {
    type Metadata = <T as Transport>::Metadata;

    async fn call<M: UnaryMethod>(
        &self,
        request: CallRequest<M::Request>,
    ) -> Result<CallResponse<M::Response, <Self as Transport>::Metadata>, CallError> {
        self.0.call::<M>(request).await
    }

    async fn call_server_stream<M: ServerStreamMethod>(
        &self,
        request: CallServerStreamRequest<M::Request>,
    ) -> Result<
        CallServerStreamResponse<
            impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>,
        >,
        CallError,
    > {
        let (message, options) = request.into_parts();
        let request = CallRequest::new(message, options);
        self.0
            .call_server_stream::<M>({
                let (message, options) = request.into_parts();
                CallServerStreamRequest::new(message, options)
            })
            .await
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        request: CallClientStreamRequest<S>,
    ) -> Result<CallClientStreamResponse<M::Response, <Self as Transport>::Metadata>, CallError>
    {
        let (requests, options) = request.into_parts();
        self.0
            .call_client_stream::<M, S>(CallClientStreamRequest::new(requests, options))
            .await
    }

    async fn call_bidi_stream<M: BidiStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        request: CallBidiStreamRequest<S>,
    ) -> Result<
        CallBidiStreamResponse<
            impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>,
        >,
        CallError,
    > {
        let (requests, options) = request.into_parts();
        self.0
            .call_bidi_stream::<M, S>(CallBidiStreamRequest::new(requests, options))
            .await
    }
}

impl<T: Transport + RawTransport> RawTransport for Observed<T> {
    type Metadata = <T as RawTransport>::Metadata;

    async fn call_raw(
        &self,
        request: CallRawRequest,
    ) -> Result<CallRawResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, message, options) = request.into_parts();
        let request = CallRequest::new(message, options);
        self.0
            .call_raw({
                let (message, options) = request.into_parts();
                CallRawRequestBuilder::new()
                    .with_path(path)
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
    }

    async fn call_raw_server_stream(
        &self,
        request: CallRawServerStreamRequest,
    ) -> Result<
        CallRawServerStreamResponse<
            impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>,
        >,
        CallError,
    > {
        let (path, message, options) = request.into_parts();
        let request = CallRequest::new(message, options);
        self.0
            .call_raw_server_stream({
                let (message, options) = request.into_parts();
                CallRawServerStreamRequestBuilder::new()
                    .with_path(path)
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
    }

    async fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        request: CallRawClientStreamRequest<S>,
    ) -> Result<CallRawClientStreamResponse<<Self as RawTransport>::Metadata>, CallError> {
        let (path, requests, options) = request.into_parts();
        self.0
            .call_raw_client_stream(
                CallRawClientStreamRequestBuilder::new()
                    .with_path(path)
                    .with_requests(requests)
                    .with_options(options)
                    .build(),
            )
            .await
    }

    async fn call_raw_bidi_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        request: CallRawBidiStreamRequest<S>,
    ) -> Result<
        CallRawBidiStreamResponse<
            impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>,
        >,
        CallError,
    > {
        let (path, requests, options) = request.into_parts();
        self.0
            .call_raw_bidi_stream(
                CallRawBidiStreamRequestBuilder::new()
                    .with_path(path)
                    .with_requests(requests)
                    .with_options(options)
                    .build(),
            )
            .await
    }
}

impl<T: Send + Sync> BreakerObservable for Observed<T> {
    fn state(
        &self,
        _req: ObserveStateRequest,
    ) -> BoxFuture<'_, Result<ObserveStateResponse, BreakerDomainError>> {
        Box::pin(async {
            Ok(ObserveStateResponse {
                state: BreakerState::Closed,
            })
        })
    }
}

/// A factory that refuses a policy without a threshold and otherwise wraps without guarding.
struct CheckingFactory {
    config: GrpcBreakerConfig,
}

impl GrpcBreakerFactory for CheckingFactory {
    fn build<T: Transport + RawTransport + Send + Sync + 'static>(
        &self,
        req: BuildGrpcBreakerRequest<T>,
    ) -> Result<impl Transport + RawTransport + BreakerObservable + use<T>, BreakerDomainError>
    {
        if self.config.failure_threshold == 0 {
            return Err(BreakerDomainError::InvalidConfig(
                "failure_threshold must be >= 1".into(),
            ));
        }
        Ok(Observed(req.inner))
    }
}

fn path() -> MethodPath {
    MethodPath::create(Ping::PATH).unwrap_or_else(|e| panic!("{e}"))
}

/// @covers: GrpcBreakerFactory::build
#[tokio::test]
async fn test_build_valid_policy_returns_a_client_that_can_be_called_both_ways_and_observed(
) -> Result<(), Box<dyn Error>> {
    let factory = CheckingFactory {
        config: GrpcBreakerConfig::default(),
    };

    let client = factory.build(BuildGrpcBreakerRequest { inner: Quiet })?;

    let raw = client
        .call_raw(
            CallRawRequestBuilder::new()
                .with_path(path())
                .with_message(Vec::new())
                .with_options(CallOptions::default())
                .build(),
        )
        .await
        .map(CallRawResponse::into_response)?;
    let typed = client
        .call::<Ping>(CallRequest::new(Bytes(Vec::new()), CallOptions::default()))
        .await?;
    assert_eq!(raw.get_message(), &Vec::<u8>::new());
    assert_eq!(typed.get_message(), &Bytes(Vec::new()));
    assert_eq!(
        client.state(ObserveStateRequest).await?.state,
        BreakerState::Closed
    );
    Ok(())
}

/// @covers: GrpcBreakerFactory::build
#[test]
fn test_build_policy_without_a_threshold_returns_invalid_config_error() {
    let factory = CheckingFactory {
        config: GrpcBreakerConfig {
            failure_threshold: 0,
            ..GrpcBreakerConfig::default()
        },
    };

    let result = factory.build(BuildGrpcBreakerRequest { inner: Quiet });

    assert!(matches!(result, Err(BreakerDomainError::InvalidConfig(_))));
}
