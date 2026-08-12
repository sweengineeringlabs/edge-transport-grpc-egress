#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ValidatorFactory`].

use edge_transport_grpc_egress_resilient::{
    ConfigValidationRequest, ResilienceConfig, ValidatorFactory,
};

fn valid() -> ResilienceConfig {
    ResilienceConfig {
        max_attempts: 3,
        initial_backoff_ms: 10,
        backoff_multiplier: 2.0,
        jitter_factor: 0.1,
        max_backoff_ms: 100,
        rate_limit_max_attempts: 2,
        rate_limit_initial_backoff_ms: 10,
        rate_limit_max_backoff_ms: 100,
        failure_threshold: 5,
        cool_down_seconds: 30,
        half_open_probe_count: 1,
    }
}

/// @covers: create
#[test]
fn test_create_accepts_valid_config_happy() {
    let validator = ValidatorFactory::create();
    let result = validator.validate(ConfigValidationRequest { config: valid() });
    assert!(result.is_ok(), "a genuinely valid config must be accepted");
    // Negative counterpart in the same test — proves this isn't a stub.
    let mut invalid = valid();
    invalid.max_attempts = 0;
    let rejected = validator.validate(ConfigValidationRequest { config: invalid });
    assert!(rejected.is_err());
}

/// @covers: create
#[test]
fn test_create_rejects_zero_max_attempts_error() {
    let validator = ValidatorFactory::create();
    let mut cfg = valid();
    cfg.max_attempts = 0;
    let err = validator
        .validate(ConfigValidationRequest { config: cfg })
        .expect_err("zero max_attempts must be rejected");
    assert!(err.to_string().contains("max_attempts"));
}

/// @covers: create
#[test]
fn test_create_produces_independent_instances_edge() {
    let first = ValidatorFactory::create();
    let second = ValidatorFactory::create();
    let r1 = first.validate(ConfigValidationRequest { config: valid() });
    let r2 = second.validate(ConfigValidationRequest { config: valid() });
    assert_eq!(r1.is_ok(), r2.is_ok());
}
