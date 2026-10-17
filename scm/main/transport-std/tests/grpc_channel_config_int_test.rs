//! Integration tests for `GrpcChannelConfig` and `create_transport_from_config`.
//!
//! Verifies the **fail-closed TLS-by-default invariant** at the
//! public-facade boundary via the `create_transport_from_config` factory.

use std::sync::Arc;

use grpc_egress_test_support::{CallFixture, ClientFixture, Fixture};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, CompressionMode, GrpcChannelConfig, GrpcChannelConfigError,
    GrpcEgressBuilder, GrpcEgressBuilderFactory, GrpcEgressInterceptor, InboundReply, OutboundCall,
};
use grpc_egress_transport_std::{
    GrpcChannelConfigFactory, GrpcEgressInterceptorChain, TonicGrpcEgressBuilderSpiFactory,
    TraceContextGrpcEgressInterceptor,
};
use proto_pattern::{CallError, CallResponse, Metadata, RawTransport, StatusCode};

fn ensure_rustls_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

/// @covers: GrpcChannelConfig::default — tls_required is `true`.
#[test]
fn transport_struct_channel_config_default_requires_tls_int_test() {
    let cfg = GrpcChannelConfig::default();
    assert!(cfg.tls_required, "TLS-by-default invariant must hold");
}

/// @covers: create_transport_from_config — plaintext endpoint rejected when tls_required.
#[test]
fn transport_struct_channel_config_from_config_rejects_plaintext_int_test() {
    ensure_rustls_provider();
    let cfg = GrpcChannelConfig::new("http://localhost:50051");
    let result = ClientFixture::from_config(&cfg);
    match result {
        Err(GrpcChannelConfigError::PlaintextRejected(endpoint)) => {
            assert!(
                endpoint.contains("localhost"),
                "endpoint in error: {endpoint}"
            );
        }
        Err(other) => panic!("expected PlaintextRejected, got: {other}"),
        Ok(_) => panic!("must reject plaintext when tls_required=true"),
    }
}

/// @covers: create_transport_from_config — plaintext accepted with allow_plaintext().
#[tokio::test]
async fn transport_struct_channel_config_from_config_accepts_plaintext_with_opt_in_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    Fixture::ensure_tls_provider();
    let closed = Fixture::reserve_closed_port()?;
    let cfg = GrpcChannelConfig::new(closed.get_endpoint()).allow_plaintext();
    let transport = ClientFixture::from_config(&cfg)?;
    // The endpoint is a closed port taken from the operating system, so a real
    // call must genuinely fail — proves this is a connectable client, not a stub.
    let result = transport
        .call_raw(
            CallFixture::path("pkg.Service/Method"),
            CallFixture::request(b"ping", std::time::Duration::from_secs(2)),
        )
        .await
        .map(CallResponse::into_message);
    assert!(
        matches!(
            result,
            Err(CallError::Transport {
                code: StatusCode::Unavailable,
                ..
            })
        ),
        "a call to an unbound port must report Unavailable, got: {result:?}"
    );
    Ok(())
}

/// @covers: create_transport_from_config — https endpoint accepted, unlike
/// plaintext without opt-in — proves tls_required is genuinely scheme-derived, not ignored.
#[test]
fn transport_struct_channel_config_from_config_accepts_https_int_test() {
    ensure_rustls_provider();
    let https_cfg = GrpcChannelConfig::new("https://example.com:443");
    assert!(ClientFixture::from_config(&https_cfg).is_ok());

    let plaintext_cfg = GrpcChannelConfig::new("http://example.com:80");
    assert!(
        matches!(
            ClientFixture::from_config(&plaintext_cfg),
            Err(GrpcChannelConfigError::PlaintextRejected(_))
        ),
        "same construction path must still reject plaintext without allow_plaintext()"
    );
}

/// @covers: GrpcEgressInterceptorChain — accepts a TraceContextGrpcEgressInterceptor.
#[test]
fn transport_struct_channel_config_with_interceptors_accepts_chain_int_test() {
    let chain = GrpcEgressInterceptorChain::new()
        .push(Arc::new(TraceContextGrpcEgressInterceptor::pass_through()));
    assert_eq!(chain.len(), 1);
}

/// @covers: GrpcEgressBuilder::with_interceptor — before_call short-circuits on first failure.
#[tokio::test]
async fn transport_struct_channel_config_interceptor_short_circuits_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Duration;

    ensure_rustls_provider();

    struct Deny;
    impl GrpcEgressInterceptor for Deny {
        fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
            Err(CallError::Status {
                path: call.get_path().into(),
                code: StatusCode::PermissionDenied,
                message: "denied".to_string(),
                details: Vec::new(),
                trailers: Metadata::default(),
            })
        }
        fn after_call(&self, _: &InboundReply<'_>) -> Result<(), CallError> {
            Ok(())
        }
    }

    // The endpoint is a closed port: a call that was not stopped by the interceptor would fail with a
    // transport error, so the error below can only come from the interceptor.
    let closed = Fixture::reserve_closed_port()?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(closed.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_interceptor(Arc::new(Deny))
        .build()?;

    let result = client
        .call_raw(
            CallFixture::path("svc/Method"),
            CallFixture::request(&[1, 2, 3], Duration::from_secs(1)),
        )
        .await
        .map(CallResponse::into_message);

    match result {
        Err(CallError::Status {
            code: StatusCode::PermissionDenied,
            message,
            ..
        }) => assert_eq!(message, "denied"),
        other => panic!("expected PermissionDenied; got {other:?}"),
    }
    Ok(())
}

/// @covers: GrpcChannelConfig::with_compression — sets mode.
#[test]
fn transport_struct_channel_config_with_compression_overrides_mode_int_test() {
    let cfg = GrpcChannelConfig::new("http://localhost:0").with_compression(CompressionMode::Gzip);
    assert_eq!(cfg.compression, CompressionMode::Gzip);
}
