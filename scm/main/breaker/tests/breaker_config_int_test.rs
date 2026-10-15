#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for [`GrpcBreakerConfig`].

use std::time::Duration;

use edge_transport_grpc_egress_breaker::GrpcBreakerConfig;
use swe_edge_configbuilder::ConfigSection as _;

/// @covers: from_config
#[test]
fn test_from_config_parses_full_toml() {
    let toml = r#"
        failure_threshold = 5
        cool_down_seconds = 30
        half_open_probe_count = 1
    "#;
    let cfg = GrpcBreakerConfig::from_config(toml).expect("parses");
    assert_eq!(cfg.failure_threshold, 5);
    assert_eq!(cfg.cool_down_seconds, 30);
    assert_eq!(cfg.half_open_probe_count, 1);
}

/// @covers: from_config
#[test]
fn test_from_config_unknown_key_is_error() {
    let toml = r#"
        failure_threshold = 5
        cool_down_seconds = 30
        half_open_probe_count = 1
        unknown = 99
    "#;
    let err = GrpcBreakerConfig::from_config(toml).expect_err("unknown field must fail");
    let s = err.to_string();
    assert!(
        s.contains("unknown") || s.contains("unknown field"),
        "expected error to name unknown field, got: {s}",
    );
}

/// @covers: from_config
#[test]
fn test_zero_threshold_is_invalid() {
    let toml = r#"
        failure_threshold = 0
        cool_down_seconds = 30
        half_open_probe_count = 1
    "#;
    let result = GrpcBreakerConfig::from_config(toml);
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
    let result = GrpcBreakerConfig::from_config(toml);
    assert!(result.is_err(), "zero probe count must fail");
}

/// @covers: default
#[test]
fn test_default_config_has_valid_positive_fields() {
    let cfg = GrpcBreakerConfig::default();
    assert!(cfg.failure_threshold >= 1);
    assert!(cfg.half_open_probe_count >= 1);
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
    assert_eq!(cfg.cool_down(), Duration::from_secs(cfg.cool_down_seconds));
}
