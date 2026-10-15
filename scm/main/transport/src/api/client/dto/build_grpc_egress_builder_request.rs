//! `BuildGrpcEgressBuilderRequest` -- the request of [`GrpcEgressBuilderFactory::build`].
//!
//! [`GrpcEgressBuilderFactory::build`]: crate::api::GrpcEgressBuilderFactory::build

/// Request for [`GrpcEgressBuilderFactory::build`](crate::api::GrpcEgressBuilderFactory::build).
///
/// It carries nothing: the one value a factory needs, the endpoint, is bound when the factory is
/// created, and every option is set on the builder it returns, so a caller is never told which
/// values a client needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildGrpcEgressBuilderRequest;
