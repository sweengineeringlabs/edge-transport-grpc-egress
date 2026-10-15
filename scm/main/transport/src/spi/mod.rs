//! SPI layer — implementations of `api/` contracts backed by external libraries.
//!
//! The gRPC egress transport is backed by hyper/hyper-rustls (tonic-style wire
//! framing). Those external-library wrapper types live here, not in `api/`.
pub(crate) mod client;
pub(crate) mod loadbalancer;
