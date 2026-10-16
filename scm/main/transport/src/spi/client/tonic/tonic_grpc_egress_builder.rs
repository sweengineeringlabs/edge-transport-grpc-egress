//! `TonicGrpcEgressBuilder` -- the builder of [`TonicGrpcEgress`], the `hyper`-backed client.
//!
//! Not part of the crate's public surface: a caller gets a builder from
//! [`crate::saf::TonicGrpcEgressBuilderSpiFactory`] and uses it through the
//! [`GrpcEgressBuilder`] trait.

use std::sync::Arc;
use std::time::Duration;

use crate::api::{
    CompressionMode, GrpcChannelConfig, GrpcChannelConfigError, GrpcEgress, GrpcEgressBuilder,
    GrpcEgressInterceptor, GrpcEgressInterceptorChain, MtlsConfig,
    TraceContextGrpcEgressInterceptor, TraceContextSource, DEFAULT_MAX_MESSAGE_BYTES,
    DEFAULT_REQUEST_TIMEOUT_SECS,
};

use super::tonic_grpc_egress::TonicGrpcEgress;
use super::tonic_grpc_egress_protocol::TonicGrpcEgressProtocol;

/// Holds the options of one client and builds it.
///
/// Defaults: TLS required, a 30 s request timeout, a 4 MiB message cap, no compression, no
/// interceptor.
pub(crate) struct TonicGrpcEgressBuilder {
    endpoint: String,
    plaintext_allowed: bool,
    request_timeout: Duration,
    max_message_bytes: usize,
    compression: CompressionMode,
    interceptors: GrpcEgressInterceptorChain,
    mtls: Option<MtlsConfig>,
    pinned_ca: Option<String>,
    /// Options of an applied [`GrpcChannelConfig`] that this client cannot apply yet. `build`
    /// refuses them, so that none is accepted and silently ignored.
    unapplied: Vec<&'static str>,
}

impl TonicGrpcEgressBuilder {
    /// A builder for `endpoint` with every option at its default.
    pub(crate) fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            plaintext_allowed: false,
            request_timeout: Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS),
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            compression: CompressionMode::None,
            interceptors: GrpcEgressInterceptorChain::new(),
            mtls: None,
            pinned_ca: None,
            unapplied: Vec::new(),
        }
    }
}

impl GrpcEgressBuilder for TonicGrpcEgressBuilder {
    fn with_endpoint(self, endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
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

    fn with_pinned_ca(self, ca_pem_path: impl Into<String>) -> Self {
        Self {
            pinned_ca: Some(ca_pem_path.into()),
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

    fn with_channel_config(mut self, config: &GrpcChannelConfig) -> Self {
        if config.keep_alive.is_some() {
            self.unapplied.push("keep_alive");
        }
        let request_timeout = config
            .request_timeout_secs
            .map_or(self.request_timeout, Duration::from_secs);
        Self {
            endpoint: config.endpoint.clone(),
            plaintext_allowed: !config.tls_required,
            request_timeout,
            max_message_bytes: config.max_message_bytes,
            compression: config.compression,
            mtls: config.mtls.clone().or(self.mtls),
            ..self
        }
    }

    fn build(self) -> Result<impl GrpcEgress + use<>, GrpcChannelConfigError> {
        if let Some(option) = self.unapplied.first() {
            return Err(GrpcChannelConfigError::OptionNotApplied(option));
        }
        if !self.plaintext_allowed && TonicGrpcEgressProtocol::is_plaintext_endpoint(&self.endpoint)
        {
            return Err(GrpcChannelConfigError::PlaintextRejected(self.endpoint));
        }
        let pinned_ca = self
            .pinned_ca
            .as_deref()
            .or_else(|| self.mtls.as_ref().and_then(|m| m.ca_pem_path.as_deref()));
        Ok(TonicGrpcEgress::new(self.endpoint)
            .with_tls(self.mtls.as_ref(), pinned_ca)?
            .with_request_timeout(self.request_timeout)
            .with_max_message_bytes(self.max_message_bytes)
            .with_compression(self.compression)
            .with_interceptors(self.interceptors))
    }
}
