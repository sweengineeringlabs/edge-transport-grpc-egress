//! Interface for obtaining a pre-seeded application config builder.

use crate::api::ConfigBuilderRequest;
use crate::api::ConfigBuilderResponse;
use crate::api::GrpcResilientSvcProcessor;
use crate::api::ResilientTransportError;

/// Contract for producing a config builder pre-populated with this crate's
/// name and version.
///
/// Implemented by [`GrpcResilientSvcProcessor`] in `core/`.
pub trait ConfigBuilderProvider: Send + Sync {
    /// Create a config builder pre-populated with this crate's name and version.
    fn create_config_builder(
        &self,
        req: ConfigBuilderRequest,
    ) -> Result<ConfigBuilderResponse, ResilientTransportError>;

    /// Construct the default provider — gives [`GrpcResilientSvcProcessor`] a genuine
    /// role in this trait's signature set, not just an impl-site `Self`.
    /// `Self: Sized` keeps this trait dyn-compatible for `Box<dyn Trait>`.
    fn default_provider() -> GrpcResilientSvcProcessor
    where
        Self: Sized,
    {
        GrpcResilientSvcProcessor
    }
}
