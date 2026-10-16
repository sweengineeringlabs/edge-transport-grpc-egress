//! `Processor` trait — primary processing contract for the retry crate.

use crate::Error;
use crate::ProcessorRequest;

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
}
