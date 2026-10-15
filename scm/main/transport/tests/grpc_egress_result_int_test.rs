//! Integration tests for `api/port/grpc/grpc_egress_result.rs`.

use edge_transport_grpc_egress_transport::{GrpcEgressError, GrpcEgressResult, GrpcStatusCode};

#[test]
fn transport_trait_grpc_egress_result_ok_variant_is_constructable_int_test() {
    let r: GrpcEgressResult<u32> = Ok(42);
    let Ok(v) = r else { panic!("expected Ok") };
    assert_eq!(v, 42);
}

#[test]
fn transport_trait_grpc_egress_result_err_variant_carries_error_int_test() {
    let r: GrpcEgressResult<u32> = Err(GrpcEgressError::Status(
        GrpcStatusCode::NotFound,
        "gone".into(),
    ));
    assert!(r.is_err());
}
