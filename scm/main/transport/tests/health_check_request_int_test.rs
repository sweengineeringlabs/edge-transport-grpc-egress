//! Integration tests for `HealthCheckRequest`.

use edge_transport_grpc_egress_transport::HealthCheckRequest;

/// @covers: HealthCheckRequest
#[test]
fn test_health_check_request_constructs_as_zero_sized_happy() {
    let req = HealthCheckRequest;
    assert_eq!(std::mem::size_of_val(&req), 0);
}

/// @covers: HealthCheckRequest
#[test]
fn test_health_check_request_used_as_a_fn_parameter_error() {
    // `HealthCheckRequest` carries no `Debug`/`Default` — its sole contract
    // is being constructible and passable by value where `GrpcEgress::health_check`
    // expects it. A type mismatch here is a compile error, not a runtime one.
    fn accepts(_req: HealthCheckRequest) -> bool {
        true
    }
    assert!(accepts(HealthCheckRequest));
}

/// @covers: HealthCheckRequest
#[test]
fn test_health_check_request_constructed_repeatedly_edge() {
    let a = HealthCheckRequest;
    let b = HealthCheckRequest;
    assert_eq!(std::mem::size_of_val(&a), std::mem::size_of_val(&b));
}
