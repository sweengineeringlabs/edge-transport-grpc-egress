//! `BuildGrpcBreakerRequest` -- the request of the breaker factory.

/// Request for [`GrpcBreakerFactory::build`](crate::GrpcBreakerFactory::build): the client to wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildGrpcBreakerRequest<T> {
    /// The client whose calls the breaker guards.
    pub inner: T,
}
