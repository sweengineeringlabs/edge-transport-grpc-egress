//! `GrpcEgressBuilderImpl` -- the builder of the client that calls through `proto-svc`.
//!
//! Not part of the crate's public surface: a caller gets a builder from
//! [`crate::saf::TonicGrpcEgressBuilderSpiFactory`] and uses it through the
//! [`GrpcEgressBuilder`] trait.

use std::sync::Arc;
use std::time::Duration;

use proto_pattern::{
    BuildTransportRequest, Compression, DualTransportFactory, RawTransport, TlsSettings, Transport,
};
use proto_tonic_spi::TonicTransportSpiFactory;

use grpc_egress_transport::{
    CompressionMode, GrpcChannelConfig, GrpcChannelConfigError, GrpcEgressBuilder,
    GrpcEgressInterceptor, KeepAliveConfig, LoadbalancerSettings, MtlsConfig, TraceContextSource,
    DEFAULT_REQUEST_TIMEOUT_SECS,
};

use grpc_egress_transport::{
    GrpcEgressInterceptorChain, ResolveTlsSettingsRequest, TlsSettingsResolver,
};

use super::EndpointTransport;
use crate::std::{
    InterceptedTransport, StdGrpcEgressInterceptorChain, StdTlsSettingsResolver,
    TraceContextGrpcEgressInterceptor,
};

/// Where the TLS settings of the client come from: given as bytes, or named as PEM files to read at `build`.
enum TlsSource {
    Settings(TlsSettings),
    Files(MtlsConfig),
}

/// Holds the options of one client and builds it.
///
/// Defaults: TLS required, a 30 s request timeout, the engine's 4 MiB response limit, no request limit, no
/// compression, no interceptor.
pub(crate) struct GrpcEgressBuilderImpl {
    endpoint: String,
    /// Whether `with_endpoint` or a channel config named an endpoint, as opposed to the one the
    /// factory was created with.
    endpoint_explicit: bool,
    plaintext_allowed: bool,
    request_timeout: Duration,
    max_decoding_message_size: Option<usize>,
    max_encoding_message_size: Option<usize>,
    send_compression: Option<Compression>,
    accept_compression: Vec<Compression>,
    interceptors: StdGrpcEgressInterceptorChain,
    tls: Option<TlsSource>,
    keep_alive: Option<KeepAliveConfig>,
    loadbalancer: Option<LoadbalancerSettings>,
}

impl From<String> for GrpcEgressBuilderImpl {
    /// A builder for the endpoint with every option at its default.
    fn from(endpoint: String) -> Self {
        Self {
            endpoint,
            endpoint_explicit: false,
            plaintext_allowed: false,
            request_timeout: Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS),
            max_decoding_message_size: None,
            max_encoding_message_size: None,
            send_compression: None,
            accept_compression: Vec::new(),
            interceptors: StdGrpcEgressInterceptorChain::default(),
            tls: None,
            keep_alive: None,
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

    fn with_max_decoding_message_size(self, bytes: usize) -> Self {
        Self {
            max_decoding_message_size: Some(bytes),
            ..self
        }
    }

    fn with_max_encoding_message_size(self, bytes: usize) -> Self {
        Self {
            max_encoding_message_size: Some(bytes),
            ..self
        }
    }

    fn with_send_compression(self, compression: Compression) -> Self {
        Self {
            send_compression: Some(compression),
            ..self
        }
    }

    fn with_accept_compression(mut self, compression: Compression) -> Self {
        // The engine enables each encoding once, however often it is named: a repeat changes nothing on the wire.
        self.accept_compression.push(compression);
        self
    }

    fn with_tls(self, tls: TlsSettings) -> Self {
        Self {
            tls: Some(TlsSource::Settings(tls)),
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
        self.with_interceptor(Arc::new(TraceContextGrpcEgressInterceptor::from(source)))
    }

    fn with_channel_config(self, config: &GrpcChannelConfig) -> Self {
        let request_timeout = config
            .request_timeout_secs
            .map_or(self.request_timeout, Duration::from_secs);
        let (send_compression, accept_compression) =
            match Self::read_compression(config.compression) {
                Some(compression) => (Some(compression), vec![compression]),
                None => (self.send_compression, self.accept_compression.clone()),
            };
        Self {
            endpoint: config.endpoint.clone(),
            endpoint_explicit: self.endpoint_explicit || !config.endpoint.is_empty(),
            plaintext_allowed: !config.tls_required,
            request_timeout,
            max_decoding_message_size: Some(config.max_message_bytes),
            max_encoding_message_size: Some(config.max_message_bytes),
            send_compression,
            accept_compression,
            tls: config.mtls.clone().map(TlsSource::Files).or(self.tls),
            keep_alive: config.keep_alive.or(self.keep_alive),
            ..self
        }
    }

    fn build(self) -> Result<impl Transport + RawTransport + use<>, GrpcChannelConfigError> {
        self.check_values()?;
        let targets = self.list_targets()?;
        if !self.plaintext_allowed {
            if let Some(url) = targets.iter().find(|url| Self::is_plaintext_endpoint(url)) {
                return Err(GrpcChannelConfigError::PlaintextRejected(url.clone()));
            }
        }
        if let Some(option) = self.name_tls_option() {
            if targets.iter().all(|url| Self::is_plaintext_endpoint(url)) {
                return Err(GrpcChannelConfigError::TlsOptionOnPlaintextEndpoint {
                    option,
                    endpoint: targets.first().cloned().unwrap_or_default(),
                });
            }
        }
        let tls = self.resolve_tls()?;
        let mut clients = Vec::with_capacity(targets.len());
        for url in &targets {
            clients.push((url.clone(), self.create_client(url, &tls)?));
        }
        let endpoint = EndpointTransport::try_from((clients, self.loadbalancer.as_ref()))?;
        Ok(InterceptedTransport::from((endpoint, self.interceptors)))
    }
}

impl GrpcEgressBuilderImpl {
    /// The compression a file configuration names; `None` is no compression.
    fn read_compression(mode: CompressionMode) -> Option<Compression> {
        match mode {
            CompressionMode::None => None,
            CompressionMode::Gzip => Some(Compression::Gzip),
            CompressionMode::Zstd => Some(Compression::Zstd),
        }
    }

    /// Refuse a zero limit or timeout, and options that exclude each other.
    fn check_values(&self) -> Result<(), GrpcChannelConfigError> {
        for (option, value) in [
            ("max_decoding_message_size", self.max_decoding_message_size),
            ("max_encoding_message_size", self.max_encoding_message_size),
        ] {
            if value == Some(0) {
                return Err(GrpcChannelConfigError::InvalidOptionValue {
                    option,
                    reason: "zero would refuse every message; use a positive number of bytes"
                        .into(),
                });
            }
        }
        if self.request_timeout.is_zero() {
            return Err(GrpcChannelConfigError::InvalidOptionValue {
                option: "request_timeout",
                reason: "zero would end every call at once; use a positive duration".into(),
            });
        }
        if self.presents_identity() && self.plaintext_allowed {
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

    /// `true` when the client is to present a certificate (mutual TLS).
    fn presents_identity(&self) -> bool {
        match &self.tls {
            Some(TlsSource::Settings(settings)) => settings.get_identity().is_some(),
            Some(TlsSource::Files(_)) => true,
            None => false,
        }
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

    /// The TLS option that is set, for an error message.
    fn name_tls_option(&self) -> Option<&'static str> {
        match &self.tls {
            Some(TlsSource::Settings(_)) => Some("tls"),
            Some(TlsSource::Files(_)) => Some("mtls"),
            None => None,
        }
    }

    /// The TLS settings of an `https` endpoint: the ones given, the files named read, or the system's roots.
    fn resolve_tls(&self) -> Result<TlsSettings, GrpcChannelConfigError> {
        match &self.tls {
            Some(TlsSource::Settings(settings)) => Ok(settings.clone()),
            Some(TlsSource::Files(mtls)) => {
                StdTlsSettingsResolver.resolve(ResolveTlsSettingsRequest {
                    identity: Some(mtls),
                    root_ca: mtls.ca_pem_path.as_deref(),
                })
            }
            None => Ok(TlsSettings::create(None, Vec::new(), None)),
        }
    }

    /// Returns `true` when `endpoint` starts with `http://` (case-insensitive).
    fn is_plaintext_endpoint(endpoint: &str) -> bool {
        endpoint.len() >= 7 && endpoint[..7].eq_ignore_ascii_case("http://")
    }

    /// The client for `url`: the `proto-svc` factory builds it from the options of this builder, and TLS is set up
    /// for an `https` endpoint only.
    fn create_client(
        &self,
        url: &str,
        tls: &TlsSettings,
    ) -> Result<impl Transport + RawTransport + 'static, GrpcChannelConfigError> {
        let mut factory = TonicTransportSpiFactory::new(url).with_timeout(self.request_timeout);
        if let Some(bytes) = self.max_decoding_message_size {
            factory = factory.with_max_decoding_message_size(bytes);
        }
        if let Some(bytes) = self.max_encoding_message_size {
            factory = factory.with_max_encoding_message_size(bytes);
        }
        if !Self::is_plaintext_endpoint(url) {
            factory = factory.with_tls(tls.clone());
        }
        if let Some(compression) = self.send_compression {
            factory = factory.with_send_compression(compression);
        }
        for compression in &self.accept_compression {
            factory = factory.with_accept_compression(*compression);
        }
        if let Some(keep_alive) = self.keep_alive {
            factory = factory
                .with_http2_keep_alive_interval(keep_alive.interval)
                .with_keep_alive_timeout(keep_alive.timeout)
                .with_keep_alive_while_idle(keep_alive.permit_without_calls);
        }
        DualTransportFactory::build(&factory, BuildTransportRequest).map_err(|e| {
            GrpcChannelConfigError::InvalidOptionValue {
                option: "endpoint",
                reason: e.to_string(),
            }
        })
    }
}
