//! Tests of message compression in the `hyper`-backed client, against a real in-process peer: a
//! compressed message is really compressed on the way out, a compressed answer is really
//! decompressed on the way in, and an answer that cannot be trusted is refused.

use grpc_egress_test_support::ClientFixture;
use grpc_egress_transport_std::{GrpcChannelConfigFactory, GrpcRequestFactory};
use std::collections::HashMap;
use std::error::Error;
use std::io::{Read as _, Write as _};
use std::time::Duration;

use futures::StreamExt as _;
use grpc_egress_test_support::{Fixture, PeerBehaviour, PeerServer};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, CallStreamRequest, CompressionMode, GrpcChannelConfig,
    GrpcEgress, GrpcEgressBuilder, GrpcEgressBuilderFactory, GrpcEgressError,
    GrpcMessageStreamResponse, GrpcRequest, GrpcStatusCode,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;

fn gzip(payload: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(payload)?;
    Ok(encoder.finish()?)
}

fn gunzip(payload: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(payload).read_to_end(&mut out)?;
    Ok(out)
}

fn zstd_compress(payload: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(zstd::stream::encode_all(
        payload,
        zstd::DEFAULT_COMPRESSION_LEVEL,
    )?)
}

fn request(payload: Vec<u8>) -> GrpcRequest {
    GrpcRequest::new("pkg.Service/Method", payload, Duration::from_secs(5))
}

async fn send_with(
    mode: CompressionMode,
    payload: Vec<u8>,
) -> Result<(grpc_egress_test_support::RunningPeer, Vec<u8>), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_compression(mode);
    let client = ClientFixture::from_config(&config)?;
    client.call_unary(request(payload.clone())).await?;
    Ok((peer, payload))
}

async fn call_peer_replying(
    behaviour: PeerBehaviour,
    max_message_bytes: usize,
) -> Result<Result<Vec<u8>, GrpcEgressError>, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(behaviour).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_message_bytes(max_message_bytes)
        .build()?;
    Ok(client
        .call_unary(request(b"ping".to_vec()))
        .await
        .map(|response| response.body))
}

/// @covers: CompressionMode::Gzip
#[tokio::test]
async fn test_call_unary_with_gzip_sends_a_compressed_message_that_unzips_to_the_original(
) -> Result<(), Box<dyn Error>> {
    let (peer, payload) = send_with(CompressionMode::Gzip, vec![b'a'; 4096]).await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("grpc-encoding"), Some("gzip"));
    let frame = request.get_frames().first().ok_or("no message")?;
    assert!(frame.is_compressed(), "the flag must say compressed");
    assert!(
        frame.get_payload().len() < payload.len() / 10,
        "4096 equal bytes must shrink a lot, got {} bytes",
        frame.get_payload().len()
    );
    assert_eq!(gunzip(frame.get_payload())?, payload);
    Ok(())
}

/// @covers: CompressionMode::Zstd
#[tokio::test]
async fn test_call_unary_with_zstd_sends_a_compressed_message_that_decompresses_to_the_original(
) -> Result<(), Box<dyn Error>> {
    let (peer, payload) = send_with(CompressionMode::Zstd, vec![b'a'; 4096]).await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("grpc-encoding"), Some("zstd"));
    let frame = request.get_frames().first().ok_or("no message")?;
    assert!(frame.is_compressed(), "the flag must say compressed");
    assert!(frame.get_payload().len() < payload.len() / 10);
    assert_eq!(
        zstd::stream::decode_all(frame.get_payload())?,
        payload,
        "the wire bytes must be a zstd stream of the original"
    );
    Ok(())
}

/// @covers: CompressionMode::Gzip
#[tokio::test]
async fn test_call_unary_with_gzip_empty_message_round_trips_edge() -> Result<(), Box<dyn Error>> {
    let (peer, _) = send_with(CompressionMode::Gzip, Vec::new()).await?;

    let requests = peer.get_recorded_requests();
    let frame = requests
        .first()
        .and_then(|r| r.get_frames().first())
        .ok_or("no message")?;
    assert!(frame.is_compressed());
    assert_eq!(gunzip(frame.get_payload())?, Vec::<u8>::new());
    Ok(())
}

/// @covers: GrpcEgress::call_unary
#[tokio::test]
async fn test_call_unary_gzip_answer_is_decompressed_happy() -> Result<(), Box<dyn Error>> {
    let original = vec![b'z'; 2048];
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: Some("gzip".into()),
            payload: gzip(&original)?,
        },
        1 << 20,
    )
    .await?;

    assert_eq!(reply?, original);
    Ok(())
}

/// @covers: GrpcEgress::call_unary
#[tokio::test]
async fn test_call_unary_zstd_answer_is_decompressed_happy() -> Result<(), Box<dyn Error>> {
    let original = vec![b'z'; 2048];
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: Some("zstd".into()),
            payload: zstd_compress(&original)?,
        },
        1 << 20,
    )
    .await?;

    assert_eq!(reply?, original);
    Ok(())
}

/// @covers: GrpcEgress::call_unary
#[tokio::test]
async fn test_call_unary_compressed_answer_without_an_announced_encoding_returns_internal_error(
) -> Result<(), Box<dyn Error>> {
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: None,
            payload: gzip(b"hello")?,
        },
        1 << 20,
    )
    .await?;

    assert!(
        matches!(reply, Err(GrpcEgressError::Internal(_))),
        "got {reply:?}"
    );
    Ok(())
}

/// @covers: GrpcEgress::call_unary
#[tokio::test]
async fn test_call_unary_answer_in_an_unsupported_encoding_returns_unimplemented_error(
) -> Result<(), Box<dyn Error>> {
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: Some("br".into()),
            payload: b"whatever".to_vec(),
        },
        1 << 20,
    )
    .await?;

    assert!(
        matches!(
            reply,
            Err(GrpcEgressError::Status(GrpcStatusCode::Unimplemented, _))
        ),
        "got {reply:?}"
    );
    Ok(())
}

/// @covers: GrpcEgress::call_unary
#[tokio::test]
async fn test_call_unary_answer_that_is_not_valid_gzip_returns_internal_error(
) -> Result<(), Box<dyn Error>> {
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: Some("gzip".into()),
            payload: b"this is not gzip".to_vec(),
        },
        1 << 20,
    )
    .await?;

    assert!(
        matches!(reply, Err(GrpcEgressError::Internal(_))),
        "got {reply:?}"
    );
    Ok(())
}

/// A few hundred bytes on the wire that expand past the cap must be refused, not allocated.
///
/// @covers: GrpcEgress::call_unary
#[tokio::test]
async fn test_call_unary_answer_that_expands_past_max_message_bytes_returns_resource_exhausted_error(
) -> Result<(), Box<dyn Error>> {
    let bomb = gzip(&vec![0u8; 1 << 20])?;
    // Under the wire cap below, so that only the decompression limit can refuse it.
    assert!(
        bomb.len() < 2048,
        "the bomb is {} bytes on the wire",
        bomb.len()
    );

    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: Some("gzip".into()),
            payload: bomb,
        },
        2048,
    )
    .await?;

    assert!(
        matches!(
            reply,
            Err(GrpcEgressError::Status(
                GrpcStatusCode::ResourceExhausted,
                _
            ))
        ),
        "got {reply:?}"
    );
    Ok(())
}

/// @covers: GrpcEgress::call_unary
#[tokio::test]
async fn test_call_unary_answer_that_expands_to_exactly_max_message_bytes_is_accepted_edge(
) -> Result<(), Box<dyn Error>> {
    let original = vec![7u8; 1024];
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: Some("gzip".into()),
            payload: gzip(&original)?,
        },
        1024,
    )
    .await?;

    assert_eq!(reply?, original);
    Ok(())
}

/// @covers: GrpcEgress::call_server_stream
#[tokio::test]
async fn test_call_server_stream_gzip_answer_is_decompressed_happy() -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let original = vec![b'q'; 512];
    let peer = PeerServer::start(PeerBehaviour::ReplyCompressed {
        encoding: Some("gzip".into()),
        payload: gzip(&original)?,
    })
    .await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .build()?;

    let mut answer = client.call_server_stream(request(b"ping".to_vec())).await?;

    let first = answer.stream.next().await.ok_or("no message")??;
    assert_eq!(first, original);
    Ok(())
}

/// @covers: GrpcEgress::call_stream
#[tokio::test]
async fn test_call_stream_with_gzip_sends_every_message_compressed_happy(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_compression(CompressionMode::Gzip);
    let client = ClientFixture::from_config(&config)?;
    let messages: Vec<Result<Vec<u8>, GrpcEgressError>> =
        vec![Ok(vec![b'a'; 1024]), Ok(vec![b'b'; 1024])];

    client
        .call_stream(CallStreamRequest {
            method: "pkg.Service/Method".to_string(),
            metadata: HashMap::new(),
            messages: GrpcMessageStreamResponse {
                stream: Box::pin(futures::stream::iter(messages)),
            },
        })
        .await?;

    let requests = peer.get_recorded_requests();
    let frames = requests.first().ok_or("no request")?.get_frames();
    assert_eq!(frames.len(), 2);
    assert!(frames.iter().all(|f| f.is_compressed()));
    assert_eq!(gunzip(frames[0].get_payload())?, vec![b'a'; 1024]);
    assert_eq!(gunzip(frames[1].get_payload())?, vec![b'b'; 1024]);
    Ok(())
}
