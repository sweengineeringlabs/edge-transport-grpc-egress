//! `StdRetryClassifierFactory` -- creates the classifier of attempts.

use grpc_egress_retry::{BuildRetryClassifierRequest, RetryClassifier, RetryClassifierFactory};

use crate::std::StdRetryClassifier;

/// Creates classifiers of gRPC's retryable failures.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdRetryClassifierFactory;

impl RetryClassifierFactory for StdRetryClassifierFactory {
    fn build(&self, _req: BuildRetryClassifierRequest) -> impl RetryClassifier + use<> {
        StdRetryClassifier
    }
}
