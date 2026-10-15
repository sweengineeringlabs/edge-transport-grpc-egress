//! `Processor` trait — primary processing contract for the breaker crate.

use crate::api::BreakerDomainError;
use crate::api::DescribeRequest;
use crate::api::DescribeResponse;

/// Primary processing trait for this crate (service_type = "processor").
/// Every circuit-breaker middleware produced by this crate implements it.
pub trait Processor: Send + Sync {
    /// Identify this processor in log / trace output.
    fn describe(&self, req: DescribeRequest) -> Result<DescribeResponse, BreakerDomainError>;
}
