#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Integration tests covering jwt_claims and edge-transport-grpc-egress dependency.

use edge_transport_grpc_egress_auth_bearer::BearerAuthError;

/// @covers: BearerAuthError::InvalidSystemTime
#[test]
fn test_bearer_auth_error_implements_std_error() {
    let e = BearerAuthError::InvalidSystemTime;
    let _ = format!("{e}");
    let _: &dyn std::error::Error = &e;
}

/// @covers: GrpcEgressInterceptor (edge-transport-grpc-egress dependency coverage)
#[test]
fn test_edge_transport_grpc_egress_interceptor_trait_is_reachable() {
    use edge_transport_grpc_egress::GrpcEgressInterceptor;
    use edge_transport_grpc_egress::GrpcRequest;
    use edge_transport_grpc_egress_auth_bearer::{
        BearerEgressConfig, BearerEgressInterceptor, BearerSecret,
    };
    use std::time::Duration;

    let config = BearerEgressConfig {
        secret: BearerSecret::Hs256 {
            secret: b"testsecret".to_vec(),
        },
        issuer: "iss".into(),
        audience: "aud".into(),
        subject: "sub".into(),
        lifetime_seconds: 300,
    };
    let interceptor = BearerEgressInterceptor::from_config(config);
    let mut req = GrpcRequest::new("/test/Method", vec![], Duration::from_secs(5));
    interceptor
        .before_call(&mut req)
        .expect("before_call must succeed with valid HS256 config");
}
