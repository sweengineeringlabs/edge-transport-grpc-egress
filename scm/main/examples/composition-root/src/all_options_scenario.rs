//! `AllOptionsScenario` -- every option of the client builder, one after the other.

use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use grpc_egress_test_support::{PeerBehaviour, PeerServer, TestPki, TlsServerMode};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, CompressionMode, GrpcEgress, GrpcEgressBuilder,
    GrpcEgressBuilderFactory, GrpcEgressError, GrpcRequest, GrpcStatusCode, KeepAliveConfig,
    MtlsConfig, TraceContextSource,
};
use grpc_egress_transport_std::{
    GrpcRequestFactory, MtlsConfigFactory, TonicGrpcEgressBuilderSpiFactory,
};

use crate::{EchoServer, OrderInterceptor, ScenarioReport};

const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
const MESSAGE_CAP_BYTES: usize = 1024;

/// Sets every option of the builder: endpoint, plaintext rule, request timeout, message cap,
/// compression, keep-alive, mTLS, interceptor and trace context, and looks for the effect of each.
pub struct AllOptionsScenario;

impl AllOptionsScenario {
    /// Run the scenario.
    ///
    /// # Errors
    ///
    /// An error of starting a peer, building a client or of a call that should have worked.
    pub async fn run(&self) -> Result<ScenarioReport, Box<dyn Error>> {
        let mut report = ScenarioReport::new("every transport option");
        let server = EchoServer::start_with(PeerBehaviour::Reply(b"pong".to_vec())).await?;
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));
        let client = TonicGrpcEgressBuilderSpiFactory::new("")
            .build(BuildGrpcEgressBuilderRequest)
            .with_endpoint(server.get_endpoint())
            .with_plaintext_allowed()
            .with_request_timeout(Duration::from_secs(5))
            .with_max_message_bytes(MESSAGE_CAP_BYTES)
            .with_compression(CompressionMode::Gzip)
            .with_keep_alive(KeepAliveConfig {
                interval: Duration::from_secs(30),
                timeout: Duration::from_secs(10),
                permit_without_calls: false,
            })
            .with_interceptor(Arc::new(OrderInterceptor::new("only", log.clone())))
            .with_trace_context(TraceContextSource::Static {
                traceparent: TRACEPARENT.to_string(),
                tracestate: None,
            })
            .build()?;

        let echoed = client.call_unary(Self::request(b"ping")).await?;
        report.record("the peer's answer arrives", echoed.body == b"pong");
        report.record(
            "the message travelled compressed",
            server.is_last_message_compressed(),
        );
        report.record(
            "grpc-encoding announces gzip",
            server.get_last_header("grpc-encoding").as_deref() == Some("gzip"),
        );
        report.record(
            "the trace context reaches the peer",
            server.get_last_header("traceparent").as_deref() == Some(TRACEPARENT),
        );
        report.record(
            "the interceptor ran around the call",
            log.lock()
                .map(|entries| entries.len() == 2)
                .unwrap_or(false),
        );

        report.record(
            "a reply over the message cap is refused",
            Self::cap_refuses().await?,
        );

        report.record(
            "a slow peer hits the request timeout",
            Self::times_out().await?,
        );
        report.record(
            "a peer demanding a client certificate is answered",
            Self::mtls_works().await?,
        );
        Ok(report)
    }

    fn request(body: &[u8]) -> GrpcRequest {
        GrpcRequest::new("pkg.Service/Method", body.to_vec(), Duration::from_secs(5))
    }

    async fn cap_refuses() -> Result<bool, Box<dyn Error>> {
        let big =
            EchoServer::start_with(PeerBehaviour::Reply(vec![7u8; MESSAGE_CAP_BYTES * 2])).await?;
        let client = TonicGrpcEgressBuilderSpiFactory::new(big.get_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .with_max_message_bytes(MESSAGE_CAP_BYTES)
            .build()?;
        let result = client.call_unary(Self::request(b"ping")).await;
        Ok(matches!(
            result,
            Err(GrpcEgressError::Status(
                GrpcStatusCode::ResourceExhausted,
                _
            ))
        ))
    }

    async fn times_out() -> Result<bool, Box<dyn Error>> {
        let slow = EchoServer::start_with(PeerBehaviour::Delay(Duration::from_millis(800))).await?;
        let client = TonicGrpcEgressBuilderSpiFactory::new(slow.get_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .with_request_timeout(Duration::from_millis(150))
            .build()?;
        let result = client.call_unary(Self::request(b"ping")).await;
        Ok(matches!(result, Err(GrpcEgressError::Timeout(_))))
    }

    async fn mtls_works() -> Result<bool, Box<dyn Error>> {
        let pki = TestPki::create()?;
        let peer = PeerServer::start_tls(
            PeerBehaviour::Reply(b"secure".to_vec()),
            &pki,
            TlsServerMode::ClientAuthRequired,
        )
        .await?;
        let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_https_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_mtls(MtlsConfig::with_root_ca(
                pki.get_client_cert_pem_path()
                    .to_string_lossy()
                    .into_owned(),
                pki.get_client_key_pem_path().to_string_lossy().into_owned(),
                pki.get_ca_pem_path().to_string_lossy().into_owned(),
            ))
            .build()?;
        let answered = client.call_unary(Self::request(b"ping")).await?;
        Ok(answered.body == b"secure")
    }
}
