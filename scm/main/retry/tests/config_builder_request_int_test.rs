#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ConfigBuilderRequest`].

use edge_transport_grpc_egress_retry::{ConfigBuilderProvider, ConfigBuilderRequest, GrpcRetrySvc};

/// @covers: ConfigBuilderRequest
#[test]
fn test_config_builder_request_is_constructible_happy() {
    let req = ConfigBuilderRequest;
    assert_eq!(std::mem::size_of_val(&req), 0);
}

/// @covers: ConfigBuilderRequest
#[test]
fn test_config_builder_request_used_by_real_provider_error() {
    let resp = GrpcRetrySvc
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
    assert_eq!(std::mem::size_of_val(&a), std::mem::size_of_val(&b));
}
