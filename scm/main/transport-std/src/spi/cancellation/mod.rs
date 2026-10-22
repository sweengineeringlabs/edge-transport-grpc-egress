//! `cancellation` theme: ending a call from the caller's side with a signal.

pub(crate) mod signal;

pub(crate) use signal::CancellableTransport;
pub use signal::TokenCancellation;
