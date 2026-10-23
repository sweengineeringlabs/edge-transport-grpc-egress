//! `GrpcRetryConfigProvider` -- parsing, derived durations and validation of a [`GrpcRetryConfig`].

use std::time::Duration;

use crate::{Error, GrpcRetryConfig};

/// Parsing, derived durations and validation of a [`GrpcRetryConfig`], which is plain data in this crate.
pub trait GrpcRetryConfigProvider {
    /// Parse a config from TOML text, then validate it.
    ///
    /// # Errors
    ///
    /// [`Error::ParseFailed`] when the text isn't valid TOML, when a required key is missing, or when an unknown key is
    /// present. [`Error::InvalidConfig`] when a value is out of range (for example `backoff_multiplier <= 0.0`).
    fn parse_config(&self, toml_text: &str) -> Result<GrpcRetryConfig, Error>;

    /// Initial backoff of `config` as a [`Duration`].
    fn initial_backoff(&self, config: &GrpcRetryConfig) -> Duration;

    /// Maximum single-retry backoff of `config` as a [`Duration`].
    fn max_backoff(&self, config: &GrpcRetryConfig) -> Duration;

    /// Initial rate-limit backoff of `config` as a [`Duration`].
    fn rate_limit_initial_backoff(&self, config: &GrpcRetryConfig) -> Duration;

    /// Maximum rate-limit backoff of `config` as a [`Duration`].
    fn rate_limit_max_backoff(&self, config: &GrpcRetryConfig) -> Duration;

    /// Validate that all numeric fields of `config` are within their valid ranges.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidConfig`] naming the first field that is out of range.
    fn validate(&self, config: &GrpcRetryConfig) -> Result<(), Error>;
}
