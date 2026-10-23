//! `StdGrpcRetryConfigProviderFactory` -- creates the reader of retry policies.

use grpc_egress_retry_api::{
    BuildGrpcRetryConfigProviderRequest, GrpcRetryConfigProvider, GrpcRetryConfigProviderFactory,
};

use crate::spi::config::StdGrpcRetryConfigProvider;

/// Creates providers that read retry policies from TOML text.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdGrpcRetryConfigProviderFactory;

impl GrpcRetryConfigProviderFactory for StdGrpcRetryConfigProviderFactory {
    fn build(
        &self,
        _req: BuildGrpcRetryConfigProviderRequest,
    ) -> impl GrpcRetryConfigProvider + use<> {
        StdGrpcRetryConfigProvider
    }
}
