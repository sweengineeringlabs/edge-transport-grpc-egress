//! Composition site for [`ConfigBuilderProvider`] — one file per trait keeps wiring focused.

use crate::api::{ConfigBuilderProvider, GrpcBreakerSvc};

/// Factory for the default [`ConfigBuilderProvider`].
pub struct ConfigBuilderProviderFactory;

impl ConfigBuilderProviderFactory {
    /// Construct the default [`ConfigBuilderProvider`] — [`GrpcBreakerSvc`].
    pub fn create() -> Box<dyn ConfigBuilderProvider> {
        Box::new(GrpcBreakerSvc)
    }
}
