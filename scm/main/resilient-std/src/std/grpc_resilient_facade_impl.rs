//! `impl GrpcResilientFacade` — composes this crate's default trait
//! implementations directly (no saf/ dependency, keeping core/ → saf/
//! import-free per the SEA dependency direction).

use crate::std::ResilienceConfigProvider;
use crate::std::{GrpcResilientFacade, GrpcResilientSvcProcessor};
use grpc_egress_resilient::ApplicationConfigBuilder;

use std::sync::Arc;

use grpc_egress_breaker_std::GrpcBreakerFacade;
use grpc_egress_retry::{BuildGrpcRetryRequest, GrpcRetryFactory};
use grpc_egress_retry_std::StdGrpcRetryFactory;
use grpc_egress_transport::GrpcEgress;

use crate::std::default_validator::DefaultValidator;
use grpc_egress_resilient::{
    ConfigBuilderProvider, ConfigBuilderRequest, ConfigValidationRequest, ResilienceConfig,
    ResilientTransportError, Validator,
};

impl GrpcResilientFacade {
    /// Return a config builder pre-seeded with this crate's name and version.
    pub fn create_config_builder() -> Result<ApplicationConfigBuilder, ResilientTransportError> {
        Ok(GrpcResilientSvcProcessor
            .create_config_builder(ConfigBuilderRequest)?
            .builder)
    }

    /// Wrap an already-built, bare `inner` transport in the retry then
    /// circuit-breaker decorators, driven by `config`.
    ///
    /// This is a decorator only — it never builds the base transport. The
    /// composition root builds `inner` itself (e.g. via `transport`'s
    /// `create_tonic_client_from_config`) and applies this on top, per the
    /// one-composer model (ADR-004 amendment, edge-bootstrap ADR-008/010).
    pub fn apply_resilience<T: GrpcEgress + Send + Sync + 'static>(
        inner: T,
        config: ResilienceConfig,
    ) -> Result<Arc<dyn GrpcEgress>, ResilientTransportError> {
        DefaultValidator.validate(ConfigValidationRequest {
            config: config.clone(),
        })?;

        let with_retry = StdGrpcRetryFactory::new(config.to_retry_config())
            .build(BuildGrpcRetryRequest { inner });
        let with_breaker = GrpcBreakerFacade::wrap_breaker(with_retry, config.to_breaker_config())?;
        Ok(with_breaker)
    }
}
