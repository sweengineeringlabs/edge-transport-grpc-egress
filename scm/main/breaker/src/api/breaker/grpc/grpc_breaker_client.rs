//! Shared constant for the `GrpcBreakerClient` inherent impl — the flat
//! api/ counterpart to the flat `core::breaker::grpc::grpc_breaker_client`
//! file. The type itself is declared in `api::breaker::types::
//! grpc_breaker_client`.

/// Label used in `tracing` events emitted by [`crate::api::GrpcBreakerClient`].
pub const GRPC_BREAKER_CLIENT_LOG_TARGET: &str = "grpc_breaker::client";
