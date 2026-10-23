//! `StdGrpcRetryConfigBuilder` -- the standard [`GrpcRetryConfigBuilder`].

use grpc_egress_retry::{Error, GrpcRetryConfig, GrpcRetryConfigBuilder, GrpcRetryConfigProvider};

use super::std_grpc_retry_config_provider::StdGrpcRetryConfigProvider;

/// Fluent builder of a [`GrpcRetryConfig`]. All fields default to the baseline values.
#[derive(Debug)]
pub(crate) struct StdGrpcRetryConfigBuilder {
    max_attempts: u32,
    initial_backoff_ms: u64,
    backoff_multiplier: f64,
    jitter_factor: f64,
    max_backoff_ms: u64,
    rate_limit_max_attempts: u32,
    rate_limit_initial_backoff_ms: u64,
    rate_limit_max_backoff_ms: u64,
}

impl Default for StdGrpcRetryConfigBuilder {
    /// A builder pre-seeded with the baseline values.
    fn default() -> Self {
        let config = GrpcRetryConfig::default();
        Self {
            max_attempts: config.max_attempts(),
            initial_backoff_ms: config.initial_backoff_ms(),
            backoff_multiplier: config.backoff_multiplier(),
            jitter_factor: config.jitter_factor(),
            max_backoff_ms: config.max_backoff_ms(),
            rate_limit_max_attempts: config.rate_limit_max_attempts(),
            rate_limit_initial_backoff_ms: config.rate_limit_initial_backoff_ms(),
            rate_limit_max_backoff_ms: config.rate_limit_max_backoff_ms(),
        }
    }
}

impl GrpcRetryConfigBuilder for StdGrpcRetryConfigBuilder {
    fn max_attempts(mut self, v: u32) -> Self {
        self.max_attempts = v;
        self
    }

    fn initial_backoff_ms(mut self, v: u64) -> Self {
        self.initial_backoff_ms = v;
        self
    }

    fn backoff_multiplier(mut self, v: f64) -> Self {
        self.backoff_multiplier = v;
        self
    }

    fn jitter_factor(mut self, v: f64) -> Self {
        self.jitter_factor = v;
        self
    }

    fn max_backoff_ms(mut self, v: u64) -> Self {
        self.max_backoff_ms = v;
        self
    }

    fn rate_limit_max_attempts(mut self, v: u32) -> Self {
        self.rate_limit_max_attempts = v;
        self
    }

    fn rate_limit_initial_backoff_ms(mut self, v: u64) -> Self {
        self.rate_limit_initial_backoff_ms = v;
        self
    }

    fn rate_limit_max_backoff_ms(mut self, v: u64) -> Self {
        self.rate_limit_max_backoff_ms = v;
        self
    }

    fn build(self) -> Result<GrpcRetryConfig, Error> {
        let config = GrpcRetryConfig::try_new(grpc_egress_retry::BuildGrpcRetryConfigRequest {
            max_attempts: self.max_attempts,
            initial_backoff_ms: self.initial_backoff_ms,
            backoff_multiplier: self.backoff_multiplier,
            jitter_factor: self.jitter_factor,
            max_backoff_ms: self.max_backoff_ms,
            rate_limit_max_attempts: self.rate_limit_max_attempts,
            rate_limit_initial_backoff_ms: self.rate_limit_initial_backoff_ms,
            rate_limit_max_backoff_ms: self.rate_limit_max_backoff_ms,
        })?;
        StdGrpcRetryConfigProvider.validate(&config)?;
        Ok(config)
    }
}
