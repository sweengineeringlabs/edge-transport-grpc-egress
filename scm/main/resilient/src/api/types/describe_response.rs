//! Response for [`crate::api::Processor::describe`].

/// Output of [`crate::api::Processor::describe`] — a short label
/// identifying the processor in log / trace output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescribeResponse {
    /// Short identifying label for this processor.
    pub label: &'static str,
}
