//! Outbound interceptor: signs a JWT and injects it as
//! `authorization: Bearer <jwt>` on every request.

use std::time::{SystemTime, UNIX_EPOCH};

use grpc_egress_transport::{GrpcEgressInterceptor, InboundReply, OutboundCall};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use proto_pattern::{CallError, StatusCode};

use grpc_egress_auth_bearer::{
    BearerAuthError, BearerEgressConfig, BearerSecret, AUTHORIZATION_HEADER,
};
use swe_edge_security::JwtClaims;

/// `GrpcEgressInterceptor` that signs and attaches a JWT bearer token.
pub(crate) struct BearerEgressInterceptor {
    pub(crate) config: BearerEgressConfig,
}

impl From<BearerEgressConfig> for BearerEgressInterceptor {
    /// The interceptor that signs as the config says.
    fn from(config: BearerEgressConfig) -> Self {
        Self { config }
    }
}

impl BearerEgressInterceptor {
    fn sign_token(&self) -> Result<String, BearerAuthError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| BearerAuthError::InvalidSystemTime)?
            .as_secs();
        let claims = JwtClaims {
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
            sub: self.config.subject.clone(),
            iat: now,
            exp: now.saturating_add(self.config.lifetime_seconds),
        };
        let (alg, key) = match &self.config.secret {
            BearerSecret::Hs256 { secret } => (Algorithm::HS256, EncodingKey::from_secret(secret)),
            BearerSecret::Rs256 { private_pem } => (
                Algorithm::RS256,
                EncodingKey::from_rsa_pem(private_pem)
                    .map_err(|e| BearerAuthError::SignFailed(Box::new(e)))?,
            ),
        };
        jsonwebtoken::encode(&Header::new(alg), &claims, &key)
            .map_err(|e| BearerAuthError::SignFailed(Box::new(e)))
    }
}

impl GrpcEgressInterceptor for BearerEgressInterceptor {
    /// Adds `authorization: Bearer <jwt>` to the call. A call that already carries an `authorization` header keeps
    /// it: the caller's credential is the caller's, and nothing is minted for it.
    fn before_call(&self, call: &mut dyn OutboundCall) -> Result<(), CallError> {
        if call.has_header(AUTHORIZATION_HEADER) {
            return Ok(());
        }
        let token = self.sign_token().map_err(|e| {
            tracing::warn!(error = %e, "failed to mint bearer token");
            CallError::Transport {
                path: call.get_path().into(),
                code: StatusCode::Internal,
                reason: "failed to mint bearer token".to_string(),
            }
        })?;
        call.add_header(AUTHORIZATION_HEADER, &format!("Bearer {token}"))
    }

    fn after_call(&self, _reply: &InboundReply<'_>) -> Result<(), CallError> {
        Ok(())
    }
}
