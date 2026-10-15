//! `ConsecutiveFailuresRequest` — request for [`crate::api::ResilientGrpcClientPort::consecutive_failures`].

/// Request marker for [`crate::api::ResilientGrpcClientPort::consecutive_failures`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ConsecutiveFailuresRequest;
