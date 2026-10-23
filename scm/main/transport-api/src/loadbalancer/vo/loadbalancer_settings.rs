//! `LoadbalancerSettings` -- the backends of a balanced client and how it picks among them.

use serde::{Deserialize, Serialize};

use crate::{BackendTarget, BalancingStrategy};

/// The option value of [`GrpcEgressBuilder::with_loadbalancer`](crate::GrpcEgressBuilder::with_loadbalancer).
///
/// A backend that fails a call is avoided until a call to it succeeds again.
///
/// ```compile_fail,E0451
/// use grpc_egress_transport_api::{BalancingStrategy, LoadbalancerSettings};
/// let _ = LoadbalancerSettings { strategy: BalancingStrategy::RoundRobin, backends: Vec::new() };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "LoadbalancerSettingsData")]
pub struct LoadbalancerSettings {
    /// How the backend of a call is picked.
    strategy: BalancingStrategy,
    /// The backends. At least one.
    backends: Vec<BackendTarget>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoadbalancerSettingsData {
    strategy: BalancingStrategy,
    backends: Vec<BackendTarget>,
}
impl TryFrom<LoadbalancerSettingsData> for LoadbalancerSettings {
    type Error = String;
    fn try_from(v: LoadbalancerSettingsData) -> Result<Self, String> {
        Self::try_new(v.strategy, v.backends)
    }
}
impl LoadbalancerSettings {
    /// Construct load-balancer settings for one or more backends.
    pub fn try_new(
        strategy: BalancingStrategy,
        backends: Vec<BackendTarget>,
    ) -> Result<Self, String> {
        if backends.is_empty() {
            return Err("backends must contain at least one target".into());
        }
        Ok(Self { strategy, backends })
    }
    /// Balancing strategy.
    pub fn strategy(&self) -> BalancingStrategy {
        self.strategy
    }
    /// Configured backend targets.
    pub fn backends(&self) -> &[BackendTarget] {
        &self.backends
    }
}
