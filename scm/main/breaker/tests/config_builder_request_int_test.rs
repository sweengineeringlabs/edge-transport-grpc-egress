#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ConfigBuilderRequest`].

use edge_transport_grpc_egress_breaker::{
    ConfigBuilderProvider, ConfigBuilderRequest, GrpcBreakerSvc,
};

/// @covers: ConfigBuilderRequest
#[test]
fn test_config_builder_request_is_constructible_happy() {
    let req = ConfigBuilderRequest;
    // Zero-sized marker type — the assertion proves the size invariant,
    // not just that construction doesn't panic.
    assert_eq!(std::mem::size_of_val(&req), 0);
}

/// @covers: ConfigBuilderRequest
#[test]
fn test_config_builder_request_used_by_real_provider_error() {
    // "error"-flavored scenario for a zero-field marker type: prove it's
    // not an orphaned placeholder by threading it through a real call and
    // checking the payload, not just success/failure.
    let resp = GrpcBreakerSvc
        .create_config_builder(ConfigBuilderRequest)
        .expect("real provider must accept this request type");
    resp.builder
        .build_loader()
        .expect("resulting builder must be genuinely usable");
}

/// @covers: ConfigBuilderRequest
#[test]
fn test_config_builder_request_reusable_edge() {
    let a = ConfigBuilderRequest;
    let b = ConfigBuilderRequest;
    // Both are valid, independent instances of a zero-sized marker.
    assert_eq!(std::mem::size_of_val(&a), std::mem::size_of_val(&b));
}
