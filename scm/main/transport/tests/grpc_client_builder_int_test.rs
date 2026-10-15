//! Integration tests for `api/client/grpc_client_builder.rs`.

use edge_transport_grpc_egress_transport::GrpcClientBuilder;

/// @covers: GrpcClientBuilder is a zero-sized marker — the interface
/// declaration counterpart for the spi/ tonic builder, not itself state.
#[test]
fn transport_struct_grpc_client_builder_is_constructable_int_test() {
    let _ = GrpcClientBuilder;
    assert_eq!(
        std::mem::size_of::<GrpcClientBuilder>(),
        0,
        "GrpcClientBuilder is a marker type and must carry no instance state"
    );
}
