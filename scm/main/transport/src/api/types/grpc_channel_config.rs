//! Outbound channel configuration — TLS-by-default, fail-closed.

use serde::{Deserialize, Serialize};

use crate::api::types::compression_mode::CompressionMode;
use crate::api::types::keep_alive_config::KeepAliveConfig;
use crate::api::types::mtls_config::MtlsConfig;

/// Default ceiling for inbound message bytes (4 MiB).
pub const DEFAULT_MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

/// Default client-side fallback timeout in seconds.
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

/// Configuration for a single outbound gRPC channel.
///
/// **TLS-by-default**.  `tls_required` is `true` in
/// `Default::default()`.  Plaintext requires explicit
/// [`GrpcChannelConfig::allow_plaintext`] — fail-closed by design.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcChannelConfig {
    /// Channel endpoint.
    pub endpoint: String,
    /// Require TLS on the wire.  When `true` (default) and the
    /// endpoint is plaintext, the transport refuses to dial.
    pub tls_required: bool,
    /// Optional mTLS client identity.
    pub mtls: Option<MtlsConfig>,
    /// Optional HTTP/2 keep-alive policy.
    pub keep_alive: Option<KeepAliveConfig>,
    /// Hard cap on a single response message in bytes.
    pub max_message_bytes: usize,
    /// Compression mode for outbound payloads.
    pub compression: CompressionMode,
    /// Client-side fallback timeout in seconds.
    ///
    /// Applied as a `tokio::time::timeout` backstop on each request, independent
    /// of the per-call `GrpcRequest::deadline` (which propagates as `grpc-timeout`
    /// and is enforced server-side).  When absent, defaults to
    /// [`DEFAULT_REQUEST_TIMEOUT_SECS`] (30 s).
    ///
    /// In TOML: `request_timeout_secs = 60`
    #[serde(default)]
    pub request_timeout_secs: Option<u64>,
}
