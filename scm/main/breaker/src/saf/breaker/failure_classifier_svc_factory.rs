//! Composition site for [`FailureClassifier`] — one file per trait keeps
//! wiring focused.

use crate::api::FailureClassifier;
use crate::core::breaker::failure_classifier::DefaultFailureClassifier;

/// Factory for the default [`FailureClassifier`].
pub struct FailureClassifierFactory;

impl FailureClassifierFactory {
    /// Construct the default [`FailureClassifier`].
    pub fn create() -> Box<dyn FailureClassifier> {
        Box::new(DefaultFailureClassifier)
    }
}
