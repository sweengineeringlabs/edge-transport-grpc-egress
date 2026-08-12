//! Integration tests for the `traits` module (`Processor`).

use edge_transport_grpc_egress_transport::{GrpcEgress, Processor};

/// @covers: GrpcEgress is object-safe
#[test]
fn transport_trait_grpc_egress_is_object_safe_int_test() {
    fn _assert(_: &dyn GrpcEgress) {}
}

/// @covers: Processor is object-safe
#[test]
fn transport_trait_processor_is_object_safe_int_test() {
    fn _assert(_: &dyn Processor) {}
}
