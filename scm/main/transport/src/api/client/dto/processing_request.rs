//! `ProcessingRequest` — request marker for [`crate::api::Processor::process`].

/// Request marker for [`crate::api::Processor::process`].
///
/// `process` runs the processor's default unit of work against its own
/// configured state; this type carries no payload.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessingRequest;
