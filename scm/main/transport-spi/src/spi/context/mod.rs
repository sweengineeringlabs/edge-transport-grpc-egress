//! Caller-identity propagation over the selected transport engine.

mod context_headers;
mod context_transport;

pub(crate) use context_headers::ContextHeaders;
pub(crate) use context_transport::ContextTransport;
