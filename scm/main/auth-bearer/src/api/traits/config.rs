//! Configuration contract for bearer auth.

use super::super::types::{
    ApplicationConfigBuilder, BearerEgressConfig, BearerEgressConfigBuilder, BearerSecret,
    JwtClaims, JwtClaimsBuilder,
};

/// Configuration trait for bearer auth types.
///
/// This trait ensures that configuration types are integrated into the API layer
/// and referenced in trait signatures, enabling polymorphic config handling.
pub trait Config: Send + Sync {
    /// Get the application config builder.
    ///
    /// Ensures `ApplicationConfigBuilder` appears in the trait signature.
    fn app_config_builder(&self) -> Option<&ApplicationConfigBuilder>;

    /// Get the bearer config builder.
    ///
    /// Ensures [`BearerEgressConfigBuilder`] appears in the trait signature.
    fn bearer_config_builder(&self) -> Option<&BearerEgressConfigBuilder>;

    /// Get the bearer egress config.
    ///
    /// Ensures [`BearerEgressConfig`] appears in the trait signature.
    fn bearer_config(&self) -> Option<&BearerEgressConfig>;

    /// Get the bearer secret.
    ///
    /// Ensures [`BearerSecret`] appears in the trait signature.
    fn bearer_secret(&self) -> Option<&BearerSecret>;

    /// Get JWT claims builder.
    ///
    /// Ensures [`JwtClaimsBuilder`] appears in the trait signature.
    fn jwt_claims_builder(&self) -> Option<&JwtClaimsBuilder>;

    /// Get JWT claims.
    ///
    /// Ensures [`JwtClaims`] appears in the trait signature.
    fn jwt_claims(&self) -> Option<&JwtClaims>;
}
