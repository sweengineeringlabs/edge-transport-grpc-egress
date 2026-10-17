//! `GrpcEgressInterceptor` trait -- observes and changes a call before it is sent and after it came back.

use proto_pattern::CallError;

use crate::{InboundReply, OutboundCall};

/// An interceptor for outbound gRPC calls.
pub trait GrpcEgressInterceptor: Send + Sync {
    /// Run before the request is sent. The call carries `call.metadata` and `call.timeout` afterwards.
    ///
    /// # Errors
    ///
    /// Returning `Err(_)` aborts the call: nothing is sent, and the caller gets the error.
    fn before_call(&self, call: &mut OutboundCall<'_>) -> Result<(), CallError>;

    /// Run after a response with a message has been read: a unary call or a client-streaming call.
    ///
    /// # Errors
    ///
    /// Returning `Err(_)` makes the call fail with that error.
    fn after_call(&self, reply: &InboundReply<'_>) -> Result<(), CallError>;
}
