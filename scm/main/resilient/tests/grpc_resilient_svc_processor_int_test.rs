//! Rule 120 coverage stub for `src/api/types/grpc_resilient_svc.rs`.

/// @covers: GrpcResilientSvcProcessor struct is publicly accessible and is a zero-sized namespace marker
#[test]
fn resilient_struct_grpc_resilient_svc_is_accessible_int_test() {
    assert_eq!(
        std::mem::size_of::<edge_transport_grpc_egress_resilient::GrpcResilientSvcProcessor>(),
        0,
        "GrpcResilientSvcProcessor is a namespace marker for factory fns and must carry no instance state"
    );
}
