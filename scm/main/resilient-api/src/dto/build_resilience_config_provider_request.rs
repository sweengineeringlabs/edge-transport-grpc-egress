//! `BuildResilienceConfigProviderRequest` -- the request of the resilience policy provider factory.

/// Request for [`ResilienceConfigProviderFactory::build`](crate::ResilienceConfigProviderFactory::build). It carries
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildResilienceConfigProviderRequest;
