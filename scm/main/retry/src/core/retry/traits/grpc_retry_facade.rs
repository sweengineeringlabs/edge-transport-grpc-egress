//! `impl GrpcRetryFacade` — composes this crate's default trait
//! implementations directly (no saf/ dependency, keeping core/ → saf/
//! import-free per the SEA dependency direction).

use edge_transport_grpc_egress::GrpcEgress;

use crate::api::{
    ApplicationConfigBuilder, ConfigBuilderProvider, ConfigBuilderRequest, Error, GrpcRetryClient,
    GrpcRetryConfig, GrpcRetryFacade, GrpcRetrySvc,
};

impl GrpcRetryFacade {
    /// Return a config builder pre-seeded with this crate's name and version.
    pub fn create_config_builder() -> Result<ApplicationConfigBuilder, Error> {
        Ok(GrpcRetrySvc
            .create_config_builder(ConfigBuilderRequest)?
            .builder)
    }

    /// Wrap `inner` with the supplied retry policy.
    pub fn wrap_retry<T: GrpcEgress + Send + Sync + 'static>(
        inner: T,
        config: GrpcRetryConfig,
    ) -> GrpcRetryClient<T> {
        GrpcRetryClient::new(inner, config)
    }

    /// Wrap `inner` with the default retry policy.
    pub fn create_retry_client<T: GrpcEgress + Send + Sync + 'static>(
        inner: T,
    ) -> GrpcRetryClient<T> {
        GrpcRetryClient::new(inner, GrpcRetryConfig::default())
    }
}
