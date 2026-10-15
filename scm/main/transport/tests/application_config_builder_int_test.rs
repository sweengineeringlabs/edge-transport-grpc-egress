#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Coverage stub for `api/types/application_config_builder.rs`.

use edge_transport_grpc_egress_transport::TransportSvc;
use swe_edge_configbuilder::{ConfigError, SectionLoaderImpl};

#[derive(serde::Deserialize, Default, PartialEq, Debug)]
struct AbsentSectionProbe {
    marker: bool,
}

/// @covers: ApplicationConfigBuilder — wraps a working `swe_edge_configbuilder` loader
#[test]
fn transport_type_application_config_builder_is_accessible_int_test() {
    let loader: SectionLoaderImpl = TransportSvc::create_config_builder()
        .build_loader()
        .expect("a builder pre-seeded with name and version must build a valid loader");
    // In a test environment there is no application.toml at any configured
    // directory, so querying any section must fail with NotFound — proves
    // the loader is genuinely wired to the filesystem, not a no-op stub.
    let err: ConfigError = loader
        .load_section::<AbsentSectionProbe>("transport_test_probe_section_that_does_not_exist")
        .expect_err("no config directory exists in the test environment");
    assert!(
        err.to_string()
            .contains("transport_test_probe_section_that_does_not_exist"),
        "error must name the missing section, got: {err}"
    );
}
