//! `ClientFixture::from_config` builds a client from a channel configuration, through the public
//! builder, and the client reaches a peer.

use std::error::Error;
use std::time::Duration;

use grpc_egress_test_support::{ClientFixture, Fixture, PeerBehaviour, PeerServer};
use grpc_egress_transport::{GrpcChannelConfig, GrpcChannelConfigError, GrpcRequest};
use grpc_egress_transport_std::{GrpcChannelConfigFactory, GrpcRequestFactory};

/// @covers: ClientFixture::from_config
#[tokio::test]
async fn test_from_config_plaintext_endpoint_builds_a_client_that_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"pong".to_vec())).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint()).allow_plaintext();

    let client = ClientFixture::from_config(&config)?;
    let response = client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            b"ping".to_vec(),
            Duration::from_secs(5),
        ))
        .await?;

    assert_eq!(response.body, b"pong");
    assert_eq!(peer.get_recorded_requests().len(), 1);
    Ok(())
}

/// @covers: ClientFixture::from_config
#[test]
fn test_from_config_plaintext_endpoint_without_the_opt_in_is_refused() {
    Fixture::ensure_tls_provider();
    let config = GrpcChannelConfig::new("http://127.0.0.1:1");

    let result = ClientFixture::from_config(&config);

    assert!(matches!(
        result,
        Err(GrpcChannelConfigError::PlaintextRejected(_))
    ));
}
