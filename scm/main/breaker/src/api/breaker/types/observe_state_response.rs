//! Response for [`crate::api::BreakerObservable::state`].

use crate::api::BreakerState;

/// Output of [`crate::api::BreakerObservable::state`] — a snapshot
/// of the breaker's state; the breaker may transition immediately after
/// this call returns.
pub struct ObserveStateResponse {
    /// The observed breaker state snapshot.
    pub state: BreakerState,
}
