//! Composition site for [`Validator`] — one file per trait keeps wiring focused.

use crate::api::Validator;
use crate::core::breaker::default_validator::DefaultValidator;

/// Factory for the default [`Validator`].
pub struct ValidatorFactory;

impl ValidatorFactory {
    /// Construct the default [`Validator`] for [`GrpcBreakerConfig`](crate::api::GrpcBreakerConfig).
    pub fn create() -> Box<dyn Validator> {
        Box::new(DefaultValidator)
    }
}
