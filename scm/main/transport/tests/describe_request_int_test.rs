//! Integration tests for `DescribeRequest`.
#![allow(clippy::default_constructed_unit_structs)] // the point of these tests is verifying ::default() matches the literal

use edge_transport_grpc_egress_transport::DescribeRequest;

/// @covers: DescribeRequest
#[test]
fn test_describe_request_default_constructs_happy() {
    let req = DescribeRequest;
    assert_eq!(format!("{req:?}"), "DescribeRequest");
}

/// @covers: DescribeRequest
#[test]
fn test_describe_request_default_trait_matches_literal_error() {
    assert_eq!(
        format!("{:?}", DescribeRequest::default()),
        "DescribeRequest"
    );
}

/// @covers: DescribeRequest
#[test]
fn test_describe_request_copy_is_independent_edge() {
    let a = DescribeRequest;
    let b = a;
    assert_eq!(format!("{a:?}"), "DescribeRequest");
    assert_eq!(format!("{b:?}"), "DescribeRequest");
    assert_eq!(std::mem::size_of_val(&a), 0);
}
