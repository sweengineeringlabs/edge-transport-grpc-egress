//! `GrpcCancellationFactory` -- binds a client to a cancellation signal.

use proto_pattern::{RawTransport, Transport};

use crate::{BuildGrpcCancellationRequest, CancellationSignal};

/// Binds a client to a [`CancellationSignal`].
///
/// Dropping the future of a call cancels the call; this is for the caller that holds a signal and wants every call, and
/// every message read from a stream, to end when it fires. The view borrows the client and costs nothing to make; it is
/// a [`Transport`] and a [`RawTransport`], so it is used as the client is.
pub trait GrpcCancellationFactory {
    /// A view of `req.inner` whose calls end with a `Cancelled` failure when `req.signal` fires.
    fn build<'a, T: Transport + RawTransport, S: CancellationSignal + 'static>(
        &self,
        req: BuildGrpcCancellationRequest<'a, T, S>,
    ) -> impl Transport + RawTransport + use<'a, Self, T, S>;
}
