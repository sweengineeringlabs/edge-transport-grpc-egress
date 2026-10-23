//! Cancellation by signal: the view that ends calls when a signal fires, and the token as a signal.

pub(crate) mod cancellable_response_stream;
pub(crate) mod cancellable_transport;

pub(crate) use cancellable_transport::CancellableTransport;
