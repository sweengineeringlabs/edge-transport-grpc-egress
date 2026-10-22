//! `BuildGrpcRetryRequest` -- the request of the retry factory.

/// Request for [`GrpcRetryFactory::build`](crate::GrpcRetryFactory::build): the client to wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildGrpcRetryRequest<T> {
    /// The client whose unary calls are retried.
    pub inner: T,
}
