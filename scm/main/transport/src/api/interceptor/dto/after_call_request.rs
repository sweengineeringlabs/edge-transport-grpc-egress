//! `AfterCallRequest` — request for [`crate::GrpcEgressInterceptor::after_call`].

use crate::GrpcResponse;

/// Request carrying the in-flight response for [`crate::GrpcEgressInterceptor::after_call`]
/// to observe or mutate before it is returned to the caller.
pub struct AfterCallRequest<'a> {
    /// The response received from the wire, mutable so interceptors can rewrite it.
    pub response: &'a mut GrpcResponse,
}
