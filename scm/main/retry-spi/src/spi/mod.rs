//! SPI layer -- the service provider implementation: the only layer that names third-party crates (`tokio`, `tracing`,
//! `toml`, `edge-transport-retry`).

pub(crate) mod config;
pub(crate) mod decorator;
