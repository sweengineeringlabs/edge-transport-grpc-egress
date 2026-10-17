//! The channel configuration is data: it is read from TOML and written back.

use std::error::Error;

use grpc_egress_transport::{CompressionMode, GrpcChannelConfig};

/// @covers: GrpcChannelConfig
#[test]
fn test_deserialize_channel_config_reads_every_field_of_the_table() -> Result<(), Box<dyn Error>> {
    let text = r#"
        endpoint = "https://api.example.com:443"
        tls_required = true
        max_message_bytes = 1048576
        compression = "Gzip"
        request_timeout_secs = 12
    "#;

    let config: GrpcChannelConfig = toml::from_str(text)?;

    assert_eq!(config.endpoint, "https://api.example.com:443");
    assert!(config.tls_required);
    assert_eq!(config.max_message_bytes, 1_048_576);
    assert_eq!(config.compression, CompressionMode::Gzip);
    assert_eq!(config.request_timeout_secs, Some(12));
    assert!(config.mtls.is_none());
    Ok(())
}

/// @covers: GrpcChannelConfig
#[test]
fn test_serialize_channel_config_round_trips_through_toml() -> Result<(), Box<dyn Error>> {
    let config = GrpcChannelConfig {
        endpoint: "http://127.0.0.1:50051".to_string(),
        tls_required: false,
        max_message_bytes: 2048,
        compression: CompressionMode::Zstd,
        request_timeout_secs: Some(3),
        ..GrpcChannelConfig::default()
    };

    let back: GrpcChannelConfig = toml::from_str(&toml::to_string(&config)?)?;

    assert_eq!(back.endpoint, config.endpoint);
    assert_eq!(back.tls_required, config.tls_required);
    assert_eq!(back.max_message_bytes, config.max_message_bytes);
    assert_eq!(back.compression, config.compression);
    assert_eq!(back.request_timeout_secs, config.request_timeout_secs);
    Ok(())
}

/// @covers: GrpcChannelConfig
#[test]
fn test_deserialize_channel_config_without_an_endpoint_is_refused() {
    let result: Result<GrpcChannelConfig, _> = toml::from_str("tls_required = true\n");

    assert!(result.is_err(), "a channel needs an endpoint");
}
