//! `grpc_egress_breaker_std` — circuit-breaker decorator around any client that is a
//! [`Transport`](Transport) and a [`RawTransport`](RawTransport).
//!
//! ## State machine
//!
//! Standard three-state breaker:
//!
//! - **Closed**: requests pass through; consecutive
//!   `Unavailable`/`Internal` failures are counted.  At
//!   `failure_threshold` the breaker trips Open.
//! - **Open**: requests short-circuit with a `CallError::Transport` whose code is
//!   `Unavailable` *without* calling the
//!   inner client.  After `cool_down`, the next request promotes
//!   to HalfOpen.
//! - **HalfOpen**: a small number of probe requests are admitted.
//!   `half_open_probe_count` consecutive successes → Closed.
//!   Any failure during probing → back to Open.
//!
//! ## Failure classification
//!
//! Only `Unavailable` (status or transport variant) and
//! `Internal` count as breaker failures (and a transport failure the
//! engine could not classify, `Unknown`, and a response that cannot be decoded).  `ResourceExhausted` is
//! a rate-limit signal that the retry layer handles, not a
//! reason to shed traffic.  `Unauthenticated` / `PermissionDenied`
//! reflect a bad credential, not an unhealthy upstream — the
//! upstream is fine; the call is just unauthorized.
//!
//! ## Streams
//!
//! A streaming call is admitted before it opens and its *open* is recorded: an open that fails counts like a
//! failed unary call, an open that succeeds closes a half-open breaker. What happens on a stream after it opened
//! is not recorded. A call refused by an open circuit never reaches the wrapped client.
//!
//! ## Observability
//!
//! State transitions emit `tracing` events at INFO (Open, Closed)
//! and DEBUG (HalfOpen probe).  No internal metrics framework —
//! consumers wire `tracing` into whatever they already use.

#![warn(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]

mod saf;
mod std;

pub use crate::std::GrpcBreakerConfigProvider;
pub use saf::StdGrpcBreakerFactory;
