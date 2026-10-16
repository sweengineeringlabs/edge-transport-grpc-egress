//! Composition site for [`Processor`] — one file per trait keeps wiring focused.

use crate::std::default_processor::DefaultProcessor;
use grpc_egress_retry::Processor;

/// Factory for the default [`Processor`].
pub struct ProcessorFactory;

impl ProcessorFactory {
    /// Construct the default [`Processor`] for the gRPC retry crate.
    pub fn create() -> Box<dyn Processor> {
        Box::new(DefaultProcessor)
    }
}
