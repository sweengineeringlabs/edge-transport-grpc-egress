//! `RetryInspector` trait — retry-decision inspection contract.

use crate::Error;
use crate::RetryInspectRequest;
use crate::RetryInspectResponse;

/// Interface for inspecting the retry loop's decision types.
///
/// Implemented by `DefaultRetryInspector`
/// in `core/`.
pub trait RetryInspector: Send + Sync {
    /// Identify this inspector for logging and metrics.
    fn describe(&self, req: RetryInspectRequest) -> Result<RetryInspectResponse, Error>;
}
