//! `AllOptionsScenario` -- every option of the client builder, one after the other.

use proto_pattern::{
    CallError, CallRawRequestBuilder, CallRawResponse, CallRequest, Compression, RawTransport,
    StatusCode,
};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use grpc_egress_test_support::{CallFixture, PeerBehaviour, PeerServer, TestPki, TlsServerMode};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory, KeepAliveConfig,
    MtlsConfig, TraceContextSource,
};
use grpc_egress_transport_std::{
    MtlsConfigFactory, TlsSettingsResolver, TonicGrpcEgressBuilderSpiFactory,
};

use crate::{EchoServer, OrderInterceptor, ScenarioReport};

const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
const MESSAGE_CAP_BYTES: usize = 1024;

/// Sets every option of the builder: endpoint, plaintext rule, request timeout, message limit, compression,
/// keep-alive, TLS, interceptor and trace context, and looks for the effect of each.
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
        let log = Arc::new(Mutex::new(Vec::new()));
        let client = TonicGrpcEgressBuilderSpiFactory::new("")
            .build(BuildGrpcEgressBuilderRequest)
            .with_endpoint(server.get_endpoint())
            .with_plaintext_allowed()
            .with_request_timeout(Duration::from_secs(5))
            .with_max_decoding_message_size(MESSAGE_CAP_BYTES)
            .with_send_compression(Compression::Gzip)
            .with_accept_compression(Compression::Gzip)
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

        let echoed = client
            .call_raw({
                let (message, options) = Self::request(b"ping").into_parts();
                CallRawRequestBuilder::new()
                    .with_path(CallFixture::path("pkg.Service/Method"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)?;
        report.record(
            "the peer's answer arrives",
            echoed.get_message().as_slice() == b"pong",
        );
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
            "a reply over the message limit is refused",
            Self::limit_refuses().await?,
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

    fn request(body: &[u8]) -> CallRequest<Vec<u8>> {
        CallFixture::request(body, Duration::from_secs(5))
    }

    async fn limit_refuses() -> Result<bool, Box<dyn Error>> {
        let big =
            EchoServer::start_with(PeerBehaviour::Reply(vec![7u8; MESSAGE_CAP_BYTES * 2])).await?;
        let client = TonicGrpcEgressBuilderSpiFactory::new(big.get_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .with_max_decoding_message_size(MESSAGE_CAP_BYTES)
            .build()?;
        let result = client
            .call_raw({
                let (message, options) = Self::request(b"ping").into_parts();
                CallRawRequestBuilder::new()
                    .with_path(CallFixture::path("pkg.Service/Method"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response);
        Ok(matches!(
            result,
            Err(CallError::Status {
                code: StatusCode::OutOfRange,
                ..
            })
        ))
    }

    async fn times_out() -> Result<bool, Box<dyn Error>> {
        let slow = EchoServer::start_with(PeerBehaviour::Delay(Duration::from_millis(800))).await?;
        let client = TonicGrpcEgressBuilderSpiFactory::new(slow.get_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .with_request_timeout(Duration::from_millis(150))
            .build()?;
        let result = client
            .call_raw({
                let (message, options) =
                    CallFixture::request(b"ping", Duration::from_secs(5)).into_parts();
                CallRawRequestBuilder::new()
                    .with_path(CallFixture::path("pkg.Service/Method"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response);
        Ok(matches!(
            result,
            Err(CallError::Status {
                code: StatusCode::DeadlineExceeded,
                ..
            })
        ))
    }

    async fn mtls_works() -> Result<bool, Box<dyn Error>> {
        let pki = TestPki::create()?;
        let peer = PeerServer::start_tls(
            PeerBehaviour::Reply(b"secure".to_vec()),
            &pki,
            TlsServerMode::ClientAuthRequired,
        )
        .await?;
        let mtls = MtlsConfig::with_root_ca(
            pki.get_client_cert_pem_path()
                .to_string_lossy()
                .into_owned(),
            pki.get_client_key_pem_path().to_string_lossy().into_owned(),
            pki.get_ca_pem_path().to_string_lossy().into_owned(),
        );
        let tls = TlsSettingsResolver::resolve(Some(&mtls), mtls.ca_pem_path.as_deref())?;
        let client = TonicGrpcEgressBuilderSpiFactory::new(peer.get_https_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_tls(tls)
            .build()?;
        let answered = client
            .call_raw({
                let (message, options) = Self::request(b"ping").into_parts();
                CallRawRequestBuilder::new()
                    .with_path(CallFixture::path("pkg.Service/Method"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)?;
        Ok(answered.get_message().as_slice() == b"secure")
    }
}
