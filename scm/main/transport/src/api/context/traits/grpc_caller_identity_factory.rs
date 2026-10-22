//! `GrpcCallerIdentityFactory` -- binds a client to the identity of a caller.

use proto_pattern::{RawTransport, Transport};

use crate::BuildGrpcCallerIdentityRequest;

/// Binds a client to a [`CallerIdentity`](crate::CallerIdentity).
///
/// Every call of the view carries the headers derived from the identity: `x-trace-id`, `x-tenant-id`, `authorization:
/// Bearer <token>` and the further headers it lists. A header the call already carries is kept. The view borrows the
/// client and is a [`Transport`] and a [`RawTransport`].
pub trait GrpcCallerIdentityFactory {
    /// A view of `req.inner` whose calls carry the headers of `req.identity`. The headers are derived when the view is
    /// made, so the view does not keep the identity borrowed.
    fn build<'a, 'c, T: Transport + RawTransport>(
        &self,
        req: BuildGrpcCallerIdentityRequest<'a, 'c, T>,
    ) -> impl Transport + RawTransport + use<'a, Self, T>;
}
