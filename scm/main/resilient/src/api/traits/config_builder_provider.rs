//! Interface for obtaining a pre-seeded application config builder.

use crate::api::ConfigBuilderRequest;
use crate::api::ConfigBuilderResponse;

use crate::api::ResilientTransportError;

/// Contract for producing a config builder pre-populated with this crate's
/// name and version.
///
/// Implemented by [`crate::std::GrpcResilientSvcProcessor`].
pub trait ConfigBuilderProvider: Send + Sync {
    /// Create a config builder pre-populated with this crate's name and version.
    fn create_config_builder(
        &self,
        req: ConfigBuilderRequest,
    ) -> Result<ConfigBuilderResponse, ResilientTransportError>;
}
