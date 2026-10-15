//! Struct declaration and constructors for [`BearerEgressInterceptor`].

use crate::api::types::bearer_egress_config::BearerEgressConfig;

/// `GrpcEgressInterceptor` that signs and attaches a JWT bearer token.
pub struct BearerEgressInterceptor {
    pub(crate) config: BearerEgressConfig,
}

impl BearerEgressInterceptor {
    /// Construct from config.
    pub fn from_config(config: BearerEgressConfig) -> Self {
        Self { config }
    }
}
