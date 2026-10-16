//! `grpc_egress_retry` -- the retry domain: the contracts, requests, responses, errors and the
//! retry policy value. The implementation (the decorator that wraps a `GrpcEgress`) is in
//! `grpc-egress-retry-std`.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;

pub use api::dto::BuildGrpcRetryRequest;
pub use api::error::Error;
pub use api::traits::GrpcRetryFactory;
pub use api::vo::GrpcRetryConfig;
