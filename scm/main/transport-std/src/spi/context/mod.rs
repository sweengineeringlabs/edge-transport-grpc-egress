//! `context` theme: the identity of the caller, read from the security runtime.

pub(crate) mod security;

pub use security::SecurityContextIdentity;
