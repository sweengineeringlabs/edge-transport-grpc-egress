//! `BackoffScheduler` trait — backoff computation contract.

use std::time::Duration;

use crate::api::BackoffSchedule;
use crate::api::BackoffScheduleRequest;
use crate::api::Error;
use crate::api::ScheduleResponse;

/// Interface for computing a retry backoff schedule.
///
/// Implemented by `DefaultBackoffScheduler`
/// in `core/`.
pub trait BackoffScheduler: Send + Sync {
    /// Compute the schedule for one retry attempt on the requested track.
    fn schedule(&self, req: BackoffScheduleRequest) -> Result<ScheduleResponse, Error>;

    /// Extract the sleep duration from a computed schedule — gives
    /// [`BackoffSchedule`] a genuine role in this trait's signature set,
    /// not just an internal core/ value. `Self: Sized` keeps this trait
    /// dyn-compatible for `Box<dyn Trait>`.
    fn describe_schedule(schedule: BackoffSchedule) -> Duration
    where
        Self: Sized,
    {
        schedule.sleep
    }
}
