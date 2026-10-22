//! `SecurityContextScenario` -- ruling 3 of ADR-008: the identity of the caller travels as headers of the call.

use proto_pattern::{
    CallRawRequestBuilder, CallRawResponse, CallRequest, Metadata, MetadataEntry, RawTransport,
};
use std::error::Error;
use std::time::Duration;

use grpc_egress_test_support::CallFixture;
use grpc_egress_transport::{
    BuildGrpcCallerIdentityRequest, BuildGrpcEgressBuilderRequest, GrpcCallerIdentityFactory,
    GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_std::{
    SecurityContext, SecurityContextIdentity, StdGrpcCallerIdentityFactory,
    TonicGrpcEgressBuilderSpiFactory,
};

use crate::{EchoServer, ScenarioReport};

/// Binds a client to the identity a security context stands for and looks at the headers the peer receives.
pub struct SecurityContextScenario;

impl SecurityContextScenario {
    /// Run the scenario.
    ///
    /// # Errors
    ///
    /// An error of starting the peer, building a client or of a call that should have worked.
    pub async fn run(&self) -> Result<ScenarioReport, Box<dyn Error>> {
        let mut report = ScenarioReport::new("security context");
        let server = EchoServer::start().await?;
        let client = TonicGrpcEgressBuilderSpiFactory::new(server.get_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .build()?;

        let mut ctx = SecurityContext::unauthenticated().with_trace_id("trace-1");
        ctx.tenant_id = Some("tenant-7".to_string());
        ctx.authenticated = true;
        ctx.token = Some("jwt-abc".to_string());
        let bound = StdGrpcCallerIdentityFactory::new().build(BuildGrpcCallerIdentityRequest {
            inner: &client,
            identity: &SecurityContextIdentity::from(&ctx),
        });
        bound
            .call_raw({
                let (message, options) = Self::request(Metadata::default()).into_parts();
                CallRawRequestBuilder::new()
                    .with_path(CallFixture::path("pkg.Service/Method"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)?;
        report.record(
            "the trace id and the tenant reach the peer",
            server.get_last_header("x-trace-id").as_deref() == Some("trace-1")
                && server.get_last_header("x-tenant-id").as_deref() == Some("tenant-7"),
        );
        report.record(
            "an authenticated context sends its token as a bearer",
            server.get_last_header("authorization").as_deref() == Some("Bearer jwt-abc"),
        );

        let own = Metadata::create(vec![MetadataEntry::create("x-trace-id", "caller")?]);
        bound
            .call_raw({
                let (message, options) = Self::request(own).into_parts();
                CallRawRequestBuilder::new()
                    .with_path(CallFixture::path("pkg.Service/Method"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)?;
        report.record(
            "a header the call carries is kept",
            server.get_last_header("x-trace-id").as_deref() == Some("caller"),
        );

        ctx.authenticated = false;
        let unauthenticated =
            StdGrpcCallerIdentityFactory::new().build(BuildGrpcCallerIdentityRequest {
                inner: &client,
                identity: &SecurityContextIdentity::from(&ctx),
            });
        unauthenticated
            .call_raw({
                let (message, options) = Self::request(Metadata::default()).into_parts();
                CallRawRequestBuilder::new()
                    .with_path(CallFixture::path("pkg.Service/Method"))
                    .with_message(message)
                    .with_options(options)
                    .build()
            })
            .await
            .map(CallRawResponse::into_response)?;
        report.record(
            "an unauthenticated context sends no authorization",
            server.get_last_header("authorization").is_none(),
        );
        Ok(report)
    }

    fn request(metadata: Metadata) -> CallRequest<Vec<u8>> {
        CallFixture::request_with(b"ping", Duration::from_secs(5), metadata)
    }
}
