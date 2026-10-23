//! `GrpcChannelConfigBuilderFactory` -- creates the builder of a channel configuration.

use crate::{BuildGrpcChannelConfigBuilderRequest, GrpcChannelConfigBuilder};

/// Creates a [`GrpcChannelConfigBuilder`]. The factory does nothing else: the settings and the work of building are
/// the builder's.
pub trait GrpcChannelConfigBuilderFactory {
    /// A new builder with nothing set and TLS required. Each call returns an independent builder.
    fn build(
        &self,
        req: BuildGrpcChannelConfigBuilderRequest,
    ) -> impl GrpcChannelConfigBuilder + use<Self>;
}
