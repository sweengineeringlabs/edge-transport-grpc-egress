//! gRPC retry policy schema.
//!
//! Policy values are built with [`GrpcRetryConfigBuilder`](crate::GrpcRetryConfigBuilder) or parsed by the standard
//! provider. Both paths validate before returning a completed value.

use crate::BuildGrpcRetryConfigRequest;
use configbuilder::ConfigSection;
use serde::Deserialize;

/// gRPC retry policy schema. Construct custom values with the domain builder trait and standard builder factory,
/// or parse TOML with the config provider. Both paths validate before returning a completed value.
///
/// Only `Unavailable` and `ResourceExhausted` gRPC status codes trigger retries;
/// `Unauthenticated`, `PermissionDenied`, `DeadlineExceeded`, and `Internal`
/// are never retried — the client must handle them directly.
///
/// # Examples
///
/// ```rust
/// use grpc_egress_retry_api::GrpcRetryConfig;
///
/// // SWE baseline defaults.
/// let cfg = GrpcRetryConfig::default();
/// assert_eq!(cfg.max_attempts(), 5);
/// assert_eq!(cfg.initial_backoff_ms(), 100);
/// assert_eq!(cfg.max_backoff_ms(), 5000);
///
/// // A custom policy.
/// let defaults = GrpcRetryConfig::default();
/// let cfg = GrpcRetryConfig::try_new(grpc_egress_retry_api::BuildGrpcRetryConfigRequest {
///     max_attempts: 3,
///     initial_backoff_ms: defaults.initial_backoff_ms(),
///     backoff_multiplier: defaults.backoff_multiplier(),
///     jitter_factor: defaults.jitter_factor(),
///     max_backoff_ms: defaults.max_backoff_ms(),
///     rate_limit_max_attempts: defaults.rate_limit_max_attempts(),
///     rate_limit_initial_backoff_ms: defaults.rate_limit_initial_backoff_ms(),
///     rate_limit_max_backoff_ms: defaults.rate_limit_max_backoff_ms(),
/// }).unwrap();
/// assert_eq!(cfg.max_attempts(), 3);
/// ```
/// ```compile_fail,E0451
/// use grpc_egress_retry_api::GrpcRetryConfig;
/// let _ = GrpcRetryConfig { max_attempts: 0, ..GrpcRetryConfig::default() };
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "GrpcRetryConfigData")]
#[serde(deny_unknown_fields)]
pub struct GrpcRetryConfig {
    /// Maximum attempts per request (1 = no retry).
    max_attempts: u32,

    /// Delay before the first retry, in milliseconds.
    initial_backoff_ms: u64,

    /// Exponential backoff base (e.g. 2.0 → 100ms, 200ms, 400ms).
    backoff_multiplier: f64,

    /// Jitter as a fraction of the computed backoff (0.0 = none,
    /// 0.1 = up to 10% random delta).
    jitter_factor: f64,

    /// Upper bound on any single retry backoff in milliseconds.
    max_backoff_ms: u64,

    // ── Rate-limit track ─────────────────────────────────────────────────────
    // Used when `classify()` returns `RetryRateLimit` (RESOURCE_EXHAUSTED
    // with a rate-limit message).  The upstream reset window is typically
    // seconds to minutes, so the floor is much higher than the standard
    // track.  The Retry-After hint from the server overrides the computed
    // backoff when present.
    /// Max attempts specifically for rate-limit `RESOURCE_EXHAUSTED`.
    /// Often lower than `max_attempts` — rate-limit retries are expensive.
    rate_limit_max_attempts: u32,

    /// Initial backoff for rate-limit errors, in milliseconds.
    /// Overridden by a `[retry-after=Ns]` hint when present.
    rate_limit_initial_backoff_ms: u64,

    /// Upper bound on the rate-limit backoff, in milliseconds.
    rate_limit_max_backoff_ms: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrpcRetryConfigData {
    max_attempts: u32,
    initial_backoff_ms: u64,
    backoff_multiplier: f64,
    jitter_factor: f64,
    max_backoff_ms: u64,
    rate_limit_max_attempts: u32,
    rate_limit_initial_backoff_ms: u64,
    rate_limit_max_backoff_ms: u64,
}

impl TryFrom<GrpcRetryConfigData> for GrpcRetryConfig {
    type Error = crate::Error;

    fn try_from(data: GrpcRetryConfigData) -> Result<Self, Self::Error> {
        Self::try_new(BuildGrpcRetryConfigRequest {
            max_attempts: data.max_attempts,
            initial_backoff_ms: data.initial_backoff_ms,
            backoff_multiplier: data.backoff_multiplier,
            jitter_factor: data.jitter_factor,
            max_backoff_ms: data.max_backoff_ms,
            rate_limit_max_attempts: data.rate_limit_max_attempts,
            rate_limit_initial_backoff_ms: data.rate_limit_initial_backoff_ms,
            rate_limit_max_backoff_ms: data.rate_limit_max_backoff_ms,
        })
    }
}

impl GrpcRetryConfig {
    /// Construct a validated retry policy.
    pub fn try_new(input: BuildGrpcRetryConfigRequest) -> Result<Self, crate::Error> {
        let BuildGrpcRetryConfigRequest {
            max_attempts,
            initial_backoff_ms,
            backoff_multiplier,
            jitter_factor,
            max_backoff_ms,
            rate_limit_max_attempts,
            rate_limit_initial_backoff_ms,
            rate_limit_max_backoff_ms,
        } = input;
        if max_attempts == 0 {
            return Err(crate::Error::InvalidConfig(
                "max_attempts must be >= 1".into(),
            ));
        }
        if rate_limit_max_attempts == 0 {
            return Err(crate::Error::InvalidConfig(
                "rate_limit_max_attempts must be >= 1".into(),
            ));
        }
        if backoff_multiplier <= 0.0 || !backoff_multiplier.is_finite() {
            return Err(crate::Error::InvalidConfig(
                "backoff_multiplier must be a positive finite number".into(),
            ));
        }
        if !(0.0..=1.0).contains(&jitter_factor) || !jitter_factor.is_finite() {
            return Err(crate::Error::InvalidConfig(
                "jitter_factor must be in [0.0, 1.0]".into(),
            ));
        }
        if max_backoff_ms < initial_backoff_ms {
            return Err(crate::Error::InvalidConfig(
                "max_backoff_ms must be >= initial_backoff_ms".into(),
            ));
        }
        if rate_limit_max_backoff_ms < rate_limit_initial_backoff_ms {
            return Err(crate::Error::InvalidConfig(
                "rate_limit_max_backoff_ms must be >= rate_limit_initial_backoff_ms".into(),
            ));
        }
        Ok(Self {
            max_attempts,
            initial_backoff_ms,
            backoff_multiplier,
            jitter_factor,
            max_backoff_ms,
            rate_limit_max_attempts,
            rate_limit_initial_backoff_ms,
            rate_limit_max_backoff_ms,
        })
    }

    /// Maximum attempts per request (1 = no retry).
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }
    /// Delay before the first retry, in milliseconds.
    pub fn initial_backoff_ms(&self) -> u64 {
        self.initial_backoff_ms
    }
    /// Exponential backoff base.
    pub fn backoff_multiplier(&self) -> f64 {
        self.backoff_multiplier
    }
    /// Jitter as a fraction of the computed backoff.
    pub fn jitter_factor(&self) -> f64 {
        self.jitter_factor
    }
    /// Upper bound on a single retry backoff in milliseconds.
    pub fn max_backoff_ms(&self) -> u64 {
        self.max_backoff_ms
    }
    /// Max attempts on the rate-limit track.
    pub fn rate_limit_max_attempts(&self) -> u32 {
        self.rate_limit_max_attempts
    }
    /// Initial rate-limit backoff in milliseconds.
    pub fn rate_limit_initial_backoff_ms(&self) -> u64 {
        self.rate_limit_initial_backoff_ms
    }
    /// Maximum rate-limit backoff in milliseconds.
    pub fn rate_limit_max_backoff_ms(&self) -> u64 {
        self.rate_limit_max_backoff_ms
    }
}

// `try_new` and validated deserialization reject NaN, so IEEE equality is reflexive for every constructible value.
impl Eq for GrpcRetryConfig {}

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

impl ConfigSection for GrpcRetryConfig {
    fn section_name() -> &'static str {
        "grpc_retry"
    }
}

impl edge_transport_retry_policy::BackoffPolicy for GrpcRetryConfig {
    fn initial_backoff_ms(&self) -> u64 {
        self.initial_backoff_ms()
    }

    fn max_backoff_ms(&self) -> u64 {
        self.max_backoff_ms()
    }

    fn backoff_multiplier(&self) -> f64 {
        self.backoff_multiplier()
    }

    fn jitter_factor(&self) -> f64 {
        self.jitter_factor()
    }
}

impl edge_transport_retry_policy::RateLimitBackoffPolicy for GrpcRetryConfig {
    fn rate_limit_max_attempts(&self) -> u32 {
        self.rate_limit_max_attempts()
    }

    fn rate_limit_initial_backoff_ms(&self) -> u64 {
        self.rate_limit_initial_backoff_ms()
    }

    fn rate_limit_max_backoff_ms(&self) -> u64 {
        self.rate_limit_max_backoff_ms()
    }
}
