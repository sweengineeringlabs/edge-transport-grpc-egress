//! ADR-005: building a client installs the process-wide `aws-lc-rs` crypto provider of `rustls`, so a TLS
//! client never panics for the lack of one. This file holds one test: a test binary is a fresh
//! process, so the provider cannot have been installed by anything else.

use std::error::Error;

use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, GrpcEgressBuilder, GrpcEgressBuilderFactory,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;

/// @covers: GrpcEgressBuilder::build
#[test]
fn test_build_installs_the_aws_lc_rs_provider_in_a_process_that_had_none(
) -> Result<(), Box<dyn Error>> {
    assert!(
        rustls::crypto::CryptoProvider::get_default().is_none(),
        "this process must start without a provider"
    );

    let _client = TonicGrpcEgressBuilderSpiFactory::new("https://service.example:443")
        .build(BuildGrpcEgressBuilderRequest)
        .build()?;

    assert!(
        rustls::crypto::CryptoProvider::get_default().is_some(),
        "building a client must leave a provider installed"
    );
    Ok(())
}
