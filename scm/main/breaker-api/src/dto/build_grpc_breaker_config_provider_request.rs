//! `BuildGrpcBreakerConfigProviderRequest` -- the request of the breaker policy provider factory.

/// Request for [`GrpcBreakerConfigProviderFactory::build`](crate::GrpcBreakerConfigProviderFactory::build). It carries
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildGrpcBreakerConfigProviderRequest;
