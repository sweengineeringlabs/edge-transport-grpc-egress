//! The retry decorator that wraps any client that is a `Transport` and a `RawTransport`.

use std::sync::Arc;

use grpc_egress_retry::GrpcRetryConfig;

/// Decorator that wraps an inner `Transport` and `RawTransport` with the
/// retry semantics described at the crate root.
///
/// `T` is the inner type; the wrapper is `T + 'static + Send + Sync`
/// so it can flow across `.await` boundaries inside the runtime.
///
/// Construct via [`GrpcRetryFacade`](crate::std::GrpcRetryFacade) (loads
/// the SWE baseline) or directly via [`GrpcRetryClient::new`].
pub struct GrpcRetryClient<T> {
    pub(crate) inner: T,
    pub(crate) config: Arc<GrpcRetryConfig>,
}

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
    pub(crate) fn new(inner: T, config: GrpcRetryConfig) -> Self {
        Self {
            inner,
            config: std::sync::Arc::new(config),
        }
    }
}
