//! Composition site for [`Processor`] — one file per trait keeps wiring focused.

use crate::api::{GrpcResilientSvcProcessor, Processor};

/// Factory for the default [`Processor`].
pub struct ProcessorFactory;

impl ProcessorFactory {
    /// Construct the default [`Processor`] — [`GrpcResilientSvcProcessor`], which
    /// identifies this crate's middleware as `"grpc-resilient"`.
    pub fn create() -> Box<dyn Processor> {
        Box::new(GrpcResilientSvcProcessor)
    }
}
