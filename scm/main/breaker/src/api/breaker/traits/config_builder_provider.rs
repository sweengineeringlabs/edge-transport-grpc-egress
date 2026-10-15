//! Interface for obtaining a pre-seeded application config builder.

use crate::api::BreakerDomainError;
use crate::api::ConfigBuilderRequest;
use crate::api::ConfigBuilderResponse;
use crate::api::GrpcBreakerSvc;

/// Contract for producing a config builder pre-populated with this crate's
/// name and version.
///
/// Implemented by [`GrpcBreakerSvc`] in `core::provider::config_builder_provider`.
pub trait ConfigBuilderProvider: Send + Sync {
    /// Create a config builder pre-populated with this crate's name and version.
    fn create_config_builder(
        &self,
        req: ConfigBuilderRequest,
    ) -> Result<ConfigBuilderResponse, BreakerDomainError>;

    /// Construct the default provider — gives [`GrpcBreakerSvc`] a genuine
    /// role in this trait's signature set, not just an impl-site `Self`.
    /// `Self: Sized` keeps this trait dyn-compatible for `Box<dyn Trait>`.
    fn default_provider() -> GrpcBreakerSvc
    where
        Self: Sized,
    {
        GrpcBreakerSvc
    }
}
