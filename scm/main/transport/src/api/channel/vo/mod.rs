//! `vo` of the `channel` theme.

pub mod compression_mode;
pub mod grpc_channel_config;
pub mod keep_alive_config;
pub mod mtls_config;

pub use compression_mode::CompressionMode;
pub use grpc_channel_config::{
    GrpcChannelConfig, DEFAULT_MAX_MESSAGE_BYTES, DEFAULT_REQUEST_TIMEOUT_SECS,
};
pub use keep_alive_config::KeepAliveConfig;
pub use mtls_config::MtlsConfig;
