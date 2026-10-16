//! [`LoadbalancerSettingsProvider`] -- `LoadbalancerSettings` from TOML text.

use grpc_egress_transport::{GrpcChannelConfigError, LoadbalancerSettings};
use serde::Deserialize;

/// The shape of the file: one `[loadbalancer]` table, as the other sections of the configuration.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoadbalancerDocument {
    loadbalancer: LoadbalancerSettings,
}

/// Reading [`LoadbalancerSettings`], which is plain data in the domain crate. Bring the trait into scope
/// to call `LoadbalancerSettings::from_config(text)`.
///
/// ```toml
/// [loadbalancer]
/// strategy = "round-robin"
///
/// [[loadbalancer.backends]]
/// url    = "https://grpc-1.internal:50051"
/// weight = 1
/// ```
pub trait LoadbalancerSettingsProvider: Sized {
    /// Parse the settings from the `[loadbalancer]` table of `toml_text`. Whether the settings are
    /// usable (a backend at all, a valid URL) is checked by `build()` of the client builder.
    fn from_config(toml_text: &str) -> Result<Self, GrpcChannelConfigError>;
}

impl LoadbalancerSettingsProvider for LoadbalancerSettings {
    fn from_config(toml_text: &str) -> Result<Self, GrpcChannelConfigError> {
        let document: LoadbalancerDocument =
            toml::from_str(toml_text).map_err(|e| GrpcChannelConfigError::InvalidOptionValue {
                option: "loadbalancer",
                reason: e.to_string(),
            })?;
        Ok(document.loadbalancer)
    }
}
