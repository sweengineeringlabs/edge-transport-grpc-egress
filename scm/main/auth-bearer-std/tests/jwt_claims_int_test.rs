//! Integration tests covering jwt_claims and edge-transport-grpc-egress dependency.

use grpc_egress_auth_bearer::BearerAuthError;
use grpc_egress_transport_std::GrpcRequestFactory;

/// @covers: BearerAuthError::InvalidSystemTime
#[test]
fn test_bearer_auth_error_implements_std_error() {
    let e = BearerAuthError::InvalidSystemTime;
    let _ = format!("{e}");
    let _: &dyn std::error::Error = &e;
}

/// @covers: GrpcEgressInterceptor (edge-transport-grpc-egress dependency coverage)
#[test]
fn test_edge_transport_grpc_egress_interceptor_trait_is_reachable(
) -> Result<(), Box<dyn std::error::Error>> {
    use grpc_egress_auth_bearer::{BearerEgressConfig, BearerSecret};
    use grpc_egress_auth_bearer_std::StdBearerInterceptorFactory;
    use grpc_egress_transport::GrpcRequest;
    use grpc_egress_transport::{
        BuildGrpcEgressInterceptorRequest, GrpcEgressInterceptor, GrpcEgressInterceptorFactory,
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
    let interceptor =
        StdBearerInterceptorFactory::new(config).build(BuildGrpcEgressInterceptorRequest);
    let mut req = GrpcRequest::new("/test/Method", vec![], Duration::from_secs(5));
    interceptor.before_call(&mut req)?;
    Ok(())
}
