//! Interface for obtaining a pre-seeded application config builder.

use crate::BreakerDomainError;
use crate::ConfigBuilderRequest;
use crate::ConfigBuilderResponse;

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
}
