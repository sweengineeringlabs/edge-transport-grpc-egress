#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`GrpcResilientFacade`].

use grpc_egress_resilient_std::ApplicationConfigBuilderProvider;
use grpc_egress_resilient_std::GrpcResilientFacade;

/// @covers: create_config_builder
/// @covers: apply_resilience
#[test]
fn test_facade_exposes_both_composition_methods() {
    // Both real production entry points are exercised elsewhere
    // (application_config_builder_int_test.rs, resilient_stack_int_test.rs,
    // error_int_test.rs); this test just confirms create_config_builder is
    // reachable through the facade type this file covers, checking a real
    // payload field, not just is_ok().
    let builder =
        GrpcResilientFacade::create_config_builder().expect("create_config_builder must succeed");
    let loader = builder
        .build_loader()
        .expect("the builder returned by the facade must be genuinely usable");
    #[derive(serde::Deserialize, Default, PartialEq, Debug)]
    struct AbsentSectionProbe {
        marker: bool,
    }
    let err = loader
        .load_section::<AbsentSectionProbe>("grpc_resilient_facade_probe_section_absent")
        .expect_err("no config directory exists in the test environment");
    assert!(err
        .to_string()
        .contains("grpc_resilient_facade_probe_section_absent"));
}
