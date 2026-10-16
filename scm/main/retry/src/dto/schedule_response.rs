//! Response for [`crate::BackoffScheduler::schedule`].

use std::time::Duration;

/// Output of [`crate::BackoffScheduler::schedule`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleResponse {
    /// How long to sleep before the next retry.
    pub sleep: Duration,
}
