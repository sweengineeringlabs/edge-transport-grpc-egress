//! Shared constant for the failure classifier implementation — the flat
//! api/ counterpart to the flat `core::breaker::failure_classifier` file.

/// Label used in `tracing` events emitted while classifying outcomes.
pub const FAILURE_CLASSIFIER_LOG_TARGET: &str = "grpc_breaker::classifier";
