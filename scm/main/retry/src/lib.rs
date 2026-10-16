//! `grpc_egress_retry` -- the retry domain: the contracts, requests, responses, errors and the
//! retry policy value. The implementation (the decorator that wraps a `GrpcEgress`) is in
//! `grpc-egress-retry-std`.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod dto;
mod error;
mod traits;
mod vo;

pub use dto::BuildGrpcRetryRequest;
pub use error::Error;
pub use traits::GrpcRetryFactory;
pub use vo::GrpcRetryConfig;
