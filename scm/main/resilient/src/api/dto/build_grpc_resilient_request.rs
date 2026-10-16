//! `BuildGrpcResilientRequest` -- the request of the resilience factory.

/// Request for [`GrpcResilientFactory::build`](crate::GrpcResilientFactory::build): the client to wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildGrpcResilientRequest<T> {
    /// The bare client that gets retry and a circuit breaker.
    pub inner: T,
}
