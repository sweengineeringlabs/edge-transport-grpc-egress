//! The contract of `GrpcRetryFactory`: an implementor takes the client to wrap in the request value and returns a
//! client that is a `Transport` and a `RawTransport`; the client of the caller keeps answering through it.

use std::error::Error;

use grpc_egress_retry::{BuildGrpcRetryRequest, GrpcRetryFactory};
use proto_pattern::{
    BidiStreamMethod, CallError, CallOptions, CallRequest, CallResponse, ClientStreamMethod,
    MessageStream, Metadata, MethodPath, RawTransport, ResponseStream, ServerStreamMethod,
    StatusCode, Transport, UnaryMethod,
};

/// A client that answers every raw call with a body naming the path it was asked for, and refuses what else it is
/// asked.
struct EchoPath;

/// The response stream of a call `EchoPath` refuses.
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

impl EchoPath {
    fn refusal(path: &str) -> CallError {
        CallError::Transport {
            path: path.into(),
            code: StatusCode::Unimplemented,
            reason: "the test client only answers raw unary calls".to_string(),
        }
    }
}

impl Transport for EchoPath {
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

impl RawTransport for EchoPath {
    type Metadata = Metadata;

    async fn call_raw(
        &self,
        path: MethodPath,
        _request: CallRequest<Vec<u8>>,
    ) -> Result<CallResponse<Vec<u8>, Metadata>, CallError> {
        Ok(CallResponse::create(
            path.get_path().as_bytes().to_vec(),
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

/// A factory that wraps nothing: the product is the client it was given.
struct PassThroughFactory;

impl GrpcRetryFactory for PassThroughFactory {
    fn build<T: Transport + RawTransport + Send + Sync + 'static>(
        &self,
        req: BuildGrpcRetryRequest<T>,
    ) -> impl Transport + RawTransport + use<T> {
        req.inner
    }
}

fn request() -> CallRequest<Vec<u8>> {
    CallRequest::create(Vec::new(), CallOptions::default())
}

/// @covers: GrpcRetryFactory::build
#[tokio::test]
async fn test_build_returns_a_client_that_answers_through_the_factory() -> Result<(), Box<dyn Error>>
{
    let client = PassThroughFactory.build(BuildGrpcRetryRequest { inner: EchoPath });

    let response = client
        .call_raw(MethodPath::create("/pkg.Service/First")?, request())
        .await?;

    assert_eq!(response.get_message(), b"/pkg.Service/First");
    Ok(())
}

/// @covers: GrpcRetryFactory::build
#[tokio::test]
async fn test_build_called_twice_returns_independent_clients() -> Result<(), Box<dyn Error>> {
    let first = PassThroughFactory.build(BuildGrpcRetryRequest { inner: EchoPath });
    let second = PassThroughFactory.build(BuildGrpcRetryRequest { inner: EchoPath });

    let from_first = first
        .call_raw(MethodPath::create("/pkg.Service/A")?, request())
        .await?;
    let from_second = second
        .call_raw(MethodPath::create("/pkg.Service/B")?, request())
        .await?;

    assert_eq!(from_first.get_message(), b"/pkg.Service/A");
    assert_eq!(from_second.get_message(), b"/pkg.Service/B");
    Ok(())
}

/// The product is a typed `Transport` as well as a raw one: the typed side of the client the factory was given
/// answers through it, here by refusing a typed call it cannot make.
///
/// @covers: GrpcRetryFactory::build
#[tokio::test]
async fn test_build_returns_a_client_that_is_a_typed_transport_too() -> Result<(), Box<dyn Error>> {
    struct Ask;
    impl proto_pattern::Method for Ask {
        type Request = Nothing;
        type Response = Nothing;
        const PATH: &'static str = "/pkg.Service/Ask";
    }
    impl UnaryMethod for Ask {}
    struct Nothing;
    impl proto_pattern::Payload for Nothing {
        const FULL_NAME: &'static str = "test.Nothing";

        fn encode_into(&self, _out: &mut Vec<u8>) -> Result<(), proto_pattern::EncodeError> {
            Ok(())
        }

        fn estimate_encoded_len(&self) -> usize {
            0
        }

        fn decode(_bytes: &[u8]) -> Result<Self, proto_pattern::DecodeError> {
            Ok(Nothing)
        }
    }

    let client = PassThroughFactory.build(BuildGrpcRetryRequest { inner: EchoPath });

    let error = client
        .call::<Ask>(CallRequest::create(Nothing, CallOptions::default()))
        .await
        .err()
        .ok_or("EchoPath refuses typed calls")?;

    assert!(matches!(
        error,
        CallError::Transport {
            code: StatusCode::Unimplemented,
            ..
        }
    ));
    Ok(())
}
