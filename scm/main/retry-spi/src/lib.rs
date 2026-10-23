//! Provider-backed gRPC retry decorator and TOML policy readers.
//!
//! Unary calls (typed and raw) are retried with exponential backoff and jitter. Streaming calls are forwarded because
//! a stream cannot be replayed. The retry policy is supplied by the standard classifier package.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod saf;
mod spi;

pub use crate::saf::{
    StdGrpcRetryConfigBuilderFactory, StdGrpcRetryConfigProviderFactory, StdGrpcRetryFactory,
};
