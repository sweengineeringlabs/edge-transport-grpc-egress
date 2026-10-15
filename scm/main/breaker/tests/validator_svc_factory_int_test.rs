#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ValidatorFactory`].

use edge_transport_grpc_egress_breaker::{
    ConfigValidationRequest, GrpcBreakerConfig, ValidatorFactory,
};

/// @covers: create
#[test]
fn test_create_accepts_valid_config_happy() {
    let validator = ValidatorFactory::create();
    let result = validator.validate(ConfigValidationRequest {
        config: GrpcBreakerConfig {
            failure_threshold: 5,
            cool_down_seconds: 30,
            half_open_probe_count: 1,
        },
    });
    assert!(result.is_ok(), "a genuinely valid config must be accepted");
    // Negative counterpart in the same test — proves this isn't a stub.
    let rejected = validator.validate(ConfigValidationRequest {
        config: GrpcBreakerConfig {
            failure_threshold: 0,
            cool_down_seconds: 30,
            half_open_probe_count: 1,
        },
    });
    assert!(rejected.is_err());
}

/// @covers: create
#[test]
fn test_create_rejects_zero_threshold_error() {
    let validator = ValidatorFactory::create();
    let err = validator
        .validate(ConfigValidationRequest {
            config: GrpcBreakerConfig {
                failure_threshold: 0,
                cool_down_seconds: 30,
                half_open_probe_count: 1,
            },
        })
        .expect_err("zero threshold must be rejected");
    assert!(err.to_string().contains("failure_threshold") || err.to_string().contains("threshold"));
}

/// @covers: create
#[test]
fn test_create_produces_independent_instances_edge() {
    let first = ValidatorFactory::create();
    let second = ValidatorFactory::create();
    let cfg = GrpcBreakerConfig {
        failure_threshold: 3,
        cool_down_seconds: 10,
        half_open_probe_count: 1,
    };
    let r1 = first.validate(ConfigValidationRequest {
        config: cfg.clone(),
    });
    let r2 = second.validate(ConfigValidationRequest { config: cfg });
    assert_eq!(r1.is_ok(), r2.is_ok());
}
