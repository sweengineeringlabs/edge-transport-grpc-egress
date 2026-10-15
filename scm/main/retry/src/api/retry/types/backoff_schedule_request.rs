//! Request for [`crate::api::BackoffScheduler::schedule`].

use crate::api::BackoffTrack;
use crate::api::GrpcRetryConfig;

/// Input to [`crate::api::BackoffScheduler::schedule`].
pub struct BackoffScheduleRequest {
    /// The retry policy driving the computation.
    pub config: GrpcRetryConfig,
    /// 0-based attempt index on the selected track.
    pub attempt: u32,
    /// Uniform random sample in `[0.0, 1.0)` used for jitter.
    pub random_unit: f64,
    /// Which track (standard vs. rate-limit) to compute for.
    pub track: BackoffTrack,
}
