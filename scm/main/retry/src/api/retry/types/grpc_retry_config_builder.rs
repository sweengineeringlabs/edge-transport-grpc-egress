//! Builder for [`GrpcRetryConfig`] — fluent API for test and integration use.
//!
//! Use this when you need to construct a `GrpcRetryConfig` programmatically
//! without a TOML file. In production, prefer [`GrpcRetryConfig::from_config`].
//!
//! Per SEA rule 160, the type *declaration* lives in api/. The methods
//! (including `new`, per-field overrides, and `build`) live in `core/`.

/// Fluent builder for [`GrpcRetryConfig`](crate::api::GrpcRetryConfig).
///
/// All fields are optional and default to the SWE baseline values.
/// Call [`GrpcRetryConfigBuilder::build`] to validate and produce a
/// [`GrpcRetryConfig`](crate::api::GrpcRetryConfig).
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
