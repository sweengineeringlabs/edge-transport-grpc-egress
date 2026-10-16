//! Interface for constructing circuit-breaker decorators.

use crate::BreakerDomainError;
use crate::WrapBreakerRequest;
use crate::WrapBreakerResponse;

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
}
