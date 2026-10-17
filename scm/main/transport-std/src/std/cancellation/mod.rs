//! `cancellation` theme: ending a call from the caller's side with a token.

pub(crate) mod cancellable_response_stream;
pub(crate) mod cancellable_transport;

pub(crate) use cancellable_response_stream::CancellableResponseStream;
pub(crate) use cancellable_transport::CancellableTransport;
