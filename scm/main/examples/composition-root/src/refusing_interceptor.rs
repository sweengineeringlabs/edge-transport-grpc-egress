//! `RefusingInterceptor` -- an interceptor that refuses every call.

use grpc_egress_transport::{
    AfterCallRequest, GrpcEgressError, GrpcEgressInterceptor, GrpcRequest,
};

/// An interceptor whose `before_call` always fails, so the call is never sent.
pub struct RefusingInterceptor;

impl GrpcEgressInterceptor for RefusingInterceptor {
    fn before_call(&self, _req: &mut GrpcRequest) -> Result<(), GrpcEgressError> {
        Err(GrpcEgressError::Internal("refused by the example".into()))
    }

    fn after_call(&self, _req: AfterCallRequest<'_>) -> Result<(), GrpcEgressError> {
        Ok(())
    }
}
