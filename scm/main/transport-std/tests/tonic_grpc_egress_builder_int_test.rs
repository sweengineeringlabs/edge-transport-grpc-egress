//! Tests of the builder of the client over the `proto-svc` engine, through its public factory, against a real
//! in-process peer: every option must reach the call it describes, and none may be ignored.

use grpc_egress_transport::GrpcChannelConfigBuilder as _;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use grpc_egress_test_support::{CallFixture, Fixture, PeerBehaviour, PeerServer};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, CompressionMode, GrpcChannelConfigError, GrpcEgressBuilder,
    GrpcEgressBuilderFactory, GrpcEgressInterceptor, InboundReply, OutboundCall,
    TraceContextSource,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;
use proto_pattern::{
    CallError, CallRequest, CallResponse, Compression, Metadata, MetadataEntry, MethodPath,
    RawTransport, StatusCode,
};

const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
const SECONDS: Duration = Duration::from_secs(5);

/// Records the order interceptors ran in, and adds the header `x-seen-<tag>`: the log shows the order and the
/// peer shows the headers that were sent.
struct TagInterceptor {
    tag: &'static str,
    log: Arc<Mutex<Vec<String>>>,
}

impl GrpcEgressInterceptor for TagInterceptor {
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
        if let Ok(mut log) = self.log.lock() {
            log.push(format!("before:{}", self.tag));
        }
        call.add_header(&format!("x-seen-{}", self.tag), "yes")
    }

    fn after_call(&self, _reply: &InboundReply<'_>) -> Result<(), CallError> {
        if let Ok(mut log) = self.log.lock() {
            log.push(format!("after:{}", self.tag));
        }
        Ok(())
    }
}

/// Refuses every call before it is sent.
struct RefusingInterceptor;

impl GrpcEgressInterceptor for RefusingInterceptor {
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
        Err(CallError::Transport {
            path: call.get_path().into(),
            code: StatusCode::Aborted,
            reason: "refused by the interceptor".to_string(),
        })
    }

    fn after_call(&self, _reply: &InboundReply<'_>) -> Result<(), CallError> {
        Ok(())
    }
}

/// Lets every call go and fails every reply.
struct FailingAfterInterceptor;

impl GrpcEgressInterceptor for FailingAfterInterceptor {
    fn before_call(&self, _call: &mut dyn OutboundCall) -> Result<(), CallError> {
        Ok(())
    }

    fn after_call(&self, reply: &InboundReply<'_>) -> Result<(), CallError> {
        Err(CallError::Transport {
            path: reply.path.into(),
            code: StatusCode::DataLoss,
            reason: "the reply was refused by the interceptor".to_string(),
        })
    }
}

fn ping() -> CallRequest<Vec<u8>> {
    CallFixture::request(b"ping", SECONDS)
}

fn path() -> MethodPath {
    CallFixture::path("pkg.Service/Method")
}

/// One raw call to `path()`: the bytes of the answer, or why it failed.
async fn send(
    client: &impl RawTransport,
    request: CallRequest<Vec<u8>>,
) -> Result<Vec<u8>, CallError> {
    client
        .call_raw(path(), request)
        .await
        .map(CallResponse::into_message)
}

/// `true` when `result` is a response message the client's decoding limit refused.
fn is_out_of_range<T>(result: &Result<T, CallError>) -> bool {
    matches!(
        result,
        Err(CallError::Status {
            code: StatusCode::OutOfRange,
            ..
        })
    )
}

/// @covers: TonicGrpcEgressBuilderSpiFactory::build
#[tokio::test]
async fn test_build_factory_called_twice_returns_independent_builders() -> Result<(), Box<dyn Error>>
{
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1025])).await?;
    let factory = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint());
    let capped = factory
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_decoding_message_size(1024)
        .build()?;
    let uncapped = factory
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .build()?;

    let refused = send(&capped, ping()).await;
    let accepted = send(&uncapped, ping()).await?;

    assert!(
        is_out_of_range(&refused),
        "the builder with a limit must refuse, got {refused:?}"
    );
    assert_eq!(
        accepted.len(),
        1025,
        "the second builder must not share the first one's limit"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_endpoint
#[tokio::test]
async fn test_with_endpoint_overrides_the_endpoint_of_the_factory() -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let closed = Fixture::reserve_closed_port()?;
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(closed.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_endpoint(peer.get_endpoint())
        .build()?;

    send(&client, ping()).await?;

    assert_eq!(peer.get_recorded_requests().len(), 1);
    Ok(())
}

/// @covers: GrpcEgressBuilder::build
#[tokio::test]
async fn test_build_plaintext_endpoint_without_plaintext_allowed_returns_plaintext_rejected_error(
) -> Result<(), Box<dyn Error>> {
    let result = TonicGrpcEgressBuilderSpiFactory::new("http://127.0.0.1:1")
        .build(BuildGrpcEgressBuilderRequest)
        .build();

    assert!(
        matches!(result, Err(GrpcChannelConfigError::PlaintextRejected(ref e)) if e == "http://127.0.0.1:1"),
        "expected PlaintextRejected naming the endpoint"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_plaintext_allowed
#[tokio::test]
async fn test_with_plaintext_allowed_plaintext_endpoint_builds_a_client_that_reaches_the_peer_happy(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .build()?;

    let response = send(&client, ping()).await?;

    assert_eq!(response, b"ping");
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_max_decoding_message_size
#[tokio::test]
async fn test_with_max_decoding_message_size_response_at_the_limit_returns_the_message_happy(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1024])).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_decoding_message_size(1024)
        .build()?;

    let response = send(&client, ping()).await?;

    assert_eq!(response.len(), 1024);
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_max_decoding_message_size
#[tokio::test]
async fn test_with_max_decoding_message_size_response_over_the_limit_returns_out_of_range_error(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1025])).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_decoding_message_size(1024)
        .build()?;

    let result = send(&client, ping()).await;

    assert!(
        is_out_of_range(&result),
        "expected OutOfRange, got {result:?}"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_max_encoding_message_size
#[tokio::test]
async fn test_with_max_encoding_message_size_request_over_the_limit_is_refused_before_it_is_sent_error(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_encoding_message_size(1024)
        .build()?;

    let result = send(&client, CallFixture::request(&[1u8; 1025], SECONDS)).await;

    assert_eq!(
        result.as_ref().err().map(CallError::get_status_code),
        Some(StatusCode::ResourceExhausted),
        "got {result:?}"
    );
    assert!(
        peer.get_recorded_requests().is_empty(),
        "an oversize request must not reach the peer"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_max_encoding_message_size
#[tokio::test]
async fn test_with_max_encoding_message_size_request_at_the_limit_is_sent_happy(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_encoding_message_size(1024)
        .build()?;

    send(&client, CallFixture::request(&[1u8; 1024], SECONDS)).await?;

    assert_eq!(peer.get_recorded_requests().len(), 1);
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_send_compression
#[tokio::test]
async fn test_with_send_compression_gzip_compresses_the_request_and_says_so_on_the_wire_happy(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_send_compression(Compression::Gzip)
        .build()?;

    send(&client, CallFixture::request(&[b'a'; 4096], SECONDS)).await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("grpc-encoding"), Some("gzip"));
    let frame = request.get_frames().first().ok_or("no message")?;
    assert!(
        frame.is_compressed(),
        "the frame must be flagged compressed"
    );
    assert!(frame.get_payload().len() < 4096 / 10);
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_accept_compression
#[tokio::test]
async fn test_with_accept_compression_two_encodings_are_both_announced_to_the_server_happy(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_accept_compression(Compression::Gzip)
        .with_accept_compression(Compression::Zstd)
        .build()?;

    send(&client, ping()).await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    let announced = request
        .get_header("grpc-accept-encoding")
        .ok_or("no grpc-accept-encoding on the wire")?;
    assert!(
        announced.contains("gzip") && announced.contains("zstd"),
        "both encodings must be announced, got {announced}"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_accept_compression
#[tokio::test]
async fn test_with_accept_compression_the_same_encoding_twice_is_announced_once_edge(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_accept_compression(Compression::Gzip)
        .with_accept_compression(Compression::Gzip)
        .build()?;

    send(&client, ping()).await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    let announced = request
        .get_header("grpc-accept-encoding")
        .ok_or("no grpc-accept-encoding on the wire")?;
    assert_eq!(
        announced.split(',').filter(|e| e.trim() == "gzip").count(),
        1,
        "got {announced}"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_interceptor
#[tokio::test]
async fn test_with_interceptor_header_it_adds_reaches_the_peer_happy() -> Result<(), Box<dyn Error>>
{
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let log = Arc::new(Mutex::new(Vec::new()));
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_interceptor(Arc::new(TagInterceptor { tag: "a", log }))
        .build()?;

    send(&client, ping()).await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("x-seen-a"), Some("yes"));
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_interceptor
#[tokio::test]
async fn test_with_interceptor_two_added_run_in_the_order_they_were_added_edge(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let log = Arc::new(Mutex::new(Vec::new()));
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_interceptor(Arc::new(TagInterceptor {
            tag: "first",
            log: Arc::clone(&log),
        }))
        .with_interceptor(Arc::new(TagInterceptor {
            tag: "second",
            log: Arc::clone(&log),
        }))
        .build()?;

    send(&client, ping()).await?;

    let order = log.lock().map_err(|e| e.to_string())?.clone();
    assert_eq!(
        order,
        [
            "before:first",
            "before:second",
            "after:first",
            "after:second"
        ]
    );
    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("x-seen-first"), Some("yes"));
    assert_eq!(request.get_header("x-seen-second"), Some("yes"));
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_interceptor
#[tokio::test]
async fn test_with_interceptor_that_refuses_aborts_the_call_before_it_is_sent_error(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_interceptor(Arc::new(RefusingInterceptor))
        .build()?;

    let result = send(&client, ping()).await;

    assert!(
        matches!(&result, Err(CallError::Transport { reason, .. }) if reason == "refused by the interceptor"),
        "expected the interceptor's error, got {result:?}"
    );
    assert!(
        peer.get_recorded_requests().is_empty(),
        "a refused call must not reach the peer"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_interceptor
#[tokio::test]
async fn test_with_interceptor_whose_after_call_fails_turns_the_response_into_that_error(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_interceptor(Arc::new(FailingAfterInterceptor))
        .build()?;

    let result = send(&client, ping()).await;

    assert!(
        matches!(
            &result,
            Err(CallError::Transport {
                code: StatusCode::DataLoss,
                ..
            })
        ),
        "got {result:?}"
    );
    assert_eq!(
        peer.get_recorded_requests().len(),
        1,
        "the call itself was made"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_trace_context
#[tokio::test]
async fn test_with_trace_context_static_injects_the_traceparent_the_peer_receives_happy(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_trace_context(TraceContextSource::Static {
            traceparent: TRACEPARENT.to_string(),
            tracestate: Some("vendor=1".to_string()),
        })
        .build()?;

    send(&client, ping()).await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("traceparent"), Some(TRACEPARENT));
    assert_eq!(request.get_header("tracestate"), Some("vendor=1"));
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_trace_context
#[tokio::test]
async fn test_with_trace_context_static_keeps_a_traceparent_the_request_already_carries_edge(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_trace_context(TraceContextSource::Static {
            traceparent: TRACEPARENT.to_string(),
            tracestate: None,
        })
        .build()?;
    let carried = "00-11111111111111111111111111111111-2222222222222222-01";
    let metadata = Metadata::create(vec![MetadataEntry::create("traceparent", carried)?]);

    send(
        &client,
        CallFixture::request_with(b"ping", SECONDS, metadata),
    )
    .await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("traceparent"), Some(carried));
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_trace_context
#[tokio::test]
async fn test_with_trace_context_pass_through_adds_no_traceparent_edge(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_trace_context(grpc_egress_transport::TraceContextSource::PassThrough)
        .build()?;

    send(&client, ping()).await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("traceparent"), None);
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_with_channel_config_applies_endpoint_plaintext_and_decoding_limit_happy(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1025])).await?;
    let closed = Fixture::reserve_closed_port()?;
    let config = config_builder(peer.get_endpoint())
        .allow_plaintext()
        .with_max_message_bytes(1024)
        .build()?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(closed.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_channel_config(&config)
        .build()?;

    let result = send(&client, ping()).await;

    assert_eq!(
        peer.get_recorded_requests().len(),
        1,
        "the endpoint of the config must replace the factory's"
    );
    assert!(
        is_out_of_range(&result),
        "the limit of the config must be applied to responses, got {result:?}"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_with_channel_config_max_message_bytes_also_limits_the_request_error(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let config = config_builder(peer.get_endpoint())
        .allow_plaintext()
        .with_max_message_bytes(1024)
        .build()?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_channel_config(&config)
        .build()?;

    let result = send(&client, CallFixture::request(&[1u8; 1025], SECONDS)).await;

    assert_eq!(
        result.as_ref().err().map(CallError::get_status_code),
        Some(StatusCode::ResourceExhausted),
        "got {result:?}"
    );
    assert!(peer.get_recorded_requests().is_empty());
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_with_channel_config_compression_gzip_sends_and_announces_gzip_happy(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?;
    let config = config_builder(peer.get_endpoint())
        .allow_plaintext()
        .with_compression(CompressionMode::Gzip)
        .build()?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_channel_config(&config)
        .build()?;

    send(&client, ping()).await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("grpc-encoding"), Some("gzip"));
    assert!(request
        .get_header("grpc-accept-encoding")
        .is_some_and(|announced| announced.contains("gzip")));
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_with_channel_config_requiring_tls_for_a_plaintext_endpoint_returns_plaintext_rejected_error(
) -> Result<(), Box<dyn Error>> {
    let config = config_builder("http://127.0.0.1:1").build()?;

    let result = TonicGrpcEgressBuilderSpiFactory::new("https://elsewhere.example:443")
        .build(BuildGrpcEgressBuilderRequest)
        .with_channel_config(&config)
        .build();

    assert!(matches!(
        result,
        Err(GrpcChannelConfigError::PlaintextRejected(_))
    ));
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_with_channel_config_then_a_single_option_the_later_call_wins_edge(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1025])).await?;
    let config = config_builder(peer.get_endpoint())
        .allow_plaintext()
        .with_max_message_bytes(1024)
        .build()?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_channel_config(&config)
        .with_max_decoding_message_size(4096)
        .build()?;

    let response = send(&client, ping()).await?;

    assert_eq!(response.len(), 1025);
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_max_decoding_message_size
#[tokio::test]
async fn test_with_max_decoding_message_size_called_twice_the_later_value_wins_edge(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1025])).await?;
    let factory = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint());
    let loosened = factory
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_decoding_message_size(1024)
        .with_max_decoding_message_size(4096)
        .build()?;
    let tightened = factory
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_decoding_message_size(4096)
        .with_max_decoding_message_size(1024)
        .build()?;

    let accepted = send(&loosened, ping()).await;
    let refused = send(&tightened, ping()).await;

    assert!(
        accepted.is_ok(),
        "the later, larger value must win: {accepted:?}"
    );
    assert!(
        is_out_of_range(&refused),
        "the later, smaller value must win: {refused:?}"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_accept_compression
#[tokio::test]
async fn test_options_set_in_a_different_order_behave_the_same_edge() -> Result<(), Box<dyn Error>>
{
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1025])).await?;
    let one = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_decoding_message_size(1024)
        .with_accept_compression(Compression::Gzip)
        .build()?;
    let other = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_accept_compression(Compression::Gzip)
        .with_max_decoding_message_size(1024)
        .with_plaintext_allowed()
        .build()?;

    let first = send(&one, ping()).await;
    let second = send(&other, ping()).await;

    assert!(is_out_of_range(&first), "got {first:?}");
    assert!(is_out_of_range(&second), "got {second:?}");
    Ok(())
}

fn config_builder(
    endpoint: impl Into<String>,
) -> impl grpc_egress_transport::GrpcChannelConfigBuilder {
    new_config_builder().with_endpoint(endpoint)
}

fn new_config_builder() -> impl grpc_egress_transport::GrpcChannelConfigBuilder {
    use grpc_egress_transport::GrpcChannelConfigBuilderFactory;
    grpc_egress_transport_std::StdGrpcChannelConfigBuilderFactory
        .build(grpc_egress_transport::BuildGrpcChannelConfigBuilderRequest)
}
