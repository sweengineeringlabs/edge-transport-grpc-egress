//! gRPC circuit-breaker policy schema.
//!
//! Policy **values**: the baseline is [`GrpcBreakerConfig::default`]; a consumer's own TOML goes through
//! `GrpcBreakerConfig::from_config` of `grpc-egress-breaker-std`.

use serde::Deserialize;

/// gRPC circuit-breaker policy schema.
///
/// Construct via [`GrpcBreakerConfig::try_new`] or [`GrpcBreakerConfig::default`]. TOML deserialization validates the
/// same constraints (`failure_threshold >= 1`, `half_open_probe_count >= 1`).
///
/// # Examples
///
/// ```rust
/// use grpc_egress_breaker::GrpcBreakerConfig;
///
/// // SWE baseline: 5 failures, 30s cool-down, 1 probe to close.
/// let cfg = GrpcBreakerConfig::default();
/// assert_eq!(cfg.failure_threshold(), 5);
/// assert_eq!(cfg.cool_down_seconds(), 30);
///
/// // A custom policy.
/// let cfg = GrpcBreakerConfig::try_new(true, 3, 10, 2).unwrap();
/// assert_eq!(cfg.failure_threshold(), 3);
/// assert_eq!(cfg.half_open_probe_count(), 2);
/// ```
///
/// ```compile_fail,E0451
/// use grpc_egress_breaker::GrpcBreakerConfig;
/// let _ = GrpcBreakerConfig { failure_threshold: 0, ..GrpcBreakerConfig::default() };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "GrpcBreakerConfigData")]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct GrpcBreakerConfig {
    /// Whether calls are admitted and outcomes affect breaker state. `false` bypasses the breaker entirely.
    enabled: bool,
    /// Consecutive failures that trip the breaker open.
    failure_threshold: u32,

    /// Seconds to wait in Open state before the next request
    /// promotes to HalfOpen.
    cool_down_seconds: u64,

    /// Consecutive probe successes required in HalfOpen to
    /// close the breaker.
    half_open_probe_count: u32,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct GrpcBreakerConfigData {
    enabled: bool,
    failure_threshold: u32,
    cool_down_seconds: u64,
    half_open_probe_count: u32,
}

impl Default for GrpcBreakerConfigData {
    fn default() -> Self {
        let config = GrpcBreakerConfig::default();
        Self {
            enabled: config.enabled,
            failure_threshold: config.failure_threshold,
            cool_down_seconds: config.cool_down_seconds,
            half_open_probe_count: config.half_open_probe_count,
        }
    }
}

impl TryFrom<GrpcBreakerConfigData> for GrpcBreakerConfig {
    type Error = crate::Error;

    fn try_from(data: GrpcBreakerConfigData) -> Result<Self, Self::Error> {
        Self::try_new(
            data.enabled,
            data.failure_threshold,
            data.cool_down_seconds,
            data.half_open_probe_count,
        )
    }
}

impl GrpcBreakerConfig {
    /// Construct a validated breaker policy.
    pub fn try_new(
        enabled: bool,
        failure_threshold: u32,
        cool_down_seconds: u64,
        half_open_probe_count: u32,
    ) -> Result<Self, crate::Error> {
        if failure_threshold == 0 {
            return Err(crate::Error::InvalidConfig(
                "failure_threshold must be >= 1".into(),
            ));
        }
        if half_open_probe_count == 0 {
            return Err(crate::Error::InvalidConfig(
                "half_open_probe_count must be >= 1".into(),
            ));
        }
        Ok(Self {
            enabled,
            failure_threshold,
            cool_down_seconds,
            half_open_probe_count,
        })
    }

    /// Whether calls bypass breaker admission and state transitions.
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    /// Consecutive failures that trip the breaker open.
    pub fn failure_threshold(&self) -> u32 {
        self.failure_threshold
    }
    /// Seconds to wait in Open state before the next request.
    pub fn cool_down_seconds(&self) -> u64 {
        self.cool_down_seconds
    }
    /// Consecutive probe successes required in HalfOpen to close the breaker.
    pub fn half_open_probe_count(&self) -> u32 {
        self.half_open_probe_count
    }
}

impl Default for GrpcBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
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
