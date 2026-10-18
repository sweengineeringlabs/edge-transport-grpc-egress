//! `LoadedConfigScenario` -- a client assembled from a loaded configuration.

use std::error::Error;
use std::time::Duration;

use grpc_egress_test_support::CallFixture;
use grpc_egress_transport::GrpcEgressBuilder;
use proto_pattern::{CallRequest, Compression, RawTransport};

use crate::{CompositionRoot, EchoServer, ScenarioReport};

/// Loads a TOML configuration, builds a client from it, then overrides one field with one option.
pub struct LoadedConfigScenario;

impl LoadedConfigScenario {
    /// Run the scenario.
    ///
    /// # Errors
    ///
    /// An error of starting the peer, loading the configuration, building a client or a call.
    pub async fn run(&self) -> Result<ScenarioReport, Box<dyn Error>> {
        let mut report = ScenarioReport::new("loaded config");
        let server = EchoServer::start().await?;
        let root = CompositionRoot::new(format!(
            "endpoint = \"{}\"\ntls_required = false\nmax_message_bytes = 65536\ncompression = \"Gzip\"\n",
            server.get_endpoint()
        ));

        let from_config = root.assemble()?.build()?;
        from_config
            .call_raw(CallFixture::path("pkg.Service/Method"), Self::request())
            .await?;
        report.record(
            "the compression of the file applies",
            server.get_last_header("grpc-encoding").as_deref() == Some("gzip"),
        );

        let overridden = root
            .assemble()?
            .with_send_compression(Compression::Zstd)
            .build()?;
        overridden
            .call_raw(CallFixture::path("pkg.Service/Method"), Self::request())
            .await?;
        report.record(
            "an option set after the file wins over it",
            server.get_last_header("grpc-encoding").as_deref() == Some("zstd"),
        );
        Ok(report)
    }

    fn request() -> CallRequest<Vec<u8>> {
        CallFixture::request(b"ping", Duration::from_secs(5))
    }
}
