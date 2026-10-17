//! `cancellation` theme: ending a call from the caller's side with a signal.

pub(crate) mod cancellable_response_stream;
pub(crate) mod cancellable_transport;
pub(crate) mod token_cancellation;

pub(crate) use cancellable_response_stream::CancellableResponseStream;
pub(crate) use cancellable_transport::CancellableTransport;
pub use token_cancellation::TokenCancellation;
