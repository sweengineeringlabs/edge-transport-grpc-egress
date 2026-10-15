#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for `GrpcEgress` (via `create_transport_from_config`).
//!
//! Tests spin up a minimal in-process HTTP/2 echo server using `hyper_util`.

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::stream;
use http_body::Frame;
use http_body_util::{BodyExt as _, Full, StreamBody};

use edge_security_runtime::SecurityContext;
use edge_transport_grpc_egress_transport::{
    CallStreamRequest, CallUnaryWithContextRequest, GrpcChannelConfig, GrpcEgress, GrpcEgressError,
    GrpcMessageStreamResponse, GrpcRequest, GrpcResponse, GrpcStatusCode, HealthCheckRequest,
    TransportConstruction,
};

fn make_client(addr: SocketAddr) -> Arc<dyn GrpcEgress> {
    TransportConstruction::create_transport_from_config(
        &GrpcChannelConfig::new(format!("http://{addr}")).allow_plaintext(),
    )
    .expect("create_transport_from_config")
}

// ── gRPC frame helpers (duplicated here to keep test self-contained) ─────────

fn encode_frame(payload: &[u8]) -> Bytes {
    let mut buf = BytesMut::with_capacity(5 + payload.len());
    buf.put_u8(0x00);
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    buf.freeze()
}

fn decode_frames(mut data: Bytes) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    while data.len() >= 5 {
        let len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
        data.advance(5);
        if data.len() < len {
            break;
        }
        out.push(data[..len].to_vec());
        data.advance(len);
    }
    out
}

// ── test server helpers ───────────────────────────────────────────────────────

/// Bind to an OS-assigned port and return the address.
async fn bind_listener() -> tokio::net::TcpListener {
    tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test listener")
}

/// Spawn an echo gRPC server that reflects every request frame back as a response frame.
///
/// Returns the bound `SocketAddr`.
async fn spawn_echo_server(listener: tokio::net::TcpListener) -> SocketAddr {
    let addr = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };

            tokio::spawn(async move {
                let io = hyper_util::rt::TokioIo::new(stream);
                let _ =
                    hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                        .serve_connection(
                            io,
                            hyper::service::service_fn(
                                |req: http::Request<hyper::body::Incoming>| async {
                                    // Collect body bytes.
                                    let collected = req.into_body().collect().await.unwrap();
                                    let body_bytes = collected.to_bytes();

                                    // Decode frames and re-encode the payloads as response frames.
                                    let frames = decode_frames(body_bytes);
                                    let mut resp_buf = BytesMut::new();
                                    for f in frames {
                                        resp_buf.put(encode_frame(&f));
                                    }

                                    let resp = http::Response::builder()
                                        .status(200)
                                        .header(http::header::CONTENT_TYPE, "application/grpc")
                                        .header("grpc-status", "0")
                                        .body(Full::new(resp_buf.freeze()))
                                        .unwrap();

                                    Ok::<_, Infallible>(resp)
                                },
                            ),
                        )
                        .await;
            });
        }
    });

    addr
}

/// Spawn a server that always responds with grpc-status=13 (Internal).
async fn spawn_error_server(listener: tokio::net::TcpListener) -> SocketAddr {
    let addr = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let io = hyper_util::rt::TokioIo::new(stream);
                let _ =
                    hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                        .serve_connection(
                            io,
                            hyper::service::service_fn(
                                |_req: http::Request<hyper::body::Incoming>| async {
                                    let resp = http::Response::builder()
                                        .status(200)
                                        .header(http::header::CONTENT_TYPE, "application/grpc")
                                        .header("grpc-status", "13")
                                        .header("grpc-message", "server-side error")
                                        .body(Full::new(Bytes::new()))
                                        .unwrap();
                                    Ok::<_, Infallible>(resp)
                                },
                            ),
                        )
                        .await;
            });
        }
    });

    addr
}

/// Spawn a gRPC server that accepts HTTP/2 connections but never sends a response.
/// Used to verify that the client's timeout fires correctly.
async fn spawn_stalling_grpc_server(listener: tokio::net::TcpListener) -> SocketAddr {
    let addr = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let io = hyper_util::rt::TokioIo::new(stream);
                let _ =
                    hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                        .serve_connection(
                            io,
                            hyper::service::service_fn(
                                |_req: http::Request<hyper::body::Incoming>| async {
                                    // Complete the HTTP/2 handshake but never return a response.
                                    tokio::time::sleep(Duration::from_secs(60)).await;
                                    Ok::<_, Infallible>(
                                        http::Response::builder()
                                            .status(200)
                                            .body(http_body_util::Full::new(bytes::Bytes::new()))
                                            .unwrap(),
                                    )
                                },
                            ),
                        )
                        .await;
            });
        }
    });

    addr
}

/// Spawn a server that echoes request frames back and sends custom metadata as HTTP/2 trailers.
async fn spawn_metadata_server(listener: tokio::net::TcpListener) -> SocketAddr {
    let addr = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let io = hyper_util::rt::TokioIo::new(stream);
                let _ =
                    hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                        .serve_connection(
                            io,
                            hyper::service::service_fn(
                                |req: http::Request<hyper::body::Incoming>| async {
                                    let collected = req.into_body().collect().await.unwrap();
                                    let body_bytes = collected.to_bytes();
                                    let frames = decode_frames(body_bytes);
                                    let mut resp_buf = BytesMut::new();
                                    for f in &frames {
                                        resp_buf.put(encode_frame(f));
                                    }

                                    let mut trailers = http::HeaderMap::new();
                                    trailers
                                        .insert("grpc-status", http::HeaderValue::from_static("0"));
                                    trailers.insert(
                                        "x-response-id",
                                        http::HeaderValue::from_static("meta-42"),
                                    );

                                    let body = StreamBody::new(futures::stream::iter(vec![
                                        Ok::<Frame<Bytes>, Infallible>(Frame::data(
                                            resp_buf.freeze(),
                                        )),
                                        Ok(Frame::trailers(trailers)),
                                    ]));

                                    let resp = http::Response::builder()
                                        .status(200)
                                        .header(http::header::CONTENT_TYPE, "application/grpc")
                                        .body(body.boxed())
                                        .unwrap();

                                    Ok::<_, Infallible>(resp)
                                },
                            ),
                        )
                        .await;
            });
        }
    });

    addr
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// Install a rustls CryptoProvider exactly once per test process.  In a
/// workspace where multiple crates pull both `aws-lc-rs` and `ring`, rustls
/// 0.23 refuses to auto-select; tests that construct a `TonicGrpcEgress`
/// must call this first so hyper-rustls can build its connector.
fn ensure_rustls_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

/// @covers: TonicGrpcEgress::call_unary — happy path echo.
#[tokio::test]
async fn transport_struct_call_unary_sends_request_and_receives_response() {
    ensure_rustls_provider();
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    let client = make_client(addr);
    let req = GrpcRequest::new("echo/Echo", b"hello".to_vec(), Duration::from_secs(5));

    let resp = client
        .call_unary(req)
        .await
        .expect("call_unary should succeed");
    assert_eq!(resp.body, b"hello");
}

/// @covers: TonicGrpcEgress::call_unary — grpc-status 13 maps to Status(Internal, _).
#[tokio::test]
async fn transport_struct_call_unary_propagates_grpc_error_status() {
    let listener = bind_listener().await;
    let addr = spawn_error_server(listener).await;

    ensure_rustls_provider();
    let client = make_client(addr);
    let req = GrpcRequest::new("echo/Echo", b"ping".to_vec(), Duration::from_secs(5));

    let result = client.call_unary(req).await;
    assert!(result.is_err(), "expected Err for grpc-status 13, got Ok");
    match result.unwrap_err() {
        GrpcEgressError::Status(GrpcStatusCode::Internal, msg) => {
            // server-side message comes through grpc-message verbatim
            assert!(
                msg.contains("server-side error"),
                "message should match grpc-message header, got: {msg}"
            );
        }
        other => panic!("expected Status(Internal, _), got {other:?}"),
    }
}

/// @covers: TonicGrpcEgress::call_stream — multiple frames echoed back.
#[tokio::test]
async fn test_call_stream_multiple_frames_happy() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    ensure_rustls_provider();
    let client = make_client(addr);
    let messages = GrpcMessageStreamResponse {
        stream: Box::pin(stream::iter(vec![
            Ok(b"frame1".to_vec()),
            Ok(b"frame2".to_vec()),
            Ok(b"frame3".to_vec()),
        ])),
    };

    let resp_stream = client
        .call_stream(CallStreamRequest {
            method: "echo/Echo".into(),
            metadata: HashMap::new(),
            messages,
        })
        .await
        .expect("call_stream should succeed");
    let mut resp_stream = resp_stream.stream;

    let mut items: Vec<Vec<u8>> = Vec::new();
    while let Some(item) = {
        use futures::StreamExt as _;
        resp_stream.next().await
    } {
        items.push(item.expect("stream item should be Ok"));
    }

    assert_eq!(items.len(), 3, "expected 3 frames, got {}", items.len());
    assert_eq!(items[0], b"frame1");
    assert_eq!(items[1], b"frame2");
    assert_eq!(items[2], b"frame3");
}

/// @covers: call_stream
#[tokio::test]
async fn test_call_stream_default_impl_unimplemented_error() {
    // `StubEgress` does not override `call_stream`, so this exercises the
    // trait's own default body — proving it fails closed with `Unimplemented`
    // rather than panicking or hanging when a `GrpcEgress` impl opts out of
    // streaming support.
    let messages = GrpcMessageStreamResponse {
        stream: Box::pin(stream::iter(Vec::<Result<Vec<u8>, GrpcEgressError>>::new())),
    };
    let result = StubEgress
        .call_stream(CallStreamRequest {
            method: "echo/Echo".into(),
            metadata: HashMap::new(),
            messages,
        })
        .await;
    assert!(
        matches!(
            &result,
            Err(GrpcEgressError::Status(GrpcStatusCode::Unimplemented, _))
        ),
        "expected Status(Unimplemented, _), got {:?}",
        result.err()
    );
}

/// @covers: call_stream
#[tokio::test]
async fn test_call_stream_empty_message_list_edge() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    ensure_rustls_provider();
    let client = make_client(addr);
    let messages = GrpcMessageStreamResponse {
        stream: Box::pin(stream::iter(Vec::<Result<Vec<u8>, GrpcEgressError>>::new())),
    };

    let resp_stream = client
        .call_stream(CallStreamRequest {
            method: "echo/Echo".into(),
            metadata: HashMap::new(),
            messages,
        })
        .await
        .expect("call_stream should succeed even with zero request frames");
    let mut resp_stream = resp_stream.stream;

    use futures::StreamExt as _;
    assert!(
        resp_stream.next().await.is_none(),
        "an empty request stream must yield an empty response stream"
    );
}

/// @covers: TonicGrpcEgress::health_check — succeeds when server is listening.
#[tokio::test]
async fn test_health_check_server_listening_happy() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    ensure_rustls_provider();
    let client = make_client(addr);
    client
        .health_check(HealthCheckRequest)
        .await
        .expect("health_check should succeed when port is open");
}

/// @covers: TonicGrpcEgress::health_check — fails when nothing is listening.
#[tokio::test]
async fn test_health_check_no_server_listening_error() {
    // Bind to get an OS-assigned port, then drop the listener so nothing is listening on it.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    drop(listener);

    ensure_rustls_provider();
    let client = make_client(addr);
    let result = client.health_check(HealthCheckRequest).await;
    assert!(
        result.is_err(),
        "expected Err when nothing is listening, got Ok"
    );
    match result.unwrap_err() {
        GrpcEgressError::Unavailable(msg) => {
            assert!(
                msg.contains(&addr.port().to_string()),
                "error should mention port, got: {msg}"
            );
        }
        other => panic!("expected Unavailable error, got {other:?}"),
    }
}

/// @covers: health_check
#[tokio::test]
async fn test_health_check_repeated_calls_are_idempotent_edge() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    ensure_rustls_provider();
    let client = make_client(addr);
    client
        .health_check(HealthCheckRequest)
        .await
        .expect("first health_check call");
    client
        .health_check(HealthCheckRequest)
        .await
        .expect("second health_check call must not be affected by the first");
}

// ── value-object regression tests (kept from original file) ──────────────────

/// @covers: GrpcRequest::new — three-argument construction with deadline.
#[test]
fn transport_struct_grpc_request_holds_method_and_body_int_test() {
    let req = GrpcRequest::new("svc/Method", vec![1, 2, 3], Duration::from_secs(1));
    assert_eq!(req.method, "svc/Method");
    assert_eq!(req.body, vec![1, 2, 3]);
    assert_eq!(req.deadline, Duration::from_secs(1));
}

/// @covers: GrpcStatusCode — distinct variants.
#[test]
fn transport_struct_grpc_status_code_ok_is_distinct_from_internal_int_test() {
    assert_ne!(GrpcStatusCode::Ok, GrpcStatusCode::Internal);
}

/// @covers: GrpcResponse — struct construction.
#[test]
fn transport_struct_grpc_response_holds_body_bytes_int_test() {
    let resp = GrpcResponse {
        body: vec![0x08, 0x01],
        metadata: HashMap::new(),
    };
    assert_eq!(resp.body, vec![0x08, 0x01]);
}

/// @covers: TonicGrpcEgress::call_unary — response metadata from HTTP/2 trailers is preserved.
#[tokio::test]
async fn transport_struct_call_unary_receives_response_metadata_from_trailers() {
    let listener = bind_listener().await;
    let addr = spawn_metadata_server(listener).await;

    ensure_rustls_provider();
    let client = make_client(addr);
    let req = GrpcRequest::new("echo/Echo", b"hi".to_vec(), Duration::from_secs(5));

    let resp = client
        .call_unary(req)
        .await
        .expect("call_unary should succeed");
    assert_eq!(resp.body, b"hi", "body must be echoed");
    assert_eq!(
        resp.metadata.get("x-response-id").map(String::as_str),
        Some("meta-42"),
        "x-response-id trailer must be present in response metadata; got: {:?}",
        resp.metadata
    );
}

/// @covers: TonicGrpcEgress::call_unary — timeout fires when server does not respond.
#[tokio::test]
async fn transport_struct_call_unary_returns_timeout_error_when_server_stalls() {
    let listener = bind_listener().await;
    let addr = spawn_stalling_grpc_server(listener).await;

    // Per-call deadline drives the timeout for unary now.
    ensure_rustls_provider();
    let client = make_client(addr);
    let req = GrpcRequest::new("svc/Method", b"ping".to_vec(), Duration::from_millis(80));
    let result = client.call_unary(req).await;
    assert!(
        matches!(result, Err(GrpcEgressError::Timeout(_))),
        "expected Timeout, got {result:?}"
    );
}

/// @covers: TonicGrpcEgress::call_unary — ConnectionFailed when nothing listens on the port.
#[tokio::test]
async fn transport_struct_call_unary_returns_connection_failed_when_no_server_is_listening() {
    let addr = {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let a = l.local_addr().unwrap();
        drop(l);
        a
    };

    ensure_rustls_provider();
    let client = make_client(addr);
    let req = GrpcRequest::new("svc/Method", b"ping".to_vec(), Duration::from_secs(5));
    let result = client.call_unary(req).await;
    assert!(
        matches!(result, Err(GrpcEgressError::ConnectionFailed(_))),
        "expected ConnectionFailed, got {result:?}"
    );
}

// ── Phase 1 enrichment: deadlines, cancellation, status mapping ──────────────

use std::sync::Mutex;

/// Echo server that records the `grpc-timeout` header value of the most recent
/// request, so tests can assert that the egress side really sent it.
async fn spawn_timeout_recording_server(
    listener: tokio::net::TcpListener,
) -> (SocketAddr, Arc<Mutex<Option<String>>>) {
    let addr = listener.local_addr().expect("local_addr");
    let captured = Arc::new(Mutex::new(None::<String>));
    let cap = captured.clone();
    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };
            let cap = cap.clone();
            tokio::spawn(async move {
                let io = hyper_util::rt::TokioIo::new(stream);
                let _ =
                    hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
                        .serve_connection(
                            io,
                            hyper::service::service_fn(
                                move |req: http::Request<hyper::body::Incoming>| {
                                    let cap = cap.clone();
                                    async move {
                                        if let Some(v) = req.headers().get("grpc-timeout") {
                                            if let Ok(s) = v.to_str() {
                                                *cap.lock().unwrap() = Some(s.to_owned());
                                            }
                                        }
                                        let collected = req.into_body().collect().await.unwrap();
                                        let body_bytes = collected.to_bytes();
                                        let frames = decode_frames(body_bytes);
                                        let mut buf = BytesMut::new();
                                        for f in frames {
                                            buf.put(encode_frame(&f));
                                        }
                                        let resp = http::Response::builder()
                                            .status(200)
                                            .header(http::header::CONTENT_TYPE, "application/grpc")
                                            .header("grpc-status", "0")
                                            .body(Full::new(buf.freeze()))
                                            .unwrap();
                                        Ok::<_, Infallible>(resp)
                                    }
                                },
                            ),
                        )
                        .await;
            });
        }
    });
    (addr, captured)
}

/// @covers: TonicGrpcEgress::call_unary — request carries `grpc-timeout` header
/// derived from the GrpcRequest deadline.
#[tokio::test]
async fn transport_struct_call_unary_sends_grpc_timeout_header_from_deadline() {
    let listener = bind_listener().await;
    let (addr, captured) = spawn_timeout_recording_server(listener).await;

    ensure_rustls_provider();
    let client = make_client(addr);
    let req = GrpcRequest::new("svc/Method", b"x".to_vec(), Duration::from_secs(2));
    client
        .call_unary(req)
        .await
        .expect("call_unary should succeed");

    let header = captured.lock().unwrap().clone();
    let header = header.expect("server must have observed a grpc-timeout header");
    // The encoder may pick any unit; just verify it ends with one of the standard suffixes
    // and parses to a non-empty positive value.
    let last = header.chars().last().expect("non-empty header");
    assert!(
        "nuMSmH".contains(last),
        "unexpected unit suffix in {header}"
    );
    let prefix = &header[..header.len() - 1];
    let value: u64 = prefix.parse().unwrap_or(0);
    assert!(
        value > 0,
        "grpc-timeout value must be positive, got {header}"
    );
}

/// @covers: TonicGrpcEgress::call_unary — caller-supplied cancellation token aborts
/// the in-flight request and yields `Cancelled` rather than `Timeout`.
#[tokio::test]
async fn transport_struct_call_unary_cancellation_token_aborts_in_flight_request() {
    use tokio_util::sync::CancellationToken;

    let listener = bind_listener().await;
    let addr = spawn_stalling_grpc_server(listener).await;

    // Long deadline so timeout doesn't beat us; cancellation must win.
    ensure_rustls_provider();
    let client = make_client(addr);
    let token = CancellationToken::new();
    let req = GrpcRequest::new("svc/Method", b"x".to_vec(), Duration::from_secs(60))
        .with_cancellation(token.clone());

    // Fire the cancel after a brief wait so the request is actually in flight.
    let cancel_handle = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        token.cancel();
    });

    let result = client.call_unary(req).await;
    cancel_handle.await.unwrap();

    match result {
        Err(GrpcEgressError::Cancelled(msg)) => {
            assert!(
                msg.to_lowercase().contains("cancel"),
                "Cancelled message should mention cancellation, got: {msg}"
            );
        }
        other => panic!("expected Cancelled, got {other:?}"),
    }
}

/// @covers: GrpcEgressError::Status — all 17 GrpcStatusCode variants round-trip
/// through the public error type without information loss.
#[test]
fn transport_struct_status_error_round_trips_all_17_grpc_status_code_variants_int_test() {
    let all_17 = [
        GrpcStatusCode::Ok,
        GrpcStatusCode::Cancelled,
        GrpcStatusCode::Unknown,
        GrpcStatusCode::InvalidArgument,
        GrpcStatusCode::DeadlineExceeded,
        GrpcStatusCode::NotFound,
        GrpcStatusCode::AlreadyExists,
        GrpcStatusCode::PermissionDenied,
        GrpcStatusCode::ResourceExhausted,
        GrpcStatusCode::FailedPrecondition,
        GrpcStatusCode::Aborted,
        GrpcStatusCode::OutOfRange,
        GrpcStatusCode::Unimplemented,
        GrpcStatusCode::Internal,
        GrpcStatusCode::Unavailable,
        GrpcStatusCode::DataLoss,
        GrpcStatusCode::Unauthenticated,
    ];
    assert_eq!(all_17.len(), 17);
    for code in all_17 {
        let err = GrpcEgressError::Status(code, "msg".into());
        match err {
            GrpcEgressError::Status(c, m) => {
                assert_eq!(c, code, "code lost in round trip for {code:?}");
                assert_eq!(m, "msg");
            }
            other => panic!("expected Status, got {other:?}"),
        }
    }
}

/// @covers: GrpcEgress::call_unary_with_context — default impl delegates to call_unary;
/// response body matches the echoed request payload.
///
/// Failure mode: if `call_unary_with_context` drops the request or returns a different
/// body, the `assert_eq!` on `resp.body` will fail.
#[tokio::test]
async fn test_call_unary_with_context_echoes_body_happy() {
    ensure_rustls_provider();
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    let client = make_client(addr);
    let req = GrpcRequest::new("echo/Echo", b"ctx-payload".to_vec(), Duration::from_secs(5));
    let ctx = SecurityContext::unauthenticated();

    let resp = client
        .call_unary_with_context(CallUnaryWithContextRequest { request: req, ctx })
        .await
        .expect("call_unary_with_context should succeed against echo server");

    assert_eq!(
        resp.body, b"ctx-payload",
        "call_unary_with_context must echo the request body; got {:?}",
        resp.body
    );
}

/// @covers: GrpcEgress::call_unary_with_context — authenticated context is accepted without error.
///
/// Failure mode: if the context type is incompatible or the method panics on a
/// non-unauthenticated context, this test will fail.
#[tokio::test]
async fn transport_struct_call_unary_with_context_accepts_authenticated_context() {
    ensure_rustls_provider();
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    let client = make_client(addr);
    let req = GrpcRequest::new(
        "echo/Echo",
        b"auth-payload".to_vec(),
        Duration::from_secs(5),
    );
    let ctx = SecurityContext::unauthenticated()
        .with_trace_id("trace-abc-123")
        .with_claim("role", "admin");

    let resp = client
        .call_unary_with_context(CallUnaryWithContextRequest { request: req, ctx })
        .await
        .expect("call_unary_with_context must succeed with authenticated-like context");

    assert_eq!(
        resp.body, b"auth-payload",
        "response body must match the request payload; got {:?}",
        resp.body
    );
}

/// @covers: GrpcEgress::call_unary_with_context — propagates error from call_unary
/// when the server returns grpc-status 13 (Internal).
///
/// Failure mode: if `call_unary_with_context` swallows the error instead of
/// forwarding the `call_unary` result, this test will fail.
#[tokio::test]
async fn test_call_unary_with_context_propagates_grpc_error_status_error() {
    ensure_rustls_provider();
    let listener = bind_listener().await;
    let addr = spawn_error_server(listener).await;

    let client = make_client(addr);
    let req = GrpcRequest::new("echo/Echo", b"ping".to_vec(), Duration::from_secs(5));
    let ctx = SecurityContext::unauthenticated();

    let result = client
        .call_unary_with_context(CallUnaryWithContextRequest { request: req, ctx })
        .await;
    assert!(
        result.is_err(),
        "call_unary_with_context must propagate grpc-status 13 as Err; got Ok"
    );
    match result.unwrap_err() {
        GrpcEgressError::Status(GrpcStatusCode::Internal, msg) => {
            assert!(
                msg.contains("server-side error"),
                "error message must include grpc-message header content; got: {msg}"
            );
        }
        other => panic!("expected Status(Internal, _), got {other:?}"),
    }
}

/// @covers: GrpcEgress::call_unary_with_context — timeout fires when server does not respond.
///
/// Failure mode: if `call_unary_with_context` bypasses the deadline from the request,
/// this test will hang past the deadline and eventually return Ok instead of Timeout.
#[tokio::test]
async fn test_call_unary_with_context_timeout_edge() {
    ensure_rustls_provider();
    let listener = bind_listener().await;
    let addr = spawn_stalling_grpc_server(listener).await;

    let client = make_client(addr);
    let req = GrpcRequest::new("svc/Method", b"ping".to_vec(), Duration::from_millis(80));
    let ctx = SecurityContext::unauthenticated();

    let result = client
        .call_unary_with_context(CallUnaryWithContextRequest { request: req, ctx })
        .await;
    assert!(
        matches!(result, Err(GrpcEgressError::Timeout(_))),
        "call_unary_with_context must propagate Timeout from call_unary; got {result:?}"
    );
}

/// @covers: GrpcEgress::call_unary_with_context — ConnectionFailed when nothing listens.
///
/// Failure mode: if `call_unary_with_context` doesn't forward connection errors,
/// we'd get Ok or a different error variant instead.
#[tokio::test]
async fn transport_struct_call_unary_with_context_returns_connection_failed_when_no_server_is_listening(
) {
    let addr = {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let a = l.local_addr().unwrap();
        drop(l);
        a
    };

    ensure_rustls_provider();
    let client = make_client(addr);
    let req = GrpcRequest::new("svc/Method", b"ping".to_vec(), Duration::from_secs(5));
    let ctx = SecurityContext::unauthenticated();

    let result = client
        .call_unary_with_context(CallUnaryWithContextRequest { request: req, ctx })
        .await;
    assert!(
        matches!(result, Err(GrpcEgressError::ConnectionFailed(_))),
        "call_unary_with_context must propagate ConnectionFailed; got {result:?}"
    );
}

/// @covers: TonicGrpcEgress::call_unary — sanitized message is on the wire when
/// the local client encounters an unexpected internal condition.  We inject one
/// by feeding a server that returns a malformed (truncated) gRPC frame in the
/// response body.
#[tokio::test]
async fn transport_struct_call_unary_sanitizes_internal_error_message_on_truncated_response() {
    use std::convert::Infallible;
    let listener = bind_listener().await;
    let addr = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let io = hyper_util::rt::TokioIo::new(stream);
        let _ = hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
            .serve_connection(
                io,
                hyper::service::service_fn(|_req: http::Request<hyper::body::Incoming>| async {
                    // Frame header claims length=100 but only 3 bytes follow → truncated frame.
                    let mut buf = BytesMut::new();
                    buf.put_u8(0x00);
                    buf.put_u32(100);
                    buf.put_slice(b"abc");
                    let resp = http::Response::builder()
                        .status(200)
                        .header(http::header::CONTENT_TYPE, "application/grpc")
                        .header("grpc-status", "0")
                        .body(Full::new(buf.freeze()))
                        .unwrap();
                    Ok::<_, Infallible>(resp)
                }),
            )
            .await;
    });

    ensure_rustls_provider();
    let client = make_client(addr);
    // call_stream is the path that decodes frames and surfaces the truncation.
    let messages = GrpcMessageStreamResponse {
        stream: Box::pin(stream::iter(vec![Ok(b"x".to_vec())])),
    };
    let result = client
        .call_stream(CallStreamRequest {
            method: "svc/Method".into(),
            metadata: HashMap::new(),
            messages,
        })
        .await;
    let err = match result {
        Ok(_) => panic!("expected an error from a truncated server response, got Ok"),
        Err(e) => e,
    };
    match err {
        GrpcEgressError::Internal(msg) => {
            // Sanitized: must NOT contain raw byte counts, file names, or struct names.
            assert!(
                !msg.contains("expected") && !msg.contains("got"),
                "internal error message leaked details: {msg}"
            );
            assert!(!msg.is_empty(), "sanitized message must be non-empty");
        }
        other => panic!("expected sanitized Internal error, got {other:?}"),
    }
}

// ── Named streaming variants ──────────────────────────────────────────────────

/// @covers: call_server_stream
#[tokio::test]
async fn test_call_server_stream_single_request_happy() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    ensure_rustls_provider();
    let client = make_client(addr);
    let req = GrpcRequest::new(
        "svc/ServerStream".to_string(),
        b"hello".to_vec(),
        std::time::Duration::from_secs(5),
    );
    let mut stream = client
        .call_server_stream(req)
        .await
        .expect("call_server_stream must succeed against echo server")
        .stream;

    use futures::StreamExt;
    let frame = stream
        .next()
        .await
        .expect("stream must yield at least one item")
        .expect("frame must be Ok");
    assert_eq!(
        frame, b"hello",
        "echo server must reflect the request payload"
    );
    assert!(
        stream.next().await.is_none(),
        "echo server yields one frame per request"
    );
}

/// @covers: call_server_stream
#[tokio::test]
async fn test_call_server_stream_default_impl_unimplemented_error() {
    // `StubEgress` does not override `call_server_stream`, exercising the
    // trait's own default body.
    let req = GrpcRequest::new(
        "svc/ServerStream",
        b"hello".to_vec(),
        Duration::from_secs(5),
    );
    let result = StubEgress.call_server_stream(req).await;
    assert!(
        matches!(
            &result,
            Err(GrpcEgressError::Status(GrpcStatusCode::Unimplemented, _))
        ),
        "expected Status(Unimplemented, _), got {:?}",
        result.err()
    );
}

/// @covers: call_server_stream
#[tokio::test]
async fn test_call_server_stream_empty_body_edge() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    ensure_rustls_provider();
    let client = make_client(addr);
    let req = GrpcRequest::new("svc/ServerStream", Vec::new(), Duration::from_secs(5));
    let mut stream = client
        .call_server_stream(req)
        .await
        .expect("call_server_stream must succeed with an empty request body")
        .stream;

    use futures::StreamExt;
    let frame = stream
        .next()
        .await
        .expect("stream must yield one item")
        .expect("frame must be Ok");
    assert!(
        frame.is_empty(),
        "echo server must reflect the empty request payload, got {frame:?}"
    );
}

/// @covers: call_client_stream
#[tokio::test]
async fn test_call_client_stream_multiple_frames_happy() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    ensure_rustls_provider();
    let client = make_client(addr);
    // Send two frames; the echo server reflects them.  call_client_stream strips
    // the first 5-byte gRPC header and returns the remainder as the response body.
    let messages = GrpcMessageStreamResponse {
        stream: Box::pin(stream::iter(vec![
            Ok(b"frame-1".to_vec()),
            Ok(b"frame-2".to_vec()),
        ])),
    };
    let response = client
        .call_client_stream(CallStreamRequest {
            method: "svc/ClientStream".into(),
            metadata: HashMap::new(),
            messages,
        })
        .await
        .expect("call_client_stream must succeed against echo server");

    // The response body starts with the payload of the first echoed frame.
    assert!(
        response.body.starts_with(b"frame-1"),
        "response body must start with first echoed frame payload, got {:?}",
        response.body
    );
}

/// @covers: call_client_stream
#[tokio::test]
async fn test_call_client_stream_default_impl_unimplemented_error() {
    // `StubEgress` does not override `call_client_stream`, exercising the
    // trait's own default body.
    let messages = GrpcMessageStreamResponse {
        stream: Box::pin(stream::iter(vec![Ok(b"frame".to_vec())])),
    };
    let result = StubEgress
        .call_client_stream(CallStreamRequest {
            method: "svc/ClientStream".into(),
            metadata: HashMap::new(),
            messages,
        })
        .await;
    assert!(
        matches!(
            &result,
            Err(GrpcEgressError::Status(GrpcStatusCode::Unimplemented, _))
        ),
        "expected Status(Unimplemented, _), got {:?}",
        result.err()
    );
}

/// @covers: call_client_stream
#[tokio::test]
async fn test_call_client_stream_single_frame_edge() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    ensure_rustls_provider();
    let client = make_client(addr);
    let messages = GrpcMessageStreamResponse {
        stream: Box::pin(stream::iter(vec![Ok(b"only-frame".to_vec())])),
    };
    let response = client
        .call_client_stream(CallStreamRequest {
            method: "svc/ClientStream".into(),
            metadata: HashMap::new(),
            messages,
        })
        .await
        .expect("call_client_stream must succeed with exactly one request frame");
    assert!(
        response.body.starts_with(b"only-frame"),
        "response body must start with the single echoed frame payload, got {:?}",
        response.body
    );
}

/// @covers: call_bidi_stream
#[tokio::test]
async fn test_call_bidi_stream_multiple_frames_happy() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    ensure_rustls_provider();
    let client = make_client(addr);
    let messages = GrpcMessageStreamResponse {
        stream: Box::pin(stream::iter(vec![
            Ok(b"alpha".to_vec()),
            Ok(b"beta".to_vec()),
        ])),
    };
    let mut stream = client
        .call_bidi_stream(CallStreamRequest {
            method: "svc/Bidi".into(),
            metadata: HashMap::new(),
            messages,
        })
        .await
        .expect("call_bidi_stream must succeed against echo server")
        .stream;

    use futures::StreamExt;
    let frame1 = stream
        .next()
        .await
        .expect("stream must yield frame 1")
        .expect("Ok");
    let frame2 = stream
        .next()
        .await
        .expect("stream must yield frame 2")
        .expect("Ok");
    assert_eq!(frame1, b"alpha");
    assert_eq!(frame2, b"beta");
    assert!(
        stream.next().await.is_none(),
        "stream must be exhausted after 2 frames"
    );
}

/// @covers: call_bidi_stream
#[tokio::test]
async fn test_call_bidi_stream_default_impl_unimplemented_error() {
    // `StubEgress` does not override `call_bidi_stream` or `call_stream` (the
    // default `call_bidi_stream` body delegates to `call_stream`), so this
    // exercises both defaults chained together.
    let messages = GrpcMessageStreamResponse {
        stream: Box::pin(stream::iter(Vec::<Result<Vec<u8>, GrpcEgressError>>::new())),
    };
    let result = StubEgress
        .call_bidi_stream(CallStreamRequest {
            method: "svc/Bidi".into(),
            metadata: HashMap::new(),
            messages,
        })
        .await;
    assert!(
        matches!(
            &result,
            Err(GrpcEgressError::Status(GrpcStatusCode::Unimplemented, _))
        ),
        "expected Status(Unimplemented, _), got {:?}",
        result.err()
    );
}

/// @covers: call_bidi_stream
#[tokio::test]
async fn test_call_bidi_stream_empty_frame_list_edge() {
    let listener = bind_listener().await;
    let addr = spawn_echo_server(listener).await;

    ensure_rustls_provider();
    let client = make_client(addr);
    let messages = GrpcMessageStreamResponse {
        stream: Box::pin(stream::iter(Vec::<Result<Vec<u8>, GrpcEgressError>>::new())),
    };
    let mut stream = client
        .call_bidi_stream(CallStreamRequest {
            method: "svc/Bidi".into(),
            metadata: HashMap::new(),
            messages,
        })
        .await
        .expect("call_bidi_stream must succeed against echo server even with zero frames")
        .stream;

    use futures::StreamExt;
    assert!(
        stream.next().await.is_none(),
        "an empty request stream must yield an empty response stream"
    );
}

// ── GrpcEgress::describe_compression / default_conversions ──────────────────
//
// `TransportSvc`'s concrete `GrpcEgress` implementor is `pub(crate)` (see SEA
// rule `pub_types_in_api_only`), so these `Self: Sized` default methods are
// exercised here through a minimal test-double instead.

// @allow: no_mocks_in_integration — hand-rolled test double, not a mock library.
struct StubEgress;

impl GrpcEgress for StubEgress {
    fn call_unary(
        &self,
        _request: GrpcRequest,
    ) -> futures::future::BoxFuture<'_, Result<GrpcResponse, GrpcEgressError>> {
        Box::pin(async {
            Ok(GrpcResponse {
                body: Vec::new(),
                metadata: HashMap::new(),
            })
        })
    }

    fn health_check(
        &self,
        _req: HealthCheckRequest,
    ) -> futures::future::BoxFuture<'_, Result<(), GrpcEgressError>> {
        Box::pin(async { Ok(()) })
    }
}

/// @covers: describe_compression
#[test]
fn test_describe_compression_none_is_disabled_happy() {
    use edge_transport_grpc_egress_transport::CompressionMode;
    assert!(!<StubEgress as GrpcEgress>::describe_compression(
        CompressionMode::None
    ));
}

/// @covers: describe_compression
#[test]
fn test_describe_compression_gzip_is_enabled_error() {
    use edge_transport_grpc_egress_transport::CompressionMode;
    assert!(<StubEgress as GrpcEgress>::describe_compression(
        CompressionMode::Gzip
    ));
}

/// @covers: describe_compression
#[test]
fn test_describe_compression_zstd_is_enabled_edge() {
    use edge_transport_grpc_egress_transport::CompressionMode;
    assert!(<StubEgress as GrpcEgress>::describe_compression(
        CompressionMode::Zstd
    ));
}

/// @covers: default_conversions
#[test]
fn test_default_conversions_returns_zero_sized_marker_happy() {
    let marker = <StubEgress as GrpcEgress>::default_conversions();
    assert_eq!(std::mem::size_of_val(&marker), 0);
}

/// @covers: default_conversions
#[test]
fn test_default_conversions_is_the_real_marker_type_error() {
    use edge_transport_grpc_egress_transport::Conversions;
    fn assert_type(marker: Conversions) -> usize {
        std::mem::size_of_val(&marker)
    }
    let size = assert_type(<StubEgress as GrpcEgress>::default_conversions());
    assert_eq!(
        size, 0,
        "the returned value must unify with the genuine zero-sized Conversions, not a look-alike"
    );
}

/// @covers: default_conversions
#[test]
fn test_default_conversions_repeated_calls_are_independent_edge() {
    let a = <StubEgress as GrpcEgress>::default_conversions();
    let b = <StubEgress as GrpcEgress>::default_conversions();
    assert_eq!(std::mem::size_of_val(&a), std::mem::size_of_val(&b));
}
