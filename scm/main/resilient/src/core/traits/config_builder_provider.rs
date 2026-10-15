//! `impl ConfigBuilderProvider for GrpcResilientSvcProcessor`.

use crate::api::{
    ApplicationConfigBuilder, ConfigBuilderProvider, ConfigBuilderRequest, ConfigBuilderResponse,
    GrpcResilientSvcProcessor, ResilientTransportError,
};

impl ConfigBuilderProvider for GrpcResilientSvcProcessor {
    fn create_config_builder(
        &self,
        _req: ConfigBuilderRequest,
    ) -> Result<ConfigBuilderResponse, ResilientTransportError> {
        let mut b = swe_edge_configbuilder::ConfigBuilderImpl::new();
        b = b.with_name(env!("CARGO_PKG_NAME"));
        b = b.with_version(env!("CARGO_PKG_VERSION"));
        Ok(ConfigBuilderResponse {
            builder: ApplicationConfigBuilder(b),
        })
    }
}

impl ApplicationConfigBuilder {
    /// Build a [`swe_edge_configbuilder::SectionLoaderImpl`] from this builder.
    pub fn build_loader(
        self,
    ) -> Result<swe_edge_configbuilder::SectionLoaderImpl, swe_edge_configbuilder::ConfigError>
    {
        self.0.build_loader()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize, Default, PartialEq, Debug)]
    struct ConfigBuilderProviderAbsentSectionProbe {
        marker: bool,
    }

    /// @covers: create_config_builder
    /// @covers: build_loader
    #[test]
    fn test_create_config_builder_build_loader_produces_a_working_loader() {
        let resp = GrpcResilientSvcProcessor
            .create_config_builder(ConfigBuilderRequest)
            .expect("infallible");
        let loader = resp
            .builder
            .build_loader()
            .expect("pre-seeded builder must build a valid loader");
        // In a test environment there is no application.toml at any
        // configured directory, so querying any section must genuinely
        // fail with NotFound — proves the loader is really wired to the
        // filesystem, not a no-op stub.
        let err = loader
            .load_section::<ConfigBuilderProviderAbsentSectionProbe>(
                "resilient_core_probe_section_that_does_not_exist",
            )
            .expect_err("no config directory exists in the test environment");
        assert!(err
            .to_string()
            .contains("resilient_core_probe_section_that_does_not_exist"));
    }

    #[test]
    fn test_default_provider_returns_a_grpc_resilient_svc_marker() {
        let svc = <GrpcResilientSvcProcessor as ConfigBuilderProvider>::default_provider();
        assert_eq!(std::mem::size_of_val(&svc), 0);
    }
}
