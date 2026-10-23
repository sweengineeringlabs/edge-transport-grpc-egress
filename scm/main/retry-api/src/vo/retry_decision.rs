//! What the retry loop does with the result of one attempt.

/// What the retry loop does with the result of one attempt. The decision is made by
/// [`RetryClassifier::classify`](crate::RetryClassifier::classify), the single source of truth for which gRPC status
/// codes the decorator retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    /// The attempt succeeded; return it.
    Success,
    /// Retry on the standard backoff track.
    Retry,
    /// Retry on the rate-limit backoff track.
    RetryRateLimit,
    /// Surface the failure to the caller.
    Terminal,
}
