//! Request for [`crate::BreakerObservable::state`].

/// Input to [`crate::BreakerObservable::state`] — no parameters are needed.
///
/// Construct requests with [`ObserveStateRequest::new`].
///
/// ```compile_fail,E0423
/// use grpc_egress_breaker_api::ObserveStateRequest;
/// let _ = ObserveStateRequest(());
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct ObserveStateRequest(());

impl ObserveStateRequest {
    /// Construct a request to observe the current breaker state.
    pub fn new() -> Self {
        Self(())
    }
}
