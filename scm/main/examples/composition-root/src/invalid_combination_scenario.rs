//! `InvalidCombinationScenario` -- invalid construction and incompatible build options are refused clearly.

use std::error::Error;

use grpc_egress_transport::{
    BackendTarget, BalancingStrategy, BuildGrpcEgressBuilderRequest, GrpcEgressBuilder,
    GrpcEgressBuilderFactory, LoadbalancerSettings,
};
use grpc_egress_transport_spi::TonicGrpcEgressBuilderSpiFactory;
use proto_pattern::{TlsIdentity, TlsSettings};

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
        let identity = TlsIdentity::create(b"certificate".to_vec(), b"key".to_vec());
        let mtls_and_plaintext = Self::builder("http://127.0.0.1:1")
            .with_plaintext_allowed()
            .with_tls(TlsSettings::create(None, Vec::new(), Some(identity)))
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
            .with_loadbalancer(Self::one_backend()?)
            .build()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        report.record(
            "an endpoint with a load balancer names both options",
            endpoint_and_balancer.contains("'endpoint'")
                && endpoint_and_balancer.contains("'loadbalancer'"),
        );

        let zero_limit = Self::builder("http://127.0.0.1:1")
            .with_plaintext_allowed()
            .with_max_decoding_message_size(0)
            .build()
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        report.record(
            "a zero message limit names the option",
            zero_limit.contains("max_decoding_message_size"),
        );

        let no_backends = LoadbalancerSettings::try_new(BalancingStrategy::RoundRobin, Vec::new())
            .err()
            .unwrap_or_default();
        report.record(
            "a balancer without backends names the missing backends",
            no_backends.contains("backends"),
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

    fn one_backend() -> Result<LoadbalancerSettings, Box<dyn Error>> {
        Ok(LoadbalancerSettings::try_new(
            BalancingStrategy::RoundRobin,
            vec![BackendTarget::try_new("http://127.0.0.1:2", 1)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?],
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?)
    }
}
