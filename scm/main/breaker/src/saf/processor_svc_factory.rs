//! Composition site for [`Processor`] — one file per trait keeps wiring focused.

use crate::api::{GrpcBreakerSvc, Processor};

/// Factory for the default [`Processor`].
pub struct ProcessorFactory;

impl ProcessorFactory {
    /// Construct the default [`Processor`] — [`GrpcBreakerSvc`], which
    /// identifies this crate's middleware as `"grpc-breaker"`.
    pub fn create() -> Box<dyn Processor> {
        Box::new(GrpcBreakerSvc)
    }
}
