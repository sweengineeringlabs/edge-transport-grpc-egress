//! `impl BackoffScheduler for DefaultBackoffScheduler` — delegates to
//! `edge_transport_retry::BackoffScheduler` (moved per ADR-003/ADR-004) so the two never drift
//! apart.

use tracing::trace;

use edge_transport_retry::BackoffScheduler as BackoffComputation;

use crate::api::BackoffScheduleRequest;
use crate::api::BackoffTrack;
use crate::api::Error;
use crate::api::ScheduleResponse;
use crate::api::BACKOFF_SCHEDULER_LOG_TARGET;

/// Default [`crate::api::BackoffScheduler`] implementation.
pub(crate) struct DefaultBackoffScheduler;

impl crate::api::BackoffScheduler for DefaultBackoffScheduler {
    fn schedule(&self, req: BackoffScheduleRequest) -> Result<ScheduleResponse, Error> {
        let sleep = match req.track {
            BackoffTrack::Standard => {
                BackoffComputation::next_backoff(&req.config, req.attempt, req.random_unit)
            }
            BackoffTrack::RateLimit { retry_after_hint } => BackoffComputation::rate_limit_backoff(
                &req.config,
                req.attempt,
                retry_after_hint,
                req.random_unit,
            ),
        };
        trace!(
            target: BACKOFF_SCHEDULER_LOG_TARGET,
            attempt = req.attempt,
            sleep_ms = sleep.as_millis() as u64,
            "grpc-retry: backoff schedule computed",
        );
        Ok(ScheduleResponse { sleep })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::BackoffScheduler as _;
    use crate::api::GrpcRetryConfig;

    fn default_config() -> GrpcRetryConfig {
        GrpcRetryConfig::default()
    }

    /// @covers: schedule
    #[test]
    fn test_schedule_standard_track_returns_positive_sleep() {
        let resp = DefaultBackoffScheduler
            .schedule(BackoffScheduleRequest {
                config: default_config(),
                attempt: 0,
                random_unit: 0.5,
                track: BackoffTrack::Standard,
            })
            .expect("infallible");
        assert!(resp.sleep.as_millis() > 0);
    }

    /// @covers: schedule
    #[test]
    fn test_schedule_rate_limit_track_with_hint_returns_hint_exactly() {
        let hint = std::time::Duration::from_secs(7);
        let resp = DefaultBackoffScheduler
            .schedule(BackoffScheduleRequest {
                config: default_config(),
                attempt: 0,
                random_unit: 0.5,
                track: BackoffTrack::RateLimit {
                    retry_after_hint: Some(hint),
                },
            })
            .expect("infallible");
        assert_eq!(resp.sleep, hint);
    }

    /// @covers: schedule
    #[test]
    fn test_schedule_rate_limit_track_without_hint_differs_from_standard_track() {
        let cfg = default_config();
        let standard = DefaultBackoffScheduler
            .schedule(BackoffScheduleRequest {
                config: cfg.clone(),
                attempt: 2,
                random_unit: 0.0,
                track: BackoffTrack::Standard,
            })
            .expect("infallible");
        let rate_limit = DefaultBackoffScheduler
            .schedule(BackoffScheduleRequest {
                config: cfg,
                attempt: 2,
                random_unit: 0.0,
                track: BackoffTrack::RateLimit {
                    retry_after_hint: None,
                },
            })
            .expect("infallible");
        // Rate-limit track uses a much higher floor (rate_limit_initial_backoff_ms
        // defaults far above initial_backoff_ms) -- proves the track selection
        // actually switches which config fields drive the computation.
        assert!(rate_limit.sleep > standard.sleep);
    }
}
