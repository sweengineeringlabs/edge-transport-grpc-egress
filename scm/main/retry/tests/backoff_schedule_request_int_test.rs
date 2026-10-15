#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`BackoffScheduleRequest`].

use edge_transport_grpc_egress_retry::{BackoffScheduleRequest, BackoffTrack, GrpcRetryConfig};

/// @covers: BackoffScheduleRequest
#[test]
fn test_backoff_schedule_request_used_by_real_scheduler_happy() {
    let scheduler = edge_transport_grpc_egress_retry::BackoffSchedulerFactory::create();
    let resp = scheduler
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 0,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect("real scheduler must accept this request type");
    assert!(resp.sleep.as_millis() > 0);
}

/// @covers: BackoffScheduleRequest
#[test]
fn test_backoff_schedule_request_rate_limit_track_error() {
    // "error"-flavored scenario: prove the RateLimit variant is a distinct
    // path, not silently treated identically to Standard.
    let scheduler = edge_transport_grpc_egress_retry::BackoffSchedulerFactory::create();
    let standard = scheduler
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 2,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect("standard track");
    let rate_limit = scheduler
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 2,
            random_unit: 0.0,
            track: BackoffTrack::RateLimit {
                retry_after_hint: None,
            },
        })
        .expect("rate-limit track");
    assert_ne!(standard.sleep, rate_limit.sleep);
}

/// @covers: BackoffScheduleRequest
#[test]
fn test_backoff_schedule_request_zero_attempt_edge() {
    let scheduler = edge_transport_grpc_egress_retry::BackoffSchedulerFactory::create();
    let resp = scheduler
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 0,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect("attempt 0 must be valid");
    assert!(resp.sleep.as_millis() > 0);
}
