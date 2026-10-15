//! `impl` blocks for [`GrpcRetryClient`]. The type *declaration* lives
//! in `api/`; the `GrpcEgress` impl lives in `core/retry_egress.rs`.

use crate::api::{GrpcRetryClient, GrpcRetryConfig};

impl<T> std::fmt::Debug for GrpcRetryClient<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcRetryClient")
            .field("max_attempts", &self.config.max_attempts)
            .field("initial_backoff_ms", &self.config.initial_backoff_ms)
            .field("backoff_multiplier", &self.config.backoff_multiplier)
            .field("jitter_factor", &self.config.jitter_factor)
            .field("max_backoff_ms", &self.config.max_backoff_ms)
            .field(
                "rate_limit_max_attempts",
                &self.config.rate_limit_max_attempts,
            )
            .field(
                "rate_limit_initial_backoff_ms",
                &self.config.rate_limit_initial_backoff_ms,
            )
            .field(
                "rate_limit_max_backoff_ms",
                &self.config.rate_limit_max_backoff_ms,
            )
            .finish()
    }
}

impl<T> GrpcRetryClient<T> {
    /// Construct a new retry decorator around `inner`.
    pub fn new(inner: T, config: GrpcRetryConfig) -> Self {
        Self {
            inner,
            config: std::sync::Arc::new(config),
        }
    }

    /// Borrow the active retry policy.
    pub fn config(&self) -> &GrpcRetryConfig {
        &self.config
    }
}
