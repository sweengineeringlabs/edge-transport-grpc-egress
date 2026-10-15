//! Interface for constructing circuit-breaker decorators.

use crate::api::BreakerDomainError;
use crate::api::GrpcBreakerFacade;
use crate::api::WrapBreakerRequest;
use crate::api::WrapBreakerResponse;

/// Contract for wrapping an inner client with a circuit-breaker policy.
///
/// The concrete implementation lives in `core::breaker_decorator`; callers
/// obtain a decorator via the `saf::breaker_decorator_svc_factory`.
pub trait BreakerDecorator<T>: Send + Sync {
    /// Wrap `inner` with the supplied breaker policy, returning the decorator.
    fn wrap(
        &self,
        req: WrapBreakerRequest<T>,
    ) -> Result<WrapBreakerResponse<T>, BreakerDomainError>;

    /// Construct the facade that composes this crate's default
    /// implementations — gives [`GrpcBreakerFacade`] a genuine role in
    /// this trait's signature set. `Self: Sized` keeps this trait
    /// dyn-compatible for `Box<dyn Trait>`.
    fn default_facade() -> GrpcBreakerFacade
    where
        Self: Sized,
    {
        GrpcBreakerFacade
    }
}
