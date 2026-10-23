//! `GrpcBreakerConfigProviderFactory` -- creates the reader of breaker policies.

use crate::{BuildGrpcBreakerConfigProviderRequest, GrpcBreakerConfigProvider};

/// Creates a [`GrpcBreakerConfigProvider`].
pub trait GrpcBreakerConfigProviderFactory {
    /// A provider that reads policies from TOML text.
    fn build(
        &self,
        req: BuildGrpcBreakerConfigProviderRequest,
    ) -> impl GrpcBreakerConfigProvider + use<Self>;
}
