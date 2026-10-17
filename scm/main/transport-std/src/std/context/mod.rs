//! `context` theme: carrying the identity of the caller on a call.

pub(crate) mod context_headers;
pub(crate) mod context_transport;
pub(crate) mod security_context_identity;

pub(crate) use context_headers::ContextHeaders;
pub(crate) use context_transport::ContextTransport;
pub use security_context_identity::SecurityContextIdentity;
