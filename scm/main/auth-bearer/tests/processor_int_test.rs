//! Integration tests for the `Processor` trait contract on `BearerEgressInterceptor`.

use edge_transport_grpc_egress_auth_bearer::{
    BearerEgressConfig, BearerEgressInterceptor, BearerSecret, Processor,
};

fn test_config() -> BearerEgressConfig {
    BearerEgressConfig {
        secret: BearerSecret::Hs256 {
            secret: b"test-secret-key".to_vec(),
        },
        issuer: "test-issuer".into(),
        audience: "test-audience".into(),
        subject: "test-subject".into(),
        lifetime_seconds: 3600,
    }
}

/// @covers: Processor — BearerEgressInterceptor satisfies marker trait
#[test]
fn bearer_trait_processor_is_satisfied_by_interceptor_int_test() {
    fn _assert<T: Processor>(_: T) {}
    let interceptor = BearerEgressInterceptor::from_config(test_config());
    _assert(interceptor);
}

/// @covers: Processor — trait is object-safe
#[test]
fn bearer_trait_processor_is_object_safe_int_test() {
    fn _assert(_: &dyn Processor) {}
}
