#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ValidatorFactory`].

use edge_transport_grpc_egress_retry::{GrpcRetryConfig, ValidationRequest, ValidatorFactory};

/// @covers: create
#[test]
fn test_create_accepts_valid_config_happy() {
    let validator = ValidatorFactory::create();
    let result = validator.validate_config(ValidationRequest {
        config: GrpcRetryConfig::default(),
    });
    assert!(result.is_ok(), "a genuinely valid config must be accepted");
    let invalid = GrpcRetryConfig {
        max_attempts: 0,
        ..GrpcRetryConfig::default()
    };
    let rejected = validator.validate_config(ValidationRequest { config: invalid });
    assert!(rejected.is_err());
}

/// @covers: create
#[test]
fn test_create_rejects_zero_max_attempts_error() {
    let validator = ValidatorFactory::create();
    let cfg = GrpcRetryConfig {
        max_attempts: 0,
        ..GrpcRetryConfig::default()
    };
    let err = validator
        .validate_config(ValidationRequest { config: cfg })
        .expect_err("zero max_attempts must be rejected");
    assert!(!err.to_string().is_empty());
}

/// @covers: create
#[test]
fn test_create_produces_independent_instances_edge() {
    let first = ValidatorFactory::create();
    let second = ValidatorFactory::create();
    let r1 = first.validate_config(ValidationRequest {
        config: GrpcRetryConfig::default(),
    });
    let r2 = second.validate_config(ValidationRequest {
        config: GrpcRetryConfig::default(),
    });
    assert_eq!(r1.is_ok(), r2.is_ok());
}
