//! `CompositionRoot` -- assembles a client the way a composition root does.

use proto_pattern::{CallOptions, CallRawRequestBuilder, MethodPath, RawTransport};
use std::error::Error;

use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcChannelConfig, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_spi::TonicGrpcEgressBuilderSpiFactory;

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
        let builder = TonicGrpcEgressBuilderSpiFactory::new(config.endpoint().to_owned())
            .build(BuildGrpcEgressBuilderRequest)
            .with_channel_config(&config);
        Ok(builder)
    }

    /// A call made through any client, bare or wrapped: generic over the contract's trait, never a `dyn`.
    ///
    /// # Errors
    ///
    /// The error of the path or of the call.
    pub async fn ping<T: RawTransport>(client: &T, body: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        let path = MethodPath::create("/pkg.Service/Method")?;
        let request = CallRawRequestBuilder::new()
            .with_path(path)
            .with_message(body.to_vec())
            .with_options(CallOptions::default())
            .build();
        Ok(client
            .call_raw(request)
            .await?
            .into_response()
            .into_message())
    }
    // END consumer code
}
