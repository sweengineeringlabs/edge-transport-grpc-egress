#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for `ResilientTransportError`, produced by
//! [`StdGrpcResilientFactory::build`].

use std::collections::HashMap;

use futures::future::BoxFuture;
use grpc_egress_resilient::{BuildGrpcResilientRequest, GrpcResilientFactory, ResilienceConfig};
use grpc_egress_resilient_std::StdGrpcResilientFactory;
use grpc_egress_transport::{
    CallStreamRequest, GrpcEgress, GrpcEgressResult, GrpcMessageStreamResponse, GrpcResponse,
    HealthCheckRequest,
};

struct AlwaysOk;
impl GrpcEgress for AlwaysOk {
    fn call_unary(
        &self,
        _req: grpc_egress_transport::GrpcRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcResponse>> {
        Box::pin(async {
            Ok(GrpcResponse {
                body: vec![],
                metadata: HashMap::new(),
            })
        })
    }
    fn call_stream(
        &self,
        req: CallStreamRequest,
    ) -> BoxFuture<'_, GrpcEgressResult<GrpcMessageStreamResponse>> {
        Box::pin(async move { Ok(req.messages) })
    }
    fn health_check(&self, _req: HealthCheckRequest) -> BoxFuture<'_, GrpcEgressResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

/// @covers: ResilientTransportError::InvalidResilience
#[test]
fn test_error_invalid_resilience_variant_produced_on_bad_config() {
    let cfg = ResilienceConfig {
        max_attempts: 0,
        ..ResilienceConfig::default()
    };
    let Err(err) =
        StdGrpcResilientFactory::new(cfg).build(BuildGrpcResilientRequest { inner: AlwaysOk })
    else {
        panic!("expected build to reject max_attempts = 0");
    };
    assert!(matches!(
        err,
        grpc_egress_resilient::ResilientTransportError::InvalidResilience(_)
    ));
    assert!(err.to_string().contains("invalid resilience config"));
}

/// @covers: StdGrpcResilientFactory::build — a valid config wraps the inner client without error
#[test]
fn test_build_valid_config_returns_ok() {
    let result = StdGrpcResilientFactory::new(ResilienceConfig::default())
        .build(BuildGrpcResilientRequest { inner: AlwaysOk });
    assert!(
        result.is_ok(),
        "a genuinely valid config must wrap successfully"
    );
}
