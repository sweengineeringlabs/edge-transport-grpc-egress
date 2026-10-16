//! Builder for [`GrpcRetryConfig`] — fluent API for test and integration use.
//!
//! Use this when you need to construct a `GrpcRetryConfig` programmatically
//! without a TOML file. In production, prefer [`GrpcRetryConfig::from_config`].

use crate::std::GrpcRetryConfigProvider;
use grpc_egress_retry::{Error, GrpcRetryConfig};

/// Fluent builder for [`GrpcRetryConfig`](grpc_egress_retry::GrpcRetryConfig).
///
/// All fields are optional and default to the SWE baseline values.
/// Call [`GrpcRetryConfigBuilder::build`] to validate and produce a
/// [`GrpcRetryConfig`](grpc_egress_retry::GrpcRetryConfig).
///
/// # Example
///
/// ```ignore
/// let config = GrpcRetryConfigBuilder::new()
///     .max_attempts(3)
///     .initial_backoff_ms(50)
///     .build()
///     .expect("valid config");
/// ```
pub struct GrpcRetryConfigBuilder {
    pub(crate) max_attempts: u32,
    pub(crate) initial_backoff_ms: u64,
    pub(crate) backoff_multiplier: f64,
    pub(crate) jitter_factor: f64,
    pub(crate) max_backoff_ms: u64,
    pub(crate) rate_limit_max_attempts: u32,
    pub(crate) rate_limit_initial_backoff_ms: u64,
    pub(crate) rate_limit_max_backoff_ms: u64,
}

impl GrpcRetryConfigBuilder {
    /// Create a builder pre-seeded with the SWE default values.
    pub fn new() -> Self {
        let d = GrpcRetryConfig::default();
        Self {
            max_attempts: d.max_attempts,
            initial_backoff_ms: d.initial_backoff_ms,
            backoff_multiplier: d.backoff_multiplier,
            jitter_factor: d.jitter_factor,
            max_backoff_ms: d.max_backoff_ms,
            rate_limit_max_attempts: d.rate_limit_max_attempts,
            rate_limit_initial_backoff_ms: d.rate_limit_initial_backoff_ms,
            rate_limit_max_backoff_ms: d.rate_limit_max_backoff_ms,
        }
    }

    /// Override `max_attempts`.
    pub fn max_attempts(mut self, v: u32) -> Self {
        self.max_attempts = v;
        self
    }

    /// Override `initial_backoff_ms`.
    pub fn initial_backoff_ms(mut self, v: u64) -> Self {
        self.initial_backoff_ms = v;
        self
    }

    /// Override `backoff_multiplier`.
    pub fn backoff_multiplier(mut self, v: f64) -> Self {
        self.backoff_multiplier = v;
        self
    }

    /// Override `jitter_factor`.
    pub fn jitter_factor(mut self, v: f64) -> Self {
        self.jitter_factor = v;
        self
    }

    /// Override `max_backoff_ms`.
    pub fn max_backoff_ms(mut self, v: u64) -> Self {
        self.max_backoff_ms = v;
        self
    }

    /// Override `rate_limit_max_attempts`.
    pub fn rate_limit_max_attempts(mut self, v: u32) -> Self {
        self.rate_limit_max_attempts = v;
        self
    }

    /// Override `rate_limit_initial_backoff_ms`.
    pub fn rate_limit_initial_backoff_ms(mut self, v: u64) -> Self {
        self.rate_limit_initial_backoff_ms = v;
        self
    }

    /// Override `rate_limit_max_backoff_ms`.
    pub fn rate_limit_max_backoff_ms(mut self, v: u64) -> Self {
        self.rate_limit_max_backoff_ms = v;
        self
    }

    /// Validate and produce a [`GrpcRetryConfig`].
    ///
    /// Returns [`Error::InvalidConfig`] when any field is out of range.
    pub fn build(self) -> Result<GrpcRetryConfig, Error> {
        let cfg = GrpcRetryConfig {
            max_attempts: self.max_attempts,
            initial_backoff_ms: self.initial_backoff_ms,
            backoff_multiplier: self.backoff_multiplier,
            jitter_factor: self.jitter_factor,
            max_backoff_ms: self.max_backoff_ms,
            rate_limit_max_attempts: self.rate_limit_max_attempts,
            rate_limit_initial_backoff_ms: self.rate_limit_initial_backoff_ms,
            rate_limit_max_backoff_ms: self.rate_limit_max_backoff_ms,
        };
        cfg.validate()?;
        Ok(cfg)
    }
}

impl Default for GrpcRetryConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
