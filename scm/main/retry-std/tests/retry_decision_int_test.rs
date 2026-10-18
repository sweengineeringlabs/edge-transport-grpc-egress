//! Integration tests for [`RetryDecision`] and [`ResourceExhaustedContext`].

use grpc_egress_retry::RetryClassifier as _;
use grpc_egress_retry::{ResourceExhaustedContext, RetryDecision};
use std::time::Duration;

use proto_pattern::{CallError, DecodeError, EncodeError, Metadata, MetadataEntry, StatusCode};

/// A status the peer answered with, carrying `trailers`.
fn status_with(code: StatusCode, message: &str, trailers: Metadata) -> CallError {
    CallError::Status {
        path: "/pkg.Service/Method".into(),
        code,
        message: message.to_string(),
        details: Vec::new(),
        trailers,
    }
}

fn status(code: StatusCode, message: &str) -> Result<(), CallError> {
    Err(status_with(code, message, Metadata::default()))
}

fn transport(code: StatusCode) -> Result<(), CallError> {
    Err(CallError::Transport {
        path: "/pkg.Service/Method".into(),
        code,
        reason: "the call did not complete".to_string(),
    })
}

fn trailers_of(entries: &[(&str, &str)]) -> Result<Metadata, Box<dyn std::error::Error>> {
    let entries = entries
        .iter()
        .map(|(key, value)| MetadataEntry::create(*key, *value))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Metadata::create(entries))
}

// ── ResourceExhaustedContext::classify ──────────────────────────────────────

/// @covers: ResourceExhaustedContext::classify
#[test]
fn test_classify_resource_exhausted_rate_limit_keywords() {
    assert_eq!(
        classifier().classify_resource_exhausted("rate limit exceeded"),
        ResourceExhaustedContext::RateLimit
    );
    assert_eq!(
        classifier().classify_resource_exhausted("too many requests"),
        ResourceExhaustedContext::RateLimit
    );
    assert_eq!(
        classifier().classify_resource_exhausted("throttled"),
        ResourceExhaustedContext::RateLimit
    );
    assert_eq!(
        classifier().classify_resource_exhausted("RATE_LIMIT_ERROR"),
        ResourceExhaustedContext::RateLimit
    );
}

/// @covers: ResourceExhaustedContext::classify
#[test]
fn test_classify_resource_exhausted_quota_keywords() {
    assert_eq!(
        classifier().classify_resource_exhausted("quota exceeded"),
        ResourceExhaustedContext::HardQuota
    );
    assert_eq!(
        classifier().classify_resource_exhausted("billing limit hit"),
        ResourceExhaustedContext::HardQuota
    );
    assert_eq!(
        classifier().classify_resource_exhausted("plan limit reached"),
        ResourceExhaustedContext::HardQuota
    );
}

/// @covers: ResourceExhaustedContext::classify
#[test]
fn test_classify_resource_exhausted_unknown_defaults_to_capacity() {
    assert_eq!(
        classifier().classify_resource_exhausted("out of memory"),
        ResourceExhaustedContext::Capacity
    );
    assert_eq!(
        classifier().classify_resource_exhausted("server overloaded"),
        ResourceExhaustedContext::Capacity
    );
    assert_eq!(
        classifier().classify_resource_exhausted(""),
        ResourceExhaustedContext::Capacity
    );
}

// ── RetryDecision::parse_retry_after_hint ───────────────────────────────────

/// @covers: RetryDecision::parse_retry_after_hint
#[test]
fn test_parse_retry_after_hint_extracts_seconds() {
    assert_eq!(
        classifier().parse_retry_after_hint("rate limit exceeded [retry-after=30s]"),
        Some(Duration::from_secs(30)),
    );
}

/// @covers: RetryDecision::parse_retry_after_hint
#[test]
fn test_parse_retry_after_hint_returns_none_when_absent() {
    assert_eq!(
        classifier().parse_retry_after_hint("rate limit exceeded"),
        None
    );
    assert_eq!(classifier().parse_retry_after_hint(""), None);
}

// ── RetryDecision::read_retry_after ─────────────────────────────────────────

/// @covers: RetryDecision::read_retry_after
#[test]
fn test_read_retry_after_trailer_is_the_hint() -> Result<(), Box<dyn std::error::Error>> {
    let error = status_with(
        StatusCode::ResourceExhausted,
        "rate limit exceeded",
        trailers_of(&[("retry-after", "7")])?,
    );
    assert_eq!(
        classifier().read_retry_after(&error),
        Some(Duration::from_secs(7))
    );
    Ok(())
}

/// @covers: RetryDecision::read_retry_after
#[test]
fn test_read_retry_after_ratelimit_reset_trailer_is_the_hint(
) -> Result<(), Box<dyn std::error::Error>> {
    let error = status_with(
        StatusCode::ResourceExhausted,
        "rate limit exceeded",
        trailers_of(&[("x-ratelimit-reset-requests", "3")])?,
    );
    assert_eq!(
        classifier().read_retry_after(&error),
        Some(Duration::from_secs(3))
    );
    Ok(())
}

/// @covers: RetryDecision::read_retry_after
#[test]
fn test_read_retry_after_trailer_wins_over_the_message_hint(
) -> Result<(), Box<dyn std::error::Error>> {
    let error = status_with(
        StatusCode::ResourceExhausted,
        "rate limit exceeded [retry-after=30s]",
        trailers_of(&[("retry-after", "2")])?,
    );
    assert_eq!(
        classifier().read_retry_after(&error),
        Some(Duration::from_secs(2))
    );
    Ok(())
}

/// @covers: RetryDecision::read_retry_after
#[test]
fn test_read_retry_after_unusable_trailer_falls_back_to_the_message_hint(
) -> Result<(), Box<dyn std::error::Error>> {
    let error = status_with(
        StatusCode::ResourceExhausted,
        "rate limit exceeded [retry-after=30s]",
        trailers_of(&[("retry-after", "Wed, 21 Oct 2026 07:28:00 GMT")])?,
    );
    assert_eq!(
        classifier().read_retry_after(&error),
        Some(Duration::from_secs(30))
    );
    Ok(())
}

/// @covers: RetryDecision::read_retry_after
#[test]
fn test_read_retry_after_no_hint_anywhere_returns_none() {
    let error = status_with(
        StatusCode::ResourceExhausted,
        "rate limit exceeded",
        Metadata::default(),
    );
    assert_eq!(classifier().read_retry_after(&error), None);
}

/// @covers: RetryDecision::read_retry_after
#[test]
fn test_read_retry_after_failure_that_is_not_a_status_returns_none() {
    let error = CallError::Transport {
        path: "/pkg.Service/Method".into(),
        code: StatusCode::Unavailable,
        reason: "refused [retry-after=9s]".to_string(),
    };
    assert_eq!(classifier().read_retry_after(&error), None);
}

// ── RetryDecision::classify ─────────────────────────────────────────────────

/// @covers: RetryDecision::classify
#[test]
fn test_classify_status_unavailable_returns_retry() {
    assert_eq!(
        classifier().classify(&status(StatusCode::Unavailable, "lb")),
        RetryDecision::Retry
    );
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_resource_exhausted_rate_limit_returns_retry_rate_limit() {
    let r = status(StatusCode::ResourceExhausted, "rate limit exceeded");
    assert_eq!(classifier().classify(&r), RetryDecision::RetryRateLimit);
    assert!(classifier().is_rate_limit(classifier().classify(&r)));
    assert!(classifier().should_retry(classifier().classify(&r)));
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_resource_exhausted_capacity_returns_retry() {
    let r = status(StatusCode::ResourceExhausted, "server overloaded");
    assert_eq!(classifier().classify(&r), RetryDecision::Retry);
    assert!(!classifier().is_rate_limit(classifier().classify(&r)));
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_resource_exhausted_hard_quota_is_terminal() {
    let r = status(StatusCode::ResourceExhausted, "quota exceeded");
    assert_eq!(classifier().classify(&r), RetryDecision::Terminal);
    assert!(!classifier().should_retry(classifier().classify(&r)));
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_status_permission_denied_is_terminal() {
    let r = status(StatusCode::PermissionDenied, "no");
    assert_eq!(classifier().classify(&r), RetryDecision::Terminal);
    assert!(!classifier().should_retry(classifier().classify(&r)));
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_status_unauthenticated_is_terminal() {
    let r = status(StatusCode::Unauthenticated, "bad token");
    assert_eq!(classifier().classify(&r), RetryDecision::Terminal);
    assert!(!classifier().should_retry(classifier().classify(&r)));
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_status_deadline_exceeded_is_terminal() {
    assert_eq!(
        classifier().classify(&status(StatusCode::DeadlineExceeded, "tick")),
        RetryDecision::Terminal
    );
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_status_internal_is_terminal() {
    assert_eq!(
        classifier().classify(&status(StatusCode::Internal, "bug")),
        RetryDecision::Terminal
    );
}

/// A call that did not reach the peer is what the old `ConnectionFailed` was: it retries.
///
/// @covers: RetryDecision::classify
#[test]
fn test_classify_transport_unavailable_retries() {
    assert_eq!(
        classifier().classify(&transport(StatusCode::Unavailable)),
        RetryDecision::Retry
    );
}

/// The engine reports a connect or handshake failure as a transport failure of code `Unknown`.
///
/// @covers: RetryDecision::classify
#[test]
fn test_classify_transport_unknown_retries() {
    assert_eq!(
        classifier().classify(&transport(StatusCode::Unknown)),
        RetryDecision::Retry
    );
}

/// The same code in a status the peer sent is the peer's own answer, and terminal: only the transport variant means
/// the call did not reach the peer.
///
/// @covers: RetryDecision::classify
#[test]
fn test_classify_status_unknown_is_terminal_while_transport_unknown_retries() {
    assert_eq!(
        classifier().classify(&status(StatusCode::Unknown, "x")),
        RetryDecision::Terminal
    );
    assert_eq!(
        classifier().classify(&transport(StatusCode::Unknown)),
        RetryDecision::Retry
    );
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_transport_failure_of_any_other_code_is_terminal() {
    for code in [
        StatusCode::Cancelled,
        StatusCode::InvalidArgument,
        StatusCode::DeadlineExceeded,
        StatusCode::ResourceExhausted,
        StatusCode::Internal,
    ] {
        assert_eq!(
            classifier().classify(&transport(code)),
            RetryDecision::Terminal,
            "expected a transport failure of {code:?} to be Terminal",
        );
    }
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_encode_failure_is_terminal() {
    let r: Result<(), CallError> = Err(CallError::Encode(EncodeError::create("t.Message", "x")));
    assert_eq!(classifier().classify(&r), RetryDecision::Terminal);
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_decode_failure_is_terminal() {
    let r: Result<(), CallError> = Err(CallError::Decode(DecodeError::create("t.Message", "x")));
    assert_eq!(classifier().classify(&r), RetryDecision::Terminal);
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_ok_returns_success() {
    let r: Result<i32, CallError> = Ok(42);
    assert_eq!(classifier().classify(&r), RetryDecision::Success);
    assert!(!classifier().should_retry(classifier().classify(&r)));
}

// ── RetryDecision::should_retry ─────────────────────────────────────────────

/// @covers: RetryDecision::should_retry
#[test]
fn test_should_retry_true_for_retry_variants() {
    assert!(classifier().should_retry(RetryDecision::Retry));
    assert!(classifier().should_retry(RetryDecision::RetryRateLimit));
    assert!(!classifier().should_retry(RetryDecision::Success));
    assert!(!classifier().should_retry(RetryDecision::Terminal));
}

// ── RetryDecision::is_rate_limit ────────────────────────────────────────────

/// @covers: RetryDecision::is_rate_limit
#[test]
fn test_is_rate_limit_only_true_for_rate_limit_variant() {
    assert!(classifier().is_rate_limit(RetryDecision::RetryRateLimit));
    assert!(!classifier().is_rate_limit(RetryDecision::Retry));
    assert!(!classifier().is_rate_limit(RetryDecision::Success));
    assert!(!classifier().is_rate_limit(RetryDecision::Terminal));
}

/// @covers: RetryDecision::classify
#[test]
fn test_classify_non_retry_status_codes_all_terminal() {
    for code in [
        StatusCode::Cancelled,
        StatusCode::Unknown,
        StatusCode::InvalidArgument,
        StatusCode::NotFound,
        StatusCode::AlreadyExists,
        StatusCode::FailedPrecondition,
        StatusCode::Aborted,
        StatusCode::OutOfRange,
        StatusCode::Unimplemented,
        StatusCode::DataLoss,
    ] {
        assert_eq!(
            classifier().classify(&status(code, "x")),
            RetryDecision::Terminal,
            "expected {code:?} to be Terminal",
        );
    }
}

fn classifier() -> impl grpc_egress_retry::RetryClassifier {
    use grpc_egress_retry::RetryClassifierFactory;
    grpc_egress_retry_std::StdRetryClassifierFactory
        .build(grpc_egress_retry::BuildRetryClassifierRequest)
}
