//! Composition site for [`Validator`] — one file per trait keeps wiring focused.

use crate::api::Validator;
use crate::core::retry::traits::default_processor::DefaultProcessor;

/// Factory for the default [`Validator`].
pub struct ValidatorFactory;

impl ValidatorFactory {
    /// Construct the default [`Validator`] for [`crate::api::GrpcRetryConfig`].
    pub fn create() -> Box<dyn Validator> {
        Box::new(DefaultProcessor)
    }
}
