//! `impl Validator for DefaultValidator` — circuit-breaker config validation.

use crate::api::{ConfigValidationRequest, Error, Validator};

/// Default [`Validator`] implementation for [`GrpcBreakerConfig`](crate::api::GrpcBreakerConfig).
pub(crate) struct DefaultValidator;

impl Validator for DefaultValidator {
    fn validate(&self, req: ConfigValidationRequest) -> Result<(), Error> {
        let config = req.config;
        if config.failure_threshold == 0 {
            return Err(Error::InvalidConfig(
                "failure_threshold must be >= 1".into(),
            ));
        }
        if config.half_open_probe_count == 0 {
            return Err(Error::InvalidConfig(
                "half_open_probe_count must be >= 1".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::GrpcBreakerConfig;

    fn valid() -> GrpcBreakerConfig {
        GrpcBreakerConfig {
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
        invalid.failure_threshold = 0;
        assert!(DefaultValidator
            .validate(ConfigValidationRequest { config: invalid })
            .is_err());
    }

    #[test]
    fn test_validate_rejects_zero_failure_threshold() {
        let mut cfg = valid();
        cfg.failure_threshold = 0;
        let err = DefaultValidator
            .validate(ConfigValidationRequest { config: cfg })
            .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig(_)));
    }

    #[test]
    fn test_validate_rejects_zero_half_open_probe_count() {
        let mut cfg = valid();
        cfg.half_open_probe_count = 0;
        let err = DefaultValidator
            .validate(ConfigValidationRequest { config: cfg })
            .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig(_)));
    }
}
