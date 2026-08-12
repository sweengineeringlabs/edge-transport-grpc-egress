//! `GrpcChannelConfigBuilder` — builder for [`crate::api::GrpcChannelConfig`].

use crate::api::types::compression_mode::CompressionMode;
use crate::api::types::keep_alive_config::KeepAliveConfig;
use crate::api::types::mtls_config::MtlsConfig;

/// Builder for [`crate::api::GrpcChannelConfig`].
#[derive(Debug, Default)]
pub struct GrpcChannelConfigBuilder {
    pub(crate) endpoint: Option<String>,
    pub(crate) tls_required: bool,
    pub(crate) mtls: Option<MtlsConfig>,
    pub(crate) keep_alive: Option<KeepAliveConfig>,
    pub(crate) max_message_bytes: Option<usize>,
    pub(crate) compression: Option<CompressionMode>,
}
