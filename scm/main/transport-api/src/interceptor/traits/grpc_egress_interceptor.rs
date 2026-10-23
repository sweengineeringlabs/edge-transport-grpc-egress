//! `GrpcEgressInterceptor` trait -- observes and changes a call before it is sent and after it came back.

use proto_pattern::{CallError, CallEvent, Metadata};

use crate::{InboundReply, OutboundCall};

/// An interceptor for outbound gRPC calls.
pub trait GrpcEgressInterceptor: Send + Sync {
    /// Run before the request is sent. The call carries the headers and the timeout `call` holds afterwards.
    ///
    /// # Errors
    ///
    /// Returning `Err(_)` aborts the call: nothing is sent, and the caller gets the error.
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError>;

    /// Run after a response with a message has been read: a unary call or a client-streaming call.
    ///
    /// # Errors
    ///
    /// Returning `Err(_)` makes the call fail with that error.
    fn after_call(&self, reply: &InboundReply<'_>) -> Result<(), CallError>;

    /// Observe the Pattern lifecycle for typed and raw unary and streaming operations.
    ///
    /// This object-safe, concrete-metadata adapter lets the egress keep runtime registration while
    /// using Pattern's generic [`CallEvent`] contract. Events are observations and cannot alter a
    /// call's result. The default keeps existing implementations source-compatible.
    fn on_event(&self, _event: &CallEvent<'_, Metadata>) {}
}
