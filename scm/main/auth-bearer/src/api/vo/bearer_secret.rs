//! Secret material for JWT signing.

use serde::{Deserialize, Serialize};

use crate::BearerAuthError;

/// Caller-owned symmetric or asymmetric material for JWT signing.
///
/// Values have private state and can only be created with non-empty key material. The `Serialize` representation
/// retains the existing tagged `kind` shape used by configuration files.
///
/// ```rust
/// use grpc_egress_auth_bearer::BearerSecret;
/// let secret = BearerSecret::hs256(b"my-secret".to_vec()).unwrap();
/// assert_eq!(secret.hs256_secret(), Some(&b"my-secret"[..]));
/// ```
///
/// ```compile_fail,E0423
/// use grpc_egress_auth_bearer::BearerSecret;
/// let _ = BearerSecret(vec![1]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SecretMaterial")]
pub struct BearerSecret(SecretMaterial);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SecretMaterial {
    Hs256 {
        secret: Vec<u8>,
    },
    Rs256 {
        #[serde(default)]
        private_pem: Vec<u8>,
    },
}

impl TryFrom<SecretMaterial> for BearerSecret {
    type Error = BearerAuthError;
    fn try_from(material: SecretMaterial) -> Result<Self, Self::Error> {
        match &material {
            SecretMaterial::Hs256 { secret } if secret.is_empty() => {
                return Err(BearerAuthError::ValidationFailed(
                    "HS256 secret must not be empty".into(),
                ))
            }
            SecretMaterial::Rs256 { private_pem } if private_pem.is_empty() => {
                return Err(BearerAuthError::ValidationFailed(
                    "RS256 private PEM must not be empty".into(),
                ))
            }
            _ => {}
        }
        Ok(Self(material))
    }
}

impl BearerSecret {
    /// Create an HS256 secret from its raw key bytes.
    pub fn hs256(secret: impl Into<Vec<u8>>) -> Result<Self, BearerAuthError> {
        Self::try_from(SecretMaterial::Hs256 {
            secret: secret.into(),
        })
    }

    /// Create an RS256 secret from PEM-encoded private-key bytes.
    pub fn rs256(private_pem: impl Into<Vec<u8>>) -> Result<Self, BearerAuthError> {
        Self::try_from(SecretMaterial::Rs256 {
            private_pem: private_pem.into(),
        })
    }

    /// Return HS256 key bytes when this value uses HS256.
    pub fn hs256_secret(&self) -> Option<&[u8]> {
        match &self.0 {
            SecretMaterial::Hs256 { secret } => Some(secret),
            _ => None,
        }
    }

    /// Return RS256 private-key PEM bytes when this value uses RS256.
    pub fn rs256_private_pem(&self) -> Option<&[u8]> {
        match &self.0 {
            SecretMaterial::Rs256 { private_pem } => Some(private_pem),
            _ => None,
        }
    }
}
