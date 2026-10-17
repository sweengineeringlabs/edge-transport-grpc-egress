//! The compression of a channel configuration, end to end: `with_compression(Gzip)` makes the client send gzip
//! and list gzip as accepted (`grpc-encoding` and `grpc-accept-encoding`), and `None` makes it send neither.

use std::error::Error;
use std::time::Duration;

use grpc_egress_test_support::{CallFixture, ClientFixture, Fixture, PeerBehaviour, PeerServer};
use grpc_egress_transport::{CompressionMode, GrpcChannelConfig};
use grpc_egress_transport_std::GrpcChannelConfigFactory;
use proto_pattern::RawTransport;

const SHORT: Duration = Duration::from_secs(2);

/// @covers: GrpcChannelConfig::with_compression
#[tokio::test]
async fn test_call_raw_with_gzip_in_the_config_advertises_grpc_encoding_gzip_and_accepts_gzip(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = ClientFixture::from_config(
        &GrpcChannelConfig::new(peer.get_endpoint())
            .allow_plaintext()
            .with_compression(CompressionMode::Gzip),
    )?;

    client
        .call_raw(
            CallFixture::path("svc/M"),
            CallFixture::request(b"payload", SHORT),
        )
        .await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer saw no request")?;
    assert_eq!(
        request.get_header("grpc-encoding"),
        Some("gzip"),
        "client must advertise grpc-encoding=gzip when CompressionMode::Gzip is set"
    );
    let accepted = request
        .get_header("grpc-accept-encoding")
        .ok_or("the client must list what it accepts")?;
    assert!(accepted.contains("gzip"), "{accepted:?}");
    Ok(())
}

/// @covers: GrpcChannelConfig::with_compression
#[tokio::test]
async fn test_call_raw_with_no_compression_in_the_config_sets_no_grpc_encoding(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client =
        ClientFixture::from_config(&GrpcChannelConfig::new(peer.get_endpoint()).allow_plaintext())?;

    client
        .call_raw(
            CallFixture::path("svc/M"),
            CallFixture::request(b"payload", SHORT),
        )
        .await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer saw no request")?;
    assert_eq!(
        request.get_header("grpc-encoding"),
        None,
        "client must NOT set grpc-encoding when compression is None"
    );
    Ok(())
}
