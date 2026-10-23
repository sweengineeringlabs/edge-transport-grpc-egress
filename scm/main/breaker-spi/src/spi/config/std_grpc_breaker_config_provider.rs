//! `StdGrpcBreakerConfigProvider` -- parsing (with `toml`) and the derived cool-down of a [`GrpcBreakerConfig`].

use std::time::Duration;

use grpc_egress_breaker_api::{Error, GrpcBreakerConfig, GrpcBreakerConfigProvider};

/// Parsing, the derived cool-down and validation of a [`GrpcBreakerConfig`].
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StdGrpcBreakerConfigProvider;

impl GrpcBreakerConfigProvider for StdGrpcBreakerConfigProvider {
    fn parse_config(&self, toml_text: &str) -> Result<GrpcBreakerConfig, Error> {
        let cfg: GrpcBreakerConfig =
            toml::from_str(toml_text).map_err(|e| Error::ParseFailed(e.to_string()))?;
        self.validate(&cfg)?;
        Ok(cfg)
    }

    fn cool_down(&self, config: &GrpcBreakerConfig) -> Duration {
        Duration::from_secs(config.cool_down_seconds())
    }

    fn validate(&self, config: &GrpcBreakerConfig) -> Result<(), Error> {
        if config.failure_threshold() == 0 {
            return Err(Error::InvalidConfig(
                "failure_threshold must be >= 1".into(),
            ));
        }
        if config.half_open_probe_count() == 0 {
            return Err(Error::InvalidConfig(
                "half_open_probe_count must be >= 1".into(),
            ));
        }
        Ok(())
    }
}
