//! Response for [`crate::Processor::describe`].

/// Output of [`crate::Processor::describe`] — a short label
/// identifying the processor in log / trace output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescribeResponse {
    /// Short identifying label for this processor.
    pub label: &'static str,
}
