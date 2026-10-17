//! `BuildGrpcCancellationRequest` -- the request of the cancellation factory.

use tokio_util::sync::CancellationToken;

/// The client to bind and the token that ends its calls.
pub struct BuildGrpcCancellationRequest<'a, T> {
    /// The client whose calls the token ends.
    pub inner: &'a T,
    /// The token: when it fires, a call in flight fails with `Cancelled`.
    pub token: CancellationToken,
}
