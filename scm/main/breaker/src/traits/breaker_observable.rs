//! Interface for observing circuit-breaker state from outside the decorator.

use futures::future::BoxFuture;

use crate::BreakerDomainError;
use crate::ObserveStateRequest;
use crate::ObserveStateResponse;

/// Observability contract for a circuit-breaker decorator.
///
/// Implemented by [`GrpcBreakerClient`] in `core::grpc::breaker_client`.
pub trait BreakerObservable: Send + Sync {
    /// Observe the current breaker state. Returns a snapshot; the breaker
    /// may transition immediately after this call returns.
    fn state(
        &self,
        req: ObserveStateRequest,
    ) -> BoxFuture<'_, Result<ObserveStateResponse, BreakerDomainError>>;
}
