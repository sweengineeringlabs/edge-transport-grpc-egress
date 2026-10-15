//! Integration tests for [`ClassifyRequest`].

use edge_transport_grpc_egress_breaker::ClassifyRequest;

/// @covers: ClassifyRequest
#[test]
fn test_classify_request_failure_signal_happy() {
    let req = ClassifyRequest {
        is_breaker_failure: true,
    };
    assert!(req.is_breaker_failure);
}

/// @covers: ClassifyRequest
#[test]
fn test_classify_request_non_failure_signal_error() {
    let req = ClassifyRequest {
        is_breaker_failure: false,
    };
    assert!(!req.is_breaker_failure);
}

/// @covers: ClassifyRequest
#[test]
fn test_classify_request_default_bool_edge() {
    let req = ClassifyRequest {
        is_breaker_failure: bool::default(),
    };
    assert!(!req.is_breaker_failure, "bool::default() is false");
}
