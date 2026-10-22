//! `RetryClassifier` -- which failures the retry decorator retries.
//!
//! The policy is hand-written, not config-driven, because the retryable set is a property of gRPC's semantic
//! contract, not a tunable knob:
//!
//! - `Unavailable` (a status the peer sent, or a call that did not reach it) and `ResourceExhausted` are retryable per
//!   the gRPC retry whitepaper (the latter with a longer backoff, see [`RetryDecision::RetryRateLimit`]).
//! - `ResourceExhausted` with a hard quota is NOT retried -- a billing cap won't clear on retry.
//! - `Unauthenticated` and `PermissionDenied` MUST NOT be retried -- a bad token won't become good by trying again.
//! - `DeadlineExceeded` must not be retried -- the caller's deadline already counts the retry budget.
//! - `Internal` is not retried -- server bug, retrying just amplifies the bug and burns the deadline.

use std::time::Duration;

use proto_pattern::CallError;

use crate::{ResourceExhaustedContext, RetryDecision};

/// Decides what to do with the result of one attempt.
pub trait RetryClassifier {
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
    /// failure with it); a `Status` of code `Unknown` is the peer's own answer and is terminal.
    fn classify<T>(&self, result: &Result<T, CallError>) -> RetryDecision;

    /// Classify a `RESOURCE_EXHAUSTED` grpc-message into a context.
    fn classify_resource_exhausted(&self, message: &str) -> ResourceExhaustedContext;

    /// `true` for `Retry` and `RetryRateLimit`.
    fn should_retry(&self, decision: RetryDecision) -> bool;

    /// `true` for `RetryRateLimit`.
    fn is_rate_limit(&self, decision: RetryDecision) -> bool;

    /// The time the peer asked the caller to wait, from the status of a failed attempt: the `retry-after` or
    /// `x-ratelimit-reset-requests` trailer first (whole seconds), then a `[retry-after=Ns]` hint at the end of the
    /// message. `None` for any other failure or when there is no usable hint.
    fn read_retry_after(&self, error: &CallError) -> Option<Duration>;

    /// Extract a `Retry-After` hint embedded in a gRPC error message as `[retry-after=Ns]`. `None` when no hint is
    /// present or when the value cannot be parsed as a whole number of seconds.
    fn parse_retry_after_hint(&self, message: &str) -> Option<Duration>;
}
