//! Coverage stub for `api/bearer/bearer_egress_config_builder.rs`.

use grpc_egress_auth_bearer::BearerEgressConfigBuilder as _;
use grpc_egress_auth_bearer::BearerSecret;

/// @covers: BearerEgressConfigBuilder — fluent builder produces valid config
#[test]
fn bearer_struct_bearer_egress_config_builder_builds_valid_config_int_test() -> Result<(), String> {
    let config = egress_config_builder()
        .secret(BearerSecret::Hs256 {
            secret: b"key".to_vec(),
        })
        .issuer("iss")
        .audience("aud")
        .subject("sub")
        .lifetime_seconds(3600)
        .build()?;
    assert_eq!(config.issuer, "iss");
    assert_eq!(config.lifetime_seconds, 3600);
    Ok(())
}

fn egress_config_builder() -> impl grpc_egress_auth_bearer::BearerEgressConfigBuilder {
    use grpc_egress_auth_bearer::BearerEgressConfigBuilderFactory;
    grpc_egress_auth_bearer_std::StdBearerEgressConfigBuilderFactory
        .build(grpc_egress_auth_bearer::BuildBearerEgressConfigBuilderRequest)
}
