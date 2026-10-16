//! API of the `channel` theme: how a channel to a peer is configured.

pub mod error;
pub mod vo;

pub use error::GrpcChannelConfigError;
pub use vo::{
    CompressionMode, GrpcChannelConfig, KeepAliveConfig, MtlsConfig, DEFAULT_MAX_MESSAGE_BYTES,
    DEFAULT_REQUEST_TIMEOUT_SECS,
};
