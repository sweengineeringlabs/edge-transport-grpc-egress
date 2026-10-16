//! [`ResilienceConfigProvider`] -- parsing, validation and projection of [`ResilienceConfig`], which is
//! plain data (with its defaults and trait impls) in the domain crate.

use grpc_egress_breaker::GrpcBreakerConfig;
use grpc_egress_resilient::{ResilienceConfig, ResilientTransportError};
use grpc_egress_retry::GrpcRetryConfig;

/// Parsing, validation and projection of a [`ResilienceConfig`]. Bring the trait into scope to call
/// `ResilienceConfig::from_config(text)` or `config.to_retry_config()`.
pub trait ResilienceConfigProvider: Sized {
    /// Parse a `[grpc_resilience]`-shaped config from TOML text, then validate it.
    ///
    /// Returns [`ResilientTransportError::InvalidResilience`] when the text isn't valid TOML, a
    /// required key is missing, or a value is out of range.
    fn from_config(toml_text: &str) -> Result<Self, ResilientTransportError>;

    /// Validate that all numeric fields are within their valid ranges.
    fn validate(&self) -> Result<(), ResilientTransportError>;

    /// Project the standard and rate-limit retry fields onto the retry crate's own config shape.
    fn to_retry_config(&self) -> GrpcRetryConfig;

    /// Project the circuit-breaker fields onto the breaker crate's own config shape.
    fn to_breaker_config(&self) -> GrpcBreakerConfig;
}

impl ResilienceConfigProvider for ResilienceConfig {
    /// Parse a `[grpc_resilience]`-shaped config from TOML text, then
    /// validate it.
    ///
    /// Returns [`ResilientTransportError::InvalidResilience`] when the text
    /// isn't valid TOML, a required key is missing, or a value is out of
    /// range.
    fn from_config(toml_text: &str) -> Result<Self, ResilientTransportError> {
        let cfg: Self = toml::from_str(toml_text).map_err(|e| {
            ResilientTransportError::InvalidResilience(format!("parse failed: {e}"))
        })?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Validate that all numeric fields are within their valid ranges.
    fn validate(&self) -> Result<(), ResilientTransportError> {
        if self.max_attempts == 0 {
            return Err(ResilientTransportError::InvalidResilience(
                "max_attempts must be >= 1".into(),
            ));
        }
        if self.rate_limit_max_attempts == 0 {
            return Err(ResilientTransportError::InvalidResilience(
                "rate_limit_max_attempts must be >= 1".into(),
            ));
        }
        if self.jitter_factor < 0.0 || self.jitter_factor > 1.0 {
            return Err(ResilientTransportError::InvalidResilience(format!(
                "jitter_factor must be in [0.0, 1.0], got {:.4}",
                self.jitter_factor
            )));
        }
        if self.half_open_probe_count == 0 {
            return Err(ResilientTransportError::InvalidResilience(
                "half_open_probe_count must be >= 1".into(),
            ));
        }
        if self.rate_limit_max_backoff_ms < self.rate_limit_initial_backoff_ms {
            return Err(ResilientTransportError::InvalidResilience(format!(
                "rate_limit_max_backoff_ms ({}) must be >= rate_limit_initial_backoff_ms ({})",
                self.rate_limit_max_backoff_ms, self.rate_limit_initial_backoff_ms
            )));
        }
        Ok(())
    }

    /// Project the standard + rate-limit retry fields onto the retry
    /// crate's own config shape.
    fn to_retry_config(&self) -> GrpcRetryConfig {
        GrpcRetryConfig {
            max_attempts: self.max_attempts,
            initial_backoff_ms: self.initial_backoff_ms,
            backoff_multiplier: self.backoff_multiplier,
            jitter_factor: self.jitter_factor,
            max_backoff_ms: self.max_backoff_ms,
            rate_limit_max_attempts: self.rate_limit_max_attempts,
            rate_limit_initial_backoff_ms: self.rate_limit_initial_backoff_ms,
            rate_limit_max_backoff_ms: self.rate_limit_max_backoff_ms,
        }
    }

    /// Project the circuit-breaker fields onto the breaker crate's own
    /// config shape.
    fn to_breaker_config(&self) -> GrpcBreakerConfig {
        GrpcBreakerConfig {
            failure_threshold: self.failure_threshold,
            cool_down_seconds: self.cool_down_seconds,
            half_open_probe_count: self.half_open_probe_count,
        }
    }
}
