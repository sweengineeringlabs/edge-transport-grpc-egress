//! `GrpcEgressInterceptor` trait — observes/mutates requests before and after dispatch.

use crate::AfterCallRequest;
use crate::GrpcEgressError;
use crate::GrpcRequest;

/// An interceptor for outbound gRPC calls.
pub trait GrpcEgressInterceptor: Send + Sync {
    /// Run before the request is sent on the wire.
    /// Returning `Err(_)` aborts the call — the transport is not invoked.
    fn before_call(&self, req: &mut GrpcRequest) -> Result<(), GrpcEgressError>;

    /// Run after a successful response has been read from the wire.
    /// Returning `Err(_)` converts the call result to that error.
    fn after_call(&self, req: AfterCallRequest<'_>) -> Result<(), GrpcEgressError>;
}
