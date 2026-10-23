//! Tests of message compression in the client over the `proto-svc` engine, against a real in-process peer: a
//! compressed message is really compressed on the way out, an answer in an encoding the client advertised is
//! really decompressed on the way in, an answer in one it did not advertise is refused, and an answer that expands
//! past the limit is refused.
//!
//! The client sends with `with_send_compression` and tells the server what it accepts with
//! `with_accept_compression` (`grpc-accept-encoding`); the protocol lets a server use only an encoding the client
//! listed, and the engine accepts only those.

use proto_pattern::{
    CallError, CallOptions, CallRawClientStreamRequestBuilder, CallRawClientStreamResponse,
    CallRawRequestBuilder, CallRawResponse, CallRawServerStreamRequestBuilder,
    CallRawServerStreamResponse, Compression, MessageStream, Metadata, RawTransport,
    ReadMessageRequest, ReadMessageResponse, StatusCode,
};
use std::error::Error;
use std::io::{Read as _, Write as _};
use std::time::Duration;

use grpc_egress_test_support::{CallFixture, Fixture, PeerBehaviour, PeerServer, RunningPeer};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_spi::TonicGrpcEgressBuilderSpiFactory;
use proto_test_peer::ListStream;

const SHORT: Duration = Duration::from_secs(5);

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

/// Sends `payload` through a client that compresses with `send`, to a peer that answers `ok`, and returns the peer.
async fn send_with(send: Compression, payload: &[u8]) -> Result<RunningPeer, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_send_compression(send)
        .build()?;
    client
        .call_raw({
            let (message, options) = CallFixture::request(payload, SHORT).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;
    Ok(peer)
}

/// The answer of a unary call to a peer that behaves as `behaviour` says, for a client that accepts `accept` and
/// refuses a response message over `max_message_bytes`.
async fn call_peer_replying(
    behaviour: PeerBehaviour,
    accept: &[Compression],
    max_message_bytes: usize,
) -> Result<Result<Vec<u8>, CallError>, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(behaviour).await?;
    let mut builder = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_decoding_message_size(max_message_bytes);
    for compression in accept {
        builder = builder.with_accept_compression(*compression);
    }
    let client = builder.build()?;
    Ok(client
        .call_raw({
            let (message, options) = CallFixture::request(b"ping", SHORT).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .map(|response| response.into_message()))
}

// ---------------------------------------------------------------- on the way out

/// @covers: Compression::Gzip
#[tokio::test]
async fn test_call_raw_with_gzip_sends_a_compressed_message_that_unzips_to_the_original(
) -> Result<(), Box<dyn Error>> {
    let payload = vec![b'a'; 4096];
    let peer = send_with(Compression::Gzip, &payload).await?;

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

/// @covers: Compression::Zstd
#[tokio::test]
async fn test_call_raw_with_zstd_sends_a_compressed_message_that_decompresses_to_the_original(
) -> Result<(), Box<dyn Error>> {
    let payload = vec![b'a'; 4096];
    let peer = send_with(Compression::Zstd, &payload).await?;

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

/// @covers: Compression::Gzip
#[tokio::test]
async fn test_call_raw_with_gzip_empty_message_round_trips_edge() -> Result<(), Box<dyn Error>> {
    let peer = send_with(Compression::Gzip, b"").await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(
        request.get_frames().len(),
        1,
        "an empty message is still a message"
    );
    Ok(())
}

/// A client that sends with one encoding and accepts none says nothing about what it accepts; one that accepts an
/// encoding lists it in `grpc-accept-encoding`.
///
/// @covers: GrpcEgressBuilder::with_accept_compression
#[tokio::test]
async fn test_call_raw_with_accept_compression_lists_the_encoding_in_grpc_accept_encoding(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_accept_compression(Compression::Gzip)
        .with_accept_compression(Compression::Zstd)
        .build()?;

    client
        .call_raw({
            let (message, options) = CallFixture::request(b"ping", SHORT).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;

    let requests = peer.get_recorded_requests();
    let accepted = requests
        .first()
        .and_then(|r| r.get_header("grpc-accept-encoding"))
        .ok_or("the client must list what it accepts")?;
    assert!(accepted.contains("gzip"), "{accepted:?}");
    assert!(accepted.contains("zstd"), "{accepted:?}");
    Ok(())
}

/// Every message of a client-streaming call is compressed.
///
/// @covers: Compression::Gzip
#[tokio::test]
async fn test_call_raw_client_stream_with_gzip_sends_every_message_compressed(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_send_compression(Compression::Gzip)
        .build()?;
    let messages = ListStream::of(vec![vec![b'a'; 1024], vec![b'b'; 1024]]);

    client
        .call_raw_client_stream(
            CallRawClientStreamRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_requests(messages)
                .with_options(CallOptions::create(Some(SHORT), Metadata::default()))
                .build(),
        )
        .await
        .map(CallRawClientStreamResponse::into_response)?;

    let requests = peer.get_recorded_requests();
    let frames = requests.first().ok_or("no request")?.get_frames();
    assert_eq!(frames.len(), 2);
    assert!(frames.iter().all(|f| f.is_compressed()));
    assert_eq!(gunzip(frames[0].get_payload())?, vec![b'a'; 1024]);
    assert_eq!(gunzip(frames[1].get_payload())?, vec![b'b'; 1024]);
    Ok(())
}

// ---------------------------------------------------------------- on the way in

/// @covers: Compression::Gzip
#[tokio::test]
async fn test_call_raw_gzip_answer_to_a_client_that_accepts_gzip_is_decompressed(
) -> Result<(), Box<dyn Error>> {
    let original = vec![b'z'; 2048];
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: Some("gzip".into()),
            payload: gzip(&original)?,
        },
        &[Compression::Gzip],
        1 << 20,
    )
    .await?;

    assert_eq!(reply?, original);
    Ok(())
}

/// @covers: Compression::Zstd
#[tokio::test]
async fn test_call_raw_zstd_answer_to_a_client_that_accepts_zstd_is_decompressed(
) -> Result<(), Box<dyn Error>> {
    let original = vec![b'z'; 2048];
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: Some("zstd".into()),
            payload: zstd_compress(&original)?,
        },
        &[Compression::Zstd],
        1 << 20,
    )
    .await?;

    assert_eq!(reply?, original);
    Ok(())
}

/// A server may use only an encoding the client listed: a gzip answer to a client that accepts none is refused.
/// (The client of before decoded any encoding it knew, announced or not.)
///
/// @covers: GrpcEgressBuilder::with_accept_compression
#[tokio::test]
async fn test_call_raw_gzip_answer_to_a_client_that_accepts_nothing_is_refused_unimplemented(
) -> Result<(), Box<dyn Error>> {
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: Some("gzip".into()),
            payload: gzip(&[b'z'; 2048])?,
        },
        &[],
        1 << 20,
    )
    .await?;

    assert!(
        matches!(
            reply,
            Err(CallError::Status {
                code: StatusCode::Unimplemented,
                ..
            })
        ),
        "got {reply:?}"
    );
    Ok(())
}

/// A gzip answer to a client that accepts only zstd is refused too: accepting one encoding does not accept another.
///
/// @covers: GrpcEgressBuilder::with_accept_compression
#[tokio::test]
async fn test_call_raw_gzip_answer_to_a_client_that_accepts_only_zstd_is_refused_unimplemented(
) -> Result<(), Box<dyn Error>> {
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: Some("gzip".into()),
            payload: gzip(&[b'z'; 2048])?,
        },
        &[Compression::Zstd],
        1 << 20,
    )
    .await?;

    assert!(
        matches!(
            reply,
            Err(CallError::Status {
                code: StatusCode::Unimplemented,
                ..
            })
        ),
        "got {reply:?}"
    );
    Ok(())
}

/// A frame flagged compressed with no `grpc-encoding` announced is a protocol error.
///
/// @covers: frames
#[tokio::test]
async fn test_call_raw_compressed_answer_without_an_announced_encoding_returns_internal_status_error(
) -> Result<(), Box<dyn Error>> {
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: None,
            payload: gzip(b"hello")?,
        },
        &[Compression::Gzip],
        1 << 20,
    )
    .await?;

    assert!(
        matches!(
            reply,
            Err(CallError::Status {
                code: StatusCode::Internal,
                ..
            })
        ),
        "got {reply:?}"
    );
    Ok(())
}

/// @covers: frames
#[tokio::test]
async fn test_call_raw_answer_in_an_unsupported_encoding_returns_unimplemented_error(
) -> Result<(), Box<dyn Error>> {
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: Some("br".into()),
            payload: b"whatever".to_vec(),
        },
        &[Compression::Gzip],
        1 << 20,
    )
    .await?;

    assert!(
        matches!(
            reply,
            Err(CallError::Status {
                code: StatusCode::Unimplemented,
                ..
            })
        ),
        "got {reply:?}"
    );
    Ok(())
}

/// @covers: frames
#[tokio::test]
async fn test_call_raw_answer_that_is_not_valid_gzip_returns_internal_status_error(
) -> Result<(), Box<dyn Error>> {
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: Some("gzip".into()),
            payload: b"this is not gzip".to_vec(),
        },
        &[Compression::Gzip],
        1 << 20,
    )
    .await?;

    assert!(
        matches!(
            reply,
            Err(CallError::Status {
                code: StatusCode::Internal,
                ..
            })
        ),
        "got {reply:?}"
    );
    Ok(())
}

/// A few hundred bytes on the wire that expand past the limit must be refused, not allocated.
///
/// @covers: GrpcEgressBuilder::with_max_decoding_message_size
#[tokio::test]
async fn test_call_raw_answer_that_expands_past_the_decoding_limit_returns_resource_exhausted_error(
) -> Result<(), Box<dyn Error>> {
    let bomb = gzip(&vec![0u8; 1 << 20])?;
    // Under the wire limit below, so that only the decompression limit can refuse it.
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
        &[Compression::Gzip],
        2048,
    )
    .await?;

    assert!(
        matches!(
            reply,
            Err(CallError::Status {
                code: StatusCode::ResourceExhausted,
                ..
            })
        ),
        "got {reply:?}"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_max_decoding_message_size
#[tokio::test]
async fn test_call_raw_answer_that_expands_to_exactly_the_decoding_limit_is_accepted_edge(
) -> Result<(), Box<dyn Error>> {
    let original = vec![7u8; 1024];
    let reply = call_peer_replying(
        PeerBehaviour::ReplyCompressed {
            encoding: Some("gzip".into()),
            payload: gzip(&original)?,
        },
        &[Compression::Gzip],
        1024,
    )
    .await?;

    assert_eq!(reply?, original);
    Ok(())
}

/// @covers: Compression::Gzip
#[tokio::test]
async fn test_call_raw_server_stream_gzip_answer_to_a_client_that_accepts_gzip_is_decompressed(
) -> Result<(), Box<dyn Error>> {
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
        .with_accept_compression(Compression::Gzip)
        .build()?;

    let mut answer = client
        .call_raw_server_stream({
            let (message, options) = CallFixture::request(b"ping", SHORT).into_parts();
            CallRawServerStreamRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawServerStreamResponse::into_stream)?;

    let first = answer
        .read_message(ReadMessageRequest::new())
        .await
        .map(ReadMessageResponse::into_message)?
        .ok_or("no message")?;
    assert_eq!(first, original);
    Ok(())
}
