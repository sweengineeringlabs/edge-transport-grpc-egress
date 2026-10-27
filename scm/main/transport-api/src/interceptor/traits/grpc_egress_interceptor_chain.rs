//! `GrpcEgressInterceptorChain` -- an ordered chain of outbound interceptors.

use std::sync::Arc;

use proto_pattern::{CallError, CallEvent, Metadata};

use crate::{GrpcEgressInterceptor, InboundReply, OutboundCall};

/// A chain of [`GrpcEgressInterceptor`]s. Chain order is the order in which the interceptors were added, for
/// `before_call` and for `after_call`.
pub trait GrpcEgressInterceptorChain: Sized {
    /// Register `interceptor` at the end of the chain.
    fn push(self, interceptor: Arc<dyn GrpcEgressInterceptor>) -> Self;

    /// Number of registered interceptors.
    fn len(&self) -> usize;

    /// `true` when no interceptors are registered.
    fn is_empty(&self) -> bool;

    /// Run every `before_call` in order until one fails or all succeed.
    ///
    /// # Errors
    ///
    /// The error of the first interceptor that fails; the ones after it do not run.
    fn run_before(&self, call: &mut dyn OutboundCall) -> Result<(), CallError>;

    /// Run every `after_call` in order until one fails or all succeed.
    ///
    /// # Errors
    ///
    /// The error of the first interceptor that fails; the ones after it do not run.
    fn run_after(&self, reply: &InboundReply<'_>) -> Result<(), CallError>;

    /// Forward one Pattern lifecycle event to registered observers in order. Implementations
    /// must explicitly forward or discard events; there is no default fallback.
    fn run_event(&self, event: &CallEvent<'_, Metadata>);
}
