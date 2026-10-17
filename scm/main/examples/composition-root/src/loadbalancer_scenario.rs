//! `LoadbalancerScenario` -- one client over several backends.

use std::error::Error;
use std::time::Duration;

use grpc_egress_test_support::{CallFixture, ClosedPort, PeerBehaviour};
use grpc_egress_transport::{
    BackendTarget, BalancingStrategy, BuildGrpcEgressBuilderRequest, GrpcEgressBuilder,
    GrpcEgressBuilderFactory, LoadbalancerSettings,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;
use proto_pattern::{CallRequest, RawTransport, Transport};

use crate::{EchoServer, ScenarioReport};

/// Balances calls over two backends, then over a backend that is down and one that is up.
pub struct LoadbalancerScenario;

impl LoadbalancerScenario {
    /// Run the scenario.
    ///
    /// # Errors
    ///
    /// An error of starting a peer or building a client.
    pub async fn run(&self) -> Result<ScenarioReport, Box<dyn Error>> {
        let mut report = ScenarioReport::new("load balancer");
        let one = EchoServer::start_with(PeerBehaviour::Reply(b"one".to_vec())).await?;
        let two = EchoServer::start_with(PeerBehaviour::Reply(b"two".to_vec())).await?;
        let client = Self::balanced(&[one.get_endpoint(), two.get_endpoint()])?;
        for _ in 0..6 {
            let _ = client
                .call_raw(CallFixture::path("pkg.Service/Method"), Self::request())
                .await;
        }
        report.record(
            "calls reach both backends",
            one.count_requests() > 0 && two.count_requests() > 0,
        );

        let dead = ClosedPort::reserve()?;
        let live = EchoServer::start_with(PeerBehaviour::Reply(b"live".to_vec())).await?;
        let client = Self::balanced(&[dead.get_endpoint(), live.get_endpoint()])?;
        let mut answered = 0;
        for _ in 0..8 {
            if client
                .call_raw(CallFixture::path("pkg.Service/Method"), Self::request())
                .await
                .is_ok()
            {
                answered += 1;
            }
        }
        report.record("a backend that is down is avoided", answered >= 7);
        Ok(report)
    }

    fn balanced(urls: &[String]) -> Result<impl Transport + RawTransport, Box<dyn Error>> {
        Ok(TonicGrpcEgressBuilderSpiFactory::new("")
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .with_loadbalancer(LoadbalancerSettings {
                strategy: BalancingStrategy::RoundRobin,
                backends: urls
                    .iter()
                    .map(|url| BackendTarget {
                        url: url.clone(),
                        weight: 1,
                    })
                    .collect(),
            })
            .build()?)
    }

    fn request() -> CallRequest<Vec<u8>> {
        CallFixture::request(b"ping", Duration::from_secs(5))
    }
}
