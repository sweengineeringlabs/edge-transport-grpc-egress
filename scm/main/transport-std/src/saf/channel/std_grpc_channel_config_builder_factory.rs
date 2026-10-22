//! `StdGrpcChannelConfigBuilderFactory` -- creates the builder of a channel configuration.

use grpc_egress_transport::{
    BuildGrpcChannelConfigBuilderRequest, GrpcChannelConfigBuilder, GrpcChannelConfigBuilderFactory,
};

use crate::std::StdGrpcChannelConfigBuilder;

/// Creates builders of channel configurations.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdGrpcChannelConfigBuilderFactory;

impl GrpcChannelConfigBuilderFactory for StdGrpcChannelConfigBuilderFactory {
    fn build(
        &self,
        _req: BuildGrpcChannelConfigBuilderRequest,
    ) -> impl GrpcChannelConfigBuilder + use<> {
        StdGrpcChannelConfigBuilder::default()
    }
}
