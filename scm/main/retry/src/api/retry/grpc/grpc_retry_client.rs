//! Shared constant for the `GrpcRetryClient` inherent impl — the flat
//! api/ counterpart to the flat `core::grpc::grpc_retry_client` file.
//! The type itself is declared in `api::types::grpc_retry_client`; the
//! trait that gives it signature presence lives in
//! `api::grpc::traits::retry_decorator`.

/// Label used in `tracing` events emitted by [`crate::api::GrpcRetryClient`].
pub const GRPC_RETRY_CLIENT_LOG_TARGET: &str = "grpc_retry::client";
