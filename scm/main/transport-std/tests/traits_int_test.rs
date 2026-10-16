//! Integration tests for the traits of the `client` theme.

use grpc_egress_transport::GrpcEgress;

/// @covers: GrpcEgress is object-safe
#[test]
fn transport_trait_grpc_egress_is_object_safe_int_test() {
    fn _assert(_: &dyn GrpcEgress) {}
}
