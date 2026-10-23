//! Standard gRPC retry classification policy, independent of the transport engine.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod saf;
mod std;

pub use saf::StdRetryClassifierFactory;
