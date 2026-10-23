//! `BuildGrpcEgressInterceptorChainRequest` -- the request of the interceptor chain factory.

/// Request for
/// [`GrpcEgressInterceptorChainFactory::build`](crate::GrpcEgressInterceptorChainFactory::build). It carries nothing:
/// interceptors are added to the chain it returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildGrpcEgressInterceptorChainRequest;
