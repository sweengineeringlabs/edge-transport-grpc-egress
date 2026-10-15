//! `impl` block for [`ApplicationConfigBuilder`]. The type *declaration* lives in `api/`.

use crate::api::ApplicationConfigBuilder;

impl Default for ApplicationConfigBuilder {
    fn default() -> Self {
        Self(swe_edge_configbuilder::ConfigBuilderImpl::new())
    }
}

impl ApplicationConfigBuilder {
    /// Set the crate name reported by the built loader.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.0 = self.0.with_name(name);
        self
    }

    /// Set the crate version reported by the built loader.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.0 = self.0.with_version(version);
        self
    }

    /// Build a [`swe_edge_configbuilder::SectionLoaderImpl`] from this builder.
    pub fn build_loader(
        self,
    ) -> Result<swe_edge_configbuilder::SectionLoaderImpl, swe_edge_configbuilder::ConfigError>
    {
        self.0.build_loader()
    }
}
