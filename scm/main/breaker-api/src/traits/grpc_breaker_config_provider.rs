//! `GrpcBreakerConfigProvider` -- parsing and the derived cool-down of a [`GrpcBreakerConfig`].

use std::time::Duration;

use crate::{Error, GrpcBreakerConfig};

/// Parsing, the derived cool-down and validation of a [`GrpcBreakerConfig`], which is plain data in this crate.
pub trait GrpcBreakerConfigProvider {
    /// Parse a config from TOML text, then validate it.
    ///
    /// # Errors
    ///
    /// [`Error::ParseFailed`] when the text is not a valid policy, [`Error::InvalidConfig`] when it cannot work.
    fn parse_config(&self, toml_text: &str) -> Result<GrpcBreakerConfig, Error>;

    /// The cool-down of `config` as a [`Duration`].
    fn cool_down(&self, config: &GrpcBreakerConfig) -> Duration;

    /// Refuse a policy the breaker cannot work with: a failure threshold or a probe count of zero.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidConfig`] naming the field.
    fn validate(&self, config: &GrpcBreakerConfig) -> Result<(), Error>;
}
