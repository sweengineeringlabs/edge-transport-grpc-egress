//! `impl GrpcBreakerFacade` — composes this crate's default trait
//! implementations directly (no saf/ dependency, keeping core/ → saf/
//! import-free per the SEA dependency direction).

use std::sync::Arc;

use edge_transport_grpc_egress::GrpcEgress;

use crate::api::{
    ApplicationConfigBuilder, BreakerDecorator, BreakerDomainError, ConfigBuilderProvider,
    ConfigBuilderRequest, GrpcBreakerConfig, GrpcBreakerFacade, GrpcBreakerSvc, WrapBreakerRequest,
};
use crate::core::breaker::breaker_decorator::DefaultBreakerDecorator;

impl GrpcBreakerFacade {
    /// Return a config builder pre-seeded with this crate's name and version.
    pub fn create_config_builder() -> Result<ApplicationConfigBuilder, BreakerDomainError> {
        Ok(GrpcBreakerSvc
            .create_config_builder(ConfigBuilderRequest)?
            .builder)
    }

    /// Wrap `inner` with the supplied breaker policy.
    pub fn wrap_breaker<T: GrpcEgress + Send + Sync + 'static>(
        inner: T,
        config: GrpcBreakerConfig,
    ) -> Result<Arc<dyn GrpcEgress>, BreakerDomainError> {
        Ok(DefaultBreakerDecorator
            .wrap(WrapBreakerRequest { inner, config })?
            .client)
    }

    /// Wrap `inner` with the default breaker policy.
    pub fn create_breaker_client<T: GrpcEgress + Send + Sync + 'static>(
        inner: T,
    ) -> Result<Arc<dyn GrpcEgress>, BreakerDomainError> {
        Self::wrap_breaker(inner, GrpcBreakerConfig::default())
    }
}
