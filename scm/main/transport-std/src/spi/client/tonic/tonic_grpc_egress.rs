//! `TonicGrpcEgress` struct declaration, type alias, and constructor.

use crate::std::GrpcEgressInterceptorChain;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::Full;
use hyper_util::client::legacy::connect::HttpConnector;

use super::tonic_tls_connector::TonicTlsConnector;
use grpc_egress_transport::{
    CompressionMode, GrpcChannelConfigError, KeepAliveConfig, MtlsConfig, DEFAULT_MAX_MESSAGE_BYTES,
};

/// Hyper HTTP/2 client type alias used by [`TonicGrpcEgress`].
pub(crate) type HyperClient =
    hyper_util::client::legacy::Client<hyper_rustls::HttpsConnector<HttpConnector>, Full<Bytes>>;

/// Concrete `GrpcEgress` implementation using hyper HTTP/2.
///
/// Not part of the crate's public surface — external consumers obtain a
/// `GrpcEgress` via `TransportSvc`/`GrpcEgressFactory`, never by naming this
/// type directly (see SEA rule `pub_types_in_api_only`).
#[derive(Clone)]
pub(crate) struct TonicGrpcEgress {
    pub(crate) base_uri: String,
    pub(crate) client: HyperClient,
    connector: hyper_rustls::HttpsConnector<HttpConnector>,
    keep_alive: Option<KeepAliveConfig>,
    pub(crate) timeout: Duration,
    pub(crate) interceptors: GrpcEgressInterceptorChain,
    pub(crate) max_message_bytes: usize,
    pub(crate) compression: CompressionMode,
    pub(crate) content_type: &'static str,
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
        Self {
            base_uri: base_uri.into(),
            client: Self::create_client(connector.clone(), None),
            connector,
            keep_alive: None,
            timeout: Duration::from_secs(30),
            interceptors: GrpcEgressInterceptorChain::new(),
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            compression: CompressionMode::None,
            content_type: "application/grpc",
        }
    }

    /// Apply the TLS options: the identity to present (mutual TLS) and the root CA to trust. With neither
    /// the client is returned as it is.
    ///
    /// # Errors
    ///
    /// `UnusableTlsFile` when a certificate, key or CA file cannot be used.
    pub(crate) fn with_tls(
        mut self,
        identity: Option<&MtlsConfig>,
        root_ca: Option<&str>,
    ) -> Result<Self, GrpcChannelConfigError> {
        if identity.is_none() && root_ca.is_none() {
            return Ok(self);
        }
        self.connector = TonicTlsConnector::create(identity, root_ca)?;
        self.client = Self::create_client(self.connector.clone(), self.keep_alive.as_ref());
        Ok(self)
    }

    /// Declare the messages prost-encoded: calls carry the content type `application/grpc+proto`.
    pub(crate) fn with_prost_codec(mut self) -> Self {
        self.content_type = "application/grpc+proto";
        self
    }

    /// The same client, sending to `base_uri`: one engine for every backend of a balanced client.
    pub(crate) fn retarget(&self, base_uri: &str) -> Self {
        Self {
            base_uri: base_uri.to_string(),
            ..self.clone()
        }
    }

    /// Send HTTP/2 PING frames as `keep_alive` says, and close the connection when one is not
    /// answered in time: a peer that has silently gone away is noticed near the timeout and not
    /// at the next call's deadline.
    pub(crate) fn with_keep_alive(mut self, keep_alive: KeepAliveConfig) -> Self {
        self.client = Self::create_client(self.connector.clone(), Some(&keep_alive));
        self.keep_alive = Some(keep_alive);
        self
    }

    fn create_client(
        connector: hyper_rustls::HttpsConnector<HttpConnector>,
        keep_alive: Option<&KeepAliveConfig>,
    ) -> HyperClient {
        let mut builder =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new());
        builder
            .timer(hyper_util::rt::TokioTimer::new())
            .http2_only(true);
        if let Some(keep_alive) = keep_alive {
            builder
                .http2_keep_alive_interval(keep_alive.interval)
                .http2_keep_alive_timeout(keep_alive.timeout)
                .http2_keep_alive_while_idle(keep_alive.permit_without_calls);
        }
        builder.build(connector)
    }
}
