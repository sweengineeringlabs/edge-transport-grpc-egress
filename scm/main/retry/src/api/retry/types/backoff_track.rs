//! Which backoff track to compute a schedule for.

use std::time::Duration;

/// Discriminates the standard-retry track from the rate-limit track —
/// each uses a different set of config fields (see [`crate::api::GrpcRetryConfig`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackoffTrack {
    /// Standard-retry track (`initial_backoff_ms` / `max_backoff_ms`).
    Standard,
    /// Rate-limit track (`rate_limit_initial_backoff_ms` / `rate_limit_max_backoff_ms`).
    /// `retry_after_hint`, when present, is returned directly instead of
    /// being computed.
    RateLimit {
        /// Upstream-supplied `Retry-After` hint, if one was parsed.
        retry_after_hint: Option<Duration>,
    },
}
