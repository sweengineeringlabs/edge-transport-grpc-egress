//! `RefusingInterceptor` -- an interceptor that refuses every call.

use grpc_egress_transport::{GrpcEgressInterceptor, InboundReply, OutboundCall};
use proto_pattern::{CallError, StatusCode};

/// An interceptor whose `before_call` always fails, so the call is never sent.
pub struct RefusingInterceptor;

impl GrpcEgressInterceptor for RefusingInterceptor {
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
        Err(CallError::Transport {
            path: call.get_path().into(),
            code: StatusCode::PermissionDenied,
            reason: "refused by the example".to_string(),
        })
    }

    fn after_call(&self, _reply: &InboundReply<'_>) -> Result<(), CallError> {
        Ok(())
    }
}
