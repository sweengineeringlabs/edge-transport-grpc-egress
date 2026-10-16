//! `InvalidCombinationScenario` -- every refusal of `build()`, each naming its options.

use std::error::Error;

use grpc_egress_transport::{
    BackendTarget, BalancingStrategy, BuildGrpcEgressBuilderRequest, GrpcEgressBuilder,
    GrpcEgressBuilderFactory, LoadbalancerSettings, MtlsConfig,
};
use grpc_egress_transport_std::{MtlsConfigFactory, TonicGrpcEgressBuilderSpiFactory};

use crate::ScenarioReport;

/// Produces each error `build()` returns for options that cannot work together or values that cannot
/// be used, and looks for the names of the options in its text.
pub struct InvalidCombinationScenario;

impl InvalidCombinationScenario {
    /// Run the scenario.
    ///
    /// # Errors
    ///
    /// None today: a build that unexpectedly succeeds is recorded as an effect that did not happen.
    pub async fn run(&self) -> Result<ScenarioReport, Box<dyn Error>> {
        let mut report = ScenarioReport::new("invalid combinations");
        let mtls_and_plaintext = Self::builder("http://127.0.0.1:1")
            .with_plaintext_allowed()
            .with_mtls(MtlsConfig::new("client.pem", "client.key"))
            .build()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        report.record(
            "mtls with plaintext_allowed names both options",
            mtls_and_plaintext.contains("'mtls'")
                && mtls_and_plaintext.contains("'plaintext_allowed'"),
        );

        let endpoint_and_balancer = Self::builder("")
            .with_endpoint("http://127.0.0.1:1")
            .with_plaintext_allowed()
            .with_loadbalancer(Self::one_backend())
            .build()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        report.record(
            "an endpoint with a load balancer names both options",
            endpoint_and_balancer.contains("'endpoint'")
                && endpoint_and_balancer.contains("'loadbalancer'"),
        );

        let zero_cap = Self::builder("http://127.0.0.1:1")
            .with_plaintext_allowed()
            .with_max_message_bytes(0)
            .build()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        report.record(
            "a zero message cap names the option",
            zero_cap.contains("max_message_bytes"),
        );

        let no_backends = Self::builder("")
            .with_plaintext_allowed()
            .with_loadbalancer(LoadbalancerSettings {
                strategy: BalancingStrategy::RoundRobin,
                backends: Vec::new(),
            })
            .build()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        report.record(
            "a balancer without backends names the option",
            no_backends.contains("loadbalancer"),
        );

        let plaintext = Self::builder("http://127.0.0.1:1")
            .build()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        report.record(
            "a plaintext endpoint without the opt-in is rejected",
            plaintext.contains("http://127.0.0.1:1"),
        );
        Ok(report)
    }

    fn builder(endpoint: &str) -> impl GrpcEgressBuilder {
        TonicGrpcEgressBuilderSpiFactory::new(endpoint).build(BuildGrpcEgressBuilderRequest)
    }

    fn one_backend() -> LoadbalancerSettings {
        LoadbalancerSettings {
            strategy: BalancingStrategy::RoundRobin,
            backends: vec![BackendTarget {
                url: "http://127.0.0.1:2".to_string(),
                weight: 1,
            }],
        }
    }
}
