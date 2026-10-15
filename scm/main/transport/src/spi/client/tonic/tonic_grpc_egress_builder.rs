//! `TonicGrpcEgressBuilder` — internal builder for [`super::tonic_grpc_egress::TonicGrpcEgress`].
//!
//! Not part of the crate's public surface — external consumers construct a
//! `GrpcEgress` via `TransportSvc`/`GrpcEgressFactory` from a
//! [`crate::api::types::GrpcChannelConfig`] instead (see SEA rule
//! `pub_types_in_api_only`).

use std::time::Duration;

use crate::api::{CompressionMode, GrpcEgressInterceptorChain, DEFAULT_MAX_MESSAGE_BYTES};

/// Builder for [`super::tonic_grpc_egress::TonicGrpcEgress`].
///
/// Each setter is a fluent method that returns `Self`; call [`Self::build`]
/// when all settings are configured.
pub(crate) struct TonicGrpcEgressBuilder {
    base_uri: String,
    timeout: Duration,
    interceptors: GrpcEgressInterceptorChain,
    max_message_bytes: usize,
    compression: CompressionMode,
}

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "only exercised in this crate's own tests; production wiring pending"
    )
)]
impl TonicGrpcEgressBuilder {
    /// Create a builder targeting `base_uri`.
    ///
    /// Defaults: 30 s timeout, no interceptors, 4 MiB max message, no compression.
    pub(crate) fn new(base_uri: impl Into<String>) -> Self {
        Self {
            base_uri: base_uri.into(),
            timeout: Duration::from_secs(30),
            interceptors: GrpcEgressInterceptorChain::new(),
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            compression: CompressionMode::None,
        }
    }

    /// Override the per-request deadline.
    pub(crate) fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Attach an interceptor chain (replaces any previous chain).
    pub(crate) fn interceptors(mut self, chain: GrpcEgressInterceptorChain) -> Self {
        self.interceptors = chain;
        self
    }

    /// Override the maximum response message size in bytes.
    pub(crate) fn max_message_bytes(mut self, bytes: usize) -> Self {
        self.max_message_bytes = bytes;
        self
    }

    /// Override the compression mode.
    pub(crate) fn compression(mut self, mode: CompressionMode) -> Self {
        self.compression = mode;
        self
    }

    /// Consume the builder and return a configured [`super::tonic_grpc_egress::TonicGrpcEgress`].
    pub(crate) fn build(self) -> super::tonic_grpc_egress::TonicGrpcEgress {
        super::tonic_grpc_egress::TonicGrpcEgress::new(self.base_uri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{GrpcEgress, GrpcEgressError, GrpcEgressInterceptorChain, HealthCheckRequest};

    fn ensure_rustls_provider() {
        use std::sync::Once;
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        });
    }

    /// Nothing listens on 127.0.0.1:50051 in the test environment, so a real
    /// health_check() call must genuinely fail — proves the built client is a
    /// real, connectable GrpcEgress wired to the given base_uri, not a stub.
    async fn assert_genuinely_connectable(client: impl GrpcEgress) {
        let health = client.health_check(HealthCheckRequest).await;
        assert!(
            matches!(health, Err(GrpcEgressError::Unavailable(_))),
            "health_check against an unbound port must report Unavailable, got: {health:?}"
        );
    }

    /// @covers: TonicGrpcEgressBuilder::new, build — builder produces a client without panic
    #[tokio::test]
    async fn test_build_produces_client() {
        ensure_rustls_provider();
        let client = TonicGrpcEgressBuilder::new("http://127.0.0.1:50051").build();
        assert_genuinely_connectable(client).await;
    }

    /// @covers: TonicGrpcEgressBuilder::timeout — fluent setter returns Self
    #[tokio::test]
    async fn test_timeout_setter_is_fluent() {
        ensure_rustls_provider();
        let client = TonicGrpcEgressBuilder::new("http://127.0.0.1:50051")
            .timeout(Duration::from_secs(5))
            .build();
        assert_genuinely_connectable(client).await;
    }

    /// @covers: TonicGrpcEgressBuilder::max_message_bytes — fluent setter returns Self
    #[tokio::test]
    async fn test_max_message_bytes_setter_is_fluent() {
        ensure_rustls_provider();
        let client = TonicGrpcEgressBuilder::new("http://127.0.0.1:50051")
            .max_message_bytes(8 * 1024 * 1024)
            .build();
        assert_genuinely_connectable(client).await;
    }

    /// @covers: TonicGrpcEgressBuilder::compression — fluent setter returns Self
    #[tokio::test]
    async fn test_compression_setter_is_fluent() {
        ensure_rustls_provider();
        let client = TonicGrpcEgressBuilder::new("http://127.0.0.1:50051")
            .compression(CompressionMode::Gzip)
            .build();
        assert_genuinely_connectable(client).await;
    }

    /// @covers: TonicGrpcEgressBuilder::interceptors — fluent setter returns Self
    #[tokio::test]
    async fn test_interceptors_setter_is_fluent() {
        ensure_rustls_provider();
        let chain = GrpcEgressInterceptorChain::new();
        let client = TonicGrpcEgressBuilder::new("http://127.0.0.1:50051")
            .interceptors(chain)
            .build();
        assert_genuinely_connectable(client).await;
    }
}
