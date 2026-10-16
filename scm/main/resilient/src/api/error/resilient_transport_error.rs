//! Error type for the resilience decorator.

use grpc_egress_breaker::Error as BreakerDomainError;

/// Error produced by [`crate::std::GrpcResilientFacade::apply_resilience`].
#[derive(Debug, thiserror::Error)]
pub enum ResilientTransportError {
    /// The resilience policy contains an invalid field combination.
    #[error("invalid resilience config: {0}")]
    InvalidResilience(String),
    /// The breaker decorator failed to wrap the inner client.
    #[error(transparent)]
    Breaker(#[from] BreakerDomainError),
}
