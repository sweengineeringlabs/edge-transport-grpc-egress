//! `StdLoadbalancerSettingsProviderFactory` -- creates the reader of the load balancer settings.

use grpc_egress_transport_api::{
    BuildLoadbalancerSettingsProviderRequest, LoadbalancerSettingsProvider,
    LoadbalancerSettingsProviderFactory,
};

use crate::spi::loadbalancer::StdLoadbalancerSettingsProvider;

/// Creates providers that read the settings from TOML text.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdLoadbalancerSettingsProviderFactory;

impl LoadbalancerSettingsProviderFactory for StdLoadbalancerSettingsProviderFactory {
    fn build(
        &self,
        _req: BuildLoadbalancerSettingsProviderRequest,
    ) -> impl LoadbalancerSettingsProvider + use<> {
        StdLoadbalancerSettingsProvider
    }
}
