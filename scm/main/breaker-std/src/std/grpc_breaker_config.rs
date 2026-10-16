//! [`GrpcBreakerConfigProvider`] -- parsing and the derived cool-down of [`GrpcBreakerConfig`], which is
//! plain data (with its defaults and trait impls) in the domain crate.

use std::time::Duration;

use grpc_egress_breaker::{ConfigValidationRequest, Error, GrpcBreakerConfig, Validator};

use crate::std::default_validator::DefaultValidator;

/// Parsing and the derived cool-down of a [`GrpcBreakerConfig`]. Bring the trait into scope to call
/// `GrpcBreakerConfig::from_config(text)` or `config.cool_down()`.
pub trait GrpcBreakerConfigProvider: Sized {
    /// Parse a config from TOML text.
    fn from_config(toml_text: &str) -> Result<Self, Error>;

    /// Cool-down as a [`Duration`].
    fn cool_down(&self) -> Duration;
}

impl GrpcBreakerConfigProvider for GrpcBreakerConfig {
    /// Parse a config from TOML text.
    fn from_config(toml_text: &str) -> Result<Self, Error> {
        let cfg: Self = toml::from_str(toml_text).map_err(|e| Error::ParseFailed(e.to_string()))?;
        DefaultValidator.validate(ConfigValidationRequest {
            config: cfg.clone(),
        })?;
        Ok(cfg)
    }

    /// Cool-down as a [`Duration`].
    fn cool_down(&self) -> Duration {
        Duration::from_secs(self.cool_down_seconds)
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
