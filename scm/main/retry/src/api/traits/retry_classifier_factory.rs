//! `RetryClassifierFactory` -- creates the classifier of attempts.

use crate::{BuildRetryClassifierRequest, RetryClassifier};

/// Creates a [`RetryClassifier`].
pub trait RetryClassifierFactory {
    /// The classifier of gRPC's retryable failures.
    fn build(&self, req: BuildRetryClassifierRequest) -> impl RetryClassifier + use<Self>;
}
