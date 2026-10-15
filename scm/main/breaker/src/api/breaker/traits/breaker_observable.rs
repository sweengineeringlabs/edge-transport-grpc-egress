//! Interface for observing circuit-breaker state from outside the decorator.

use futures::future::BoxFuture;

use crate::api::BreakerDomainError;
use crate::api::GrpcBreakerClient;
use crate::api::GrpcBreakerConfig;
use crate::api::ObserveStateRequest;
use crate::api::ObserveStateResponse;

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

    /// Construct the concrete [`GrpcBreakerClient`] that implements this
    /// trait — gives it a genuine role in this trait's signature set, not
    /// just an impl-site `Self`. `Self: Sized` keeps this trait
    /// dyn-compatible for `Box<dyn Trait>`.
    fn default_client<T>(inner: T, config: GrpcBreakerConfig) -> GrpcBreakerClient<T>
    where
        Self: Sized,
    {
        GrpcBreakerClient::new(inner, config)
    }
}
