//! `grpc_egress_retry_std` — decorator that wraps a client that is a `Transport` and a `RawTransport` with
//! exponential-backoff-with-jitter retry on gRPC-canonical
//! retryable status codes.
//!
//! ```compile_fail
//! use grpc_egress_retry_std::saf::StdGrpcRetryFactory;
//! ```
//!
//! ## Retry policy
//!
//! Retries on `Unavailable` and `ResourceExhausted` (both are
//! transient on the gRPC side and have a real chance of
//! succeeding on a fresh attempt).  Never retries
//! `Unauthenticated` / `PermissionDenied` (those don't get better
//! by trying again — they need a credential refresh upstream).
//! Never retries `DeadlineExceeded` (the caller's deadline is
//! the retry budget; eating the deadline on a re-issue is
//! pointless) or `Internal` (server bug — amplifying it just
//! amplifies the bug).
//!
//! ## Deadline budget
//!
//! Total retries are bounded by the timeout the call carries in its `CallOptions`; a call with no timeout is
//! bounded by `max_attempts` alone. Even if the configured `max_attempts` allows more tries, the
//! loop stops when the elapsed time + next backoff would
//! overrun the timeout, and each attempt is given what is left of it.
//!
//! ## What is retried
//!
//! Unary calls, typed and raw. A typed call is encoded once with its own `Payload` (an attempt after the
//! first makes the message again from those bytes) and goes to the inner client as a typed call; a raw call sends
//! the same bytes each time. The three streaming shapes are forwarded as they are: a stream cannot be re-sent.
//!
//! ## Composition
//!
//! Wrap any client that is a `Transport` and a `RawTransport` with
//! [`StdGrpcRetryFactory`]: `StdGrpcRetryFactory::new(config).build(BuildGrpcRetryRequest { inner })`.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod saf;
mod spi;
mod std;

pub use saf::{
    StdGrpcRetryConfigBuilderFactory, StdGrpcRetryConfigProviderFactory, StdGrpcRetryFactory,
    StdRetryClassifierFactory,
};
