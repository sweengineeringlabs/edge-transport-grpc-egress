//! Composition site for [`FailureClassifier`] — one file per trait keeps
//! wiring focused.

use crate::std::failure_classifier::DefaultFailureClassifier;
use grpc_egress_breaker::FailureClassifier;

/// Factory for the default [`FailureClassifier`].
pub struct FailureClassifierFactory;

impl FailureClassifierFactory {
    /// Construct the default [`FailureClassifier`].
    pub fn create() -> Box<dyn FailureClassifier> {
        Box::new(DefaultFailureClassifier)
    }
}
