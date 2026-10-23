//! `StdGrpcBreakerConfigProviderFactory` -- creates the reader of breaker policies.

use grpc_egress_breaker_api::{
    BuildGrpcBreakerConfigProviderRequest, GrpcBreakerConfigProvider,
    GrpcBreakerConfigProviderFactory,
};

use crate::spi::config::StdGrpcBreakerConfigProvider;

/// Creates providers that read breaker policies from TOML text.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdGrpcBreakerConfigProviderFactory;

impl GrpcBreakerConfigProviderFactory for StdGrpcBreakerConfigProviderFactory {
    fn build(
        &self,
        _req: BuildGrpcBreakerConfigProviderRequest,
    ) -> impl GrpcBreakerConfigProvider + use<> {
        StdGrpcBreakerConfigProvider
    }
}
