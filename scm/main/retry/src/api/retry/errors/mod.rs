//! Error types for the gRPC retry decorator.

// Moved to edge-transport-retry-policy (ADR-003/ADR-004): ParseFailed/InvalidConfig are
// protocol-agnostic config-loading errors.
pub use edge_transport_retry_policy::RetryError as Error;
