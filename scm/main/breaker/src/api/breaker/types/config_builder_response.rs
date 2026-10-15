//! Response for [`crate::api::ConfigBuilderProvider::create_config_builder`].

use crate::api::ApplicationConfigBuilder;

/// Output of [`crate::api::ConfigBuilderProvider::create_config_builder`].
pub struct ConfigBuilderResponse {
    /// The config builder, pre-seeded with this crate's name and version.
    pub builder: ApplicationConfigBuilder,
}
