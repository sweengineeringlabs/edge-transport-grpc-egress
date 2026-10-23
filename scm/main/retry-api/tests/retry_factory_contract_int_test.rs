//! The contract of `GrpcRetryFactory`: an implementor takes the client to wrap in the request value and returns a
//! client that is a `Transport` and a `RawTransport`; the client of the caller keeps answering through it.

use proto_pattern::{
    BidiStreamMethod, CallBidiStreamRequest, CallBidiStreamResponse, CallClientStreamRequest,
    CallClientStreamResponse, CallError, CallOptions, CallRawBidiStreamRequest,
    CallRawBidiStreamResponse, CallRawClientStreamRequest, CallRawClientStreamResponse,
    CallRawRequest, CallRawRequestBuilder, CallRawResponse, CallRawServerStreamRequest,
    CallRawServerStreamResponse, CallRequest, CallResponse, CallServerStreamRequest,
    CallServerStreamResponse, ClientStreamMethod, DecodeError, EncodeError, GetHeadersRequest,
    GetHeadersResponse, MessageStream, Metadata, Method, MethodPath, Payload, RawTransport,
    ReadMessageRequest, ReadMessageResponse, ReadTrailersRequest, ReadTrailersResponse,
    ResponseStream, ServerStreamMethod, StatusCode, Transport, UnaryMethod,
};
use std::error::Error;

use grpc_egress_retry_api::{BuildGrpcRetryRequest, GrpcRetryFactory};

/// A client that answers every raw call with a body naming the path it was asked for, and refuses what else it is
/// asked.
struct EchoPath;

/// The response stream of a call `EchoPath` refuses.
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

impl RawTransport for EchoPath {
    type Metadata = Metadata;

    async fn call_raw(
        &self,
        request: CallRawRequest,
    ) -> Result<CallRawResponse<Metadata>, CallError> {
        let (path, message, options) = request.into_parts();
        let _request = CallRequest::new(message, options);
        Ok(CallRawResponse::new(CallResponse::new(
            path.get_path().as_bytes().to_vec(),
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
    CallRequest::new(Vec::new(), CallOptions::default())
}

/// @covers: GrpcRetryFactory::build
#[tokio::test]
async fn test_build_returns_a_client_that_answers_through_the_factory() -> Result<(), Box<dyn Error>>
{
    let client = PassThroughFactory.build(BuildGrpcRetryRequest { inner: EchoPath });

    let response = client
        .call_raw({
            let (message, options) = request().into_parts();
            CallRawRequestBuilder::new()
                .with_path(MethodPath::create("/pkg.Service/First")?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;

    assert_eq!(response.get_message(), b"/pkg.Service/First");
    Ok(())
}

/// @covers: GrpcRetryFactory::build
#[tokio::test]
async fn test_build_called_twice_returns_independent_clients() -> Result<(), Box<dyn Error>> {
    let first = PassThroughFactory.build(BuildGrpcRetryRequest { inner: EchoPath });
    let second = PassThroughFactory.build(BuildGrpcRetryRequest { inner: EchoPath });

    let from_first = first
        .call_raw({
            let (message, options) = request().into_parts();
            CallRawRequestBuilder::new()
                .with_path(MethodPath::create("/pkg.Service/A")?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;
    let from_second = second
        .call_raw({
            let (message, options) = request().into_parts();
            CallRawRequestBuilder::new()
                .with_path(MethodPath::create("/pkg.Service/B")?)
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;

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
    impl Method for Ask {
        type Request = Nothing;
        type Response = Nothing;
        const PATH: &'static str = "/pkg.Service/Ask";
    }
    impl UnaryMethod for Ask {}
    struct Nothing;
    impl Payload for Nothing {
        const FULL_NAME: &'static str = "test.Nothing";

        fn encode_into(&self, _out: &mut Vec<u8>) -> Result<(), EncodeError> {
            Ok(())
        }

        fn estimate_encoded_len(&self) -> usize {
            0
        }

        fn decode(_bytes: &[u8]) -> Result<Self, DecodeError> {
            Ok(Nothing)
        }
    }

    let client = PassThroughFactory.build(BuildGrpcRetryRequest { inner: EchoPath });

    let error = client
        .call::<Ask>(CallRequest::new(Nothing, CallOptions::default()))
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
