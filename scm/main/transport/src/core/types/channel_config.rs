//! `impl` blocks for [`GrpcChannelConfig`] and [`GrpcChannelConfigBuilder`].
//! The type *declarations* live in `api/`.
//!
//! Merged into one file (rather than one file per type) so the
//! `grpc_channel_config`/`grpc_channel_config_builder`/`grpc_egress_interceptor_chain`/
//! `grpc_request`/`grpc_request_builder` filenames don't share a prefix as
//! flat siblings of `core/` — see `shared_prefix_grouping`. Subdirectory
//! nesting was tried first and rejected: it fails
//! `core_api_module_correspondence`, since api/'s kind-based layout
//! (types/, traits/, errors/) has no theme subdirectory to correspond to a
//! new core/grpc/ directory.

use crate::api::{CompressionMode, GrpcChannelConfig, DEFAULT_MAX_MESSAGE_BYTES};
use crate::api::{GrpcChannelConfigBuilder, KeepAliveConfig, MtlsConfig};

impl GrpcChannelConfig {
    /// Construct a TLS-required channel for `endpoint`.
    pub fn new(endpoint: impl Into<String>) -> Self {
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

    /// Explicitly relax the TLS requirement.
    pub fn allow_plaintext(mut self) -> Self {
        self.tls_required = false;
        self
    }

    /// Attach an mTLS client identity.
    pub fn with_mtls(mut self, mtls: MtlsConfig) -> Self {
        self.mtls = Some(mtls);
        self
    }

    /// Attach an HTTP/2 keep-alive policy.
    pub fn with_keep_alive(mut self, keep_alive: KeepAliveConfig) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }

    /// Set the max-message-bytes ceiling.
    pub fn with_max_message_bytes(mut self, bytes: usize) -> Self {
        self.max_message_bytes = bytes;
        self
    }

    /// Set the compression mode.
    pub fn with_compression(mut self, mode: CompressionMode) -> Self {
        self.compression = mode;
        self
    }

    /// Set the client-side fallback request timeout.
    ///
    /// This backstop is applied on every request via `tokio::time::timeout`,
    /// independent of the per-call `GrpcRequest::deadline` header. Use it to
    /// defend against unresponsive upstreams when your call deadlines vary or
    /// are not set.
    pub fn with_request_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.request_timeout_secs = Some(timeout.as_secs().max(1));
        self
    }
}

impl Default for GrpcChannelConfig {
    /// Defaults: empty endpoint, TLS required, no mTLS, no keep-alive,
    /// 4 MiB message cap, no compression.
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            tls_required: true,
            mtls: None,
            keep_alive: None,
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            compression: CompressionMode::None,
            request_timeout_secs: None,
        }
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
    pub fn endpoint(mut self, v: impl Into<String>) -> Self {
        self.endpoint = Some(v.into());
        self
    }
    /// Allow plaintext (non-TLS) connections.
    pub fn allow_plaintext(mut self) -> Self {
        self.tls_required = false;
        self
    }
    /// Configure mutual TLS.
    pub fn mtls(mut self, v: MtlsConfig) -> Self {
        self.mtls = Some(v);
        self
    }
    /// Configure HTTP/2 keep-alive settings.
    pub fn keep_alive(mut self, v: KeepAliveConfig) -> Self {
        self.keep_alive = Some(v);
        self
    }
    /// Set the maximum allowed message size in bytes.
    pub fn max_message_bytes(mut self, v: usize) -> Self {
        self.max_message_bytes = Some(v);
        self
    }
    /// Set the compression mode for outbound messages.
    pub fn compression(mut self, v: CompressionMode) -> Self {
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
