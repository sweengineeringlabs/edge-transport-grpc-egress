//! `impl` blocks for [`GrpcRetryConfig`] — parsing, derived durations,
//! and validation. The type *declaration* lives in `api/`.

use std::time::Duration;

use crate::api::{Error, GrpcRetryConfig};

impl Default for GrpcRetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_backoff_ms: 5000,
            rate_limit_max_attempts: 2,
            rate_limit_initial_backoff_ms: 1000,
            rate_limit_max_backoff_ms: 10000,
        }
    }
}

impl swe_edge_configbuilder::ConfigSection for GrpcRetryConfig {
    fn section_name() -> &'static str {
        // @allow: no_stub_fn_bodies
        "grpc_retry"
    }
}

impl edge_transport_retry_policy::BackoffPolicy for GrpcRetryConfig {
    fn initial_backoff_ms(&self) -> u64 {
        self.initial_backoff_ms
    }

    fn max_backoff_ms(&self) -> u64 {
        self.max_backoff_ms
    }

    fn backoff_multiplier(&self) -> f64 {
        self.backoff_multiplier
    }

    fn jitter_factor(&self) -> f64 {
        self.jitter_factor
    }
}

impl edge_transport_retry_policy::RateLimitBackoffPolicy for GrpcRetryConfig {
    fn rate_limit_max_attempts(&self) -> u32 {
        self.rate_limit_max_attempts
    }

    fn rate_limit_initial_backoff_ms(&self) -> u64 {
        self.rate_limit_initial_backoff_ms
    }

    fn rate_limit_max_backoff_ms(&self) -> u64 {
        self.rate_limit_max_backoff_ms
    }
}

impl GrpcRetryConfig {
    /// Parse a config from TOML text.
    ///
    /// Returns [`Error::ParseFailed`] when the text isn't valid
    /// TOML, when a required key is missing, or when an unknown
    /// key is present (`deny_unknown_fields` is set).  Returns
    /// [`Error::InvalidConfig`] when a value is out of range
    /// (e.g. `backoff_multiplier <= 0.0`).
    pub fn from_config(toml_text: &str) -> Result<Self, Error> {
        let cfg: Self = toml::from_str(toml_text).map_err(|e| Error::ParseFailed(e.to_string()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Initial backoff as a [`Duration`].
    pub fn initial_backoff(&self) -> Duration {
        Duration::from_millis(self.initial_backoff_ms)
    }

    /// Maximum single-retry backoff as a [`Duration`].
    pub fn max_backoff(&self) -> Duration {
        Duration::from_millis(self.max_backoff_ms)
    }

    /// Initial rate-limit backoff as a [`Duration`].
    pub fn rate_limit_initial_backoff(&self) -> Duration {
        Duration::from_millis(self.rate_limit_initial_backoff_ms)
    }

    /// Maximum rate-limit backoff as a [`Duration`].
    pub fn rate_limit_max_backoff(&self) -> Duration {
        Duration::from_millis(self.rate_limit_max_backoff_ms)
    }

    /// Validate that all numeric fields are within their valid ranges.
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.max_attempts == 0 {
            return Err(Error::InvalidConfig("max_attempts must be >= 1".into()));
        }
        if self.backoff_multiplier <= 0.0 || !self.backoff_multiplier.is_finite() {
            return Err(Error::InvalidConfig(
                "backoff_multiplier must be a positive finite number".into(),
            ));
        }
        if !(0.0..=1.0).contains(&self.jitter_factor) || !self.jitter_factor.is_finite() {
            return Err(Error::InvalidConfig(
                "jitter_factor must be in [0.0, 1.0]".into(),
            ));
        }
        if self.max_backoff_ms < self.initial_backoff_ms {
            return Err(Error::InvalidConfig(
                "max_backoff_ms must be >= initial_backoff_ms".into(),
            ));
        }
        if self.rate_limit_max_attempts == 0 {
            return Err(Error::InvalidConfig(
                "rate_limit_max_attempts must be >= 1".into(),
            ));
        }
        if self.rate_limit_max_backoff_ms < self.rate_limit_initial_backoff_ms {
            return Err(Error::InvalidConfig(
                "rate_limit_max_backoff_ms must be >= rate_limit_initial_backoff_ms".into(),
            ));
        }
        Ok(())
    }
}
