//! Classification of [`CallError`] values into the breaker's binary success/failure outcome.
//!
//! Single source of truth for which failures trip the breaker.
//!
//! ## Counted as failures
//!
//! - `CallError::Transport` with `Unavailable`, `Unknown` or `Internal`: the peer could not be reached, or the
//!   connection broke.
//! - `CallError::Status` with `Unavailable` or `Internal`: the peer says it is unhealthy.
//! - `CallError::Decode`: a response that cannot be read is an `Internal` condition of the peer.
//!
//! ## NOT counted as failures
//!
//! - `Status(ResourceExhausted, _)` — rate-limit signal, not an unhealthy upstream. The retry layer handles it.
//! - `Status(Unauthenticated, _)` / `Status(PermissionDenied, _)` — credentials are wrong; the upstream is fine.
//!   Tripping the breaker on auth failures would shed traffic that has nothing to do with upstream health.
//! - `Status(NotFound, _)`, `InvalidArgument`, `FailedPrecondition`, etc. — caller-visible business errors.
//!   Upstream working as intended.
//! - `DeadlineExceeded` (a status, or a transport failure) — the caller's own deadline, not a breaker condition.
//! - `Transport` with `Cancelled`, `InvalidArgument` or any other code — the caller cancelled, or the caller's
//!   own input was refused before anything was sent.
//! - `CallError::Encode` — the caller's own message could not be encoded.

use proto_pattern::{CallError, StatusCode};
use tracing::trace;

use grpc_egress_breaker::{FailureClassifier, Outcome, FAILURE_CLASSIFIER_LOG_TARGET};

/// Classifies the result of an outbound call into the breaker's outcome, from the [`CallError`] taxonomy of
/// the contract.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StdFailureClassifier;

impl FailureClassifier for StdFailureClassifier {
    fn classify_result<T>(&self, result: &Result<T, CallError>) -> Outcome {
        let is_breaker_failure = match result {
            Ok(_) => false,
            Err(CallError::Transport { code, .. }) => matches!(
                code,
                StatusCode::Unavailable | StatusCode::Unknown | StatusCode::Internal
            ),
            Err(CallError::Status { code, .. }) => {
                matches!(code, StatusCode::Unavailable | StatusCode::Internal)
            }
            Err(CallError::Decode(_)) => true,
            Err(CallError::Encode(_)) => false,
        };
        trace!(
            target: FAILURE_CLASSIFIER_LOG_TARGET,
            is_breaker_failure,
            "grpc-breaker: classifying outcome",
        );
        if is_breaker_failure {
            Outcome::Failure
        } else {
            Outcome::Success
        }
    }
}
