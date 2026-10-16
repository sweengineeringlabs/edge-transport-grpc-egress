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
//!
//! Per SEA rule 160, the type *declaration* lives in api/. The
//! methods (`should_retry`, `is_rate_limit`, `classify`,
//! `parse_retry_after_hint`) live in `core/`.

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
