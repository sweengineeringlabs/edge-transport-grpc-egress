//! `StdResilienceConfigProvider` -- parsing (with `toml`), validation and projection of a [`ResilienceConfig`].

use grpc_egress_breaker_api::GrpcBreakerConfig;
use grpc_egress_resilient_api::{
    ResilienceConfig, ResilienceConfigProvider, ResilientTransportError,
};
use grpc_egress_retry_api::GrpcRetryConfig;

/// Parsing, validation and projection of a [`ResilienceConfig`].
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StdResilienceConfigProvider;

impl ResilienceConfigProvider for StdResilienceConfigProvider {
    fn parse_config(&self, toml_text: &str) -> Result<ResilienceConfig, ResilientTransportError> {
        let cfg: ResilienceConfig = toml::from_str(toml_text).map_err(|e| {
            ResilientTransportError::InvalidResilience(format!("parse failed: {e}"))
        })?;
        self.validate(&cfg)?;
        Ok(cfg)
    }

    /// Validate that all numeric fields are within their valid ranges.
    fn validate(&self, config: &ResilienceConfig) -> Result<(), ResilientTransportError> {
        if config.max_attempts() == 0 {
            return Err(ResilientTransportError::InvalidResilience(
                "max_attempts must be >= 1".into(),
            ));
        }
        if config.rate_limit_max_attempts() == 0 {
            return Err(ResilientTransportError::InvalidResilience(
                "rate_limit_max_attempts must be >= 1".into(),
            ));
        }
        if config.jitter_factor() < 0.0 || config.jitter_factor() > 1.0 {
            return Err(ResilientTransportError::InvalidResilience(format!(
                "jitter_factor must be in [0.0, 1.0], got {:.4}",
                config.jitter_factor()
            )));
        }
        if config.half_open_probe_count() == 0 {
            return Err(ResilientTransportError::InvalidResilience(
                "half_open_probe_count must be >= 1".into(),
            ));
        }
        if config.rate_limit_max_backoff_ms() < config.rate_limit_initial_backoff_ms() {
            return Err(ResilientTransportError::InvalidResilience(format!(
                "rate_limit_max_backoff_ms ({}) must be >= rate_limit_initial_backoff_ms ({})",
                config.rate_limit_max_backoff_ms(),
                config.rate_limit_initial_backoff_ms()
            )));
        }
        Ok(())
    }

    /// crate's own config shape.
    fn to_retry_config(
        &self,
        config: &ResilienceConfig,
    ) -> Result<GrpcRetryConfig, ResilientTransportError> {
        GrpcRetryConfig::try_new(grpc_egress_retry_api::BuildGrpcRetryConfigRequest {
            max_attempts: config.max_attempts(),
            initial_backoff_ms: config.initial_backoff_ms(),
            backoff_multiplier: config.backoff_multiplier(),
            jitter_factor: config.jitter_factor(),
            max_backoff_ms: config.max_backoff_ms(),
            rate_limit_max_attempts: config.rate_limit_max_attempts(),
            rate_limit_initial_backoff_ms: config.rate_limit_initial_backoff_ms(),
            rate_limit_max_backoff_ms: config.rate_limit_max_backoff_ms(),
        })
        .map_err(|error| ResilientTransportError::InvalidResilience(error.to_string()))
    }

    /// config shape.
    fn to_breaker_config(
        &self,
        config: &ResilienceConfig,
    ) -> Result<GrpcBreakerConfig, ResilientTransportError> {
        GrpcBreakerConfig::try_new(
            config.breaker_enabled(),
            config.failure_threshold(),
            config.cool_down_seconds(),
            config.half_open_probe_count(),
        )
        .map_err(|error| ResilientTransportError::InvalidResilience(error.to_string()))
    }
}
