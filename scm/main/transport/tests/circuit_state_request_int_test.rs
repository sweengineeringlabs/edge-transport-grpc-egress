//! Integration tests for `CircuitStateRequest`.
#![allow(clippy::default_constructed_unit_structs)] // the point of these tests is verifying ::default() matches the literal

use edge_transport_grpc_egress_transport::CircuitStateRequest;

/// @covers: CircuitStateRequest
#[test]
fn test_circuit_state_request_default_constructs_happy() {
    let req = CircuitStateRequest;
    assert_eq!(format!("{req:?}"), "CircuitStateRequest");
}

/// @covers: CircuitStateRequest
#[test]
fn test_circuit_state_request_default_trait_matches_literal_error() {
    // If `Default` ever diverged from the literal unit value (e.g. a future
    // field added without updating `Default`), this would no longer match
    // the expected Debug output.
    assert_eq!(
        format!("{:?}", CircuitStateRequest::default()),
        "CircuitStateRequest"
    );
}

/// @covers: CircuitStateRequest
#[test]
fn test_circuit_state_request_copy_is_independent_edge() {
    let a = CircuitStateRequest;
    let b = a;
    // `Copy` means using `a` after `b` is valid — proves it wasn't moved.
    assert_eq!(format!("{a:?}"), "CircuitStateRequest");
    assert_eq!(format!("{b:?}"), "CircuitStateRequest");
    assert_eq!(std::mem::size_of_val(&a), 0);
}
