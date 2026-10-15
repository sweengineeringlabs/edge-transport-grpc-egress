//! Resilience configuration — retry + circuit breaker as a first-class,
//! composer-facing policy. This crate is the sole owner of this shape:
//! `transport`'s `GrpcChannelConfig` carries no resilience-shaped field
//! (ADR-004) — a composition root builds a bare client, then applies
//! [`crate::api::GrpcResilientFacade::apply_resilience`] with this config.
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

use serde::{Deserialize, Serialize};

/// Resilience policy for a single outbound gRPC channel.
///
/// Used by [`crate::api::GrpcResilientFacade::apply_resilience`] to compose
/// an [`edge_transport_grpc_egress_retry::GrpcRetryClient`] and
/// [`edge_transport_grpc_egress_breaker::GrpcBreakerClient`] around an
/// already-built bare [`GrpcEgress`](edge_transport_grpc_egress::GrpcEgress)
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResilienceConfig {
    // ── Standard retry ────────────────────────────────────────────────────────
    /// Total attempts including the first call. `1` disables retry.
    pub max_attempts: u32,
    /// Wait before the first standard retry (ms).
    pub initial_backoff_ms: u64,
    /// Exponential growth factor per retry index.
    pub backoff_multiplier: f64,
    /// Jitter as a fraction of the computed backoff (0.0 = none, 0.1 = ±10%).
    pub jitter_factor: f64,
    /// Hard cap on standard retry backoff (ms).
    pub max_backoff_ms: u64,

    // ── Rate-limit retry track ────────────────────────────────────────────────
    /// Max attempts on the rate-limit track (usually lower than `max_attempts`).
    pub rate_limit_max_attempts: u32,
    /// Wait before the first rate-limit retry (ms). Overridden by
    /// `[retry-after=Ns]` hint when present.
    pub rate_limit_initial_backoff_ms: u64,
    /// Hard cap on rate-limit backoff (ms).
    pub rate_limit_max_backoff_ms: u64,

    // ── Circuit breaker ───────────────────────────────────────────────────────
    /// Consecutive post-retry transport failures before opening the circuit.
    pub failure_threshold: u32,
    /// Seconds the circuit stays open before probing.
    pub cool_down_seconds: u64,
    /// Consecutive probe successes required in HalfOpen to close.
    pub half_open_probe_count: u32,
}
