//! `CancellationSignal` -- tells a call when it must end.

use std::future::Future;

/// Something that says when the calls of a client must end: a token, a channel, a shutdown flag, a deadline.
///
/// The view of a client that ends its calls (`GrpcCancellationFactory`) knows only this interface, so the caller keeps
/// the cancellation mechanism it already has and implements the trait for it.
pub trait CancellationSignal: Send + Sync {
    /// Resolves when the calls must end. It may be awaited any number of times, by any number of calls at once, and
    /// once it has resolved it resolves at once.
    fn wait_cancelled(&self) -> impl Future<Output = ()> + Send;
}
