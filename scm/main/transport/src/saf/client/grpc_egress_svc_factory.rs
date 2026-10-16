//! Composition site for [`GrpcEgress`] — one file per trait keeps wiring focused.

use std::sync::Arc;

use crate::api::{GrpcChannelConfig, GrpcChannelConfigError, GrpcEgress};
use crate::saf::TransportConstruction;

/// Factory for the default [`GrpcEgress`] transport.
pub struct GrpcEgressFactory;

impl GrpcEgressFactory {
    /// Construct a [`GrpcEgress`] from a validated [`GrpcChannelConfig`].
    pub fn create(
        config: &GrpcChannelConfig,
    ) -> Result<Arc<dyn GrpcEgress>, GrpcChannelConfigError> {
        TransportConstruction::create_transport_from_config(config)
    }
}
