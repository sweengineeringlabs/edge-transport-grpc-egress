//! `GrpcBreakerFactory` -- wraps a client with a circuit breaker.

use grpc_egress_transport::GrpcEgress;

use crate::{BreakerDomainError, BreakerObservable, BuildGrpcBreakerRequest};

/// Wraps a [`GrpcEgress`] with a circuit breaker: after enough consecutive failures the breaker opens
/// and calls are refused without reaching the wrapped client, until a cool-down has passed. The policy
/// is bound in the implementor's constructor.
///
/// The wrapped client is also a [`BreakerObservable`], so the caller can read the state of the breaker.
/// A factory only constructs: `build` returns the client, never a trait object.
pub trait GrpcBreakerFactory {
    /// Wrap `req.inner` with the breaker. Fails when the policy is invalid.
    fn build<T: GrpcEgress + Send + Sync + 'static>(
        &self,
        req: BuildGrpcBreakerRequest<T>,
    ) -> Result<impl GrpcEgress + BreakerObservable + use<Self, T>, BreakerDomainError>;
}
