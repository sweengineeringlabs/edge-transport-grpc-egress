//! gRPC retry policy schema.
//!
//! Policy **values**: the baseline is [`GrpcRetryConfig::default`]; a consumer's own TOML goes through
//! `GrpcRetryConfig::from_config` of `grpc-egress-retry-std`.

use serde::Deserialize;

/// gRPC retry policy schema.  Construct via `GrpcRetryConfig::from_config` of
/// `grpc-egress-retry-std` (custom TOML) or [`GrpcRetryConfig::default`] (crate baseline).
///
/// Only `Unavailable` and `ResourceExhausted` gRPC status codes trigger retries;
/// `Unauthenticated`, `PermissionDenied`, `DeadlineExceeded`, and `Internal`
/// are never retried — the client must handle them directly.
///
/// # Examples
///
/// ```rust
/// use grpc_egress_retry::GrpcRetryConfig;
///
/// // SWE baseline defaults.
/// let cfg = GrpcRetryConfig::default();
/// assert_eq!(cfg.max_attempts, 5);
/// assert_eq!(cfg.initial_backoff_ms, 100);
/// assert_eq!(cfg.max_backoff_ms, 5000);
///
/// // A custom policy.
/// let cfg = GrpcRetryConfig { max_attempts: 3, ..GrpcRetryConfig::default() };
/// assert_eq!(cfg.max_attempts, 3);
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrpcRetryConfig {
    /// Maximum attempts per request (1 = no retry).
    pub max_attempts: u32,

    /// Delay before the first retry, in milliseconds.
    pub initial_backoff_ms: u64,

    /// Exponential backoff base (e.g. 2.0 → 100ms, 200ms, 400ms).
    pub backoff_multiplier: f64,

    /// Jitter as a fraction of the computed backoff (0.0 = none,
    /// 0.1 = up to 10% random delta).
    pub jitter_factor: f64,

    /// Upper bound on any single retry backoff in milliseconds.
    pub max_backoff_ms: u64,

    // ── Rate-limit track ─────────────────────────────────────────────────────
    // Used when `classify()` returns `RetryRateLimit` (RESOURCE_EXHAUSTED
    // with a rate-limit message).  The upstream reset window is typically
    // seconds to minutes, so the floor is much higher than the standard
    // track.  The Retry-After hint from the server overrides the computed
    // backoff when present.
    /// Max attempts specifically for rate-limit `RESOURCE_EXHAUSTED`.
    /// Often lower than `max_attempts` — rate-limit retries are expensive.
    pub rate_limit_max_attempts: u32,

    /// Initial backoff for rate-limit errors, in milliseconds.
    /// Overridden by a `[retry-after=Ns]` hint when present.
    pub rate_limit_initial_backoff_ms: u64,

    /// Upper bound on the rate-limit backoff, in milliseconds.
    pub rate_limit_max_backoff_ms: u64,
}

impl Default for GrpcRetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_backoff_ms: 5000,
            rate_limit_max_attempts: 2,
            rate_limit_initial_backoff_ms: 1000,
            rate_limit_max_backoff_ms: 10000,
        }
    }
}

impl swe_edge_configbuilder::ConfigSection for GrpcRetryConfig {
    fn section_name() -> &'static str {
        "grpc_retry"
    }
}

impl edge_transport_retry_policy::BackoffPolicy for GrpcRetryConfig {
    fn initial_backoff_ms(&self) -> u64 {
        self.initial_backoff_ms
    }

    fn max_backoff_ms(&self) -> u64 {
        self.max_backoff_ms
    }

    fn backoff_multiplier(&self) -> f64 {
        self.backoff_multiplier
    }

    fn jitter_factor(&self) -> f64 {
        self.jitter_factor
    }
}

impl edge_transport_retry_policy::RateLimitBackoffPolicy for GrpcRetryConfig {
    fn rate_limit_max_attempts(&self) -> u32 {
        self.rate_limit_max_attempts
    }

    fn rate_limit_initial_backoff_ms(&self) -> u64 {
        self.rate_limit_initial_backoff_ms
    }

    fn rate_limit_max_backoff_ms(&self) -> u64 {
        self.rate_limit_max_backoff_ms
    }
}
