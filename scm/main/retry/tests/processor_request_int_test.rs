#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ProcessorRequest`].

use edge_transport_grpc_egress_retry::{GrpcRetryConfig, ProcessorFactory, ProcessorRequest};

/// @covers: ProcessorRequest
#[test]
fn test_process_request_preserves_config_happy() {
    let req = ProcessorRequest {
        config: GrpcRetryConfig::default(),
    };
    assert_eq!(
        req.config.max_attempts,
        GrpcRetryConfig::default().max_attempts
    );
}

/// @covers: ProcessorRequest
#[test]
fn test_process_request_used_by_real_processor_error() {
    let processor = ProcessorFactory::create();
    let cfg = GrpcRetryConfig {
        max_attempts: 0,
        ..GrpcRetryConfig::default()
    };
    let err = processor
        .validate(ProcessorRequest { config: cfg })
        .expect_err("invalid config must be rejected");
    assert!(!err.to_string().is_empty());
}

/// @covers: ProcessorRequest
#[test]
fn test_process_request_reusable_edge() {
    let a = ProcessorRequest {
        config: GrpcRetryConfig::default(),
    };
    let b = ProcessorRequest {
        config: GrpcRetryConfig::default(),
    };
    assert_eq!(a.config.max_attempts, b.config.max_attempts);
}
