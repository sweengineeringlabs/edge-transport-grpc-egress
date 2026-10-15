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

use edge_transport_grpc_egress::{GrpcEgressError, GrpcStatusCode};
use tracing::trace;

use crate::api::{
    BreakerDomainError, ClassifyRequest, ClassifyResponse, FailureClassifier, Outcome,
    FAILURE_CLASSIFIER_LOG_TARGET,
};

/// Default [`FailureClassifier`] — inspects the foreign `GrpcEgressError`/
/// `GrpcStatusCode` taxonomy (kept out of the api/ trait boundary) and
/// reduces it to the boolean [`ClassifyRequest::is_breaker_failure`] signal
/// before delegating to the trait.
pub(crate) struct DefaultFailureClassifier;

impl FailureClassifier for DefaultFailureClassifier {
    fn classify(&self, req: ClassifyRequest) -> Result<ClassifyResponse, BreakerDomainError> {
        trace!(
            target: FAILURE_CLASSIFIER_LOG_TARGET,
            is_breaker_failure = req.is_breaker_failure,
            "grpc-breaker: classifying outcome",
        );
        Ok(ClassifyResponse {
            outcome: if req.is_breaker_failure {
                Outcome::Failure
            } else {
                Outcome::Success
            },
        })
    }
}

impl DefaultFailureClassifier {
    /// Classify an outbound result into the breaker's outcome.
    pub(crate) fn classify_result<T>(
        result: &Result<T, GrpcEgressError>,
    ) -> Result<Outcome, BreakerDomainError> {
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
        Ok(Self
            .classify(ClassifyRequest { is_breaker_failure })?
            .outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_request_failure_returns_outcome_failure() {
        assert_eq!(
            DefaultFailureClassifier
                .classify(ClassifyRequest {
                    is_breaker_failure: true
                })
                .expect("infallible")
                .outcome,
            Outcome::Failure
        );
    }

    #[test]
    fn test_classify_request_non_failure_returns_outcome_success() {
        assert_eq!(
            DefaultFailureClassifier
                .classify(ClassifyRequest {
                    is_breaker_failure: false
                })
                .expect("infallible")
                .outcome,
            Outcome::Success
        );
    }

    #[test]
    fn test_classify_result_ok_is_success() {
        let result: Result<(), GrpcEgressError> = Ok(());
        assert_eq!(
            DefaultFailureClassifier::classify_result(&result).expect("infallible"),
            Outcome::Success
        );
    }

    #[test]
    fn test_classify_result_unavailable_is_failure() {
        let result: Result<(), GrpcEgressError> = Err(GrpcEgressError::Unavailable("down".into()));
        assert_eq!(
            DefaultFailureClassifier::classify_result(&result).expect("infallible"),
            Outcome::Failure
        );
    }

    #[test]
    fn test_classify_result_permission_denied_is_success() {
        let result: Result<(), GrpcEgressError> = Err(GrpcEgressError::Status(
            GrpcStatusCode::PermissionDenied,
            "no".into(),
        ));
        assert_eq!(
            DefaultFailureClassifier::classify_result(&result).expect("infallible"),
            Outcome::Success
        );
    }

    #[test]
    fn test_classify_result_timeout_is_success() {
        let result: Result<(), GrpcEgressError> = Err(GrpcEgressError::Timeout("slow".into()));
        assert_eq!(
            DefaultFailureClassifier::classify_result(&result).expect("infallible"),
            Outcome::Success
        );
    }
}
