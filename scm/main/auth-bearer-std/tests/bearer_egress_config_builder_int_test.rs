//! Coverage stub for `api/bearer/bearer_egress_config_builder.rs`.

use grpc_egress_auth_bearer::BearerEgressConfigBuilder as _;
use grpc_egress_auth_bearer::BearerSecret;

/// @covers: BearerEgressConfigBuilder — fluent builder produces valid config
#[test]
fn bearer_struct_bearer_egress_config_builder_builds_valid_config_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let config = egress_config_builder()
        .secret(BearerSecret::hs256(b"key".to_vec())?)
        .issuer("iss")
        .audience("aud")
        .subject("sub")
        .lifetime_seconds(3600)
        .build()
        .map_err(std::io::Error::other)?;
    assert_eq!(config.issuer(), "iss");
    assert_eq!(config.audience(), "aud");
    assert_eq!(config.subject(), "sub");
    assert_eq!(config.lifetime_seconds(), 3600);
    assert_eq!(config.secret().hs256_secret(), Some(&b"key"[..]));
    Ok(())
}

/// @covers: BearerEgressConfigBuilder::build — a field that was not set is named in the refusal
#[test]
fn bearer_struct_bearer_egress_config_builder_build_without_a_subject_names_it_int_test(
) -> Result<(), Box<dyn std::error::Error>> {
    let result = egress_config_builder()
        .secret(BearerSecret::hs256(b"key".to_vec())?)
        .issuer("iss")
        .audience("aud")
        .lifetime_seconds(60)
        .build();
    assert_eq!(result.err().as_deref(), Some("subject is required"));
    Ok(())
}

fn egress_config_builder() -> impl grpc_egress_auth_bearer::BearerEgressConfigBuilder {
    use grpc_egress_auth_bearer::BearerEgressConfigBuilderFactory;
    grpc_egress_auth_bearer_std::StdBearerEgressConfigBuilderFactory
        .build(grpc_egress_auth_bearer::BuildBearerEgressConfigBuilderRequest)
}
