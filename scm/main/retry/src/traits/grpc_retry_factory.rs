//! `GrpcRetryFactory` -- wraps a client with retry.

use grpc_egress_transport::GrpcEgress;

use crate::BuildGrpcRetryRequest;

/// Wraps a [`GrpcEgress`] so that its unary calls are retried with exponential backoff and jitter on
/// the retryable gRPC status codes. The policy is bound in the implementor's constructor.
///
/// A factory only constructs: `build` returns the wrapped client, never a trait object.
pub trait GrpcRetryFactory {
    /// Wrap `req.inner` with retry.
    fn build<T: GrpcEgress + Send + Sync + 'static>(
        &self,
        req: BuildGrpcRetryRequest<T>,
    ) -> impl GrpcEgress + use<Self, T>;
}
