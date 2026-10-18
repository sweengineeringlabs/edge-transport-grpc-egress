//! Characterisation of the options of the plain client, against a real in-process peer.
//!
//! Each test pins what one option of `GrpcChannelConfig` does, so that a change cannot slip in silently, and
//! each fails when its option is ignored or changed.

use std::error::Error;
use std::time::{Duration, Instant};

use grpc_egress_test_support::{CallFixture, ClientFixture, Fixture, PeerBehaviour, PeerServer};
use grpc_egress_transport::{CompressionMode, GrpcChannelConfig};
use grpc_egress_transport_std::GrpcChannelConfigFactory;
use proto_pattern::{CallError, RawTransport, StatusCode};

const SHORT: Duration = Duration::from_secs(5);

/// @covers: GrpcChannelConfig::with_max_message_bytes
#[tokio::test]
async fn test_call_response_at_max_message_bytes_returns_the_message() -> Result<(), Box<dyn Error>>
{
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1024])).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_max_message_bytes(1024);
    let client = ClientFixture::from_config(&config)?;

    let response = client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(b"ping", SHORT),
        )
        .await?;

    assert_eq!(response.get_message().len(), 1024);
    Ok(())
}

/// The engine's own code for a response over the limit is `OutOfRange` (the old client said `ResourceExhausted`).
///
/// @covers: GrpcChannelConfig::with_max_message_bytes
#[tokio::test]
async fn test_call_response_one_byte_over_max_message_bytes_returns_out_of_range(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1025])).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_max_message_bytes(1024);
    let client = ClientFixture::from_config(&config)?;

    let result = client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(b"ping", SHORT),
        )
        .await;

    assert!(
        matches!(
            result,
            Err(CallError::Status {
                code: StatusCode::OutOfRange,
                ..
            })
        ),
        "expected OutOfRange, got {:?}",
        result.as_ref().err()
    );
    Ok(())
}

/// The message cap of a configuration limits what the client sends as well as what it reads (the old client capped
/// responses only): a request one byte over the cap never leaves the client.
///
/// @covers: GrpcChannelConfig::with_max_message_bytes
#[tokio::test]
async fn test_call_request_one_byte_over_max_message_bytes_fails_before_it_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_max_message_bytes(1024);
    let client = ClientFixture::from_config(&config)?;

    let result = client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(&[1u8; 1025], SHORT),
        )
        .await;

    assert!(
        matches!(&result, Err(CallError::Encode(e)) if e.is_too_large()),
        "expected a too-large encode failure, got {:?}",
        result.as_ref().err()
    );
    assert!(
        peer.get_recorded_requests().is_empty(),
        "the peer must not see the request"
    );
    Ok(())
}

/// @covers: CallOptions
#[tokio::test]
async fn test_call_slower_than_the_timeout_returns_status_deadline_exceeded_at_the_timeout(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Delay(Duration::from_secs(3))).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint()).allow_plaintext();
    let client = ClientFixture::from_config(&config)?;

    let started = Instant::now();
    let result = client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(b"ping", Duration::from_millis(300)),
        )
        .await;
    let elapsed = started.elapsed();

    assert!(
        matches!(
            result,
            Err(CallError::Status {
                code: StatusCode::DeadlineExceeded,
                ..
            })
        ),
        "expected DeadlineExceeded, got {:?}",
        result.as_ref().err()
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "the timeout of 300 ms must end the call long before the 3 s delay, took {elapsed:?}"
    );
    Ok(())
}

/// The client's request timeout is a backstop: a call whose own timeout is longer is cut at the request timeout.
///
/// @covers: GrpcChannelConfig::with_request_timeout
#[tokio::test]
async fn test_call_request_timeout_shorter_than_the_call_timeout_returns_status_deadline_exceeded(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Delay(Duration::from_millis(3000))).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_request_timeout(Duration::from_millis(300));
    let client = ClientFixture::from_config(&config)?;

    let started = Instant::now();
    let result = client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(b"ping", Duration::from_secs(10)),
        )
        .await;
    let elapsed = started.elapsed();

    assert!(
        matches!(
            result,
            Err(CallError::Status {
                code: StatusCode::DeadlineExceeded,
                ..
            })
        ),
        "the 300 ms request timeout must cut the call, got {:?}",
        result.as_ref().err()
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "the call must end long before the 3 s delay, took {elapsed:?}"
    );
    Ok(())
}

/// A request timeout longer than the call's own timeout does not extend it.
///
/// @covers: GrpcChannelConfig::with_request_timeout
#[tokio::test]
async fn test_call_request_timeout_longer_than_the_call_timeout_keeps_the_call_timeout(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Delay(Duration::from_millis(3000))).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_request_timeout(Duration::from_secs(30));
    let client = ClientFixture::from_config(&config)?;

    let started = Instant::now();
    let result = client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(b"ping", Duration::from_millis(300)),
        )
        .await;
    let elapsed = started.elapsed();

    assert!(
        matches!(
            result,
            Err(CallError::Status {
                code: StatusCode::DeadlineExceeded,
                ..
            })
        ),
        "the 300 ms call timeout must cut the call, got {:?}",
        result.as_ref().err()
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
    let client = ClientFixture::from_config(&config)?;
    let payload = vec![b'a'; 4096];

    client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(&payload, SHORT),
        )
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
    let client = ClientFixture::from_config(&config)?;

    client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(b"ping", SHORT),
        )
        .await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_path(), "/pkg.Service/Method");
    Ok(())
}

/// @covers: GrpcChannelConfig::new
#[tokio::test]
async fn test_call_to_a_closed_endpoint_returns_transport_unavailable() -> Result<(), Box<dyn Error>>
{
    Fixture::ensure_tls_provider();
    let closed = Fixture::reserve_closed_port()?;
    let config = GrpcChannelConfig::new(closed.get_endpoint()).allow_plaintext();
    let client = ClientFixture::from_config(&config)?;

    let result = client
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(b"ping", SHORT),
        )
        .await;

    assert!(
        matches!(
            result,
            Err(CallError::Transport {
                code: StatusCode::Unavailable,
                ..
            })
        ),
        "expected Transport Unavailable, got {:?}",
        result.as_ref().err()
    );
    Ok(())
}
