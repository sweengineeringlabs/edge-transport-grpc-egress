//! `BalancingStrategy` -- how a balanced client picks the backend of a call.

/// How a balanced client picks the backend of each call among the healthy ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BalancingStrategy {
    /// Each healthy backend in turn.
    #[default]
    RoundRobin,
    /// In proportion to the [`weight`](crate::BackendTarget::weight) of each backend.
    Weighted,
    /// The healthy backend with the fewest calls in flight.
    LeastConnections,
}
