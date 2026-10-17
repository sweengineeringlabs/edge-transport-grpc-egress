//! `GrpcSecurityContextFactory` -- binds a client to a security context.

use proto_pattern::{RawTransport, Transport};

use crate::BuildGrpcSecurityContextRequest;

/// Binds a client to the [`SecurityContext`](edge_security_runtime::SecurityContext) of the caller.
///
/// Every call of the view carries the headers derived from the context: `x-trace-id`, `x-tenant-id`, `authorization:
/// Bearer <token>` when the context is authenticated and holds a token, and the entries of its `metadata`. A header the
/// call already carries is kept. The view borrows the client and is a [`Transport`] and a [`RawTransport`].
pub trait GrpcSecurityContextFactory {
    /// A view of `req.inner` whose calls carry the headers of `req.ctx`.
    fn build<'a, 'c, T: Transport + RawTransport>(
        &self,
        req: BuildGrpcSecurityContextRequest<'a, 'c, T>,
    ) -> impl Transport + RawTransport + use<'a, Self, T>;
}
