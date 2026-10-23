//! Breaker policy construction and deserialization reject unusable values.

use grpc_egress_breaker::GrpcBreakerConfig;

#[test]
fn test_try_new_rejects_zero_failure_threshold() {
    assert!(GrpcBreakerConfig::try_new(true, 0, 10, 1)
        .err()
        .is_some_and(|error| error.to_string().contains("failure_threshold")));
}

#[test]
fn test_try_new_rejects_zero_probe_count() {
    assert!(GrpcBreakerConfig::try_new(true, 1, 10, 0)
        .err()
        .is_some_and(|error| error.to_string().contains("half_open_probe_count")));
}

#[test]
fn test_deserialization_validates_and_preserves_default_enabled(
) -> Result<(), Box<dyn std::error::Error>> {
    let config: GrpcBreakerConfig = toml::from_str(
        "failure_threshold = 3\ncool_down_seconds = 10\nhalf_open_probe_count = 2\n",
    )?;
    assert!(config.enabled());
    assert_eq!(config.failure_threshold(), 3);
    assert!(toml::from_str::<GrpcBreakerConfig>(
        "failure_threshold = 0\ncool_down_seconds = 10\nhalf_open_probe_count = 1\n"
    )
    .err()
    .is_some_and(|error| error.to_string().contains("failure_threshold")));
    Ok(())
}
