//! Interface for obtaining a pre-seeded application config builder.

use crate::ConfigBuilderRequest;
use crate::ConfigBuilderResponse;

use crate::ResilientTransportError;

/// Contract for producing a config builder pre-populated with this crate's
/// name and version.
///
/// Implemented by `GrpcResilientSvcProcessor` of `grpc-egress-resilient-std`.
pub trait ConfigBuilderProvider: Send + Sync {
    /// Create a config builder pre-populated with this crate's name and version.
    fn create_config_builder(
        &self,
        req: ConfigBuilderRequest,
    ) -> Result<ConfigBuilderResponse, ResilientTransportError>;
}
