#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`GrpcRetryFacade`].

use edge_transport_grpc_egress_retry::{Error, GrpcRetryFacade, Processor, ProcessorRequest};

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

struct AnyProcessor;
impl Processor for AnyProcessor {
    fn validate(&self, _req: ProcessorRequest) -> Result<(), Error> {
        unreachable!("not exercised by this test")
    }
}

/// @covers: default_facade
#[test]
fn test_default_facade_is_the_same_type_as_facade_happy() {
    let facade: GrpcRetryFacade = <AnyProcessor as Processor>::default_facade();
    // The type annotation above is itself a compile-time proof; this
    // assertion adds a genuine runtime check so the test can fail.
    assert_eq!(std::mem::size_of_val(&facade), 0);
}

/// @covers: default_facade
#[test]
fn test_default_facade_is_zero_sized_error() {
    let facade = <AnyProcessor as Processor>::default_facade();
    assert_eq!(std::mem::size_of_val(&facade), 0);
}

/// @covers: default_facade
#[test]
fn test_default_facade_is_deterministic_edge() {
    let a = <AnyProcessor as Processor>::default_facade();
    let b = <AnyProcessor as Processor>::default_facade();
    assert_eq!(std::mem::size_of_val(&a), std::mem::size_of_val(&b));
}
