//! `JwtClaimsBuilder` builds the `JwtClaims` of `edge-security`: every claim set is carried, and a
//! builder missing one is refused.

use grpc_egress_auth_bearer::JwtClaimsBuilder as _;
use swe_edge_security::JwtClaims;

/// @covers: JwtClaimsBuilder::build
#[test]
fn test_build_every_claim_set_returns_the_claims() -> Result<(), String> {
    let claims: JwtClaims = claims_builder()
        .iss("iss")
        .aud("aud")
        .subject("sub")
        .exp(9_999_999_999)
        .iat(1_000_000_000)
        .build()?;

    assert_eq!(claims.iss, "iss");
    assert_eq!(claims.aud, "aud");
    assert_eq!(claims.sub, "sub");
    assert_eq!(claims.exp, 9_999_999_999);
    assert_eq!(claims.iat, 1_000_000_000);
    Ok(())
}

/// @covers: JwtClaimsBuilder::build
#[test]
fn test_build_without_a_subject_is_refused() {
    let result: Result<JwtClaims, String> = claims_builder()
        .iss("iss")
        .aud("aud")
        .exp(9_999_999_999)
        .iat(1_000_000_000)
        .build();

    assert!(result.is_err(), "a token needs a subject");
}

fn claims_builder() -> impl grpc_egress_auth_bearer::JwtClaimsBuilder {
    use grpc_egress_auth_bearer::JwtClaimsBuilderFactory;
    grpc_egress_auth_bearer_std::StdJwtClaimsBuilderFactory
        .build(grpc_egress_auth_bearer::BuildJwtClaimsBuilderRequest)
}
