//! `GrpcEgressInterceptor` mutates or rejects a call around message exchange and observes lifecycle events.

use proto_pattern::{CallError, CallEvent, Metadata};

use crate::{InboundReply, OutboundCall};

/// An interceptor for outbound gRPC calls.
pub trait GrpcEgressInterceptor: Send + Sync {
    /// Mutate or reject a request before it is sent. The call carries the headers and timeout afterward.
    ///
    /// # Errors
    ///
    /// Returning `Err(_)` aborts the call: nothing is sent, and the caller gets the error.
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError>;

    /// Inspect or reject a successful response after a message has been read (unary or client-streaming).
    ///
    /// # Errors
    ///
    /// Returning `Err(_)` makes the call fail with that error.
    fn after_call(&self, reply: &InboundReply<'_>) -> Result<(), CallError>;

    /// Observe the Pattern lifecycle for typed and raw unary and streaming operations.
    ///
    /// This object-safe, concrete-metadata adapter lets the egress keep runtime registration while
    /// using Pattern's generic [`CallEvent`] contract. Events are observations and cannot alter a
    /// call's result. Implementations with no observer behavior must provide an explicit no-op.
    fn on_event(&self, event: &CallEvent<'_, Metadata>);
}
