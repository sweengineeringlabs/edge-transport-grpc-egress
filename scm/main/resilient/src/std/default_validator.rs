//! `impl Validator for DefaultValidator` — validates this crate's own
//! [`crate::api::ResilienceConfig`], the sole owner of this shape (ADR-004).

use crate::api::{ConfigValidationRequest, ResilientTransportError, Validator};

/// Default [`Validator`] implementation for [`crate::api::ResilienceConfig`].
pub(crate) struct DefaultValidator;

impl Validator for DefaultValidator {
    fn validate(&self, req: ConfigValidationRequest) -> Result<(), ResilientTransportError> {
        req.config.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ResilienceConfig;

    fn valid() -> ResilienceConfig {
        ResilienceConfig {
            max_attempts: 3,
            initial_backoff_ms: 10,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            max_backoff_ms: 100,
            rate_limit_max_attempts: 2,
            rate_limit_initial_backoff_ms: 10,
            rate_limit_max_backoff_ms: 100,
            failure_threshold: 3,
            cool_down_seconds: 10,
            half_open_probe_count: 1,
        }
    }

    #[test]
    fn test_validate_valid_config_returns_ok() {
        assert!(DefaultValidator
            .validate(ConfigValidationRequest { config: valid() })
            .is_ok());
        // Sibling negative case in the same test: a single field flipped to
        // invalid on an otherwise-valid config must fail, proving is_ok()
        // above isn't just a stub that always succeeds regardless of input.
        let mut invalid = valid();
        invalid.max_attempts = 0;
        assert!(DefaultValidator
            .validate(ConfigValidationRequest { config: invalid })
            .is_err());
    }

    #[test]
    fn test_validate_rejects_zero_max_attempts() {
        let mut cfg = valid();
        cfg.max_attempts = 0;
        let err = DefaultValidator
            .validate(ConfigValidationRequest { config: cfg })
            .unwrap_err();
        assert!(matches!(err, ResilientTransportError::InvalidResilience(_)));
    }
}
