//! `BackoffScheduler` trait — backoff computation contract.

use crate::BackoffScheduleRequest;
use crate::Error;
use crate::ScheduleResponse;

/// Interface for computing a retry backoff schedule.
///
/// Implemented by `DefaultBackoffScheduler`
/// in `core/`.
pub trait BackoffScheduler: Send + Sync {
    /// Compute the schedule for one retry attempt on the requested track.
    fn schedule(&self, req: BackoffScheduleRequest) -> Result<ScheduleResponse, Error>;
}
