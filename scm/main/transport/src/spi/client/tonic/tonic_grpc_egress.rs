//! `TonicGrpcEgress` struct declaration, type alias, and constructor.

use std::time::Duration;

use bytes::Bytes;
use http_body_util::Full;
use hyper_util::client::legacy::connect::HttpConnector;

use super::tonic_tls_connector::TonicTlsConnector;
use crate::api::{
    CompressionMode, GrpcChannelConfigError, GrpcEgressInterceptorChain, MtlsConfig,
    DEFAULT_MAX_MESSAGE_BYTES,
};

/// Hyper HTTP/2 client type alias used by [`TonicGrpcEgress`].
pub(crate) type HyperClient =
    hyper_util::client::legacy::Client<hyper_rustls::HttpsConnector<HttpConnector>, Full<Bytes>>;

/// Concrete `GrpcEgress` implementation using hyper HTTP/2.
///
/// Not part of the crate's public surface — external consumers obtain a
/// `GrpcEgress` via `TransportSvc`/`GrpcEgressFactory`, never by naming this
/// type directly (see SEA rule `pub_types_in_api_only`).
pub(crate) struct TonicGrpcEgress {
    pub(crate) base_uri: String,
    pub(crate) client: HyperClient,
    pub(crate) timeout: Duration,
    pub(crate) interceptors: GrpcEgressInterceptorChain,
    pub(crate) max_message_bytes: usize,
    pub(crate) compression: CompressionMode,
}

impl TonicGrpcEgress {
    /// Create a client with a 30-second fallback timeout.
    pub(crate) fn new(base_uri: impl Into<String>) -> Self {
        // rustls 0.23 requires a process-wide CryptoProvider before the first
        // ClientConfig construction. hyper-rustls does not install one in any
        // production code path. We own the transport construction so we own
        // this precondition. Idempotent — Err means a provider is already set.
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http2()
            .build();
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .http2_only(true)
                .build(connector);
        Self {
            base_uri: base_uri.into(),
            client,
            timeout: Duration::from_secs(30),
            interceptors: GrpcEgressInterceptorChain::new(),
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            compression: CompressionMode::None,
        }
    }

    /// Apply the TLS options: the identity to present (mutual TLS) and the CA to pin. With neither
    /// the client is returned as it is.
    ///
    /// # Errors
    ///
    /// - `TlsOptionOnPlaintextEndpoint` when an option is set but the endpoint is plaintext;
    /// - `UnusableTlsFile` when a certificate, key or CA file cannot be used.
    pub(crate) fn with_tls(
        mut self,
        identity: Option<&MtlsConfig>,
        pinned_ca: Option<&str>,
    ) -> Result<Self, GrpcChannelConfigError> {
        let option = match (identity, pinned_ca) {
            (None, None) => return Ok(self),
            (Some(_), _) => "mtls",
            (None, Some(_)) => "pinned_ca",
        };
        if super::tonic_grpc_egress_protocol::TonicGrpcEgressProtocol::is_plaintext_endpoint(
            &self.base_uri,
        ) {
            return Err(GrpcChannelConfigError::TlsOptionOnPlaintextEndpoint {
                option,
                endpoint: self.base_uri,
            });
        }
        let connector = TonicTlsConnector::create(identity, pinned_ca)?;
        self.client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .http2_only(true)
                .build(connector);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: new
    #[test]
    fn test_new_constructs_client_with_default_timeout() {
        let client = TonicGrpcEgress::new("http://127.0.0.1:50051");
        assert_eq!(client.base_uri, "http://127.0.0.1:50051");
        assert_eq!(client.timeout, Duration::from_secs(30));
    }
}
