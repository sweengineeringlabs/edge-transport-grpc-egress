//! Tests of `with_loadbalancer` against real in-process peers: calls reach every backend in the order the strategy
//! says, a backend that is down is avoided, and every other option applies to a balanced call exactly as to a
//! single-endpoint one.

use proto_pattern::{
    CallError, CallRawRequestBuilder, CallRawResponse, CallRawServerStreamRequestBuilder,
    CallRawServerStreamResponse, CallRequest, Compression, MessageStream, MethodPath, RawTransport,
    ReadMessageRequest, ReadMessageResponse, StatusCode, TlsIdentity, TlsSettings,
};
use std::error::Error;
use std::io::Read as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

use grpc_egress_test_support::{
    CallFixture, ClosedPort, Fixture, PeerBehaviour, PeerServer, RunningPeer, TestPki,
    TlsServerMode,
};
use grpc_egress_transport::{
    BackendTarget, BalancingStrategy, BuildGrpcEgressBuilderRequest, GrpcEgressBuilder,
    GrpcEgressBuilderFactory, GrpcEgressInterceptor, InboundReply, LoadbalancerSettings,
    OutboundCall, TraceContextSource,
};
use grpc_egress_transport_spi::TonicGrpcEgressBuilderSpiFactory;

const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

/// Adds a header to every call: the header shows that the interceptor ran before the call.
struct TagInterceptor;

impl GrpcEgressInterceptor for TagInterceptor {
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
        call.add_header("x-tag", "tagged")
    }

    fn after_call(&self, _: &InboundReply<'_>) -> Result<(), CallError> {
        Ok(())
    }
}

fn settings(
    strategy: BalancingStrategy,
    urls: &[String],
) -> Result<LoadbalancerSettings, Box<dyn Error>> {
    let backends = urls
        .iter()
        .map(|url| {
            BackendTarget::try_new(url.clone(), 1)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(LoadbalancerSettings::try_new(strategy, backends)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?)
}

fn builder_over(urls: &[String]) -> Result<impl GrpcEgressBuilder, Box<dyn Error>> {
    Ok(TonicGrpcEgressBuilderSpiFactory::new("")
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_loadbalancer(settings(BalancingStrategy::RoundRobin, urls)?))
}

fn request() -> CallRequest<Vec<u8>> {
    CallFixture::request(b"ping", Duration::from_secs(5))
}

fn path() -> MethodPath {
    CallFixture::path("pkg.Service/Method")
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
async fn test_call_balanced_round_robin_alternates_between_the_backends_in_order(
) -> Result<(), Box<dyn Error>> {
    let (one, two, urls) = start_two(
        PeerBehaviour::Reply(b"one".to_vec()),
        PeerBehaviour::Reply(b"two".to_vec()),
    )
    .await?;
    let client = builder_over(&urls)?.build()?;

    let mut replies = Vec::new();
    for _ in 0..4 {
        replies.push(
            client
                .call_raw({
                    let (message, options) = request().into_parts();
                    CallRawRequestBuilder::new()
                        .with_path(path())
                        .with_message(message)
                        .with_options(options)
                        .build()
                })
                .await
                .map(CallRawResponse::into_response)?
                .into_message(),
        );
    }

    assert_eq!(
        replies,
        [
            b"one".to_vec(),
            b"two".to_vec(),
            b"one".to_vec(),
            b"two".to_vec()
        ]
    );
    assert_eq!(one.get_recorded_requests().len(), 2);
    assert_eq!(two.get_recorded_requests().len(), 2);
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_over_the_decoding_limit_returns_out_of_range_from_every_backend(
) -> Result<(), Box<dyn Error>> {
    let (one, two, urls) = start_two(
        PeerBehaviour::Reply(vec![0u8; 1025]),
        PeerBehaviour::Reply(vec![0u8; 1025]),
    )
    .await?;
    let client = builder_over(&urls)?
        .with_max_decoding_message_size(1024)
        .build()?;

    let first = client
        .call_raw({
            let (message, options) = request().into_parts();
            CallRawRequestBuilder::new()
                .with_path(path())
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response);
    let second = client
        .call_raw({
            let (message, options) = request().into_parts();
            CallRawRequestBuilder::new()
                .with_path(path())
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response);

    for result in [&first, &second] {
        assert!(
            matches!(
                result,
                Err(CallError::Status {
                    code: StatusCode::OutOfRange,
                    ..
                })
            ),
            "got {:?}",
            result.as_ref().err()
        );
    }
    assert_eq!(
        one.get_recorded_requests().len(),
        1,
        "the limit applied to backend one"
    );
    assert_eq!(
        two.get_recorded_requests().len(),
        1,
        "the limit applied to backend two"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_with_send_compression_reaches_every_backend_compressed(
) -> Result<(), Box<dyn Error>> {
    let (one, two, urls) = start_two(
        PeerBehaviour::Reply(b"ok".to_vec()),
        PeerBehaviour::Reply(b"ok".to_vec()),
    )
    .await?;
    let client = builder_over(&urls)?
        .with_send_compression(Compression::Gzip)
        .build()?;

    for _ in 0..2 {
        client
            .call_raw({
                let (message, options) =
                    CallFixture::request(&[b'a'; 2048], Duration::from_secs(5)).into_parts();
                CallRawRequestBuilder::new()
                    .with_path(path())
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)?;
    }

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
async fn test_call_balanced_with_interceptor_runs_it_before_each_call_to_every_backend(
) -> Result<(), Box<dyn Error>> {
    let (one, two, urls) = start_two(
        PeerBehaviour::Reply(b"ok".to_vec()),
        PeerBehaviour::Reply(b"ok".to_vec()),
    )
    .await?;
    let client = builder_over(&urls)?
        .with_interceptor(Arc::new(TagInterceptor))
        .build()?;

    for _ in 0..4 {
        client
            .call_raw({
                let (message, options) = request().into_parts();
                CallRawRequestBuilder::new()
                    .with_path(path())
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)?;
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
    let client = builder_over(&urls)?
        .with_trace_context(TraceContextSource::Static {
            traceparent: TRACEPARENT.to_string(),
            tracestate: None,
        })
        .build()?;

    for _ in 0..2 {
        client
            .call_raw({
                let (message, options) = request().into_parts();
                CallRawRequestBuilder::new()
                    .with_path(path())
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)?;
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
async fn test_call_balanced_with_request_timeout_ends_a_slow_call_at_it_on_every_backend(
) -> Result<(), Box<dyn Error>> {
    let (one, two, urls) = start_two(
        PeerBehaviour::Delay(Duration::from_secs(3)),
        PeerBehaviour::Delay(Duration::from_secs(3)),
    )
    .await?;
    let client = builder_over(&urls)?
        .with_request_timeout(Duration::from_millis(150))
        .build()?;
    let started = Instant::now();

    let first = client
        .call_raw(
            CallRawRequestBuilder::new()
                .with_path(path())
                .with_message(b"ping".to_vec())
                .with_options(Default::default())
                .build(),
        )
        .await
        .map(CallRawResponse::into_response);
    let second = client
        .call_raw(
            CallRawRequestBuilder::new()
                .with_path(path())
                .with_message(b"ping".to_vec())
                .with_options(Default::default())
                .build(),
        )
        .await
        .map(CallRawResponse::into_response);

    for result in [&first, &second] {
        assert_eq!(
            result.as_ref().err().map(CallError::get_status_code),
            Some(StatusCode::DeadlineExceeded),
            "got {:?}",
            result.as_ref().err()
        );
    }
    assert!(
        started.elapsed() < Duration::from_millis(2500),
        "each call ends at the request timeout, took {:?}",
        started.elapsed()
    );
    assert_eq!(one.get_recorded_requests().len(), 1);
    assert_eq!(two.get_recorded_requests().len(), 1);
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_with_tls_identity_presents_the_client_certificate_to_every_backend(
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
    let tls = TlsSettings::create(
        None,
        vec![std::fs::read(pki.get_ca_pem_path())?],
        Some(TlsIdentity::create(
            std::fs::read(pki.get_client_cert_pem_path())?,
            std::fs::read(pki.get_client_key_pem_path())?,
        )),
    );
    let client = TonicGrpcEgressBuilderSpiFactory::new("")
        .build(BuildGrpcEgressBuilderRequest)
        .with_tls(tls)
        .with_loadbalancer(settings(
            BalancingStrategy::RoundRobin,
            &[one.get_https_endpoint(), two.get_https_endpoint()],
        )?)
        .build()?;

    for _ in 0..4 {
        client
            .call_raw({
                let (message, options) = request().into_parts();
                CallRawRequestBuilder::new()
                    .with_path(path())
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)?;
    }

    assert_eq!(one.get_recorded_requests().len(), 2, "backend one");
    assert_eq!(two.get_recorded_requests().len(), 2, "backend two");
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_one_backend_down_uses_the_other() -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let live = PeerServer::start(PeerBehaviour::Reply(b"live".to_vec())).await?;
    let dead = ClosedPort::reserve()?;
    let client = builder_over(&[dead.get_endpoint(), live.get_endpoint()])?.build()?;

    let mut failures = 0;
    let mut answered = 0;
    for _ in 0..8 {
        match client
            .call_raw({
                let (message, options) = request().into_parts();
                CallRawRequestBuilder::new()
                    .with_path(path())
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)
        {
            Ok(response) => {
                assert_eq!(response.get_message(), b"live");
                answered += 1;
            }
            Err(CallError::Transport {
                code: StatusCode::Unavailable,
                ..
            }) => failures += 1,
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
async fn test_call_balanced_all_backends_down_returns_unavailable_each_time(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let one = ClosedPort::reserve()?;
    let two = ClosedPort::reserve()?;
    let client = builder_over(&[one.get_endpoint(), two.get_endpoint()])?.build()?;

    let mut outcomes = Vec::new();
    for _ in 0..3 {
        outcomes.push(
            client
                .call_raw({
                    let (message, options) = request().into_parts();
                    CallRawRequestBuilder::new()
                        .with_path(path())
                        .with_message(message)
                        .with_options(options)
                        .build()
                })
                .await
                .map(CallRawResponse::into_response),
        );
    }

    for (i, outcome) in outcomes.iter().enumerate() {
        assert!(
            matches!(
                outcome,
                Err(CallError::Transport {
                    code: StatusCode::Unavailable,
                    ..
                })
            ),
            "call {i}: got {:?}",
            outcome.as_ref().err()
        );
    }
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_raw_server_stream_reaches_a_backend() -> Result<(), Box<dyn Error>> {
    let (one, two, urls) = start_two(
        PeerBehaviour::Reply(b"s1".to_vec()),
        PeerBehaviour::Reply(b"s2".to_vec()),
    )
    .await?;
    let client = builder_over(&urls)?.build()?;

    let mut stream = client
        .call_raw_server_stream({
            let (message, options) = request().into_parts();
            CallRawServerStreamRequestBuilder::new()
                .with_path(path())
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawServerStreamResponse::into_stream)?;

    let first = stream
        .read_message(ReadMessageRequest::new())
        .await
        .map(ReadMessageResponse::into_message)?
        .ok_or("no message")?;
    assert_eq!(first, b"s1", "round robin starts at the first backend");
    assert_eq!(
        one.get_recorded_requests().len() + two.get_recorded_requests().len(),
        1
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_weighted_strategy_follows_the_weights_exactly(
) -> Result<(), Box<dyn Error>> {
    let (heavy, light, urls) = start_two(
        PeerBehaviour::Reply(b"heavy".to_vec()),
        PeerBehaviour::Reply(b"light".to_vec()),
    )
    .await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new("")
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_loadbalancer(LoadbalancerSettings::try_new(
            BalancingStrategy::Weighted,
            vec![
                BackendTarget::try_new(urls[0].clone(), 9)?,
                BackendTarget::try_new(urls[1].clone(), 1)?,
            ],
        )?)
        .build()?;

    for _ in 0..20 {
        client
            .call_raw({
                let (message, options) = request().into_parts();
                CallRawRequestBuilder::new()
                    .with_path(path())
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)?;
    }

    // The pool walks the weights in turn: in every 10 calls the heavier backend takes 9 and the lighter 1.
    assert_eq!(heavy.get_recorded_requests().len(), 18, "weight 9");
    assert_eq!(light.get_recorded_requests().len(), 2, "weight 1");
    Ok(())
}

/// The pool of `swe-edge-loadbalancer` 0.4.0 keeps a connection count per backend that nothing ever changes (the
/// field is private to the crate and `select` and `report_outcome` leave it alone), so `LeastConnections` always
/// takes the first healthy backend, busy or not. This test states what happens today, so that a pool that starts
/// counting makes it fail on purpose, and its author reads this comment.
///
/// @covers: GrpcEgressBuilder::with_loadbalancer
#[tokio::test]
async fn test_call_balanced_least_connections_takes_the_first_backend_even_while_it_is_busy_because_the_pool_counts_no_connections(
) -> Result<(), Box<dyn Error>> {
    let (first, second, urls) = start_two(
        PeerBehaviour::Delay(Duration::from_millis(600)),
        PeerBehaviour::Delay(Duration::from_millis(600)),
    )
    .await?;
    let client = Arc::new(
        TonicGrpcEgressBuilderSpiFactory::new("")
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .with_loadbalancer(settings(BalancingStrategy::LeastConnections, &urls)?)
            .build()?,
    );
    let busy = {
        let client = Arc::clone(&client);
        tokio::spawn(async move {
            client
                .call_raw({
                    let (message, options) = request().into_parts();
                    CallRawRequestBuilder::new()
                        .with_path(path())
                        .with_message(message)
                        .with_options(options)
                        .build()
                })
                .await
                .map(CallRawResponse::into_response)
                .map(|_| ())
        })
    };
    // The first call is in flight at the first backend: a strategy that counts connections sends the next elsewhere.
    tokio::time::sleep(Duration::from_millis(200)).await;

    client
        .call_raw({
            let (message, options) = request().into_parts();
            CallRawRequestBuilder::new()
                .with_path(path())
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;
    busy.await??;

    assert_eq!(
        first.get_recorded_requests().len(),
        2,
        "both calls went to the first backend"
    );
    assert_eq!(second.get_recorded_requests().len(), 0);
    Ok(())
}
