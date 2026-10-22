//! `GrpcRetryConfigBuilder` -- the fluent construction of a [`GrpcRetryConfig`].

use crate::{Error, GrpcRetryConfig};

/// Collects the fields of a retry policy and builds the [`GrpcRetryConfig`]. A builder starts at the baseline
/// values ([`GrpcRetryConfig::default`]); a method overrides one field and returns the builder.
pub trait GrpcRetryConfigBuilder: Sized {
    /// Override `max_attempts`.
    fn max_attempts(self, v: u32) -> Self;

    /// Override `initial_backoff_ms`.
    fn initial_backoff_ms(self, v: u64) -> Self;

    /// Override `backoff_multiplier`.
    fn backoff_multiplier(self, v: f64) -> Self;

    /// Override `jitter_factor`.
    fn jitter_factor(self, v: f64) -> Self;

    /// Override `max_backoff_ms`.
    fn max_backoff_ms(self, v: u64) -> Self;

    /// Override `rate_limit_max_attempts`.
    fn rate_limit_max_attempts(self, v: u32) -> Self;

    /// Override `rate_limit_initial_backoff_ms`.
    fn rate_limit_initial_backoff_ms(self, v: u64) -> Self;

    /// Override `rate_limit_max_backoff_ms`.
    fn rate_limit_max_backoff_ms(self, v: u64) -> Self;

    /// Validate and produce a [`GrpcRetryConfig`].
    ///
    /// # Errors
    ///
    /// [`Error::InvalidConfig`] when any field is out of range.
    fn build(self) -> Result<GrpcRetryConfig, Error>;
}
