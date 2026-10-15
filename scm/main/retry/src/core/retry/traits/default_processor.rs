//! Default implementation of [`Processor`] and [`Validator`] for
//! the retry crate.
//!
//! [`DefaultProcessor`] validates a [`GrpcRetryConfig`] for correctness.

use crate::api::{Error, Processor, ProcessorRequest, ValidationRequest, Validator};

/// Default [`Processor`] and [`Validator`] implementation for the
/// gRPC retry crate.
pub(crate) struct DefaultProcessor;

impl Processor for DefaultProcessor {
    fn validate(&self, req: ProcessorRequest) -> Result<(), Error> {
        req.config.validate()
    }
}

impl Validator for DefaultProcessor {
    fn validate_config(&self, req: ValidationRequest) -> Result<(), Error> {
        req.config.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::GrpcRetryConfig;

    fn valid() -> GrpcRetryConfig {
        GrpcRetryConfig::default()
    }

    #[test]
    fn test_validate_valid_config_returns_ok() {
        assert!(DefaultProcessor
            .validate(ProcessorRequest { config: valid() })
            .is_ok());
        let mut invalid = valid();
        invalid.max_attempts = 0;
        assert!(DefaultProcessor
            .validate(ProcessorRequest { config: invalid })
            .is_err());
    }

    #[test]
    fn test_validate_config_valid_config_returns_ok() {
        assert!(DefaultProcessor
            .validate_config(ValidationRequest { config: valid() })
            .is_ok());
        let mut invalid = valid();
        invalid.max_attempts = 0;
        assert!(DefaultProcessor
            .validate_config(ValidationRequest { config: invalid })
            .is_err());
    }
}
