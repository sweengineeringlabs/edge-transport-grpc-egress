#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`ScheduleResponse`].

use grpc_egress_retry::{BackoffScheduleRequest, BackoffTrack, GrpcRetryConfig};
use grpc_egress_retry_std::BackoffSchedulerFactory;

/// @covers: ScheduleResponse
#[test]
fn test_schedule_response_produced_by_real_scheduler_happy() {
    let scheduler = BackoffSchedulerFactory::create();
    let resp = scheduler
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 0,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect("real scheduler must produce this response type");
    assert!(resp.sleep.as_millis() > 0);
}

/// @covers: ScheduleResponse
#[test]
fn test_schedule_response_differs_across_attempts_error() {
    // "error"-flavored scenario for an infallible computation: prove the
    // response isn't a constant regardless of input.
    let scheduler = BackoffSchedulerFactory::create();
    let first = scheduler
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 0,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect("attempt 0");
    let second = scheduler
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 5,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect("attempt 5");
    assert_ne!(first.sleep, second.sleep);
}

/// @covers: ScheduleResponse
#[test]
fn test_schedule_response_equality_is_by_value_edge() {
    let a = grpc_egress_retry::ScheduleResponse {
        sleep: std::time::Duration::from_millis(100),
    };
    let b = grpc_egress_retry::ScheduleResponse {
        sleep: std::time::Duration::from_millis(100),
    };
    assert_eq!(a, b);
}
