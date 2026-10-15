//! Response for [`crate::api::RetryInspector::describe`].

/// Output of [`crate::api::RetryInspector::describe`] — a short label
/// identifying the inspector in log / trace output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryInspectResponse {
    /// Short identifying label for this inspector.
    pub label: &'static str,
}
