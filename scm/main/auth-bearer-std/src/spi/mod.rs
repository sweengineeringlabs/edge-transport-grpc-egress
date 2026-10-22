//! SPI layer -- the service provider implementation: the only layer that names third-party crates (`jsonwebtoken`,
//! `subtle`, `tracing`, `proto-pattern`).

pub(crate) mod jsonwebtoken;
pub(crate) mod subtle;
