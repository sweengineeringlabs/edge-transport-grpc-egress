//! [`ClientFixture`] -- a client built from a channel configuration, through the public builder.

use std::sync::Arc;

use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcChannelConfig, GrpcChannelConfigError, GrpcEgress,
    GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;

/// Builds clients for tests the way a caller does: a factory for the endpoint, a builder, the
/// configuration applied with `with_channel_config`, `build`.
pub struct ClientFixture;

impl ClientFixture {
    /// A client for `config`.
    ///
    /// # Errors
    ///
    /// The [`GrpcChannelConfigError`] that `build` returns for the configuration.
    pub fn from_config(
        config: &GrpcChannelConfig,
    ) -> Result<Arc<dyn GrpcEgress>, GrpcChannelConfigError> {
        let client = TonicGrpcEgressBuilderSpiFactory::new(config.endpoint.clone())
            .build(BuildGrpcEgressBuilderRequest)
            .with_channel_config(config)
            .build()?;
        Ok(Arc::new(client))
    }
}
