//! Integration tests for `GrpcChannelConfigBuilder`.

use grpc_egress_transport::{CompressionMode, KeepAliveConfig, MtlsConfig};
use grpc_egress_transport_std::GrpcChannelConfigBuilder;
use grpc_egress_transport_std::MtlsConfigFactory;
use std::time::Duration;

/// @covers: GrpcChannelConfigBuilder::build — valid endpoint returns Ok
#[test]
fn transport_struct_grpc_channel_config_builder_build_with_endpoint_returns_ok_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = GrpcChannelConfigBuilder::new()
        .with_endpoint("https://example.com:443")
        .build()?;
    assert_eq!(cfg.endpoint, "https://example.com:443");
    Ok(())
}

/// @covers: GrpcChannelConfigBuilder::build — missing endpoint returns Err
#[test]
fn transport_struct_grpc_channel_config_builder_build_without_endpoint_returns_err_int_test() {
    assert!(GrpcChannelConfigBuilder::new().build().is_err());
}

/// @covers: GrpcChannelConfigBuilder::allow_plaintext — disables TLS
#[test]
fn transport_struct_grpc_channel_config_builder_allow_plaintext_disables_tls_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = GrpcChannelConfigBuilder::new()
        .with_endpoint("http://localhost:50051")
        .allow_plaintext()
        .build()?;
    assert!(!cfg.tls_required);
    Ok(())
}

/// @covers: GrpcChannelConfigBuilder::with_mtls — stores mTLS identity
#[test]
fn transport_struct_grpc_channel_config_builder_mtls_stores_client_identity_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let m = MtlsConfig::new("cert.pem", "key.pem");
    let cfg = GrpcChannelConfigBuilder::new()
        .with_endpoint("https://example.com:443")
        .with_mtls(m)
        .build()?;
    assert!(cfg.mtls.is_some());
    Ok(())
}

/// @covers: GrpcChannelConfigBuilder::with_keep_alive — stores config
#[test]
fn transport_struct_grpc_channel_config_builder_keep_alive_stores_config_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let ka = KeepAliveConfig {
        interval: Duration::from_secs(5),
        timeout: Duration::from_secs(10),
        permit_without_calls: true,
    };
    let cfg = GrpcChannelConfigBuilder::new()
        .with_endpoint("https://example.com:443")
        .with_keep_alive(ka)
        .build()?;
    assert!(cfg.keep_alive.is_some());
    Ok(())
}

/// @covers: GrpcChannelConfigBuilder::with_max_message_bytes — overrides default
#[test]
fn transport_struct_grpc_channel_config_builder_max_message_bytes_overrides_default_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = GrpcChannelConfigBuilder::new()
        .with_endpoint("https://example.com:443")
        .with_max_message_bytes(8 * 1024 * 1024)
        .build()?;
    assert_eq!(cfg.max_message_bytes, 8 * 1024 * 1024);
    Ok(())
}

/// @covers: GrpcChannelConfigBuilder::with_compression — sets mode
#[test]
fn transport_struct_grpc_channel_config_builder_compression_sets_mode_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = GrpcChannelConfigBuilder::new()
        .with_endpoint("https://example.com:443")
        .with_compression(CompressionMode::Gzip)
        .build()?;
    assert_eq!(cfg.compression, CompressionMode::Gzip);
    Ok(())
}
