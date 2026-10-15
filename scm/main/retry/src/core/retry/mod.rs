//! Core layer — this crate's single domain (gRPC retry).

pub(crate) mod backoff;
pub(crate) mod default_retry_inspector;
pub(crate) mod grpc;
pub(crate) mod resource_exhausted_context;
pub(crate) mod retry_decision;
pub(crate) mod retry_egress;
pub(crate) mod traits;
