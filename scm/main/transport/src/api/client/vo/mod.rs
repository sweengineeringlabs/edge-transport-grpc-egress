//! Value objects of the `client` theme: the options that are values, not behaviour.

pub mod backend_target;
pub mod balancing_strategy;
pub mod loadbalancer_settings;

pub use backend_target::BackendTarget;
pub use balancing_strategy::BalancingStrategy;
pub use loadbalancer_settings::LoadbalancerSettings;
