//! `GrpcResilientFactory` -- wraps a client with retry and a circuit breaker.

use grpc_egress_breaker_api::BreakerObservable;
use proto_pattern::{RawTransport, Transport};

use crate::{BuildGrpcResilientRequest, ResilientTransportError};

/// Wraps a client with retry inside a circuit breaker (ADR-007).
///
/// The client is a [`Transport`] and a [`RawTransport`]; the product is both again, and is also a
/// [`BreakerObservable`], so a caller can ask the breaker for its state.
pub trait GrpcResilientFactory {
    /// A client that retries the unary calls of `req.inner` and short-circuits when too many fail.
    ///
    /// # Errors
    ///
    /// [`ResilientTransportError::InvalidResilience`] when the policy the factory holds is not valid, and
    /// [`ResilientTransportError::Breaker`] when the breaker cannot be built from it.
    fn build<T: Transport + RawTransport + Send + Sync + 'static>(
        &self,
        req: BuildGrpcResilientRequest<T>,
    ) -> Result<
        impl Transport + RawTransport + BreakerObservable + use<Self, T>,
        ResilientTransportError,
    >;
}
