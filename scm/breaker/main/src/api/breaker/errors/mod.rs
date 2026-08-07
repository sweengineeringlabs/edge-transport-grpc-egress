//! Domain error types for `edge_transport_grpc_egress_breaker`.

// Moved to edge-transport-breaker-policy (ADR-004/ADR-003): the error variants
// (ParseFailed/InvalidConfig) are protocol-agnostic config-loading errors.
pub use edge_transport_breaker_policy::BreakerError as BreakerDomainError;
pub use edge_transport_breaker_policy::BreakerError as Error;
