#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for [`BackoffSchedulerFactory`].

use edge_transport_grpc_egress_retry::{
    BackoffScheduleRequest, BackoffSchedulerFactory, BackoffTrack, GrpcRetryConfig,
};

/// @covers: create
#[test]
fn test_create_produces_a_working_scheduler_happy() {
    let scheduler = BackoffSchedulerFactory::create();
    let resp = scheduler
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 0,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect("factory-produced scheduler must succeed");
    assert!(resp.sleep.as_millis() > 0);
}

/// @covers: create
#[test]
fn test_create_produces_independent_instances_error() {
    let first = BackoffSchedulerFactory::create();
    let second = BackoffSchedulerFactory::create();
    let resp1 = first
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 1,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect("first must succeed");
    let resp2 = second
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 1,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect("second must succeed");
    assert_eq!(resp1.sleep, resp2.sleep);
}

/// @covers: create
#[test]
fn test_create_repeated_requests_edge() {
    let scheduler = BackoffSchedulerFactory::create();
    let resp1 = scheduler
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 0,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect("first request");
    let resp2 = scheduler
        .schedule(BackoffScheduleRequest {
            config: GrpcRetryConfig::default(),
            attempt: 0,
            random_unit: 0.0,
            track: BackoffTrack::Standard,
        })
        .expect("second request on same scheduler instance");
    assert_eq!(resp1.sleep, resp2.sleep);
}
