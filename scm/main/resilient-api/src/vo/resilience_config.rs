//! Resilience configuration — retry + circuit breaker as a first-class,
//! composer-facing policy. This crate is the sole owner of this shape:
//! `transport`'s `GrpcChannelConfig` carries no resilience-shaped field
//! (ADR-004) — a composition root builds a bare client, then applies
//! [`crate::std::GrpcResilientFacade::apply_resilience`] with this config.
//!
//! Uses millisecond integer fields for retry and seconds for the breaker
//! cool-down so the struct maps directly to TOML without serde helpers:
//!
//! ```toml
//! [grpc_resilience]
//! max_attempts                  = 3
//! initial_backoff_ms            = 100
//! backoff_multiplier            = 2.0
//! jitter_factor                 = 0.1
//! max_backoff_ms                = 2000
//! rate_limit_max_attempts       = 2
//! rate_limit_initial_backoff_ms = 1000
//! rate_limit_max_backoff_ms     = 10000
//! failure_threshold             = 5
//! cool_down_seconds             = 10
//! half_open_probe_count         = 1
//! ```
//!
//! ```compile_fail,E0451
//! use grpc_egress_resilient_api::ResilienceConfig;
//! let _ = ResilienceConfig { max_attempts: 0, ..ResilienceConfig::default() };
//! ```

use configbuilder::ConfigSection;
use grpc_egress_breaker_api::GrpcBreakerConfig;
use grpc_egress_retry_api::GrpcRetryConfig;
use serde::{Deserialize, Serialize};

/// Resilience policy for a single outbound gRPC channel.
///
/// Used by `GrpcResilientFacade::apply_resilience` (in `grpc-egress-resilient-spi`) to compose
/// a retry client and a circuit-breaker client around an already-built bare client (a `Transport` and a `RawTransport`)
/// transport client.
///
/// ## Retry tracks
///
/// Two independent tracks because rate-limit and capacity errors recover
/// on different time scales:
///
/// - **Standard** — for `UNAVAILABLE`, `ConnectionFailed`, and
///   `RESOURCE_EXHAUSTED(Capacity)`: fast exponential backoff.
/// - **Rate-limit** — for `RESOURCE_EXHAUSTED(RateLimit)`: slower,
///   separate attempt ceiling. When the upstream response carries a
///   `Retry-After` header the transport has embedded as
///   `[retry-after=Ns]`, that value overrides the computed backoff.
/// - **HardQuota** (`RESOURCE_EXHAUSTED` with billing / quota message):
///   never retried regardless of `max_attempts`.
///
/// ## Circuit breaker
///
/// Opens after `failure_threshold` consecutive post-retry transport
/// failures (`UNAVAILABLE`, `INTERNAL`, `ConnectionFailed`).  Stays
/// open for `cool_down_seconds`, then allows `half_open_probe_count`
/// consecutive probe successes to close it.
///
/// Note: `RESOURCE_EXHAUSTED` is **not** a breaker failure; it is
/// handled entirely by the retry layer.
///
/// ## Calibrated service profiles
///
/// Measured with in-process load tests (retry amplification ≤ 1.5× confirmed
/// at all three profiles). Choose the profile that matches your upstream.
///
/// **Fast stateless gRPC** (`vecdb`, `bm25`, `rrf` — sub-ms, no rate limits):
/// ```toml
/// max_attempts = 5; initial_backoff_ms = 100; backoff_multiplier = 2.0
/// jitter_factor = 0.1; max_backoff_ms = 5000
/// rate_limit_max_attempts = 2; rate_limit_initial_backoff_ms = 1000; rate_limit_max_backoff_ms = 10000
/// failure_threshold = 5; cool_down_seconds = 30; half_open_probe_count = 1
/// ```
/// This is [`ResilienceConfig::default()`].
///
/// **CPU-bound embedding** (`justembed`-class — single-threaded, slow recovery):
/// ```toml
/// max_attempts = 2; initial_backoff_ms = 200; backoff_multiplier = 2.0
/// jitter_factor = 0.2; max_backoff_ms = 3000
/// rate_limit_max_attempts = 2; rate_limit_initial_backoff_ms = 1000; rate_limit_max_backoff_ms = 10000
/// failure_threshold = 3; cool_down_seconds = 60; half_open_probe_count = 2
/// ```
/// Use a lower `failure_threshold` because recovery is slow; a longer `cool_down`
/// ensures the autoscaler has time to provision a new instance before the probe fires.
///
/// **External LLM APIs** (Anthropic-class — rate limits, variable latency):
/// ```toml
/// max_attempts = 2; initial_backoff_ms = 500; backoff_multiplier = 2.0
/// jitter_factor = 0.3; max_backoff_ms = 5000
/// rate_limit_max_attempts = 1; rate_limit_initial_backoff_ms = 5000; rate_limit_max_backoff_ms = 60000
/// failure_threshold = 5; cool_down_seconds = 60; half_open_probe_count = 1
/// ```
/// Set `rate_limit_max_attempts = 1` (no rate-limit retries) because the API
/// `Retry-After` window is typically 30-60 s — far beyond a typical call deadline.
/// Let the caller handle quota exhaustion instead of burning the deadline in a retry loop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ResilienceConfigData")]
pub struct ResilienceConfig {
    // ── Standard retry ────────────────────────────────────────────────────────
    /// Total attempts including the first call. `1` disables retry.
    max_attempts: u32,
    /// Wait before the first standard retry (ms).
    initial_backoff_ms: u64,
    /// Exponential growth factor per retry index.
    backoff_multiplier: f64,
    /// Jitter as a fraction of the computed backoff (0.0 = none, 0.1 = ±10%).
    jitter_factor: f64,
    /// Hard cap on standard retry backoff (ms).
    max_backoff_ms: u64,

    // ── Rate-limit retry track ────────────────────────────────────────────────
    /// Max attempts on the rate-limit track (usually lower than `max_attempts`).
    rate_limit_max_attempts: u32,
    /// Wait before the first rate-limit retry (ms). Overridden by
    /// `[retry-after=Ns]` hint when present.
    rate_limit_initial_backoff_ms: u64,
    /// Hard cap on rate-limit backoff (ms).
    rate_limit_max_backoff_ms: u64,

    // ── Circuit breaker ───────────────────────────────────────────────────────
    /// Whether the breaker is active; `false` bypasses admission and state updates.
    breaker_enabled: bool,
    /// Consecutive post-retry transport failures before opening the circuit.
    failure_threshold: u32,
    /// Seconds the circuit stays open before probing.
    cool_down_seconds: u64,
    /// Consecutive probe successes required in HalfOpen to close.
    half_open_probe_count: u32,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ResilienceConfigData {
    max_attempts: u32,
    initial_backoff_ms: u64,
    backoff_multiplier: f64,
    jitter_factor: f64,
    max_backoff_ms: u64,
    rate_limit_max_attempts: u32,
    rate_limit_initial_backoff_ms: u64,
    rate_limit_max_backoff_ms: u64,
    breaker_enabled: bool,
    failure_threshold: u32,
    cool_down_seconds: u64,
    half_open_probe_count: u32,
}

impl Default for ResilienceConfigData {
    fn default() -> Self {
        let v = ResilienceConfig::default();
        Self {
            max_attempts: v.max_attempts,
            initial_backoff_ms: v.initial_backoff_ms,
            backoff_multiplier: v.backoff_multiplier,
            jitter_factor: v.jitter_factor,
            max_backoff_ms: v.max_backoff_ms,
            rate_limit_max_attempts: v.rate_limit_max_attempts,
            rate_limit_initial_backoff_ms: v.rate_limit_initial_backoff_ms,
            rate_limit_max_backoff_ms: v.rate_limit_max_backoff_ms,
            breaker_enabled: v.breaker_enabled,
            failure_threshold: v.failure_threshold,
            cool_down_seconds: v.cool_down_seconds,
            half_open_probe_count: v.half_open_probe_count,
        }
    }
}

impl TryFrom<ResilienceConfigData> for ResilienceConfig {
    type Error = crate::ResilientTransportError;
    fn try_from(v: ResilienceConfigData) -> Result<Self, Self::Error> {
        let retry = GrpcRetryConfig::try_new(grpc_egress_retry_api::BuildGrpcRetryConfigRequest {
            max_attempts: v.max_attempts,
            initial_backoff_ms: v.initial_backoff_ms,
            backoff_multiplier: v.backoff_multiplier,
            jitter_factor: v.jitter_factor,
            max_backoff_ms: v.max_backoff_ms,
            rate_limit_max_attempts: v.rate_limit_max_attempts,
            rate_limit_initial_backoff_ms: v.rate_limit_initial_backoff_ms,
            rate_limit_max_backoff_ms: v.rate_limit_max_backoff_ms,
        })
        .map_err(|e| crate::ResilientTransportError::InvalidResilience(e.to_string()))?;
        let breaker = GrpcBreakerConfig::try_new(
            v.breaker_enabled,
            v.failure_threshold,
            v.cool_down_seconds,
            v.half_open_probe_count,
        )
        .map_err(|e| crate::ResilientTransportError::InvalidResilience(e.to_string()))?;
        Ok(Self::from_policies(retry, breaker))
    }
}

impl ResilienceConfig {
    /// Compose independently validated retry and breaker policies into one resilient policy.
    pub fn from_policies(retry: GrpcRetryConfig, breaker: GrpcBreakerConfig) -> Self {
        Self {
            max_attempts: retry.max_attempts(),
            initial_backoff_ms: retry.initial_backoff_ms(),
            backoff_multiplier: retry.backoff_multiplier(),
            jitter_factor: retry.jitter_factor(),
            max_backoff_ms: retry.max_backoff_ms(),
            rate_limit_max_attempts: retry.rate_limit_max_attempts(),
            rate_limit_initial_backoff_ms: retry.rate_limit_initial_backoff_ms(),
            rate_limit_max_backoff_ms: retry.rate_limit_max_backoff_ms(),
            breaker_enabled: breaker.enabled(),
            failure_threshold: breaker.failure_threshold(),
            cool_down_seconds: breaker.cool_down_seconds(),
            half_open_probe_count: breaker.half_open_probe_count(),
        }
    }
    /// Total standard attempts, including the initial call.
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }
    /// Initial standard retry delay in milliseconds.
    pub fn initial_backoff_ms(&self) -> u64 {
        self.initial_backoff_ms
    }
    /// Exponential backoff multiplier.
    pub fn backoff_multiplier(&self) -> f64 {
        self.backoff_multiplier
    }
    /// Retry jitter factor in the inclusive range from zero to one.
    pub fn jitter_factor(&self) -> f64 {
        self.jitter_factor
    }
    /// Maximum standard retry delay in milliseconds.
    pub fn max_backoff_ms(&self) -> u64 {
        self.max_backoff_ms
    }
    /// Maximum attempts for rate-limit responses.
    pub fn rate_limit_max_attempts(&self) -> u32 {
        self.rate_limit_max_attempts
    }
    /// Initial rate-limit delay in milliseconds.
    pub fn rate_limit_initial_backoff_ms(&self) -> u64 {
        self.rate_limit_initial_backoff_ms
    }
    /// Maximum rate-limit delay in milliseconds.
    pub fn rate_limit_max_backoff_ms(&self) -> u64 {
        self.rate_limit_max_backoff_ms
    }
    /// Whether the breaker admits calls and records outcomes.
    pub fn breaker_enabled(&self) -> bool {
        self.breaker_enabled
    }
    /// Consecutive failures required to open the circuit.
    pub fn failure_threshold(&self) -> u32 {
        self.failure_threshold
    }
    /// Open-circuit cool-down in seconds.
    pub fn cool_down_seconds(&self) -> u64 {
        self.cool_down_seconds
    }
    /// Successful half-open probes required to close the circuit.
    pub fn half_open_probe_count(&self) -> u32 {
        self.half_open_probe_count
    }
}

// Every instance is created from validated retry and breaker policies or validated deserialization.
impl Eq for ResilienceConfig {}

impl Default for ResilienceConfig {
    /// Returns the fast-stateless-gRPC profile: the calibrated baseline that
    /// confirmed ≤ 1.5× retry amplification in load tests.
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_backoff_ms: 5_000,
            rate_limit_max_attempts: 2,
            rate_limit_initial_backoff_ms: 1_000,
            rate_limit_max_backoff_ms: 10_000,
            breaker_enabled: true,
            failure_threshold: 5,
            cool_down_seconds: 30,
            half_open_probe_count: 1,
        }
    }
}

impl ConfigSection for ResilienceConfig {
    fn section_name() -> &'static str {
        "grpc_resilience"
    }
}
