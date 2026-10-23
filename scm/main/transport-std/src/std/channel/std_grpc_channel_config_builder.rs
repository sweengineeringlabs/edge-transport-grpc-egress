//! `StdGrpcChannelConfigBuilder` -- the standard [`GrpcChannelConfigBuilder`].

use std::time::Duration;

use grpc_egress_transport::{
    CompressionMode, GrpcChannelConfig, GrpcChannelConfigBuilder, KeepAliveConfig, MtlsConfig,
    DEFAULT_MAX_MESSAGE_BYTES,
};

/// Collects the settings of a channel and builds the [`GrpcChannelConfig`]. TLS is required until
/// `allow_plaintext` is called.
#[derive(Debug)]
pub(crate) struct StdGrpcChannelConfigBuilder {
    endpoint: Option<String>,
    tls_required: bool,
    mtls: Option<MtlsConfig>,
    keep_alive: Option<KeepAliveConfig>,
    max_message_bytes: Option<usize>,
    compression: Option<CompressionMode>,
    request_timeout_secs: Option<u64>,
}

impl Default for StdGrpcChannelConfigBuilder {
    /// A builder with nothing set and TLS required.
    fn default() -> Self {
        Self {
            endpoint: None,
            tls_required: true,
            mtls: None,
            keep_alive: None,
            max_message_bytes: None,
            compression: None,
            request_timeout_secs: None,
        }
    }
}

impl GrpcChannelConfigBuilder for StdGrpcChannelConfigBuilder {
    fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    fn allow_plaintext(mut self) -> Self {
        self.tls_required = false;
        self
    }

    fn with_mtls(mut self, mtls: MtlsConfig) -> Self {
        self.mtls = Some(mtls);
        self
    }

    fn with_keep_alive(mut self, keep_alive: KeepAliveConfig) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }

    fn with_max_message_bytes(mut self, bytes: usize) -> Self {
        self.max_message_bytes = Some(bytes);
        self
    }

    fn with_compression(mut self, mode: CompressionMode) -> Self {
        self.compression = Some(mode);
        self
    }

    fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout_secs = Some(timeout.as_secs().max(1));
        self
    }

    fn build(self) -> Result<GrpcChannelConfig, String> {
        let endpoint = self.endpoint.ok_or("endpoint required")?;
        GrpcChannelConfig::try_new(
            endpoint,
            self.tls_required,
            self.mtls,
            self.keep_alive,
            self.max_message_bytes.unwrap_or(DEFAULT_MAX_MESSAGE_BYTES),
            self.compression.unwrap_or(CompressionMode::None),
            self.request_timeout_secs,
        )
    }
}
