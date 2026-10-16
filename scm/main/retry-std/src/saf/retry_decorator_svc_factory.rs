//! Composition site for [`RetryDecorator`] — one file per trait keeps wiring focused.

use crate::std::default_retry_decorator::DefaultRetryDecorator;
use grpc_egress_retry::RetryDecorator;

/// Factory for the default [`RetryDecorator`].
pub struct RetryDecoratorFactory;

impl RetryDecoratorFactory {
    /// Construct the default [`RetryDecorator`].
    pub fn create() -> Box<dyn RetryDecorator> {
        Box::new(DefaultRetryDecorator)
    }
}
