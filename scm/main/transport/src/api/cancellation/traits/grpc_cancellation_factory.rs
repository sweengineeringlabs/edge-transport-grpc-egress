//! `GrpcCancellationFactory` -- binds a client to a cancellation token.

use proto_pattern::{RawTransport, Transport};

use crate::BuildGrpcCancellationRequest;

/// Binds a client to a [`CancellationToken`](tokio_util::sync::CancellationToken).
///
/// Dropping the future of a call cancels the call; this is for the caller that holds a token and wants every
/// call, and every message read from a stream, to end when the token fires. The view borrows the client
/// and costs nothing to make; it is a [`Transport`] and a [`RawTransport`], so it is used as the client is.
pub trait GrpcCancellationFactory {
    /// A view of `req.inner` whose calls end with a `Cancelled` failure when `req.token` fires.
    fn build<'a, T: Transport + RawTransport>(
        &self,
        req: BuildGrpcCancellationRequest<'a, T>,
    ) -> impl Transport + RawTransport + use<'a, Self, T>;
}
