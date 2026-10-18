//! `StdGrpcRetryConfigProvider` -- parsing (with `toml`), derived durations and validation of a [`GrpcRetryConfig`].

use std::time::Duration;

use grpc_egress_retry::{Error, GrpcRetryConfig, GrpcRetryConfigProvider};

/// Parsing, derived durations and validation of a [`GrpcRetryConfig`].
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StdGrpcRetryConfigProvider;

impl GrpcRetryConfigProvider for StdGrpcRetryConfigProvider {
    fn parse_config(&self, toml_text: &str) -> Result<GrpcRetryConfig, Error> {
        let cfg: GrpcRetryConfig =
            toml::from_str(toml_text).map_err(|e| Error::ParseFailed(e.to_string()))?;
        self.validate(&cfg)?;
        Ok(cfg)
    }

    fn initial_backoff(&self, config: &GrpcRetryConfig) -> Duration {
        Duration::from_millis(config.initial_backoff_ms)
    }

    fn max_backoff(&self, config: &GrpcRetryConfig) -> Duration {
        Duration::from_millis(config.max_backoff_ms)
    }

    fn rate_limit_initial_backoff(&self, config: &GrpcRetryConfig) -> Duration {
        Duration::from_millis(config.rate_limit_initial_backoff_ms)
    }

    fn rate_limit_max_backoff(&self, config: &GrpcRetryConfig) -> Duration {
        Duration::from_millis(config.rate_limit_max_backoff_ms)
    }

    fn validate(&self, config: &GrpcRetryConfig) -> Result<(), Error> {
        if config.max_attempts == 0 {
            return Err(Error::InvalidConfig("max_attempts must be >= 1".into()));
        }
        if config.backoff_multiplier <= 0.0 || !config.backoff_multiplier.is_finite() {
            return Err(Error::InvalidConfig(
                "backoff_multiplier must be a positive finite number".into(),
            ));
        }
        if !(0.0..=1.0).contains(&config.jitter_factor) || !config.jitter_factor.is_finite() {
            return Err(Error::InvalidConfig(
                "jitter_factor must be in [0.0, 1.0]".into(),
            ));
        }
        if config.max_backoff_ms < config.initial_backoff_ms {
            return Err(Error::InvalidConfig(
                "max_backoff_ms must be >= initial_backoff_ms".into(),
            ));
        }
        if config.rate_limit_max_attempts == 0 {
            return Err(Error::InvalidConfig(
                "rate_limit_max_attempts must be >= 1".into(),
            ));
        }
        if config.rate_limit_max_backoff_ms < config.rate_limit_initial_backoff_ms {
            return Err(Error::InvalidConfig(
                "rate_limit_max_backoff_ms must be >= rate_limit_initial_backoff_ms".into(),
            ));
        }
        Ok(())
    }
}
