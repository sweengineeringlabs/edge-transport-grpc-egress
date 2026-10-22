//! The bearer interceptor in the real client, against a real in-process peer: the token is on the wire, the
//! client's own credentials are respected, and a failure to mint one stops the call before it is sent.

use proto_pattern::{
    CallError, CallRawRequestBuilder, CallRawResponse, Metadata, MetadataEntry, RawTransport,
    StatusCode,
};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use grpc_egress_auth_bearer::{BearerEgressConfig, BearerSecret, AUTHORIZATION_HEADER};
use grpc_egress_auth_bearer_std::StdBearerInterceptorFactory;
use grpc_egress_test_support::{CallFixture, Fixture, PeerBehaviour, PeerServer, RunningPeer};
use grpc_egress_transport::{
    BuildGrpcEgressBuilderRequest, BuildGrpcEgressInterceptorRequest, GrpcEgressBuilder,
    GrpcEgressBuilderFactory, GrpcEgressInterceptorFactory,
};
use grpc_egress_transport_std::TonicGrpcEgressBuilderSpiFactory;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

const RS256_PRIVATE_PEM: &[u8] = include_bytes!("data/rs256_test_private.pem");
const RS256_PUBLIC_PEM: &[u8] = include_bytes!("data/rs256_test_public.pem");
const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    aud: String,
}

fn config(secret: BearerSecret) -> BearerEgressConfig {
    BearerEgressConfig {
        secret,
        issuer: "wire-iss".into(),
        audience: "wire-aud".into(),
        subject: "wire-sub".into(),
        lifetime_seconds: 60,
    }
}

async fn peer() -> Result<RunningPeer, Box<dyn Error>> {
    Fixture::ensure_tls_provider();
    Ok(PeerServer::start(PeerBehaviour::Reply(b"ok".to_vec())).await?)
}

/// A client for `peer` that signs a bearer token into every call.
fn client_signing(
    peer: &RunningPeer,
    config: BearerEgressConfig,
) -> Result<impl RawTransport, Box<dyn Error>> {
    let interceptor =
        StdBearerInterceptorFactory::new(config).build(BuildGrpcEgressInterceptorRequest);
    Ok(TonicGrpcEgressBuilderSpiFactory::new(peer.get_endpoint())
        .build(BuildGrpcEgressBuilderRequest)
        .with_plaintext_allowed()
        .with_interceptor(Arc::new(interceptor))
        .build()?)
}

fn header_on_the_wire(peer: &RunningPeer) -> Result<String, Box<dyn Error>> {
    let requests = peer.get_recorded_requests();
    let [request] = requests.as_slice() else {
        return Err(format!("expected one request at the peer, got {}", requests.len()).into());
    };
    Ok(request
        .get_header(AUTHORIZATION_HEADER)
        .ok_or("the request carried no authorization header")?
        .to_string())
}

fn verified(
    token_header: &str,
    algorithm: Algorithm,
    key: &DecodingKey,
) -> Result<Claims, Box<dyn Error>> {
    let token = token_header
        .strip_prefix("Bearer ")
        .ok_or("the authorization header is not a bearer token")?;
    let mut validation = Validation::new(algorithm);
    validation.set_audience(&["wire-aud"]);
    validation.set_issuer(&["wire-iss"]);
    Ok(decode::<Claims>(token, key, &validation)?.claims)
}

/// @covers: StdBearerInterceptorFactory::build — on the wire (HS256)
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_hs256_sends_a_bearer_token_that_verifies_with_the_secret(
) -> Result<(), Box<dyn Error>> {
    let peer = peer().await?;
    let client = client_signing(
        &peer,
        config(BearerSecret::Hs256 {
            secret: b"wire-secret".to_vec(),
        }),
    )?;

    client
        .call_raw({
            let (message, options) = CallFixture::request(b"ping", TIMEOUT).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;

    let claims = verified(
        &header_on_the_wire(&peer)?,
        Algorithm::HS256,
        &DecodingKey::from_secret(b"wire-secret"),
    )?;
    assert_eq!(claims.sub, "wire-sub");
    assert_eq!(claims.aud, "wire-aud");
    Ok(())
}

/// @covers: StdBearerInterceptorFactory::build — on the wire (RS256)
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_rs256_sends_a_bearer_token_that_verifies_with_the_public_key(
) -> Result<(), Box<dyn Error>> {
    let peer = peer().await?;
    let client = client_signing(
        &peer,
        config(BearerSecret::Rs256 {
            private_pem: RS256_PRIVATE_PEM.to_vec(),
        }),
    )?;

    client
        .call_raw({
            let (message, options) = CallFixture::request(b"ping", TIMEOUT).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;

    let claims = verified(
        &header_on_the_wire(&peer)?,
        Algorithm::RS256,
        &DecodingKey::from_rsa_pem(RS256_PUBLIC_PEM)?,
    )?;
    assert_eq!(claims.sub, "wire-sub");
    Ok(())
}

/// @covers: StdBearerInterceptorFactory::build — a credential the caller set is the one on the wire
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_with_an_authorization_header_sends_the_callers_and_not_a_minted_one(
) -> Result<(), Box<dyn Error>> {
    let peer = peer().await?;
    let client = client_signing(
        &peer,
        config(BearerSecret::Hs256 {
            secret: b"wire-secret".to_vec(),
        }),
    )?;
    let mine = Metadata::create(vec![MetadataEntry::create(
        AUTHORIZATION_HEADER,
        "Bearer caller-supplied",
    )?]);

    client
        .call_raw({
            let (message, options) = CallFixture::request_with(b"ping", TIMEOUT, mine).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)?;

    assert_eq!(header_on_the_wire(&peer)?, "Bearer caller-supplied");
    Ok(())
}

/// @covers: StdBearerInterceptorFactory::build — nothing is sent when no token can be minted
#[tokio::test(flavor = "multi_thread")]
async fn test_call_raw_with_an_rs256_key_that_is_not_a_key_fails_with_transport_internal_and_reaches_no_peer(
) -> Result<(), Box<dyn Error>> {
    let peer = peer().await?;
    let client = client_signing(
        &peer,
        config(BearerSecret::Rs256 {
            private_pem: b"this is not a pem key".to_vec(),
        }),
    )?;

    let failure = client
        .call_raw({
            let (message, options) = CallFixture::request(b"ping", TIMEOUT).into_parts();
            CallRawRequestBuilder::new()
                .with_path(CallFixture::path("pkg.Service/Method"))
                .with_message(message)
                .with_options(options)
                .build()
        })
        .await
        .map(CallRawResponse::into_response)
        .err();

    match failure {
        Some(CallError::Transport { path, code, .. }) => {
            assert_eq!(path.as_ref(), "/pkg.Service/Method");
            assert_eq!(code, StatusCode::Internal);
        }
        other => panic!("expected a transport failure, got {other:?}"),
    }
    assert!(
        peer.get_recorded_requests().is_empty(),
        "a call that could not be signed must not reach the peer"
    );
    Ok(())
}
