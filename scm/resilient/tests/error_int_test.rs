#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests for `ResilientTransportError`, produced by
//! [`GrpcResilientFacade::apply_resilience`].

use std::collections::HashMap;

use edge_transport_grpc_egress::{
    CallStreamRequest, GrpcEgress, GrpcEgressResult, GrpcMessageStreamResponse, GrpcResponse,
    HealthCheckRequest,
};
use edge_transport_grpc_egress_resilient::{GrpcResilientFacade, ResilienceConfig};
use futures::future::BoxFuture;

struct AlwaysOk;
impl GrpcEgress for AlwaysOk {
    fn call_unary(
        &self,
        _req: edge_transport_grpc_egress::GrpcRequest,
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
    let Err(err) = GrpcResilientFacade::apply_resilience(AlwaysOk, cfg) else {
        panic!("expected apply_resilience to reject max_attempts = 0");
    };
    assert!(matches!(
        err,
        edge_transport_grpc_egress_resilient::ResilientTransportError::InvalidResilience(_)
    ));
    assert!(err.to_string().contains("invalid resilience config"));
}

/// @covers: apply_resilience — a valid config wraps the inner client without error
#[test]
fn test_apply_resilience_valid_config_returns_ok() {
    let result = GrpcResilientFacade::apply_resilience(AlwaysOk, ResilienceConfig::default());
    assert!(
        result.is_ok(),
        "a genuinely valid config must wrap successfully"
    );
}
