//! `StdGrpcRetryConfigBuilder` -- the standard [`GrpcRetryConfigBuilder`].

use grpc_egress_retry::{Error, GrpcRetryConfig, GrpcRetryConfigBuilder, GrpcRetryConfigProvider};

use super::std_grpc_retry_config_provider::StdGrpcRetryConfigProvider;

/// Fluent builder of a [`GrpcRetryConfig`]. All fields default to the baseline values.
#[derive(Debug)]
pub(crate) struct StdGrpcRetryConfigBuilder {
    config: GrpcRetryConfig,
}

impl Default for StdGrpcRetryConfigBuilder {
    /// A builder pre-seeded with the baseline values.
    fn default() -> Self {
        Self {
            config: GrpcRetryConfig::default(),
        }
    }
}

impl GrpcRetryConfigBuilder for StdGrpcRetryConfigBuilder {
    fn max_attempts(mut self, v: u32) -> Self {
        self.config.max_attempts = v;
        self
    }

    fn initial_backoff_ms(mut self, v: u64) -> Self {
        self.config.initial_backoff_ms = v;
        self
    }

    fn backoff_multiplier(mut self, v: f64) -> Self {
        self.config.backoff_multiplier = v;
        self
    }

    fn jitter_factor(mut self, v: f64) -> Self {
        self.config.jitter_factor = v;
        self
    }

    fn max_backoff_ms(mut self, v: u64) -> Self {
        self.config.max_backoff_ms = v;
        self
    }

    fn rate_limit_max_attempts(mut self, v: u32) -> Self {
        self.config.rate_limit_max_attempts = v;
        self
    }

    fn rate_limit_initial_backoff_ms(mut self, v: u64) -> Self {
        self.config.rate_limit_initial_backoff_ms = v;
        self
    }

    fn rate_limit_max_backoff_ms(mut self, v: u64) -> Self {
        self.config.rate_limit_max_backoff_ms = v;
        self
    }

    fn build(self) -> Result<GrpcRetryConfig, Error> {
        StdGrpcRetryConfigProvider.validate(&self.config)?;
        Ok(self.config)
    }
}
