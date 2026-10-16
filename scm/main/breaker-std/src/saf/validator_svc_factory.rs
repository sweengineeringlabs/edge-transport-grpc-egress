//! Composition site for [`Validator`] — one file per trait keeps wiring focused.

use crate::std::default_validator::DefaultValidator;
use grpc_egress_breaker::Validator;

/// Factory for the default [`Validator`].
pub struct ValidatorFactory;

impl ValidatorFactory {
    /// Construct the default [`Validator`] for [`GrpcBreakerConfig`](grpc_egress_breaker::GrpcBreakerConfig).
    pub fn create() -> Box<dyn Validator> {
        Box::new(DefaultValidator)
    }
}
