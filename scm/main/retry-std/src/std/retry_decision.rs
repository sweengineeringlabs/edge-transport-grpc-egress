//! Per-status retry decision.  Single source of truth for which
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

use proto_pattern::{CallError, GetValuesRequest, GetValuesResponse, MetadataView, StatusCode};
use std::time::Duration;

use crate::std::ResourceExhaustedContext;

/// Trailer names that carry the time the peer asks the caller to wait, in whole seconds.
const RETRY_AFTER_KEYS: [&str; 2] = ["retry-after", "x-ratelimit-reset-requests"];

/// What the retry loop does with the result of one attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    /// The attempt succeeded; return it.
    Success,
    /// Retry on the standard backoff track.
    Retry,
    /// Retry on the rate-limit backoff track.
    RetryRateLimit,
    /// Surface the failure to the caller.
    Terminal,
}

impl RetryDecision {
    /// `true` for `Retry` and `RetryRateLimit`.
    pub fn should_retry(self) -> bool {
        matches!(self, RetryDecision::Retry | RetryDecision::RetryRateLimit)
    }

    /// `true` for `RetryRateLimit`.
    pub fn is_rate_limit(self) -> bool {
        matches!(self, RetryDecision::RetryRateLimit)
    }

    /// Classify the result of one attempt.
    ///
    /// | Result                                              | Decision          |
    /// |-----------------------------------------------------|-------------------|
    /// | `Ok(_)`                                             | `Success`         |
    /// | `Status(Unavailable, _)`                            | `Retry`           |
    /// | `Transport(Unavailable or Unknown, _)` (the call did not reach the peer) | `Retry` |
    /// | `Status(ResourceExhausted, _)`, message of a capacity | `Retry`         |
    /// | `Status(ResourceExhausted, _)`, message of a rate limit | `RetryRateLimit` |
    /// | `Status(ResourceExhausted, _)`, message of a hard quota | `Terminal`    |
    /// | `Status(Unauthenticated or PermissionDenied, _)`    | `Terminal`        |
    /// | `Status(DeadlineExceeded, _)`                       | `Terminal`        |
    /// | `Status(Internal, _)`, `Encode`, `Decode`           | `Terminal`        |
    /// | `Status(Cancelled, _)`, `Transport(Cancelled, _)`   | `Terminal`        |
    /// | any other status, any other transport code          | `Terminal`        |
    ///
    /// A `Transport` failure of code `Unknown` is a connection that failed (the engine reports a handshake or connect
    /// failure with it), as the old `ConnectionFailed` was; a `Status` of code `Unknown` is the peer's own answer and
    /// is terminal.
    pub fn classify<T>(result: &Result<T, CallError>) -> Self {
        match result {
            Ok(_) => RetryDecision::Success,
            Err(CallError::Status { code, message, .. }) => Self::classify_status(*code, message),
            Err(CallError::Transport { code, .. }) => match code {
                StatusCode::Unavailable | StatusCode::Unknown => RetryDecision::Retry,
                _ => RetryDecision::Terminal,
            },
            Err(CallError::Encode(_) | CallError::Decode(_)) => RetryDecision::Terminal,
        }
    }

    fn classify_status(code: StatusCode, msg: &str) -> Self {
        match code {
            StatusCode::Unavailable => RetryDecision::Retry,
            StatusCode::ResourceExhausted => match ResourceExhaustedContext::classify(msg) {
                ResourceExhaustedContext::HardQuota => RetryDecision::Terminal,
                ResourceExhaustedContext::RateLimit => RetryDecision::RetryRateLimit,
                ResourceExhaustedContext::Capacity => RetryDecision::Retry,
            },
            // Explicit non-retryable variants — listed here so adding a new variant on
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

    /// The time the peer asked the caller to wait, from the status of a failed attempt: the `retry-after` or
    /// `x-ratelimit-reset-requests` trailer first (whole seconds), then a `[retry-after=Ns]` hint at the end of the
    /// message. `None` for any other failure or when there is no usable hint.
    pub fn read_retry_after(error: &CallError) -> Option<Duration> {
        let CallError::Status {
            message, trailers, ..
        } = error
        else {
            return None;
        };
        let from_trailers = RETRY_AFTER_KEYS.iter().find_map(|key| {
            trailers
                .get_values(GetValuesRequest::new(key))
                .map(GetValuesResponse::into_values)
                .unwrap_or_default()
                .first()
                .and_then(|value| value.trim().parse::<u64>().ok())
        });
        match from_trailers {
            Some(secs) => Some(Duration::from_secs(secs)),
            None => Self::parse_retry_after_hint(message),
        }
    }

    /// Extract a `Retry-After` hint embedded in a gRPC error message.
    ///
    /// The transport embeds the HTTP `Retry-After` (or
    /// `x-ratelimit-reset-requests`) header value as `[retry-after=Ns]`
    /// at the end of the `grpc-message` when it receives a
    /// `RESOURCE_EXHAUSTED` response.  This lets the retry loop honour
    /// the upstream reset window for rate-limit errors without requiring
    /// a new error variant or extra fields.
    ///
    /// Returns `None` when no hint is present or when the value cannot
    /// be parsed as a whole number of seconds.
    pub fn parse_retry_after_hint(message: &str) -> Option<Duration> {
        let tag = "[retry-after=";
        let start = message.find(tag)? + tag.len();
        let rest = &message[start..];
        let end = rest.find('s')?;
        let secs: u64 = rest[..end].parse().ok()?;
        Some(Duration::from_secs(secs))
    }
}
