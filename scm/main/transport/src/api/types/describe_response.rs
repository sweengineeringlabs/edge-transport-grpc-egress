//! `DescribeResponse` — response for [`crate::api::Processor::describe`].

/// Response for [`crate::api::Processor::describe`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescribeResponse {
    /// A short label identifying this processor unit for logging and metrics.
    pub label: &'static str,
}
