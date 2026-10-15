//! `Default`, `ConfigSection`, and inherent methods for [`GrpcBreakerConfig`].

use std::time::Duration;

use crate::api::{ConfigValidationRequest, Error, GrpcBreakerConfig, Validator};
use crate::core::breaker::default_validator::DefaultValidator;

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
        // @allow: no_stub_fn_bodies
        "grpc_breaker"
    }
}

impl GrpcBreakerConfig {
    /// Parse a config from TOML text.
    pub fn from_config(toml_text: &str) -> Result<Self, Error> {
        let cfg: Self = toml::from_str(toml_text).map_err(|e| Error::ParseFailed(e.to_string()))?;
        DefaultValidator.validate(ConfigValidationRequest {
            config: cfg.clone(),
        })?;
        Ok(cfg)
    }

    /// Cool-down as a [`Duration`].
    pub fn cool_down(&self) -> Duration {
        Duration::from_secs(self.cool_down_seconds)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_has_positive_baseline_values() {
        let cfg = GrpcBreakerConfig::default();
        assert_eq!(cfg.failure_threshold, 5);
        assert_eq!(cfg.cool_down_seconds, 30);
        assert_eq!(cfg.half_open_probe_count, 1);
    }

    /// @covers: cool_down
    #[test]
    fn test_cool_down_converts_seconds_to_duration() {
        let cfg = GrpcBreakerConfig {
            failure_threshold: 1,
            cool_down_seconds: 42,
            half_open_probe_count: 1,
        };
        assert_eq!(cfg.cool_down(), Duration::from_secs(42));
    }

    /// @covers: from_config
    #[test]
    fn test_from_config_rejects_invalid_toml() {
        let err = GrpcBreakerConfig::from_config("not valid toml {{{").unwrap_err();
        assert!(matches!(err, Error::ParseFailed(_)));
    }
}
