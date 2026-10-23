//! Response for [`crate::BreakerObservable::state`].

use crate::BreakerState;

/// Output of [`crate::BreakerObservable::state`] — a snapshot of the breaker's state; the breaker may transition
/// immediately after this call returns.
///
/// The response owns the state snapshot. Read it with [`ObserveStateResponse::state`].
///
/// ```compile_fail,E0451
/// use grpc_egress_breaker::{BreakerState, ObserveStateResponse};
/// let _ = ObserveStateResponse { state: BreakerState::Closed };
/// ```
pub struct ObserveStateResponse {
    state: BreakerState,
}

impl ObserveStateResponse {
    /// Construct a response that owns the observed state snapshot.
    pub fn new(state: BreakerState) -> Self {
        Self { state }
    }

    /// The observed breaker state snapshot.
    pub fn state(&self) -> BreakerState {
        self.state
    }
}
