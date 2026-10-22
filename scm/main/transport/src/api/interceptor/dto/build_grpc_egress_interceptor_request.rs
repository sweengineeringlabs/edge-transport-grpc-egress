//! `BuildGrpcEgressInterceptorRequest` -- the request of the interceptor factory.

/// Request for [`GrpcEgressInterceptorFactory::build`](crate::GrpcEgressInterceptorFactory::build).
///
/// It carries nothing: the one value a factory needs, the source of the trace context, is bound when
/// the factory is created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildGrpcEgressInterceptorRequest;
