//! Integration tests for `LastErrorRequest`.
#![allow(clippy::default_constructed_unit_structs)] // the point of these tests is verifying ::default() matches the literal

use edge_transport_grpc_egress_transport::LastErrorRequest;

/// @covers: LastErrorRequest
#[test]
fn test_last_error_request_default_constructs_happy() {
    let req = LastErrorRequest;
    assert_eq!(format!("{req:?}"), "LastErrorRequest");
}

/// @covers: LastErrorRequest
#[test]
fn test_last_error_request_default_trait_matches_literal_error() {
    assert_eq!(
        format!("{:?}", LastErrorRequest::default()),
        "LastErrorRequest"
    );
}

/// @covers: LastErrorRequest
#[test]
fn test_last_error_request_copy_is_independent_edge() {
    let a = LastErrorRequest;
    let b = a;
    assert_eq!(format!("{a:?}"), "LastErrorRequest");
    assert_eq!(format!("{b:?}"), "LastErrorRequest");
    assert_eq!(std::mem::size_of_val(&a), 0);
}
