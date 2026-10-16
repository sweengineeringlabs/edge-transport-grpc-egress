//! Composition site for [`ConfigBuilderProvider`] — one file per trait keeps wiring focused.

use crate::std::GrpcRetrySvc;

use grpc_egress_retry::ConfigBuilderProvider;

/// Factory for the default [`ConfigBuilderProvider`].
pub struct ConfigBuilderProviderFactory;

impl ConfigBuilderProviderFactory {
    /// Construct the default [`ConfigBuilderProvider`] — [`GrpcRetrySvc`].
    pub fn create() -> Box<dyn ConfigBuilderProvider> {
        Box::new(GrpcRetrySvc)
    }
}
