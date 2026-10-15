//! Response for [`crate::api::RetryDecorator::describe_policy`].

/// Output of [`crate::api::RetryDecorator::describe_policy`] — a short
/// human-readable summary of the configured retry policy, for logging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescribePolicyResponse {
    /// Summary of the key policy fields.
    pub summary: String,
}
