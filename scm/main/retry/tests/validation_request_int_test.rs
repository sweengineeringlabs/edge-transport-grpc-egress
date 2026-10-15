#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ValidationRequest`].

use edge_transport_grpc_egress_retry::{GrpcRetryConfig, ValidationRequest, ValidatorFactory};

/// @covers: ValidationRequest
#[test]
fn test_validate_config_request_preserves_config_happy() {
    let req = ValidationRequest {
        config: GrpcRetryConfig::default(),
    };
    assert_eq!(
        req.config.max_attempts,
        GrpcRetryConfig::default().max_attempts
    );
}

/// @covers: ValidationRequest
#[test]
fn test_validate_config_request_used_by_real_validator_error() {
    let validator = ValidatorFactory::create();
    let cfg = GrpcRetryConfig {
        max_attempts: 0,
        ..GrpcRetryConfig::default()
    };
    let err = validator
        .validate_config(ValidationRequest { config: cfg })
        .expect_err("invalid config must be rejected");
    assert!(!err.to_string().is_empty());
}

/// @covers: ValidationRequest
#[test]
fn test_validate_config_request_reusable_edge() {
    let a = ValidationRequest {
        config: GrpcRetryConfig::default(),
    };
    let b = ValidationRequest {
        config: GrpcRetryConfig::default(),
    };
    assert_eq!(a.config.max_attempts, b.config.max_attempts);
}
