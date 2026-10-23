//! `LoadbalancerSettingsProvider` -- `LoadbalancerSettings` from TOML text.

use crate::{GrpcChannelConfigError, LoadbalancerSettings};

/// Reading [`LoadbalancerSettings`], which is plain data in this crate.
///
/// ```toml
/// [loadbalancer]
/// strategy = "round-robin"
///
/// [[loadbalancer.backends]]
/// url    = "https://grpc-1.internal:50051"
/// weight = 1
/// ```
pub trait LoadbalancerSettingsProvider {
    /// Parse the settings from the `[loadbalancer]` table of `toml_text`. Whether the settings are
    /// usable (a backend at all, a valid URL) is checked by `build()` of the client builder.
    ///
    /// # Errors
    ///
    /// `InvalidOptionValue` for `loadbalancer` when the text is not valid TOML of that shape.
    fn parse_config(&self, toml_text: &str)
        -> Result<LoadbalancerSettings, GrpcChannelConfigError>;
}
