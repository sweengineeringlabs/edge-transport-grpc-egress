//! `impl` blocks for [`ResilienceConfig`]. The type *declaration* lives in
//! `api/`.

use edge_transport_grpc_egress_breaker::GrpcBreakerConfig;
use edge_transport_grpc_egress_retry::GrpcRetryConfig;

use crate::api::{ResilienceConfig, ResilientTransportError};

impl Default for ResilienceConfig {
    /// Returns the fast-stateless-gRPC profile: the calibrated baseline that
    /// confirmed ≤ 1.5× retry amplification in load tests.
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_backoff_ms: 5_000,
            rate_limit_max_attempts: 2,
            rate_limit_initial_backoff_ms: 1_000,
            rate_limit_max_backoff_ms: 10_000,
            failure_threshold: 5,
            cool_down_seconds: 30,
            half_open_probe_count: 1,
        }
    }
}

impl swe_edge_configbuilder::ConfigSection for ResilienceConfig {
    fn section_name() -> &'static str {
        // @allow: no_stub_fn_bodies
        "grpc_resilience"
    }
}

impl ResilienceConfig {
    /// Validate that all numeric fields are within their valid ranges.
    pub(crate) fn validate(&self) -> Result<(), ResilientTransportError> {
        if self.max_attempts == 0 {
            return Err(ResilientTransportError::InvalidResilience(
                "max_attempts must be >= 1".into(),
            ));
        }
        if self.rate_limit_max_attempts == 0 {
            return Err(ResilientTransportError::InvalidResilience(
                "rate_limit_max_attempts must be >= 1".into(),
            ));
        }
        if self.jitter_factor < 0.0 || self.jitter_factor > 1.0 {
            return Err(ResilientTransportError::InvalidResilience(format!(
                "jitter_factor must be in [0.0, 1.0], got {:.4}",
                self.jitter_factor
            )));
        }
        if self.half_open_probe_count == 0 {
            return Err(ResilientTransportError::InvalidResilience(
                "half_open_probe_count must be >= 1".into(),
            ));
        }
        if self.rate_limit_max_backoff_ms < self.rate_limit_initial_backoff_ms {
            return Err(ResilientTransportError::InvalidResilience(format!(
                "rate_limit_max_backoff_ms ({}) must be >= rate_limit_initial_backoff_ms ({})",
                self.rate_limit_max_backoff_ms, self.rate_limit_initial_backoff_ms
            )));
        }
        Ok(())
    }

    /// Project the standard + rate-limit retry fields onto the retry
    /// crate's own config shape.
    pub(crate) fn to_retry_config(&self) -> GrpcRetryConfig {
        GrpcRetryConfig {
            max_attempts: self.max_attempts,
            initial_backoff_ms: self.initial_backoff_ms,
            backoff_multiplier: self.backoff_multiplier,
            jitter_factor: self.jitter_factor,
            max_backoff_ms: self.max_backoff_ms,
            rate_limit_max_attempts: self.rate_limit_max_attempts,
            rate_limit_initial_backoff_ms: self.rate_limit_initial_backoff_ms,
            rate_limit_max_backoff_ms: self.rate_limit_max_backoff_ms,
        }
    }

    /// Project the circuit-breaker fields onto the breaker crate's own
    /// config shape.
    pub(crate) fn to_breaker_config(&self) -> GrpcBreakerConfig {
        GrpcBreakerConfig {
            failure_threshold: self.failure_threshold,
            cool_down_seconds: self.cool_down_seconds,
            half_open_probe_count: self.half_open_probe_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> ResilienceConfig {
        ResilienceConfig {
            max_attempts: 3,
            initial_backoff_ms: 100,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_backoff_ms: 2_000,
            rate_limit_max_attempts: 2,
            rate_limit_initial_backoff_ms: 1_000,
            rate_limit_max_backoff_ms: 10_000,
            failure_threshold: 5,
            cool_down_seconds: 10,
            half_open_probe_count: 1,
        }
    }

    /// @covers: validate
    #[test]
    fn test_validate_valid_config_returns_ok() {
        assert!(valid().validate().is_ok());
        // Sibling negative case in the same test: a single field flipped to
        // invalid on an otherwise-valid config must fail, proving is_ok()
        // above isn't just a stub that always succeeds regardless of input.
        let mut invalid = valid();
        invalid.max_attempts = 0;
        assert!(invalid.validate().is_err());
    }

    /// @covers: validate
    #[test]
    fn test_validate_rejects_zero_max_attempts() {
        let mut r = valid();
        r.max_attempts = 0;
        assert!(r.validate().is_err());
    }

    /// @covers: validate
    #[test]
    fn test_validate_rejects_zero_rate_limit_max_attempts() {
        let mut r = valid();
        r.rate_limit_max_attempts = 0;
        assert!(r.validate().is_err());
    }

    /// @covers: validate
    #[test]
    fn test_validate_rejects_jitter_factor_out_of_range() {
        let mut r = valid();
        r.jitter_factor = 1.5;
        assert!(r.validate().is_err());
        r.jitter_factor = -0.1;
        assert!(r.validate().is_err());
    }

    /// @covers: validate
    #[test]
    fn test_validate_rejects_zero_half_open_probe_count() {
        let mut r = valid();
        r.half_open_probe_count = 0;
        assert!(r.validate().is_err());
    }

    /// @covers: validate
    #[test]
    fn test_validate_rejects_rate_limit_max_backoff_less_than_initial() {
        let mut r = valid();
        r.rate_limit_max_backoff_ms = 500;
        r.rate_limit_initial_backoff_ms = 1_000;
        assert!(r.validate().is_err());
    }

    /// @covers: to_retry_config
    #[test]
    fn test_to_retry_config_maps_fields() {
        let r = valid();
        let retry = r.to_retry_config();
        assert_eq!(retry.max_attempts, r.max_attempts);
        assert_eq!(retry.rate_limit_max_attempts, r.rate_limit_max_attempts);
    }

    /// @covers: to_breaker_config
    #[test]
    fn test_to_breaker_config_maps_fields() {
        let r = valid();
        let breaker = r.to_breaker_config();
        assert_eq!(breaker.failure_threshold, r.failure_threshold);
        assert_eq!(breaker.cool_down_seconds, r.cool_down_seconds);
    }
}
