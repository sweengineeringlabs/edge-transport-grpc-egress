//! Shared constant for the `BackoffScheduler` implementation — the flat
//! api/ counterpart to the flat `core::backoff::backoff_scheduler` file.
//! The trait itself lives in `api::backoff::traits::backoff_scheduler`.

/// Label used in `tracing` events emitted during backoff computation.
pub const BACKOFF_SCHEDULER_LOG_TARGET: &str = "grpc_retry::backoff";
