//! `BuildGrpcSecurityContextRequest` -- the request of the security-context factory.

use edge_security_runtime::SecurityContext;

/// The client to bind and the context its calls carry. The headers are derived from the context when the view is
/// made, so the view borrows the client (`'a`) and the context only while it is being made (`'c`).
pub struct BuildGrpcSecurityContextRequest<'a, 'c, T> {
    /// The client whose calls carry the context.
    pub inner: &'a T,
    /// The security context of the caller.
    pub ctx: &'c SecurityContext,
}
