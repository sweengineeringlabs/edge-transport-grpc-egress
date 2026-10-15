//! Validator implementation for BearerEgressConfig.

use crate::api::{BearerAuthError, BearerEgressConfig, Validator};

impl Validator for BearerEgressConfig {
    fn validate(&self) -> Result<(), BearerAuthError> {
        if self.issuer.is_empty() {
            return Err(BearerAuthError::ValidationFailed(
                "issuer must not be empty".into(),
            ));
        }
        if self.audience.is_empty() {
            return Err(BearerAuthError::ValidationFailed(
                "audience must not be empty".into(),
            ));
        }
        if self.subject.is_empty() {
            return Err(BearerAuthError::ValidationFailed(
                "subject must not be empty".into(),
            ));
        }
        Ok(())
    }

    fn from_config(config: BearerEgressConfig) -> Self {
        config
    }
}
