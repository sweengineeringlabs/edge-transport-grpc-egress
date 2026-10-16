//! `Processor` trait — primary processing contract for this crate.

use crate::DescribeRequest;
use crate::DescribeResponse;

use crate::ResilientTransportError;

/// Primary processing trait for this crate (service_type = "processor").
/// Implemented by `GrpcResilientSvcProcessor` of `grpc-egress-resilient-std`.
pub trait Processor: Send + Sync {
    /// Identify this processor unit for logging and metrics.
    fn describe(&self, req: DescribeRequest) -> Result<DescribeResponse, ResilientTransportError>;
}
