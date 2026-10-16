//! `impl` blocks for [`GrpcChannelConfig`] and [`GrpcChannelConfigBuilder`]: the `with_*` setters,
//! the defaults and `build`. `GrpcChannelConfig` is declared in `api/channel/vo/`, the builder in
//! `grpc_channel_config_builder.rs` beside this file.

use crate::std::GrpcChannelConfigBuilder;
use grpc_egress_transport::{CompressionMode, GrpcChannelConfig, DEFAULT_MAX_MESSAGE_BYTES};
use grpc_egress_transport::{KeepAliveConfig, MtlsConfig};

/// Constructors and fluent setters of [`GrpcChannelConfig`], which is plain data in the domain
/// crate. Bring the trait into scope to write `GrpcChannelConfig::new(endpoint).allow_plaintext()`.
pub trait GrpcChannelConfigFactory: Sized {
    /// Construct a TLS-required channel for `endpoint`.
    fn new(endpoint: impl Into<String>) -> Self;

    /// Explicitly relax the TLS requirement.
    fn allow_plaintext(self) -> Self;

    /// Attach an mTLS client identity.
    fn with_mtls(self, mtls: MtlsConfig) -> Self;

    /// Attach an HTTP/2 keep-alive policy.
    fn with_keep_alive(self, keep_alive: KeepAliveConfig) -> Self;

    /// Set the max-message-bytes ceiling.
    fn with_max_message_bytes(self, bytes: usize) -> Self;

    /// Set the compression mode.
    fn with_compression(self, mode: CompressionMode) -> Self;

    /// Set the client-side fallback request timeout.
    ///
    /// This backstop is applied on every request, independent of the per-call
    /// `GrpcRequest::deadline`. Use it to defend against unresponsive upstreams when your call
    /// deadlines vary or are not set.
    fn with_request_timeout(self, timeout: std::time::Duration) -> Self;
}

impl GrpcChannelConfigFactory for GrpcChannelConfig {
    fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            tls_required: true,
            mtls: None,
            keep_alive: None,
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            compression: CompressionMode::None,
            request_timeout_secs: None,
        }
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
        self.max_message_bytes = bytes;
        self
    }

    fn with_compression(mut self, mode: CompressionMode) -> Self {
        self.compression = mode;
        self
    }

    fn with_request_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.request_timeout_secs = Some(timeout.as_secs().max(1));
        self
    }
}

impl GrpcChannelConfigBuilder {
    /// Create a new builder with TLS required by default.
    pub fn new() -> Self {
        Self {
            tls_required: true,
            ..Default::default()
        }
    }
    /// Set the gRPC endpoint URI.
    pub fn with_endpoint(mut self, v: impl Into<String>) -> Self {
        self.endpoint = Some(v.into());
        self
    }
    /// Allow plaintext (non-TLS) connections.
    pub fn allow_plaintext(mut self) -> Self {
        self.tls_required = false;
        self
    }
    /// Configure mutual TLS.
    pub fn with_mtls(mut self, v: MtlsConfig) -> Self {
        self.mtls = Some(v);
        self
    }
    /// Configure HTTP/2 keep-alive settings.
    pub fn with_keep_alive(mut self, v: KeepAliveConfig) -> Self {
        self.keep_alive = Some(v);
        self
    }
    /// Set the maximum allowed message size in bytes.
    pub fn with_max_message_bytes(mut self, v: usize) -> Self {
        self.max_message_bytes = Some(v);
        self
    }
    /// Set the compression mode for outbound messages.
    pub fn with_compression(mut self, v: CompressionMode) -> Self {
        self.compression = Some(v);
        self
    }

    /// Build the [`GrpcChannelConfig`]. Returns `Err` when endpoint is unset.
    pub fn build(self) -> Result<GrpcChannelConfig, String> {
        let endpoint = self.endpoint.ok_or("endpoint required")?;
        let mut cfg = GrpcChannelConfig::new(endpoint);
        cfg.tls_required = self.tls_required;
        cfg.mtls = self.mtls;
        cfg.keep_alive = self.keep_alive;
        cfg.max_message_bytes = self.max_message_bytes.unwrap_or(DEFAULT_MAX_MESSAGE_BYTES);
        cfg.compression = self.compression.unwrap_or(CompressionMode::None);
        Ok(cfg)
    }
}
