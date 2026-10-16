//! Classification of [`GrpcEgressError`] values into the
//! breaker's binary success/failure outcome.
//!
//! Single source of truth for which gRPC errors trip the breaker.
//!
//! ## Counted as failures
//!
//! - `GrpcEgressError::Unavailable(_)`
//! - `GrpcEgressError::ConnectionFailed(_)`
//! - `GrpcEgressError::Internal(_)`
//! - `GrpcEgressError::Status(Unavailable, _)`
//! - `GrpcEgressError::Status(Internal, _)`
//!
//! ## NOT counted as failures
//!
//! - `Status(ResourceExhausted, _)` — rate-limit signal, not an
//!   unhealthy upstream.  Retry layer handles it.
//! - `Status(Unauthenticated, _)` / `Status(PermissionDenied, _)` —
//!   credentials are wrong; the upstream is fine.  Tripping the
//!   breaker on auth failures would shed traffic that has nothing
//!   to do with upstream health.
//! - `Status(NotFound, _)`, `InvalidArgument`, `FailedPrecondition`,
//!   etc. — caller-visible business errors.  Upstream working as
//!   intended.
//! - `Timeout(_)` — caller's own deadline, not a breaker condition.
//! - `Cancelled(_)` — caller cancelled.

use grpc_egress_transport::{GrpcEgressError, GrpcStatusCode};
use tracing::trace;

use edge_transport_breaker_policy::Outcome;
use grpc_egress_breaker::FAILURE_CLASSIFIER_LOG_TARGET;

/// Classifies the result of an outbound call into the breaker's outcome, from the `GrpcEgressError` /
/// `GrpcStatusCode` taxonomy of the transport domain.
pub(crate) struct StdFailureClassifier;

impl StdFailureClassifier {
    /// Classify an outbound result into the breaker's outcome.
    pub(crate) fn classify_result<T>(result: &Result<T, GrpcEgressError>) -> Outcome {
        let is_breaker_failure = match result {
            Ok(_) => false,
            Err(GrpcEgressError::Unavailable(_))
            | Err(GrpcEgressError::ConnectionFailed(_))
            | Err(GrpcEgressError::Internal(_)) => true,
            Err(GrpcEgressError::Status(code, _)) => {
                matches!(code, GrpcStatusCode::Unavailable | GrpcStatusCode::Internal)
            }
            Err(GrpcEgressError::Timeout(_)) | Err(GrpcEgressError::Cancelled(_)) => false,
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
