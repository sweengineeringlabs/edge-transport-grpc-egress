//! `GrpcChannelConfigBuilder` -- the fluent construction of a [`GrpcChannelConfig`].

use std::time::Duration;

use crate::{CompressionMode, GrpcChannelConfig, KeepAliveConfig, MtlsConfig};

/// Collects the settings of a channel and builds the [`GrpcChannelConfig`], which is plain data in this crate.
///
/// A builder starts with TLS required, no mutual TLS, no keep-alive, the default message cap, no compression and no
/// request timeout. A method takes `self` and returns `Self`, so the settings chain; the last call for one setting
/// wins.
pub trait GrpcChannelConfigBuilder: Sized {
    /// Set the gRPC endpoint URI.
    fn with_endpoint(self, endpoint: impl Into<String>) -> Self;

    /// Explicitly relax the TLS requirement: a plaintext (`http://`) endpoint is then accepted.
    fn allow_plaintext(self) -> Self;

    /// Attach an mTLS client identity.
    fn with_mtls(self, mtls: MtlsConfig) -> Self;

    /// Attach an HTTP/2 keep-alive policy.
    fn with_keep_alive(self, keep_alive: KeepAliveConfig) -> Self;

    /// Set the ceiling of a single response message, in bytes.
    fn with_max_message_bytes(self, bytes: usize) -> Self;

    /// Set the compression mode of outbound messages.
    fn with_compression(self, mode: CompressionMode) -> Self;

    /// Set the client-side fallback request timeout, in whole seconds (at least one).
    ///
    /// This backstop is applied on every request, independent of the per-call deadline. Use it to defend against
    /// unresponsive upstreams when your call deadlines vary or are not set.
    fn with_request_timeout(self, timeout: Duration) -> Self;

    /// Build the [`GrpcChannelConfig`] from what was set.
    ///
    /// # Errors
    ///
    /// A text saying that the endpoint is required, when none was set.
    fn build(self) -> Result<GrpcChannelConfig, String>;
}
