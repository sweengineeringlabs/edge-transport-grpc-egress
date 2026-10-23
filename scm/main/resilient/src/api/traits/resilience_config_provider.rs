//! `ResilienceConfigProvider` -- parsing, validation and projection of a [`ResilienceConfig`].

use grpc_egress_breaker::GrpcBreakerConfig;
use grpc_egress_retry::GrpcRetryConfig;

use crate::{ResilienceConfig, ResilientTransportError};

/// Parsing, validation and projection of a [`ResilienceConfig`], which is plain data in this crate.
pub trait ResilienceConfigProvider {
    /// Parse a `[grpc_resilience]`-shaped config from TOML text, then validate it.
    ///
    /// # Errors
    ///
    /// [`ResilientTransportError::InvalidResilience`] when the text isn't valid TOML, a required key is missing, or a
    /// value is out of range.
    fn parse_config(&self, toml_text: &str) -> Result<ResilienceConfig, ResilientTransportError>;

    /// Validate that all numeric fields of `config` are within their valid ranges.
    ///
    /// # Errors
    ///
    /// [`ResilientTransportError::InvalidResilience`] naming the first field that is out of range.
    fn validate(&self, config: &ResilienceConfig) -> Result<(), ResilientTransportError>;

    /// Project the standard and rate-limit retry fields of `config` onto the retry crate's own config shape.
    fn to_retry_config(
        &self,
        config: &ResilienceConfig,
    ) -> Result<GrpcRetryConfig, ResilientTransportError>;

    /// Project the circuit-breaker fields of `config` onto the breaker crate's own config shape.
    fn to_breaker_config(
        &self,
        config: &ResilienceConfig,
    ) -> Result<GrpcBreakerConfig, ResilientTransportError>;
}
