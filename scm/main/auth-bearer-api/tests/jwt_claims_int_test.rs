//! The claims of a bearer token are the `JwtClaims` of `edge-security`, re-exported by the domain crate:
//! the five registered claims, as plain data.

use swe_edge_security::JwtClaims;

fn claims() -> JwtClaims {
    JwtClaims {
        iss: "svc-a".to_string(),
        aud: "svc-b".to_string(),
        sub: "svc-a-client".to_string(),
        iat: 1_000,
        exp: 1_300,
    }
}

/// @covers: JwtClaims
#[test]
fn test_claims_hold_the_issuer_audience_subject_and_times() {
    let claims = claims();

    assert_eq!(claims.iss, "svc-a");
    assert_eq!(claims.aud, "svc-b");
    assert_eq!(claims.sub, "svc-a-client");
    assert_eq!(claims.exp - claims.iat, 300);
}

/// @covers: JwtClaims
#[test]
fn test_claims_with_another_subject_are_not_equal_in_that_field() {
    let other = JwtClaims {
        sub: "someone-else".to_string(),
        ..claims()
    };

    assert_ne!(other.sub, claims().sub);
}
