//! Shared constant for the `GrpcEgress` impl on `GrpcRetryClient` — the
//! flat api/ counterpart to the flat `core::retry::retry_egress` file.
//! The trait that gives `RetryDecision`/`ResourceExhaustedContext`
//! signature presence lives in `api::retry::traits::retry_inspector`.

/// Label used in `tracing` events emitted by the retry loop.
pub const RETRY_EGRESS_LOG_TARGET: &str = "grpc_retry::egress";
