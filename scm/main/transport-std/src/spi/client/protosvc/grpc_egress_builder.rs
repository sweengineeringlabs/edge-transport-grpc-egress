//! `GrpcEgressBuilderImpl` -- the builder of the client that calls through `proto-svc`.
//!
//! Not part of the crate's public surface: a caller gets a builder from
//! [`crate::saf::TonicGrpcEgressBuilderSpiFactory`] and uses it through the
//! [`GrpcEgressBuilder`] trait.

use crate::std::{GrpcEgressInterceptorChain, TraceContextGrpcEgressInterceptor};
use std::sync::Arc;
use std::time::Duration;

use crate::spi::loadbalancer::protosvc::BalancedGrpcEgress;
use proto_pattern::{
    BuildTransportRequest, Compression, RawTransport, RawTransportFactory, TlsSettings,
};
use proto_tonic_spi::TonicTransportSpiFactory;

use grpc_egress_transport::{
    CompressionMode, GrpcChannelConfig, GrpcChannelConfigError, GrpcEgress, GrpcEgressBuilder,
    GrpcEgressInterceptor, KeepAliveConfig, LoadbalancerSettings, MtlsConfig, TraceContextSource,
    DEFAULT_MAX_MESSAGE_BYTES, DEFAULT_REQUEST_TIMEOUT_SECS,
};

use super::grpc_egress_client::GrpcEgressClient;
use super::proto_svc_grpc_egress::ProtoSvcGrpcEgress;
use super::tls_file_reader::TlsFileReader;

/// Holds the options of one client and builds it.
///
/// Defaults: TLS required, a 30 s request timeout, a 4 MiB message cap, no compression, no
/// interceptor.
pub(crate) struct GrpcEgressBuilderImpl {
    endpoint: String,
    /// Whether `with_endpoint` or a channel config named an endpoint, as opposed to the one the
    /// factory was created with.
    endpoint_explicit: bool,
    plaintext_allowed: bool,
    request_timeout: Duration,
    max_message_bytes: usize,
    compression: CompressionMode,
    interceptors: GrpcEgressInterceptorChain,
    mtls: Option<MtlsConfig>,
    keep_alive: Option<KeepAliveConfig>,
    root_ca: Option<String>,
    loadbalancer: Option<LoadbalancerSettings>,
}

impl GrpcEgressBuilderImpl {
    /// A builder for `endpoint` with every option at its default.
    pub(crate) fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            endpoint_explicit: false,
            plaintext_allowed: false,
            request_timeout: Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS),
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            compression: CompressionMode::None,
            interceptors: GrpcEgressInterceptorChain::new(),
            mtls: None,
            keep_alive: None,
            root_ca: None,
            loadbalancer: None,
        }
    }
}

impl GrpcEgressBuilder for GrpcEgressBuilderImpl {
    fn with_endpoint(self, endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            endpoint_explicit: true,
            ..self
        }
    }

    fn with_plaintext_allowed(self) -> Self {
        Self {
            plaintext_allowed: true,
            ..self
        }
    }

    fn with_request_timeout(self, timeout: Duration) -> Self {
        Self {
            request_timeout: timeout,
            ..self
        }
    }

    fn with_max_message_bytes(self, bytes: usize) -> Self {
        Self {
            max_message_bytes: bytes,
            ..self
        }
    }

    fn with_compression(self, mode: CompressionMode) -> Self {
        Self {
            compression: mode,
            ..self
        }
    }

    fn with_mtls(self, mtls: MtlsConfig) -> Self {
        Self {
            mtls: Some(mtls),
            ..self
        }
    }

    fn with_root_ca(self, ca_pem_path: impl Into<String>) -> Self {
        Self {
            root_ca: Some(ca_pem_path.into()),
            ..self
        }
    }

    fn with_keep_alive(self, keep_alive: KeepAliveConfig) -> Self {
        Self {
            keep_alive: Some(keep_alive),
            ..self
        }
    }

    fn with_loadbalancer(self, settings: LoadbalancerSettings) -> Self {
        Self {
            loadbalancer: Some(settings),
            ..self
        }
    }

    fn with_interceptor(self, interceptor: Arc<dyn GrpcEgressInterceptor>) -> Self {
        Self {
            interceptors: self.interceptors.push(interceptor),
            ..self
        }
    }

    fn with_trace_context(self, source: TraceContextSource) -> Self {
        let interceptor = match source {
            TraceContextSource::PassThrough => TraceContextGrpcEgressInterceptor::pass_through(),
            TraceContextSource::Static {
                traceparent,
                tracestate,
            } => TraceContextGrpcEgressInterceptor::with_static(traceparent, tracestate),
        };
        self.with_interceptor(Arc::new(interceptor))
    }

    fn with_channel_config(self, config: &GrpcChannelConfig) -> Self {
        let request_timeout = config
            .request_timeout_secs
            .map_or(self.request_timeout, Duration::from_secs);
        Self {
            endpoint: config.endpoint.clone(),
            endpoint_explicit: self.endpoint_explicit || !config.endpoint.is_empty(),
            plaintext_allowed: !config.tls_required,
            request_timeout,
            max_message_bytes: config.max_message_bytes,
            compression: config.compression,
            mtls: config.mtls.clone().or(self.mtls),
            keep_alive: config.keep_alive.or(self.keep_alive),
            ..self
        }
    }

    fn build(self) -> Result<impl GrpcEgress + use<>, GrpcChannelConfigError> {
        self.check_values()?;
        let targets = self.list_targets()?;
        if !self.plaintext_allowed {
            if let Some(url) = targets.iter().find(|url| Self::is_plaintext_endpoint(url)) {
                return Err(GrpcChannelConfigError::PlaintextRejected(url.clone()));
            }
        }
        let root_ca = self
            .root_ca
            .as_deref()
            .or_else(|| self.mtls.as_ref().and_then(|m| m.ca_pem_path.as_deref()));
        if let Some(option) = self.name_tls_option(root_ca) {
            if targets.iter().all(|url| Self::is_plaintext_endpoint(url)) {
                return Err(GrpcChannelConfigError::TlsOptionOnPlaintextEndpoint {
                    option,
                    endpoint: targets.first().cloned().unwrap_or_default(),
                });
            }
        }
        let tls = TlsFileReader::read(self.mtls.as_ref(), root_ca)?;
        match &self.loadbalancer {
            Some(settings) => {
                let mut clients = Vec::with_capacity(targets.len());
                for url in &targets {
                    clients.push((url.clone(), self.create_client(url, &tls)?));
                }
                Ok(GrpcEgressClient::Balanced(BalancedGrpcEgress::create(
                    clients, settings,
                )?))
            }
            None => {
                let first = targets.first().cloned().unwrap_or_default();
                Ok(GrpcEgressClient::Direct(self.create_client(&first, &tls)?))
            }
        }
    }
}

impl GrpcEgressBuilderImpl {
    /// Refuse a zero cap or timeout, and options that exclude each other.
    fn check_values(&self) -> Result<(), GrpcChannelConfigError> {
        if self.max_message_bytes == 0 {
            return Err(GrpcChannelConfigError::InvalidOptionValue {
                option: "max_message_bytes",
                reason: "zero would refuse every message; use a positive number of bytes".into(),
            });
        }
        if self.request_timeout.is_zero() {
            return Err(GrpcChannelConfigError::InvalidOptionValue {
                option: "request_timeout",
                reason: "zero would end every call at once; use a positive duration".into(),
            });
        }
        if self.mtls.is_some() && self.plaintext_allowed {
            return Err(GrpcChannelConfigError::ConflictingOptions {
                first: "mtls",
                second: "plaintext_allowed",
                reason: "mutual TLS needs TLS, and plaintext_allowed lets the client fall back to plaintext",
            });
        }
        if self.loadbalancer.is_some() && self.endpoint_explicit {
            return Err(GrpcChannelConfigError::ConflictingOptions {
                first: "endpoint",
                second: "loadbalancer",
                reason: "the backends of the load balancer carry the URLs; drop the endpoint",
            });
        }
        Ok(())
    }

    /// The URLs calls go to: the endpoint, or each backend of the load balancer.
    fn list_targets(&self) -> Result<Vec<String>, GrpcChannelConfigError> {
        let Some(settings) = &self.loadbalancer else {
            return Ok(vec![self.endpoint.clone()]);
        };
        if settings.backends.is_empty() {
            return Err(GrpcChannelConfigError::InvalidOptionValue {
                option: "loadbalancer",
                reason: "no backends; give at least one".into(),
            });
        }
        let mut urls = Vec::with_capacity(settings.backends.len());
        for backend in &settings.backends {
            if backend.url.parse::<http::Uri>().is_err() {
                return Err(GrpcChannelConfigError::InvalidOptionValue {
                    option: "loadbalancer",
                    reason: format!("backend '{}' is not a valid URL", backend.url),
                });
            }
            urls.push(backend.url.clone());
        }
        Ok(urls)
    }

    /// The TLS option that is set, for an error message: mutual TLS first, then the root CA.
    fn name_tls_option(&self, root_ca: Option<&str>) -> Option<&'static str> {
        match (&self.mtls, root_ca) {
            (Some(_), _) => Some("mtls"),
            (None, Some(_)) => Some("root_ca"),
            (None, None) => None,
        }
    }
}

impl GrpcEgressBuilderImpl {
    /// Returns `true` when `endpoint` starts with `http://` (case-insensitive).
    fn is_plaintext_endpoint(endpoint: &str) -> bool {
        endpoint.len() >= 7 && endpoint[..7].eq_ignore_ascii_case("http://")
    }

    /// The client for `url`: its transport is built by the `proto-svc` factory from the options of this
    /// builder, and TLS is set up for an `https` endpoint only.
    fn create_client(
        &self,
        url: &str,
        tls: &TlsSettings,
    ) -> Result<ProtoSvcGrpcEgress<impl RawTransport + 'static>, GrpcChannelConfigError> {
        let mut factory = TonicTransportSpiFactory::new(url)
            .with_max_decoding_message_size(self.max_message_bytes);
        if !Self::is_plaintext_endpoint(url) {
            factory = factory.with_tls(tls.clone());
        }
        factory = match self.compression {
            CompressionMode::None => factory,
            CompressionMode::Gzip => factory
                .with_send_compression(Compression::Gzip)
                .with_accept_compression(Compression::Gzip),
            CompressionMode::Zstd => factory
                .with_send_compression(Compression::Zstd)
                .with_accept_compression(Compression::Zstd),
        };
        if let Some(keep_alive) = self.keep_alive {
            factory = factory
                .with_http2_keep_alive_interval(keep_alive.interval)
                .with_keep_alive_timeout(keep_alive.timeout)
                .with_keep_alive_while_idle(keep_alive.permit_without_calls);
        }
        let transport =
            RawTransportFactory::build(&factory, BuildTransportRequest).map_err(|e| {
                GrpcChannelConfigError::InvalidOptionValue {
                    option: "endpoint",
                    reason: e.to_string(),
                }
            })?;
        Ok(ProtoSvcGrpcEgress::create(
            transport,
            url,
            self.request_timeout,
            self.interceptors.clone(),
        ))
    }
}
