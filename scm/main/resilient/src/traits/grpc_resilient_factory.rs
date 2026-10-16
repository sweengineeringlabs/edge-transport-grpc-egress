//! `GrpcResilientFactory` -- wraps a client with retry and a circuit breaker.

use grpc_egress_breaker::BreakerObservable;
use grpc_egress_transport::GrpcEgress;

use crate::{BuildGrpcResilientRequest, ResilientTransportError};

/// Wraps a bare [`GrpcEgress`] with retry, and the result with a circuit breaker, as the
/// [`ResilienceConfig`](crate::ResilienceConfig) of the implementor says.
///
/// It is a decorator only: the caller builds the bare client. The returned client is also a
/// [`BreakerObservable`], so the caller can read the state of the breaker. A factory only constructs:
/// `build` returns the client, never a trait object.
pub trait GrpcResilientFactory {
    /// Wrap `req.inner`. Fails when the policy is invalid.
    fn build<T: GrpcEgress + Send + Sync + 'static>(
        &self,
        req: BuildGrpcResilientRequest<T>,
    ) -> Result<impl GrpcEgress + BreakerObservable + use<Self, T>, ResilientTransportError>;
}
