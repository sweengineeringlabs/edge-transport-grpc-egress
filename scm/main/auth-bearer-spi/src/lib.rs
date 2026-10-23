//! Provider-specific bearer implementations: JWT signing and constant-time secret comparison.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod saf;
mod spi;

pub use crate::saf::{StdBearerInterceptorFactory, StdBearerSecretGuardFactory};
