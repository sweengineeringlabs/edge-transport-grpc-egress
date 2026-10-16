//! `DecoratorScenario` -- retry and breaker applied by the root, after `build()`.

use std::error::Error;
use std::time::Duration;

use grpc_egress_breaker::{
    BreakerObservable, BreakerState, BuildGrpcBreakerRequest, GrpcBreakerConfig,
    GrpcBreakerFactory, ObserveStateRequest,
};
use grpc_egress_breaker_std::StdGrpcBreakerFactory;
use grpc_egress_retry::{BuildGrpcRetryRequest, GrpcRetryConfig, GrpcRetryFactory};
use grpc_egress_retry_std::StdGrpcRetryFactory;
use grpc_egress_test_support::PeerBehaviour;
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcEgress, GrpcEgressBuilder, GrpcEgressBuilderFactory,
    GrpcEgressError, GrpcRequest,
};
use grpc_egress_transport_std::{GrpcRequestFactory, TonicGrpcEgressBuilderSpiFactory};

use crate::{EchoServer, ScenarioReport};

const ATTEMPTS: usize = 3;

/// Wraps a built client in the retry decorator and then the breaker decorator, against a peer that
/// always answers `Unavailable`: a call is retried, and after two failed calls the breaker opens and
/// short-circuits the third.
pub struct DecoratorScenario;

impl DecoratorScenario {
    /// Run the scenario.
    ///
    /// # Errors
    ///
    /// An error of starting the peer or building a client.
    pub async fn run(&self) -> Result<ScenarioReport, Box<dyn Error>> {
        let mut report = ScenarioReport::new("decorators applied by the root");
        let server = EchoServer::start_with(PeerBehaviour::ReplyStatus {
            status: "14".to_string(),
            message: Some("down".to_string()),
            retry_after_secs: None,
        })
        .await?;
        let bare = TonicGrpcEgressBuilderSpiFactory::new(server.get_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .build()?;
        let retrying = StdGrpcRetryFactory::new(GrpcRetryConfig {
            max_attempts: ATTEMPTS as u32,
            initial_backoff_ms: 1,
            backoff_multiplier: 1.0,
            jitter_factor: 0.0,
            max_backoff_ms: 1,
            ..GrpcRetryConfig::default()
        })
        .build(BuildGrpcRetryRequest { inner: bare });
        let client = StdGrpcBreakerFactory::new(GrpcBreakerConfig {
            failure_threshold: 2,
            cool_down_seconds: 60,
            half_open_probe_count: 1,
        })
        .build(BuildGrpcBreakerRequest { inner: retrying })?;

        let _ = client.call_unary(Self::request()).await;
        report.record(
            "a failing call is retried up to the attempt limit",
            server.count_requests() == ATTEMPTS,
        );

        let _ = client.call_unary(Self::request()).await;
        let state = client.state(ObserveStateRequest).await?.state;
        report.record(
            "the breaker opens after the threshold of failed calls",
            matches!(state, BreakerState::Open { .. }),
        );

        let hits = server.count_requests();
        let refused = client.call_unary(Self::request()).await;
        report.record(
            "an open breaker answers Unavailable without reaching the peer",
            matches!(refused, Err(GrpcEgressError::Unavailable(_)))
                && server.count_requests() == hits,
        );
        Ok(report)
    }

    fn request() -> GrpcRequest {
        GrpcRequest::new(
            "pkg.Service/Method",
            b"ping".to_vec(),
            Duration::from_secs(5),
        )
    }
}
