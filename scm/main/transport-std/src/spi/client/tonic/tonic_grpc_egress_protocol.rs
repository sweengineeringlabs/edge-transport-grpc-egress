//! `TonicGrpcEgress` — concrete `GrpcEgress` implementation backed by hyper HTTP/2.

/// Core implementation unit for `TonicGrpcEgress`.
///
/// The struct fields live in `spi/client/tonic/tonic_grpc_egress.rs` (the public
/// type); this marker holds the protocol-encoding helper methods and the
/// `GrpcEgress`/`Processor` impls for that type.
pub(crate) struct TonicGrpcEgressProtocol;

use crate::std::CompressionModeProvider;
use crate::std::Conversions as StatusConversions;
use crate::std::GrpcEgressInterceptorChain;
use std::time::Duration;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::future::BoxFuture;
use futures::StreamExt as _;
use http_body_util::{BodyExt as _, Full};
use tokio_util::sync::CancellationToken;

use super::grpc_message_codec::GrpcMessageCodec;
use super::tonic_grpc_egress::TonicGrpcEgress;

use grpc_egress_transport::{
    CompressionMode, GrpcEgress, GrpcEgressError, GrpcEgressResult, GrpcMessageStreamResponse,
    GrpcRequest, GrpcResponse, GrpcStatusCode,
};

const SANITIZED_INTERNAL_MSG: &str = "internal client error";

// ── internal helpers ─────────────────────────────────────────────────────────

impl TonicGrpcEgressProtocol {
    /// Encode `payload` as a single gRPC data frame: `[flag][len_u32_be][payload]`.
    ///
    /// With a compression `mode` the payload is compressed and the flag is `0x01`; without one
    /// the message is sent as it is and the flag is `0x00`.
    fn encode_grpc_frame(payload: &[u8], mode: CompressionMode) -> GrpcEgressResult<Bytes> {
        let compressed = GrpcMessageCodec::compress(mode, payload)?;
        let (flag, wire) = match &compressed {
            Some(bytes) => (0x01, bytes.as_slice()),
            None => (0x00, payload),
        };
        let len = u32::try_from(wire.len()).map_err(|_| {
            GrpcEgressError::Status(
                GrpcStatusCode::ResourceExhausted,
                "message exceeds the 4 GiB a gRPC frame can carry".into(),
            )
        })?;
        let mut buf = BytesMut::with_capacity(5 + wire.len());
        buf.put_u8(flag);
        buf.put_u32(len);
        buf.put_slice(wire);
        Ok(buf.freeze())
    }

    /// Decode all complete gRPC frames from `data`.
    ///
    /// Each frame: `[compression_flag: u8][length: u32_be][payload: length bytes]`. A frame
    /// with the flag set is decompressed with the `grpc-encoding` the peer announced
    /// (`encoding`), to at most `max_bytes`; a set flag without an announced encoding, a flag
    /// other than 0 or 1, and bytes left over that do not make a frame are errors.
    fn decode_grpc_frames(
        mut data: Bytes,
        encoding: Option<&str>,
        max_bytes: usize,
    ) -> GrpcEgressResult<Vec<Vec<u8>>> {
        const FRAME_HEADER: usize = 5;
        let mut out = Vec::new();
        while data.len() >= FRAME_HEADER {
            let flag = data[0];
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
            let message = match (flag, encoding) {
                (0x00, _) => data[..len].to_vec(),
                (0x01, Some(name)) => GrpcMessageCodec::decompress(name, &data[..len], max_bytes)?,
                (0x01, None) => {
                    tracing::warn!("compressed gRPC frame without a grpc-encoding header");
                    return Err(GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into()));
                }
                (other, _) => {
                    tracing::warn!(flag = other, "gRPC frame with an invalid compressed flag");
                    return Err(GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into()));
                }
            };
            out.push(message);
            data.advance(len);
        }
        if !data.is_empty() {
            tracing::warn!(
                left = data.len(),
                "bytes after the last gRPC frame do not make a frame"
            );
            return Err(GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into()));
        }
        Ok(out)
    }

    /// The one message of a unary response body; none is an empty message, more than one an error.
    fn read_unary_message(
        data: Bytes,
        encoding: Option<&str>,
        max_bytes: usize,
    ) -> GrpcEgressResult<Vec<u8>> {
        let mut messages = Self::decode_grpc_frames(data, encoding, max_bytes)?.into_iter();
        match (messages.next(), messages.next()) {
            (Some(message), None) => Ok(message),
            (None, _) => Ok(Vec::new()),
            (Some(_), Some(_)) => {
                tracing::warn!("unary gRPC response carried more than one message");
                Err(GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into()))
            }
        }
    }

    /// The `grpc-encoding` a response announced, if any.
    fn read_grpc_encoding(headers: &http::HeaderMap) -> Option<String> {
        headers
            .get("grpc-encoding")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
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
        content_type: &str,
    ) -> GrpcEgressResult<http::Request<Full<Bytes>>> {
        let uri: http::Uri = uri_str.parse().map_err(|e| {
            tracing::warn!(error = %e, uri = %uri_str, "invalid gRPC URI supplied by caller");
            GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into())
        })?;

        let mut builder = http::Request::builder()
            .method(http::Method::POST)
            .uri(uri)
            .header(http::header::CONTENT_TYPE, content_type)
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
    /// as `[retry-after=Ns]`. `grpc-egress-retry-std`'s retry decorator
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
    /// Set the client-side timeout that a call may not outlast, whatever its own deadline says.
    pub(crate) fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Attach an interceptor chain to the client.  Replaces any previous chain.
    pub(crate) fn with_interceptors(mut self, chain: GrpcEgressInterceptorChain) -> Self {
        self.interceptors = chain;
        self
    }

    /// Override the max-message-bytes cap.
    pub(crate) fn with_max_message_bytes(mut self, bytes: usize) -> Self {
        self.max_message_bytes = bytes;
        self
    }

    /// Override the compression mode.
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
        let body_bytes =
            match TonicGrpcEgressProtocol::encode_grpc_frame(&request.body, self.compression) {
                Ok(b) => b,
                Err(e) => return Box::pin(futures::future::ready(Err(e))),
            };
        // The client's request timeout is a backstop: a call never outlasts it, whatever deadline it carries.
        let deadline = request.deadline.min(self.timeout);
        let cancel = request.cancellation.clone();
        let max_bytes = self.max_message_bytes;
        let interceptors = self.interceptors.clone();
        let http_req = match TonicGrpcEgressProtocol::build_http_request(
            &uri_str,
            body_bytes,
            &request.metadata,
            Some(deadline),
            self.content_type,
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
            let response_encoding = TonicGrpcEgressProtocol::read_grpc_encoding(&response_headers);
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
            let body = TonicGrpcEgressProtocol::read_unary_message(
                data,
                response_encoding.as_deref(),
                max_bytes,
            )?;

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
        req: grpc_egress_transport::CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        let grpc_egress_transport::CallStreamRequest {
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
                let frame = TonicGrpcEgressProtocol::encode_grpc_frame(&payload, self.compression)?;
                body_buf.put(frame);
            }

            let http_req = TonicGrpcEgressProtocol::build_http_request(
                &uri_str,
                body_buf.freeze(),
                &metadata,
                Some(self.timeout),
                self.content_type,
            )?;

            let resp = tokio::time::timeout(self.timeout, self.client.request(http_req))
                .await
                .map_err(|_| GrpcEgressError::Timeout("stream request deadline exceeded".into()))?
                .map_err(|e| {
                    tracing::warn!(error = %e, "hyper transport error during gRPC stream");
                    GrpcEgressError::ConnectionFailed("transport error".into())
                })?;

            let response_encoding = TonicGrpcEgressProtocol::read_grpc_encoding(resp.headers());
            // Check grpc-status in the initial response headers first.
            TonicGrpcEgressProtocol::check_grpc_status(Some(resp.headers()), Some(resp.headers()))?;

            let collected = resp.into_body().collect().await.map_err(|e| {
                tracing::warn!(error = %e, "failed to read gRPC stream response body");
                GrpcEgressError::Internal(SANITIZED_INTERNAL_MSG.into())
            })?;

            // Also check trailers.
            TonicGrpcEgressProtocol::check_grpc_status(collected.trailers(), None)?;

            let data = collected.to_bytes();
            let frames = TonicGrpcEgressProtocol::decode_grpc_frames(
                data,
                response_encoding.as_deref(),
                self.max_message_bytes,
            )?;
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
        let body_bytes =
            match TonicGrpcEgressProtocol::encode_grpc_frame(&request.body, self.compression) {
                Ok(b) => b,
                Err(e) => return Box::pin(futures::future::ready(Err(e))),
            };
        let deadline = request.deadline;
        let cancel = request.cancellation.clone();
        let max_bytes = self.max_message_bytes;
        let http_req = match TonicGrpcEgressProtocol::build_http_request(
            &uri_str,
            body_bytes,
            &request.metadata,
            Some(deadline),
            self.content_type,
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
            let response_encoding = TonicGrpcEgressProtocol::read_grpc_encoding(&response_headers);
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
            let frames = TonicGrpcEgressProtocol::decode_grpc_frames(
                data,
                response_encoding.as_deref(),
                self.max_message_bytes,
            )?;
            let items: Vec<GrpcEgressResult<Vec<u8>>> = frames.into_iter().map(Ok).collect();
            Ok(GrpcMessageStreamResponse {
                stream: Box::pin(futures::stream::iter(items)),
            })
        })
    }

    /// Send a client-streaming request — multiple request frames, single response.
    fn call_client_stream(
        &self,
        req: grpc_egress_transport::CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        let grpc_egress_transport::CallStreamRequest {
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
                body_buf.put(TonicGrpcEgressProtocol::encode_grpc_frame(
                    &payload,
                    self.compression,
                )?);
            }

            let http_req = TonicGrpcEgressProtocol::build_http_request(
                &uri_str,
                body_buf.freeze(),
                &metadata,
                Some(deadline),
                self.content_type,
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
            let response_encoding = TonicGrpcEgressProtocol::read_grpc_encoding(&response_headers);
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
            let body = TonicGrpcEgressProtocol::read_unary_message(
                data,
                response_encoding.as_deref(),
                max_bytes,
            )?;

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
        req: grpc_egress_transport::CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        self.call_stream(req)
    }

    fn health_check(
        &self,
        _req: grpc_egress_transport::HealthCheckRequest,
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

// ── unit tests ────────────────────────────────────────────────────────────────
