//! Reusable example: sign a bearer token and see it added to an outbound request.
//!
//! The interceptor is built from a [`BearerEgressConfig`], pushed into a
//! [`GrpcEgressInterceptorChain`], and the chain is run over a request exactly as the client runs it
//! before the wire call. No server is needed.
//!
//! [`BearerEgressConfig`]: edge_transport_grpc_egress_auth_bearer::BearerEgressConfig
//! [`GrpcEgressInterceptorChain`]: grpc_egress_transport_std::GrpcEgressInterceptorChain

use grpc_egress_transport_std::GrpcRequestFactory;
use std::sync::Arc;
use std::time::Duration;

use edge_transport_grpc_egress_auth_bearer::{
    BearerEgressConfigBuilder, BearerEgressInterceptor, BearerSecret, AUTHORIZATION_HEADER,
};
use grpc_egress_transport::GrpcRequest;
use grpc_egress_transport_std::GrpcEgressInterceptorChain;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = BearerEgressConfigBuilder::new()
        .secret(BearerSecret::Hs256 {
            secret: b"an-example-secret-of-32-bytes!!!".to_vec(),
        })
        .issuer("svc-a")
        .audience("svc-b")
        .subject("svc-a-client")
        .lifetime_seconds(60)
        .build()?;

    let chain = GrpcEgressInterceptorChain::new()
        .push(Arc::new(BearerEgressInterceptor::from_config(config)));

    let mut request = GrpcRequest::new("example.Service/Method", vec![], Duration::from_secs(3));
    chain.run_before(&mut request)?;

    let header = request
        .metadata
        .get(AUTHORIZATION_HEADER)
        .ok_or("the interceptor did not add an authorization header")?;
    let token = header
        .strip_prefix("Bearer ")
        .ok_or("the authorization header is not a bearer token")?;

    // A signed JWT has three dot-separated parts: header, claims, signature.
    assert_eq!(
        token.split('.').count(),
        3,
        "the token must be a signed JWT"
    );
    println!("authorization: Bearer <jwt of {} characters>", token.len());
    Ok(())
}
