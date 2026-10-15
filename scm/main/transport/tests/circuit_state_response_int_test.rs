//! Integration tests for `CircuitStateResponse`.

use edge_transport_grpc_egress_transport::CircuitStateResponse;

/// @covers: CircuitStateResponse
#[test]
fn test_circuit_state_response_carries_state_label_happy() {
    let resp = CircuitStateResponse { state: "Closed" };
    assert_eq!(resp.state, "Closed");
}

/// @covers: CircuitStateResponse
#[test]
fn test_circuit_state_response_distinguishes_labels_error() {
    let closed = CircuitStateResponse { state: "Closed" };
    let open = CircuitStateResponse { state: "Open" };
    assert_ne!(closed, open);
}

/// @covers: CircuitStateResponse
#[test]
fn test_circuit_state_response_half_open_edge() {
    let resp = CircuitStateResponse { state: "HalfOpen" };
    assert_eq!(resp, CircuitStateResponse { state: "HalfOpen" });
}
