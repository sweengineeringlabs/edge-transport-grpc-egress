//! `LoadbalancerSettings` -- the backends of a balanced client and how it picks among them.

use crate::{BackendTarget, BalancingStrategy};

/// The option value of [`GrpcEgressBuilder::with_loadbalancer`](crate::GrpcEgressBuilder::with_loadbalancer).
///
/// A backend that fails a call is avoided until a call to it succeeds again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadbalancerSettings {
    /// How the backend of a call is picked.
    pub strategy: BalancingStrategy,
    /// The backends. At least one.
    pub backends: Vec<BackendTarget>,
}
