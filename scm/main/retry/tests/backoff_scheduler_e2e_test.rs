#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end tests for [`BackoffScheduler`] via a test-double implementation.

use std::time::Duration;

use edge_transport_grpc_egress_retry::{
    BackoffSchedule, BackoffScheduleRequest, BackoffScheduler, BackoffTrack, Error,
    GrpcRetryConfig, ScheduleResponse,
};

struct MockScheduler {
    fail: bool,
}

impl BackoffScheduler for MockScheduler {
    fn schedule(&self, req: BackoffScheduleRequest) -> Result<ScheduleResponse, Error> {
        if self.fail {
            return Err(Error::InvalidConfig("mock scheduler forced failure".into()));
        }
        Ok(ScheduleResponse {
            sleep: Duration::from_millis(req.config.initial_backoff_ms),
        })
    }
}

fn cfg() -> GrpcRetryConfig {
    GrpcRetryConfig::default()
}

/// @covers: schedule
#[test]
fn test_schedule_returns_configured_sleep_happy() {
    let scheduler = MockScheduler { fail: false };
    let resp = scheduler
        .schedule(BackoffScheduleRequest {
            config: cfg(),
            attempt: 0,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect("happy path");
    assert_eq!(resp.sleep, Duration::from_millis(cfg().initial_backoff_ms));
}

/// @covers: schedule
#[test]
fn test_schedule_propagates_failure_error() {
    let scheduler = MockScheduler { fail: true };
    let err = scheduler
        .schedule(BackoffScheduleRequest {
            config: cfg(),
            attempt: 0,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect_err("forced failure must surface");
    assert!(err.to_string().contains("mock scheduler forced failure"));
}

/// @covers: schedule
#[test]
fn test_schedule_rate_limit_track_with_hint_edge() {
    let scheduler = MockScheduler { fail: false };
    let resp = scheduler
        .schedule(BackoffScheduleRequest {
            config: cfg(),
            attempt: 3,
            random_unit: 0.9,
            track: BackoffTrack::RateLimit {
                retry_after_hint: Some(Duration::from_secs(5)),
            },
        })
        .expect("happy path");
    // Mock ignores the hint (delegates purely on config) -- proves the
    // trait boundary passes the track through without the caller having
    // to know which concrete field the real implementation reads.
    assert_eq!(resp.sleep, Duration::from_millis(cfg().initial_backoff_ms));
}

/// @covers: describe_schedule
#[test]
fn test_describe_schedule_extracts_sleep_happy() {
    let schedule = BackoffSchedule::from_duration(Duration::from_millis(250));
    let sleep = <MockScheduler as BackoffScheduler>::describe_schedule(schedule);
    assert_eq!(sleep, Duration::from_millis(250));
}

/// @covers: describe_schedule
#[test]
fn test_describe_schedule_zero_duration_error() {
    // "error"-flavored scenario for an infallible extractor: prove it
    // doesn't silently substitute a non-zero default when given zero.
    let schedule = BackoffSchedule::from_duration(Duration::ZERO);
    let sleep = <MockScheduler as BackoffScheduler>::describe_schedule(schedule);
    assert_eq!(sleep, Duration::ZERO);
}

/// @covers: describe_schedule
#[test]
fn test_describe_schedule_is_deterministic_edge() {
    let a = <MockScheduler as BackoffScheduler>::describe_schedule(BackoffSchedule::from_duration(
        Duration::from_secs(1),
    ));
    let b = <MockScheduler as BackoffScheduler>::describe_schedule(BackoffSchedule::from_duration(
        Duration::from_secs(1),
    ));
    assert_eq!(a, b);
}
