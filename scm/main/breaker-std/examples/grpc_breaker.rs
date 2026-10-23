//! Reusable example: drive real calls through [`StdGrpcBreakerFactory`] so the circuit-breaker state machine
//! (`DefaultBreakerTransition`, moved to the shared `edge-transport-breaker` crate) actually executes â€” not just
//! constructs.
//!
//! The upstream is a real in-process peer that answers `UNAVAILABLE`; the breaker is wrapped around a real client
//! built through the public builder. Recovery (half-open, then closed) needs an upstream that changes its answer
//! and is proved by `tests/breaker_int_test.rs`.
//!
//! [`StdGrpcBreakerFactory`]: grpc_egress_breaker_std::StdGrpcBreakerFactory
use grpc_egress_breaker::proto_pattern::{
    CallError, CallRawRequestBuilder, CallRawResponse, RawTransport, StatusCode,
};
use std::time::Duration;

use grpc_egress_breaker::{
    BreakerObservable, BreakerState, BuildGrpcBreakerRequest, GrpcBreakerConfig,
    GrpcBreakerFactory, ObserveStateRequest,
};
use grpc_egress_breaker_std::StdGrpcBreakerFactory;
use grpc_egress_test_support::{CallFixture, ClientFixture, Fixture, PeerBehaviour, PeerServer};
use grpc_egress_transport::GrpcChannelConfigBuilder as _;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Fixture::ensure_tls_provider();
    let upstream = PeerServer::start(PeerBehaviour::ReplyStatus {
        status: StatusCode::Unavailable.get_code().to_string(),
        message: Some("upstream is down".to_string()),
        retry_after_secs: None,
    })
    .await?;
    let client = ClientFixture::from_config(
        &config_builder(upstream.get_endpoint())
            .allow_plaintext()
            .build()?,
    )?;
    let client = StdGrpcBreakerFactory::new(GrpcBreakerConfig::try_new(true, 2, 30, 1)?)
        .build(BuildGrpcBreakerRequest { inner: client })?;
    let call = || async {
        client
            .call_raw({
                let (message, options) =
                    CallFixture::request(b"hi", Duration::from_secs(1)).into_parts();
                CallRawRequestBuilder::new()
                    .with_path(CallFixture::path("example.Service/Method"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)
    };

    // Two real failures reach the upstream and trip the breaker.
    for attempt in 1..=2 {
        let result = call().await.map(|response| response.get_message().len());
        println!(
            "attempt {attempt}: {result:?} (upstream calls so far: {})",
            upstream.get_recorded_requests().len()
        );
        assert!(matches!(
            result,
            Err(CallError::Status {
                code: StatusCode::Unavailable,
                ..
            })
        ));
    }
    let state = client.state(ObserveStateRequest::new()).await?.state();
    assert!(matches!(state, BreakerState::Open { .. }));

    // Third call: the circuit is now open. This is the real proof that `DefaultBreakerTransition::admit` executed
    // and changed state â€” the call is rejected WITHOUT reaching the upstream (call count unchanged).
    let before = upstream.get_recorded_requests().len();
    let result = call().await.map(|response| response.get_message().len());
    let after = upstream.get_recorded_requests().len();
    println!("attempt 3 (circuit open): {result:?}");
    assert!(matches!(
        result,
        Err(CallError::Transport {
            code: StatusCode::Unavailable,
            ..
        })
    ));
    assert_eq!(
        before, after,
        "an open circuit must reject without reaching the upstream"
    );
    println!(
        "confirmed: circuit rejected the call without invoking the upstream ({after} total upstream calls)"
    );
    Ok(())
}

fn config_builder(
    endpoint: impl Into<String>,
) -> impl grpc_egress_transport::GrpcChannelConfigBuilder {
    use grpc_egress_transport::GrpcChannelConfigBuilderFactory;
    grpc_egress_transport_std::StdGrpcChannelConfigBuilderFactory
        .build(grpc_egress_transport::BuildGrpcChannelConfigBuilderRequest)
        .with_endpoint(endpoint)
}
