//! Composition site for [`Processor`] — one file per trait keeps wiring focused.

use crate::api::{GrpcChannelConfig, GrpcChannelConfigError, Processor};
use crate::saf::TransportConstruction;

/// Factory for the default [`Processor`].
pub struct ProcessorFactory;

impl ProcessorFactory {
    /// Construct the default [`Processor`] — the concrete gRPC transport client.
    pub fn create(
        config: &GrpcChannelConfig,
    ) -> Result<Box<dyn Processor>, GrpcChannelConfigError> {
        let client = TransportConstruction::create_tonic_client_from_config(config)?;
        Ok(Box::new(client))
    }
}
