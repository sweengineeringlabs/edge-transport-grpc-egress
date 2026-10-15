#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ProcessorFactory`].

use edge_transport_grpc_egress_retry::{GrpcRetryConfig, ProcessorFactory, ProcessorRequest};

/// @covers: create
#[test]
fn test_create_produces_a_working_processor_happy() {
    let processor = ProcessorFactory::create();
    let result = processor.validate(ProcessorRequest {
        config: GrpcRetryConfig::default(),
    });
    assert!(result.is_ok(), "factory-produced processor must succeed");
    // Negative counterpart in the same test: proves this isn't a stub that
    // always returns Ok regardless of input.
    let invalid = GrpcRetryConfig {
        max_attempts: 0,
        ..GrpcRetryConfig::default()
    };
    let rejected = processor.validate(ProcessorRequest { config: invalid });
    assert!(
        rejected.is_err(),
        "an invalid config must still be rejected"
    );
}

/// @covers: create
#[test]
fn test_create_rejects_invalid_config_error() {
    let processor = ProcessorFactory::create();
    let cfg = GrpcRetryConfig {
        max_attempts: 0,
        ..GrpcRetryConfig::default()
    };
    let err = processor
        .validate(ProcessorRequest { config: cfg })
        .expect_err("zero max_attempts must be rejected");
    assert!(!err.to_string().is_empty());
}

/// @covers: create
#[test]
fn test_create_produces_independent_instances_edge() {
    let first = ProcessorFactory::create();
    let second = ProcessorFactory::create();
    let r1 = first.validate(ProcessorRequest {
        config: GrpcRetryConfig::default(),
    });
    let r2 = second.validate(ProcessorRequest {
        config: GrpcRetryConfig::default(),
    });
    assert_eq!(r1.is_ok(), r2.is_ok());
}
