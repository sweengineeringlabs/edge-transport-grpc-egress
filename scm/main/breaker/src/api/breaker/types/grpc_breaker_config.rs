//! gRPC circuit-breaker policy schema.
//!
//! Policy **values** live in TOML:
//! - crate-shipped baseline: `config/application.toml`
//! - consumer override: whatever TOML the binary loads and
//!   passes to [`GrpcBreakerConfig::from_config`](crate::saf::GrpcBreakerConfig::from_config).

use serde::Deserialize;

/// gRPC circuit-breaker policy schema.
///
/// Construct via [`GrpcBreakerConfig::default`] for SWE baseline values or
/// `GrpcBreakerConfig::from_config` (in `core::grpc_breaker_config`) to parse
/// custom TOML. Both validate the constraints (`failure_threshold >= 1`,
/// `half_open_probe_count >= 1`).
///
/// # Examples
///
/// ```rust
/// use edge_transport_grpc_egress_breaker::GrpcBreakerConfig;
/// use std::time::Duration;
///
/// // SWE baseline: 5 failures, 30s cool-down, 1 probe to close.
/// let cfg = GrpcBreakerConfig::default();
/// assert_eq!(cfg.failure_threshold, 5);
/// assert_eq!(cfg.cool_down(), Duration::from_secs(30));
///
/// // Custom policy from TOML.
/// let cfg = GrpcBreakerConfig::from_config(
///     "failure_threshold = 3\ncool_down_seconds = 10\nhalf_open_probe_count = 2"
/// ).unwrap();
/// assert_eq!(cfg.failure_threshold, 3);
/// assert_eq!(cfg.half_open_probe_count, 2);
/// ```
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrpcBreakerConfig {
    /// Consecutive failures that trip the breaker open.
    pub failure_threshold: u32,

    /// Seconds to wait in Open state before the next request
    /// promotes to HalfOpen.
    pub cool_down_seconds: u64,

    /// Consecutive probe successes required in HalfOpen to
    /// close the breaker.
    pub half_open_probe_count: u32,
}
