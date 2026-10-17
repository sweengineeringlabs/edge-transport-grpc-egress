//! `BuildGrpcCancellationRequest` -- the request of the cancellation factory.

/// The client to bind and the signal that ends its calls.
pub struct BuildGrpcCancellationRequest<'a, T, S> {
    /// The client whose calls the signal ends.
    pub inner: &'a T,
    /// The signal: when it fires, a call in flight fails with `Cancelled`.
    pub signal: S,
}
