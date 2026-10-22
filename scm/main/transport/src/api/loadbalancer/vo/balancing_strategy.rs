//! `BalancingStrategy` -- how a balanced client picks the backend of a call.

use serde::{Deserialize, Serialize};

/// How a balanced client picks the backend of each call among the healthy ones. In a configuration
/// file: `round-robin`, `weighted` or `least-connections`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BalancingStrategy {
    /// Each healthy backend in turn.
    #[default]
    RoundRobin,
    /// In proportion to the [`weight`](crate::BackendTarget::weight) of each backend.
    Weighted,
    /// The healthy backend with the fewest calls in flight.
    LeastConnections,
}
