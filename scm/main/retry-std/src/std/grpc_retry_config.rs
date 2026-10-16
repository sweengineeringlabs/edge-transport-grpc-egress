//! [`GrpcRetryConfigProvider`] -- parsing, derived durations and validation of [`GrpcRetryConfig`],
//! which is plain data (with its defaults and trait impls) in the domain crate.

use std::time::Duration;

use grpc_egress_retry::{Error, GrpcRetryConfig};

/// Parsing, derived durations and validation of a [`GrpcRetryConfig`]. Bring the trait into scope to
/// call `GrpcRetryConfig::from_config(text)` or `config.initial_backoff()`.
pub trait GrpcRetryConfigProvider: Sized {
    /// Parse a config from TOML text.
    ///
    /// Returns [`Error::ParseFailed`] when the text isn't valid TOML, when a required key is
    /// missing, or when an unknown key is present. Returns [`Error::InvalidConfig`] when a value
    /// is out of range (for example `backoff_multiplier <= 0.0`).
    fn from_config(toml_text: &str) -> Result<Self, Error>;

    /// Initial backoff as a [`Duration`].
    fn initial_backoff(&self) -> Duration;

    /// Maximum single-retry backoff as a [`Duration`].
    fn max_backoff(&self) -> Duration;

    /// Initial rate-limit backoff as a [`Duration`].
    fn rate_limit_initial_backoff(&self) -> Duration;

    /// Maximum rate-limit backoff as a [`Duration`].
    fn rate_limit_max_backoff(&self) -> Duration;

    /// Validate that all numeric fields are within their valid ranges.
    fn validate(&self) -> Result<(), Error>;
}

impl GrpcRetryConfigProvider for GrpcRetryConfig {
    /// Parse a config from TOML text.
    ///
    /// Returns [`Error::ParseFailed`] when the text isn't valid
    /// TOML, when a required key is missing, or when an unknown
    /// key is present (`deny_unknown_fields` is set).  Returns
    /// [`Error::InvalidConfig`] when a value is out of range
    /// (e.g. `backoff_multiplier <= 0.0`).
    fn from_config(toml_text: &str) -> Result<Self, Error> {
        let cfg: Self = toml::from_str(toml_text).map_err(|e| Error::ParseFailed(e.to_string()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Initial backoff as a [`Duration`].
    fn initial_backoff(&self) -> Duration {
        Duration::from_millis(self.initial_backoff_ms)
    }

    /// Maximum single-retry backoff as a [`Duration`].
    fn max_backoff(&self) -> Duration {
        Duration::from_millis(self.max_backoff_ms)
    }

    /// Initial rate-limit backoff as a [`Duration`].
    fn rate_limit_initial_backoff(&self) -> Duration {
        Duration::from_millis(self.rate_limit_initial_backoff_ms)
    }

    /// Maximum rate-limit backoff as a [`Duration`].
    fn rate_limit_max_backoff(&self) -> Duration {
        Duration::from_millis(self.rate_limit_max_backoff_ms)
    }

    /// Validate that all numeric fields are within their valid ranges.
    fn validate(&self) -> Result<(), Error> {
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
