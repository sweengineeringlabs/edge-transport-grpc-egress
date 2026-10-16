//! Tests of the gRPC wire protocol of the `hyper`-backed client, against a real in-process peer: how a
//! request is put on the wire, and how every kind of answer is read -- the status codes, the frames, and
//! the deadline.

use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

use futures::StreamExt as _;
use grpc_egress_test_support::{Fixture, PeerBehaviour, PeerServer, RunningPeer};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, CallStreamRequest, GrpcChannelConfigError, GrpcEgress,
    GrpcEgressBuilder, GrpcEgressBuilderFactory, GrpcEgressError, GrpcMessageStreamResponse,
    GrpcRequest, GrpcStatusCode,
};
use grpc_egress_transport_std::{GrpcRequestFactory, TonicGrpcEgressBuilderSpiFactory};

const SANITIZED: &str = "internal client error";

fn frame(flag: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![flag];
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

async fn start(behaviour: PeerBehaviour) -> Result<RunningPeer, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    Ok(PeerServer::start(behaviour).await?)
}

fn client_for(peer: &RunningPeer) -> Result<impl GrpcEgress, Box<dyn Error>> {
    Ok(TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .build()?)
}

fn request(deadline: Duration) -> GrpcRequest {
    GrpcRequest::new("pkg.Service/Method", b"ping".to_vec(), deadline)
}

async fn call_status(
    status: &str,
    message: Option<&str>,
    retry_after_secs: Option<u64>,
) -> Result<GrpcEgressError, Box<dyn Error>> {
    let peer = start(PeerBehaviour::ReplyStatus {
        status: status.to_string(),
        message: message.map(str::to_string),
        retry_after_secs,
    })
    .await?;
    let client = client_for(&peer)?;
    match client.call_unary(request(Duration::from_secs(5))).await {
        Err(error) => Ok(error),
        Ok(_) => Err("a non-zero grpc-status must be an error".into()),
    }
}

async fn call_raw(body: Vec<u8>, encoding: Option<&str>) -> Result<Vec<u8>, GrpcEgressError> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::ReplyRaw {
        body,
        encoding: encoding.map(str::to_string),
    })
    .await
    .map_err(|e| GrpcEgressError::Internal(e.to_string()))?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .build()
        .map_err(|e| GrpcEgressError::Internal(e.to_string()))?;
    client
        .call_unary(request(Duration::from_secs(5)))
        .await
        .map(|response| response.body)
}

// ---------------------------------------------------------------- status codes

/// @covers: grpc-status
#[tokio::test]
async fn test_call_unary_every_grpc_status_code_maps_to_its_status_code(
) -> Result<(), Box<dyn Error>> {
    let canonical = [
        (1, GrpcStatusCode::Cancelled),
        (2, GrpcStatusCode::Unknown),
        (3, GrpcStatusCode::InvalidArgument),
        (4, GrpcStatusCode::DeadlineExceeded),
        (5, GrpcStatusCode::NotFound),
        (6, GrpcStatusCode::AlreadyExists),
        (7, GrpcStatusCode::PermissionDenied),
        (8, GrpcStatusCode::ResourceExhausted),
        (9, GrpcStatusCode::FailedPrecondition),
        (10, GrpcStatusCode::Aborted),
        (11, GrpcStatusCode::OutOfRange),
        (12, GrpcStatusCode::Unimplemented),
        (13, GrpcStatusCode::Internal),
        (14, GrpcStatusCode::Unavailable),
        (15, GrpcStatusCode::DataLoss),
        (16, GrpcStatusCode::Unauthenticated),
    ];
    for (wire, expected) in canonical {
        match call_status(&wire.to_string(), Some("peer says no"), None).await? {
            GrpcEgressError::Status(code, message) => {
                assert_eq!(code, expected, "grpc-status {wire}");
                assert_eq!(message, "peer says no", "grpc-status {wire}");
            }
            other => return Err(format!("grpc-status {wire}: expected Status, got {other}").into()),
        }
    }
    Ok(())
}

/// @covers: grpc-status
#[tokio::test]
async fn test_call_unary_unknown_or_malformed_grpc_status_maps_to_unknown(
) -> Result<(), Box<dyn Error>> {
    for status in ["99", "-1", "abc"] {
        match call_status(status, None, None).await? {
            GrpcEgressError::Status(code, _) => {
                assert_eq!(code, GrpcStatusCode::Unknown, "grpc-status {status:?}")
            }
            other => return Err(format!("{status:?}: expected Status, got {other}").into()),
        }
    }
    Ok(())
}

/// @covers: grpc-status
#[tokio::test]
async fn test_call_unary_resource_exhausted_with_retry_after_embeds_the_hint(
) -> Result<(), Box<dyn Error>> {
    match call_status("8", Some("slow down"), Some(7)).await? {
        GrpcEgressError::Status(GrpcStatusCode::ResourceExhausted, message) => {
            assert_eq!(message, "slow down [retry-after=7s]");
            Ok(())
        }
        other => Err(format!("expected ResourceExhausted, got {other}").into()),
    }
}

/// @covers: grpc-status
#[tokio::test]
async fn test_call_unary_other_status_with_retry_after_leaves_the_message_alone(
) -> Result<(), Box<dyn Error>> {
    match call_status("14", Some("down"), Some(7)).await? {
        GrpcEgressError::Status(GrpcStatusCode::Unavailable, message) => {
            assert_eq!(message, "down");
            Ok(())
        }
        other => Err(format!("expected Unavailable, got {other}").into()),
    }
}

// ---------------------------------------------------------------- frames

/// @covers: frames
#[tokio::test]
async fn test_call_unary_empty_body_is_an_empty_message() -> Result<(), Box<dyn Error>> {
    assert_eq!(call_raw(Vec::new(), None).await?, Vec::<u8>::new());
    Ok(())
}

/// @covers: frames
#[tokio::test]
async fn test_call_unary_one_frame_is_its_payload() -> Result<(), Box<dyn Error>> {
    assert_eq!(call_raw(frame(0, b"hello"), None).await?, b"hello");
    Ok(())
}

/// @covers: frames
#[tokio::test]
async fn test_call_unary_truncated_frame_is_a_sanitized_internal_error() {
    let mut body = frame(0, b"hello");
    body.truncate(body.len() - 2);

    assert!(matches!(
        call_raw(body, None).await,
        Err(GrpcEgressError::Internal(message)) if message == SANITIZED
    ));
}

/// @covers: frames
#[tokio::test]
async fn test_call_unary_bytes_after_the_last_frame_are_a_sanitized_internal_error() {
    let mut body = frame(0, b"hello");
    body.extend_from_slice(&[1, 2, 3]);

    assert!(matches!(
        call_raw(body, None).await,
        Err(GrpcEgressError::Internal(message)) if message == SANITIZED
    ));
}

/// @covers: frames
#[tokio::test]
async fn test_call_unary_invalid_compressed_flag_is_a_sanitized_internal_error() {
    assert!(matches!(
        call_raw(frame(2, b"hello"), None).await,
        Err(GrpcEgressError::Internal(message)) if message == SANITIZED
    ));
}

/// @covers: frames
#[tokio::test]
async fn test_call_unary_compressed_flag_without_an_encoding_is_a_sanitized_internal_error() {
    assert!(matches!(
        call_raw(frame(1, b"hello"), None).await,
        Err(GrpcEgressError::Internal(message)) if message == SANITIZED
    ));
}

/// @covers: frames
#[tokio::test]
async fn test_call_unary_two_frames_are_a_sanitized_internal_error() {
    let mut body = frame(0, b"one");
    body.extend_from_slice(&frame(0, b"two"));

    assert!(matches!(
        call_raw(body, None).await,
        Err(GrpcEgressError::Internal(message)) if message == SANITIZED
    ));
}

/// @covers: frames
#[tokio::test]
async fn test_call_stream_two_frames_are_two_messages() -> Result<(), Box<dyn Error>> {
    let mut body = frame(0, b"one");
    body.extend_from_slice(&frame(0, b"two"));
    let peer = start(PeerBehaviour::ReplyRaw {
        body,
        encoding: None,
    })
    .await?;
    let client = client_for(&peer)?;

    let response = client
        .call_stream(CallStreamRequest {
            method: "pkg.Service/Stream".to_string(),
            metadata: HashMap::new(),
            messages: GrpcMessageStreamResponse {
                stream: Box::pin(futures::stream::iter(vec![Ok(b"in".to_vec())])),
            },
        })
        .await?;
    let messages: Vec<Vec<u8>> = response
        .stream
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<_, _>>()?;

    assert_eq!(messages, vec![b"one".to_vec(), b"two".to_vec()]);
    Ok(())
}

// ---------------------------------------------------------------- the request on the wire

/// @covers: request headers
#[tokio::test]
async fn test_call_unary_puts_the_grpc_headers_the_path_and_the_framed_message_on_the_wire(
) -> Result<(), Box<dyn Error>> {
    let peer = start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = client_for(&peer)?;
    let mut call = request(Duration::from_secs(5));
    call.metadata
        .insert("x-trace-id".to_string(), "abc123".to_string());

    client.call_unary(call).await?;

    let recorded = peer.get_recorded_requests();
    let first = recorded.first().ok_or("the peer saw no request")?;
    assert_eq!(first.get_path(), "/pkg.Service/Method");
    assert_eq!(first.get_header("content-type"), Some("application/grpc"));
    assert_eq!(first.get_header("te"), Some("trailers"));
    assert_eq!(first.get_header("x-trace-id"), Some("abc123"));
    let frames = first.get_frames();
    assert_eq!(frames.len(), 1);
    assert!(!frames[0].is_compressed());
    assert_eq!(frames[0].get_payload(), b"ping");
    Ok(())
}

/// @covers: request headers
#[tokio::test]
async fn test_call_unary_encodes_the_deadline_as_a_grpc_timeout_in_the_smallest_fitting_unit(
) -> Result<(), Box<dyn Error>> {
    let expectations = [
        (Duration::from_millis(50), "50000000n"),
        (Duration::from_secs(5), "5000000u"),
    ];
    for (deadline, expected) in expectations {
        let peer = start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
        let client = client_for(&peer)?;

        client.call_unary(request(deadline)).await?;

        let recorded = peer.get_recorded_requests();
        let first = recorded.first().ok_or("the peer saw no request")?;
        assert_eq!(
            first.get_header("grpc-timeout"),
            Some(expected),
            "{deadline:?}"
        );
    }
    Ok(())
}

/// @covers: request uri
#[tokio::test]
async fn test_call_unary_method_that_is_no_uri_path_is_a_sanitized_internal_error(
) -> Result<(), Box<dyn Error>> {
    let peer = start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = client_for(&peer)?;
    let call = GrpcRequest::new("pkg.Service/bad method", Vec::new(), Duration::from_secs(5));

    let result = client.call_unary(call).await;

    assert!(matches!(result, Err(GrpcEgressError::Internal(message)) if message == SANITIZED));
    assert!(
        peer.get_recorded_requests().is_empty(),
        "nothing may be sent"
    );
    Ok(())
}

// ---------------------------------------------------------------- deadline

/// @covers: deadline
#[tokio::test]
async fn test_call_unary_peer_slower_than_the_deadline_is_a_timeout() -> Result<(), Box<dyn Error>>
{
    let peer = start(PeerBehaviour::Delay(Duration::from_millis(500))).await?;
    let client = client_for(&peer)?;

    let result = client.call_unary(request(Duration::from_millis(50))).await;

    assert!(
        matches!(result, Err(GrpcEgressError::Timeout(_))),
        "{result:?}"
    );
    Ok(())
}

/// @covers: cancellation
#[tokio::test]
async fn test_call_unary_cancelled_while_in_flight_is_cancelled() -> Result<(), Box<dyn Error>> {
    let peer = start(PeerBehaviour::Delay(Duration::from_secs(5))).await?;
    let client = client_for(&peer)?;
    let token = tokio_util::sync::CancellationToken::new();
    let mut call = request(Duration::from_secs(10));
    call.cancellation = Some(token.clone());
    let canceller = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        token.cancel();
    });

    let result = client.call_unary(call).await;
    canceller.await?;

    assert!(
        matches!(result, Err(GrpcEgressError::Cancelled(_))),
        "{result:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------- the scheme of the endpoint

/// @covers: plaintext endpoint
#[test]
fn test_build_http_scheme_in_any_case_is_a_plaintext_endpoint_and_is_refused(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    for endpoint in [
        "http://127.0.0.1:1",
        "HTTP://127.0.0.1:1",
        "Http://127.0.0.1:1",
    ] {
        let result = TonicGrpcEgressBuilderSpiFactory::new(endpoint)
            .build(BuildGrpcEgressBuilderRequest)
            .build();

        assert!(
            matches!(result, Err(GrpcChannelConfigError::PlaintextRejected(ref e)) if e == endpoint),
            "{endpoint}"
        );
    }
    Ok(())
}

/// @covers: plaintext endpoint
#[test]
fn test_build_https_scheme_is_not_a_plaintext_endpoint() {
    Fixture::ensure_tls_provider();

    let result = TonicGrpcEgressBuilderSpiFactory::new("https://service.example:443")
        .build(BuildGrpcEgressBuilderRequest)
        .build();

    assert!(result.is_ok());
}
