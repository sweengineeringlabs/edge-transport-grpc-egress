//! Integration tests for the `Validator` trait contract.

use edge_transport_grpc_egress_retry::Validator;

/// @covers: Validator — trait is object-safe
#[test]
fn retry_trait_validator_is_object_safe_int_test() {
    fn _assert(_: &dyn Validator) {}
}
