//! Primary processing contract for bearer egress interceptors.

use super::super::types::BearerEgressInterceptor;

/// Primary processing contract for bearer egress interceptors.
///
/// Implemented by [`BearerEgressInterceptor`] to mark the type as a
/// first-class processor in the SEA pipeline.
pub trait Processor: Send + Sync {
    /// Get a reference to the interceptor instance.
    ///
    /// This ensures [`BearerEgressInterceptor`] appears in the trait signature
    /// and enables runtime polymorphism.
    fn as_interceptor(&self) -> &BearerEgressInterceptor;
}
