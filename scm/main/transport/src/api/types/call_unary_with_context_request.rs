//! Request for [`crate::api::GrpcEgress::call_unary_with_context`].

use edge_security_runtime::SecurityContext;

use super::grpc_request::GrpcRequest;

/// Input to [`crate::api::GrpcEgress::call_unary_with_context`] — a unary
/// request plus the caller's security context to propagate.
pub struct CallUnaryWithContextRequest {
    /// The unary request to send.
    pub request: GrpcRequest,
    /// The caller's security context (e.g. for trace-id / JWT forwarding).
    pub ctx: SecurityContext,
}
