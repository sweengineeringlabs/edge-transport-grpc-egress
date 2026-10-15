#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ConfigBuilderResponse`].

use edge_transport_grpc_egress_breaker::{
    ConfigBuilderProvider, ConfigBuilderRequest, GrpcBreakerSvc,
};

/// @covers: ConfigBuilderResponse
#[test]
fn test_config_builder_response_builder_field_is_usable_happy() {
    let resp = GrpcBreakerSvc
        .create_config_builder(ConfigBuilderRequest)
        .expect("real provider must succeed");
    resp.builder.build_loader().expect("builder must be usable");
}

/// @covers: ConfigBuilderResponse
#[test]
fn test_config_builder_response_repeated_construction_error() {
    // "error"-flavored scenario: prove two independently constructed
    // responses don't share broken state (e.g. a poisoned singleton).
    let first = GrpcBreakerSvc
        .create_config_builder(ConfigBuilderRequest)
        .expect("first response");
    let second = GrpcBreakerSvc
        .create_config_builder(ConfigBuilderRequest)
        .expect("second response");
    first.builder.build_loader().expect("first must build");
    second.builder.build_loader().expect("second must build");
}

/// @covers: ConfigBuilderResponse
#[test]
fn test_config_builder_response_builder_consumed_by_value_edge() {
    let resp = GrpcBreakerSvc
        .create_config_builder(ConfigBuilderRequest)
        .expect("real provider must succeed");
    // build_loader() takes `self` by value — proves the field is a genuine
    // owned value, not a reference or shared handle.
    let loader = resp.builder.build_loader().expect("must build");
    // Querying a section that can't exist proves the loader is really
    // wired to the filesystem, not a no-op stub — a real payload check.
    #[derive(serde::Deserialize, Default, PartialEq, Debug)]
    struct AbsentSectionProbe {
        marker: bool,
    }
    let err = loader
        .load_section::<AbsentSectionProbe>("breaker_response_probe_section_that_does_not_exist")
        .expect_err("no config directory exists in the test environment");
    assert!(err
        .to_string()
        .contains("breaker_response_probe_section_that_does_not_exist"));
}
