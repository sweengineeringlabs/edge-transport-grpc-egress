//! `BuildLoadbalancerSettingsProviderRequest` -- the request of the load balancer settings provider factory.

/// Request for
/// [`LoadbalancerSettingsProviderFactory::build`](crate::LoadbalancerSettingsProviderFactory::build). It carries
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuildLoadbalancerSettingsProviderRequest;
