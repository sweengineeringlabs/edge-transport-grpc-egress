//! `CancellationScenario` -- ruling 2 of ADR-008: dropping a call's future cancels it, and a token can too.

use proto_pattern::{
    CallError, CallOptions, CallRawRequestBuilder, CallRawResponse, CallRequest, MethodPath,
    RawTransport, StatusCode,
};
use std::error::Error;
use std::time::{Duration, Instant};

use grpc_egress_transport::{
    BuildGrpcCancellationRequest, BuildGrpcEgressBuilderRequest, GrpcCancellationFactory,
    GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_std::{
    StdGrpcCancellationFactory, TokenCancellation, TonicGrpcEgressBuilderSpiFactory,
};
use proto_test_peer::{RunningPeer, Say};
use tokio_util::sync::CancellationToken;

use crate::ScenarioReport;

/// How long the peer takes to answer a slow call.
const PEER_DELAY_MS: u32 = 5000;
/// The longest a call may take to end once it is cancelled.
const CANCELLED_WITHIN: Duration = Duration::from_secs(2);
/// The longest the scenario waits for the peer to notice a cancelled call.
const NOTICE_LIMIT: Duration = Duration::from_secs(3);

/// Against a real `tonic` server that answers slowly: a call whose future is dropped ends on the server; a call
/// bound to a token that fires fails with `Cancelled` long before the peer answers; a token that never fires
/// changes nothing.
pub struct CancellationScenario;

impl CancellationScenario {
    /// Run the scenario.
    ///
    /// # Errors
    ///
    /// An error of building a client or of a call that should have worked.
    pub async fn run(&self) -> Result<ScenarioReport, Box<dyn Error>> {
        let mut report = ScenarioReport::new("cancellation");
        let peer = RunningPeer::start().await;
        let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .build()?;

        let path = Self::path()?;
        let abandoned = tokio::time::timeout(Duration::from_millis(100), async {
            client
                .call_raw({
                    let (message, options) = Self::request(PEER_DELAY_MS).into_parts();
                    CallRawRequestBuilder::new()
                        .with_path(path)
                        .with_message(message)
                        .with_options(options)
                        .build()
                })
                .await
                .map(CallRawResponse::into_response)
        })
        .await;
        report.record(
            "dropping the future of a call ends the call on the peer",
            abandoned.is_err() && Self::wait_until_idle(&peer).await,
        );

        let token = CancellationToken::new();
        let bound = StdGrpcCancellationFactory::new().build(BuildGrpcCancellationRequest {
            inner: &client,
            signal: TokenCancellation::from(token.clone()),
        });
        let canceller = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            token.cancel();
        });
        let started = Instant::now();
        let result = bound
            .call_raw({
                let (message, options) = Self::request(PEER_DELAY_MS).into_parts();
                CallRawRequestBuilder::new()
                    .with_path(Self::path()?)
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response);
        canceller.await?;
        report.record(
            "a fired token fails the call with Cancelled long before the peer answers",
            matches!(
                result,
                Err(CallError::Transport {
                    code: StatusCode::Cancelled,
                    ..
                })
            ) && started.elapsed() < CANCELLED_WITHIN,
        );

        let quiet = StdGrpcCancellationFactory::new().build(BuildGrpcCancellationRequest {
            inner: &client,
            signal: TokenCancellation::from(CancellationToken::new()),
        });
        let answered = quiet
            .call_raw({
                let (message, options) = Self::request(0).into_parts();
                CallRawRequestBuilder::new()
                    .with_path(Self::path()?)
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response);
        report.record(
            "a token that never fires leaves the call alone",
            answered.is_ok(),
        );
        Ok(report)
    }

    fn path() -> Result<MethodPath, Box<dyn Error>> {
        Ok(MethodPath::create("/test.Peer/Echo")?)
    }

    fn request(sleep_ms: u32) -> CallRequest<Vec<u8>> {
        let message = Say {
            text: "slow".to_string(),
            fail_with: 0,
            sleep_ms,
        };
        CallRequest::new(
            prost::Message::encode_to_vec(&message),
            CallOptions::default(),
        )
    }

    /// Waits until the peer handles no call, or `NOTICE_LIMIT` passes.
    async fn wait_until_idle(peer: &RunningPeer) -> bool {
        let started = Instant::now();
        while started.elapsed() < NOTICE_LIMIT {
            if peer.get_in_flight() == 0 {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        false
    }
}
