//! `ConsecutiveFailuresResponse` — response for [`crate::api::ResilientGrpcClientPort::consecutive_failures`].

/// Response carrying the consecutive post-retry failure count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsecutiveFailuresResponse {
    /// Consecutive post-retry failures tracked since the circuit last closed.
    pub count: u32,
}
