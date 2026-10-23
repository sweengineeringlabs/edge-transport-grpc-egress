//! `GrpcBreakerFactory` -- wraps a client with a circuit breaker.

use proto_pattern::{RawTransport, Transport};

use crate::{BreakerDomainError, BreakerObservable, BuildGrpcBreakerRequest};

/// Wraps a client (a [`Transport`] and a [`RawTransport`]) with a circuit breaker: after enough consecutive failures
/// the breaker opens and calls are refused without reaching the wrapped client, until a cool-down has passed. The
/// policy is bound in the implementor's constructor.
///
/// The wrapped client is also a [`BreakerObservable`], so the caller can read the state of the breaker. A factory
/// only constructs: `build` returns the client, never a trait object.
pub trait GrpcBreakerFactory {
    /// Wrap `req.inner` with the breaker. Fails when the policy is invalid.
    ///
    /// # Errors
    ///
    /// [`BreakerDomainError::InvalidConfig`] when the policy the factory was given cannot work: a failure threshold
    /// or a probe count of zero.
    fn build<T: Transport + RawTransport + Send + Sync + 'static>(
        &self,
        req: BuildGrpcBreakerRequest<T>,
    ) -> Result<impl Transport + RawTransport + BreakerObservable + use<Self, T>, BreakerDomainError>;
}
