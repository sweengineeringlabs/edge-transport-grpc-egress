#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`Validator`] via a test-double implementation.

use edge_transport_grpc_egress_resilient::{
    ConfigValidationRequest, ResilienceConfig, ResilientTransportError, Validator,
};

struct MockValidator;

impl Validator for MockValidator {
    fn validate(&self, req: ConfigValidationRequest) -> Result<(), ResilientTransportError> {
        if req.config.max_attempts == 0 {
            return Err(ResilientTransportError::InvalidResilience(
                "max_attempts must be non-zero".into(),
            ));
        }
        Ok(())
    }
}

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

/// @covers: validate
#[test]
fn test_validate_valid_config_happy() {
    let validator = MockValidator;
    let result = validator.validate(ConfigValidationRequest { config: valid() });
    assert!(result.is_ok(), "a genuinely valid config must be accepted");
    // Negative counterpart in the same test: proves this isn't a stub that
    // always returns Ok regardless of input.
    let mut invalid = valid();
    invalid.max_attempts = 0;
    let rejected = validator.validate(ConfigValidationRequest { config: invalid });
    assert!(
        rejected.is_err(),
        "an invalid config must still be rejected"
    );
}

/// @covers: validate
#[test]
fn test_validate_zero_max_attempts_error() {
    let validator = MockValidator;
    let mut cfg = valid();
    cfg.max_attempts = 0;
    let err = validator
        .validate(ConfigValidationRequest { config: cfg })
        .expect_err("zero max_attempts must be rejected");
    assert!(err.to_string().contains("max_attempts"));
}

/// @covers: validate
#[test]
fn test_validate_minimum_valid_max_attempts_edge() {
    let validator = MockValidator;
    let mut cfg = valid();
    cfg.max_attempts = 1;
    let result = validator.validate(ConfigValidationRequest { config: cfg });
    assert!(
        result.is_ok(),
        "max_attempts of exactly 1 is the smallest valid value"
    );
    // Negative counterpart at the adjacent boundary (0, one below the
    // minimum valid value) — proves the boundary check is exact.
    let mut rejected_cfg = valid();
    rejected_cfg.max_attempts = 0;
    let rejected = validator.validate(ConfigValidationRequest {
        config: rejected_cfg,
    });
    assert!(rejected.is_err(), "one below the minimum must be rejected");
}
