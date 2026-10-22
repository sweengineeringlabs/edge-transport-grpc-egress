//! gRPC circuit-breaker policy schema.
//!
//! Policy **values**: the baseline is [`GrpcBreakerConfig::default`]; a consumer's own TOML goes through
//! `GrpcBreakerConfig::from_config` of `grpc-egress-breaker-std`.

use serde::Deserialize;

/// gRPC circuit-breaker policy schema.
///
/// Construct via [`GrpcBreakerConfig::default`] for SWE baseline values or
/// `GrpcBreakerConfig::from_config` (in `grpc-egress-breaker-std`) to parse custom TOML. Parsing
/// validates the constraints (`failure_threshold >= 1`, `half_open_probe_count >= 1`).
///
/// # Examples
///
/// ```rust
/// use grpc_egress_breaker::GrpcBreakerConfig;
///
/// // SWE baseline: 5 failures, 30s cool-down, 1 probe to close.
/// let cfg = GrpcBreakerConfig::default();
/// assert_eq!(cfg.failure_threshold, 5);
/// assert_eq!(cfg.cool_down_seconds, 30);
///
/// // A custom policy.
/// let cfg = GrpcBreakerConfig { failure_threshold: 3, cool_down_seconds: 10, half_open_probe_count: 2 };
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

impl Default for GrpcBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            cool_down_seconds: 30,
            half_open_probe_count: 1,
        }
    }
}

impl swe_edge_configbuilder::ConfigSection for GrpcBreakerConfig {
    fn section_name() -> &'static str {
        "grpc_breaker"
    }
}

impl From<GrpcBreakerConfig> for edge_transport_breaker_policy::BreakerConfig {
    fn from(cfg: GrpcBreakerConfig) -> Self {
        Self {
            failure_threshold: cfg.failure_threshold,
            cool_down_seconds: cfg.cool_down_seconds,
            half_open_probe_count: cfg.half_open_probe_count,
        }
    }
}
