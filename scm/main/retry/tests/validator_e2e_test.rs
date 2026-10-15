#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`Validator`] via a test-double implementation.

use edge_transport_grpc_egress_retry::{Error, GrpcRetryConfig, ValidationRequest, Validator};

struct MockValidator;

impl Validator for MockValidator {
    fn validate_config(&self, req: ValidationRequest) -> Result<(), Error> {
        if req.config.max_attempts == 0 {
            return Err(Error::InvalidConfig("max_attempts must be non-zero".into()));
        }
        Ok(())
    }
}

/// @covers: validate_config
#[test]
fn test_validate_config_valid_config_happy() {
    let validator = MockValidator;
    let result = validator.validate_config(ValidationRequest {
        config: GrpcRetryConfig::default(),
    });
    assert!(result.is_ok(), "a genuinely valid config must be accepted");
    let invalid = GrpcRetryConfig {
        max_attempts: 0,
        ..GrpcRetryConfig::default()
    };
    let rejected = validator.validate_config(ValidationRequest { config: invalid });
    assert!(
        rejected.is_err(),
        "an invalid config must still be rejected"
    );
}

/// @covers: validate_config
#[test]
fn test_validate_config_zero_max_attempts_error() {
    let validator = MockValidator;
    let cfg = GrpcRetryConfig {
        max_attempts: 0,
        ..GrpcRetryConfig::default()
    };
    let err = validator
        .validate_config(ValidationRequest { config: cfg })
        .expect_err("zero max_attempts must be rejected");
    assert!(err.to_string().contains("max_attempts"));
}

/// @covers: validate_config
#[test]
fn test_validate_config_minimum_valid_max_attempts_edge() {
    let validator = MockValidator;
    let cfg = GrpcRetryConfig {
        max_attempts: 1,
        ..GrpcRetryConfig::default()
    };
    let result = validator.validate_config(ValidationRequest { config: cfg });
    assert!(
        result.is_ok(),
        "max_attempts of exactly 1 is the smallest valid value"
    );
    let rejected_cfg = GrpcRetryConfig {
        max_attempts: 0,
        ..GrpcRetryConfig::default()
    };
    let rejected = validator.validate_config(ValidationRequest {
        config: rejected_cfg,
    });
    assert!(rejected.is_err(), "one below the minimum must be rejected");
}
