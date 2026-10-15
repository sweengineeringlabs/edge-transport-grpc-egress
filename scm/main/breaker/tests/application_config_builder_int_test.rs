#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Coverage stub for `src/api/types/application_config_builder.rs`.

use edge_transport_grpc_egress_breaker::GrpcBreakerFacade;

#[derive(serde::Deserialize, Default, PartialEq, Debug)]
struct AbsentSectionProbe {
    marker: bool,
}

/// @covers: ApplicationConfigBuilder — create_config_builder returns a working loader
#[test]
fn breaker_type_application_config_builder_is_accessible_int_test() {
    let loader = GrpcBreakerFacade::create_config_builder()
        .expect("create_config_builder is infallible")
        .build_loader()
        .expect("a builder pre-seeded with name and version must build a valid loader");
    // In a test environment there is no application.toml at any configured
    // directory, so querying any section must fail with NotFound — proves
    // the loader is genuinely wired to the filesystem, not a no-op stub.
    let err = loader
        .load_section::<AbsentSectionProbe>("breaker_test_probe_section_that_does_not_exist")
        .expect_err("no config directory exists in the test environment");
    assert!(
        err.to_string()
            .contains("breaker_test_probe_section_that_does_not_exist"),
        "error must name the missing section, got: {err}"
    );
}
