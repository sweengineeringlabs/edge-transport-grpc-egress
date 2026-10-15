#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ConfigBuilderResponse`].

use edge_transport_grpc_egress_retry::{ConfigBuilderProvider, ConfigBuilderRequest, GrpcRetrySvc};

/// @covers: ConfigBuilderResponse
#[test]
fn test_config_builder_response_builder_field_is_usable_happy() {
    let resp = GrpcRetrySvc
        .create_config_builder(ConfigBuilderRequest)
        .expect("real provider must succeed");
    resp.builder.build_loader().expect("builder must be usable");
}

/// @covers: ConfigBuilderResponse
#[test]
fn test_config_builder_response_repeated_construction_error() {
    let first = GrpcRetrySvc
        .create_config_builder(ConfigBuilderRequest)
        .expect("first response");
    let second = GrpcRetrySvc
        .create_config_builder(ConfigBuilderRequest)
        .expect("second response");
    first.builder.build_loader().expect("first must build");
    second.builder.build_loader().expect("second must build");
}

/// @covers: ConfigBuilderResponse
#[test]
fn test_config_builder_response_builder_consumed_by_value_edge() {
    let resp = GrpcRetrySvc
        .create_config_builder(ConfigBuilderRequest)
        .expect("real provider must succeed");
    let loader = resp.builder.build_loader().expect("must build");
    #[derive(serde::Deserialize, Default, PartialEq, Debug)]
    struct AbsentSectionProbe {
        marker: bool,
    }
    let err = loader
        .load_section::<AbsentSectionProbe>("retry_response_probe_section_that_does_not_exist")
        .expect_err("no config directory exists in the test environment");
    assert!(err
        .to_string()
        .contains("retry_response_probe_section_that_does_not_exist"));
}
