//! `Processor` trait — primary processing contract for this crate.

use futures::future::BoxFuture;

use crate::api::error::GrpcEgressError;
use crate::api::TransportSvc;
use crate::api::{ApplicationConfigBuilder, DescribeRequest, DescribeResponse, ProcessingRequest};

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

    /// Start a config builder pre-populated with this crate's name and
    /// version — gives [`ApplicationConfigBuilder`] a genuine role in this
    /// trait's signature set, not just an impl-site helper. `Self: Sized`
    /// keeps this trait dyn-compatible for `Box<dyn Trait>`.
    fn default_config_builder() -> ApplicationConfigBuilder
    where
        Self: Sized,
    {
        ApplicationConfigBuilder::default()
            .with_name(env!("CARGO_PKG_NAME"))
            .with_version(env!("CARGO_PKG_VERSION"))
    }

    /// Construct the SAF facade that exposes this crate's factory functions
    /// — gives [`TransportSvc`] a genuine role in this trait's signature
    /// set. `Self: Sized` keeps this trait dyn-compatible for `Box<dyn Trait>`.
    fn default_facade() -> TransportSvc
    where
        Self: Sized,
    {
        TransportSvc
    }
}
