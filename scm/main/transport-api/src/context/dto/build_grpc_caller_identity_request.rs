//! `BuildGrpcCallerIdentityRequest` -- the request of the caller-identity factory.

use crate::CallerIdentity;

/// The client to bind and the identity its calls carry. The headers are derived from the identity when the view is
/// made, so the view borrows the client (`'a`) and the identity only while it is being made (`'c`).
pub struct BuildGrpcCallerIdentityRequest<'a, 'c, T> {
    /// The client whose calls carry the identity.
    pub inner: &'a T,
    /// The identity of the caller.
    pub identity: &'c dyn CallerIdentity,
}
