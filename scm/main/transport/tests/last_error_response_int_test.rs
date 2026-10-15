//! Integration tests for `LastErrorResponse`.

use edge_transport_grpc_egress_transport::{GrpcEgressError, LastErrorResponse};

/// @covers: LastErrorResponse
#[test]
fn test_last_error_response_none_when_healthy_happy() {
    let resp = LastErrorResponse { error: None };
    assert!(resp.error.is_none());
}

/// @covers: LastErrorResponse
#[test]
fn test_last_error_response_carries_error_variant_error() {
    let resp = LastErrorResponse {
        error: Some(GrpcEgressError::Unavailable("backend down".to_string())),
    };
    match resp.error {
        Some(GrpcEgressError::Unavailable(msg)) => assert_eq!(msg, "backend down"),
        other => panic!("expected Some(Unavailable), got {other:?}"),
    }
}

/// @covers: LastErrorResponse
#[test]
fn test_last_error_response_clone_is_independent_edge() {
    let resp = LastErrorResponse {
        error: Some(GrpcEgressError::Unavailable("backend down".to_string())),
    };
    let cloned = resp.clone();
    match (resp.error, cloned.error) {
        (Some(GrpcEgressError::Unavailable(a)), Some(GrpcEgressError::Unavailable(b))) => {
            assert_eq!(a, b);
        }
        other => panic!("expected both to carry Unavailable, got {other:?}"),
    }
}
