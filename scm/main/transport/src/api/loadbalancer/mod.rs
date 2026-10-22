//! API of the `loadbalancer` theme: spreading calls over several backends.

pub mod dto;
pub mod traits;
pub mod vo;

pub use dto::BuildLoadbalancerSettingsProviderRequest;
pub use traits::{LoadbalancerSettingsProvider, LoadbalancerSettingsProviderFactory};
pub use vo::{BackendTarget, BalancingStrategy, LoadbalancerSettings};
