//! Composition site for [`GrpcEgressProstCodec`] — one file per trait keeps wiring focused.

use crate::api::{GrpcChannelConfig, GrpcChannelConfigError, GrpcEgressProstCodec};
use crate::saf::TransportConstruction;

/// Factory for the default [`GrpcEgressProstCodec`] transport.
pub struct GrpcEgressProstCodecFactory;

impl GrpcEgressProstCodecFactory {
    /// Construct a [`GrpcEgressProstCodec`] transport from a validated [`GrpcChannelConfig`].
    pub fn create(
        config: &GrpcChannelConfig,
    ) -> Result<Box<dyn GrpcEgressProstCodec>, GrpcChannelConfigError> {
        TransportConstruction::create_prost_transport_from_config(config)
    }
}
