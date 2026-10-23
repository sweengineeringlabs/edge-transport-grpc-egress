//! Integration tests for `GrpcChannelConfigBuilder`.

use grpc_egress_transport::GrpcChannelConfigBuilder as _;
use grpc_egress_transport::{CompressionMode, KeepAliveConfig};
use std::time::Duration;

/// @covers: GrpcChannelConfigBuilder::build — valid endpoint returns Ok
#[test]
fn transport_struct_grpc_channel_config_builder_build_with_endpoint_returns_ok_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = new_config_builder()
        .with_endpoint("https://example.com:443")
        .build()?;
    assert_eq!(cfg.endpoint(), "https://example.com:443");
    Ok(())
}

/// @covers: GrpcChannelConfigBuilder::build — missing endpoint returns Err
#[test]
fn transport_struct_grpc_channel_config_builder_build_without_endpoint_returns_err_int_test() {
    assert!(new_config_builder().build().is_err());
}

/// @covers: GrpcChannelConfigBuilder::allow_plaintext — disables TLS
#[test]
fn transport_struct_grpc_channel_config_builder_allow_plaintext_disables_tls_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = new_config_builder()
        .with_endpoint("http://localhost:50051")
        .allow_plaintext()
        .build()?;
    assert!(!cfg.tls_required());
    Ok(())
}

/// @covers: GrpcChannelConfigBuilder::with_mtls — stores mTLS identity
#[test]
fn transport_struct_grpc_channel_config_builder_mtls_stores_client_identity_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let m = mtls_config("cert.pem", "key.pem")?;
    let cfg = new_config_builder()
        .with_endpoint("https://example.com:443")
        .with_mtls(m)
        .build()?;
    assert!(cfg.mtls().is_some());
    Ok(())
}

/// @covers: GrpcChannelConfigBuilder::with_keep_alive — stores config
#[test]
fn transport_struct_grpc_channel_config_builder_keep_alive_stores_config_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let ka = KeepAliveConfig::try_new(Duration::from_secs(5), Duration::from_secs(10), true)?;
    let cfg = new_config_builder()
        .with_endpoint("https://example.com:443")
        .with_keep_alive(ka)
        .build()?;
    assert!(cfg.keep_alive().is_some());
    Ok(())
}

/// @covers: GrpcChannelConfigBuilder::with_max_message_bytes — overrides default
#[test]
fn transport_struct_grpc_channel_config_builder_max_message_bytes_overrides_default_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = new_config_builder()
        .with_endpoint("https://example.com:443")
        .with_max_message_bytes(8 * 1024 * 1024)
        .build()?;
    assert_eq!(cfg.max_message_bytes(), 8 * 1024 * 1024);
    Ok(())
}

/// @covers: GrpcChannelConfigBuilder::with_compression — sets mode
#[test]
fn transport_struct_grpc_channel_config_builder_compression_sets_mode_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = new_config_builder()
        .with_endpoint("https://example.com:443")
        .with_compression(CompressionMode::Gzip)
        .build()?;
    assert_eq!(cfg.compression(), CompressionMode::Gzip);
    Ok(())
}

/// @covers: GrpcChannelConfigBuilder::with_request_timeout — whole seconds are stored
#[test]
fn transport_struct_grpc_channel_config_builder_request_timeout_stores_whole_seconds_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = new_config_builder()
        .with_endpoint("https://example.com:443")
        .with_request_timeout(Duration::from_secs(45))
        .build()?;
    assert_eq!(cfg.request_timeout_secs(), Some(45));
    Ok(())
}

/// @covers: GrpcChannelConfigBuilder::with_request_timeout — below a second is one second, never zero
#[test]
fn transport_struct_grpc_channel_config_builder_request_timeout_below_a_second_is_one_second_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = new_config_builder()
        .with_endpoint("https://example.com:443")
        .with_request_timeout(Duration::from_millis(400))
        .build()?;
    assert_eq!(cfg.request_timeout_secs(), Some(1));
    Ok(())
}

/// @covers: GrpcChannelConfigBuilder::build — no request timeout set leaves the field unset
#[test]
fn transport_struct_grpc_channel_config_builder_build_without_request_timeout_leaves_it_unset_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = new_config_builder()
        .with_endpoint("https://example.com:443")
        .build()?;
    assert_eq!(cfg.request_timeout_secs(), None);
    assert!(
        cfg.tls_required(),
        "TLS is required until plaintext is allowed"
    );
    assert_eq!(
        cfg.max_message_bytes(),
        grpc_egress_transport::DEFAULT_MAX_MESSAGE_BYTES
    );
    Ok(())
}

fn new_config_builder() -> impl grpc_egress_transport::GrpcChannelConfigBuilder {
    use grpc_egress_transport::GrpcChannelConfigBuilderFactory;
    grpc_egress_transport_std::StdGrpcChannelConfigBuilderFactory
        .build(grpc_egress_transport::BuildGrpcChannelConfigBuilderRequest)
}

fn mtls_config(
    cert: impl Into<String>,
    key: impl Into<String>,
) -> Result<grpc_egress_transport::MtlsConfig, grpc_egress_transport::GrpcChannelConfigError> {
    use grpc_egress_transport::MtlsConfigFactory;
    grpc_egress_transport_std::StdMtlsConfigFactory.build(
        grpc_egress_transport::BuildMtlsConfigRequest {
            cert_pem_path: cert.into(),
            key_pem_path: key.into(),
            ca_pem_path: None,
        },
    )
}
