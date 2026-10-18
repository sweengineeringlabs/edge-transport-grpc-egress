//! [`ClientFixture`] -- a client built from a channel configuration, through the public builder.

use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcChannelConfig, GrpcChannelConfigError, GrpcEgressBuilder,
    GrpcEgressBuilderFactory,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;
use proto_pattern::{RawTransport, Transport};

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
    ) -> Result<impl Transport + RawTransport + use<>, GrpcChannelConfigError> {
        TonicGrpcEgressBuilderSpiFactory::new(config.endpoint.clone())
            .build(BuildGrpcEgressBuilderRequest)
            .with_channel_config(config)
            .build()
    }
}
