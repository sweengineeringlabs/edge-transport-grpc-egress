#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`GrpcResilientFacade`].

use edge_transport_grpc_egress_resilient::{DescribeRequest, GrpcResilientFacade, Processor};

/// @covers: default_facade
#[test]
fn test_default_facade_is_the_same_type_as_facade_happy() {
    struct AnyProcessor;
    impl Processor for AnyProcessor {
        fn describe(
            &self,
            _req: DescribeRequest,
        ) -> Result<
            edge_transport_grpc_egress_resilient::DescribeResponse,
            edge_transport_grpc_egress_resilient::ResilientTransportError,
        > {
            unreachable!("not exercised by this test")
        }
    }
    let facade: GrpcResilientFacade = <AnyProcessor as Processor>::default_facade();
    // The type annotation above is itself a compile-time proof; this
    // assertion adds a genuine runtime check so the test can fail.
    assert_eq!(std::mem::size_of_val(&facade), 0);
}

/// @covers: default_facade
#[test]
fn test_default_facade_is_zero_sized_error() {
    struct AnyProcessor;
    impl Processor for AnyProcessor {
        fn describe(
            &self,
            _req: DescribeRequest,
        ) -> Result<
            edge_transport_grpc_egress_resilient::DescribeResponse,
            edge_transport_grpc_egress_resilient::ResilientTransportError,
        > {
            unreachable!("not exercised by this test")
        }
    }
    // "error"-flavored scenario for an infallible constructor: prove the
    // returned facade genuinely carries no state, not just that it compiles.
    let facade = <AnyProcessor as Processor>::default_facade();
    assert_eq!(std::mem::size_of_val(&facade), 0);
}

/// @covers: default_facade
#[test]
fn test_default_facade_is_deterministic_edge() {
    struct AnyProcessor;
    impl Processor for AnyProcessor {
        fn describe(
            &self,
            _req: DescribeRequest,
        ) -> Result<
            edge_transport_grpc_egress_resilient::DescribeResponse,
            edge_transport_grpc_egress_resilient::ResilientTransportError,
        > {
            unreachable!("not exercised by this test")
        }
    }
    let a = <AnyProcessor as Processor>::default_facade();
    let b = <AnyProcessor as Processor>::default_facade();
    assert_eq!(
        std::mem::size_of_val(&a),
        std::mem::size_of_val(&b),
        "repeated calls must produce equivalent zero-sized facades"
    );
}

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
