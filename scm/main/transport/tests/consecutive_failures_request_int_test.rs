//! Integration tests for `ConsecutiveFailuresRequest`.
#![allow(clippy::default_constructed_unit_structs)] // the point of these tests is verifying ::default() matches the literal

use edge_transport_grpc_egress_transport::ConsecutiveFailuresRequest;

/// @covers: ConsecutiveFailuresRequest
#[test]
fn test_consecutive_failures_request_default_constructs_happy() {
    let req = ConsecutiveFailuresRequest;
    assert_eq!(format!("{req:?}"), "ConsecutiveFailuresRequest");
}

/// @covers: ConsecutiveFailuresRequest
#[test]
fn test_consecutive_failures_request_default_trait_matches_literal_error() {
    assert_eq!(
        format!("{:?}", ConsecutiveFailuresRequest::default()),
        "ConsecutiveFailuresRequest"
    );
}

/// @covers: ConsecutiveFailuresRequest
#[test]
fn test_consecutive_failures_request_copy_is_independent_edge() {
    let a = ConsecutiveFailuresRequest;
    let b = a;
    assert_eq!(format!("{a:?}"), "ConsecutiveFailuresRequest");
    assert_eq!(format!("{b:?}"), "ConsecutiveFailuresRequest");
    assert_eq!(std::mem::size_of_val(&a), 0);
}
