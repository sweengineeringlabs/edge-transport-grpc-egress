//! `ProstScenario` -- the prost codec option, and failing fast without its Cargo feature.

use std::error::Error;

use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcEgress, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;

use crate::{EchoServer, ScenarioReport};

/// With the Cargo feature `prost` the option works: calls carry `application/grpc+proto`. Without it
/// `build()` fails at once, naming the option and the feature.
pub struct ProstScenario;

impl ProstScenario {
    /// Run the scenario.
    ///
    /// # Errors
    ///
    /// An error of starting the peer or of a call that should have worked.
    pub async fn run(&self) -> Result<ScenarioReport, Box<dyn Error>> {
        let mut report = ScenarioReport::new("prost codec and fail fast");
        let server = EchoServer::start().await?;
        let built = TonicGrpcEgressBuilderSpiFactory::new(server.get_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .with_prost_codec()
            .build();
        Self::record_outcome(&mut report, &server, built).await?;
        Ok(report)
    }

    #[cfg(feature = "prost")]
    async fn record_outcome(
        report: &mut ScenarioReport,
        server: &EchoServer,
        built: Result<impl GrpcEgress, grpc_egress_transport::GrpcChannelConfigError>,
    ) -> Result<(), Box<dyn Error>> {
        use std::time::Duration;

        use grpc_egress_transport::GrpcRequest;
        use grpc_egress_transport_std::GrpcRequestFactory;

        let client = built?;
        client
            .call_unary(GrpcRequest::new(
                "pkg.Service/Method",
                b"ping".to_vec(),
                Duration::from_secs(5),
            ))
            .await?;
        report.record(
            "with the feature, calls carry the proto content type",
            server.get_last_header("content-type").as_deref() == Some("application/grpc+proto"),
        );
        Ok(())
    }

    #[cfg(not(feature = "prost"))]
    async fn record_outcome(
        report: &mut ScenarioReport,
        server: &EchoServer,
        built: Result<impl GrpcEgress, grpc_egress_transport::GrpcChannelConfigError>,
    ) -> Result<(), Box<dyn Error>> {
        let named = match built {
            Err(grpc_egress_transport::GrpcChannelConfigError::DisabledFeatureOptions {
                options,
            }) => options == vec![("prost_codec", "prost")],
            _ => false,
        };
        report.record(
            "without the feature, build names the option and the feature",
            named,
        );
        report.record("and nothing was sent", server.count_requests() == 0);
        Ok(())
    }
}
