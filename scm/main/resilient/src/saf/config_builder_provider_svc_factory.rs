//! Composition site for [`ConfigBuilderProvider`] — one file per trait keeps wiring focused.

use crate::std::GrpcResilientSvcProcessor;

use crate::api::ConfigBuilderProvider;

/// Factory for the default [`ConfigBuilderProvider`].
pub struct ConfigBuilderProviderFactory;

impl ConfigBuilderProviderFactory {
    /// Construct the default [`ConfigBuilderProvider`] — [`GrpcResilientSvcProcessor`].
    pub fn create() -> Box<dyn ConfigBuilderProvider> {
        Box::new(GrpcResilientSvcProcessor)
    }
}
