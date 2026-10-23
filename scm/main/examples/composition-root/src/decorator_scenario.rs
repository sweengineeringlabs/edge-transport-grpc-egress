//! `DecoratorScenario` -- retry and breaker applied by the root, after `build()`.

use proto_pattern::{
    CallError, CallRawRequestBuilder, CallRawResponse, CallRequest, RawTransport, StatusCode,
};
use std::error::Error;
use std::time::Duration;

use grpc_egress_breaker::{
    BreakerObservable, BreakerState, BuildGrpcBreakerRequest, GrpcBreakerConfig,
    GrpcBreakerFactory, ObserveStateRequest,
};
use grpc_egress_breaker_spi::StdGrpcBreakerFactory;
use grpc_egress_retry::{
    BuildGrpcRetryConfigBuilderRequest, BuildGrpcRetryRequest, GrpcRetryConfigBuilder,
    GrpcRetryConfigBuilderFactory, GrpcRetryFactory,
};
use grpc_egress_retry_spi::{StdGrpcRetryConfigBuilderFactory, StdGrpcRetryFactory};
use grpc_egress_test_support::{CallFixture, PeerBehaviour};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_spi::TonicGrpcEgressBuilderSpiFactory;

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
        let retry_config = StdGrpcRetryConfigBuilderFactory
            .build(BuildGrpcRetryConfigBuilderRequest)
            .max_attempts(ATTEMPTS as u32)
            .initial_backoff_ms(1)
            .backoff_multiplier(1.0)
            .jitter_factor(0.0)
            .max_backoff_ms(1)
            .build()?;
        let retrying =
            StdGrpcRetryFactory::new(retry_config).build(BuildGrpcRetryRequest { inner: bare });
        let client = StdGrpcBreakerFactory::new(GrpcBreakerConfig::try_new(true, 2, 60, 1)?)
            .build(BuildGrpcBreakerRequest { inner: retrying })?;

        let _ = client
            .call_raw({
                let (message, options) = Self::request().into_parts();
                CallRawRequestBuilder::new()
                    .with_path(CallFixture::path("pkg.Service/Method"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response);
        report.record(
            "a failing call is retried up to the attempt limit",
            server.count_requests() == ATTEMPTS,
        );

        let _ = client
            .call_raw({
                let (message, options) = Self::request().into_parts();
                CallRawRequestBuilder::new()
                    .with_path(CallFixture::path("pkg.Service/Method"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response);
        let state = client.state(ObserveStateRequest::new()).await?.state();
        report.record(
            "the breaker opens after the threshold of failed calls",
            matches!(state, BreakerState::Open { .. }),
        );

        let hits = server.count_requests();
        let refused = client
            .call_raw({
                let (message, options) = Self::request().into_parts();
                CallRawRequestBuilder::new()
                    .with_path(CallFixture::path("pkg.Service/Method"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response);
        report.record(
            "an open breaker answers Unavailable without reaching the peer",
            matches!(
                refused,
                Err(CallError::Transport {
                    code: StatusCode::Unavailable,
                    ..
                })
            ) && server.count_requests() == hits,
        );
        Ok(report)
    }

    fn request() -> CallRequest<Vec<u8>> {
        CallFixture::request(b"ping", Duration::from_secs(5))
    }
}
