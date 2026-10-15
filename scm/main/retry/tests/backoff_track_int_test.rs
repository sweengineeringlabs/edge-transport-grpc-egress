#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`BackoffTrack`].

use std::time::Duration;

use edge_transport_grpc_egress_retry::{
    BackoffScheduleRequest, BackoffSchedulerFactory, BackoffTrack, GrpcRetryConfig,
};

/// @covers: BackoffTrack
#[test]
fn test_backoff_track_standard_used_by_real_scheduler_happy() {
    let scheduler = BackoffSchedulerFactory::create();
    let resp = scheduler
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 0,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect("Standard variant must be accepted");
    assert!(resp.sleep.as_millis() > 0);
}

/// @covers: BackoffTrack
#[test]
fn test_backoff_track_rate_limit_with_hint_returns_hint_error() {
    // "error"-flavored scenario for an infallible variant: prove the hint
    // actually overrides the computed value rather than being ignored.
    let scheduler = BackoffSchedulerFactory::create();
    let hint = Duration::from_secs(3);
    let resp = scheduler
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 0,
            random_unit: 0.0,
            track: BackoffTrack::RateLimit {
                retry_after_hint: Some(hint),
            },
        })
        .expect("RateLimit variant must be accepted");
    assert_eq!(resp.sleep, hint);
}

/// @covers: BackoffTrack
#[test]
fn test_backoff_track_rate_limit_without_hint_edge() {
    let scheduler = BackoffSchedulerFactory::create();
    let resp = scheduler
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 0,
            random_unit: 0.0,
            track: BackoffTrack::RateLimit {
                retry_after_hint: None,
            },
        })
        .expect("RateLimit variant without a hint must fall back to computation");
    assert!(resp.sleep.as_millis() > 0);
}
