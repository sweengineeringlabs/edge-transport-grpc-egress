//! Composition site for [`Processor`] — one file per trait keeps wiring focused.

use crate::api::Processor;
use crate::core::retry::traits::default_processor::DefaultProcessor;

/// Factory for the default [`Processor`].
pub struct ProcessorFactory;

impl ProcessorFactory {
    /// Construct the default [`Processor`] for the gRPC retry crate.
    pub fn create() -> Box<dyn Processor> {
        Box::new(DefaultProcessor)
    }
}
