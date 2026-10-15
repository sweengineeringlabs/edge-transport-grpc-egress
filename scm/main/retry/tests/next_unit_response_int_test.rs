//! Integration tests for [`NextUnitResponse`].

use edge_transport_grpc_egress_retry::NextUnitResponse;

/// @covers: NextUnitResponse
#[test]
fn test_next_unit_response_preserves_value_happy() {
    let resp = NextUnitResponse { value: 0.42 };
    assert_eq!(resp.value, 0.42);
}

/// @covers: NextUnitResponse
#[test]
fn test_next_unit_response_zero_is_valid_error() {
    let resp = NextUnitResponse { value: 0.0 };
    assert_eq!(resp.value, 0.0);
}

/// @covers: NextUnitResponse
#[test]
fn test_next_unit_response_equality_edge() {
    let a = NextUnitResponse { value: 0.5 };
    let b = NextUnitResponse { value: 0.5 };
    assert_eq!(a, b);
}
