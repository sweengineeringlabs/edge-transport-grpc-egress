//! Integration tests for `GrpcEgressFactory`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use edge_transport_grpc_egress_transport::{
    GrpcChannelConfig, GrpcChannelConfigError, GrpcEgressError, GrpcEgressFactory,
    HealthCheckRequest,
};

fn ensure_rustls_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

/// @covers: create
#[tokio::test]
async fn test_create_valid_config_returns_connectable_transport_happy() {
    ensure_rustls_provider();
    let cfg = GrpcChannelConfig::new("http://127.0.0.1:50051").allow_plaintext();
    let transport = GrpcEgressFactory::create(&cfg)
        .expect("assembly must succeed for a valid plaintext config");
    // Nothing listens on 127.0.0.1:50051 in the test environment, so a real
    // call must genuinely fail — proves this is a connectable client, not a stub.
    let health = transport.health_check(HealthCheckRequest).await;
    assert!(
        matches!(health, Err(GrpcEgressError::Unavailable(_))),
        "health_check against an unbound port must report Unavailable, got: {health:?}"
    );
}

/// @covers: create
#[test]
fn test_create_tls_required_rejects_plaintext_endpoint_error() {
    ensure_rustls_provider();
    let cfg = GrpcChannelConfig::new("http://127.0.0.1:50051");
    assert!(matches!(
        GrpcEgressFactory::create(&cfg),
        Err(GrpcChannelConfigError::PlaintextRejected(_))
    ));
}

/// @covers: create
#[test]
fn test_create_https_endpoint_does_not_require_allow_plaintext_edge() {
    ensure_rustls_provider();
    let https = GrpcChannelConfig::new("https://127.0.0.1:50051");
    assert!(
        GrpcEgressFactory::create(&https).is_ok(),
        "an https:// endpoint must not be rejected even without allow_plaintext()"
    );

    // Contrast in the same test: the equivalent http:// endpoint without
    // allow_plaintext() must still be rejected — proves the exemption is
    // scheme-specific, not a stub that always accepts.
    let http = GrpcChannelConfig::new("http://127.0.0.1:50051");
    assert!(
        GrpcEgressFactory::create(&http).is_err(),
        "http:// without allow_plaintext() must still be rejected"
    );
}
