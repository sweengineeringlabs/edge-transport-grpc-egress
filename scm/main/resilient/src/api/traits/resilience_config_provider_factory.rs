//! `ResilienceConfigProviderFactory` -- creates the reader of resilience policies.

use crate::{BuildResilienceConfigProviderRequest, ResilienceConfigProvider};

/// Creates a [`ResilienceConfigProvider`].
pub trait ResilienceConfigProviderFactory {
    /// A provider that reads policies from TOML text.
    fn build(
        &self,
        req: BuildResilienceConfigProviderRequest,
    ) -> impl ResilienceConfigProvider + use<Self>;
}
