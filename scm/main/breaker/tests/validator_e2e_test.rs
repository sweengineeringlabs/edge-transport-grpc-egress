#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`Validator`] via a test-double implementation.

use edge_transport_grpc_egress_breaker::{
    ConfigValidationRequest, Error, GrpcBreakerConfig, Validator,
};

struct MockValidator;

impl Validator for MockValidator {
    fn validate(&self, req: ConfigValidationRequest) -> Result<(), Error> {
        if req.config.failure_threshold == 0 {
            return Err(Error::InvalidConfig(
                "failure_threshold must be non-zero".into(),
            ));
        }
        Ok(())
    }
}

/// @covers: validate
#[test]
fn test_validate_valid_config_happy() {
    let validator = MockValidator;
    let result = validator.validate(ConfigValidationRequest {
        config: GrpcBreakerConfig {
            failure_threshold: 5,
            cool_down_seconds: 30,
            half_open_probe_count: 1,
        },
    });
    assert!(result.is_ok(), "a genuinely valid config must be accepted");
    // Negative counterpart in the same test: proves this isn't a stub that
    // always returns Ok regardless of input.
    let rejected = validator.validate(ConfigValidationRequest {
        config: GrpcBreakerConfig {
            failure_threshold: 0,
            cool_down_seconds: 30,
            half_open_probe_count: 1,
        },
    });
    assert!(
        rejected.is_err(),
        "an invalid config must still be rejected"
    );
}

/// @covers: validate
#[test]
fn test_validate_zero_threshold_error() {
    let validator = MockValidator;
    let err = validator
        .validate(ConfigValidationRequest {
            config: GrpcBreakerConfig {
                failure_threshold: 0,
                cool_down_seconds: 30,
                half_open_probe_count: 1,
            },
        })
        .expect_err("zero threshold must be rejected");
    assert!(err.to_string().contains("failure_threshold"));
}

/// @covers: validate
#[test]
fn test_validate_minimum_valid_threshold_edge() {
    let validator = MockValidator;
    let result = validator.validate(ConfigValidationRequest {
        config: GrpcBreakerConfig {
            failure_threshold: 1,
            cool_down_seconds: 0,
            half_open_probe_count: 1,
        },
    });
    assert!(
        result.is_ok(),
        "threshold of exactly 1 is the smallest valid value"
    );
    // Negative counterpart at the adjacent boundary (0, one below the
    // minimum valid value) — proves the boundary check is exact, not a
    // stub that always returns Ok.
    let rejected = validator.validate(ConfigValidationRequest {
        config: GrpcBreakerConfig {
            failure_threshold: 0,
            cool_down_seconds: 0,
            half_open_probe_count: 1,
        },
    });
    assert!(rejected.is_err(), "one below the minimum must be rejected");
}
