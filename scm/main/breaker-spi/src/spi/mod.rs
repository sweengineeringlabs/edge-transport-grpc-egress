//! SPI layer -- the service provider implementation: the only layer that names third-party crates (`tokio`, `futures`,
//! `tracing`, `toml`, the breaker policy crates).

pub(crate) mod config;
pub(crate) mod decorator;
