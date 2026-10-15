//! `LastErrorResponse` — response for [`crate::api::ResilientGrpcClientPort::last_error`].

use crate::api::error::GrpcEgressError;

/// Response carrying the last transport error observed by the resilience layer.
///
/// `error` is `None` when no failure has been recorded (circuit is `Closed`
/// and no retry storms have fired).
#[derive(Debug, Clone)]
pub struct LastErrorResponse {
    /// The last observed transport error, if any.
    pub error: Option<GrpcEgressError>,
}
