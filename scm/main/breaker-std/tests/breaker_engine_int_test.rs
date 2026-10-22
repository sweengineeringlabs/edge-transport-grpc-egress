//! The circuit breaker around the real client, built through the public builder, against real in-process peers:
//! what the engine reports for a refused connection and for the statuses a peer answers is what the breaker
//! classifies, and an open circuit keeps calls away from the peer.

<<<<<<< HEAD
use grpc_egress_breaker::GrpcBreakerConfigProvider as _;
use grpc_egress_transport::GrpcChannelConfigBuilder as _;
=======
use proto_pattern::{
    CallError, CallOptions, CallRawRequestBuilder, CallRawResponse,
    CallRawServerStreamRequestBuilder, CallRawServerStreamResponse, CallRequest, CallResponse,
    DecodeError, EncodeError, MessageStream, Metadata, Method, Payload, RawTransport,
    ReadMessageRequest, ReadMessageResponse, StatusCode, Transport, UnaryMethod,
};
>>>>>>> fix/18-deny-allow-git
use std::error::Error;
use std::time::Duration;

use grpc_egress_breaker::{
    BreakerObservable, BreakerState, BuildGrpcBreakerRequest, GrpcBreakerConfig,
    GrpcBreakerFactory, ObserveStateRequest,
};
use grpc_egress_breaker_std::StdGrpcBreakerFactory;
use grpc_egress_test_support::{
    CallFixture, ClientFixture, ClosedPort, Fixture, PeerBehaviour, PeerServer, RunningPeer,
};
<<<<<<< HEAD
use proto_pattern::{
    CallError, DecodeError, EncodeError, MessageStream, Method, Payload, RawTransport, StatusCode,
    Transport, UnaryMethod,
};
=======
use grpc_egress_transport::GrpcChannelConfig;
use grpc_egress_transport_std::GrpcChannelConfigFactory;
>>>>>>> fix/18-deny-allow-git

const TIMEOUT: Duration = Duration::from_secs(5);

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
    const PATH: &'static str = "/pkg.Service/Ping";
}

impl UnaryMethod for Ping {}

fn fast_config() -> Result<GrpcBreakerConfig, Box<dyn Error>> {
    Ok(breaker_provider().parse_config(
        r#"
            failure_threshold = 3
            cool_down_seconds = 30
            half_open_probe_count = 1
        "#,
    )?)
}

/// A breaker around a real client for `endpoint`.
fn guarded(
    endpoint: &str,
) -> Result<impl Transport + RawTransport + BreakerObservable, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let config = config_builder(endpoint).allow_plaintext().build()?;
    let client = ClientFixture::from_config(&config)?;
    Ok(StdGrpcBreakerFactory::new(fast_config()?)
        .build(BuildGrpcBreakerRequest { inner: client })?)
}

async fn peer_answering_status(code: StatusCode) -> Result<RunningPeer, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    Ok(PeerServer::start(PeerBehaviour::ReplyStatus {
        status: code.get_code().to_string(),
        message: Some("scripted".to_string()),
        retry_after_secs: None,
    })
    .await?)
}

async fn state_of(client: &impl BreakerObservable) -> Result<BreakerState, Box<dyn Error>> {
    Ok(client.state(ObserveStateRequest).await?.state)
}

async fn call_raw(client: &impl RawTransport) -> Result<Vec<u8>, CallError> {
    client
        .call_raw({
            let (message, options) = CallFixture::request(b"ping", TIMEOUT).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .map(CallResponse::into_message)
}

/// @covers: StdGrpcBreakerFactory::build
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_unavailable_peer_opens_the_circuit_and_an_open_circuit_never_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = peer_answering_status(StatusCode::Unavailable).await?;
    let client = guarded(&peer.get_endpoint())?;

    for attempt in 1..=3 {
        let failure = call_raw(&client).await.err();
        assert!(
            matches!(
                failure,
                Some(CallError::Status {
                    code: StatusCode::Unavailable,
                    ..
                })
            ),
            "attempt {attempt} must carry the status of the peer, got {failure:?}"
        );
    }
    assert!(matches!(
        state_of(&client).await?,
        BreakerState::Open { .. }
    ));
    assert_eq!(peer.get_recorded_requests().len(), 3);

    for _ in 0..5 {
        match call_raw(&client).await.err() {
            Some(CallError::Transport {
                code: StatusCode::Unavailable,
                reason,
                ..
            }) => assert!(reason.contains("circuit open"), "{reason}"),
            other => panic!("expected the refusal of the open circuit, got {other:?}"),
        }
    }
    assert_eq!(
        peer.get_recorded_requests().len(),
        3,
        "an open circuit must not reach the peer"
    );
    Ok(())
}

/// @covers: StdGrpcBreakerFactory::build
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_refused_connection_counts_as_a_failure_and_opens_the_circuit(
) -> Result<(), Box<dyn Error>> {
    let closed = ClosedPort::reserve()?;
    let client = guarded(&closed.get_endpoint())?;

    for attempt in 1..=3 {
        let failure = call_raw(&client).await.err();
        match failure {
            Some(CallError::Transport { reason, .. }) => assert!(
                !reason.contains("circuit open"),
                "attempt {attempt} must reach the engine, got the refusal of the circuit"
            ),
            other => panic!("attempt {attempt}: expected a transport failure, got {other:?}"),
        }
    }

    assert!(matches!(
        state_of(&client).await?,
        BreakerState::Open { .. }
    ));
    match call_raw(&client).await.err() {
        Some(CallError::Transport { reason, .. }) => {
            assert!(reason.contains("circuit open"), "{reason}");
        }
        other => panic!("expected the refusal of the open circuit, got {other:?}"),
    }
    Ok(())
}

/// @covers: StdGrpcBreakerFactory::build
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_statuses_that_are_not_the_peers_fault_never_open_the_circuit(
) -> Result<(), Box<dyn Error>> {
    for code in [
        StatusCode::InvalidArgument,
        StatusCode::ResourceExhausted,
        StatusCode::DeadlineExceeded,
        StatusCode::PermissionDenied,
        StatusCode::Unauthenticated,
        StatusCode::NotFound,
    ] {
        let peer = peer_answering_status(code).await?;
        let client = guarded(&peer.get_endpoint())?;

        for _ in 0..6 {
            match call_raw(&client).await.err() {
                Some(CallError::Status { code: got, .. }) => assert_eq!(got, code),
                other => panic!("{code}: expected the status of the peer, got {other:?}"),
            }
        }

        assert_eq!(state_of(&client).await?, BreakerState::Closed, "{code}");
        assert_eq!(
            peer.get_recorded_requests().len(),
            6,
            "{code}: every call must reach the peer"
        );
    }
    Ok(())
}

/// @covers: StdGrpcBreakerFactory::build
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_server_stream_failed_opens_open_the_circuit_and_a_refused_open_never_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = peer_answering_status(StatusCode::Unavailable).await?;
    let client = guarded(&peer.get_endpoint())?;

    for attempt in 1..=3 {
        let failure = client
            .call_raw_server_stream({
                let (message, options) = CallFixture::request(b"watch", TIMEOUT).into_parts();
                CallRawServerStreamRequestBuilder::new()
                    .with_path(CallFixture::path("pkg.Service/Watch"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawServerStreamResponse::into_stream)
            .err();
        assert!(
            matches!(
                failure,
                Some(CallError::Status {
                    code: StatusCode::Unavailable,
                    ..
                })
            ),
            "open {attempt} must fail with the status of the peer, got {failure:?}"
        );
    }
    assert_eq!(peer.get_recorded_requests().len(), 3);

    let refused = client
        .call_raw_server_stream({
            let (message, options) = CallFixture::request(b"watch", TIMEOUT).into_parts();
            CallRawServerStreamRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Watch"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawServerStreamResponse::into_stream)
        .err();
    assert!(
        matches!(
            refused,
            Some(CallError::Transport {
                code: StatusCode::Unavailable,
                ..
            })
        ),
        "got {refused:?}"
    );
    assert_eq!(
        peer.get_recorded_requests().len(),
        3,
        "a refused open must not reach the peer"
    );
    Ok(())
}

/// @covers: StdGrpcBreakerFactory::build
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_server_stream_admitted_open_returns_the_stream_and_keeps_the_circuit_closed(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"hello".to_vec())).await?;
    let client = guarded(&peer.get_endpoint())?;

    let mut stream = client
        .call_raw_server_stream({
            let (message, options) = CallFixture::request(b"watch", TIMEOUT).into_parts();
            CallRawServerStreamRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Watch"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawServerStreamResponse::into_stream)?;
    let first = stream
        .read_message(ReadMessageRequest::new())
        .await
        .map(ReadMessageResponse::into_message)?;

    assert_eq!(first, Some(b"hello".to_vec()));
    assert_eq!(state_of(&client).await?, BreakerState::Closed);
    assert_eq!(peer.get_recorded_requests().len(), 1);
    Ok(())
}

/// @covers: StdGrpcBreakerFactory::build
#[tokio::test(flavor = "multi_thread")]
async fn test_call_typed_unavailable_peer_opens_the_circuit_and_the_refusal_names_the_method(
) -> Result<(), Box<dyn Error>> {
    let peer = peer_answering_status(StatusCode::Unavailable).await?;
    let client = guarded(&peer.get_endpoint())?;
    let request = || {
        CallRequest::new(
            Bytes(b"ping".to_vec()),
            CallOptions::create(Some(TIMEOUT), Metadata::default()),
        )
    };

    for _ in 0..3 {
        let _ = client.call::<Ping>(request()).await;
    }
    let refused = client.call::<Ping>(request()).await.err();

    match refused {
        Some(CallError::Transport {
            path,
            code: StatusCode::Unavailable,
            reason,
        }) => {
            assert_eq!(path.as_ref(), Ping::PATH);
            assert!(reason.contains("circuit open"), "{reason}");
        }
        other => panic!("expected the refusal of the open circuit, got {other:?}"),
    }
    assert_eq!(peer.get_recorded_requests().len(), 3);
    Ok(())
}

/// @covers: StdGrpcBreakerFactory::build
#[tokio::test(flavor = "multi_thread")]
async fn test_call_typed_answered_call_returns_the_typed_response_and_keeps_the_circuit_closed(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"pong".to_vec())).await?;
    let client = guarded(&peer.get_endpoint())?;

    let response = client
        .call::<Ping>(CallRequest::new(
            Bytes(b"ping".to_vec()),
            CallOptions::create(Some(TIMEOUT), Metadata::default()),
        ))
        .await?;

    assert_eq!(response.get_message(), &Bytes(b"pong".to_vec()));
    assert_eq!(state_of(&client).await?, BreakerState::Closed);
    Ok(())
}

fn config_builder(
    endpoint: impl Into<String>,
) -> impl grpc_egress_transport::GrpcChannelConfigBuilder {
    new_config_builder().with_endpoint(endpoint)
}

fn new_config_builder() -> impl grpc_egress_transport::GrpcChannelConfigBuilder {
    use grpc_egress_transport::GrpcChannelConfigBuilderFactory;
    grpc_egress_transport_std::StdGrpcChannelConfigBuilderFactory
        .build(grpc_egress_transport::BuildGrpcChannelConfigBuilderRequest)
}

fn breaker_provider() -> impl grpc_egress_breaker::GrpcBreakerConfigProvider {
    use grpc_egress_breaker::GrpcBreakerConfigProviderFactory;
    grpc_egress_breaker_std::StdGrpcBreakerConfigProviderFactory
        .build(grpc_egress_breaker::BuildGrpcBreakerConfigProviderRequest)
}
