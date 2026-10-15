//! gRPC retry policy schema.
//!
//! Policy **values** live in TOML:
//! - crate-shipped baseline: `config/application.toml`
//! - consumer override: whatever TOML the binary loads and
//!   passes to [`GrpcRetryConfig::from_config`].
//!
//! No `Default` impl — per the config-driven principle, policy
//! is data in a file, not literals in a source tree.

use serde::Deserialize;

/// gRPC retry policy schema.  Construct via
/// [`GrpcRetryConfig::from_config`] (custom TOML) or
/// [`GrpcRetryConfig::default`] (crate baseline).
///
/// Only `Unavailable` and `ResourceExhausted` gRPC status codes trigger retries;
/// `Unauthenticated`, `PermissionDenied`, `DeadlineExceeded`, and `Internal`
/// are never retried — the client must handle them directly.
///
/// # Examples
///
/// ```rust
/// use edge_transport_grpc_egress_retry::GrpcRetryConfig;
/// use std::time::Duration;
///
/// // SWE baseline defaults.
/// let cfg = GrpcRetryConfig::default();
/// assert_eq!(cfg.max_attempts, 5);
/// assert_eq!(cfg.initial_backoff(), Duration::from_millis(100));
/// assert_eq!(cfg.max_backoff(), Duration::from_secs(5));
///
/// // Custom policy from TOML.
/// let toml = "max_attempts = 3
/// initial_backoff_ms = 50
/// backoff_multiplier = 2.0
/// jitter_factor = 0.1
/// max_backoff_ms = 2000
/// rate_limit_max_attempts = 1
/// rate_limit_initial_backoff_ms = 500
/// rate_limit_max_backoff_ms = 5000";
/// let cfg = GrpcRetryConfig::from_config(toml).unwrap();
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
