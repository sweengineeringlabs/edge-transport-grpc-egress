//! Rule 120 coverage stub for `src/api/types/grpc/grpc_retry_svc.rs`.

/// @covers: GrpcRetrySvc struct is publicly accessible and is a zero-sized namespace marker
#[test]
fn retry_struct_grpc_retry_svc_is_accessible_int_test() {
    assert_eq!(
        std::mem::size_of::<grpc_egress_retry_std::GrpcRetrySvc>(),
        0,
        "GrpcRetrySvc is a namespace marker for factory fns and must carry no instance state"
    );
}
