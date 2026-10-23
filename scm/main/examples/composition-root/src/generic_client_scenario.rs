//! `GenericClientScenario` -- ruling 1 of ADR-008: a client is a generic bound, never a `dyn`.

use std::error::Error;

use grpc_egress_breaker::{BuildGrpcBreakerRequest, GrpcBreakerConfig, GrpcBreakerFactory};
use grpc_egress_breaker_spi::StdGrpcBreakerFactory;
use grpc_egress_retry::{BuildGrpcRetryRequest, GrpcRetryConfig, GrpcRetryFactory};
use grpc_egress_retry_spi::StdGrpcRetryFactory;
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_spi::TonicGrpcEgressBuilderSpiFactory;
use proto_pattern::{CallOptions, CallRequest, Transport};

use crate::echo_text::{EchoText, Text};
use crate::{CompositionRoot, EchoServer, ScenarioReport};

/// Passes the bare client, the client wrapped in retry, and that wrapped in the breaker to the same generic function
/// of the root, and makes a typed call through the wrapped one: every layer is a `Transport` and a `RawTransport`,
/// so the root holds no `dyn` and pays for no virtual call or boxed future.
pub struct GenericClientScenario;

impl GenericClientScenario {
    /// Run the scenario.
    ///
    /// # Errors
    ///
    /// An error of starting the peer, building a client or of a call that should have worked.
    pub async fn run(&self) -> Result<ScenarioReport, Box<dyn Error>> {
        let mut report = ScenarioReport::new("generic client");
        let server = EchoServer::start().await?;
        let build = || -> Result<_, Box<dyn Error>> {
            Ok(TonicGrpcEgressBuilderSpiFactory::new(server.get_endpoint())
                .build(BuildGrpcEgressBuilderRequest)
                .with_plaintext_allowed()
                .build()?)
        };

        let bare = build()?;
        report.record(
            "the bare client is passed to a generic function",
            CompositionRoot::ping(&bare, b"bare").await? == b"bare",
        );

        let retrying = StdGrpcRetryFactory::new(GrpcRetryConfig::default())
            .build(BuildGrpcRetryRequest { inner: build()? });
        report.record(
            "the same function takes the client wrapped in retry",
            CompositionRoot::ping(&retrying, b"retry").await? == b"retry",
        );

        let guarded = StdGrpcBreakerFactory::new(GrpcBreakerConfig::default())
            .build(BuildGrpcBreakerRequest { inner: retrying })?;
        report.record(
            "and the one wrapped in the breaker on top of it",
            CompositionRoot::ping(&guarded, b"breaker").await? == b"breaker",
        );

        let typed = guarded
            .call::<EchoText>(CallRequest::new(
                Text("typed".to_string()),
                CallOptions::default(),
            ))
            .await?;
        report.record(
            "a typed call goes through the same wrapped client",
            typed.get_message() == &Text("typed".to_string()),
        );
        Ok(report)
    }
}
