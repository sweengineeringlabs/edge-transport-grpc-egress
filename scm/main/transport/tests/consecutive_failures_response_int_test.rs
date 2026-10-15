//! Integration tests for `ConsecutiveFailuresResponse`.

use edge_transport_grpc_egress_transport::ConsecutiveFailuresResponse;

/// @covers: ConsecutiveFailuresResponse
#[test]
fn test_consecutive_failures_response_carries_count_happy() {
    let resp = ConsecutiveFailuresResponse { count: 3 };
    assert_eq!(resp.count, 3);
}

/// @covers: ConsecutiveFailuresResponse
#[test]
fn test_consecutive_failures_response_distinguishes_counts_error() {
    let a = ConsecutiveFailuresResponse { count: 0 };
    let b = ConsecutiveFailuresResponse { count: 1 };
    assert_ne!(a, b);
}

/// @covers: ConsecutiveFailuresResponse
#[test]
fn test_consecutive_failures_response_max_boundary_edge() {
    let resp = ConsecutiveFailuresResponse { count: u32::MAX };
    assert_eq!(resp.count, u32::MAX);
}
