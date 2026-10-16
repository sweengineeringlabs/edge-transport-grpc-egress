//! `GrpcChannelConfigBuilder` — builder for [`crate::api::GrpcChannelConfig`].

use crate::api::CompressionMode;
use crate::api::KeepAliveConfig;
use crate::api::MtlsConfig;

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
