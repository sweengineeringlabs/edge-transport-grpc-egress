#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ConfigBuilderProviderFactory`].

use edge_transport_grpc_egress_retry::{ConfigBuilderProviderFactory, ConfigBuilderRequest};

/// @covers: create
#[test]
fn test_create_produces_a_working_provider_happy() {
    let provider = ConfigBuilderProviderFactory::create();
    let resp = provider
        .create_config_builder(ConfigBuilderRequest)
        .expect("factory-produced provider must succeed");
    resp.builder
        .build_loader()
        .expect("must build a real loader");
}

/// @covers: create
#[test]
fn test_create_produces_independent_instances_error() {
    let first = ConfigBuilderProviderFactory::create();
    let second = ConfigBuilderProviderFactory::create();
    let resp1 = first
        .create_config_builder(ConfigBuilderRequest)
        .expect("first must succeed");
    let resp2 = second
        .create_config_builder(ConfigBuilderRequest)
        .expect("second must succeed");
    resp1.builder.build_loader().expect("first must build");
    resp2.builder.build_loader().expect("second must build");
}

/// @covers: create
#[test]
fn test_create_repeated_requests_edge() {
    let provider = ConfigBuilderProviderFactory::create();
    let resp1 = provider
        .create_config_builder(ConfigBuilderRequest)
        .expect("first request");
    let resp2 = provider
        .create_config_builder(ConfigBuilderRequest)
        .expect("second request on same provider instance");
    resp1.builder.build_loader().expect("first must build");
    resp2.builder.build_loader().expect("second must build");
}
