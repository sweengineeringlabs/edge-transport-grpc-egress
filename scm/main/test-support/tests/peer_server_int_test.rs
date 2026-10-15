//! Tests of `PeerServer`: what it answers for each behaviour, what it records, and that it stops
//! when dropped. The requests are made with a raw HTTP/2 client, not with the client under test.

use std::error::Error;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use bytes::Bytes;
use grpc_egress_test_support::{DecodedFrame, GrpcFrame, PeerBehaviour, PeerServer};
use http_body_util::{BodyExt as _, Full};
use hyper::client::conn::http2::handshake;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpStream;

/// Sends one unary request to a peer and returns the response head and body.
struct RawGrpcClient;

impl RawGrpcClient {
    async fn post(
        addr: SocketAddr,
        path: &str,
        headers: &[(&str, &str)],
        body: Bytes,
    ) -> Result<(http::response::Parts, Bytes), Box<dyn Error>> {
        let stream = TcpStream::connect(addr).await?;
        let (mut sender, connection) =
            handshake(TokioExecutor::new(), TokioIo::new(stream)).await?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        let mut builder = http::Request::builder()
            .method("POST")
            .uri(format!("http://{addr}{path}"))
            .header("content-type", "application/grpc");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let response = sender.send_request(builder.body(Full::new(body))?).await?;
        let (parts, incoming) = response.into_parts();
        Ok((parts, incoming.collect().await?.to_bytes()))
    }
}

/// @covers: PeerServer::start
#[tokio::test]
async fn test_start_echo_returns_the_messages_of_the_request_with_status_zero(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let mut body = GrpcFrame::encode(b"one").to_vec();
    body.extend_from_slice(&GrpcFrame::encode(b"two"));

    let (head, reply) =
        RawGrpcClient::post(peer.get_addr(), "/pkg.Service/Method", &[], body.into()).await?;

    assert_eq!(head.status, 200);
    assert_eq!(
        head.headers.get("grpc-status").map(|v| v.as_bytes()),
        Some(&b"0"[..])
    );
    assert_eq!(
        head.headers.get("content-type").map(|v| v.as_bytes()),
        Some(&b"application/grpc"[..])
    );
    assert_eq!(
        GrpcFrame::decode_all(&reply),
        vec![
            DecodedFrame::create(false, b"one".to_vec()),
            DecodedFrame::create(false, b"two".to_vec())
        ]
    );
    Ok(())
}

/// @covers: PeerServer::start
#[tokio::test]
async fn test_start_reply_returns_its_one_message_whatever_the_request_was(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Reply(b"fixed".to_vec())).await?;

    let (_, reply) = RawGrpcClient::post(
        peer.get_addr(),
        "/pkg.Service/Method",
        &[],
        GrpcFrame::encode(b"something else"),
    )
    .await?;

    assert_eq!(
        GrpcFrame::decode_all(&reply),
        vec![DecodedFrame::create(false, b"fixed".to_vec())]
    );
    Ok(())
}

/// @covers: PeerServer::start
#[tokio::test]
async fn test_start_delay_answers_after_the_delay_with_the_echo() -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Delay(Duration::from_millis(300))).await?;

    let started = Instant::now();
    let (_, reply) = RawGrpcClient::post(
        peer.get_addr(),
        "/pkg.Service/Method",
        &[],
        GrpcFrame::encode(b"late"),
    )
    .await?;
    let elapsed = started.elapsed();

    assert!(
        elapsed >= Duration::from_millis(300),
        "answered after {elapsed:?}"
    );
    assert_eq!(
        GrpcFrame::decode_all(&reply),
        vec![DecodedFrame::create(false, b"late".to_vec())]
    );
    Ok(())
}

/// @covers: PeerServer::start
#[tokio::test]
async fn test_start_echo_request_without_messages_returns_an_empty_body_edge(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;

    let (head, reply) =
        RawGrpcClient::post(peer.get_addr(), "/pkg.Service/Method", &[], Bytes::new()).await?;

    assert_eq!(head.status, 200);
    assert!(
        reply.is_empty(),
        "nothing to echo, got {} bytes",
        reply.len()
    );
    Ok(())
}

/// @covers: RunningPeer::get_recorded_requests
#[tokio::test]
async fn test_get_recorded_requests_holds_path_headers_and_messages_in_arrival_order(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;

    RawGrpcClient::post(
        peer.get_addr(),
        "/pkg.Service/First",
        &[("x-test", "yes")],
        GrpcFrame::encode_compressed(b"zz"),
    )
    .await?;
    RawGrpcClient::post(
        peer.get_addr(),
        "/pkg.Service/Second",
        &[],
        GrpcFrame::encode(b"b"),
    )
    .await?;

    let requests = peer.get_recorded_requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].get_path(), "/pkg.Service/First");
    assert_eq!(requests[0].get_header("x-test"), Some("yes"));
    assert_eq!(
        requests[0].get_frames(),
        &[DecodedFrame::create(true, b"zz".to_vec())]
    );
    assert_eq!(requests[1].get_path(), "/pkg.Service/Second");
    assert_eq!(requests[1].get_header("x-test"), None);
    Ok(())
}

/// @covers: RunningPeer::get_addr
#[tokio::test]
async fn test_drop_peer_stops_accepting_connections_with_connection_refused(
) -> Result<(), Box<dyn Error>> {
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let addr = peer.get_addr();

    drop(peer);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let error = TcpStream::connect(addr)
        .await
        .err()
        .ok_or("a dropped peer accepted a connection")?;
    assert_eq!(error.kind(), ErrorKind::ConnectionRefused);
    Ok(())
}
