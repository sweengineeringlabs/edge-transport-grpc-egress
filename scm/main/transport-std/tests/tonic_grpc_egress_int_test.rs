//! Characterisation of the options of the plain client, against a real in-process peer.
//!
//! Each test pins what one option of `GrpcChannelConfig` does today, so that the builder refactor
//! of issue 16 cannot change it silently, and each fails when its option is ignored or changed.
//! Where today's behaviour is a defect (an option that is accepted and has no effect) the test
//! name and the doc comment say so, and the fix replaces the test on purpose.

use grpc_egress_transport_std::{GrpcChannelConfigFactory, GrpcRequestFactory};
use std::error::Error;
use std::time::{Duration, Instant};

use grpc_egress_test_support::{Fixture, PeerBehaviour, PeerServer};
use grpc_egress_transport::{
    CompressionMode, GrpcChannelConfig, GrpcEgressError, GrpcRequest, GrpcStatusCode,
};
use grpc_egress_transport_std::TransportConstruction;

/// @covers: GrpcChannelConfig::with_max_message_bytes
#[tokio::test]
async fn test_call_response_at_max_message_bytes_returns_the_message() -> Result<(), Box<dyn Error>>
{
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1024])).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_max_message_bytes(1024);
    let client = TransportConstruction::create_transport_from_config(&config)?;

    let response = client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            b"ping".to_vec(),
            Duration::from_secs(5),
        ))
        .await?;

    assert_eq!(response.body.len(), 1024);
    Ok(())
}

/// @covers: GrpcChannelConfig::with_max_message_bytes
#[tokio::test]
async fn test_call_response_one_byte_over_max_message_bytes_returns_resource_exhausted(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1025])).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_max_message_bytes(1024);
    let client = TransportConstruction::create_transport_from_config(&config)?;

    let result = client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            b"ping".to_vec(),
            Duration::from_secs(5),
        ))
        .await;

    assert!(
        matches!(
            result,
            Err(GrpcEgressError::Status(
                GrpcStatusCode::ResourceExhausted,
                _
            ))
        ),
        "expected ResourceExhausted, got {result:?}"
    );
    Ok(())
}

/// @covers: GrpcRequest::new
#[tokio::test]
async fn test_call_slower_than_the_deadline_returns_timeout_at_the_deadline(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Delay(Duration::from_secs(3))).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint()).allow_plaintext();
    let client = TransportConstruction::create_transport_from_config(&config)?;

    let started = Instant::now();
    let result = client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            b"ping".to_vec(),
            Duration::from_millis(300),
        ))
        .await;
    let elapsed = started.elapsed();

    assert!(
        matches!(result, Err(GrpcEgressError::Timeout(_))),
        "expected Timeout, got {result:?}"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "the deadline of 300 ms must end the call long before the 3 s delay, took {elapsed:?}"
    );
    Ok(())
}

/// The client's request timeout is a backstop: a call whose own deadline is longer is cut at the timeout.
///
/// @covers: GrpcChannelConfig::with_request_timeout
#[tokio::test]
async fn test_call_unary_request_timeout_shorter_than_the_deadline_returns_timeout_error(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Delay(Duration::from_millis(3000))).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_request_timeout(Duration::from_millis(300));
    let client = TransportConstruction::create_transport_from_config(&config)?;

    let started = Instant::now();
    let result = client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            b"ping".to_vec(),
            Duration::from_secs(10),
        ))
        .await;
    let elapsed = started.elapsed();

    assert!(
        matches!(result, Err(GrpcEgressError::Timeout(_))),
        "the 300 ms request timeout must cut the call, got {result:?}"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "the call must end long before the 3 s delay, took {elapsed:?}"
    );
    Ok(())
}

/// A request timeout longer than the call's own deadline does not extend the deadline.
///
/// @covers: GrpcChannelConfig::with_request_timeout
#[tokio::test]
async fn test_call_unary_request_timeout_longer_than_the_deadline_keeps_the_deadline_edge(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Delay(Duration::from_millis(3000))).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_request_timeout(Duration::from_secs(30));
    let client = TransportConstruction::create_transport_from_config(&config)?;

    let started = Instant::now();
    let result = client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            b"ping".to_vec(),
            Duration::from_millis(300),
        ))
        .await;
    let elapsed = started.elapsed();

    assert!(
        matches!(result, Err(GrpcEgressError::Timeout(_))),
        "the 300 ms deadline must cut the call, got {result:?}"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "the call must end long before the 3 s delay, took {elapsed:?}"
    );
    Ok(())
}

/// @covers: GrpcChannelConfig::with_compression
#[tokio::test]
async fn test_call_without_compression_sends_no_encoding_header_and_the_message_as_it_is(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_compression(CompressionMode::None);
    let client = TransportConstruction::create_transport_from_config(&config)?;
    let payload = vec![b'a'; 4096];

    client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            payload.clone(),
            Duration::from_secs(5),
        ))
        .await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("grpc-encoding"), None);
    let frame = request
        .get_frames()
        .first()
        .ok_or("the request carried no message")?;
    assert!(!frame.is_compressed());
    assert_eq!(frame.get_payload(), payload.as_slice());
    Ok(())
}

/// @covers: GrpcChannelConfig::new
#[tokio::test]
async fn test_call_sends_the_request_to_the_endpoint_of_the_config_with_a_leading_slash_path(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint()).allow_plaintext();
    let client = TransportConstruction::create_transport_from_config(&config)?;

    client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            b"ping".to_vec(),
            Duration::from_secs(5),
        ))
        .await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_path(), "/pkg.Service/Method");
    Ok(())
}

/// @covers: GrpcChannelConfig::new
#[tokio::test]
async fn test_call_to_a_closed_endpoint_returns_connection_failed() -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let closed = Fixture::reserve_closed_port()?;
    let config = GrpcChannelConfig::new(closed.get_endpoint()).allow_plaintext();
    let client = TransportConstruction::create_transport_from_config(&config)?;

    let result = client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            b"ping".to_vec(),
            Duration::from_secs(5),
        ))
        .await;

    assert!(
        matches!(result, Err(GrpcEgressError::ConnectionFailed(_))),
        "expected ConnectionFailed, got {result:?}"
    );
    Ok(())
}
