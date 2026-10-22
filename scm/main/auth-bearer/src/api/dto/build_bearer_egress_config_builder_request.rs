//! `BuildBearerEgressConfigBuilderRequest` -- the request of the outbound config builder factory.

/// Request for [`BearerEgressConfigBuilderFactory::build`](crate::BearerEgressConfigBuilderFactory::build). It
/// carries nothing: every field is set on the builder it returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildBearerEgressConfigBuilderRequest;
