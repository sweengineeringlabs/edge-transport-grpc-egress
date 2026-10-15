//! Integration tests for `ProcessingRequest`.
#![allow(clippy::default_constructed_unit_structs)] // the point of these tests is verifying ::default() matches the literal

use edge_transport_grpc_egress_transport::ProcessingRequest;

/// @covers: ProcessingRequest
#[test]
fn test_processing_request_default_constructs_happy() {
    let req = ProcessingRequest;
    assert_eq!(format!("{req:?}"), "ProcessingRequest");
}

/// @covers: ProcessingRequest
#[test]
fn test_processing_request_default_trait_matches_literal_error() {
    assert_eq!(
        format!("{:?}", ProcessingRequest::default()),
        "ProcessingRequest"
    );
}

/// @covers: ProcessingRequest
#[test]
fn test_processing_request_copy_is_independent_edge() {
    let a = ProcessingRequest;
    let b = a;
    assert_eq!(format!("{a:?}"), "ProcessingRequest");
    assert_eq!(format!("{b:?}"), "ProcessingRequest");
    assert_eq!(std::mem::size_of_val(&a), 0);
}
