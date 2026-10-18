//! Reusable example: sign a bearer token and see it added to an outbound call.
//!
//! The interceptor is built from a [`BearerEgressConfig`], pushed into a
//! [`GrpcEgressInterceptorChain`], and the chain is run over the call exactly as the client runs it before the wire
//! call. No server is needed.
//!
//! [`BearerEgressConfig`]: grpc_egress_auth_bearer::BearerEgressConfig
//! [`GrpcEgressInterceptorChain`]: grpc_egress_transport_std::GrpcEgressInterceptorChain

use grpc_egress_transport::GrpcEgressInterceptorChain as _;
use std::sync::Arc;
use std::time::Duration;

use grpc_egress_auth_bearer::{BearerSecret, AUTHORIZATION_HEADER};
use grpc_egress_auth_bearer_std::{BearerEgressConfigBuilder, StdBearerInterceptorFactory};
use grpc_egress_test_support::OutboundCallFixture;
use grpc_egress_transport::{
    BuildGrpcEgressInterceptorRequest, GrpcEgressInterceptorFactory, OutboundCall,
};
use proto_pattern::Metadata;

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

    let chain = new_chain().push(Arc::new(
        StdBearerInterceptorFactory::new(config).build(BuildGrpcEgressInterceptorRequest),
    ));

    let mut call = OutboundCallFixture::create("/example.Service/Method")
        .with_timeout(Some(Duration::from_secs(3)))
        .with_metadata(Metadata::default());
    chain.run_before(&mut call)?;

    let values = call.get_header_values(AUTHORIZATION_HEADER);
    let [header] = values.as_slice() else {
        return Err("the interceptor did not add exactly one authorization header".into());
    };
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

fn new_chain() -> impl grpc_egress_transport::GrpcEgressInterceptorChain {
    use grpc_egress_transport::GrpcEgressInterceptorChainFactory;
    grpc_egress_transport_std::StdGrpcEgressInterceptorChainFactory
        .build(grpc_egress_transport::BuildGrpcEgressInterceptorChainRequest)
}
