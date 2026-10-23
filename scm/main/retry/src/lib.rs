//! `grpc_egress_retry` -- the retry domain: the contracts, requests, responses, errors and the
//! retry policy value. The implementation (the decorator that wraps a `Transport` and a `RawTransport`) is in
//! `grpc-egress-retry-spi` (with the standard policy implementation from `grpc-egress-retry-std`).
//!
//! ```compile_fail
//! use grpc_egress_retry::api::vo::GrpcRetryConfig;
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod api;

pub use api::dto::{
    BuildGrpcRetryConfigBuilderRequest, BuildGrpcRetryConfigProviderRequest,
    BuildGrpcRetryConfigRequest, BuildGrpcRetryRequest, BuildRetryClassifierRequest,
};
pub use api::error::Error;
pub use api::traits::{
    GrpcRetryConfigBuilder, GrpcRetryConfigBuilderFactory, GrpcRetryConfigProvider,
    GrpcRetryConfigProviderFactory, GrpcRetryFactory, RetryClassifier, RetryClassifierFactory,
};
pub use api::vo::{GrpcRetryConfig, ResourceExhaustedContext, RetryDecision};

/// The contract the interfaces of this crate are written over: an implementation names `CallError`, `StatusCode`,
/// `Transport` and the rest through the domain crate, and so depends on nothing but the domain crate.
pub use proto_pattern;
