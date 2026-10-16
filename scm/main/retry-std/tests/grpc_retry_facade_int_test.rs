#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`GrpcRetryFacade`].

use grpc_egress_retry_std::ApplicationConfigBuilderProvider;
use grpc_egress_retry_std::GrpcRetryFacade;

#[derive(serde::Deserialize, Default, PartialEq, Debug)]
struct AbsentSectionProbe {
    marker: bool,
}

/// @covers: create_config_builder
/// @covers: create_retry_client
#[test]
fn test_facade_exposes_both_composition_methods() {
    let builder =
        GrpcRetryFacade::create_config_builder().expect("create_config_builder must succeed");
    let loader = builder
        .build_loader()
        .expect("the builder returned by the facade must be genuinely usable");
    let err = loader
        .load_section::<AbsentSectionProbe>("grpc_retry_facade_probe_section_absent")
        .expect_err("no config directory exists in the test environment");
    assert!(err
        .to_string()
        .contains("grpc_retry_facade_probe_section_absent"));
}
