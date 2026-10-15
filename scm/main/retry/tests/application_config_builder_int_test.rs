#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Coverage stub for `src/api/types/application_config_builder.rs`.

use edge_transport_grpc_egress_retry::GrpcRetryFacade;

#[derive(serde::Deserialize, Default, PartialEq, Debug)]
struct AbsentSectionProbe {
    marker: bool,
}

/// @covers: create_config_builder
#[test]
fn test_create_config_builder_returns_a_working_loader_happy() {
    let loader = GrpcRetryFacade::create_config_builder()
        .expect("infallible")
        .build_loader()
        .expect("a builder pre-seeded with name and version must build a valid loader");
    // In a test environment there is no application.toml at any configured
    // directory, so querying any section must fail with NotFound — proves
    // the loader is genuinely wired to the filesystem, not a no-op stub.
    let err = loader
        .load_section::<AbsentSectionProbe>("retry_test_probe_section_that_does_not_exist")
        .expect_err("no config directory exists in the test environment");
    assert!(
        err.to_string()
            .contains("retry_test_probe_section_that_does_not_exist"),
        "error must name the missing section, got: {err}"
    );
}

/// @covers: create_config_builder
#[test]
fn test_create_config_builder_different_section_name_reports_that_name_edge() {
    let loader = GrpcRetryFacade::create_config_builder()
        .expect("infallible")
        .build_loader()
        .expect("a builder pre-seeded with name and version must build a valid loader");
    // A second, differently-named absent section proves the loader's error
    // genuinely echoes back whatever section name is requested, rather
    // than a single hardcoded message from the happy-path test above.
    let err = loader
        .load_section::<AbsentSectionProbe>("retry_test_probe_section_that_does_not_exist_2")
        .expect_err("no config directory exists in the test environment");
    assert!(err
        .to_string()
        .contains("retry_test_probe_section_that_does_not_exist_2"));
}
