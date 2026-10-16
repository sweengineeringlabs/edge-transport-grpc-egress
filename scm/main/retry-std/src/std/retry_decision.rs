//! Per-status retry decision.  Single source of truth for which
//! gRPC status codes the decorator will retry.
//!
//! The policy is hand-written, not config-driven, because the
//! retryable set is a property of gRPC's semantic contract — not
//! a tunable knob.  Specifically:
//!
//! - `Unavailable` and `ResourceExhausted` are retryable per the
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

use crate::std::ResourceExhaustedContext;
use grpc_egress_transport::{GrpcEgressError, GrpcStatusCode};

/// Decision returned by [`RetryDecision::classify`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    /// Treat as success — return to caller, no retry.
    Success,
    /// Retry-eligible failure — use the standard backoff schedule.
    Retry,
    /// Retry-eligible `ResourceExhausted(RateLimit)` — use the
    /// rate-limit backoff track (slower, respects Retry-After hints).
    RetryRateLimit,
    /// Terminal failure — surface to caller without retrying.
    Terminal,
}

impl RetryDecision {
    /// True if the decision indicates the call should be re-issued.
    pub fn should_retry(self) -> bool {
        matches!(self, RetryDecision::Retry | RetryDecision::RetryRateLimit)
    }

    /// True if the decision is the rate-limit variant.
    pub fn is_rate_limit(self) -> bool {
        matches!(self, RetryDecision::RetryRateLimit)
    }

    /// Classify an outbound result into a retry decision.
    ///
    /// Mapping table (for non-`Ok` outcomes):
    ///
    /// | Variant                                            | Decision          |
    /// |----------------------------------------------------|-------------------|
    /// | `Status(Unavailable, _)` / `Unavailable(_)`        | `Retry`           |
    /// | `Status(ResourceExhausted, _)` — RateLimit msg     | `RetryRateLimit`  |
    /// | `Status(ResourceExhausted, _)` — Capacity msg      | `Retry`           |
    /// | `Status(ResourceExhausted, _)` — HardQuota msg     | `Terminal`        |
    /// | `Status(Unauthenticated, _)`                       | `Terminal`        |
    /// | `Status(PermissionDenied, _)`                      | `Terminal`        |
    /// | `Status(DeadlineExceeded, _)` / `Timeout(_)`       | `Terminal`        |
    /// | `Status(Internal, _)` / `Internal(_)`              | `Terminal`        |
    /// | `ConnectionFailed(_)`                              | `Retry`           |
    /// | `Cancelled(_)` / `Status(Cancelled, _)`            | `Terminal`        |
    /// | other `Status(_, _)`                               | `Terminal`        |
    ///
    /// `ConnectionFailed` is treated as `Retry` because it's a
    /// transport-level transient (DNS hiccup, TCP RST during a rolling
    /// deploy) matching canonical `Unavailable` gRPC semantics.
    pub fn classify<T>(result: &Result<T, GrpcEgressError>) -> Self {
        match result {
            Ok(_) => RetryDecision::Success,
            Err(GrpcEgressError::Status(code, msg)) => Self::classify_status(*code, msg),
            Err(GrpcEgressError::ConnectionFailed(_)) => RetryDecision::Retry,
            Err(GrpcEgressError::Unavailable(_)) => RetryDecision::Retry,
            Err(GrpcEgressError::Timeout(_))
            | Err(GrpcEgressError::Internal(_))
            | Err(GrpcEgressError::Cancelled(_)) => RetryDecision::Terminal,
        }
    }

    /// The decision for a gRPC status the peer answered with.
    fn classify_status(code: GrpcStatusCode, msg: &str) -> Self {
        match code {
            GrpcStatusCode::Unavailable => RetryDecision::Retry,
            GrpcStatusCode::ResourceExhausted => match ResourceExhaustedContext::classify(msg) {
                ResourceExhaustedContext::HardQuota => RetryDecision::Terminal,
                ResourceExhaustedContext::RateLimit => RetryDecision::RetryRateLimit,
                ResourceExhaustedContext::Capacity => RetryDecision::Retry,
            },
            // Explicit non-retryable variants — listed here so adding a new variant on
            // `GrpcStatusCode` surfaces as a missing arm, not a silent default.
            GrpcStatusCode::Unauthenticated
            | GrpcStatusCode::PermissionDenied
            | GrpcStatusCode::DeadlineExceeded
            | GrpcStatusCode::Internal
            | GrpcStatusCode::Cancelled
            | GrpcStatusCode::Ok
            | GrpcStatusCode::Unknown
            | GrpcStatusCode::InvalidArgument
            | GrpcStatusCode::NotFound
            | GrpcStatusCode::AlreadyExists
            | GrpcStatusCode::FailedPrecondition
            | GrpcStatusCode::Aborted
            | GrpcStatusCode::OutOfRange
            | GrpcStatusCode::Unimplemented
            | GrpcStatusCode::DataLoss => RetryDecision::Terminal,
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
