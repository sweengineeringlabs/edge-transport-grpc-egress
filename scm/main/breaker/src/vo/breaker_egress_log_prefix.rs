//! Shared logging prefix for the `GrpcEgress` decorator wiring — the flat
//! api/ counterpart to the flat `core::breaker::breaker_egress` file.

/// Prefix used on every log/error message emitted by the decorator's
/// `GrpcEgress` implementation.
pub const BREAKER_EGRESS_LOG_PREFIX: &str = "grpc-breaker";
