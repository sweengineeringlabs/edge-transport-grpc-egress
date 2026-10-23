//! Outbound channel configuration — TLS-by-default, fail-closed.

use serde::{Deserialize, Serialize};

use crate::CompressionMode;
use crate::KeepAliveConfig;
use crate::MtlsConfig;

/// Default ceiling for inbound message bytes (4 MiB).
pub const DEFAULT_MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

/// Default client-side fallback timeout in seconds.
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

/// Configuration for a single outbound gRPC channel.
///
/// **TLS-by-default**.  `tls_required` is `true` in
/// `Default::default()`.  Plaintext requires explicit
/// `GrpcChannelConfig::allow_plaintext` (`GrpcChannelConfigFactory` of `grpc-egress-transport-std`) — fail-closed by design.
///
/// ```compile_fail,E0451
/// use grpc_egress_transport::GrpcChannelConfig;
/// let _ = GrpcChannelConfig { endpoint: String::from("https://example"), ..GrpcChannelConfig::default() };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "GrpcChannelConfigData")]
pub struct GrpcChannelConfig {
    /// Channel endpoint.
    endpoint: String,
    /// Require TLS on the wire.  When `true` (default) and the
    /// endpoint is plaintext, the transport refuses to dial.
    tls_required: bool,
    /// Optional mTLS client identity.
    mtls: Option<MtlsConfig>,
    /// Optional HTTP/2 keep-alive policy.
    keep_alive: Option<KeepAliveConfig>,
    /// Hard cap on a single response message in bytes.
    max_message_bytes: usize,
    /// Compression mode for outbound payloads.
    compression: CompressionMode,
    /// Client-side fallback timeout in seconds.
    ///
    /// Applied as a `tokio::time::timeout` backstop on each request, independent
    /// of the per-call `GrpcRequest::deadline` (which propagates as `grpc-timeout`
    /// and is enforced server-side).  When absent, defaults to
    /// [`DEFAULT_REQUEST_TIMEOUT_SECS`] (30 s).
    ///
    /// In TOML: `request_timeout_secs = 60`
    #[serde(default)]
    request_timeout_secs: Option<u64>,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct GrpcChannelConfigData {
    endpoint: String,
    tls_required: bool,
    mtls: Option<MtlsConfig>,
    keep_alive: Option<KeepAliveConfig>,
    max_message_bytes: usize,
    compression: CompressionMode,
    request_timeout_secs: Option<u64>,
}

impl Default for GrpcChannelConfigData {
    fn default() -> Self {
        let c = GrpcChannelConfig::default();
        Self {
            endpoint: c.endpoint,
            tls_required: c.tls_required,
            mtls: c.mtls,
            keep_alive: c.keep_alive,
            max_message_bytes: c.max_message_bytes,
            compression: c.compression,
            request_timeout_secs: c.request_timeout_secs,
        }
    }
}

impl TryFrom<GrpcChannelConfigData> for GrpcChannelConfig {
    type Error = String;
    fn try_from(c: GrpcChannelConfigData) -> Result<Self, String> {
        Self::try_new(
            c.endpoint,
            c.tls_required,
            c.mtls,
            c.keep_alive,
            c.max_message_bytes,
            c.compression,
            c.request_timeout_secs,
        )
    }
}

impl GrpcChannelConfig {
    /// Construct a channel configuration and reject unusable endpoint or size/time limits.
    pub fn try_new(
        endpoint: impl Into<String>,
        tls_required: bool,
        mtls: Option<MtlsConfig>,
        keep_alive: Option<KeepAliveConfig>,
        max_message_bytes: usize,
        compression: CompressionMode,
        request_timeout_secs: Option<u64>,
    ) -> Result<Self, String> {
        let endpoint = endpoint.into();
        if endpoint.trim().is_empty() {
            return Err("endpoint must not be empty".into());
        }
        if max_message_bytes == 0 {
            return Err("max_message_bytes must be >= 1".into());
        }
        if request_timeout_secs == Some(0) {
            return Err("request_timeout_secs must be >= 1 when set".into());
        }
        Ok(Self {
            endpoint,
            tls_required,
            mtls,
            keep_alive,
            max_message_bytes,
            compression,
            request_timeout_secs,
        })
    }
    /// Channel endpoint URI.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
    /// Whether the channel requires TLS.
    pub fn tls_required(&self) -> bool {
        self.tls_required
    }
    /// Optional mTLS identity.
    pub fn mtls(&self) -> Option<&MtlsConfig> {
        self.mtls.as_ref()
    }
    /// Optional HTTP/2 keep-alive policy.
    pub fn keep_alive(&self) -> Option<KeepAliveConfig> {
        self.keep_alive
    }
    /// Maximum message size in bytes.
    pub fn max_message_bytes(&self) -> usize {
        self.max_message_bytes
    }
    /// Outbound compression mode.
    pub fn compression(&self) -> CompressionMode {
        self.compression
    }
    /// Optional client-side timeout in seconds.
    pub fn request_timeout_secs(&self) -> Option<u64> {
        self.request_timeout_secs
    }
}

impl Default for GrpcChannelConfig {
    /// Defaults: empty endpoint, TLS required, no mTLS, no keep-alive, 4 MiB message cap, no
    /// compression.
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
