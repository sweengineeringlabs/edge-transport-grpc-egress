//! Secret material for JWT signing.

use serde::{Deserialize, Serialize};

/// Symmetric / asymmetric secret material for JWT signing.
///
/// `Hs256` carries a raw byte secret; comparisons MUST go through
/// `BearerSecret::ct_eq_hs256` (`BearerSecretGuard` of `grpc-egress-auth-bearer-std`), which uses
/// `subtle::ConstantTimeEq`.
/// Asymmetric variants carry PEM-encoded key bytes.
///
/// # Examples
///
/// ```rust
/// use grpc_egress_auth_bearer::BearerSecret;
///
/// let secret = BearerSecret::Hs256 { secret: b"my-secret".to_vec() };
/// assert!(matches!(secret, BearerSecret::Hs256 { .. }));
///
/// // The constant-time comparison, `ct_eq_hs256`, is in `grpc-egress-auth-bearer-std`.
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BearerSecret {
    /// HS256 — shared symmetric secret.
    Hs256 {
        /// Raw secret bytes (UTF-8 in TOML; arbitrary bytes via API).
        secret: Vec<u8>,
    },
    /// RS256 — private PEM bytes for signing.
    Rs256 {
        /// PEM-encoded private key.
        #[serde(default)]
        private_pem: Vec<u8>,
    },
}
