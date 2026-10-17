//! The contract of `GrpcBreakerFactory`: an implementor returns a client that is a `Transport`, a `RawTransport`
//! and a `BreakerObservable`, or refuses the policy with a `BreakerDomainError`.

use std::error::Error;

use futures::future::BoxFuture;
use grpc_egress_breaker::{
    BreakerDomainError, BreakerObservable, BreakerState, BuildGrpcBreakerRequest,
    GrpcBreakerConfig, GrpcBreakerFactory, ObserveStateRequest, ObserveStateResponse,
};
use proto_pattern::{
    BidiStreamMethod, CallError, CallOptions, CallRequest, CallResponse, ClientStreamMethod,
    DecodeError, EncodeError, MessageStream, Metadata, Method, MethodPath, Payload, RawTransport,
    ResponseStream, ServerStreamMethod, StatusCode, Transport, UnaryMethod,
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
    async fn read_message(&mut self) -> Result<Option<T>, CallError> {
        Ok(None)
    }
}

impl<T: Send> ResponseStream<T> for NoStream {
    type Metadata = Metadata;

    fn get_headers(&self) -> &Metadata {
        &self.0
    }

    async fn read_trailers(&mut self) -> Result<Metadata, CallError> {
        Ok(Metadata::default())
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
        Ok(CallResponse::create(
            empty,
            Metadata::default(),
            Metadata::default(),
        ))
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

impl RawTransport for Quiet {
    type Metadata = Metadata;

    async fn call_raw(
        &self,
        _path: MethodPath,
        _request: CallRequest<Vec<u8>>,
    ) -> Result<CallResponse<Vec<u8>, Metadata>, CallError> {
        Ok(CallResponse::create(
            Vec::new(),
            Metadata::default(),
            Metadata::default(),
        ))
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
        request: CallRequest<M::Request>,
    ) -> Result<impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>, CallError>
    {
        self.0.call_server_stream::<M>(request).await
    }

    async fn call_client_stream<M: ClientStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        requests: S,
        options: CallOptions,
    ) -> Result<CallResponse<M::Response, <Self as Transport>::Metadata>, CallError> {
        self.0.call_client_stream::<M, S>(requests, options).await
    }

    async fn call_bidi_stream<M: BidiStreamMethod, S: MessageStream<M::Request> + 'static>(
        &self,
        requests: S,
        options: CallOptions,
    ) -> Result<impl ResponseStream<M::Response, Metadata = <Self as Transport>::Metadata>, CallError>
    {
        self.0.call_bidi_stream::<M, S>(requests, options).await
    }
}

impl<T: Transport + RawTransport> RawTransport for Observed<T> {
    type Metadata = <T as RawTransport>::Metadata;

    async fn call_raw(
        &self,
        path: MethodPath,
        request: CallRequest<Vec<u8>>,
    ) -> Result<CallResponse<Vec<u8>, <Self as RawTransport>::Metadata>, CallError> {
        self.0.call_raw(path, request).await
    }

    async fn call_raw_server_stream(
        &self,
        path: MethodPath,
        request: CallRequest<Vec<u8>>,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>, CallError>
    {
        self.0.call_raw_server_stream(path, request).await
    }

    async fn call_raw_client_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        path: MethodPath,
        requests: S,
        options: CallOptions,
    ) -> Result<CallResponse<Vec<u8>, <Self as RawTransport>::Metadata>, CallError> {
        self.0.call_raw_client_stream(path, requests, options).await
    }

    async fn call_raw_bidi_stream<S: MessageStream<Vec<u8>> + 'static>(
        &self,
        path: MethodPath,
        requests: S,
        options: CallOptions,
    ) -> Result<impl ResponseStream<Vec<u8>, Metadata = <Self as RawTransport>::Metadata>, CallError>
    {
        self.0.call_raw_bidi_stream(path, requests, options).await
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
            path(),
            CallRequest::create(Vec::new(), CallOptions::default()),
        )
        .await?;
    let typed = client
        .call::<Ping>(CallRequest::create(
            Bytes(Vec::new()),
            CallOptions::default(),
        ))
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
