//! `ClientFixture::from_config` builds a client from a channel configuration, through the public
//! builder, and the client reaches a peer.

use proto_pattern::{CallRawRequestBuilder, CallRawResponse, RawTransport};
use std::error::Error;
use std::time::Duration;

use grpc_egress_test_support::{CallFixture, ClientFixture, Fixture, PeerBehaviour, PeerServer};
use grpc_egress_transport::{GrpcChannelConfig, GrpcChannelConfigError};
use grpc_egress_transport_std::GrpcChannelConfigFactory;

/// @covers: ClientFixture::from_config
#[tokio::test]
async fn test_from_config_plaintext_endpoint_builds_a_client_that_reaches_the_peer(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"pong".to_vec())).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint()).allow_plaintext();

    let client = ClientFixture::from_config(&config)?;
    let response = client
        .call_raw({
            let (message, options) =
                CallFixture::request(b"ping", Duration::from_secs(5)).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;

    assert_eq!(response.get_message(), b"pong");
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

/// @covers: CallFixture::path
#[test]
fn test_path_with_and_without_the_leading_slash_is_the_same_path() {
    assert_eq!(
        CallFixture::path("pkg.Service/Method"),
        CallFixture::path("/pkg.Service/Method")
    );
}
