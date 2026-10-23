//! `StdLoadbalancerSettingsProvider` -- `LoadbalancerSettings` from TOML text, read with `toml`.

use grpc_egress_transport::{
    GrpcChannelConfigError, LoadbalancerSettings, LoadbalancerSettingsProvider,
};
use serde::Deserialize;

/// The shape of the file: one `[loadbalancer]` table, as the other sections of the configuration.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoadbalancerDocument {
    loadbalancer: LoadbalancerSettings,
}

/// Reads [`LoadbalancerSettings`] from the `[loadbalancer]` table of TOML text.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StdLoadbalancerSettingsProvider;

impl LoadbalancerSettingsProvider for StdLoadbalancerSettingsProvider {
    fn parse_config(
        &self,
        toml_text: &str,
    ) -> Result<LoadbalancerSettings, GrpcChannelConfigError> {
        let document: LoadbalancerDocument =
            toml::from_str(toml_text).map_err(|e| GrpcChannelConfigError::InvalidOptionValue {
                option: "loadbalancer",
                reason: e.to_string(),
            })?;
        Ok(document.loadbalancer)
    }
}
