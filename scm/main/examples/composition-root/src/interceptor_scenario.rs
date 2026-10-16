//! `InterceptorScenario` -- interceptors and the trace context.

use std::error::Error;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcEgress, GrpcEgressBuilder, GrpcEgressBuilderFactory,
    GrpcRequest, TraceContextSource,
};
use grpc_egress_transport_std::{GrpcRequestFactory, TonicGrpcEgressBuilderSpiFactory};

use crate::{EchoServer, OrderInterceptor, RefusingInterceptor, ScenarioReport};

const TRACEPARENT: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

/// Shows the order interceptors run in, the trace context reaching the peer, and an interceptor that
/// refuses a call before it is sent.
pub struct InterceptorScenario;

impl InterceptorScenario {
    /// Run the scenario.
    ///
    /// # Errors
    ///
    /// An error of starting the peer, building a client or of a call that should have worked.
    pub async fn run(&self) -> Result<ScenarioReport, Box<dyn Error>> {
        let mut report = ScenarioReport::new("interceptor and trace context");
        let server = EchoServer::start().await?;
        let log = Arc::new(Mutex::new(Vec::new()));
        let client = TonicGrpcEgressBuilderSpiFactory::new(server.get_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .with_interceptor(Arc::new(OrderInterceptor::new("a", log.clone())))
            .with_interceptor(Arc::new(OrderInterceptor::new("b", log.clone())))
            .with_trace_context(TraceContextSource::Static {
                traceparent: TRACEPARENT.to_string(),
                tracestate: Some("vendor=1".to_string()),
            })
            .build()?;
        client.call_unary(Self::request()).await?;

        let order = log.lock().unwrap_or_else(PoisonError::into_inner).clone();
        report.record(
            "before_call and after_call both run in registration order",
            order == ["before:a", "before:b", "after:a", "after:b"],
        );
        report.record(
            "the trace context reaches the peer",
            server.get_last_header("traceparent").as_deref() == Some(TRACEPARENT)
                && server.get_last_header("tracestate").as_deref() == Some("vendor=1"),
        );

        let before = server.count_requests();
        let refusing = TonicGrpcEgressBuilderSpiFactory::new(server.get_endpoint())
            .build(BuildGrpcEgressBuilderRequest)
            .with_plaintext_allowed()
            .with_interceptor(Arc::new(RefusingInterceptor))
            .build()?;
        let refused = refusing.call_unary(Self::request()).await;
        report.record(
            "an interceptor that refuses stops the call before it is sent",
            refused.is_err() && server.count_requests() == before,
        );
        Ok(report)
    }

    fn request() -> GrpcRequest {
        GrpcRequest::new(
            "pkg.Service/Method",
            b"ping".to_vec(),
            Duration::from_secs(5),
        )
    }
}
