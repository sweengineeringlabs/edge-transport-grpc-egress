//! `StdRetryClassifier` -- per-status retry decision.  Single source of truth for which
//! gRPC status codes the decorator will retry.
//!
//! The policy is hand-written, not config-driven, because the
//! retryable set is a property of gRPC's semantic contract — not
//! a tunable knob.  Specifically:
//!
//! - `Unavailable` (a status the peer sent, or a call that did not reach it) and `ResourceExhausted` are retryable per the
//!   gRPC retry whitepaper (the latter with a longer backoff,
//!   surfaced via [`RetryDecision::is_rate_limit`]).
//! - `ResourceExhausted(HardQuota)` is NOT retried — a billing
//!   cap won't clear on retry.
//! - `Unauthenticated` and `PermissionDenied` MUST NOT be
//!   retried — a bad token won't become good by trying again,
//!   and silent retries hide auth failures from the caller.
//! - `DeadlineExceeded` must not be retried — the caller's
//!   deadline already counts the retry budget; re-issuing
//!   guarantees a second deadline trip.
//! - `Internal` is not retried — server bug, retrying just
//!   amplifies the bug and burns the deadline.

use std::time::Duration;

use grpc_egress_retry::proto_pattern::{CallError, StatusCode};
use grpc_egress_retry::{ResourceExhaustedContext, RetryClassifier, RetryDecision};

/// Trailer names that carry the time the peer asks the caller to wait, in whole seconds.
const RETRY_AFTER_KEYS: [&str; 2] = ["retry-after", "x-ratelimit-reset-requests"];

/// The standard [`RetryClassifier`]: gRPC's retryable failures, as the crate documents.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StdRetryClassifier;

impl RetryClassifier for StdRetryClassifier {
    fn classify<T>(&self, result: &Result<T, CallError>) -> RetryDecision {
        match result {
            Ok(_) => RetryDecision::Success,
            Err(CallError::Status { code, message, .. }) => self.classify_status(*code, message),
            Err(CallError::Transport { code, .. }) => match code {
                StatusCode::Unavailable | StatusCode::Unknown => RetryDecision::Retry,
                _ => RetryDecision::Terminal,
            },
            Err(CallError::Encode(_) | CallError::Decode(_)) => RetryDecision::Terminal,
        }
    }

    fn classify_resource_exhausted(&self, message: &str) -> ResourceExhaustedContext {
        let msg = message.to_ascii_lowercase();
        if msg.contains("quota") || msg.contains("billing") || msg.contains("plan limit") {
            ResourceExhaustedContext::HardQuota
        } else if msg.contains("rate")
            || msg.contains("too many requests")
            || msg.contains("throttl")
        {
            ResourceExhaustedContext::RateLimit
        } else {
            ResourceExhaustedContext::Capacity
        }
    }

    fn should_retry(&self, decision: RetryDecision) -> bool {
        matches!(
            decision,
            RetryDecision::Retry | RetryDecision::RetryRateLimit
        )
    }

    fn is_rate_limit(&self, decision: RetryDecision) -> bool {
        matches!(decision, RetryDecision::RetryRateLimit)
    }

    fn read_retry_after(&self, error: &CallError) -> Option<Duration> {
        let CallError::Status {
            message, trailers, ..
        } = error
        else {
            return None;
        };
        let from_trailers = RETRY_AFTER_KEYS.iter().find_map(|key| {
            trailers
                .get_values(key)
                .first()
                .and_then(|value| value.trim().parse::<u64>().ok())
        });
        match from_trailers {
            Some(secs) => Some(Duration::from_secs(secs)),
            None => self.parse_retry_after_hint(message),
        }
    }

    fn parse_retry_after_hint(&self, message: &str) -> Option<Duration> {
        let tag = "[retry-after=";
        let start = message.find(tag)? + tag.len();
        let rest = &message[start..];
        let end = rest.find('s')?;
        let secs: u64 = rest[..end].parse().ok()?;
        Some(Duration::from_secs(secs))
    }
}

impl StdRetryClassifier {
    fn classify_status(&self, code: StatusCode, msg: &str) -> RetryDecision {
        match code {
            StatusCode::Unavailable => RetryDecision::Retry,
            StatusCode::ResourceExhausted => match self.classify_resource_exhausted(msg) {
                ResourceExhaustedContext::HardQuota => RetryDecision::Terminal,
                ResourceExhaustedContext::RateLimit => RetryDecision::RetryRateLimit,
                ResourceExhaustedContext::Capacity => RetryDecision::Retry,
            },
            // Explicit non-retryable variants -- listed here so adding a new variant on
            // `StatusCode` surfaces as a missing arm, not a silent default.
            StatusCode::Unauthenticated
            | StatusCode::PermissionDenied
            | StatusCode::DeadlineExceeded
            | StatusCode::Internal
            | StatusCode::Cancelled
            | StatusCode::Ok
            | StatusCode::Unknown
            | StatusCode::InvalidArgument
            | StatusCode::NotFound
            | StatusCode::AlreadyExists
            | StatusCode::FailedPrecondition
            | StatusCode::Aborted
            | StatusCode::OutOfRange
            | StatusCode::Unimplemented
            | StatusCode::DataLoss => RetryDecision::Terminal,
        }
    }
}
