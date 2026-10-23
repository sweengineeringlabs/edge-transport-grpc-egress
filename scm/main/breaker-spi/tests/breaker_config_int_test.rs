//! Tests for [`GrpcBreakerConfig`].

use grpc_egress_breaker::GrpcBreakerConfigProvider as _;
use std::time::Duration;

use grpc_egress_breaker::GrpcBreakerConfig;
use swe_edge_configbuilder::ConfigSection as _;

/// @covers: from_config
#[test]
fn test_from_config_parses_full_toml() -> Result<(), Box<dyn std::error::Error>> {
    let toml = r#"
        failure_threshold = 5
        cool_down_seconds = 30
        half_open_probe_count = 1
    "#;
    let cfg = breaker_provider().parse_config(toml)?;
    assert_eq!(cfg.failure_threshold(), 5);
    assert_eq!(cfg.cool_down_seconds(), 30);
    assert_eq!(cfg.half_open_probe_count(), 1);
    Ok(())
}

/// @covers: from_config
#[test]
fn test_from_config_unknown_key_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let toml = r#"
        failure_threshold = 5
        cool_down_seconds = 30
        half_open_probe_count = 1
        unknown = 99
    "#;
    let err = breaker_provider()
        .parse_config(toml)
        .err()
        .ok_or("unknown field must fail")?;
    let s = err.to_string();
    assert!(
        s.contains("unknown") || s.contains("unknown field"),
        "expected error to name unknown field, got: {s}",
    );
    Ok(())
}

/// @covers: from_config
#[test]
fn test_zero_threshold_is_invalid() {
    let toml = r#"
        failure_threshold = 0
        cool_down_seconds = 30
        half_open_probe_count = 1
    "#;
    let result = breaker_provider().parse_config(toml);
    assert!(result.is_err(), "zero threshold must fail");
}

/// @covers: from_config
#[test]
fn test_zero_probe_count_is_invalid() {
    let toml = r#"
        failure_threshold = 5
        cool_down_seconds = 30
        half_open_probe_count = 0
    "#;
    let result = breaker_provider().parse_config(toml);
    assert!(result.is_err(), "zero probe count must fail");
}

/// @covers: default
#[test]
fn test_default_config_has_valid_positive_fields() {
    let cfg = GrpcBreakerConfig::default();
    assert!(cfg.failure_threshold() >= 1);
    assert!(cfg.half_open_probe_count() >= 1);
}

/// @covers: section_name
#[test]
fn test_section_name_is_grpc_breaker() {
    assert_eq!(GrpcBreakerConfig::section_name(), "grpc_breaker");
}

/// @covers: cool_down
#[test]
fn test_cool_down_returns_duration_from_seconds() {
    let cfg = GrpcBreakerConfig::default();
    assert_eq!(
        breaker_provider().cool_down(&cfg),
        Duration::from_secs(cfg.cool_down_seconds())
    );
}

#[test]
fn test_validate_accepts_the_default_policy() {
    assert!(breaker_provider()
        .validate(&GrpcBreakerConfig::default())
        .is_ok());
}

fn breaker_provider() -> impl grpc_egress_breaker::GrpcBreakerConfigProvider {
    use grpc_egress_breaker::GrpcBreakerConfigProviderFactory;
    grpc_egress_breaker_spi::StdGrpcBreakerConfigProviderFactory
        .build(grpc_egress_breaker::BuildGrpcBreakerConfigProviderRequest)
}
