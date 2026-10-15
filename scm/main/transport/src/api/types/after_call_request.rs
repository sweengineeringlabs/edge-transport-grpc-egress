//! `AfterCallRequest` — request for [`crate::api::GrpcEgressInterceptor::after_call`].

use crate::api::types::GrpcResponse;

/// Request carrying the in-flight response for [`crate::api::GrpcEgressInterceptor::after_call`]
/// to observe or mutate before it is returned to the caller.
pub struct AfterCallRequest<'a> {
    /// The response received from the wire, mutable so interceptors can rewrite it.
    pub response: &'a mut GrpcResponse,
}
