//! Response for [`crate::BreakerObservable::state`].

use crate::BreakerState;

/// Output of [`crate::BreakerObservable::state`] — a snapshot
/// of the breaker's state; the breaker may transition immediately after
/// this call returns.
pub struct ObserveStateResponse {
    /// The observed breaker state snapshot.
    pub state: BreakerState,
}
