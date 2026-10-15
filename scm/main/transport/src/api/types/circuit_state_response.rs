//! `CircuitStateResponse` — response for [`crate::api::ResilientGrpcClientPort::circuit_state`].

/// Response carrying the current circuit-breaker state label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CircuitStateResponse {
    /// One of: `"Closed"`, `"Open"`, `"HalfOpen"`.
    pub state: &'static str,
}
