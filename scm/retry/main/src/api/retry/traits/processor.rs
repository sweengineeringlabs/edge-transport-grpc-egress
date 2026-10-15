//! `Processor` trait — primary processing contract for the retry crate.

use crate::api::Error;
use crate::api::GrpcRetryFacade;
use crate::api::ProcessorRequest;

/// Processes a retry decision given a gRPC result.
///
/// Implemented by `DefaultProcessor`
/// in `core/`.
pub trait Processor {
    /// Validate the retry configuration.
    ///
    /// Returns `Ok(())` when the configuration is valid, or
    /// [`Error::InvalidConfig`] when a field is out of range.
    fn validate(&self, req: ProcessorRequest) -> Result<(), Error>;

    /// Construct the facade that composes this crate's default
    /// implementations — gives [`GrpcRetryFacade`] a genuine role in
    /// this trait's signature set. `Self: Sized` keeps this trait
    /// dyn-compatible for `Box<dyn Trait>`.
    fn default_facade() -> GrpcRetryFacade
    where
        Self: Sized,
    {
        GrpcRetryFacade
    }
}
