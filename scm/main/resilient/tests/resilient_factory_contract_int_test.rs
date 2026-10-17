//! The contract of `GrpcResilientFactory`: an implementor builds a client that is also a
//! `BreakerObservable` from the breaker domain, and refuses an invalid policy with a
//! `ResilientTransportError`.

use std::error::Error;

use futures::future::BoxFuture;
use grpc_egress_breaker::{
    BreakerDomainError, BreakerObservable, BreakerState, ObserveStateRequest, ObserveStateResponse,
};
use grpc_egress_resilient::{
    BuildGrpcResilientRequest, GrpcResilientFactory, ResilienceConfig, ResilientTransportError,
};
use proto_pattern::{
    BidiStreamMethod, CallError, CallOptions, CallRequest, CallResponse, ClientStreamMethod,
    MessageStream, Metadata, MethodPath, RawTransport, ResponseStream, ServerStreamMethod,
    StatusCode, Transport, UnaryMethod,
};

/// A client that answers every raw call with an empty success, refuses what else it is asked, and reports a closed
/// breaker.
struct ClosedClient;

/// The response stream of a call `ClosedClient` refuses.
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

impl RawTransport for ClosedClient {
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

impl BreakerObservable for ClosedClient {
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

/// A factory that wraps nothing: it checks the policy and hands back [`ClosedClient`].
struct CheckingFactory {
    config: ResilienceConfig,
}

impl GrpcResilientFactory for CheckingFactory {
    fn build<T: Transport + RawTransport + Send + Sync + 'static>(
        &self,
        _req: BuildGrpcResilientRequest<T>,
    ) -> Result<impl Transport + RawTransport + BreakerObservable + use<T>, ResilientTransportError>
    {
        if self.config.max_attempts == 0 {
            return Err(ResilientTransportError::InvalidResilience(
                "max_attempts must be >= 1".into(),
            ));
        }
        Ok(ClosedClient)
    }
}

/// @covers: GrpcResilientFactory::build
#[tokio::test]
async fn test_build_valid_policy_returns_a_client_that_can_be_called_and_observed(
) -> Result<(), Box<dyn Error>> {
    let factory = CheckingFactory {
        config: ResilienceConfig::default(),
    };

    let client = factory.build(BuildGrpcResilientRequest {
        inner: ClosedClient,
    })?;

    let answered = client
        .call_raw(
            MethodPath::create("/pkg.Service/Method")?,
            CallRequest::create(Vec::new(), CallOptions::default()),
        )
        .await?;
    let observed = client.state(ObserveStateRequest).await?;
    assert!(answered.get_message().is_empty());
    assert_eq!(observed.state, BreakerState::Closed);
    Ok(())
}

/// @covers: GrpcResilientFactory::build
#[test]
fn test_build_invalid_policy_returns_invalid_resilience_error() {
    let factory = CheckingFactory {
        config: ResilienceConfig {
            max_attempts: 0,
            ..ResilienceConfig::default()
        },
    };

    let result = factory.build(BuildGrpcResilientRequest {
        inner: ClosedClient,
    });

    assert!(matches!(
        result,
        Err(ResilientTransportError::InvalidResilience(_))
    ));
}
