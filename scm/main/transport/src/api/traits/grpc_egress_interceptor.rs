//! `GrpcEgressInterceptor` trait — observes/mutates requests before and after dispatch.

use crate::api::error::GrpcEgressError;
use crate::api::types::GrpcRequest;
use crate::api::{
    AfterCallRequest, GrpcClientBuilder, GrpcEgressInterceptorChain, GrpcRequestBuilder,
    TraceContextSource,
};

/// An interceptor for outbound gRPC calls.
pub trait GrpcEgressInterceptor: Send + Sync {
    /// Run before the request is sent on the wire.
    /// Returning `Err(_)` aborts the call — the transport is not invoked.
    fn before_call(&self, req: &mut GrpcRequest) -> Result<(), GrpcEgressError>;

    /// Run after a successful response has been read from the wire.
    /// Returning `Err(_)` converts the call result to that error.
    fn after_call(&self, req: AfterCallRequest<'_>) -> Result<(), GrpcEgressError>;

    /// Start a fluent [`GrpcRequestBuilder`] for programmatic request
    /// construction — gives it a genuine role in this trait's signature
    /// set, not just an impl-site helper. `Self: Sized` keeps this trait
    /// dyn-compatible for `Box<dyn Trait>`.
    fn default_request_builder() -> GrpcRequestBuilder
    where
        Self: Sized,
    {
        GrpcRequestBuilder::new()
    }

    /// Report how many interceptors are registered in `chain` — gives
    /// [`GrpcEgressInterceptorChain`] a genuine role in this trait's
    /// signature set. `Self: Sized` keeps this trait dyn-compatible for
    /// `Box<dyn Trait>`.
    fn describe_chain_len(chain: &GrpcEgressInterceptorChain) -> usize
    where
        Self: Sized,
    {
        chain.len()
    }

    /// Report whether `source` injects a static trace context — gives
    /// [`TraceContextSource`] a genuine role in this trait's signature set,
    /// not just an internal interceptor field. `Self: Sized` keeps this
    /// trait dyn-compatible for `Box<dyn Trait>`.
    fn describe_trace_source(source: TraceContextSource) -> bool
    where
        Self: Sized,
    {
        matches!(source, TraceContextSource::Static { .. })
    }

    /// Construct the gRPC client builder marker — gives
    /// [`GrpcClientBuilder`] a genuine role in this trait's signature set,
    /// not just an impl-site helper. `Self: Sized` keeps this trait
    /// dyn-compatible for `Box<dyn Trait>`.
    fn default_client_builder() -> GrpcClientBuilder
    where
        Self: Sized,
    {
        GrpcClientBuilder
    }
}
