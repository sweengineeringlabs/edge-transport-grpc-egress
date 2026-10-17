//! `SecurityContextScenario` -- ruling 3 of ADR-008: the security context travels as headers of the call.

use std::error::Error;
use std::time::Duration;

use grpc_egress_test_support::CallFixture;
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, BuildGrpcSecurityContextRequest, GrpcEgressBuilder,
    GrpcEgressBuilderFactory, GrpcSecurityContextFactory, SecurityContext,
};
use grpc_egress_transport_std::{StdGrpcSecurityContextFactory, TonicGrpcEgressBuilderSpiFactory};
use proto_pattern::{Metadata, MetadataEntry, RawTransport};

use crate::{EchoServer, ScenarioReport};

/// Binds a client to a security context and looks at the headers the peer receives.
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
        let bound = StdGrpcSecurityContextFactory::new().build(BuildGrpcSecurityContextRequest {
            inner: &client,
            ctx: &ctx,
        });
        bound
            .call_raw(
                CallFixture::path("pkg.Service/Method"),
                Self::request(Metadata::default()),
            )
            .await?;
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
            .call_raw(CallFixture::path("pkg.Service/Method"), Self::request(own))
            .await?;
        report.record(
            "a header the call carries is kept",
            server.get_last_header("x-trace-id").as_deref() == Some("caller"),
        );

        ctx.authenticated = false;
        let unauthenticated =
            StdGrpcSecurityContextFactory::new().build(BuildGrpcSecurityContextRequest {
                inner: &client,
                ctx: &ctx,
            });
        unauthenticated
            .call_raw(
                CallFixture::path("pkg.Service/Method"),
                Self::request(Metadata::default()),
            )
            .await?;
        report.record(
            "an unauthenticated context sends no authorization",
            server.get_last_header("authorization").is_none(),
        );
        Ok(report)
    }

    fn request(metadata: Metadata) -> proto_pattern::CallRequest<Vec<u8>> {
        CallFixture::request_with(b"ping", Duration::from_secs(5), metadata)
    }
}
