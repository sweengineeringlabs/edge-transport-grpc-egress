//! Construction input for a validated [`crate::GrpcRetryConfig`].

/// Caller-owned policy parameters for constructing a retry configuration.
///
/// This is a phase-one construction bundle. Its fields are inputs, not completed policy state; pass it to
/// [`crate::GrpcRetryConfig::try_new`] to validate and create the immutable value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuildGrpcRetryConfigRequest {
    /// Maximum attempts per request (1 means no retry).
    pub max_attempts: u32,
    /// Delay before the first retry, in milliseconds.
    pub initial_backoff_ms: u64,
    /// Exponential backoff base.
    pub backoff_multiplier: f64,
    /// Jitter fraction in the inclusive range 0 through 1.
    pub jitter_factor: f64,
    /// Upper bound on a standard retry backoff, in milliseconds.
    pub max_backoff_ms: u64,
    /// Maximum attempts on the rate-limit track.
    pub rate_limit_max_attempts: u32,
    /// Initial delay for rate-limit retries, in milliseconds.
    pub rate_limit_initial_backoff_ms: u64,
    /// Upper bound on a rate-limit retry backoff, in milliseconds.
    pub rate_limit_max_backoff_ms: u64,
}
