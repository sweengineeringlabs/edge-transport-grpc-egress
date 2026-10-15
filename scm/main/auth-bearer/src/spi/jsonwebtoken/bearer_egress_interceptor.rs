//! Outbound interceptor: signs a JWT and injects it as
//! `authorization: Bearer <jwt>` on every request.

use std::time::{SystemTime, UNIX_EPOCH};

use edge_transport_grpc_egress::{
    AfterCallRequest, GrpcEgressError, GrpcEgressInterceptor, GrpcRequest, GrpcStatusCode,
};
use jsonwebtoken::{Algorithm, EncodingKey, Header};

use crate::api::{
    BearerAuthError, BearerEgressInterceptor, BearerSecret, JwtClaims, Processor,
    AUTHORIZATION_HEADER,
};

impl Processor for BearerEgressInterceptor {
    fn as_interceptor(&self) -> &BearerEgressInterceptor {
        self
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
    fn before_call(&self, req: &mut GrpcRequest) -> Result<(), GrpcEgressError> {
        let token = self.sign_token().map_err(|e| {
            tracing::warn!(error = %e, "failed to mint bearer token");
            GrpcEgressError::Status(
                GrpcStatusCode::Internal,
                "failed to mint bearer token".into(),
            )
        })?;
        req.metadata
            .insert(AUTHORIZATION_HEADER.to_string(), format!("Bearer {token}"));
        Ok(())
    }

    fn after_call(&self, _req: AfterCallRequest<'_>) -> Result<(), GrpcEgressError> {
        Ok(())
    }
}
