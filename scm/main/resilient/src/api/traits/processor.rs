//! `Processor` trait — primary processing contract for this crate.

use crate::api::DescribeRequest;
use crate::api::DescribeResponse;

use crate::api::ResilientTransportError;

/// Primary processing trait for this crate (service_type = "processor").
/// Implemented by [`crate::std::GrpcResilientSvcProcessor`] in `core/`.
pub trait Processor: Send + Sync {
    /// Identify this processor unit for logging and metrics.
    fn describe(&self, req: DescribeRequest) -> Result<DescribeResponse, ResilientTransportError>;
}
