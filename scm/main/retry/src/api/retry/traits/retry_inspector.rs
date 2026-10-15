//! `RetryInspector` trait — retry-decision inspection contract.

use crate::api::Error;
use crate::api::ResourceExhaustedContext;
use crate::api::RetryDecision;
use crate::api::RetryInspectRequest;
use crate::api::RetryInspectResponse;

/// Interface for inspecting the retry loop's decision types.
///
/// Implemented by `DefaultRetryInspector`
/// in `core/`.
pub trait RetryInspector: Send + Sync {
    /// Identify this inspector for logging and metrics.
    fn describe(&self, req: RetryInspectRequest) -> Result<RetryInspectResponse, Error>;

    /// Report whether a decision indicates the call should be retried —
    /// gives [`RetryDecision`] a genuine role in this trait's signature
    /// set, not just an internal loop variable. `Self: Sized` keeps this
    /// trait dyn-compatible for `Box<dyn Trait>`.
    fn should_retry(decision: RetryDecision) -> bool
    where
        Self: Sized,
    {
        decision.should_retry()
    }

    /// Classify a `RESOURCE_EXHAUSTED` status message — gives
    /// [`ResourceExhaustedContext`] a genuine role in this trait's
    /// signature set.
    fn classify_resource_exhausted(message: &str) -> ResourceExhaustedContext
    where
        Self: Sized,
    {
        ResourceExhaustedContext::classify(message)
    }
}
