//! Integration tests for `ProcessorFactory`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use edge_transport_grpc_egress_transport::{
    DescribeRequest, GrpcChannelConfig, GrpcChannelConfigError, ProcessorFactory,
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
fn test_create_valid_config_returns_describable_processor_happy() {
    ensure_rustls_provider();
    let cfg = GrpcChannelConfig::new("http://127.0.0.1:50051").allow_plaintext();
    let processor =
        ProcessorFactory::create(&cfg).expect("assembly must succeed for a valid plaintext config");
    let desc = processor
        .describe(DescribeRequest)
        .expect("describe must succeed on a freshly-constructed processor");
    assert_eq!(
        desc.label, "tonic-grpc-client",
        "describe() must report the real adapter label, not a stub"
    );
}

/// @covers: create
#[test]
fn test_create_tls_required_rejects_plaintext_endpoint_error() {
    ensure_rustls_provider();
    let cfg = GrpcChannelConfig::new("http://127.0.0.1:50051");
    assert!(matches!(
        ProcessorFactory::create(&cfg),
        Err(GrpcChannelConfigError::PlaintextRejected(_))
    ));
}

/// @covers: create
#[test]
fn test_create_https_endpoint_does_not_require_allow_plaintext_edge() {
    ensure_rustls_provider();
    let https = GrpcChannelConfig::new("https://127.0.0.1:50051");
    assert!(
        ProcessorFactory::create(&https).is_ok(),
        "an https:// endpoint must not be rejected even without allow_plaintext()"
    );

    // Contrast in the same test: the equivalent http:// endpoint without
    // allow_plaintext() must still be rejected — proves the exemption is
    // scheme-specific, not a stub that always accepts.
    let http = GrpcChannelConfig::new("http://127.0.0.1:50051");
    assert!(
        ProcessorFactory::create(&http).is_err(),
        "http:// without allow_plaintext() must still be rejected"
    );
}
