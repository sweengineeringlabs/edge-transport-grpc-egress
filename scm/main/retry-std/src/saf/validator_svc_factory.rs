//! Composition site for [`Validator`] — one file per trait keeps wiring focused.

use crate::std::default_processor::DefaultProcessor;
use grpc_egress_retry::Validator;

/// Factory for the default [`Validator`].
pub struct ValidatorFactory;

impl ValidatorFactory {
    /// Construct the default [`Validator`] for [`grpc_egress_retry::GrpcRetryConfig`].
    pub fn create() -> Box<dyn Validator> {
        Box::new(DefaultProcessor)
    }
}
