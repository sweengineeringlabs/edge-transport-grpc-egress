//! API of the `channel` theme: how a channel to a peer is configured.

pub mod dto;
pub mod error;
pub mod traits;
pub mod vo;

pub use dto::{
    BuildGrpcChannelConfigBuilderRequest, BuildMtlsConfigRequest, BuildTlsSettingsResolverRequest,
    ResolveTlsSettingsRequest,
};
pub use error::GrpcChannelConfigError;
pub use traits::{
    GrpcChannelConfigBuilder, GrpcChannelConfigBuilderFactory, MtlsConfigFactory,
    TlsSettingsResolver, TlsSettingsResolverFactory,
};
pub use vo::{
    CompressionMode, GrpcChannelConfig, KeepAliveConfig, MtlsConfig, DEFAULT_MAX_MESSAGE_BYTES,
    DEFAULT_REQUEST_TIMEOUT_SECS,
};
