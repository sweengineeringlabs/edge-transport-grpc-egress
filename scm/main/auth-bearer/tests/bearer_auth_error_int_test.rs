//! Integration tests for [`BearerAuthError`].

use edge_transport_grpc_egress_auth_bearer::BearerAuthError;

/// @covers: BearerAuthError::InvalidSystemTime
#[test]
fn test_bearer_auth_error_invalid_system_time_implements_std_error() {
    let e = BearerAuthError::InvalidSystemTime;
    let _: &dyn std::error::Error = &e;
    assert!(
        e.to_string().contains("clock"),
        "expected error message to mention clock, got: {}",
        e
    );
}

/// @covers: BearerAuthError::SignFailed
#[test]
fn test_bearer_auth_error_sign_failed_wraps_source() {
    use jsonwebtoken::{errors::ErrorKind, EncodingKey};
    // Attempt RS256 with an invalid PEM — triggers SignFailed at encode time.
    let bad_key = EncodingKey::from_rsa_pem(b"not-a-pem");
    match bad_key {
        Err(inner) => {
            let e = BearerAuthError::SignFailed(Box::new(inner));
            let _: &dyn std::error::Error = &e;
            assert!(
                e.to_string().contains("mint"),
                "expected 'mint' in message, got: {}",
                e
            );
        }
        Ok(_) => {
            // EncodingKey::from_rsa_pem accepted garbage — construct error differently
            let dummy: jsonwebtoken::errors::Error =
                jsonwebtoken::errors::Error::from(ErrorKind::InvalidAlgorithm);
            let e = BearerAuthError::SignFailed(Box::new(dummy));
            assert!(e.to_string().contains("mint"));
        }
    }
}
