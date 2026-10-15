//! Integration tests for `api/port/grpc/grpc_egress_error.rs`.

use edge_transport_grpc_egress_transport::{GrpcEgressError, GrpcStatusCode};

#[test]
fn transport_struct_status_variant_carries_code_and_message_int_test() {
    let err = GrpcEgressError::Status(GrpcStatusCode::NotFound, "no such row".into());
    let s = err.to_string();
    assert!(s.contains("NotFound"));
    assert!(s.contains("no such row"));
}

#[test]
fn transport_struct_cancelled_variant_renders_with_reason_int_test() {
    let err = GrpcEgressError::Cancelled("token fired".into());
    assert!(err.to_string().contains("token fired"));
}
