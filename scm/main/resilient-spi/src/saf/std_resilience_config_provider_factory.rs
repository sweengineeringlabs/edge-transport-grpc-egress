//! `StdResilienceConfigProviderFactory` -- creates the reader of resilience policies.

use grpc_egress_resilient::{
    BuildResilienceConfigProviderRequest, ResilienceConfigProvider, ResilienceConfigProviderFactory,
};

use crate::spi::config::StdResilienceConfigProvider;

/// Creates providers that read resilience policies from TOML text.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdResilienceConfigProviderFactory;

impl ResilienceConfigProviderFactory for StdResilienceConfigProviderFactory {
    fn build(
        &self,
        _req: BuildResilienceConfigProviderRequest,
    ) -> impl ResilienceConfigProvider + use<> {
        StdResilienceConfigProvider
    }
}
