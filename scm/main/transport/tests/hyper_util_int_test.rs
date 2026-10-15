#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests that directly exercise the `hyper-util` dependency used
//! by `TonicGrpcEgress`'s HTTP/2 transport layer.
//!
//! These tests verify that:
//!   - `hyper_util::rt::TokioExecutor` can drive an HTTP/2 client connection.
//!   - `hyper_util::rt::TokioIo` correctly wraps a `tokio::net::TcpStream`.
//!   - The `hyper_util` client-legacy path (`Client::builder`) is exercised via
//!     a real TCP connection to an in-process HTTP/2 server.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use http_body_util::Full;
use hyper_util::rt::{TokioExecutor, TokioIo};

use edge_transport_grpc_egress_transport::{
    GrpcChannelConfig, GrpcRequest, HealthCheckRequest, TransportConstruction,
};

fn ensure_rustls_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

/// Bind an OS-assigned TCP port and return the `TcpListener`.
async fn bind_listener() -> tokio::net::TcpListener {
    tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test listener")
}

/// Spawn a minimal HTTP/2 server using `hyper_util::rt::TokioIo` and
/// `hyper_util::rt::TokioExecutor` — the exact types from the `hyper-util`
/// crate that `TonicGrpcEgress` relies on — and return the bound address.
async fn spawn_hyper_util_http2_server(listener: tokio::net::TcpListener) -> SocketAddr {
    let addr = listener.local_addr().expect("local_addr");

    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };

            tokio::spawn(async move {
                // Use hyper_util's TokioIo adapter and TokioExecutor directly,
                // proving the hyper-util dependency is exercised in integration tests.
                let io = TokioIo::new(stream);
                let _ = hyper::server::conn::http2::Builder::new(TokioExecutor::new())
                    .serve_connection(
                        io,
                        hyper::service::service_fn(
                            |_req: http::Request<hyper::body::Incoming>| async {
                                // Return a valid gRPC response with status OK.
                                let mut buf = bytes::BytesMut::new();
                                use bytes::BufMut as _;
                                buf.put_u8(0x00); // not compressed
                                buf.put_u32(5_u32);
                                buf.put_slice(b"pong!");
                                let resp = http::Response::builder()
                                    .status(200)
                                    .header(http::header::CONTENT_TYPE, "application/grpc")
                                    .header("grpc-status", "0")
                                    .body(Full::new(buf.freeze()))
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

/// Verify that the `hyper_util` client-legacy path can complete an end-to-end
/// round trip: connect → send gRPC frame → receive gRPC frame → decode status.
///
/// This test exercises `hyper_util::client::legacy::Client` (via the
/// `TonicGrpcEgress` internals) against a server built with
/// `hyper_util::rt::TokioIo` and `hyper_util::rt::TokioExecutor`.
#[tokio::test]
async fn transport_struct_hyper_util_tokio_executor_drives_http2_client_round_trip() {
    ensure_rustls_provider();
    let listener = bind_listener().await;
    let addr = spawn_hyper_util_http2_server(listener).await;

    let cfg = GrpcChannelConfig::new(format!("http://{addr}")).allow_plaintext();
    let client = TransportConstruction::create_transport_from_config(&cfg)
        .expect("create_transport_from_config");

    let req = GrpcRequest::new("svc/Method", b"ping".to_vec(), Duration::from_secs(5));
    let resp = client
        .call_unary(req)
        .await
        .expect("call_unary should succeed via hyper-util HTTP/2 transport");

    assert_eq!(
        resp.body, b"pong!",
        "body must be echoed back through hyper-util layer"
    );
}

/// Verify that `hyper_util::rt::TokioIo` correctly adapts a
/// `tokio::net::TcpStream` so the HTTP/2 handshake completes.  This is a
/// structural test: if `TokioIo` were broken, the server would never accept
/// a connection and `health_check` would time out.
#[tokio::test]
async fn transport_struct_hyper_util_tokio_io_adapts_tcp_stream_for_http2_handshake() {
    ensure_rustls_provider();
    let listener = bind_listener().await;
    let addr = spawn_hyper_util_http2_server(listener).await;

    let cfg = GrpcChannelConfig::new(format!("http://{addr}")).allow_plaintext();
    let client = TransportConstruction::create_transport_from_config(&cfg)
        .expect("create_transport_from_config");

    // health_check opens a TCP connection — proves TokioIo wrapping works.
    client
        .health_check(HealthCheckRequest)
        .await
        .expect("health_check should succeed — TokioIo TCP adapter is functional");
}

/// Verify the hyper-util client-legacy builder connects to an HTTP/2 server
/// and that multiple sequential requests succeed (connection reuse path).
#[tokio::test]
async fn transport_struct_hyper_util_client_legacy_handles_sequential_requests() {
    ensure_rustls_provider();
    let listener = bind_listener().await;
    let addr = spawn_hyper_util_http2_server(listener).await;

    let cfg = GrpcChannelConfig::new(format!("http://{addr}")).allow_plaintext();
    let client = TransportConstruction::create_transport_from_config(&cfg)
        .expect("create_transport_from_config");

    for i in 0..3_u8 {
        let payload = vec![i];
        let req = GrpcRequest::new("svc/Ping", payload, Duration::from_secs(5));
        let result = client.call_unary(req).await;
        assert!(
            result.is_ok(),
            "request {i} failed via hyper-util transport: {:?}",
            result.unwrap_err()
        );
    }
}
