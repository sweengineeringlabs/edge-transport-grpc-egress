//! `BuildGrpcChannelConfigBuilderRequest` -- the request of the channel-configuration builder factory.

/// Request for [`GrpcChannelConfigBuilderFactory::build`](crate::GrpcChannelConfigBuilderFactory::build).
///
/// It carries nothing: every setting is made on the builder it returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildGrpcChannelConfigBuilderRequest;
