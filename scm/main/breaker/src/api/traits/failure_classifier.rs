//! `FailureClassifier` -- which failures trip the breaker.
//!
//! Counted as failures: `Transport` with `Unavailable`, `Unknown` or `Internal` (the peer could not be reached, or the
//! connection broke); `Status` with `Unavailable` or `Internal` (the peer says it is unhealthy); `Decode` (a response
//! that cannot be read is an `Internal` condition of the peer).
//!
//! Not counted: `Status(ResourceExhausted)` (a rate-limit signal the retry layer handles), `Unauthenticated` and
//! `PermissionDenied` (the credentials are wrong, the upstream is fine), business errors such as `NotFound`,
//! `InvalidArgument` or `FailedPrecondition`, `DeadlineExceeded` (the caller's own deadline), a `Transport` failure
//! with `Cancelled`, `InvalidArgument` or any other code, and `Encode` (the caller's own message).

use proto_pattern::CallError;

use crate::Outcome;

/// Classifies the result of an outbound call into the breaker's outcome, from the [`CallError`] taxonomy of the
/// contract. The single source of truth for which failures trip the breaker.
pub trait FailureClassifier {
    /// Classify an outbound result into the breaker's outcome.
    fn classify_result<T>(&self, result: &Result<T, CallError>) -> Outcome;
}
