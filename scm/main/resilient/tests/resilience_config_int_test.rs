#![allow(clippy::expect_used)]
//! Integration tests for [`ResilienceConfig`] — this crate's own owned
//! resilience policy shape (ADR-004: no longer a newtype over a foreign
//! transport-crate type).

use edge_transport_grpc_egress_resilient::ResilienceConfig;

/// @covers: ResilienceConfig::default
#[test]
fn test_default_is_the_fast_stateless_grpc_profile_happy() {
    let cfg = ResilienceConfig::default();
    assert_eq!(cfg.max_attempts, 5);
    assert_eq!(cfg.failure_threshold, 5);
}

/// @covers: ResilienceConfig
#[test]
fn test_zero_max_attempts_is_preserved_error() {
    let cfg = ResilienceConfig {
        max_attempts: 0,
        ..ResilienceConfig::default()
    };
    // The struct itself does not validate on construction — proving it
    // faithfully preserves an invalid value (not silently correcting it)
    // confirms field access isn't a stub.
    assert_eq!(cfg.max_attempts, 0);
}

/// @covers: ResilienceConfig
#[test]
fn test_clone_is_independent_of_the_original_edge() {
    let original = ResilienceConfig::default();
    let mut cloned = original.clone();
    cloned.failure_threshold = 99;
    assert_ne!(original.failure_threshold, cloned.failure_threshold);
}

/// @covers: ResilienceConfig
#[test]
fn test_toml_round_trip_preserves_fields_happy() {
    let cfg = ResilienceConfig {
        max_attempts: 3,
        initial_backoff_ms: 10,
        backoff_multiplier: 2.0,
        jitter_factor: 0.1,
        max_backoff_ms: 100,
        rate_limit_max_attempts: 2,
        rate_limit_initial_backoff_ms: 10,
        rate_limit_max_backoff_ms: 100,
        failure_threshold: 7,
        cool_down_seconds: 10,
        half_open_probe_count: 2,
    };
    let toml_text = toml::to_string(&cfg).expect("serialize");
    let round_tripped: ResilienceConfig = toml::from_str(&toml_text).expect("deserialize");
    assert_eq!(round_tripped.failure_threshold, 7);
    assert_eq!(round_tripped.max_attempts, 3);
}
