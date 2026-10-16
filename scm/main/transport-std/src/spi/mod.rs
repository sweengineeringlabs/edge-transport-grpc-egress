//! SPI layer — implementations of `api/` contracts backed by external libraries.
//!
//! The gRPC egress transport is backed by `proto-svc` (`tonic`): the wire format, the connection, TLS,
//! compression and the status codes are its. Those external-library wrapper types live here, not in
//! `api/`.
pub(crate) mod client;
pub(crate) mod loadbalancer;
