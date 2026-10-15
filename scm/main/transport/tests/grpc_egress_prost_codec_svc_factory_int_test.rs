//! Integration tests for `GrpcEgressProstCodecFactory` (prost feature only).
#![cfg(feature = "prost")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use edge_transport_grpc_egress_transport::{
    GrpcChannelConfig, GrpcChannelConfigError, GrpcEgressProstCodecFactory, GrpcEgressResult,
    TransportSvc,
};

fn ensure_rustls_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

/// @covers: create
#[test]
fn test_create_valid_config_returns_ok_happy() {
    ensure_rustls_provider();
    let valid = GrpcChannelConfig::new("http://127.0.0.1:50051").allow_plaintext();
    assert!(
        GrpcEgressProstCodecFactory::create(&valid).is_ok(),
        "assembly must succeed for a valid plaintext config"
    );

    // Negative counterpart in the same test: the same endpoint without
    // allow_plaintext() must be rejected — proves `create` isn't a stub
    // that always says Ok.
    let invalid = GrpcChannelConfig::new("http://127.0.0.1:50051");
    assert!(
        GrpcEgressProstCodecFactory::create(&invalid).is_err(),
        "a plaintext endpoint without allow_plaintext() must be rejected"
    );
}

/// @covers: create
#[test]
fn test_create_tls_required_rejects_plaintext_endpoint_error() {
    ensure_rustls_provider();
    let cfg = GrpcChannelConfig::new("http://127.0.0.1:50051");
    assert!(matches!(
        GrpcEgressProstCodecFactory::create(&cfg),
        Err(GrpcChannelConfigError::PlaintextRejected(_))
    ));
}

/// @covers: create
#[tokio::test]
async fn test_create_returns_working_call_unary_encoded_edge() {
    ensure_rustls_provider();
    let cfg = GrpcChannelConfig::new("http://127.0.0.1:50051").allow_plaintext();
    let client = GrpcEgressProstCodecFactory::create(&cfg).expect("valid plaintext config");
    let request = edge_transport_grpc_egress_transport::GrpcRequest::new(
        "svc/Method",
        Vec::new(),
        Duration::from_secs(1),
    );
    // Nothing listens on 127.0.0.1:50051 — a real call must genuinely fail,
    // proving `call_unary_encoded`'s default delegates to a real transport.
    let result = client.call_unary_encoded(request).await;
    assert!(result.is_err(), "call against an unbound port must fail");
}

/// Round-trips as both request and response type for
/// [`test_call_unary_typed_connection_failed_when_no_server_is_listening`].
#[derive(Clone, PartialEq, prost::Message)]
struct ProbeMessage {
    #[prost(string, tag = "1")]
    text: String,
}

/// @covers: call_unary_typed
#[tokio::test]
async fn test_call_unary_typed_connection_failed_when_no_server_is_listening() {
    ensure_rustls_provider();
    let cfg = GrpcChannelConfig::new("http://127.0.0.1:50051").allow_plaintext();
    let client = GrpcEgressProstCodecFactory::create(&cfg).expect("valid plaintext config");
    let ping = ProbeMessage {
        text: "hello".to_string(),
    };
    let result: GrpcEgressResult<ProbeMessage> = TransportSvc::call_unary_typed(
        client.as_ref(),
        "svc/Method",
        &ping,
        Duration::from_secs(1),
    )
    .await;
    assert!(
        result.is_err(),
        "a call against an unbound port must genuinely fail"
    );
}
