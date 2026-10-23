//! SPI layer -- the service provider implementation: the only layer that names third-party crates.
//!
//! The gRPC egress transport is backed by `proto-svc` (`tonic`): the wire format, the connection, TLS,
//! compression and the status codes are its; the load balancer is `swe-edge-loadbalancer`. Those external-library
//! wrapper types live here, not in `api/` and not in `std/`.
pub(crate) mod cancellation;
pub(crate) mod client;
pub(crate) mod context;
pub(crate) mod loadbalancer;
