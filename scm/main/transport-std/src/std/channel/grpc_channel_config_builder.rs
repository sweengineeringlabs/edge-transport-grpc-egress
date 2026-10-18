//! `GrpcChannelConfigBuilder` — builder for [`grpc_egress_transport::GrpcChannelConfig`].

use grpc_egress_transport::CompressionMode;
use grpc_egress_transport::KeepAliveConfig;
use grpc_egress_transport::MtlsConfig;

/// Builder for [`grpc_egress_transport::GrpcChannelConfig`].
#[derive(Debug, Default)]
pub struct GrpcChannelConfigBuilder {
    pub(crate) endpoint: Option<String>,
    pub(crate) tls_required: bool,
    pub(crate) mtls: Option<MtlsConfig>,
    pub(crate) keep_alive: Option<KeepAliveConfig>,
    pub(crate) max_message_bytes: Option<usize>,
    pub(crate) compression: Option<CompressionMode>,
}
