//! `LoadbalancerSettingsProviderFactory` -- creates the reader of the load balancer settings.

use crate::{BuildLoadbalancerSettingsProviderRequest, LoadbalancerSettingsProvider};

/// Creates a [`LoadbalancerSettingsProvider`].
pub trait LoadbalancerSettingsProviderFactory {
    /// A provider that reads the settings from TOML text.
    fn build(
        &self,
        req: BuildLoadbalancerSettingsProviderRequest,
    ) -> impl LoadbalancerSettingsProvider + use<Self>;
}
