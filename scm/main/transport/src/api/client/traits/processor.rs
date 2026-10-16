//! `Processor` trait — primary processing contract for this crate.

use futures::future::BoxFuture;

use crate::api::GrpcEgressError;
use crate::api::{DescribeRequest, DescribeResponse, ProcessingRequest};

/// Primary processing trait — required because `service_type = "processor"` in Cargo.toml.
///
/// Implemented by the concrete gRPC egress client and the resilient client wrapper.
pub trait Processor: Send + Sync {
    /// Execute this processor unit's primary operation.
    ///
    /// Returns `Err` when the underlying transport or business logic fails.
    fn process(&self, req: ProcessingRequest) -> BoxFuture<'_, Result<(), GrpcEgressError>>;

    /// Identify this processor unit for logging and metrics.
    fn describe(&self, req: DescribeRequest) -> Result<DescribeResponse, GrpcEgressError>;
}
