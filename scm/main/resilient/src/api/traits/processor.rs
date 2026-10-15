//! `Processor` trait — primary processing contract for this crate.

use crate::api::DescribeRequest;
use crate::api::DescribeResponse;
use crate::api::GrpcResilientFacade;
use crate::api::ResilientTransportError;

/// Primary processing trait for this crate (service_type = "processor").
/// Implemented by [`crate::api::GrpcResilientSvcProcessor`] in `core/`.
pub trait Processor: Send + Sync {
    /// Identify this processor unit for logging and metrics.
    fn describe(&self, req: DescribeRequest) -> Result<DescribeResponse, ResilientTransportError>;

    /// Construct the facade that composes this crate's default
    /// implementations — gives [`GrpcResilientFacade`] a genuine role in
    /// this trait's signature set. `Self: Sized` keeps this trait
    /// dyn-compatible for `Box<dyn Trait>`.
    fn default_facade() -> GrpcResilientFacade
    where
        Self: Sized,
    {
        GrpcResilientFacade
    }
}
