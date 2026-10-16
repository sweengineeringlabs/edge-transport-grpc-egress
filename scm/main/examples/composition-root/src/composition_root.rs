//! `CompositionRoot` -- assembles a client the way a composition root does.

use std::error::Error;

use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcChannelConfig, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;

/// Loads a channel configuration from TOML text and assembles clients from it.
pub struct CompositionRoot {
    config_text: String,
}

impl CompositionRoot {
    /// A root whose configuration is `config_text`, the text of a `GrpcChannelConfig` in TOML.
    pub fn new(config_text: impl Into<String>) -> Self {
        Self {
            config_text: config_text.into(),
        }
    }

    /// Load the channel configuration the root was created with.
    ///
    /// # Errors
    ///
    /// The error of parsing the TOML text.
    pub fn load_config(&self) -> Result<GrpcChannelConfig, Box<dyn Error>> {
        Ok(toml::from_str(&self.config_text)?)
    }

    // BEGIN consumer code
    /// Start a builder with the loaded configuration applied; the caller adds options and builds.
    ///
    /// # Errors
    ///
    /// The error of loading the configuration.
    pub fn assemble(&self) -> Result<impl GrpcEgressBuilder, Box<dyn Error>> {
        let config = self.load_config()?;
        let builder = TonicGrpcEgressBuilderSpiFactory::new(config.endpoint.clone())
            .build(BuildGrpcEgressBuilderRequest)
            .with_channel_config(&config);
        Ok(builder)
    }
    // END consumer code
}
