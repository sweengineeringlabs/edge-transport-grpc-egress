//! Integration tests for the bearer interceptor, called as an interceptor is: on the call it is about to carry.

use std::time::Duration;

use grpc_egress_auth_bearer_api::{BearerEgressConfig, BearerSecret, AUTHORIZATION_HEADER};
use grpc_egress_auth_bearer_spi::StdBearerInterceptorFactory;
use grpc_egress_test_support::OutboundCallFixture;
use grpc_egress_transport_api::{
    BuildGrpcEgressInterceptorRequest, GrpcEgressInterceptor, GrpcEgressInterceptorFactory,
    InboundReply, OutboundCall,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use proto_pattern::{CallError, Metadata, MetadataEntry, StatusCode};
use serde::{Deserialize, Serialize};

const RS256_PRIVATE_PEM: &[u8] = include_bytes!("data/rs256_test_private.pem");
const RS256_PUBLIC_PEM: &[u8] = include_bytes!("data/rs256_test_public.pem");

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    aud: String,
    sub: String,
    exp: u64,
    iat: u64,
}

fn hs256_config(
    secret: &[u8],
) -> Result<BearerEgressConfig, grpc_egress_auth_bearer_api::BearerAuthError> {
    BearerEgressConfig::try_new(
        BearerSecret::hs256(secret.to_vec())?,
        "test-iss",
        "test-aud",
        "test-sub",
        60,
    )
}

fn rs256_config(
    private_pem: &[u8],
) -> Result<BearerEgressConfig, grpc_egress_auth_bearer_api::BearerAuthError> {
    BearerEgressConfig::try_new(
        BearerSecret::rs256(private_pem.to_vec())?,
        "test-iss",
        "test-aud",
        "test-sub",
        60,
    )
}

fn interceptor(config: BearerEgressConfig) -> impl GrpcEgressInterceptor {
    StdBearerInterceptorFactory::new(config).build(BuildGrpcEgressInterceptorRequest)
}

fn call_with(metadata: Metadata) -> OutboundCallFixture {
    OutboundCallFixture::create("/svc/M")
        .with_timeout(Some(Duration::from_secs(1)))
        .with_metadata(metadata)
}

fn token_of(call: &OutboundCallFixture) -> Result<String, Box<dyn std::error::Error>> {
    let values = call.header_values(AUTHORIZATION_HEADER);
    let [value] = values.as_slice() else {
        return Err(format!("expected exactly one authorization header, got {values:?}").into());
    };
    Ok(value
        .strip_prefix("Bearer ")
        .ok_or("the authorization header is not a bearer token")?
        .to_string())
}

fn verify(
    token: &str,
    algorithm: Algorithm,
    key: &DecodingKey,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(algorithm);
    validation.set_audience(&["test-aud"]);
    validation.set_issuer(&["test-iss"]);
    Ok(decode::<Claims>(token, key, &validation)?.claims)
}

/// @covers: StdBearerInterceptorFactory::build — before_call
#[test]
fn test_before_call_without_an_authorization_header_adds_one_bearer_jwt(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut call = call_with(Metadata::default());

    interceptor(hs256_config(b"sec")?).before_call(&mut call)?;

    let token = token_of(&call)?;
    assert_eq!(
        token.matches('.').count(),
        2,
        "JWT must have 3 segments: {token}"
    );
    Ok(())
}

/// @covers: StdBearerInterceptorFactory::build — sign_token (HS256) round-trips through the jsonwebtoken verifier
#[test]
fn test_before_call_hs256_token_verifies_with_the_secret_and_carries_the_configured_claims(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut call = call_with(Metadata::default());
    interceptor(hs256_config(b"sec")?).before_call(&mut call)?;

    let claims = verify(
        &token_of(&call)?,
        Algorithm::HS256,
        &DecodingKey::from_secret(b"sec"),
    )?;

    assert_eq!(claims.iss, "test-iss");
    assert_eq!(claims.aud, "test-aud");
    assert_eq!(claims.sub, "test-sub");
    assert_eq!(
        claims.exp - claims.iat,
        60,
        "the lifetime is the configured one"
    );
    Ok(())
}

/// @covers: StdBearerInterceptorFactory::build — a token signed with one secret is refused under another
#[test]
fn test_before_call_hs256_token_does_not_verify_with_another_secret(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut call = call_with(Metadata::default());
    interceptor(hs256_config(b"sec")?).before_call(&mut call)?;

    let verified = verify(
        &token_of(&call)?,
        Algorithm::HS256,
        &DecodingKey::from_secret(b"another"),
    );

    assert!(
        verified.is_err(),
        "a wrong secret must not verify the token"
    );
    Ok(())
}

/// @covers: StdBearerInterceptorFactory::build — sign_token (RS256) round-trips through the jsonwebtoken verifier
#[test]
fn test_before_call_rs256_token_verifies_with_the_public_key(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut call = call_with(Metadata::default());
    interceptor(rs256_config(RS256_PRIVATE_PEM)?).before_call(&mut call)?;

    let claims = verify(
        &token_of(&call)?,
        Algorithm::RS256,
        &DecodingKey::from_rsa_pem(RS256_PUBLIC_PEM)?,
    )?;

    assert_eq!(claims.sub, "test-sub");
    Ok(())
}

/// @covers: StdBearerInterceptorFactory::build — a call that already carries credentials keeps them
#[test]
fn test_before_call_with_an_authorization_header_keeps_it_and_adds_no_second(
) -> Result<(), Box<dyn std::error::Error>> {
    let mine = Metadata::create(vec![MetadataEntry::create(
        AUTHORIZATION_HEADER,
        "Bearer caller-supplied",
    )?]);
    let mut call = call_with(mine);

    interceptor(hs256_config(b"sec")?).before_call(&mut call)?;

    assert_eq!(
        call.header_values(AUTHORIZATION_HEADER),
        ["Bearer caller-supplied"]
    );
    Ok(())
}

/// @covers: StdBearerInterceptorFactory::build — other headers of the call are kept
#[test]
fn test_before_call_keeps_the_other_headers_of_the_call() -> Result<(), Box<dyn std::error::Error>>
{
    let mut call = call_with(Metadata::create(vec![MetadataEntry::create(
        "x-trace", "t-1",
    )?]));

    interceptor(hs256_config(b"sec")?).before_call(&mut call)?;

    assert_eq!(call.header_values("x-trace"), ["t-1"]);
    assert_eq!(call.header_values(AUTHORIZATION_HEADER).len(), 1);
    Ok(())
}

/// @covers: StdBearerInterceptorFactory::build — a key that cannot sign fails the call before anything is sent
#[test]
fn test_before_call_with_an_rs256_key_that_is_not_a_key_returns_transport_internal_naming_the_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut call = call_with(Metadata::default());

    let failure = interceptor(rs256_config(b"this is not a pem key")?).before_call(&mut call);

    match failure {
        Err(CallError::Transport { path, code, reason }) => {
            assert_eq!(path.as_ref(), "/svc/M");
            assert_eq!(code, StatusCode::Internal);
            assert_eq!(reason, "failed to mint bearer token");
            assert!(
                !reason.contains("pem"),
                "the cause stays in the log, not in the error: {reason}"
            );
        }
        other => panic!("expected a transport failure, got {other:?}"),
    }
    assert!(
        call.header_values(AUTHORIZATION_HEADER).is_empty(),
        "no header is added when no token could be minted"
    );
    Ok(())
}

/// @covers: StdBearerInterceptorFactory::build — after_call
#[test]
fn test_after_call_returns_ok_and_changes_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let reply = InboundReply::new("/svc/M", Metadata::default(), Metadata::default());

    assert!(interceptor(hs256_config(b"sec")?)
        .after_call(&reply)
        .is_ok());
    Ok(())
}
