//! Response for [`crate::ConfigBuilderProvider::create_config_builder`].

use crate::ApplicationConfigBuilder;

/// Output of [`crate::ConfigBuilderProvider::create_config_builder`].
pub struct ConfigBuilderResponse {
    /// The config builder, pre-seeded with this crate's name and version.
    pub builder: ApplicationConfigBuilder,
}
