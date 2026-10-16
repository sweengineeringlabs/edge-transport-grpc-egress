//! Tests of `with_loadbalancer` against real in-process peers: calls reach every backend, a backend
//! that is down is avoided, and every other option applies to a balanced call exactly as to a
//! single-endpoint one.

use grpc_egress_transport_std::{GrpcRequestFactory, MtlsConfigFactory};
use std::error::Error;
use std::io::Read as _;
use std::sync::Arc;
use std::time::Duration;

use grpc_egress_test_support::{
    ClosedPort, Fixture, PeerBehaviour, PeerServer, RunningPeer, TestPki, TlsServerMode,
};
use grpc_egress_transport::{
    AfterCallRequest, BackendTarget, BalancingStrategy, BuildGrpcEgressBuilderRequest,
    CompressionMode, GrpcEgress, GrpcEgressBuilder, GrpcEgressBuilderFactory, GrpcEgressError,
    GrpcEgressInterceptor, GrpcRequest, GrpcStatusCode, LoadbalancerSettings, MtlsConfig,
    TraceContextSource,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;

const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

/// Adds a header to every call: the header shows that the interceptor ran before the call.
struct TagInterceptor;

impl GrpcEgressInterceptor for TagInterceptor {
    fn before_call(&self, req: &mut GrpcRequest) -> Result<(), GrpcEgressError> {
        req.metadata
            .insert("x-tag".to_string(), "tagged".to_string());
        Ok(())
    }

    fn after_call(&self, _req: AfterCallRequest<'_>) -> Result<(), GrpcEgressError> {
        Ok(())
    }
}

fn settings(urls: &[String]) -> LoadbalancerSettings {
    LoadbalancerSettings {
        strategy: BalancingStrategy::RoundRobin,
        backends: urls
            .iter()
            .map(|url| BackendTarget {
                url: url.clone(),
                weight: 1,
            })
            .collect(),
    }
}

fn builder_over(urls: &[String]) -> impl GrpcEgressBuilder {
    TonicGrpcEgressBuilderSpiFactory::new("")
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_loadbalancer(settings(urls))
}

fn request() -> GrpcRequest {
    GrpcRequest::new(
        "pkg.Service/Method",
        b"ping".to_vec(),
        Duration::from_secs(5),
    )
}

async fn start_two(
    first: PeerBehaviour,
    second: PeerBehaviour,
) -> Result<(RunningPeer, RunningPeer, Vec<String>), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let one = PeerServer::start(first).await?;
    let two = PeerServer::start(second).await?;
    let urls = vec![one.get_endpoint(), two.get_endpoint()];
    Ok((one, two, urls))
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_two_backends_alternates_between_them() -> Result<(), Box<dyn Error>> {
    let (one, two, urls) = start_two(
        PeerBehaviour::Reply(b"one".to_vec()),
        PeerBehaviour::Reply(b"two".to_vec()),
    )
    .await?;
    let client = builder_over(&urls).build()?;

    let mut replies = Vec::new();
    for _ in 0..4 {
        replies.push(client.call_unary(request()).await?.body);
    }

    assert_eq!(one.get_recorded_requests().len(), 2, "replies: {replies:?}");
    assert_eq!(two.get_recorded_requests().len(), 2, "replies: {replies:?}");
    assert!(replies.contains(&b"one".to_vec()) && replies.contains(&b"two".to_vec()));
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_over_max_message_bytes_returns_out_of_range_error(
) -> Result<(), Box<dyn Error>> {
    let (_one, _two, urls) = start_two(
        PeerBehaviour::Reply(vec![0u8; 1025]),
        PeerBehaviour::Reply(vec![0u8; 1025]),
    )
    .await?;
    let client = builder_over(&urls).with_max_message_bytes(1024).build()?;

    let result = client.call_unary(request()).await;

    assert!(
        matches!(
            result,
            Err(GrpcEgressError::Status(GrpcStatusCode::OutOfRange, _))
        ),
        "got {result:?}"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_with_compression_reaches_backend_compressed(
) -> Result<(), Box<dyn Error>> {
    let (one, two, urls) = start_two(
        PeerBehaviour::Reply(b"ok".to_vec()),
        PeerBehaviour::Reply(b"ok".to_vec()),
    )
    .await?;
    let client = builder_over(&urls)
        .with_compression(CompressionMode::Gzip)
        .build()?;

    client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            vec![b'a'; 2048],
            Duration::from_secs(5),
        ))
        .await?;
    client
        .call_unary(GrpcRequest::new(
            "pkg.Service/Method",
            vec![b'a'; 2048],
            Duration::from_secs(5),
        ))
        .await?;

    for peer in [&one, &two] {
        let requests = peer.get_recorded_requests();
        let request = requests.first().ok_or("a backend received no call")?;
        assert_eq!(request.get_header("grpc-encoding"), Some("gzip"));
        let frame = request.get_frames().first().ok_or("no message")?;
        assert!(frame.is_compressed());
        let mut original = Vec::new();
        flate2::read::GzDecoder::new(frame.get_payload()).read_to_end(&mut original)?;
        assert_eq!(original, vec![b'a'; 2048]);
    }
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_with_interceptor_runs_interceptor_before_each_call(
) -> Result<(), Box<dyn Error>> {
    let (one, two, urls) = start_two(
        PeerBehaviour::Reply(b"ok".to_vec()),
        PeerBehaviour::Reply(b"ok".to_vec()),
    )
    .await?;
    let client = builder_over(&urls)
        .with_interceptor(Arc::new(TagInterceptor))
        .build()?;

    for _ in 0..4 {
        client.call_unary(request()).await?;
    }

    for peer in [&one, &two] {
        let requests = peer.get_recorded_requests();
        assert_eq!(requests.len(), 2);
        assert!(requests
            .iter()
            .all(|r| r.get_header("x-tag") == Some("tagged")));
    }
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_with_trace_context_sends_the_traceparent_to_every_backend(
) -> Result<(), Box<dyn Error>> {
    let (one, two, urls) = start_two(
        PeerBehaviour::Reply(b"ok".to_vec()),
        PeerBehaviour::Reply(b"ok".to_vec()),
    )
    .await?;
    let client = builder_over(&urls)
        .with_trace_context(TraceContextSource::Static {
            traceparent: TRACEPARENT.to_string(),
            tracestate: None,
        })
        .build()?;

    for _ in 0..2 {
        client.call_unary(request()).await?;
    }

    for peer in [&one, &two] {
        let requests = peer.get_recorded_requests();
        let received = requests.first().ok_or("a backend received no call")?;
        assert_eq!(received.get_header("traceparent"), Some(TRACEPARENT));
    }
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_with_mtls_presents_the_client_certificate_to_every_backend(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let pki = TestPki::create()?;
    let one = PeerServer::start_tls(
        PeerBehaviour::Reply(b"ok".to_vec()),
        &pki,
        TlsServerMode::ClientAuthRequired,
    )
    .await?;
    let two = PeerServer::start_tls(
        PeerBehaviour::Reply(b"ok".to_vec()),
        &pki,
        TlsServerMode::ClientAuthRequired,
    )
    .await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new("")
        .build(BuildGrpcEgressBuilderRequest)
        .with_mtls(MtlsConfig::with_root_ca(
            pki.get_client_cert_pem_path()
                .to_string_lossy()
                .into_owned(),
            pki.get_client_key_pem_path().to_string_lossy().into_owned(),
            pki.get_ca_pem_path().to_string_lossy().into_owned(),
        ))
        .with_loadbalancer(settings(&[
            one.get_https_endpoint(),
            two.get_https_endpoint(),
        ]))
        .build()?;

    for _ in 0..4 {
        client.call_unary(request()).await?;
    }

    assert!(
        !one.get_recorded_requests().is_empty(),
        "backend one saw no call"
    );
    assert!(
        !two.get_recorded_requests().is_empty(),
        "backend two saw no call"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_one_backend_down_uses_the_other() -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let live = PeerServer::start(PeerBehaviour::Reply(b"live".to_vec())).await?;
    let dead = ClosedPort::reserve()?;
    let client = builder_over(&[dead.get_endpoint(), live.get_endpoint()]).build()?;

    let mut failures = 0;
    let mut answered = 0;
    for _ in 0..8 {
        match client.call_unary(request()).await {
            Ok(response) => {
                assert_eq!(response.body, b"live");
                answered += 1;
            }
            Err(GrpcEgressError::ConnectionFailed(_)) => failures += 1,
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }

    assert!(
        failures <= 1,
        "the dead backend must be avoided after a failure, got {failures}"
    );
    assert!(answered >= 7, "got {answered} answers");
    assert_eq!(live.get_recorded_requests().len(), answered);
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_all_backends_down_returns_an_error_not_a_hang_error(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let one = ClosedPort::reserve()?;
    let two = ClosedPort::reserve()?;
    let client = builder_over(&[one.get_endpoint(), two.get_endpoint()]).build()?;

    let mut outcomes = Vec::new();
    for _ in 0..3 {
        outcomes.push(client.call_unary(request()).await);
    }

    assert!(
        outcomes.iter().all(Result::is_err),
        "no backend can answer, got {outcomes:?}"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_server_stream_reaches_a_backend_happy() -> Result<(), Box<dyn Error>> {
    use futures::StreamExt as _;
    let (one, two, urls) = start_two(
        PeerBehaviour::Reply(b"s1".to_vec()),
        PeerBehaviour::Reply(b"s2".to_vec()),
    )
    .await?;
    let client = builder_over(&urls).build()?;

    let mut answer = client.call_server_stream(request()).await?;

    let first = answer.stream.next().await.ok_or("no message")??;
    assert!(first == b"s1" || first == b"s2");
    assert_eq!(
        one.get_recorded_requests().len() + two.get_recorded_requests().len(),
        1
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_weighted_strategy_sends_more_calls_to_the_heavier_backend(
) -> Result<(), Box<dyn Error>> {
    let (heavy, light, urls) = start_two(
        PeerBehaviour::Reply(b"heavy".to_vec()),
        PeerBehaviour::Reply(b"light".to_vec()),
    )
    .await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new("")
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_loadbalancer(LoadbalancerSettings {
            strategy: BalancingStrategy::Weighted,
            backends: vec![
                BackendTarget {
                    url: urls[0].clone(),
                    weight: 9,
                },
                BackendTarget {
                    url: urls[1].clone(),
                    weight: 1,
                },
            ],
        })
        .build()?;

    for _ in 0..20 {
        client.call_unary(request()).await?;
    }

    let (heavy, light) = (
        heavy.get_recorded_requests().len(),
        light.get_recorded_requests().len(),
    );
    assert_eq!(heavy + light, 20);
    assert!(
        heavy > light,
        "weights 9:1 must favour the first backend, got {heavy}:{light}"
    );
    Ok(())
}
