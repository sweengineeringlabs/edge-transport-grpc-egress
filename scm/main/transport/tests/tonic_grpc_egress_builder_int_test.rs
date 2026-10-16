//! Tests of the builder of the `hyper`-backed client, through its public factory, against a real
//! in-process peer: every option must reach the call it describes, and none may be ignored.

use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use edge_transport_grpc_egress_transport::{
    AfterCallRequest, BuildGrpcEgressBuilderRequest, CompressionMode, GrpcChannelConfig,
    GrpcChannelConfigError, GrpcEgress, GrpcEgressBuilder, GrpcEgressBuilderFactory,
    GrpcEgressError, GrpcEgressInterceptor, GrpcRequest, GrpcStatusCode, KeepAliveConfig,
    TonicGrpcEgressBuilderSpiFactory, TraceContextSource,
};
use grpc_egress_test_support::{Fixture, PeerBehaviour, PeerServer};

const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

/// Appends its tag to the `x-order` header: the header shows the order interceptors ran in.
struct TagInterceptor {
    tag: &'static str,
}

impl GrpcEgressInterceptor for TagInterceptor {
    fn before_call(&self, req: &mut GrpcRequest) -> Result<(), GrpcEgressError> {
        req.metadata
            .entry("x-order".to_string())
            .and_modify(|v| {
                v.push(',');
                v.push_str(self.tag);
            })
            .or_insert_with(|| self.tag.to_string());
        Ok(())
    }

    fn after_call(&self, _req: AfterCallRequest<'_>) -> Result<(), GrpcEgressError> {
        Ok(())
    }
}

/// Refuses every call before it is sent.
struct RefusingInterceptor;

impl GrpcEgressInterceptor for RefusingInterceptor {
    fn before_call(&self, _req: &mut GrpcRequest) -> Result<(), GrpcEgressError> {
        Err(GrpcEgressError::Internal(
            "refused by the interceptor".into(),
        ))
    }

    fn after_call(&self, _req: AfterCallRequest<'_>) -> Result<(), GrpcEgressError> {
        Ok(())
    }
}

fn new_request() -> GrpcRequest {
    GrpcRequest::new(
        "pkg.Service/Method",
        b"ping".to_vec(),
        Duration::from_secs(5),
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
        .with_max_message_bytes(1024)
        .build()?;
    let uncapped = factory
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .build()?;

    let refused = capped.call_unary(new_request()).await;
    let accepted = uncapped.call_unary(new_request()).await?;

    assert!(
        matches!(
            refused,
            Err(GrpcEgressError::Status(
                GrpcStatusCode::ResourceExhausted,
                _
            ))
        ),
        "the builder with a cap must refuse, got {refused:?}"
    );
    assert_eq!(
        accepted.body.len(),
        1025,
        "the second builder must not share the first one's cap"
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

    client.call_unary(new_request()).await?;

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

    let response = client.call_unary(new_request()).await?;

    assert_eq!(response.body, b"ping");
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_max_message_bytes
#[tokio::test]
async fn test_with_max_message_bytes_response_at_the_cap_returns_the_message_happy(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1024])).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_message_bytes(1024)
        .build()?;

    let response = client.call_unary(new_request()).await?;

    assert_eq!(response.body.len(), 1024);
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_max_message_bytes
#[tokio::test]
async fn test_with_max_message_bytes_response_over_the_cap_returns_resource_exhausted_error(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1025])).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_message_bytes(1024)
        .build()?;

    let result = client.call_unary(new_request()).await;

    assert!(
        matches!(
            result,
            Err(GrpcEgressError::Status(
                GrpcStatusCode::ResourceExhausted,
                _
            ))
        ),
        "expected ResourceExhausted, got {result:?}"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_interceptor
#[tokio::test]
async fn test_with_interceptor_header_it_adds_reaches_the_peer_happy() -> Result<(), Box<dyn Error>>
{
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_interceptor(Arc::new(TagInterceptor { tag: "a" }))
        .build()?;

    client.call_unary(new_request()).await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("x-order"), Some("a"));
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_interceptor
#[tokio::test]
async fn test_with_interceptor_two_added_run_in_the_order_they_were_added_edge(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Echo).await?;
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_interceptor(Arc::new(TagInterceptor { tag: "first" }))
        .with_interceptor(Arc::new(TagInterceptor { tag: "second" }))
        .build()?;

    client.call_unary(new_request()).await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("x-order"), Some("first,second"));
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

    let result = client.call_unary(new_request()).await;

    assert!(
        matches!(result, Err(GrpcEgressError::Internal(ref m)) if m == "refused by the interceptor"),
        "expected the interceptor's error, got {result:?}"
    );
    assert!(
        peer.get_recorded_requests().is_empty(),
        "a refused call must not reach the peer"
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

    client.call_unary(new_request()).await?;

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

    client
        .call_unary(new_request().with_header("traceparent", carried))
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
        .with_trace_context(TraceContextSource::PassThrough)
        .build()?;

    client.call_unary(new_request()).await?;

    let requests = peer.get_recorded_requests();
    let request = requests.first().ok_or("the peer received no request")?;
    assert_eq!(request.get_header("traceparent"), None);
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_with_channel_config_applies_endpoint_plaintext_and_cap_happy(
) -> Result<(), Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1025])).await?;
    let closed = Fixture::reserve_closed_port()?;
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_max_message_bytes(1024);
    let client = TonicGrpcEgressBuilderSpiFactory::new(closed.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_channel_config(&config)
        .build()?;

    let result = client.call_unary(new_request()).await;

    assert_eq!(
        peer.get_recorded_requests().len(),
        1,
        "the endpoint of the config must replace the factory's"
    );
    assert!(
        matches!(
            result,
            Err(GrpcEgressError::Status(
                GrpcStatusCode::ResourceExhausted,
                _
            ))
        ),
        "the cap of the config must be applied, got {result:?}"
    );
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_with_channel_config_requiring_tls_for_a_plaintext_endpoint_returns_plaintext_rejected_error(
) -> Result<(), Box<dyn Error>> {
    let config = GrpcChannelConfig::new("http://127.0.0.1:1");

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
    let config = GrpcChannelConfig::new(peer.get_endpoint())
        .allow_plaintext()
        .with_max_message_bytes(1024);
    let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_channel_config(&config)
        .with_max_message_bytes(4096)
        .build()?;

    let response = client.call_unary(new_request()).await?;

    assert_eq!(response.body.len(), 1025);
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_channel_config
#[tokio::test]
async fn test_with_channel_config_keep_alive_is_refused_until_the_client_applies_it_error(
) -> Result<(), Box<dyn Error>> {
    let config = GrpcChannelConfig::new("https://service.example:443")
        .with_keep_alive(KeepAliveConfig::default());

    let result = TonicGrpcEgressBuilderSpiFactory::new("https://service.example:443")
        .build(BuildGrpcEgressBuilderRequest)
        .with_channel_config(&config)
        .build();

    assert!(matches!(
        result,
        Err(GrpcChannelConfigError::OptionNotApplied("keep_alive"))
    ));
    Ok(())
}

/// @covers: GrpcEgressBuilder::with_compression
#[tokio::test]
async fn test_options_set_in_a_different_order_behave_the_same_edge() -> Result<(), Box<dyn Error>>
{
    Fixture::ensure_tls_provider();
    let peer = PeerServer::start(PeerBehaviour::Reply(vec![7u8; 1025])).await?;
    let one = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_max_message_bytes(1024)
        .with_compression(CompressionMode::None)
        .build()?;
    let other = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_compression(CompressionMode::None)
        .with_max_message_bytes(1024)
        .with_plaintext_allowed()
        .build()?;

    let first = one.call_unary(new_request()).await;
    let second = other.call_unary(new_request()).await;

    assert!(matches!(
        first,
        Err(GrpcEgressError::Status(
            GrpcStatusCode::ResourceExhausted,
            _
        ))
    ));
    assert!(matches!(
        second,
        Err(GrpcEgressError::Status(
            GrpcStatusCode::ResourceExhausted,
            _
        ))
    ));
    Ok(())
}
