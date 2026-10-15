//! Composition site for [`RetryDecorator`] — one file per trait keeps wiring focused.

use crate::api::RetryDecorator;
use crate::core::retry::grpc::default_retry_decorator::DefaultRetryDecorator;

/// Factory for the default [`RetryDecorator`].
pub struct RetryDecoratorFactory;

impl RetryDecoratorFactory {
    /// Construct the default [`RetryDecorator`].
    pub fn create() -> Box<dyn RetryDecorator> {
        Box::new(DefaultRetryDecorator)
    }
}
