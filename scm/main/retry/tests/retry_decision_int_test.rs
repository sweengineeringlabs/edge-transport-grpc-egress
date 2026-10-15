//! Integration tests for [`RetryDecision`] and [`ResourceExhaustedContext`].

use std::time::Duration;

use edge_transport_grpc_egress::{GrpcEgressError, GrpcStatusCode};
use edge_transport_grpc_egress_retry::{ResourceExhaustedContext, RetryDecision};

// ── ResourceExhaustedContext::classify ──────────────────────────────────────

/// @covers: ResourceExhaustedContext::classify
#[test]
fn test_classify_resource_exhausted_rate_limit_keywords() {
    assert_eq!(
        ResourceExhaustedContext::classify("rate limit exceeded"),
        ResourceExhaustedContext::RateLimit
    );
    assert_eq!(
        ResourceExhaustedContext::classify("too many requests"),
        ResourceExhaustedContext::RateLimit
    );
    assert_eq!(
        ResourceExhaustedContext::classify("throttled"),
        ResourceExhaustedContext::RateLimit
    );
    assert_eq!(
        ResourceExhaustedContext::classify("RATE_LIMIT_ERROR"),
        ResourceExhaustedContext::RateLimit
    );
}

/// @covers: ResourceExhaustedContext::classify
#[test]
fn test_classify_resource_exhausted_quota_keywords() {
    assert_eq!(
        ResourceExhaustedContext::classify("quota exceeded"),
        ResourceExhaustedContext::HardQuota
    );
    assert_eq!(
        ResourceExhaustedContext::classify("billing limit hit"),
        ResourceExhaustedContext::HardQuota
    );
    assert_eq!(
        ResourceExhaustedContext::classify("plan limit reached"),
        ResourceExhaustedContext::HardQuota
    );
}

/// @covers: ResourceExhaustedContext::classify
#[test]
fn test_classify_resource_exhausted_unknown_defaults_to_capacity() {
    assert_eq!(
        ResourceExhaustedContext::classify("out of memory"),
        ResourceExhaustedContext::Capacity
    );
    assert_eq!(
        ResourceExhaustedContext::classify("server overloaded"),
        ResourceExhaustedContext::Capacity
    );
    assert_eq!(
        ResourceExhaustedContext::classify(""),
        ResourceExhaustedContext::Capacity
    );
}

// ── RetryDecision::parse_retry_after_hint ───────────────────────────────────

/// @covers: RetryDecision::parse_retry_after_hint
#[test]
fn test_parse_retry_after_hint_extracts_seconds() {
    assert_eq!(
        RetryDecision::parse_retry_after_hint("rate limit exceeded [retry-after=30s]"),
        Some(Duration::from_secs(30)),
    );
}

/// @covers: RetryDecision::parse_retry_after_hint
#[test]
fn test_parse_retry_after_hint_returns_none_when_absent() {
    assert_eq!(
        RetryDecision::parse_retry_after_hint("rate limit exceeded"),
        None
    );
    assert_eq!(RetryDecision::parse_retry_after_hint(""), None);
}

// ── RetryDecision::classify ─────────────────────────────────────────────────

/// @covers: RetryDecision::classify
#[test]
fn test_classify_status_unavailable_returns_retry() {
    let r: Result<(), _> = Err(GrpcEgressError::Status(
        GrpcStatusCode::Unavailable,
        "lb".into(),
    ));
    assert_eq!(RetryDecision::classify(&r), RetryDecision::Retry);
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_resource_exhausted_rate_limit_returns_retry_rate_limit() {
    let r: Result<(), _> = Err(GrpcEgressError::Status(
        GrpcStatusCode::ResourceExhausted,
        "rate limit exceeded".into(),
    ));
    assert_eq!(RetryDecision::classify(&r), RetryDecision::RetryRateLimit);
    assert!(RetryDecision::classify(&r).is_rate_limit());
    assert!(RetryDecision::classify(&r).should_retry());
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_resource_exhausted_capacity_returns_retry() {
    let r: Result<(), _> = Err(GrpcEgressError::Status(
        GrpcStatusCode::ResourceExhausted,
        "server overloaded".into(),
    ));
    assert_eq!(RetryDecision::classify(&r), RetryDecision::Retry);
    assert!(!RetryDecision::classify(&r).is_rate_limit());
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_resource_exhausted_hard_quota_is_terminal() {
    let r: Result<(), _> = Err(GrpcEgressError::Status(
        GrpcStatusCode::ResourceExhausted,
        "quota exceeded".into(),
    ));
    assert_eq!(RetryDecision::classify(&r), RetryDecision::Terminal);
    assert!(!RetryDecision::classify(&r).should_retry());
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_status_permission_denied_is_terminal() {
    let r: Result<(), _> = Err(GrpcEgressError::Status(
        GrpcStatusCode::PermissionDenied,
        "no".into(),
    ));
    assert_eq!(RetryDecision::classify(&r), RetryDecision::Terminal);
    assert!(!RetryDecision::classify(&r).should_retry());
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_status_unauthenticated_is_terminal() {
    let r: Result<(), _> = Err(GrpcEgressError::Status(
        GrpcStatusCode::Unauthenticated,
        "bad token".into(),
    ));
    assert_eq!(RetryDecision::classify(&r), RetryDecision::Terminal);
    assert!(!RetryDecision::classify(&r).should_retry());
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_status_deadline_exceeded_is_terminal() {
    let r: Result<(), _> = Err(GrpcEgressError::Status(
        GrpcStatusCode::DeadlineExceeded,
        "tick".into(),
    ));
    assert_eq!(RetryDecision::classify(&r), RetryDecision::Terminal);
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_status_internal_is_terminal() {
    let r: Result<(), _> = Err(GrpcEgressError::Status(
        GrpcStatusCode::Internal,
        "bug".into(),
    ));
    assert_eq!(RetryDecision::classify(&r), RetryDecision::Terminal);
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_connection_failed_retries() {
    let r: Result<(), _> = Err(GrpcEgressError::ConnectionFailed("rst".into()));
    assert_eq!(RetryDecision::classify(&r), RetryDecision::Retry);
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_timeout_is_terminal() {
    let r: Result<(), _> = Err(GrpcEgressError::Timeout("deadline".into()));
    assert_eq!(RetryDecision::classify(&r), RetryDecision::Terminal);
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_cancelled_is_terminal() {
    let r: Result<(), _> = Err(GrpcEgressError::Cancelled("token".into()));
    assert_eq!(RetryDecision::classify(&r), RetryDecision::Terminal);
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_ok_returns_success() {
    let r: Result<i32, GrpcEgressError> = Ok(42);
    assert_eq!(RetryDecision::classify(&r), RetryDecision::Success);
    assert!(!RetryDecision::classify(&r).should_retry());
}

// ── RetryDecision::should_retry ─────────────────────────────────────────────

/// @covers: RetryDecision::should_retry
#[test]
fn test_should_retry_true_for_retry_variants() {
    assert!(RetryDecision::Retry.should_retry());
    assert!(RetryDecision::RetryRateLimit.should_retry());
    assert!(!RetryDecision::Success.should_retry());
    assert!(!RetryDecision::Terminal.should_retry());
}

// ── RetryDecision::is_rate_limit ────────────────────────────────────────────

/// @covers: RetryDecision::is_rate_limit
#[test]
fn test_is_rate_limit_only_true_for_rate_limit_variant() {
    assert!(RetryDecision::RetryRateLimit.is_rate_limit());
    assert!(!RetryDecision::Retry.is_rate_limit());
    assert!(!RetryDecision::Success.is_rate_limit());
    assert!(!RetryDecision::Terminal.is_rate_limit());
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_non_retry_status_codes_all_terminal() {
    for code in [
        GrpcStatusCode::Cancelled,
        GrpcStatusCode::Unknown,
        GrpcStatusCode::InvalidArgument,
        GrpcStatusCode::NotFound,
        GrpcStatusCode::AlreadyExists,
        GrpcStatusCode::FailedPrecondition,
        GrpcStatusCode::Aborted,
        GrpcStatusCode::OutOfRange,
        GrpcStatusCode::Unimplemented,
        GrpcStatusCode::DataLoss,
    ] {
        let r: Result<(), _> = Err(GrpcEgressError::Status(code, "x".into()));
        assert_eq!(
            RetryDecision::classify(&r),
            RetryDecision::Terminal,
            "expected {code:?} to be Terminal",
        );
    }
}
