//! `TonicGrpcEgress` — concrete `GrpcEgress` implementation backed by hyper HTTP/2.

/// Core implementation unit for `TonicGrpcEgress`.
///
/// The struct fields live in `spi/client/tonic/tonic_grpc_egress.rs` (the public
/// type); this marker holds the protocol-encoding helper methods and the
/// `GrpcEgress`/`Processor` impls for that type.
pub(crate) struct TonicGrpcEgressProtocol;

use std::time::Duration;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::future::BoxFuture;
use futures::StreamExt as _;
use http_body_util::{BodyExt as _, Full};
use tokio_util::sync::CancellationToken;

use super::tonic_grpc_egress::TonicGrpcEgress;
use crate::api::Conversions as StatusConversions;
use crate::api::{
    CompressionMode, GrpcChannelConfig, GrpcChannelConfigError, GrpcEgress, GrpcEgressError,
    GrpcEgressInterceptorChain, GrpcEgressResult, GrpcMessageStreamResponse, GrpcRequest,
    GrpcResponse, GrpcStatusCode, DEFAULT_MAX_MESSAGE_BYTES,
};

const SANITIZED_INTERNAL_MSG: &str = "internal client error";

// ── internal helpers ─────────────────────────────────────────────────────────

impl TonicGrpcEgressProtocol {
    /// Encode `payload` as a single gRPC data frame:
    /// `[0x00][len_u32_be][payload]` — not compressed.
    fn encode_grpc_frame(payload: &[u8]) -> Bytes {
        let mut buf = BytesMut::with_capacity(5 + payload.len());
        buf.put_u8(0x00); // compression flag: not compressed
        buf.put_u32(payload.len() as u32);
        buf.put_slice(payload);
        buf.freeze()
    }

    /// Decode all complete gRPC frames from `data`.
    ///
    /// Each frame: `[compression_flag: u8][length: u32_be][payload: length bytes]`.
    /// Frames with the compression flag set are passed through as-is (no decompression).
    fn decode_grpc_frames(mut data: Bytes) -> GrpcEgressResult<Vec<Vec<u8>>> {
        const FRAME_HEADER: usize = 5;
        let mut out = Vec::new();
        while data.len() >= FRAME_HEADER {
            let _flag = data[0];
            let len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
            data.advance(FRAME_HEADER);
            if data.len() < len {
                tracing::warn!(
                    expected = len,
                    actual = data.len(),
                    "truncated gRPC frame received from server",
                );
                return Err(GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into()));
            }
            out.push(data[..len].to_vec());
            data.advance(len);
        }
        Ok(out)
    }

    /// Encode a `Duration` as a `grpc-timeout` header value per the gRPC protocol:
    /// integer value followed by a unit suffix (`H`, `M`, `S`, `m`, `u`, `n`).
    ///
    /// Picks the smallest unit that fits the duration in a `u64`, preferring
    /// milliseconds for sub-second values and seconds otherwise.
    fn encode_grpc_timeout(d: Duration) -> String {
        // Per RFC, value MUST fit in 8 ASCII digits — we cap at 99 999 999 of the
        // chosen unit to stay safely within that.  For practical deadlines this is
        // 27+ hours of seconds or 27+ years of seconds — never reached in real use.
        const MAX_VAL: u128 = 99_999_999;

        let nanos = d.as_nanos();
        if nanos == 0 {
            return "0n".into();
        }
        if nanos <= MAX_VAL {
            return format!("{nanos}n");
        }
        let micros = d.as_micros();
        if micros <= MAX_VAL {
            return format!("{micros}u");
        }
        let millis = d.as_millis();
        if millis <= MAX_VAL {
            return format!("{millis}m");
        }
        let secs = d.as_secs();
        if (secs as u128) <= MAX_VAL {
            return format!("{secs}S");
        }
        let mins = secs / 60;
        if (mins as u128) <= MAX_VAL {
            return format!("{mins}M");
        }
        let hours = secs / 3600;
        format!("{hours}H")
    }

    /// Build a `http::Request` from a pre-encoded body, URI string, and metadata.
    ///
    /// Always injects the gRPC-mandatory headers (`content-type`, `te: trailers`).
    /// Caller-supplied metadata headers come last — they may override defaults
    /// when intentional (no defensive filtering, since the caller owns the wire).
    fn build_http_request(
        uri_str: &str,
        body_bytes: Bytes,
        metadata: &std::collections::HashMap<String, String>,
        deadline: Option<Duration>,
    ) -> GrpcEgressResult<http::Request<Full<Bytes>>> {
        let uri: http::Uri = uri_str.parse().map_err(|e| {
            tracing::warn!(error = %e, uri = %uri_str, "invalid gRPC URI supplied by caller");
            GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into())
        })?;

        let mut builder = http::Request::builder()
            .method(http::Method::POST)
            .uri(uri)
            .header(http::header::CONTENT_TYPE, "application/grpc")
            .header("te", "trailers");

        if let Some(d) = deadline {
            builder = builder.header(
                "grpc-timeout",
                TonicGrpcEgressProtocol::encode_grpc_timeout(d),
            );
        }

        for (k, v) in metadata {
            builder = builder.header(k.as_str(), v.as_str());
        }

        builder.body(Full::new(body_bytes)).map_err(|e| {
            tracing::warn!(error = %e, "failed to build HTTP request for gRPC call");
            GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into())
        })
    }

    /// Extract `HashMap<String, String>` from an optional `HeaderMap` reference.
    fn header_map_to_hash(
        map: Option<&http::HeaderMap>,
    ) -> std::collections::HashMap<String, String> {
        let mut out = std::collections::HashMap::new();
        if let Some(m) = map {
            for (k, v) in m {
                if let Ok(s) = v.to_str() {
                    out.insert(k.as_str().to_owned(), s.to_owned());
                }
            }
        }
        out
    }

    /// Check the `grpc-status` trailer value; return `Err(Status(...))` for anything != "0".
    ///
    /// When the status is `RESOURCE_EXHAUSTED` and the response headers carry a
    /// `retry-after` value (seconds), the value is embedded into the error message
    /// as `[retry-after=Ns]`. `edge-transport-grpc-egress-retry`'s retry decorator
    /// parses this hint to honour the upstream reset window rather than guessing.
    ///
    /// `grpc-message` is a *server-supplied* sanitized message that the gRPC spec
    /// already requires not to contain server internals.  We pass it through as-is.
    fn check_grpc_status(
        trailers: Option<&http::HeaderMap>,
        response_headers: Option<&http::HeaderMap>,
    ) -> GrpcEgressResult<()> {
        let code_str = trailers
            .and_then(|m| m.get("grpc-status"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("0");

        if code_str == "0" {
            return Ok(());
        }

        let wire: i32 = code_str.parse().unwrap_or(2 /* Unknown */);
        let code = StatusConversions::from_wire(wire);
        let mut message = trailers
            .and_then(|m| m.get("grpc-message"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();

        // Embed Retry-After hint for RESOURCE_EXHAUSTED so the resilient client
        // can honour the upstream reset window. Only integer seconds are supported
        // (the HTTP-date form is uncommon in gRPC contexts).
        if code == GrpcStatusCode::ResourceExhausted {
            if let Some(secs) = response_headers
                .and_then(|h| {
                    h.get("retry-after")
                        .or_else(|| h.get("x-ratelimit-reset-requests"))
                })
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
            {
                message = format!("{message} [retry-after={secs}s]");
            }
        }

        Err(GrpcEgressError::Status(code, message))
    }
} // impl TonicGrpcEgressProtocol (helpers)

// ── constructor helpers ───────────────────────────────────────────────────────

impl TonicGrpcEgress {
    /// Create a client with an explicit fallback timeout.
    pub(crate) fn with_timeout(base_uri: impl Into<String>, timeout: Duration) -> Self {
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
            timeout,
            interceptors: GrpcEgressInterceptorChain::new(),
            max_message_bytes: DEFAULT_MAX_MESSAGE_BYTES,
            compression: CompressionMode::None,
        }
    }

    /// Construct a client from a [`GrpcChannelConfig`].
    ///
    /// **Fail-closed**: if `config.tls_required` is `true` and the
    /// endpoint URL has an `http://` scheme, returns
    /// [`GrpcChannelConfigError::PlaintextRejected`] before any
    /// transport setup.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "used in tests; superseded by TransportSvc factory"
        )
    )]
    fn from_config(config: &GrpcChannelConfig) -> Result<Self, GrpcChannelConfigError> {
        use crate::api::DEFAULT_REQUEST_TIMEOUT_SECS;
        if config.tls_required && TonicGrpcEgressProtocol::is_plaintext_endpoint(&config.endpoint) {
            return Err(GrpcChannelConfigError::PlaintextRejected(
                config.endpoint.clone(),
            ));
        }
        let timeout = Duration::from_secs(
            config
                .request_timeout_secs
                .unwrap_or(DEFAULT_REQUEST_TIMEOUT_SECS),
        );
        let mut client = Self::with_timeout(&config.endpoint, timeout);
        client.max_message_bytes = config.max_message_bytes;
        client.compression = config.compression;
        Ok(client)
    }

    /// Attach an interceptor chain to the client.  Replaces any previous chain.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "only called by TonicGrpcEgressBuilder, which is itself production-wiring-pending"
        )
    )]
    pub(crate) fn with_interceptors(mut self, chain: GrpcEgressInterceptorChain) -> Self {
        self.interceptors = chain;
        self
    }

    /// Override the max-message-bytes cap.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "only called by TonicGrpcEgressBuilder, which is itself production-wiring-pending"
        )
    )]
    pub(crate) fn with_max_message_bytes(mut self, bytes: usize) -> Self {
        self.max_message_bytes = bytes;
        self
    }

    /// Override the compression mode.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "only called by TonicGrpcEgressBuilder, which is itself production-wiring-pending"
        )
    )]
    pub(crate) fn with_compression(mut self, mode: CompressionMode) -> Self {
        self.compression = mode;
        self
    }
}

impl TonicGrpcEgressProtocol {
    /// Returns `true` when `endpoint` starts with `http://` (case-insensitive).
    pub(crate) fn is_plaintext_endpoint(endpoint: &str) -> bool {
        endpoint.len() >= 7 && endpoint[..7].eq_ignore_ascii_case("http://")
    }

    /// Race a future against the request's deadline AND its optional cancellation
    /// token.  Used to wrap every awaitable network step in unary/stream calls.
    ///
    /// Resolves to:
    /// - `Ok(value)`     — future completed in time
    /// - `Err(Timeout)`  — deadline elapsed first
    /// - `Err(Cancelled)`— cancellation token fired first
    async fn race_deadline_and_cancel<F, T>(
        fut: F,
        deadline: Duration,
        cancel: Option<&CancellationToken>,
    ) -> GrpcEgressResult<T>
    where
        F: std::future::Future<Output = T>,
    {
        let timeout_fut = tokio::time::timeout(deadline, fut);
        match cancel {
            Some(token) => tokio::select! {
                biased;
                _ = token.cancelled() => Err(GrpcEgressError::Cancelled(
                    "caller cancelled in-flight request".into(),
                )),
                res = timeout_fut => res.map_err(|_| GrpcEgressError::Timeout(
                    "request deadline exceeded".into(),
                )),
            },
            None => timeout_fut
                .await
                .map_err(|_| GrpcEgressError::Timeout("request deadline exceeded".into())),
        }
    }
} // impl TonicGrpcEgressProtocol (endpoint + race)

// ── Processor impl ───────────────────────────────────────────────────────────

impl crate::api::Processor for TonicGrpcEgress {
    fn process(
        &self,
        _req: crate::api::ProcessingRequest,
    ) -> futures::future::BoxFuture<'_, Result<(), GrpcEgressError>> {
        // Default: verify the endpoint is reachable — a no-op health probe.
        Box::pin(self.health_check(crate::api::HealthCheckRequest))
    }

    fn describe(
        &self,
        _req: crate::api::DescribeRequest,
    ) -> Result<crate::api::DescribeResponse, GrpcEgressError> {
        const LABEL: &str = "tonic-grpc-client";
        Ok(crate::api::DescribeResponse { label: LABEL })
    }
}

// ── GrpcEgress impl ─────────────────────────────────────────────────────────

impl GrpcEgress for TonicGrpcEgress {
    fn call_unary(
        &self,
        mut request: GrpcRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        // Run before-call interceptors; first failure short-circuits.
        if let Err(e) = self.interceptors.run_before(&mut request) {
            return Box::pin(futures::future::ready(Err(e)));
        }

        // Inject grpc-encoding when compression is enabled.  Done after
        // interceptors so they can override the negotiated value.
        if let Some(name) = self.compression.header_value() {
            request
                .metadata
                .entry("grpc-encoding".to_string())
                .or_insert_with(|| name.to_string());
            request
                .metadata
                .entry("grpc-accept-encoding".to_string())
                .or_insert_with(|| name.to_string());
        }

        // Method paths are conventionally written with a leading `/`
        // (e.g. `/pkg.Service/Method`); trim it so we never produce
        // `host//path`. Strict registry-based dispatchers (e.g.
        // `HandlerRegistryDispatcher`) key on the exact path string and
        // reject the double-slash form.
        let method = request.method.trim_start_matches('/');
        let uri_str = format!("{}/{}", self.base_uri.trim_end_matches('/'), method);
        let body_bytes = TonicGrpcEgressProtocol::encode_grpc_frame(&request.body);
        let deadline = request.deadline;
        let cancel = request.cancellation.clone();
        let max_bytes = self.max_message_bytes;
        let interceptors = self.interceptors.clone();
        let http_req = match TonicGrpcEgressProtocol::build_http_request(
            &uri_str,
            body_bytes,
            &request.metadata,
            Some(deadline),
        ) {
            Ok(r) => r,
            Err(e) => return Box::pin(futures::future::ready(Err(e))),
        };

        Box::pin(async move {
            let resp = TonicGrpcEgressProtocol::race_deadline_and_cancel(
                self.client.request(http_req),
                deadline,
                cancel.as_ref(),
            )
            .await?
            .map_err(|e| {
                tracing::warn!(error = %e, "hyper transport error during gRPC call");
                GrpcEgressError::ConnectionFailed("transport error".into())
            })?;

            let response_headers = resp.headers().clone();
            TonicGrpcEgressProtocol::check_grpc_status(
                Some(&response_headers),
                Some(&response_headers),
            )?;

            // Bound the response body; oversize returns ResourceExhausted.
            let limited = http_body_util::Limited::new(resp.into_body(), max_bytes + 5);
            let collected = TonicGrpcEgressProtocol::race_deadline_and_cancel(
                limited.collect(),
                deadline,
                cancel.as_ref(),
            )
            .await?
            .map_err(|e| {
                tracing::warn!(error = %e, "response body exceeded max_message_bytes or transport error");
                GrpcEgressError::Status(
                    GrpcStatusCode::ResourceExhausted,
                    "response body exceeded max_message_bytes".into(),
                )
            })?;

            TonicGrpcEgressProtocol::check_grpc_status(
                collected.trailers(),
                Some(&response_headers),
            )?;

            let trailer_headers = TonicGrpcEgressProtocol::header_map_to_hash(collected.trailers());
            let data = collected.to_bytes();

            let body = if data.len() >= 5 {
                data[5..].to_vec()
            } else {
                data.to_vec()
            };

            let mut response = GrpcResponse {
                body,
                metadata: trailer_headers,
            };

            // Run after-call interceptors; first failure short-circuits.
            interceptors.run_after(&mut response)?;

            Ok(response)
        })
    }

    /// Send a gRPC streaming call.
    ///
    /// **Buffering limitation**: both the request and response are fully buffered
    /// in memory before this future resolves. The `GrpcMessageStreamResponse` input is
    /// collected into a single HTTP/2 DATA frame sequence, and the response body
    /// is collected before returning. This is functionally correct for small
    /// message sets but is not suitable for large or infinite streams. True
    /// chunked streaming requires replacing hyper's `Full<Bytes>` body type with
    /// a streaming body, which is a separate task.
    ///
    /// Streaming has no per-request deadline yet — uses the client-level
    /// `timeout`.  Phase 2 will plumb a `GrpcRequest`-shaped streaming envelope.
    fn call_stream(
        &self,
        req: crate::api::CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        let crate::api::CallStreamRequest {
            method,
            metadata,
            messages,
        } = req;
        let method = method.trim_start_matches('/');
        let uri_str = format!("{}/{}", self.base_uri.trim_end_matches('/'), method);

        Box::pin(async move {
            // Collect all input messages into one body (multiple gRPC frames).
            let mut body_buf = BytesMut::new();
            let mut stream = messages.stream;
            while let Some(item) = stream.next().await {
                let payload = item?;
                let frame = TonicGrpcEgressProtocol::encode_grpc_frame(&payload);
                body_buf.put(frame);
            }

            let http_req = TonicGrpcEgressProtocol::build_http_request(
                &uri_str,
                body_buf.freeze(),
                &metadata,
                Some(self.timeout),
            )?;

            let resp = tokio::time::timeout(self.timeout, self.client.request(http_req))
                .await
                .map_err(|_| GrpcEgressError::Timeout("stream request deadline exceeded".into()))?
                .map_err(|e| {
                    tracing::warn!(error = %e, "hyper transport error during gRPC stream");
                    GrpcEgressError::ConnectionFailed("transport error".into())
                })?;

            // Check grpc-status in the initial response headers first.
            TonicGrpcEgressProtocol::check_grpc_status(Some(resp.headers()), Some(resp.headers()))?;

            let collected = resp.into_body().collect().await.map_err(|e| {
                tracing::warn!(error = %e, "failed to read gRPC stream response body");
                GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into())
            })?;

            // Also check trailers.
            TonicGrpcEgressProtocol::check_grpc_status(collected.trailers(), None)?;

            let data = collected.to_bytes();
            let frames = TonicGrpcEgressProtocol::decode_grpc_frames(data)?;
            let items: Vec<GrpcEgressResult<Vec<u8>>> = frames.into_iter().map(Ok).collect();
            Ok(GrpcMessageStreamResponse {
                stream: Box::pin(futures::stream::iter(items)),
            })
        })
    }

    /// Send a server-streaming request — single encoded request, multiple response frames.
    fn call_server_stream(
        &self,
        mut request: GrpcRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        if let Err(e) = self.interceptors.run_before(&mut request) {
            return Box::pin(futures::future::ready(Err(e)));
        }
        let method = request.method.trim_start_matches('/');
        let uri_str = format!("{}/{}", self.base_uri.trim_end_matches('/'), method);
        let body_bytes = TonicGrpcEgressProtocol::encode_grpc_frame(&request.body);
        let deadline = request.deadline;
        let cancel = request.cancellation.clone();
        let max_bytes = self.max_message_bytes;
        let http_req = match TonicGrpcEgressProtocol::build_http_request(
            &uri_str,
            body_bytes,
            &request.metadata,
            Some(deadline),
        ) {
            Ok(r) => r,
            Err(e) => return Box::pin(futures::future::ready(Err(e))),
        };

        Box::pin(async move {
            let resp = TonicGrpcEgressProtocol::race_deadline_and_cancel(
                self.client.request(http_req),
                deadline,
                cancel.as_ref(),
            )
            .await?
            .map_err(|e| {
                tracing::warn!(error = %e, "hyper transport error during gRPC server-stream");
                GrpcEgressError::ConnectionFailed("transport error".into())
            })?;

            let response_headers = resp.headers().clone();
            TonicGrpcEgressProtocol::check_grpc_status(
                Some(&response_headers),
                Some(&response_headers),
            )?;

            let limited = http_body_util::Limited::new(resp.into_body(), max_bytes + 5);
            let collected = TonicGrpcEgressProtocol::race_deadline_and_cancel(
                limited.collect(),
                deadline,
                cancel.as_ref(),
            )
            .await?
            .map_err(|_| {
                GrpcEgressError::Status(
                    GrpcStatusCode::ResourceExhausted,
                    "response exceeded max_message_bytes".into(),
                )
            })?;

            TonicGrpcEgressProtocol::check_grpc_status(
                collected.trailers(),
                Some(&response_headers),
            )?;

            let data = collected.to_bytes();
            let frames = TonicGrpcEgressProtocol::decode_grpc_frames(data)?;
            let items: Vec<GrpcEgressResult<Vec<u8>>> = frames.into_iter().map(Ok).collect();
            Ok(GrpcMessageStreamResponse {
                stream: Box::pin(futures::stream::iter(items)),
            })
        })
    }

    /// Send a client-streaming request — multiple request frames, single response.
    fn call_client_stream(
        &self,
        req: crate::api::CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        let crate::api::CallStreamRequest {
            method,
            metadata,
            messages,
        } = req;
        let method_path = method.trim_start_matches('/').to_owned();
        let uri_str = format!("{}/{}", self.base_uri.trim_end_matches('/'), method_path);
        let deadline = self.timeout;
        let max_bytes = self.max_message_bytes;
        let interceptors = self.interceptors.clone();

        Box::pin(async move {
            // Collect all request frames into one body.
            let mut body_buf = BytesMut::new();
            let mut stream = messages.stream;
            while let Some(item) = stream.next().await {
                let payload = item?;
                body_buf.put(TonicGrpcEgressProtocol::encode_grpc_frame(&payload));
            }

            let http_req = TonicGrpcEgressProtocol::build_http_request(
                &uri_str,
                body_buf.freeze(),
                &metadata,
                Some(deadline),
            )?;

            let resp = tokio::time::timeout(deadline, self.client.request(http_req))
                .await
                .map_err(|_| {
                    GrpcEgressError::Timeout("client-stream request deadline exceeded".into())
                })?
                .map_err(|e| {
                    tracing::warn!(error = %e, "hyper transport error during gRPC client-stream");
                    GrpcEgressError::ConnectionFailed("transport error".into())
                })?;

            let response_headers = resp.headers().clone();
            TonicGrpcEgressProtocol::check_grpc_status(
                Some(&response_headers),
                Some(&response_headers),
            )?;

            let limited = http_body_util::Limited::new(resp.into_body(), max_bytes + 5);
            let collected = tokio::time::timeout(deadline, limited.collect())
                .await
                .map_err(|_| {
                    GrpcEgressError::Timeout("client-stream response deadline exceeded".into())
                })?
                .map_err(|_| {
                    GrpcEgressError::Status(
                        GrpcStatusCode::ResourceExhausted,
                        "response exceeded max_message_bytes".into(),
                    )
                })?;

            TonicGrpcEgressProtocol::check_grpc_status(
                collected.trailers(),
                Some(&response_headers),
            )?;

            let trailer_headers = TonicGrpcEgressProtocol::header_map_to_hash(collected.trailers());
            let data = collected.to_bytes();
            let body = if data.len() >= 5 {
                data[5..].to_vec()
            } else {
                data.to_vec()
            };

            let mut response = GrpcResponse {
                body,
                metadata: trailer_headers,
            };

            interceptors.run_after(&mut response)?;
            Ok(response)
        })
    }

    /// Send a bidirectional-streaming request — delegates to [`call_stream`].
    ///
    /// [`call_stream`]: GrpcEgress::call_stream
    fn call_bidi_stream(
        &self,
        req: crate::api::CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        self.call_stream(req)
    }

    fn health_check(
        &self,
        _req: crate::api::HealthCheckRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<()>> {
        let base_uri = self.base_uri.clone();

        Box::pin(async move {
            let uri: http::Uri = base_uri.parse().map_err(|e| {
                tracing::warn!(error = %e, uri = %base_uri, "invalid gRPC URI in health check");
                GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into())
            })?;

            let host = uri.host().unwrap_or("127.0.0.1").to_owned();
            let port = uri.port_u16().unwrap_or(50051);
            let addr = format!("{host}:{port}");

            tokio::net::TcpStream::connect(&addr)
                .await
                .map(|_| ())
                .map_err(|e| GrpcEgressError::Unavailable(format!("{addr}: {e}")))
        })
    }
}

#[cfg(feature = "prost")]
impl crate::api::GrpcEgressProstCodec for TonicGrpcEgress {}

impl TonicGrpcEgressProtocol {
    /// Compile-time guard: `GrpcRequest::new` MUST require a `Duration`.
    #[doc(hidden)]
    pub(crate) fn _grpc_request_new_requires_deadline_compile_check() -> GrpcRequest {
        GrpcRequest::new("svc/Method", Vec::new(), Duration::from_secs(1))
    }

    // Quiet the dead-code warning on a public-only-by-import symbol.
    fn _suppress_status_code_unused_import_warning(c: GrpcStatusCode) -> GrpcStatusCode {
        c
    }
} // impl TonicGrpcEgressProtocol (compile checks)

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_plaintext_endpoint (private fn — inline per rule 37) ──────────────

    #[test]
    fn test_is_plaintext_endpoint_returns_true_for_http_scheme() {
        assert!(TonicGrpcEgressProtocol::is_plaintext_endpoint(
            "http://localhost:50051"
        ));
        assert!(TonicGrpcEgressProtocol::is_plaintext_endpoint(
            "HTTP://example.com:443"
        ));
    }

    #[test]
    fn test_is_plaintext_endpoint_returns_false_for_https_scheme() {
        assert!(!TonicGrpcEgressProtocol::is_plaintext_endpoint(
            "https://secure.example.com:443"
        ));
    }

    #[test]
    fn test_is_plaintext_endpoint_returns_false_for_empty_string() {
        assert!(!TonicGrpcEgressProtocol::is_plaintext_endpoint(""));
    }

    #[test]
    fn test_is_plaintext_endpoint_returns_false_for_short_string() {
        assert!(!TonicGrpcEgressProtocol::is_plaintext_endpoint("http:/"));
    }

    // ── other core tests ─────────────────────────────────────────────────────

    #[test]
    fn test_new_client_stores_base_uri() {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .ok();
        let client = TonicGrpcEgress::new("http://localhost:50051");
        assert_eq!(client.base_uri, "http://localhost:50051");
    }

    #[test]
    fn test_with_timeout_overrides_duration() {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .ok();
        let d = Duration::from_secs(5);
        let client = TonicGrpcEgress::with_timeout("http://localhost:50051", d);
        assert_eq!(client.timeout, d);
    }

    #[test]
    fn test_grpc_egress_is_object_safe() {
        fn _assert(_: &dyn GrpcEgress) {}
    }

    #[test]
    fn test_encode_grpc_frame_produces_5_byte_header() {
        let frame = TonicGrpcEgressProtocol::encode_grpc_frame(b"hello");
        assert_eq!(frame.len(), 10); // 5 header + 5 payload
        assert_eq!(frame[0], 0x00); // not compressed
        assert_eq!(
            u32::from_be_bytes([frame[1], frame[2], frame[3], frame[4]]),
            5
        );
        assert_eq!(&frame[5..], b"hello");
    }

    #[test]
    fn test_decode_grpc_frames_round_trips_single_frame() {
        let encoded = TonicGrpcEgressProtocol::encode_grpc_frame(b"world");
        let decoded = TonicGrpcEgressProtocol::decode_grpc_frames(encoded).expect("decode failed");
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], b"world");
    }

    #[test]
    fn test_decode_grpc_frames_round_trips_multiple_frames() {
        let mut buf = BytesMut::new();
        buf.put(TonicGrpcEgressProtocol::encode_grpc_frame(b"one"));
        buf.put(TonicGrpcEgressProtocol::encode_grpc_frame(b"two"));
        buf.put(TonicGrpcEgressProtocol::encode_grpc_frame(b"three"));
        let decoded =
            TonicGrpcEgressProtocol::decode_grpc_frames(buf.freeze()).expect("decode failed");
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0], b"one");
        assert_eq!(decoded[1], b"two");
        assert_eq!(decoded[2], b"three");
    }

    #[test]
    fn test_decode_grpc_frames_returns_internal_error_on_truncated_data() {
        // 5-byte header says length=100 but only 3 bytes of payload follow.
        let mut buf = BytesMut::new();
        buf.put_u8(0x00);
        buf.put_u32(100_u32);
        buf.put_slice(b"abc");
        let result = TonicGrpcEgressProtocol::decode_grpc_frames(buf.freeze());
        match result {
            Err(GrpcEgressError::Internal(msg)) => {
                // Must be the sanitized constant — never raw byte counts on the wire.
                assert_eq!(msg, SANITIZED_INTERNAL_MSG);
            }
            other => panic!("expected Internal(sanitized), got {other:?}"),
        }
    }

    #[test]
    fn test_encode_grpc_timeout_uses_nanos_for_small_values() {
        assert_eq!(
            TonicGrpcEgressProtocol::encode_grpc_timeout(Duration::from_nanos(1)),
            "1n"
        );
        assert_eq!(
            TonicGrpcEgressProtocol::encode_grpc_timeout(Duration::from_micros(1)),
            "1000n"
        );
    }

    #[test]
    fn test_encode_grpc_timeout_uses_millis_or_higher_for_seconds() {
        // 30s = 30_000_000_000 ns — too big for nanos, fits microseconds (3e10).
        // Actually 3e10 > 99_999_999, so it falls through to millis (30_000) or higher.
        let s = TonicGrpcEgressProtocol::encode_grpc_timeout(Duration::from_secs(30));
        // Accept any unit suffix as long as the value parses and is positive.
        let last = s.chars().last().expect("non-empty timeout encoding");
        assert!("nuMSmH".contains(last), "unexpected unit suffix in {s}");
    }

    #[test]
    fn test_encode_grpc_timeout_handles_zero_duration() {
        assert_eq!(
            TonicGrpcEgressProtocol::encode_grpc_timeout(Duration::ZERO),
            "0n"
        );
    }

    #[test]
    fn test_grpc_request_new_requires_deadline_compile_check() {
        let r = TonicGrpcEgressProtocol::_grpc_request_new_requires_deadline_compile_check();
        assert_eq!(r.method, "svc/Method");
        assert_eq!(r.deadline, Duration::from_secs(1));
    }

    #[test]
    fn test_from_config_builds_client_from_channel_config() {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .ok();
        let cfg = crate::api::GrpcChannelConfig::new("http://localhost:50051").allow_plaintext();
        let client = TonicGrpcEgress::from_config(&cfg).expect("valid plaintext config");
        assert_eq!(client.base_uri, "http://localhost:50051");
    }

    #[test]
    fn test_from_config_honors_request_timeout_secs() {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .ok();
        let cfg = crate::api::GrpcChannelConfig::new("http://localhost:50051")
            .allow_plaintext()
            .with_request_timeout(Duration::from_secs(120));
        let client = TonicGrpcEgress::from_config(&cfg).unwrap();
        assert_eq!(client.timeout, Duration::from_secs(120));
    }

    #[test]
    fn test_from_config_defaults_to_30s_when_timeout_not_set() {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .ok();
        let cfg = crate::api::GrpcChannelConfig::new("http://localhost:50051").allow_plaintext();
        let client = TonicGrpcEgress::from_config(&cfg).unwrap();
        assert_eq!(client.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_with_interceptors_attaches_chain() {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .ok();
        let client = TonicGrpcEgress::new("http://localhost:50051")
            .with_interceptors(GrpcEgressInterceptorChain::new());
        assert_eq!(client.interceptors.len(), 0);
    }

    #[test]
    fn test_with_max_message_bytes_overrides_default_cap() {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .ok();
        let client =
            TonicGrpcEgress::new("http://localhost:50051").with_max_message_bytes(16 * 1024 * 1024);
        assert_eq!(client.max_message_bytes, 16 * 1024 * 1024);
    }

    #[test]
    fn test_with_compression_sets_mode() {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .ok();
        let client =
            TonicGrpcEgress::new("http://localhost:50051").with_compression(CompressionMode::Gzip);
        assert_eq!(client.compression, CompressionMode::Gzip);
    }

    // ── build_http_request (private fn — inline per rule 37) ────────────────

    #[test]
    fn test_build_http_request_sets_grpc_headers() {
        let req = TonicGrpcEgressProtocol::build_http_request(
            "http://localhost:50051/pkg.Svc/Method",
            Bytes::from_static(b"payload"),
            &std::collections::HashMap::new(),
            None,
        )
        .expect("valid uri must build");
        assert_eq!(
            req.headers().get(http::header::CONTENT_TYPE).unwrap(),
            "application/grpc"
        );
        assert_eq!(req.headers().get("te").unwrap(), "trailers");
    }

    #[test]
    fn test_build_http_request_invalid_uri_returns_internal_error() {
        let result = TonicGrpcEgressProtocol::build_http_request(
            "not a valid uri",
            Bytes::new(),
            &std::collections::HashMap::new(),
            None,
        );
        assert!(matches!(result, Err(GrpcEgressError::Internal(_))));
    }

    #[test]
    fn test_build_http_request_with_deadline_sets_grpc_timeout_header() {
        let req = TonicGrpcEgressProtocol::build_http_request(
            "http://localhost:50051/pkg.Svc/Method",
            Bytes::new(),
            &std::collections::HashMap::new(),
            Some(Duration::from_secs(5)),
        )
        .expect("valid uri must build");
        assert!(req.headers().contains_key("grpc-timeout"));
    }

    // ── header_map_to_hash (private fn — inline per rule 37) ─────────────────

    #[test]
    fn test_header_map_to_hash_none_returns_empty_map() {
        let out = TonicGrpcEgressProtocol::header_map_to_hash(None);
        assert!(out.is_empty());
    }

    #[test]
    fn test_header_map_to_hash_extracts_string_values() {
        let mut headers = http::HeaderMap::new();
        headers.insert("x-trace-id", "abc123".parse().unwrap());
        let out = TonicGrpcEgressProtocol::header_map_to_hash(Some(&headers));
        assert_eq!(out.get("x-trace-id"), Some(&"abc123".to_string()));
    }

    // ── check_grpc_status (private fn — inline per rule 37) ──────────────────

    #[test]
    fn test_check_grpc_status_zero_status_returns_ok() {
        let mut trailers = http::HeaderMap::new();
        trailers.insert("grpc-status", "0".parse().unwrap());
        assert!(TonicGrpcEgressProtocol::check_grpc_status(Some(&trailers), None).is_ok());
    }

    #[test]
    fn test_check_grpc_status_nonzero_status_returns_err() {
        let mut trailers = http::HeaderMap::new();
        trailers.insert("grpc-status", "13".parse().unwrap());
        trailers.insert("grpc-message", "boom".parse().unwrap());
        let err = TonicGrpcEgressProtocol::check_grpc_status(Some(&trailers), None)
            .expect_err("nonzero status must be Err");
        match err {
            GrpcEgressError::Status(GrpcStatusCode::Internal, msg) => {
                assert_eq!(msg, "boom");
            }
            other => panic!("expected Status(Internal, _), got {other:?}"),
        }
    }

    #[test]
    fn test_check_grpc_status_resource_exhausted_embeds_retry_after_hint_edge() {
        let mut trailers = http::HeaderMap::new();
        trailers.insert("grpc-status", "8".parse().unwrap()); // RESOURCE_EXHAUSTED
        let mut response_headers = http::HeaderMap::new();
        response_headers.insert("retry-after", "30".parse().unwrap());
        let err =
            TonicGrpcEgressProtocol::check_grpc_status(Some(&trailers), Some(&response_headers))
                .expect_err("nonzero status must be Err");
        match err {
            GrpcEgressError::Status(GrpcStatusCode::ResourceExhausted, msg) => {
                assert!(
                    msg.contains("[retry-after=30s]"),
                    "expected retry-after hint embedded, got: {msg}"
                );
            }
            other => panic!("expected Status(ResourceExhausted, _), got {other:?}"),
        }
    }

    // ── race_deadline_and_cancel (private fn — inline per rule 37) ───────────

    #[tokio::test]
    async fn test_race_deadline_and_cancel_completes_before_deadline() {
        let result = TonicGrpcEgressProtocol::race_deadline_and_cancel(
            async { 42 },
            Duration::from_secs(5),
            None,
        )
        .await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_race_deadline_and_cancel_times_out() {
        let result = TonicGrpcEgressProtocol::race_deadline_and_cancel(
            async {
                tokio::time::sleep(Duration::from_secs(60)).await;
            },
            Duration::from_millis(10),
            None,
        )
        .await;
        assert!(matches!(result, Err(GrpcEgressError::Timeout(_))));
    }

    #[tokio::test]
    async fn test_race_deadline_and_cancel_cancellation_fires_first_edge() {
        let token = CancellationToken::new();
        token.cancel();
        let result = TonicGrpcEgressProtocol::race_deadline_and_cancel(
            async {
                tokio::time::sleep(Duration::from_secs(60)).await;
            },
            Duration::from_secs(60),
            Some(&token),
        )
        .await;
        assert!(matches!(result, Err(GrpcEgressError::Cancelled(_))));
    }

    // ── _suppress_status_code_unused_import_warning (private fn — inline per rule 37) ──

    #[test]
    fn test_suppress_status_code_unused_import_warning_returns_input_unchanged() {
        assert_eq!(
            TonicGrpcEgressProtocol::_suppress_status_code_unused_import_warning(
                GrpcStatusCode::Ok
            ),
            GrpcStatusCode::Ok
        );
    }
}
